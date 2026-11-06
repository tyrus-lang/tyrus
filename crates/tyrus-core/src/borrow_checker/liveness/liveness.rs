use crate::ast::*;
use crate::collections::{HashMap, HashSet};
use crate::mentions::{mention_expr, mention_stmt};

pub(crate) fn closure_capture_names(params: &[Param], body: &[Stmt]) -> HashSet<String> {
    fn expr_refs(expr: &Expr, names: &mut HashSet<String>) {
        match expr.kind() {
            ExprKind::Ident(name) | ExprKind::Borrow { name, .. } => {
                names.insert(name.clone());
            }
            ExprKind::CallValue { callee, args, .. } => {
                expr_refs(callee, names);
                for arg in args {
                    expr_refs(arg, names);
                }
            }
            ExprKind::FnCall { callee, args, .. } => {
                names.insert(callee.clone());
                for arg in args {
                    expr_refs(arg, names);
                }
            }
            ExprKind::ObjectLit(fields) => {
                for (_, value) in fields {
                    expr_refs(value, names);
                }
            }
            ExprKind::ObjectUpdate { base, fields } => {
                expr_refs(base, names);
                for (_, value) in fields {
                    expr_refs(value, names);
                }
            }
            ExprKind::ArrayLit(values) => {
                for value in values {
                    expr_refs(value, names);
                }
            }
            ExprKind::ArrayRepeat { value, count } => {
                expr_refs(value, names);
                expr_refs(count, names);
            }
            ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
                expr_refs(object, names);
            }
            ExprKind::IndexAccess { object, index } => {
                expr_refs(object, names);
                expr_refs(index, names);
            }
            ExprKind::BinOp { left, right, .. } => {
                expr_refs(left, names);
                expr_refs(right, names);
            }
            ExprKind::ChainedComparison { terms, .. } => {
                for term in terms {
                    expr_refs(term, names);
                }
            }
            ExprKind::Pipeline { left, right } => {
                expr_refs(left, names);
                expr_refs(right, names);
            }
            ExprKind::Not(v)
            | ExprKind::Neg(v)
            | ExprKind::BitNot(v)
            | ExprKind::Typeof(v)
            | ExprKind::Await(v)
            | ExprKind::Try(v)
            | ExprKind::Unwrap(v)
            | ExprKind::Spread(v) => expr_refs(v, names),
            ExprKind::TypeAssert { value, .. }
            | ExprKind::Is { expr: value, .. }
            | ExprKind::ConstAssert(value)
            | ExprKind::Comptime(value) => expr_refs(value, names),
            ExprKind::PrefixIncr(v)
            | ExprKind::PrefixDecr(v)
            | ExprKind::PostfixIncr(v)
            | ExprKind::PostfixDecr(v) => expr_refs(v, names),
            ExprKind::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                expr_refs(cond, names);
                expr_refs(then_expr, names);
                expr_refs(else_expr, names);
            }
            ExprKind::TemplateLit(parts) => {
                for part in parts {
                    if let TemplatePart::Expr(e, _) = part {
                        expr_refs(e, names);
                    }
                }
            }
            ExprKind::MethodCall { object, args, .. }
            | ExprKind::OptionalMethodCall { object, args, .. } => {
                expr_refs(object, names);
                for arg in args {
                    expr_refs(arg, names);
                }
            }
            ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => {
                if let Some(initial) = initial {
                    expr_refs(initial, names);
                }
            }
            ExprKind::ArrowFn { .. } => {}
            ExprKind::If {
                cond,
                then_block,
                else_block,
            } => {
                expr_refs(cond, names);
                stmt_refs(then_block, names);
                if let Some(block) = else_block {
                    stmt_refs(block, names);
                }
            }
            ExprKind::Loop { body } => stmt_refs(body, names),
            ExprKind::In { key, object } => {
                expr_refs(key, names);
                expr_refs(object, names);
            }
            ExprKind::Range { start, end, .. } => {
                expr_refs(start, names);
                expr_refs(end, names);
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

    fn stmt_refs(body: &[Stmt], names: &mut HashSet<String>) {
        for stmt in body {
            match stmt.kind() {
                StmtKind::VarDecl { value, .. } => {
                    if let Some(value) = value {
                        expr_refs(value, names);
                    }
                }
                StmtKind::Assign { value, .. }
                | StmtKind::Return(Some(value))
                | StmtKind::Expr(value) => expr_refs(value, names),
                StmtKind::IndexAssign { name, index, value } => {
                    names.insert(name.clone());
                    expr_refs(index, names);
                    expr_refs(value, names);
                }
                StmtKind::FieldAssign { object, value, .. } => {
                    expr_refs(object, names);
                    expr_refs(value, names);
                }
                StmtKind::While { cond, body } => {
                    expr_refs(cond, names);
                    stmt_refs(body, names);
                }
                StmtKind::DoWhile { body, cond } => {
                    stmt_refs(body, names);
                    expr_refs(cond, names);
                }
                StmtKind::For {
                    init,
                    cond,
                    update,
                    body,
                } => {
                    stmt_refs(std::slice::from_ref(init), names);
                    expr_refs(cond, names);
                    stmt_refs(std::slice::from_ref(update), names);
                    stmt_refs(body, names);
                }
                StmtKind::ForOf { iterable, body, .. } => {
                    expr_refs(iterable, names);
                    stmt_refs(body, names);
                }
                StmtKind::ForIn { object, body, .. } => {
                    expr_refs(object, names);
                    stmt_refs(body, names);
                }
                StmtKind::Return(None) | StmtKind::Continue => {}
                StmtKind::Break(value) => {
                    if let Some(value) = value {
                        expr_refs(value, names);
                    }
                }
                StmtKind::ArrayDestructure {
                    bindings, value, ..
                }
                | StmtKind::ObjectDestructure {
                    bindings, value, ..
                } => {
                    for b in bindings {
                        names.insert(b.name.clone());
                    }
                    expr_refs(value, names);
                }
                StmtKind::Switch {
                    expr,
                    cases,
                    default,
                    ..
                } => {
                    expr_refs(expr, names);
                    for case in cases {
                        expr_refs(&case.value, names);
                        stmt_refs(&case.body, names);
                    }
                    if let Some(body) = default {
                        stmt_refs(body, names);
                    }
                }
                StmtKind::Match {
                    expr,
                    arms,
                    default,
                } => {
                    expr_refs(expr, names);
                    for arm in arms {
                        stmt_refs(&arm.body, names);
                        if let Some(guard) = &arm.guard {
                            expr_refs(guard, names);
                        }
                    }
                    if let Some(body) = default {
                        stmt_refs(body, names);
                    }
                }
                StmtKind::Throw(expr) => {
                    expr_refs(expr, names);
                }
                StmtKind::TryCatch {
                    try_body,
                    catch_body,
                    finally_body,
                    ..
                } => {
                    stmt_refs(try_body, names);
                    stmt_refs(catch_body, names);
                    if let Some(body) = finally_body {
                        stmt_refs(body, names);
                    }
                }
                StmtKind::Defer { body, .. } => stmt_refs(body, names),
            }
        }
    }

    let mut refs = HashSet::default();
    stmt_refs(body, &mut refs);
    for p in params {
        for name in p.pattern.binding_names() {
            refs.remove(name);
        }
    }
    for stmt in body {
        match stmt.kind() {
            StmtKind::VarDecl { name, .. } => {
                refs.remove(name);
            }
            StmtKind::ArrayDestructure { bindings, .. }
            | StmtKind::ObjectDestructure { bindings, .. } => {
                for b in bindings {
                    refs.remove(&b.name);
                }
            }
            _ => {}
        }
    }
    refs
}

pub(super) fn last_mentions(stmts: &[Stmt]) -> HashMap<String, usize> {
    let mut last = HashMap::default();
    let mut carriers: Vec<(Vec<String>, Vec<String>)> = Vec::new();
    for (index, stmt) in stmts.iter().enumerate() {
        let mut defined = Vec::new();
        defined_names(stmt, &mut |name| defined.push(name.to_string()));
        let carries = !defined.is_empty();
        let mut mentioned = Vec::new();
        mention_stmt(stmt, &mut |name| {
            if let Some(slot) = last.get_mut(name) {
                *slot = index;
            } else {
                last.insert(name.to_string(), index);
            }
            if carries {
                mentioned.push(name.to_string());
            }
        });
        if carries {
            carriers.push((defined, mentioned));
        }
    }

    if let Some(last_index) = stmts.len().checked_sub(1) {
        for stmt in stmts {
            if !matches!(stmt.kind(), StmtKind::Defer { .. }) {
                continue;
            }
            mention_stmt(stmt, &mut |name| {
                if let Some(slot) = last.get_mut(name) {
                    *slot = last_index;
                } else {
                    last.insert(name.to_string(), last_index);
                }
            });
        }
    }

    loop {
        let mut changed = false;
        for (defined, mentioned) in &carriers {
            let Some(holder_last) = defined
                .iter()
                .filter_map(|name| last.get(name))
                .max()
                .copied()
            else {
                continue;
            };
            for name in mentioned {
                let entry = last.entry(name.clone()).or_insert(holder_last);
                if *entry < holder_last {
                    *entry = holder_last;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    last
}

pub(super) fn defined_names(stmt: &Stmt, visit: &mut dyn FnMut(&str)) {
    match stmt.kind() {
        StmtKind::VarDecl { name, .. }
        | StmtKind::Assign { name, .. }
        | StmtKind::IndexAssign { name, .. } => visit(name),
        StmtKind::ArrayDestructure { bindings, .. }
        | StmtKind::ObjectDestructure { bindings, .. } => {
            for binding in bindings {
                visit(&binding.name);
            }
        }
        StmtKind::FieldAssign { object, .. } => mention_expr(object, visit),
        StmtKind::Return(_)
        | StmtKind::Break(_)
        | StmtKind::Continue
        | StmtKind::Defer { .. }
        | StmtKind::Throw(_)
        | StmtKind::Expr(_)
        | StmtKind::TryCatch { .. }
        | StmtKind::While { .. }
        | StmtKind::DoWhile { .. }
        | StmtKind::For { .. }
        | StmtKind::ForOf { .. }
        | StmtKind::ForIn { .. }
        | StmtKind::Switch { .. }
        | StmtKind::Match { .. } => {}
    }
}
