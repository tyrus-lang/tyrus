use super::*;

impl<'ctx> Codegen<'ctx> {
    /// The methods that search the array and answer without changing it.
    pub(super) fn emit_array_search_method(
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
            "equals" => {
                let other = self.emit_expr_as(&args[0], &Type::Array(Box::new(element.clone())))?;
                let other_slot = self.entry_alloca(llvm_array_type, "array_equals_other")?;
                self.builder
                    .build_store(other_slot, other)
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_array_equals",
                    &[
                        pointer.into(),
                        pointer.into(),
                        i64_type.into(),
                        i32_type.into(),
                    ],
                    Some(self.context.bool_type().into()),
                );
                Ok(self
                    .builder
                    .build_call(
                        function,
                        &[
                            array_slot.into(),
                            other_slot.into(),
                            element_size.into(),
                            i32_type
                                .const_int(self.array_element_kind(element), false)
                                .into(),
                        ],
                        "array_equals",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap())
            }
            "includes" | "indexOf" | "lastIndexOf" => {
                let value = self.array_method_value_ptr(&args[0], element)?;
                let function = self.declare_libc_fn(
                    "tyrus_array_index_of",
                    &[
                        pointer.into(),
                        pointer.into(),
                        i64_type.into(),
                        i32_type.into(),
                        self.context.bool_type().into(),
                    ],
                    Some(i64_type.into()),
                );
                let index = self
                    .builder
                    .build_call(
                        function,
                        &[
                            array_slot.into(),
                            value.into(),
                            element_size.into(),
                            i32_type
                                .const_int(self.array_element_kind(element), false)
                                .into(),
                            self.context
                                .bool_type()
                                .const_int((method == "lastIndexOf") as u64, false)
                                .into(),
                        ],
                        "array_index_of",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                if method == "includes" {
                    Ok(self
                        .builder
                        .build_int_compare(
                            IntPredicate::NE,
                            index,
                            i64_type.const_int(u64::MAX, true),
                            "array_includes",
                        )
                        .unwrap()
                        .into())
                } else {
                    Ok(index.into())
                }
            }
            _ => Err(format!(
                "internal: `{method}` is not an array searches method"
            )),
        }
    }
}
