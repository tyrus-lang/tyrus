use super::*;

impl TypeChecker {
    pub(super) fn check_branching_stmt(&mut self, statement: &Stmt) {
        match statement.kind() {
            StmtKind::Switch {
                expr,
                cases,
                default,
                ..
            } => {
                let _expr_ty = self.check_expr(expr);
                self.branch_depth += 1;
                self.control_stack.push(ControlContext::Switch);
                for case in cases {
                    let _ = self.check_expr(&case.value);
                    self.check_block(&case.body);
                }
                if let Some(default_body) = default {
                    self.check_block(default_body);
                }
                self.control_stack.pop();
                self.branch_depth -= 1;
            }
            StmtKind::Match {
                expr,
                arms,
                default,
            } => {
                let expr_ty = self.check_expr(expr);
                if self.check_match_exhaustiveness(&expr_ty, arms, default.is_some())
                    && !statement.span.is_dummy()
                {
                    self.exhaustive_matches.insert(statement.span);
                }
                self.branch_depth += 1;
                self.control_stack.push(ControlContext::Branch("match"));
                for (i, arm) in arms.iter().enumerate() {
                    self.push_scope();
                    self.check_match_pattern_support(&arm.pattern, &expr_ty, arm.pattern_span);
                    self.declare_match_bindings(&arm.pattern, &expr_ty, i);
                    if let Some(guard) = &arm.guard {
                        let guard_ty = self.check_expr(guard);
                        self.require_assignable(&Type::Bool, &guard_ty, "match guard", "E0308");
                    }
                    for stmt in &arm.body {
                        self.check_stmt(stmt);
                    }
                    self.pop_scope();
                }
                if let Some(default_body) = default {
                    self.check_block(default_body);
                }
                self.control_stack.pop();
                self.branch_depth -= 1;
            }
            StmtKind::Throw(expr) => {
                if self.in_finalizer() {
                    self.error_at_with_help(
                        "E0D08",
                        format!(
                            "`throw` is not allowed inside a `{}` finalizer",
                            self.active_finalizer_name().unwrap_or("defer")
                        ),
                        statement.span,
                        vec![
                            "handle the cleanup failure before the finalizer completes".to_string(),
                        ],
                    );
                }
                let actual = self.check_expr(expr);
                let open_catches = self.try_catch_type.clone();
                let thrown_is_error =
                    matches!(&actual, ValueType::Known(actual) if self.type_extends_error(actual));
                let accepting = open_catches
                    .iter()
                    .rev()
                    .find(|expected| {
                        (thrown_is_error
                            && matches!(expected, Type::Named(name) if name == "Error"))
                            || self.is_assignable(expected, &actual)
                    })
                    .cloned();
                let propagated = matches!(&actual, ValueType::Known(actual) if self
                    .current_throws
                    .last()
                    .is_some_and(|types| types.contains(actual)));
                if let ValueType::Known(thrown) = &actual
                    && (accepting.is_some() || propagated)
                {
                    let catching_try = open_catches.iter().rposition(|expected| {
                        (thrown_is_error
                            && matches!(expected, Type::Named(name) if name == "Error"))
                            || self.is_assignable(expected, &actual)
                    });
                    self.refuse_throw_past_awaiting_finally(
                        thrown,
                        "`throw`",
                        statement.span,
                        catching_try,
                    );
                }
                if let Some(expected) = accepting {
                    if !thrown_is_error {
                        self.require_assignable(&expected, &actual, "throw", "E0308");
                    }
                } else if let Some(expected) = open_catches.last().cloned() {
                    if !propagated {
                        self.require_assignable(&expected, &actual, "throw", "E0308");
                    }
                } else if let ValueType::Known(actual) = &actual
                    && !propagated
                {
                    self.error(
                            "E3011",
                            format!("thrown error `{actual}` must be caught or declared with `throws {actual}`"),
                            expression_subject(expr),
                        );
                }
            }
            StmtKind::Defer {
                kind,
                keyword_span,
                body,
            } => {
                if self.async_function_stack.last() == Some(&true) {
                    self.error_at_with_help(
                        "E0D09",
                        "`defer` and `errdefer` are not supported inside async functions yet"
                            .to_string(),
                        *keyword_span,
                        vec!["perform cleanup in an explicit async control-flow path".to_string()],
                    );
                }
                if matches!(kind, DeferKind::OnError)
                    && !matches!(
                        self.expected_return.as_ref(),
                        Some(Type::Result { .. } | Type::Optional(_))
                    )
                {
                    self.error_at_with_help(
                        "E0D01",
                        "`errdefer` requires a function or closure with a fallible return type"
                            .to_string(),
                        *keyword_span,
                        vec!["use `errdefer` only in a function or closure that returns `Result<T, E>` or `T?`".to_string()],
                    );
                }
                if let Some(await_span) = first_await_in_block(body) {
                    self.error_at_with_help(
                        "E0D05",
                        "`await` is not supported inside a `defer` or `errdefer` body".to_string(),
                        await_span,
                        vec![
                            "move the awaited operation before registering the finalizer"
                                .to_string(),
                        ],
                    );
                }
                let captures = closure_capture_names(&[], body);
                self.register_finalizer(*kind, *keyword_span, captures.clone());
                self.finalizer_contexts.push(FinalizerContext {
                    kind: *kind,
                    keyword_span: *keyword_span,
                    captures,
                });
                self.check_block(body);
                self.finalizer_contexts.pop();
            }
            StmtKind::TryCatch {
                try_body,
                catch_binding,
                catch_type,
                catch_type_span,
                catch_body,
                finally_body,
            } => {
                self.branch_depth += 1;
                let catch_type_known = self.check_type_name(catch_type, *catch_type_span);
                self.try_catch_type.push(catch_type.clone());
                self.try_body_suspends
                    .push(first_await_in_block(try_body).is_some());
                self.try_catch_suspends
                    .push(first_await_in_block(catch_body).is_some());
                self.try_finally_suspends.push(
                    finally_body
                        .as_ref()
                        .is_some_and(|body| first_await_in_block(body).is_some()),
                );
                self.check_block(try_body);
                self.try_finally_suspends.pop();
                self.try_catch_suspends.pop();
                self.try_body_suspends.pop();
                self.try_catch_type.pop();

                self.push_scope();
                self.declare(
                    catch_binding,
                    Symbol {
                        ty: if catch_type_known {
                            ValueType::Known(catch_type.clone())
                        } else {
                            ValueType::Error
                        },
                        mutable: false,
                        kind: SymbolKind::Variable,
                        used: false,
                        array_len: None,
                    },
                );
                let binds_error = matches!(catch_type, Type::Named(name) if name == "Error");
                if binds_error {
                    self.error_catch_bindings
                        .push((catch_binding.clone(), self.fn_depth));
                }
                for stmt in catch_body {
                    self.check_stmt(stmt);
                }
                if binds_error {
                    self.error_catch_bindings.pop();
                }
                self.pop_scope();

                if let Some(finally_stmts) = finally_body {
                    self.check_block(finally_stmts);
                }
                self.branch_depth -= 1;
            }
            _ => {}
        }
    }
}
