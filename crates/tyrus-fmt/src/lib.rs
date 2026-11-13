use tyrus_core::ast::ExprKind;
use tyrus_core::ast::StmtKind;
#[cfg(feature = "benchmark-internals")]
pub mod benchmark;
#[path = "comments/comments.rs"]
pub mod comments;
#[path = "config/config.rs"]
pub mod config;
#[path = "printer/printer.rs"]
pub mod printer;

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::fs;
use std::path::Path;

use config::Config;
use printer::Printer;
use tyrus_core::{ast::*, parser::Parser};

type Anchored = BTreeMap<usize, Vec<(bool, String)>>;

pub fn format_source(source: &str, config: &Config) -> String {
    let mut parser = match Parser::from_source(source) {
        Ok(parser) => parser,
        Err(_) => return source.to_string(),
    };
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(_) => return source.to_string(),
    };
    format_source_with_program(source, config, &program)
}

pub fn format_source_with_program(source: &str, config: &Config, program: &Program) -> String {
    let comments = comments::extract_comments(source);
    let src_lines: Vec<&str> = source.lines().collect();
    let anchored = anchor_comments(&src_lines, &comments);
    let mut program = program.clone();
    if config.remove_unused_imports {
        remove_unused_imports(&mut program);
    }
    if config.imports_sort {
        sort_imports(&mut program);
    }

    let mut p = Printer::new(config.clone(), source);
    p.print_program(&program);
    let formatted = p.into_string();

    let needs_source_line_mapping =
        !comments.is_empty() || src_lines.iter().any(|line| line.trim().is_empty());
    if !needs_source_line_mapping {
        return preserve_blank_lines_only(&formatted, &src_lines, &vec![None; src_lines.len()]);
    }

    let line_map = build_line_map(source, &formatted);

    if !comments.is_empty() {
        reinsert_comments(&formatted, &src_lines, &anchored, &line_map, true)
    } else {
        preserve_blank_lines_only(&formatted, &src_lines, &line_map)
    }
}

fn anchor_comments(src_lines: &[&str], comments: &[comments::Comment]) -> Anchored {
    let mut next_code_line = vec![None; src_lines.len() + 1];
    for line in (0..src_lines.len()).rev() {
        let trimmed = src_lines[line].trim();
        next_code_line[line] =
            if !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with("/*") {
                Some(line)
            } else {
                next_code_line[line + 1]
            };
    }
    let mut anchored: Anchored = BTreeMap::new();
    for comment in comments {
        let line = comment.line;
        let trimmed_src = src_lines[line].trim();
        let is_trailing = !trimmed_src.starts_with("//")
            && !trimmed_src.starts_with("/*")
            && !trimmed_src.starts_with("/**");
        let anchor_line = if is_trailing {
            line
        } else {
            next_code_line[line].unwrap_or(line)
        };
        anchored
            .entry(anchor_line)
            .or_default()
            .push((is_trailing, comment.text.trim_end().to_string()));
    }
    anchored
}

fn remove_unused_imports(program: &mut Program) {
    let mut used = BTreeSet::new();
    for item in &program.items {
        collect_item_names(item, &mut used);
    }
    for export in &program.exports {
        if let ExportDecl::Named { local, .. } = export {
            used.insert(local.clone());
        }
    }
    program.imports.iter_mut().for_each(|import| {
        import
            .specifiers
            .retain(|specifier| used.contains(&specifier.local));
    });
    program
        .imports
        .retain(|import| !import.specifiers.is_empty());
}

fn sort_imports(program: &mut Program) {
    for import in &mut program.imports {
        import.specifiers.sort_by(|a, b| {
            a.imported
                .cmp(&b.imported)
                .then_with(|| a.local.cmp(&b.local))
        });
    }
    program.imports.sort_by(|a, b| {
        let a_is_lang = a.source.starts_with("tyrus:");
        let b_is_lang = b.source.starts_with("tyrus:");
        if a_is_lang != b_is_lang {
            return b_is_lang.cmp(&a_is_lang);
        }
        let a_key = a.source.trim_start_matches(['.', '/']);
        let b_key = b.source.trim_start_matches(['.', '/']);
        a_key.cmp(b_key)
    });
}

fn collect_item_names(item: &Item, used: &mut BTreeSet<String>) {
    match item {
        Item::InterfaceDecl {
            type_params,
            extends,
            fields,
            ..
        } => {
            for type_param in type_params {
                if let Some(constraint) = &type_param.constraint {
                    collect_type_names(constraint, used);
                }
            }
            for supertype in extends {
                collect_type_names(supertype, used);
            }
            for field in fields {
                collect_type_names(&field.ty, used);
            }
        }
        Item::BehaviorInterfaceDecl {
            type_params,
            targets,
            extends,
            methods,
            bindings,
            ..
        } => {
            for type_param in type_params {
                if let Some(constraint) = &type_param.constraint {
                    collect_type_names(constraint, used);
                }
            }
            for target in targets {
                collect_type_names(target, used);
            }
            for supertype in extends {
                collect_type_names(supertype, used);
            }
            for method in methods {
                for param in &method.params {
                    collect_param_names(param, used);
                }
                collect_type_names(&method.ret_type, used);
                for ty in &method.throws {
                    collect_type_names(ty, used);
                }
                for statement in &method.body {
                    collect_stmt_names(statement, used);
                }
            }
            for binding in bindings {
                used.insert(binding.clone());
            }
        }
        Item::TypeAlias {
            ty, merged_fields, ..
        } => {
            collect_type_names(ty, used);
            for field in merged_fields {
                collect_type_names(&field.ty, used);
            }
        }
        Item::EnumDecl { repr, .. } => {
            collect_type_names(repr, used);
        }
        Item::FnDecl {
            type_params,
            params,
            ret_type,
            throws,
            body,
            ..
        } => {
            for type_param in type_params {
                if let Some(constraint) = &type_param.constraint {
                    collect_type_names(constraint, used);
                }
            }
            for param in params {
                collect_param_names(param, used);
            }
            collect_type_names(ret_type, used);
            for ty in throws {
                collect_type_names(ty, used);
            }
            for statement in body {
                collect_stmt_names(statement, used);
            }
        }
        Item::Stmt(statement) => collect_stmt_names(statement, used),
    }
}

fn collect_param_names(param: &Param, used: &mut BTreeSet<String>) {
    collect_type_names(&param.ty, used);
    if let Some(default_value) = &param.default_value {
        collect_expr_names(default_value, used);
    }
}

fn collect_stmt_names(stmt: &Stmt, used: &mut BTreeSet<String>) {
    match stmt.kind() {
        StmtKind::VarDecl {
            ty, value, onfail, ..
        } => {
            collect_type_names(ty, used);
            if let Some(value) = value {
                collect_expr_names(value, used);
            }
            if let Some(onfail) = onfail {
                match &onfail.handler {
                    OnFailHandler::Expr(handler) => collect_expr_names(handler, used),
                    OnFailHandler::Block(handler) => collect_stmt_list_names(handler, used),
                }
            }
        }
        StmtKind::ArrayDestructure {
            bindings, value, ..
        }
        | StmtKind::ObjectDestructure {
            bindings, value, ..
        } => {
            for binding in bindings {
                collect_type_names(&binding.ty, used);
            }
            collect_expr_names(value, used);
        }
        StmtKind::Assign { value, .. } => collect_expr_names(value, used),
        StmtKind::IndexAssign { index, value, .. } => {
            collect_expr_names(index, used);
            collect_expr_names(value, used);
        }
        StmtKind::FieldAssign { object, value, .. } => {
            collect_expr_names(object, used);
            collect_expr_names(value, used);
        }
        StmtKind::Return(Some(expr)) | StmtKind::Expr(expr) => collect_expr_names(expr, used),
        StmtKind::Return(None) | StmtKind::Continue => {}
        StmtKind::Break(value) => {
            if let Some(value) = value {
                collect_expr_names(value, used);
            }
        }
        StmtKind::While { cond, body } => {
            collect_expr_names(cond, used);
            collect_stmt_list_names(body, used);
        }
        StmtKind::DoWhile { body, cond } => {
            collect_stmt_list_names(body, used);
            collect_expr_names(cond, used);
        }
        StmtKind::For {
            init,
            cond,
            update,
            body,
        } => {
            collect_stmt_names(init, used);
            collect_expr_names(cond, used);
            collect_stmt_names(update, used);
            collect_stmt_list_names(body, used);
        }
        StmtKind::ForOf { iterable, body, .. } => {
            collect_expr_names(iterable, used);
            collect_stmt_list_names(body, used);
        }
        StmtKind::ForIn { object, body, .. } => {
            collect_expr_names(object, used);
            collect_stmt_list_names(body, used);
        }
        StmtKind::Switch {
            expr,
            cases,
            default,
            ..
        } => {
            collect_expr_names(expr, used);
            for case in cases {
                collect_expr_names(&case.value, used);
                collect_stmt_list_names(&case.body, used);
            }
            if let Some(default) = default {
                collect_stmt_list_names(default, used);
            }
        }
        StmtKind::Match {
            expr,
            arms,
            default,
        } => {
            collect_expr_names(expr, used);
            for arm in arms {
                collect_match_pattern_names(&arm.pattern, used);
                if let Some(guard) = &arm.guard {
                    collect_expr_names(guard, used);
                }
                collect_stmt_list_names(&arm.body, used);
            }
            if let Some(default) = default {
                collect_stmt_list_names(default, used);
            }
        }
        StmtKind::Throw(expr) => {
            collect_expr_names(expr, used);
        }
        StmtKind::TryCatch {
            try_body,
            catch_binding,
            catch_type,
            catch_type_span: _,
            catch_body,
            finally_body,
        } => {
            collect_stmt_list_names(try_body, used);
            used.insert(catch_binding.clone());
            collect_type_names(catch_type, used);
            collect_stmt_list_names(catch_body, used);
            if let Some(stmts) = finally_body {
                collect_stmt_list_names(stmts, used);
            }
        }
        StmtKind::Defer { body, .. } => collect_stmt_list_names(body, used),
    }
}

fn collect_stmt_list_names(statements: &[Stmt], used: &mut BTreeSet<String>) {
    for statement in statements {
        collect_stmt_names(statement, used);
    }
}

fn collect_expr_names(expr: &Expr, used: &mut BTreeSet<String>) {
    match expr.kind() {
        ExprKind::Ident(name) => {
            used.insert(name.clone());
        }
        ExprKind::Borrow { name, .. } => {
            used.insert(name.clone());
        }
        ExprKind::ObjectLit(fields) => {
            for (_, value) in fields {
                collect_expr_names(value, used);
            }
        }
        ExprKind::ObjectUpdate { base, fields } => {
            collect_expr_names(base, used);
            for (_, value) in fields {
                collect_expr_names(value, used);
            }
        }
        ExprKind::ArrayLit(values) => {
            for value in values {
                collect_expr_names(value, used);
            }
        }
        ExprKind::ArrayRepeat { value, count } => {
            collect_expr_names(value, used);
            collect_expr_names(count, used);
        }
        ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
            collect_expr_names(object, used);
        }
        ExprKind::IndexAccess { object, index } => {
            collect_expr_names(object, used);
            collect_expr_names(index, used);
        }
        ExprKind::BinOp { left, right, .. } => {
            collect_expr_names(left, used);
            collect_expr_names(right, used);
        }
        ExprKind::ChainedComparison { terms, .. } => {
            for term in terms {
                collect_expr_names(term, used);
            }
        }
        ExprKind::Pipeline { left, right } => {
            collect_expr_names(left, used);
            collect_expr_names(right, used);
        }
        ExprKind::Not(inner)
        | ExprKind::Neg(inner)
        | ExprKind::BitNot(inner)
        | ExprKind::PrefixIncr(inner)
        | ExprKind::PrefixDecr(inner)
        | ExprKind::PostfixIncr(inner)
        | ExprKind::PostfixDecr(inner)
        | ExprKind::Unwrap(inner)
        | ExprKind::Typeof(inner)
        | ExprKind::Await(inner)
        | ExprKind::Try(inner)
        | ExprKind::Spread(inner)
        | ExprKind::ConstAssert(inner)
        | ExprKind::Comptime(inner) => collect_expr_names(inner, used),
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            collect_expr_names(cond, used);
            collect_expr_names(then_expr, used);
            collect_expr_names(else_expr, used);
        }
        ExprKind::TypeAssert { value, ty } => {
            collect_expr_names(value, used);
            collect_type_names(ty, used);
        }
        ExprKind::OptionalMethodCall { object, args, .. }
        | ExprKind::MethodCall { object, args, .. } => {
            collect_expr_names(object, used);
            for arg in args {
                collect_expr_names(arg, used);
            }
        }
        ExprKind::NewSet { ty, initial } => {
            if let Some(ty) = ty {
                collect_type_names(ty, used);
            }
            if let Some(initial) = initial {
                collect_expr_names(initial, used);
            }
        }
        ExprKind::NewMap {
            key,
            value,
            initial,
        } => {
            if let Some(key) = key {
                collect_type_names(key, used);
            }
            if let Some(value) = value {
                collect_type_names(value, used);
            }
            if let Some(initial) = initial {
                collect_expr_names(initial, used);
            }
        }
        ExprKind::CallValue {
            callee,
            type_args,
            args,
        } => {
            collect_expr_names(callee, used);
            for type_arg in type_args {
                collect_type_names(type_arg, used);
            }
            for arg in args {
                collect_expr_names(arg, used);
            }
        }
        ExprKind::FnCall {
            callee,
            type_args,
            args,
        } => {
            used.insert(callee.clone());
            for type_arg in type_args {
                collect_type_names(type_arg, used);
            }
            for arg in args {
                collect_expr_names(arg, used);
            }
        }
        ExprKind::ArrowFn {
            is_async: _,
            type_params,
            params,
            ret_type,
            throws,
            body,
        } => {
            for type_param in type_params {
                if let Some(constraint) = &type_param.constraint {
                    collect_type_names(constraint, used);
                }
            }
            for param in params {
                collect_param_names(param, used);
            }
            collect_type_names(ret_type, used);
            for ty in throws {
                collect_type_names(ty, used);
            }
            collect_stmt_list_names(body, used);
        }
        ExprKind::If {
            cond,
            then_block,
            else_block,
        } => {
            collect_expr_names(cond, used);
            collect_stmt_list_names(then_block, used);
            if let Some(else_block) = else_block {
                collect_stmt_list_names(else_block, used);
            }
        }
        ExprKind::Loop { body } => collect_stmt_list_names(body, used),
        ExprKind::TemplateLit(parts) => {
            for part in parts {
                if let TemplatePart::Expr(expr, _) = part {
                    collect_expr_names(expr, used);
                }
            }
        }
        ExprKind::In { key, object } => {
            collect_expr_names(key, used);
            collect_expr_names(object, used);
        }
        ExprKind::Is { expr, ty } => {
            collect_expr_names(expr, used);
            collect_type_names(ty, used);
        }
        ExprKind::Range { start, end, .. } => {
            collect_expr_names(start, used);
            collect_expr_names(end, used);
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
        | ExprKind::Topic => {}
    }
}

fn collect_type_names(ty: &Type, used: &mut BTreeSet<String>) {
    match ty {
        Type::Named(name) => {
            used.insert(name.clone());
        }
        Type::Array(inner)
        | Type::Readonly(inner)
        | Type::Ref(inner)
        | Type::MutRef(inner)
        | Type::Optional(inner) => collect_type_names(inner, used),
        Type::Result { success, error } => {
            collect_type_names(success, used);
            collect_type_names(error, used);
        }
        Type::Tuple(elements) | Type::Union(elements) => {
            for element in elements {
                collect_type_names(element, used);
            }
        }
        Type::Function { params, ret } | Type::GenericFunction { params, ret, .. } => {
            for param in params {
                collect_type_names(param, used);
            }
            collect_type_names(ret, used);
        }
        Type::Throws { function, effects } => {
            collect_type_names(function, used);
            for effect in effects {
                collect_type_names(effect, used);
            }
        }
        Type::Object(fields) => {
            for field in fields {
                collect_type_names(&field.ty, used);
            }
        }
        Type::Lit(expr) | Type::TypeQuery(expr) => collect_expr_names(expr, used),
        Type::Apply { name, args } => {
            used.insert(name.clone());
            for arg in args {
                collect_type_names(arg, used);
            }
        }
        Type::KeyOf(inner) => collect_type_names(inner, used),
        Type::Map(key, value) => {
            collect_type_names(key, used);
            collect_type_names(value, used);
        }
        Type::Set(value) => collect_type_names(value, used),
        Type::BinaryHeap(value) => collect_type_names(value, used),
        Type::BinomialHeap(value) => collect_type_names(value, used),
        Type::UnionFind(value) => collect_type_names(value, used),
        Type::Counter(value) => collect_type_names(value, used),
        Type::Deque(value) => collect_type_names(value, used),
        Type::LinkedList(value) => collect_type_names(value, used),
        Type::DoubleLinkedList(value) => collect_type_names(value, used),
        Type::AvlTree(value) => collect_type_names(value, used),
        Type::RedBlackTree(value) => collect_type_names(value, used),
        Type::SplayTree(value) => collect_type_names(value, used),
        Type::Treap(value) => collect_type_names(value, used),
        Type::SortedSet(value) => collect_type_names(value, used),
        Type::SkipList(value) => collect_type_names(value, used),
        Type::SortedMap(key, value) => {
            collect_type_names(key, used);
            collect_type_names(value, used);
        }
        Type::Graph(node, weight) => {
            collect_type_names(node, used);
            collect_type_names(weight, used);
        }
        Type::Range(value) => collect_type_names(value, used),
        Type::Conditional {
            check,
            bound,
            then,
            otherwise,
        } => {
            collect_type_names(check, used);
            collect_type_names(bound, used);
            collect_type_names(then, used);
            collect_type_names(otherwise, used);
        }
        Type::InferVar(_)
        | Type::Generic(_)
        | Type::Infer
        | Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::I128
        | Type::Isize
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::U128
        | Type::Usize
        | Type::F32
        | Type::F64
        | Type::F128
        | Type::Decimal
        | Type::Char
        | Type::Bool
        | Type::Regex
        | Type::Str
        | Type::Void
        | Type::Unknown
        | Type::Never
        | Type::FenwickTree
        | Type::BubbleSort
        | Type::SegmentTree
        | Type::SparseTable
        | Type::SegmentTreeLazy
        | Type::BitSet
        | Type::Trie
        | Type::BloomFilter
        | Type::SuffixTree
        | Type::Lca
        | Type::Hld
        | Type::Centroid => {}
    }
}

fn collect_match_pattern_names(pattern: &MatchPattern, used: &mut BTreeSet<String>) {
    match pattern {
        MatchPattern::Lit(expr) => collect_expr_names(expr, used),
        MatchPattern::Object(fields) => {
            for field in fields {
                if let Some(literal) = &field.literal {
                    collect_expr_names(literal, used);
                }
            }
        }
        MatchPattern::Array { elements, .. } => {
            for element in elements {
                collect_match_pattern_names(&element.pattern, used);
            }
        }
        MatchPattern::Or(patterns) => {
            for pattern in patterns {
                collect_match_pattern_names(pattern, used);
            }
        }
        MatchPattern::Wildcard | MatchPattern::Binding(_) | MatchPattern::ResultVariant { .. } => {}
        MatchPattern::Tuple(elements) => {
            for element in elements {
                collect_match_pattern_names(element, used);
            }
        }
        MatchPattern::EnumVariant { binding, .. } => {
            if let Some(binding) = binding {
                collect_match_pattern_names(binding, used);
            }
        }
        MatchPattern::Range { start, end, .. } => {
            if let Some(start) = start {
                collect_expr_names(start, used);
            }
            if let Some(end) = end {
                collect_expr_names(end, used);
            }
        }
    }
}

fn build_line_map(original: &str, formatted: &str) -> Vec<Option<usize>> {
    let orig_lines: Vec<&str> = original.lines().collect();
    let fmt_lines: Vec<&str> = formatted.lines().collect();

    fn normalize(s: &str) -> String {
        let mut normalized = String::with_capacity(s.len());
        for part in s.split_whitespace() {
            if !normalized.is_empty() {
                normalized.push(' ');
            }
            normalized.push_str(part);
        }
        normalized
    }

    let orig_code: Vec<(usize, String)> = orig_lines
        .iter()
        .enumerate()
        .filter_map(|(i, line)| {
            let t = line.trim();
            if t.is_empty() || t.starts_with("//") || t.starts_with("/*") {
                return None;
            }
            let code = if let Some(pos) = t.find("//") {
                t[..pos].trim_end()
            } else {
                t.trim_end()
            };
            Some((i, normalize(code)))
        })
        .collect();

    let fmt_code: Vec<(usize, String)> = fmt_lines
        .iter()
        .enumerate()
        .filter_map(|(i, line)| {
            let t = line.trim();
            if t.is_empty() {
                return None;
            }
            Some((i, normalize(t)))
        })
        .collect();

    let mut orig_to_fmt = vec![None; orig_lines.len()];
    let mut used_fmt: Vec<bool> = vec![false; fmt_lines.len()];
    let mut fmt_by_normalized: HashMap<String, VecDeque<usize>> = HashMap::new();
    for (fmt_idx, fmt_norm) in &fmt_code {
        fmt_by_normalized
            .entry(fmt_norm.clone())
            .or_default()
            .push_back(*fmt_idx);
    }
    let mut fmt_by_prefix: HashMap<(&str, &str), VecDeque<usize>> = HashMap::new();
    let mut fmt_by_start: HashMap<&str, VecDeque<usize>> = HashMap::new();
    for (fmt_idx, fmt_norm) in &fmt_code {
        let mut words = fmt_norm.split_whitespace();
        if let Some(first) = words.next() {
            fmt_by_prefix
                .entry((first, words.next().unwrap_or_default()))
                .or_default()
                .push_back(*fmt_idx);
        }
        if fmt_norm.chars().count() >= 5 {
            fmt_by_start
                .entry(fmt_norm.as_str())
                .or_default()
                .push_back(*fmt_idx);
        }
    }
    let mut fmt_by_pipeline: HashMap<String, VecDeque<usize>> = HashMap::new();
    for (fmt_idx, fmt_norm) in &fmt_code {
        if fmt_norm.starts_with("|>") {
            continue;
        }
        let mut combined = fmt_norm.clone();
        let mut next = *fmt_idx + 1;
        while next < fmt_lines.len() {
            let trimmed = fmt_lines[next].trim();
            if !trimmed.starts_with("|>") {
                break;
            }
            combined.push(' ');
            combined.push_str(&normalize(trimmed));
            next += 1;
        }
        fmt_by_pipeline
            .entry(combined)
            .or_default()
            .push_back(*fmt_idx);
    }

    for (orig_idx, orig_norm) in &orig_code {
        if let Some(fmt_idx) = fmt_by_normalized
            .get_mut(orig_norm)
            .and_then(VecDeque::pop_front)
        {
            orig_to_fmt[*orig_idx] = Some(fmt_idx);
            used_fmt[fmt_idx] = true;
        }
    }

    for (orig_idx, orig_norm) in &orig_code {
        if orig_to_fmt[*orig_idx].is_some() || !orig_norm.contains("|>") {
            continue;
        }
        let Some(candidates) = fmt_by_pipeline.get_mut(orig_norm) else {
            continue;
        };
        while let Some(fmt_idx) = candidates.pop_front() {
            if used_fmt[fmt_idx] {
                continue;
            }
            orig_to_fmt[*orig_idx] = Some(fmt_idx);
            used_fmt[fmt_idx] = true;
            break;
        }
    }

    for (orig_idx, orig_norm) in &orig_code {
        if orig_to_fmt[*orig_idx].is_some() {
            continue;
        }
        let mut combined = orig_norm.clone();
        let mut next = *orig_idx + 1;
        let mut has_pipe_segment = false;
        while next < orig_lines.len() {
            let trimmed = orig_lines[next].trim();
            if !trimmed.starts_with("|>") {
                break;
            }
            combined.push(' ');
            combined.push_str(&normalize(trimmed));
            has_pipe_segment = true;
            next += 1;
        }
        if !has_pipe_segment {
            continue;
        }
        let Some(candidates) = fmt_by_normalized.get_mut(&combined) else {
            continue;
        };
        while let Some(fmt_idx) = candidates.pop_front() {
            if used_fmt[fmt_idx] {
                continue;
            }
            orig_to_fmt[*orig_idx] = Some(fmt_idx);
            used_fmt[fmt_idx] = true;
            break;
        }
    }

    for (orig_idx, orig_norm) in &orig_code {
        if orig_to_fmt[*orig_idx].is_some() {
            continue;
        }
        let mut words = orig_norm.split_whitespace();
        let Some(first) = words.next() else {
            continue;
        };
        let prefix = (first, words.next().unwrap_or_default());
        if let Some(candidates) = fmt_by_prefix.get_mut(&prefix) {
            while let Some(fmt_idx) = candidates.pop_front() {
                if used_fmt[fmt_idx] {
                    continue;
                }
                orig_to_fmt[*orig_idx] = Some(fmt_idx);
                used_fmt[fmt_idx] = true;
                break;
            }
        }
    }

    for (orig_idx, orig_norm) in &orig_code {
        if orig_to_fmt[*orig_idx].is_some() {
            continue;
        }
        let mut best: Option<(usize, &str)> = None;
        for (start, character) in orig_norm.char_indices() {
            let end = start + character.len_utf8();
            if end < 5 {
                continue;
            }
            let prefix = &orig_norm[..end];
            let Some(candidates) = fmt_by_start.get_mut(prefix) else {
                continue;
            };
            while candidates.front().is_some_and(|fmt_idx| used_fmt[*fmt_idx]) {
                candidates.pop_front();
            }
            if let Some(fmt_idx) = candidates.front().copied()
                && best.is_none_or(|(best_idx, _)| fmt_idx < best_idx)
            {
                best = Some((fmt_idx, prefix));
            }
        }
        if let Some((fmt_idx, prefix)) = best {
            if let Some(candidates) = fmt_by_start.get_mut(prefix) {
                candidates.pop_front();
            }
            orig_to_fmt[*orig_idx] = Some(fmt_idx);
            used_fmt[fmt_idx] = true;
        }
    }

    orig_to_fmt
}

fn reinsert_comments(
    formatted: &str,
    orig_lines: &[&str],
    anchored: &Anchored,
    orig_to_fmt: &[Option<usize>],
    preserve_blank_lines: bool,
) -> String {
    let fmt_lines: Vec<&str> = formatted.lines().collect();

    let mut orig_blank_before: Vec<bool> = vec![false; orig_lines.len()];
    let mut blank_before_comments: Vec<bool> = vec![false; orig_lines.len()];
    let mut blank_after_comments: Vec<bool> = vec![false; orig_lines.len()];
    if preserve_blank_lines {
        for (i, _line) in orig_lines.iter().enumerate() {
            let trimmed = orig_lines[i].trim();
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
                continue;
            }
            if i == 0 {
                continue;
            }
            let mut j = i - 1;
            if orig_lines[j].trim().is_empty() {
                blank_after_comments[i] = true;
                j = j.saturating_sub(1);
            }
            while j > 0 {
                let t = orig_lines[j].trim();
                if t.starts_with("//") || t.starts_with("/*") {
                    j -= 1;
                } else {
                    break;
                }
            }
            if j > 0 && orig_lines[j].trim().is_empty() {
                blank_before_comments[i] = true;
            }
            orig_blank_before[i] = blank_before_comments[i] || blank_after_comments[i];
        }
    }

    let fmt_to_orig: Vec<Option<usize>> = {
        let mut rev = vec![None; fmt_lines.len()];
        for (orig, fmt) in orig_to_fmt.iter().enumerate() {
            if let Some(f) = fmt {
                rev[*f] = Some(orig);
            }
        }
        rev
    };

    let mut leading_for_fmt: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    let mut trailing_for_fmt: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    let mut unplaced: Vec<String> = Vec::new();

    for (&anchor_line, comments) in anchored {
        if let Some(fmt_line) = orig_to_fmt.get(anchor_line).copied().flatten() {
            for (is_trailing, text) in comments {
                if *is_trailing {
                    let fmt_line = trailing_comment_line(&fmt_lines, fmt_line);
                    trailing_for_fmt
                        .entry(fmt_line)
                        .or_default()
                        .push(text.clone());
                } else {
                    leading_for_fmt
                        .entry(fmt_line)
                        .or_default()
                        .push(text.clone());
                }
            }
        } else {
            let next = (anchor_line + 1..orig_to_fmt.len())
                .find(|&i| orig_to_fmt[i].is_some())
                .and_then(|i| orig_to_fmt[i]);
            if let Some(fl) = next {
                for (is_trailing, text) in comments {
                    if !is_trailing {
                        leading_for_fmt.entry(fl).or_default().push(text.clone());
                    }
                }
            } else {
                for (t, text) in comments {
                    if !t {
                        unplaced.push(text.clone());
                    }
                }
            }
        }
    }

    let mut result: Vec<String> = Vec::new();
    let mut last_was_blank = false;

    for (fmt_idx, line) in fmt_lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if !last_was_blank {
                result.push(String::new());
                last_was_blank = true;
            }
            continue;
        }
        last_was_blank = false;

        let indent = " ".repeat(line.len() - line.trim_start().len());

        if preserve_blank_lines
            && let Some(orig_idx) = fmt_to_orig[fmt_idx]
            && blank_before_comments[orig_idx]
            && result.last().is_some_and(|l| !l.trim().is_empty())
        {
            result.push(String::new());
        }

        if let Some(comments) = leading_for_fmt.remove(&fmt_idx) {
            for c in &comments {
                result.push(format!("{indent}{c}"));
            }
        }

        if preserve_blank_lines
            && let Some(orig_idx) = fmt_to_orig[fmt_idx]
            && blank_after_comments[orig_idx]
            && result.last().is_some_and(|l| !l.trim().is_empty())
        {
            result.push(String::new());
        }

        let is_code = !trimmed.starts_with("//") && !trimmed.starts_with("/*");
        if is_code {
            if let Some(comments) = trailing_for_fmt.remove(&fmt_idx) {
                result.push(format!("{}  {}", line.trim_end(), comments.join("  ")));
            } else {
                result.push(line.to_string());
            }
        } else {
            result.push(line.to_string());
        }
    }

    for c in &unplaced {
        result.push(c.clone());
    }
    for (_, comments) in leading_for_fmt {
        for c in &comments {
            result.push(c.clone());
        }
    }
    for (_, comments) in trailing_for_fmt {
        for c in &comments {
            result.push(c.clone());
        }
    }

    let mut out = result.join("\n");
    if formatted.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn trailing_comment_line(fmt_lines: &[&str], fmt_line: usize) -> usize {
    let mut end = fmt_line;
    let mut next = fmt_line + 1;
    while next < fmt_lines.len() && fmt_lines[next].trim_start().starts_with("|>") {
        end = next;
        next += 1;
    }
    end
}

fn preserve_blank_lines_only(
    formatted: &str,
    orig_lines: &[&str],
    orig_to_fmt: &[Option<usize>],
) -> String {
    let fmt_lines: Vec<&str> = formatted.lines().collect();

    let mut orig_blank_before: Vec<bool> = vec![false; orig_lines.len()];
    for (i, line) in orig_lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
            continue;
        }
        let blank_before = if i == 0 {
            false
        } else {
            let mut j = i - 1;
            while j > 0 {
                let t = orig_lines[j].trim();
                if t.starts_with("//") || t.starts_with("/*") {
                    j -= 1;
                } else {
                    break;
                }
            }
            orig_lines[j].trim().is_empty()
        };
        orig_blank_before[i] = blank_before;
    }

    let fmt_to_orig: Vec<Option<usize>> = {
        let mut rev = vec![None; fmt_lines.len()];
        for (orig, fmt) in orig_to_fmt.iter().enumerate() {
            if let Some(f) = fmt {
                rev[*f] = Some(orig);
            }
        }
        rev
    };

    let mut result: Vec<String> = Vec::new();
    let mut last_was_blank = false;

    for (fmt_idx, line) in fmt_lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if !last_was_blank {
                result.push(String::new());
                last_was_blank = true;
            }
            continue;
        }
        last_was_blank = false;

        if let Some(orig_idx) = fmt_to_orig[fmt_idx]
            && orig_blank_before[orig_idx]
            && result.last().is_some_and(|l| !l.trim().is_empty())
        {
            result.push(String::new());
        }

        result.push(line.to_string());
    }

    if formatted.ends_with('\n') && !result.last().is_none_or(std::string::String::is_empty) {
        result.push(String::new());
    }

    result.join("\n")
}

pub fn format_file(path: &Path, config_dir: Option<&Path>) -> Result<String, String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("error: could not read `{}`: {e}", path.display()))?;

    let dir = config_dir.unwrap_or_else(|| path.parent().unwrap_or(Path::new(".")));
    let config = Config::load(dir);

    Ok(format_source(&source, &config))
}

fn collect_tyrus_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let src_dir = dir.join("src");
    if !src_dir.is_dir() {
        return files;
    }
    collect_tyrus_files_rec(&src_dir, &mut files);
    files
}

fn collect_tyrus_files_rec(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_tyrus_files_rec(&path, files);
            } else if path.extension().is_some_and(|e| e == "tyr") {
                files.push(path);
            }
        }
    }
}

pub fn format_project(dir: &Path) -> Result<Vec<(String, String)>, String> {
    let config = Config::load(dir);
    let mut results = Vec::new();

    let files = collect_tyrus_files(dir);
    if files.is_empty() {
        return Err(format!(
            "error: no .tyr files found in '{}'",
            dir.join("src").display()
        ));
    }

    for file in files {
        let source = fs::read_to_string(&file)
            .map_err(|e| format!("error: could not read `{}`: {e}", file.display()))?;
        let formatted = format_source(&source, &config);
        results.push((file.to_string_lossy().to_string(), formatted));
    }

    Ok(results)
}

#[cfg(test)]
mod utility_type_fmt_tests {
    use super::*;
    use crate::config::Config;

    fn fmt(source: &str) -> String {
        format_source(source, &Config::default())
    }

    #[test]
    fn formatting_with_a_reused_program_matches_full_parsing() {
        let source = "import { unused, used } from \"./values\";\n\
                      const result:string=used; // keep this comment\n";
        let mut parser = Parser::from_source(source).expect("source should lex");
        let program = parser.parse_program().expect("source should parse");
        let config = Config::default();
        assert_eq!(
            format_source_with_program(source, &config, &program),
            format_source(source, &config)
        );
    }

    #[test]
    fn line_map_pairs_duplicate_lines_in_source_order() {
        assert_eq!(
            build_line_map(
                "const value = 1;\nconst value = 1;",
                "const value = 1;\nconst value = 1;"
            ),
            vec![Some(0), Some(1)]
        );
    }

    #[test]
    fn line_map_matches_pipeline_chains_in_either_layout() {
        assert_eq!(
            build_line_map(
                "const result = value |> first |> second;",
                "const result = value\n|> first\n|> second;"
            ),
            vec![Some(0)]
        );
        assert_eq!(
            build_line_map(
                "const result = value\n|> first\n|> second;",
                "const result = value |> first |> second;"
            ),
            vec![Some(0), None, None]
        );
    }

    #[test]
    fn formats_generic_and_utility_type_forms() {
        let out = fmt("type Ret<T> = T extends (x: i32) => infer R ? R : never;\n\
             interface Admin<T> extends User, Timestamps { role: T; keys: keyof T; }\n\
             type Cfg = Partial<Pick<User, keyof User>>;\n");
        assert!(out.contains("type Ret<T> ="));
        assert!(out.contains("T extends (i32) => infer R ? R : never"));
        assert!(out.contains("interface Admin<T> extends User, Timestamps"));
        assert!(out.contains("keyof T"));
        assert!(out.contains("Partial<Pick<User, keyof User>>"));
        assert_eq!(out, fmt(&out), "formatter must be idempotent");
    }

    #[test]
    fn preserves_compact_match_arms() {
        let out = fmt(
            "function classify(score: i32): string { match (score) { 1 => \"one\"; _ => \"other\"; } }",
        );
        assert!(out.contains("1 => \"one\";"), "{out}");
        assert!(out.contains("_ => \"other\";"), "{out}");
    }

    #[test]
    fn formats_behavior_interfaces_and_shorthand_bindings() {
        let out = fmt("interface User { age: u32; }\n\
             interface UserMethods at User { isAdult(user: User): boolean { return user.age >= 18; } }\n\
             const ageText = (user: User): string => user.age.toString();\n\
             interface UserText at User { ageText; }\n");
        assert!(out.contains("interface UserMethods at User"));
        assert!(out.contains("isAdult(user: User): boolean"));
        assert!(out.contains("interface UserText at User"));
        assert!(out.contains("ageText;"));
        assert_eq!(out, fmt(&out), "formatter must be idempotent");
    }

    #[test]
    fn formats_every_import_and_export_form() {
        let out = fmt(concat!(
            "import sum, { minus as sub } from \"./math\";\n",
            "import * as math from \"./math\";\n",
            "const local: i32 = sub(3, 1) + math.scale(2);\n",
            "export { local };\n",
            "export { local as alias };\n",
            "export { add as plus } from \"./math\";\n",
            "export * from \"./extra\";\n",
            "export const shown: i32 = 2;\n",
            "export default sum;\n",
        ));
        assert!(out.contains("import sum, { minus as sub } from \"./math\";"));
        assert!(out.contains("import * as math from \"./math\";"));
        assert!(out.contains("export { local, local as alias };"));
        assert!(out.contains("export { add as plus } from \"./math\";"));
        assert!(out.contains("export * from \"./extra\";"));
        assert!(out.contains("export const shown: i32 = 2;"));
        assert!(out.contains("export default sum;"));
        assert_eq!(out, fmt(&out), "formatter must be idempotent");
    }

    #[test]
    fn formats_the_protect_export_modifier() {
        let out = fmt(concat!(
            "const hidden: i32 = 1;\n",
            "const shared: i32 = 2;\n",
            "export protect { hidden, shared as alias };\n",
            "export { shared };\n",
            "export protect const near: i32 = 3;\n",
            "export protect function helper(): void {}\n",
        ));
        assert!(
            out.contains("export protect { hidden, shared as alias };"),
            "got: {out}"
        );
        assert!(
            out.contains("export { shared };"),
            "a public clause must not be merged into the protected one, got: {out}"
        );
        assert!(
            out.contains("export protect const near: i32 = 3;"),
            "got: {out}"
        );
        assert!(
            out.contains("export protect function helper()"),
            "got: {out}"
        );
        assert_eq!(out, fmt(&out), "formatter must be idempotent");
    }

    /// The printer used to drop `readonly` from every interface field, with any
    /// name, so running the formatter silently removed the guarantee.
    #[test]
    fn formats_keeps_readonly_on_an_interface_field() {
        let out = fmt("interface Config {\nreadonly id: i32;\nname: string;\n}\n");
        assert!(out.contains("readonly id: i32;"), "got: {out}");
        assert!(out.contains("name: string;"), "got: {out}");
        assert_eq!(out, fmt(&out), "formatter must be idempotent");
    }

    #[test]
    fn formats_reserved_words_used_as_field_names() {
        let out = fmt(concat!(
            "interface Event {\n",
            "readonly const: string;\n",
            "readonly: string;\n",
            "type: string;\n",
            "match?: i32;\n",
            "}\n",
            "type Inline = { type: string; interface: i32; };\n",
        ));
        assert!(out.contains("readonly const: string;"), "got: {out}");
        assert!(
            out.contains("\n    readonly: string;"),
            "a field named `readonly` must not gain a modifier, got: {out}"
        );
        assert!(out.contains("type: string;"), "got: {out}");
        assert!(out.contains("match?: i32;"), "got: {out}");
        assert!(out.contains("interface: i32;"), "got: {out}");
        assert_eq!(out, fmt(&out), "formatter must be idempotent");
    }

    #[test]
    fn an_export_clause_keeps_the_import_it_refers_to() {
        let out = fmt("import sum from \"./math\";\nexport default sum;\n");
        assert!(
            out.contains("import sum from \"./math\";"),
            "the import backing the export must survive, got: {out}"
        );
    }

    #[test]
    fn formats_dynamic_map_type() {
        let out = fmt("type Scores = Record<string, i32>;\n");
        assert!(out.contains("type Scores = Record<string, i32>"));
        assert_eq!(out, fmt(&out), "formatter must be idempotent");
    }

    #[test]
    fn formats_set_and_map_collections() {
        let out = fmt(
            "type Ids = Set<i32>;\ntype Scores = Map<string, i32>;\nconst run = (): void => {\nlet s: Ids = Set<i32>([1, 2]);\nlet m: Scores = Map<string, i32>();\nconsole.log(s.size);\nconsole.log(m.size);\n};\n",
        );
        assert!(out.contains("type Ids = Set<i32>"));
        assert!(out.contains("type Scores = Map<string, i32>"));
        assert!(out.contains("Set<i32>([1, 2])"));
        assert!(out.contains("Map<string, i32>()"));
        assert_eq!(out, fmt(&out), "formatter must be idempotent");
    }

    #[test]
    fn preserves_a_middle_switch_default() {
        let out = fmt(
            "switch (1) { case 1: { break; } default: { console.log(\"d\"); } case 2: { break; } }\n",
        );
        let first_case = out.find("case 1").unwrap();
        let default = out.find("default").unwrap();
        let second_case = out.find("case 2").unwrap();
        assert!(first_case < default && default < second_case);
        assert_eq!(out, fmt(&out), "formatter must be idempotent");
    }

    #[test]
    fn preserves_typed_numeric_literal_suffixes() {
        let out = fmt("const byte = 10u8; const ratio = 3.14f32;\n");
        assert!(out.contains("10u8"));
        assert!(out.contains("3.14f32"));
        assert_eq!(out, fmt(&out), "formatter must be idempotent");
    }
}
