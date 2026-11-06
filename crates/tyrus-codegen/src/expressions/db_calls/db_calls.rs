use super::*;
use tyrus_core::ast::{ExprKind, Field};

impl<'ctx> Codegen<'ctx> {
    fn is_mongodb_database(&self) -> bool {
        self.database_path
            .as_deref()
            .and_then(|target| serde_json::from_str::<serde_json::Value>(target).ok())
            .and_then(|target| target.get("backend")?.as_str().map(str::to_owned))
            .is_some_and(|backend| backend == "mongodb")
    }

    fn is_postgresql_database(&self) -> bool {
        self.database_path
            .as_deref()
            .and_then(|target| serde_json::from_str::<serde_json::Value>(target).ok())
            .and_then(|target| target.get("backend")?.as_str().map(str::to_owned))
            .is_some_and(|backend| backend == "postgresql")
    }

    fn is_mysql_database(&self) -> bool {
        self.database_path
            .as_deref()
            .and_then(|target| serde_json::from_str::<serde_json::Value>(target).ok())
            .and_then(|target| target.get("backend")?.as_str().map(str::to_owned))
            .is_some_and(|backend| backend == "mysql")
    }

    fn supports_sql_returning(&self) -> bool {
        !self.is_mysql_database() && !self.is_mongodb_database()
    }

    fn mongodb_model_row_type(&self, model_name: &str) -> Result<Type, String> {
        let fields = self
            .declared_interfaces
            .get(model_name)
            .ok_or("internal: MongoDB Model interface fields are unavailable")?;
        Ok(tyrus_core::db::mongo_model_result_type(fields))
    }

    fn emit_db_async_query(
        &mut self,
        operation: u32,
        sql: &Expr,
        parameters: &[Expr],
        result_type: Type,
        row_type: Option<&Type>,
        count_sql: Option<&str>,
        count_parameters: &[Expr],
        page_size: Option<&Expr>,
        page_number: Option<&Expr>,
        after: Option<&Expr>,
        cursor_field: Option<&str>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        self.emit_db_async_query_with_transaction(
            operation,
            sql,
            parameters,
            result_type,
            row_type,
            count_sql,
            count_parameters,
            page_size,
            page_number,
            after,
            cursor_field,
            None,
        )
    }

    fn emit_db_async_query_with_transaction(
        &mut self,
        operation: u32,
        sql: &Expr,
        parameters: &[Expr],
        result_type: Type,
        row_type: Option<&Type>,
        count_sql: Option<&str>,
        count_parameters: &[Expr],
        page_size: Option<&Expr>,
        page_number: Option<&Expr>,
        after: Option<&Expr>,
        cursor_field: Option<&str>,
        transaction_id: Option<&Expr>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let target = self
            .database_path
            .as_ref()
            .ok_or("database must be configured before calling `db.query` or a model method")?
            .clone();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let encoded_parameters = self.emit_db_encode_parameters(parameters, "db.async.params")?;
        let encoded_count_parameters = if count_parameters.is_empty() {
            self.const_string_abi("[]", "db.async.count.params")
                .into_struct_value()
        } else {
            self.emit_db_encode_parameters(count_parameters, "db.async.count.params")?
        };
        let sql = match sql.kind() {
            ExprKind::StrLit(value) if value.contains("?/*__tyrus_uuid_parameter__*/") => {
                let replacement = if self.is_postgresql_database() {
                    "CAST(? AS UUID)"
                } else {
                    "?"
                };
                Expr::synthetic(ExprKind::StrLit(
                    value.replace("?/*__tyrus_uuid_parameter__*/", replacement),
                ))
            }
            _ => sql.clone(),
        };
        let sql = if let Some(row_type) = row_type {
            self.emit_db_read_sql(&sql, row_type, "db.async.sql")?
        } else {
            self.emit_expr_as(&sql, &Type::Str)?.into_struct_value()
        };
        let path = self.const_string_abi(&target, "db.async.target");
        let count_sql = self.const_string_abi(count_sql.unwrap_or(""), "db.async.count.sql");
        let after = if let Some(value) = after {
            self.emit_expr_as(value, &Type::Str)?.into_struct_value()
        } else {
            self.const_string_abi("", "db.async.after")
                .into_struct_value()
        };
        let cursor_field = self
            .const_string_abi(cursor_field.unwrap_or(""), "db.async.cursor.field")
            .into_struct_value();
        let page_size = if let Some(value) = page_size {
            self.emit_expr_as(value, &Type::U64)?.into_int_value()
        } else {
            self.context.i64_type().const_zero()
        };
        let page_number = if let Some(value) = page_number {
            self.emit_expr_as(value, &Type::U64)?.into_int_value()
        } else {
            self.context.i64_type().const_zero()
        };
        let spill_string = |this: &Self, value, name| -> Result<_, String> {
            let slot = this.entry_alloca(this.string_abi_type(), name)?;
            this.builder
                .build_store(slot, value)
                .map_err(|error| error.to_string())?;
            Ok(slot)
        };
        let path_ptr = spill_string(self, path, "db.async.target.slot")?;
        let sql_ptr = spill_string(self, sql.into(), "db.async.sql.slot")?;
        let parameters_ptr = spill_string(self, encoded_parameters.into(), "db.async.params.slot")?;
        let count_sql_ptr = spill_string(self, count_sql, "db.async.count.sql.slot")?;
        let count_parameters_ptr = spill_string(
            self,
            encoded_count_parameters.into(),
            "db.async.count.params.slot",
        )?;
        let after_ptr = spill_string(self, after.into(), "db.async.after.slot")?;
        let cursor_field_ptr =
            spill_string(self, cursor_field.into(), "db.async.cursor.field.slot")?;
        let transaction_id_value = if let Some(transaction_id) = transaction_id {
            self.emit_expr_as(transaction_id, &Type::U64)?
                .into_int_value()
        } else {
            self.context.i64_type().const_zero()
        };
        let promise = self
            .builder
            .build_call(
                self.declare_libc_fn(
                    "tyrus_db_async_query",
                    &[
                        self.context.i32_type().into(),
                        pointer.into(),
                        pointer.into(),
                        pointer.into(),
                        pointer.into(),
                        pointer.into(),
                        pointer.into(),
                        pointer.into(),
                        self.context.i64_type().into(),
                        self.context.i64_type().into(),
                        self.context.i64_type().into(),
                    ],
                    Some(pointer.into()),
                ),
                &[
                    self.context
                        .i32_type()
                        .const_int(u64::from(operation), false)
                        .into(),
                    path_ptr.into(),
                    sql_ptr.into(),
                    parameters_ptr.into(),
                    count_sql_ptr.into(),
                    count_parameters_ptr.into(),
                    after_ptr.into(),
                    cursor_field_ptr.into(),
                    page_size.into(),
                    page_number.into(),
                    transaction_id_value.into(),
                ],
                "db.async.promise",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: async database dispatcher returned no Promise")?;
        self.emit_string_release(encoded_parameters)?;
        self.emit_string_release(encoded_count_parameters)?;
        self.emit_string_release(after)?;
        self.emit_string_release(cursor_field)?;
        let promise_type = Type::Apply {
            name: "DbPromise".to_string(),
            args: vec![result_type],
        };
        self.record_owned_native_result(promise, promise_type);
        Ok(promise)
    }

    fn emit_db_encode_parameters(
        &mut self,
        parameters: &[Expr],
        name: &str,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let builder = self
            .builder
            .build_call(
                self.declare_libc_fn("tyrus_json_array_builder_new", &[], Some(pointer.into())),
                &[],
                &format!("{name}.builder"),
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: database parameter builder returned no handle")?
            .into_pointer_value();
        for parameter in parameters {
            let ty = self
                .expr_type_hint(parameter)
                .ok_or("internal: missing type for database parameter")?;
            let value = self.emit_expr(parameter)?;
            if let Type::Optional(inner) = &ty {
                self.emit_db_optional_parameter(
                    builder,
                    value.into_pointer_value(),
                    inner,
                    self.is_mongodb_database(),
                )?;
            } else {
                self.emit_db_parameter_value(builder, value, &ty, self.is_mongodb_database())?;
            }
        }
        let encoded = self
            .builder
            .build_call(
                self.declare_libc_fn(
                    "tyrus_json_array_builder_finish",
                    &[pointer.into()],
                    Some(self.string_abi_type()),
                ),
                &[builder.into()],
                &format!("{name}.encoded"),
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: database parameter encoder returned no text")?
            .into_struct_value();
        self.builder
            .build_call(
                self.declare_libc_fn("tyrus_json_array_builder_free", &[pointer.into()], None),
                &[builder.into()],
                &format!("{name}.builder.free"),
            )
            .map_err(|error| error.to_string())?;
        Ok(encoded)
    }

    fn emit_db_optional_parameter(
        &mut self,
        builder: inkwell::values::PointerValue<'ctx>,
        optional: inkwell::values::PointerValue<'ctx>,
        inner: &Type,
        mongodb: bool,
    ) -> Result<(), String> {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: SQL parameter outside a function")?;
        let present = self
            .context
            .append_basic_block(function, "db.param.present");
        let absent = self.context.append_basic_block(function, "db.param.absent");
        let done = self.context.append_basic_block(function, "db.param.done");
        let has_value = self
            .builder
            .build_is_not_null(optional, "db.param.has_value")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(has_value, present, absent)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(present);
        let value = self
            .builder
            .build_load(self.llvm_type(inner), optional, "db.param.value")
            .map_err(|error| error.to_string())?;
        self.emit_db_parameter_value(builder, value, inner, mongodb)?;
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(absent);
        let add = self.declare_libc_fn(
            "tyrus_json_array_builder_add_raw",
            &[
                self.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into(),
                self.string_abi_type(),
            ],
            Some(self.context.i32_type().into()),
        );
        let null = self.const_string_abi("null", "db.param.null");
        self.builder
            .build_call(add, &[builder.into(), null.into()], "db.param.add_null")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(done);
        Ok(())
    }

    fn emit_db_parameter_value(
        &mut self,
        builder: inkwell::values::PointerValue<'ctx>,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
        mongodb: bool,
    ) -> Result<(), String> {
        let instant_parameter = !mongodb
            && matches!(transparent_codegen_type(ty), Type::Named(name) if name == "Instant");
        if !mongodb && !instant_parameter {
            return self.emit_automatic_json_array_add(builder, value, ty);
        }
        let encoded = match transparent_codegen_type(ty) {
            Type::Array(element) if !matches!(transparent_codegen_type(element), Type::U8) => {
                Some(self.emit_automatic_json_array_encode(value.into_struct_value(), element)?)
            }
            Type::Object(_) => {
                Some(self.emit_automatic_json_encode(value.into_struct_value(), ty, false)?)
            }
            Type::Named(name) if self.declared_fields_with_index(ty).is_some() => {
                let _ = name;
                Some(self.emit_automatic_json_encode(value.into_struct_value(), ty, false)?)
            }
            _ => None,
        };
        let Some(encoded) = encoded else {
            return self.emit_automatic_json_array_add(builder, value, ty);
        };
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let concat = self.declare_libc_fn(
            "tyrus_string_concat",
            &[self.string_abi_type(), self.string_abi_type()],
            Some(self.string_abi_type()),
        );
        let (prefix_text, prefix_name, value_name, suffix_name, add_name) = if instant_parameter {
            (
                "{\"$tyrusInstantParam\":",
                "db.instant.param.prefix",
                "db.instant.param.open",
                "db.instant.param.close",
                "db.instant.param.add",
            )
        } else {
            (
                "{\"$tyrusJson\":",
                "db.mongo.json.prefix",
                "db.mongo.json.open",
                "db.mongo.json.close",
                "db.mongo.json.add",
            )
        };
        let prefix = self.const_string_abi(prefix_text, prefix_name);
        let partial = self
            .builder
            .build_call(concat, &[prefix.into(), encoded.into()], value_name)
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: MongoDB JSON parameter wrapper did not return text")?
            .into_struct_value();
        self.emit_string_release(encoded)?;
        let suffix = self.const_string_abi("}", "db.mongo.json.suffix");
        let wrapped = self
            .builder
            .build_call(concat, &[partial.into(), suffix.into()], suffix_name)
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: MongoDB JSON parameter wrapper did not return text")?
            .into_struct_value();
        self.emit_string_release(partial)?;
        let add = self.declare_libc_fn(
            "tyrus_json_array_builder_add_raw",
            &[pointer.into(), self.string_abi_type()],
            Some(self.context.i32_type().into()),
        );
        self.builder
            .build_call(add, &[builder.into(), wrapped.into()], add_name)
            .map_err(|error| error.to_string())?;
        self.emit_string_release(wrapped)
    }
    fn emit_db_error_value(
        &mut self,
        error_type: &Type,
        code: &str,
        message: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fields = self
            .declared_fields_with_index(error_type)
            .ok_or("internal: DbError interface is unavailable")?;
        let mut value = self.llvm_type(error_type).into_struct_type().const_zero();
        for (index, field) in fields {
            let text = match field.name.as_str() {
                "code" => code,
                "message" => message,
                _ => continue,
            };
            let string = self.const_string_abi(text, "db.decode.error");
            value = self
                .builder
                .build_insert_value(value, string, index, "db.decode.error.field")
                .map_err(|error| error.to_string())?
                .into_struct_value();
        }
        Ok(value.into())
    }

    pub(crate) fn emit_db_async_result_decode(
        &mut self,
        envelope: inkwell::values::StructValue<'ctx>,
        result_type: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let Type::Result { success, error } = result_type else {
            return Err("internal: async database promise payload is not a Result".to_string());
        };
        if **error != Type::Named("DbError".to_string()) {
            return Err("internal: async database Result does not use DbError".to_string());
        }
        let success_is_void = matches!(success.as_ref(), Type::Void);

        let field = |name: &str, ty: Type, optional: bool| Field {
            name: name.to_string(),
            ty,
            readonly: true,
            optional,
            decorators: Vec::new(),
            span: tyrus_core::span::Span::DUMMY,
        };
        let envelope_type = Type::Object(vec![
            field("ok", Type::Bool, false),
            field(
                "value",
                Type::Optional(Box::new(if success_is_void {
                    Type::Bool
                } else {
                    success.as_ref().clone()
                })),
                true,
            ),
            field(
                "error",
                Type::Optional(Box::new(Type::Named("DbError".to_string()))),
                true,
            ),
        ]);
        let decoded_type = Type::Result {
            success: Box::new(envelope_type.clone()),
            error: Box::new(Type::Named("JsonError".to_string())),
        };
        let decoded_variants = vec![envelope_type.clone(), Type::Named("JsonError".to_string())];
        let decoded_slot = self.entry_alloca(
            self.union_llvm_type(&decoded_variants),
            "db.async.envelope.decoded",
        )?;
        let decoded = self.emit_automatic_json_decode_type(&envelope_type, envelope)?;
        self.builder
            .build_store(decoded_slot, decoded)
            .map_err(|error| error.to_string())?;

        let result_variants = vec![(**success).clone(), Type::Named("DbError".to_string())];
        let result_slot =
            self.entry_alloca(self.union_llvm_type(&result_variants), "db.async.result")?;
        self.builder
            .build_store(
                result_slot,
                self.union_llvm_type(&result_variants).const_zero(),
            )
            .map_err(|error| error.to_string())?;
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: database result decode outside a function")?;
        let valid_envelope = self
            .context
            .append_basic_block(function, "db.async.envelope.valid");
        let invalid_envelope = self
            .context
            .append_basic_block(function, "db.async.envelope.invalid");
        let parsed = self.load_union_tag(&decoded_variants, decoded_slot)?;
        let parsed_ok = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                parsed,
                self.context.i8_type().const_zero(),
                "db.async.envelope.parsed",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(parsed_ok, valid_envelope, invalid_envelope)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(invalid_envelope);
        self.emit_async_drop_value_in_place(function, decoded_slot, &decoded_type)?;
        let invalid_error = self.emit_db_error_value(
            &Type::Named("DbError".to_string()),
            "ERR_DB_INVALID_RESULT",
            "database worker returned an invalid result envelope",
        )?;
        self.store_union_variant(&result_variants, result_slot, 1, invalid_error)?;
        let invalid_end = self
            .builder
            .get_insert_block()
            .ok_or("internal: invalid database envelope block disappeared")?;

        self.builder.position_at_end(valid_envelope);
        let envelope_value = self
            .load_union_payload(
                &decoded_variants,
                decoded_slot,
                &envelope_type,
                "db.async.envelope.value",
            )?
            .into_struct_value();
        let fields = self
            .declared_fields_with_index(&envelope_type)
            .ok_or("internal: async database envelope fields are unavailable")?;
        let field_index = |name: &str| {
            fields
                .iter()
                .find(|(_, field)| field.name == name)
                .map(|(index, _)| *index)
                .ok_or_else(|| format!("internal: async database envelope lacks `{name}`"))
        };
        let ok = self
            .builder
            .build_extract_value(envelope_value, field_index("ok")?, "db.async.ok")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let success_value = self
            .builder
            .build_extract_value(envelope_value, field_index("value")?, "db.async.value")
            .map_err(|error| error.to_string())?
            .into_pointer_value();
        let error_value = self
            .builder
            .build_extract_value(envelope_value, field_index("error")?, "db.async.error")
            .map_err(|error| error.to_string())?
            .into_pointer_value();
        let success_block = self
            .context
            .append_basic_block(function, "db.async.result.success");
        let error_block = self
            .context
            .append_basic_block(function, "db.async.result.error");
        let missing_block = self
            .context
            .append_basic_block(function, "db.async.result.missing");
        let success_present = (!success_is_void).then(|| {
            self.context
                .append_basic_block(function, "db.async.success.present")
        });
        let error_present = self
            .context
            .append_basic_block(function, "db.async.error.present");
        let is_success = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::NE,
                ok,
                self.context.bool_type().const_zero(),
                "db.async.is_success",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(is_success, success_block, error_block)
            .map_err(|error| error.to_string())?;

        let success_end = if success_is_void {
            let ignored_value = self
                .context
                .append_basic_block(function, "db.async.success.ignored.value");
            let void_success = self
                .context
                .append_basic_block(function, "db.async.success.void");
            self.builder.position_at_end(success_block);
            let has_ignored_value = self
                .builder
                .build_is_not_null(success_value, "db.async.has_ignored_success")
                .map_err(|error| error.to_string())?;
            self.builder
                .build_conditional_branch(has_ignored_value, ignored_value, void_success)
                .map_err(|error| error.to_string())?;
            self.builder.position_at_end(ignored_value);
            self.builder
                .build_call(
                    self.declare_libc_fn(
                        "free",
                        &[self
                            .context
                            .ptr_type(inkwell::AddressSpace::default())
                            .into()],
                        None,
                    ),
                    &[success_value.into()],
                    "db.async.ignored.success.box.free",
                )
                .map_err(|error| error.to_string())?;
            self.builder
                .build_unconditional_branch(void_success)
                .map_err(|error| error.to_string())?;
            self.builder.position_at_end(void_success);
            self.store_union_variant_tag(self.union_llvm_type(&result_variants), result_slot, 0)?;
            void_success
        } else {
            self.builder.position_at_end(success_block);
            let has_success = self
                .builder
                .build_is_not_null(success_value, "db.async.has_success")
                .map_err(|error| error.to_string())?;
            self.builder
                .build_conditional_branch(
                    has_success,
                    success_present.ok_or("internal: non-void success block is unavailable")?,
                    missing_block,
                )
                .map_err(|error| error.to_string())?;
            self.builder.position_at_end(success_present.unwrap());
            let success_payload = self
                .builder
                .build_load(
                    self.llvm_type(success),
                    success_value,
                    "db.async.success.payload",
                )
                .map_err(|error| error.to_string())?;
            self.builder
                .build_call(
                    self.declare_libc_fn(
                        "free",
                        &[self
                            .context
                            .ptr_type(inkwell::AddressSpace::default())
                            .into()],
                        None,
                    ),
                    &[success_value.into()],
                    "db.async.success.box.free",
                )
                .map_err(|error| error.to_string())?;
            self.store_union_variant(&result_variants, result_slot, 0, success_payload)?;
            self.builder
                .get_insert_block()
                .ok_or("internal: async database success block disappeared")?
        };

        self.builder.position_at_end(error_block);
        let has_error = self
            .builder
            .build_is_not_null(error_value, "db.async.has_error")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(has_error, error_present, missing_block)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(error_present);
        let db_error = self
            .builder
            .build_load(
                self.llvm_type(&Type::Named("DbError".to_string())),
                error_value,
                "db.async.error.payload",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_call(
                self.declare_libc_fn(
                    "free",
                    &[self
                        .context
                        .ptr_type(inkwell::AddressSpace::default())
                        .into()],
                    None,
                ),
                &[error_value.into()],
                "db.async.error.box.free",
            )
            .map_err(|error| error.to_string())?;
        self.store_union_variant(&result_variants, result_slot, 1, db_error)?;
        let error_end = self
            .builder
            .get_insert_block()
            .ok_or("internal: async database error block disappeared")?;

        self.builder.position_at_end(missing_block);
        let missing_error = self.emit_db_error_value(
            &Type::Named("DbError".to_string()),
            "ERR_DB_INVALID_RESULT",
            "database worker result is missing its value or error",
        )?;
        self.store_union_variant(&result_variants, result_slot, 1, missing_error)?;
        let missing_end = self
            .builder
            .get_insert_block()
            .ok_or("internal: missing database result block disappeared")?;

        let merge = self
            .context
            .append_basic_block(function, "db.async.result.merge");
        for block in [invalid_end, success_end, error_end, missing_end] {
            self.builder.position_at_end(block);
            self.builder
                .build_unconditional_branch(merge)
                .map_err(|error| error.to_string())?;
        }
        self.builder.position_at_end(merge);
        let value = self
            .builder
            .build_load(
                self.union_llvm_type(&result_variants),
                result_slot,
                "db.async.result.value",
            )
            .map_err(|error| error.to_string())?;
        Ok(value)
    }

    fn emit_db_read_sql(
        &mut self,
        expression: &Expr,
        result_type: &Type,
        name: &str,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        let ExprKind::StrLit(sql) = expression.kind() else {
            return Ok(self
                .emit_expr_as(expression, &Type::Str)?
                .into_struct_value());
        };
        let query = tyrus_core::db::parse_select_sql(sql).map_err(|error| {
            format!(
                "internal: validated database SELECT became invalid: {}",
                error.message
            )
        })?;
        let fields = self
            .declared_fields(result_type)
            .ok_or("internal: SELECT result interface fields are unavailable")?;
        if fields.is_empty() {
            return Ok(self
                .emit_expr_as(expression, &Type::Str)?
                .into_struct_value());
        }
        let tokens = tyrus_core::db::tokenize_sql(sql).map_err(|error| {
            format!(
                "internal: validated database SELECT became invalid: {}",
                error.message
            )
        })?;
        let model = match result_type {
            Type::Named(type_name) => self
                .db_models
                .get(type_name)
                .filter(|model| model.table == query.table && query.joins.is_empty()),
            _ => None,
        };
        let sql = if query.select_all {
            let star = tokens
                .windows(2)
                .find(|pair| {
                    pair[0].text.eq_ignore_ascii_case("SELECT")
                        && pair[1].kind == tyrus_core::db::SqlTokenKind::Star
                })
                .map(|pair| pair[1].span.clone())
                .ok_or("internal: SELECT * token is missing after successful validation")?;
            let projection = fields
                .iter()
                .map(|field| {
                    let column = model
                        .and_then(|model| {
                            model
                                .fields
                                .iter()
                                .find(|mapped| mapped.field == field.name)
                        })
                        .map_or(field.name.as_str(), |mapped| mapped.column.as_str());
                    let quoted_column = format!("\"{}\"", column.replace('"', "\"\""));
                    let instant_field = is_db_instant_type(&field.ty);
                    let unsigned_field = is_db_u64_type(&field.ty);
                    let result_name = if unsigned_field {
                        format!("{}{}", tyrus_core::db::UNSIGNED_ALIAS_PREFIX, field.name)
                    } else if instant_field {
                        format!("{}{}", tyrus_core::db::INSTANT_ALIAS_PREFIX, field.name)
                    } else {
                        field.name.clone()
                    };
                    if unsigned_field && self.is_mysql_database() {
                        format!(
                            "CAST({quoted_column} AS CHAR) AS \"{}\"",
                            result_name.replace('"', "\"\"")
                        )
                    } else if instant_field && self.is_postgresql_database() {
                        cast_postgresql_instant_projection(&quoted_column, &result_name)
                    } else if instant_field && self.is_mysql_database() {
                        cast_mysql_instant_projection(&quoted_column, &result_name)
                    } else if is_db_uuid_type(&field.ty) && self.is_postgresql_database() {
                        cast_uuid_projection(&quoted_column, &result_name)
                    } else if column == result_name {
                        quoted_column
                    } else {
                        format!(
                            "{quoted_column} AS \"{}\"",
                            result_name.replace('"', "\"\"")
                        )
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}{}{}", &sql[..star.start], projection, &sql[star.end..])
        } else {
            let mut replacements = Vec::new();
            for projection in &query.projections {
                let projected_name = projection.alias.as_deref().unwrap_or(&projection.name);
                let field = fields
                    .iter()
                    .find(|field| field.name == projected_name)
                    .or_else(|| mapped_model_projection_field(&projection.name, &fields, model));
                let Some(field) = field.filter(|field| {
                    is_db_u64_type(&field.ty)
                        || is_db_uuid_type(&field.ty)
                        || is_db_instant_type(&field.ty)
                }) else {
                    continue;
                };
                let source = &sql[projection.span.clone()];
                if is_db_instant_type(&field.ty) {
                    if self.is_postgresql_database() {
                        let end = projection
                            .alias_span
                            .as_ref()
                            .map_or(projection.span.end, |span| span.end);
                        replacements.push((
                            projection.span.start..end,
                            cast_postgresql_instant_projection(
                                source,
                                &format!("{}{}", tyrus_core::db::INSTANT_ALIAS_PREFIX, field.name),
                            ),
                        ));
                    } else if self.is_mysql_database() {
                        let end = projection
                            .alias_span
                            .as_ref()
                            .map_or(projection.span.end, |span| span.end);
                        replacements.push((
                            projection.span.start..end,
                            cast_mysql_instant_projection(
                                source,
                                &format!("{}{}", tyrus_core::db::INSTANT_ALIAS_PREFIX, field.name),
                            ),
                        ));
                    }
                    continue;
                }
                if is_db_uuid_type(&field.ty) {
                    if self.is_postgresql_database() {
                        let end = projection
                            .alias_span
                            .as_ref()
                            .map_or(projection.span.end, |span| span.end);
                        replacements.push((
                            projection.span.start..end,
                            cast_uuid_projection(source, &field.name),
                        ));
                    }
                    continue;
                }
                let alias = format!("{}{}", tyrus_core::db::UNSIGNED_ALIAS_PREFIX, field.name);
                let cast = if self.is_mysql_database() {
                    format!("CAST({source} AS CHAR)")
                } else {
                    source.to_string()
                };
                let end = projection
                    .alias_span
                    .as_ref()
                    .map_or(projection.span.end, |span| span.end);
                replacements.push((projection.span.start..end, format!("{cast} AS \"{alias}\"")));
            }
            let mut rewritten = sql.to_string();
            for (range, replacement) in replacements.into_iter().rev() {
                rewritten.replace_range(range, &replacement);
            }
            rewritten
        };
        Ok(self.const_string_abi(&sql, name).into_struct_value())
    }

    fn emit_db_model_mutation_call(
        &mut self,
        operation: &str,
        type_args: &[Type],
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        if type_args.len() != 1 {
            return Err(format!(
                "internal: checked model `{operation}` call has invalid type arguments"
            ));
        }
        let Type::Named(model_name) = &type_args[0] else {
            return Err(format!(
                "internal: checked model `{operation}` type is invalid"
            ));
        };
        let model = self
            .db_models
            .get(model_name)
            .ok_or("internal: checked model mutation metadata is unavailable")?;
        let required_arguments = if matches!(
            operation,
            "insertOne" | "insertMany" | "deleteOne" | "deleteMany"
        ) {
            1
        } else {
            2
        };
        if args.len() < required_arguments || args.len() > required_arguments + 1 {
            return Err("internal: checked model mutation has invalid argument count".to_string());
        }
        let transaction_id = args.get(required_arguments).and_then(db_transaction_option);
        if self.is_mongodb_database() {
            let mutation = tyrus_core::db::mongo_model_mutation(
                operation,
                model,
                &args[..required_arguments],
                &self.interfaces,
            )
            .map_err(|_| "internal: checked MongoDB model mutation is invalid")?;
            let descriptor = Expr::synthetic(ExprKind::StrLit(mutation.descriptor));
            let fields = self
                .declared_interfaces
                .get(model_name)
                .ok_or("internal: MongoDB Model interface fields are unavailable")?;
            let result_type = Type::Result {
                success: Box::new(
                    tyrus_core::db::mongo_model_mutation_result_type(operation, fields)
                        .ok_or("internal: checked model mutation result type is unavailable")?,
                ),
                error: Box::new(Type::Named("DbError".to_string())),
            };
            return self
                .emit_db_async_query(
                    9,
                    &descriptor,
                    &mutation.parameters,
                    result_type,
                    None,
                    None,
                    &[],
                    None,
                    None,
                    None,
                    None,
                )
                .map(Some);
        }
        let mut query =
            tyrus_core::db::mutation_query(operation, model, &args[..required_arguments])
                .map_err(|_| "internal: checked model mutation arguments are invalid")?;
        let result_success = tyrus_core::db::model_mutation_result_type(operation, model_name)
            .ok_or("internal: checked model mutation result type is unavailable")?;
        let mut operation_code = 0;
        let mut mysql_readback = None;
        let mut mysql_batch = None;
        let mut read_before = false;
        let mut use_inserted_id = false;
        match operation {
            "insertOne" | "updateOne" | "deleteOne" if self.supports_sql_returning() => {
                query.sql.push_str(" RETURNING ");
                query
                    .sql
                    .push_str(&tyrus_core::db::mutation_returning_clause(model));
                operation_code = if transaction_id.is_some() { 15 } else { 1 };
            }
            "insertMany" if self.supports_sql_returning() => {
                query.sql.push_str(" RETURNING ");
                query
                    .sql
                    .push_str(&tyrus_core::db::mutation_returning_clause(model));
                operation_code = if transaction_id.is_some() { 16 } else { 2 };
            }
            "updateMany" | "deleteMany" => {
                operation_code = if transaction_id.is_some() { 21 } else { 20 };
            }
            "insertOne" if self.is_mysql_database() => {
                let id_field = model
                    .fields
                    .iter()
                    .find(|field| field.id)
                    .ok_or("internal: MySQL Model has no id field")?;
                let explicit_id = match args[0].kind() {
                    ExprKind::ObjectLit(properties) => properties
                        .iter()
                        .find(|(name, _)| name == &id_field.field)
                        .map(|(_, value)| value.clone()),
                    _ => None,
                };
                mysql_readback = if let Some(value) = explicit_id {
                    let filter =
                        Expr::synthetic(ExprKind::ObjectLit(vec![(id_field.field.clone(), value)]));
                    Some(tyrus_core::db::find_one_query(model, &filter).map_err(
                        |_| "internal: checked MySQL insert id readback filter is invalid",
                    )?)
                } else {
                    use_inserted_id = true;
                    Some(
                        tyrus_core::db::find_one_by_id_parameter_query(model)
                            .map_err(|_| "internal: checked MySQL insert id readback is invalid")?,
                    )
                };
                operation_code = 22;
            }
            "insertMany" if self.is_mysql_database() => {
                let ExprKind::ArrayLit(rows) = args[0].kind() else {
                    return Err("internal: checked MySQL insertMany rows are invalid".to_string());
                };
                let id_field = model
                    .fields
                    .iter()
                    .find(|field| field.id)
                    .ok_or("internal: MySQL Model has no id field")?;
                let mut statements = Vec::with_capacity(rows.len());
                let mut mutation_parameters = Vec::new();
                let mut read_parameters = Vec::new();
                for row in rows {
                    let mutation = tyrus_core::db::mutation_query(
                        "insertOne",
                        model,
                        std::slice::from_ref(row),
                    )
                    .map_err(|_| "internal: checked MySQL insertMany row is invalid")?;
                    let explicit_id = match row.kind() {
                        ExprKind::ObjectLit(properties) => properties
                            .iter()
                            .find(|(name, _)| name == &id_field.field)
                            .map(|(_, value)| value.clone()),
                        _ => None,
                    };
                    let (readback, use_id) = if let Some(value) = explicit_id {
                        let filter = Expr::synthetic(ExprKind::ObjectLit(vec![(
                            id_field.field.clone(),
                            value,
                        )]));
                        (
                            tyrus_core::db::find_one_query(model, &filter).map_err(
                                |_| "internal: checked MySQL batch readback filter is invalid",
                            )?,
                            false,
                        )
                    } else {
                        (
                            tyrus_core::db::find_one_by_id_parameter_query(model)
                                .map_err(|_| "internal: checked MySQL batch readback is invalid")?,
                            true,
                        )
                    };
                    let readback_sql =
                        tyrus_core::db::mysql_instant_projection_sql(&readback.sql, model);
                    statements.push(serde_json::json!({
                        "sql": mutation.sql,
                        "parameterCount": mutation.parameters.len(),
                        "readSql": readback_sql,
                        "readParameterCount": readback.parameters.len(),
                        "useInsertedId": use_id,
                    }));
                    mutation_parameters.extend(mutation.parameters);
                    read_parameters.extend(readback.parameters);
                }
                mysql_batch = Some((
                    Expr::synthetic(ExprKind::StrLit(
                        serde_json::to_string(&serde_json::json!({ "statements": statements }))
                            .map_err(|_| "internal: MySQL batch descriptor serialization failed")?,
                    )),
                    mutation_parameters,
                    read_parameters,
                ));
                operation_code = 23;
            }
            "updateOne" | "deleteOne" if self.is_mysql_database() => {
                mysql_readback = Some(
                    tyrus_core::db::find_one_query(model, &args[0])
                        .map_err(|_| "internal: checked MySQL mutation readback is invalid")?,
                );
                read_before = operation == "deleteOne";
                operation_code = 22;
            }
            _ => {}
        }
        if operation_code == 0 {
            return Err(format!(
                "internal: `{operation}` needs a backend-specific resource readback path"
            ));
        }
        let query_sql = if self.is_postgresql_database() {
            tyrus_core::db::postgresql_instant_projection_sql(&query.sql, model)
        } else {
            query.sql
        };
        let sql = Expr::synthetic(ExprKind::StrLit(query_sql));
        let result_type = Type::Result {
            success: Box::new(result_success),
            error: Box::new(Type::Named("DbError".to_string())),
        };
        if let Some((descriptor, mutation_parameters, read_parameters)) = mysql_batch {
            return self
                .emit_db_async_query_with_transaction(
                    operation_code,
                    &descriptor,
                    &mutation_parameters,
                    result_type,
                    None,
                    None,
                    &read_parameters,
                    None,
                    None,
                    None,
                    None,
                    transaction_id,
                )
                .map(Some);
        }
        if let Some(readback) = mysql_readback {
            let readback_sql = tyrus_core::db::mysql_instant_projection_sql(&readback.sql, model);
            let cursor_field = if use_inserted_id {
                "insert-id"
            } else if read_before {
                "before"
            } else {
                "after"
            };
            return self
                .emit_db_async_query_with_transaction(
                    operation_code,
                    &sql,
                    &query.parameters,
                    result_type,
                    None,
                    Some(&readback_sql),
                    &readback.parameters,
                    None,
                    None,
                    None,
                    Some(cursor_field),
                    transaction_id,
                )
                .map(Some);
        }
        self.emit_db_async_query_with_transaction(
            operation_code,
            &sql,
            &query.parameters,
            result_type,
            None,
            None,
            &[],
            None,
            None,
            None,
            None,
            transaction_id,
        )
        .map(Some)
    }

    fn emit_db_find_many_call(
        &mut self,
        type_args: &[Type],
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        if !(1..=2).contains(&args.len()) || type_args.len() != 1 {
            return Err(
                "internal: checked model `findMany` call has invalid arguments".to_string(),
            );
        }
        let Type::Named(type_name) = &type_args[0] else {
            return Err(
                "internal: checked model `findMany` type is not a model interface".to_string(),
            );
        };
        let model = self
            .db_models
            .get(type_name)
            .cloned()
            .ok_or("internal: checked model `findMany` metadata is unavailable")?;
        let options: &[(String, Expr)] = if args.len() == 2 {
            let ExprKind::ObjectLit(options) = args[1].kind() else {
                return Err("internal: checked `findMany` options are invalid".to_string());
            };
            options.as_slice()
        } else {
            &[]
        };
        let includes =
            if let Some((_, value)) = options.iter().find(|(name, _)| name == "include") {
                let ExprKind::ArrayLit(items) = value.kind() else {
                    return Err("internal: checked `findMany` include is invalid".to_string());
                };
                items
                    .iter()
                    .map(|item| match item.kind() {
                        ExprKind::StrLit(field) => Ok(tyrus_core::db::DbRelationInclude {
                            field: field.clone(),
                            projection: None,
                        }),
                        ExprKind::ObjectLit(properties) if properties.len() == 1 => {
                            let (field, projection) = &properties[0];
                            let ExprKind::ArrayLit(names) = projection.kind() else {
                                return Err("internal: checked relationship projection is invalid"
                                    .to_string());
                            };
                            let selected =
                                names
                                    .iter()
                                    .map(|name| match name.kind() {
                                        ExprKind::StrLit(name) => Ok(name.clone()),
                                        _ => Err("internal: checked projected field is invalid"
                                            .to_string()),
                                    })
                                    .collect::<Result<Vec<_>, _>>()?;
                            Ok(tyrus_core::db::DbRelationInclude {
                                field: field.clone(),
                                projection: Some(selected),
                            })
                        }
                        _ => Err("internal: checked relationship include is invalid".to_string()),
                    })
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                Vec::new()
            };
        let transaction_id = options
            .iter()
            .find(|(name, _)| name == "transaction")
            .map(|(_, value)| value);
        let mongo_pagination_requested = options.iter().any(|(name, value)| {
            (name == "paginate" || name == "cursor")
                && matches!(value.kind(), ExprKind::BoolLit(true))
        });
        if self.is_mongodb_database() && includes.is_empty() {
            let query = tyrus_core::db::mongo_model_read(
                &model,
                &args[0],
                tyrus_core::db::MongoReadKind::FindMany,
                &self.interfaces,
            )
            .map_err(|_| "internal: checked MongoDB model `findMany` filter is invalid")?;
            let descriptor = Expr::synthetic(ExprKind::StrLit(query.descriptor));
            let row_type = self.mongodb_model_row_type(type_name)?;
            if mongo_pagination_requested {
                let cursor_mode = options.iter().any(|(name, value)| {
                    name == "cursor" && matches!(value.kind(), ExprKind::BoolLit(true))
                });
                let page_size = options
                    .iter()
                    .find(|(name, _)| name == "pageSize")
                    .map(|(_, value)| value)
                    .ok_or("internal: checked MongoDB pagination pageSize is missing")?;
                let page_number = options
                    .iter()
                    .find(|(name, _)| name == "page")
                    .map(|(_, value)| value);
                let after = options
                    .iter()
                    .find(|(name, _)| name == "after")
                    .map(|(_, value)| value);
                let default_page = Expr::synthetic(ExprKind::UIntLit(1));
                let page_number = page_number.or_else(|| (!cursor_mode).then_some(&default_page));
                let result_type = Type::Result {
                    success: Box::new(db_page_result_type(&row_type)),
                    error: Box::new(Type::Named("DbError".to_string())),
                };
                return self
                    .emit_db_async_query(
                        if cursor_mode { 11 } else { 10 },
                        &descriptor,
                        &query.parameters,
                        result_type,
                        None,
                        None,
                        &[],
                        Some(page_size),
                        page_number,
                        after,
                        None,
                    )
                    .map(Some);
            }
            let result_type = Type::Result {
                success: Box::new(Type::Array(Box::new(row_type))),
                error: Box::new(Type::Named("DbError".to_string())),
            };
            return self
                .emit_db_async_query_with_transaction(
                    8,
                    &descriptor,
                    &query.parameters,
                    result_type,
                    None,
                    None,
                    &[],
                    None,
                    None,
                    None,
                    None,
                    transaction_id,
                )
                .map(Some);
        }
        let row_type = if includes.iter().any(|include| include.projection.is_some()) {
            tyrus_core::db::project_model_result_type(
                type_name,
                &self.db_models,
                &self.declared_interfaces,
                &includes,
            )?
        } else {
            Type::Named(type_name.clone())
        };
        let relation_limit = options
            .iter()
            .find(|(name, _)| name == "relationLimit")
            .and_then(|(_, value)| match value.kind() {
                ExprKind::UIntLit(limit) => u64::try_from(*limit).ok(),
                ExprKind::IntLit(limit) => u64::try_from(*limit).ok(),
                _ => None,
            });
        let paginate = options.iter().any(|(name, value)| {
            name == "paginate" && matches!(value.kind(), ExprKind::BoolLit(true))
        });
        if !includes.is_empty() && !paginate {
            let query = tyrus_core::db::find_many_with_relations_query(
                type_name,
                &self.db_models,
                &args[0],
                &includes,
                false,
                false,
                relation_limit,
            )
            .map_err(|_| "internal: checked model relationships are invalid")?;
            let sql = Expr::synthetic(ExprKind::StrLit(query.sql));
            let result_type = Type::Result {
                success: Box::new(Type::Array(Box::new(row_type.clone()))),
                error: Box::new(Type::Named("DbError".to_string())),
            };
            return self
                .emit_db_async_query_with_transaction(
                    if transaction_id.is_some() { 16 } else { 2 },
                    &sql,
                    &query.parameters,
                    result_type,
                    Some(&row_type),
                    None,
                    &[],
                    None,
                    None,
                    None,
                    None,
                    transaction_id,
                )
                .map(Some);
        }
        if paginate {
            let cursor_mode = options.iter().any(|(name, value)| {
                name == "cursor" && matches!(value.kind(), ExprKind::BoolLit(true))
            });
            let query = if !includes.is_empty() {
                tyrus_core::db::find_many_with_relations_query(
                    type_name,
                    &self.db_models,
                    &args[0],
                    &includes,
                    true,
                    cursor_mode,
                    relation_limit,
                )
            } else if cursor_mode {
                tyrus_core::db::find_many_cursor_query(&model, &args[0])
            } else {
                tyrus_core::db::find_many_page_query(&model, &args[0])
            }
            .map_err(|_| "internal: checked model `findMany` filter is invalid")?;
            let page_size = options
                .iter()
                .find(|(name, _)| name == "pageSize")
                .map(|(_, value)| value)
                .ok_or("internal: checked `findMany` pageSize is missing")?;
            let page_number = options
                .iter()
                .find(|(name, _)| name == "page")
                .map(|(_, value)| value);
            let default_page_number = Expr::synthetic(ExprKind::UIntLit(1));
            let page_number =
                page_number.or_else(|| (!cursor_mode).then_some(&default_page_number));
            let after = options
                .iter()
                .find(|(name, _)| name == "after")
                .map(|(_, value)| value);
            let cursor_field = model
                .fields
                .iter()
                .find(|field| field.id)
                .map(|field| field.field.as_str());
            let count = tyrus_core::db::count_query(&model, &args[0])
                .map_err(|_| "internal: checked `findMany` count filter is invalid")?;
            let rows_sql = Expr::synthetic(ExprKind::StrLit(query.sql));
            let result_type = Type::Result {
                success: Box::new(db_page_result_type(&row_type)),
                error: Box::new(Type::Named("DbError".to_string())),
            };
            return self
                .emit_db_async_query_with_transaction(
                    if cursor_mode { 5 } else { 4 },
                    &rows_sql,
                    &query.parameters,
                    result_type,
                    Some(&row_type),
                    Some(&count.sql),
                    &count.parameters,
                    Some(page_size),
                    page_number,
                    after,
                    if cursor_mode { cursor_field } else { None },
                    transaction_id,
                )
                .map(Some);
        }
        let query = tyrus_core::db::find_many_query(&model, &args[0])
            .map_err(|_| "internal: checked model `findMany` filter is invalid")?;
        let sql = Expr::synthetic(ExprKind::StrLit(query.sql));
        let result_type = Type::Result {
            success: Box::new(Type::Array(Box::new(row_type.clone()))),
            error: Box::new(Type::Named("DbError".to_string())),
        };
        self.emit_db_async_query_with_transaction(
            if transaction_id.is_some() { 16 } else { 2 },
            &sql,
            &query.parameters,
            result_type,
            Some(&row_type),
            None,
            &[],
            None,
            None,
            None,
            None,
            transaction_id,
        )
        .map(Some)
    }

    pub(crate) fn emit_db_call(
        &mut self,
        callee: &str,
        type_args: &[Type],
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        if callee == tyrus_core::db::BEGIN_TRANSACTION_TARGET {
            let sql = Expr::synthetic(ExprKind::StrLit(String::new()));
            let result_type = Type::Result {
                success: Box::new(Type::U64),
                error: Box::new(Type::Named("DbError".to_string())),
            };
            return self
                .emit_db_async_query(
                    13,
                    &sql,
                    &[],
                    result_type,
                    None,
                    None,
                    &[],
                    None,
                    None,
                    None,
                    None,
                )
                .map(Some);
        }
        if matches!(
            callee,
            tyrus_core::db::COMMIT_TRANSACTION_TARGET | tyrus_core::db::ROLLBACK_TRANSACTION_TARGET
        ) {
            let transaction_id = args
                .first()
                .ok_or("internal: checked database transaction handle is missing")?;
            let sql = Expr::synthetic(ExprKind::StrLit(String::new()));
            let result_type = Type::Result {
                success: Box::new(Type::Bool),
                error: Box::new(Type::Named("DbError".to_string())),
            };
            return self
                .emit_db_async_query_with_transaction(
                    if callee == tyrus_core::db::COMMIT_TRANSACTION_TARGET {
                        17
                    } else {
                        18
                    },
                    &sql,
                    &[],
                    result_type,
                    None,
                    None,
                    &[],
                    None,
                    None,
                    None,
                    None,
                    Some(transaction_id),
                )
                .map(Some);
        }
        if callee == tyrus_core::db::COUNT_TARGET {
            if type_args.len() != 1 || !(1..=2).contains(&args.len()) {
                return Err(
                    "internal: checked model `count` call has invalid type arguments".to_string(),
                );
            }
            let Type::Named(model_name) = &type_args[0] else {
                return Err("internal: checked model `count` type is invalid".to_string());
            };
            let model = self
                .db_models
                .get(model_name)
                .ok_or("internal: checked model `count` metadata is unavailable")?;
            let transaction_id = args.get(1).and_then(db_transaction_option);
            if self.is_mongodb_database() {
                let query = tyrus_core::db::mongo_model_read(
                    model,
                    &args[0],
                    tyrus_core::db::MongoReadKind::Count,
                    &self.interfaces,
                )
                .map_err(|_| "internal: checked MongoDB model `count` filter is invalid")?;
                let descriptor = Expr::synthetic(ExprKind::StrLit(query.descriptor));
                let result_type = Type::Result {
                    success: Box::new(Type::U64),
                    error: Box::new(Type::Named("DbError".to_string())),
                };
                return self
                    .emit_db_async_query(
                        6,
                        &descriptor,
                        &query.parameters,
                        result_type,
                        None,
                        None,
                        &[],
                        None,
                        None,
                        None,
                        None,
                    )
                    .map(Some);
            }
            let query = tyrus_core::db::count_query(model, &args[0])
                .map_err(|_| "internal: checked model `count` filter is invalid")?;
            let sql = Expr::synthetic(ExprKind::StrLit(query.sql));
            let result_type = Type::Result {
                success: Box::new(Type::U64),
                error: Box::new(Type::Named("DbError".to_string())),
            };
            return self
                .emit_db_async_query_with_transaction(
                    if transaction_id.is_some() { 19 } else { 3 },
                    &sql,
                    &query.parameters,
                    result_type,
                    None,
                    None,
                    &[],
                    None,
                    None,
                    None,
                    None,
                    transaction_id,
                )
                .map(Some);
        }
        if callee == tyrus_core::db::FIND_ONE_TARGET {
            if !(1..=2).contains(&args.len()) || type_args.len() != 1 {
                return Err(
                    "internal: checked model `findOne` call has invalid arguments".to_string(),
                );
            }
            let Type::Named(model_name) = &type_args[0] else {
                return Err("internal: checked model `findOne` type is invalid".to_string());
            };
            let model = self
                .db_models
                .get(model_name)
                .ok_or("internal: checked model `findOne` metadata is unavailable")?;
            let transaction_id = args.get(1).and_then(db_transaction_option);
            if self.is_mongodb_database() {
                let query = tyrus_core::db::mongo_model_read(
                    model,
                    &args[0],
                    tyrus_core::db::MongoReadKind::FindOne,
                    &self.interfaces,
                )
                .map_err(|_| "internal: checked MongoDB model `findOne` filter is invalid")?;
                let descriptor = Expr::synthetic(ExprKind::StrLit(query.descriptor));
                let row_type = self.mongodb_model_row_type(model_name)?;
                let result_type = Type::Result {
                    success: Box::new(Type::Optional(Box::new(row_type))),
                    error: Box::new(Type::Named("DbError".to_string())),
                };
                return self
                    .emit_db_async_query(
                        7,
                        &descriptor,
                        &query.parameters,
                        result_type,
                        None,
                        None,
                        &[],
                        None,
                        None,
                        None,
                        None,
                    )
                    .map(Some);
            }
            let query = tyrus_core::db::find_one_query(model, &args[0])
                .map_err(|_| "internal: checked model `findOne` filter is invalid")?;
            let sql = Expr::synthetic(ExprKind::StrLit(query.sql));
            let result_type = Type::Result {
                success: Box::new(Type::Optional(Box::new(type_args[0].clone()))),
                error: Box::new(Type::Named("DbError".to_string())),
            };
            return self
                .emit_db_async_query_with_transaction(
                    if transaction_id.is_some() { 15 } else { 1 },
                    &sql,
                    &query.parameters,
                    result_type,
                    Some(&type_args[0]),
                    None,
                    &[],
                    None,
                    None,
                    None,
                    None,
                    transaction_id,
                )
                .map(Some);
        }
        if let Some(operation) = tyrus_core::db::model_method_name(callee) {
            return self.emit_db_model_mutation_call(operation, type_args, args);
        }
        if callee == tyrus_core::db::FIND_MANY_TARGET {
            return self.emit_db_find_many_call(type_args, args);
        }
        if callee == tyrus_core::db::QUERY_TARGET
            && matches!(args.first().map(Expr::kind), Some(ExprKind::StrLit(sql)) if tyrus_core::db::tokenize_sql(sql).ok().and_then(|tokens| tokens.first().map(|token| token.text.eq_ignore_ascii_case("SELECT"))).unwrap_or(false))
        {
            let Some(row_type) = type_args.first() else {
                return Err("internal: checked SQL SELECT result type is unavailable".to_string());
            };
            let Some(parameters_expr) = args.get(1) else {
                return Err("internal: checked SQL parameters are missing".to_string());
            };
            let ExprKind::ArrayLit(parameters) = parameters_expr.kind() else {
                return Err("internal: checked SQL parameters are not an array literal".to_string());
            };
            let result_type = Type::Result {
                success: Box::new(Type::Array(Box::new(row_type.clone()))),
                error: Box::new(Type::Named("DbError".to_string())),
            };
            let transaction_id = args.get(2).and_then(|options| match options.kind() {
                ExprKind::ObjectLit(properties) => properties
                    .iter()
                    .find(|(name, _)| name == "transaction")
                    .map(|(_, value)| value),
                _ => None,
            });
            return self
                .emit_db_async_query_with_transaction(
                    if transaction_id.is_some() { 16 } else { 2 },
                    &args[0],
                    parameters,
                    result_type,
                    Some(row_type),
                    None,
                    &[],
                    None,
                    None,
                    None,
                    None,
                    transaction_id,
                )
                .map(Some);
        }
        if callee != tyrus_core::db::QUERY_TARGET {
            return Ok(None);
        }
        if !(2..=3).contains(&args.len()) {
            return Err("internal: checked `db.query` call has wrong arity".to_string());
        }
        let ExprKind::ArrayLit(parameters) = args[1].kind() else {
            return Err(
                "internal: checked `db.query` parameters are not an array literal".to_string(),
            );
        };
        let parameters = parameters.clone();
        let result_type = Type::Result {
            success: Box::new(Type::Void),
            error: Box::new(Type::Named("DbError".to_string())),
        };
        let transaction_id = args.get(2).and_then(|options| match options.kind() {
            ExprKind::ObjectLit(properties) => properties
                .iter()
                .find(|(name, _)| name == "transaction")
                .map(|(_, value)| value),
            _ => None,
        });
        self.emit_db_async_query_with_transaction(
            if transaction_id.is_some() { 14 } else { 0 },
            &args[0],
            &parameters,
            result_type,
            None,
            None,
            &[],
            None,
            None,
            None,
            None,
            transaction_id,
        )
        .map(Some)
    }
}

fn db_page_result_type(row_type: &Type) -> Type {
    let fields = tyrus_core::builtin_interfaces::db_page_fields()
        .into_iter()
        .map(|mut field| {
            if field.name == "items" {
                field.ty = Type::Array(Box::new(row_type.clone()));
            }
            field
        })
        .collect();
    Type::Object(fields)
}

fn db_transaction_option(options: &Expr) -> Option<&Expr> {
    let ExprKind::ObjectLit(properties) = options.kind() else {
        return None;
    };
    properties
        .iter()
        .find(|(name, _)| name == "transaction")
        .map(|(_, transaction)| transaction)
}

fn is_db_instant_type(ty: &Type) -> bool {
    match ty {
        Type::Named(name) => name == "Instant",
        Type::Optional(inner) | Type::Readonly(inner) => is_db_instant_type(inner),
        _ => false,
    }
}

fn mapped_model_projection_field<'a>(
    projection_name: &str,
    fields: &'a [Field],
    model: Option<&tyrus_core::ast::DbModelMetadata>,
) -> Option<&'a Field> {
    let column_name = projection_name
        .rsplit_once('.')
        .map_or(projection_name, |(_, column)| column);
    let mapped = model?
        .fields
        .iter()
        .find(|mapped| mapped.column.eq_ignore_ascii_case(column_name))?;
    fields.iter().find(|field| {
        field.name == mapped.field
            && (is_db_u64_type(&field.ty)
                || is_db_uuid_type(&field.ty)
                || is_db_instant_type(&field.ty))
    })
}

fn is_db_u64_type(ty: &Type) -> bool {
    match ty {
        Type::U64 => true,
        Type::Optional(inner) | Type::Readonly(inner) => is_db_u64_type(inner),
        _ => false,
    }
}

fn is_db_uuid_type(ty: &Type) -> bool {
    match ty {
        Type::Named(name) => name == "Uuid" || name == "Uuid7",
        Type::Optional(inner) | Type::Readonly(inner) => is_db_uuid_type(inner),
        _ => false,
    }
}

fn cast_uuid_projection(source: &str, alias: &str) -> String {
    format!(
        "CAST({source} AS TEXT) AS \"{}\"",
        alias.replace('"', "\"\"")
    )
}

fn cast_postgresql_instant_projection(source: &str, alias: &str) -> String {
    format!(
        "CAST(CAST({source} AS TIMESTAMP) AS TEXT) AS \"{}\"",
        alias.replace('"', "\"\""),
    )
}

fn cast_mysql_instant_projection(source: &str, alias: &str) -> String {
    format!(
        "CAST({source} AS CHAR) AS \"{}\"",
        alias.replace('"', "\"\""),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        cast_mysql_instant_projection, cast_postgresql_instant_projection, cast_uuid_projection,
        is_db_uuid_type, mapped_model_projection_field,
    };
    use tyrus_core::ast::{DbModelFieldMetadata, DbModelMetadata, Field, Type};
    use tyrus_core::span::Span;

    #[test]
    fn postgres_uuid_projection_is_cast_to_text_and_keeps_its_result_name() {
        assert_eq!(
            cast_uuid_projection("\"user_id\"", "id"),
            "CAST(\"user_id\" AS TEXT) AS \"id\""
        );
        assert_eq!(
            cast_uuid_projection("\"user_id\"", "user\"id"),
            "CAST(\"user_id\" AS TEXT) AS \"user\"\"id\""
        );
        assert!(is_db_uuid_type(&Type::Named("Uuid".to_string())));
        assert!(is_db_uuid_type(&Type::Named("Uuid7".to_string())));
        assert!(is_db_uuid_type(&Type::Optional(Box::new(Type::Named(
            "Uuid".to_string()
        )))));
        assert!(!is_db_uuid_type(&Type::Str));
    }

    #[test]
    fn postgres_instant_projection_uses_a_textual_utc_value() {
        assert_eq!(
            cast_postgresql_instant_projection("\"createdAt\"", "__tyrus_instant__createdAt"),
            "CAST(CAST(\"createdAt\" AS TIMESTAMP) AS TEXT) AS \"__tyrus_instant__createdAt\""
        );
    }

    #[test]
    fn mysql_instant_projection_is_cast_to_text_with_an_internal_alias() {
        assert_eq!(
            cast_mysql_instant_projection(
                "`__tyrus_root`.`createdAt`",
                "__tyrus_instant__createdAt",
            ),
            "CAST(`__tyrus_root`.`createdAt` AS CHAR) AS \"__tyrus_instant__createdAt\""
        );
    }

    #[test]
    fn maps_postgresql_instant_projection_alias_back_to_model_field() {
        let field = Field {
            name: "createdAt".to_string(),
            ty: Type::Named("Instant".to_string()),
            readonly: false,
            optional: false,
            decorators: Vec::new(),
            span: Span::DUMMY,
        };
        let model = DbModelMetadata {
            table: "users".to_string(),
            span: Span::DUMMY,
            timestamps: true,
            snake_case: false,
            fields: vec![DbModelFieldMetadata {
                field: "createdAt".to_string(),
                column: "createdAt".to_string(),
                ty: Type::Named("Instant".to_string()),
                id: false,
                id_strategy: None,
                unique: false,
                default_value: None,
                foreign_key: None,
                span: Span::DUMMY,
            }],
            relations: Vec::new(),
        };

        assert_eq!(
            mapped_model_projection_field("createdAt", std::slice::from_ref(&field), Some(&model)),
            Some(&field)
        );
        assert_eq!(
            mapped_model_projection_field(
                "__tyrus_root.createdAt",
                std::slice::from_ref(&field),
                Some(&model),
            ),
            Some(&field)
        );
    }
}
