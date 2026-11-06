use super::*;

impl<'ctx> Codegen<'ctx> {
    /// The methods that change the length of the array.
    pub(super) fn emit_array_mutation_method(
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
            appends_owned_string_elements,
            ..
        } = receiver;
        match method {
            "push" | "unshift" => {
                let owns_string_elements = appends_owned_string_elements;
                let symbol = if owns_string_elements {
                    format!("tyrus_array_{method}_owned_string")
                } else {
                    format!("tyrus_array_{method}")
                };
                let params = if owns_string_elements {
                    vec![pointer.into(), self.string_abi_type()]
                } else {
                    vec![pointer.into(), pointer.into(), i64_type.into()]
                };
                let function = self.declare_libc_fn(&symbol, &params, Some(i64_type.into()));
                let mut result = self.array_length_from_slot(array_slot, llvm_array_type);
                let values: Box<dyn Iterator<Item = &Expr>> = if method == "unshift" {
                    Box::new(args.iter().rev())
                } else {
                    Box::new(args.iter())
                };
                for source_expression in values {
                    let appended = if owns_string_elements {
                        Some(
                            self.emit_expr_as(source_expression, element)?
                                .into_struct_value(),
                        )
                    } else {
                        None
                    };
                    let arguments = match appended {
                        Some(value) => vec![array_slot.into(), value.into()],
                        None => vec![
                            array_slot.into(),
                            self.array_method_value_ptr(source_expression, element)?
                                .into(),
                            element_size.into(),
                        ],
                    };
                    result = self
                        .builder
                        .build_call(function, &arguments, "array_length")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_int_value();
                    self.emit_abort_if_negative(result, method, "array.method")?;
                    if let Some(value) = appended {
                        self.emit_async_cleanup_source_expression(
                            value.into(),
                            element,
                            source_expression,
                        )?;
                    }
                }
                Ok(result.into())
            }
            "pop" | "shift" => {
                let function = self.declare_libc_fn(
                    &format!("tyrus_array_{method}"),
                    &[pointer.into(), i64_type.into()],
                    Some(pointer.into()),
                );
                let removed = self
                    .builder
                    .build_call(
                        function,
                        &[array_slot.into(), element_size.into()],
                        "array_removed_element",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| format!("internal: array.{method} returned void"))?;
                Ok(removed)
            }
            "splice" => {
                let start = self.emit_expr_as(&args[0], &Type::I64)?.into_int_value();
                let length = self.array_length_from_slot(array_slot, llvm_array_type);
                let delete_count = if args.len() > 1 {
                    self.emit_expr_as(&args[1], &Type::I64)?.into_int_value()
                } else {
                    length
                };
                let inserted_value = self.emit_array_literal(&args[2..], element)?;
                let inserted_slot = self
                    .entry_alloca(llvm_array_type, "array_splice_inserted")
                    .unwrap();
                self.builder
                    .build_store(inserted_slot, inserted_value)
                    .unwrap();
                let inserted_data = self
                    .builder
                    .build_extract_value(
                        inserted_value.into_struct_value(),
                        0,
                        "array_splice_inserted_data",
                    )
                    .unwrap()
                    .into_pointer_value();
                let output = self
                    .entry_alloca(llvm_array_type, "array_splice_removed")
                    .unwrap();
                let owned_string = owns_string_elements;
                let function = if owned_string {
                    self.declare_libc_fn(
                        "tyrus_array_splice_owned_strings",
                        &[
                            pointer.into(),
                            i64_type.into(),
                            i64_type.into(),
                            pointer.into(),
                            i64_type.into(),
                            pointer.into(),
                        ],
                        Some(i32_type.into()),
                    )
                } else {
                    self.declare_libc_fn(
                        "tyrus_array_splice",
                        &[
                            pointer.into(),
                            i64_type.into(),
                            i64_type.into(),
                            pointer.into(),
                            i64_type.into(),
                            i64_type.into(),
                            pointer.into(),
                        ],
                        Some(i32_type.into()),
                    )
                };
                let mut arguments = vec![
                    array_slot.into(),
                    start.into(),
                    delete_count.into(),
                    inserted_data.into(),
                    i64_type
                        .const_int(args.len().saturating_sub(2) as u64, false)
                        .into(),
                ];
                if !owned_string {
                    arguments.push(element_size.into());
                }
                arguments.push(output.into());
                let status = self
                    .builder
                    .build_call(function, &arguments, "array_splice")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                self.emit_abort_if_negative(status, "splice", "array.splice")?;
                self.builder
                    .build_load(llvm_array_type, output, "array_splice_result")
                    .map_err(|error| error.to_string())
            }
            _ => Err(format!(
                "internal: `{method}` is not an array mutations method"
            )),
        }
    }
}
