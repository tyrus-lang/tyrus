use crate::ast::ExprKind;
use crate::ast::{
    BehaviorInterfaceInfo, Expr, Field, Item, Param, ParamPattern, Program, Stmt, TemplatePart,
    Type, contains_outer_topic, resolve_enum,
};
use crate::ast::{DeferKind, StmtKind};
use crate::borrow_checker::BorrowChecker;
use crate::collections::{HashMap, HashSet};
use crate::doc_comments::{DocComment, doc_comment_before_decl, extract_doc_comments};
use crate::lexer::Token;
use crate::parser::Parser;
use crate::span::Span;
use crate::type_checker::{CheckSeverity, TypeChecker};
use std::collections::VecDeque;
use std::ops::Range;
use std::path::{Path, PathBuf};

#[path = "diagnostics/diagnostics.rs"]
mod diagnostics;
pub use diagnostics::render_diagnostic;
pub(crate) use diagnostics::*;
#[path = "refinement/refinement.rs"]
mod refinement;
pub(crate) use refinement::*;
#[path = "symbols/symbols.rs"]
mod symbols;
pub(crate) use symbols::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct AnalysisDiagnostic {
    pub code: String,
    pub range: Range<usize>,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub related: Vec<AnalysisRelatedInformation>,
    pub help: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AnalysisRelatedInformation {
    pub range: Range<usize>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Variable,
    Parameter,
    Import,
    Interface,
    TypeAlias,
    Property,
    Enum,
    EnumMember,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub ty: String,
    pub semantic_type: Option<Type>,
    pub parameters: Vec<Param>,
    pub kind: SymbolKind,
    pub is_validator: bool,
    pub declaration: Range<usize>,
    pub scope: Range<usize>,
    pub exported: bool,
    pub import_source: Option<String>,
    pub imported_name: Option<String>,
    pub documentation: Option<DocComment>,
}

#[derive(Debug, Clone)]
pub struct Reference {
    pub name: String,
    pub range: Range<usize>,
    pub definition: Option<Range<usize>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AstNodeKind {
    Program,
    Import,
    ImportSpecifier,
    Item,
    Parameter,
    Binding,
    Statement,
    Expression,
}

#[derive(Debug, Clone)]
pub struct NodeSpan {
    pub id: u32,
    pub kind: AstNodeKind,
    pub range: Range<usize>,
}

#[derive(Debug, Clone)]
pub struct ExpressionTypeInfo {
    pub range: Range<usize>,
    pub ty: String,
}

#[derive(Debug, Clone)]
pub struct CallSite {
    pub callee: String,
    pub arguments: Vec<Range<usize>>,
    pub arg_offset: usize,
    pub piped_value: Option<Range<usize>>,
    pub range: Range<usize>,
    pub receiver: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DocumentAnalysis {
    pub symbols: Vec<Symbol>,
    pub symbol_name_index: Vec<usize>,
    pub import_symbol_index: Vec<usize>,
    pub import_symbol_count: usize,
    pub named_export_index: Vec<usize>,
    pub wildcard_export_index: Vec<usize>,
    pub references: Vec<Reference>,
    pub symbol_declaration_index: Vec<usize>,
    pub reference_range_index: Vec<usize>,
    pub references_by_definition: HashMap<(usize, usize), Vec<Range<usize>>>,
    pub diagnostics: Vec<AnalysisDiagnostic>,
    pub diagnostic_range_index: Vec<(usize, usize, usize)>,
    pub diagnostic_range_prefix_max_end: Vec<usize>,
    pub node_spans: Vec<NodeSpan>,
    pub expression_types: Vec<ExpressionTypeInfo>,
    pub expression_type_index: Vec<usize>,
    pub expression_type_prefix_max_end: Vec<usize>,
    pub call_sites: Vec<CallSite>,
    pub call_site_prefix_max_end: Vec<usize>,
    pub comptime_facts: crate::comptime::ComptimeFacts,
    pub ctfe: crate::ctfe::CtfeResult,
    pub tokens: Vec<SpannedToken>,
    pub program: Option<Program>,
}

impl DocumentAnalysis {
    pub fn token_at_inclusive(&self, offset: usize) -> Option<&SpannedToken> {
        let index = self
            .tokens
            .partition_point(|token| token.range.end < offset);
        self.tokens
            .get(index)
            .filter(|token| token.range.start <= offset && offset <= token.range.end)
    }

    pub fn symbol_at(&self, offset: usize) -> Option<&Symbol> {
        if let Some(symbol) = self.symbol_at_indexed(offset) {
            return Some(symbol);
        }
        let reference = self.reference_at(offset)?;
        reference.definition.as_ref().and_then(|definition| {
            self.symbols
                .iter()
                .find(|symbol| symbol.declaration == *definition)
        })
    }

    fn symbol_at_indexed(&self, offset: usize) -> Option<&Symbol> {
        if self.symbol_declaration_index.is_empty() {
            return self
                .symbols
                .iter()
                .find(|symbol| symbol.declaration.contains(&offset));
        }
        let end = self
            .symbol_declaration_index
            .partition_point(|index| self.symbols[*index].declaration.start <= offset);
        self.symbol_declaration_index[..end]
            .iter()
            .rev()
            .take_while(|index| self.symbols[**index].declaration.end > offset)
            .find_map(|index| {
                self.symbols[*index]
                    .declaration
                    .contains(&offset)
                    .then(|| &self.symbols[*index])
            })
    }

    fn reference_at(&self, offset: usize) -> Option<&Reference> {
        if self.reference_range_index.is_empty() {
            return self
                .references
                .iter()
                .find(|reference| reference.range.contains(&offset));
        }
        let end = self
            .reference_range_index
            .partition_point(|index| self.references[*index].range.start <= offset);
        self.reference_range_index[..end]
            .iter()
            .rev()
            .take_while(|index| self.references[**index].range.end > offset)
            .find_map(|index| {
                self.references[*index]
                    .range
                    .contains(&offset)
                    .then(|| &self.references[*index])
            })
    }

    pub fn symbol_named(&self, name: &str) -> Option<&Symbol> {
        if self.symbol_name_index.len() != self.symbols.len() {
            return self.symbols.iter().find(|symbol| symbol.name == name);
        }
        let start = self
            .symbol_name_index
            .partition_point(|index| self.symbols[*index].name.as_str() < name);
        self.symbol_name_index[start..]
            .iter()
            .take_while(|index| self.symbols[**index].name == name)
            .min()
            .map(|index| &self.symbols[*index])
    }

    pub fn exported_symbol_named(&self, name: &str) -> Option<&Symbol> {
        if self.symbol_name_index.len() != self.symbols.len() {
            return self
                .symbols
                .iter()
                .find(|symbol| symbol.exported && symbol.name == name);
        }
        let start = self
            .symbol_name_index
            .partition_point(|index| self.symbols[*index].name.as_str() < name);
        self.symbol_name_index[start..]
            .iter()
            .take_while(|index| self.symbols[**index].name == name)
            .find_map(|index| self.symbols[*index].exported.then(|| &self.symbols[*index]))
    }

    pub fn imported_symbol_indexes_named(&self, name: &str) -> Option<&[usize]> {
        if self.program.is_none() || self.import_symbol_index.len() != self.import_symbol_count {
            return None;
        }
        let start = self.import_symbol_index.partition_point(|index| {
            self.symbols[*index]
                .imported_name
                .as_deref()
                .unwrap_or(&self.symbols[*index].name)
                < name
        });
        let end = self.import_symbol_index.partition_point(|index| {
            self.symbols[*index]
                .imported_name
                .as_deref()
                .unwrap_or(&self.symbols[*index].name)
                <= name
        });
        Some(&self.import_symbol_index[start..end])
    }

    pub fn named_export_indexes(&self, name: &str) -> &[usize] {
        let start = self.named_export_index.partition_point(|index| {
            self.program
                .as_ref()
                .and_then(|program| program.exports[*index].exported_name())
                .is_some_and(|exported| exported < name)
        });
        let end = self.named_export_index.partition_point(|index| {
            self.program
                .as_ref()
                .and_then(|program| program.exports[*index].exported_name())
                .is_some_and(|exported| exported <= name)
        });
        &self.named_export_index[start..end]
    }

    pub fn wildcard_export_indexes(&self) -> &[usize] {
        &self.wildcard_export_index
    }

    pub fn symbol_by_declaration(&self, range: &Range<usize>) -> Option<&Symbol> {
        if self.symbol_declaration_index.is_empty() {
            return self
                .symbols
                .iter()
                .find(|symbol| symbol.declaration == *range);
        }
        let end = self
            .symbol_declaration_index
            .partition_point(|index| self.symbols[*index].declaration.start <= range.start);
        self.symbol_declaration_index[..end]
            .iter()
            .rev()
            .take_while(|index| self.symbols[**index].declaration.start == range.start)
            .find_map(|index| {
                (self.symbols[*index].declaration == *range).then(|| &self.symbols[*index])
            })
    }

    pub fn diagnostic_at(&self, offset: usize) -> Option<&AnalysisDiagnostic> {
        if self.diagnostic_range_index.is_empty() {
            return self.diagnostics.iter().find(|diagnostic| {
                diagnostic.range.contains(&offset)
                    || diagnostic
                        .related
                        .iter()
                        .any(|related| related.range.contains(&offset))
            });
        }
        let first = self
            .diagnostic_range_prefix_max_end
            .partition_point(|maximum_end| *maximum_end <= offset);
        let end = self
            .diagnostic_range_index
            .partition_point(|(start, _, _)| *start <= offset);
        self.diagnostic_range_index[first.min(end)..end]
            .iter()
            .filter(|(_, range_end, _)| *range_end > offset)
            .map(|(_, _, diagnostic_index)| diagnostic_index)
            .min()
            .map(|diagnostic_index| &self.diagnostics[*diagnostic_index])
    }

    pub fn type_at(&self, offset: usize) -> Option<&ExpressionTypeInfo> {
        if self.expression_type_index.len() != self.expression_types.len() {
            return self
                .expression_types
                .iter()
                .rev()
                .filter(|expression| expression.range.contains(&offset))
                .min_by_key(|expression| expression.range.end - expression.range.start);
        }
        let first = self
            .expression_type_prefix_max_end
            .partition_point(|maximum_end| *maximum_end <= offset);
        let end = self
            .expression_type_index
            .partition_point(|index| self.expression_types[*index].range.start <= offset);
        self.expression_type_index[first.min(end)..end]
            .iter()
            .filter(|index| self.expression_types[**index].range.contains(&offset))
            .min_by_key(|index| {
                (
                    self.expression_types[**index].range.end
                        - self.expression_types[**index].range.start,
                    std::cmp::Reverse(**index),
                )
            })
            .map(|index| &self.expression_types[*index])
    }

    pub fn call_site_at(&self, offset: usize) -> Option<&CallSite> {
        if self.call_site_prefix_max_end.len() != self.call_sites.len() {
            return self
                .call_sites
                .iter()
                .filter(|call| call.range.contains(&offset))
                .min_by_key(|call| call.range.end - call.range.start);
        }
        let first = self
            .call_site_prefix_max_end
            .partition_point(|maximum_end| *maximum_end <= offset);
        let end = self
            .call_sites
            .partition_point(|call| call.range.start <= offset);
        self.call_sites[first.min(end)..end]
            .iter()
            .enumerate()
            .filter(|(_, call)| call.range.contains(&offset))
            .min_by_key(|(index, call)| {
                (call.range.end - call.range.start, first.min(end) + *index)
            })
            .map(|(_, call)| call)
    }

    pub fn parameter_refs(&self, _source: &str) -> Vec<Range<usize>> {
        let parameters: Vec<&Symbol> = self
            .symbols
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::Parameter)
            .collect();
        self.references
            .iter()
            .filter(|reference| {
                reference.definition.as_ref().is_some_and(|definition| {
                    parameters
                        .iter()
                        .any(|parameter| parameter.declaration == *definition)
                })
            })
            .map(|reference| reference.range.clone())
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub range: Range<usize>,
}

#[derive(Debug, Clone)]
struct SymbolPrototype {
    ty: Type,
    display_ty: Option<String>,
    parameters: Vec<Param>,
    kind: SymbolKind,
}

#[derive(Default, Clone, Copy)]
pub struct AnalyzeContext<'a> {
    pub import_types: Option<&'a HashMap<String, Type>>,
    pub import_targets: Option<&'a HashMap<String, String>>,
    pub entry_point: bool,
    pub import_interfaces: Option<&'a HashMap<String, Vec<Field>>>,
    pub import_behaviors: Option<&'a HashMap<String, BehaviorInterfaceInfo>>,
    pub import_type_aliases: Option<&'a HashMap<String, Type>>,
    pub import_enums: Option<&'a HashMap<String, crate::ast::ResolvedEnum>>,
    pub import_throws: Option<&'a HashMap<String, Vec<Type>>>,
    pub import_db_models: Option<&'a HashMap<String, crate::ast::DbModelMetadata>>,
    pub import_validators: Option<&'a HashMap<String, Type>>,
    pub reachable_interfaces: Option<&'a HashMap<String, Vec<Field>>>,
    pub env_schema: Option<&'a crate::env::EnvSchema>,
    pub db_schema: Option<&'a crate::db::DbSchemaSnapshot>,
    pub db_schema_error: Option<&'a str>,
    pub db_backend: Option<&'a crate::db::DatabaseBackend>,
}

pub fn collect_reachable_interfaces(
    roots: &[Vec<Field>],
    lookup: &dyn Fn(&str) -> Option<Vec<Field>>,
    reachable: &mut HashMap<String, Vec<Field>>,
) {
    let mut pending = roots
        .iter()
        .flatten()
        .map(|field| field.ty.clone())
        .collect::<Vec<_>>();
    while let Some(ty) = pending.pop() {
        let mut names = Vec::new();
        named_types_in(&ty, &mut names);
        for name in names {
            if reachable.contains_key(&name) {
                continue;
            }
            if let Some(fields) = lookup(&name) {
                pending.extend(fields.iter().map(|field| field.ty.clone()));
                reachable.insert(name, fields);
            }
        }
    }
}

fn named_types_in(ty: &Type, names: &mut Vec<String>) {
    match ty {
        Type::Named(name) => names.push(name.clone()),
        Type::Apply { name, args } => {
            names.push(name.clone());
            for arg in args {
                named_types_in(arg, names);
            }
        }
        Type::Array(inner)
        | Type::Optional(inner)
        | Type::Readonly(inner)
        | Type::Ref(inner)
        | Type::MutRef(inner)
        | Type::KeyOf(inner) => named_types_in(inner, names),
        Type::Result { success, error } => {
            named_types_in(success, names);
            named_types_in(error, names);
        }
        Type::Tuple(items) | Type::Union(items) => {
            for item in items {
                named_types_in(item, names);
            }
        }
        Type::Object(fields) => {
            for field in fields {
                named_types_in(&field.ty, names);
            }
        }
        _ => {}
    }
}

pub fn analyze(source: &str) -> DocumentAnalysis {
    analyze_with_context(source, AnalyzeContext::default())
}

/// Analyzes a standalone source document with declarations for its imported native exports.
/// Project-relative imports are intentionally not resolved by this entry point.
pub fn analyze_standalone_source(source: &str) -> DocumentAnalysis {
    analyze_standalone_source_cancellable(
        source,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    )
    .expect("analysis without a cancellation signal cannot be cancelled")
}

pub fn analyze_standalone_source_cancellable(
    source: &str,
    cancellation: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<DocumentAnalysis, AnalysisCancelled> {
    let (_, program) =
        match parse_for_analysis_cancellable(source, std::sync::Arc::clone(&cancellation)) {
            Ok(parsed) => parsed,
            Err(ParseForAnalysis::Cancelled) => return Err(AnalysisCancelled),
            Err(_) => {
                return analyze_with_context_cancellable(
                    source,
                    AnalyzeContext::default(),
                    cancellation,
                );
            }
        };
    let mut import_types = HashMap::default();
    let mut import_interfaces = HashMap::default();
    let mut import_behaviors = HashMap::default();
    let mut import_type_aliases = HashMap::default();
    let mut import_enums = HashMap::default();
    let mut import_throws = HashMap::default();
    let mut import_db_models = HashMap::default();
    for import in &program.imports {
        if !import.source.starts_with("tyrus:") {
            continue;
        }
        for specifier in &import.specifiers {
            if specifier.kind == crate::ast::ImportKind::Namespace {
                continue;
            }
            let Some(info) = crate::module_loader::native_export_analysis_info(
                &import.source,
                &specifier.imported,
            ) else {
                continue;
            };
            import_types.insert(specifier.local.clone(), info.ty);
            if let Some(fields) = info.interface_fields {
                import_interfaces.insert(specifier.local.clone(), fields);
            }
            if let Some(behavior) = info.behavior_info {
                import_behaviors.insert(specifier.local.clone(), behavior);
            }
            if let Some(alias) = info.type_alias_ty {
                import_type_aliases.insert(specifier.local.clone(), alias);
            }
            if let Some(enum_info) = info.enum_info {
                import_enums.insert(specifier.local.clone(), enum_info);
            }
            if !info.throws.is_empty() {
                import_throws.insert(specifier.local.clone(), info.throws);
            }
            if let Some(db_model) = info.db_model {
                import_db_models.insert(specifier.local.clone(), db_model);
            }
        }
    }
    analyze_with_context_cancellable(
        source,
        AnalyzeContext {
            import_types: Some(&import_types),
            import_interfaces: Some(&import_interfaces),
            import_behaviors: Some(&import_behaviors),
            import_type_aliases: Some(&import_type_aliases),
            import_enums: Some(&import_enums),
            import_throws: Some(&import_throws),
            import_db_models: Some(&import_db_models),
            ..AnalyzeContext::default()
        },
        cancellation,
    )
}

pub fn analyze_with_imports(
    source: &str,
    import_types: &HashMap<String, Type>,
    import_interfaces: &HashMap<String, Vec<Field>>,
    import_type_aliases: &HashMap<String, Type>,
) -> DocumentAnalysis {
    analyze_with_context(
        source,
        AnalyzeContext {
            import_types: Some(import_types),
            import_targets: None,
            entry_point: false,
            import_interfaces: Some(import_interfaces),
            import_behaviors: None,
            import_type_aliases: Some(import_type_aliases),
            import_enums: None,
            import_throws: None,
            import_db_models: None,
            import_validators: None,
            reachable_interfaces: None,
            env_schema: None,
            db_schema: None,
            db_schema_error: None,
            db_backend: None,
        },
    )
}

pub fn analyze_with_context(source: &str, context: AnalyzeContext) -> DocumentAnalysis {
    analyze_with_context_controlled(source, context, None)
        .expect("analysis without a cancellation signal cannot be cancelled")
}

pub fn analyze_with_context_cancellable(
    source: &str,
    context: AnalyzeContext,
    cancellation: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<DocumentAnalysis, AnalysisCancelled> {
    analyze_with_context_controlled(source, context, Some(cancellation))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalysisCancelled;

fn analyze_with_context_controlled(
    source: &str,
    context: AnalyzeContext,
    cancellation: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
) -> Result<DocumentAnalysis, AnalysisCancelled> {
    if cancellation
        .as_ref()
        .is_some_and(|signal| signal.load(std::sync::atomic::Ordering::Acquire))
    {
        return Err(AnalysisCancelled);
    }
    let mut diagnostics = Vec::new();
    let lexed = if let Some(signal) = &cancellation {
        match crate::lexer::lex_spanned_cancellable(source, signal) {
            Ok(tokens) => Ok(tokens),
            Err(crate::lexer::LexControlError::Lex(error)) => Err(error),
            Err(crate::lexer::LexControlError::Cancelled) => return Err(AnalysisCancelled),
        }
    } else {
        crate::lexer::lex_spanned(source)
    };
    let (parser_tokens, parser_ranges) = match lexed {
        Ok(tokens) => tokens.into_iter().unzip(),
        Err(error) => {
            diagnostics.push(AnalysisDiagnostic {
                code: "E0001".to_string(),
                range: error.range,
                severity: DiagnosticSeverity::Error,
                message: "invalid token".to_string(),
                related: Vec::new(),
                help: Vec::new(),
            });
            (Vec::new(), Vec::new())
        }
    };

    // A missing `;` is what a half-typed statement looks like, and losing the
    // whole analysis over it would take hover, completion and every semantic
    // diagnostic away exactly while the file is being written. So the missing
    // terminator is reported and then spliced in, and the parse is retried, which
    // keeps the editor working on the rest of the file. MISSING_TERMINATOR_RETRIES
    // bounds the loop; a file with more unterminated statements than that gets the
    // plain error.
    const MISSING_TERMINATOR_RETRIES: usize = 16;
    let mut parser = Parser::new_with_ranges(parser_tokens, parser_ranges);
    if let Some(signal) = &cancellation {
        parser = parser.with_cancellation(std::sync::Arc::clone(signal));
    }
    let mut recovered_terminators = 0;
    let mut program = loop {
        match parser.parse_program() {
            Ok(program) => break program,
            Err(error) => {
                if parser.is_cancelled() {
                    return Err(AnalysisCancelled);
                }
                let range = error
                    .span
                    .clone()
                    .unwrap_or_else(|| parser.position_range().unwrap_or(0..source.len().min(1)));
                let code = error.code.unwrap_or("E0002").to_string();
                let recoverable =
                    code == "E3218" && recovered_terminators < MISSING_TERMINATOR_RETRIES;
                diagnostics.push(AnalysisDiagnostic {
                    code,
                    range: range.clone(),
                    severity: DiagnosticSeverity::Error,
                    message: error.message,
                    related: Vec::new(),
                    help: error.help,
                });
                if !recoverable {
                    return Ok(DocumentAnalysis {
                        diagnostics,
                        ..DocumentAnalysis::default()
                    });
                }
                recovered_terminators += 1;
                let at = parser.cursor();
                let (mut tokens, mut ranges) = parser.into_tokens();
                let at = at.min(tokens.len());
                // The synthetic `;` is empty and sits where the real one belongs,
                // so every range after it keeps pointing at the source as written.
                let insert_at = ranges
                    .get(at.saturating_sub(1))
                    .map_or(range.end, |previous| previous.end);
                tokens.insert(at, Token::Semicolon);
                ranges.insert(at, insert_at..insert_at);
                parser = Parser::new_with_ranges(tokens, ranges);
                if let Some(signal) = &cancellation {
                    parser = parser.with_cancellation(std::sync::Arc::clone(signal));
                }
            }
        }
    };

    if cancellation
        .as_ref()
        .is_some_and(|signal| signal.load(std::sync::atomic::Ordering::Acquire))
    {
        return Err(AnalysisCancelled);
    }

    if let Some(backend) = context.db_backend {
        crate::db::apply_backend_model_defaults(&mut program, Some(backend));
    }

    // The parser is done with the stream, so the tokens move out instead of
    // being cloned; the passes below need the same tokens with their ranges.
    let (parser_tokens, parser_ranges) = parser.into_tokens();
    let tokens = parser_tokens
        .into_iter()
        .zip(parser_ranges)
        .map(|(token, range)| SpannedToken { token, range })
        .collect::<Vec<_>>();

    let analysis = analyze_parsed_program(
        source,
        tokens,
        program,
        context,
        diagnostics,
        true,
        cancellation.clone(),
    );
    if cancellation
        .as_ref()
        .is_some_and(|signal| signal.load(std::sync::atomic::Ordering::Acquire))
    {
        return Err(AnalysisCancelled);
    }
    Ok(analysis)
}

/// Lexes and parses `source` the way `analyze_with_context` does, for a caller
/// that wants the pieces rather than the analysis. `ModuleLoader` uses it so a
/// module is read once instead of once here and once in the loader.
pub fn parse_for_analysis(source: &str) -> Result<(Vec<SpannedToken>, Program), ParseForAnalysis> {
    let spanned = crate::lexer::lex_spanned(source).map_err(ParseForAnalysis::Lex)?;
    let (parser_tokens, parser_ranges) = spanned.into_iter().unzip();
    let mut parser = Parser::new_with_ranges(parser_tokens, parser_ranges);
    let program = match parser.parse_program() {
        Ok(program) => program,
        Err(error) => {
            let range = error
                .span
                .clone()
                .or_else(|| parser.position_range())
                .unwrap_or(0..source.len().min(1));
            return Err(ParseForAnalysis::Parse(Box::new(error), range));
        }
    };
    let (parser_tokens, parser_ranges) = parser.into_tokens();
    let tokens = parser_tokens
        .into_iter()
        .zip(parser_ranges)
        .map(|(token, range)| SpannedToken { token, range })
        .collect::<Vec<_>>();
    Ok((tokens, program))
}

pub fn parse_for_analysis_cancellable(
    source: &str,
    cancellation: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<(Vec<SpannedToken>, Program), ParseForAnalysis> {
    parse_for_analysis_cancellable_with_metrics(source, cancellation).0
}

/// Timings and token count collected while lexing and parsing one document.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ParseForAnalysisMetrics {
    pub lexing: std::time::Duration,
    pub parsing: std::time::Duration,
    pub token_count: usize,
}

/// Lexes and parses a document while reporting the cost of each stage.
pub fn parse_for_analysis_cancellable_with_metrics(
    source: &str,
    cancellation: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> (
    Result<(Vec<SpannedToken>, Program), ParseForAnalysis>,
    ParseForAnalysisMetrics,
) {
    let mut metrics = ParseForAnalysisMetrics::default();
    if cancellation.load(std::sync::atomic::Ordering::Acquire) {
        return (Err(ParseForAnalysis::Cancelled), metrics);
    }
    let lexing_started = std::time::Instant::now();
    let spanned = match crate::lexer::lex_spanned_cancellable(source, &cancellation) {
        Ok(spanned) => spanned,
        Err(crate::lexer::LexControlError::Lex(error)) => {
            metrics.lexing = lexing_started.elapsed();
            return (Err(ParseForAnalysis::Lex(error)), metrics);
        }
        Err(crate::lexer::LexControlError::Cancelled) => {
            metrics.lexing = lexing_started.elapsed();
            return (Err(ParseForAnalysis::Cancelled), metrics);
        }
    };
    metrics.lexing = lexing_started.elapsed();
    metrics.token_count = spanned.len();
    if cancellation.load(std::sync::atomic::Ordering::Acquire) {
        return (Err(ParseForAnalysis::Cancelled), metrics);
    }
    let (parser_tokens, parser_ranges) = spanned.into_iter().unzip();
    let mut parser = Parser::new_with_ranges(parser_tokens, parser_ranges)
        .with_cancellation(std::sync::Arc::clone(&cancellation));
    let parsing_started = std::time::Instant::now();
    let program = match parser.parse_program() {
        Ok(program) => program,
        Err(_) if parser.is_cancelled() => {
            metrics.parsing = parsing_started.elapsed();
            return (Err(ParseForAnalysis::Cancelled), metrics);
        }
        Err(error) => {
            metrics.parsing = parsing_started.elapsed();
            let range = error
                .span
                .clone()
                .or_else(|| parser.position_range())
                .unwrap_or(0..source.len().min(1));
            return (
                Err(ParseForAnalysis::Parse(Box::new(error), range)),
                metrics,
            );
        }
    };
    metrics.parsing = parsing_started.elapsed();
    if parser.is_cancelled() {
        return (Err(ParseForAnalysis::Cancelled), metrics);
    }
    let (parser_tokens, parser_ranges) = parser.into_tokens();
    let tokens = parser_tokens
        .into_iter()
        .zip(parser_ranges)
        .map(|(token, range)| SpannedToken { token, range })
        .collect::<Vec<_>>();
    (Ok((tokens, program)), metrics)
}

/// What `parse_for_analysis` could not do.
pub enum ParseForAnalysis {
    Lex(crate::lexer::LexError),
    Parse(Box<crate::parser::ParseError>, Range<usize>),
    Cancelled,
}

/// The analysis itself, on a program that is already lexed and parsed.
pub fn analyze_parsed(
    source: &str,
    tokens: Vec<SpannedToken>,
    program: Program,
    context: AnalyzeContext,
) -> DocumentAnalysis {
    analyze_parsed_with_db_model_schema_validation(source, tokens, program, context, true)
}

pub fn analyze_parsed_for_db_migrations(
    source: &str,
    tokens: Vec<SpannedToken>,
    program: Program,
    context: AnalyzeContext,
) -> DocumentAnalysis {
    analyze_parsed_with_db_model_schema_validation(source, tokens, program, context, false)
}

fn analyze_parsed_with_db_model_schema_validation(
    source: &str,
    tokens: Vec<SpannedToken>,
    program: Program,
    context: AnalyzeContext,
    validate_db_models: bool,
) -> DocumentAnalysis {
    analyze_parsed_program(
        source,
        tokens,
        program,
        context,
        Vec::new(),
        validate_db_models,
        None,
    )
}

#[allow(clippy::too_many_lines)]
fn analyze_parsed_program(
    source: &str,
    tokens: Vec<SpannedToken>,
    mut program: Program,
    context: AnalyzeContext,
    mut diagnostics: Vec<AnalysisDiagnostic>,
    validate_db_models: bool,
    cancellation: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
) -> DocumentAnalysis {
    macro_rules! stop_if_cancelled {
        () => {
            if cancellation
                .as_ref()
                .is_some_and(|signal| signal.load(std::sync::atomic::Ordering::Acquire))
            {
                return DocumentAnalysis::default();
            }
        };
    }
    stop_if_cancelled!();
    let lint_program = program.clone();
    let empty_types = HashMap::default();
    let empty_interfaces = HashMap::default();
    let empty_aliases = HashMap::default();
    let empty_enums = HashMap::default();
    let empty_throws = HashMap::default();
    let import_types = context.import_types.unwrap_or(&empty_types);
    let import_interfaces = context.import_interfaces.unwrap_or(&empty_interfaces);
    let empty_behaviors = HashMap::default();
    let import_behaviors = context.import_behaviors.unwrap_or(&empty_behaviors);
    let import_type_aliases = context.import_type_aliases.unwrap_or(&empty_aliases);
    let import_enums = context.import_enums.unwrap_or(&empty_enums);
    let import_throws = context.import_throws.unwrap_or(&empty_throws);
    let empty_reachable = HashMap::default();
    let reachable_interfaces = context.reachable_interfaces.unwrap_or(&empty_reachable);

    crate::module_loader::desugar_namespace_imports(&mut program);
    stop_if_cancelled!();
    let resolve_diagnostics = crate::type_resolve::normalize_program_with_imports(
        &mut program,
        import_type_aliases,
        import_types,
    );
    stop_if_cancelled!();

    let mut prototypes: HashMap<String, VecDeque<SymbolPrototype>> = HashMap::default();
    let (node_spans, mut call_sites) =
        NodeSpanCollector::new(&tokens, source.len()).collect(&program);
    call_sites.sort_by_key(|call| (call.range.start, call.range.end));
    let mut call_site_prefix_max_end = Vec::with_capacity(call_sites.len());
    let mut maximum_end = 0;
    for call in &call_sites {
        stop_if_cancelled!();
        maximum_end = maximum_end.max(call.range.end);
        call_site_prefix_max_end.push(maximum_end);
    }
    let interface_fields = collect_interface_fields(&program);
    collect_program_prototypes(&program, &mut prototypes, &interface_fields);
    let exported = program
        .exports
        .iter()
        .filter_map(|export| match export {
            crate::ast::ExportDecl::Named { local, .. } => Some(local.clone()),
            crate::ast::ExportDecl::Re { .. } | crate::ast::ExportDecl::All { .. } => None,
        })
        .collect::<HashSet<_>>();
    let doc_comments = extract_doc_comments(source);
    let mut symbols = Vec::new();
    let mut declaration_tokens = HashSet::default();
    let mut import_tokens = HashSet::default();

    collect_import_symbols(
        &program,
        &tokens,
        &mut symbols,
        &mut declaration_tokens,
        &mut import_tokens,
    );
    collect_type_declaration_symbols(
        &program,
        &tokens,
        source,
        &doc_comments,
        &exported,
        &mut symbols,
        &mut declaration_tokens,
    );
    for symbol in &mut symbols {
        stop_if_cancelled!();
        if let Some(ty) = import_types.get(&symbol.name) {
            symbol.ty = ty.to_string();
            symbol.semantic_type = Some(ty.clone());
        }
    }

    for (index, token) in tokens.iter().enumerate() {
        stop_if_cancelled!();
        let Token::Ident(name) = &token.token else {
            continue;
        };
        let variable_declaration = index > 0
            && matches!(tokens[index - 1].token, Token::Const | Token::Let)
            || node_spans.iter().any(|node| {
                node.kind == AstNodeKind::Binding
                    && node.range.start <= token.range.start
                    && token.range.end <= node.range.end
            });
        let parameter_declaration = node_spans.iter().any(|node| {
            node.kind == AstNodeKind::Parameter
                && node.range.start <= token.range.start
                && token.range.end <= node.range.end
        });
        if !variable_declaration && !parameter_declaration {
            continue;
        }
        if declaration_tokens.contains(&index) {
            continue;
        }
        let prototype = prototypes
            .get_mut(name)
            .and_then(VecDeque::pop_front)
            .unwrap_or(SymbolPrototype {
                ty: Type::Void,
                display_ty: None,
                parameters: Vec::new(),
                kind: if parameter_declaration {
                    SymbolKind::Parameter
                } else {
                    SymbolKind::Variable
                },
            });
        declaration_tokens.insert(index);
        let display_ty = prototype
            .display_ty
            .unwrap_or_else(|| prototype.ty.to_string());
        symbols.push(Symbol {
            name: name.clone(),
            ty: display_ty,
            semantic_type: Some(prototype.ty),
            parameters: prototype.parameters,
            is_validator: false,
            kind: prototype.kind,
            declaration: token.range.clone(),
            scope: 0..source.len(),
            exported: exported.contains(name),
            import_source: None,
            imported_name: None,
            documentation: None,
        });
    }

    attach_documentation(source, &doc_comments, &mut symbols);

    let mut references = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        stop_if_cancelled!();
        if let Token::TemplateLit(content) = &token.token {
            collect_template_references(content, token.range.start + 1, &mut references);
        }
        let Token::Ident(name) = &token.token else {
            continue;
        };
        if declaration_tokens.contains(&index)
            || import_tokens.contains(&index)
            || name == "mut"
            || (index > 0 && matches!(tokens[index - 1].token, Token::Dot | Token::OptChain))
        {
            continue;
        }
        references.push(Reference {
            name: name.clone(),
            range: token.range.clone(),
            definition: None,
        });
    }

    assign_symbol_scopes(&mut symbols, &tokens, source.len());
    for reference in &mut references {
        stop_if_cancelled!();
        reference.definition = symbols
            .iter()
            .filter(|symbol| {
                symbol.name == reference.name
                    && symbol.scope.start <= reference.range.start
                    && reference.range.start <= symbol.scope.end
                    && (symbol.declaration.start <= reference.range.start
                        || matches!(symbol.kind, SymbolKind::Function | SymbolKind::Import))
            })
            .max_by_key(|symbol| (symbol.scope.start, symbol.declaration.start))
            .map(|symbol| symbol.declaration.clone());
    }

    // After the spans above are collected from the source as written, and
    // before any checker runs: the module loader applies the same rewrite to
    // the program it hands to code generation, so both see one program.
    crate::await_hoist::hoist_nested_awaits(&mut program);
    stop_if_cancelled!();
    let mut checker = TypeChecker::new();
    if let Some(signal) = cancellation.clone() {
        checker.set_cancellation(signal);
    }
    checker.set_source_text(source);
    checker.set_db_schema(context.db_schema.cloned(), context.db_schema_error);
    checker.set_db_backend(context.db_backend.cloned());
    checker.set_db_schema_validation(validate_db_models);
    if let Some(schema) = context.env_schema {
        checker.set_env_schema(schema.object_type());
    }
    if let Some(targets) = context.import_targets {
        checker.set_import_targets(targets.clone());
    }
    checker.set_entry_point(context.entry_point);
    let empty_import_validators = HashMap::default();
    let empty_import_db_models = HashMap::default();
    let mut check_result = checker.analyze_program_with_imports(
        &program,
        crate::type_checker::ImportedProgramSymbols {
            types: import_types,
            interfaces: import_interfaces,
            behaviors: import_behaviors,
            type_aliases: import_type_aliases,
            enums: import_enums,
            throws: import_throws,
            db_models: context.import_db_models.unwrap_or(&empty_import_db_models),
            validators: context
                .import_validators
                .unwrap_or(&empty_import_validators),
            reachable_interfaces,
        },
    );
    stop_if_cancelled!();
    for symbol in symbols
        .iter_mut()
        .filter(|symbol| matches!(symbol.kind, SymbolKind::Function | SymbolKind::Variable))
    {
        stop_if_cancelled!();
        let local_binding = check_result
            .resolved_bindings
            .iter()
            .filter(|binding| {
                binding.name == symbol.name
                    && binding.span.start <= symbol.declaration.start
                    && symbol.declaration.end <= binding.span.end
            })
            .min_by_key(|binding| binding.span.end.saturating_sub(binding.span.start));
        let ty = local_binding
            .map(|binding| &binding.ty)
            .or_else(|| check_result.resolved_symbols.get(&symbol.name));
        if let Some(ty) = ty {
            let rejected_inference = matches!(ty, Type::Unknown)
                && check_result.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == "E0310"
                        && diagnostic.subject.as_deref() == Some(symbol.name.as_str())
                });
            if rejected_inference {
                continue;
            }
            symbol.ty = display_symbol_type(symbol, ty);
            symbol.semantic_type = Some(ty.clone());
        }
    }
    for binding in &check_result.onfail_bindings {
        stop_if_cancelled!();
        let range = binding.span.to_range();
        symbols.push(Symbol {
            name: binding.name.clone(),
            ty: binding.ty.to_string(),
            semantic_type: Some(binding.ty.clone()),
            parameters: Vec::new(),
            is_validator: false,
            kind: SymbolKind::Variable,
            declaration: range.clone(),
            scope: range,
            exported: false,
            import_source: None,
            imported_name: None,
            documentation: None,
        });
    }
    for reference in &mut references {
        stop_if_cancelled!();
        reference.definition = symbols
            .iter()
            .filter(|symbol| {
                symbol.name == reference.name
                    && symbol.scope.start <= reference.range.start
                    && reference.range.start <= symbol.scope.end
                    && (symbol.declaration.start <= reference.range.start
                        || matches!(symbol.kind, SymbolKind::Function | SymbolKind::Import))
            })
            .max_by_key(|symbol| (symbol.scope.start, symbol.declaration.start))
            .map(|symbol| symbol.declaration.clone());
    }
    for diagnostic in resolve_diagnostics {
        check_result
            .diagnostics
            .push(crate::type_checker::CheckDiagnostic {
                code: diagnostic.code,
                severity: crate::type_checker::CheckSeverity::Error,
                message: diagnostic.message,
                subject: diagnostic.subject,
                help: Vec::new(),
                prefer_declaration: false,
                span: None,
            });
    }
    let contextual_parameters = &check_result.contextual_parameters;
    for symbol in symbols.iter_mut().filter(|symbol| {
        matches!(symbol.kind, SymbolKind::Parameter | SymbolKind::Variable)
            && matches!(symbol.semantic_type, Some(Type::Infer))
    }) {
        if let Some(inferred) = contextual_parameters
            .iter()
            .find(|parameter| parameter.name == symbol.name)
        {
            symbol.ty = inferred.ty.to_string();
            symbol.semantic_type = Some(inferred.ty.clone());
        }
    }
    // The filter hides the length from the iterator, so collecting grows the
    // vector step by step; the zip bounds it by the inferred expressions.
    let mut expression_types = Vec::with_capacity(check_result.expressions.len());
    expression_types.extend(
        node_spans
            .iter()
            .filter(|node| node.kind == AstNodeKind::Expression)
            .zip(check_result.expressions.iter())
            .map(|(node, inferred)| ExpressionTypeInfo {
                range: node.range.clone(),
                ty: inferred.ty.clone(),
            }),
    );
    let mut sites = UsedDiagnosticSites::default();
    for diagnostic in check_result.diagnostics {
        push_refined_diagnostic(
            diagnostic,
            &mut diagnostics,
            DiagnosticInputs {
                source,
                tokens: &tokens,
                symbols: &symbols,
                references: &references,
                env_schema: context.env_schema,
            },
            &mut sites,
        );
    }

    let comptime_facts = crate::comptime::analyze(&program, source);
    diagnostics.extend(comptime_facts.diagnostics.iter().map(|diagnostic| {
        AnalysisDiagnostic {
            code: diagnostic.code.to_string(),
            range: diagnostic.span.to_range(),
            severity: DiagnosticSeverity::Error,
            message: diagnostic.message.clone(),
            related: diagnostic
                .related
                .iter()
                .map(|related| AnalysisRelatedInformation {
                    range: related.span.to_range(),
                    message: related.message.clone(),
                })
                .collect(),
            help: diagnostic.help.clone(),
        }
    }));

    let ctfe = crate::ctfe::evaluate(&program, &comptime_facts);
    diagnostics.extend(ctfe.diagnostics.iter().map(|diagnostic| {
        AnalysisDiagnostic {
            code: diagnostic.code.to_string(),
            range: diagnostic.span.to_range(),
            severity: DiagnosticSeverity::Error,
            message: diagnostic.message.clone(),
            related: diagnostic
                .related
                .iter()
                .map(|related| AnalysisRelatedInformation {
                    range: related.span.to_range(),
                    message: related.message.clone(),
                })
                .collect(),
            help: diagnostic.help.clone(),
        }
    }));

    let local_validators = program
        .items
        .iter()
        .filter_map(|item| match item {
            Item::FnDecl {
                name,
                is_validator: true,
                ..
            } => Some(name.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();

    // Nothing reads `program` after this point, so the constant folding is
    // applied in place; copying the whole tree here was the single largest
    // source of `Expr::clone` in the allocation profile.
    let mut projected_program = program;
    crate::ctfe::apply_evaluations(&mut projected_program, &ctfe.evaluations);
    let mut checker = BorrowChecker::new();
    let mut interface_fields = crate::builtin_interfaces::builtin_interfaces();
    interface_fields.extend(
        import_interfaces
            .iter()
            .chain(reachable_interfaces.iter())
            .map(|(name, fields)| (name.clone(), fields.clone())),
    );
    checker.set_interface_fields(interface_fields);
    for message in checker.check_program(&projected_program) {
        push_checker_diagnostic(
            &mut diagnostics,
            &symbols,
            &references,
            &tokens,
            DiagnosticSeverity::Error,
            "E0500",
            message,
        );
    }
    for message in checker.warnings() {
        push_checker_diagnostic(
            &mut diagnostics,
            &symbols,
            &references,
            &tokens,
            DiagnosticSeverity::Warning,
            "W0500",
            message.clone(),
        );
    }

    for symbol in &mut symbols {
        symbol.is_validator = local_validators.contains(&symbol.name)
            || context
                .import_validators
                .is_some_and(|validators| validators.contains_key(&symbol.name));
    }

    let mut symbol_declaration_index = (0..symbols.len()).collect::<Vec<_>>();
    symbol_declaration_index.sort_by_key(|index| symbols[*index].declaration.start);
    let mut symbol_name_index = (0..symbols.len()).collect::<Vec<_>>();
    symbol_name_index.sort_by(|left, right| {
        symbols[*left]
            .name
            .cmp(&symbols[*right].name)
            .then_with(|| left.cmp(right))
    });
    let mut import_symbol_index = symbols
        .iter()
        .enumerate()
        .filter_map(|(index, symbol)| (symbol.kind == SymbolKind::Import).then_some(index))
        .collect::<Vec<_>>();
    import_symbol_index.sort_by(|left, right| {
        symbols[*left]
            .imported_name
            .as_deref()
            .unwrap_or(&symbols[*left].name)
            .cmp(
                symbols[*right]
                    .imported_name
                    .as_deref()
                    .unwrap_or(&symbols[*right].name),
            )
            .then_with(|| left.cmp(right))
    });
    let import_symbol_count = import_symbol_index.len();
    let mut named_export_index = lint_program
        .exports
        .iter()
        .enumerate()
        .filter_map(|(index, export)| export.exported_name().map(|_| index))
        .collect::<Vec<_>>();
    named_export_index.sort_by(|left, right| {
        lint_program.exports[*left]
            .exported_name()
            .cmp(&lint_program.exports[*right].exported_name())
            .then_with(|| left.cmp(right))
    });
    let wildcard_export_index = lint_program
        .exports
        .iter()
        .enumerate()
        .filter_map(|(index, export)| {
            matches!(export, crate::ast::ExportDecl::All { .. }).then_some(index)
        })
        .collect::<Vec<_>>();
    let mut diagnostic_range_index = diagnostics
        .iter()
        .enumerate()
        .flat_map(|(diagnostic_index, diagnostic)| {
            std::iter::once((
                diagnostic.range.start,
                diagnostic.range.end,
                diagnostic_index,
            ))
            .chain(
                diagnostic
                    .related
                    .iter()
                    .map(move |related| (related.range.start, related.range.end, diagnostic_index)),
            )
        })
        .collect::<Vec<_>>();
    diagnostic_range_index.sort_by_key(|(start, _, diagnostic_index)| (*start, *diagnostic_index));
    let mut diagnostic_range_prefix_max_end = Vec::with_capacity(diagnostic_range_index.len());
    let mut maximum_diagnostic_end = 0;
    for (_, range_end, _) in &diagnostic_range_index {
        maximum_diagnostic_end = maximum_diagnostic_end.max(*range_end);
        diagnostic_range_prefix_max_end.push(maximum_diagnostic_end);
    }
    let mut reference_range_index = (0..references.len()).collect::<Vec<_>>();
    reference_range_index.sort_by_key(|index| references[*index].range.start);
    let mut references_by_definition = HashMap::default();
    for reference in &references {
        if let Some(definition) = &reference.definition {
            references_by_definition
                .entry((definition.start, definition.end))
                .or_insert_with(Vec::new)
                .push(reference.range.clone());
        }
    }
    let mut expression_type_index = (0..expression_types.len()).collect::<Vec<_>>();
    expression_type_index.sort_by_key(|index| expression_types[*index].range.start);
    let mut expression_type_prefix_max_end = Vec::with_capacity(expression_type_index.len());
    let mut maximum_expression_end = 0;
    for index in &expression_type_index {
        maximum_expression_end = maximum_expression_end.max(expression_types[*index].range.end);
        expression_type_prefix_max_end.push(maximum_expression_end);
    }

    DocumentAnalysis {
        symbols,
        symbol_name_index,
        import_symbol_index,
        import_symbol_count,
        named_export_index,
        wildcard_export_index,
        references,
        symbol_declaration_index,
        reference_range_index,
        references_by_definition,
        diagnostics,
        diagnostic_range_index,
        diagnostic_range_prefix_max_end,
        node_spans,
        expression_types,
        expression_type_index,
        expression_type_prefix_max_end,
        call_sites,
        call_site_prefix_max_end,
        comptime_facts,
        ctfe,
        tokens,
        program: Some(lint_program),
    }
}

fn object_key_matches(token: &Token, name: &str) -> bool {
    matches!(token, Token::Ident(field) if field == name)
        || crate::parser::keyword_as_name(token) == Some(name)
}

struct NodeSpanCollector<'a> {
    tokens: &'a [SpannedToken],
    cursor: usize,
    next_id: u32,
    nodes: Vec<NodeSpan>,
    calls: Vec<CallSite>,
    source_len: usize,
    pipe_rhs: bool,
    pending_piped: Option<Range<usize>>,
}

impl<'a> NodeSpanCollector<'a> {
    fn new(tokens: &'a [SpannedToken], source_len: usize) -> Self {
        Self {
            tokens,
            cursor: 0,
            next_id: 0,
            nodes: Vec::new(),
            calls: Vec::new(),
            source_len,
            pipe_rhs: false,
            pending_piped: None,
        }
    }

    fn collect(mut self, program: &Program) -> (Vec<NodeSpan>, Vec<CallSite>) {
        for import in &program.imports {
            let start = self.current_start();
            self.seek(|token| matches!(token, Token::Import));
            for specifier in &import.specifiers {
                let specifier_start = self.current_start();
                let writes_imported_ident = specifier.kind == crate::ast::ImportKind::Named
                    && specifier.imported != crate::ast::DEFAULT_EXPORT;
                if writes_imported_ident {
                    self.seek(
                        |token| matches!(token, Token::Ident(name) if name == &specifier.imported),
                    );
                }
                if !writes_imported_ident || specifier.imported != specifier.local {
                    self.seek(
                        |token| matches!(token, Token::Ident(name) if name == &specifier.local),
                    );
                }
                self.record(
                    AstNodeKind::ImportSpecifier,
                    specifier_start..self.last_end(),
                );
            }
            self.seek(
                |token| matches!(token, Token::StringLit(source) if source == &import.source),
            );
            self.record(AstNodeKind::Import, start..self.last_end());
        }
        for item in &program.items {
            self.item(item);
        }
        self.record(AstNodeKind::Program, 0..self.source_len);
        (self.nodes, self.calls)
    }

    fn item(&mut self, item: &Item) -> Range<usize> {
        let start = self.current_start();
        match item {
            Item::InterfaceDecl {
                name,
                extends,
                fields,
                from_type_alias,
                ..
            } => {
                if *from_type_alias {
                    self.seek(|token| matches!(token, Token::Type));
                } else {
                    self.seek(|token| matches!(token, Token::Interface));
                }
                self.seek(|token| matches!(token, Token::Ident(value) if value == name));
                // An intersection alias (`type A = B & C;`) carries its operands in
                // `extends` and declares no field block, so seeking a closing brace
                // that is not there would swallow the rest of the file and misplace
                // every range after it.
                if *from_type_alias && !extends.is_empty() {
                    self.seek(|token| matches!(token, Token::Semicolon));
                } else {
                    for field in fields {
                        self.seek(
                            |token| matches!(token, Token::Ident(value) if value == &field.name),
                        );
                    }
                    self.seek(|token| matches!(token, Token::RBrace));
                }
            }
            Item::BehaviorInterfaceDecl { name, .. } => {
                self.seek(|token| matches!(token, Token::Interface));
                self.seek(|token| matches!(token, Token::Ident(value) if value == name));
                self.seek(|token| matches!(token, Token::RBrace));
            }
            Item::FnDecl {
                name, params, body, ..
            } => {
                self.seek(|token| matches!(token, Token::Ident(value) if value == name));
                for param in params {
                    self.param(param);
                }
                for statement in body {
                    self.statement(statement);
                }
            }
            Item::TypeAlias { .. } => {}
            Item::EnumDecl { name, members, .. } => {
                self.seek(|token| matches!(token, Token::Enum));
                self.seek(|token| matches!(token, Token::Ident(value) if value == name));
                for member in members {
                    self.seek(
                        |token| matches!(token, Token::Ident(value) if value == &member.name),
                    );
                }
                self.seek(|token| matches!(token, Token::RBrace));
            }
            Item::Stmt(statement) => {
                self.statement(statement);
            }
        }
        let range = start..self.last_end();
        self.record(AstNodeKind::Item, range.clone());
        range
    }

    fn param(&mut self, param: &Param) -> Range<usize> {
        let start = self.current_start();
        match &param.pattern {
            ParamPattern::Name(name) => {
                self.seek(|token| matches!(token, Token::Ident(n) if n == name));
            }
            ParamPattern::Array(names) => {
                self.seek(|token| matches!(token, Token::LBracket));
                for name in names {
                    self.seek(|token| matches!(token, Token::Ident(n) if n == name));
                }
                self.seek(|token| matches!(token, Token::RBracket));
            }
            ParamPattern::Object(names) => {
                self.seek(|token| matches!(token, Token::LBrace));
                for name in names {
                    self.seek(|token| matches!(token, Token::Ident(n) if n == name));
                }
                self.seek(|token| matches!(token, Token::RBrace));
            }
        }
        let range = start..self.last_end();
        self.record(AstNodeKind::Parameter, range.clone());
        range
    }

    fn statement(&mut self, statement: &Stmt) -> Range<usize> {
        let start = self.current_start();
        match statement.kind() {
            StmtKind::VarDecl { name, value, .. } => {
                self.seek(|token| matches!(token, Token::Ident(value) if value == name));
                if let Some(value) = value {
                    self.seek(|token| matches!(token, Token::Eq));
                    self.expression(value);
                }
            }
            StmtKind::Assign { name, value, .. } => {
                self.seek(|token| matches!(token, Token::Ident(value) if value == name));
                self.expression(value);
            }
            StmtKind::IndexAssign { name, index, value } => {
                self.seek(|token| matches!(token, Token::Ident(value) if value == name));
                self.expression(index);
                self.expression(value);
            }
            StmtKind::FieldAssign { object, value, .. } => {
                self.expression(object);
                self.expression(value);
            }
            StmtKind::Return(Some(expr)) => {
                let explicit_return = self.tokens[self.cursor..]
                    .iter()
                    .take_while(|token| !matches!(token.token, Token::Semicolon | Token::RBrace))
                    .any(|token| matches!(token.token, Token::Return));
                if explicit_return {
                    self.seek(|token| matches!(token, Token::Return));
                }
                self.expression(expr);
            }
            StmtKind::Return(None) => {
                self.seek(|token| matches!(token, Token::Return));
            }

            StmtKind::Break(value) => {
                self.seek(|token| matches!(token, Token::Break));
                if let Some(value) = value {
                    self.expression(value);
                }
            }

            StmtKind::Continue => {
                self.seek(|token| matches!(token, Token::Continue));
            }

            StmtKind::Defer { kind, body, .. } => {
                self.seek(|token| {
                    matches!(
                        (kind, token),
                        (DeferKind::Always, Token::Defer) | (DeferKind::OnError, Token::ErrDefer)
                    )
                });
                for statement in body {
                    self.statement(statement);
                }
            }

            StmtKind::ArrayDestructure {
                bindings, value, ..
            } => {
                self.seek(|token| matches!(token, Token::LBracket));
                for binding in bindings {
                    if binding.discarded {
                        self.seek(|token| matches!(token, Token::Ident(n) if n == "_"));
                    } else {
                        self.seek(|token| matches!(token, Token::Ident(n) if n == &binding.name));
                    }
                    self.record(AstNodeKind::Binding, self.last_start()..self.last_end());
                }
                self.expression(value);
            }
            StmtKind::ObjectDestructure {
                bindings, value, ..
            } => {
                self.seek(|token| matches!(token, Token::LBrace));
                for binding in bindings {
                    self.seek(|token| matches!(token, Token::Ident(n) if n == &binding.name));
                    self.record(AstNodeKind::Binding, self.last_start()..self.last_end());
                }
                self.expression(value);
            }
            StmtKind::While { cond, body } => {
                self.seek(|token| matches!(token, Token::While));
                self.expression(cond);
                for statement in body {
                    self.statement(statement);
                }
            }
            StmtKind::DoWhile { body, cond } => {
                self.seek(|token| matches!(token, Token::Do));
                for statement in body {
                    self.statement(statement);
                }
                self.seek(|token| matches!(token, Token::While));
                self.expression(cond);
            }
            StmtKind::For {
                init,
                cond,
                update,
                body,
            } => {
                self.seek(|token| matches!(token, Token::For));
                self.statement(init);
                self.expression(cond);
                self.statement(update);
                for statement in body {
                    self.statement(statement);
                }
            }
            StmtKind::ForOf {
                name,
                iterable,
                body,
                ..
            } => {
                self.seek(|token| matches!(token, Token::For));
                self.seek(|token| matches!(token, Token::Ident(value) if value == name));
                self.record(AstNodeKind::Parameter, self.last_start()..self.last_end());
                self.expression(iterable);
                for statement in body {
                    self.statement(statement);
                }
            }
            StmtKind::ForIn { name, object, body } => {
                self.seek(|token| matches!(token, Token::For));
                self.seek(|token| matches!(token, Token::Ident(value) if value == name));
                self.record(AstNodeKind::Parameter, self.last_start()..self.last_end());
                self.expression(object);
                for statement in body {
                    self.statement(statement);
                }
            }
            StmtKind::Expr(expr) => {
                self.expression(expr);
            }
            StmtKind::Switch {
                expr,
                cases,
                default,
                ..
            } => {
                self.expression(expr);
                for case in cases {
                    self.expression(&case.value);
                    for stmt in &case.body {
                        self.statement(stmt);
                    }
                }
                if let Some(default_body) = default {
                    for stmt in default_body {
                        self.statement(stmt);
                    }
                }
            }
            StmtKind::Match {
                expr,
                arms,
                default,
            } => {
                self.expression(expr);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.expression(guard);
                    }
                    for stmt in &arm.body {
                        self.statement(stmt);
                    }
                }
                if let Some(default_body) = default {
                    for stmt in default_body {
                        self.statement(stmt);
                    }
                }
            }
            StmtKind::Throw(expr) => {
                self.expression(expr);
            }
            StmtKind::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                for stmt in try_body {
                    self.statement(stmt);
                }
                for stmt in catch_body {
                    self.statement(stmt);
                }
                if let Some(stmts) = finally_body {
                    for stmt in stmts {
                        self.statement(stmt);
                    }
                }
            }
        }
        let range = start..self.last_end();
        self.record(AstNodeKind::Statement, range.clone());
        range
    }

    #[allow(clippy::too_many_lines)]
    fn expression(&mut self, expr: &Expr) -> Range<usize> {
        let mut start = self.current_start();
        match expr.kind() {
            ExprKind::StrLit(value) => {
                self.seek(|token| matches!(token, Token::StringLit(text) if text == value));
                start = self.last_start();
            }
            ExprKind::RegexLit { pattern, flags } => {
                self.seek(|token| {
                    matches!(token, Token::RegexLit(literal) if literal.pattern == *pattern && literal.flags == *flags)
                });
                start = self.last_start();
            }
            ExprKind::DecimalLit(value) => {
                self.seek(|token| matches!(token, Token::DecimalLit(text) if text == value));
                start = self.last_start();
            }
            ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => {
                self.seek(
                    |token| matches!(token, Token::Ident(name) if name == "Set" || name == "Map"),
                );
                start = self.last_start();
                if let Some(initial) = initial {
                    self.expression(initial);
                }
            }
            ExprKind::TemplateLit(parts) => {
                self.seek(|token| matches!(token, Token::TemplateLit(_)));
                start = self.last_start();
                let template_range = start..self.last_end();
                for part in parts {
                    if let TemplatePart::Expr(expr, _) = part {
                        self.synthetic_expression(expr, &template_range);
                    }
                }
            }
            ExprKind::IntLit(value) => {
                self.seek(|token| {
                    matches!(token, Token::IntLit(number) if number == value)
                        || matches!(token, Token::IntOverflowLit(raw) if *value == i128::MIN && raw.parse::<u128>().is_ok_and(|magnitude| magnitude == i128::MAX as u128 + 1))
                });
                start = self.last_start();
            }
            ExprKind::UIntLit(value) => {
                self.seek(|token| {
                    matches!(token, Token::IntOverflowLit(raw) if raw.parse::<u128>().is_ok_and(|number| number == *value))
                });
                start = self.last_start();
            }
            ExprKind::FloatLit(value) => {
                self.seek(|token| {
                    matches!(token, Token::FloatLit(number) if number == value)
                        || matches!(token, Token::SpecialFloatLit(number) if (number.value() == value.value()) || (number.value().is_nan() && value.value().is_nan()))
                });
                start = self.last_start();
            }
            ExprKind::BoolLit(value) => {
                self.seek(|token| {
                    matches!((token, value), (Token::True, true) | (Token::False, false))
                });
                start = self.last_start();
            }
            ExprKind::Null | ExprKind::None => {
                self.seek(|token| matches!(token, Token::Null));
                start = self.last_start();
            }
            ExprKind::ObjectLit(fields) => {
                self.seek(|token| matches!(token, Token::LBrace));
                start = self.last_start();
                for (name, value) in fields {
                    let key_start = self.current_start();
                    self.seek(|token| object_key_matches(token, name));
                    let value_range = if matches!(value.kind(), ExprKind::Ident(ident) if ident == name)
                    {
                        let range = key_start..self.last_end();
                        self.record(AstNodeKind::Expression, range.clone());
                        range
                    } else {
                        self.expression(value)
                    };
                    if let Some(node) = self.nodes.iter_mut().rev().find(|node| {
                        node.kind == AstNodeKind::Expression && node.range == value_range
                    }) {
                        node.range.start = key_start;
                    }
                }
                self.seek(|token| matches!(token, Token::RBrace));
            }
            ExprKind::ObjectUpdate { base, fields } => {
                self.seek(|token| matches!(token, Token::LBrace));
                start = self.last_start();
                self.expression(base);
                for (name, value) in fields {
                    let key_start = self.current_start();
                    self.seek(|token| object_key_matches(token, name));
                    if !matches!(value.kind(), ExprKind::Ident(ident) if ident == name) {
                        self.expression(value);
                    } else {
                        let range = key_start..self.last_end();
                        self.record(AstNodeKind::Expression, range);
                    }
                }
                self.seek(|token| matches!(token, Token::RBrace));
            }
            ExprKind::ArrayLit(values) => {
                self.seek(|token| matches!(token, Token::LBracket));
                start = self.last_start();
                for value in values {
                    self.expression(value);
                }
                self.seek(|token| matches!(token, Token::RBracket));
            }
            ExprKind::ArrayRepeat { value, count } => {
                self.seek(|token| matches!(token, Token::LBracket));
                start = self.last_start();
                self.expression(value);
                self.seek(|token| matches!(token, Token::Semicolon));
                self.expression(count);
                self.seek(|token| matches!(token, Token::RBracket));
            }
            ExprKind::Ident(name) => {
                self.seek(|token| matches!(token, Token::Ident(value) if value == name));
                start = self.last_start();
            }
            ExprKind::Topic => {
                self.seek(|token| matches!(token, Token::Percent));
                start = self.last_start();
            }
            ExprKind::FieldAccess { object, field }
            | ExprKind::OptionalFieldAccess { object, field } => {
                start = self.expression(object).start;
                self.seek(|token| matches!(token, Token::Ident(name) if name == field));
            }
            ExprKind::IndexAccess { object, index } => {
                start = self.expression(object).start;
                self.expression(index);
                self.seek(|token| matches!(token, Token::RBracket));
            }
            ExprKind::Borrow { name, .. } => {
                self.seek(|token| matches!(token, Token::Ident(value) if value == name));
            }
            ExprKind::BinOp { left, right, .. } => {
                start = self.expression(left).start;
                self.expression(right);
            }
            ExprKind::ChainedComparison { terms, .. } => {
                start = self.expression(&terms[0]).start;
                for term in &terms[1..] {
                    self.expression(term);
                }
            }
            ExprKind::Pipeline { left, right } => {
                let piped = self.expression(left);
                start = piped.start;
                let previous_pipe_rhs = self.pipe_rhs;
                let previous_piped = self.pending_piped.take();
                let has_topic = contains_outer_topic(right);
                self.pipe_rhs = !has_topic;
                self.pending_piped = (!has_topic).then_some(piped);
                self.expression(right);
                self.pipe_rhs = previous_pipe_rhs;
                self.pending_piped = previous_piped;
            }
            ExprKind::Not(inner)
            | ExprKind::Neg(inner)
            | ExprKind::BitNot(inner)
            | ExprKind::Typeof(inner)
            | ExprKind::Await(inner)
            | ExprKind::Try(inner)
            | ExprKind::Unwrap(inner)
            | ExprKind::Spread(inner) => {
                self.expression(inner);
            }
            ExprKind::TypeAssert { value, .. } => {
                self.expression(value);
            }
            ExprKind::Is { expr, .. } => {
                self.expression(expr);
            }
            ExprKind::ConstAssert(inner) | ExprKind::Comptime(inner) => {
                self.expression(inner);
            }
            ExprKind::PrefixIncr(inner)
            | ExprKind::PrefixDecr(inner)
            | ExprKind::PostfixIncr(inner)
            | ExprKind::PostfixDecr(inner) => {
                self.expression(inner);
            }
            ExprKind::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                self.expression(cond);
                self.expression(then_expr);
                self.expression(else_expr);
            }
            ExprKind::FnCall { callee, args, .. } => {
                let arg_offset = usize::from(std::mem::take(&mut self.pipe_rhs));
                let piped_value = self.pending_piped.take();
                self.seek(|token| matches!(token, Token::Ident(name) if name == callee));
                start = self.last_start();
                let mut arguments = Vec::with_capacity(args.len());
                for arg in args {
                    arguments.push(self.expression(arg));
                }
                self.seek(|token| matches!(token, Token::RParen));
                self.calls.push(CallSite {
                    callee: callee.clone(),
                    arguments,
                    arg_offset,
                    piped_value,
                    range: start..self.last_end(),
                    receiver: None,
                });
            }
            ExprKind::CallValue { callee, args, .. } => {
                self.expression(callee);
                for arg in args {
                    self.expression(arg);
                }
                self.seek(|token| matches!(token, Token::RParen));
            }
            ExprKind::MethodCall {
                object,
                method,
                args,
                ..
            }
            | ExprKind::OptionalMethodCall {
                object,
                method,
                args,
                ..
            } => {
                start = self.expression(object).start;
                self.seek(|token| matches!(token, Token::Ident(name) if name == method));
                let callee = if matches!(object.as_ref().kind(), ExprKind::Ident(name) if name == "Promise")
                {
                    format!("Promise.{method}")
                } else {
                    method.clone()
                };
                let mut arguments = Vec::with_capacity(args.len());
                for arg in args {
                    arguments.push(self.expression(arg));
                }
                self.seek(|token| matches!(token, Token::RParen));
                self.calls.push(CallSite {
                    callee,
                    arguments,
                    arg_offset: 0,
                    piped_value: None,
                    range: start..self.last_end(),
                    receiver: match object.as_ref().kind() {
                        ExprKind::Ident(name) => Some(name.clone()),
                        _ => None,
                    },
                });
            }
            ExprKind::ArrowFn { params, body, .. } => {
                for param in params {
                    self.param(param);
                }
                for statement in body {
                    self.statement(statement);
                }
            }
            ExprKind::If {
                cond,
                then_block,
                else_block,
            } => {
                self.seek(|token| matches!(token, Token::If));
                self.expression(cond);
                for statement in then_block {
                    self.statement(statement);
                }
                if let Some(block) = else_block {
                    for statement in block {
                        self.statement(statement);
                    }
                }
            }
            ExprKind::Loop { body } => {
                self.seek(|token| matches!(token, Token::Loop));
                for statement in body {
                    self.statement(statement);
                }
            }
            ExprKind::In { key, object } => {
                self.expression(key);
                self.seek(|token| matches!(token, Token::In));
                self.expression(object);
            }
            ExprKind::Range {
                start: lo, end: hi, ..
            } => {
                start = self.expression(lo).start;
                self.seek(|token| matches!(token, Token::DotDot | Token::DotDotEq));
                self.expression(hi);
            }
        }
        let range = start..self.last_end();
        self.record(AstNodeKind::Expression, range.clone());
        range
    }

    fn seek(&mut self, predicate: impl Fn(&Token) -> bool) {
        while let Some(token) = self.tokens.get(self.cursor) {
            self.cursor += 1;
            if predicate(&token.token) {
                break;
            }
        }
    }

    fn synthetic_expression(&mut self, expr: &Expr, range: &Range<usize>) {
        match expr.kind() {
            ExprKind::ObjectLit(fields) => {
                for (_, value) in fields {
                    self.synthetic_expression(value, range);
                }
            }
            ExprKind::ObjectUpdate { base, fields } => {
                self.synthetic_expression(base, range);
                for (_, value) in fields {
                    self.synthetic_expression(value, range);
                }
            }
            ExprKind::ArrayLit(values) => {
                for value in values {
                    self.synthetic_expression(value, range);
                }
            }
            ExprKind::ArrayRepeat { value, count } => {
                self.synthetic_expression(value, range);
                self.synthetic_expression(count, range);
            }
            ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
                self.synthetic_expression(object, range);
            }
            ExprKind::IndexAccess { object, index } => {
                self.synthetic_expression(object, range);
                self.synthetic_expression(index, range);
            }
            ExprKind::BinOp { left, right, .. } => {
                self.synthetic_expression(left, range);
                self.synthetic_expression(right, range);
            }
            ExprKind::Pipeline { left, right } => {
                self.synthetic_expression(left, range);
                self.synthetic_expression(right, range);
            }
            ExprKind::ChainedComparison { terms, .. } => {
                for term in terms {
                    self.synthetic_expression(term, range);
                }
            }
            ExprKind::Not(inner)
            | ExprKind::Neg(inner)
            | ExprKind::BitNot(inner)
            | ExprKind::Typeof(inner)
            | ExprKind::Await(inner)
            | ExprKind::Try(inner)
            | ExprKind::Unwrap(inner)
            | ExprKind::Spread(inner) => self.synthetic_expression(inner, range),
            ExprKind::TypeAssert { value, .. } => self.synthetic_expression(value, range),
            ExprKind::Is { expr, .. } => self.synthetic_expression(expr, range),
            ExprKind::ConstAssert(inner) | ExprKind::Comptime(inner) => {
                self.synthetic_expression(inner, range);
            }
            ExprKind::PrefixIncr(inner)
            | ExprKind::PrefixDecr(inner)
            | ExprKind::PostfixIncr(inner)
            | ExprKind::PostfixDecr(inner) => self.synthetic_expression(inner, range),
            ExprKind::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                self.synthetic_expression(cond, range);
                self.synthetic_expression(then_expr, range);
                self.synthetic_expression(else_expr, range);
            }
            ExprKind::TemplateLit(parts) => {
                for part in parts {
                    if let TemplatePart::Expr(expr, _) = part {
                        self.synthetic_expression(expr, range);
                    }
                }
            }
            ExprKind::MethodCall { object, args, .. }
            | ExprKind::OptionalMethodCall { object, args, .. } => {
                self.synthetic_expression(object, range);
                for arg in args {
                    self.synthetic_expression(arg, range);
                }
            }
            ExprKind::FnCall { args, .. } => {
                for arg in args {
                    self.synthetic_expression(arg, range);
                }
            }
            ExprKind::CallValue { callee, args, .. } => {
                self.synthetic_expression(callee, range);
                for arg in args {
                    self.synthetic_expression(arg, range);
                }
            }
            ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => {
                if let Some(initial) = initial {
                    self.synthetic_expression(initial, range);
                }
            }
            ExprKind::If {
                cond,
                then_block,
                else_block,
            } => {
                self.synthetic_expression(cond, range);
                for statement in then_block {
                    self.synthetic_statement(statement, range);
                }
                if let Some(block) = else_block {
                    for statement in block {
                        self.synthetic_statement(statement, range);
                    }
                }
            }
            ExprKind::Loop { body } => {
                for statement in body {
                    self.synthetic_statement(statement, range);
                }
            }
            ExprKind::ArrowFn { body, .. } => {
                for statement in body {
                    self.synthetic_statement(statement, range);
                }
            }
            ExprKind::In { key, object } => {
                self.synthetic_expression(key, range);
                self.synthetic_expression(object, range);
            }
            ExprKind::Range { start, end, .. } => {
                self.synthetic_expression(start, range);
                self.synthetic_expression(end, range);
            }
            ExprKind::StrLit(_)
            | ExprKind::RegexLit { .. }
            | ExprKind::IntLit(_)
            | ExprKind::UIntLit(_)
            | ExprKind::FloatLit(_)
            | ExprKind::DecimalLit(_)
            | ExprKind::BoolLit(_)
            | ExprKind::Null
            | ExprKind::None
            | ExprKind::Ident(_)
            | ExprKind::Topic
            | ExprKind::Borrow { .. } => {}
        }
        self.record(AstNodeKind::Expression, range.clone());
    }

    fn synthetic_statement(&mut self, statement: &Stmt, range: &Range<usize>) {
        match statement.kind() {
            StmtKind::VarDecl { value, .. } => {
                if let Some(value) = value {
                    self.synthetic_expression(value, range);
                }
            }
            StmtKind::Assign { value, .. }
            | StmtKind::Return(Some(value))
            | StmtKind::Expr(value) => self.synthetic_expression(value, range),
            StmtKind::IndexAssign { index, value, .. } => {
                self.synthetic_expression(index, range);
                self.synthetic_expression(value, range);
            }
            StmtKind::FieldAssign { object, value, .. } => {
                self.synthetic_expression(object, range);
                self.synthetic_expression(value, range);
            }
            StmtKind::Return(None) => {}
            StmtKind::Break(value) => {
                if let Some(value) = value {
                    self.synthetic_expression(value, range);
                }
            }
            StmtKind::Continue => {}
            StmtKind::Defer { body, .. } => {
                for statement in body {
                    self.synthetic_statement(statement, range);
                }
            }
            StmtKind::ArrayDestructure {
                bindings, value, ..
            }
            | StmtKind::ObjectDestructure {
                bindings, value, ..
            } => {
                for binding in bindings {
                    self.record(
                        AstNodeKind::Parameter,
                        range.start..range.start + binding.name.len(),
                    );
                }
                self.synthetic_expression(value, range);
            }
            StmtKind::While { cond, body } => {
                self.synthetic_expression(cond, range);
                for statement in body {
                    self.synthetic_statement(statement, range);
                }
            }
            StmtKind::DoWhile { body, cond } => {
                for statement in body {
                    self.synthetic_statement(statement, range);
                }
                self.synthetic_expression(cond, range);
            }
            StmtKind::For {
                init,
                cond,
                update: _,
                body,
            } => {
                self.synthetic_statement(init, range);
                self.synthetic_expression(cond, range);
                for statement in body {
                    self.synthetic_statement(statement, range);
                }
            }
            StmtKind::ForOf {
                name,
                iterable,
                body,
                ..
            } => {
                self.record(
                    AstNodeKind::Parameter,
                    range.start..range.start + name.len(),
                );
                self.synthetic_expression(iterable, range);
                for statement in body {
                    self.synthetic_statement(statement, range);
                }
            }
            StmtKind::ForIn { name, object, body } => {
                self.record(
                    AstNodeKind::Parameter,
                    range.start..range.start + name.len(),
                );
                self.synthetic_expression(object, range);
                for statement in body {
                    self.synthetic_statement(statement, range);
                }
            }
            StmtKind::Switch {
                expr,
                cases,
                default,
                ..
            } => {
                self.synthetic_expression(expr, range);
                for case in cases {
                    for stmt in &case.body {
                        self.synthetic_statement(stmt, range);
                    }
                }
                if let Some(default_body) = default {
                    for stmt in default_body {
                        self.synthetic_statement(stmt, range);
                    }
                }
            }
            StmtKind::Match {
                expr,
                arms,
                default,
            } => {
                self.synthetic_expression(expr, range);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.synthetic_expression(guard, range);
                    }
                    for stmt in &arm.body {
                        self.synthetic_statement(stmt, range);
                    }
                }
                if let Some(default_body) = default {
                    for stmt in default_body {
                        self.synthetic_statement(stmt, range);
                    }
                }
            }
            StmtKind::Throw(expr) => {
                self.synthetic_expression(expr, range);
            }
            StmtKind::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                for stmt in try_body {
                    self.synthetic_statement(stmt, range);
                }
                for stmt in catch_body {
                    self.synthetic_statement(stmt, range);
                }
                if let Some(stmts) = finally_body {
                    for stmt in stmts {
                        self.synthetic_statement(stmt, range);
                    }
                }
            }
        }
        self.record(AstNodeKind::Statement, range.clone());
    }

    fn current_start(&self) -> usize {
        self.tokens
            .get(self.cursor)
            .map(|token| token.range.start)
            .unwrap_or(self.source_len)
    }

    fn last_end(&self) -> usize {
        self.cursor
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
            .map(|token| token.range.end)
            .unwrap_or(0)
    }

    fn last_start(&self) -> usize {
        self.cursor
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
            .map(|token| token.range.start)
            .unwrap_or(0)
    }

    fn record(&mut self, kind: AstNodeKind, range: Range<usize>) {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.push(NodeSpan { id, kind, range });
    }
}

fn checker_related_information(
    message: &str,
    references: &[Reference],
) -> Vec<AnalysisRelatedInformation> {
    let moved = message.contains("use of moved value");
    let dropped = message.contains("use of dropped value");
    if !moved && !dropped {
        return Vec::new();
    }
    let Some(name) = message.split('`').nth(1) else {
        return Vec::new();
    };
    let matching = references
        .iter()
        .filter(|reference| reference.name == name)
        .collect::<Vec<_>>();
    let Some(primary) = matching.last() else {
        return Vec::new();
    };
    matching
        .iter()
        .rev()
        .skip(1)
        .find(|reference| reference.range.start < primary.range.start)
        .map(|reference| {
            vec![AnalysisRelatedInformation {
                range: reference.range.clone(),
                message: if moved {
                    format!("value moved here; borrow `{name}` instead if it must remain usable")
                } else {
                    format!("value dropped here; `{name}` cannot be used afterward")
                },
            }]
        })
        .unwrap_or_default()
}

pub fn numeric_mismatch_types(message: &str) -> Option<(String, String)> {
    const NUMERIC: [&str; 16] = [
        "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
        "f32", "f64", "f128", "decimal",
    ];
    let (_, tail) = message.split_once("expected `")?;
    let (expected, tail) = tail.split_once('`')?;
    let (_, tail) = tail.split_once("found `")?;
    let (found, _) = tail.split_once('`')?;
    (NUMERIC.contains(&expected) && NUMERIC.contains(&found) && expected != found)
        .then(|| (expected.to_string(), found.to_string()))
}

fn display_function_params(symbol: &Symbol, params: &[Type]) -> String {
    params
        .iter()
        .enumerate()
        .map(|(index, ty)| {
            let name = symbol
                .parameters
                .get(index)
                .and_then(|param| param.pattern.binding_names().first().copied())
                .unwrap_or("param");
            format!("{name}: {ty}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn locate_async_return_annotation(
    tokens: &[SpannedToken],
    used: &mut Vec<Range<usize>>,
) -> Option<Range<usize>> {
    for (index, token) in tokens.iter().enumerate() {
        if token.token != Token::Async {
            continue;
        }
        let mut cursor = index + 1;
        if matches!(
            tokens.get(cursor).map(|token| &token.token),
            Some(Token::Function)
        ) {
            cursor += 1;
            if matches!(
                tokens.get(cursor).map(|token| &token.token),
                Some(Token::Ident(_))
            ) {
                cursor += 1;
            }
        }
        let Some(params_start) = tokens[cursor..]
            .iter()
            .position(|token| token.token == Token::LParen)
            .map(|offset| cursor + offset)
        else {
            continue;
        };
        let Some(params_end) = matching_token(tokens, params_start, Token::LParen, Token::RParen)
        else {
            continue;
        };
        let Some(colon_index) = tokens[params_end + 1..]
            .iter()
            .position(|token| token.token == Token::Colon)
            .map(|offset| params_end + 1 + offset)
        else {
            continue;
        };
        let Some(end_index) = tokens[colon_index + 1..]
            .iter()
            .position(|token| matches!(token.token, Token::LBrace | Token::Arrow))
            .map(|offset| colon_index + 1 + offset)
        else {
            continue;
        };
        if end_index <= colon_index + 1 {
            continue;
        }
        let range = tokens[colon_index + 1].range.start..tokens[end_index - 1].range.end;
        if used.iter().any(|claimed| claimed == &range) {
            continue;
        }
        used.push(range.clone());
        return Some(range);
    }
    None
}

fn locate_async_return_annotation_before(
    tokens: &[SpannedToken],
    before: usize,
) -> Option<Range<usize>> {
    let mut used = Vec::new();
    let mut closest = None;
    while let Some(range) = locate_async_return_annotation(tokens, &mut used) {
        if range.end <= before {
            closest = Some(range);
        }
    }
    closest
}

fn locate_return_annotation_before(tokens: &[SpannedToken], before: usize) -> Option<Range<usize>> {
    let mut closest = None;
    for (colon, token) in tokens.iter().enumerate() {
        if token.token != Token::Colon {
            continue;
        }
        let Some(end) = tokens[colon + 1..]
            .iter()
            .position(|candidate| matches!(candidate.token, Token::LBrace | Token::Arrow))
            .map(|offset| colon + 1 + offset)
        else {
            continue;
        };
        if end <= colon + 1 || tokens[end].range.start > before {
            continue;
        }
        if tokens[colon + 1..end]
            .iter()
            .any(|candidate| matches!(candidate.token, Token::Eq | Token::Semicolon | Token::Comma))
        {
            continue;
        }
        closest = Some(tokens[colon + 1].range.start..tokens[end - 1].range.end);
    }
    closest
}

fn locate_finalizer_keyword_before(tokens: &[SpannedToken], before: usize) -> Option<Range<usize>> {
    tokens
        .iter()
        .rev()
        .find(|token| {
            token.range.end <= before && matches!(token.token, Token::Defer | Token::ErrDefer)
        })
        .map(|token| token.range.clone())
}

fn locate_async_fallthrough(
    tokens: &[SpannedToken],
    annotation: &Range<usize>,
) -> Option<Range<usize>> {
    let body_start = tokens
        .iter()
        .position(|token| token.range.start >= annotation.end && token.token == Token::LBrace)?;
    let body_end = matching_token(tokens, body_start, Token::LBrace, Token::RBrace)?;
    Some(tokens[body_end].range.clone())
}

fn locate_typeof_compared_with(
    tokens: &[SpannedToken],
    literal: &Range<usize>,
) -> Option<Range<usize>> {
    let index = tokens.iter().position(|token| token.range == *literal)?;
    let before = tokens[index.saturating_sub(8)..index]
        .iter()
        .rev()
        .find(|token| token.token == Token::Typeof);
    let after = tokens
        .get(index + 2)
        .filter(|token| token.token == Token::Typeof);
    after.or(before).map(|token| token.range.clone())
}

fn locate_token(
    needle: Token,
    tokens: &[SpannedToken],
    used: &mut Vec<Range<usize>>,
) -> Option<Range<usize>> {
    for token in tokens.iter().filter(|token| token.token == needle) {
        if used.iter().any(|claimed| claimed == &token.range) {
            continue;
        }
        used.push(token.range.clone());
        return Some(token.range.clone());
    }
    None
}

fn locate_loop_guard_before_await(
    tokens: &[SpannedToken],
    await_start: usize,
) -> Option<Range<usize>> {
    let and_index = tokens
        .iter()
        .rposition(|token| token.token == Token::And && token.range.end <= await_start)?;
    let mut depth = 0usize;
    for index in (0..and_index).rev() {
        match tokens[index].token {
            Token::RParen => depth += 1,
            Token::LParen => {
                if depth == 0 {
                    let start = tokens.get(index + 1)?.range.start;
                    let end = tokens.get(and_index.checked_sub(1)?)?.range.end;
                    return (start < end).then_some(start..end);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    None
}

fn locate_field_access(tokens: &[SpannedToken], owner: &str, field: &str) -> Option<Range<usize>> {
    let mut wanted: Vec<&str> = owner.split('.').collect();
    wanted.push(field);
    if wanted.iter().any(|segment| segment.is_empty()) {
        return None;
    }
    let span = wanted.len() * 2 - 1;
    let mut first = None;
    for (start, window) in tokens.windows(span).enumerate() {
        let mut matched = true;
        for (index, expected) in wanted.iter().enumerate() {
            let Token::Ident(found) = &window[index * 2].token else {
                matched = false;
                break;
            };
            if found != expected
                || (index + 1 < wanted.len() && window[index * 2 + 1].token != Token::Dot)
            {
                matched = false;
                break;
            }
        }
        if !matched {
            continue;
        }
        let range = window[0].range.start..window[span - 1].range.end;
        if start > 0 && tokens[start - 1].token == Token::Eq {
            return Some(range);
        }
        first.get_or_insert(range);
    }
    first
}

fn locate_last_field_access(
    tokens: &[SpannedToken],
    owner: &str,
    field: &str,
) -> Option<Range<usize>> {
    let mut wanted: Vec<&str> = owner.split('.').collect();
    wanted.push(field);
    if wanted.iter().any(|segment| segment.is_empty()) {
        return None;
    }
    let span = wanted.len() * 2 - 1;
    tokens
        .windows(span)
        .rfind(|window| {
            wanted.iter().enumerate().all(|(index, expected)| {
                matches!(&window[index * 2].token, Token::Ident(found) if found == expected)
                    && (index + 1 == wanted.len() || window[index * 2 + 1].token == Token::Dot)
            })
        })
        .map(|window| window[0].range.start..window[span - 1].range.end)
}

fn locate_token_after(
    needle: Token,
    tokens: &[SpannedToken],
    after: usize,
) -> Option<Range<usize>> {
    tokens
        .iter()
        .find(|token| token.token == needle && token.range.start >= after)
        .map(|token| token.range.clone())
}

fn matching_token(
    tokens: &[SpannedToken],
    start: usize,
    open: Token,
    close: Token,
) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        if token.token == open {
            depth += 1;
        } else if token.token == close {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn moved_value_fix_example(
    moved_name: &str,
    related: &[AnalysisRelatedInformation],
    symbols: &[Symbol],
) -> Option<String> {
    let move_site = related.first()?.range.start;
    let target = symbols
        .iter()
        .filter(|symbol| symbol.kind == SymbolKind::Variable)
        .filter(|symbol| symbol.declaration.start < move_site)
        .filter(|symbol| symbol.scope.contains(&move_site))
        .max_by_key(|symbol| symbol.declaration.start)?;
    let borrowed_ty = match target.ty.strip_prefix("mut ") {
        Some(ty) => format!("mut &{ty}"),
        None => format!("&{}", target.ty),
    };
    Some(format!(
        "const {}: {} = &{};",
        target.name, borrowed_ty, moved_name
    ))
}

pub(crate) struct EnrichedDiagnostic {
    range: Range<usize>,
    message: String,
    related: Vec<AnalysisRelatedInformation>,
    help: Vec<String>,
}

fn enrich_duplicate_declaration(
    subject: &str,
    duplicate_range: &Range<usize>,
    symbols: &[Symbol],
) -> Option<EnrichedDiagnostic> {
    let original = symbols
        .iter()
        .find(|s| s.name == subject && s.declaration != *duplicate_range)?;
    Some(EnrichedDiagnostic {
        range: duplicate_range.clone(),
        message: format!("identifier `{subject}` is declared more than once in this scope"),
        related: vec![AnalysisRelatedInformation {
            range: original.declaration.clone(),
            message: format!("previous declaration of `{subject}` is here"),
        }],
        help: Vec::new(),
    })
}

struct TypeMismatch<'a> {
    subject: &'a str,
    expected: &'a str,
    found: &'a str,
}

fn parse_type_mismatch(message: &str) -> Option<TypeMismatch<'_>> {
    if !message.starts_with("type mismatch for `") {
        return None;
    }
    let mut quoted = message.split('`').skip(1).step_by(2);
    let subject = quoted.next()?;
    let expected = quoted.next()?;
    let found = quoted.next()?;
    Some(TypeMismatch {
        subject,
        expected,
        found,
    })
}

fn enrich_callback_parameter_type_mismatch(
    message: &str,
    subject: Option<&str>,
    tokens: &[SpannedToken],
) -> Option<EnrichedDiagnostic> {
    let target = parse_callback_parameter_subject(subject?)?;
    let mismatch = parse_type_mismatch(message)?;
    if mismatch.subject != subject? {
        return None;
    }
    let range = locate_callback_parameter_annotation(tokens, &target)?;
    Some(EnrichedDiagnostic {
        range,
        message: format!(
            "type mismatch for callback parameter `{}` of `{}`: expected `{}`, found `{}`",
            target.name, target.method, mismatch.expected, mismatch.found
        ),
        related: Vec::new(),
        help: vec![format!(
            "change the callback parameter type to `{}`",
            mismatch.expected
        )],
    })
}

struct CallbackParameterSubject<'a> {
    method: &'a str,
    index: usize,
    name: &'a str,
}

fn parse_callback_parameter_subject(subject: &str) -> Option<CallbackParameterSubject<'_>> {
    let mut parts = subject.strip_prefix("callback-param:")?.splitn(3, ':');
    let method = parts.next()?;
    let index = parts.next()?.parse().ok()?;
    let name = parts.next()?;
    Some(CallbackParameterSubject {
        method,
        index,
        name,
    })
}

fn locate_callback_parameter_annotation(
    tokens: &[SpannedToken],
    target: &CallbackParameterSubject<'_>,
) -> Option<Range<usize>> {
    let mut cursor = 0;
    while cursor + 2 < tokens.len() {
        if !matches!(&tokens[cursor].token, Token::Dot) {
            cursor += 1;
            continue;
        }
        let Token::Ident(method) = &tokens[cursor + 1].token else {
            cursor += 1;
            continue;
        };
        if method != target.method || !matches!(tokens[cursor + 2].token, Token::LParen) {
            cursor += 1;
            continue;
        }
        if let Some(range) =
            locate_callback_parameter_annotation_in_call(tokens, cursor + 2, target)
        {
            return Some(range);
        }
        cursor += 1;
    }
    None
}

fn locate_callback_parameter_annotation_in_call(
    tokens: &[SpannedToken],
    lparen: usize,
    target: &CallbackParameterSubject<'_>,
) -> Option<Range<usize>> {
    let callback_start = lparen + 1;
    let params_lparen = if matches!(tokens.get(callback_start)?.token, Token::LParen) {
        callback_start
    } else {
        return None;
    };
    let arrow = find_matching_arrow(tokens, params_lparen)?;
    let mut current_param = 0usize;
    let mut cursor = params_lparen + 1;
    while cursor < arrow {
        match &tokens[cursor].token {
            Token::Ident(name) if current_param == target.index && name == target.name => {
                if !matches!(tokens.get(cursor + 1)?.token, Token::Colon) {
                    return None;
                }
                let type_start = cursor + 2;
                let type_end = tokens
                    .iter()
                    .enumerate()
                    .take(arrow)
                    .skip(type_start)
                    .find(|(_, token)| matches!(token.token, Token::Comma | Token::RParen))
                    .map(|(index, _)| index)?;
                let first = tokens.get(type_start)?;
                let last = tokens.get(type_end.checked_sub(1)?)?;
                return Some(first.range.start..last.range.end);
            }
            Token::Comma => current_param += 1,
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn find_matching_arrow(tokens: &[SpannedToken], params_lparen: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(params_lparen) {
        match token.token {
            Token::LParen => depth += 1,
            Token::RParen => depth = depth.checked_sub(1)?,
            Token::Arrow if depth == 0 => return Some(index),
            _ => {}
        }
        if depth == 0 && index > params_lparen {
            return tokens
                .iter()
                .enumerate()
                .skip(index + 1)
                .find(|(_, token)| matches!(token.token, Token::Arrow))
                .map(|(arrow, _)| arrow);
        }
    }
    None
}

fn find_type_annotation_range(
    tokens: &[SpannedToken],
    declaration_start: usize,
    expected: &str,
) -> Option<Range<usize>> {
    let start = tokens
        .iter()
        .position(|token| token.range.start == declaration_start)?;
    let colon = tokens
        .iter()
        .enumerate()
        .skip(start)
        .find(|(_, token)| token.token == Token::Colon)?
        .0;
    let end = tokens
        .iter()
        .enumerate()
        .skip(colon + 1)
        .find(|(_, token)| matches!(token.token, Token::Eq | Token::Semicolon | Token::Comma))
        .map(|(index, _)| index)
        .unwrap_or(tokens.len());
    tokens[colon + 1..end]
        .iter()
        .find(|token| token_text(token, expected))
        .map(|token| token.range.clone())
        .or_else(|| {
            let first = tokens.get(colon + 1)?;
            let last = tokens.get(end.checked_sub(1)?)?;
            Some(first.range.start..last.range.end)
        })
}

fn find_borrowed_initializer_range(
    tokens: &[SpannedToken],
    references: &[Reference],
    declaration: &Range<usize>,
) -> Option<Range<usize>> {
    let declaration_token = tokens
        .iter()
        .position(|token| token.range.start == declaration.start)?;
    let equals = tokens
        .iter()
        .enumerate()
        .skip(declaration_token)
        .find(|(_, token)| token.token == Token::Eq)?
        .0;
    let end = tokens
        .iter()
        .enumerate()
        .skip(equals + 1)
        .find(|(_, token)| token.token == Token::Semicolon)
        .map(|(index, _)| index)
        .unwrap_or(tokens.len());
    let ampersand = tokens
        .iter()
        .enumerate()
        .take(end)
        .skip(equals + 1)
        .find(|(_, token)| token.token == Token::Ampersand)?
        .0;
    let reference = references.iter().find(|reference| {
        reference.range.start >= tokens[ampersand].range.end
            && reference.range.end <= tokens[end - 1].range.end
    })?;
    Some(tokens[ampersand].range.start..reference.range.end)
}

fn token_text(token: &SpannedToken, text: &str) -> bool {
    token.range.end.saturating_sub(token.range.start) == text.len()
}

fn locate_process_env(
    code: &str,
    subject: Option<&str>,
    tokens: &[SpannedToken],
    used: &mut Vec<Range<usize>>,
) -> Option<Range<usize>> {
    let is_ident = |index: usize, name: &str| {
        tokens
            .get(index)
            .is_some_and(|token| matches!(&token.token, Token::Ident(value) if value == name))
    };
    let is_dot = |index: usize| {
        tokens
            .get(index)
            .is_some_and(|token| token.token == Token::Dot)
    };
    for (index, token) in tokens.iter().enumerate() {
        if !matches!(&token.token, Token::Ident(name) if name == "process") {
            continue;
        }
        let candidate = match code {
            "E3141" => subject.and_then(|name| {
                (is_dot(index + 1)
                    && is_ident(index + 2, "env")
                    && is_dot(index + 3)
                    && is_ident(index + 4, name))
                .then(|| tokens[index + 4].range.clone())
            }),
            "E3140" | "E3144" => (is_dot(index + 1) && is_ident(index + 2, "env"))
                .then(|| token.range.start..tokens[index + 2].range.end),
            "E3145" => match subject {
                Some(field) => (is_dot(index + 1) && is_ident(index + 2, field))
                    .then(|| token.range.start..tokens[index + 2].range.end),
                None => (!is_dot(index + 1)).then(|| token.range.clone()),
            },
            _ => None,
        };
        if let Some(range) = candidate {
            if used.contains(&range) {
                continue;
            }
            used.push(range.clone());
            return Some(range);
        }
    }
    None
}

fn locate_index_access(
    subject: Option<&str>,
    message: &str,
    tokens: &[SpannedToken],
    used: &mut Vec<Range<usize>>,
) -> Option<Range<usize>> {
    let index_value: i64 = message
        .strip_prefix("array index ")?
        .split(' ')
        .next()?
        .parse()
        .ok()?;
    for (position, token) in tokens.iter().enumerate() {
        if !matches!(token.token, Token::LBracket) {
            continue;
        }
        let Some(object) = position.checked_sub(1).and_then(|prev| tokens.get(prev)) else {
            continue;
        };
        let start = match &object.token {
            Token::Ident(name) if subject == Some(name.as_str()) => object.range.start,
            Token::RBracket if subject.is_none() => token.range.start,
            _ => continue,
        };
        let mut cursor = position + 1;
        let negative = matches!(tokens.get(cursor).map(|t| &t.token), Some(Token::Minus));
        if negative {
            cursor += 1;
        }
        let Some(Token::IntLit(literal)) = tokens.get(cursor).map(|t| &t.token) else {
            continue;
        };
        let value = if negative { -*literal } else { *literal };
        if value != i128::from(index_value) {
            continue;
        }
        let Some(close) = tokens.get(cursor + 1) else {
            continue;
        };
        if !matches!(close.token, Token::RBracket) {
            continue;
        }
        let range = start..close.range.end;
        if used.contains(&range) {
            continue;
        }
        used.push(range.clone());
        return Some(range);
    }
    None
}

const ASSIGNMENT_OPERATORS: [Token; 16] = [
    Token::Eq,
    Token::PlusEq,
    Token::MinusEq,
    Token::StarEq,
    Token::SlashEq,
    Token::PercentEq,
    Token::PowerEq,
    Token::AndEq,
    Token::OrEq,
    Token::AmpersandEq,
    Token::PipeEq,
    Token::CaretEq,
    Token::ShlEq,
    Token::ShrEq,
    Token::UShrEq,
    Token::NullCoalesceEq,
];

fn locate_assignment_value_range(
    subject: Option<&str>,
    node: &Range<usize>,
    tokens: &[SpannedToken],
) -> Option<Range<usize>> {
    let subject = subject?;
    let mut last: Option<(usize, usize)> = None;
    for (index, token) in tokens.iter().enumerate() {
        if token.range.end > node.end {
            break;
        }
        if token.range.start < node.start {
            continue;
        }
        if !matches!(&token.token, Token::Ident(name) if name == subject) {
            continue;
        }
        let Some(assign) = tokens.get(index + 1) else {
            continue;
        };
        if !ASSIGNMENT_OPERATORS.contains(&assign.token) {
            continue;
        }
        let within_node = |cursor: &(usize, &SpannedToken)| cursor.1.range.end <= node.end;
        let value_start = index + 2;
        let end = tokens
            .iter()
            .enumerate()
            .skip(value_start)
            .take_while(within_node)
            .find(|(_, token)| token.token == Token::Semicolon)
            .map(|(cursor, _)| cursor)
            .or_else(|| {
                tokens
                    .iter()
                    .enumerate()
                    .skip(value_start)
                    .take_while(within_node)
                    .last()
                    .map(|(cursor, _)| cursor + 1)
            })?;
        if end == value_start {
            continue;
        }
        last = Some((value_start, end.saturating_sub(1)));
    }
    let (start, end) = last?;
    Some(tokens[start].range.start..tokens[end].range.end)
}

fn locate_force_unwrap(subject: Option<&str>, tokens: &[SpannedToken]) -> Option<Range<usize>> {
    for (position, token) in tokens.iter().enumerate() {
        if !matches!(token.token, Token::Bang) {
            continue;
        }
        let Some(previous) = position.checked_sub(1).and_then(|index| tokens.get(index)) else {
            continue;
        };
        match (&previous.token, subject) {
            (Token::Ident(name), Some(subject)) if name == subject => {
                return Some(previous.range.start..token.range.end);
            }
            (Token::RParen | Token::RBracket, None) => {
                return Some(previous.range.start..token.range.end);
            }
            _ => {}
        }
    }
    None
}

fn locate_validator(
    subject: Option<&str>,
    tokens: &[SpannedToken],
    used: &mut Vec<Range<usize>>,
) -> Option<Range<usize>> {
    let subject = subject?;
    let (target, validator) = match subject.split_once('\u{1f}') {
        Some((target, validator)) => (Some(target), validator),
        None => (None, subject),
    };
    let raw_name = validator.strip_prefix('@')?;
    let (name, occurrence) = raw_name
        .split_once('#')
        .map(|(name, occurrence)| (name, occurrence.parse::<usize>().ok()))
        .unwrap_or((raw_name, None));

    let Some(target) = target else {
        return find_validator_in_block(name, occurrence, tokens, used);
    };

    for (field_index, token) in tokens.iter().enumerate() {
        if !matches!(&token.token, Token::Ident(value) if value == target) {
            continue;
        }
        if !matches!(
            tokens.get(field_index + 1).map(|token| &token.token),
            Some(Token::Colon)
        ) {
            continue;
        }
        let block_start = tokens[..field_index]
            .iter()
            .rposition(|token| {
                matches!(
                    token.token,
                    Token::Semicolon | Token::LBrace | Token::RBrace | Token::Comma
                )
            })
            .map_or(0, |position| position + 1);
        if let Some(range) =
            find_validator_in_block(name, occurrence, &tokens[block_start..field_index], used)
        {
            return Some(range);
        }
    }
    None
}

fn find_validator_in_block(
    name: &str,
    occurrence: Option<usize>,
    tokens: &[SpannedToken],
    used: &mut Vec<Range<usize>>,
) -> Option<Range<usize>> {
    let mut seen = 0;
    for (index, token) in tokens.iter().enumerate() {
        if !matches!(token.token, Token::AtSign) {
            continue;
        }
        let Some(next) = tokens.get(index + 1) else {
            continue;
        };
        if !matches!(&next.token, Token::Ident(value) if value == name) {
            continue;
        }
        seen += 1;
        if occurrence.is_some_and(|target| seen != target) {
            continue;
        }
        let range = token.range.start..next.range.end;
        if used.contains(&range) {
            continue;
        }
        used.push(range.clone());
        return Some(range);
    }
    None
}

struct StyleTextShape {
    range: Range<usize>,
    is_chain: bool,
    piped: bool,
    called: bool,
    arg_count: usize,
    first_arg_is_literal: bool,
}

impl StyleTextShape {
    fn semantic_args(&self) -> usize {
        self.arg_count + usize::from(self.piped)
    }

    fn matches(&self, code: &str) -> bool {
        match code {
            "E3271" => !self.called && !self.piped,
            "E3270" => {
                let expected = if self.is_chain { 1 } else { 2 };
                (self.called || self.piped) && self.semantic_args() != expected
            }
            "E3272" => !self.is_chain && self.semantic_args() == 2 && !self.first_arg_is_literal,
            _ => false,
        }
    }
}

fn style_text_shape(tokens: &[SpannedToken], index: usize) -> Option<StyleTextShape> {
    let piped = index
        .checked_sub(1)
        .and_then(|prev| tokens.get(prev))
        .is_some_and(|token| matches!(token.token, Token::PipeOperator));
    let next = tokens.get(index + 1).map(|token| &token.token);
    if !piped && !matches!(next, Some(Token::LParen | Token::Dot)) {
        return None;
    }
    let start = tokens[index].range.start;
    let mut cursor = index + 1;
    let mut is_chain = false;
    while matches!(
        tokens.get(cursor).map(|token| &token.token),
        Some(Token::Dot | Token::Ident(_))
    ) {
        if matches!(tokens[cursor].token, Token::Dot) {
            is_chain = true;
        }
        cursor += 1;
    }
    let (called, arg_count, first_arg_is_literal, end) =
        if matches!(tokens.get(cursor).map(|t| &t.token), Some(Token::LParen)) {
            let first_arg_is_literal = matches!(
                tokens.get(cursor + 1).map(|token| &token.token),
                Some(Token::StringLit(_) | Token::LBracket)
            );
            let mut depth = 0_i32;
            let mut close = None;
            let mut arg_count = 0usize;
            let mut seen_token = false;
            for token in tokens.iter().skip(cursor) {
                match &token.token {
                    Token::LParen | Token::LBracket | Token::LBrace => depth += 1,
                    Token::RParen if depth == 1 => {
                        if seen_token {
                            arg_count += 1;
                        }
                        close = Some(token.range.end);
                        break;
                    }
                    Token::RParen | Token::RBracket | Token::RBrace => depth -= 1,
                    Token::Comma if depth == 1 => {
                        arg_count += 1;
                        continue;
                    }
                    _ => {}
                }
                if depth >= 1 && !matches!(token.token, Token::LParen if depth == 1) {
                    seen_token = true;
                }
            }
            (
                true,
                arg_count,
                first_arg_is_literal,
                close.unwrap_or(tokens[cursor].range.end),
            )
        } else {
            let end = tokens
                .get(cursor.saturating_sub(1))
                .map(|token| token.range.end)
                .unwrap_or(tokens[index].range.end);
            (false, 0, false, end)
        };
    Some(StyleTextShape {
        range: start..end,
        is_chain,
        piped,
        called,
        arg_count,
        first_arg_is_literal,
    })
}

fn locate_style_text(
    code: &str,
    subject: Option<&str>,
    tokens: &[SpannedToken],
    used: &mut Vec<Range<usize>>,
) -> Option<Range<usize>> {
    if subject? != "styleText" {
        return None;
    }
    for (index, token) in tokens.iter().enumerate() {
        if !matches!(&token.token, Token::Ident(name) if name == "styleText") {
            continue;
        }
        let Some(shape) = style_text_shape(tokens, index) else {
            continue;
        };
        if !shape.matches(code) {
            continue;
        }
        if used.contains(&shape.range) {
            continue;
        }
        used.push(shape.range.clone());
        return Some(shape.range);
    }
    None
}

fn locate_figlet(
    code: &str,
    subject: Option<&str>,
    tokens: &[SpannedToken],
    used: &mut Vec<Range<usize>>,
) -> Option<Range<usize>> {
    let subject = subject?;
    for (index, token) in tokens.iter().enumerate() {
        if !matches!(&token.token, Token::Ident(name) if name == "figlet") {
            continue;
        }
        if code == "E3275" {
            let range = token.range.clone();
            if used.contains(&range) {
                continue;
            }
            used.push(range.clone());
            return Some(range);
        }
        if !matches!(
            tokens.get(index + 1).map(|token| &token.token),
            Some(Token::LParen)
        ) {
            continue;
        }
        let mut depth = 0_i32;
        let mut close = None;
        let mut font = None;
        let mut non_literal_font = None;
        let mut top_level_arg = 0_usize;
        for token in tokens.iter().skip(index + 1) {
            match &token.token {
                Token::LParen | Token::LBracket | Token::LBrace => depth += 1,
                Token::RParen | Token::RBracket | Token::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(token.range.end);
                        break;
                    }
                }
                Token::Comma if depth == 1 => top_level_arg += 1,
                Token::StringLit(value)
                    if code == "E3277"
                        && top_level_arg == 1
                        && value == subject
                        && font.is_none() =>
                {
                    font = Some(token.range.clone());
                }
                Token::Ident(value)
                    if code == "E3276"
                        && top_level_arg == 1
                        && value == subject
                        && non_literal_font.is_none() =>
                {
                    non_literal_font = Some(token.range.clone());
                }
                _ => {}
            }
        }
        let range = match code {
            "E3277" => match font {
                Some(range) => range,
                None => continue,
            },
            "E3276" => match non_literal_font {
                Some(range) => range,
                None => token.range.start..close.unwrap_or(token.range.end),
            },
            _ => token.range.start..close.unwrap_or(token.range.end),
        };
        if used.contains(&range) {
            continue;
        }
        used.push(range.clone());
        return Some(range);
    }
    None
}

fn find_subject_range(
    subject: &str,
    symbols: &[Symbol],
    references: &[Reference],
    prefer_declaration: bool,
) -> Option<Range<usize>> {
    let declaration = || {
        symbols
            .iter()
            .rfind(|symbol| symbol.name == subject)
            .map(|symbol| symbol.declaration.clone())
    };
    let reference = || {
        references
            .iter()
            .rfind(|reference| reference.name == subject)
            .map(|reference| reference.range.clone())
    };
    if prefer_declaration {
        declaration().or_else(reference)
    } else {
        reference().or_else(declaration)
    }
}

fn locate_unsupported_regex_flag(
    message: &str,
    source: &str,
    tokens: &[SpannedToken],
) -> Option<Range<usize>> {
    let flag = message.split('`').nth(3)?.chars().next()?;
    tokens.iter().find_map(|token| {
        let Token::RegexLit(literal) = &token.token else {
            return None;
        };
        if !literal.flags.contains(flag) {
            return None;
        }
        let text = source.get(token.range.clone())?;
        let flags_start = text.rfind('/').map(|index| index + 1)?;
        let relative = text[flags_start..]
            .char_indices()
            .find(|(_, character)| *character == flag)
            .map(|(index, _)| flags_start + index)?;
        let start = token.range.start + relative;
        Some(start..start + flag.len_utf8())
    })
}

fn attach_documentation(source: &str, docs: &[DocComment], symbols: &mut [Symbol]) {
    for symbol in symbols {
        symbol.documentation =
            doc_comment_before_decl(source, symbol.declaration.start, docs).cloned();
    }
}

fn advance_token(
    tokens: &[SpannedToken],
    from: usize,
    predicate: impl Fn(&Token) -> bool,
) -> usize {
    let mut index = from;
    while index < tokens.len() {
        if predicate(&tokens[index].token) {
            return index + 1;
        }
        index += 1;
    }
    index
}

fn collect_program_prototypes(
    program: &Program,
    prototypes: &mut HashMap<String, VecDeque<SymbolPrototype>>,
    interfaces: &HashMap<String, Vec<Field>>,
) {
    for item in &program.items {
        match item {
            Item::InterfaceDecl { .. } => {}
            Item::BehaviorInterfaceDecl { .. } => {}
            Item::TypeAlias { .. } => {}
            Item::EnumDecl { .. } => {}
            Item::FnDecl {
                name,
                params,
                ret_type,
                body,
                ..
            } => {
                let ty = Type::Function {
                    params: params.iter().map(|param| param.ty.clone()).collect(),
                    ret: Box::new(ret_type.clone()),
                };
                let display_ty = format!(
                    "({}) => {ret_type}",
                    params
                        .iter()
                        .map(|param| {
                            let pattern_str = match &param.pattern {
                                ParamPattern::Name(n) => n.clone(),
                                ParamPattern::Array(names) => format!("[{}]", names.join(", ")),
                                ParamPattern::Object(names) => format!("{{{}}}", names.join(", ")),
                            };
                            format!("{pattern_str}: {}", param.ty)
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                prototypes
                    .entry(name.clone())
                    .or_default()
                    .push_back(SymbolPrototype {
                        ty,
                        display_ty: Some(display_ty),
                        parameters: params.clone(),
                        kind: SymbolKind::Function,
                    });
                collect_params(params, prototypes, interfaces);
                collect_statements(body, prototypes, interfaces);
            }
            Item::Stmt(statement) => collect_statement(statement, prototypes, interfaces),
        }
    }
}

fn collect_params(
    params: &[Param],
    prototypes: &mut HashMap<String, VecDeque<SymbolPrototype>>,
    interfaces: &HashMap<String, Vec<Field>>,
) {
    for param in params {
        match &param.pattern {
            ParamPattern::Name(name) => {
                push_prototype(prototypes, name, param.ty.clone(), SymbolKind::Parameter);
            }
            ParamPattern::Array(names) => {
                let element_ty = match &param.ty {
                    Type::Array(element) => (**element).clone(),
                    _ => Type::Infer,
                };
                for name in names {
                    push_prototype(prototypes, name, element_ty.clone(), SymbolKind::Parameter);
                }
            }
            ParamPattern::Object(names) => {
                let fields = destructured_param_fields(&param.ty, interfaces);
                for name in names {
                    let field_ty = fields
                        .iter()
                        .find(|field| &field.name == name)
                        .map(|field| field.ty.clone())
                        .unwrap_or(Type::Infer);
                    push_prototype(prototypes, name, field_ty, SymbolKind::Parameter);
                }
            }
        }
    }
}

fn destructured_param_fields<'a>(
    ty: &'a Type,
    interfaces: &'a HashMap<String, Vec<Field>>,
) -> &'a [Field] {
    match ty {
        Type::Named(name) => interfaces.get(name).map(Vec::as_slice).unwrap_or(&[]),
        Type::Object(fields) => fields,
        _ => &[],
    }
}

fn collect_interface_fields(program: &Program) -> HashMap<String, Vec<Field>> {
    program
        .items
        .iter()
        .filter_map(|item| match item {
            Item::InterfaceDecl { name, fields, .. } => Some((name.clone(), fields.clone())),
            _ => None,
        })
        .collect()
}

fn collect_statements(
    statements: &[Stmt],
    prototypes: &mut HashMap<String, VecDeque<SymbolPrototype>>,
    interfaces: &HashMap<String, Vec<Field>>,
) {
    for statement in statements {
        collect_statement(statement, prototypes, interfaces);
    }
}

fn collect_statement(
    statement: &Stmt,
    prototypes: &mut HashMap<String, VecDeque<SymbolPrototype>>,
    interfaces: &HashMap<String, Vec<Field>>,
) {
    match statement.kind() {
        StmtKind::VarDecl {
            name, ty, value, ..
        } => {
            push_prototype(prototypes, name, ty.clone(), SymbolKind::Variable);
            if let Some(value) = value {
                collect_expr(value, prototypes, interfaces);
            }
        }
        StmtKind::Assign { value, .. } | StmtKind::Return(Some(value)) | StmtKind::Expr(value) => {
            collect_expr(value, prototypes, interfaces);
        }
        StmtKind::IndexAssign { index, value, .. } => {
            collect_expr(index, prototypes, interfaces);
            collect_expr(value, prototypes, interfaces);
        }
        StmtKind::FieldAssign { object, value, .. } => {
            collect_expr(object, prototypes, interfaces);
            collect_expr(value, prototypes, interfaces);
        }
        StmtKind::Return(None) => {}
        StmtKind::Break(value) => {
            if let Some(value) = value {
                collect_expr(value, prototypes, interfaces);
            }
        }
        StmtKind::Continue => {}
        StmtKind::Defer { body, .. } => collect_statements(body, prototypes, interfaces),
        StmtKind::ArrayDestructure {
            bindings, value, ..
        }
        | StmtKind::ObjectDestructure {
            bindings, value, ..
        } => {
            for binding in bindings {
                push_prototype(
                    prototypes,
                    &binding.name,
                    binding.ty.clone(),
                    SymbolKind::Variable,
                );
            }
            collect_expr(value, prototypes, interfaces);
        }
        StmtKind::While { cond, body } => {
            collect_expr(cond, prototypes, interfaces);
            collect_statements(body, prototypes, interfaces);
        }
        StmtKind::DoWhile { body, cond } => {
            collect_statements(body, prototypes, interfaces);
            collect_expr(cond, prototypes, interfaces);
        }
        StmtKind::For {
            init,
            cond,
            update,
            body,
        } => {
            collect_statement(init, prototypes, interfaces);
            collect_expr(cond, prototypes, interfaces);
            collect_statement(update, prototypes, interfaces);
            collect_statements(body, prototypes, interfaces);
        }
        StmtKind::ForOf {
            name,
            iterable,
            body,
            ..
        } => {
            push_prototype(prototypes, name, Type::Infer, SymbolKind::Parameter);
            collect_expr(iterable, prototypes, interfaces);
            collect_statements(body, prototypes, interfaces);
        }
        StmtKind::ForIn { name, object, body } => {
            push_prototype(prototypes, name, Type::Str, SymbolKind::Parameter);
            collect_expr(object, prototypes, interfaces);
            collect_statements(body, prototypes, interfaces);
        }
        StmtKind::Switch {
            expr,
            cases,
            default,
            ..
        } => {
            collect_expr(expr, prototypes, interfaces);
            for case in cases {
                collect_statements(&case.body, prototypes, interfaces);
            }
            if let Some(default_body) = default {
                collect_statements(default_body, prototypes, interfaces);
            }
        }
        StmtKind::Match {
            expr,
            arms,
            default,
        } => {
            collect_expr(expr, prototypes, interfaces);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_expr(guard, prototypes, interfaces);
                }
                collect_statements(&arm.body, prototypes, interfaces);
            }
            if let Some(default_body) = default {
                collect_statements(default_body, prototypes, interfaces);
            }
        }
        StmtKind::Throw(expr) => {
            collect_expr(expr, prototypes, interfaces);
        }
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            collect_statements(try_body, prototypes, interfaces);
            collect_statements(catch_body, prototypes, interfaces);
            if let Some(stmts) = finally_body {
                collect_statements(stmts, prototypes, interfaces);
            }
        }
    }
}

fn collect_expr(
    expr: &Expr,
    prototypes: &mut HashMap<String, VecDeque<SymbolPrototype>>,
    interfaces: &HashMap<String, Vec<Field>>,
) {
    match expr.kind() {
        ExprKind::ObjectLit(fields) => {
            for (_, value) in fields {
                collect_expr(value, prototypes, interfaces);
            }
        }
        ExprKind::ObjectUpdate { base, fields } => {
            collect_expr(base, prototypes, interfaces);
            for (_, value) in fields {
                collect_expr(value, prototypes, interfaces);
            }
        }
        ExprKind::ArrayLit(values) => {
            for value in values {
                collect_expr(value, prototypes, interfaces);
            }
        }
        ExprKind::ArrayRepeat { value, count } => {
            collect_expr(value, prototypes, interfaces);
            collect_expr(count, prototypes, interfaces);
        }
        ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
            collect_expr(object, prototypes, interfaces);
        }
        ExprKind::IndexAccess { object, index } => {
            collect_expr(object, prototypes, interfaces);
            collect_expr(index, prototypes, interfaces);
        }
        ExprKind::ArrowFn { params, body, .. } => {
            collect_params(params, prototypes, interfaces);
            collect_statements(body, prototypes, interfaces);
        }
        ExprKind::BinOp { left, right, .. } => {
            collect_expr(left, prototypes, interfaces);
            collect_expr(right, prototypes, interfaces);
        }
        ExprKind::Pipeline { left, right } => {
            collect_expr(left, prototypes, interfaces);
            collect_expr(right, prototypes, interfaces);
        }
        ExprKind::ChainedComparison { terms, .. } => {
            for term in terms {
                collect_expr(term, prototypes, interfaces);
            }
        }
        ExprKind::Not(inner)
        | ExprKind::Neg(inner)
        | ExprKind::BitNot(inner)
        | ExprKind::Typeof(inner)
        | ExprKind::Await(inner)
        | ExprKind::Try(inner)
        | ExprKind::Unwrap(inner)
        | ExprKind::Spread(inner) => collect_expr(inner, prototypes, interfaces),
        ExprKind::TypeAssert { value, .. } => collect_expr(value, prototypes, interfaces),
        ExprKind::Is { expr, .. } => collect_expr(expr, prototypes, interfaces),
        ExprKind::ConstAssert(inner) | ExprKind::Comptime(inner) => {
            collect_expr(inner, prototypes, interfaces);
        }
        ExprKind::PrefixIncr(inner)
        | ExprKind::PrefixDecr(inner)
        | ExprKind::PostfixIncr(inner)
        | ExprKind::PostfixDecr(inner) => collect_expr(inner, prototypes, interfaces),
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            collect_expr(cond, prototypes, interfaces);
            collect_expr(then_expr, prototypes, interfaces);
            collect_expr(else_expr, prototypes, interfaces);
        }
        ExprKind::TemplateLit(parts) => {
            for part in parts {
                if let TemplatePart::Expr(expr, _) = part {
                    collect_expr(expr, prototypes, interfaces);
                }
            }
        }
        ExprKind::MethodCall { object, args, .. }
        | ExprKind::OptionalMethodCall { object, args, .. } => {
            collect_expr(object, prototypes, interfaces);
            for arg in args {
                collect_expr(arg, prototypes, interfaces);
            }
        }
        ExprKind::CallValue { callee, args, .. } => {
            collect_expr(callee, prototypes, interfaces);
            for arg in args {
                collect_expr(arg, prototypes, interfaces);
            }
        }
        ExprKind::FnCall { args, .. } => {
            for arg in args {
                collect_expr(arg, prototypes, interfaces);
            }
        }
        ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => {
            if let Some(initial) = initial {
                collect_expr(initial, prototypes, interfaces);
            }
        }
        ExprKind::If {
            cond,
            then_block,
            else_block,
        } => {
            collect_expr(cond, prototypes, interfaces);
            collect_statements(then_block, prototypes, interfaces);
            if let Some(block) = else_block {
                collect_statements(block, prototypes, interfaces);
            }
        }
        ExprKind::Loop { body } => collect_statements(body, prototypes, interfaces),
        ExprKind::In { key, object } => {
            collect_expr(key, prototypes, interfaces);
            collect_expr(object, prototypes, interfaces);
        }
        ExprKind::Range { start, end, .. } => {
            collect_expr(start, prototypes, interfaces);
            collect_expr(end, prototypes, interfaces);
        }
        ExprKind::StrLit(_)
        | ExprKind::RegexLit { .. }
        | ExprKind::IntLit(_)
        | ExprKind::UIntLit(_)
        | ExprKind::FloatLit(_)
        | ExprKind::DecimalLit(_)
        | ExprKind::BoolLit(_)
        | ExprKind::Null
        | ExprKind::None
        | ExprKind::Ident(_)
        | ExprKind::Topic
        | ExprKind::Borrow { .. } => {}
    }
}

fn push_prototype(
    prototypes: &mut HashMap<String, VecDeque<SymbolPrototype>>,
    name: &str,
    ty: Type,
    kind: SymbolKind,
) {
    prototypes
        .entry(name.to_string())
        .or_default()
        .push_back(SymbolPrototype {
            ty,
            display_ty: None,
            parameters: Vec::new(),
            kind,
        });
}

pub fn resolve_import_path(document: &Path, source: &str) -> Option<PathBuf> {
    if source.starts_with('#') {
        return crate::project::resolve_import_path(document, source)
            .ok()
            .flatten();
    }
    if !source.starts_with('.') {
        return None;
    }
    let mut path = document.parent()?.join(source);
    if path.is_dir() {
        path = path.join("index.tyr");
    } else if path.extension().is_none() {
        path.set_extension("tyr");
    }
    Some(path)
}

#[cfg(test)]
mod cancellation_tests {
    use super::{AnalysisCancelled, AnalyzeContext, ParseForAnalysis};
    use std::sync::{Arc, atomic::AtomicBool};

    #[test]
    fn cancellable_analysis_and_parser_stop_before_starting_when_cancelled() {
        let cancellation = Arc::new(AtomicBool::new(true));
        assert!(matches!(
            super::parse_for_analysis_cancellable(
                "const value: i32 = 1;",
                Arc::clone(&cancellation),
            ),
            Err(ParseForAnalysis::Cancelled)
        ));
        assert!(matches!(
            super::analyze_with_context_cancellable(
                "const value: i32 = 1;",
                AnalyzeContext::default(),
                cancellation,
            ),
            Err(AnalysisCancelled)
        ));
    }

    #[test]
    fn cancellable_parser_reports_separate_lexing_and_parsing_metrics() {
        let cancellation = Arc::new(AtomicBool::new(false));
        let (parsed, metrics) = super::parse_for_analysis_cancellable_with_metrics(
            "const value: i32 = 1;",
            cancellation,
        );
        let (tokens, _) = match parsed {
            Ok(parsed) => parsed,
            Err(_) => panic!("source should parse"),
        };
        assert_eq!(metrics.token_count, tokens.len());
    }
}

#[cfg(test)]
#[path = "../../tests/unit/analysis.rs"]
mod tests;
