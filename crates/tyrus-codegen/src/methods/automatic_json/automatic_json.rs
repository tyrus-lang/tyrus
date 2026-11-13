use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(crate) fn emit_automatic_json_value_encode(
        &mut self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
    ) -> Result<StructValue<'ctx>, String> {
        let ty = self.resolve_type(ty);
        if let Type::Named(name) = transparent_codegen_type(&ty)
            && let Some(repr) = self.http_schema_enum_repr(name)
        {
            return self.emit_automatic_json_value_encode(value, &repr);
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let create =
            self.declare_libc_fn("tyrus_json_array_builder_new", &[], Some(pointer.into()));
        let builder = self
            .builder
            .build_call(create, &[], "automatic.json.value.builder")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: automatic JSON value builder did not return a handle")?
            .into_pointer_value();
        self.emit_automatic_json_array_add(builder, value, &ty)?;
        let finish = self.declare_libc_fn(
            "tyrus_json_array_builder_finish_value",
            &[pointer.into()],
            Some(self.string_abi_type()),
        );
        let text = self
            .builder
            .build_call(finish, &[builder.into()], "automatic.json.value.finish")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: automatic JSON value encoder did not return text")?
            .into_struct_value();
        let free = self.declare_libc_fn("tyrus_json_array_builder_free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[builder.into()], "automatic.json.value.builder.free")
            .map_err(|error| error.to_string())?;
        Ok(text)
    }

    pub(crate) fn emit_automatic_json_encode(
        &mut self,
        value: StructValue<'ctx>,
        ty: &Type,
        pretty: bool,
    ) -> Result<StructValue<'ctx>, String> {
        if matches!(transparent_codegen_type(ty), Type::Named(name) if name == "Instant") {
            return self.emit_automatic_json_value_encode(value.into(), ty);
        }
        let fields = self
            .declared_fields_with_index(ty)
            .ok_or_else(|| format!("error: `{ty}` has no fields to encode as a JSON object"))?;
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let create = self.declare_libc_fn("tyrus_json_builder_new", &[], Some(pointer.into()));
        let builder = self
            .builder
            .build_call(create, &[], "automatic.json.builder")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: automatic JSON builder did not return a handle")?
            .into_pointer_value();
        for (index, field) in fields {
            let field_type = field_storage_type(&field);
            let field_value = self
                .builder
                .build_extract_value(value, index, "automatic.json.field")
                .map_err(|error| error.to_string())?;
            if let Type::Optional(inner) = transparent_codegen_type(&field_type) {
                let optional = field_value.into_pointer_value();
                let function = self
                    .builder
                    .get_insert_block()
                    .and_then(inkwell::basic_block::BasicBlock::get_parent)
                    .ok_or("internal: automatic JSON encoding outside a function")?;
                let present = self
                    .context
                    .append_basic_block(function, "automatic.json.optional.encode");
                let next = self
                    .context
                    .append_basic_block(function, "automatic.json.optional.skip");
                let has_value = self
                    .builder
                    .build_is_not_null(optional, "automatic.json.optional.has_value")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_conditional_branch(has_value, present, next)
                    .map_err(|error| error.to_string())?;
                self.builder.position_at_end(present);
                let inner_value = self
                    .builder
                    .build_load(
                        self.llvm_type(inner),
                        optional,
                        "automatic.json.optional.inner",
                    )
                    .map_err(|error| error.to_string())?;
                self.emit_automatic_json_builder_add(builder, &field.name, inner_value, inner)?;
                self.builder
                    .build_unconditional_branch(next)
                    .map_err(|error| error.to_string())?;
                self.builder.position_at_end(next);
                continue;
            }
            self.emit_automatic_json_builder_add(builder, &field.name, field_value, &field_type)?;
        }
        let finish = self.declare_libc_fn(
            if pretty {
                "tyrus_json_builder_finish_pretty"
            } else {
                "tyrus_json_builder_finish"
            },
            &[pointer.into()],
            Some(self.string_abi_type()),
        );
        let text = self
            .builder
            .build_call(finish, &[builder.into()], "automatic.json.finish")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: automatic JSON builder did not return text")?
            .into_struct_value();
        let free = self.declare_libc_fn("tyrus_json_builder_free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[builder.into()], "automatic.json.builder.free")
            .map_err(|error| error.to_string())?;
        Ok(text)
    }

    pub(crate) fn emit_automatic_json_builder_add(
        &mut self,
        builder: PointerValue<'ctx>,
        field: &str,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
    ) -> Result<(), String> {
        let ty = &self.resolve_type(ty);
        if let Type::Named(name) = transparent_codegen_type(ty)
            && let Some(repr) = self.http_schema_enum_repr(name)
        {
            return self.emit_automatic_json_builder_add(builder, field, value, &repr);
        }
        match transparent_codegen_type(ty) {
            Type::Array(element) => {
                let text =
                    self.emit_automatic_json_array_encode(value.into_struct_value(), element)?;
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let add = self.declare_libc_fn(
                    "tyrus_json_builder_add_raw",
                    &[
                        pointer.into(),
                        self.string_abi_type(),
                        self.string_abi_type(),
                    ],
                    Some(self.context.i32_type().into()),
                );
                let key = self.const_string_abi(field, "automatic.json.array.key");
                self.builder
                    .build_call(
                        add,
                        &[builder.into(), key.into(), text.into()],
                        "automatic.json.add_array",
                    )
                    .map_err(|error| error.to_string())?;
                self.emit_string_release(text)?;
                Ok(())
            }
            Type::Lit(literal) => {
                let underlying = self.lit_underlying_type(literal);
                self.emit_automatic_json_builder_add(builder, field, value, &underlying)
            }
            Type::Union(variants) => {
                let variants = variants.clone();
                let function = self
                    .builder
                    .get_insert_block()
                    .and_then(inkwell::basic_block::BasicBlock::get_parent)
                    .ok_or("internal: automatic JSON encoding outside a function")?;
                let union_ty = self.union_llvm_type(&variants);
                let slot = self.entry_alloca(union_ty, "automatic.json.union")?;
                self.builder
                    .build_store(slot, value)
                    .map_err(|error| error.to_string())?;
                let tag = self.load_union_tag(&variants, slot)?;
                let blocks = (0..variants.len())
                    .map(|index| {
                        self.context.append_basic_block(
                            function,
                            &format!("automatic.json.union.variant{index}"),
                        )
                    })
                    .collect::<Vec<_>>();
                let done = self
                    .context
                    .append_basic_block(function, "automatic.json.union.done");
                let cases = blocks
                    .iter()
                    .enumerate()
                    .skip(1)
                    .map(|(index, block)| {
                        (
                            self.context.i8_type().const_int(index as u64, false),
                            *block,
                        )
                    })
                    .collect::<Vec<_>>();
                self.builder
                    .build_switch(tag, blocks[0], &cases)
                    .map_err(|error| error.to_string())?;
                for (index, variant) in variants.iter().enumerate() {
                    self.builder.position_at_end(blocks[index]);
                    let payload = self.load_union_payload(
                        &variants,
                        slot,
                        variant,
                        "automatic.json.union.value",
                    )?;
                    let text = self.emit_automatic_json_value_encode(payload, variant)?;
                    let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                    let add = self.declare_libc_fn(
                        "tyrus_json_builder_add_raw",
                        &[
                            pointer.into(),
                            self.string_abi_type(),
                            self.string_abi_type(),
                        ],
                        Some(self.context.i32_type().into()),
                    );
                    let key = self.const_string_abi(field, "automatic.json.union.key");
                    self.builder
                        .build_call(
                            add,
                            &[builder.into(), key.into(), text.into()],
                            "automatic.json.add_union",
                        )
                        .map_err(|error| error.to_string())?;
                    self.emit_string_release(text)?;
                    self.builder
                        .build_unconditional_branch(done)
                        .map_err(|error| error.to_string())?;
                }
                self.builder.position_at_end(done);
                Ok(())
            }
            Type::Named(_) | Type::Object(_) => {
                let text = self.emit_automatic_json_encode(value.into_struct_value(), ty, false)?;
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let add = self.declare_libc_fn(
                    "tyrus_json_builder_add_raw",
                    &[
                        pointer.into(),
                        self.string_abi_type(),
                        self.string_abi_type(),
                    ],
                    Some(self.context.i32_type().into()),
                );
                let key = self.const_string_abi(field, "automatic.json.object.key");
                self.builder
                    .build_call(
                        add,
                        &[builder.into(), key.into(), text.into()],
                        "automatic.json.add_object",
                    )
                    .map_err(|error| error.to_string())?;
                self.emit_string_release(text)?;
                Ok(())
            }
            _ => self.emit_http_json_builder_add(builder, field, value, ty),
        }
    }

    pub(crate) fn emit_automatic_json_array_encode(
        &mut self,
        array: StructValue<'ctx>,
        element: &Type,
    ) -> Result<StructValue<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let create =
            self.declare_libc_fn("tyrus_json_array_builder_new", &[], Some(pointer.into()));
        let builder = self
            .builder
            .build_call(create, &[], "automatic.json.array.builder")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: JSON array builder did not return a handle")?
            .into_pointer_value();
        let data = self
            .builder
            .build_extract_value(array, 0, "automatic.json.array.data")
            .map_err(|error| error.to_string())?
            .into_pointer_value();
        let length = self.truncate_to_usize(
            self.builder
                .build_extract_value(array, 1, "automatic.json.array.length")
                .map_err(|error| error.to_string())?
                .into_int_value(),
        );
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: JSON array encoding outside a function")?;
        let index_slot = self.entry_alloca(self.usize_type(), "automatic.json.array.index")?;
        self.builder
            .build_store(index_slot, self.usize_type().const_zero())
            .map_err(|error| error.to_string())?;
        let condition = self
            .context
            .append_basic_block(function, "automatic.json.array.condition");
        let body = self
            .context
            .append_basic_block(function, "automatic.json.array.body");
        let done = self
            .context
            .append_basic_block(function, "automatic.json.array.done");
        self.builder
            .build_unconditional_branch(condition)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(condition);
        let index = self
            .builder
            .build_load(
                self.usize_type(),
                index_slot,
                "automatic.json.array.index.value",
            )
            .map_err(|error| error.to_string())?
            .into_int_value();
        let more = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::ULT,
                index,
                length,
                "automatic.json.array.more",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(more, body, done)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(body);
        let element_value = unsafe {
            self.builder.build_gep(
                self.llvm_type(element),
                data,
                &[index],
                "automatic.json.array.element",
            )
        }
        .map_err(|error| error.to_string())?;
        let element_value = self
            .builder
            .build_load(
                self.llvm_type(element),
                element_value,
                "automatic.json.array.element.value",
            )
            .map_err(|error| error.to_string())?;
        self.emit_automatic_json_array_add(builder, element_value, element)?;
        let next = self
            .builder
            .build_int_add(
                index,
                self.usize_type().const_int(1, false),
                "automatic.json.array.next",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(index_slot, next)
            .map_err(|error| error.to_string())?;
        self.builder
            .build_unconditional_branch(condition)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(done);
        let finish = self.declare_libc_fn(
            "tyrus_json_array_builder_finish",
            &[pointer.into()],
            Some(self.string_abi_type()),
        );
        let text = self
            .builder
            .build_call(finish, &[builder.into()], "automatic.json.array.finish")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: JSON array builder did not return text")?
            .into_struct_value();
        let free = self.declare_libc_fn("tyrus_json_array_builder_free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[builder.into()], "automatic.json.array.builder.free")
            .map_err(|error| error.to_string())?;
        Ok(text)
    }

    pub(crate) fn emit_automatic_json_array_add(
        &mut self,
        builder: PointerValue<'ctx>,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
    ) -> Result<(), String> {
        let ty = &self.resolve_type(ty);
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        if matches!(transparent_codegen_type(ty), Type::Named(name) if name == "Instant") {
            let text = self.emit_automatic_json_instant_string(value)?;
            let add = self.declare_libc_fn(
                "tyrus_json_array_builder_add_string",
                &[pointer.into(), self.string_abi_type()],
                Some(self.context.i32_type().into()),
            );
            self.builder
                .build_call(
                    add,
                    &[builder.into(), text.into()],
                    "automatic.json.instant.add",
                )
                .map_err(|error| error.to_string())?;
            self.emit_string_release(text)?;
            return Ok(());
        }
        let (runtime, value, value_type): (
            &str,
            BasicMetadataValueEnum<'ctx>,
            BasicTypeEnum<'ctx>,
        ) = match transparent_codegen_type(ty) {
            Type::Str => (
                "tyrus_json_array_builder_add_string",
                value.into_struct_value().into(),
                self.string_abi_type(),
            ),
            Type::Bool => (
                "tyrus_json_array_builder_add_bool",
                value.into_int_value().into(),
                self.context.bool_type().into(),
            ),
            Type::I32 => (
                "tyrus_json_array_builder_add_i64",
                self.builder
                    .build_int_s_extend(
                        value.into_int_value(),
                        self.context.i64_type(),
                        "automatic.json.array.i64",
                    )
                    .map_err(|error| error.to_string())?
                    .into(),
                self.context.i64_type().into(),
            ),
            Type::I64 | Type::Isize => (
                "tyrus_json_array_builder_add_i64",
                value.into_int_value().into(),
                self.context.i64_type().into(),
            ),
            Type::U32 => (
                "tyrus_json_array_builder_add_u64",
                self.builder
                    .build_int_z_extend(
                        value.into_int_value(),
                        self.context.i64_type(),
                        "automatic.json.array.u64",
                    )
                    .map_err(|error| error.to_string())?
                    .into(),
                self.context.i64_type().into(),
            ),
            Type::U64 | Type::Usize => (
                "tyrus_json_array_builder_add_u64",
                value.into_int_value().into(),
                self.context.i64_type().into(),
            ),
            Type::F32 => (
                "tyrus_json_array_builder_add_f64",
                self.builder
                    .build_float_ext(
                        value.into_float_value(),
                        self.context.f64_type(),
                        "automatic.json.array.f64",
                    )
                    .map_err(|error| error.to_string())?
                    .into(),
                self.context.f64_type().into(),
            ),
            Type::F64 => (
                "tyrus_json_array_builder_add_f64",
                value.into_float_value().into(),
                self.context.f64_type().into(),
            ),
            Type::Decimal => {
                let decimal = self
                    .builder
                    .build_alloca(
                        self.llvm_type(&Type::Decimal),
                        "automatic.json.array.decimal",
                    )
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(decimal, value)
                    .map_err(|error| error.to_string())?;
                (
                    "tyrus_json_array_builder_add_decimal",
                    decimal.into(),
                    pointer.into(),
                )
            }
            Type::Lit(literal) => {
                let underlying = self.lit_underlying_type(literal);
                return self.emit_automatic_json_array_add(builder, value, &underlying);
            }
            Type::Union(variants) => {
                let variants = variants.clone();
                let function = self
                    .builder
                    .get_insert_block()
                    .and_then(inkwell::basic_block::BasicBlock::get_parent)
                    .ok_or("internal: automatic JSON encoding outside a function")?;
                let union_ty = self.union_llvm_type(&variants);
                let slot = self.entry_alloca(union_ty, "automatic.json.union")?;
                self.builder
                    .build_store(slot, value)
                    .map_err(|error| error.to_string())?;
                let tag = self.load_union_tag(&variants, slot)?;
                let blocks = (0..variants.len())
                    .map(|index| {
                        self.context.append_basic_block(
                            function,
                            &format!("automatic.json.union.variant{index}"),
                        )
                    })
                    .collect::<Vec<_>>();
                let done = self
                    .context
                    .append_basic_block(function, "automatic.json.union.done");
                let cases = blocks
                    .iter()
                    .enumerate()
                    .skip(1)
                    .map(|(index, block)| {
                        (
                            self.context.i8_type().const_int(index as u64, false),
                            *block,
                        )
                    })
                    .collect::<Vec<_>>();
                self.builder
                    .build_switch(tag, blocks[0], &cases)
                    .map_err(|error| error.to_string())?;
                for (index, variant) in variants.iter().enumerate() {
                    self.builder.position_at_end(blocks[index]);
                    let payload = self.load_union_payload(
                        &variants,
                        slot,
                        variant,
                        "automatic.json.union.value",
                    )?;
                    let text = self.emit_automatic_json_value_encode(payload, variant)?;
                    let add = self.declare_libc_fn(
                        "tyrus_json_array_builder_add_raw",
                        &[pointer.into(), self.string_abi_type()],
                        Some(self.context.i32_type().into()),
                    );
                    self.builder
                        .build_call(
                            add,
                            &[builder.into(), text.into()],
                            "automatic.json.array.add_union",
                        )
                        .map_err(|error| error.to_string())?;
                    self.emit_string_release(text)?;
                    self.builder
                        .build_unconditional_branch(done)
                        .map_err(|error| error.to_string())?;
                }
                self.builder.position_at_end(done);
                return Ok(());
            }
            Type::Named(_) | Type::Object(_) => {
                let text = self.emit_automatic_json_encode(value.into_struct_value(), ty, false)?;
                let add = self.declare_libc_fn(
                    "tyrus_json_array_builder_add_raw",
                    &[pointer.into(), self.string_abi_type()],
                    Some(self.context.i32_type().into()),
                );
                self.builder
                    .build_call(
                        add,
                        &[builder.into(), text.into()],
                        "automatic.json.array.add_object",
                    )
                    .map_err(|error| error.to_string())?;
                self.emit_string_release(text)?;
                return Ok(());
            }
            Type::Array(inner) => {
                let text =
                    self.emit_automatic_json_array_encode(value.into_struct_value(), inner)?;
                let add = self.declare_libc_fn(
                    "tyrus_json_array_builder_add_raw",
                    &[pointer.into(), self.string_abi_type()],
                    Some(self.context.i32_type().into()),
                );
                self.builder
                    .build_call(
                        add,
                        &[builder.into(), text.into()],
                        "automatic.json.array.add_nested",
                    )
                    .map_err(|error| error.to_string())?;
                self.emit_string_release(text)?;
                return Ok(());
            }
            other => {
                return Err(format!(
                    "internal: unsupported automatic JSON array element `{other}`"
                ));
            }
        };
        let add = self.declare_libc_fn(
            runtime,
            &[pointer.into(), value_type],
            Some(self.context.i32_type().into()),
        );
        self.builder
            .build_call(add, &[builder.into(), value], "automatic.json.array.add")
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn emit_automatic_json_instant_string(
        &mut self,
        value: BasicValueEnum<'ctx>,
    ) -> Result<StructValue<'ctx>, String> {
        let instant_type = self.llvm_type(&Type::Named("Instant".to_string()));
        let input = self.entry_alloca(instant_type, "automatic.json.instant.input")?;
        self.builder
            .build_store(input, value)
            .map_err(|error| error.to_string())?;
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let format = self.declare_libc_fn(
            "tyrus_temporal_instant_to_string",
            &[pointer.into()],
            Some(self.string_abi_type()),
        );
        let text = self
            .builder
            .build_call(format, &[input.into()], "automatic.json.instant.string")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: Instant JSON formatter returned no value")?
            .into_struct_value();
        self.guard_temporal_runtime_error()?;
        Ok(text)
    }

    pub(super) fn emit_automatic_json_decode(
        &mut self,
        type_name: &str,
        text: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let input = self.emit_expr_as(text, &Type::Str)?.into_struct_value();
        let result = self.emit_automatic_json_decode_value(type_name, input)?;
        self.record_owned_native_result(
            result,
            Type::Result {
                success: Box::new(Type::Named(type_name.to_string())),
                error: Box::new(Type::Named("JsonError".to_string())),
            },
        );
        Ok(result)
    }

    pub(crate) fn emit_automatic_json_decode_value(
        &mut self,
        type_name: &str,
        input: StructValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        self.emit_automatic_json_decode_type(&Type::Named(type_name.to_string()), input)
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn emit_automatic_json_decode_type(
        &mut self,
        value_type: &Type,
        input: StructValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let value_type = value_type.clone();
        let type_name = value_type.to_string();
        let type_name = type_name.as_str();
        let variants = vec![value_type.clone(), Type::Named("JsonError".to_string())];
        let union_type = self.union_llvm_type(&variants);
        let result_slot = self.entry_alloca(union_type, "automatic.json.result")?;
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let status_slot = self.entry_alloca(self.context.i32_type(), "automatic.json.status")?;
        let parse = self.declare_libc_fn(
            "tyrus_json_document_parse",
            &[self.string_abi_type(), pointer.into()],
            Some(pointer.into()),
        );
        let document = self
            .builder
            .build_call(
                parse,
                &[input.into(), status_slot.into()],
                "automatic.json.document",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: JSON document parser did not return a handle")?
            .into_pointer_value();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: automatic JSON decoding outside a function")?;
        let invalid = self
            .context
            .append_basic_block(function, "automatic.json.invalid");
        let fields = self
            .context
            .append_basic_block(function, "automatic.json.fields");
        let done = self
            .context
            .append_basic_block(function, "automatic.json.done");
        let parsed = self
            .builder
            .build_is_not_null(document, "automatic.json.parsed")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(parsed, fields, invalid)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(invalid);
        self.emit_automatic_json_error(
            result_slot,
            &variants,
            "ERR_JSON_INVALID_SYNTAX",
            "JSON input is not a valid object",
            "",
        )?;
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(fields);
        let declared = self
            .declared_fields_with_index(&value_type)
            .ok_or_else(|| format!("error: unknown interface `{type_name}`"))?;
        let known_fields = declared
            .iter()
            .map(|(_, field)| field.name.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let known = self
            .context
            .append_basic_block(function, "automatic.json.known_fields");
        {
            let unknown_field = self.const_string_abi(&known_fields, "automatic.json.known_fields");
            let unknown = self.declare_libc_fn(
                "tyrus_json_document_unknown_field",
                &[pointer.into(), self.string_abi_type()],
                Some(self.string_abi_type()),
            );
            let unexpected = self
                .builder
                .build_call(
                    unknown,
                    &[document.into(), unknown_field.into()],
                    "automatic.json.unexpected_field",
                )
                .map_err(|error| error.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or("internal: JSON unknown-field checker did not return a string")?
                .into_struct_value();
            let unexpected_data = self
                .builder
                .build_extract_value(unexpected, 0, "automatic.json.unexpected_field.data")
                .map_err(|error| error.to_string())?
                .into_pointer_value();
            let has_unexpected = self
                .builder
                .build_is_not_null(unexpected_data, "automatic.json.has_unexpected_field")
                .map_err(|error| error.to_string())?;
            let unknown_error = self
                .context
                .append_basic_block(function, "automatic.json.unknown_field");
            self.builder
                .build_conditional_branch(has_unexpected, unknown_error, known)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(unknown_error);
            self.emit_automatic_json_error_with_path(
                result_slot,
                &variants,
                "ERR_JSON_UNKNOWN_FIELD",
                "JSON contains a field that is not declared by this interface",
                unexpected,
            )?;
            let free = self.declare_libc_fn("tyrus_json_document_free", &[pointer.into()], None);
            self.builder
                .build_call(
                    free,
                    &[document.into()],
                    "automatic.json.document.free.unknown",
                )
                .map_err(|error| error.to_string())?;
            self.builder
                .build_unconditional_branch(done)
                .map_err(|error| error.to_string())?;
        }

        self.builder.position_at_end(known);
        let mut decoded = self.llvm_type(&value_type).into_struct_type().const_zero();
        // One slot for every failure exit below to release through. A field
        // that fails leaves the fields before it decoded and owned by nobody,
        // since the object is never returned and nothing downstream can release
        // it: every malformed document leaked whatever it got through first,
        // which is a leak an untrusted caller chooses the size of.
        let partial_slot =
            self.entry_alloca(self.llvm_type(&value_type), "automatic.json.partial")?;
        for (index, field) in declared {
            let field_type = field_storage_type(&field);
            if let Type::Optional(inner) = transparent_codegen_type(&field_type) {
                let (out, status) = self.emit_automatic_json_field(document, &field.name, inner)?;
                let field_ok = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::EQ,
                        status,
                        self.context.i32_type().const_zero(),
                        "automatic.json.optional_field.ok",
                    )
                    .map_err(|error| error.to_string())?;
                let present = self
                    .context
                    .append_basic_block(function, "automatic.json.optional.present");
                let absent_or_error = self
                    .context
                    .append_basic_block(function, "automatic.json.optional.absent_or_error");
                let next = self
                    .context
                    .append_basic_block(function, "automatic.json.optional.next");
                self.builder
                    .build_conditional_branch(field_ok, present, absent_or_error)
                    .map_err(|error| error.to_string())?;

                self.builder.position_at_end(present);
                let present_value = self
                    .builder
                    .build_load(self.llvm_type(inner), out, "automatic.json.optional.value")
                    .map_err(|error| error.to_string())?;
                let present_box = self.emit_optional_box(present_value, inner)?;
                let present_end = self
                    .builder
                    .get_insert_block()
                    .ok_or("internal: JSON optional present block disappeared")?;
                self.builder
                    .build_unconditional_branch(next)
                    .map_err(|error| error.to_string())?;

                self.builder.position_at_end(absent_or_error);
                let is_missing = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::EQ,
                        status,
                        self.context.i32_type().const_int(1, false),
                        "automatic.json.optional.missing",
                    )
                    .map_err(|error| error.to_string())?;
                let absent = self
                    .context
                    .append_basic_block(function, "automatic.json.optional.absent");
                let invalid = self
                    .context
                    .append_basic_block(function, "automatic.json.optional.invalid");
                self.builder
                    .build_conditional_branch(is_missing, absent, invalid)
                    .map_err(|error| error.to_string())?;

                self.builder.position_at_end(absent);
                let absent_end = self
                    .builder
                    .get_insert_block()
                    .ok_or("internal: JSON optional absent block disappeared")?;
                self.builder
                    .build_unconditional_branch(next)
                    .map_err(|error| error.to_string())?;

                self.builder.position_at_end(invalid);
                self.emit_automatic_json_error(
                    result_slot,
                    &variants,
                    "ERR_JSON_TYPE_MISMATCH",
                    "JSON field does not match the declared type",
                    &field.name,
                )?;
                let free =
                    self.declare_libc_fn("tyrus_json_document_free", &[pointer.into()], None);
                self.builder
                    .build_call(
                        free,
                        &[document.into()],
                        "automatic.json.document.free.optional_error",
                    )
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(partial_slot, decoded)
                    .map_err(|error| error.to_string())?;
                self.emit_async_drop_value_in_place(function, partial_slot, &value_type)?;
                self.builder
                    .build_unconditional_branch(done)
                    .map_err(|error| error.to_string())?;

                self.builder.position_at_end(next);
                let optional = self
                    .builder
                    .build_phi(pointer, "automatic.json.optional.field")
                    .map_err(|error| error.to_string())?;
                optional.add_incoming(&[
                    (&present_box, present_end),
                    (&pointer.const_null(), absent_end),
                ]);
                decoded = self
                    .builder
                    .build_insert_value(
                        decoded,
                        optional.as_basic_value(),
                        index,
                        "automatic.json.optional.insert",
                    )
                    .map_err(|error| error.to_string())?
                    .into_struct_value();
                continue;
            }
            let (out, status) =
                self.emit_automatic_json_field(document, &field.name, &field_type)?;
            let field_ok = self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    status,
                    self.context.i32_type().const_zero(),
                    "automatic.json.field.ok",
                )
                .map_err(|error| error.to_string())?;
            let next = self
                .context
                .append_basic_block(function, "automatic.json.next_field");
            let missing = self
                .context
                .append_basic_block(function, "automatic.json.missing_field");
            self.builder
                .build_conditional_branch(field_ok, next, missing)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(missing);
            let is_missing = self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    status,
                    self.context.i32_type().const_int(1, false),
                    "automatic.json.field.missing",
                )
                .map_err(|error| error.to_string())?;
            let missing_error = self
                .context
                .append_basic_block(function, "automatic.json.missing_error");
            let type_error = self
                .context
                .append_basic_block(function, "automatic.json.type_error");
            self.builder
                .build_conditional_branch(is_missing, missing_error, type_error)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(missing_error);
            self.emit_automatic_json_error(
                result_slot,
                &variants,
                "ERR_JSON_MISSING_FIELD",
                "JSON is missing a required field",
                &field.name,
            )?;
            let free = self.declare_libc_fn("tyrus_json_document_free", &[pointer.into()], None);
            self.builder
                .build_call(
                    free,
                    &[document.into()],
                    "automatic.json.document.free.missing",
                )
                .map_err(|error| error.to_string())?;
            self.builder
                .build_store(partial_slot, decoded)
                .map_err(|error| error.to_string())?;
            self.emit_async_drop_value_in_place(function, partial_slot, &value_type)?;
            self.builder
                .build_unconditional_branch(done)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(type_error);
            let unknown_variant = self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    status,
                    self.context.i32_type().const_int(3, false),
                    "automatic.json.field.unknown_variant",
                )
                .map_err(|error| error.to_string())?;
            let unknown_variant_error = self
                .context
                .append_basic_block(function, "automatic.json.unknown_variant");
            let other_type_error = self
                .context
                .append_basic_block(function, "automatic.json.other_type_error");
            self.builder
                .build_conditional_branch(unknown_variant, unknown_variant_error, other_type_error)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(unknown_variant_error);
            self.emit_automatic_json_error(
                result_slot,
                &variants,
                "ERR_JSON_UNKNOWN_VARIANT",
                "JSON names a variant this union does not declare",
                &field.name,
            )?;
            let free = self.declare_libc_fn("tyrus_json_document_free", &[pointer.into()], None);
            self.builder
                .build_call(
                    free,
                    &[document.into()],
                    "automatic.json.document.free.unknown_variant",
                )
                .map_err(|error| error.to_string())?;
            self.builder
                .build_store(partial_slot, decoded)
                .map_err(|error| error.to_string())?;
            self.emit_async_drop_value_in_place(function, partial_slot, &value_type)?;
            self.builder
                .build_unconditional_branch(done)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(other_type_error);
            let is_out_of_range = self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    status,
                    self.context.i32_type().const_int(4, false),
                    "automatic.json.field.out_of_range",
                )
                .map_err(|error| error.to_string())?;
            let range_error = self
                .context
                .append_basic_block(function, "automatic.json.range_error");
            let incompatible_type = self
                .context
                .append_basic_block(function, "automatic.json.incompatible_type");
            self.builder
                .build_conditional_branch(is_out_of_range, range_error, incompatible_type)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(range_error);
            self.emit_automatic_json_error(
                result_slot,
                &variants,
                "ERR_JSON_NUMBER_OUT_OF_RANGE",
                "JSON number is outside the declared numeric range",
                &field.name,
            )?;
            let free = self.declare_libc_fn("tyrus_json_document_free", &[pointer.into()], None);
            self.builder
                .build_call(
                    free,
                    &[document.into()],
                    "automatic.json.document.free.range",
                )
                .map_err(|error| error.to_string())?;
            self.builder
                .build_store(partial_slot, decoded)
                .map_err(|error| error.to_string())?;
            self.emit_async_drop_value_in_place(function, partial_slot, &value_type)?;
            self.builder
                .build_unconditional_branch(done)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(incompatible_type);
            self.emit_automatic_json_error(
                result_slot,
                &variants,
                "ERR_JSON_TYPE_MISMATCH",
                "JSON field does not match the declared type",
                &field.name,
            )?;
            let free = self.declare_libc_fn("tyrus_json_document_free", &[pointer.into()], None);
            self.builder
                .build_call(
                    free,
                    &[document.into()],
                    "automatic.json.document.free.error",
                )
                .map_err(|error| error.to_string())?;
            self.builder
                .build_store(partial_slot, decoded)
                .map_err(|error| error.to_string())?;
            self.emit_async_drop_value_in_place(function, partial_slot, &value_type)?;
            self.builder
                .build_unconditional_branch(done)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(next);
            let field_value = self
                .builder
                .build_load(
                    self.llvm_type(&field_type),
                    out,
                    "automatic.json.field.loaded",
                )
                .map_err(|error| error.to_string())?;
            decoded = self
                .builder
                .build_insert_value(decoded, field_value, index, "automatic.json.field.value")
                .map_err(|error| error.to_string())?
                .into_struct_value();
        }
        self.store_union_variant(&variants, result_slot, 0, decoded.into())?;
        let free = self.declare_libc_fn("tyrus_json_document_free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[document.into()], "automatic.json.document.free")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(done);
        let result = self
            .builder
            .build_load(union_type, result_slot, "automatic.json.result.value")
            .map_err(|error| error.to_string())?;
        Ok(result)
    }

    pub(super) fn emit_automatic_json_field(
        &mut self,
        document: PointerValue<'ctx>,
        field: &str,
        ty: &Type,
    ) -> Result<(PointerValue<'ctx>, inkwell::values::IntValue<'ctx>), String> {
        let ty = &self.resolve_type(ty);
        if let Type::Optional(inner) = transparent_codegen_type(ty) {
            return self.emit_automatic_json_optional_field(document, field, inner);
        }
        let out = self.entry_alloca(self.llvm_type(ty), "automatic.json.field.out")?;
        let key = self.const_string_abi(field, "automatic.json.field.key");
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let runtime = match transparent_codegen_type(ty) {
            Type::Str => "tyrus_json_document_get_string",
            Type::Bool => "tyrus_json_document_get_bool",
            Type::I32 => "tyrus_json_document_get_i32",
            Type::I64 => "tyrus_json_document_get_i64",
            Type::U32 => "tyrus_json_document_get_u32",
            Type::U64 => "tyrus_json_document_get_u64",
            Type::F32 => "tyrus_json_document_get_f32",
            Type::F64 => "tyrus_json_document_get_f64",
            Type::Decimal => "tyrus_json_document_get_decimal",
            Type::Array(element) => {
                return self.emit_automatic_json_array_field(document, field, element);
            }
            Type::Named(name) => {
                return self.emit_automatic_json_nested_field(document, field, name);
            }
            Type::Object(_) => {
                return self.emit_automatic_json_nested_typed_field(document, field, ty);
            }
            Type::Union(variants) => {
                let variants = variants.clone();
                return self.emit_automatic_json_union_field(document, field, &variants);
            }
            Type::Lit(literal) => {
                let underlying = self.lit_underlying_type(literal);
                return self.emit_automatic_json_field(document, field, &underlying);
            }
            other => {
                return Err(format!(
                    "internal: unsupported automatic JSON field type `{other}`"
                ));
            }
        };
        let get = self.declare_libc_fn(
            runtime,
            &[pointer.into(), self.string_abi_type(), pointer.into()],
            Some(self.context.i32_type().into()),
        );
        let status = self
            .builder
            .build_call(
                get,
                &[document.into(), key.into(), out.into()],
                "automatic.json.field.status",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: automatic JSON field decoder did not return a status")?
            .into_int_value();
        Ok((out, status))
    }

    fn emit_automatic_json_optional_field(
        &mut self,
        document: PointerValue<'ctx>,
        field: &str,
        inner: &Type,
    ) -> Result<(PointerValue<'ctx>, inkwell::values::IntValue<'ctx>), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_type = self.context.i32_type();
        let out = self.entry_alloca(pointer, "automatic.json.optional.out")?;
        let status_slot = self.entry_alloca(i32_type, "automatic.json.optional.status")?;
        let key = self.const_string_abi(field, "automatic.json.optional.key");
        let raw_get = self.declare_libc_fn(
            "tyrus_json_document_get_raw",
            &[pointer.into(), self.string_abi_type()],
            Some(self.string_abi_type()),
        );
        let raw = self
            .builder
            .build_call(
                raw_get,
                &[document.into(), key.into()],
                "automatic.json.optional.raw",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: JSON optional getter did not return text")?
            .into_struct_value();
        let raw_data = self
            .builder
            .build_extract_value(raw, 0, "automatic.json.optional.raw.data")
            .map_err(|error| error.to_string())?
            .into_pointer_value();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: optional JSON decoding outside a function")?;
        let missing = self
            .context
            .append_basic_block(function, "automatic.json.optional.missing");
        let present = self
            .context
            .append_basic_block(function, "automatic.json.optional.present");
        let null = self
            .context
            .append_basic_block(function, "automatic.json.optional.null");
        let decode = self
            .context
            .append_basic_block(function, "automatic.json.optional.decode");
        let decoded = self
            .context
            .append_basic_block(function, "automatic.json.optional.decoded");
        let invalid = self
            .context
            .append_basic_block(function, "automatic.json.optional.invalid");
        let done = self
            .context
            .append_basic_block(function, "automatic.json.optional.done");
        let exists = self
            .builder
            .build_is_not_null(raw_data, "automatic.json.optional.exists")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(exists, present, missing)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(missing);
        self.builder
            .build_store(out, pointer.const_null())
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(status_slot, i32_type.const_int(1, false))
            .map_err(|error| error.to_string())?;
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(present);
        let is_null = self
            .builder
            .build_call(
                self.declare_libc_fn(
                    "tyrus_json_is_null_literal",
                    &[self.string_abi_type()],
                    Some(i32_type.into()),
                ),
                &[raw.into()],
                "automatic.json.optional.is_null",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: JSON null checker did not return a status")?
            .into_int_value();
        let is_null = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::NE,
                is_null,
                i32_type.const_zero(),
                "automatic.json.optional.is_null",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(is_null, null, decode)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(null);
        self.builder
            .build_store(out, pointer.const_null())
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(status_slot, i32_type.const_zero())
            .map_err(|error| error.to_string())?;
        self.emit_string_release(raw)?;
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(decode);
        let (inner_out, inner_status) = self.emit_automatic_json_field(document, field, inner)?;
        self.emit_string_release(raw)?;
        let inner_valid = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                inner_status,
                i32_type.const_zero(),
                "automatic.json.optional.inner_valid",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(inner_valid, decoded, invalid)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(decoded);
        let inner_value = self
            .builder
            .build_load(
                self.llvm_type(inner),
                inner_out,
                "automatic.json.optional.value",
            )
            .map_err(|error| error.to_string())?;
        let boxed = self.emit_optional_box(inner_value, inner)?;
        self.builder
            .build_store(out, boxed)
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(status_slot, i32_type.const_zero())
            .map_err(|error| error.to_string())?;
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(invalid);
        self.builder
            .build_store(out, pointer.const_null())
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(status_slot, inner_status)
            .map_err(|error| error.to_string())?;
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(done);
        let status = self
            .builder
            .build_load(
                i32_type,
                status_slot,
                "automatic.json.optional.status.value",
            )
            .map_err(|error| error.to_string())?
            .into_int_value();
        Ok((out, status))
    }

    pub(crate) fn emit_automatic_json_nested_field(
        &mut self,
        document: PointerValue<'ctx>,
        field: &str,
        name: &str,
    ) -> Result<(PointerValue<'ctx>, inkwell::values::IntValue<'ctx>), String> {
        self.emit_automatic_json_nested_typed_field(document, field, &Type::Named(name.to_string()))
    }

    fn emit_automatic_json_nested_typed_field(
        &mut self,
        document: PointerValue<'ctx>,
        field: &str,
        nested_type: &Type,
    ) -> Result<(PointerValue<'ctx>, inkwell::values::IntValue<'ctx>), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_type = self.context.i32_type();
        let out = self.entry_alloca(self.llvm_type(nested_type), "automatic.json.nested.out")?;
        let key = self.const_string_abi(field, "automatic.json.nested.key");
        let raw_get = self.declare_libc_fn(
            "tyrus_json_document_get_raw",
            &[pointer.into(), self.string_abi_type()],
            Some(self.string_abi_type()),
        );
        let raw = self
            .builder
            .build_call(
                raw_get,
                &[document.into(), key.into()],
                "automatic.json.nested.raw",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: JSON nested getter did not return text")?
            .into_struct_value();
        let raw_data = self
            .builder
            .build_extract_value(raw, 0, "automatic.json.nested.raw.data")
            .map_err(|error| error.to_string())?
            .into_pointer_value();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: nested JSON decoding outside a function")?;
        let present = self
            .context
            .append_basic_block(function, "automatic.json.nested.present");
        let missing = self
            .context
            .append_basic_block(function, "automatic.json.nested.missing");
        let success = self
            .context
            .append_basic_block(function, "automatic.json.nested.success");
        let failed = self
            .context
            .append_basic_block(function, "automatic.json.nested.failed");
        let done = self
            .context
            .append_basic_block(function, "automatic.json.nested.done");
        let exists = self
            .builder
            .build_is_not_null(raw_data, "automatic.json.nested.exists")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(exists, present, missing)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(missing);
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(present);
        let result = self.emit_automatic_json_decode_type(nested_type, raw)?;
        self.emit_string_release(raw)?;
        let variants = vec![nested_type.clone(), Type::Named("JsonError".to_string())];
        let result_slot = self.entry_alloca(
            self.union_llvm_type(&variants),
            "automatic.json.nested.result",
        )?;
        self.builder
            .build_store(result_slot, result)
            .map_err(|error| error.to_string())?;
        let tag = self.load_union_tag(&variants, result_slot)?;
        let decoded = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                tag,
                self.context.i8_type().const_zero(),
                "automatic.json.nested.decoded",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(decoded, success, failed)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(success);
        let value = self.load_union_payload(
            &variants,
            result_slot,
            nested_type,
            "automatic.json.nested.value",
        )?;
        self.builder
            .build_store(out, value)
            .map_err(|error| error.to_string())?;
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(failed);
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(done);
        let status = self
            .builder
            .build_phi(i32_type, "automatic.json.nested.status")
            .map_err(|error| error.to_string())?;
        let success_end = success;
        status.add_incoming(&[
            (&i32_type.const_int(1, false), missing),
            (&i32_type.const_zero(), success_end),
            (&i32_type.const_int(2, false), failed),
        ]);
        Ok((out, status.as_basic_value().into_int_value()))
    }

    pub(crate) fn emit_automatic_json_union_field(
        &mut self,
        document: PointerValue<'ctx>,
        field: &str,
        variants: &[Type],
    ) -> Result<(PointerValue<'ctx>, inkwell::values::IntValue<'ctx>), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_type = self.context.i32_type();
        let union_type = Type::Union(variants.to_vec());
        let discriminant = tyrus_core::automatic_data_methods::union_discriminant_field(variants)
            .ok_or_else(|| {
            format!("error: `{field}` is not a discriminated union, so it has no variant to read")
        })?;
        let out = self.entry_alloca(self.llvm_type(&union_type), "automatic.json.union.out")?;
        let status_slot = self.entry_alloca(i32_type, "automatic.json.union.status")?;
        self.builder
            .build_store(status_slot, i32_type.const_int(2, false))
            .map_err(|error| error.to_string())?;
        let key = self.const_string_abi(field, "automatic.json.union.key");
        let raw_get = self.declare_libc_fn(
            "tyrus_json_document_get_raw",
            &[pointer.into(), self.string_abi_type()],
            Some(self.string_abi_type()),
        );
        let raw = self
            .builder
            .build_call(
                raw_get,
                &[document.into(), key.into()],
                "automatic.json.union.raw",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: JSON raw getter did not return text")?
            .into_struct_value();
        let raw_data = self
            .builder
            .build_extract_value(raw, 0, "automatic.json.union.raw.data")
            .map_err(|error| error.to_string())?
            .into_pointer_value();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: union JSON decoding outside a function")?;
        let present = self
            .context
            .append_basic_block(function, "automatic.json.union.present");
        let missing = self
            .context
            .append_basic_block(function, "automatic.json.union.missing");
        let done = self
            .context
            .append_basic_block(function, "automatic.json.union.done");
        let tag_slot =
            self.entry_alloca(self.string_abi_type(), "automatic.json.union.tag.slot")?;
        self.builder
            .build_store(tag_slot, self.null_string())
            .map_err(|error| error.to_string())?;
        let exists = self
            .builder
            .build_is_not_null(raw_data, "automatic.json.union.exists")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(exists, present, missing)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(missing);
        self.builder
            .build_store(status_slot, i32_type.const_int(1, false))
            .map_err(|error| error.to_string())?;
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(present);
        let discriminant_key =
            self.const_string_abi(&discriminant, "automatic.json.union.discriminant.key");
        let tag_reader = self.declare_libc_fn(
            "tyrus_json_variant_tag",
            &[self.string_abi_type(), self.string_abi_type()],
            Some(self.string_abi_type()),
        );
        let tag_text = self
            .builder
            .build_call(
                tag_reader,
                &[raw.into(), discriminant_key.into()],
                "automatic.json.union.tag",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: JSON variant tag reader did not return text")?
            .into_struct_value();
        self.builder
            .build_store(tag_slot, tag_text)
            .map_err(|error| error.to_string())?;
        let tag_data = self
            .builder
            .build_extract_value(tag_text, 0, "automatic.json.union.tag.data")
            .map_err(|error| error.to_string())?
            .into_pointer_value();
        let has_tag = self
            .builder
            .build_is_not_null(tag_data, "automatic.json.union.has_tag")
            .map_err(|error| error.to_string())?;
        let compare = self
            .context
            .append_basic_block(function, "automatic.json.union.compare");
        let shapeless = self
            .context
            .append_basic_block(function, "automatic.json.union.shapeless");
        self.builder
            .build_conditional_branch(has_tag, compare, shapeless)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(shapeless);
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(compare);
        let mut next_test = self
            .context
            .append_basic_block(function, "automatic.json.union.variant0");
        self.builder
            .build_unconditional_branch(next_test)
            .map_err(|error| error.to_string())?;
        for (index, variant) in variants.iter().enumerate() {
            self.builder.position_at_end(next_test);
            let literal = tyrus_core::automatic_data_methods::union_variant_discriminant_text(
                variant,
                &discriminant,
            )
            .ok_or_else(|| {
                format!("error: variant {index} of `{field}` has no discriminant to compare")
            })?;
            let expected = self.const_string_abi(&literal, "automatic.json.union.literal");
            let matches = self.emit_string_equals(
                tag_text,
                expected.into_struct_value(),
                "automatic.json.union.matches",
            )?;
            let decode = self
                .context
                .append_basic_block(function, "automatic.json.union.decode");
            let following = if index + 1 < variants.len() {
                self.context
                    .append_basic_block(function, "automatic.json.union.variant_next")
            } else {
                self.context
                    .append_basic_block(function, "automatic.json.union.unknown")
            };
            self.builder
                .build_conditional_branch(matches, decode, following)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(decode);
            let result = self.emit_automatic_json_decode_type(variant, raw)?;
            let result_variants = vec![variant.clone(), Type::Named("JsonError".to_string())];
            let result_slot = self.entry_alloca(
                self.union_llvm_type(&result_variants),
                "automatic.json.union.variant.result",
            )?;
            self.builder
                .build_store(result_slot, result)
                .map_err(|error| error.to_string())?;
            let result_tag = self.load_union_tag(&result_variants, result_slot)?;
            let decoded = self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    result_tag,
                    self.context.i8_type().const_zero(),
                    "automatic.json.union.variant.decoded",
                )
                .map_err(|error| error.to_string())?;
            let stored = self
                .context
                .append_basic_block(function, "automatic.json.union.variant.stored");
            let rejected = self
                .context
                .append_basic_block(function, "automatic.json.union.variant.rejected");
            self.builder
                .build_conditional_branch(decoded, stored, rejected)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(stored);
            let payload = self.load_union_payload(
                &result_variants,
                result_slot,
                variant,
                "automatic.json.union.variant.value",
            )?;
            self.store_union_variant(variants, out, index as u8, payload)?;
            self.builder
                .build_store(status_slot, i32_type.const_zero())
                .map_err(|error| error.to_string())?;
            self.builder
                .build_unconditional_branch(done)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(rejected);
            self.builder
                .build_unconditional_branch(done)
                .map_err(|error| error.to_string())?;

            next_test = following;
        }
        self.builder.position_at_end(next_test);
        self.builder
            .build_store(status_slot, i32_type.const_int(3, false))
            .map_err(|error| error.to_string())?;
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(done);
        self.emit_string_release(raw)?;
        let tag = self
            .builder
            .build_load(
                self.string_abi_type(),
                tag_slot,
                "automatic.json.union.tag.owned",
            )
            .map_err(|error| error.to_string())?
            .into_struct_value();
        self.emit_string_release(tag)?;
        let status = self
            .builder
            .build_load(i32_type, status_slot, "automatic.json.union.status.value")
            .map_err(|error| error.to_string())?
            .into_int_value();
        Ok((out, status))
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn emit_automatic_json_array_field(
        &mut self,
        document: PointerValue<'ctx>,
        field: &str,
        element: &Type,
    ) -> Result<(PointerValue<'ctx>, inkwell::values::IntValue<'ctx>), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_type = self.context.i32_type();
        let usize_type = self.usize_type();
        let array_type = Type::Array(Box::new(element.clone()));
        let out = self.entry_alloca(self.llvm_type(&array_type), "automatic.json.array.out")?;
        let status_slot = self.entry_alloca(i32_type, "automatic.json.array.status")?;
        let key = self.const_string_abi(field, "automatic.json.array.key");
        let get = self.declare_libc_fn(
            "tyrus_json_document_get_array",
            &[pointer.into(), self.string_abi_type(), pointer.into()],
            Some(pointer.into()),
        );
        let handle = self
            .builder
            .build_call(
                get,
                &[document.into(), key.into(), status_slot.into()],
                "automatic.json.array.handle",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: JSON array getter did not return a handle")?
            .into_pointer_value();
        let initial_status = self
            .builder
            .build_load(i32_type, status_slot, "automatic.json.array.initial_status")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: JSON array decoding outside a function")?;
        let build = self
            .context
            .append_basic_block(function, "automatic.json.array.build");
        let failed = self
            .context
            .append_basic_block(function, "automatic.json.array.failed");
        let done = self
            .context
            .append_basic_block(function, "automatic.json.array.done");
        let is_ok = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                initial_status,
                i32_type.const_zero(),
                "automatic.json.array.available",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(is_ok, build, failed)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(failed);
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(build);
        let len_fn = self.declare_libc_fn(
            "tyrus_json_array_len",
            &[pointer.into()],
            Some(usize_type.into()),
        );
        let length = self
            .builder
            .build_call(len_fn, &[handle.into()], "automatic.json.array.length")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: JSON array length did not return a value")?
            .into_int_value();
        let element_type = self.llvm_type(element);
        let element_size = element_type.size_of().ok_or_else(|| {
            format!("internal: JSON array element `{element}` has no native size")
        })?;
        let data = self.emit_array_buffer(element_size, length)?;
        let index_slot = self.entry_alloca(usize_type, "automatic.json.array.index")?;
        self.builder
            .build_store(index_slot, usize_type.const_zero())
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(status_slot, i32_type.const_zero())
            .map_err(|error| error.to_string())?;
        let condition = self
            .context
            .append_basic_block(function, "automatic.json.array.condition");
        let body = self
            .context
            .append_basic_block(function, "automatic.json.array.body");
        let finish = self
            .context
            .append_basic_block(function, "automatic.json.array.finish");
        self.builder
            .build_unconditional_branch(condition)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(condition);
        let index = self
            .builder
            .build_load(usize_type, index_slot, "automatic.json.array.index.value")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let status = self
            .builder
            .build_load(i32_type, status_slot, "automatic.json.array.status.value")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let has_next = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::ULT,
                index,
                length,
                "automatic.json.array.has_next",
            )
            .map_err(|error| error.to_string())?;
        let no_error = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                status,
                i32_type.const_zero(),
                "automatic.json.array.no_error",
            )
            .map_err(|error| error.to_string())?;
        let continue_loop = self
            .builder
            .build_and(has_next, no_error, "automatic.json.array.continue")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(continue_loop, body, finish)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(body);
        let slot = unsafe {
            self.builder
                .build_gep(element_type, data, &[index], "automatic.json.array.element")
        }
        .map_err(|error| error.to_string())?;
        if matches!(
            transparent_codegen_type(element),
            Type::Named(_) | Type::Object(_)
        ) {
            let raw_get = self.declare_libc_fn(
                "tyrus_json_array_get_raw",
                &[pointer.into(), usize_type.into()],
                Some(self.string_abi_type()),
            );
            let raw = self
                .builder
                .build_call(
                    raw_get,
                    &[handle.into(), index.into()],
                    "automatic.json.array.nested.raw",
                )
                .map_err(|error| error.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or("internal: JSON array nested getter did not return text")?
                .into_struct_value();
            let raw_data = self
                .builder
                .build_extract_value(raw, 0, "automatic.json.array.nested.raw.data")
                .map_err(|error| error.to_string())?
                .into_pointer_value();
            let raw_present = self
                .context
                .append_basic_block(function, "automatic.json.array.nested.present");
            let raw_missing = self
                .context
                .append_basic_block(function, "automatic.json.array.nested.missing");
            let nested_success = self
                .context
                .append_basic_block(function, "automatic.json.array.nested.success");
            let nested_failed = self
                .context
                .append_basic_block(function, "automatic.json.array.nested.failed");
            let advance = self
                .context
                .append_basic_block(function, "automatic.json.array.nested.advance");
            let raw_exists = self
                .builder
                .build_is_not_null(raw_data, "automatic.json.array.nested.exists")
                .map_err(|error| error.to_string())?;
            self.builder
                .build_conditional_branch(raw_exists, raw_present, raw_missing)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(raw_missing);
            self.builder
                .build_store(status_slot, i32_type.const_int(2, false))
                .map_err(|error| error.to_string())?;
            self.builder
                .build_unconditional_branch(advance)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(raw_present);
            let nested_type = element.clone();
            let result = self.emit_automatic_json_decode_type(&nested_type, raw)?;
            self.emit_string_release(raw)?;
            let variants = vec![nested_type.clone(), Type::Named("JsonError".to_string())];
            let result_slot = self.entry_alloca(
                self.union_llvm_type(&variants),
                "automatic.json.array.nested.result",
            )?;
            self.builder
                .build_store(result_slot, result)
                .map_err(|error| error.to_string())?;
            let tag = self.load_union_tag(&variants, result_slot)?;
            let decoded = self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    tag,
                    self.context.i8_type().const_zero(),
                    "automatic.json.array.nested.decoded",
                )
                .map_err(|error| error.to_string())?;
            self.builder
                .build_conditional_branch(decoded, nested_success, nested_failed)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(nested_success);
            let value = self.load_union_payload(
                &variants,
                result_slot,
                &nested_type,
                "automatic.json.array.nested.value",
            )?;
            self.builder
                .build_store(slot, value)
                .map_err(|error| error.to_string())?;
            self.builder
                .build_store(status_slot, i32_type.const_zero())
                .map_err(|error| error.to_string())?;
            self.builder
                .build_unconditional_branch(advance)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(nested_failed);
            self.builder
                .build_store(status_slot, i32_type.const_int(2, false))
                .map_err(|error| error.to_string())?;
            self.builder
                .build_unconditional_branch(advance)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(advance);
            let next = self
                .builder
                .build_int_add(
                    index,
                    usize_type.const_int(1, false),
                    "automatic.json.array.next",
                )
                .map_err(|error| error.to_string())?;
            self.builder
                .build_store(index_slot, next)
                .map_err(|error| error.to_string())?;
            self.builder
                .build_unconditional_branch(condition)
                .map_err(|error| error.to_string())?;
        } else {
            let runtime = match transparent_codegen_type(element) {
                Type::Str => "tyrus_json_array_get_string",
                Type::Bool => "tyrus_json_array_get_bool",
                Type::I32 => "tyrus_json_array_get_i32",
                Type::I64 | Type::Isize => "tyrus_json_array_get_i64",
                Type::U32 => "tyrus_json_array_get_u32",
                Type::U64 | Type::Usize => "tyrus_json_array_get_u64",
                Type::F32 => "tyrus_json_array_get_f32",
                Type::F64 => "tyrus_json_array_get_f64",
                Type::Decimal => "tyrus_json_array_get_decimal",
                other => {
                    return Err(format!(
                        "internal: unsupported automatic JSON array element `{other}`"
                    ));
                }
            };
            let get_element = self.declare_libc_fn(
                runtime,
                &[pointer.into(), usize_type.into(), pointer.into()],
                Some(i32_type.into()),
            );
            let element_status = self
                .builder
                .build_call(
                    get_element,
                    &[handle.into(), index.into(), slot.into()],
                    "automatic.json.array.element.status",
                )
                .map_err(|error| error.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or("internal: JSON array element getter did not return a status")?
                .into_int_value();
            self.builder
                .build_store(status_slot, element_status)
                .map_err(|error| error.to_string())?;
            let next = self
                .builder
                .build_int_add(
                    index,
                    usize_type.const_int(1, false),
                    "automatic.json.array.next",
                )
                .map_err(|error| error.to_string())?;
            self.builder
                .build_store(index_slot, next)
                .map_err(|error| error.to_string())?;
            self.builder
                .build_unconditional_branch(condition)
                .map_err(|error| error.to_string())?;
        }
        self.builder.position_at_end(finish);
        let mut array = self.llvm_type(&array_type).into_struct_type().get_undef();
        array = self
            .builder
            .build_insert_value(array, data, 0, "automatic.json.array.data")
            .map_err(|error| error.to_string())?
            .into_struct_value();
        array = self
            .builder
            .build_insert_value(array, length, 1, "automatic.json.array.length")
            .map_err(|error| error.to_string())?
            .into_struct_value();
        array = self
            .builder
            .build_insert_value(array, length, 2, "automatic.json.array.capacity")
            .map_err(|error| error.to_string())?
            .into_struct_value();
        self.builder
            .build_store(out, array)
            .map_err(|error| error.to_string())?;
        let free = self.declare_libc_fn("tyrus_json_array_free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[handle.into()], "automatic.json.array.free")
            .map_err(|error| error.to_string())?;
        let final_status = self
            .builder
            .build_load(i32_type, status_slot, "automatic.json.array.final_status")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let finish_end = self
            .builder
            .get_insert_block()
            .ok_or("internal: JSON array finish block disappeared")?;
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(done);
        let status = self
            .builder
            .build_phi(i32_type, "automatic.json.array.result_status")
            .map_err(|error| error.to_string())?;
        status.add_incoming(&[(&initial_status, failed), (&final_status, finish_end)]);
        Ok((out, status.as_basic_value().into_int_value()))
    }

    pub(super) fn emit_automatic_json_error(
        &self,
        result_slot: PointerValue<'ctx>,
        variants: &[Type],
        code: &str,
        message: &str,
        path: &str,
    ) -> Result<(), String> {
        self.emit_automatic_json_error_with_path(
            result_slot,
            variants,
            code,
            message,
            self.const_string_abi(path, "automatic.json.error.path")
                .into_struct_value(),
        )
    }

    pub(super) fn emit_automatic_json_error_with_path(
        &self,
        result_slot: PointerValue<'ctx>,
        variants: &[Type],
        code: &str,
        message: &str,
        path: StructValue<'ctx>,
    ) -> Result<(), String> {
        let mut error = self
            .llvm_type(&Type::Named("JsonError".to_string()))
            .into_struct_type()
            .get_undef();
        for (index, value) in [
            self.const_string_abi(code, "automatic.json.error.code"),
            self.const_string_abi(message, "automatic.json.error.message"),
            path.into(),
        ]
        .into_iter()
        .enumerate()
        {
            error = self
                .builder
                .build_insert_value(error, value, index as u32, "automatic.json.error.field")
                .map_err(|error| error.to_string())?
                .into_struct_value();
        }
        self.store_union_variant(variants, result_slot, 1, error.into())
    }
}
