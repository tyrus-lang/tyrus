use crate::ast::{BinOp, Expr, ExprKind, TemplatePart};
use crate::span::Span;

pub fn collect_await_expressions<'a>(expression: &'a Expr, awaits: &mut Vec<&'a Expr>) {
    match expression.kind() {
        ExprKind::Await(dependency) => {
            collect_await_expressions(dependency, awaits);
            awaits.push(expression);
        }
        ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => {
            if let Some(initial) = initial {
                collect_await_expressions(initial, awaits);
            }
        }
        ExprKind::TemplateLit(parts) => {
            for part in parts {
                if let TemplatePart::Expr(expression, _) = part {
                    collect_await_expressions(expression, awaits);
                }
            }
        }
        ExprKind::ObjectLit(fields) => {
            for (_, value) in fields {
                collect_await_expressions(value, awaits);
            }
        }
        ExprKind::ObjectUpdate { base, fields } => {
            collect_await_expressions(base, awaits);
            for (_, value) in fields {
                collect_await_expressions(value, awaits);
            }
        }
        ExprKind::ArrayLit(values) => {
            for value in values {
                collect_await_expressions(value, awaits);
            }
        }
        ExprKind::ArrayRepeat { value, count } => {
            collect_await_expressions(value, awaits);
            collect_await_expressions(count, awaits);
        }
        ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
            collect_await_expressions(object, awaits);
        }
        ExprKind::IndexAccess { object, index } => {
            collect_await_expressions(object, awaits);
            collect_await_expressions(index, awaits);
        }
        ExprKind::BinOp { left, right, .. }
        | ExprKind::Pipeline { left, right }
        | ExprKind::In {
            key: left,
            object: right,
        } => {
            collect_await_expressions(left, awaits);
            collect_await_expressions(right, awaits);
        }
        ExprKind::Range { start, end, .. } => {
            collect_await_expressions(start, awaits);
            collect_await_expressions(end, awaits);
        }
        ExprKind::ChainedComparison { terms, .. } => {
            for term in terms {
                collect_await_expressions(term, awaits);
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
        | ExprKind::Try(inner)
        | ExprKind::Spread(inner)
        | ExprKind::TypeAssert { value: inner, .. }
        | ExprKind::Is { expr: inner, .. }
        | ExprKind::ConstAssert(inner)
        | ExprKind::Comptime(inner) => collect_await_expressions(inner, awaits),
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            collect_await_expressions(cond, awaits);
            collect_await_expressions(then_expr, awaits);
            collect_await_expressions(else_expr, awaits);
        }
        ExprKind::OptionalMethodCall { object, args, .. }
        | ExprKind::MethodCall { object, args, .. } => {
            collect_await_expressions(object, awaits);
            for argument in args {
                collect_await_expressions(argument, awaits);
            }
        }
        ExprKind::FnCall { args, .. } => {
            for argument in args {
                collect_await_expressions(argument, awaits);
            }
        }
        ExprKind::CallValue { callee, args, .. } => {
            collect_await_expressions(callee, awaits);
            for argument in args {
                collect_await_expressions(argument, awaits);
            }
        }
        ExprKind::If { cond, .. } => collect_await_expressions(cond, awaits),
        ExprKind::Loop { .. }
        | ExprKind::ArrowFn { .. }
        | ExprKind::StrLit(_)
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

pub fn async_expression_children(expression: &Expr) -> Vec<&Expr> {
    match expression.kind() {
        ExprKind::Await(dependency) => vec![dependency],
        ExprKind::Try(inner) => vec![inner],
        ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => {
            initial.as_deref().into_iter().collect()
        }
        ExprKind::TemplateLit(parts) => parts
            .iter()
            .filter_map(|part| match part {
                TemplatePart::Text(_) => None,
                TemplatePart::Expr(expression, _) => Some(expression),
            })
            .collect(),
        ExprKind::ObjectLit(fields) => fields.iter().map(|(_, value)| value).collect(),
        ExprKind::ObjectUpdate { base, fields } => std::iter::once(base.as_ref())
            .chain(fields.iter().map(|(_, value)| value))
            .collect(),
        ExprKind::ArrayLit(values) => values.iter().collect(),
        ExprKind::ArrayRepeat { value, count } => vec![value, count],
        ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
            vec![object]
        }
        ExprKind::IndexAccess { object, index } => vec![object, index],
        ExprKind::BinOp { left, right, .. }
        | ExprKind::Pipeline { left, right }
        | ExprKind::In {
            key: left,
            object: right,
        } => vec![left, right],
        ExprKind::Range { start, end, .. } => vec![start, end],
        ExprKind::ChainedComparison { terms, .. } => terms.iter().collect(),
        ExprKind::Not(inner)
        | ExprKind::Neg(inner)
        | ExprKind::BitNot(inner)
        | ExprKind::PrefixIncr(inner)
        | ExprKind::PrefixDecr(inner)
        | ExprKind::PostfixIncr(inner)
        | ExprKind::PostfixDecr(inner)
        | ExprKind::Unwrap(inner)
        | ExprKind::Typeof(inner)
        | ExprKind::Spread(inner)
        | ExprKind::TypeAssert { value: inner, .. }
        | ExprKind::Is { expr: inner, .. }
        | ExprKind::ConstAssert(inner)
        | ExprKind::Comptime(inner) => vec![inner],
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => vec![cond, then_expr, else_expr],
        ExprKind::OptionalMethodCall { object, args, .. }
        | ExprKind::MethodCall { object, args, .. } => std::iter::once(object.as_ref())
            .chain(args.iter())
            .collect(),
        ExprKind::FnCall { args, .. } => args.iter().collect(),
        ExprKind::CallValue { callee, args, .. } => {
            let mut children = vec![callee.as_ref()];
            children.extend(args.iter());
            children
        }
        ExprKind::If { cond, .. } => vec![cond],
        ExprKind::Loop { .. }
        | ExprKind::ArrowFn { .. }
        | ExprKind::StrLit(_)
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
        | ExprKind::Borrow { .. } => Vec::new(),
    }
}

pub fn expression_contains_await_span(expression: &Expr, target: Span) -> bool {
    if expression.span == target && matches!(expression.kind(), ExprKind::Await(_)) {
        return true;
    }
    async_expression_children(expression)
        .into_iter()
        .any(|child| expression_contains_await_span(child, target))
}

pub fn collect_prefixes_before_await<'a>(
    expression: &'a Expr,
    target: Span,
    prefixes: &mut Vec<&'a Expr>,
) -> bool {
    if expression.span == target && matches!(expression.kind(), ExprKind::Await(_)) {
        return true;
    }
    if let ExprKind::Ternary {
        cond,
        then_expr,
        else_expr,
    } = expression.kind()
    {
        if expression_contains_await_span(then_expr, target) {
            prefixes.push(cond);
            return collect_prefixes_before_await(then_expr, target, prefixes);
        }
        if expression_contains_await_span(else_expr, target) {
            prefixes.push(cond);
            return collect_prefixes_before_await(else_expr, target, prefixes);
        }
    }
    for child in async_expression_children(expression) {
        if expression_contains_await_span(child, target) {
            return collect_prefixes_before_await(child, target, prefixes);
        }
        prefixes.push(child);
    }
    false
}

pub fn async_prefix_requires_storage(expression: &Expr) -> bool {
    !matches!(
        expression.kind(),
        ExprKind::StrLit(_)
            | ExprKind::RegexLit { .. }
            | ExprKind::IntLit(_)
            | ExprKind::UIntLit(_)
            | ExprKind::FloatLit(_)
            | ExprKind::BoolLit(_)
            | ExprKind::Null
            | ExprKind::None
            | ExprKind::Ident(_)
            | ExprKind::Topic
    )
}

pub fn first_await_expression(expression: &Expr) -> Option<&Expr> {
    match expression.kind() {
        ExprKind::Await(_) => Some(expression),
        ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => {
            initial.as_deref().and_then(first_await_expression)
        }
        ExprKind::TemplateLit(parts) => parts.iter().find_map(|part| match part {
            TemplatePart::Text(_) => None,
            TemplatePart::Expr(expression, _) => first_await_expression(expression),
        }),
        ExprKind::ObjectLit(fields) => fields
            .iter()
            .find_map(|(_, value)| first_await_expression(value)),
        ExprKind::ObjectUpdate { base, fields } => first_await_expression(base).or_else(|| {
            fields
                .iter()
                .find_map(|(_, value)| first_await_expression(value))
        }),
        ExprKind::ArrayLit(values) => values.iter().find_map(first_await_expression),
        ExprKind::ArrayRepeat { value, count } => {
            first_await_expression(value).or_else(|| first_await_expression(count))
        }
        ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
            first_await_expression(object)
        }
        ExprKind::IndexAccess { object, index } => {
            first_await_expression(object).or_else(|| first_await_expression(index))
        }
        ExprKind::BinOp { left, right, .. }
        | ExprKind::Pipeline { left, right }
        | ExprKind::In {
            key: left,
            object: right,
        } => first_await_expression(left).or_else(|| first_await_expression(right)),
        ExprKind::Range { start, end, .. } => {
            first_await_expression(start).or_else(|| first_await_expression(end))
        }
        ExprKind::ChainedComparison { terms, .. } => {
            terms.iter().find_map(|term| first_await_expression(term))
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
        | ExprKind::Try(inner)
        | ExprKind::Spread(inner)
        | ExprKind::TypeAssert { value: inner, .. }
        | ExprKind::Is { expr: inner, .. }
        | ExprKind::ConstAssert(inner)
        | ExprKind::Comptime(inner) => first_await_expression(inner),
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => first_await_expression(cond)
            .or_else(|| first_await_expression(then_expr))
            .or_else(|| first_await_expression(else_expr)),
        ExprKind::OptionalMethodCall { object, args, .. }
        | ExprKind::MethodCall { object, args, .. } => {
            first_await_expression(object).or_else(|| args.iter().find_map(first_await_expression))
        }
        ExprKind::FnCall { args, .. } => args.iter().find_map(first_await_expression),
        ExprKind::CallValue { callee, args, .. } => {
            first_await_expression(callee).or_else(|| args.iter().find_map(first_await_expression))
        }
        ExprKind::If { cond, .. } => first_await_expression(cond),
        ExprKind::Loop { .. } => None,
        ExprKind::ArrowFn { .. } => None,
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
        | ExprKind::Borrow { .. } => None,
    }
}

pub fn nested_condition_await(condition: &Expr) -> Option<&Expr> {
    if matches!(condition.kind(), ExprKind::Await(_)) {
        return None;
    }
    let mut awaited = Vec::new();
    collect_await_expressions(condition, &mut awaited);
    let [only] = awaited[..] else {
        return None;
    };
    let ExprKind::Await(dependency) = only.kind() else {
        return None;
    };
    if first_await_expression(dependency).is_some() {
        return None;
    }
    let mut prefixes = Vec::new();
    collect_prefixes_before_await(condition, only.span, &mut prefixes);
    if prefixes
        .iter()
        .any(|prefix| async_prefix_requires_storage(prefix))
    {
        return None;
    }
    Some(only)
}

pub fn guarded_nested_condition_await(condition: &Expr) -> Option<(&Expr, &Expr, &Expr)> {
    let ExprKind::BinOp {
        op: BinOp::And,
        left,
        right,
    } = condition.kind()
    else {
        return None;
    };
    if first_await_expression(left).is_some() {
        return None;
    }
    let awaited = match right.kind() {
        ExprKind::Await(dependency) if first_await_expression(dependency).is_none() => {
            right.as_ref()
        }
        _ => nested_condition_await(right)?,
    };
    Some((left, right, awaited))
}
