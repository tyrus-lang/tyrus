use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_encoding_runtime_call(
        &mut self,
        target: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let returns_result = tyrus_core::encoding::encoding_target_returns_result(target);
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        match target {
            tyrus_core::encoding::UTF8_ENCODE_TARGET
            | tyrus_core::encoding::UTF16_ENCODE_TARGET
            | tyrus_core::encoding::UTF16_ENCODE_LE_TARGET
            | tyrus_core::encoding::UTF16_ENCODE_BE_TARGET
            | tyrus_core::encoding::HEX_DECODE_TARGET
            | tyrus_core::encoding::BASE64_DECODE_TARGET
            | tyrus_core::encoding::BASE64URL_DECODE_TARGET
            | tyrus_core::encoding::BASE32_DECODE_TARGET => {
                if args.len() != 1 {
                    return Err(format!("error: `{target}` expects 1 argument"));
                }
                let output_ty = match target {
                    tyrus_core::encoding::UTF16_ENCODE_TARGET => Type::Array(Box::new(Type::U16)),
                    _ => Type::Array(Box::new(Type::U8)),
                };
                let value = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let result_ty = Type::Result {
                    success: Box::new(output_ty.clone()),
                    error: Box::new(Type::Named("EncodingError".to_string())),
                };
                let output_llvm_ty = self.llvm_type(if returns_result {
                    &result_ty
                } else {
                    &output_ty
                });
                let output = self
                    .builder
                    .build_alloca(output_llvm_ty, "encoding.output")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(output, output_llvm_ty.const_zero())
                    .map_err(|error| error.to_string())?;
                let function_name = match target {
                    tyrus_core::encoding::UTF8_ENCODE_TARGET => "tyrus_encoding_utf8_encode",
                    tyrus_core::encoding::UTF16_ENCODE_TARGET => "tyrus_encoding_utf16_encode",
                    tyrus_core::encoding::UTF16_ENCODE_LE_TARGET => {
                        "tyrus_encoding_utf16_encode_le"
                    }
                    tyrus_core::encoding::UTF16_ENCODE_BE_TARGET => {
                        "tyrus_encoding_utf16_encode_be"
                    }
                    tyrus_core::encoding::HEX_DECODE_TARGET => "tyrus_encoding_hex_decode_result",
                    tyrus_core::encoding::BASE64_DECODE_TARGET => {
                        "tyrus_encoding_base64_decode_result"
                    }
                    tyrus_core::encoding::BASE64URL_DECODE_TARGET => {
                        "tyrus_encoding_base64url_decode_result"
                    }
                    tyrus_core::encoding::BASE32_DECODE_TARGET => {
                        "tyrus_encoding_base32_decode_result"
                    }
                    _ => unreachable!(),
                };
                let function = self.declare_libc_fn(
                    function_name,
                    &[self.string_abi_type(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(function, &[value.into(), output.into()], "encoding.encode")
                    .map_err(|error| error.to_string())?;
                let result_type = if returns_result { result_ty } else { output_ty };
                let result = self
                    .builder
                    .build_load(self.llvm_type(&result_type), output, "encoding.result")
                    .map_err(|error| error.to_string())?;
                self.record_owned_native_result(result, result_type);
                Ok(result)
            }
            tyrus_core::encoding::UTF8_DECODE_TARGET
            | tyrus_core::encoding::UTF8_DECODE_LOSSY_TARGET
            | tyrus_core::encoding::UTF16_DECODE_TARGET
            | tyrus_core::encoding::UTF16_DECODE_LOSSY_TARGET
            | tyrus_core::encoding::UTF16_DECODE_LE_TARGET
            | tyrus_core::encoding::UTF16_DECODE_LE_LOSSY_TARGET
            | tyrus_core::encoding::UTF16_DECODE_BE_TARGET
            | tyrus_core::encoding::UTF16_DECODE_BE_LOSSY_TARGET
            | tyrus_core::encoding::HEX_ENCODE_TARGET
            | tyrus_core::encoding::BASE64_ENCODE_TARGET
            | tyrus_core::encoding::BASE64URL_ENCODE_TARGET
            | tyrus_core::encoding::BASE32_ENCODE_TARGET => {
                if args.len() != 1 {
                    return Err(format!("error: `{target}` expects 1 argument"));
                }
                let element_ty = match target {
                    tyrus_core::encoding::UTF16_DECODE_TARGET
                    | tyrus_core::encoding::UTF16_DECODE_LOSSY_TARGET => Type::U16,
                    _ => Type::U8,
                };
                let (data, length) = self.emit_encoding_array_ptr_len(&args[0], element_ty)?;
                let raw_function_name = match target {
                    tyrus_core::encoding::UTF8_DECODE_TARGET => "tyrus_encoding_utf8_decode",
                    tyrus_core::encoding::UTF8_DECODE_LOSSY_TARGET => {
                        "tyrus_encoding_utf8_decode_lossy"
                    }
                    tyrus_core::encoding::UTF16_DECODE_TARGET => "tyrus_encoding_utf16_decode",
                    tyrus_core::encoding::UTF16_DECODE_LOSSY_TARGET => {
                        "tyrus_encoding_utf16_decode_lossy"
                    }
                    tyrus_core::encoding::UTF16_DECODE_LE_TARGET => {
                        "tyrus_encoding_utf16_decode_le"
                    }
                    tyrus_core::encoding::UTF16_DECODE_LE_LOSSY_TARGET => {
                        "tyrus_encoding_utf16_decode_le_lossy"
                    }
                    tyrus_core::encoding::UTF16_DECODE_BE_TARGET => {
                        "tyrus_encoding_utf16_decode_be"
                    }
                    tyrus_core::encoding::UTF16_DECODE_BE_LOSSY_TARGET => {
                        "tyrus_encoding_utf16_decode_be_lossy"
                    }
                    tyrus_core::encoding::HEX_ENCODE_TARGET => "tyrus_encoding_hex_encode",
                    tyrus_core::encoding::BASE64_ENCODE_TARGET => "tyrus_encoding_base64_encode",
                    tyrus_core::encoding::BASE64URL_ENCODE_TARGET => {
                        "tyrus_encoding_base64url_encode"
                    }
                    tyrus_core::encoding::BASE32_ENCODE_TARGET => "tyrus_encoding_base32_encode",
                    _ => return Err(format!("error: unsupported encoding target `{target}`")),
                };
                if returns_result {
                    let result_type = Type::Result {
                        success: Box::new(Type::Str),
                        error: Box::new(Type::Named("EncodingError".to_string())),
                    };
                    let result_llvm_type = self.llvm_type(&result_type);
                    let output = self
                        .builder
                        .build_alloca(result_llvm_type, "encoding.decode.result")
                        .map_err(|error| error.to_string())?;
                    let function = self.declare_libc_fn(
                        &format!("{raw_function_name}_result"),
                        &[pointer.into(), self.usize_type().into(), pointer.into()],
                        None,
                    );
                    self.builder
                        .build_call(
                            function,
                            &[data.into(), length.into(), output.into()],
                            "encoding.decode",
                        )
                        .map_err(|error| error.to_string())?;
                    let result = self
                        .builder
                        .build_load(result_llvm_type, output, "encoding.decode.value")
                        .map_err(|error| error.to_string())?;
                    self.record_owned_native_result(result, result_type);
                    return Ok(result);
                }
                let function = self.declare_libc_fn(
                    raw_function_name,
                    &[pointer.into(), self.usize_type().into()],
                    Some(self.string_abi_type()),
                );
                let result = self
                    .builder
                    .build_call(function, &[data.into(), length.into()], "encoding.decode")
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.record_owned_native_result(result, Type::Str);
                Ok(result)
            }
            tyrus_core::encoding::RADIX_FORMAT_TARGET => {
                if args.len() != 2 {
                    return Err(format!("error: `{target}` expects 2 arguments"));
                }
                let value = self.emit_expr_as(&args[0], &Type::I64)?.into_int_value();
                let base = self.emit_expr_as(&args[1], &Type::I32)?.into_int_value();
                let function = self.declare_libc_fn(
                    "tyrus_encoding_radix_format",
                    &[
                        self.context.i64_type().into(),
                        self.context.i32_type().into(),
                    ],
                    Some(self.string_abi_type()),
                );
                let result = self
                    .builder
                    .build_call(
                        function,
                        &[value.into(), base.into()],
                        "encoding.radix.format",
                    )
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.record_owned_native_result(result, Type::Str);
                Ok(result)
            }
            tyrus_core::encoding::RADIX_PARSE_I64_TARGET
            | tyrus_core::encoding::RADIX_PARSE_U64_TARGET => {
                if args.len() != 2 {
                    return Err(format!("error: `{target}` expects 2 arguments"));
                }
                let value = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let base = self.emit_expr_as(&args[1], &Type::I32)?.into_int_value();
                let success_type = if target == tyrus_core::encoding::RADIX_PARSE_I64_TARGET {
                    Type::I64
                } else {
                    Type::U64
                };
                let result_type = Type::Result {
                    success: Box::new(success_type),
                    error: Box::new(Type::Named("EncodingError".to_string())),
                };
                let result_llvm_type = self.llvm_type(&result_type);
                let output = self
                    .builder
                    .build_alloca(result_llvm_type, "encoding.radix.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    match target {
                        tyrus_core::encoding::RADIX_PARSE_I64_TARGET => {
                            "tyrus_encoding_radix_parse_i64_result"
                        }
                        _ => "tyrus_encoding_radix_parse_u64_result",
                    },
                    &[
                        self.string_abi_type(),
                        self.context.i32_type().into(),
                        pointer.into(),
                    ],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[value.into(), base.into(), output.into()],
                        "encoding.radix.parse",
                    )
                    .map_err(|error| error.to_string())?;
                let result = self
                    .builder
                    .build_load(result_llvm_type, output, "encoding.radix.value")
                    .map_err(|error| error.to_string())?;
                self.record_owned_native_result(result, result_type);
                Ok(result)
            }
            _ => Err(format!("error: unsupported encoding target `{target}`")),
        }
    }

    fn emit_encoding_array_ptr_len(
        &mut self,
        arg: &Expr,
        element: Type,
    ) -> Result<(PointerValue<'ctx>, IntValue<'ctx>), String> {
        let array = self
            .emit_expr_as(arg, &Type::Array(Box::new(element)))?
            .into_struct_value();
        let data = self
            .builder
            .build_extract_value(array, 0, "encoding.array.data")
            .unwrap()
            .into_pointer_value();
        let length = self
            .builder
            .build_extract_value(array, 1, "encoding.array.length")
            .unwrap()
            .into_int_value();
        Ok((data, length))
    }
}
