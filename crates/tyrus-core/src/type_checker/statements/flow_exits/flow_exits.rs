use super::*;

impl TypeChecker {
    #[allow(clippy::too_many_lines)]
    pub(super) fn check_flow_exit_stmt(&mut self, statement: &Stmt) {
        match statement.kind() {
            StmtKind::Return(value) => {
                if self.in_finalizer() {
                    self.error(
                        "E0D02",
                        format!(
                            "`return` cannot appear inside a `{}` body",
                            self.active_finalizer_name().unwrap_or("defer")
                        ),
                        Some("return"),
                    );
                }
                let expected = self.expected_return.clone();
                match (expected, value) {
                    (None, _) => self.error(
                        "E0572",
                        "return statement outside a function".to_string(),
                        None,
                    ),
                    (Some(Type::Void), None) => {}
                    (Some(Type::Void), Some(expr)) => {
                        let actual = self.check_expr(expr);
                        if !matches!(actual, ValueType::Known(Type::Void) | ValueType::Error) {
                            self.error(
                                "E0069",
                                "a void function cannot return a value".to_string(),
                                None,
                            );
                        }
                    }
                    (Some(Type::Infer), Some(expr)) => {
                        let actual = self.check_expr(expr);
                        if let Some(returns) = self.inferred_returns.last_mut() {
                            returns.push(actual);
                        }
                    }
                    (Some(Type::Infer), None) => {
                        if let Some(returns) = self.inferred_returns.last_mut() {
                            returns.push(ValueType::Known(Type::Void));
                        }
                    }
                    (Some(Type::Result { success, error }), Some(expr)) => {
                        if let ExprKind::FnCall {
                            callee,
                            args,
                            type_args,
                        } = expr.kind()
                            && matches!(callee.as_str(), "Ok" | "Err")
                        {
                            let target = if callee == "Ok" {
                                success.as_ref()
                            } else {
                                error.as_ref()
                            };
                            if callee == "Ok"
                                && type_args.is_empty()
                                && args.is_empty()
                                && matches!(target, Type::Void)
                            {
                                return;
                            }
                            if !type_args.is_empty() || args.len() != 1 {
                                self.error(
                                        "E3070",
                                        format!("`{callee}` requires exactly one payload argument, except `Ok()` for `Result<void, E>`"),
                                        Some(callee),
                                    );
                                return;
                            }
                            let actual = self.check_expr(&args[0]);
                            let actual_ty = value_type_to_type(&actual);
                            let exact = actual_ty.as_ref().is_some_and(|ty| {
                                ty == target
                                    || matches!(
                                        (&actual, target),
                                        (
                                            ValueType::IntegerLiteral
                                                | ValueType::NegativeIntegerLiteral,
                                            Type::I32
                                        ) | (ValueType::FloatLiteral, Type::F64)
                                    )
                            });
                            if !exact && !matches!(actual, ValueType::Error) {
                                self.error(
                                        "E0308",
                                        format!("`{callee}` payload must have exactly type `{target}`, found `{}`", actual.display()),
                                        expression_subject(&args[0]),
                                    );
                            }
                            return;
                        }
                        let actual = self.check_expr(expr);
                        let actual_ty = value_type_to_type(&actual);
                        if let Some(Type::Result {
                            success: actual_success,
                            error: actual_error,
                        }) = actual_ty.as_ref()
                            && actual_success.as_ref() == success.as_ref()
                            && actual_error.as_ref() == error.as_ref()
                        {
                            return;
                        }
                        let success_match = actual_ty.as_ref().is_some_and(|ty| {
                            ty == success.as_ref()
                                || matches!(
                                    (&actual, success.as_ref()),
                                    (
                                        ValueType::IntegerLiteral
                                            | ValueType::NegativeIntegerLiteral,
                                        Type::I32
                                    ) | (ValueType::FloatLiteral, Type::F64)
                                )
                        });
                        let error_match = actual_ty.as_ref().is_some_and(|ty| {
                            ty == error.as_ref()
                                || matches!(
                                    (&actual, error.as_ref()),
                                    (
                                        ValueType::IntegerLiteral
                                            | ValueType::NegativeIntegerLiteral,
                                        Type::I32
                                    ) | (ValueType::FloatLiteral, Type::F64)
                                )
                        });
                        if !matches!(actual, ValueType::Error) {
                            let (code, constructor, payload) = if success_match && !error_match {
                                ("E3074", "Ok", success.as_ref())
                            } else if error_match && !success_match {
                                ("E3074", "Err", error.as_ref())
                            } else if success_match && error_match {
                                self.error_at_with_help(
                                    "E3074",
                                    format!(
                                        "a `Result<{success}, {error}>` return must use `Ok(...)` or `Err(...)`; found `{}`",
                                        actual.display()
                                    ),
                                    expr.span,
                                    vec![
                                        format!("wrap this `{success}` value in `Ok(...)`"),
                                        format!("wrap this `{error}` value in `Err(...)`"),
                                    ],
                                );
                                return;
                            } else {
                                self.error(
                                    "E0308",
                                    format!(
                                        "return value `{}` does not exactly match success type `{}` or error type `{}`",
                                        actual.display(), success, error
                                    ),
                                    Some("return"),
                                );
                                return;
                            };
                            self.error_at_with_help(
                                code,
                                format!("a `Result<{success}, {error}>` return must use `{constructor}(...)`; found `{}`", actual.display()),
                                expr.span,
                                vec![format!("wrap this `{payload}` value in `{constructor}(...)`")],
                            );
                        }
                    }
                    (Some(Type::Result { success, error }), None)
                        if matches!(success.as_ref(), Type::Void) =>
                    {
                        self.error_at_with_help(
                            "E3075",
                            format!(
                                "a `Result<{success}, {error}>` return must use `Ok()` instead of a bare return"
                            ),
                            statement.span,
                            vec!["replace `return;` with `return Ok();`".to_string()],
                        );
                    }
                    (Some(expected), Some(expr)) => {
                        let actual = self.check_expr_as(expr, &expected);
                        let code = if self.async_function_stack.last().copied().unwrap_or(false) {
                            "E3296"
                        } else {
                            "E0308"
                        };
                        self.span_stack.push(expr.span);
                        let direct_promise_payload = if code == "E3296" {
                            match &actual {
                                ValueType::Known(actual_ty) => promise_payload(actual_ty),
                                _ => None,
                            }
                        } else {
                            None
                        };
                        let assignable = self.is_assignable(&expected, &actual)
                            || direct_promise_payload.is_some_and(|payload| {
                                self.is_assignable(&expected, &ValueType::Known(payload.clone()))
                            });
                        if code == "E3296"
                            && !assignable
                            && (self.missing_object_field(&expected, &actual).is_some()
                                || self.excess_object_field(&expected, &actual).is_some())
                        {
                            self.require_assignable(&expected, &actual, "return", code);
                        } else if code == "E3296" && !assignable {
                            let expected_subject = expected.to_string();
                            self.error(
                                code,
                                format!(
                                    "async return type mismatch: expected `{expected}`, found `{}`",
                                    actual.display()
                                ),
                                Some(&expected_subject),
                            );
                        } else {
                            if !assignable {
                                self.require_assignable(&expected, &actual, "return", code);
                            }
                        }
                        self.span_stack.pop();
                    }
                    (Some(expected), None) => {
                        if !matches!(transparent_type(&expected), Type::Optional(_)) {
                            let is_async =
                                self.async_function_stack.last().copied().unwrap_or(false);
                            let code = if is_async { "E3296" } else { "E0069" };
                            let message = if is_async {
                                format!(
                                    "async return type mismatch: expected `{expected}`, found `void`"
                                )
                            } else {
                                format!("expected return value of type `{expected}`")
                            };
                            let expected_subject = expected.to_string();
                            self.error(
                                code,
                                message,
                                is_async.then_some(expected_subject.as_str()),
                            );
                        }
                    }
                }
            }
            StmtKind::Break(value) => {
                if self.in_finalizer() {
                    self.error(
                        "E0D03",
                        format!(
                            "`break` cannot appear inside a `{}` body",
                            self.active_finalizer_name().unwrap_or("defer")
                        ),
                        Some("break"),
                    );
                }
                let value_type = value.as_ref().map(|value| self.check_expr(value));
                match self.control_stack.last() {
                    Some(ControlContext::LoopExpr { value_type: slot }) => {
                        if let Some((value_expr, value_ty)) =
                            value.as_ref().zip(value_type.as_ref())
                        {
                            let slot = slot.clone();
                            let slot_ty = slot.borrow().clone();
                            match slot_ty {
                                Some(expected) => {
                                    self.require_assignable(
                                        &expected,
                                        value_ty,
                                        "break value",
                                        "E0308",
                                    );
                                }
                                None => {
                                    if let Some(ty) = infer_binding_type(value_expr, value_ty) {
                                        *slot.borrow_mut() = Some(ty);
                                    }
                                }
                            }
                        }
                    }
                    Some(ControlContext::Loop | ControlContext::Switch) => {
                        if value.is_some() {
                            self.error(
                                "E0571",
                                "`break` with a value can only be used inside a `loop` expression"
                                    .to_string(),
                                Some("break"),
                            );
                        }
                    }
                    Some(ControlContext::Branch(kind))
                        if self.control_stack.iter().any(|context| {
                            matches!(
                                context,
                                ControlContext::Loop
                                    | ControlContext::LoopExpr { .. }
                                    | ControlContext::Switch
                            )
                        }) =>
                    {
                        self.error(
                            "E0269",
                            format!(
                                "`break` inside a `{kind}` arm would exit the enclosing control structure, not the `{kind}`"
                            ),
                            Some("break"),
                        );
                    }
                    _ => self.error(
                        "E0268",
                        "`break` outside a loop or switch".to_string(),
                        Some("break"),
                    ),
                }
            }
            StmtKind::Continue => {
                if self.in_finalizer() {
                    self.error(
                        "E0D04",
                        format!(
                            "`continue` cannot appear inside a `{}` body",
                            self.active_finalizer_name().unwrap_or("defer")
                        ),
                        Some("continue"),
                    );
                }
                if !self.control_stack.iter().any(|context| {
                    matches!(
                        context,
                        ControlContext::Loop | ControlContext::LoopExpr { .. }
                    )
                }) {
                    self.error(
                        "E0268",
                        "`continue` outside a loop".to_string(),
                        Some("continue"),
                    );
                }
            }
            _ => {}
        }
    }
}
