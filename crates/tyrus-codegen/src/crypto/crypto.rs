#[path = "cipher_calls/cipher_calls.rs"]
mod cipher_calls;
#[path = "digest_stream_calls/digest_stream_calls.rs"]
mod digest_stream_calls;
#[path = "token_calls/token_calls.rs"]
mod token_calls;
use super::*;

enum JwtCleanup<'ctx> {
    Handle(PointerValue<'ctx>, &'static str),
    Text(StructValue<'ctx>),
}

fn crypto_target_returns_result(target: &str) -> bool {
    matches!(
        target,
        tyrus_core::crypto::HMAC_SHA256_TARGET
            | tyrus_core::crypto::HMAC_SHA384_TARGET
            | tyrus_core::crypto::HMAC_SHA512_TARGET
            | tyrus_core::crypto::DIGEST_SHA256_TARGET
            | tyrus_core::crypto::DIGEST_SHA384_TARGET
            | tyrus_core::crypto::DIGEST_SHA512_TARGET
            | tyrus_core::crypto::DIGEST_MD5_TARGET
            | tyrus_core::crypto::RAND_ULID_TARGET
            | tyrus_core::crypto::RAND_RANDOM_BYTES_TARGET
            | tyrus_core::crypto::RAND_RANDOM_INT_TARGET
            | tyrus_core::crypto::CIPHER_AES_ENCRYPT_TARGET
            | tyrus_core::crypto::CIPHER_AES_DECRYPT_STR_TARGET
            | tyrus_core::crypto::CIPHER_AES_DECRYPT_BYTES_TARGET
            | tyrus_core::crypto::ARGON2_HASH_TARGET
            | tyrus_core::crypto::ARGON2_VERIFY_TARGET
            | tyrus_core::crypto::ARGON2_NEEDS_REHASH_TARGET
            | tyrus_core::crypto::JWT_FETCH_JWKS_TARGET
            | tyrus_core::crypto::DIGEST_CREATE_TARGET
            | tyrus_core::crypto::DIGEST_STREAM_UPDATE_TARGET
            | tyrus_core::crypto::DIGEST_STREAM_DIGEST_TARGET
            | tyrus_core::crypto::DIGEST_STREAM_RESET_TARGET
            | tyrus_core::crypto::DIGEST_STREAM_DISPOSE_TARGET
    )
}

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_crypto_runtime_call_typed(
        &mut self,
        target: &str,
        type_args: &[Type],
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if matches!(
            target,
            tyrus_core::crypto::JWT_SIGN_TARGET
                | tyrus_core::crypto::JWT_VERIFY_TARGET
                | tyrus_core::crypto::JWT_VERIFY_WITH_JWKS_TARGET
                | tyrus_core::crypto::JWT_DECODE_TARGET
        ) {
            return self.emit_typed_jwt_call(target, type_args, args);
        }
        self.emit_crypto_runtime_call(target, args)
    }

    fn jwt_claim_fields(&self, claim_type: &Type) -> Result<Vec<Field>, String> {
        match claim_type {
            Type::Named(name) => self
                .interfaces
                .get(name)
                .cloned()
                .or_else(|| match self.resolve_type(claim_type) {
                    Type::Object(fields) => Some(fields),
                    _ => None,
                })
                .ok_or_else(|| format!("error: unknown JWT claims type `{name}`")),
            Type::Object(fields) => Ok(fields.clone()),
            other => Err(format!(
                "error: JWT claims must use a typed object, found `{other}`"
            )),
        }
    }

    fn emit_typed_jwt_call(
        &mut self,
        target: &str,
        type_args: &[Type],
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        self.clear_crypto_runtime_error();
        if target == tyrus_core::crypto::JWT_SIGN_TARGET {
            if !(2..=3).contains(&args.len()) {
                return Err("error: `jwt.sign` expects 2 or 3 arguments".into());
            }
            let claim_type = type_args
                .first()
                .cloned()
                .or_else(|| self.infer_jwt_claim_type(&args[0]))
                .ok_or_else(|| "error: cannot infer the JWT claims type".to_string())?;
            let result_type = Type::Result {
                success: Box::new(Type::Str),
                error: Box::new(Type::Named("CryptoError".to_string())),
            };
            let result_llvm_type = self.llvm_type(&result_type);
            let result_slot = self.entry_alloca(result_llvm_type, "crypto.jwt.sign.result")?;
            self.builder
                .build_store(result_slot, result_llvm_type.const_zero())
                .map_err(|error| error.to_string())?;
            let function_value = self
                .builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_parent)
                .ok_or("internal: JWT signing emitted outside a function")?;
            let merge = self
                .context
                .append_basic_block(function_value, "crypto.jwt.sign.done");
            let payload = self.emit_jwt_claims_json(&args[0], &claim_type, result_slot, merge)?;
            let key = self.emit_string_arg(&args[1])?;
            let options = self.emit_crypto_options_pointer(
                args.get(2),
                &tyrus_core::crypto::jwt_sign_options_type(),
                "crypto.jwt.sign.options",
            )?;
            let function = self.declare_libc_fn(
                "tyrus_crypto_jwt_sign_result",
                &[
                    self.string_abi_type(),
                    self.string_abi_type(),
                    pointer.into(),
                    pointer.into(),
                ],
                None,
            );
            self.builder
                .build_call(
                    function,
                    &[
                        payload.into(),
                        key.into(),
                        options.into(),
                        result_slot.into(),
                    ],
                    "crypto.jwt.sign",
                )
                .map_err(|error| error.to_string())?;
            self.emit_string_release(payload)?;
            self.builder
                .build_unconditional_branch(merge)
                .map_err(|error| error.to_string())?;
            self.builder.position_at_end(merge);
            let result = self
                .builder
                .build_load(result_llvm_type, result_slot, "crypto.jwt.sign.value")
                .map_err(|error| error.to_string())?;
            self.record_owned_native_result(result, result_type);
            return Ok(result);
        }

        let claim_type = type_args
            .first()
            .cloned()
            .ok_or_else(|| "error: JWT verification requires a claims type argument".to_string())?;
        self.emit_typed_jwt_result_call(target, &claim_type, args)
    }

    fn emit_typed_jwt_result_call(
        &mut self,
        target: &str,
        claim_type: &Type,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let (runtime, options) = match target {
            tyrus_core::crypto::JWT_VERIFY_TARGET
            | tyrus_core::crypto::JWT_VERIFY_WITH_JWKS_TARGET => {
                if !(2..=3).contains(&args.len()) {
                    let method = if target == tyrus_core::crypto::JWT_VERIFY_TARGET {
                        "jwt.verify"
                    } else {
                        "jwt.verifyWithJwks"
                    };
                    return Err(format!("error: `{method}` expects 2 or 3 arguments"));
                }
                let runtime = if target == tyrus_core::crypto::JWT_VERIFY_TARGET {
                    "tyrus_crypto_jwt_verify_result"
                } else {
                    "tyrus_crypto_jwt_verify_with_jwks_result"
                };
                (
                    runtime,
                    Some(self.emit_crypto_options_pointer(
                        args.get(2),
                        &tyrus_core::crypto::jwt_verify_options_type(),
                        "crypto.jwt.verify.options",
                    )?),
                )
            }
            tyrus_core::crypto::JWT_DECODE_TARGET => {
                if args.len() != 1 {
                    return Err("error: `jwt.decodeUnverified` expects 1 argument".into());
                }
                ("tyrus_crypto_jwt_decode_unverified_result", None)
            }
            _ => unreachable!(),
        };
        let token = self.emit_string_arg(&args[0])?;
        let mut call_args = vec![token.into()];
        if target != tyrus_core::crypto::JWT_DECODE_TARGET {
            call_args.push(self.emit_string_arg(&args[1])?.into());
            call_args.push(options.unwrap().into());
        }
        let result_type = Type::Result {
            success: Box::new(claim_type.clone()),
            error: Box::new(Type::Named("CryptoError".to_string())),
        };
        let ffi_type = Type::Result {
            success: Box::new(Type::Str),
            error: Box::new(Type::Named("CryptoError".to_string())),
        };
        let result_variants = vec![claim_type.clone(), Type::Named("CryptoError".to_string())];
        let ffi_variants = vec![Type::Str, Type::Named("CryptoError".to_string())];
        let result_slot = self.entry_alloca(self.llvm_type(&result_type), "crypto.jwt.result")?;
        self.builder
            .build_store(result_slot, self.llvm_type(&result_type).const_zero())
            .map_err(|error| error.to_string())?;
        let ffi_slot = self.entry_alloca(self.llvm_type(&ffi_type), "crypto.jwt.ffi.result")?;
        let signature = if target == tyrus_core::crypto::JWT_DECODE_TARGET {
            vec![self.string_abi_type(), pointer.into()]
        } else {
            vec![
                self.string_abi_type(),
                self.string_abi_type(),
                pointer.into(),
                pointer.into(),
            ]
        };
        let function = self.declare_libc_fn(runtime, &signature, None);
        call_args.push(ffi_slot.into());
        self.builder
            .build_call(function, &call_args, "crypto.jwt.verify")
            .map_err(|error| error.to_string())?;

        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: JWT Result conversion outside a function")?;
        let merge = self
            .context
            .append_basic_block(function, "crypto.jwt.result.merge");
        let error_block = self
            .context
            .append_basic_block(function, "crypto.jwt.result.error");
        let success_block = self
            .context
            .append_basic_block(function, "crypto.jwt.result.success");
        let tag = self.load_union_tag(&ffi_variants, ffi_slot)?;
        let succeeded = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                tag,
                self.context.i8_type().const_zero(),
                "crypto.jwt.result.is_ok",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(succeeded, success_block, error_block)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(error_block);
        let error = self.load_union_payload(
            &ffi_variants,
            ffi_slot,
            &Type::Named("CryptoError".to_string()),
            "crypto.jwt.error",
        )?;
        self.store_union_variant(&result_variants, result_slot, 1, error)?;
        self.builder
            .build_unconditional_branch(merge)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(success_block);
        let json =
            self.load_union_payload(&ffi_variants, ffi_slot, &Type::Str, "crypto.jwt.json")?;
        self.emit_jwt_claims_result_from_json(
            json.into_struct_value(),
            claim_type,
            result_slot,
            merge,
        )?;
        self.builder.position_at_end(merge);
        let result = self
            .builder
            .build_load(
                self.llvm_type(&result_type),
                result_slot,
                "crypto.jwt.result.value",
            )
            .map_err(|error| error.to_string())?;
        self.record_owned_native_result(result, result_type);
        Ok(result)
    }

    fn infer_jwt_claim_type(&self, expression: &Expr) -> Option<Type> {
        if let Some(ty) = self.codegen_expr_type(expression) {
            return Some(ty);
        }
        let ExprKind::ObjectLit(values) = expression.kind() else {
            return None;
        };
        let mut fields = Vec::with_capacity(values.len());
        for (name, value) in values {
            fields.push(Field {
                name: name.clone(),
                ty: self.codegen_expr_type(value)?,
                readonly: false,
                optional: false,
                decorators: Vec::new(),
                span: expression.span,
            });
        }
        Some(Type::Object(fields))
    }

    fn emit_jwt_claims_json(
        &mut self,
        expression: &Expr,
        claim_type: &Type,
        result_slot: PointerValue<'ctx>,
        merge: inkwell::basic_block::BasicBlock<'ctx>,
    ) -> Result<StructValue<'ctx>, String> {
        let fields = self.jwt_claim_fields(claim_type)?;
        let value = self
            .emit_expr_as(expression, claim_type)?
            .into_struct_value();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let builder_new = self.declare_libc_fn("tyrus_json_builder_new", &[], Some(pointer.into()));
        let builder = self
            .builder
            .build_call(builder_new, &[], "crypto.jwt.json.builder")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();
        for (index, field) in fields.iter().enumerate() {
            let field_type = self.resolve_type(&field_storage_type(field));
            let field_value = self
                .builder
                .build_extract_value(value, index as u32, &field.name)
                .map_err(|error| error.to_string())?;
            let status =
                self.emit_jwt_json_builder_add(builder, &field.name, field_value, &field_type)?;
            self.guard_jwt_json_result_status(
                status,
                &[JwtCleanup::Handle(builder, "tyrus_json_builder_free")],
                result_slot,
                merge,
            )?;
        }
        let finish = self.declare_libc_fn(
            "tyrus_json_builder_finish",
            &[pointer.into()],
            Some(self.string_abi_type()),
        );
        let json = self
            .builder
            .build_call(finish, &[builder.into()], "crypto.jwt.json")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_struct_value();
        let free_builder = self.declare_libc_fn("tyrus_json_builder_free", &[pointer.into()], None);
        self.builder
            .build_call(free_builder, &[builder.into()], "")
            .map_err(|error| error.to_string())?;
        let valid = self
            .builder
            .build_not(self.string_is_null(json)?, "crypto.jwt.json.valid")
            .map_err(|error| error.to_string())?;
        self.guard_jwt_json_condition_result(valid, &[], result_slot, merge)?;
        Ok(json)
    }

    fn emit_jwt_json_builder_add(
        &self,
        builder: PointerValue<'ctx>,
        field: &str,
        value: BasicValueEnum<'ctx>,
        field_type: &Type,
    ) -> Result<IntValue<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let key = self
            .const_string_abi(field, "crypto.jwt.json.key")
            .into_struct_value();
        let (runtime, argument, argument_type): (
            &str,
            BasicMetadataValueEnum<'ctx>,
            BasicTypeEnum<'ctx>,
        ) = match transparent_codegen_type(field_type) {
            Type::Str => (
                "tyrus_json_builder_add_string",
                value.into_struct_value().into(),
                self.string_abi_type(),
            ),
            Type::Bool => (
                "tyrus_json_builder_add_bool",
                value.into_int_value().into(),
                self.context.bool_type().into(),
            ),
            Type::F32 => {
                let widened = self
                    .builder
                    .build_float_ext(
                        value.into_float_value(),
                        self.context.f64_type(),
                        "crypto.jwt.json.f64",
                    )
                    .map_err(|error| error.to_string())?;
                (
                    "tyrus_json_builder_add_f64",
                    widened.into(),
                    self.context.f64_type().into(),
                )
            }
            Type::F64 => (
                "tyrus_json_builder_add_f64",
                value.into_float_value().into(),
                self.context.f64_type().into(),
            ),
            Type::Decimal => {
                let decimal = self
                    .builder
                    .build_alloca(self.llvm_type(&Type::Decimal), "crypto.jwt.json.decimal")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(decimal, value)
                    .map_err(|error| error.to_string())?;
                (
                    "tyrus_json_builder_add_decimal",
                    decimal.into(),
                    pointer.into(),
                )
            }
            Type::I32 => {
                let widened = self
                    .builder
                    .build_int_s_extend(
                        value.into_int_value(),
                        self.context.i64_type(),
                        "crypto.jwt.json.i64",
                    )
                    .map_err(|error| error.to_string())?;
                (
                    "tyrus_json_builder_add_i64",
                    widened.into(),
                    self.context.i64_type().into(),
                )
            }
            Type::I64 => (
                "tyrus_json_builder_add_i64",
                value.into_int_value().into(),
                self.context.i64_type().into(),
            ),
            Type::U32 => {
                let widened = self
                    .builder
                    .build_int_z_extend(
                        value.into_int_value(),
                        self.context.i64_type(),
                        "crypto.jwt.json.u64",
                    )
                    .map_err(|error| error.to_string())?;
                (
                    "tyrus_json_builder_add_u64",
                    widened.into(),
                    self.context.i64_type().into(),
                )
            }
            Type::U64 => (
                "tyrus_json_builder_add_u64",
                value.into_int_value().into(),
                self.context.i64_type().into(),
            ),
            other => {
                return Err(format!(
                    "error: JWT claim field `{field}` uses unsupported type `{other}`"
                ));
            }
        };
        let function = self.declare_libc_fn(
            runtime,
            &[pointer.into(), self.string_abi_type(), argument_type],
            Some(self.context.i32_type().into()),
        );
        Ok(self
            .builder
            .build_call(
                function,
                &[builder.into(), key.into(), argument],
                "crypto.jwt.json.status",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value())
    }

    fn emit_jwt_claims_result_from_json(
        &mut self,
        json: StructValue<'ctx>,
        claim_type: &Type,
        result_slot: PointerValue<'ctx>,
        merge: inkwell::basic_block::BasicBlock<'ctx>,
    ) -> Result<(), String> {
        let fields = self.jwt_claim_fields(claim_type)?;
        let result_variants = vec![claim_type.clone(), Type::Named("CryptoError".to_string())];
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let status_slot = self
            .builder
            .build_alloca(self.context.i32_type(), "crypto.jwt.json.parse.status")
            .map_err(|error| error.to_string())?;
        let parse = self.declare_libc_fn(
            "tyrus_json_document_parse",
            &[self.string_abi_type(), pointer.into()],
            Some(pointer.into()),
        );
        let document = self
            .builder
            .build_call(
                parse,
                &[json.into(), status_slot.into()],
                "crypto.jwt.json.document",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();
        self.emit_string_release(json)?;
        let parse_status = self
            .builder
            .build_load(
                self.context.i32_type(),
                status_slot,
                "crypto.jwt.json.parse.status.value",
            )
            .map_err(|error| error.to_string())?
            .into_int_value();
        self.guard_jwt_json_result_status(parse_status, &[], result_slot, merge)?;
        let mut claims = self.llvm_type(claim_type).into_struct_type().get_undef();
        let mut owned_strings = Vec::new();
        for (index, field) in fields.iter().enumerate() {
            let field_type = self.resolve_type(&field_storage_type(field));
            let (field_value, status) =
                self.emit_jwt_json_document_field(document, &field.name, &field_type)?;
            let mut failure_cleanups =
                vec![JwtCleanup::Handle(document, "tyrus_json_document_free")];
            failure_cleanups.extend(owned_strings.iter().copied().map(JwtCleanup::Text));
            self.guard_jwt_json_result_status(status, &failure_cleanups, result_slot, merge)?;
            if matches!(transparent_codegen_type(&field_type), Type::Str) {
                owned_strings.push(field_value.into_struct_value());
            }
            claims = self
                .builder
                .build_insert_value(claims, field_value, index as u32, &field.name)
                .map_err(|error| error.to_string())?
                .into_struct_value();
        }
        let free_document =
            self.declare_libc_fn("tyrus_json_document_free", &[pointer.into()], None);
        self.builder
            .build_call(free_document, &[document.into()], "")
            .map_err(|error| error.to_string())?;
        self.store_union_variant(&result_variants, result_slot, 0, claims.into())?;
        self.builder
            .build_unconditional_branch(merge)
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn emit_jwt_json_document_field(
        &self,
        document: PointerValue<'ctx>,
        field: &str,
        field_type: &Type,
    ) -> Result<(BasicValueEnum<'ctx>, IntValue<'ctx>), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let llvm_type = self.llvm_type(field_type);
        let output = self
            .builder
            .build_alloca(llvm_type, "crypto.jwt.json.field")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(output, llvm_type.const_zero())
            .map_err(|error| error.to_string())?;
        let key = self
            .const_string_abi(field, "crypto.jwt.json.key")
            .into_struct_value();
        let runtime = match transparent_codegen_type(field_type) {
            Type::Str => "tyrus_json_document_get_string",
            Type::Bool => "tyrus_json_document_get_bool",
            Type::F32 => "tyrus_json_document_get_f32",
            Type::F64 => "tyrus_json_document_get_f64",
            Type::Decimal => "tyrus_json_document_get_decimal",
            Type::I32 => "tyrus_json_document_get_i32",
            Type::I64 => "tyrus_json_document_get_i64",
            Type::U32 => "tyrus_json_document_get_u32",
            Type::U64 => "tyrus_json_document_get_u64",
            other => {
                return Err(format!(
                    "error: JWT claim field `{field}` uses unsupported type `{other}`"
                ));
            }
        };
        let function = self.declare_libc_fn(
            runtime,
            &[pointer.into(), self.string_abi_type(), pointer.into()],
            Some(self.context.i32_type().into()),
        );
        let status = self
            .builder
            .build_call(
                function,
                &[document.into(), key.into(), output.into()],
                "crypto.jwt.json.field.status",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        let value = self
            .builder
            .build_load(llvm_type, output, "crypto.jwt.json.field.value")
            .map_err(|error| error.to_string())?;
        Ok((value, status))
    }

    fn guard_jwt_json_result_status(
        &mut self,
        status: IntValue<'ctx>,
        cleanups: &[JwtCleanup<'ctx>],
        result_slot: PointerValue<'ctx>,
        merge: inkwell::basic_block::BasicBlock<'ctx>,
    ) -> Result<(), String> {
        let valid = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                status,
                self.context.i32_type().const_zero(),
                "crypto.jwt.json.status.valid",
            )
            .map_err(|error| error.to_string())?;
        self.guard_jwt_json_condition_result(valid, cleanups, result_slot, merge)
    }

    fn guard_jwt_json_condition_result(
        &mut self,
        valid: IntValue<'ctx>,
        cleanups: &[JwtCleanup<'ctx>],
        result_slot: PointerValue<'ctx>,
        merge: inkwell::basic_block::BasicBlock<'ctx>,
    ) -> Result<(), String> {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or_else(|| "error: JWT JSON conversion is not inside a function".to_string())?;
        let success = self
            .context
            .append_basic_block(function, "crypto.jwt.json.ok");
        let failure = self
            .context
            .append_basic_block(function, "crypto.jwt.json.error");
        self.builder
            .build_conditional_branch(valid, success, failure)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(failure);
        for cleanup in cleanups {
            match cleanup {
                JwtCleanup::Handle(allocation, symbol) => {
                    let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                    let free = self.declare_libc_fn(symbol, &[pointer.into()], None);
                    self.builder
                        .build_call(free, &[(*allocation).into()], "")
                        .map_err(|error| error.to_string())?;
                }
                JwtCleanup::Text(value) => self.emit_string_release(*value)?,
            }
        }
        let set_error =
            self.declare_libc_fn("tyrus_crypto_set_invalid_jwt_claims_error", &[], None);
        self.builder
            .build_call(set_error, &[], "")
            .map_err(|error| error.to_string())?;
        let take_error = self.declare_libc_fn(
            "tyrus_crypto_take_error_result",
            &[self
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into()],
            None,
        );
        self.builder
            .build_call(take_error, &[result_slot.into()], "crypto.jwt.claims.error")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_unconditional_branch(merge)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(success);
        Ok(())
    }

    fn clear_crypto_runtime_error(&self) {
        let clear = self.declare_libc_fn("tyrus_crypto_clear_error", &[], None);
        self.builder
            .build_call(clear, &[], "crypto.error.clear")
            .unwrap();
    }

    fn guard_crypto_runtime_error(&mut self) -> Result<(), String> {
        let take =
            self.declare_libc_fn("tyrus_crypto_take_error", &[], Some(self.string_abi_type()));
        let message = self
            .builder
            .build_call(take, &[], "crypto.error.take")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_struct_value();
        let failed = self
            .builder
            .build_not(self.string_is_null(message)?, "crypto.error.present")
            .map_err(|error| error.to_string())?;
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or_else(|| "error: crypto call is not inside a function".to_string())?;
        let failure = self.context.append_basic_block(function, "crypto.error");
        let success = self.context.append_basic_block(function, "crypto.ok");
        self.builder
            .build_conditional_branch(failed, failure, success)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(failure);
        let error_type = Type::Named("Error".to_string());
        let error = self
            .builder
            .build_insert_value(
                self.llvm_type(&error_type).into_struct_type().const_zero(),
                message,
                0,
                "crypto.error.value",
            )
            .map_err(|error| error.to_string())?;
        let error = self
            .builder
            .build_insert_value(
                error,
                self.const_string_abi("ERR_CRYPTO", "crypto.error.code"),
                1,
                "crypto.error.value.code",
            )
            .map_err(|error| error.to_string())?
            .into_struct_value();
        let async_throw = self
            .async_results
            .last()
            .copied()
            .map(|promise| (promise, crate::AsyncRejection::Message(message.into())));
        self.emit_recoverable_runtime_error_cleanups()?;
        self.emit_throw_value_in_async(
            error.into(),
            super::statements::ThrownValue::ErrorMessage,
            function,
            async_throw,
            self.context.bool_type().const_zero(),
            &error_type,
        )?;

        self.builder.position_at_end(success);
        Ok(())
    }

    pub(super) fn emit_crypto_runtime_call(
        &mut self,
        target: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        if !crypto_target_returns_result(target) {
            self.clear_crypto_runtime_error();
        }

        let result = match target {
            tyrus_core::crypto::TIMING_SAFE_EQUAL_TARGET => {
                if args.len() != 2 {
                    return Err("error: `timingSafeEqual` expects 2 arguments".into());
                }
                let a = self.emit_bytes_as_ptr_len(&args[0])?;
                let b = self.emit_bytes_as_ptr_len(&args[1])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_crypto_timing_safe_equal",
                    &[
                        pointer.into(),
                        self.usize_type().into(),
                        pointer.into(),
                        self.usize_type().into(),
                    ],
                    Some(self.context.bool_type().into()),
                );
                let result = self
                    .builder
                    .build_call(
                        fn_val,
                        &[a.0.into(), a.1.into(), b.0.into(), b.1.into()],
                        "crypto.timing_safe",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                Ok(result)
            }
            tyrus_core::crypto::HMAC_SHA256_TARGET
            | tyrus_core::crypto::HMAC_SHA384_TARGET
            | tyrus_core::crypto::HMAC_SHA512_TARGET => {
                if args.len() != 2 {
                    return Err(format!("error: `{target}` expects 2 arguments"));
                }
                let data = self.emit_bytes_as_ptr_len(&args[0])?;
                let key = self.emit_bytes_as_ptr_len(&args[1])?;
                let c_fn_name = match target {
                    tyrus_core::crypto::HMAC_SHA256_TARGET => "tyrus_crypto_hmac_sha256_result",
                    tyrus_core::crypto::HMAC_SHA384_TARGET => "tyrus_crypto_hmac_sha384_result",
                    _ => "tyrus_crypto_hmac_sha512_result",
                };
                let result_type = Type::Result {
                    success: Box::new(Type::Array(Box::new(Type::U8))),
                    error: Box::new(Type::Named("CryptoError".to_string())),
                };
                let result_llvm_type = self.llvm_type(&result_type);
                let output = self
                    .builder
                    .build_alloca(result_llvm_type, "crypto.hmac.result")
                    .map_err(|error| error.to_string())?;
                let fn_val = self.declare_libc_fn(
                    c_fn_name,
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
                        fn_val,
                        &[
                            data.0.into(),
                            data.1.into(),
                            key.0.into(),
                            key.1.into(),
                            output.into(),
                        ],
                        "crypto.hmac",
                    )
                    .unwrap();
                let value = self
                    .builder
                    .build_load(result_llvm_type, output, "crypto.hmac.value")
                    .map_err(|error| error.to_string())?;
                self.record_owned_native_result(value, result_type);
                Ok(value)
            }
            tyrus_core::crypto::DIGEST_SHA256_TARGET
            | tyrus_core::crypto::DIGEST_SHA384_TARGET
            | tyrus_core::crypto::DIGEST_SHA512_TARGET
            | tyrus_core::crypto::DIGEST_MD5_TARGET => {
                if args.len() != 1 {
                    return Err(format!("error: `{target}` expects 1 argument"));
                }
                let data = self.emit_bytes_as_ptr_len(&args[0])?;
                let c_fn_name = match target {
                    tyrus_core::crypto::DIGEST_SHA256_TARGET => "tyrus_crypto_digest_sha256_result",
                    tyrus_core::crypto::DIGEST_SHA384_TARGET => "tyrus_crypto_digest_sha384_result",
                    tyrus_core::crypto::DIGEST_SHA512_TARGET => "tyrus_crypto_digest_sha512_result",
                    _ => "tyrus_crypto_digest_md5_result",
                };
                let result_type = Type::Result {
                    success: Box::new(Type::Array(Box::new(Type::U8))),
                    error: Box::new(Type::Named("CryptoError".to_string())),
                };
                let result_llvm_type = self.llvm_type(&result_type);
                let output = self
                    .builder
                    .build_alloca(result_llvm_type, "crypto.digest.result")
                    .map_err(|error| error.to_string())?;
                let fn_val = self.declare_libc_fn(
                    c_fn_name,
                    &[pointer.into(), self.usize_type().into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[data.0.into(), data.1.into(), output.into()],
                        "crypto.digest",
                    )
                    .unwrap();
                let value = self
                    .builder
                    .build_load(result_llvm_type, output, "crypto.digest.value")
                    .map_err(|error| error.to_string())?;
                self.record_owned_native_result(value, result_type);
                Ok(value)
            }
            tyrus_core::crypto::RAND_UUIDV7_TARGET
            | tyrus_core::crypto::RAND_UUIDV4_TARGET
            | tyrus_core::crypto::RAND_ULID_TARGET => {
                if target == tyrus_core::crypto::RAND_ULID_TARGET {
                    let result_type = Type::Result {
                        success: Box::new(Type::Str),
                        error: Box::new(Type::Named("CryptoError".to_string())),
                    };
                    let result_llvm_type = self.llvm_type(&result_type);
                    let output = self
                        .builder
                        .build_alloca(result_llvm_type, "crypto.ulid.result")
                        .map_err(|error| error.to_string())?;
                    let function = self.declare_libc_fn(
                        "tyrus_crypto_rand_ulid_result",
                        &[pointer.into()],
                        None,
                    );
                    self.builder
                        .build_call(function, &[output.into()], "crypto.ulid")
                        .map_err(|error| error.to_string())?;
                    let result = self
                        .builder
                        .build_load(result_llvm_type, output, "crypto.ulid.value")
                        .map_err(|error| error.to_string())?;
                    self.record_owned_native_result(result, result_type);
                    return Ok(result);
                }
                let c_fn_name = match target {
                    tyrus_core::crypto::RAND_UUIDV7_TARGET => "tyrus_crypto_rand_uuidv7",
                    tyrus_core::crypto::RAND_UUIDV4_TARGET => "tyrus_crypto_rand_uuidv4",
                    _ => "tyrus_crypto_rand_ulid",
                };
                let fn_val = self.declare_libc_fn(c_fn_name, &[], Some(self.string_abi_type()));
                let result = self
                    .builder
                    .build_call(fn_val, &[], "crypto.rand")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                Ok(result)
            }
            tyrus_core::crypto::RAND_RANDOM_BYTES_TARGET => {
                if args.len() != 1 {
                    return Err("error: `randomBytes` expects 1 argument".into());
                }
                let len = self.emit_expr_as(&args[0], &Type::U32)?;
                let result_type = Type::Result {
                    success: Box::new(Type::Array(Box::new(Type::U8))),
                    error: Box::new(Type::Named("CryptoError".to_string())),
                };
                let result_llvm_type = self.llvm_type(&result_type);
                let output = self
                    .builder
                    .build_alloca(result_llvm_type, "crypto.random_bytes.result")
                    .map_err(|error| error.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_crypto_rand_random_bytes_result",
                    &[self.context.i32_type().into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(fn_val, &[len.into(), output.into()], "crypto.random_bytes")
                    .unwrap();
                let result = self
                    .builder
                    .build_load(result_llvm_type, output, "crypto.random_bytes.value")
                    .map_err(|error| error.to_string())?;
                self.record_owned_native_result(result, result_type);
                return Ok(result);
            }
            tyrus_core::crypto::RAND_RANDOM_INT_TARGET => {
                if args.len() != 2 {
                    return Err("error: `randomInt` expects 2 arguments".into());
                }
                let min = self.emit_expr_as(&args[0], &Type::I64)?;
                let max = self.emit_expr_as(&args[1], &Type::I64)?;
                let result_type = Type::Result {
                    success: Box::new(Type::I64),
                    error: Box::new(Type::Named("CryptoError".to_string())),
                };
                let result_llvm_type = self.llvm_type(&result_type);
                let output = self
                    .builder
                    .build_alloca(result_llvm_type, "crypto.random_int.result")
                    .map_err(|error| error.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_crypto_rand_random_int_result",
                    &[
                        self.context.i64_type().into(),
                        self.context.i64_type().into(),
                        pointer.into(),
                    ],
                    None,
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[min.into(), max.into(), output.into()],
                        "crypto.random_int",
                    )
                    .unwrap();
                let result = self
                    .builder
                    .build_load(result_llvm_type, output, "crypto.random_int.value")
                    .map_err(|error| error.to_string())?;
                self.record_owned_native_result(result, result_type);
                return Ok(result);
            }
            tyrus_core::crypto::RAND_RANDOM_FLOAT_TARGET => {
                let fn_val = self.declare_libc_fn(
                    "tyrus_crypto_rand_random_float",
                    &[],
                    Some(self.context.f64_type().into()),
                );
                let result = self
                    .builder
                    .build_call(fn_val, &[], "crypto.random_float")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                Ok(result)
            }
            tyrus_core::crypto::CIPHER_AES_ENCRYPT_TARGET
            | tyrus_core::crypto::CIPHER_AES_DECRYPT_STR_TARGET
            | tyrus_core::crypto::CIPHER_AES_DECRYPT_BYTES_TARGET => {
                self.emit_crypto_cipher_call(target, args)
            }
            tyrus_core::crypto::ARGON2_HASH_TARGET
            | tyrus_core::crypto::ARGON2_VERIFY_TARGET
            | tyrus_core::crypto::ARGON2_NEEDS_REHASH_TARGET
            | tyrus_core::crypto::JWT_FETCH_JWKS_TARGET => {
                self.emit_crypto_token_call(target, args)
            }
            tyrus_core::crypto::DIGEST_CREATE_TARGET
            | tyrus_core::crypto::DIGEST_STREAM_UPDATE_TARGET
            | tyrus_core::crypto::DIGEST_STREAM_DIGEST_TARGET
            | tyrus_core::crypto::DIGEST_STREAM_RESET_TARGET
            | tyrus_core::crypto::DIGEST_STREAM_DISPOSE_TARGET => {
                self.emit_crypto_digest_stream_call(target, args)
            }
            _ => Err(format!("error: unknown crypto target `{target}`")),
        };
        if let Ok(value) = result {
            if crypto_target_returns_result(target) {
                return Ok(value);
            }
            self.guard_crypto_runtime_error()?;
            if matches!(
                target,
                tyrus_core::crypto::RAND_ULID_TARGET
                    | tyrus_core::crypto::RAND_RANDOM_BYTES_TARGET
                    | tyrus_core::crypto::RAND_RANDOM_INT_TARGET
                    | tyrus_core::crypto::HMAC_SHA256_TARGET
                    | tyrus_core::crypto::HMAC_SHA384_TARGET
                    | tyrus_core::crypto::HMAC_SHA512_TARGET
                    | tyrus_core::crypto::DIGEST_SHA256_TARGET
                    | tyrus_core::crypto::DIGEST_SHA384_TARGET
                    | tyrus_core::crypto::DIGEST_SHA512_TARGET
                    | tyrus_core::crypto::DIGEST_MD5_TARGET
            ) {
                return Ok(value);
            }
            if matches!(
                target,
                tyrus_core::crypto::CIPHER_AES_ENCRYPT_TARGET
                    | tyrus_core::crypto::CIPHER_AES_DECRYPT_STR_TARGET
                    | tyrus_core::crypto::CIPHER_AES_DECRYPT_BYTES_TARGET
            ) {
                return Ok(value);
            }
            let owned_type = match target {
                tyrus_core::crypto::HMAC_SHA256_TARGET
                | tyrus_core::crypto::HMAC_SHA384_TARGET
                | tyrus_core::crypto::HMAC_SHA512_TARGET
                | tyrus_core::crypto::DIGEST_SHA256_TARGET
                | tyrus_core::crypto::DIGEST_SHA384_TARGET
                | tyrus_core::crypto::DIGEST_SHA512_TARGET
                | tyrus_core::crypto::DIGEST_MD5_TARGET
                | tyrus_core::crypto::RAND_RANDOM_BYTES_TARGET
                | tyrus_core::crypto::CIPHER_AES_ENCRYPT_TARGET
                | tyrus_core::crypto::CIPHER_AES_DECRYPT_BYTES_TARGET => {
                    Some(Type::Array(Box::new(Type::U8)))
                }
                tyrus_core::crypto::RAND_UUIDV7_TARGET
                | tyrus_core::crypto::RAND_UUIDV4_TARGET
                | tyrus_core::crypto::RAND_ULID_TARGET
                | tyrus_core::crypto::CIPHER_AES_DECRYPT_STR_TARGET => Some(Type::Str),
                _ => None,
            };
            if let Some(ty) = owned_type {
                self.record_owned_native_result(value, ty);
            }
        }
        result
    }

    pub(super) fn crypto_call_target(&self, object: &Expr, method: &str) -> Option<&'static str> {
        let obj_ty = self.expr_type_hint(object)?;
        let Type::Named(ty_name) = &obj_ty else {
            return None;
        };
        if tyrus_core::crypto::is_crypto_namespace(ty_name) {
            let import_name =
                tyrus_core::crypto::crypto_type_to_import_name(ty_name).unwrap_or(ty_name);
            if let Some(target) = tyrus_core::crypto::crypto_method_target(import_name, method) {
                return Some(target);
            }
        }
        if let ExprKind::FieldAccess {
            object: inner_obj,
            field: inner_field,
        } = object.kind()
        {
            let inner_ty_name = match self.expr_type_hint(inner_obj) {
                Some(Type::Named(name)) => name,
                _ => return None,
            };
            if tyrus_core::crypto::is_crypto_namespace(&inner_ty_name) {
                let inner_ns_name =
                    tyrus_core::crypto::nested_crypto_inner_type_name(&inner_ty_name, inner_field)?;
                return tyrus_core::crypto::nested_crypto_method_target(inner_ns_name, method);
            }
        }
        None
    }

    pub(super) fn owned_crypto_claims_call(&self, expression: &Expr) -> bool {
        let ExprKind::MethodCall { object, method, .. } = expression.kind() else {
            return false;
        };
        self.crypto_call_target(object, method)
            .is_some_and(tyrus_core::crypto::method_returns_owned_claims)
    }

    fn emit_string_arg(
        &mut self,
        arg: &Expr,
    ) -> Result<inkwell::values::BasicValueEnum<'ctx>, String> {
        self.emit_expr_as(arg, &Type::Str)
    }

    fn emit_crypto_options_pointer(
        &mut self,
        argument: Option<&Expr>,
        options_type: &Type,
        name: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let Some(argument) = argument else {
            return Ok(pointer.const_null());
        };
        let value = if let (ExprKind::ObjectLit(values), Type::Object(fields)) =
            (argument.kind(), transparent_codegen_type(options_type))
        {
            let struct_type = self.llvm_type(options_type).into_struct_type();
            let mut object = struct_type.get_undef();
            for (index, field) in fields.iter().enumerate() {
                let optional = field_storage_type(field);
                let Type::Optional(inner) = transparent_codegen_type(&optional) else {
                    return Err(format!(
                        "internal: crypto option `{}` must be optional",
                        field.name
                    ));
                };
                let field_pointer = if let Some((_, expression)) = values
                    .iter()
                    .find(|(candidate, _)| candidate == &field.name)
                {
                    let field_value = self.emit_crypto_option_value(expression, inner)?;
                    let field_slot = self
                        .builder
                        .build_alloca(self.llvm_type(inner), &format!("{name}.{}", field.name))
                        .map_err(|error| error.to_string())?;
                    self.builder
                        .build_store(field_slot, field_value)
                        .map_err(|error| error.to_string())?;
                    field_slot
                } else {
                    pointer.const_null()
                };
                object = self
                    .builder
                    .build_insert_value(
                        object,
                        field_pointer,
                        index as u32,
                        &format!("{name}.{}.optional", field.name),
                    )
                    .map_err(|error| error.to_string())?
                    .into_struct_value();
            }
            object.into()
        } else {
            self.emit_expr_as(argument, options_type)?
        };
        let slot = self
            .builder
            .build_alloca(self.llvm_type(options_type), name)
            .unwrap();
        self.builder.build_store(slot, value).unwrap();
        Ok(slot)
    }

    fn emit_crypto_option_value(
        &mut self,
        expression: &Expr,
        expected: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let Type::Array(element) = transparent_codegen_type(expected) else {
            return self.emit_expr_as(expression, expected);
        };
        let ExprKind::ArrayLit(values) = expression.kind() else {
            return self.emit_expr_as(expression, expected);
        };
        let array_type = self.llvm_type(expected).into_struct_type();
        let mut array = array_type.const_zero();
        if values.is_empty() {
            return Ok(array.into());
        }
        let element_type = self.llvm_type(element);
        let storage_type = element_type.array_type(values.len() as u32);
        let storage = self
            .builder
            .build_alloca(storage_type, "crypto.options.array")
            .map_err(|error| error.to_string())?;
        for (index, value) in values.iter().enumerate() {
            let slot = unsafe {
                self.builder
                    .build_gep(
                        storage_type,
                        storage,
                        &[
                            self.context.i32_type().const_zero(),
                            self.context.i32_type().const_int(index as u64, false),
                        ],
                        "crypto.options.array.element",
                    )
                    .map_err(|error| error.to_string())?
            };
            let value = self.emit_expr_as(value, element)?;
            self.builder
                .build_store(slot, value)
                .map_err(|error| error.to_string())?;
        }
        let length = self.usize_type().const_int(values.len() as u64, false);
        array = self
            .builder
            .build_insert_value(array, storage, 0, "crypto.options.array.data")
            .map_err(|error| error.to_string())?
            .into_struct_value();
        array = self
            .builder
            .build_insert_value(array, length, 1, "crypto.options.array.length")
            .map_err(|error| error.to_string())?
            .into_struct_value();
        Ok(self
            .builder
            .build_insert_value(array, length, 2, "crypto.options.array.capacity")
            .map_err(|error| error.to_string())?
            .into_struct_value()
            .into())
    }

    fn emit_u8_array_as_ptr_len(
        &mut self,
        arg: &Expr,
    ) -> Result<
        (
            inkwell::values::BasicValueEnum<'ctx>,
            inkwell::values::BasicValueEnum<'ctx>,
        ),
        String,
    > {
        let arr = self
            .emit_expr_as(arg, &Type::Array(Box::new(Type::U8)))?
            .into_struct_value();
        let data_ptr = self
            .builder
            .build_extract_value(arr, 0, "crypto.arr.ptr")
            .unwrap()
            .into_pointer_value();
        let len = self
            .builder
            .build_extract_value(arr, 1, "crypto.arr.len")
            .unwrap()
            .into_int_value();
        Ok((data_ptr.into(), len.into()))
    }

    fn emit_bytes_as_ptr_len(
        &mut self,
        arg: &Expr,
    ) -> Result<(PointerValue<'ctx>, IntValue<'ctx>), String> {
        if matches!(
            arg.kind(),
            ExprKind::ArrayLit(_) | ExprKind::ArrayRepeat { .. }
        ) {
            let (pointer, length) = self.emit_u8_array_as_ptr_len(arg)?;
            return Ok((pointer.into_pointer_value(), length.into_int_value()));
        }
        let ty = self
            .expr_type_hint(arg)
            .or_else(|| match arg.kind() {
                ExprKind::MethodCall { object, method, .. }
                    if matches!(object.kind(), ExprKind::Ident(name) if name == "utf8")
                        && method == "encode" =>
                {
                    Some(Type::Array(Box::new(Type::U8)))
                }
                _ => None,
            })
            .ok_or_else(|| {
                format!(
                    "error: cannot infer crypto byte input type for `{:?}`",
                    arg.kind()
                )
            })?;
        match transparent_codegen_type(&ty) {
            Type::Str => {
                let value = self.emit_expr_as(arg, &Type::Str)?.into_struct_value();
                Ok((self.string_data_ptr(value)?, self.string_byte_len(value)?))
            }
            Type::Array(element) if matches!(element.as_ref(), Type::U8) => {
                let (pointer, length) = self.emit_u8_array_as_ptr_len(arg)?;
                Ok((pointer.into_pointer_value(), length.into_int_value()))
            }
            other => Err(format!(
                "error: crypto byte input must be `string` or `u8[]`, found `{other}`"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::crypto_target_returns_result;

    #[test]
    fn fallible_crypto_targets_bypass_legacy_throw_guard() {
        assert!(crypto_target_returns_result(
            tyrus_core::crypto::DIGEST_SHA256_TARGET
        ));
        assert!(crypto_target_returns_result(
            tyrus_core::crypto::ARGON2_VERIFY_TARGET
        ));
        assert!(crypto_target_returns_result(
            tyrus_core::crypto::DIGEST_STREAM_UPDATE_TARGET
        ));
        assert!(!crypto_target_returns_result(
            tyrus_core::crypto::TIMING_SAFE_EQUAL_TARGET
        ));
        assert!(!crypto_target_returns_result(
            tyrus_core::crypto::RAND_UUIDV4_TARGET
        ));
    }
}
