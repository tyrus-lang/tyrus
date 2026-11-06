use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_crypto_digest_stream_call(
        &mut self,
        target: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        match target {
            tyrus_core::crypto::DIGEST_CREATE_TARGET => {
                if args.len() != 1 {
                    return Err("error: `digest.create` expects 1 argument".into());
                }
                let alg = self.emit_string_arg(&args[0])?;
                let result_type = Type::Result {
                    success: Box::new(Type::Named("DigestStream".to_string())),
                    error: Box::new(Type::Named("CryptoError".to_string())),
                };
                let llvm_type = self.llvm_type(&result_type);
                let output = self
                    .builder
                    .build_alloca(llvm_type, "crypto.digest.create.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_crypto_digest_create_result",
                    &[self.string_abi_type(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[alg.into(), output.into()],
                        "crypto.digest.create",
                    )
                    .map_err(|error| error.to_string())?;
                let value = self
                    .builder
                    .build_load(llvm_type, output, "crypto.digest.create.value")
                    .map_err(|error| error.to_string())?;
                self.record_owned_native_result(value, result_type);
                Ok(value)
            }
            tyrus_core::crypto::DIGEST_STREAM_UPDATE_TARGET => {
                if args.len() != 2 {
                    return Err(
                        "error: `DigestStream.update` expects 2 arguments (self, data)".into(),
                    );
                }
                let stream = self.emit_expr(&args[0])?;
                let data = self.emit_bytes_as_ptr_len(&args[1])?;
                let result_type = Type::Result {
                    success: Box::new(Type::Named("DigestStream".to_string())),
                    error: Box::new(Type::Named("CryptoError".to_string())),
                };
                let llvm_type = self.llvm_type(&result_type);
                let output = self
                    .builder
                    .build_alloca(llvm_type, "crypto.digest.update.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_crypto_digest_stream_update_result",
                    &[
                        pointer.into(),
                        pointer.into(),
                        self.usize_type().into(),
                        pointer.into(),
                    ],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[stream.into(), data.0.into(), data.1.into(), output.into()],
                        "crypto.digest.update",
                    )
                    .map_err(|error| error.to_string())?;
                let value = self
                    .builder
                    .build_load(llvm_type, output, "crypto.digest.update.value")
                    .map_err(|error| error.to_string())?;
                Ok(value)
            }
            tyrus_core::crypto::DIGEST_STREAM_DIGEST_TARGET => {
                if args.len() != 1 {
                    return Err("error: `DigestStream.digest` expects 1 argument (self)".into());
                }
                let stream = self.emit_expr(&args[0])?;
                let result_type = Type::Result {
                    success: Box::new(Type::Array(Box::new(Type::U8))),
                    error: Box::new(Type::Named("CryptoError".to_string())),
                };
                let llvm_type = self.llvm_type(&result_type);
                let output = self
                    .builder
                    .build_alloca(llvm_type, "crypto.digest.stream.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_crypto_digest_stream_digest_result",
                    &[pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[stream.into(), output.into()],
                        "crypto.digest.digest",
                    )
                    .map_err(|error| error.to_string())?;
                let value = self
                    .builder
                    .build_load(llvm_type, output, "crypto.digest.digest.value")
                    .map_err(|error| error.to_string())?;
                self.record_owned_native_result(value, result_type);
                Ok(value)
            }
            tyrus_core::crypto::DIGEST_STREAM_RESET_TARGET => {
                if args.len() != 1 {
                    return Err("error: `DigestStream.reset` expects 1 argument (self)".into());
                }
                let stream = self.emit_expr(&args[0])?;
                let result_type = Type::Result {
                    success: Box::new(Type::Named("DigestStream".to_string())),
                    error: Box::new(Type::Named("CryptoError".to_string())),
                };
                let llvm_type = self.llvm_type(&result_type);
                let output = self
                    .builder
                    .build_alloca(llvm_type, "crypto.digest.reset.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_crypto_digest_stream_reset_result",
                    &[pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[stream.into(), output.into()],
                        "crypto.digest.reset",
                    )
                    .map_err(|error| error.to_string())?;
                let value = self
                    .builder
                    .build_load(llvm_type, output, "crypto.digest.reset.value")
                    .map_err(|error| error.to_string())?;
                Ok(value)
            }
            tyrus_core::crypto::DIGEST_STREAM_DISPOSE_TARGET => {
                if args.len() != 1 {
                    return Err("error: `DigestStream.dispose` expects 1 argument (self)".into());
                }
                let stream = self.emit_expr(&args[0])?;
                let function = self.declare_libc_fn(
                    "tyrus_crypto_digest_stream_free",
                    &[pointer.into()],
                    None,
                );
                self.builder
                    .build_call(function, &[stream.into()], "crypto.digest.dispose")
                    .map_err(|error| error.to_string())?;
                if let ExprKind::Ident(name) = args[0].kind() {
                    self.dispose_owned_native_binding(name);
                }
                Ok(stream)
            }
            _ => Err(format!(
                "internal: `{target}` is not handled by `emit_crypto_digest_stream_call`"
            )),
        }
    }
}
