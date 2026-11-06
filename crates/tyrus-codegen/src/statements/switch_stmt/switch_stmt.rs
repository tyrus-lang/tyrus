use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_switch_stmt(
        &mut self,
        stmt: &Stmt,
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        match stmt.kind() {
            StmtKind::Switch {
                expr,
                cases,
                default,
                default_index,
            } => {
                let subject_type = self
                    .expr_type_hint(expr)
                    .ok_or_else(|| "error: cannot infer type for switch expression".to_string())?;

                if subject_type == Type::Str {
                    return self.emit_string_switch(expr, cases, default, *default_index, fn_val);
                }

                let subject = self.emit_expr(expr)?;
                self.forget_owned_native_result(subject);
                self.discard_owned_native_result();
                let subject_int = subject.into_int_value();

                let end_block = self.context.append_basic_block(fn_val, "switch.end");
                let case_blocks = (0..cases.len())
                    .map(|index| {
                        self.context
                            .append_basic_block(fn_val, &format!("switch.case.{index}"))
                    })
                    .collect::<Vec<_>>();
                let default_block = default
                    .as_ref()
                    .map(|_| self.context.append_basic_block(fn_val, "switch.default"));

                let jump_table = self.switch_jump_table_labels(&subject_type, cases)?;

                if let Some(labels) = jump_table {
                    let else_block = default_block.unwrap_or(end_block);
                    let arms = labels
                        .into_iter()
                        .zip(case_blocks.iter().copied())
                        .collect::<Vec<_>>();
                    self.builder
                        .build_switch(subject_int, else_block, &arms)
                        .unwrap();
                } else {
                    for (i, case) in cases.iter().enumerate() {
                        let next_comparison = if i + 1 < cases.len() {
                            self.context
                                .append_basic_block(fn_val, &format!("switch.next.{i}"))
                        } else {
                            default_block.unwrap_or(end_block)
                        };

                        let case_val = if is_integer_type(&subject_type) {
                            self.emit_expr_as(&case.value, &subject_type)?
                        } else {
                            self.emit_expr(&case.value)?
                        };
                        let eq = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                subject_int,
                                case_val.into_int_value(),
                                &format!("switch.cmp.{i}"),
                            )
                            .unwrap();
                        self.builder
                            .build_conditional_branch(eq, case_blocks[i], next_comparison)
                            .unwrap();
                        if i + 1 < cases.len() {
                            self.builder.position_at_end(next_comparison);
                        }
                    }
                    if cases.is_empty() {
                        self.builder
                            .build_unconditional_branch(default_block.unwrap_or(end_block))
                            .unwrap();
                    }
                }

                self.push_break_target(end_block);
                for (i, case) in cases.iter().enumerate() {
                    self.builder.position_at_end(case_blocks[i]);
                    self.push_scope();
                    for stmt in &case.body {
                        if self.current_block_terminated() {
                            break;
                        }
                        self.emit_stmt(stmt, fn_val)?;
                    }
                    self.pop_scope();
                    if !self.current_block_terminated() {
                        let fallthrough = if default_index == &Some(i + 1) {
                            default_block.expect("switch default index must have a block")
                        } else {
                            case_blocks.get(i + 1).copied().unwrap_or(end_block)
                        };
                        self.builder
                            .build_unconditional_branch(fallthrough)
                            .unwrap();
                    }
                }

                if let (Some(default_body), Some(default_block)) = (default, default_block) {
                    self.builder.position_at_end(default_block);
                    self.push_scope();
                    for stmt in default_body {
                        if self.current_block_terminated() {
                            break;
                        }
                        self.emit_stmt(stmt, fn_val)?;
                    }
                    self.pop_scope();
                    if !self.current_block_terminated() {
                        let fallthrough = case_blocks
                            .get(default_index.expect("switch default must have an index"))
                            .copied()
                            .unwrap_or(end_block);
                        self.builder
                            .build_unconditional_branch(fallthrough)
                            .unwrap();
                    }
                }
                self.pop_break_target();

                self.builder.position_at_end(end_block);
                Ok(())
            }
            StmtKind::Throw(expr) => {
                self.emit_throw(expr, fn_val)?;
                Ok(())
            }

            StmtKind::TryCatch {
                try_body,
                catch_binding,
                catch_type,
                catch_type_span: _,
                catch_body,
                finally_body,
            } => self.emit_try_catch(
                try_body,
                catch_binding,
                catch_type,
                catch_body,
                finally_body.as_deref(),
                fn_val,
            ),
            _ => Err("internal: not a switch, throw or try statement".to_string()),
        }
    }
}
