use super::*;

impl TypeChecker {
    pub(super) fn check_unary_expr(&mut self, expr: &Expr) -> ValueType {
        match expr.kind() {
            ExprKind::Borrow {
                mutable,
                name,
                place,
            } => {
                let borrowed = match place {
                    Some(place) => self.check_expr(place),
                    None => self.read_symbol(name),
                };
                match borrowed {
                    ValueType::Known(ty) if *mutable => {
                        self.invalidate_array_len(name);
                        ValueType::Known(Type::MutRef(Box::new(ty)))
                    }
                    ValueType::Known(ty) => ValueType::Known(Type::Ref(Box::new(ty))),
                    other => other,
                }
            }
            ExprKind::Unwrap(inner) => {
                let subject = expression_subject(inner).map(str::to_string);
                match self.check_expr(inner) {
                    ValueType::Known(Type::Optional(optional_inner)) => {
                        self.error(
                            "E0716",
                            "forced unwrap of optional value is not allowed".to_string(),
                            subject.as_deref(),
                        );
                        ValueType::Known(*optional_inner)
                    }
                    ValueType::Error => ValueType::Error,
                    actual => {
                        self.error(
                            "E0277",
                            format!("cannot unwrap non-optional type `{}`", actual.display()),
                            expression_subject(inner),
                        );
                        ValueType::Error
                    }
                }
            }
            ExprKind::Try(inner) => {
                if self.in_finalizer() {
                    self.error(
                        "E0D06",
                        format!(
                            "`?` cannot appear inside a `{}` body",
                            self.active_finalizer_name().unwrap_or("defer")
                        ),
                        None,
                    );
                    self.check_expr(inner);
                    return ValueType::Error;
                }
                let actual = self.check_expr(inner);
                match actual {
                    ValueType::Known(Type::Result { success, error }) => {
                        let Some(expected) = self.expected_return.as_ref() else {
                            self.error(
                                "E3077",
                                "`?` can only be used inside a function returning `Result`"
                                    .to_string(),
                                expression_subject(inner),
                            );
                            return ValueType::Error;
                        };
                        let Type::Result {
                            error: expected_error,
                            ..
                        } = expected
                        else {
                            self.error(
                                "E3078",
                                "`?` requires the enclosing function to return `Result<T, E>`"
                                    .to_string(),
                                expression_subject(inner),
                            );
                            return ValueType::Error;
                        };
                        if *error != **expected_error {
                            self.error(
                                "E3071",
                                format!("`?` propagates `{error}`, but the enclosing function returns `{expected_error}` as its error type"),
                                expression_subject(inner),
                            );
                        }
                        ValueType::Known(*success)
                    }
                    ValueType::Error => ValueType::Error,
                    other => {
                        let awaits_a_result = matches!(
                            &other,
                            ValueType::Known(Type::Apply { name, args })
                                if name == "Promise"
                                    && args.len() == 1
                                    && matches!(args[0], Type::Result { .. })
                        );
                        self.error(
                            "E3076",
                            format!(
                                "`?` requires a `Result<T, E>` value, found `{}`",
                                other.display()
                            ),
                            expression_subject(inner),
                        );
                        if awaits_a_result {
                            let written = match inner.kind() {
                                ExprKind::FnCall { callee, .. } => format!("{callee}(...)"),
                                ExprKind::MethodCall { method, .. } => format!("...{method}(...)"),
                                _ => crate::ast::expr_display(inner),
                            };
                            if let Some(last) = self.diagnostics.last_mut() {
                                last.help = vec![format!(
                                    "await the promise before propagating its result: `await {written}?` awaits first, then applies `?` to the `Result`"
                                )];
                            }
                        }
                        ValueType::Error
                    }
                }
            }
            ExprKind::Not(inner) => {
                let actual = self.check_expr(inner);
                if !is_optional_value(&actual) && !is_result_value(&actual) {
                    self.require_bool(&actual, "operand of `!`");
                }
                ValueType::Known(Type::Bool)
            }
            ExprKind::Neg(inner) => {
                let actual = self.check_expr(inner);
                if !is_numeric(&actual) {
                    self.error(
                        "E0308",
                        format!(
                            "operand of unary `-` must be a number, found `{}`",
                            actual.display()
                        ),
                        expression_subject(inner),
                    );
                }
                if matches!(
                    actual,
                    ValueType::IntegerLiteral | ValueType::NegativeIntegerLiteral
                ) {
                    return ValueType::NegativeIntegerLiteral;
                }
                actual
            }
            ExprKind::BitNot(inner) => {
                let actual = self.check_expr(inner);
                if !is_integer_value(&actual) {
                    self.error(
                        "E0308",
                        format!(
                            "operand of unary `~` must be an integer, found `{}`",
                            actual.display()
                        ),
                        expression_subject(inner),
                    );
                }
                actual
            }
            ExprKind::Typeof(inner) => {
                self.check_expr(inner);
                ValueType::Known(Type::Str)
            }
            _ => unreachable!("internal: not a unary expression"),
        }
    }
}
