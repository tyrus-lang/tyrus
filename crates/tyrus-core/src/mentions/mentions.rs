use crate::ast::{
    Expr, ExprKind, MatchPattern, OnFailHandler, Param, Stmt, StmtKind, TemplatePart,
};

pub(crate) fn expr_mentions(expr: &Expr, name: &str) -> bool {
    let mut found = false;
    mention_expr(expr, &mut |mentioned| {
        if mentioned == name {
            found = true;
        }
    });
    found
}

pub(crate) fn mention_stmt(stmt: &Stmt, visit: &mut dyn FnMut(&str)) {
    match stmt.kind() {
        StmtKind::VarDecl {
            name,
            value,
            onfail,
            mutable: _,
            ty: _,
        } => {
            visit(name);
            if let Some(value) = value {
                mention_expr(value, visit);
            }
            if let Some(onfail) = onfail {
                match &onfail.handler {
                    OnFailHandler::Expr(expr) => mention_expr(expr, visit),
                    OnFailHandler::Block(body) => mention_stmts(body, visit),
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
                visit(&binding.name);
            }
            mention_expr(value, visit);
        }
        StmtKind::Assign { name, value, .. } => {
            visit(name);
            mention_expr(value, visit);
        }
        StmtKind::IndexAssign { name, index, value } => {
            visit(name);
            mention_expr(index, visit);
            mention_expr(value, visit);
        }
        StmtKind::FieldAssign { object, value, .. } => {
            mention_expr(object, visit);
            mention_expr(value, visit);
        }
        StmtKind::Return(value) | StmtKind::Break(value) => {
            if let Some(value) = value {
                mention_expr(value, visit);
            }
        }
        StmtKind::Continue => {}
        StmtKind::Defer { body, .. } => mention_stmts(body, visit),
        StmtKind::Throw(value) | StmtKind::Expr(value) => mention_expr(value, visit),
        StmtKind::TryCatch {
            try_body,
            catch_binding,
            catch_body,
            finally_body,
            ..
        } => {
            mention_stmts(try_body, visit);
            visit(catch_binding);
            mention_stmts(catch_body, visit);
            if let Some(finally_body) = finally_body {
                mention_stmts(finally_body, visit);
            }
        }
        StmtKind::While { cond, body } | StmtKind::DoWhile { body, cond } => {
            mention_expr(cond, visit);
            mention_stmts(body, visit);
        }
        StmtKind::For {
            init,
            cond,
            update,
            body,
        } => {
            mention_stmt(init, visit);
            mention_expr(cond, visit);
            mention_stmt(update, visit);
            mention_stmts(body, visit);
        }
        StmtKind::ForOf {
            name,
            iterable,
            body,
            ..
        } => {
            visit(name);
            mention_expr(iterable, visit);
            mention_stmts(body, visit);
        }
        StmtKind::ForIn { name, object, body } => {
            visit(name);
            mention_expr(object, visit);
            mention_stmts(body, visit);
        }
        StmtKind::Switch {
            expr,
            cases,
            default,
            ..
        } => {
            mention_expr(expr, visit);
            for case in cases {
                mention_expr(&case.value, visit);
                mention_stmts(&case.body, visit);
            }
            if let Some(default) = default {
                mention_stmts(default, visit);
            }
        }
        StmtKind::Match {
            expr,
            arms,
            default,
        } => {
            mention_expr(expr, visit);
            for arm in arms {
                mention_pattern(&arm.pattern, visit);
                if let Some(guard) = &arm.guard {
                    mention_expr(guard, visit);
                }
                mention_stmts(&arm.body, visit);
            }
            if let Some(default) = default {
                mention_stmts(default, visit);
            }
        }
    }
}

pub(crate) fn mention_stmts(stmts: &[Stmt], visit: &mut dyn FnMut(&str)) {
    for stmt in stmts {
        mention_stmt(stmt, visit);
    }
}

pub(crate) fn mention_pattern(pattern: &MatchPattern, visit: &mut dyn FnMut(&str)) {
    match pattern {
        MatchPattern::Lit(expr) => mention_expr(expr, visit),
        MatchPattern::Wildcard => {}
        MatchPattern::Binding(name) => visit(name),
        MatchPattern::ResultVariant { binding, .. } => {
            if let Some(binding) = binding {
                visit(binding);
            }
        }
        MatchPattern::Tuple(patterns) => {
            for pattern in patterns {
                mention_pattern(pattern, visit);
            }
        }
        MatchPattern::EnumVariant {
            enum_name,
            binding,
            variant: _,
        } => {
            visit(enum_name);
            if let Some(binding) = binding {
                mention_pattern(binding, visit);
            }
        }
        MatchPattern::Object(fields) => {
            for field in fields {
                if let Some(binding) = &field.binding {
                    visit(binding);
                }
                if let Some(literal) = &field.literal {
                    mention_expr(literal, visit);
                }
            }
        }
        MatchPattern::Array { elements, .. } => {
            for element in elements {
                mention_pattern(&element.pattern, visit);
            }
        }
        MatchPattern::Or(patterns) => {
            for pattern in patterns {
                mention_pattern(pattern, visit);
            }
        }
        MatchPattern::Range { start, end, .. } => {
            if let Some(start) = start {
                mention_expr(start, visit);
            }
            if let Some(end) = end {
                mention_expr(end, visit);
            }
        }
    }
}

pub(crate) fn mention_params(params: &[Param], visit: &mut dyn FnMut(&str)) {
    for param in params {
        if let Some(default_value) = &param.default_value {
            mention_expr(default_value, visit);
        }
    }
}

pub(crate) fn mention_expr(expr: &Expr, visit: &mut dyn FnMut(&str)) {
    match expr.kind() {
        ExprKind::Ident(name) => visit(name),
        ExprKind::Borrow { name, .. } => visit(name),
        ExprKind::FnCall { callee, args, .. } => {
            visit(callee);
            for arg in args {
                mention_expr(arg, visit);
            }
        }
        ExprKind::CallValue { callee, args, .. } => {
            mention_expr(callee, visit);
            for arg in args {
                mention_expr(arg, visit);
            }
        }
        ExprKind::MethodCall { object, args, .. }
        | ExprKind::OptionalMethodCall { object, args, .. } => {
            mention_expr(object, visit);
            for arg in args {
                mention_expr(arg, visit);
            }
        }
        ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
            mention_expr(object, visit);
        }
        ExprKind::IndexAccess { object, index } => {
            mention_expr(object, visit);
            mention_expr(index, visit);
        }
        ExprKind::BinOp { left, right, .. } | ExprKind::Pipeline { left, right } => {
            mention_expr(left, visit);
            mention_expr(right, visit);
        }
        ExprKind::ChainedComparison { terms, .. } => {
            for term in terms {
                mention_expr(term, visit);
            }
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
        | ExprKind::Comptime(inner)
        | ExprKind::TypeAssert { value: inner, .. } => mention_expr(inner, visit),
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            mention_expr(cond, visit);
            mention_expr(then_expr, visit);
            mention_expr(else_expr, visit);
        }
        ExprKind::ObjectLit(fields) => {
            for (_, value) in fields {
                mention_expr(value, visit);
            }
        }
        ExprKind::ObjectUpdate { base, fields } => {
            mention_expr(base, visit);
            for (_, value) in fields {
                mention_expr(value, visit);
            }
        }
        ExprKind::ArrayLit(values) => {
            for value in values {
                mention_expr(value, visit);
            }
        }
        ExprKind::NewSet { initial, .. } => {
            if let Some(initial) = initial {
                mention_expr(initial, visit);
            }
        }
        ExprKind::NewMap { initial, .. } => {
            if let Some(initial) = initial {
                mention_expr(initial, visit);
            }
        }
        ExprKind::TemplateLit(parts) => {
            for part in parts {
                match part {
                    TemplatePart::Text(_) => {}
                    TemplatePart::Expr(expr, _) => mention_expr(expr, visit),
                }
            }
        }
        ExprKind::ArrowFn { params, body, .. } => {
            mention_params(params, visit);
            mention_stmts(body, visit);
        }
        ExprKind::If {
            cond,
            then_block,
            else_block,
        } => {
            mention_expr(cond, visit);
            mention_stmts(then_block, visit);
            if let Some(else_block) = else_block {
                mention_stmts(else_block, visit);
            }
        }
        ExprKind::Loop { body } => mention_stmts(body, visit),
        ExprKind::ArrayRepeat { value, count } => {
            mention_expr(value, visit);
            mention_expr(count, visit);
        }
        ExprKind::Is { expr, .. } => mention_expr(expr, visit),
        ExprKind::In { key, object } => {
            mention_expr(key, visit);
            mention_expr(object, visit);
        }
        ExprKind::Range { start, end, .. } => {
            mention_expr(start, visit);
            mention_expr(end, visit);
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
