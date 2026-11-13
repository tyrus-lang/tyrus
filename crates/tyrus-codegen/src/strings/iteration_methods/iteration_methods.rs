use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_string_iteration_method(
        &mut self,
        method: &str,
        args: &[Expr],
        s: inkwell::values::StructValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let str_ty = self.string_abi_type();
        let i64_type = self.context.i64_type();
        match method {
            "matchAll" => {
                let pattern = self.emit_expr(&args[0])?.into_struct_value();
                let outer_array_type =
                    self.llvm_type(&Type::Array(Box::new(Type::Array(Box::new(Type::Str)))));
                let out_slot = self
                    .entry_alloca(outer_array_type, "match_all_out")
                    .expect("a stack slot needs an enclosing function");
                let function = self.declare_libc_fn(
                    "tyrus_regex_exec_all",
                    &[str_ty, str_ty, ptr_type.into()],
                    Some(i64_type.into()),
                );
                self.builder
                    .build_call(
                        function,
                        &[pattern.into(), s.into(), out_slot.into()],
                        "match_all",
                    )
                    .unwrap();
                let result = self
                    .builder
                    .build_load(outer_array_type, out_slot, "match_all_result")
                    .unwrap();
                Ok(result)
            }

            "chars" => {
                let array_type = self.llvm_type(&Type::Array(Box::new(Type::Char)));
                let out_slot = self
                    .entry_alloca(array_type, "chars_out")
                    .expect("a stack slot needs an enclosing function");
                let function =
                    self.declare_libc_fn("tyrus_string_chars", &[str_ty, ptr_type.into()], None);
                self.builder
                    .build_call(function, &[s.into(), out_slot.into()], "chars")
                    .unwrap();
                let result = self
                    .builder
                    .build_load(array_type, out_slot, "chars_result")
                    .unwrap();
                Ok(result)
            }

            "graphemes" => {
                let array_type = self.llvm_type(&Type::Array(Box::new(Type::Str)));
                let out_slot = self
                    .entry_alloca(array_type, "graphemes_out")
                    .expect("a stack slot needs an enclosing function");
                let function = self.declare_libc_fn(
                    "tyrus_string_graphemes",
                    &[str_ty, ptr_type.into()],
                    None,
                );
                self.builder
                    .build_call(function, &[s.into(), out_slot.into()], "graphemes")
                    .unwrap();
                let result = self
                    .builder
                    .build_load(array_type, out_slot, "graphemes_result")
                    .unwrap();
                Ok(result)
            }

            "split" => {
                let array_type = self.llvm_type(&Type::Array(Box::new(Type::Str)));
                let out_slot = self
                    .entry_alloca(array_type, "split_out")
                    .expect("a stack slot needs an enclosing function");
                let is_regex = matches!(self.expr_type_hint(&args[0]), Some(Type::Regex));
                if is_regex {
                    let pattern = self.emit_expr(&args[0])?.into_struct_value();
                    let function = self.declare_libc_fn(
                        "tyrus_regex_split",
                        &[str_ty, str_ty, ptr_type.into()],
                        None,
                    );
                    self.builder
                        .build_call(
                            function,
                            &[pattern.into(), s.into(), out_slot.into()],
                            "regex_split",
                        )
                        .unwrap();
                } else {
                    let separator = self.emit_expr(&args[0])?.into_struct_value();
                    let function = self.declare_libc_fn(
                        "tyrus_string_split",
                        &[str_ty, str_ty, ptr_type.into()],
                        None,
                    );
                    self.builder
                        .build_call(
                            function,
                            &[s.into(), separator.into(), out_slot.into()],
                            "split",
                        )
                        .unwrap();
                }
                let result = self
                    .builder
                    .build_load(array_type, out_slot, "split_result")
                    .unwrap();
                Ok(result)
            }

            "toSlugCase" | "toDashCase" | "toKebabCase" | "toPascalCase" | "toCamelCase"
            | "toSnakeCase" | "stripHtml" | "escapeHtml" | "urlEncode" | "urlDecode" => {
                let runtime_name = match method {
                    "toSlugCase" => "tyrus_string_to_slug_case",
                    "toDashCase" | "toKebabCase" => "tyrus_string_to_dash_case",
                    "toPascalCase" => "tyrus_string_to_pascal_case",
                    "toCamelCase" => "tyrus_string_to_camel_case",
                    "toSnakeCase" => "tyrus_string_to_snake_case",
                    "stripHtml" => "tyrus_string_strip_html",
                    "escapeHtml" => "tyrus_string_escape_html",
                    "urlEncode" => "tyrus_string_url_encode",
                    "urlDecode" => "tyrus_string_url_decode",
                    _ => unreachable!(),
                };
                let function = self.declare_libc_fn(runtime_name, &[str_ty], Some(str_ty));
                let result = self
                    .builder
                    .build_call(function, &[s.into()], method)
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_struct_value();
                Ok(result.into())
            }

            "truncate" | "truncateEllipsis" => {
                let max_length = self.emit_expr(&args[0])?.into_int_value();
                let max_length =
                    self.widen_string_index(max_length, &args[0], "truncate_max_length");
                let runtime_name = if method == "truncate" {
                    "tyrus_string_truncate"
                } else {
                    "tyrus_string_truncate_ellipsis"
                };
                let function =
                    self.declare_libc_fn(runtime_name, &[str_ty, i64_type.into()], Some(str_ty));
                let result = self
                    .builder
                    .build_call(function, &[s.into(), max_length.into()], method)
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_struct_value();
                Ok(result.into())
            }

            _ => Err(format!(
                "internal: `{method}` is not handled by `emit_string_iteration_method`"
            )),
        }
    }
}
