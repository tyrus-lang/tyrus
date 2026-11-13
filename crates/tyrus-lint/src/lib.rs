use std::borrow::Cow;
use std::ops::Range;
use std::path::Path;
use tyrus_core::analysis::{
    AnalysisDiagnostic, AnalyzeContext, DiagnosticSeverity, DocumentAnalysis, SymbolKind, analyze,
    analyze_with_context, resolve_import_path,
};
use tyrus_core::ast::{ExprKind, Field, Item, Program, StmtKind, Type};
use tyrus_core::collections::{HashMap, HashSet};
use tyrus_core::parser::Parser;

#[path = "config/config.rs"]
pub mod config;
#[path = "rules/rules.rs"]
mod rules;

pub use config::{Category, LintConfig, RULES, RuleMeta, Severity};

pub use tyrus_core::analysis::SpannedToken;

pub struct LintContext<'a> {
    pub source: &'a str,
    pub file: Option<&'a Path>,
    pub program: &'a Program,
    pub analysis: &'a DocumentAnalysis,
    pub tokens: &'a [SpannedToken],
    pub config: &'a LintConfig,
}

#[derive(Debug, Clone)]
pub struct LintFix {
    pub message: String,
    pub range: Range<usize>,
    pub replacement: String,
}

#[derive(Debug, Clone)]
pub struct RuleDiagnostic {
    pub range: Range<usize>,
    pub message: String,
    pub help: Option<String>,
    pub note: Option<String>,
    pub fix: Option<LintFix>,
}

#[derive(Debug, Clone)]
pub struct LintDiagnostic {
    pub code: String,
    pub severity: Severity,
    pub range: Range<usize>,
    pub message: String,
    pub help: Option<String>,
    pub note: Option<String>,
    pub fix: Option<LintFix>,
}

pub fn lex(source: &str) -> Vec<SpannedToken> {
    tyrus_core::lexer::lex_spanned(source)
        .unwrap_or_default()
        .into_iter()
        .map(|(token, range)| SpannedToken { token, range })
        .collect()
}

pub fn lint_source(source: &str, config: &LintConfig) -> Vec<LintDiagnostic> {
    lint_source_with_options(source, config, true)
}

pub fn lint_rules_source(source: &str, config: &LintConfig) -> Vec<LintDiagnostic> {
    lint_source_with_options(source, config, false)
}

pub fn lint_rules_source_with_analysis(
    source: &str,
    analysis: &DocumentAnalysis,
    config: &LintConfig,
) -> Vec<LintDiagnostic> {
    let mut diagnostics = lint_rules_with_analysis(source, analysis, config);
    diagnostics.sort_by_key(|diagnostic| (diagnostic.range.start, diagnostic.range.end));
    diagnostics
}

pub fn lint_rules_source_with_analysis_cancellable(
    source: &str,
    analysis: &DocumentAnalysis,
    config: &LintConfig,
    cancellation: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Option<Vec<LintDiagnostic>> {
    let mut diagnostics =
        lint_rules_with_analysis_cancellable(source, analysis, config, Some(cancellation))?;
    diagnostics.sort_by_key(|diagnostic| (diagnostic.range.start, diagnostic.range.end));
    Some(diagnostics)
}

fn lint_source_with_options(
    source: &str,
    config: &LintConfig,
    include_compiler_diagnostics: bool,
) -> Vec<LintDiagnostic> {
    let tokens = lex(source);
    let analysis = analyze(source);
    lint_source_with_analysis(
        None,
        source,
        config,
        &tokens,
        &analysis,
        include_compiler_diagnostics,
    )
}

fn lint_file_source(
    file: &Path,
    source: &str,
    config: &LintConfig,
    env_schema: Option<&tyrus_core::env::EnvSchema>,
) -> Vec<LintDiagnostic> {
    let tokens = lex(source);
    let analysis = analyze_file_source(file, source, env_schema);
    lint_source_with_analysis(Some(file), source, config, &tokens, &analysis, true)
}

fn lint_source_with_analysis(
    file: Option<&Path>,
    source: &str,
    config: &LintConfig,
    tokens: &[SpannedToken],
    analysis: &DocumentAnalysis,
    include_compiler_diagnostics: bool,
) -> Vec<LintDiagnostic> {
    let mut diagnostics = if include_compiler_diagnostics {
        analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| !is_lint_owned_analysis_diagnostic(diagnostic))
            .map(compiler_diagnostic)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    diagnostics.extend(lint_rules_with_tokens(
        file, source, analysis, tokens, config,
    ));
    diagnostics.sort_by_key(|diagnostic| (diagnostic.range.start, diagnostic.range.end));
    diagnostics
}

fn analyze_file_source(
    file: &Path,
    source: &str,
    env_schema: Option<&tyrus_core::env::EnvSchema>,
) -> DocumentAnalysis {
    analyze_file_source_inner(file, source, env_schema, &mut HashSet::default())
}

fn analyze_file_source_inner(
    file: &Path,
    source: &str,
    env_schema: Option<&tyrus_core::env::EnvSchema>,
    visiting: &mut HashSet<std::path::PathBuf>,
) -> DocumentAnalysis {
    let entry_point = tyrus_core::project::is_entry_module(file);
    let project_root = tyrus_core::project::find_project_root(file);
    let project_database = project_root
        .as_deref()
        .and_then(|root| tyrus_core::project::load_project(root).ok())
        .and_then(|project| project.database);
    let db_backend = project_database
        .as_ref()
        .map(|database| database.backend.clone());
    let db_schema_path = project_root
        .as_deref()
        .zip(project_database.as_ref())
        .map(|(root, database)| root.join(&database.schema));
    let db_schema_result = db_schema_path
        .as_deref()
        .map(tyrus_core::db::DbSchemaSnapshot::load);
    let db_schema = db_schema_result
        .as_ref()
        .and_then(|result| result.as_ref().ok());
    let db_schema_error = db_schema_result
        .as_ref()
        .and_then(|result| result.as_ref().err());
    if !visiting.insert(file.to_path_buf()) {
        return analyze_with_context(
            source,
            AnalyzeContext {
                env_schema,
                entry_point,
                ..AnalyzeContext::default()
            },
        );
    }
    let mut analysis = analyze_with_context(
        source,
        AnalyzeContext {
            env_schema,
            entry_point,
            ..AnalyzeContext::default()
        },
    );
    let mut import_types = HashMap::default();
    let mut import_interfaces = HashMap::default();
    let mut import_type_aliases = HashMap::default();
    let mut import_enums = HashMap::default();
    let mut import_db_models = HashMap::default();
    let mut reachable_interfaces = HashMap::default();
    let mut import_diagnostics = Vec::new();
    let has_implicit_timestamps =
        tyrus_core::analysis::parse_for_analysis(source).is_ok_and(|(_, program)| {
            program.items.iter().any(|item| {
                matches!(
                    item,
                    Item::InterfaceDecl {
                        db_model: Some(model),
                        ..
                    } if model.timestamps
                )
            })
        });

    for import in analysis
        .symbols
        .iter()
        .filter(|symbol| symbol.kind == SymbolKind::Import)
    {
        let Some(module) = import.import_source.as_deref() else {
            continue;
        };
        let imported = import.imported_name.as_deref().unwrap_or(&import.name);
        if imported.is_empty() {
            continue;
        }
        if module.starts_with("tyrus:") {
            if let Some((ty, interface_fields, alias_ty)) =
                tyrus_core::module_loader::native_export_info(module, imported)
            {
                insert_import_info(
                    &import.name,
                    ty,
                    interface_fields,
                    alias_ty,
                    &mut import_types,
                    &mut import_interfaces,
                    &mut import_type_aliases,
                );
                if let Some(enum_info) =
                    tyrus_core::module_loader::native_export_enum(module, imported)
                {
                    import_enums.insert(import.name.clone(), enum_info);
                }
            } else {
                import_diagnostics.push(AnalysisDiagnostic {
                    code: "E0432".to_string(),
                    range: import.declaration.clone(),
                    severity: DiagnosticSeverity::Error,
                    message: format!("module `{module}` does not export `{imported}`"),
                    related: Vec::new(),
                    help: Vec::new(),
                });
            }
            continue;
        }

        let Some(target) = resolve_import_path(file, module) else {
            import_diagnostics.push(AnalysisDiagnostic {
                code: "E0432".to_string(),
                range: import.declaration.clone(),
                severity: DiagnosticSeverity::Error,
                message: format!("unsupported module `{module}`"),
                related: Vec::new(),
                help: vec![
                    "use a relative module path such as `./module` or a supported `tyrus:` module"
                        .to_string(),
                ],
            });
            continue;
        };
        add_local_import_info(
            ImportResolutionContext {
                file,
                import_types: &mut import_types,
                import_interfaces: &mut import_interfaces,
                import_type_aliases: &mut import_type_aliases,
                import_enums: &mut import_enums,
                import_db_models: &mut import_db_models,
                reachable_interfaces: &mut reachable_interfaces,
                import_diagnostics: &mut import_diagnostics,
                env_schema,
                visiting,
            },
            &target,
            module,
            imported,
            &import.name,
            &import.declaration,
        );
    }

    if has_implicit_timestamps
        && let Some((ty, fields, alias_ty)) =
            tyrus_core::module_loader::native_export_info("tyrus:temporal", "Instant")
    {
        insert_import_info(
            "Instant",
            ty,
            fields,
            alias_ty,
            &mut import_types,
            &mut import_interfaces,
            &mut import_type_aliases,
        );
    }

    if !import_types.is_empty()
        || !import_interfaces.is_empty()
        || !import_type_aliases.is_empty()
        || !import_enums.is_empty()
    {
        analysis = analyze_with_context(
            source,
            AnalyzeContext {
                import_types: Some(&import_types),
                import_targets: None,
                entry_point,
                import_interfaces: Some(&import_interfaces),
                import_behaviors: None,
                import_type_aliases: Some(&import_type_aliases),
                import_enums: Some(&import_enums),
                import_throws: None,
                import_db_models: Some(&import_db_models),
                import_validators: None,
                reachable_interfaces: Some(&reachable_interfaces),
                env_schema,
                db_schema,
                db_schema_error: db_schema_error.map(String::as_str),
                db_backend: db_backend.as_ref(),
            },
        );
    }
    analysis.diagnostics.extend(import_diagnostics);
    visiting.remove(file);
    analysis
}

#[allow(
    clippy::too_many_arguments,
    reason = "the arguments update the three parallel semantic import maps without allocating an aggregate"
)]
fn insert_import_info(
    local: &str,
    ty: Type,
    interface_fields: Option<Vec<Field>>,
    alias_ty: Option<Type>,
    import_types: &mut HashMap<String, Type>,
    import_interfaces: &mut HashMap<String, Vec<Field>>,
    import_type_aliases: &mut HashMap<String, Type>,
) {
    import_types.insert(local.to_string(), ty);
    if let Some(fields) = interface_fields {
        import_interfaces.insert(local.to_string(), fields);
    }
    if let Some(alias_ty) = alias_ty {
        import_type_aliases.insert(local.to_string(), alias_ty);
    }
}

fn resolve_public_export(
    module_path: &Path,
    program: Option<&tyrus_core::ast::Program>,
    public_name: &str,
    visited: &mut HashSet<std::path::PathBuf>,
) -> Option<(std::path::PathBuf, String)> {
    use tyrus_core::ast::ExportDecl;

    if !visited.insert(module_path.to_path_buf()) {
        return None;
    }
    let program = program?;
    let mut wildcards = Vec::new();
    for export in &program.exports {
        match export {
            ExportDecl::Named {
                exported, local, ..
            } if exported == public_name => {
                return Some((module_path.to_path_buf(), local.clone()));
            }
            ExportDecl::Re {
                exported,
                imported,
                source,
                ..
            } if exported == public_name => {
                if source.starts_with("tyrus:") {
                    return Some((module_path.to_path_buf(), imported.clone()));
                }
                let next = resolve_import_path(module_path, source)?;
                let next_source = std::fs::read_to_string(&next).ok()?;
                let next_program = Parser::from_source(&next_source)
                    .ok()
                    .and_then(|mut parser| parser.parse_program().ok());
                return resolve_public_export(&next, next_program.as_ref(), imported, visited);
            }
            ExportDecl::All { source, .. } => wildcards.push(source.clone()),
            _ => {}
        }
    }
    if public_name == tyrus_core::ast::DEFAULT_EXPORT {
        return None;
    }
    for source in wildcards {
        if source.starts_with("tyrus:") {
            continue;
        }
        let Some(next) = resolve_import_path(module_path, &source) else {
            continue;
        };
        let Ok(next_source) = std::fs::read_to_string(&next) else {
            continue;
        };
        let next_program = Parser::from_source(&next_source)
            .ok()
            .and_then(|mut parser| parser.parse_program().ok());
        if let Some(found) =
            resolve_public_export(&next, next_program.as_ref(), public_name, visited)
        {
            return Some(found);
        }
    }
    None
}

fn protected_import_diagnostic(
    imported: &str,
    module: &str,
    range: &Range<usize>,
    violation: &tyrus_core::module_loader::ProtectedImport,
) -> AnalysisDiagnostic {
    AnalysisDiagnostic {
        code: "E3308".to_string(),
        range: range.clone(),
        severity: DiagnosticSeverity::Error,
        message: violation.message(imported, module),
        related: Vec::new(),
        help: violation.help(imported),
    }
}

struct ImportResolutionContext<'a> {
    file: &'a Path,
    import_types: &'a mut HashMap<String, Type>,
    import_interfaces: &'a mut HashMap<String, Vec<Field>>,
    import_type_aliases: &'a mut HashMap<String, Type>,
    import_enums: &'a mut HashMap<String, tyrus_core::ast::ResolvedEnum>,
    import_db_models: &'a mut HashMap<String, tyrus_core::ast::DbModelMetadata>,
    reachable_interfaces: &'a mut HashMap<String, Vec<Field>>,
    import_diagnostics: &'a mut Vec<AnalysisDiagnostic>,
    env_schema: Option<&'a tyrus_core::env::EnvSchema>,
    visiting: &'a mut HashSet<std::path::PathBuf>,
}

fn add_local_import_info(
    context: ImportResolutionContext<'_>,
    target: &Path,
    module: &str,
    imported: &str,
    local: &str,
    range: &Range<usize>,
) {
    let ImportResolutionContext {
        file,
        import_types,
        import_interfaces,
        import_type_aliases,
        import_enums,
        import_db_models,
        reachable_interfaces,
        import_diagnostics,
        env_schema,
        visiting,
    } = context;
    let Ok(target_source) = std::fs::read_to_string(target) else {
        import_diagnostics.push(AnalysisDiagnostic {
            code: "E0583".to_string(),
            range: range.clone(),
            severity: DiagnosticSeverity::Error,
            message: format!("cannot resolve module `{module}`"),
            related: Vec::new(),
            help: Vec::new(),
        });
        return;
    };
    let target_program = Parser::from_source(&target_source)
        .ok()
        .and_then(|mut parser| parser.parse_program().ok());
    let target_analysis = analyze_file_source_inner(target, &target_source, env_schema, visiting);
    if imported.is_empty() {
        return;
    }
    if let Some(violation) =
        tyrus_core::module_loader::protected_import_violation(file, target, imported)
    {
        import_diagnostics.push(protected_import_diagnostic(
            imported, module, range, &violation,
        ));
        return;
    }
    let Some((declaring_module, declared_name)) = resolve_public_export(
        target,
        target_program.as_ref(),
        imported,
        &mut HashSet::default(),
    ) else {
        import_diagnostics.push(AnalysisDiagnostic {
            code: "E0432".to_string(),
            range: range.clone(),
            severity: DiagnosticSeverity::Error,
            message: if imported == tyrus_core::ast::DEFAULT_EXPORT {
                format!("module `{module}` has no default export")
            } else {
                format!("module `{module}` does not export `{imported}`")
            },
            related: Vec::new(),
            help: if imported == tyrus_core::ast::DEFAULT_EXPORT {
                vec![format!(
                    "add `export default <name>;` to `{module}`, or import a named export instead"
                )]
            } else {
                Vec::new()
            },
        });
        return;
    };
    let target_analysis = if declaring_module == target {
        target_analysis
    } else {
        let Ok(source) = std::fs::read_to_string(&declaring_module) else {
            return;
        };
        analyze_file_source_inner(&declaring_module, &source, env_schema, visiting)
    };
    let imported = declared_name.as_str();

    let Some(target_symbol) = target_analysis
        .symbols
        .iter()
        .find(|symbol| symbol.exported && symbol.name == imported)
    else {
        import_diagnostics.push(AnalysisDiagnostic {
            code: "E0432".to_string(),
            range: range.clone(),
            severity: DiagnosticSeverity::Error,
            message: format!("module `{module}` does not export `{imported}`"),
            related: Vec::new(),
            help: Vec::new(),
        });
        return;
    };

    if target_symbol.kind == SymbolKind::Interface {
        if let Some(model) = target_program.as_ref().and_then(|program| {
            program.items.iter().find_map(|item| match item {
                Item::InterfaceDecl {
                    name,
                    db_model: Some(model),
                    ..
                } if name == imported => Some(model.clone()),
                _ => None,
            })
        }) {
            import_db_models.insert(local.to_string(), model);
        }
        if let Some(Type::Object(fields)) = &target_symbol.semantic_type {
            import_interfaces.insert(local.to_string(), fields.clone());
            import_types.insert(local.to_string(), Type::Named(imported.to_string()));
            tyrus_core::analysis::collect_reachable_interfaces(
                std::slice::from_ref(fields),
                &|name| {
                    target_analysis
                        .symbols
                        .iter()
                        .find(|symbol| {
                            symbol.exported
                                && symbol.kind == SymbolKind::Interface
                                && symbol.name == name
                        })
                        .and_then(|symbol| match &symbol.semantic_type {
                            Some(Type::Object(fields)) => Some(fields.clone()),
                            _ => None,
                        })
                },
                reachable_interfaces,
            );
        }
    } else if target_symbol.kind == SymbolKind::TypeAlias {
        if let Some(ty) = &target_symbol.semantic_type {
            import_type_aliases.insert(local.to_string(), ty.clone());
            import_types.insert(local.to_string(), Type::Named(imported.to_string()));
        }
    } else if target_symbol.kind == SymbolKind::Enum {
        if let Some(program) = &target_program
            && let Some(Item::EnumDecl { repr, members, .. }) = program
                .items
                .iter()
                .find(|item| matches!(item, Item::EnumDecl { name, .. } if name == imported))
        {
            let (resolved, diagnostics) = tyrus_core::ast::resolve_enum(repr, members);
            if diagnostics.is_empty() {
                import_enums.insert(local.to_string(), resolved);
                import_types.insert(local.to_string(), Type::Named(imported.to_string()));
            }
        }
    } else if let Some(ty) = &target_symbol.semantic_type {
        let declared_ty = target_program
            .as_ref()
            .and_then(|program| declared_export_type(program, imported))
            .unwrap_or_else(|| ty.clone());
        let ty = target_program
            .as_ref()
            .map(|program| expand_local_type(&declared_ty, program, &mut HashSet::default()))
            .unwrap_or(declared_ty);
        import_types.insert(local.to_string(), ty);
    }
}

fn declared_export_type(program: &Program, name: &str) -> Option<Type> {
    program.items.iter().find_map(|item| match item {
        Item::FnDecl {
            name: declared,
            params,
            ret_type,
            ..
        } if declared == name => Some(Type::Function {
            params: params.iter().map(|param| param.ty.clone()).collect(),
            ret: Box::new(ret_type.clone()),
        }),
        Item::Stmt(statement) => match statement.kind() {
            StmtKind::VarDecl {
                name: declared,
                value: Some(value),
                ..
            } if declared == name => match value.kind() {
                ExprKind::ArrowFn {
                    params, ret_type, ..
                } => Some(Type::Function {
                    params: params.iter().map(|param| param.ty.clone()).collect(),
                    ret: Box::new(ret_type.clone()),
                }),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    })
}

fn expand_local_type(ty: &Type, program: &Program, visiting: &mut HashSet<String>) -> Type {
    match ty {
        Type::Named(name) if visiting.insert(name.clone()) => {
            let expanded = program.items.iter().find_map(|item| match item {
                Item::InterfaceDecl {
                    name: declared,
                    fields,
                    ..
                } if declared == name => Some(Type::Object(
                    fields
                        .iter()
                        .map(|field| Field {
                            ty: expand_local_type(&field.ty, program, visiting),
                            ..field.clone()
                        })
                        .collect(),
                )),
                Item::TypeAlias {
                    name: declared, ty, ..
                } if declared == name => Some(expand_local_type(ty, program, visiting)),
                _ => None,
            });
            visiting.remove(name);
            expanded.unwrap_or_else(|| ty.clone())
        }
        Type::Array(inner) => Type::Array(Box::new(expand_local_type(inner, program, visiting))),
        Type::Tuple(types) => Type::Tuple(
            types
                .iter()
                .map(|ty| expand_local_type(ty, program, visiting))
                .collect(),
        ),
        Type::Readonly(inner) => {
            Type::Readonly(Box::new(expand_local_type(inner, program, visiting)))
        }
        Type::Ref(inner) => Type::Ref(Box::new(expand_local_type(inner, program, visiting))),
        Type::MutRef(inner) => Type::MutRef(Box::new(expand_local_type(inner, program, visiting))),
        Type::Optional(inner) => {
            Type::Optional(Box::new(expand_local_type(inner, program, visiting)))
        }
        Type::Function { params, ret } => Type::Function {
            params: params
                .iter()
                .map(|ty| expand_local_type(ty, program, visiting))
                .collect(),
            ret: Box::new(expand_local_type(ret, program, visiting)),
        },
        Type::GenericFunction {
            type_params,
            params,
            ret,
        } => Type::GenericFunction {
            type_params: type_params
                .iter()
                .map(|param| tyrus_core::ast::TypeParam {
                    name: param.name.clone(),
                    constraint: param
                        .constraint
                        .as_ref()
                        .map(|ty| expand_local_type(ty, program, visiting)),
                })
                .collect(),
            params: params
                .iter()
                .map(|ty| expand_local_type(ty, program, visiting))
                .collect(),
            ret: Box::new(expand_local_type(ret, program, visiting)),
        },
        Type::Object(fields) => Type::Object(fields.clone()),
        Type::Union(types) => Type::Union(
            types
                .iter()
                .map(|ty| expand_local_type(ty, program, visiting))
                .collect(),
        ),
        Type::Apply { name, args } if name == "HttpReply" => Type::Apply {
            name: name.clone(),
            args: args.clone(),
        },
        Type::Apply { name, args } => Type::Apply {
            name: name.clone(),
            args: args
                .iter()
                .map(|ty| expand_local_type(ty, program, visiting))
                .collect(),
        },
        Type::KeyOf(inner) => Type::KeyOf(Box::new(expand_local_type(inner, program, visiting))),
        Type::Conditional {
            check,
            bound,
            then,
            otherwise,
        } => Type::Conditional {
            check: Box::new(expand_local_type(check, program, visiting)),
            bound: Box::new(expand_local_type(bound, program, visiting)),
            then: Box::new(expand_local_type(then, program, visiting)),
            otherwise: Box::new(expand_local_type(otherwise, program, visiting)),
        },
        Type::Map(key, value) => Type::Map(
            Box::new(expand_local_type(key, program, visiting)),
            Box::new(expand_local_type(value, program, visiting)),
        ),
        Type::Set(inner) => Type::Set(Box::new(expand_local_type(inner, program, visiting))),
        Type::BinaryHeap(inner) => {
            Type::BinaryHeap(Box::new(expand_local_type(inner, program, visiting)))
        }
        Type::BinomialHeap(inner) => {
            Type::BinomialHeap(Box::new(expand_local_type(inner, program, visiting)))
        }
        Type::UnionFind(inner) => {
            Type::UnionFind(Box::new(expand_local_type(inner, program, visiting)))
        }
        _ => ty.clone(),
    }
}

fn lint_rules_with_analysis(
    source: &str,
    analysis: &DocumentAnalysis,
    config: &LintConfig,
) -> Vec<LintDiagnostic> {
    lint_rules_with_analysis_cancellable(source, analysis, config, None).unwrap_or_default()
}

fn lint_rules_with_analysis_cancellable(
    source: &str,
    analysis: &DocumentAnalysis,
    config: &LintConfig,
    cancellation: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
) -> Option<Vec<LintDiagnostic>> {
    if cancellation
        .as_ref()
        .is_some_and(|signal| signal.load(std::sync::atomic::Ordering::Acquire))
    {
        return None;
    }
    let tokenization_started = std::time::Instant::now();
    let used_cached_tokens = !analysis.tokens.is_empty();
    let tokens: Cow<'_, [SpannedToken]> = if analysis.tokens.is_empty() {
        Cow::Owned(lex(source))
    } else {
        Cow::Borrowed(&analysis.tokens)
    };
    if std::env::var_os("TYRUS_LSP_TRACE").is_some() {
        eprintln!(
            "tyrus-lint trace stage=tokenization source={} duration_ms={:.3} tokens={}",
            if used_cached_tokens {
                "analysis_cache"
            } else {
                "fallback_lex"
            },
            tokenization_started.elapsed().as_secs_f64() * 1000.0,
            tokens.len()
        );
    }
    if cancellation
        .as_ref()
        .is_some_and(|signal| signal.load(std::sync::atomic::Ordering::Acquire))
    {
        return None;
    }
    if let Some(program) = &analysis.program {
        lint_rules_with_program_cancellable(
            None,
            source,
            analysis,
            program,
            tokens.as_ref(),
            config,
            cancellation.as_deref(),
        )
    } else {
        let parse_tokens = tokens.iter().map(|token| token.token.clone()).collect();
        let parse_ranges = tokens.iter().map(|token| token.range.clone()).collect();
        let parser = Parser::new_with_ranges(parse_tokens, parse_ranges);
        let mut parser = if let Some(signal) = &cancellation {
            parser.with_cancellation(std::sync::Arc::clone(signal))
        } else {
            parser
        };
        let parsing_started = std::time::Instant::now();
        let Ok(program) = parser.parse_program() else {
            if std::env::var_os("TYRUS_LSP_TRACE").is_some() {
                eprintln!(
                    "tyrus-lint trace stage=fallback_parsing duration_ms={:.3} result=error",
                    parsing_started.elapsed().as_secs_f64() * 1000.0
                );
            }
            return if cancellation
                .as_ref()
                .is_some_and(|signal| signal.load(std::sync::atomic::Ordering::Acquire))
            {
                None
            } else {
                Some(Vec::new())
            };
        };
        if std::env::var_os("TYRUS_LSP_TRACE").is_some() {
            eprintln!(
                "tyrus-lint trace stage=fallback_parsing duration_ms={:.3} result=ok",
                parsing_started.elapsed().as_secs_f64() * 1000.0
            );
        }
        lint_rules_with_program_cancellable(
            None,
            source,
            analysis,
            &program,
            tokens.as_ref(),
            config,
            cancellation.as_deref(),
        )
    }
}

fn lint_rules_with_tokens(
    file: Option<&Path>,
    source: &str,
    analysis: &DocumentAnalysis,
    tokens: &[SpannedToken],
    config: &LintConfig,
) -> Vec<LintDiagnostic> {
    let parse_tokens = tokens.iter().map(|t| t.token.clone()).collect::<Vec<_>>();
    let parse_ranges = tokens.iter().map(|t| t.range.clone()).collect::<Vec<_>>();
    let Ok(program) = Parser::new_with_ranges(parse_tokens, parse_ranges).parse_program() else {
        return Vec::new();
    };
    lint_rules_with_program(file, source, analysis, &program, tokens, config)
}

fn lint_rules_with_program(
    file: Option<&Path>,
    source: &str,
    analysis: &DocumentAnalysis,
    program: &Program,
    tokens: &[SpannedToken],
    config: &LintConfig,
) -> Vec<LintDiagnostic> {
    lint_rules_with_program_cancellable(file, source, analysis, program, tokens, config, None)
        .unwrap_or_default()
}

fn lint_rules_with_program_cancellable(
    file: Option<&Path>,
    source: &str,
    analysis: &DocumentAnalysis,
    program: &Program,
    tokens: &[SpannedToken],
    config: &LintConfig,
    cancellation: Option<&std::sync::atomic::AtomicBool>,
) -> Option<Vec<LintDiagnostic>> {
    let context = LintContext {
        source,
        file,
        program,
        analysis,
        tokens,
        config,
    };

    let mut diagnostics = Vec::new();
    for rule in RULES.iter().filter(|rule| rule.implemented) {
        if cancellation.is_some_and(|signal| signal.load(std::sync::atomic::Ordering::Acquire)) {
            return None;
        }
        if !config.is_enabled(rule.name) {
            continue;
        }
        let severity = config.severity(rule.name);
        let started = std::time::Instant::now();
        let rule_diagnostics = rules::run_cancellable(rule.name, &context, cancellation);
        if cancellation.is_some_and(|signal| signal.load(std::sync::atomic::Ordering::Acquire)) {
            return None;
        }
        if std::env::var_os("TYRUS_LSP_TRACE").is_some() {
            eprintln!(
                "tyrus-lint trace rule={} duration_ms={:.3} diagnostics={}",
                rule.name,
                started.elapsed().as_secs_f64() * 1000.0,
                rule_diagnostics.len()
            );
        }
        for raw in rule_diagnostics {
            diagnostics.push(LintDiagnostic {
                code: rule.name.to_string(),
                severity,
                range: raw.range,
                message: raw.message,
                help: raw.help,
                note: raw.note,
                fix: raw.fix,
            });
        }
    }
    Some(diagnostics)
}

fn compiler_diagnostic(diagnostic: &AnalysisDiagnostic) -> LintDiagnostic {
    LintDiagnostic {
        code: diagnostic.code.clone(),
        severity: match diagnostic.severity {
            DiagnosticSeverity::Error => Severity::Error,
            DiagnosticSeverity::Warning => Severity::Warn,
        },
        range: diagnostic.range.clone(),
        message: diagnostic.message.clone(),
        help: diagnostic.help.first().cloned(),
        note: diagnostic
            .related
            .first()
            .map(|related| related.message.clone()),
        fix: None,
    }
}

fn is_lint_owned_analysis_diagnostic(diagnostic: &AnalysisDiagnostic) -> bool {
    diagnostic.code == tyrus_core::definite_assignment::USE_BEFORE_ASSIGN
        || diagnostic.code == "http-no-raw-status-code"
        || (diagnostic.code == tyrus_core::definite_assignment::NEVER_ASSIGNED
            && diagnostic
                .message
                .contains("declared but never assigned a value"))
}

pub struct FileReport {
    pub path: String,
    pub diagnostics: Vec<LintDiagnostic>,
}

pub fn lint_paths(
    dir: &Path,
    explicit: &[&str],
    config: &LintConfig,
) -> Result<Vec<FileReport>, String> {
    let env_schema = load_project_env_schema(dir);
    let files = if explicit.is_empty() {
        collect_tyrus_files(&dir.join("src"))
    } else {
        let mut files = Vec::new();
        for entry in explicit {
            let path = dir.join(entry);
            if path.is_dir() {
                files.extend(collect_tyrus_files(&path));
            } else if path.extension().is_some_and(|ext| ext == "tyr") {
                files.push(path);
            }
        }
        files
    };
    if files.is_empty() {
        return Err(format!(
            "no .tyr files found in `{}`",
            dir.join("src").display()
        ));
    }
    let mut reports = Vec::new();
    for file in files {
        let source = std::fs::read_to_string(&file)
            .map_err(|error| format!("could not read `{}`: {error}", file.display()))?;
        let diagnostics = lint_file_source(&file, &source, config, env_schema.as_ref());
        reports.push(FileReport {
            path: file.to_string_lossy().to_string(),
            diagnostics,
        });
    }
    Ok(reports)
}

fn load_project_env_schema(dir: &Path) -> Option<tyrus_core::env::EnvSchema> {
    let config = tyrus_core::project::load_project(dir).ok()?;
    tyrus_core::env::load_env_schema(dir, &config)
        .ok()
        .flatten()
        .map(|(schema, _diagnostics)| schema)
}

fn collect_tyrus_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_tyrus_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "tyr") {
            files.push(path);
        }
    }
    files.sort();
    files
}

pub fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(source.len());
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let column = source[line_start..offset].chars().count() + 1;
    (line, column)
}

pub fn render_pretty(path: &str, source: &str, diagnostic: &LintDiagnostic) -> String {
    let (line, column) = line_column(source, diagnostic.range.start);
    let offset = diagnostic.range.start.min(source.len());
    let line_start = source[..offset].rfind('\n').map_or(0, |index| index + 1);
    let line_text = source[line_start..].split('\n').next().unwrap_or("");
    let width = diagnostic
        .range
        .end
        .saturating_sub(diagnostic.range.start)
        .max(1);
    let gutter = format!("{line} ");
    let pad = " ".repeat(gutter.len());
    let mut out = format!(
        "{severity}[{code}]: {message}\n{pad}--> {path}:{line}:{column}\n{pad}|\n{gutter}| {line_text}\n{pad}| {caret}{underline}",
        severity = diagnostic.severity.label(),
        code = diagnostic.code,
        message = diagnostic.message,
        caret = " ".repeat(column.saturating_sub(1)),
        underline = "^".repeat(width),
    );
    if let Some(help) = &diagnostic.help {
        out.push_str(&format!("\n{pad}|\nhelp: {help}"));
    }
    if let Some(note) = &diagnostic.note {
        out.push_str(&format!("\nnote: {note}"));
    }
    out
}

pub fn render_json(reports: &[FileReport], sources: &[(String, String)]) -> String {
    let mut items = Vec::new();
    for report in reports {
        let source = sources
            .iter()
            .find(|(path, _)| path == &report.path)
            .map_or("", |(_, source)| source.as_str());
        for diagnostic in &report.diagnostics {
            let (line, column) = line_column(source, diagnostic.range.start);
            items.push(serde_json::json!({
                "code": diagnostic.code,
                "severity": severity_config_label(diagnostic.severity),
                "file": report.path,
                "line": line,
                "column": column,
                "message": diagnostic.message,
                "help": diagnostic.help,
                "note": diagnostic.note,
            }));
        }
    }
    serde_json::to_string_pretty(&serde_json::Value::Array(items)).unwrap_or_else(|_| "[]".into())
}

fn severity_config_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Off => "off",
        Severity::Info => "info",
        Severity::Warn => "warn",
        Severity::Error => "error",
    }
}

#[cfg(test)]
#[path = "tests/tests.rs"]
mod tests;
