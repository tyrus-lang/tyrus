use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_cartesian_product(
        &mut self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let other = self.functional_array_view(&args[0])?;
        let tuple_element =
            Type::Tuple(vec![view.element_type.clone(), other.element_type.clone()]);
        let (output, output_type) = self.empty_array_slot(&tuple_element, "cartesian_output");
        let product = self
            .builder
            .build_int_mul(view.length, other.length, "cartesian_product_length")
            .unwrap();
        let overflow = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                product,
                view.length,
                "cartesian_overflow",
            )
            .unwrap();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: cartesianProduct outside function")?;
        let error = self
            .context
            .append_basic_block(function, "cartesian.overflow");
        let proceed = self
            .context
            .append_basic_block(function, "cartesian.proceed");
        self.builder
            .build_conditional_branch(overflow, error, proceed)
            .unwrap();
        self.builder.position_at_end(error);
        let message = self
            .builder
            .build_global_string_ptr(
                "error: cartesianProduct result is too large\n",
                "cartesian_error",
            )
            .unwrap();
        self.builder
            .build_call(
                self.declare_dprintf(),
                &[
                    self.context.i32_type().const_int(2, false).into(),
                    message.as_pointer_value().into(),
                ],
                "",
            )
            .unwrap();
        self.emit_exit_failure();
        self.builder.position_at_end(proceed);
        self.reserve_array(output, product, &tuple_element)?;
        self.emit_forward_loop(view.length, "array.cartesian.outer", |this, left_index| {
            let left = this.load_array_element(view, left_index);
            this.emit_forward_loop(
                other.length,
                "array.cartesian.inner",
                |this, right_index| {
                    let right = this.load_array_element(&other, right_index);
                    let tuple_type = this.llvm_type(&tuple_element).into_struct_type();
                    let tuple = tuple_type.get_undef();
                    let tuple = this
                        .builder
                        .build_insert_value(tuple, left, 0, "cartesian_left")
                        .map_err(|error| error.to_string())?;
                    let tuple = this
                        .builder
                        .build_insert_value(tuple, right, 1, "cartesian_right")
                        .map_err(|error| error.to_string())?;
                    this.push_array_value(output, &tuple_element, tuple.into_struct_value().into())
                },
            )
        })?;
        self.builder
            .build_load(output_type, output, "cartesian_result")
            .map_err(|error| error.to_string())
    }

    pub(super) fn emit_predicate_terminal(
        &mut self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
        initial: bool,
        terminal: bool,
        label: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let available = [
            view.element_type.clone(),
            Type::Usize,
            view.array_type.clone(),
        ];
        let predicate = self.hash_callback_expr(&args[0]);
        let (callback, callback_params) =
            self.emit_callback(&predicate, &available, &Type::Bool)?;
        let result = self
            .entry_alloca(self.context.bool_type(), "predicate_result")
            .unwrap();
        self.builder
            .build_store(
                result,
                self.context.bool_type().const_int(initial as u64, false),
            )
            .unwrap();
        let function = self
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();
        let done = self
            .context
            .append_basic_block(function, &format!("array.{label}.terminal"));
        self.emit_forward_loop(view.length, &format!("array.{label}"), |this, index| {
            let element = this.load_array_element(view, index);
            let arguments =
                this.callback_arguments(callback_params.len(), element, index, view.value);
            let predicate = this
                .emit_closure_call(callback, &callback_params, &Type::Bool, &arguments)?
                .unwrap()
                .into_int_value();
            let reached = if terminal {
                predicate
            } else {
                this.builder
                    .build_not(predicate, "predicate_negated")
                    .unwrap()
            };
            let resume = this
                .context
                .append_basic_block(function, "predicate.resume");
            let terminal_block = this
                .context
                .append_basic_block(function, "predicate.terminal");
            this.builder
                .build_conditional_branch(reached, terminal_block, resume)
                .unwrap();
            this.builder.position_at_end(terminal_block);
            this.builder
                .build_store(
                    result,
                    this.context.bool_type().const_int(terminal as u64, false),
                )
                .unwrap();
            this.builder.build_unconditional_branch(done).unwrap();
            this.builder.position_at_end(resume);
            Ok(())
        })?;
        self.builder.build_unconditional_branch(done).unwrap();
        self.builder.position_at_end(done);
        self.release_synchronous_closure(callback, false)?;
        self.builder
            .build_load(self.context.bool_type(), result, "predicate_result")
            .map_err(|error| error.to_string())
    }

    pub(super) fn emit_count(
        &mut self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i64_type = self.context.i64_type();
        let Some(arg) = args.first() else {
            return Ok(view.length.into());
        };
        match self.count_free_variable(arg) {
            None => {
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let i32_type = self.context.i32_type();
                let value_slot = self
                    .entry_alloca(self.llvm_type(&view.element_type), "count_value")
                    .unwrap();
                let value = self.emit_expr_as(arg, &view.element_type)?;
                self.builder.build_store(value_slot, value).unwrap();
                let element_size =
                    self.llvm_type(&view.element_type)
                        .size_of()
                        .ok_or_else(|| {
                            format!(
                                "error: array element type `{}` has no native size",
                                view.element_type
                            )
                        })?;
                let kind = i32_type.const_int(self.array_element_kind(&view.element_type), false);
                let function = self.declare_libc_fn(
                    "tyrus_array_count_equal",
                    &[
                        pointer.into(),
                        pointer.into(),
                        i64_type.into(),
                        i32_type.into(),
                    ],
                    Some(i64_type.into()),
                );
                Ok(self
                    .builder
                    .build_call(
                        function,
                        &[
                            view.slot.into(),
                            value_slot.into(),
                            element_size.into(),
                            kind.into(),
                        ],
                        "array_count_equal",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap())
            }
            Some(free_name) => {
                let available = [
                    view.element_type.clone(),
                    Type::Usize,
                    view.array_type.clone(),
                ];
                let synthetic = implicit_count_condition(&free_name, arg.clone());
                let (callback, callback_params) =
                    self.emit_callback(&synthetic, &available, &Type::Bool)?;
                let counter = self.entry_alloca(i64_type, "count_result").unwrap();
                self.builder
                    .build_store(counter, i64_type.const_zero())
                    .unwrap();
                self.emit_forward_loop(view.length, "array.count", |this, index| {
                    let element = this.load_array_element(view, index);
                    let arguments =
                        this.callback_arguments(callback_params.len(), element, index, view.value);
                    let matched = this
                        .emit_closure_call(callback, &callback_params, &Type::Bool, &arguments)?
                        .unwrap()
                        .into_int_value();
                    let current = this
                        .builder
                        .build_load(i64_type, counter, "count_current")
                        .unwrap()
                        .into_int_value();
                    let incremented = this
                        .builder
                        .build_int_add(current, i64_type.const_int(1, false), "count_incremented")
                        .unwrap();
                    let next = this
                        .builder
                        .build_select(matched, incremented, current, "count_next")
                        .unwrap()
                        .into_int_value();
                    this.builder.build_store(counter, next).unwrap();
                    Ok(())
                })?;
                self.release_synchronous_closure(callback, false)?;
                self.builder
                    .build_load(i64_type, counter, "count_result")
                    .map_err(|error| error.to_string())
            }
        }
    }

    pub(super) fn count_free_variable(&self, arg: &Expr) -> Option<String> {
        let mut free: Vec<String> = tyrus_core::ast::expr_referenced_identifiers(arg)
            .into_iter()
            .filter(|name| self.load_var(name).is_none() && !self.fn_signatures.contains_key(name))
            .collect();
        free.sort();
        free.into_iter().next()
    }

    pub(super) fn reduce_callback_result_is_owned(&self, accumulator_type: &Type) -> bool {
        matches!(transparent_codegen_type(accumulator_type), Type::Str)
    }

    pub(super) fn emit_reduce(
        &mut self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
        expected: Option<&Type>,
        reverse: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let accumulator_type = expected
            .cloned()
            .or_else(|| args.get(1).and_then(|value| self.codegen_expr_type(value)))
            .unwrap_or_else(|| view.element_type.clone());
        let callback_builds_fresh_string = self.reduce_callback_result_is_owned(&accumulator_type);
        let owned_slot = self
            .entry_alloca(self.context.bool_type(), "reduce_accumulator_owned")
            .unwrap();
        let accumulator = self
            .entry_alloca(self.llvm_type(&accumulator_type), "reduce_accumulator")
            .unwrap();
        let start = if let Some(initial) = args.get(1) {
            let value = self.emit_expr_as(initial, &accumulator_type)?;
            self.builder.build_store(accumulator, value).unwrap();
            if callback_builds_fresh_string {
                let is_owned = self.forget_owned_native_result(value);
                self.builder
                    .build_store(
                        owned_slot,
                        self.context.bool_type().const_int(is_owned as u64, false),
                    )
                    .unwrap();
            }
            if reverse {
                self.builder
                    .build_int_sub(
                        view.length,
                        self.context.i64_type().const_int(1, false),
                        "reduce_right_start",
                    )
                    .unwrap()
            } else {
                self.context.i64_type().const_zero()
            }
        } else {
            self.emit_array_bounds_check(self.context.i64_type().const_zero(), view.length)?;
            let initial_index = if reverse {
                self.builder
                    .build_int_sub(
                        view.length,
                        self.context.i64_type().const_int(1, false),
                        "reduce_right_initial",
                    )
                    .unwrap()
            } else {
                self.context.i64_type().const_zero()
            };
            let value = self.load_array_element(view, initial_index);
            self.builder.build_store(accumulator, value).unwrap();
            if callback_builds_fresh_string {
                self.builder
                    .build_store(owned_slot, self.context.bool_type().const_zero())
                    .unwrap();
            }
            if reverse {
                self.builder
                    .build_int_sub(
                        initial_index,
                        self.context.i64_type().const_int(1, false),
                        "reduce_right_next",
                    )
                    .unwrap()
            } else {
                self.context.i64_type().const_int(1, false)
            }
        };
        let available = [
            accumulator_type.clone(),
            view.element_type.clone(),
            Type::Usize,
            view.array_type.clone(),
        ];
        let (callback, callback_params) =
            self.emit_callback(&args[0], &available, &accumulator_type)?;
        let function = self
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();
        let index_slot = self
            .entry_alloca(self.context.i64_type(), "reduce_index")
            .unwrap();
        self.builder.build_store(index_slot, start).unwrap();
        let condition = self
            .context
            .append_basic_block(function, "reduce.condition");
        let body = self.context.append_basic_block(function, "reduce.body");
        let done = self.context.append_basic_block(function, "reduce.done");
        self.builder.build_unconditional_branch(condition).unwrap();
        self.builder.position_at_end(condition);
        let index = self
            .builder
            .build_load(self.context.i64_type(), index_slot, "reduce_index")
            .unwrap()
            .into_int_value();
        let continues = if reverse {
            self.builder
                .build_int_compare(
                    IntPredicate::SGE,
                    index,
                    self.context.i64_type().const_zero(),
                    "reduce_right_continues",
                )
                .unwrap()
        } else {
            self.builder
                .build_int_compare(IntPredicate::ULT, index, view.length, "reduce_continues")
                .unwrap()
        };
        self.builder
            .build_conditional_branch(continues, body, done)
            .unwrap();
        self.builder.position_at_end(body);
        let current = self
            .builder
            .build_load(
                self.llvm_type(&accumulator_type),
                accumulator,
                "accumulator",
            )
            .unwrap();
        let element = self.load_array_element(view, index);
        let all = [current, element, index.into(), view.value.into()];
        let callback_args = all
            .into_iter()
            .take(callback_params.len())
            .collect::<Vec<_>>();
        let next = self
            .emit_closure_call(
                callback,
                &callback_params,
                &accumulator_type,
                &callback_args,
            )?
            .unwrap();
        if callback_builds_fresh_string {
            // `next` is a brand new allocation on every call this predicate
            // proved (never the same pointer as `current`) so freeing
            // whatever `current` held right before it is overwritten can
            // never double free `next` itself. Whether *this* runtime pass
            // needs that free depends on the bit, not on anything knowable
            // while compiling the loop body once.
            let owned_now = self
                .builder
                .build_load(
                    self.context.bool_type(),
                    owned_slot,
                    "reduce_accumulator_owned",
                )
                .unwrap()
                .into_int_value();
            let free_block = self
                .context
                .append_basic_block(function, "reduce.accumulator.free");
            let after_free_block = self
                .context
                .append_basic_block(function, "reduce.accumulator.after_free");
            self.builder
                .build_conditional_branch(owned_now, free_block, after_free_block)
                .unwrap();
            self.builder.position_at_end(free_block);
            self.emit_owned_native_release(current, &accumulator_type, &ReleasePlan::Deep);
            self.builder
                .build_unconditional_branch(after_free_block)
                .unwrap();
            self.builder.position_at_end(after_free_block);
            self.builder
                .build_store(owned_slot, self.context.bool_type().const_int(1, false))
                .unwrap();
        }
        self.builder.build_store(accumulator, next).unwrap();
        let next_index = if reverse {
            self.builder
                .build_int_sub(
                    index,
                    self.context.i64_type().const_int(1, false),
                    "reduce_right_next",
                )
                .unwrap()
        } else {
            self.builder
                .build_int_add(
                    index,
                    self.context.i64_type().const_int(1, false),
                    "reduce_next",
                )
                .unwrap()
        };
        self.builder.build_store(index_slot, next_index).unwrap();
        self.builder.build_unconditional_branch(condition).unwrap();
        self.builder.position_at_end(done);
        self.release_synchronous_closure(callback, false)?;
        let result = self
            .builder
            .build_load(
                self.llvm_type(&accumulator_type),
                accumulator,
                "reduce_result",
            )
            .map_err(|error| error.to_string())?;
        if callback_builds_fresh_string {
            let owned_bit = self
                .builder
                .build_load(
                    self.context.bool_type(),
                    owned_slot,
                    "reduce_accumulator_owned_final",
                )
                .unwrap()
                .into_int_value();
            self.record_conditionally_owned_result(result, accumulator_type, owned_bit);
        }
        Ok(result)
    }
}
