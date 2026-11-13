use super::*;

impl<'ctx> Codegen<'ctx> {
    /// The methods that rewrite the array in place without changing its length.
    pub(super) fn emit_array_rewrite_method(
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
            owns_string_elements,
            ..
        } = receiver;
        match method {
            "reverse" | "fill" | "copyWithin" => {
                let symbol = format!(
                    "tyrus_array_{}",
                    if method == "copyWithin" {
                        "copy_within"
                    } else {
                        method
                    }
                );
                let result = if method == "reverse" {
                    let function = self.declare_libc_fn(
                        &symbol,
                        &[pointer.into(), i64_type.into()],
                        Some(i32_type.into()),
                    );
                    self.builder.build_call(
                        function,
                        &[array_slot.into(), element_size.into()],
                        "array_mutation",
                    )
                } else if method == "fill" {
                    let owned_string = owns_string_elements;
                    let filled = if owned_string {
                        Some(self.emit_expr_as(&args[0], element)?.into_struct_value())
                    } else {
                        None
                    };
                    let value: BasicMetadataValueEnum<'ctx> = match filled {
                        Some(value) => value.into(),
                        None => self.array_method_value_ptr(&args[0], element)?.into(),
                    };
                    let length = self.array_length_from_slot(array_slot, llvm_array_type);
                    let start = if args.len() > 1 {
                        self.emit_expr_as(&args[1], &Type::I64)?.into_int_value()
                    } else {
                        i64_type.const_zero()
                    };
                    let end = if args.len() > 2 {
                        self.emit_expr_as(&args[2], &Type::I64)?.into_int_value()
                    } else {
                        length
                    };
                    let function = if owned_string {
                        self.declare_libc_fn(
                            "tyrus_array_fill_owned_string",
                            &[
                                pointer.into(),
                                self.string_abi_type(),
                                i64_type.into(),
                                i64_type.into(),
                            ],
                            Some(i32_type.into()),
                        )
                    } else {
                        self.declare_libc_fn(
                            &symbol,
                            &[
                                pointer.into(),
                                pointer.into(),
                                i64_type.into(),
                                i64_type.into(),
                                i64_type.into(),
                            ],
                            Some(i32_type.into()),
                        )
                    };
                    let mut arguments = vec![array_slot.into(), value, start.into(), end.into()];
                    if !owned_string {
                        arguments.push(element_size.into());
                    }
                    let result = self
                        .builder
                        .build_call(function, &arguments, "array_mutation");
                    if let Some(filled) = filled {
                        self.emit_async_cleanup_source_expression(
                            filled.into(),
                            element,
                            &args[0],
                        )?;
                    }
                    result
                } else {
                    let length = self.array_length_from_slot(array_slot, llvm_array_type);
                    let target = self.emit_expr_as(&args[0], &Type::I64)?.into_int_value();
                    let start = self.emit_expr_as(&args[1], &Type::I64)?.into_int_value();
                    let end = if args.len() > 2 {
                        self.emit_expr_as(&args[2], &Type::I64)?.into_int_value()
                    } else {
                        length
                    };
                    let owned_string = owns_string_elements;
                    let function = if owned_string {
                        self.declare_libc_fn(
                            "tyrus_array_copy_within_owned_strings",
                            &[
                                pointer.into(),
                                i64_type.into(),
                                i64_type.into(),
                                i64_type.into(),
                            ],
                            Some(i32_type.into()),
                        )
                    } else {
                        self.declare_libc_fn(
                            &symbol,
                            &[
                                pointer.into(),
                                i64_type.into(),
                                i64_type.into(),
                                i64_type.into(),
                                i64_type.into(),
                            ],
                            Some(i32_type.into()),
                        )
                    };
                    let mut arguments =
                        vec![array_slot.into(), target.into(), start.into(), end.into()];
                    if !owned_string {
                        arguments.push(element_size.into());
                    }
                    self.builder
                        .build_call(function, &arguments, "array_mutation")
                };
                let result = result
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                self.emit_abort_if_negative(result, method, "array.method")?;
                self.builder
                    .build_load(llvm_array_type, array_slot, "array_result")
                    .map_err(|error| error.to_string())
            }
            "rotate" => self.emit_array_rotate(array_slot, llvm_array_type, element, &args[0]),
            _ => Err(format!(
                "internal: `{method}` is not an array rewrites method"
            )),
        }
    }
}
