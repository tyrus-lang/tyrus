use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_crypto_token_call(
        &mut self,
        target: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        match target {
            tyrus_core::crypto::ARGON2_HASH_TARGET => {
                if !(1..=2).contains(&args.len()) {
                    return Err("error: `argon2.hash` expects 1 or 2 arguments".into());
                }
                let pw = self.emit_bytes_as_ptr_len(&args[0])?;
                let options = self.emit_crypto_options_pointer(
                    args.get(1),
                    &tyrus_core::crypto::argon2_hash_options_type(),
                    "crypto.argon2.hash.options",
                )?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_crypto_argon2_hash",
                    &[pointer.into(), self.usize_type().into(), pointer.into()],
                    Some(pointer.into()),
                );
                let result = self
                    .builder
                    .build_call(
                        fn_val,
                        &[pw.0.into(), pw.1.into(), options.into()],
                        "crypto.argon2.hash",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                Ok(result)
            }
            tyrus_core::crypto::ARGON2_VERIFY_TARGET => {
                if !(2..=3).contains(&args.len()) {
                    return Err("error: `argon2.verify` expects 2 or 3 arguments".into());
                }
                let hash = self.emit_string_arg(&args[0])?;
                let pw = self.emit_bytes_as_ptr_len(&args[1])?;
                let options = self.emit_crypto_options_pointer(
                    args.get(2),
                    &tyrus_core::crypto::argon2_verify_options_type(),
                    "crypto.argon2.verify.options",
                )?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_crypto_argon2_verify",
                    &[
                        self.string_abi_type(),
                        pointer.into(),
                        self.usize_type().into(),
                        pointer.into(),
                    ],
                    Some(pointer.into()),
                );
                let result = self
                    .builder
                    .build_call(
                        fn_val,
                        &[hash.into(), pw.0.into(), pw.1.into(), options.into()],
                        "crypto.argon2.verify",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                Ok(result)
            }
            tyrus_core::crypto::ARGON2_NEEDS_REHASH_TARGET => {
                if args.len() != 1 {
                    return Err("error: `argon2.needsRehash` expects 1 argument".into());
                }
                let hash = self.emit_string_arg(&args[0])?;
                let result_type = Type::Result {
                    success: Box::new(Type::Bool),
                    error: Box::new(Type::Named("CryptoError".to_string())),
                };
                let result_llvm_type = self.llvm_type(&result_type);
                let output = self
                    .builder
                    .build_alloca(result_llvm_type, "crypto.argon2.needs_rehash.result")
                    .map_err(|error| error.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_crypto_argon2_needs_rehash_result",
                    &[self.string_abi_type(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[hash.into(), output.into()],
                        "crypto.argon2.needs_rehash",
                    )
                    .map_err(|error| error.to_string())?;
                let result = self
                    .builder
                    .build_load(result_llvm_type, output, "crypto.argon2.needs_rehash.value")
                    .map_err(|error| error.to_string())?;
                self.record_owned_native_result(result, result_type);
                Ok(result)
            }
            tyrus_core::crypto::JWT_FETCH_JWKS_TARGET => {
                if !(1..=2).contains(&args.len()) {
                    return Err("error: `jwt.fetchJwks` expects 1 or 2 arguments".into());
                }
                let url = self.emit_string_arg(&args[0])?;
                let options = self.emit_crypto_options_pointer(
                    args.get(1),
                    &tyrus_core::crypto::jwks_fetch_options_type(),
                    "crypto.jwt.fetch_jwks.options",
                )?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_crypto_jwt_fetch_jwks",
                    &[self.string_abi_type(), pointer.into()],
                    Some(pointer.into()),
                );
                let result = self
                    .builder
                    .build_call(
                        fn_val,
                        &[url.into(), options.into()],
                        "crypto.jwt.fetch_jwks",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                Ok(result)
            }
            _ => Err(format!(
                "internal: `{target}` is not handled by `emit_crypto_token_call`"
            )),
        }
    }
}
