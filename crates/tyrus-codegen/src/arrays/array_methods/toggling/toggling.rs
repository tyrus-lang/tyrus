use super::*;

impl<'ctx> Codegen<'ctx> {
    /// The two toggling methods, which add an element or remove it depending on whether the array already holds it.
    pub(super) fn emit_array_toggle_method(
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
            "toggle" => {
                let output = self
                    .entry_alloca(llvm_array_type, "array_toggle_out")
                    .unwrap();
                let item_value = self.array_method_value_ptr(&args[0], element)?;
                let has_key = args.len() > 1;
                let (key_offset, key_size) = if has_key {
                    match args[1].kind() {
                        ExprKind::StrLit(key) => {
                            let field_index =
                                self.struct_field_index(element, key).ok_or_else(|| {
                                    format!("error: `toggle` key `{key}` is not a field")
                                })?;
                            let field_type = self
                                .struct_field_type(element, key)
                                .expect("field index and field type must resolve together");
                            let llvm_field_type = self.llvm_type(&field_type);
                            let element_llvm_type = self.llvm_type(element);
                            let struct_type = match element_llvm_type {
                                BasicTypeEnum::StructType(s) => s,
                                _ => {
                                    return Err(
                                        "error: `toggle` key requires a struct element type"
                                            .to_string(),
                                    );
                                }
                            };
                            let offset_ptr = self
                                .builder
                                .build_struct_gep(
                                    struct_type,
                                    self.context
                                        .ptr_type(inkwell::AddressSpace::default())
                                        .const_null(),
                                    field_index,
                                    "toggle_field_offset_ptr",
                                )
                                .map_err(|e| e.to_string())?;
                            let offset = self
                                .builder
                                .build_ptr_to_int(offset_ptr, i64_type, "toggle_field_offset")
                                .map_err(|e| e.to_string())?;
                            let size = llvm_field_type.size_of().ok_or_else(|| {
                                format!("error: `toggle` key field `{key}` has no fixed size")
                            })?;
                            (offset, size)
                        }
                        _ => unreachable!("type checker already validated key is a string literal"),
                    }
                } else {
                    (i64_type.const_zero(), i64_type.const_zero())
                };
                let element_kind = self.array_element_kind(element);
                let function = self.declare_libc_fn(
                    "tyrus_array_toggle",
                    &[
                        pointer.into(),
                        pointer.into(),
                        i64_type.into(),
                        i32_type.into(),
                        i64_type.into(),
                        i64_type.into(),
                        pointer.into(),
                    ],
                    Some(i32_type.into()),
                );
                let result = self
                    .builder
                    .build_call(
                        function,
                        &[
                            array_slot.into(),
                            item_value.into(),
                            element_size.into(),
                            i32_type.const_int(element_kind, false).into(),
                            key_offset.into(),
                            key_size.into(),
                            output.into(),
                        ],
                        "array_toggle",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                self.emit_abort_if_negative(result, method, "array.method")?;
                self.builder
                    .build_load(llvm_array_type, output, "array_toggle_result")
                    .map_err(|error| error.to_string())
            }
            "toggleAll" => {
                let output = self
                    .entry_alloca(llvm_array_type, "array_toggle_all_out")
                    .unwrap();
                let items_value = self.emit_expr_as(&args[0], array_type)?;
                let items_alloca = self
                    .entry_alloca(llvm_array_type, "array_toggle_all_items")
                    .unwrap();
                self.builder.build_store(items_alloca, items_value).unwrap();
                let element_kind = self.array_element_kind(element);
                let function = self.declare_libc_fn(
                    "tyrus_array_toggle_all",
                    &[
                        pointer.into(),
                        pointer.into(),
                        i64_type.into(),
                        i32_type.into(),
                        pointer.into(),
                    ],
                    Some(i32_type.into()),
                );
                let result = self
                    .builder
                    .build_call(
                        function,
                        &[
                            array_slot.into(),
                            items_alloca.into(),
                            element_size.into(),
                            i32_type.const_int(element_kind, false).into(),
                            output.into(),
                        ],
                        "array_toggle_all",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                self.emit_abort_if_negative(result, method, "array.method")?;
                self.emit_async_cleanup_source_expression(items_value, array_type, &args[0])?;
                self.builder
                    .build_load(llvm_array_type, output, "array_toggle_all_result")
                    .map_err(|error| error.to_string())
            }
            _ => Err(format!(
                "internal: `{method}` is not an array toggling method"
            )),
        }
    }
}
