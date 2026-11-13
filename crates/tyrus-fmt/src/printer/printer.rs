use std::collections::VecDeque;

use tyrus_core::ast::*;
use tyrus_core::lexer::{Token, lex_spanned};
use tyrus_core::span::Span;

use crate::config::{Config, PipelineChainLayout};

pub struct Printer {
    pub config: Config,
    source: String,
    buf: String,
    indent: usize,
    line_len: usize,
    readonly_keyword_allowed: bool,
    source_block_brace_newlines: VecDeque<bool>,
    source_continuation_newlines: VecDeque<bool>,
}

#[path = "expressions/expressions.rs"]
mod expressions;
#[path = "statements/statements.rs"]
mod statements;

impl Printer {
    pub fn new(config: Config, source: &str) -> Self {
        let (source_block_brace_newlines, source_continuation_newlines) =
            source_layout_newlines(source);
        Self {
            config,
            source: source.to_string(),
            buf: String::new(),
            indent: 0,
            line_len: 0,
            readonly_keyword_allowed: true,
            source_block_brace_newlines,
            source_continuation_newlines,
        }
    }

    fn fn_decl_written_as_function_keyword(&self, span: Span) -> bool {
        if span == Span::DUMMY {
            return false;
        }
        self.source
            .get(span.start..)
            .map(str::trim_start)
            .is_some_and(|rest| {
                rest.starts_with("validator function")
                    || rest.starts_with("function")
                    || rest.starts_with("async")
                    || rest.starts_with("comptime function")
            })
    }

    pub fn into_string(self) -> String {
        self.buf
    }

    fn indent_str(&self) -> String {
        self.config.indent_string()
    }

    fn push(&mut self, s: &str) {
        self.buf.push_str(s);
        self.line_len += s.len();
    }

    fn newline(&mut self) {
        self.buf.push('\n');
        self.line_len = 0;
        let indent = self.indent_str();
        for _ in 0..self.indent {
            self.buf.push_str(&indent);
            self.line_len += indent.len();
        }
    }

    fn newline_dedent(&mut self) {
        self.indent = self.indent.saturating_sub(1);
        self.newline();
    }

    fn newline_indent(&mut self) {
        self.indent += 1;
        self.newline();
    }

    fn push_space(&mut self) {
        if self.config.brace_spacing {
            self.push(" ");
        }
    }

    fn typed_numeric_source(&self, expr: &Expr) -> Option<String> {
        let source = self.source.get(expr.span.start..expr.span.end)?.trim();
        if source.contains(" as ") {
            return None;
        }
        [
            "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
            "f32", "f64", "f128", "d", "D",
        ]
        .iter()
        .any(|suffix| {
            source
                .strip_suffix(suffix)
                .is_some_and(|value| value.as_bytes().last().is_some_and(u8::is_ascii_digit))
        })
        .then_some(source.to_string())
    }

    fn brace_keyword_sep(&mut self) {
        match self.config.brace_style {
            crate::config::BraceStyle::Off => {
                if self
                    .source_continuation_newlines
                    .pop_front()
                    .unwrap_or(false)
                {
                    self.newline();
                } else {
                    self.push_space();
                }
            }
            crate::config::BraceStyle::SameLine => self.push_space(),
            crate::config::BraceStyle::NextLine => self.newline(),
        }
    }

    pub fn print_program(&mut self, program: &Program) {
        let mut first = true;
        let mut prev_was_lang = false;
        for import in &program.imports {
            if !first {
                self.newline();
                if prev_was_lang && !import.source.starts_with("tyrus:") && self.config.imports_sort
                {
                    self.newline();
                }
            }
            first = false;
            prev_was_lang = import.source.starts_with("tyrus:");
            self.print_import(import);
        }
        if !program.imports.is_empty() && !program.items.is_empty() {
            if matches!(self.config.blank_lines, crate::config::BlankLines::Collapse) {
                self.newline();
                self.newline();
            } else {
                self.newline();
                if self.config.imports_sort {
                    self.newline();
                }
            }
            first = true;
        }
        let mut prev_block = false;
        for item in &program.items {
            if !first {
                self.newline();
            }
            let curr_block = self.is_block_item(item);
            if !first && self.config.blank_lines_around_blocks && (curr_block || prev_block) {
                self.newline();
            }
            first = false;
            prev_block = curr_block;
            let item_name = match item {
                Item::FnDecl { name, .. }
                | Item::InterfaceDecl { name, .. }
                | Item::TypeAlias { name, .. }
                | Item::EnumDecl { name, .. }
                | Item::Stmt(Stmt {
                    kind: StmtKind::VarDecl { name, .. },
                    ..
                }) => Some(name),
                _ => None,
            };
            let inline_export = item_name.and_then(|name| {
                program.exports.iter().find_map(|export| match export {
                    ExportDecl::Named {
                        exported,
                        local,
                        inline: true,
                        visibility,
                        ..
                    } if local == name => Some((exported.as_str(), *visibility)),
                    _ => None,
                })
            });
            let exported = inline_export.is_some();
            let default_export =
                inline_export.is_some_and(|(exported, _)| exported == DEFAULT_EXPORT);
            let protect =
                inline_export.is_some_and(|(_, visibility)| visibility == Visibility::Protect);

            self.print_item(item, exported, protect, default_export);
        }
        self.print_export_clauses(program);
        if !first && self.config.final_newline {
            self.newline();
        }
    }

    fn print_import(&mut self, import: &ImportDecl) {
        let default_local = import
            .specifiers
            .iter()
            .find_map(|spec| (spec.kind == ImportKind::Default).then_some(spec.local.as_str()));
        let namespace_local = import
            .specifiers
            .iter()
            .find_map(|spec| (spec.kind == ImportKind::Namespace).then_some(spec.local.as_str()));
        let named = import
            .specifiers
            .iter()
            .filter(|spec| spec.kind == ImportKind::Named)
            .collect::<Vec<_>>();

        if default_local.is_some() || namespace_local.is_some() {
            self.push("import ");
            let mut first = true;
            if let Some(local) = default_local {
                self.push(local);
                first = false;
            }
            if let Some(local) = namespace_local {
                if !first {
                    self.push(", ");
                }
                self.push("* as ");
                self.push(local);
                first = false;
            }
            if !named.is_empty() {
                if !first {
                    self.push(", ");
                }
                self.push("{ ");
                for (index, spec) in named.iter().enumerate() {
                    if index > 0 {
                        self.push(", ");
                    }
                    self.push_import_specifier(spec);
                }
                self.push(" }");
            }
            self.push(" from \"");
            self.push(&import.source);
            self.push("\";");
            return;
        }

        let inline_len = self.import_inline_len(import);
        if self.will_exceed_width(inline_len) {
            self.push("import {");
            self.indent += 1;
            for spec in &import.specifiers {
                self.newline();
                if spec.imported == spec.local {
                    self.push(&spec.imported);
                } else {
                    self.push(&spec.imported);
                    self.push(" as ");
                    self.push(&spec.local);
                }
                self.push(",");
            }
            self.indent -= 1;
            self.newline();
            self.push("} from \"");
            self.push(&import.source);
            self.push("\";");
        } else {
            self.push("import { ");
            let mut first = true;
            for spec in &import.specifiers {
                if !first {
                    self.push(", ");
                }
                first = false;
                if spec.imported == spec.local {
                    self.push(&spec.imported);
                } else {
                    self.push(&spec.imported);
                    self.push(" as ");
                    self.push(&spec.local);
                }
            }
            self.push(" } from \"");
            self.push(&import.source);
            self.push("\";");
        }
    }

    fn push_import_specifier(&mut self, spec: &ImportSpecifier) {
        if spec.imported == spec.local {
            self.push(&spec.imported);
        } else {
            self.push(&spec.imported);
            self.push(" as ");
            self.push(&spec.local);
        }
    }

    fn print_export_clauses(&mut self, program: &Program) {
        enum Clause {
            List {
                source: Option<String>,
                protect: bool,
                entries: Vec<String>,
            },
            All(String),
            Default(String),
        }
        let mut clauses: Vec<Clause> = Vec::new();
        for export in &program.exports {
            let (source, protect, entry) = match export {
                ExportDecl::Named { inline: true, .. } => continue,
                ExportDecl::Named {
                    exported, local, ..
                } if exported == DEFAULT_EXPORT => {
                    clauses.push(Clause::Default(local.clone()));
                    continue;
                }
                ExportDecl::Named {
                    exported,
                    local,
                    visibility,
                    ..
                } => (
                    None,
                    *visibility == Visibility::Protect,
                    if exported == local {
                        local.clone()
                    } else {
                        format!("{local} as {exported}")
                    },
                ),
                ExportDecl::Re {
                    exported,
                    imported,
                    source,
                    ..
                } => (
                    Some(source.clone()),
                    false,
                    if exported == imported {
                        imported.clone()
                    } else {
                        format!("{imported} as {exported}")
                    },
                ),
                ExportDecl::All { source, .. } => {
                    clauses.push(Clause::All(source.clone()));
                    continue;
                }
            };
            match clauses.last_mut() {
                Some(Clause::List {
                    source: last,
                    protect: last_protect,
                    entries,
                }) if *last == source && *last_protect == protect => entries.push(entry),
                _ => clauses.push(Clause::List {
                    source,
                    protect,
                    entries: vec![entry],
                }),
            }
        }
        for clause in clauses {
            self.newline();
            match clause {
                Clause::All(source) => {
                    self.push("export * from \"");
                    self.push(&source);
                    self.push("\";");
                }
                Clause::Default(local) => {
                    self.push("export default ");
                    self.push(&local);
                    self.push(";");
                }
                Clause::List {
                    source,
                    protect,
                    entries,
                } => {
                    self.push(if protect {
                        "export protect { "
                    } else {
                        "export { "
                    });
                    self.push(&entries.join(", "));
                    self.push(" }");
                    if let Some(source) = source {
                        self.push(" from \"");
                        self.push(&source);
                        self.push("\"");
                    }
                    self.push(";");
                }
            }
        }
    }

    fn import_inline_len(&self, import: &ImportDecl) -> usize {
        let specs_len: usize = import
            .specifiers
            .iter()
            .map(|s| {
                if s.imported == s.local {
                    s.imported.len()
                } else {
                    s.imported.len() + 4 + s.local.len()
                }
            })
            .sum();
        let commas = 2 * import.specifiers.len().saturating_sub(1);
        21 + import.source.len() + specs_len + commas
    }

    fn print_item(&mut self, item: &Item, exported: bool, protect: bool, default_export: bool) {
        if exported {
            self.push("export ");
        }
        if protect {
            self.push("protect ");
        }
        if default_export {
            self.push("default ");
        }
        match item {
            Item::InterfaceDecl {
                name,
                type_params,
                extends,
                fields,
                from_type_alias,
                ..
            } => {
                if *from_type_alias {
                    self.push("type ");
                    self.push(name);
                    self.print_type_params(type_params);
                    self.push(" = ");
                    if extends.is_empty() {
                        self.print_field_block(fields);
                    } else {
                        for (index, operand) in extends.iter().enumerate() {
                            if index > 0 {
                                self.push(" & ");
                            }
                            self.print_type(operand);
                        }
                    }
                    self.push(";");
                } else {
                    self.print_interface(name, type_params, extends, fields);
                }
            }
            Item::BehaviorInterfaceDecl {
                name,
                type_params,
                targets,
                extends,
                methods,
                bindings,
                ..
            } => {
                self.push("interface ");
                self.push(name);
                self.print_type_params(type_params);
                self.push(" at ");
                for (index, target) in targets.iter().enumerate() {
                    if index > 0 {
                        self.push(", ");
                    }
                    self.print_type(target);
                }
                for (index, supertype) in extends.iter().enumerate() {
                    if index == 0 {
                        self.push(" extends ");
                    } else {
                        self.push(", ");
                    }
                    self.print_type(supertype);
                }
                self.push_space();
                self.push("{");
                for method in methods {
                    self.newline_indent();
                    self.push(&method.name);
                    self.push("(");
                    self.print_params(&method.params);
                    self.push(")");
                    self.print_type_annotation(&method.ret_type);
                    self.print_throws_clause(&method.throws);
                    self.push(" ");
                    self.open_block();
                    if !method.body.is_empty() {
                        self.print_body(&method.body);
                        self.newline();
                    }
                    self.push("}");
                }
                for binding in bindings {
                    self.newline_indent();
                    self.push(binding);
                    self.push(";");
                }
                if !methods.is_empty() {
                    self.newline_dedent();
                } else {
                    self.push_space();
                }
                self.push("}");
            }
            Item::EnumDecl {
                name,
                repr,
                members,
                ..
            } => {
                self.print_enum(name, repr, members);
            }
            Item::FnDecl {
                name,
                is_async,
                is_comptime,
                is_validator,
                type_params,
                params,
                ret_type,
                throws,
                body,
                span,
            } => {
                let anonymous_default_arrow =
                    default_export && name.starts_with("__tyrus_default_export_");
                let written_as_function = self.fn_decl_written_as_function_keyword(*span);
                let as_arrow = if *is_validator {
                    !written_as_function
                } else {
                    anonymous_default_arrow
                        || self.config.prefer_arrow_function
                        || !written_as_function
                };
                if as_arrow {
                    if *is_validator {
                        self.push("validator ");
                    }
                    if *is_comptime {
                        self.push("comptime ");
                    }
                    if !anonymous_default_arrow {
                        self.push("const ");
                        self.push(name);
                        self.push(" = ");
                    }
                    if *is_async {
                        self.push("async ");
                    }
                    self.print_type_params(type_params);
                    self.push("(");
                    self.print_params(params);
                    self.push(")");
                    self.print_type_annotation(ret_type);
                    self.print_throws_clause(throws);
                    self.push(" =>");
                    self.open_block();
                    if !body.is_empty() {
                        self.print_body(body);
                        self.newline();
                    }
                    self.push("}");
                    self.push(";");
                } else {
                    if *is_validator {
                        self.push("validator ");
                    }
                    if *is_async {
                        self.push("async ");
                    }
                    if *is_comptime {
                        self.push("comptime ");
                    }
                    self.push("function ");
                    self.push(name);
                    self.print_type_params(type_params);
                    self.push("(");
                    self.print_params(params);
                    self.push(")");
                    self.print_type_annotation(ret_type);
                    self.print_throws_clause(throws);
                    self.open_block();
                    if !body.is_empty() {
                        self.print_body(body);
                        self.newline();
                    }
                    self.push("}");
                }
            }
            Item::TypeAlias {
                name,
                type_params,
                ty,
                ..
            } => {
                self.push("type ");
                self.push(name);
                self.print_type_params(type_params);
                self.push(" =");
                if let Type::Union(variants) = ty {
                    let inline_len = type_inline_len(ty);
                    if self.will_exceed_width(inline_len) {
                        self.indent += 1;
                        for variant in variants {
                            self.newline();
                            self.push("| ");
                            self.print_type(variant);
                        }
                        self.indent -= 1;
                        self.push(";");
                        return;
                    }
                }
                self.push(" ");
                self.print_type(ty);
                self.push(";");
            }
            Item::Stmt(stmt) => {
                self.print_stmt(stmt, true);
            }
        }
    }

    fn print_interface(
        &mut self,
        name: &str,
        type_params: &[TypeParam],
        extends: &[Type],
        fields: &[Field],
    ) {
        self.push("interface ");
        self.push(name);
        self.print_type_params(type_params);
        if !extends.is_empty() {
            self.push(" extends ");
            for (index, supertype) in extends.iter().enumerate() {
                if index > 0 {
                    self.push(", ");
                }
                self.print_type(supertype);
            }
        }
        self.push_space();
        self.print_field_block(fields);
    }

    fn print_enum(&mut self, name: &str, repr: &Type, members: &[EnumMember]) {
        self.push("enum ");
        self.push(name);
        if !matches!(repr, Type::Void) {
            self.push(": ");
            self.print_type(repr);
        }
        self.push_space();
        self.push("{");
        if members.is_empty() {
            self.push(" }");
            return;
        }
        self.newline_indent();
        let mut first = true;
        for member in members {
            if !first {
                self.push(",");
                self.newline();
            }
            first = false;
            self.push(&member.name);
            match &member.payload {
                EnumPayload::Unit => {}
                EnumPayload::Tuple(types) => {
                    self.push("(");
                    for (index, ty) in types.iter().enumerate() {
                        if index > 0 {
                            self.push(", ");
                        }
                        self.print_type(ty);
                    }
                    self.push(")");
                }
                EnumPayload::Struct(fields) => {
                    self.push(" { ");
                    for (index, field) in fields.iter().enumerate() {
                        if index > 0 {
                            self.push(", ");
                        }
                        self.push(&field.name);
                        self.push(": ");
                        self.print_type(&field.ty);
                    }
                    self.push(" }");
                }
            }
            if let Some(value) = &member.value {
                self.push(" = ");
                self.print_expr(value);
            }
        }
        self.push(",");
        self.newline_dedent();
        self.push("}");
    }

    #[allow(clippy::too_many_lines)]
    fn print_type(&mut self, ty: &Type) {
        match ty {
            Type::Infer => {}
            Type::I8 => self.push("i8"),
            Type::I16 => self.push("i16"),
            Type::I32 => self.push("i32"),
            Type::I64 => self.push("i64"),
            Type::I128 => self.push("i128"),
            Type::Isize => self.push("isize"),
            Type::U8 => self.push("u8"),
            Type::U16 => self.push("u16"),
            Type::U32 => self.push("u32"),
            Type::U64 => self.push("u64"),
            Type::U128 => self.push("u128"),
            Type::Usize => self.push("usize"),
            Type::F32 => self.push("f32"),
            Type::F64 => self.push("f64"),
            Type::F128 => self.push("f128"),
            Type::Decimal => self.push("decimal"),
            Type::Bool => self.push("boolean"),
            Type::Char => self.push("char"),
            Type::Regex => self.push("Regex"),
            Type::Str => self.push("string"),
            Type::Void => self.push("void"),
            Type::Named(name) | Type::Generic(name) => self.push(name),
            Type::Array(element) => {
                self.print_type(element);
                self.push("[]");
            }
            Type::Tuple(elements) => {
                self.push("[");
                for (index, element) in elements.iter().enumerate() {
                    if index > 0 {
                        self.push(", ");
                    }
                    self.print_type(element);
                }
                self.push("]");
            }
            Type::Readonly(inner) => {
                self.push("readonly ");
                self.print_type(inner);
            }
            Type::Ref(t) => {
                self.push("&");
                self.print_type(t);
            }
            Type::MutRef(t) => {
                self.push("&mut ");
                self.print_type(t);
            }
            Type::Optional(t) => {
                self.print_type(t);
                self.push("?");
            }
            Type::Result { success, error } => {
                self.push("Result<");
                self.print_type(success);
                self.push(", ");
                self.print_type(error);
                self.push(">");
            }
            Type::Throws { function, effects } => {
                self.print_type(function);
                self.push(" throws ");
                for (index, effect) in effects.iter().enumerate() {
                    if index > 0 {
                        self.push(" | ");
                    }
                    self.print_type(effect);
                }
            }
            Type::Function { params, ret } => {
                self.push("(");
                let mut first = true;
                for param in params {
                    if !first {
                        self.push(", ");
                    }
                    first = false;
                    self.print_type(param);
                }
                self.push(") => ");
                self.print_type(ret);
            }
            Type::Object(fields) => {
                self.push("{ ");
                let mut first = true;
                for field in fields {
                    if !first {
                        self.push("; ");
                    }
                    first = false;
                    for decorator in &field.decorators {
                        self.print_decorator(decorator);
                        self.push(" ");
                    }
                    self.push(&field.name);
                    if field.optional {
                        self.push("?");
                    }
                    self.push(": ");
                    self.print_type(&field.ty);
                }
                self.push(" }");
            }
            Type::Union(variants) => {
                let mut first = true;
                for variant in variants {
                    if !first {
                        self.push(" | ");
                    }
                    first = false;
                    self.print_type(variant);
                }
            }
            Type::GenericFunction {
                type_params,
                params,
                ret,
            } => {
                self.print_type_params_list(type_params);
                self.push("(");
                let mut first = true;
                for param in params {
                    if !first {
                        self.push(", ");
                    }
                    first = false;
                    self.print_type(param);
                }
                self.push(") => ");
                self.print_type(ret);
            }
            Type::Unknown => self.push("unknown"),
            Type::Never => self.push("never"),
            Type::Lit(expr) => self.push(&lit_display(expr)),
            Type::TypeQuery(expr) => {
                self.push("typeof ");
                self.print_expr(expr);
            }
            Type::Apply { name, args } => {
                self.push(name);
                self.push("<");
                for (index, arg) in args.iter().enumerate() {
                    if index > 0 {
                        self.push(", ");
                    }
                    self.print_type(arg);
                }
                self.push(">");
            }
            Type::KeyOf(inner) => {
                self.push("keyof ");
                self.print_type(inner);
            }
            Type::Conditional {
                check,
                bound,
                then,
                otherwise,
            } => {
                self.print_type(check);
                self.push(" extends ");
                self.print_type(bound);
                self.push(" ? ");
                self.print_type(then);
                self.push(" : ");
                self.print_type(otherwise);
            }
            Type::InferVar(name) => {
                self.push("infer ");
                self.push(name);
            }
            Type::Map(key, value) => {
                self.push("Record<");
                self.print_type(key);
                self.push(", ");
                self.print_type(value);
                self.push(">");
            }
            Type::Set(value) => {
                self.push("Set<");
                self.print_type(value);
                self.push(">");
            }
            Type::BinaryHeap(value) => {
                self.push("BinaryHeap<");
                self.print_type(value);
                self.push(">");
            }
            Type::BinomialHeap(value) => {
                self.push("BinomialHeap<");
                self.print_type(value);
                self.push(">");
            }
            Type::UnionFind(value) => {
                self.push("UnionFind<");
                self.print_type(value);
                self.push(">");
            }
            Type::Counter(value) => {
                self.push("Counter<");
                self.print_type(value);
                self.push(">");
            }
            Type::Deque(value) => {
                self.push("Deque<");
                self.print_type(value);
                self.push(">");
            }
            Type::FenwickTree => {
                self.push("FenwickTree");
            }
            Type::BubbleSort => {
                self.push("BubbleSort");
            }
            Type::SegmentTree => {
                self.push("SegmentTree");
            }
            Type::SparseTable => {
                self.push("SparseTable");
            }
            Type::SegmentTreeLazy => {
                self.push("SegmentTreeLazy");
            }
            Type::BitSet => {
                self.push("BitSet");
            }
            Type::Trie => {
                self.push("Trie");
            }
            Type::SuffixTree => {
                self.push("SuffixTree");
            }
            Type::Lca => {
                self.push("Lca");
            }
            Type::Hld => {
                self.push("Hld");
            }
            Type::Centroid => {
                self.push("Centroid");
            }
            Type::BloomFilter => {
                self.push("BloomFilter");
            }
            Type::LinkedList(value) => {
                self.push("LinkedList<");
                self.print_type(value);
                self.push(">");
            }
            Type::DoubleLinkedList(value) => {
                self.push("DoubleLinkedList<");
                self.print_type(value);
                self.push(">");
            }
            Type::AvlTree(value) => {
                self.push("AvlTree<");
                self.print_type(value);
                self.push(">");
            }
            Type::RedBlackTree(value) => {
                self.push("RedBlackTree<");
                self.print_type(value);
                self.push(">");
            }
            Type::SplayTree(value) => {
                self.push("SplayTree<");
                self.print_type(value);
                self.push(">");
            }
            Type::Treap(value) => {
                self.push("Treap<");
                self.print_type(value);
                self.push(">");
            }
            Type::SortedSet(value) => {
                self.push("SortedSet<");
                self.print_type(value);
                self.push(">");
            }
            Type::SkipList(value) => {
                self.push("SkipList<");
                self.print_type(value);
                self.push(">");
            }
            Type::SortedMap(key, value) => {
                self.push("SortedMap<");
                self.print_type(key);
                self.push(", ");
                self.print_type(value);
                self.push(">");
            }
            Type::Graph(node, weight) => {
                self.push("Graph<");
                self.print_type(node);
                self.push(", ");
                self.print_type(weight);
                self.push(">");
            }
            Type::Range(element) => {
                self.push("Range<");
                self.print_type(element);
                self.push(">");
            }
        }
    }

    fn print_type_params(&mut self, type_params: &[TypeParam]) {
        if type_params.is_empty() {
            return;
        }
        self.push("<");
        let mut first = true;
        for tp in type_params {
            if !first {
                self.push(", ");
            }
            first = false;
            self.push(&tp.name);
            if let Some(constraint) = &tp.constraint {
                self.push(" extends ");
                self.print_type(constraint);
            }
        }
        self.push(">");
    }

    fn print_type_params_list(&mut self, type_params: &[TypeParam]) {
        if type_params.is_empty() {
            return;
        }
        self.push("<");
        let mut first = true;
        for tp in type_params {
            if !first {
                self.push(", ");
            }
            first = false;
            self.push(&tp.name);
            if let Some(constraint) = &tp.constraint {
                self.push(" extends ");
                self.print_type(constraint);
            }
        }
        self.push(">");
    }

    fn print_type_annotation(&mut self, ty: &Type) {
        if !matches!(ty, Type::Infer) {
            self.push(": ");
            self.print_type(ty);
        }
    }

    fn print_throws_clause(&mut self, throws: &[Type]) {
        if throws.is_empty() {
            return;
        }
        self.push(" throws ");
        for (index, ty) in throws.iter().enumerate() {
            if index > 0 {
                self.push(" | ");
            }
            self.print_type(ty);
        }
    }

    fn print_params(&mut self, params: &[Param]) {
        let mut first = true;
        for param in params {
            if !first {
                self.push(", ");
            }
            first = false;
            for decorator in &param.decorators {
                self.print_decorator(decorator);
                self.push(" ");
            }
            if param.rest {
                self.push("...");
            }
            if param.mutable {
                self.push("mut ");
            }
            match &param.pattern {
                ParamPattern::Name(n) => self.push(n),
                ParamPattern::Array(names) => {
                    self.push("[");
                    for (i, n) in names.iter().enumerate() {
                        if i > 0 {
                            self.push(", ");
                        }
                        self.push(n);
                    }
                    self.push("]");
                }
                ParamPattern::Object(names) => {
                    self.push("{");
                    for (i, n) in names.iter().enumerate() {
                        if i > 0 {
                            self.push(", ");
                        }
                        self.push(n);
                    }
                    self.push("}");
                }
            }
            if param.optional {
                self.push("?");
            }
            self.print_type_annotation(&param.ty);
            if let Some(default) = &param.default_value {
                self.push(" = ");
                if !self.print_char_literal(&param.ty, default) {
                    self.print_expr(default);
                }
            }
        }
    }

    fn print_decorator(&mut self, decorator: &ValidatorDecorator) {
        self.push("@");
        self.push(&decorator.name);
        self.push("(");
        for (index, arg) in decorator.args.iter().enumerate() {
            if index > 0 {
                self.push(", ");
            }
            self.print_expr(arg);
        }
        self.push(")");
    }

    fn value_renders_multiline(&mut self, value: &Expr, prefix_len: usize) -> bool {
        let saved = self.line_len;
        self.line_len += prefix_len;
        let result = match value.kind() {
            ExprKind::ObjectLit(fields) => self.should_expand_object(fields),
            ExprKind::ArrayLit(elements) => self.should_expand_array(elements),
            _ => false,
        };
        self.line_len = saved;
        result
    }

    fn print_body(&mut self, stmts: &[Stmt]) {
        if stmts.is_empty() {
            return;
        }
        self.indent += 1;
        for (i, stmt) in stmts.iter().enumerate() {
            self.newline();

            if self.config.blank_lines_around_blocks && i > 0 {
                let prev_is_block = self.is_block_stmt(&stmts[i - 1]);
                let curr_is_block = self.is_block_stmt(stmt);
                if prev_is_block || curr_is_block {
                    self.newline();
                }
            }

            self.print_stmt(stmt, true);
        }
        self.indent -= 1;
    }

    fn assign_op_str(op: &AssignOp) -> &'static str {
        match op {
            AssignOp::Assign => " = ",
            AssignOp::AddAssign => " += ",
            AssignOp::SubAssign => " -= ",
            AssignOp::MulAssign => " *= ",
            AssignOp::DivAssign => " /= ",
            AssignOp::ModAssign => " %= ",
            AssignOp::PowAssign => " **= ",
            AssignOp::AndAssign => " &&= ",
            AssignOp::OrAssign => " ||= ",
            AssignOp::NullCoalesceAssign => " ??= ",
            AssignOp::BitAndAssign => " &= ",
            AssignOp::BitOrAssign => " |= ",
            AssignOp::BitXorAssign => " ^= ",
            AssignOp::ShlAssign => " <<= ",
            AssignOp::ShrAssign => " >>= ",
            AssignOp::UShrAssign => " >>>= ",
        }
    }

    fn print_ternary_chain(&mut self, then_expr: &Expr, else_expr: &Expr) {
        self.newline();
        self.push("? ");
        self.print_expr(then_expr);
        match else_expr.kind() {
            ExprKind::Ternary {
                cond,
                then_expr: t,
                else_expr: e,
            } => {
                self.newline();
                self.push(": ");
                self.print_expr(cond);
                self.print_ternary_chain(t, e);
            }
            _ => {
                self.push(" : ");
                self.print_expr(else_expr);
            }
        }
    }

    fn print_pipeline(&mut self, left: &Expr, right: &Expr) {
        let mut parts = Vec::new();
        collect_pipeline_parts(left, &mut parts);
        parts.push(right);

        let preserve_inline = matches!(self.config.pipeline_chain_layout, PipelineChainLayout::Off)
            && !self.pipeline_was_multiline(left, right);
        if parts.len() <= 2
            || matches!(
                self.config.pipeline_chain_layout,
                PipelineChainLayout::Inline
            )
            || preserve_inline
        {
            self.print_expr(left);
            self.push(" |> ");
            self.print_expr(right);
            return;
        }

        self.print_expr(parts[0]);
        self.indent += 1;
        for part in parts.iter().skip(1) {
            self.newline();
            self.push("|> ");
            self.print_expr(part);
        }
        self.indent -= 1;
    }

    fn pipeline_was_multiline(&self, left: &Expr, right: &Expr) -> bool {
        self.source
            .get(left.span.start..right.span.end)
            .is_some_and(|source| {
                source.contains("|>\n")
                    || source
                        .lines()
                        .skip(1)
                        .any(|line| line.trim_start().starts_with("|>"))
            })
    }

    fn print_str_lit(&mut self, expr: &Expr, s: &str) {
        if matches!(self.config.quote_style, crate::config::QuoteStyle::Off)
            && let Some(source) = self.source.get(expr.span.start..expr.span.end)
            && matches!(source.as_bytes().first(), Some(b'\'' | b'"'))
            && source.as_bytes().first() == source.as_bytes().last()
        {
            let source = source.to_string();
            self.push(&source);
            return;
        }
        let quote = match self.config.quote_style {
            crate::config::QuoteStyle::Single => '\'',
            crate::config::QuoteStyle::Double | crate::config::QuoteStyle::Off => '"',
        };
        self.push(&encode_string(s, quote));
    }

    fn print_args_or_list(&mut self, args: &[Expr]) {
        if self.should_expand_args(args) {
            self.newline_indent();
            let mut first = true;
            for arg in args {
                if !first {
                    self.push(",");
                    self.newline();
                }
                first = false;
                self.print_expr(arg);
            }
            if self.has_trailing_comma() {
                self.push(",");
            }
            self.newline_dedent();
            self.push(")");
        } else {
            self.print_expr_list(args);
            self.push(")");
        }
    }

    fn print_binop(&mut self, op: &BinOp, left: &Expr, right: &Expr) {
        let prec = binop_precedence(op);
        let left_paren = needs_parens_left(left, prec);
        let right_paren = needs_parens_right(right, prec, is_right_assoc(op));

        let op_str = binop_str(op);
        let right_extra = if right_paren {
            1 + expr_inline_len(right) + 1
        } else {
            expr_inline_len(right)
        };
        let left_extra = if left_paren {
            1 + expr_inline_len(left) + 1
        } else {
            expr_inline_len(left)
        };
        let total_extra = left_extra + 1 + op_str.len() + 1 + right_extra;

        if total_extra > 30 && self.will_exceed_width(total_extra) {
            if left_paren {
                self.push("(");
            }
            self.print_expr(left);
            if left_paren {
                self.push(")");
            }
            self.push(" ");
            self.push(op_str);
            self.indent += 1;
            self.newline();
            if right_paren {
                self.push("(");
            }
            self.print_expr(right);
            if right_paren {
                self.push(")");
            }
            self.indent -= 1;
        } else {
            if left_paren {
                self.push("(");
            }
            self.print_expr(left);
            if left_paren {
                self.push(")");
            }

            self.push(" ");
            self.push(op_str);
            self.push(" ");

            if right_paren {
                self.push("(");
            }
            self.print_expr(right);
            if right_paren {
                self.push(")");
            }
        }
    }

    fn print_chained_comparison(&mut self, terms: &[Expr], ops: &[BinOp]) {
        const CHAIN_PREC: u8 = 7;
        let first_paren = needs_parens_left(&terms[0], CHAIN_PREC);
        if first_paren {
            self.push("(");
        }
        self.print_expr(&terms[0]);
        if first_paren {
            self.push(")");
        }
        for (op, term) in ops.iter().zip(&terms[1..]) {
            self.push(" ");
            self.push(binop_str(op));
            self.push(" ");
            let paren = needs_parens_left(term, CHAIN_PREC);
            if paren {
                self.push("(");
            }
            self.print_expr(term);
            if paren {
                self.push(")");
            }
        }
    }

    fn has_trailing_comma(&self) -> bool {
        self.config.trailing_comma
    }

    fn will_exceed_width(&self, extra: usize) -> bool {
        self.line_len + extra > self.config.line_width as usize
    }

    fn should_expand_args(&self, args: &[Expr]) -> bool {
        let args_len = inline_expr_list_len(args);
        if args.len() <= 1 {
            return self.will_exceed_width(1 + args_len + 1);
        }
        self.will_exceed_width(1 + args_len + 1)
    }
}

fn binop_str(op: &BinOp) -> &str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Pow => "**",
        BinOp::Eq => "===",
        BinOp::Neq => "!==",
        BinOp::Gt => ">",
        BinOp::Lt => "<",
        BinOp::Gte => ">=",
        BinOp::Lte => "<=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::NullCoalesce => "??",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
        BinOp::UShr => ">>>",
    }
}

fn binop_precedence(op: &BinOp) -> u8 {
    match op {
        BinOp::NullCoalesce => 1,
        BinOp::Or => 2,
        BinOp::And => 3,
        BinOp::BitOr => 4,
        BinOp::BitXor => 5,
        BinOp::BitAnd => 6,
        BinOp::Eq | BinOp::Neq => 7,
        BinOp::Gt | BinOp::Lt | BinOp::Gte | BinOp::Lte => 8,
        BinOp::Shl | BinOp::Shr | BinOp::UShr => 9,
        BinOp::Add | BinOp::Sub => 10,
        BinOp::Mul | BinOp::Div | BinOp::Mod => 11,
        BinOp::Pow => 12,
    }
}

fn is_right_assoc(op: &BinOp) -> bool {
    matches!(op, BinOp::Pow)
}

fn expr_precedence(expr: &Expr) -> Option<u8> {
    match expr.kind() {
        ExprKind::BinOp { op, .. } => Some(binop_precedence(op)),
        ExprKind::ChainedComparison { .. } => Some(7),
        _ => None,
    }
}

fn needs_parens_left(expr: &Expr, parent_prec: u8) -> bool {
    match expr_precedence(expr) {
        Some(prec) => parent_prec >= prec,
        None => false,
    }
}

fn needs_parens_right(expr: &Expr, parent_prec: u8, right_assoc: bool) -> bool {
    match expr_precedence(expr) {
        Some(prec) => {
            if right_assoc {
                parent_prec > prec
            } else {
                parent_prec >= prec
            }
        }
        None => false,
    }
}

fn collect_pipeline_parts<'a>(expr: &'a Expr, parts: &mut Vec<&'a Expr>) {
    if let ExprKind::Pipeline { left, right } = expr.kind() {
        collect_pipeline_parts(left, parts);
        parts.push(right);
    } else {
        parts.push(expr);
    }
}

fn call_type_args_inline_len(type_args: &[Type]) -> usize {
    if type_args.is_empty() {
        0
    } else {
        2 + type_args.iter().map(type_inline_len).sum::<usize>() + 2 * (type_args.len() - 1)
    }
}

fn expr_inline_len(expr: &Expr) -> usize {
    match expr.kind() {
        ExprKind::StrLit(s) => s.len() + 2,
        ExprKind::RegexLit { pattern, flags } => pattern.len() + flags.len() + 2,
        ExprKind::NewSet { ty, initial } => {
            let type_len = ty.as_ref().map_or(0, |ty| 2 + type_inline_len(ty));
            let inner = initial.as_deref().map_or(0, expr_inline_len);
            "Set()".len() + type_len + inner
        }
        ExprKind::NewMap {
            key,
            value,
            initial,
        } => {
            let type_len = match (key, value) {
                (Some(key), Some(value)) => 4 + type_inline_len(key) + type_inline_len(value),
                _ => 0,
            };
            let inner = initial.as_deref().map_or(0, expr_inline_len);
            "Map()".len() + type_len + inner
        }
        ExprKind::TemplateLit(parts) => {
            let inner: usize = parts
                .iter()
                .map(|p| match p {
                    TemplatePart::Text(t) => t.len(),
                    TemplatePart::Expr(e, spec) => {
                        2 + expr_inline_len(e) + spec.map_or(0, |spec| 1 + spec.as_written().len())
                    }
                })
                .sum();
            inner + 2
        }
        ExprKind::IntLit(n) => {
            let s = n.to_string();
            s.len()
        }
        ExprKind::UIntLit(n) => n.to_string().len(),
        ExprKind::FloatLit(n) => format(n).len(),
        ExprKind::DecimalLit(n) => n.text().len() + 1,
        ExprKind::BoolLit(true) => 4,
        ExprKind::BoolLit(false) => 5,
        ExprKind::Null => 4,
        ExprKind::None => 4,
        ExprKind::Topic => 1,
        ExprKind::ObjectLit(fields) => {
            if fields.is_empty() {
                return 2;
            }
            let inner: usize = fields
                .iter()
                .map(|(k, v)| k.len() + 2 + expr_inline_len(v))
                .sum();
            let commas = 2 * (fields.len().saturating_sub(1));
            2 + inner + commas + 2
        }
        ExprKind::ObjectUpdate { base, fields } => {
            let fields_len: usize = fields
                .iter()
                .map(|(key, value)| key.len() + 2 + expr_inline_len(value))
                .sum();
            6 + expr_inline_len(base) + fields_len + 2 * fields.len() + 2
        }
        ExprKind::ArrayLit(elements) => {
            if elements.is_empty() {
                return 2;
            }
            let inner: usize = elements.iter().map(expr_inline_len).sum();
            let commas = 2 * (elements.len().saturating_sub(1));
            2 + inner + commas + 2
        }
        ExprKind::ArrayRepeat { value, count } => {
            1 + expr_inline_len(value) + 2 + expr_inline_len(count) + 1
        }
        ExprKind::Ident(s) => s.len(),
        ExprKind::FieldAccess { object, field } => expr_inline_len(object) + 1 + field.len(),
        ExprKind::OptionalFieldAccess { object, field } => {
            expr_inline_len(object) + 2 + field.len()
        }
        ExprKind::IndexAccess { object, index } => {
            expr_inline_len(object) + 1 + expr_inline_len(index) + 1
        }
        ExprKind::BinOp { left, right, op } => {
            let op_str = binop_str(op);
            expr_inline_len(left) + 1 + op_str.len() + 1 + expr_inline_len(right)
        }
        ExprKind::ChainedComparison { terms, ops } => {
            let terms_len: usize = terms.iter().map(expr_inline_len).sum();
            let ops_len: usize = ops.iter().map(|op| binop_str(op).len() + 2).sum();
            terms_len + ops_len
        }
        ExprKind::Pipeline { left, right } => expr_inline_len(left) + 4 + expr_inline_len(right),
        ExprKind::Not(inner) => 1 + expr_inline_len(inner),
        ExprKind::Neg(inner) => 1 + expr_inline_len(inner),
        ExprKind::BitNot(inner) => 1 + expr_inline_len(inner),
        ExprKind::Typeof(inner) => 7 + expr_inline_len(inner),
        ExprKind::Await(inner) => 6 + expr_inline_len(inner),
        ExprKind::Try(inner) => expr_inline_len(inner) + 1,
        ExprKind::PrefixIncr(inner) => 2 + expr_inline_len(inner),
        ExprKind::PrefixDecr(inner) => 2 + expr_inline_len(inner),
        ExprKind::PostfixIncr(inner) => expr_inline_len(inner) + 2,
        ExprKind::PostfixDecr(inner) => expr_inline_len(inner) + 2,
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            expr_inline_len(cond) + 3 + expr_inline_len(then_expr) + 3 + expr_inline_len(else_expr)
        }
        ExprKind::Borrow {
            mutable,
            name,
            place,
        } => {
            let prefix = if *mutable { 5 } else { 1 };
            prefix
                + match place {
                    Some(place) => expr_inline_len(place),
                    None => name.len(),
                }
        }
        ExprKind::Unwrap(inner) => expr_inline_len(inner) + 1,
        ExprKind::OptionalMethodCall {
            object,
            method,
            type_args,
            args,
            ..
        } => {
            expr_inline_len(object)
                + 2
                + method.len()
                + call_type_args_inline_len(type_args)
                + 1
                + inline_expr_list_len(args)
                + 1
        }
        ExprKind::MethodCall {
            object,
            method,
            type_args,
            args,
            ..
        } => {
            expr_inline_len(object)
                + 1
                + method.len()
                + call_type_args_inline_len(type_args)
                + 1
                + inline_expr_list_len(args)
                + 1
        }
        ExprKind::FnCall {
            callee,
            type_args,
            args,
            ..
        } => {
            let type_args_len = if type_args.is_empty() {
                0
            } else {
                let inner: usize = type_args.iter().map(type_inline_len).sum();
                let commas = 2 * (type_args.len() - 1);
                1 + inner + commas + 1
            };
            callee.len() + type_args_len + 1 + inline_expr_list_len(args) + 1
        }
        ExprKind::CallValue { callee, args, .. } => {
            expr_inline_len(callee) + 1 + inline_expr_list_len(args) + 1
        }
        ExprKind::Spread(inner) => 3 + expr_inline_len(inner),
        ExprKind::TypeAssert { value, ty } => 4 + expr_inline_len(value) + type_inline_len(ty),
        ExprKind::ConstAssert(inner) => expr_inline_len(inner) + 9,
        ExprKind::Comptime(inner) => expr_inline_len(inner) + 10,
        ExprKind::In { key, object } => expr_inline_len(key) + 4 + expr_inline_len(object),
        ExprKind::Is { expr, ty } => expr_inline_len(expr) + 4 + type_inline_len(ty),
        ExprKind::Range {
            start,
            end,
            inclusive,
        } => expr_inline_len(start) + expr_inline_len(end) + if *inclusive { 3 } else { 2 },
        ExprKind::ArrowFn { .. } | ExprKind::If { .. } | ExprKind::Loop { .. } => 30,
    }
}

fn inline_expr_list_len(args: &[Expr]) -> usize {
    if args.is_empty() {
        return 0;
    }
    args.iter().map(expr_inline_len).sum::<usize>() + 2 * (args.len() - 1)
}

fn type_inline_len(ty: &Type) -> usize {
    match ty {
        Type::Infer => 10,
        Type::I8 => 2,
        Type::I16 => 3,
        Type::I32 => 3,
        Type::I64 => 3,
        Type::I128 => 4,
        Type::Isize => 5,
        Type::U8 => 2,
        Type::U16 => 3,
        Type::U32 => 3,
        Type::U64 => 3,
        Type::U128 => 4,
        Type::Usize => 5,
        Type::F32 => 3,
        Type::F64 => 3,
        Type::F128 => 4,
        Type::Decimal => 7,
        Type::Bool => 7,
        Type::Char => 4,
        Type::Regex => 6,
        Type::Str => 6,
        Type::Void => 4,
        Type::Named(s) | Type::Generic(s) => s.len(),
        Type::Array(element) => type_inline_len(element) + 2,
        Type::Tuple(elements) => {
            elements.iter().map(type_inline_len).sum::<usize>()
                + elements.len().saturating_sub(1) * 2
                + 2
        }
        Type::Readonly(inner) => 9 + type_inline_len(inner),
        Type::Ref(t) => 1 + type_inline_len(t),
        Type::MutRef(t) => 5 + type_inline_len(t),
        Type::Optional(t) => type_inline_len(t) + 1,
        Type::Throws { function, effects } => {
            type_inline_len(function)
                + " throws ".len()
                + effects.iter().map(type_inline_len).sum::<usize>()
                + effects.len().saturating_sub(1) * " | ".len()
        }
        Type::Function { params, ret } => {
            let inner: usize = params.iter().map(type_inline_len).sum();
            let commas = 2 * (params.len().saturating_sub(1));
            inner + commas + 4 + type_inline_len(ret)
        }
        Type::GenericFunction {
            type_params,
            params,
            ret,
        } => {
            let tps: usize = type_params.iter().map(|tp| tp.name.len()).sum();
            let tp_commas = 2 * (type_params.len().saturating_sub(1));
            let inner: usize = params.iter().map(type_inline_len).sum();
            let commas = 2 * (params.len().saturating_sub(1));
            1 + tps + tp_commas + 1 + inner + commas + 4 + type_inline_len(ret)
        }
        Type::Object(fields) => {
            fields
                .iter()
                .map(|f| f.name.len() + 2 + type_inline_len(&f.ty))
                .sum::<usize>()
                + 4
        }
        Type::Union(variants) => {
            variants.iter().map(type_inline_len).sum::<usize>()
                + 3 * (variants.len().saturating_sub(1))
        }
        Type::Unknown => 7,
        Type::Never => 5,
        Type::Lit(expr) => lit_display(expr).len(),
        Type::TypeQuery(expr) => 7 + expr_inline_len(expr),
        Type::Apply { name, args } => {
            let inner: usize = args.iter().map(type_inline_len).sum();
            name.len() + 2 + inner + 2 * args.len().saturating_sub(1)
        }
        Type::KeyOf(inner) => 6 + type_inline_len(inner),
        Type::Conditional {
            check,
            bound,
            then,
            otherwise,
        } => {
            type_inline_len(check)
                + type_inline_len(bound)
                + type_inline_len(then)
                + type_inline_len(otherwise)
                + 15
        }
        Type::InferVar(name) => 6 + name.len(),
        Type::Map(key, value) => 10 + type_inline_len(key) + type_inline_len(value),
        Type::Set(value) => 6 + type_inline_len(value),
        Type::BinaryHeap(value) => 13 + type_inline_len(value),
        Type::BinomialHeap(value) => 14 + type_inline_len(value),
        Type::UnionFind(value) => 11 + type_inline_len(value),
        Type::Counter(value) => 9 + type_inline_len(value),
        Type::Deque(value) => 7 + type_inline_len(value),
        Type::FenwickTree => 11,
        Type::BubbleSort => 10,
        Type::SegmentTree => 12,
        Type::SparseTable => 12,
        Type::SegmentTreeLazy => 16,
        Type::BitSet => 6,
        Type::Trie => 4,
        Type::BloomFilter => 12,
        Type::SuffixTree => 11,
        Type::Lca => 3,
        Type::Hld => 3,
        Type::Centroid => 8,
        Type::LinkedList(value) => 13 + type_inline_len(value),
        Type::DoubleLinkedList(value) => 20 + type_inline_len(value),
        Type::AvlTree(value) => 10 + type_inline_len(value),
        Type::RedBlackTree(value) => 15 + type_inline_len(value),
        Type::SplayTree(value) => 11 + type_inline_len(value),
        Type::Treap(value) => 7 + type_inline_len(value),
        Type::SortedSet(value) => 12 + type_inline_len(value),
        Type::SkipList(value) => 11 + type_inline_len(value),
        Type::SortedMap(key, value) => 12 + type_inline_len(key) + 2 + type_inline_len(value),
        Type::Graph(node, weight) => 7 + type_inline_len(node) + 2 + type_inline_len(weight),
        Type::Range(element) => 8 + type_inline_len(element),
        Type::Result { success, error } => 8 + type_inline_len(success) + type_inline_len(error),
    }
}

fn source_layout_newlines(source: &str) -> (VecDeque<bool>, VecDeque<bool>) {
    let Ok(tokens) = lex_spanned(source) else {
        return (VecDeque::new(), VecDeque::new());
    };
    let mut block_brace_newlines = VecDeque::new();
    let mut continuation_newlines = VecDeque::new();
    for (index, (token, range)) in tokens.iter().enumerate() {
        let Some((previous, previous_range)) = index
            .checked_sub(1)
            .and_then(|previous| tokens.get(previous))
        else {
            continue;
        };
        let has_newline = source
            .get(previous_range.end..range.start)
            .is_some_and(|between| between.contains('\n'));
        if matches!(token, Token::LBrace)
            && matches!(
                previous,
                Token::Arrow
                    | Token::RParen
                    | Token::Else
                    | Token::Try
                    | Token::Finally
                    | Token::Loop
                    | Token::Do
                    | Token::Defer
                    | Token::ErrDefer
            )
        {
            block_brace_newlines.push_back(has_newline);
        }
        if matches!(
            token,
            Token::Else | Token::Catch | Token::Finally | Token::While
        ) && matches!(previous, Token::RBrace)
        {
            continuation_newlines.push_back(has_newline);
        }
    }
    (block_brace_newlines, continuation_newlines)
}

fn format(n: &tyrus_core::ast::FloatLiteral) -> String {
    tyrus_core::ast::float_literal_source(n)
}

fn encode_string(s: &str, quote: char) -> String {
    let mut out = String::new();
    out.push(quote);
    for c in s.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            '\\' => out.push_str("\\\\"),
            '\'' if quote == '\'' => out.push_str("\\'"),
            '"' if quote == '"' => out.push_str("\\\""),
            other => out.push(other),
        }
    }
    out.push(quote);
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::field_reassign_with_default)]
    use super::*;
    use crate::config::{Config, PipelineChainLayout};

    fn fmt(source: &str) -> String {
        let config = Config::default();
        let mut parser = tyrus_core::parser::Parser::from_source(source).unwrap();
        let program = parser.parse_program().unwrap();
        let mut p = Printer::new(config, source);
        p.print_program(&program);
        p.into_string()
    }

    #[test]
    fn keeps_every_operand_of_a_type_intersection() {
        assert_eq!(fmt("type A = B & C;"), "type A = B & C;\n");
        assert_eq!(
            fmt("type A = { name: string; } & B & C;"),
            "type A = { name: string } & B & C;\n"
        );
    }

    #[test]
    fn formatting_a_type_intersection_is_idempotent() {
        let once = fmt("type A={name:string;}&B&C;");
        assert_eq!(fmt(&once), once);
    }

    #[test]
    fn keeps_an_object_type_alias_without_operands_as_a_block() {
        assert_eq!(
            fmt("type A = { only: boolean; };"),
            "type A = {\n    only: boolean;\n};\n"
        );
    }

    #[test]
    fn settles_a_readonly_const_on_the_keyword_spelling() {
        assert_eq!(
            fmt("const DAYS: readonly i32[] = [1, 2, 3];"),
            "readonly const DAYS: i32[] = [1, 2, 3];\n"
        );
        assert_eq!(
            fmt("readonly const DAYS: i32[] = [1, 2, 3];"),
            "readonly const DAYS: i32[] = [1, 2, 3];\n"
        );
    }

    #[test]
    fn keeps_a_for_initializer_on_the_plain_const_spelling() {
        let source = "const main=():void=>{for(const i:readonly i32[]=[1];false;i=i){}};";
        assert!(
            fmt(source).contains("for (const i: readonly i32[]"),
            "{}",
            fmt(source)
        );
    }

    #[test]
    fn formats_the_char_type_with_single_quotes() {
        let result = fmt("const main=():void=>{const letter:char=\"A\";const text:string='A';};");
        assert_eq!(
            result,
            "const main = (): void => {\n    const letter: char = \'A\';\n    const text: string = \"A\";\n};\n"
        );
    }

    #[test]
    fn quote_style_off_preserves_string_delimiters_and_escapes() {
        let source = r#"const single:string='it\'s';const double:string="say \"hi\"";"#;
        let config = Config {
            quote_style: crate::config::QuoteStyle::Off,
            ..Config::default()
        };
        let mut parser = tyrus_core::parser::Parser::from_source(source).unwrap();
        let program = parser.parse_program().unwrap();
        let mut printer = Printer::new(config, source);
        printer.print_program(&program);
        assert_eq!(
            printer.into_string(),
            "const single: string = 'it\\'s';\nconst double: string = \"say \\\"hi\\\"\";\n"
        );
    }

    #[test]
    fn literal_expand_off_keeps_arrays_and_objects_inline_past_the_line_width() {
        let source = "const values=[firstValue,secondValue];const options={first:firstValue,second:secondValue};";
        let config = Config {
            array_expand: crate::config::LiteralExpand::Off,
            object_expand: crate::config::LiteralExpand::Off,
            line_width: 1,
            ..Config::default()
        };
        let mut parser = tyrus_core::parser::Parser::from_source(source).unwrap();
        let program = parser.parse_program().unwrap();
        let mut printer = Printer::new(config, source);
        printer.print_program(&program);
        assert_eq!(
            printer.into_string(),
            "const values = [firstValue, secondValue];\nconst options = { first: firstValue, second: secondValue };\n"
        );
    }

    #[test]
    fn formats_a_char_parameter_default_with_single_quotes() {
        let result = fmt("const pad=(fill:char=\"-\"):void=>{};");
        assert_eq!(result, "const pad = (fill: char = \'-\'): void => {};\n");
    }

    #[test]
    fn formats_narrow_integer_types() {
        let result =
            fmt("const main=():void=>{const a:i8=1;const b:i16=2;const c:u8=3;const d:u16=4;};");
        assert_eq!(
            result,
            "const main = (): void => {\n    const a: i8 = 1;\n    const b: i16 = 2;\n    const c: u8 = 3;\n    const d: u16 = 4;\n};\n"
        );
    }

    #[test]
    fn preserves_template_format_specifiers() {
        let result = fmt(
            "const main=():void=>{const pi:f64=3.14;const id:i32=42;console.log(`Valor: ${pi:.2f} | Codigo: ${id:04d}`);};",
        );
        assert_eq!(
            result,
            "const main = (): void => {\n    const pi: f64 = 3.14;\n    const id: i32 = 42;\n    console.log(`Valor: ${pi:.2f} | Codigo: ${id:04d}`);\n};\n"
        );
    }

    #[test]
    fn collapses_a_spelled_out_field_into_shorthand_when_it_matches_its_key() {
        let result = fmt(
            "const main=():void=>{const name:string=\"Neo\";const age:i32=34;const user={name:name,age:age};};",
        );
        assert_eq!(
            result,
            "const main = (): void => {\n    const name: string = \"Neo\";\n    const age: i32 = 34;\n    const user = { name, age };\n};\n"
        );
    }

    #[test]
    fn keeps_a_field_expanded_when_its_value_does_not_match_the_key() {
        let result =
            fmt("const main=():void=>{const name:string=\"Neo\";const user={name:name,age:34};};");
        assert_eq!(
            result,
            "const main = (): void => {\n    const name: string = \"Neo\";\n    const user = { name, age: 34 };\n};\n"
        );
    }

    #[test]
    fn collapses_shorthand_inside_a_spread_update() {
        let result = fmt(
            "const main=():void=>{const name:string=\"Neo\";const updated={...base,name:name};};",
        );
        assert_eq!(
            result,
            "const main = (): void => {\n    const name: string = \"Neo\";\n    const updated = { ...base, name };\n};\n"
        );
    }

    #[test]
    fn two_term_comparison_formats_as_a_plain_binop() {
        let result = fmt("const value=1<2;");
        assert_eq!(result, "const value = 1 < 2;\n");
    }

    #[test]
    fn formats_a_chained_comparison_with_normalized_spacing() {
        let result = fmt("const value=1<2<3;");
        assert_eq!(result, "const value = 1 < 2 < 3;\n");
    }

    #[test]
    fn formats_a_chained_comparison_mixing_relational_and_strict_equality() {
        let result = fmt("const value=a<=b===c;");
        assert_eq!(result, "const value = a <= b === c;\n");
    }

    #[test]
    fn chained_comparison_formatting_is_idempotent() {
        let once = fmt("const value=1<2<3;");
        let twice = fmt(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn formats_fn_decl_written_as_function_keyword() {
        let result = fmt("function main(): void { return 1; }");
        assert_eq!(result, "function main(): void {\n    return 1;\n}\n");
    }

    #[test]
    fn preserves_validator_function_declarations_even_when_arrow_style_is_preferred() {
        let config = Config {
            prefer_arrow_function: true,
            ..Config::default()
        };
        let result = fmt_with_config(
            "validator function check(value:string):boolean{return true;}",
            config,
        );
        assert_eq!(
            result,
            "validator function check(value: string): boolean {\n    return true;\n}\n"
        );
    }

    #[test]
    fn preserves_validator_arrow_const_declarations() {
        let result =
            fmt("validator const check = (value:string):Validator=>{return {isValid:true};};");
        assert_eq!(
            result,
            "validator const check = (value: string): Validator => {\n    return { isValid: true };\n};\n"
        );
    }

    #[test]
    fn formats_fn_decl_written_as_arrow_const() {
        let result = fmt("const main=():void=>{return 1;};");
        assert_eq!(result, "const main = (): void => {\n    return 1;\n};\n");
    }

    #[test]
    fn formats_fn_decl_as_arrow_function_when_configured() {
        let config = Config {
            prefer_arrow_function: true,
            ..Config::default()
        };
        let result = fmt_with_config("function main(): void { return 1; }", config);
        assert_eq!(result, "const main = (): void => {\n    return 1;\n};\n");
    }

    #[test]
    fn formats_async_fn_decl() {
        let result = fmt("async function load(): Promise<i32> { return 1; }");
        assert_eq!(
            result,
            "async function load(): Promise<i32> {\n    return 1;\n}\n"
        );
    }

    #[test]
    fn keeps_returned_generic_method_calls_after_return_when_arguments_wrap() {
        let result = fmt(
            "function searchUsers(term: string): void { return db.query<UserResponse>(\"SELECT id, name FROM users WHERE name LIKE ? ORDER BY id\", [`%${term}%`]); }",
        );
        assert!(
            result.contains(
                "return db.query<UserResponse>(\n        \"SELECT id, name FROM users WHERE name LIKE ? ORDER BY id\","
            ),
            "expected `return` and the generic DB method call to stay together while arguments wrap, got: {result:?}"
        );
    }

    #[test]
    fn formats_comptime_declarations_and_expressions() {
        let result = fmt(
            "comptime function double(value:i32):i32{return value*2;} comptime const mask=():i32=>3; const result=comptime(double(mask()));",
        );
        assert_eq!(
            result,
            "comptime function double(value: i32): i32 {\n    return value * 2;\n}\n\ncomptime const mask = (): i32 => {\n    return 3;\n};\n\nconst result = comptime(double(mask()));\n"
        );
    }

    #[test]
    fn formats_await_expression() {
        let result =
            fmt("async function load(value: Promise<i32>): Promise<i32> { return await value; }");
        assert_eq!(
            result,
            "async function load(value: Promise<i32>): Promise<i32> {\n    return await value;\n}\n"
        );
    }

    #[test]
    fn formats_method_call_type_arguments() {
        let result = fmt("const capability=Promise.withResolvers<i32>();");
        assert_eq!(result, "const capability = Promise.withResolvers<i32>();\n");
    }

    #[test]
    fn preserves_type_alias_object_vs_interface() {
        let result = fmt("type A={a:string;};");
        assert_eq!(result, "type A = {\n    a: string;\n};\n");
        let result = fmt("interface B{b:i32;}");
        assert_eq!(result, "interface B {\n    b: i32;\n}\n");
    }

    #[test]
    fn formats_validator_decorators() {
        let result = fmt(
            "interface UserInput{@minLength(3)@isTrimmed()name:string;}const divide=(a:f64,@notEqual(0)b:f64):f64=>a/b;",
        );
        assert!(result.contains("    @minLength(3)\n    @isTrimmed()\n    name: string;"));
        assert!(result.contains("const divide = (a: f64, @notEqual(0) b: f64): f64 =>"));
    }

    #[test]
    fn formats_import_then_fn() {
        let result = fmt("import{x}from\"mod\";const main=():void=>{};");
        assert_eq!(
            result,
            "import { x } from \"mod\";\n\nconst main = (): void => {};\n"
        );
        let result2 = fmt("const main=():void=>{return 1;};");
        assert_eq!(result2, "const main = (): void => {\n    return 1;\n};\n");
    }

    #[test]
    fn formats_var_decl() {
        let result = fmt("const x:i32=42;");
        assert_eq!(result, "const x: i32 = 42;\n");
    }

    #[test]
    fn formats_index_assignment_with_semicolon() {
        let result = fmt("const f=():void=>{let t:[i32,i32]=[1,2];t[0]=5;};");
        assert!(
            result.contains("t[0] = 5;"),
            "expected `t[0] = 5;`, got: {result}"
        );
    }

    #[test]
    fn formats_nominal_enum() {
        let result = fmt("enum Status{Success,Error,Pending}");
        assert_eq!(
            result,
            "enum Status {\n    Success,\n    Error,\n    Pending,\n}\n"
        );
    }

    #[test]
    fn formats_valued_enum_with_repr() {
        let result = fmt("enum HttpStatus:i64{Ok=200,NotFound=404}");
        assert_eq!(
            result,
            "enum HttpStatus: i64 {\n    Ok = 200,\n    NotFound = 404,\n}\n"
        );
    }

    #[test]
    fn formats_string_enum() {
        let result = fmt("enum Role{Admin=\"admin\",User=\"user\"}");
        assert_eq!(
            result,
            "enum Role {\n    Admin = \"admin\",\n    User = \"user\",\n}\n"
        );
    }

    #[test]
    fn enum_formatting_is_idempotent() {
        let once = fmt("enum Flag{Enabled=true,Disabled=false}");
        let twice = fmt(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn preserves_algebraic_enum_payloads_and_patterns() {
        let source = r#"
const main=():void=>{
    const describe=(command:Command):i32=>{
        match(command){
            Command.Quit=>{return 0;}
            Command.Move(x,y)=>{return x+y;}
            Command.ChangeColor{r,g,b}=>{return (r+g+b)as i32;}
        }
    };
};

enum Command{Quit,Move(i32,i32),ChangeColor{r:u8,g:u8,b:u8}}
"#;
        let formatted = fmt(source);

        assert!(formatted.contains("Move(i32, i32),"), "{formatted}");
        assert!(
            formatted.contains("ChangeColor { r: u8, g: u8, b: u8 },"),
            "{formatted}"
        );
        assert!(formatted.contains("Command.Quit =>"), "{formatted}");
        assert!(formatted.contains("Command.Move(x, y) =>"), "{formatted}");
        assert!(
            formatted.contains("Command.ChangeColor { r, g, b } =>"),
            "{formatted}"
        );
        assert!(!formatted.contains("::"), "{formatted}");
        assert_eq!(formatted, fmt(&formatted));
    }

    #[test]
    fn formats_if_else() {
        let result = fmt("const f=():void=>{if(true){return 1;}else{return 2;}};");
        assert!(result.contains("if (true) {"));
        assert!(result.contains("} else {"));
    }

    #[test]
    fn formats_while_loop() {
        let result = fmt("let i:i32=0;while(i<10){i+=1;}");
        assert!(result.contains("while ("));
    }

    #[test]
    fn formats_for_of_loop() {
        let result =
            fmt("const values:i32[]=[1,2];for(const value of values){console.log(value);}");
        assert!(result.contains("for (const value of values) {"));
    }

    #[test]
    fn formats_optional_object_fields_and_object_update() {
        let result = fmt(
            r#"interface User{name:string;nickname?:string;}const updated:User={...user,name:"Grace"};"#,
        );
        assert!(result.contains("nickname?: string;"));
        assert!(result.contains(r#"const updated: User = { ...user, name: "Grace" };"#));
    }

    fn fmt_with_config(source: &str, config: Config) -> String {
        let mut parser = tyrus_core::parser::Parser::from_source(source).unwrap();
        let program = parser.parse_program().unwrap();
        let mut p = Printer::new(config, source);
        p.print_program(&program);
        p.into_string()
    }

    #[test]
    fn default_none_blank_lines() {
        let config = Config::default();
        let result = fmt_with_config(
            "const a: i32 = 1;\nconst b: i32 = 2;\nconst c: i32 = 3;",
            config,
        );
        assert_eq!(
            result,
            "const a: i32 = 1;\nconst b: i32 = 2;\nconst c: i32 = 3;\n"
        );
    }

    #[test]
    fn off_blank_lines_with_imports() {
        let mut config = Config::default();
        config.blank_lines = crate::config::BlankLines::Off;
        config.imports_sort = false;
        let result = fmt_with_config(
            "import { x } from \"mod1\";\nimport { y } from \"mod2\";\nconst a: i32 = 1;",
            config,
        );
        assert_eq!(
            result,
            "import { x } from \"mod1\";\nimport { y } from \"mod2\";\nconst a: i32 = 1;\n"
        );
    }

    #[test]
    fn blank_lines_off_still_removes_unused_imports() {
        let mut config = Config::default();
        config.blank_lines = crate::config::BlankLines::Off;
        config.remove_unused_imports = true;
        let source = "import { unused } from \"mod\";\n\nconst value:i32=1;\n";
        let result = crate::format_source(source, &config);
        assert_eq!(result, "const value: i32 = 1;\n");
    }

    #[test]
    fn match_arms_format_without_when() {
        let result = crate::format_source(
            "const main=(value:i32):void=>{match(value){(1)=>{return;}_=>{return;}}};",
            &Config::default(),
        );
        assert!(result.contains("\n        1 =>"));
        assert!(result.contains("\n        _ =>"));
        assert!(!result.contains("when"));
        let mut parser = tyrus_core::parser::Parser::from_source(&result).unwrap();
        assert!(parser.parse_program().is_ok());
    }

    #[test]
    fn formats_array_match_patterns_with_rest_bindings() {
        let result = crate::format_source(
            "const main=(values:i32[]):void=>{match(values){[first,...rest]=>{console.log(first);console.log(rest);}_=>{return;}}};",
            &Config::default(),
        );
        assert!(result.contains("[first, ...rest] =>"), "{result}");
        assert_eq!(result, crate::format_source(&result, &Config::default()));
        let mut parser = tyrus_core::parser::Parser::from_source(&result).unwrap();
        assert!(parser.parse_program().is_ok());
    }

    #[test]
    fn collapse_blank_lines() {
        let mut config = Config::default();
        config.blank_lines = crate::config::BlankLines::Collapse;
        let result = fmt_with_config(
            "import { x } from \"mod1\";\nimport { y } from \"mod2\";\nconst a: i32 = 1;",
            config,
        );
        assert_eq!(
            result,
            "import { x } from \"mod1\";\nimport { y } from \"mod2\";\n\nconst a: i32 = 1;\n"
        );
    }

    #[test]
    fn brace_style_next_line_moves_braces_to_own_line() {
        let mut config = Config::default();
        config.brace_style = crate::config::BraceStyle::NextLine;
        let result = crate::format_source(
            "const f=():void=>{if(x){a();}else{b();}while(y){c();}};",
            &config,
        );
        assert_eq!(
            result,
            "const f = (): void =>\n\
             {\n\
             \x20   if (x)\n\
             \x20   {\n\
             \x20       a();\n\
             \x20   }\n\
             \x20   else\n\
             \x20   {\n\
             \x20       b();\n\
             \x20   }\n\n\
             \x20   while (y)\n\
             \x20   {\n\
             \x20       c();\n\
             \x20   }\n\
             };\n"
        );
    }

    #[test]
    fn brace_style_off_preserves_each_block_and_continuation_layout() {
        let source = "const main=():void=>\n{if(true) {first();}\nelse\n{second();}};";
        let mut config = Config::default();
        config.brace_style = crate::config::BraceStyle::Off;
        assert_eq!(
            fmt_with_config(source, config),
            "const main = (): void =>\n{\n    if (true) {\n        first();\n    }\n    else\n    {\n        second();\n    }\n};\n"
        );
    }

    #[test]
    fn else_block_with_single_statement_keeps_braces() {
        let result = fmt("const f=():void=>{if(x){a();}else{b();}};");
        assert_eq!(
            result,
            "const f = (): void => {\n    if (x) {\n        a();\n    } else {\n        b();\n    }\n};\n"
        );
    }

    #[test]
    fn object_literal_value_keeps_brace_on_same_line() {
        let result = fmt(
            "const filter: ProductFilter =\n{\nquery: \"notebook\",\ncategory: \"electronics\",\npageSize: 20,\nlimit: 20,\n};",
        );
        assert_eq!(
            result,
            "const filter: ProductFilter = {\n    query: \"notebook\",\n    category: \"electronics\",\n    pageSize: 20,\n    limit: 20,\n};\n"
        );
    }

    #[test]
    fn blank_lines_around_multiline_object_initializer() {
        let result = crate::format_source(
            "const a: i32 = 1;\nconst filter: ProductFilter = { query: \"notebook\", category: \"electronics\", pageSize: 20, limit: 20 };\nconst p: Point = { x: 1, y: 2 };\nconst c: i32 = 3;\n",
            &Config::default(),
        );
        assert_eq!(
            result,
            "const a: i32 = 1;\n\n\
             const filter: ProductFilter = {\n\
             \x20   query: \"notebook\",\n\
             \x20   category: \"electronics\",\n\
             \x20   pageSize: 20,\n\
             \x20   limit: 20,\n\
             };\n\n\
             const p: Point = { x: 1, y: 2 };\n\
             const c: i32 = 3;\n"
        );
    }

    #[test]
    fn blank_lines_around_decorated_fields() {
        let result = crate::format_source(
            "interface F{@notEmpty()query:string;@min(5)pageSize:i32;}",
            &Config::default(),
        );
        assert_eq!(
            result,
            "interface F {\n\
             \x20   @notEmpty()\n\
             \x20   query: string;\n\n\
             \x20   @min(5)\n\
             \x20   pageSize: i32;\n\
             }\n"
        );
    }

    #[test]
    fn blank_lines_around_decorations_disabled() {
        let mut config = Config::default();
        config.blank_lines_around_decorations = false;
        let result = fmt_with_config(
            "interface F{@notEmpty()query:string;@min(5)pageSize:i32;}",
            config,
        );
        assert_eq!(
            result,
            "interface F {\n\
             \x20   @notEmpty()\n\
             \x20   query: string;\n\
             \x20   @min(5)\n\
             \x20   pageSize: i32;\n\
             }\n"
        );
    }

    #[test]
    fn blank_lines_around_interface_type_and_enum() {
        let result =
            fmt("const a:i32=1;type A={a:string;};interface B{b:i32;}enum E{X,Y}const b:i32=2;");
        assert_eq!(
            result,
            "const a: i32 = 1;\n\n\
             type A = {\n    a: string;\n};\n\n\
             interface B {\n    b: i32;\n}\n\n\
             enum E {\n    X,\n    Y,\n}\n\n\
             const b: i32 = 2;\n"
        );
    }

    #[test]
    fn removes_unused_imports_when_enabled() {
        let mut config = Config::default();
        config.remove_unused_imports = true;
        let result = crate::format_source(
            r#"import { used, unused, Renamed as Local } from "mod";
import { OnlyUnused } from "other";

const value: used = Local();
"#,
            &config,
        );
        assert_eq!(
            result,
            "import { Renamed as Local, used } from \"mod\";\n\nconst value: used = Local();\n"
        );
    }

    #[test]
    fn prints_what_a_function_type_throws() {
        assert_eq!(
            fmt("type Load = (path: string) => string throws FsError | StreamError;"),
            "type Load = (string) => string throws FsError | StreamError;\n"
        );
    }

    #[test]
    fn keeps_a_catch_without_an_annotation_as_it_was_written() {
        let unannotated =
            fmt("const main = (): void => { try { risky(); } catch (error) { report(error); } };");
        assert!(unannotated.contains("catch (error) {"), "{unannotated}");
        let annotated = fmt(
            "const main = (): void => { try { risky(); } catch (error: Error) { report(error); } };",
        );
        assert!(annotated.contains("catch (error: Error) {"), "{annotated}");
    }

    #[test]
    fn a_type_only_used_in_a_catch_clause_annotation_is_not_removed() {
        let mut config = Config::default();
        config.remove_unused_imports = true;
        let source = r#"import { OsError, openUrl } from "tyrus:os";

const main = (): void => {
    try {
        openUrl("https://example.com");
    } catch (error: OsError) {
        console.log(error.code);
    }
};
"#;
        let result = crate::format_source(source, &config);
        assert!(
            result.contains("OsError"),
            "OsError is used in the catch clause and must not be removed, got: {result:?}"
        );
    }

    #[test]
    fn removes_unused_imports_without_leading_blank_line() {
        let mut config = Config::default();
        config.remove_unused_imports = true;
        let result = crate::format_source(
            r#"import { unused } from "mod";

const value: i32 = 1;
"#,
            &config,
        );
        assert_eq!(result, "const value: i32 = 1;\n");
    }

    #[test]
    fn keeps_imports_used_only_in_type_positions() {
        let mut config = Config::default();
        config.remove_unused_imports = true;
        let result = crate::format_source(
            r#"import { User } from "models";

const printUser = (user: User): void => {
    console.log(user);
};
"#,
            &config,
        );
        assert_eq!(
            result,
            "import { User } from \"models\";\n\nconst printUser = (user: User): void => {\n    console.log(user);\n};\n"
        );
    }

    #[test]
    fn formats_const_assertions_and_readonly_types() {
        let result = fmt("const values: readonly i32[] = [1,2] as const;");
        assert_eq!(result, "readonly const values: i32[] = [1, 2] as const;\n");
    }

    #[test]
    fn no_final_newline() {
        let mut config = Config::default();
        config.final_newline = false;
        let result = fmt_with_config("const x: i32 = 42;", config);
        assert_eq!(result, "const x: i32 = 42;");
    }

    #[test]
    fn no_final_newline_multiple_items() {
        let mut config = Config::default();
        config.final_newline = false;
        let result = fmt_with_config("const a: i32 = 1;\nconst b: i32 = 2;", config);
        assert_eq!(result, "const a: i32 = 1;\nconst b: i32 = 2;");
    }

    #[test]
    fn collapse_and_no_final() {
        let mut config = Config::default();
        config.blank_lines = crate::config::BlankLines::Collapse;
        config.final_newline = false;
        let result = fmt_with_config(
            "import { x } from \"mod\";\nconst a: i32 = 1;\nconst b: i32 = 2;",
            config,
        );
        assert_eq!(
            result,
            "import { x } from \"mod\";\n\nconst a: i32 = 1;\nconst b: i32 = 2;"
        );
    }

    #[test]
    fn type_assert_parenthesizes_a_lower_precedence_binop_operand() {
        let source = "const main = (): void => {\n    const seconds: u64 = (10 * 365 * 24 * 60 * 60) as u64;\n    console.log(seconds);\n};\n";
        let result = crate::format_source(source, &Config::default());
        assert!(
            result.contains(") as u64"),
            "expected the whole product still wrapped before `as u64`, got: {result:?}"
        );
        assert!(
            !result.contains("* 60 as u64"),
            "the cast must not bind to only the trailing operand, got: {result:?}"
        );
    }

    #[test]
    fn sorts_imports_alphabetically_with_language_at_top() {
        let source = "import { readFileSync } from \"tyrus:fs\";\nimport { add } from \"./math\";\nconst result: i32 = add(1, 2);\nconsole.log(readFileSync(\"test\"));\n";
        let result = crate::format_source(source, &Config::default());
        assert_eq!(
            result,
            "import { readFileSync } from \"tyrus:fs\";\n\nimport { add } from \"./math\";\n\nconst result: i32 = add(1, 2);\nconsole.log(readFileSync(\"test\"));\n"
        );
    }

    #[test]
    fn sorts_named_import_specifiers_alphabetically() {
        let source = "import { range, double, clamp, increment } from \"./pipe/examples\";\nconst result = 5 |> double |> increment |> double;\nconst ranged = 100 |> range(50);\nconst clamped = 80 |> clamp(30, 100);\n";
        let result = crate::format_source(source, &Config::default());
        assert_eq!(
            result,
            "import { clamp, double, increment, range } from \"./pipe/examples\";\n\nconst result = 5\n    |> double\n    |> increment\n    |> double;\nconst ranged = 100 |> range(50);\nconst clamped = 80 |> clamp(30, 100);\n"
        );
    }

    #[test]
    fn preserves_single_blank_lines_inside_blocks_with_inline_comments() {
        let mut config = Config::default();
        config.blank_lines = crate::config::BlankLines::Off;
        let source = r#"import { clamp, double, increment, range } from "./pipe/examples";

const main = (): void => {
    const result = 5 |> double |> increment |> double;
    console.log(result); // 22

    const ranged = 100 |> range(50);
    console.log(ranged); // 50

    const clamped = 80 |> clamp(30, 100);
    console.log(clamped); // 80

    42 |> double |> console.log; // 84
};
"#;
        let result = crate::format_source(source, &config);
        assert_eq!(
            result,
            r#"import { clamp, double, increment, range } from "./pipe/examples";

const main = (): void => {
    const result = 5
        |> double
        |> increment
        |> double;
    console.log(result);  // 22

    const ranged = 100 |> range(50);
    console.log(ranged);  // 50

    const clamped = 80 |> clamp(30, 100);
    console.log(clamped);  // 80

    42
        |> double
        |> console.log;  // 84
};
"#
        );
        assert!(!result.contains("\n\n\n"));
    }

    #[test]
    fn preserves_pipeline_operator() {
        assert_eq!(
            fmt("const r=x|>a|>b;"),
            "const r = x\n    |> a\n    |> b;\n"
        );
        assert_eq!(
            fmt("const r=value|>style([1,2]);"),
            "const r = value |> style([1, 2]);\n"
        );
        assert_eq!(
            fmt("const r=value|>style(%)|>console.log;"),
            "const r = value\n    |> style(%)\n    |> console.log;\n"
        );
    }

    #[test]
    fn pipeline_chain_layout_inline_keeps_chain_on_one_line() {
        let mut config = Config::default();
        config.pipeline_chain_layout = PipelineChainLayout::Inline;
        assert_eq!(
            fmt_with_config("const r=x|>a|>b;", config),
            "const r = x |> a |> b;\n"
        );
    }

    #[test]
    fn pipeline_chain_layout_off_preserves_each_chain_layout() {
        let mut config = Config::default();
        config.pipeline_chain_layout = PipelineChainLayout::Off;
        let source = "const inline=value|>first|>second;const multiline=value\n|>first\n|>second;";
        assert_eq!(
            fmt_with_config(source, config),
            "const inline = value |> first |> second;\nconst multiline = value\n    |> first\n    |> second;\n"
        );
    }

    #[test]
    fn pipeline_chain_layout_inline_preserves_leading_comments_after_collapsed_chain() {
        let mut config = Config::default();
        config.blank_lines = crate::config::BlankLines::Off;
        config.pipeline_chain_layout = PipelineChainLayout::Inline;
        let source = r#"const main = (): void => {
    // Pipe chains can mix implicit and topic-token steps.
    const mixed: i64 = 5
        |> double
        |> clamp(0, %, 20)
        |> increment;
    console.log(mixed);

    // Piping straight into console.log.
    42
        |> double
        |> console.log;
};
"#;
        assert_eq!(
            crate::format_source(source, &config),
            r#"const main = (): void => {
    // Pipe chains can mix implicit and topic-token steps.
    const mixed: i64 = 5 |> double |> clamp(0, %, 20) |> increment;
    console.log(mixed);

    // Piping straight into console.log.
    42 |> double |> console.log;
};
"#
        );
    }
}
