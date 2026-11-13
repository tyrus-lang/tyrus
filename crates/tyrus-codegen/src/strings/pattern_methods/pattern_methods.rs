use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_string_pattern_method(
        &mut self,
        method: &str,
        args: &[Expr],
        s: inkwell::values::StructValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let str_ty = self.string_abi_type();
        let i64_type = self.context.i64_type();
        let i32_type = self.context.i32_type();
        match method {
            "match" => {
                let pattern = self.emit_expr(&args[0])?.into_struct_value();
                let array_type = self.llvm_type(&Type::Array(Box::new(Type::Str)));
                let out_slot = self
                    .entry_alloca(array_type, "regex_match_out")
                    .expect("a stack slot needs an enclosing function");
                let function = self.declare_libc_fn(
                    "tyrus_regex_exec",
                    &[str_ty, str_ty, ptr_type.into()],
                    Some(i64_type.into()),
                );
                let matched = self
                    .builder
                    .build_call(
                        function,
                        &[pattern.into(), s.into(), out_slot.into()],
                        "regex_match",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                let fn_val = self
                    .builder
                    .get_insert_block()
                    .and_then(inkwell::basic_block::BasicBlock::get_parent)
                    .ok_or("internal: regex.match outside function")?;
                let present = self
                    .context
                    .append_basic_block(fn_val, "regex_match_present");
                let absent = self
                    .context
                    .append_basic_block(fn_val, "regex_match_absent");
                let merge = self.context.append_basic_block(fn_val, "regex_match_merge");
                let is_zero = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        matched,
                        i64_type.const_int(0, false),
                        "regex_match_matched",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(is_zero, absent, present)
                    .unwrap();
                self.builder.position_at_end(absent);
                let null_ptr = ptr_type.const_null();
                self.builder.build_unconditional_branch(merge).unwrap();
                let absent_block = self.builder.get_insert_block().unwrap();
                self.builder.position_at_end(present);
                let array_val = self
                    .builder
                    .build_load(array_type, out_slot, "regex_match_array")
                    .unwrap();
                let size = array_type
                    .size_of()
                    .ok_or("error: regex match array has no native size")?;
                let (malloc, size) = self.declare_malloc(size);
                let storage = self
                    .builder
                    .build_call(malloc, &[size.into()], "regex_match_storage")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_pointer_value();
                self.builder.build_store(storage, array_val).unwrap();
                self.builder.build_unconditional_branch(merge).unwrap();
                let present_block = self.builder.get_insert_block().unwrap();
                self.builder.position_at_end(merge);
                let phi = self
                    .builder
                    .build_phi(ptr_type, "regex_match_result")
                    .unwrap();
                phi.add_incoming(&[
                    (&null_ptr, absent_block),
                    (&(storage as PointerValue<'ctx>), present_block),
                ]);
                self.record_owned_native_result_with(
                    phi.as_basic_value(),
                    Type::Optional(Box::new(Type::Array(Box::new(Type::Str)))),
                    crate::ReleasePlan::Box(Box::new(crate::ReleasePlan::Deep)),
                );
                Ok(phi.as_basic_value())
            }

            "search" => {
                let pattern = self.emit_expr(&args[0])?.into_struct_value();
                let function = self.declare_libc_fn(
                    "tyrus_string_search_regex",
                    &[str_ty, str_ty],
                    Some(i32_type.into()),
                );
                let result = self
                    .builder
                    .build_call(function, &[s.into(), pattern.into()], "search")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                Ok(result.into())
            }

            "replace" => {
                let is_regex = matches!(self.expr_type_hint(&args[0]), Some(Type::Regex));
                if is_regex {
                    let pattern = self.emit_expr(&args[0])?.into_struct_value();
                    let replacement = self.emit_expr(&args[1])?.into_struct_value();
                    let function = self.declare_libc_fn(
                        "tyrus_regex_replace",
                        &[str_ty, str_ty, str_ty],
                        Some(str_ty),
                    );
                    let result = self
                        .builder
                        .build_call(
                            function,
                            &[pattern.into(), s.into(), replacement.into()],
                            "regex_replace",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_struct_value();
                    Ok(result.into())
                } else {
                    let search = self.emit_expr(&args[0])?.into_struct_value();
                    let replacement = self.emit_expr(&args[1])?.into_struct_value();
                    let function = self.declare_libc_fn(
                        "tyrus_string_replace",
                        &[str_ty, str_ty, str_ty],
                        Some(str_ty),
                    );
                    let result = self
                        .builder
                        .build_call(
                            function,
                            &[s.into(), search.into(), replacement.into()],
                            "replace",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_struct_value();
                    Ok(result.into())
                }
            }

            _ => Err(format!(
                "internal: `{method}` is not handled by `emit_string_pattern_method`"
            )),
        }
    }
}
