use super::*;

#[path = "iteration_methods/iteration_methods.rs"]
mod iteration_methods;
#[path = "pattern_methods/pattern_methods.rs"]
mod pattern_methods;

impl<'ctx> Codegen<'ctx> {
    fn widen_string_index(
        &mut self,
        value: IntValue<'ctx>,
        argument: &Expr,
        name: &str,
    ) -> IntValue<'ctx> {
        let i64_type = self.context.i64_type();
        if value.get_type() == i64_type {
            return value;
        }
        let unsigned = self
            .expr_type_hint(argument)
            .map(|ty| self.resolve_type(&ty))
            .is_some_and(|ty| is_unsigned_int_type(&ty));
        let widened = if unsigned {
            self.builder.build_int_z_extend(value, i64_type, name)
        } else {
            self.builder.build_int_s_extend(value, i64_type, name)
        };
        widened.unwrap()
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_string_method(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // How a `string` is spelled in a native signature. Still a bare pointer
        // today (see `string_abi_type`) so marking a site changes nothing yet
        // and mistaking an out-parameter for a string shows up as a failing test
        // now rather than as a corrupt argument after the flip.
        let str_ty = self.string_abi_type();
        let i64_type = self.context.i64_type();
        let i32_type = self.context.i32_type();

        let s = match self
            .expr_type_hint(object)
            .and_then(|ty| self.payload_free_union_underlying(&ty))
        {
            Some((variants, _)) => {
                let union_value = self.emit_expr(object)?;
                self.emit_union_underlying_value(union_value, &variants, "union.text")?
                    .into_struct_value()
            }
            None => self.emit_expr(object)?.into_struct_value(),
        };

        match method {
            "charAt" => {
                let index = self.emit_expr(&args[0])?.into_int_value();
                let index = self.widen_string_index(index, &args[0], "char_at_index");
                let function = self.declare_libc_fn(
                    "tyrus_string_char_at",
                    &[str_ty, i64_type.into()],
                    Some(str_ty),
                );
                let result = self
                    .builder
                    .build_call(function, &[s.into(), index.into()], "char_at")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_struct_value();
                Ok(result.into())
            }

            "charCodeAt" => {
                let index = self.emit_expr(&args[0])?.into_int_value();
                let index = self.widen_string_index(index, &args[0], "char_code_at_index");
                let function = self.declare_libc_fn(
                    "tyrus_string_char_code_at",
                    &[str_ty, i64_type.into()],
                    Some(i32_type.into()),
                );
                let result = self
                    .builder
                    .build_call(function, &[s.into(), index.into()], "char_code_at")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                Ok(result.into())
            }

            "count" => {
                if let Some(sub) = args.first() {
                    let sub = self.emit_expr(sub)?.into_struct_value();
                    let function = self.declare_libc_fn(
                        "tyrus_string_count_occurrences",
                        &[str_ty, str_ty],
                        Some(i64_type.into()),
                    );
                    let result = self
                        .builder
                        .build_call(function, &[s.into(), sub.into()], "string_count")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    Ok(result)
                } else {
                    Ok(self.string_char_len(s)?.into())
                }
            }

            "includes" | "startsWith" | "endsWith" => {
                let search = self.emit_expr(&args[0])?.into_struct_value();
                let runtime_name = match method {
                    "includes" => "tyrus_string_includes",
                    "startsWith" => "tyrus_string_starts_with",
                    "endsWith" => "tyrus_string_ends_with",
                    _ => unreachable!(),
                };
                let function =
                    self.declare_libc_fn(runtime_name, &[str_ty, str_ty], Some(i64_type.into()));
                let result = self
                    .builder
                    .build_call(function, &[s.into(), search.into()], method)
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                let is_true = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        result,
                        i64_type.const_int(0, false),
                        method,
                    )
                    .unwrap();
                Ok(is_true.into())
            }

            "indexOf" => {
                let search = self.emit_expr(&args[0])?.into_struct_value();
                let from_index = if args.len() > 1 {
                    let idx = self.emit_expr(&args[1])?.into_int_value();
                    self.widen_string_index(idx, &args[1], "from_index")
                } else {
                    i64_type.const_int(0, false)
                };
                let function = self.declare_libc_fn(
                    "tyrus_string_index_of",
                    &[str_ty, str_ty, i64_type.into()],
                    Some(i64_type.into()),
                );
                let result = self
                    .builder
                    .build_call(
                        function,
                        &[s.into(), search.into(), from_index.into()],
                        "index_of",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                Ok(result.into())
            }

            "lastIndexOf" => {
                let search = self.emit_expr(&args[0])?.into_struct_value();
                let from_index = if args.len() > 1 {
                    let idx = self.emit_expr(&args[1])?.into_int_value();
                    self.widen_string_index(idx, &args[1], "from_index")
                } else {
                    i64_type.const_int(i64::MAX as u64, false)
                };
                let function = self.declare_libc_fn(
                    "tyrus_string_last_index_of",
                    &[str_ty, str_ty, i64_type.into()],
                    Some(i64_type.into()),
                );
                let result = self
                    .builder
                    .build_call(
                        function,
                        &[s.into(), search.into(), from_index.into()],
                        "last_index_of",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                Ok(result.into())
            }

            "slice" | "substring" => {
                let start = self.emit_expr(&args[0])?.into_int_value();
                let start = self.widen_string_index(start, &args[0], "slice_start");
                let end = if args.len() > 1 {
                    let e = self.emit_expr(&args[1])?.into_int_value();
                    self.widen_string_index(e, &args[1], "slice_end")
                } else {
                    i64_type.const_int(i64::MAX as u64, false)
                };
                let runtime_name = if method == "slice" {
                    "tyrus_string_slice"
                } else {
                    "tyrus_string_substring"
                };
                let function = self.declare_libc_fn(
                    runtime_name,
                    &[str_ty, i64_type.into(), i64_type.into()],
                    Some(str_ty),
                );
                let result = self
                    .builder
                    .build_call(function, &[s.into(), start.into(), end.into()], method)
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_struct_value();
                Ok(result.into())
            }

            "trim" | "trimStart" | "trimEnd" => {
                let runtime_name = match method {
                    "trim" => "tyrus_string_trim",
                    "trimStart" => "tyrus_string_trim_start",
                    "trimEnd" => "tyrus_string_trim_end",
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

            "toLowerCase" | "toUpperCase" => {
                let runtime_name = if method == "toLowerCase" {
                    "tyrus_string_to_lower"
                } else {
                    "tyrus_string_to_upper"
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

            "match" | "search" | "replace" => self.emit_string_pattern_method(method, args, s),
            "repeat" => {
                let count = self.emit_expr(&args[0])?.into_int_value();
                let count = self.widen_string_index(count, &args[0], "repeat_count");
                let function = self.declare_libc_fn(
                    "tyrus_string_repeat",
                    &[str_ty, i64_type.into()],
                    Some(str_ty),
                );
                let result = self
                    .builder
                    .build_call(function, &[s.into(), count.into()], "repeat")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_struct_value();
                Ok(result.into())
            }

            "padStart" | "padEnd" => {
                let target_length = self.emit_expr(&args[0])?.into_int_value();
                let target_length =
                    self.widen_string_index(target_length, &args[0], "pad_target_length");
                let pad_string = if args.len() > 1 {
                    self.emit_expr(&args[1])?.into_struct_value()
                } else {
                    self.const_string_abi(" ", "pad_default")
                        .into_struct_value()
                };
                let runtime_name = if method == "padStart" {
                    "tyrus_string_pad_start"
                } else {
                    "tyrus_string_pad_end"
                };
                let function = self.declare_libc_fn(
                    runtime_name,
                    &[str_ty, i64_type.into(), str_ty],
                    Some(str_ty),
                );
                let result = self
                    .builder
                    .build_call(
                        function,
                        &[s.into(), target_length.into(), pad_string.into()],
                        method,
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_struct_value();
                Ok(result.into())
            }

            "concat" => {
                let mut result = s;
                let mut owns_result = false;
                for arg in args {
                    let other = self.emit_expr(arg)?.into_struct_value();
                    let function = self.declare_libc_fn(
                        "tyrus_string_concat",
                        &[str_ty, str_ty],
                        Some(str_ty),
                    );
                    let call_result = self
                        .builder
                        .build_call(function, &[result.into(), other.into()], "concat")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_struct_value();
                    if owns_result {
                        self.emit_string_release(result)?;
                    }
                    result = call_result;
                    owns_result = true;
                }
                Ok(result.into())
            }

            "matchAll" | "chars" | "graphemes" | "split" | "toSlugCase" | "toDashCase"
            | "toKebabCase" | "toPascalCase" | "toCamelCase" | "toSnakeCase" | "stripHtml"
            | "escapeHtml" | "urlEncode" | "urlDecode" | "truncate" | "truncateEllipsis" => {
                self.emit_string_iteration_method(method, args, s)
            }
            other => Err(format!("error: unknown string method `{other}`")),
        }
    }
}
