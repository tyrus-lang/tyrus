use crate::ast::{Expr, ExprKind, Item, Program, Stmt, StmtKind, Type};
use crate::mentions::expr_mentions;

pub fn fuse(program: &mut Program) {
    for item in &mut program.items {
        match item {
            Item::FnDecl { body, .. } => fuse_block(body),
            Item::BehaviorInterfaceDecl { methods, .. } => {
                for method in methods {
                    fuse_block(&mut method.body);
                }
            }
            Item::Stmt(stmt) => fuse_stmt(stmt),
            Item::InterfaceDecl { .. } | Item::TypeAlias { .. } | Item::EnumDecl { .. } => {}
        }
    }
}

fn fuse_block(stmts: &mut Vec<Stmt>) {
    for stmt in stmts.iter_mut() {
        fuse_stmt(stmt);
    }
    fuse_push_runs(stmts);
}

fn fuse_stmt(stmt: &mut Stmt) {
    match &mut stmt.kind {
        StmtKind::VarDecl {
            value: Some(value), ..
        }
        | StmtKind::ArrayDestructure { value, .. }
        | StmtKind::ObjectDestructure { value, .. }
        | StmtKind::Assign { value, .. }
        | StmtKind::Throw(value)
        | StmtKind::Expr(value) => fuse_expr(value),
        StmtKind::VarDecl { value: None, .. } | StmtKind::Continue => {}
        StmtKind::Defer { body, .. } => fuse_block(body),
        StmtKind::Return(value) | StmtKind::Break(value) => {
            if let Some(value) = value {
                fuse_expr(value);
            }
        }
        StmtKind::IndexAssign { index, value, .. } => {
            fuse_expr(index);
            fuse_expr(value);
        }
        StmtKind::FieldAssign { object, value, .. } => {
            fuse_expr(object);
            fuse_expr(value);
        }
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            fuse_block(try_body);
            fuse_block(catch_body);
            if let Some(finally_body) = finally_body {
                fuse_block(finally_body);
            }
        }
        StmtKind::While { cond, body } | StmtKind::DoWhile { body, cond } => {
            fuse_expr(cond);
            fuse_block(body);
        }
        StmtKind::For {
            init,
            cond,
            update,
            body,
        } => {
            fuse_stmt(init);
            fuse_expr(cond);
            fuse_stmt(update);
            fuse_block(body);
        }
        StmtKind::ForOf { iterable, body, .. } => {
            fuse_expr(iterable);
            fuse_block(body);
        }
        StmtKind::ForIn { object, body, .. } => {
            fuse_expr(object);
            fuse_block(body);
        }
        StmtKind::Switch {
            expr,
            cases,
            default,
            ..
        } => {
            fuse_expr(expr);
            for case in cases {
                fuse_expr(&mut case.value);
                fuse_block(&mut case.body);
            }
            if let Some(default) = default {
                fuse_block(default);
            }
        }
        StmtKind::Match {
            expr,
            arms,
            default,
        } => {
            fuse_expr(expr);
            for arm in arms {
                if let Some(guard) = &mut arm.guard {
                    fuse_expr(guard);
                }
                fuse_block(&mut arm.body);
            }
            if let Some(default) = default {
                fuse_block(default);
            }
        }
    }
}

fn fuse_expr(expr: &mut Expr) {
    match &mut expr.kind {
        ExprKind::ArrowFn { body, .. } => fuse_block(body),
        ExprKind::Loop { body } => fuse_block(body),
        ExprKind::If {
            cond,
            then_block,
            else_block,
        } => {
            fuse_expr(cond);
            fuse_block(then_block);
            if let Some(else_block) = else_block {
                fuse_block(else_block);
            }
        }
        ExprKind::MethodCall { object, args, .. }
        | ExprKind::OptionalMethodCall { object, args, .. } => {
            fuse_expr(object);
            for arg in args {
                fuse_expr(arg);
            }
        }
        ExprKind::FnCall { args, .. } => {
            for arg in args {
                fuse_expr(arg);
            }
        }
        ExprKind::CallValue { callee, args, .. } => {
            fuse_expr(callee);
            for arg in args {
                fuse_expr(arg);
            }
        }
        ExprKind::BinOp { left, right, .. } | ExprKind::Pipeline { left, right } => {
            fuse_expr(left);
            fuse_expr(right);
        }
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            fuse_expr(cond);
            fuse_expr(then_expr);
            fuse_expr(else_expr);
        }
        ExprKind::ArrayLit(values) => {
            for value in values {
                fuse_expr(value);
            }
        }
        ExprKind::ObjectLit(fields) => {
            for (_, value) in fields {
                fuse_expr(value);
            }
        }
        ExprKind::ArrayRepeat { value, count } => {
            fuse_expr(value);
            fuse_expr(count);
        }
        ExprKind::Is { expr, .. } => fuse_expr(expr),
        ExprKind::Await(inner)
        | ExprKind::Try(inner)
        | ExprKind::Not(inner)
        | ExprKind::Neg(inner)
        | ExprKind::Spread(inner)
        | ExprKind::Unwrap(inner)
        | ExprKind::TypeAssert { value: inner, .. } => fuse_expr(inner),
        _ => {}
    }
}

fn fuse_push_runs(stmts: &mut Vec<Stmt>) {
    let mut index = 0;
    while index < stmts.len() {
        let Some(name) = empty_array_declaration(&stmts[index]) else {
            index += 1;
            continue;
        };
        let name = name.to_string();
        let mut pushed = Vec::new();
        let mut next = index + 1;
        while let Some(element) = pushed_element(stmts.get(next), &name) {
            pushed.push(element.clone());
            next += 1;
        }
        if pushed.is_empty() {
            index += 1;
            continue;
        }
        let StmtKind::VarDecl {
            value: Some(value), ..
        } = &mut stmts[index].kind
        else {
            unreachable!("empty_array_declaration matched a declaration with an initializer");
        };
        value.kind = ExprKind::ArrayLit(pushed);
        stmts.drain(index + 1..next);
        index += 1;
    }
}

fn empty_array_declaration(stmt: &Stmt) -> Option<&str> {
    let StmtKind::VarDecl {
        mutable: true,
        name,
        ty,
        value: Some(value),
        onfail: None,
    } = stmt.kind()
    else {
        return None;
    };
    let Type::Array(element) = ty else {
        return None;
    };
    if !element.is_copy() {
        return None;
    }
    match value.kind() {
        ExprKind::ArrayLit(values) if values.is_empty() => Some(name),
        _ => None,
    }
}

fn pushed_element<'a>(stmt: Option<&'a Stmt>, name: &str) -> Option<&'a Expr> {
    let StmtKind::Expr(expr) = stmt?.kind() else {
        return None;
    };
    let ExprKind::MethodCall {
        object,
        method,
        type_args,
        args,
        ..
    } = expr.kind()
    else {
        return None;
    };
    if method != "push" || !type_args.is_empty() || args.len() != 1 {
        return None;
    }
    if !matches!(object.kind(), ExprKind::Ident(object_name) if object_name == name) {
        return None;
    }
    let element = &args[0];
    if matches!(element.kind(), ExprKind::Spread(_)) || expr_mentions(element, name) {
        return None;
    }
    Some(element)
}

#[cfg(test)]
#[path = "../../tests/unit/array_literal_fusion.rs"]
mod tests;
