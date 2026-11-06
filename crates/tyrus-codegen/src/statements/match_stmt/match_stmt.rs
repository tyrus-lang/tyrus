use super::*;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_match_stmt(
        &mut self,
        stmt: &Stmt,
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        match stmt.kind() {
            StmtKind::Match {
                expr,
                arms,
                default,
            } => {
                let subject_type = self
                    .expr_type_hint(expr)
                    .ok_or_else(|| "error: cannot infer type for match expression".to_string())?;
                let is_string_match = subject_type == Type::Str;

                let mut subject = self.emit_expr(expr)?;
                let mut subject_release = self.take_owned_native_result(subject);
                if subject_release.is_none()
                    && !self.async_results.is_empty()
                    && !subject_type.is_copy()
                    && self.async_owned_value_supported(&subject_type)
                {
                    let slot = self.entry_alloca(
                        self.llvm_type(&subject_type),
                        "match.awaited.subject.owner",
                    )?;
                    self.emit_async_clone_value_into(slot, subject, &subject_type)?;
                    subject = self
                        .builder
                        .build_load(
                            self.llvm_type(&subject_type),
                            slot,
                            "match.awaited.subject.value",
                        )
                        .map_err(|error| error.to_string())?;
                    subject_release = Some(ReleasePlan::Deep);
                }
                self.discard_owned_native_result();
                self.push_scope();
                if let Some(plan) = subject_release {
                    const MATCH_SUBJECT: &str = "<match subject>";
                    let subject_slot =
                        self.entry_alloca(self.llvm_type(&subject_type), "match.owned.subject")?;
                    self.builder
                        .build_store(subject_slot, subject)
                        .map_err(|error| error.to_string())?;
                    self.store_var(MATCH_SUBJECT, subject_slot, subject_type.clone());
                    if let Some(scope) = self.owned_native_vars.last_mut() {
                        scope.insert(MATCH_SUBJECT.to_string(), plan);
                    }
                }

                let end_block = self.context.append_basic_block(fn_val, "match.end");
                let parent = self.builder.get_insert_block().unwrap();
                let mut current_cont = parent;

                for (i, arm) in arms.iter().enumerate() {
                    let body_block = self
                        .context
                        .append_basic_block(fn_val, &format!("match.arm.{i}"));
                    let next_block = if i < arms.len() - 1 || default.is_some() {
                        self.context
                            .append_basic_block(fn_val, &format!("match.next.{i}"))
                    } else {
                        end_block
                    };

                    self.builder.position_at_end(current_cont);

                    if let MatchPattern::Array { elements, .. } = &arm.pattern {
                        let array_type = self
                            .expr_type_hint(expr)
                            .map(|ty| self.resolve_type(&ty))
                            .ok_or_else(|| {
                                "error: cannot infer type for array match expression".to_string()
                            })?;
                        let Type::Array(element_type) = transparent_codegen_type(&array_type)
                        else {
                            return Err(format!(
                                "error: array match pattern requires an array subject, found `{array_type}`"
                            ));
                        };
                        let element_type = element_type.as_ref().clone();
                        let has_rest = elements.last().is_some_and(MatchArrayElement::is_rest);
                        let required = elements.len() - usize::from(has_rest);
                        let length = self
                            .builder
                            .build_extract_value(
                                subject.into_struct_value(),
                                1,
                                &format!("match.array.{i}.length"),
                            )
                            .map_err(|error| error.to_string())?
                            .into_int_value();
                        let length_matches = self
                            .builder
                            .build_int_compare(
                                if has_rest {
                                    IntPredicate::UGE
                                } else {
                                    IntPredicate::EQ
                                },
                                length,
                                self.context.i64_type().const_int(required as u64, false),
                                &format!("match.array.{i}.length.matches"),
                            )
                            .map_err(|error| error.to_string())?;
                        let bindings_block = self
                            .context
                            .append_basic_block(fn_val, &format!("match.array.bindings.{i}"));
                        let guard_block = self
                            .context
                            .append_basic_block(fn_val, &format!("match.array.guard.{i}"));
                        self.builder
                            .build_conditional_branch(length_matches, bindings_block, next_block)
                            .map_err(|error| error.to_string())?;

                        self.builder.position_at_end(bindings_block);
                        self.push_scope();
                        let mut rest_cleanup = None;
                        let data = self
                            .builder
                            .build_extract_value(
                                subject.into_struct_value(),
                                0,
                                &format!("match.array.{i}.data"),
                            )
                            .map_err(|error| error.to_string())?
                            .into_pointer_value();
                        let mut literals_match = self.context.bool_type().const_all_ones();
                        for (element_index, element) in elements.iter().enumerate() {
                            if element.is_rest() {
                                if let MatchPattern::Binding(name) = &element.pattern {
                                    let subject_slot = self.entry_alloca(
                                        self.llvm_type(&array_type),
                                        "match.array.rest.subject",
                                    )?;
                                    self.builder
                                        .build_store(subject_slot, subject)
                                        .map_err(|error| error.to_string())?;
                                    let output =
                                        self.entry_alloca(self.llvm_type(&array_type), name)?;
                                    let status = self.emit_array_slice_into(
                                        subject_slot,
                                        output,
                                        self.context
                                            .i64_type()
                                            .const_int(element_index as u64, false),
                                        length,
                                        self.llvm_type(&element_type).size_of().ok_or_else(
                                            || {
                                                "error: array element has no native size"
                                                    .to_string()
                                            },
                                        )?,
                                        matches!(
                                            transparent_codegen_type(&element_type),
                                            Type::Str
                                        ),
                                    )?;
                                    self.emit_abort_if_negative(
                                        status,
                                        "array match rest",
                                        "match.array.rest",
                                    )?;
                                    if !matches!(transparent_codegen_type(&element_type), Type::Str)
                                        && !self.resolve_type(&element_type).is_copy()
                                    {
                                        let clone_element =
                                            self.emit_array_element_clone(&element_type)?;
                                        let own_elements = self.declare_libc_fn(
                                            "tyrus_array_own_elements",
                                            &[
                                                self.context
                                                    .ptr_type(inkwell::AddressSpace::default())
                                                    .into(),
                                                self.context.i64_type().into(),
                                                self.context
                                                    .ptr_type(inkwell::AddressSpace::default())
                                                    .into(),
                                            ],
                                            Some(self.context.i32_type().into()),
                                        );
                                        let clone_status = self
                                            .builder
                                            .build_call(
                                                own_elements,
                                                &[
                                                    output.into(),
                                                    self.llvm_type(&element_type)
                                                        .size_of()
                                                        .unwrap()
                                                        .into(),
                                                    clone_element
                                                        .as_global_value()
                                                        .as_pointer_value()
                                                        .into(),
                                                ],
                                                "match_array_rest_own_elements",
                                            )
                                            .unwrap()
                                            .try_as_basic_value()
                                            .basic()
                                            .unwrap()
                                            .into_int_value();
                                        self.emit_abort_if_negative(
                                            clone_status,
                                            "array match rest clone",
                                            "match.array.rest.clone",
                                        )?;
                                    }
                                    let value = self
                                        .builder
                                        .build_load(
                                            self.llvm_type(&array_type),
                                            output,
                                            "match.array.rest.value",
                                        )
                                        .map_err(|error| error.to_string())?;
                                    self.record_owned_native_result(value, array_type.clone());
                                    self.store_var(name, output, array_type.clone());
                                    self.transfer_owned_native_result(name, value);
                                    rest_cleanup = Some(output);
                                }
                                break;
                            }
                            let slot = unsafe {
                                self.builder.build_gep(
                                    self.llvm_type(&element_type),
                                    data,
                                    &[self
                                        .context
                                        .i64_type()
                                        .const_int(element_index as u64, false)],
                                    &format!("match.array.{i}.element.{element_index}"),
                                )
                            }
                            .map_err(|error| error.to_string())?;
                            if let MatchPattern::Lit(literal) = &element.pattern {
                                let value = self
                                    .builder
                                    .build_load(
                                        self.llvm_type(&element_type),
                                        slot,
                                        "match.array.literal.value",
                                    )
                                    .map_err(|error| error.to_string())?;
                                let literal_value = self.emit_expr_as(literal, &element_type)?;
                                let equals =
                                    if matches!(transparent_codegen_type(&element_type), Type::Str)
                                    {
                                        self.emit_string_equality(
                                            value.into_struct_value(),
                                            literal_value.into_struct_value(),
                                            true,
                                        )?
                                        .into_int_value()
                                    } else {
                                        self.emit_comparison(
                                            value,
                                            literal_value,
                                            IntPredicate::EQ,
                                            inkwell::FloatPredicate::OEQ,
                                            &format!("match.array.{i}.literal.{element_index}"),
                                        )?
                                        .into_int_value()
                                    };
                                literals_match = self
                                    .builder
                                    .build_and(literals_match, equals, "match.array.literals")
                                    .map_err(|error| error.to_string())?;
                            }
                            if let MatchPattern::Binding(name) = &element.pattern {
                                self.store_var(name, slot, element_type.clone());
                            }
                        }
                        let failed_pattern_block = rest_cleanup.map(|_| {
                            self.context.append_basic_block(
                                fn_val,
                                &format!("match.array.rest.cleanup.{i}"),
                            )
                        });
                        let failed_pattern_target = failed_pattern_block.unwrap_or(next_block);
                        self.builder
                            .build_conditional_branch(
                                literals_match,
                                guard_block,
                                failed_pattern_target,
                            )
                            .map_err(|error| error.to_string())?;
                        self.builder.position_at_end(guard_block);
                        let guard_matches = if let Some(guard) = &arm.guard {
                            self.emit_expr(guard)?.into_int_value()
                        } else {
                            self.context.bool_type().const_all_ones()
                        };
                        self.builder
                            .build_conditional_branch(
                                guard_matches,
                                body_block,
                                failed_pattern_target,
                            )
                            .map_err(|error| error.to_string())?;

                        if let (Some(cleanup_block), Some(output)) =
                            (failed_pattern_block, rest_cleanup)
                        {
                            self.builder.position_at_end(cleanup_block);
                            let value = self
                                .builder
                                .build_load(
                                    self.llvm_type(&array_type),
                                    output,
                                    "match.array.rest.cleanup.value",
                                )
                                .map_err(|error| error.to_string())?;
                            self.emit_owned_native_value_free(value, &array_type);
                            self.builder
                                .build_unconditional_branch(next_block)
                                .map_err(|error| error.to_string())?;
                        }

                        self.builder.position_at_end(body_block);
                        for stmt in &arm.body {
                            if self.current_block_terminated() {
                                break;
                            }
                            self.emit_stmt(stmt, fn_val)?;
                        }
                        self.pop_scope();
                        if !self.current_block_terminated() {
                            self.builder.build_unconditional_branch(end_block).unwrap();
                        }
                        current_cont = next_block;
                        continue;
                    }

                    self.match_result_bindings.push(HashSet::default());
                    let condition = self.emit_match_condition(
                        &arm.pattern,
                        &arm.guard,
                        expr,
                        subject,
                        is_string_match,
                        i,
                    )?;
                    self.builder
                        .build_conditional_branch(condition, body_block, next_block)
                        .unwrap();

                    self.builder.position_at_end(body_block);
                    self.push_scope();
                    for stmt in &arm.body {
                        if self.current_block_terminated() {
                            break;
                        }
                        self.emit_stmt(stmt, fn_val)?;
                    }
                    self.pop_scope();
                    self.match_result_bindings.pop();
                    if !self.current_block_terminated() {
                        self.builder.build_unconditional_branch(end_block).unwrap();
                    }

                    current_cont = next_block;
                }

                if let Some(default_body) = default {
                    self.builder.position_at_end(current_cont);
                    self.push_scope();
                    for stmt in default_body {
                        if self.current_block_terminated() {
                            break;
                        }
                        self.emit_stmt(stmt, fn_val)?;
                    }
                    self.pop_scope();
                    if !self.current_block_terminated() {
                        self.builder.build_unconditional_branch(end_block).unwrap();
                    }
                }

                self.builder.position_at_end(end_block);
                self.pop_scope();
                Ok(())
            }
            _ => Err("internal: not a match statement".to_string()),
        }
    }
}
