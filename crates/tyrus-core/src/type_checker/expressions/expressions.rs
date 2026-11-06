use super::*;

#[path = "access/access.rs"]
mod access;
#[path = "conditionals/conditionals.rs"]
mod conditionals;
#[path = "literals/literals.rs"]
mod literals;
#[path = "unary/unary.rs"]
mod unary;
impl TypeChecker {
    pub(super) fn reject_bare_literal_annotation(&mut self, ty: &Type, subject: Option<&str>) {
        let resolved = match ty {
            Type::Named(name) => self
                .type_aliases
                .get(name)
                .cloned()
                .unwrap_or_else(|| ty.clone()),
            _ => ty.clone(),
        };
        if let Type::Lit(expr) = &resolved {
            let display = crate::ast::lit_display(expr);
            self.error_on_declaration(
                "E0316",
                format!(
                    "a bare literal type `{display}` cannot be used as an annotation; use its primitive type (`string`, `i32`, `boolean`, …) or a union like `\"a\" | \"b\"`"
                ),
                subject,
            );
        }
    }

    pub(super) fn probe_expr(&mut self, expr: &Expr) -> ValueType {
        let reported = self.diagnostics.len();
        let ty = self.check_expr(expr);
        self.diagnostics.truncate(reported);
        ty
    }

    pub(super) fn check_expr(&mut self, expr: &Expr) -> ValueType {
        self.check_expr_with_expected(expr, None)
    }

    pub(super) fn check_expr_as(&mut self, expr: &Expr, expected: &Type) -> ValueType {
        if self.is_cancelled() {
            return ValueType::Unknown;
        }
        let resolved = match expected {
            Type::Named(name) => self
                .type_aliases
                .get(name)
                .cloned()
                .unwrap_or_else(|| expected.clone()),
            _ => expected.clone(),
        };
        if matches!(
            transparent_type(&resolved),
            Type::Function { .. } | Type::GenericFunction { .. }
        ) {
            let declared = match &resolved {
                Type::Throws { effects, .. } => effects.clone(),
                _ => Vec::new(),
            };
            let undeclared = self
                .expression_function_effects(expr)
                .into_iter()
                .filter(|effect| !declared.contains(effect))
                .map(|effect| effect.to_string())
                .collect::<Vec<_>>();
            if !undeclared.is_empty() {
                let undeclared = undeclared.join(" | ");
                let shown = transparent_type(&resolved).to_string();
                self.error_at_with_help(
                    "E3298",
                    format!(
                        "a function that throws `{undeclared}` cannot be used as `{shown}`, which does not declare it"
                    ),
                    expr.span,
                    vec![format!(
                        "write the type as `{shown} throws {undeclared}` and handle the call where it is made"
                    )],
                );
            }
        }
        if let Type::Tuple(elements) = transparent_type(&resolved) {
            let expected_count = elements.len();
            let mismatch = match expr.kind() {
                ExprKind::ArrayLit(values) if values.len() != expected_count => Some((
                    expr.span,
                    format!(
                        "fixed-length type requires {expected_count} elements, but this initializer contains {}",
                        values.len()
                    ),
                    format!("provide exactly {expected_count} elements in this initializer"),
                )),
                ExprKind::ArrayRepeat { count, .. } => {
                    let count_span = count.span;
                    integer_constant_value(count)
                        .and_then(Result::ok)
                        .and_then(|count| usize::try_from(count).ok())
                        .filter(|count| *count != expected_count)
                        .map(|actual_count| (
                            count_span,
                            format!(
                                "fixed-length type requires {expected_count} elements, but this repetition produces {actual_count}"
                            ),
                            format!("change the repeat count to {expected_count}"),
                        ))
                }
                _ => None,
            };
            if let Some((span, message, help)) = mismatch {
                self.error_at_with_help("E0308", message, span, vec![help]);
                return ValueType::Error;
            }
        }
        let resolved_named = match transparent_type(&resolved) {
            Type::Optional(inner) => (**inner).clone(),
            other => other.clone(),
        };
        if let (Type::Named(name), ExprKind::ObjectLit(fields)) = (&resolved_named, expr.kind()) {
            if name == "CorsOptions" {
                self.check_http_cors_literals(fields);
            }
            if name == "FetchOptions"
                && let Some((_, value)) = fields.iter().find(|(field, _)| field == "method")
            {
                self.error_at_with_help(
                        "E3295",
                        "`method` is not a `fetch` option; the verb is what names it".to_string(),
                        value.span,
                        vec![
                            "write `fetch.post(url, ...)`, `fetch.put(url, ...)` and so on; a plain `fetch(url, ...)` sends `GET`".to_string(),
                        ],
                    );
            }
        }
        if matches!(transparent_type(&resolved), Type::Char)
            && let ExprKind::StrLit(text) = expr.kind()
        {
            let mut characters = text.chars();
            return match (characters.next(), characters.next()) {
                (Some(_), None) => ValueType::Known(Type::Char),
                _ => {
                    self.error_at(
                        "E0708",
                        format!(
                            "a `char` holds exactly one character, but `{text}` has {}",
                            text.chars().count()
                        ),
                        expr.span,
                    );
                    ValueType::Error
                }
            };
        }
        if let Type::Lit(expected_lit) = transparent_type(&resolved)
            && literal_expr_matches(expected_lit, expr)
        {
            return ValueType::Known(resolved.clone());
        }
        if let Type::Array(element) = transparent_type(&resolved)
            && matches!(transparent_type(element), Type::Union(_))
            && let ExprKind::ArrayLit(values) = expr.kind()
            && !values
                .iter()
                .any(|value| matches!(value.kind(), ExprKind::Spread(_)))
        {
            let element = element.as_ref().clone();
            for value in values {
                let value_ty = self.check_expr_as(value, &element);
                self.require_assignable(
                    &element,
                    &value_ty,
                    expression_subject(value).unwrap_or("array element"),
                    "E0308",
                );
            }
            return ValueType::Known(resolved.clone());
        }
        if matches!(transparent_type(&resolved), Type::Union(_))
            && let ExprKind::Ternary {
                cond,
                then_expr,
                else_expr,
            } = expr.kind()
        {
            let condition = self.check_expr(cond);
            if !is_optional_value(&condition) {
                self.require_bool(&condition, "ternary condition");
            }
            for branch in [then_expr, else_expr] {
                let branch_ty = self.check_expr_as(branch, &resolved);
                self.require_assignable(
                    &resolved,
                    &branch_ty,
                    expression_subject(branch).unwrap_or("ternary branch"),
                    "E0308",
                );
            }
            return ValueType::Known(resolved.clone());
        }
        if let Type::Union(variants) = transparent_type(&resolved)
            && variants
                .iter()
                .any(|variant| matches!(variant, Type::Lit(lit) if literal_expr_matches(lit, expr)))
        {
            return ValueType::Known(resolved.clone());
        }
        if let (
            Type::Map(_, value_ty),
            Expr {
                kind: ExprKind::ObjectLit(fields),
                ..
            },
        ) = (transparent_type(&resolved), expr)
        {
            let value_ty = value_ty.as_ref().clone();
            for (_, value) in fields {
                let actual = self.check_expr_as(value, &value_ty);
                self.require_assignable(&value_ty, &actual, "value", "E0308");
            }
            return ValueType::Known(resolved.clone());
        }
        self.check_expr_with_expected(expr, Some(expected))
    }

    pub(super) fn check_expr_with_expected(
        &mut self,
        expr: &Expr,
        expected: Option<&Type>,
    ) -> ValueType {
        if self.is_cancelled() {
            return ValueType::Unknown;
        }
        if let Some(expected_ty) = expected
            && let Some(parsed) = self.check_parse_builtin_at_width(expr, expected_ty)
        {
            return parsed;
        }
        if let (
            Expr {
                kind: ExprKind::ArrayLit(values),
                ..
            },
            Some(expected_ty),
        ) = (expr, expected)
            && let Type::Tuple(positions) = transparent_type(expected_ty)
            && positions.len() == values.len()
            && !values
                .iter()
                .any(|value| matches!(value.kind(), ExprKind::Spread(_)))
        {
            let positions = positions.clone();
            let element_types = values
                .iter()
                .zip(positions.iter())
                .map(|(value, position)| {
                    let actual = self.check_expr_as(value, position);
                    value_type_to_type(&actual).unwrap_or_else(|| position.clone())
                })
                .collect();
            let ty = ValueType::Known(Type::Tuple(element_types));
            let ordinal = self.expressions.len() as u32;
            self.expressions.push(InferredExpression {
                ordinal,
                ty: ty.display(),
            });
            return ty;
        }
        if let (
            Expr {
                kind: ExprKind::ObjectLit(values),
                ..
            },
            Some(expected_ty),
        ) = (expr, expected)
            && let Some(expected_fields) = self.decorated_object_fields(expected_ty)
        {
            let mut fields = Vec::with_capacity(values.len());
            for (name, value) in values {
                let actual =
                    if let Some(field) = expected_fields.iter().find(|field| field.name == *name) {
                        self.check_expr_as(value, &field.ty)
                    } else {
                        self.check_object_field_expr(value)
                    };
                fields.push((name.clone(), actual));
            }
            let ty = ValueType::Object(fields);
            self.validate_object_literal_decorators(expected_ty, values);
            self.expressions.push(InferredExpression {
                ordinal: self.expressions.len() as u32,
                ty: ty.display(),
            });
            return ty;
        }
        let ty = if let ExprKind::ArrowFn {
            is_async,
            type_params,
            params,
            ret_type,
            throws,
            body,
        } = expr.kind()
        {
            self.check_arrow(
                type_params,
                params,
                ret_type,
                throws,
                body,
                expected,
                *is_async,
                expr.span,
            )
        } else {
            self.check_expr_inner(expr, expected)
        };
        if let (
            Expr {
                kind: ExprKind::ObjectLit(fields),
                ..
            },
            Some(expected_ty),
        ) = (expr, expected)
        {
            self.validate_object_literal_decorators(expected_ty, fields);
        }
        let ordinal = self.expressions.len() as u32;
        let recorded = match (&ty, expected) {
            (ValueType::IntegerLiteral, Some(expected_ty))
                if is_integer_type(transparent_type(expected_ty)) =>
            {
                transparent_type(expected_ty).to_string()
            }
            _ => ty.display(),
        };
        self.expressions.push(InferredExpression {
            ordinal,
            ty: recorded,
        });
        if let Some(inferred) = value_type_to_type(&ty) {
            self.expression_types_by_span.insert(expr.span, inferred);
        }
        ty
    }

    pub(super) fn check_expr_inner(&mut self, expr: &Expr, expected: Option<&Type>) -> ValueType {
        self.span_stack.push(expr.span);
        let result = self.check_expr_kind(expr, expected);
        self.span_stack.pop();
        result
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn check_expr_kind(&mut self, expr: &Expr, expected: Option<&Type>) -> ValueType {
        if let Some(styled) = self.check_style_text(expr) {
            return styled;
        }
        if let Some(rendered) = self.check_figlet(expr) {
            return rendered;
        }
        match expr.kind() {
            ExprKind::CallValue {
                callee,
                type_args,
                args,
            } => {
                if !type_args.is_empty() {
                    self.error(
                        "E0308",
                        "explicit type arguments are only supported when calling a named function"
                            .to_string(),
                        None,
                    );
                }
                let callee_ty = self.check_expr(callee);
                let (params, ret) = match value_type_to_type(&callee_ty) {
                    Some(Type::Function { params, ret }) => (params, *ret),
                    Some(other) => {
                        for arg in args {
                            self.check_expr(arg);
                        }
                        if !matches!(other, Type::Unknown) {
                            self.error_at(
                                "E0618",
                                format!("this expression is not callable: it has type `{other}`"),
                                callee.span,
                            );
                        }
                        return ValueType::Error;
                    }
                    None => {
                        for arg in args {
                            self.check_expr(arg);
                        }
                        return ValueType::Error;
                    }
                };
                if args.len() != params.len() {
                    self.error(
                        "E0061",
                        format!(
                            "this call expects {} argument(s), but {} were provided",
                            params.len(),
                            args.len()
                        ),
                        None,
                    );
                }
                for (arg, param) in args.iter().zip(params.iter()) {
                    let actual = self.check_expr_as(arg, param);
                    self.require_assignable(param, &actual, "call", "E0308");
                }
                for arg in args.iter().skip(params.len()) {
                    self.check_expr(arg);
                }
                ValueType::Known(self.resolve_type_annotation(&ret))
            }
            ExprKind::StrLit { .. }
            | ExprKind::TemplateLit { .. }
            | ExprKind::RegexLit { .. }
            | ExprKind::NewSet { .. }
            | ExprKind::NewMap { .. }
            | ExprKind::IntLit { .. }
            | ExprKind::UIntLit { .. }
            | ExprKind::FloatLit { .. }
            | ExprKind::DecimalLit { .. }
            | ExprKind::BoolLit { .. }
            | ExprKind::Null
            | ExprKind::None
            | ExprKind::ObjectLit { .. }
            | ExprKind::ObjectUpdate { .. }
            | ExprKind::ArrayLit { .. }
            | ExprKind::ArrayRepeat { .. } => self.check_literal_expr(expr, expected),
            ExprKind::Ident(name) => self.read_symbol(name),
            ExprKind::FieldAccess { .. }
            | ExprKind::OptionalFieldAccess { .. }
            | ExprKind::IndexAccess { .. } => self.check_access_expr(expr),
            ExprKind::Borrow { .. }
            | ExprKind::Unwrap { .. }
            | ExprKind::Try { .. }
            | ExprKind::Not { .. }
            | ExprKind::Neg { .. }
            | ExprKind::BitNot { .. }
            | ExprKind::Typeof { .. } => self.check_unary_expr(expr),
            ExprKind::BinOp { op, left, right } => self.check_binary(op, left, right),
            ExprKind::ChainedComparison { terms, ops } => self.check_chained_comparison(terms, ops),
            ExprKind::Pipeline { left, right } => {
                if contains_outer_topic(right) {
                    let topic = self.check_expr(left);
                    self.topic_stack.push(topic);
                    let result = self.check_expr(right);
                    self.topic_stack.pop();
                    result
                } else {
                    match desugar_pipeline(left, right) {
                        Some(call) => self.check_expr(&call),
                        None => {
                            self.error(
                                "E3290",
                                "the right side of `|>` must be a function or contain `%`"
                                    .to_string(),
                                expression_subject(right),
                            );
                            self.check_expr(left);
                            ValueType::Error
                        }
                    }
                }
            }
            ExprKind::Topic => {
                if let Some(topic) = self.topic_stack.last() {
                    topic.clone()
                } else {
                    self.error(
                        "E3291",
                        "`%` can only be used on the right side of `|>`".to_string(),
                        Some("%"),
                    );
                    ValueType::Error
                }
            }
            ExprKind::FnCall {
                callee,
                type_args,
                args,
            } => self.check_call(callee, type_args, args, expr.span),
            ExprKind::MethodCall {
                object,
                method,
                method_span,
                type_args,
                args,
            } => {
                if matches!(object.kind(), ExprKind::Ident(name) if name == "Array")
                    && !self.has_symbol("Array")
                    && method == "withCapacity"
                    && let Some(expected) = expected
                    && let Type::Array(_element) = transparent_type(expected)
                {
                    if args.len() != 1 {
                        self.error(
                            "E0061",
                            format!(
                                "`Array.withCapacity` expects 1 argument, but {} were provided",
                                args.len()
                            ),
                            Some(method),
                        );
                    } else {
                        let actual = self.check_expr_as(args.first().unwrap(), &Type::Usize);
                        self.require_assignable(&Type::Usize, &actual, "capacity", "E0308");
                    }
                    return ValueType::Known(expected.clone());
                }
                let result = self.check_method_call(
                    object,
                    method,
                    Some(*method_span),
                    type_args,
                    args,
                    false,
                );
                self.update_len_after_method(object, method, args);
                result
            }
            ExprKind::OptionalMethodCall {
                object,
                method,
                type_args,
                args,
            } => {
                let result = self.check_method_call(object, method, None, type_args, args, true);
                self.update_len_after_method(object, method, args);
                result
            }
            ExprKind::ArrowFn { .. } => unreachable!("arrow functions are checked contextually"),
            ExprKind::PrefixIncr(inner) | ExprKind::PrefixDecr(inner) => {
                let actual = self.check_expr(inner);
                self.require_mutable_numeric_target(inner, &actual);
                actual
            }
            ExprKind::PostfixIncr(inner) | ExprKind::PostfixDecr(inner) => {
                let actual = self.check_expr(inner);
                self.require_mutable_numeric_target(inner, &actual);
                actual
            }
            ExprKind::Ternary { .. }
            | ExprKind::If { .. }
            | ExprKind::Loop { .. }
            | ExprKind::TypeAssert { .. }
            | ExprKind::ConstAssert { .. } => self.check_conditional_expr(expr),
            ExprKind::Comptime(value) => self.check_expr(value),
            ExprKind::Spread(inner) => self.check_expr(inner),
            ExprKind::Await(inner) => {
                if let Some(name) = self.live_mutex_guard_name() {
                    self.error_at(
                        "E3034",
                        format!(
                            "`{name}` is a lock guard still holding a lock; it cannot be alive across `await`, holding a lock across a suspension point is a common source of deadlock and starvation"
                        ),
                        expr.span,
                    );
                }
                if self.async_function_stack.last() != Some(&true) {
                    self.error_at_with_help(
                        "E3000",
                        "`await` can only be used inside an async function".to_string(),
                        expr.span,
                        vec!["move this `await` into an `async` function or mark its enclosing function `async`".to_string()],
                    );
                }
                let previous_awaited = self.awaited_operand.replace(inner.span);
                let awaits_spawn_directly = self.is_spawn_task_call(inner);
                let awaits_spawn_join_chain = matches!(
                    inner.kind(),
                    ExprKind::MethodCall { object, method, .. }
                        if method == "join" && self.is_spawn_task_call(object)
                );
                let actual = self.check_expr(inner);
                self.awaited_operand = previous_awaited;
                if let ExprKind::Ident(name) = inner.kind()
                    && matches!(&actual, ValueType::Known(ty) if promise_payload(ty).is_some())
                {
                    if self.is_promise_awaited(name, inner.span) {
                        self.error_at_with_help(
                                "E3006",
                                format!(
                                    "`{name}` was already consumed by an earlier `await`; a `Promise<T>` can only be awaited once"
                                ),
                                inner.span,
                                vec![
                                    "store the awaited value in a binding and read that instead of awaiting `{name}` again"
                                        .replace("{name}", name),
                                ],
                            );
                    } else {
                        self.mark_promise_awaited(name, inner.span);
                    }
                }
                if awaits_spawn_directly {
                    self.error_at_with_help(
                        "E3031",
                        "awaiting `spawn(...)` directly defeats the purpose of running it in the background".to_string(),
                        inner.span,
                        vec!["call the task function directly instead of `spawn(...)`, or store the `JoinHandle` and `await` its `.join()` later".to_string()],
                    );
                    return ValueType::Error;
                }
                if awaits_spawn_join_chain {
                    self.error_at_with_help(
                        "E3031",
                        "awaiting `spawn(...).join()` in the same expression defeats the purpose of running it in the background".to_string(),
                        inner.span,
                        vec!["store the `JoinHandle` in a variable, do other work, then `await` its `.join()`".to_string()],
                    );
                }
                match actual {
                    ValueType::Known(ty) => match promise_payload(&ty) {
                        Some(payload) => ValueType::Known(payload.clone()),
                        None => {
                            if is_external_thenable(&ty, &self.interfaces) {
                                self.error(
                                    "E3008",
                                    "external thenables are not supported; `await` accepts only the native `Promise<T>` type"
                                        .to_string(),
                                    expression_subject(inner),
                                );
                            } else {
                                self.error(
                                    "E3001",
                                    format!("`await` expects `Promise<T>`, found `{ty}`"),
                                    expression_subject(inner),
                                );
                            }
                            ValueType::Error
                        }
                    },
                    ValueType::Error => ValueType::Error,
                    ValueType::Unknown => ValueType::Unknown,
                    other => {
                        self.error(
                            "E3001",
                            format!("`await` expects `Promise<T>`, found `{}`", other.display()),
                            expression_subject(inner),
                        );
                        ValueType::Error
                    }
                }
            }
            ExprKind::In { key, object } => self.check_in(key, object),
            ExprKind::Is { expr, ty } => self.check_is(expr, ty),
            ExprKind::Range { start, end, .. } => {
                let start_ty = self.check_expr(start);
                let end_ty = self.check_expr(end);
                match self.numeric_result("..", &start_ty, &end_ty, start, end) {
                    ValueType::Error => ValueType::Error,
                    ValueType::Unknown => ValueType::Known(Type::Range(Box::new(Type::Unknown))),
                    ValueType::Known(elem) => ValueType::Known(Type::Range(Box::new(elem))),
                    _ => unreachable!("numeric_result only ever returns Known, Unknown or Error"),
                }
            }
        }
    }

    pub(super) fn check_typeof_category_literal(&mut self, left: &Expr, right: &Expr) {
        let literal = match (left.kind(), right.kind()) {
            (ExprKind::Typeof(_), ExprKind::StrLit(_)) => right,
            (ExprKind::StrLit(_), ExprKind::Typeof(_)) => left,
            _ => return,
        };
        let ExprKind::StrLit(name) = literal.kind() else {
            return;
        };
        if crate::typeof_category::is_typeof_category(name) || self.is_producible_typeof_name(name)
        {
            return;
        }
        let mut help = Vec::new();
        if let Some(category) = crate::typeof_category::typeof_category_suggestion(name) {
            help.push(format!("replace `\"{name}\"` with `\"{category}\"`"));
        }
        let categories = crate::typeof_category::TYPEOF_CATEGORIES
            .iter()
            .map(|category| format!("`\"{category}\"`"))
            .collect::<Vec<_>>()
            .join(", ");
        help.push(format!("`typeof` produces one of {categories}"));
        self.error_at_with_help(
            "E0795",
            format!("`typeof` never produces `\"{name}\"`, so this comparison is constant"),
            literal.span,
            help,
        );
    }

    pub(super) fn check_binary(&mut self, op: &BinOp, left: &Expr, right: &Expr) -> ValueType {
        if matches!(op, BinOp::And | BinOp::Or) {
            let left_ty = self.check_expr(left);
            if !is_optional_value(&left_ty) {
                self.require_bool(&left_ty, "left logical operand");
            }
            let narrow = null_narrowing(left);
            let right_ty =
                self.check_expr_narrowed(right, narrow.as_ref(), matches!(op, BinOp::And));
            if !is_optional_value(&right_ty) {
                self.require_bool(&right_ty, "right logical operand");
            }
            self.warn_validator_fact_condition(op, left, right);
            return ValueType::Known(Type::Bool);
        }
        if matches!(op, BinOp::Eq | BinOp::Neq) {
            self.check_typeof_category_literal(left, right);
        }
        let left_ty = self.check_expr(left);
        let right_ty = self.check_expr(right);
        match op {
            BinOp::Add => {
                if is_string(&left_ty) || is_string(&right_ty) {
                    if is_string(&left_ty) && is_string(&right_ty) {
                        ValueType::Known(Type::Str)
                    } else {
                        self.binary_mismatch("+", &left_ty, &right_ty, left, right)
                    }
                } else {
                    self.numeric_result("+", &left_ty, &right_ty, left, right)
                }
            }
            BinOp::Sub => self.numeric_result("-", &left_ty, &right_ty, left, right),
            BinOp::Mul => self.numeric_result("*", &left_ty, &right_ty, left, right),
            BinOp::Div => self.numeric_result("/", &left_ty, &right_ty, left, right),
            BinOp::Mod => self.numeric_result("%", &left_ty, &right_ty, left, right),
            BinOp::Pow => self.numeric_result("**", &left_ty, &right_ty, left, right),
            BinOp::Gt | BinOp::Lt | BinOp::Gte | BinOp::Lte => {
                self.numeric_result("comparison", &left_ty, &right_ty, left, right);
                self.warn_validator_fact_condition(op, left, right);
                ValueType::Known(Type::Bool)
            }
            BinOp::Eq | BinOp::Neq => {
                if matches!(left.kind(), ExprKind::None) || matches!(right.kind(), ExprKind::None) {
                    let optional = if matches!(left.kind(), ExprKind::None) {
                        right
                    } else {
                        left
                    };
                    let help = if let ExprKind::Ident(name) = optional.kind() {
                        format!(
                            "use `if ({name})` to test presence or `if (!{name})` to test absence"
                        )
                    } else {
                        "use presence narrowing with `if (value)` or `if (!value)`".to_string()
                    };
                    let none = if matches!(left.kind(), ExprKind::None) {
                        left
                    } else {
                        right
                    };
                    self.error_at_with_help(
                        "E3079",
                        "optional presence cannot be compared with `None`".to_string(),
                        none.span,
                        vec![help],
                    );
                }
                if value_type_is_decimal(&left_ty) != value_type_is_decimal(&right_ty) {
                    self.error(
                        "E0308",
                        format!(
                            "cannot compare `decimal` with `{}` without an explicit conversion",
                            if value_type_is_decimal(&left_ty) {
                                right_ty.display()
                            } else {
                                left_ty.display()
                            }
                        ),
                        expression_subject(right),
                    );
                } else if !types_overlap(&left_ty, &right_ty) {
                    self.error(
                        "E0308",
                        format!(
                            "cannot compare `{}` with `{}`",
                            left_ty.display(),
                            right_ty.display()
                        ),
                        expression_subject(right),
                    );
                } else if let Some((kind, has_equals)) = self.compared_by_contents(&left_ty) {
                    let operator = if matches!(op, BinOp::Eq) {
                        "==="
                    } else {
                        "!=="
                    };
                    let help = if has_equals {
                        format!(
                            "compare them with `left.equals(right)`, which compares {kind} by their contents"
                        )
                    } else {
                        "compare the fields that matter, one by one".to_string()
                    };
                    let message = format!(
                        "`{operator}` cannot compare `{}` with `{}`, because {kind} are compared by their contents",
                        left_ty.display(),
                        right_ty.display()
                    );
                    let span = left.span.join(right.span);
                    if span.is_dummy() {
                        self.error("E0369", message, expression_subject(right));
                        if let Some(last) = self.diagnostics.last_mut() {
                            last.help = vec![help];
                        }
                    } else {
                        self.error_at_with_help("E0369", message, span, vec![help]);
                    }
                }
                self.warn_validator_fact_condition(op, left, right);
                ValueType::Known(Type::Bool)
            }
            BinOp::And | BinOp::Or => ValueType::Known(Type::Bool),
            BinOp::BitAnd
            | BinOp::BitOr
            | BinOp::BitXor
            | BinOp::Shl
            | BinOp::Shr
            | BinOp::UShr => {
                if !is_integer_value(&left_ty) || !is_integer_value(&right_ty) {
                    self.binary_mismatch(binop_display_name(op), &left_ty, &right_ty, left, right);
                }
                self.numeric_result(binop_display_name(op), &left_ty, &right_ty, left, right)
            }
            BinOp::NullCoalesce => match &left_ty {
                ValueType::Known(Type::Optional(inner)) => {
                    let inner = inner.as_ref().clone();
                    let right_ty = self.check_expr_as(right, &inner);
                    self.require_assignable(&inner, &right_ty, "right operand of `??`", "E0308");
                    ValueType::Known(inner)
                }
                ValueType::Unknown | ValueType::Error => left_ty,
                _ => {
                    self.error(
                        "E0277",
                        format!(
                            "left operand of `??` must be optional, found `{}`",
                            left_ty.display()
                        ),
                        expression_subject(left),
                    );
                    ValueType::Error
                }
            },
        }
    }

    pub(super) fn check_object_field_expr(&mut self, expr: &Expr) -> ValueType {
        match expr.kind() {
            ExprKind::StrLit(_)
            | ExprKind::IntLit(_)
            | ExprKind::FloatLit(_)
            | ExprKind::BoolLit(_)
            | ExprKind::Null => {
                let ty = ValueType::Known(Type::Lit(Box::new(expr.clone())));
                self.expressions.push(InferredExpression {
                    ordinal: self.expressions.len() as u32,
                    ty: ty.display(),
                });
                ty
            }
            _ => self.check_expr(expr),
        }
    }

    pub(super) fn expression_function_effects(&self, expr: &Expr) -> Vec<Type> {
        match expr.kind() {
            ExprKind::Ident(name) => {
                let symbol_type = self
                    .lookup(name)
                    .and_then(|symbol| value_type_to_type(&symbol.ty));
                if let Some(Type::Throws { effects, .. }) = &symbol_type {
                    return effects.clone();
                }
                let names_a_function = symbol_type.as_ref().is_some_and(|ty| {
                    matches!(
                        transparent_type(ty),
                        Type::Function { .. } | Type::GenericFunction { .. }
                    )
                });
                if names_a_function && !self.async_fns.contains(name) {
                    self.fn_throws.get(name).cloned().unwrap_or_default()
                } else {
                    Vec::new()
                }
            }
            ExprKind::ArrowFn {
                is_async: false,
                throws,
                ..
            } => throws.clone(),
            _ => Vec::new(),
        }
    }

    pub(super) fn check_binary_heap_method(
        &mut self,
        _object: &Expr,
        element: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let (minimum, maximum, return_type) = match method {
            "push" => (1, 1, Type::Void),
            "pop" | "peek" => (0, 0, Type::Optional(Box::new(element.clone()))),
            "len" => (0, 0, Type::Usize),
            "isEmpty" => (0, 0, Type::Bool),
            "clear" => (0, 0, Type::Void),
            _ => {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for `BinaryHeap<{element}>`"),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        for arg in args {
            let actual = self.check_expr_as(arg, element);
            self.require_assignable(element, &actual, method, "E0308");
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_expr_narrowed(
        &mut self,
        expr: &Expr,
        null_narrowing: Option<&(String, bool)>,
        positive_branch: bool,
    ) -> ValueType {
        self.push_scope();
        if let Some((name, non_null_in_then)) = null_narrowing
            && *non_null_in_then == positive_branch
            && let Some(symbol) = self.lookup(name).cloned()
            && let ValueType::Known(Type::Optional(inner)) = symbol.ty.clone()
        {
            self.declare(
                name,
                Symbol {
                    ty: ValueType::Known(*inner),
                    ..symbol
                },
            );
        }
        let ty = self.check_expr(expr);
        self.pop_scope();
        ty
    }

    pub(super) fn binary_mismatch(
        &mut self,
        operator: &str,
        left: &ValueType,
        right: &ValueType,
        _left_expr: &Expr,
        right_expr: &Expr,
    ) -> ValueType {
        self.error(
            "E0369",
            format!(
                "operator `{operator}` cannot be applied to `{}` and `{}`",
                left.display(),
                right.display()
            ),
            expression_subject(right_expr),
        );
        ValueType::Error
    }

    pub(super) fn require_integer_literal_fits(&mut self, expected: &Type, value: &Expr) {
        let expected = transparent_type(expected).clone();
        match (&expected, value.kind()) {
            (Type::Array(element), ExprKind::ArrayLit(values)) => {
                let element = element.as_ref().clone();
                for value in values {
                    if let ExprKind::Spread(inner) = value.kind() {
                        self.require_integer_literal_fits(
                            &Type::Array(Box::new(element.clone())),
                            inner,
                        );
                    } else {
                        self.require_integer_literal_fits(&element, value);
                    }
                }
                return;
            }
            (Type::Array(element), ExprKind::ArrayRepeat { value, .. }) => {
                self.require_integer_literal_fits(element, value);
                return;
            }
            (Type::Tuple(elements), ExprKind::ArrayLit(values))
                if elements.len() == values.len() =>
            {
                for (element, value) in elements.iter().zip(values) {
                    self.require_integer_literal_fits(element, value);
                }
                return;
            }
            (Type::Tuple(elements), ExprKind::ArrayRepeat { value, .. }) => {
                if let Some(element) = elements.first() {
                    self.require_integer_literal_fits(element, value);
                }
                return;
            }
            _ => {}
        }
        if let ExprKind::ObjectLit(fields) = value.kind() {
            let declared = match &expected {
                Type::Object(fields) => Some(fields.clone()),
                Type::Named(name) => self.interfaces.get(name).cloned(),
                _ => None,
            };
            if let Some(declared) = declared {
                for (name, field_value) in fields {
                    let Some(field) = declared.iter().find(|field| &field.name == name) else {
                        continue;
                    };
                    let field_ty = field.ty.clone();
                    self.require_integer_literal_fits(&field_ty, field_value);
                }
            }
            return;
        }
        let Some((min, max)) = integer_type_bounds(&expected) else {
            return;
        };
        let Some(constant) = integer_constant_value(value) else {
            return;
        };
        let literal = match constant {
            Ok(value) => value,
            Err(()) => {
                self.error_at_with_help(
                    "E0706",
                    format!("integer constant expression overflows `{expected}`"),
                    value.span,
                    vec![format!(
                        "keep the operation within the bounds of `{expected}`"
                    )],
                );
                return;
            }
        };
        if literal >= min && max.is_none_or(|max| literal <= max) {
            return;
        }
        let bounds = match max {
            Some(max) => format!("values from {min} to {max}"),
            None => format!("values from {min} up"),
        };
        let value_kind = if is_integer_literal(value) {
            "integer literal"
        } else {
            "integer constant result"
        };
        let mut help = Vec::new();
        if let Some(wider) = narrowest_integer_type_for(literal, &expected) {
            help.push(format!(
                "the narrowest type that holds this value is `{wider}`; widen the type or change the value to fit"
            ));
        } else {
            help.push(format!("keep the value within the bounds of `{expected}`"));
        }
        self.error_at_with_help(
            "E0706",
            format!("{value_kind} `{literal}` does not fit in `{expected}`, which holds {bounds}"),
            value.span,
            help,
        );
    }

    pub(super) fn error_on_expr(&mut self, code: &'static str, message: String, expr: &Expr) {
        if expr.span.is_dummy() {
            self.error(code, message, expression_subject(expr));
        } else {
            self.error_at(code, message, expr.span);
        }
    }
}

#[cfg(test)]
mod cancellation_tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn expression_checking_stops_before_work_when_cancelled() {
        let mut checker = TypeChecker::new();
        checker.set_cancellation(Arc::new(AtomicBool::new(true)));
        let expression = Expr::synthetic(ExprKind::IntLit(1));

        assert_eq!(checker.check_expr(&expression), ValueType::Unknown);
        assert!(checker.expressions.is_empty());
        assert!(checker.diagnostics.is_empty());
    }
}
