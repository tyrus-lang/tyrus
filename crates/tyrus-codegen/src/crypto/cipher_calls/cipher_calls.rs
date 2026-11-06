use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_crypto_cipher_call(
        &mut self,
        target: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        match target {
            tyrus_core::crypto::CIPHER_AES_ENCRYPT_TARGET => {
                if !(2..=3).contains(&args.len()) {
                    return Err("error: AES encrypt expects 2 or 3 arguments".into());
                }
                let data = self.emit_bytes_as_ptr_len(&args[0])?;
                let key = self.emit_bytes_as_ptr_len(&args[1])?;
                let aad = args
                    .get(2)
                    .map(|value| self.emit_bytes_as_ptr_len(value))
                    .transpose()?;
                let aad_ptr = aad.map_or_else(|| pointer.const_null(), |value| value.0);
                let aad_len = aad.map_or_else(|| self.usize_type().const_zero(), |value| value.1);
                let result_type = Type::Result {
                    success: Box::new(Type::Array(Box::new(Type::U8))),
                    error: Box::new(Type::Named("CryptoError".to_string())),
                };
                let result_llvm_type = self.llvm_type(&result_type);
                let output = self
                    .builder
                    .build_alloca(result_llvm_type, "crypto.cipher.encrypt.result")
                    .map_err(|error| error.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_crypto_cipher_aes_encrypt_result",
                    &[
                        pointer.into(),
                        self.usize_type().into(),
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
                        fn_val,
                        &[
                            data.0.into(),
                            data.1.into(),
                            key.0.into(),
                            key.1.into(),
                            aad_ptr.into(),
                            aad_len.into(),
                            output.into(),
                        ],
                        "crypto.cipher.encrypt",
                    )
                    .unwrap();
                let result = self
                    .builder
                    .build_load(result_llvm_type, output, "crypto.cipher.encrypt.value")
                    .map_err(|error| error.to_string())?;
                self.record_owned_native_result(result, result_type);
                Ok(result)
            }
            tyrus_core::crypto::CIPHER_AES_DECRYPT_STR_TARGET => {
                if !(2..=3).contains(&args.len()) {
                    return Err("error: AES decryptText expects 2 or 3 arguments".into());
                }
                let ct = self.emit_bytes_as_ptr_len(&args[0])?;
                let key = self.emit_bytes_as_ptr_len(&args[1])?;
                let aad = args
                    .get(2)
                    .map(|value| self.emit_bytes_as_ptr_len(value))
                    .transpose()?;
                let aad_ptr = aad.map_or_else(|| pointer.const_null(), |value| value.0);
                let aad_len = aad.map_or_else(|| self.usize_type().const_zero(), |value| value.1);
                let result_type = Type::Result {
                    success: Box::new(Type::Str),
                    error: Box::new(Type::Named("CryptoError".to_string())),
                };
                let result_llvm_type = self.llvm_type(&result_type);
                let output = self
                    .builder
                    .build_alloca(result_llvm_type, "crypto.cipher.decrypt_text.result")
                    .map_err(|error| error.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_crypto_cipher_aes_decrypt_str_result",
                    &[
                        pointer.into(),
                        self.usize_type().into(),
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
                        fn_val,
                        &[
                            ct.0.into(),
                            ct.1.into(),
                            key.0.into(),
                            key.1.into(),
                            aad_ptr.into(),
                            aad_len.into(),
                            output.into(),
                        ],
                        "crypto.cipher.decrypt_text",
                    )
                    .map_err(|error| error.to_string())?;
                let result = self
                    .builder
                    .build_load(result_llvm_type, output, "crypto.cipher.decrypt_text.value")
                    .map_err(|error| error.to_string())?;
                self.record_owned_native_result(result, result_type);
                Ok(result)
            }
            tyrus_core::crypto::CIPHER_AES_DECRYPT_BYTES_TARGET => {
                if !(2..=3).contains(&args.len()) {
                    return Err("error: AES decryptBytes expects 2 or 3 arguments".into());
                }
                let ct = self.emit_bytes_as_ptr_len(&args[0])?;
                let key = self.emit_bytes_as_ptr_len(&args[1])?;
                let aad = args
                    .get(2)
                    .map(|value| self.emit_bytes_as_ptr_len(value))
                    .transpose()?;
                let aad_ptr = aad.map_or_else(|| pointer.const_null(), |value| value.0);
                let aad_len = aad.map_or_else(|| self.usize_type().const_zero(), |value| value.1);
                let result_type = Type::Result {
                    success: Box::new(Type::Array(Box::new(Type::U8))),
                    error: Box::new(Type::Named("CryptoError".to_string())),
                };
                let result_llvm_type = self.llvm_type(&result_type);
                let output = self
                    .builder
                    .build_alloca(result_llvm_type, "crypto.cipher.decrypt_bytes.result")
                    .map_err(|error| error.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_crypto_cipher_aes_decrypt_bytes_result",
                    &[
                        pointer.into(),
                        self.usize_type().into(),
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
                        fn_val,
                        &[
                            ct.0.into(),
                            ct.1.into(),
                            key.0.into(),
                            key.1.into(),
                            aad_ptr.into(),
                            aad_len.into(),
                            output.into(),
                        ],
                        "crypto.cipher.decrypt_bytes",
                    )
                    .unwrap();
                let result = self
                    .builder
                    .build_load(
                        result_llvm_type,
                        output,
                        "crypto.cipher.decrypt_bytes.value",
                    )
                    .map_err(|error| error.to_string())?;
                self.record_owned_native_result(result, result_type);
                Ok(result)
            }
            _ => Err(format!(
                "internal: `{target}` is not handled by `emit_crypto_cipher_call`"
            )),
        }
    }
}
