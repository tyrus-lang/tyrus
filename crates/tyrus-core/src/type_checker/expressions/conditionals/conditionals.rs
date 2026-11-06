use super::*;

impl TypeChecker {
    pub(super) fn check_conditional_expr(&mut self, expr: &Expr) -> ValueType {
        match expr.kind() {
            ExprKind::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                let condition = self.check_expr(cond);
                if !is_optional_value(&condition) {
                    self.require_bool(&condition, "ternary condition");
                }
                let narrow = null_narrowing(cond);
                let then_ty = self.check_expr_narrowed(then_expr, narrow.as_ref(), true);
                let else_ty = self.check_expr_narrowed(else_expr, narrow.as_ref(), false);
                if is_same_value_type(&then_ty, &else_ty) {
                    then_ty
                } else if let Some(else_concrete) = concrete_value_type(&else_ty) {
                    if self.is_assignable(&else_concrete, &then_ty) {
                        ValueType::Known(else_concrete)
                    } else {
                        self.error(
                            "E0308",
                            format!(
                                "ternary types do not match: `{}` and `{}`",
                                then_ty.display(),
                                else_ty.display()
                            ),
                            expression_subject(cond),
                        );
                        ValueType::Error
                    }
                } else if let Some(then_concrete) = concrete_value_type(&then_ty) {
                    if self.is_assignable(&then_concrete, &else_ty) {
                        ValueType::Known(then_concrete)
                    } else {
                        self.error(
                            "E0308",
                            format!(
                                "ternary types do not match: `{}` and `{}`",
                                then_ty.display(),
                                else_ty.display()
                            ),
                            expression_subject(cond),
                        );
                        ValueType::Error
                    }
                } else {
                    self.error(
                        "E0308",
                        format!(
                            "ternary types do not match: `{}` and `{}`",
                            then_ty.display(),
                            else_ty.display()
                        ),
                        expression_subject(cond),
                    );
                    ValueType::Error
                }
            }
            ExprKind::If {
                cond,
                then_block,
                else_block,
            } => {
                let condition = self.check_expr(cond);
                if !is_optional_value(&condition) {
                    self.require_bool(&condition, "if condition");
                }
                let null_narrow = null_narrowing(cond);
                let result_narrow = result_narrowing(cond);
                let typeof_narrow = typeof_narrowing(cond);
                let discriminant_narrow = discriminant_narrowing(cond);
                let is_narrow = is_narrowing(cond);
                let known_condition = self.known_integer_condition(cond);
                let initial_lengths = self.array_length_snapshot();
                self.check_narrowed_block(
                    then_block,
                    null_narrow.as_ref(),
                    result_narrow.as_ref(),
                    typeof_narrow.as_ref(),
                    discriminant_narrow.as_ref(),
                    is_narrow.as_ref(),
                    true,
                    known_condition == Some(true),
                );
                let then_lengths = self.array_length_snapshot();
                if known_condition == Some(false) {
                    self.restore_array_lengths(&initial_lengths);
                }
                if let Some(block) = else_block {
                    self.check_narrowed_block(
                        block,
                        null_narrow.as_ref(),
                        result_narrow.as_ref(),
                        typeof_narrow.as_ref(),
                        discriminant_narrow.as_ref(),
                        is_narrow.as_ref(),
                        false,
                        known_condition == Some(false),
                    );
                }
                match known_condition {
                    Some(true) => self.restore_array_lengths(&then_lengths),
                    Some(false) if else_block.is_none() => {
                        self.restore_array_lengths(&initial_lengths);
                    }
                    _ => {}
                }
                ValueType::Known(Type::Void)
            }
            ExprKind::Loop { body } => {
                self.invalidate_loop_mutations(body);
                let value_type = Rc::new(RefCell::new(None));
                self.control_stack.push(ControlContext::LoopExpr {
                    value_type: value_type.clone(),
                });
                self.branch_depth += 1;
                self.check_block(body);
                self.branch_depth -= 1;
                self.control_stack.pop();
                let loop_type = value_type.borrow().clone();
                match loop_type {
                    Some(ty) => ValueType::Known(ty),
                    None => ValueType::Known(Type::Void),
                }
            }
            ExprKind::TypeAssert { value, ty } => {
                let source = self.check_expr(value);
                self.check_type_assertion(&source, ty, value);
                ValueType::Known(ty.clone())
            }
            ExprKind::ConstAssert(value) => {
                if !is_const_assertion_value(value) {
                    self.error(
                        "E1355",
                        "`as const` can only be applied to literals, arrays, and object literals"
                            .to_string(),
                        expression_subject(value),
                    );
                }
                self.check_expr(value);
                ValueType::Known(const_assertion_type(value))
            }
            _ => unreachable!("internal: not a conditional expression"),
        }
    }
}
