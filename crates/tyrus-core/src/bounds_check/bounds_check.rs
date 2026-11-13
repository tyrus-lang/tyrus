use crate::ast::{
    AssignOp, BinOp, Expr, ExprKind, Item, MatchPattern, Param, Program, Stmt, StmtKind,
};
use crate::collections::{HashMap, HashSet};
use crate::escape_analysis::Subject;
use crate::escape_analysis::{child_expressions, expression_bodies, leaves_frame, nested_bodies};
use crate::span::Span;

#[derive(Debug, Default, Clone)]
pub struct CheckedIndexes {
    spans: HashSet<Span>,
}

impl CheckedIndexes {
    pub fn contains(&self, span: Span) -> bool {
        !span.is_dummy() && self.spans.contains(&span)
    }

    pub fn len(&self) -> usize {
        self.spans.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }
}

pub fn analyze(program: &Program) -> CheckedIndexes {
    let mut checked = CheckedIndexes::default();
    let globals = fixed_length_globals(program);
    for item in &program.items {
        match item {
            Item::FnDecl { params, body, .. } => {
                let bound = bound_parameters(params);
                let mut lengths = globals.clone();
                for name in &bound {
                    lengths.remove(name);
                }
                analyze_body(body, &bound, &lengths, &mut checked);
            }
            Item::Stmt(stmt) => {
                analyze_nested_bodies(stmt, &HashSet::default(), &globals, &mut checked);
            }
            Item::InterfaceDecl { .. }
            | Item::BehaviorInterfaceDecl { .. }
            | Item::TypeAlias { .. }
            | Item::EnumDecl { .. } => {}
        }
    }
    let top_level = top_level_statements(program);
    for stmt in &top_level {
        collect_constant_indexes(stmt, &globals, &mut checked);
    }
    checked
}

fn fixed_length_globals(program: &Program) -> HashMap<String, usize> {
    let top_level = top_level_statements(program);
    let mut lengths = HashMap::default();
    for stmt in &top_level {
        let StmtKind::VarDecl {
            mutable: false,
            name,
            value: Some(value),
            ..
        } = stmt.kind()
        else {
            continue;
        };
        let Some(len) = literal_element_count(value) else {
            continue;
        };
        if mentions_beyond_reading(name, &top_level, program) {
            continue;
        }
        lengths.insert(name.clone(), len);
    }
    lengths
}

fn top_level_statements(program: &Program) -> Vec<Stmt> {
    program
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Stmt(stmt) => Some(stmt.clone()),
            _ => None,
        })
        .collect()
}

fn mentions_beyond_reading(name: &str, top_level: &[Stmt], program: &Program) -> bool {
    leaves_frame(name, top_level, Subject::Array)
        || program.items.iter().any(|item| match item {
            Item::FnDecl { body, .. } => leaves_frame(name, body, Subject::Array),
            Item::Stmt(_)
            | Item::InterfaceDecl { .. }
            | Item::BehaviorInterfaceDecl { .. }
            | Item::TypeAlias { .. }
            | Item::EnumDecl { .. } => false,
        })
}

fn literal_element_count(value: &Expr) -> Option<usize> {
    match value.kind() {
        ExprKind::ArrayLit(elements) => (!elements
            .iter()
            .any(|element| matches!(element.kind(), ExprKind::Spread(_))))
        .then_some(elements.len()),
        _ => None,
    }
}

fn bound_parameters(params: &[Param]) -> HashSet<String> {
    params
        .iter()
        .flat_map(|param| param.pattern.binding_names())
        .map(str::to_string)
        .collect()
}

fn analyze_body(
    body: &[Stmt],
    outer_locals: &HashSet<String>,
    outer_lengths: &HashMap<String, usize>,
    checked: &mut CheckedIndexes,
) {
    let mut locals = outer_locals.clone();
    let mut lengths = outer_lengths.clone();
    for stmt in body {
        if let StmtKind::For {
            init,
            cond,
            update,
            body: loop_body,
        } = stmt.kind()
            && let Some((array, counter)) =
                proven_loop(init, cond, update, loop_body, &locals, &lengths)
        {
            collect_indexes(loop_body, array, counter, checked);
        }
        let mut inner_lengths = lengths.clone();
        for name in names_bound_within(stmt) {
            inner_lengths.remove(&name);
        }
        collect_constant_indexes(stmt, &inner_lengths, checked);
        analyze_nested_bodies(stmt, &locals, &inner_lengths, checked);
        declare(stmt, &mut locals);
        declare_length(stmt, body, &mut lengths);
    }
}

fn analyze_nested_bodies(
    stmt: &Stmt,
    locals: &HashSet<String>,
    lengths: &HashMap<String, usize>,
    checked: &mut CheckedIndexes,
) {
    if let StmtKind::For { init, body, .. } = stmt.kind() {
        let mut inner = locals.clone();
        declare(init, &mut inner);
        let mut inner_lengths = lengths.clone();
        declare_length(init, body, &mut inner_lengths);
        analyze_body(body, &inner, &inner_lengths, checked);
        return;
    }
    for nested in nested_bodies(stmt) {
        analyze_body(nested, locals, lengths, checked);
    }
    for (nested, _) in expression_bodies(stmt) {
        analyze_body(&nested, locals, lengths, checked);
    }
}

fn declare(stmt: &Stmt, locals: &mut HashSet<String>) {
    match stmt.kind() {
        StmtKind::VarDecl { name, .. } => {
            locals.insert(name.clone());
        }
        StmtKind::ArrayDestructure { bindings, .. } => {
            for binding in bindings {
                locals.insert(binding.name.clone());
            }
        }
        StmtKind::ObjectDestructure { bindings, .. } => {
            for binding in bindings {
                locals.insert(binding.name.clone());
            }
        }
        _ => {}
    }
}

fn declare_length(stmt: &Stmt, enclosing: &[Stmt], lengths: &mut HashMap<String, usize>) {
    match stmt.kind() {
        StmtKind::VarDecl {
            mutable,
            name,
            value,
            ..
        } => {
            let len = (!mutable)
                .then_some(value.as_ref())
                .flatten()
                .and_then(literal_element_count)
                .filter(|_| !leaves_frame(name, enclosing, Subject::Array));
            match len {
                Some(len) => lengths.insert(name.clone(), len),
                None => lengths.remove(name),
            };
        }
        StmtKind::ArrayDestructure { bindings, .. } => {
            for binding in bindings {
                lengths.remove(&binding.name);
            }
        }
        StmtKind::ObjectDestructure { bindings, .. } => {
            for binding in bindings {
                lengths.remove(&binding.name);
            }
        }
        StmtKind::ForOf { name, .. } | StmtKind::ForIn { name, .. } => {
            lengths.remove(name);
        }
        StmtKind::TryCatch { catch_binding, .. } => {
            lengths.remove(catch_binding);
        }
        StmtKind::Assign { .. }
        | StmtKind::IndexAssign { .. }
        | StmtKind::FieldAssign { .. }
        | StmtKind::Return(_)
        | StmtKind::Break(_)
        | StmtKind::Continue
        | StmtKind::Throw(_)
        | StmtKind::Expr(_)
        | StmtKind::While { .. }
        | StmtKind::DoWhile { .. }
        | StmtKind::For { .. }
        | StmtKind::Switch { .. }
        | StmtKind::Match { .. }
        | StmtKind::Defer { .. } => {}
    }
}

fn collect_constant_indexes(
    stmt: &Stmt,
    lengths: &HashMap<String, usize>,
    checked: &mut CheckedIndexes,
) {
    if let StmtKind::IndexAssign { name, index, .. } = stmt.kind()
        && let Some(len) = lengths.get(name)
    {
        record_constant(index, *len, checked);
    }
    for expr in statement_expressions(stmt) {
        collect_constant_in_expression(expr, lengths, checked);
    }
}

fn collect_constant_in_expression(
    expr: &Expr,
    lengths: &HashMap<String, usize>,
    checked: &mut CheckedIndexes,
) {
    if let ExprKind::IndexAccess { object, index } = expr.kind()
        && let ExprKind::Ident(name) = object.kind()
        && let Some(len) = lengths.get(name)
    {
        record_constant(index, *len, checked);
    }
    for child in child_expressions(expr) {
        collect_constant_in_expression(child, lengths, checked);
    }
}

fn record_constant(index: &Expr, len: usize, checked: &mut CheckedIndexes) {
    let value = match index.kind() {
        ExprKind::IntLit(value) if *value >= 0 => *value as u128,
        ExprKind::UIntLit(value) => *value,
        _ => return,
    };
    if value < len as u128 && !index.span.is_dummy() {
        checked.spans.insert(index.span);
    }
}

fn proven_loop<'a>(
    init: &'a Stmt,
    cond: &'a Expr,
    update: &'a Stmt,
    body: &'a [Stmt],
    locals: &HashSet<String>,
    lengths: &HashMap<String, usize>,
) -> Option<(&'a str, &'a str)> {
    let counter = counter_declaration(init)?;
    let array = compared_against_length(cond, counter)?;
    if !locals.contains(array) && !lengths.contains_key(array) {
        return None;
    }
    if !increments_counter(update, counter) {
        return None;
    }
    if leaves_frame(array, body, Subject::Array) {
        return None;
    }
    if leaves_frame(counter, body, Subject::Counter) {
        return None;
    }
    if binds_name(body, array) || binds_name(body, counter) {
        return None;
    }
    Some((array, counter))
}

fn counter_declaration(init: &Stmt) -> Option<&str> {
    let StmtKind::VarDecl {
        mutable: true,
        name,
        value: Some(value),
        ..
    } = init.kind()
    else {
        return None;
    };
    non_negative_literal(value).then_some(name.as_str())
}

fn non_negative_literal(expr: &Expr) -> bool {
    matches!(expr.kind(), ExprKind::IntLit(value) if *value >= 0)
        || matches!(expr.kind(), ExprKind::UIntLit(_))
}

fn positive_literal(expr: &Expr) -> bool {
    matches!(expr.kind(), ExprKind::IntLit(value) if *value > 0)
        || matches!(expr.kind(), ExprKind::UIntLit(value) if *value > 0)
}

fn compared_against_length<'a>(cond: &'a Expr, counter: &str) -> Option<&'a str> {
    let ExprKind::BinOp {
        op: BinOp::Lt,
        left,
        right,
    } = cond.kind()
    else {
        return None;
    };
    if !matches!(left.kind(), ExprKind::Ident(name) if name == counter) {
        return None;
    }
    let ExprKind::FieldAccess { object, field } = right.kind() else {
        return None;
    };
    if field != "length" {
        return None;
    }
    match object.kind() {
        ExprKind::Ident(name) => Some(name.as_str()),
        _ => None,
    }
}

fn increments_counter(update: &Stmt, counter: &str) -> bool {
    match update.kind() {
        StmtKind::Assign {
            name,
            op: AssignOp::AddAssign,
            value,
        } => name == counter && positive_literal(value),
        StmtKind::Expr(expr) => match expr.kind() {
            ExprKind::PrefixIncr(inner) | ExprKind::PostfixIncr(inner) => {
                matches!(inner.kind(), ExprKind::Ident(name) if name == counter)
            }
            _ => false,
        },
        _ => false,
    }
}

fn binds_name(body: &[Stmt], name: &str) -> bool {
    body.iter().any(|stmt| statement_binds_name(stmt, name))
}

fn statement_binds_name(stmt: &Stmt, name: &str) -> bool {
    let mut bound = HashSet::default();
    statement_own_bindings(stmt, &mut bound);
    bound.contains(name)
        || nested_bodies(stmt)
            .into_iter()
            .any(|nested| binds_name(nested, name))
        || expression_bodies(stmt)
            .into_iter()
            .any(|(nested, _)| binds_name(&nested, name))
}

fn names_bound_within(stmt: &Stmt) -> HashSet<String> {
    let mut bound = HashSet::default();
    statement_own_bindings(stmt, &mut bound);
    for nested in nested_bodies(stmt) {
        for inner in nested {
            bound.extend(names_bound_within(inner));
        }
    }
    for (nested, _) in expression_bodies(stmt) {
        for inner in &nested {
            bound.extend(names_bound_within(inner));
        }
    }
    bound
}

fn statement_own_bindings(stmt: &Stmt, bound: &mut HashSet<String>) {
    match stmt.kind() {
        StmtKind::VarDecl { name, .. }
        | StmtKind::ForOf { name, .. }
        | StmtKind::ForIn { name, .. } => {
            bound.insert(name.clone());
        }
        StmtKind::ArrayDestructure { bindings, .. } => {
            bound.extend(bindings.iter().map(|binding| binding.name.clone()));
        }
        StmtKind::ObjectDestructure { bindings, .. } => {
            bound.extend(bindings.iter().map(|binding| binding.name.clone()));
        }
        StmtKind::TryCatch { catch_binding, .. } => {
            bound.insert(catch_binding.clone());
        }
        StmtKind::For { init, .. } => statement_own_bindings(init, bound),
        StmtKind::Match { arms, .. } => {
            for arm in arms {
                pattern_bindings(&arm.pattern, bound);
            }
        }
        StmtKind::Assign { .. }
        | StmtKind::IndexAssign { .. }
        | StmtKind::FieldAssign { .. }
        | StmtKind::Return(_)
        | StmtKind::Break(_)
        | StmtKind::Continue
        | StmtKind::Throw(_)
        | StmtKind::Expr(_)
        | StmtKind::While { .. }
        | StmtKind::DoWhile { .. }
        | StmtKind::Switch { .. }
        | StmtKind::Defer { .. } => {}
    }
}

fn pattern_bindings(pattern: &MatchPattern, bound: &mut HashSet<String>) {
    match pattern {
        MatchPattern::Binding(name) => {
            bound.insert(name.clone());
        }
        MatchPattern::ResultVariant {
            binding: Some(name),
            ..
        } => {
            bound.insert(name.clone());
        }
        MatchPattern::Object(fields) => {
            for field in fields {
                bound.insert(field.binding.clone().unwrap_or_else(|| field.key.clone()));
            }
        }
        MatchPattern::Array { elements, .. } => {
            for element in elements {
                pattern_bindings(&element.pattern, bound);
            }
        }
        MatchPattern::Or(patterns) => {
            for pattern in patterns {
                pattern_bindings(pattern, bound);
            }
        }
        MatchPattern::EnumVariant { binding, .. } => {
            if let Some(binding) = binding {
                pattern_bindings(binding, bound);
            }
        }
        MatchPattern::Tuple(patterns) => {
            for pattern in patterns {
                pattern_bindings(pattern, bound);
            }
        }
        MatchPattern::Lit(_)
        | MatchPattern::Wildcard
        | MatchPattern::ResultVariant { binding: None, .. }
        | MatchPattern::Range { .. } => {}
    }
}

fn collect_indexes(body: &[Stmt], array: &str, counter: &str, checked: &mut CheckedIndexes) {
    for stmt in body {
        if let StmtKind::IndexAssign { name, index, .. } = stmt.kind()
            && name == array
        {
            record(index, counter, checked);
        }
        for expr in statement_expressions(stmt) {
            collect_in_expression(expr, array, counter, checked);
        }
        for nested in nested_bodies(stmt) {
            collect_indexes(nested, array, counter, checked);
        }
        for (nested, _) in expression_bodies(stmt) {
            collect_indexes(&nested, array, counter, checked);
        }
    }
}

fn collect_in_expression(expr: &Expr, array: &str, counter: &str, checked: &mut CheckedIndexes) {
    if let ExprKind::IndexAccess { object, index } = expr.kind()
        && matches!(object.kind(), ExprKind::Ident(name) if name == array)
    {
        record(index, counter, checked);
    }
    for child in child_expressions(expr) {
        collect_in_expression(child, array, counter, checked);
    }
}

fn record(index: &Expr, counter: &str, checked: &mut CheckedIndexes) {
    if matches!(index.kind(), ExprKind::Ident(name) if name == counter) && !index.span.is_dummy() {
        checked.spans.insert(index.span);
    }
}

fn statement_expressions(stmt: &Stmt) -> Vec<&Expr> {
    match stmt.kind() {
        StmtKind::VarDecl { value, .. } | StmtKind::Return(value) | StmtKind::Break(value) => {
            value.iter().collect()
        }
        StmtKind::Assign { value, .. }
        | StmtKind::Throw(value)
        | StmtKind::Expr(value)
        | StmtKind::ArrayDestructure { value, .. }
        | StmtKind::ObjectDestructure { value, .. } => vec![value],
        StmtKind::IndexAssign { index, value, .. } => vec![index, value],
        StmtKind::FieldAssign { object, value, .. } => vec![object, value],
        StmtKind::While { cond, .. } | StmtKind::DoWhile { cond, .. } => vec![cond],
        StmtKind::For {
            init, cond, update, ..
        } => {
            let mut expressions = vec![cond];
            expressions.extend(statement_expressions(init));
            expressions.extend(statement_expressions(update));
            expressions
        }
        StmtKind::ForOf { iterable, .. } => vec![iterable],
        StmtKind::ForIn { object, .. } => vec![object],
        StmtKind::Switch { expr, cases, .. } => {
            let mut expressions = vec![expr];
            expressions.extend(cases.iter().map(|case| &case.value));
            expressions
        }
        StmtKind::Match { expr, arms, .. } => {
            let mut expressions = vec![expr];
            expressions.extend(arms.iter().filter_map(|arm| arm.guard.as_ref()));
            expressions
        }
        StmtKind::TryCatch { .. } | StmtKind::Defer { .. } | StmtKind::Continue => Vec::new(),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/bounds_check.rs"]
mod tests;
