use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(crate) fn emit_string_search_call(
        &mut self,
        target: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let expected = if target == tyrus_core::algorithm::Z_ALGORITHM_TARGET {
            1
        } else {
            2
        };
        if args.len() != expected {
            return Err("error: string algorithm expects 2 arguments".into());
        }
        let string_pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let _strlen = self.declare_libc_fn(
            "strlen",
            &[string_pointer.into()],
            Some(self.usize_type().into()),
        );
        let text_value = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
        let text = self.string_data_ptr(text_value)?;
        let text_length = self.string_byte_len(text_value)?;
        let output_ty = if target == tyrus_core::algorithm::AHO_CORASICK_TARGET {
            Type::Array(Box::new(Type::Tuple(vec![Type::Usize, Type::Usize])))
        } else {
            Type::Array(Box::new(Type::Usize))
        };
        let output = self
            .builder
            .build_alloca(self.llvm_type(&output_ty), "string_search.output")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(output, self.llvm_type(&output_ty).const_zero())
            .map_err(|e| e.to_string())?;
        let (runtime_name, values): (&str, Vec<inkwell::values::BasicMetadataValueEnum<'ctx>>) =
            match target {
                tyrus_core::algorithm::KMP_SEARCH_TARGET
                | tyrus_core::algorithm::RABIN_KARP_TARGET
                | tyrus_core::algorithm::BOYER_MOORE_TARGET => {
                    let pattern_value =
                        self.emit_expr_as(&args[1], &Type::Str)?.into_struct_value();
                    let pattern = self.string_data_ptr(pattern_value)?;
                    let pattern_length = self.string_byte_len(pattern_value)?;
                    let name = match target {
                        tyrus_core::algorithm::KMP_SEARCH_TARGET => "tyrus_algorithm_kmp_search",
                        tyrus_core::algorithm::RABIN_KARP_TARGET => "tyrus_algorithm_rabin_karp",
                        _ => "tyrus_algorithm_boyer_moore",
                    };
                    (
                        name,
                        vec![
                            text.into(),
                            text_length.into(),
                            pattern.into(),
                            pattern_length.into(),
                            output.into(),
                        ],
                    )
                }
                tyrus_core::algorithm::Z_ALGORITHM_TARGET => (
                    "tyrus_algorithm_z_algorithm",
                    vec![text.into(), text_length.into(), output.into()],
                ),
                tyrus_core::algorithm::AHO_CORASICK_TARGET => {
                    let patterns_ty = Type::Array(Box::new(Type::Str));
                    let patterns = self.emit_expr_as(&args[1], &patterns_ty)?;
                    let patterns_ptr = self
                        .builder
                        .build_alloca(self.llvm_type(&patterns_ty), "string_search.patterns")
                        .map_err(|e| e.to_string())?;
                    self.builder
                        .build_store(patterns_ptr, patterns)
                        .map_err(|e| e.to_string())?;
                    (
                        "tyrus_algorithm_aho_corasick",
                        vec![
                            text.into(),
                            text_length.into(),
                            patterns_ptr.into(),
                            output.into(),
                        ],
                    )
                }
                tyrus_core::algorithm::LEVENSHTEIN_TARGET => {
                    let other_value = self.emit_expr_as(&args[1], &Type::Str)?.into_struct_value();
                    let other = self.string_data_ptr(other_value)?;
                    let other_length = self.string_byte_len(other_value)?;
                    let value_output = self
                        .builder
                        .build_alloca(self.usize_type(), "levenshtein.output")
                        .map_err(|e| e.to_string())?;
                    let function = self.declare_libc_fn(
                        "tyrus_algorithm_levenshtein",
                        &[
                            string_pointer.into(),
                            self.usize_type().into(),
                            string_pointer.into(),
                            self.usize_type().into(),
                            string_pointer.into(),
                        ],
                        Some(self.context.i32_type().into()),
                    );
                    self.builder
                        .build_call(
                            function,
                            &[
                                text.into(),
                                text_length.into(),
                                other.into(),
                                other_length.into(),
                                value_output.into(),
                            ],
                            "levenshtein.call",
                        )
                        .map_err(|e| e.to_string())?;
                    return self
                        .builder
                        .build_load(self.usize_type(), value_output, "levenshtein.result")
                        .map_err(|e| e.to_string());
                }
                _ => return Err("error: unknown string algorithm".into()),
            };
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let params = if target == tyrus_core::algorithm::Z_ALGORITHM_TARGET {
            vec![pointer.into(), self.usize_type().into(), pointer.into()]
        } else if target == tyrus_core::algorithm::AHO_CORASICK_TARGET {
            vec![
                pointer.into(),
                self.usize_type().into(),
                pointer.into(),
                pointer.into(),
            ]
        } else {
            vec![
                pointer.into(),
                self.usize_type().into(),
                pointer.into(),
                if target == tyrus_core::algorithm::AHO_CORASICK_TARGET {
                    pointer.into()
                } else {
                    self.usize_type().into()
                },
                pointer.into(),
            ]
        };
        let function = self.declare_libc_fn(runtime_name, &params, None);
        self.builder
            .build_call(function, &values, "string_search.call")
            .map_err(|e| e.to_string())?;
        let result = self
            .builder
            .build_load(self.llvm_type(&output_ty), output, "string_search.result")
            .map_err(|e| e.to_string())?;
        self.record_owned_native_result(result, output_ty);
        Ok(result)
    }

    pub(crate) fn emit_kmp_search_info_call(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let _strlen =
            self.declare_libc_fn("strlen", &[pointer.into()], Some(self.usize_type().into()));
        let text_value = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
        let text = self.string_data_ptr(text_value)?;
        let pattern_value = self.emit_expr_as(&args[1], &Type::Str)?.into_struct_value();
        let pattern = self.string_data_ptr(pattern_value)?;
        let text_len = self.string_byte_len(text_value)?;
        let pattern_len = self.string_byte_len(pattern_value)?;
        let result_ty = Type::Named("KmpSearchInfo".to_string());
        let output = self
            .builder
            .build_alloca(self.llvm_type(&result_ty), "kmp.info.output")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(output, self.llvm_type(&result_ty).const_zero())
            .map_err(|e| e.to_string())?;
        let function = self.declare_libc_fn(
            "tyrus_algorithm_kmp_search_info",
            &[
                pointer.into(),
                self.usize_type().into(),
                pointer.into(),
                self.usize_type().into(),
                pointer.into(),
            ],
            None,
        );
        self.builder
            .build_call(
                function,
                &[
                    text.into(),
                    text_len.into(),
                    pattern.into(),
                    pattern_len.into(),
                    output.into(),
                ],
                "kmp.info.call",
            )
            .map_err(|e| e.to_string())?;
        let result = self
            .builder
            .build_load(self.llvm_type(&result_ty), output, "kmp.info.result")
            .map_err(|e| e.to_string())?;
        self.record_owned_native_result(result, result_ty);
        Ok(result)
    }

    pub(crate) fn emit_z_algorithm_info_call(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let _strlen =
            self.declare_libc_fn("strlen", &[pointer.into()], Some(self.usize_type().into()));
        let text_value = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
        let text = self.string_data_ptr(text_value)?;
        let text_len = self.string_byte_len(text_value)?;
        let result_ty = Type::Named("ZAlgorithmInfo".to_string());
        let output = self
            .builder
            .build_alloca(self.llvm_type(&result_ty), "z.info.output")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(output, self.llvm_type(&result_ty).const_zero())
            .map_err(|e| e.to_string())?;
        let function = self.declare_libc_fn(
            "tyrus_algorithm_z_algorithm_info",
            &[pointer.into(), self.usize_type().into(), pointer.into()],
            None,
        );
        self.builder
            .build_call(
                function,
                &[text.into(), text_len.into(), output.into()],
                "z.info.call",
            )
            .map_err(|e| e.to_string())?;
        let result = self
            .builder
            .build_load(self.llvm_type(&result_ty), output, "z.info.result")
            .map_err(|e| e.to_string())?;
        self.record_owned_native_result(result, result_ty);
        Ok(result)
    }

    pub(crate) fn emit_rabin_karp_info_call(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let _strlen =
            self.declare_libc_fn("strlen", &[pointer.into()], Some(self.usize_type().into()));
        let text_value = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
        let text = self.string_data_ptr(text_value)?;
        let pattern_value = self.emit_expr_as(&args[1], &Type::Str)?.into_struct_value();
        let pattern = self.string_data_ptr(pattern_value)?;
        let text_len = self.string_byte_len(text_value)?;
        let pattern_len = self.string_byte_len(pattern_value)?;
        let result_ty = Type::Named("RabinKarpInfo".to_string());
        let output = self
            .builder
            .build_alloca(self.llvm_type(&result_ty), "rabin.info.output")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(output, self.llvm_type(&result_ty).const_zero())
            .map_err(|e| e.to_string())?;
        let function = self.declare_libc_fn(
            "tyrus_algorithm_rabin_karp_info",
            &[
                pointer.into(),
                self.usize_type().into(),
                pointer.into(),
                self.usize_type().into(),
                pointer.into(),
            ],
            None,
        );
        self.builder
            .build_call(
                function,
                &[
                    text.into(),
                    text_len.into(),
                    pattern.into(),
                    pattern_len.into(),
                    output.into(),
                ],
                "rabin.info.call",
            )
            .map_err(|e| e.to_string())?;
        let result = self
            .builder
            .build_load(self.llvm_type(&result_ty), output, "rabin.info.result")
            .map_err(|e| e.to_string())?;
        self.record_owned_native_result(result, result_ty);
        Ok(result)
    }

    pub(crate) fn emit_boyer_moore_info_call(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let _strlen =
            self.declare_libc_fn("strlen", &[pointer.into()], Some(self.usize_type().into()));
        let text_value = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
        let text = self.string_data_ptr(text_value)?;
        let pattern_value = self.emit_expr_as(&args[1], &Type::Str)?.into_struct_value();
        let pattern = self.string_data_ptr(pattern_value)?;
        let text_len = self.string_byte_len(text_value)?;
        let pattern_len = self.string_byte_len(pattern_value)?;
        let result_ty = Type::Named("BoyerMooreInfo".to_string());
        let output = self
            .builder
            .build_alloca(self.llvm_type(&result_ty), "boyer.info.output")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(output, self.llvm_type(&result_ty).const_zero())
            .map_err(|e| e.to_string())?;
        let function = self.declare_libc_fn(
            "tyrus_algorithm_boyer_moore_info",
            &[
                pointer.into(),
                self.usize_type().into(),
                pointer.into(),
                self.usize_type().into(),
                pointer.into(),
            ],
            None,
        );
        self.builder
            .build_call(
                function,
                &[
                    text.into(),
                    text_len.into(),
                    pattern.into(),
                    pattern_len.into(),
                    output.into(),
                ],
                "boyer.info.call",
            )
            .map_err(|e| e.to_string())?;
        let result = self
            .builder
            .build_load(self.llvm_type(&result_ty), output, "boyer.info.result")
            .map_err(|e| e.to_string())?;
        self.record_owned_native_result(result, result_ty);
        Ok(result)
    }

    pub(crate) fn emit_aho_corasick_info_call(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let _strlen =
            self.declare_libc_fn("strlen", &[pointer.into()], Some(self.usize_type().into()));
        let text_value = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
        let text = self.string_data_ptr(text_value)?;
        let text_len = self.string_byte_len(text_value)?;
        let patterns_ty = Type::Array(Box::new(Type::Str));
        let patterns = self.emit_expr_as(&args[1], &patterns_ty)?;
        let patterns_ptr = self
            .builder
            .build_alloca(self.llvm_type(&patterns_ty), "aho.info.patterns")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(patterns_ptr, patterns)
            .map_err(|e| e.to_string())?;
        let result_ty = Type::Named("AhoCorasickInfo".to_string());
        let output = self
            .builder
            .build_alloca(self.llvm_type(&result_ty), "aho.info.output")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(output, self.llvm_type(&result_ty).const_zero())
            .map_err(|e| e.to_string())?;
        let function = self.declare_libc_fn(
            "tyrus_algorithm_aho_corasick_info",
            &[
                pointer.into(),
                self.usize_type().into(),
                pointer.into(),
                pointer.into(),
            ],
            None,
        );
        self.builder
            .build_call(
                function,
                &[
                    text.into(),
                    text_len.into(),
                    patterns_ptr.into(),
                    output.into(),
                ],
                "aho.info.call",
            )
            .map_err(|e| e.to_string())?;
        let result = self
            .builder
            .build_load(self.llvm_type(&result_ty), output, "aho.info.result")
            .map_err(|e| e.to_string())?;
        self.record_owned_native_result(result, result_ty);
        Ok(result)
    }

    pub(crate) fn emit_levenshtein_info_call(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let _strlen =
            self.declare_libc_fn("strlen", &[pointer.into()], Some(self.usize_type().into()));
        let left_value = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
        let left = self.string_data_ptr(left_value)?;
        let right_value = self.emit_expr_as(&args[1], &Type::Str)?.into_struct_value();
        let right = self.string_data_ptr(right_value)?;
        let left_len = self.string_byte_len(left_value)?;
        let right_len = self.string_byte_len(right_value)?;
        let result_ty = Type::Named("EditDistanceInfo".to_string());
        let output = self
            .builder
            .build_alloca(self.llvm_type(&result_ty), "levenshtein.info.output")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(output, self.llvm_type(&result_ty).const_zero())
            .map_err(|e| e.to_string())?;
        let function = self.declare_libc_fn(
            "tyrus_algorithm_levenshtein_info",
            &[
                pointer.into(),
                self.usize_type().into(),
                pointer.into(),
                self.usize_type().into(),
                pointer.into(),
            ],
            None,
        );
        self.builder
            .build_call(
                function,
                &[
                    left.into(),
                    left_len.into(),
                    right.into(),
                    right_len.into(),
                    output.into(),
                ],
                "levenshtein.info.call",
            )
            .map_err(|e| e.to_string())?;
        let result = self
            .builder
            .build_load(
                self.llvm_type(&result_ty),
                output,
                "levenshtein.info.result",
            )
            .map_err(|e| e.to_string())?;
        self.record_owned_native_result(result, result_ty);
        Ok(result)
    }
}
