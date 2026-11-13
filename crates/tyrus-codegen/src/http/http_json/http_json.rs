use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_http_schema_request_from_json(
        &mut self,
        name: &str,
        body: StructValue<'ctx>,
        response_pointer: PointerValue<'ctx>,
        channel: &str,
    ) -> Result<StructValue<'ctx>, String> {
        let schema_type = Type::Named(name.to_string());
        let invalid_message = http_invalid_channel_message(channel);
        let fields = self
            .declared_fields_with_index(&schema_type)
            .ok_or_else(|| format!("error: unknown HTTP request schema `{name}`"))?;
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let status_slot = self.entry_alloca(self.context.i32_type(), "http.json.status")?;
        let parse = self.declare_libc_fn(
            if channel == "body" {
                "tyrus_json_document_parse"
            } else {
                "tyrus_json_document_parse_text"
            },
            &[self.string_abi_type(), pointer.into()],
            Some(pointer.into()),
        );
        let document = self
            .builder
            .build_call(
                parse,
                &[body.into(), status_slot.into()],
                "http.json.document",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: HTTP JSON document parser did not return a handle")?
            .into_pointer_value();
        let parsed = self
            .builder
            .build_is_not_null(document, "http.json.parsed")
            .map_err(|error| error.to_string())?;
        self.emit_http_validation_response_if_invalid(
            parsed,
            response_pointer,
            400,
            &invalid_message,
        );
        if channel == "body" {
            let known_fields = fields
                .iter()
                .map(|(_, field)| field.name.as_str())
                .collect::<Vec<_>>()
                .join(",");
            let known_fields = self.const_string_abi(&known_fields, "http.json.known_fields");
            let only_known_fields = self.declare_libc_fn(
                "tyrus_json_document_has_only_fields",
                &[pointer.into(), self.string_abi_type()],
                Some(self.context.bool_type().into()),
            );
            let known = self
                .builder
                .build_call(
                    only_known_fields,
                    &[document.into(), known_fields.into()],
                    "http.json.known_fields.valid",
                )
                .map_err(|error| error.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or("internal: HTTP JSON known-fields checker did not return a value")?
                .into_int_value();
            self.emit_http_json_document_validation_response_if_invalid(
                known,
                response_pointer,
                document,
                &invalid_message,
            );
        }
        let result_slot = self.entry_alloca(self.llvm_type(&schema_type), "http.json.result")?;
        self.builder
            .build_store(result_slot, self.llvm_type(&schema_type).const_zero())
            .map_err(|error| error.to_string())?;
        let mut result = self.llvm_type(&schema_type).into_struct_type().const_zero();
        for (index, field) in &fields {
            let storage_type = field_storage_type(field);
            let value = self.emit_http_json_field(
                document,
                response_pointer,
                result_slot,
                &schema_type,
                &field.name,
                &storage_type,
                field.optional,
                &invalid_message,
            )?;
            result = self
                .builder
                .build_insert_value(result, value, *index, &field.name)
                .map_err(|error| error.to_string())?
                .into_struct_value();
            self.builder
                .build_store(result_slot, result)
                .map_err(|error| error.to_string())?;
        }
        let free = self.declare_libc_fn("tyrus_json_document_free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[document.into()], "http.json.document.free")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_load(self.llvm_type(&schema_type), result_slot, "http.json.value")
            .map_err(|error| error.to_string())
            .map(|value| value.into_struct_value())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_http_json_field(
        &mut self,
        document: PointerValue<'ctx>,
        response_pointer: PointerValue<'ctx>,
        result_slot: PointerValue<'ctx>,
        schema_type: &Type,
        field: &str,
        ty: &Type,
        optional: bool,
        invalid_message: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if let Type::Optional(inner) = transparent_codegen_type(ty).clone() {
            return self.emit_http_json_optional_field(
                document,
                response_pointer,
                result_slot,
                schema_type,
                field,
                &inner,
                invalid_message,
            );
        }
        if optional {
            return Err(format!(
                "internal: HTTP request schema field `{field}` is optional but its storage type is not"
            ));
        }

        let llvm_type = self.llvm_type(ty);
        let out = self
            .builder
            .build_alloca(llvm_type, "http.json.out")
            .unwrap();
        self.builder
            .build_store(out, llvm_type.const_zero())
            .map_err(|error| error.to_string())?;
        let key = self
            .const_string_abi(field, "http.json.key")
            .into_struct_value();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_type = self.context.i32_type();
        let runtime = match transparent_codegen_type(ty) {
            Type::Array(element) => {
                let (out, status) =
                    self.emit_automatic_json_array_field(document, field, element)?;
                let value = self
                    .builder
                    .build_load(llvm_type, out, "http.json.array.value")
                    .map_err(|error| error.to_string())?;
                self.emit_http_json_status_check(
                    status,
                    response_pointer,
                    document,
                    result_slot,
                    schema_type,
                    Some((value, ty)),
                    invalid_message,
                )?;
                return Ok(value);
            }
            Type::Named(name) if self.http_schema_enum_repr(name).is_some() => {
                let repr = self
                    .http_schema_enum_repr(name)
                    .expect("the guard above proved the repr exists");
                let value = self.emit_http_json_enum_field(
                    document,
                    response_pointer,
                    result_slot,
                    schema_type,
                    field,
                    name,
                    &repr,
                    invalid_message,
                )?;
                return Ok(value);
            }
            Type::Union(variants) => {
                let variants = variants.clone();
                let (out, status) =
                    self.emit_automatic_json_union_field(document, field, &variants)?;
                let value = self
                    .builder
                    .build_load(llvm_type, out, "http.json.union.value")
                    .map_err(|error| error.to_string())?;
                self.emit_http_json_status_check(
                    status,
                    response_pointer,
                    document,
                    result_slot,
                    schema_type,
                    Some((value, ty)),
                    invalid_message,
                )?;
                return Ok(value);
            }
            Type::Named(name) => {
                let (out, status) = self.emit_automatic_json_nested_field(document, field, name)?;
                let value = self
                    .builder
                    .build_load(llvm_type, out, "http.json.object.value")
                    .map_err(|error| error.to_string())?;
                self.emit_http_json_status_check(
                    status,
                    response_pointer,
                    document,
                    result_slot,
                    schema_type,
                    Some((value, ty)),
                    invalid_message,
                )?;
                return Ok(value);
            }
            other => match http_json_scalar_getter(other) {
                Some(runtime) => runtime,
                None => {
                    return Err(format!(
                        "error: HTTP JSON schema field `{field}` uses unsupported type `{other}`"
                    ));
                }
            },
        };
        let function = self.declare_libc_fn(
            runtime,
            &[pointer.into(), self.string_abi_type(), pointer.into()],
            Some(i32_type.into()),
        );
        let status = self
            .builder
            .build_call(
                function,
                &[document.into(), key.into(), out.into()],
                "http.json.status",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        let value = self
            .builder
            .build_load(llvm_type, out, "http.json.value")
            .map_err(|error| error.to_string())?;
        self.emit_http_json_status_check(
            status,
            response_pointer,
            document,
            result_slot,
            schema_type,
            Some((value, ty)),
            invalid_message,
        )?;
        Ok(value)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_http_json_enum_field(
        &mut self,
        document: PointerValue<'ctx>,
        response_pointer: PointerValue<'ctx>,
        result_slot: PointerValue<'ctx>,
        schema_type: &Type,
        field: &str,
        enum_name: &str,
        repr: &Type,
        invalid_message: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let Some(runtime) = http_json_scalar_getter(repr) else {
            return Err(format!(
                "internal: enum `{enum_name}` has no JSON reader for its `{repr}` representation"
            ));
        };
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_type = self.context.i32_type();
        let llvm_type = self.llvm_type(repr);
        let out = self
            .builder
            .build_alloca(llvm_type, "http.json.enum.out")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(out, llvm_type.const_zero())
            .map_err(|error| error.to_string())?;
        let key = self
            .const_string_abi(field, "http.json.key")
            .into_struct_value();
        let function = self.declare_libc_fn(
            runtime,
            &[pointer.into(), self.string_abi_type(), pointer.into()],
            Some(i32_type.into()),
        );
        let status = self
            .builder
            .build_call(
                function,
                &[document.into(), key.into(), out.into()],
                "http.json.enum.status",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        let value = self
            .builder
            .build_load(llvm_type, out, "http.json.enum.value")
            .map_err(|error| error.to_string())?;
        let members = self
            .enums
            .get(enum_name)
            .map(|info| info.members.clone())
            .unwrap_or_default();
        let mut is_member = self.context.bool_type().const_zero();
        for (member, literal) in &members {
            let declared = self.emit_expr_as(literal, repr).map_err(|error| {
                format!("internal: enum `{enum_name}` member `{member}`: {error}")
            })?;
            let matches = if matches!(repr, Type::Str) {
                self.emit_string_equals(
                    value.into_struct_value(),
                    declared.into_struct_value(),
                    "http.json.enum.member",
                )?
            } else if matches!(repr, Type::F32 | Type::F64) {
                self.builder
                    .build_float_compare(
                        inkwell::FloatPredicate::OEQ,
                        value.into_float_value(),
                        declared.into_float_value(),
                        "http.json.enum.member",
                    )
                    .map_err(|error| error.to_string())?
            } else {
                self.builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        value.into_int_value(),
                        declared.into_int_value(),
                        "http.json.enum.member",
                    )
                    .map_err(|error| error.to_string())?
            };
            is_member = self
                .builder
                .build_or(is_member, matches, "http.json.enum.any")
                .map_err(|error| error.to_string())?;
        }
        let membership_status = self
            .builder
            .build_select(
                is_member,
                i32_type.const_zero(),
                i32_type.const_int(1, false),
                "http.json.enum.membership",
            )
            .map_err(|error| error.to_string())?
            .into_int_value();
        let read_failed = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                status,
                i32_type.const_zero(),
                "http.json.enum.read_failed",
            )
            .map_err(|error| error.to_string())?;
        let combined = self
            .builder
            .build_select(
                read_failed,
                status,
                membership_status,
                "http.json.enum.check",
            )
            .map_err(|error| error.to_string())?
            .into_int_value();
        self.emit_http_json_status_check(
            combined,
            response_pointer,
            document,
            result_slot,
            schema_type,
            Some((value, repr)),
            invalid_message,
        )?;
        Ok(value)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_http_json_optional_field(
        &mut self,
        document: PointerValue<'ctx>,
        response_pointer: PointerValue<'ctx>,
        result_slot: PointerValue<'ctx>,
        schema_type: &Type,
        field: &str,
        inner: &Type,
        invalid_message: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let Some(runtime) = http_json_scalar_getter(transparent_codegen_type(inner)) else {
            return Err(format!(
                "error: optional HTTP request schema field `{field}` uses unsupported type `{inner}`"
            ));
        };
        let current_function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: HTTP optional field emitted outside function")?;
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_type = self.context.i32_type();
        let inner_llvm = self.llvm_type(inner);
        let out = self
            .builder
            .build_alloca(inner_llvm, "http.json.optional.out")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(out, inner_llvm.const_zero())
            .map_err(|error| error.to_string())?;
        let key = self
            .const_string_abi(field, "http.json.optional.key")
            .into_struct_value();
        let getter = self.declare_libc_fn(
            runtime,
            &[pointer.into(), self.string_abi_type(), pointer.into()],
            Some(i32_type.into()),
        );
        let status = self
            .builder
            .build_call(
                getter,
                &[document.into(), key.into(), out.into()],
                "http.json.optional.status",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: HTTP JSON getter did not return a status")?
            .into_int_value();

        let present_bb = self
            .context
            .append_basic_block(current_function, "http.json.optional.present");
        let missing_check_bb = self
            .context
            .append_basic_block(current_function, "http.json.optional.check_missing");
        let absent_bb = self
            .context
            .append_basic_block(current_function, "http.json.optional.absent");
        let invalid_bb = self
            .context
            .append_basic_block(current_function, "http.json.optional.invalid");
        let join_bb = self
            .context
            .append_basic_block(current_function, "http.json.optional.done");

        let is_present = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                status,
                i32_type.const_zero(),
                "http.json.optional.is_present",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(is_present, present_bb, missing_check_bb)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(missing_check_bb);
        let is_absent = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                status,
                i32_type.const_int(1, false),
                "http.json.optional.is_absent",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(is_absent, absent_bb, invalid_bb)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(invalid_bb);
        self.emit_http_json_rejection(
            document,
            response_pointer,
            result_slot,
            schema_type,
            None,
            invalid_message,
        )?;

        self.builder.position_at_end(present_bb);
        let value = self
            .builder
            .build_load(inner_llvm, out, "http.json.optional.value")
            .map_err(|error| error.to_string())?;
        let boxed = self.emit_optional_box(value, inner)?;
        self.builder
            .build_unconditional_branch(join_bb)
            .map_err(|error| error.to_string())?;
        let present_end = self.builder.get_insert_block().unwrap();

        self.builder.position_at_end(absent_bb);
        self.builder
            .build_unconditional_branch(join_bb)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(join_bb);
        let phi = self
            .builder
            .build_phi(pointer, "http.json.optional.result")
            .map_err(|error| error.to_string())?;
        phi.add_incoming(&[(&boxed, present_end), (&pointer.const_null(), absent_bb)]);
        Ok(phi.as_basic_value())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_http_json_status_check(
        &mut self,
        status: IntValue<'ctx>,
        response_pointer: PointerValue<'ctx>,
        document: PointerValue<'ctx>,
        result_slot: PointerValue<'ctx>,
        schema_type: &Type,
        current_value: Option<(BasicValueEnum<'ctx>, &Type)>,
        invalid_message: &str,
    ) -> Result<(), String> {
        let current_function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .expect("HTTP JSON status check emitted outside function");
        let ok_bb = self
            .context
            .append_basic_block(current_function, "http.json.ok");
        let invalid_bb = self
            .context
            .append_basic_block(current_function, "http.json.invalid");
        let i32_type = self.context.i32_type();
        let ok = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                status,
                i32_type.const_zero(),
                "http.json.is_ok",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(ok, ok_bb, invalid_bb)
            .unwrap();
        self.builder.position_at_end(invalid_bb);
        self.emit_http_json_rejection(
            document,
            response_pointer,
            result_slot,
            schema_type,
            current_value,
            invalid_message,
        )?;
        self.builder.position_at_end(ok_bb);
        Ok(())
    }

    pub(super) fn emit_http_json_rejection(
        &mut self,
        document: PointerValue<'ctx>,
        response_pointer: PointerValue<'ctx>,
        result_slot: PointerValue<'ctx>,
        schema_type: &Type,
        current_value: Option<(BasicValueEnum<'ctx>, &Type)>,
        invalid_message: &str,
    ) -> Result<(), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let result = self
            .builder
            .build_load(
                self.llvm_type(schema_type),
                result_slot,
                "http.json.failed.value",
            )
            .map_err(|error| error.to_string())?;
        self.emit_owned_native_release(result, schema_type, &ReleasePlan::Deep);
        if let Some((value, ty)) = current_value {
            self.emit_owned_native_release(value, ty, &ReleasePlan::Deep);
        }
        let free = self.declare_libc_fn("tyrus_json_document_free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[document.into()], "http.json.document.free.invalid")
            .map_err(|error| error.to_string())?;
        let function = self.declare_libc_fn(
            "tyrus_http_set_validation_error",
            &[
                pointer.into(),
                self.context.i32_type().into(),
                self.string_abi_type(),
            ],
            None,
        );
        let message = self
            .const_string_abi(invalid_message, "http.json.invalid.message")
            .into_struct_value();
        self.builder
            .build_call(
                function,
                &[
                    response_pointer.into(),
                    self.context.i32_type().const_int(400, false).into(),
                    message.into(),
                ],
                "http.json.invalid.response",
            )
            .unwrap();
        self.builder
            .build_return(Some(&pointer.const_null()))
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub(super) fn emit_http_json_document_validation_response_if_invalid(
        &mut self,
        valid: IntValue<'ctx>,
        response_pointer: PointerValue<'ctx>,
        document: PointerValue<'ctx>,
        invalid_message: &str,
    ) {
        let current_function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .expect("HTTP JSON validation response emitted outside function");
        let invalid_bb = self
            .context
            .append_basic_block(current_function, "http.json.document.invalid");
        let ok_bb = self
            .context
            .append_basic_block(current_function, "http.json.document.ok");
        self.builder
            .build_conditional_branch(valid, ok_bb, invalid_bb)
            .unwrap();
        self.builder.position_at_end(invalid_bb);
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let free = self.declare_libc_fn("tyrus_json_document_free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[document.into()], "http.json.document.free.unknown")
            .unwrap();
        let function = self.declare_libc_fn(
            "tyrus_http_set_validation_error",
            &[
                pointer.into(),
                self.context.i32_type().into(),
                self.string_abi_type(),
            ],
            None,
        );
        let message = self
            .const_string_abi(invalid_message, "http.json.invalid.message")
            .into_struct_value();
        self.builder
            .build_call(
                function,
                &[
                    response_pointer.into(),
                    self.context.i32_type().const_int(400, false).into(),
                    message.into(),
                ],
                "http.json.invalid.response",
            )
            .unwrap();
        self.builder
            .build_return(Some(&pointer.const_null()))
            .unwrap();
        self.builder.position_at_end(ok_bb);
    }

    pub(super) fn emit_http_schema_response_json(
        &mut self,
        name: &str,
        value: StructValue<'ctx>,
    ) -> Result<StructValue<'ctx>, String> {
        let body = self.emit_automatic_json_encode(value, &Type::Named(name.to_string()), false)?;
        let valid_body = self
            .builder
            .build_not(self.string_is_null(body)?, "http.json.body.valid")
            .map_err(|error| error.to_string())?;
        self.emit_http_response_error_if_invalid(
            valid_body,
            self.builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_parent)
                .and_then(|function| function.get_nth_param(1))
                .unwrap()
                .into_pointer_value(),
            500,
            "the response body could not be serialized as JSON",
        );
        Ok(body)
    }

    pub(crate) fn emit_http_json_builder_add(
        &mut self,
        builder: PointerValue<'ctx>,
        field: &str,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
    ) -> Result<(), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let key = self
            .const_string_abi(field, "http.json.key")
            .into_struct_value();
        if let Type::Array(element) = transparent_codegen_type(ty) {
            let text = self.emit_automatic_json_array_encode(value.into_struct_value(), element)?;
            let add = self.declare_libc_fn(
                "tyrus_json_builder_add_raw",
                &[
                    pointer.into(),
                    self.string_abi_type(),
                    self.string_abi_type(),
                ],
                Some(self.context.i32_type().into()),
            );
            self.builder
                .build_call(
                    add,
                    &[builder.into(), key.into(), text.into()],
                    "http.json.add.array",
                )
                .map_err(|error| error.to_string())?;
            self.emit_string_release(text)?;
            return Ok(());
        }
        if let Type::Named(name) = transparent_codegen_type(ty)
            && let Some(repr) = self.http_schema_enum_repr(name)
        {
            return self.emit_http_json_builder_add(builder, field, value, &repr);
        }
        if matches!(
            transparent_codegen_type(ty),
            Type::Union(_) | Type::Object(_) | Type::Lit(_)
        ) {
            return self.emit_automatic_json_builder_add(builder, field, value, ty);
        }
        if matches!(transparent_codegen_type(ty), Type::Named(_)) {
            let text = self.emit_automatic_json_encode(value.into_struct_value(), ty, false)?;
            let add = self.declare_libc_fn(
                "tyrus_json_builder_add_raw",
                &[
                    pointer.into(),
                    self.string_abi_type(),
                    self.string_abi_type(),
                ],
                Some(self.context.i32_type().into()),
            );
            self.builder
                .build_call(
                    add,
                    &[builder.into(), key.into(), text.into()],
                    "http.json.add.object",
                )
                .map_err(|error| error.to_string())?;
            self.emit_string_release(text)?;
            return Ok(());
        }
        let (runtime, value): (&str, BasicMetadataValueEnum<'ctx>) = match transparent_codegen_type(
            ty,
        ) {
            Type::Str => (
                "tyrus_json_builder_add_string",
                value.into_struct_value().into(),
            ),
            Type::Bool => ("tyrus_json_builder_add_bool", value.into_int_value().into()),
            Type::F32 => {
                let widened = self
                    .builder
                    .build_float_ext(
                        value.into_float_value(),
                        self.context.f64_type(),
                        "http.json.f64",
                    )
                    .map_err(|error| error.to_string())?;
                ("tyrus_json_builder_add_f64", widened.into())
            }
            Type::F64 => (
                "tyrus_json_builder_add_f64",
                value.into_float_value().into(),
            ),
            Type::Decimal => {
                let decimal = self
                    .builder
                    .build_alloca(self.llvm_type(&Type::Decimal), "http.json.decimal")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(decimal, value)
                    .map_err(|error| error.to_string())?;
                ("tyrus_json_builder_add_decimal", decimal.into())
            }
            Type::I32 | Type::Isize => {
                let widened = self
                    .builder
                    .build_int_s_extend(
                        value.into_int_value(),
                        self.context.i64_type(),
                        "http.json.i64",
                    )
                    .map_err(|error| error.to_string())?;
                ("tyrus_json_builder_add_i64", widened.into())
            }
            Type::I64 => ("tyrus_json_builder_add_i64", value.into_int_value().into()),
            Type::U32 | Type::Usize => {
                let widened = self
                    .builder
                    .build_int_z_extend(
                        value.into_int_value(),
                        self.context.i64_type(),
                        "http.json.u64",
                    )
                    .map_err(|error| error.to_string())?;
                ("tyrus_json_builder_add_u64", widened.into())
            }
            Type::U64 => ("tyrus_json_builder_add_u64", value.into_int_value().into()),
            other => {
                return Err(format!(
                    "error: HTTP response schema field `{field}` uses unsupported type `{other}`"
                ));
            }
        };
        let i32_type = self.context.i32_type();
        let value_type = match transparent_codegen_type(ty) {
            Type::Str => self.string_abi_type(),
            Type::Bool => self.context.bool_type().into(),
            Type::F32 | Type::F64 => self.context.f64_type().into(),
            Type::Decimal => pointer.into(),
            Type::I32 | Type::I64 | Type::Isize | Type::U32 | Type::U64 | Type::Usize => {
                self.context.i64_type().into()
            }
            _ => unreachable!(),
        };
        let function = self.declare_libc_fn(
            runtime,
            &[pointer.into(), self.string_abi_type(), value_type],
            Some(i32_type.into()),
        );
        self.builder
            .build_call(
                function,
                &[builder.into(), key.into(), value],
                "http.json.add",
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}
