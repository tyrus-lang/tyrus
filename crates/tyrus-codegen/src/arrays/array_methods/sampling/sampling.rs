use super::*;

impl<'ctx> Codegen<'ctx> {
    /// The methods that pick elements: indexed access and the random ones.
    pub(super) fn emit_array_sampling_method(
        &mut self,
        method: &str,
        args: &[Expr],
        receiver: ArrayReceiver<'_, 'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ArrayReceiver {
            element,
            llvm_array_type,
            pointer,
            i64_type,
            i32_type,
            array_slot,
            element_size,
            ..
        } = receiver;
        match method {
            "at" | "random" => {
                let index = if method == "at" {
                    self.emit_expr_as(&args[0], &Type::I64)?.into_int_value()
                } else {
                    let length = self.array_length_from_slot(array_slot, llvm_array_type);
                    let random_index = self.declare_libc_fn(
                        "tyrus_array_random_index",
                        &[i64_type.into()],
                        Some(i64_type.into()),
                    );
                    self.builder
                        .build_call(random_index, &[length.into()], "array_random_index")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_int_value()
                };
                let function = self.declare_libc_fn(
                    "tyrus_array_at",
                    &[pointer.into(), i64_type.into(), i64_type.into()],
                    Some(pointer.into()),
                );
                let result = self
                    .builder
                    .build_call(
                        function,
                        &[array_slot.into(), index.into(), element_size.into()],
                        "array_at",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| "internal: array.at returned void".to_string())?;
                if method == "at" {
                    Ok(result)
                } else {
                    let pointer = result.into_pointer_value();
                    let function = self
                        .builder
                        .get_insert_block()
                        .and_then(inkwell::basic_block::BasicBlock::get_parent)
                        .ok_or("internal: array.random outside function")?;
                    let failure = self
                        .context
                        .append_basic_block(function, "array_random_empty");
                    let present = self
                        .context
                        .append_basic_block(function, "array_random_present");
                    let empty = self
                        .builder
                        .build_is_null(pointer, "array_random_is_empty")
                        .unwrap();
                    self.builder
                        .build_conditional_branch(empty, failure, present)
                        .unwrap();
                    self.builder.position_at_end(failure);
                    self.emit_runtime_panic(
                        "tyrus runtime error: cannot select a random element from an empty array",
                    );
                    self.builder.position_at_end(present);
                    let value = self
                        .builder
                        .build_load(self.llvm_type(element), pointer, "array_random_value")
                        .map_err(|error| error.to_string())?;
                    if matches!(transparent_codegen_type(element), Type::Str) {
                        Ok(self.emit_string_clone(
                            value.into_struct_value(),
                            "array_random_string_clone",
                        )?)
                    } else {
                        Ok(value)
                    }
                }
            }
            "shuffle" | "sample" => {
                let output = self
                    .entry_alloca(llvm_array_type, "array_random_output")
                    .unwrap();
                let status = if method == "shuffle" {
                    let function = self.declare_libc_fn(
                        "tyrus_array_shuffle",
                        &[pointer.into(), i64_type.into(), pointer.into()],
                        Some(i32_type.into()),
                    );
                    self.builder.build_call(
                        function,
                        &[array_slot.into(), element_size.into(), output.into()],
                        "array_shuffle",
                    )
                } else {
                    let count = self.emit_expr_as(&args[0], &Type::Usize)?.into_int_value();
                    let repeat = if args.len() == 1 {
                        i32_type.const_zero()
                    } else {
                        let ExprKind::ObjectLit(fields) = args[1].kind() else {
                            return Err(
                                "error: array.sample options must be an object literal".to_string()
                            );
                        };
                        match fields.iter().find(|(name, _)| name == "repeat") {
                            Some((_, value)) => {
                                let value = self.emit_expr_as(value, &Type::Bool)?.into_int_value();
                                self.builder
                                    .build_int_z_extend(value, i32_type, "array_sample_repeat")
                                    .map_err(|error| error.to_string())?
                            }
                            None => i32_type.const_zero(),
                        }
                    };
                    let function = self.declare_libc_fn(
                        "tyrus_array_sample",
                        &[
                            pointer.into(),
                            i64_type.into(),
                            i32_type.into(),
                            i64_type.into(),
                            pointer.into(),
                        ],
                        Some(i32_type.into()),
                    );
                    self.builder.build_call(
                        function,
                        &[
                            array_slot.into(),
                            count.into(),
                            repeat.into(),
                            element_size.into(),
                            output.into(),
                        ],
                        "array_sample",
                    )
                }
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap()
                .into_int_value();
                self.emit_abort_if_negative(status, method, "array.method")?;
                if !self.resolve_type(element).is_copy() {
                    let clone_element = self.emit_array_element_clone(element)?;
                    let own_elements = self.declare_libc_fn(
                        "tyrus_array_own_elements",
                        &[pointer.into(), i64_type.into(), pointer.into()],
                        Some(i32_type.into()),
                    );
                    let status = self
                        .builder
                        .build_call(
                            own_elements,
                            &[
                                output.into(),
                                element_size.into(),
                                clone_element.as_global_value().as_pointer_value().into(),
                            ],
                            "array_random_own_elements",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_int_value();
                    self.emit_abort_if_negative(status, method, "array.method")?;
                }
                self.builder
                    .build_load(llvm_array_type, output, "array_random_result")
                    .map_err(|error| error.to_string())
            }
            _ => Err(format!(
                "internal: `{method}` is not an array sampling method"
            )),
        }
    }
}
