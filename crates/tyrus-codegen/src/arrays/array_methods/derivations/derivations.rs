use super::*;

impl<'ctx> Codegen<'ctx> {
    /// The methods that answer a new array or a string instead of touching the receiver.
    pub(super) fn emit_array_derivation_method(
        &mut self,
        method: &str,
        args: &[Expr],
        receiver: ArrayReceiver<'_, 'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ArrayReceiver {
            array_type,
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
            "entries" => self.emit_array_entries(array_slot, llvm_array_type, element),
            "slice" | "concat" | "toReversed" | "with" | "values" => {
                let output = self.entry_alloca(llvm_array_type, "array_output").unwrap();
                let owned_string = matches!(transparent_codegen_type(element), Type::Str);
                let result = if method == "concat" {
                    let right_value = self.emit_expr_as(&args[0], array_type)?;
                    let right = self
                        .entry_alloca(llvm_array_type, "array_concat_right")
                        .unwrap();
                    self.builder.build_store(right, right_value).unwrap();
                    let result = if owned_string {
                        let function = self.declare_libc_fn(
                            "tyrus_array_concat_owned_strings",
                            &[pointer.into(), pointer.into(), pointer.into()],
                            Some(i32_type.into()),
                        );
                        self.builder.build_call(
                            function,
                            &[array_slot.into(), right.into(), output.into()],
                            "array_copy",
                        )
                    } else {
                        let function = self.declare_libc_fn(
                            "tyrus_array_concat",
                            &[
                                pointer.into(),
                                pointer.into(),
                                i64_type.into(),
                                pointer.into(),
                            ],
                            Some(i32_type.into()),
                        );
                        self.builder.build_call(
                            function,
                            &[
                                array_slot.into(),
                                right.into(),
                                element_size.into(),
                                output.into(),
                            ],
                            "array_copy",
                        )
                    };
                    self.emit_async_cleanup_source_expression(right_value, array_type, &args[0])?;
                    result
                } else if method == "with" {
                    let index = self.emit_expr_as(&args[0], &Type::I64)?.into_int_value();
                    if owned_string {
                        let source_value = self.emit_expr_as(&args[1], element)?;
                        let value = source_value.into_struct_value();
                        let function = self.declare_libc_fn(
                            "tyrus_array_with_owned_strings",
                            &[
                                pointer.into(),
                                i64_type.into(),
                                self.string_abi_type(),
                                pointer.into(),
                            ],
                            Some(i32_type.into()),
                        );
                        let result = self.builder.build_call(
                            function,
                            &[array_slot.into(), index.into(), value.into(), output.into()],
                            "array_copy",
                        );
                        self.emit_async_cleanup_source_expression(source_value, element, &args[1])?;
                        result
                    } else {
                        let value = self.array_method_value_ptr(&args[1], element)?;
                        let function = self.declare_libc_fn(
                            "tyrus_array_with",
                            &[
                                pointer.into(),
                                i64_type.into(),
                                pointer.into(),
                                i64_type.into(),
                                pointer.into(),
                            ],
                            Some(i32_type.into()),
                        );
                        self.builder.build_call(
                            function,
                            &[
                                array_slot.into(),
                                index.into(),
                                value.into(),
                                element_size.into(),
                                output.into(),
                            ],
                            "array_copy",
                        )
                    }
                } else {
                    let length = self.array_length_from_slot(array_slot, llvm_array_type);
                    let start = if method == "toReversed" || args.is_empty() {
                        i64_type.const_zero()
                    } else {
                        self.emit_expr_as(&args[0], &Type::I64)?.into_int_value()
                    };
                    let end = if method == "toReversed" || args.len() < 2 {
                        length
                    } else {
                        self.emit_expr_as(&args[1], &Type::I64)?.into_int_value()
                    };
                    if owned_string {
                        let function = self.declare_libc_fn(
                            "tyrus_array_slice_owned_strings",
                            &[
                                pointer.into(),
                                i64_type.into(),
                                i64_type.into(),
                                pointer.into(),
                            ],
                            Some(i32_type.into()),
                        );
                        self.builder.build_call(
                            function,
                            &[array_slot.into(), start.into(), end.into(), output.into()],
                            "array_copy",
                        )
                    } else {
                        let function = self.declare_libc_fn(
                            "tyrus_array_slice",
                            &[
                                pointer.into(),
                                i64_type.into(),
                                i64_type.into(),
                                i64_type.into(),
                                pointer.into(),
                            ],
                            Some(i32_type.into()),
                        );
                        self.builder.build_call(
                            function,
                            &[
                                array_slot.into(),
                                start.into(),
                                end.into(),
                                element_size.into(),
                                output.into(),
                            ],
                            "array_copy",
                        )
                    }
                };
                let result = result
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                self.emit_abort_if_negative(result, method, "array.method")?;
                if !owned_string && !self.resolve_type(element).is_copy() {
                    let clone_element = self.emit_array_element_clone(element)?;
                    let own_elements = self.declare_libc_fn(
                        "tyrus_array_own_elements",
                        &[
                            pointer.into(),
                            self.context.i64_type().into(),
                            pointer.into(),
                        ],
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
                            "array_own_elements",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_int_value();
                    self.emit_abort_if_negative(status, method, "array.method")?;
                }
                if method == "toReversed" {
                    let reverse = self.declare_libc_fn(
                        "tyrus_array_reverse",
                        &[pointer.into(), i64_type.into()],
                        Some(i32_type.into()),
                    );
                    self.builder
                        .build_call(
                            reverse,
                            &[output.into(), element_size.into()],
                            "array_reverse",
                        )
                        .unwrap();
                }
                self.builder
                    .build_load(llvm_array_type, output, "array_result")
                    .map_err(|error| error.to_string())
            }
            "splitAt" => {
                let length = self.array_length_from_slot(array_slot, llvm_array_type);
                let index = self.emit_expr_as(&args[0], &Type::I64)?.into_int_value();
                self.emit_split_at_bounds_check(index, length)?;
                let owned_string = matches!(transparent_codegen_type(element), Type::Str);
                let left_output = self
                    .entry_alloca(llvm_array_type, "array_split_left")
                    .unwrap();
                let left_result = self.emit_array_slice_into(
                    array_slot,
                    left_output,
                    i64_type.const_zero(),
                    index,
                    element_size,
                    owned_string,
                )?;
                self.emit_abort_if_negative(left_result, method, "array.method")?;
                let right_output = self
                    .entry_alloca(llvm_array_type, "array_split_right")
                    .unwrap();
                let right_result = self.emit_array_slice_into(
                    array_slot,
                    right_output,
                    index,
                    length,
                    element_size,
                    owned_string,
                )?;
                self.emit_abort_if_negative(right_result, method, "array.method")?;
                let left_value = self
                    .builder
                    .build_load(llvm_array_type, left_output, "array_split_left_value")
                    .unwrap();
                let right_value = self
                    .builder
                    .build_load(llvm_array_type, right_output, "array_split_right_value")
                    .unwrap();
                let tuple_ty = Type::Tuple(vec![array_type.clone(), array_type.clone()]);
                let tuple_llvm = self.llvm_type(&tuple_ty).into_struct_type();
                let tuple = tuple_llvm.get_undef();
                let tuple = self
                    .builder
                    .build_insert_value(tuple, left_value, 0, "array_split_tuple")
                    .map_err(|error| error.to_string())?;
                let tuple = self
                    .builder
                    .build_insert_value(tuple, right_value, 1, "array_split_tuple")
                    .map_err(|error| error.to_string())?;
                Ok(tuple.into_struct_value().into())
            }
            "join" => {
                let separator = if args.is_empty() {
                    self.const_string_abi(",", "array_join_separator")
                        .into_struct_value()
                } else {
                    self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value()
                };
                let function = self.declare_libc_fn(
                    "tyrus_array_join",
                    &[pointer.into(), self.string_abi_type()],
                    Some(self.string_abi_type()),
                );
                Ok(self
                    .builder
                    .build_call(
                        function,
                        &[array_slot.into(), separator.into()],
                        "array_join",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap())
            }
            _ => Err(format!(
                "internal: `{method}` is not an array derivations method"
            )),
        }
    }
}
