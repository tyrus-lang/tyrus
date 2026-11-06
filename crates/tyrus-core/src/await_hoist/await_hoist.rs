//! Moves an `await` out of an expression position the async lowering cannot
//! suspend in, such as `match (await load())`, `return Ok((await load())? + 1)`
//! or `show(await load())` inside a loop, into a `const` placed right before
//! the statement, where every `await` is supported:
//!
//! ```text
//! match (await load()) { ... }   =>   const await#0 = await load();
//!                                     match (await#0) { ... }
//! ```
//!
//! The type checker and the code generator both run on the rewritten program,
//! so what one accepts the other can lower. An `await` is moved only when it is
//! the first thing in its statement with an observable effect: everything
//! evaluated before it must be a literal, a name or pure arithmetic over them.
//! Moving it past a call, an increment, a `?` or an unwrap would reorder
//! effects, and an `await` in a branch that may not run (`&&`, `||`, `??`, a
//! ternary arm) must not run unconditionally; those stay where they are and
//! keep their existing diagnostic.

use crate::ast::{
    BinOp, Expr, ExprKind, Item, OnFailHandler, Program, Stmt, StmtKind, TemplatePart, Type,
};
use crate::async_shapes::collect_await_expressions;

/// Prefix of the names given to hoisted `await` results. `#` cannot appear in
/// an identifier the lexer accepts, so the name never collides with the
/// program's own.
pub const HOISTED_AWAIT_PREFIX: &str = "await#";

pub fn is_hoisted_await_name(name: &str) -> bool {
    name.starts_with(HOISTED_AWAIT_PREFIX)
}

pub fn hoist_nested_awaits(program: &mut Program) {
    let mut counter = 0usize;
    for item in &mut program.items {
        match item {
            Item::FnDecl { is_async, body, .. } => hoist_block(body, *is_async, &mut counter),
            Item::Stmt(statement) => visit_nested(statement, false, &mut counter),
            _ => {}
        }
    }
}

fn hoist_block(block: &mut Vec<Stmt>, in_async: bool, counter: &mut usize) {
    let original = std::mem::take(block);
    for mut statement in original {
        visit_nested(&mut statement, in_async, counter);
        if in_async {
            while let Some(hoisted) = hoist_one(&mut statement, counter) {
                block.push(hoisted);
            }
        }
        block.push(statement);
    }
}

/// Rewrites the blocks nested in `statement`: its own sub-blocks keep the
/// enclosing function's `async`-ness, a closure body takes its own.
fn visit_nested(statement: &mut Stmt, in_async: bool, counter: &mut usize) {
    match &mut statement.kind {
        StmtKind::VarDecl { value, onfail, .. } => {
            if let Some(value) = value {
                visit_expr_blocks(value, in_async, counter);
            }
            if let Some(onfail) = onfail {
                match &mut onfail.handler {
                    OnFailHandler::Block(body) => hoist_block(body, in_async, counter),
                    OnFailHandler::Expr(expression) => {
                        visit_expr_blocks(expression, in_async, counter);
                    }
                }
            }
        }
        StmtKind::ArrayDestructure { value, .. }
        | StmtKind::ObjectDestructure { value, .. }
        | StmtKind::Assign { value, .. }
        | StmtKind::Throw(value)
        | StmtKind::Expr(value) => visit_expr_blocks(value, in_async, counter),
        StmtKind::IndexAssign { index, value, .. } => {
            visit_expr_blocks(index, in_async, counter);
            visit_expr_blocks(value, in_async, counter);
        }
        StmtKind::FieldAssign { object, value, .. } => {
            visit_expr_blocks(object, in_async, counter);
            visit_expr_blocks(value, in_async, counter);
        }
        StmtKind::Return(value) | StmtKind::Break(value) => {
            if let Some(value) = value {
                visit_expr_blocks(value, in_async, counter);
            }
        }
        StmtKind::Defer { body, .. } => hoist_block(body, in_async, counter),
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            hoist_block(try_body, in_async, counter);
            hoist_block(catch_body, in_async, counter);
            if let Some(finally_body) = finally_body {
                hoist_block(finally_body, in_async, counter);
            }
        }
        StmtKind::While { cond, body } | StmtKind::DoWhile { body, cond } => {
            visit_expr_blocks(cond, in_async, counter);
            hoist_block(body, in_async, counter);
        }
        StmtKind::For {
            init,
            cond,
            update,
            body,
        } => {
            visit_nested(init, in_async, counter);
            visit_expr_blocks(cond, in_async, counter);
            visit_nested(update, in_async, counter);
            hoist_block(body, in_async, counter);
        }
        StmtKind::ForOf { iterable, body, .. } => {
            visit_expr_blocks(iterable, in_async, counter);
            hoist_block(body, in_async, counter);
        }
        StmtKind::ForIn { object, body, .. } => {
            visit_expr_blocks(object, in_async, counter);
            hoist_block(body, in_async, counter);
        }
        StmtKind::Switch {
            expr,
            cases,
            default,
            ..
        } => {
            visit_expr_blocks(expr, in_async, counter);
            for case in cases {
                hoist_block(&mut case.body, in_async, counter);
            }
            if let Some(default) = default {
                hoist_block(default, in_async, counter);
            }
        }
        StmtKind::Match {
            expr,
            arms,
            default,
        } => {
            visit_expr_blocks(expr, in_async, counter);
            for arm in arms {
                hoist_block(&mut arm.body, in_async, counter);
            }
            if let Some(default) = default {
                hoist_block(default, in_async, counter);
            }
        }
        StmtKind::Continue => {}
    }
}

/// Finds the blocks written inside an expression: closure bodies, and the
/// branches of an `if` or the body of a `loop` used as a statement.
fn visit_expr_blocks(expression: &mut Expr, in_async: bool, counter: &mut usize) {
    match &mut expression.kind {
        ExprKind::ArrowFn { is_async, body, .. } => {
            let is_async = *is_async;
            hoist_block(body, is_async, counter);
        }
        ExprKind::If {
            cond,
            then_block,
            else_block,
        } => {
            visit_expr_blocks(cond, in_async, counter);
            hoist_block(then_block, in_async, counter);
            if let Some(else_block) = else_block {
                hoist_block(else_block, in_async, counter);
            }
        }
        ExprKind::Loop { body } => hoist_block(body, in_async, counter),
        _ => crate::ctfe::visit_expr_children_mut(expression, &mut |child| {
            visit_expr_blocks(child, in_async, counter);
        }),
    }
}

fn contains_await(expression: &Expr) -> bool {
    let mut awaits = Vec::new();
    collect_await_expressions(expression, &mut awaits);
    !awaits.is_empty()
}

/// Takes the first `await` out of `statement` when it may move, leaving the
/// temporary's name in its place, and returns the declaration to put before.
fn hoist_one(statement: &mut Stmt, counter: &mut usize) -> Option<Stmt> {
    let target = match &mut statement.kind {
        StmtKind::VarDecl {
            value: Some(value),
            onfail: None,
            ..
        }
        | StmtKind::Assign { value, .. }
        | StmtKind::Throw(value)
        | StmtKind::Return(Some(value)) => nested(value)?,
        StmtKind::Expr(value) => match &mut value.kind {
            ExprKind::If { cond, .. } => cond.as_mut(),
            _ => nested(value)?,
        },
        StmtKind::Match { expr, .. } | StmtKind::Switch { expr, .. } => expr,
        _ => return None,
    };
    let awaited = take_first_await(target, counter)?;
    let name = format!("{HOISTED_AWAIT_PREFIX}{}", *counter - 1);
    // The `await` expression's own span: a diagnostic about the hoisted value
    // points at the `await` the program wrote.
    let span = awaited.span;
    Some(Stmt::new(
        StmtKind::VarDecl {
            mutable: false,
            name,
            ty: Type::Infer,
            value: Some(awaited),
            onfail: None,
        },
        span,
    ))
}

/// The value itself, unless it is a direct `await`, which the lowering
/// already suspends on as it is.
fn nested(value: &mut Expr) -> Option<&mut Expr> {
    (!matches!(value.kind, ExprKind::Await(_))).then_some(value)
}

enum Walk {
    /// The first `await` was moved out.
    Taken(Expr),
    /// No `await` here; `pure` says whether evaluating this had no effect an
    /// `await` after it could observe.
    Clear { pure: bool },
    /// An `await` exists but cannot move.
    Stuck,
}

fn take_first_await(expression: &mut Expr, counter: &mut usize) -> Option<Expr> {
    match walk(expression, counter) {
        Walk::Taken(awaited) => Some(awaited),
        Walk::Clear { .. } | Walk::Stuck => None,
    }
}

/// Visits operands in evaluation order. Stops at the first `await`: moves it
/// when everything before was pure, or reports it stuck.
fn walk_in_order<'a>(
    operands: impl IntoIterator<Item = &'a mut Expr>,
    counter: &mut usize,
) -> Walk {
    let mut pure = true;
    for operand in operands {
        if !pure && contains_await(operand) {
            return Walk::Stuck;
        }
        match walk(operand, counter) {
            Walk::Clear { pure: operand_pure } => pure &= operand_pure,
            found => return found,
        }
    }
    Walk::Clear { pure }
}

fn effect(inner: Walk) -> Walk {
    match inner {
        Walk::Clear { .. } => Walk::Clear { pure: false },
        found => found,
    }
}

fn walk(expression: &mut Expr, counter: &mut usize) -> Walk {
    if !contains_await(expression) {
        return Walk::Clear {
            pure: matches!(
                expression.kind,
                ExprKind::StrLit(_)
                    | ExprKind::IntLit(_)
                    | ExprKind::UIntLit(_)
                    | ExprKind::FloatLit(_)
                    | ExprKind::DecimalLit(_)
                    | ExprKind::BoolLit(_)
                    | ExprKind::Ident(_)
                    | ExprKind::ArrowFn { .. }
            ),
        };
    }
    let span = expression.span;
    match &mut expression.kind {
        ExprKind::Await(inner) if !contains_await(inner) => {
            let name = format!("{HOISTED_AWAIT_PREFIX}{}", *counter);
            *counter += 1;
            let awaited = std::mem::replace(expression, Expr::new(ExprKind::Ident(name), span));
            Walk::Taken(awaited)
        }
        ExprKind::Await(inner) => walk(inner, counter),
        ExprKind::FnCall { args, .. } => effect(walk_in_order(args.iter_mut(), counter)),
        ExprKind::CallValue { callee, args, .. } => effect(walk_in_order(
            std::iter::once(callee.as_mut()).chain(args.iter_mut()),
            counter,
        )),
        ExprKind::MethodCall { object, args, .. } => effect(walk_in_order(
            std::iter::once(object.as_mut()).chain(args.iter_mut()),
            counter,
        )),
        ExprKind::BinOp {
            op: BinOp::And | BinOp::Or | BinOp::NullCoalesce,
            left,
            ..
        } => match walk(left, counter) {
            Walk::Clear { .. } => Walk::Stuck,
            found => found,
        },
        ExprKind::BinOp { left, right, .. } => {
            walk_in_order([left.as_mut(), right.as_mut()], counter)
        }
        ExprKind::Ternary { cond, .. } => match walk(cond, counter) {
            Walk::Clear { .. } => Walk::Stuck,
            found => found,
        },
        ExprKind::Not(inner)
        | ExprKind::Neg(inner)
        | ExprKind::BitNot(inner)
        | ExprKind::FieldAccess { object: inner, .. }
        | ExprKind::TypeAssert { value: inner, .. }
        | ExprKind::Is { expr: inner, .. } => walk(inner, counter),
        ExprKind::IndexAccess { object, index } => {
            walk_in_order([object.as_mut(), index.as_mut()], counter)
        }
        ExprKind::Try(inner) | ExprKind::Unwrap(inner) => effect(walk(inner, counter)),
        ExprKind::ArrayLit(values) => walk_in_order(values.iter_mut(), counter),
        ExprKind::ObjectLit(fields) => {
            walk_in_order(fields.iter_mut().map(|(_, value)| value), counter)
        }
        ExprKind::TemplateLit(parts) => walk_in_order(
            parts.iter_mut().filter_map(|part| match part {
                TemplatePart::Expr(expression, _) => Some(expression),
                TemplatePart::Text(_) => None,
            }),
            counter,
        ),
        _ => Walk::Stuck,
    }
}

#[cfg(test)]
#[path = "tests/tests.rs"]
mod tests;
