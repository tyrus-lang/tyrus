use super::*;
use tyrus_core::ast::ExprKind;
use tyrus_core::validators::{normalize_decorator_set, validator_display, validator_message};

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_parameter_validator_call(
        &mut self,
        owner: &str,
        target: &str,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
        decorators: &[ValidatorDecorator],
    ) -> Result<(), String> {
        if decorators.is_empty() {
            return Ok(());
        }
        let resolved = self.resolve_type(ty);
        let function =
            self.ensure_parameter_validator_function(owner, target, &resolved, decorators)?;
        self.builder
            .build_call(function, &[value.into()], "validator.param")
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub(super) fn emit_named_object_validator_call(
        &mut self,
        name: &str,
        value: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        let fields = self
            .declared_fields_with_index(&Type::Named(name.to_string()))
            .ok_or_else(|| format!("error: unknown interface `{name}`"))?;
        if fields.iter().all(|(_, field)| field.decorators.is_empty()) {
            return Ok(());
        }
        self.emit_object_value_validator_checks(value.into_struct_value(), &fields)
    }

    pub(super) fn ensure_named_object_validation_predicate_function(
        &mut self,
        name: &str,
    ) -> Result<FunctionValue<'ctx>, String> {
        let fields = self
            .declared_fields_with_index(&Type::Named(name.to_string()))
            .ok_or_else(|| format!("error: unknown interface `{name}`"))?;
        let symbol = format!("tyrus_validate_{name}_ok");
        if let Some(function) = self.module.get_function(&symbol) {
            return Ok(function);
        }

        let object_type = Type::Named(name.to_string());
        let llvm_type = self.llvm_type(&object_type);
        let bool_type = self.context.bool_type();
        let function_type = bool_type.fn_type(&[llvm_type.into()], false);
        let function = self.module.add_function(&symbol, function_type, None);
        let previous_block = self.builder.get_insert_block();
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        let result = (|| -> Result<(), String> {
            let value = function
                .get_first_param()
                .ok_or_else(|| format!("internal: validator `{symbol}` has no object parameter"))?
                .into_struct_value();
            self.emit_object_value_validator_predicate_checks(value, &fields)?;
            if !self.current_block_terminated() {
                self.builder
                    .build_return(Some(&bool_type.const_int(1, false)))
                    .unwrap();
            }
            Ok(())
        })();

        if let Some(block) = previous_block {
            self.builder.position_at_end(block);
        }
        result?;
        Ok(function)
    }

    pub(super) fn field_decorators(
        &mut self,
        object: &Expr,
        field: &str,
    ) -> Option<(Vec<ValidatorDecorator>, Type)> {
        let object_ty = self.expr_type_hint(object)?;
        let Type::Named(name) = transparent_codegen_type(&self.resolve_type(&object_ty)).clone()
        else {
            return None;
        };
        let declared = self
            .interfaces
            .get(name.as_str())?
            .iter()
            .find(|candidate| candidate.name == field)?;
        if declared.decorators.is_empty() {
            return None;
        }
        Some((declared.decorators.clone(), field_storage_type(declared)))
    }

    pub(super) fn emit_assigned_field_validator_checks(
        &mut self,
        object: &Expr,
        field: &str,
        stored: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        let Some((decorators, ty)) = self.field_decorators(object, field) else {
            return Ok(());
        };
        let owns_collection = self.validation_depth == 0;
        if owns_collection {
            self.emit_validation_issues_begin();
        }
        self.emit_validator_checks(stored, &ty, &decorators, field)?;
        if owns_collection {
            self.emit_validation_issues_finish();
        }
        Ok(())
    }

    pub(super) fn emit_object_value_validator_checks(
        &mut self,
        value: StructValue<'ctx>,
        fields: &[(u32, Field)],
    ) -> Result<(), String> {
        let owns_collection = self.validation_depth == 0;
        if owns_collection {
            self.emit_validation_issues_begin();
        }
        for (index, field) in fields {
            if field.decorators.is_empty() {
                continue;
            }
            let storage_type = field_storage_type(field);
            let field_value = self
                .builder
                .build_extract_value(value, *index, &field.name)
                .map_err(|error| error.to_string())?;
            self.emit_validator_checks(field_value, &storage_type, &field.decorators, &field.name)?;
        }
        if owns_collection {
            self.emit_validation_issues_finish();
        }
        Ok(())
    }

    fn emit_object_value_validator_predicate_checks(
        &mut self,
        value: StructValue<'ctx>,
        fields: &[(u32, Field)],
    ) -> Result<(), String> {
        for (index, field) in fields {
            if field.decorators.is_empty() {
                continue;
            }
            let storage_type = field_storage_type(field);
            let field_value = self
                .builder
                .build_extract_value(value, *index, &field.name)
                .map_err(|error| error.to_string())?;
            self.emit_validator_predicate_checks(
                field_value,
                &storage_type,
                &field.decorators,
                &field.name,
            )?;
        }
        Ok(())
    }

    fn ensure_parameter_validator_function(
        &mut self,
        owner: &str,
        target: &str,
        ty: &Type,
        decorators: &[ValidatorDecorator],
    ) -> Result<FunctionValue<'ctx>, String> {
        let symbol = format!(
            "tyrus_validate_param_{}_{}_{}",
            sanitize_validator_symbol(owner),
            sanitize_validator_symbol(target),
            mangle_type(ty)
        );
        if let Some(function) = self.module.get_function(&symbol) {
            return Ok(function);
        }

        let llvm_type = self.llvm_type(ty);
        let function_type = self.context.void_type().fn_type(&[llvm_type.into()], false);
        let function = self.module.add_function(&symbol, function_type, None);
        let previous_block = self.builder.get_insert_block();
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        let result = (|| -> Result<(), String> {
            let value = function
                .get_first_param()
                .ok_or_else(|| format!("internal: validator `{symbol}` has no parameter value"))?;
            self.emit_validator_checks(value, ty, decorators, target)?;
            if !self.current_block_terminated() {
                self.builder.build_return(None).unwrap();
            }
            Ok(())
        })();

        if let Some(block) = previous_block {
            self.builder.position_at_end(block);
        }
        result?;
        Ok(function)
    }

    pub(super) fn emit_validator_checks(
        &mut self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
        decorators: &[ValidatorDecorator],
        target: &str,
    ) -> Result<(), String> {
        if decorators.is_empty() {
            return Ok(());
        }
        let resolved = self.resolve_type(ty);
        if let Type::Optional(inner) = transparent_codegen_type(&resolved).clone() {
            return self
                .emit_optional_validator_checks(value, &inner, &resolved, decorators, target);
        }
        let decorators = normalize_decorator_set(decorators);
        for decorator in &decorators {
            if self.user_validator_names.contains(&decorator.name) {
                let (invalid, result, result_type) =
                    self.user_validator_result(value, &resolved, decorator)?;
                self.emit_user_validator_failure_if(
                    invalid,
                    result,
                    &result_type,
                    decorator,
                    target,
                    false,
                )?;
                continue;
            }
            self.emit_membership_validator_check(value, &resolved, decorator, target)?;
            self.emit_boolean_validator_check(value, &resolved, decorator, target)?;
            self.emit_numeric_validator_check(value, &resolved, decorator, target)?;
            self.emit_string_validator_check(value, &resolved, decorator, target)?;
            self.emit_array_validator_check(value, &resolved, decorator, target)?;
            self.emit_presence_validator_check(value, &resolved, decorator, target)?;
        }
        Ok(())
    }

    fn emit_optional_validator_checks(
        &mut self,
        value: BasicValueEnum<'ctx>,
        inner: &Type,
        optional_ty: &Type,
        decorators: &[ValidatorDecorator],
        target: &str,
    ) -> Result<(), String> {
        let (presence, payload): (Vec<_>, Vec<_>) = normalize_decorator_set(decorators)
            .into_iter()
            .partition(|decorator| matches!(decorator.name.as_str(), "notNull" | "required"));
        for decorator in &presence {
            self.emit_presence_validator_check(value, optional_ty, decorator, target)?;
        }
        if payload.is_empty() || !value.is_pointer_value() {
            return Ok(());
        }
        let pointer = value.into_pointer_value();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: optional validator emitted outside a function")?;
        let present = self
            .context
            .append_basic_block(function, "validator.present");
        let done = self
            .context
            .append_basic_block(function, "validator.absent");
        let is_absent = self
            .builder
            .build_is_null(pointer, "validator.is_absent")
            .unwrap();
        self.builder
            .build_conditional_branch(is_absent, done, present)
            .unwrap();

        self.builder.position_at_end(present);
        let payload_value = self
            .builder
            .build_load(self.llvm_type(inner), pointer, "validator.payload")
            .map_err(|error| error.to_string())?;
        self.emit_validator_checks(payload_value, inner, &payload, target)?;
        if !self.current_block_terminated() {
            self.builder.build_unconditional_branch(done).unwrap();
        }
        self.builder.position_at_end(done);
        Ok(())
    }

    fn emit_validator_predicate_checks(
        &mut self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
        decorators: &[ValidatorDecorator],
        target: &str,
    ) -> Result<(), String> {
        if decorators.is_empty() {
            return Ok(());
        }
        let resolved = self.resolve_type(ty);
        if let Type::Optional(inner) = transparent_codegen_type(&resolved).clone() {
            return self.emit_optional_validator_predicate_checks(
                value, &inner, &resolved, decorators, target,
            );
        }
        let decorators = normalize_decorator_set(decorators);
        for decorator in &decorators {
            if self.user_validator_names.contains(&decorator.name) {
                let (invalid, result, result_type) =
                    self.user_validator_result(value, &resolved, decorator)?;
                self.emit_user_validator_failure_if(
                    invalid,
                    result,
                    &result_type,
                    decorator,
                    target,
                    true,
                )?;
                continue;
            }
            if let Some(invalid) =
                self.membership_validator_invalid_condition(value, &resolved, decorator)?
            {
                self.emit_issue_and_return_false_if(invalid, decorator, target);
            }
            if let Some(invalid) =
                self.boolean_validator_invalid_condition(value, &resolved, decorator)
            {
                self.emit_issue_and_return_false_if(invalid, decorator, target);
            }
            if let Some(invalid) =
                self.numeric_validator_invalid_condition(value, &resolved, decorator)?
            {
                self.emit_issue_and_return_false_if(invalid, decorator, target);
            }
            if matches!(transparent_codegen_type(&resolved), Type::Str)
                && value.is_struct_value()
                && let Some(invalid) =
                    self.string_validator_invalid_condition(value.into_struct_value(), decorator)?
            {
                self.emit_issue_and_return_false_if(invalid, decorator, target);
            }
            if let Some(invalid) =
                self.array_validator_invalid_condition(value, &resolved, decorator)?
            {
                self.emit_issue_and_return_false_if(invalid, decorator, target);
            }
            if let Some(invalid) =
                self.presence_validator_invalid_condition(value, &resolved, decorator)
            {
                self.emit_issue_and_return_false_if(invalid, decorator, target);
            }
        }
        Ok(())
    }

    fn emit_optional_validator_predicate_checks(
        &mut self,
        value: BasicValueEnum<'ctx>,
        inner: &Type,
        optional_ty: &Type,
        decorators: &[ValidatorDecorator],
        target: &str,
    ) -> Result<(), String> {
        let (presence, payload): (Vec<_>, Vec<_>) = normalize_decorator_set(decorators)
            .into_iter()
            .partition(|decorator| matches!(decorator.name.as_str(), "notNull" | "required"));
        for decorator in &presence {
            if let Some(invalid) =
                self.presence_validator_invalid_condition(value, optional_ty, decorator)
            {
                self.emit_issue_and_return_false_if(invalid, decorator, target);
            }
        }
        if payload.is_empty() || !value.is_pointer_value() {
            return Ok(());
        }
        let pointer = value.into_pointer_value();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: optional validator emitted outside a function")?;
        let present = self
            .context
            .append_basic_block(function, "validator.pred_present");
        let done = self
            .context
            .append_basic_block(function, "validator.pred_absent");
        let is_absent = self
            .builder
            .build_is_null(pointer, "validator.pred_is_absent")
            .unwrap();
        self.builder
            .build_conditional_branch(is_absent, done, present)
            .unwrap();

        self.builder.position_at_end(present);
        let payload_value = self
            .builder
            .build_load(self.llvm_type(inner), pointer, "validator.pred_payload")
            .map_err(|error| error.to_string())?;
        self.emit_validator_predicate_checks(payload_value, inner, &payload, target)?;
        if !self.current_block_terminated() {
            self.builder.build_unconditional_branch(done).unwrap();
        }
        self.builder.position_at_end(done);
        Ok(())
    }

    fn user_validator_result(
        &mut self,
        value: BasicValueEnum<'ctx>,
        source_type: &Type,
        decorator: &ValidatorDecorator,
    ) -> Result<(IntValue<'ctx>, PointerValue<'ctx>, Type), String> {
        let function = self.fns.get(&decorator.name).copied().ok_or_else(|| {
            format!(
                "internal: custom validator `{}` was not declared",
                decorator.name
            )
        })?;
        let (parameter_types, return_type) = self
            .fn_signatures
            .get(&decorator.name)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "internal: custom validator `{}` has no signature",
                    decorator.name
                )
            })?;
        let Some(value_parameter) = parameter_types.first() else {
            return Err(format!(
                "internal: custom validator `{}` has no value parameter",
                decorator.name
            ));
        };
        if decorator.args.len() + 1 != parameter_types.len() {
            return Err(format!(
                "internal: custom validator `{}` received {} configuration argument(s), expected {}",
                decorator.name,
                decorator.args.len(),
                parameter_types.len().saturating_sub(1)
            ));
        }
        let value_argument = self.coerce_value_with_source(
            &Expr::synthetic(ExprKind::Null),
            value,
            Some(source_type),
            value_parameter,
        )?;
        let mut arguments = Vec::with_capacity(parameter_types.len());
        arguments.push(value_argument.into());
        for (argument, parameter_type) in decorator.args.iter().zip(parameter_types.iter().skip(1))
        {
            arguments.push(self.emit_expr_as(argument, parameter_type)?.into());
        }
        let return_type = self.resolve_type(&return_type);
        let Type::Union(variants) = transparent_codegen_type(&return_type) else {
            return Err(format!(
                "internal: custom validator `{}` does not return a Validator union",
                decorator.name
            ));
        };
        let result = self
            .builder
            .build_call(function, &arguments, "validator.custom.result")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| {
                format!(
                    "internal: custom validator `{}` returned no value",
                    decorator.name
                )
            })?;
        let result_ptr =
            self.entry_alloca(self.llvm_type(&return_type), "validator.custom.storage")?;
        self.builder
            .build_store(result_ptr, result)
            .map_err(|error| error.to_string())?;
        let is_valid = self
            .emit_union_common_field(variants, result_ptr, "isValid")?
            .into_int_value();
        let invalid = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                is_valid,
                is_valid.get_type().const_zero(),
                "validator.custom.invalid",
            )
            .map_err(|error| error.to_string())?;
        Ok((invalid, result_ptr, return_type))
    }

    fn emit_user_validator_failure_if(
        &mut self,
        invalid: IntValue<'ctx>,
        result: PointerValue<'ctx>,
        result_type: &Type,
        decorator: &ValidatorDecorator,
        target: &str,
        return_false: bool,
    ) -> Result<(), String> {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: custom validator emitted outside a function")?;
        let failed = self
            .context
            .append_basic_block(function, "validator.custom.failed");
        let passed = self
            .context
            .append_basic_block(function, "validator.custom.passed");
        self.builder
            .build_conditional_branch(invalid, failed, passed)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(failed);
        let detail = self.validator_result_message(result, result_type)?;
        let validator = validator_display(decorator);
        let target = self.validation_target(target);
        let target_value = self
            .const_string_abi(&target, "validation.target")
            .into_struct_value();
        let validator_value = self
            .const_string_abi(&validator, "validation.validator")
            .into_struct_value();
        if self.env_validation_mode {
            let format = self.declare_libc_fn(
                "tyrus_validation_format_issue",
                &[
                    self.string_abi_type(),
                    self.string_abi_type(),
                    self.string_abi_type(),
                ],
                Some(self.string_abi_type()),
            );
            let message = self
                .builder
                .build_call(
                    format,
                    &[target_value.into(), validator_value.into(), detail.into()],
                    "validation.message",
                )
                .map_err(|error| error.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or("internal: validation formatter returned no message")?
                .into_struct_value();
            let panic =
                self.declare_libc_fn("tyrus_panic_validation", &[self.string_abi_type()], None);
            self.builder
                .build_call(panic, &[message.into()], "")
                .map_err(|error| error.to_string())?;
            self.builder
                .build_unreachable()
                .map_err(|error| error.to_string())?;
        } else {
            let add = self.declare_libc_fn(
                "tyrus_validation_issue_add",
                &[
                    self.string_abi_type(),
                    self.string_abi_type(),
                    self.string_abi_type(),
                ],
                None,
            );
            self.builder
                .build_call(
                    add,
                    &[target_value.into(), validator_value.into(), detail.into()],
                    "",
                )
                .map_err(|error| error.to_string())?;
            if return_false {
                self.builder
                    .build_return(Some(&self.context.bool_type().const_zero()))
                    .map_err(|error| error.to_string())?;
            } else {
                self.builder
                    .build_unconditional_branch(passed)
                    .map_err(|error| error.to_string())?;
            }
        }
        self.builder.position_at_end(passed);
        Ok(())
    }

    fn validator_result_message(
        &mut self,
        result: PointerValue<'ctx>,
        result_type: &Type,
    ) -> Result<StructValue<'ctx>, String> {
        let Type::Union(variants) = transparent_codegen_type(result_type) else {
            return Err("internal: Validator result is not a union".to_string());
        };
        let (invalid_variant, message_index) = variants
            .iter()
            .find_map(|variant| {
                let Type::Object(fields) = variant else {
                    return None;
                };
                let is_invalid = fields.iter().any(|field| {
                    field.name == "isValid"
                        && matches!(
                            &field.ty,
                            Type::Lit(value) if matches!(value.kind(), ExprKind::BoolLit(false))
                        )
                });
                let message_index = fields.iter().position(|field| field.name == "message");
                if is_invalid {
                    message_index.map(|index| (variant, index))
                } else {
                    None
                }
            })
            .ok_or("internal: Validator union has no invalid message variant")?;
        let payload = self
            .load_union_payload(
                variants,
                result,
                invalid_variant,
                "validator.custom.message.object",
            )?
            .into_struct_value();
        self.builder
            .build_extract_value(payload, message_index as u32, "validator.custom.message")
            .map_err(|error| error.to_string())
            .map(BasicValueEnum::into_struct_value)
    }

    fn custom_validator_message(decorator: &ValidatorDecorator) -> String {
        validator_message(decorator)
            .or_else(|| {
                decorator
                    .args
                    .first()
                    .and_then(|argument| match argument.kind() {
                        ExprKind::StrLit(value) => Some(value.clone()),
                        _ => None,
                    })
            })
            .unwrap_or_else(|| "value is invalid".to_string())
    }

    fn emit_issue_and_return_false_if(
        &self,
        invalid: IntValue<'ctx>,
        decorator: &ValidatorDecorator,
        target: &str,
    ) {
        let validator = validator_display(decorator);
        let detail = Self::custom_validator_message(decorator);
        let current_function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .expect("validator predicate emitted outside function");
        let failed = self
            .context
            .append_basic_block(current_function, "validator.failed");
        let passed = self
            .context
            .append_basic_block(current_function, "validator.passed");
        self.builder
            .build_conditional_branch(invalid, failed, passed)
            .unwrap();
        self.builder.position_at_end(failed);
        let target = self.validation_target(target);
        let target = self
            .const_string_abi(&target, "validation.target")
            .into_struct_value();
        let validator = self
            .const_string_abi(&validator, "validation.validator")
            .into_struct_value();
        let detail = self
            .const_string_abi(&detail, "validation.detail")
            .into_struct_value();
        let add = self.declare_libc_fn(
            "tyrus_validation_issue_add",
            &[
                self.string_abi_type(),
                self.string_abi_type(),
                self.string_abi_type(),
            ],
            None,
        );
        self.builder
            .build_call(add, &[target.into(), validator.into(), detail.into()], "")
            .unwrap();
        self.builder
            .build_return(Some(&self.context.bool_type().const_zero()))
            .unwrap();
        self.builder.position_at_end(passed);
    }

    fn emit_membership_validator_check(
        &self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
        decorator: &ValidatorDecorator,
        target: &str,
    ) -> Result<(), String> {
        let Some(invalid) = self.membership_validator_invalid_condition(value, ty, decorator)?
        else {
            return Ok(());
        };
        self.emit_validator_failure_if(invalid, decorator, target);
        Ok(())
    }

    fn emit_numeric_validator_check(
        &self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
        decorator: &ValidatorDecorator,
        target: &str,
    ) -> Result<(), String> {
        let Some(invalid) = self.numeric_validator_invalid_condition(value, ty, decorator)? else {
            return Ok(());
        };
        self.emit_validator_failure_if(invalid, decorator, target);
        Ok(())
    }

    fn emit_boolean_validator_check(
        &self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
        decorator: &ValidatorDecorator,
        target: &str,
    ) -> Result<(), String> {
        let Some(invalid) = self.boolean_validator_invalid_condition(value, ty, decorator) else {
            return Ok(());
        };
        self.emit_validator_failure_if(invalid, decorator, target);
        Ok(())
    }

    fn emit_string_validator_check(
        &self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
        decorator: &ValidatorDecorator,
        target: &str,
    ) -> Result<(), String> {
        if !matches!(transparent_codegen_type(ty), Type::Str) || !value.is_struct_value() {
            return Ok(());
        }
        let Some(invalid) =
            self.string_validator_invalid_condition(value.into_struct_value(), decorator)?
        else {
            return Ok(());
        };
        self.emit_validator_failure_if(invalid, decorator, target);
        Ok(())
    }

    fn emit_array_validator_check(
        &mut self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
        decorator: &ValidatorDecorator,
        target: &str,
    ) -> Result<(), String> {
        let Some(invalid) = self.array_validator_invalid_condition(value, ty, decorator)? else {
            return Ok(());
        };
        self.emit_validator_failure_if(invalid, decorator, target);
        Ok(())
    }

    fn emit_presence_validator_check(
        &self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
        decorator: &ValidatorDecorator,
        target: &str,
    ) -> Result<(), String> {
        let Some(invalid) = self.presence_validator_invalid_condition(value, ty, decorator) else {
            return Ok(());
        };
        self.emit_validator_failure_if(invalid, decorator, target);
        Ok(())
    }

    fn array_validator_invalid_condition(
        &mut self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
        decorator: &ValidatorDecorator,
    ) -> Result<Option<IntValue<'ctx>>, String> {
        let Type::Array(element) = transparent_codegen_type(ty) else {
            return Ok(None);
        };
        if !value.is_struct_value() {
            return Ok(None);
        }
        let array = value.into_struct_value();
        let length = self
            .builder
            .build_extract_value(array, 1, "validator.array_len")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let invalid = match decorator.name.as_str() {
            "minItems" => self.integer_arg(&decorator.args, 0).map(|min| {
                self.builder
                    .build_int_compare(
                        IntPredicate::SLT,
                        length,
                        length.get_type().const_int(min as u64, true),
                        "validator.min_items",
                    )
                    .unwrap()
            }),
            "maxItems" => self.integer_arg(&decorator.args, 0).map(|max| {
                self.builder
                    .build_int_compare(
                        IntPredicate::SGT,
                        length,
                        length.get_type().const_int(max as u64, true),
                        "validator.max_items",
                    )
                    .unwrap()
            }),
            "lengthItems" => self.integer_arg(&decorator.args, 0).map(|expected| {
                self.builder
                    .build_int_compare(
                        IntPredicate::NE,
                        length,
                        length.get_type().const_int(expected as u64, true),
                        "validator.length_items",
                    )
                    .unwrap()
            }),
            "notEmptyItems" => Some(
                self.builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        length,
                        length.get_type().const_zero(),
                        "validator.not_empty_items",
                    )
                    .unwrap(),
            ),
            "unique" => self
                .emit_array_unique_predicate(array, element.as_ref())?
                .map(|valid| self.invert_runtime_predicate(valid)),
            _ => None,
        };
        Ok(invalid)
    }

    fn presence_validator_invalid_condition(
        &self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
        decorator: &ValidatorDecorator,
    ) -> Option<IntValue<'ctx>> {
        if !matches!(decorator.name.as_str(), "notNull" | "required")
            || !matches!(transparent_codegen_type(ty), Type::Optional(_))
            || !value.is_pointer_value()
        {
            return None;
        }
        Some(
            self.builder
                .build_is_null(value.into_pointer_value(), "validator.required")
                .unwrap(),
        )
    }

    fn string_validator_invalid_condition(
        &self,
        value: StructValue<'ctx>,
        decorator: &ValidatorDecorator,
    ) -> Result<Option<IntValue<'ctx>>, String> {
        let i64_type = self.context.i64_type();
        let invalid = match decorator.name.as_str() {
            "minLength" => self.integer_arg(&decorator.args, 0).map(|min| {
                let length = self.emit_string_length(value);
                self.builder
                    .build_int_compare(
                        IntPredicate::SLT,
                        length,
                        i64_type.const_int(min as u64, true),
                        "validator.min_length",
                    )
                    .unwrap()
            }),
            "maxLength" => self.integer_arg(&decorator.args, 0).map(|max| {
                let length = self.emit_string_length(value);
                self.builder
                    .build_int_compare(
                        IntPredicate::SGT,
                        length,
                        i64_type.const_int(max as u64, true),
                        "validator.max_length",
                    )
                    .unwrap()
            }),
            "length" => self.integer_arg(&decorator.args, 0).map(|expected| {
                let length = self.emit_string_length(value);
                self.builder
                    .build_int_compare(
                        IntPredicate::NE,
                        length,
                        i64_type.const_int(expected as u64, true),
                        "validator.length",
                    )
                    .unwrap()
            }),
            "notEmpty" => {
                let length = self.emit_string_length(value);
                Some(
                    self.builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            length,
                            i64_type.const_zero(),
                            "validator.not_empty",
                        )
                        .unwrap(),
                )
            }
            "notBlank" => self
                .emit_string_predicate(value, "tyrus_string_not_blank")
                .map(|valid| self.invert_runtime_predicate(valid)),
            "isTrimmed" => self
                .emit_string_predicate(value, "tyrus_string_is_trimmed")
                .map(|valid| self.invert_runtime_predicate(valid)),
            "isLowercase" => self
                .emit_string_predicate(value, "tyrus_string_is_lowercase")
                .map(|valid| self.invert_runtime_predicate(valid)),
            "isUppercase" => self
                .emit_string_predicate(value, "tyrus_string_is_uppercase")
                .map(|valid| self.invert_runtime_predicate(valid)),
            "startsWith" | "endsWith" | "includes" => {
                let Some(search) = self.string_arg_pointer(&decorator.args, 0) else {
                    return Ok(None);
                };
                let runtime_name = match decorator.name.as_str() {
                    "startsWith" => "tyrus_string_starts_with",
                    "endsWith" => "tyrus_string_ends_with",
                    "includes" => "tyrus_string_includes",
                    _ => unreachable!(),
                };
                let function = self.declare_libc_fn(
                    runtime_name,
                    &[self.string_abi_type(), self.string_abi_type()],
                    Some(i64_type.into()),
                );
                let result = self
                    .builder
                    .build_call(function, &[value.into(), search.into()], "validator.string")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                Some(self.invert_runtime_predicate(result))
            }
            "email" => self
                .emit_string_predicate(value, "tyrus_string_is_email")
                .map(|valid| self.invert_runtime_predicate(valid)),
            "url" => self
                .emit_string_predicate(value, "tyrus_string_is_url")
                .map(|valid| self.invert_runtime_predicate(valid)),
            "uuid" => {
                let version = self.integer_arg(&decorator.args, 0).map_or_else(
                    || i64_type.const_zero(),
                    |value| i64_type.const_int(value as u64, true),
                );
                let function = self.declare_libc_fn(
                    "tyrus_string_is_uuid",
                    &[self.string_abi_type(), i64_type.into()],
                    Some(i64_type.into()),
                );
                let result = self
                    .builder
                    .build_call(function, &[value.into(), version.into()], "validator.uuid")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                Some(self.invert_runtime_predicate(result))
            }
            "ip" => self
                .emit_string_predicate(value, "tyrus_string_is_ip")
                .map(|valid| self.invert_runtime_predicate(valid)),
            "ipv4" => self
                .emit_string_predicate(value, "tyrus_string_is_ipv4")
                .map(|valid| self.invert_runtime_predicate(valid)),
            "ipv6" => self
                .emit_string_predicate(value, "tyrus_string_is_ipv6")
                .map(|valid| self.invert_runtime_predicate(valid)),
            "e164" => self
                .emit_string_predicate(value, "tyrus_string_is_e164")
                .map(|valid| self.invert_runtime_predicate(valid)),
            "cep" => self
                .emit_string_predicate(value, "tyrus_string_is_cep")
                .map(|valid| self.invert_runtime_predicate(valid)),
            "cpf" => self
                .emit_string_predicate(value, "tyrus_string_is_cpf")
                .map(|valid| self.invert_runtime_predicate(valid)),
            "cnpj" => self
                .emit_string_predicate(value, "tyrus_string_is_cnpj")
                .map(|valid| self.invert_runtime_predicate(valid)),
            "equal" => self.string_arg_pointer(&decorator.args, 0).map(|expected| {
                let equal = self.emit_validator_string_equality(value, expected, "validator.equal");
                self.builder.build_not(equal, "validator.equal").unwrap()
            }),
            "notEqual" => self.string_arg_pointer(&decorator.args, 0).map(|expected| {
                self.emit_validator_string_equality(value, expected, "validator.not_equal")
            }),
            "regex" => {
                let Some(pattern) = self.regex_arg_pointer(&decorator.args, 0) else {
                    return Ok(None);
                };
                let function = self.declare_libc_fn(
                    "tyrus_string_matches_regex",
                    &[self.string_abi_type(), self.string_abi_type()],
                    Some(i64_type.into()),
                );
                let result = self
                    .builder
                    .build_call(function, &[value.into(), pattern.into()], "validator.regex")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                Some(self.invert_runtime_predicate(result))
            }
            _ => None,
        };
        Ok(invalid)
    }

    fn numeric_validator_invalid_condition(
        &self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
        decorator: &ValidatorDecorator,
    ) -> Result<Option<IntValue<'ctx>>, String> {
        if is_integer_type(ty) && value.is_int_value() {
            return self.integer_validator_invalid_condition(value.into_int_value(), ty, decorator);
        }
        if is_float_type(ty) && value.is_float_value() {
            return self.float_validator_invalid_condition(value.into_float_value(), ty, decorator);
        }
        Ok(None)
    }

    fn boolean_validator_invalid_condition(
        &self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
        decorator: &ValidatorDecorator,
    ) -> Option<IntValue<'ctx>> {
        if !matches!(transparent_codegen_type(ty), Type::Bool) || !value.is_int_value() {
            return None;
        }
        let expected = self.bool_arg(&decorator.args, 0)?;
        let expected = self.context.bool_type().const_int(expected as u64, false);
        let predicate = match decorator.name.as_str() {
            "equal" => IntPredicate::NE,
            "notEqual" => IntPredicate::EQ,
            _ => return None,
        };
        Some(
            self.builder
                .build_int_compare(
                    predicate,
                    value.into_int_value(),
                    expected,
                    "validator.bool",
                )
                .unwrap(),
        )
    }

    fn membership_validator_invalid_condition(
        &self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
        decorator: &ValidatorDecorator,
    ) -> Result<Option<IntValue<'ctx>>, String> {
        if !matches!(decorator.name.as_str(), "in" | "notIn") {
            return Ok(None);
        }
        let Some(Expr {
            kind: ExprKind::ArrayLit(values),
            ..
        }) = decorator.args.first()
        else {
            return Ok(None);
        };
        let bool_type = self.context.bool_type();
        let mut matched = bool_type.const_zero();
        for candidate in values {
            let Some(candidate_match) = self.literal_membership_match(value, ty, candidate)? else {
                continue;
            };
            matched = self
                .builder
                .build_or(matched, candidate_match, "validator.membership")
                .unwrap();
        }
        let invalid = if decorator.name == "in" {
            self.builder
                .build_int_compare(
                    IntPredicate::EQ,
                    matched,
                    bool_type.const_zero(),
                    "validator.in",
                )
                .unwrap()
        } else {
            matched
        };
        Ok(Some(invalid))
    }

    fn literal_membership_match(
        &self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
        candidate: &Expr,
    ) -> Result<Option<IntValue<'ctx>>, String> {
        let ty = transparent_codegen_type(ty);
        if matches!(ty, Type::Str) && self.is_string_value(value) {
            let Expr {
                kind: ExprKind::StrLit(expected),
                ..
            } = candidate
            else {
                return Ok(None);
            };
            let expected = self
                .const_string_abi(expected, "validator.in_string")
                .into_struct_value();
            return Ok(Some(self.emit_string_equals(
                value.into_struct_value(),
                expected,
                "validator.in_string",
            )?));
        }
        if matches!(ty, Type::Bool) && value.is_int_value() {
            let Expr {
                kind: ExprKind::BoolLit(expected),
                ..
            } = candidate
            else {
                return Ok(None);
            };
            return Ok(Some(
                self.builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        value.into_int_value(),
                        self.context.bool_type().const_int(*expected as u64, false),
                        "validator.in_bool",
                    )
                    .unwrap(),
            ));
        }
        if is_integer_type(ty) && value.is_int_value() {
            let Some(expected) = self.integer_expr(candidate) else {
                return Ok(None);
            };
            let value = value.into_int_value();
            return Ok(Some(
                self.builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        value,
                        value
                            .get_type()
                            .const_int(expected as u64, !is_unsigned_int_type(ty)),
                        "validator.in_integer",
                    )
                    .unwrap(),
            ));
        }
        if is_float_type(ty) && value.is_float_value() {
            let Some(expected) = self.float_expr(candidate) else {
                return Ok(None);
            };
            let value = value.into_float_value();
            return Ok(Some(
                self.builder
                    .build_float_compare(
                        inkwell::FloatPredicate::OEQ,
                        value,
                        value.get_type().const_float(expected),
                        "validator.in_float",
                    )
                    .unwrap(),
            ));
        }
        Ok(None)
    }

    fn integer_validator_invalid_condition(
        &self,
        value: IntValue<'ctx>,
        ty: &Type,
        decorator: &ValidatorDecorator,
    ) -> Result<Option<IntValue<'ctx>>, String> {
        let signed = !is_unsigned_int_type(ty);
        let invalid = match decorator.name.as_str() {
            "min" => self.integer_arg(&decorator.args, 0).map(|min| {
                let min = value.get_type().const_int(min as u64, signed);
                self.builder
                    .build_int_compare(
                        self.int_less_than_predicate(signed),
                        value,
                        min,
                        "validator.min",
                    )
                    .unwrap()
            }),
            "max" => self.integer_arg(&decorator.args, 0).map(|max| {
                let max = value.get_type().const_int(max as u64, signed);
                self.builder
                    .build_int_compare(
                        self.int_greater_than_predicate(signed),
                        value,
                        max,
                        "validator.max",
                    )
                    .unwrap()
            }),
            "range" => {
                let Some(min) = self.integer_arg(&decorator.args, 0) else {
                    return Ok(None);
                };
                let Some(max) = self.integer_arg(&decorator.args, 1) else {
                    return Ok(None);
                };
                let min = value.get_type().const_int(min as u64, signed);
                let max = value.get_type().const_int(max as u64, signed);
                let below = self
                    .builder
                    .build_int_compare(
                        self.int_less_than_predicate(signed),
                        value,
                        min,
                        "validator.range.min",
                    )
                    .unwrap();
                let above = self
                    .builder
                    .build_int_compare(
                        self.int_greater_than_predicate(signed),
                        value,
                        max,
                        "validator.range.max",
                    )
                    .unwrap();
                Some(
                    self.builder
                        .build_or(below, above, "validator.range")
                        .unwrap(),
                )
            }
            "positive" => Some(
                self.builder
                    .build_int_compare(
                        self.int_less_equal_predicate(signed),
                        value,
                        value.get_type().const_zero(),
                        "validator.positive",
                    )
                    .unwrap(),
            ),
            "negative" => Some(
                self.builder
                    .build_int_compare(
                        self.int_greater_equal_predicate(signed),
                        value,
                        value.get_type().const_zero(),
                        "validator.negative",
                    )
                    .unwrap(),
            ),
            "nonNegative" => Some(
                self.builder
                    .build_int_compare(
                        self.int_less_than_predicate(signed),
                        value,
                        value.get_type().const_zero(),
                        "validator.non_negative",
                    )
                    .unwrap(),
            ),
            "nonPositive" => Some(
                self.builder
                    .build_int_compare(
                        self.int_greater_than_predicate(signed),
                        value,
                        value.get_type().const_zero(),
                        "validator.non_positive",
                    )
                    .unwrap(),
            ),
            "equal" => self.integer_arg(&decorator.args, 0).map(|expected| {
                let expected = value.get_type().const_int(expected as u64, signed);
                self.builder
                    .build_int_compare(IntPredicate::NE, value, expected, "validator.equal")
                    .unwrap()
            }),
            "notEqual" => self.integer_arg(&decorator.args, 0).map(|expected| {
                let expected = value.get_type().const_int(expected as u64, signed);
                self.builder
                    .build_int_compare(IntPredicate::EQ, value, expected, "validator.not_equal")
                    .unwrap()
            }),
            "multipleOf" => self.integer_arg(&decorator.args, 0).and_then(|divisor| {
                if divisor == 0 {
                    return None;
                }
                let divisor = value.get_type().const_int(divisor as u64, signed);
                let rem = if signed {
                    self.builder
                        .build_int_signed_rem(value, divisor, "validator.rem")
                        .unwrap()
                } else {
                    self.builder
                        .build_int_unsigned_rem(value, divisor, "validator.rem")
                        .unwrap()
                };
                Some(
                    self.builder
                        .build_int_compare(
                            IntPredicate::NE,
                            rem,
                            value.get_type().const_zero(),
                            "validator.multiple_of",
                        )
                        .unwrap(),
                )
            }),
            "integer" | "safeInteger" => Some(self.context.bool_type().const_zero()),
            "powerOfTwo" => {
                let zero = value.get_type().const_zero();
                let one = value.get_type().const_int(1, false);
                let positive = self
                    .builder
                    .build_int_compare(
                        self.int_greater_than_predicate(signed),
                        value,
                        zero,
                        "validator.power_of_two.positive",
                    )
                    .unwrap();
                let previous = self
                    .builder
                    .build_int_sub(value, one, "validator.power_of_two.previous")
                    .unwrap();
                let masked = self
                    .builder
                    .build_and(value, previous, "validator.power_of_two.mask")
                    .unwrap();
                let single_bit = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        masked,
                        zero,
                        "validator.power_of_two.single_bit",
                    )
                    .unwrap();
                let valid = self
                    .builder
                    .build_and(positive, single_bit, "validator.power_of_two.valid")
                    .unwrap();
                Some(self.invert_runtime_predicate(valid))
            }
            "port" => {
                let min = value.get_type().const_int(1024, signed);
                let max = value.get_type().const_int(49_151, signed);
                let below = self
                    .builder
                    .build_int_compare(
                        self.int_less_than_predicate(signed),
                        value,
                        min,
                        "validator.port.min",
                    )
                    .unwrap();
                let above = self
                    .builder
                    .build_int_compare(
                        self.int_greater_than_predicate(signed),
                        value,
                        max,
                        "validator.port.max",
                    )
                    .unwrap();
                Some(
                    self.builder
                        .build_or(below, above, "validator.port")
                        .unwrap(),
                )
            }
            "statusCode" => {
                let min = value.get_type().const_int(100, signed);
                let max = value.get_type().const_int(599, signed);
                let below = self
                    .builder
                    .build_int_compare(
                        self.int_less_than_predicate(signed),
                        value,
                        min,
                        "validator.status_code.min",
                    )
                    .unwrap();
                let above = self
                    .builder
                    .build_int_compare(
                        self.int_greater_than_predicate(signed),
                        value,
                        max,
                        "validator.status_code.max",
                    )
                    .unwrap();
                Some(
                    self.builder
                        .build_or(below, above, "validator.status_code")
                        .unwrap(),
                )
            }
            _ => None,
        };
        Ok(invalid)
    }

    fn float_validator_invalid_condition(
        &self,
        value: FloatValue<'ctx>,
        ty: &Type,
        decorator: &ValidatorDecorator,
    ) -> Result<Option<IntValue<'ctx>>, String> {
        let invalid = match decorator.name.as_str() {
            "min" => self.float_arg(&decorator.args, 0).map(|min| {
                self.builder
                    .build_float_compare(
                        inkwell::FloatPredicate::OLT,
                        value,
                        value.get_type().const_float(min),
                        "validator.min",
                    )
                    .unwrap()
            }),
            "max" => self.float_arg(&decorator.args, 0).map(|max| {
                self.builder
                    .build_float_compare(
                        inkwell::FloatPredicate::OGT,
                        value,
                        value.get_type().const_float(max),
                        "validator.max",
                    )
                    .unwrap()
            }),
            "range" => {
                let Some(min) = self.float_arg(&decorator.args, 0) else {
                    return Ok(None);
                };
                let Some(max) = self.float_arg(&decorator.args, 1) else {
                    return Ok(None);
                };
                let below = self
                    .builder
                    .build_float_compare(
                        inkwell::FloatPredicate::OLT,
                        value,
                        value.get_type().const_float(min),
                        "validator.range.min",
                    )
                    .unwrap();
                let above = self
                    .builder
                    .build_float_compare(
                        inkwell::FloatPredicate::OGT,
                        value,
                        value.get_type().const_float(max),
                        "validator.range.max",
                    )
                    .unwrap();
                Some(
                    self.builder
                        .build_or(below, above, "validator.range")
                        .unwrap(),
                )
            }
            "positive" => Some(
                self.builder
                    .build_float_compare(
                        inkwell::FloatPredicate::OLE,
                        value,
                        value.get_type().const_float(0.0),
                        "validator.positive",
                    )
                    .unwrap(),
            ),
            "negative" => Some(
                self.builder
                    .build_float_compare(
                        inkwell::FloatPredicate::OGE,
                        value,
                        value.get_type().const_float(0.0),
                        "validator.negative",
                    )
                    .unwrap(),
            ),
            "nonNegative" => Some(
                self.builder
                    .build_float_compare(
                        inkwell::FloatPredicate::OLT,
                        value,
                        value.get_type().const_float(0.0),
                        "validator.non_negative",
                    )
                    .unwrap(),
            ),
            "nonPositive" => Some(
                self.builder
                    .build_float_compare(
                        inkwell::FloatPredicate::OGT,
                        value,
                        value.get_type().const_float(0.0),
                        "validator.non_positive",
                    )
                    .unwrap(),
            ),
            "equal" => self.float_arg(&decorator.args, 0).map(|expected| {
                self.builder
                    .build_float_compare(
                        inkwell::FloatPredicate::ONE,
                        value,
                        value.get_type().const_float(expected),
                        "validator.equal",
                    )
                    .unwrap()
            }),
            "notEqual" => self.float_arg(&decorator.args, 0).map(|expected| {
                self.builder
                    .build_float_compare(
                        inkwell::FloatPredicate::OEQ,
                        value,
                        value.get_type().const_float(expected),
                        "validator.not_equal",
                    )
                    .unwrap()
            }),
            "multipleOf" => self.float_arg(&decorator.args, 0).and_then(|divisor| {
                if divisor == 0.0 {
                    return None;
                }
                let rem = self
                    .builder
                    .build_float_rem(
                        value,
                        value.get_type().const_float(divisor),
                        "validator.rem",
                    )
                    .unwrap();
                Some(
                    self.builder
                        .build_float_compare(
                            inkwell::FloatPredicate::ONE,
                            rem,
                            value.get_type().const_float(0.0),
                            "validator.multiple_of",
                        )
                        .unwrap(),
                )
            }),
            "integer" | "safeInteger" => {
                let rem = self
                    .builder
                    .build_float_rem(
                        value,
                        value.get_type().const_float(1.0),
                        "validator.int.rem",
                    )
                    .unwrap();
                let not_integer = self
                    .builder
                    .build_float_compare(
                        inkwell::FloatPredicate::UNE,
                        rem,
                        value.get_type().const_float(0.0),
                        "validator.integer",
                    )
                    .unwrap();
                if decorator.name == "integer" {
                    Some(not_integer)
                } else {
                    let limit = if matches!(transparent_codegen_type(ty), Type::F32) {
                        16_777_216.0
                    } else {
                        9_007_199_254_740_992.0
                    };
                    let below = self
                        .builder
                        .build_float_compare(
                            inkwell::FloatPredicate::OLT,
                            value,
                            value.get_type().const_float(-limit),
                            "validator.safe_integer.min",
                        )
                        .unwrap();
                    let above = self
                        .builder
                        .build_float_compare(
                            inkwell::FloatPredicate::OGT,
                            value,
                            value.get_type().const_float(limit),
                            "validator.safe_integer.max",
                        )
                        .unwrap();
                    let unordered = self
                        .builder
                        .build_float_compare(
                            inkwell::FloatPredicate::UNO,
                            value,
                            value,
                            "validator.safe_integer.nan",
                        )
                        .unwrap();
                    let out_of_range = self
                        .builder
                        .build_or(below, above, "validator.safe_integer.range")
                        .unwrap();
                    let invalid = self
                        .builder
                        .build_or(not_integer, out_of_range, "validator.safe_integer")
                        .unwrap();
                    Some(
                        self.builder
                            .build_or(invalid, unordered, "validator.safe_integer")
                            .unwrap(),
                    )
                }
            }
            _ => None,
        };
        Ok(invalid)
    }

    fn emit_validation_failure_if(
        &self,
        invalid: IntValue<'ctx>,
        target: &str,
        validator: &str,
        detail: &str,
        message: &str,
    ) {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .expect("internal: validator check outside function");
        let failure = self
            .context
            .append_basic_block(function, "validator.failure");
        let valid = self.context.append_basic_block(function, "validator.ok");
        self.builder
            .build_conditional_branch(invalid, failure, valid)
            .unwrap();
        self.builder.position_at_end(failure);
        let message = self.const_string_named(message, "validator.message");
        if self.env_validation_mode {
            let panic_fn =
                self.declare_libc_fn("tyrus_panic_validation", &[self.string_abi_type()], None);
            self.builder
                .build_call(panic_fn, &[message.into()], "")
                .unwrap();
            self.builder.build_unreachable().unwrap();
        } else {
            let target = self.validation_target(target);
            let target = self
                .const_string_abi(&target, "validation.target")
                .into_struct_value();
            let validator = self
                .const_string_abi(validator, "validation.validator")
                .into_struct_value();
            let detail = self
                .const_string_abi(detail, "validation.detail")
                .into_struct_value();
            let add = self.declare_libc_fn(
                "tyrus_validation_issue_add",
                &[
                    self.string_abi_type(),
                    self.string_abi_type(),
                    self.string_abi_type(),
                ],
                None,
            );
            self.builder
                .build_call(add, &[target.into(), validator.into(), detail.into()], "")
                .unwrap();
            self.builder.build_unconditional_branch(valid).unwrap();
        }
        self.builder.position_at_end(valid);
    }

    fn validation_target(&self, target: &str) -> String {
        let mut path = String::new();
        for segment in &self.validation_target_path {
            if segment.starts_with('[') {
                path.push_str(segment);
            } else {
                if !path.is_empty() {
                    path.push('.');
                }
                path.push_str(segment);
            }
        }
        if !target.is_empty() {
            if !path.is_empty() {
                path.push('.');
            }
            path.push_str(target);
        }
        path
    }

    pub(super) fn emit_validation_issues_begin(&self) {
        let begin = self.declare_libc_fn("tyrus_validation_issues_begin", &[], None);
        self.builder.build_call(begin, &[], "").unwrap();
    }

    pub(super) fn emit_validation_issues_finish(&mut self) {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .expect("internal: validation collection outside function");
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let usize_type = self.usize_type();
        let count_fn = self.declare_libc_fn(
            "tyrus_validation_issues_count",
            &[],
            Some(usize_type.into()),
        );
        let count = self
            .builder
            .build_call(count_fn, &[], "validation.issues.count")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        let has_issues = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                count,
                usize_type.const_zero(),
                "validation.issues.present",
            )
            .unwrap();
        let failure = self
            .context
            .append_basic_block(function, "validation.collection.failure");
        let valid = self
            .context
            .append_basic_block(function, "validation.collection.ok");
        self.builder
            .build_conditional_branch(has_issues, failure, valid)
            .unwrap();
        self.builder.position_at_end(failure);
        let take = self.declare_libc_fn("tyrus_validation_issues_take", &[], Some(ptr_type.into()));
        let data = self
            .builder
            .build_call(take, &[], "validation.issues.data")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();
        let array_type = self
            .llvm_type(&Type::Array(Box::new(Type::Named(
                "ValidationIssue".to_string(),
            ))))
            .into_struct_type();
        let mut issues = array_type.const_zero();
        issues = self
            .builder
            .build_insert_value(issues, data, 0, "validation.issues.array.data")
            .unwrap()
            .into_struct_value();
        issues = self
            .builder
            .build_insert_value(issues, count, 1, "validation.issues.array.length")
            .unwrap()
            .into_struct_value();
        issues = self
            .builder
            .build_insert_value(issues, count, 2, "validation.issues.array.capacity")
            .unwrap()
            .into_struct_value();
        let error_type = self
            .llvm_type(&Type::Named("ValidationError".to_string()))
            .into_struct_type();
        let name = self
            .const_string_abi("ValidationError", "validation.name")
            .into_struct_value();
        let message = self
            .const_string_abi(
                "one or more validation rules failed",
                "validation.error.message",
            )
            .into_struct_value();
        let mut error = error_type.const_zero();
        error = self
            .builder
            .build_insert_value(error, name, 0, "validation.error.name")
            .unwrap()
            .into_struct_value();
        error = self
            .builder
            .build_insert_value(error, message, 1, "validation.error.message.value")
            .unwrap()
            .into_struct_value();
        error = self
            .builder
            .build_insert_value(error, issues, 2, "validation.error.issues")
            .unwrap()
            .into_struct_value();
        self.emit_recoverable_runtime_error_cleanups()
            .expect("validation failure cleanup emission failed");
        self.emit_throw_value(
            error.into(),
            ThrownValue::Validation,
            function,
            &Type::Named("ValidationError".to_string()),
        )
        .expect("validation throw emission failed");
        self.builder.position_at_end(valid);
    }

    fn emit_validator_failure_if(
        &self,
        invalid: IntValue<'ctx>,
        decorator: &ValidatorDecorator,
        target: &str,
    ) {
        let validator = validator_display(decorator);
        let detail = Self::custom_validator_message(decorator);
        let message =
            format!("tyrus validation error: `{validator}` failed for `{target}`: {detail}");
        self.emit_validation_failure_if(invalid, target, &validator, &detail, &message);
    }

    fn emit_string_length(&self, value: StructValue<'ctx>) -> IntValue<'ctx> {
        self.string_char_len(value)
            .expect("reading a string's length is always emittable")
    }

    fn emit_string_predicate(
        &self,
        value: StructValue<'ctx>,
        name: &str,
    ) -> Option<IntValue<'ctx>> {
        let i64_type = self.context.i64_type();
        let function = self.declare_libc_fn(name, &[self.string_abi_type()], Some(i64_type.into()));
        Some(
            self.builder
                .build_call(function, &[value.into()], "validator.string_predicate")
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap()
                .into_int_value(),
        )
    }

    fn invert_runtime_predicate(&self, result: IntValue<'ctx>) -> IntValue<'ctx> {
        self.builder
            .build_int_compare(
                IntPredicate::EQ,
                result,
                result.get_type().const_zero(),
                "validator.invalid",
            )
            .unwrap()
    }

    fn string_arg_pointer(&self, args: &[Expr], index: usize) -> Option<StructValue<'ctx>> {
        let Expr {
            kind: ExprKind::StrLit(value),
            ..
        } = args.get(index)?
        else {
            return None;
        };
        Some(
            self.const_string_abi(value, "validator.string_arg")
                .into_struct_value(),
        )
    }

    fn regex_arg_pointer(&self, args: &[Expr], index: usize) -> Option<StructValue<'ctx>> {
        let Expr {
            kind: ExprKind::RegexLit { pattern, flags },
            ..
        } = args.get(index)?
        else {
            return None;
        };
        let pattern = tyrus_core::validators::regex_runtime_pattern(pattern, flags).ok()?;
        Some(
            self.const_string_abi(&pattern, "validator.regex_arg")
                .into_struct_value(),
        )
    }

    fn integer_arg(&self, args: &[Expr], index: usize) -> Option<i64> {
        self.integer_expr(args.get(index)?)
    }

    fn integer_expr(&self, expr: &Expr) -> Option<i64> {
        match expr.kind() {
            ExprKind::IntLit(value) => i64::try_from(*value).ok(),
            ExprKind::UIntLit(value) => i64::try_from(*value).ok(),
            ExprKind::Neg(inner) => self.integer_expr(inner).map(|value| -value),
            _ => None,
        }
    }

    fn float_arg(&self, args: &[Expr], index: usize) -> Option<f64> {
        self.float_expr(args.get(index)?)
    }

    fn bool_arg(&self, args: &[Expr], index: usize) -> Option<bool> {
        let Expr {
            kind: ExprKind::BoolLit(value),
            ..
        } = args.get(index)?
        else {
            return None;
        };
        Some(*value)
    }

    fn float_expr(&self, expr: &Expr) -> Option<f64> {
        match expr.kind() {
            ExprKind::IntLit(value) => Some(*value as f64),
            ExprKind::UIntLit(value) => Some(*value as f64),
            ExprKind::FloatLit(value) => Some(value.value()),
            ExprKind::Neg(inner) => self.float_expr(inner).map(|value| -value),
            _ => None,
        }
    }

    fn emit_array_unique_predicate(
        &mut self,
        array: StructValue<'ctx>,
        element: &Type,
    ) -> Result<Option<IntValue<'ctx>>, String> {
        let element = self.resolve_type(element);
        if self.requires_generated_unique_predicate(&element) {
            return self
                .emit_structural_array_unique_predicate(array, &element)
                .map(Some);
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i64_type = self.context.i64_type();
        let i32_type = self.context.i32_type();
        let array_type = self.llvm_type(&Type::Array(Box::new(element.clone())));
        let slot = self
            .builder
            .build_alloca(array_type, "validator.unique_array")
            .unwrap();
        self.builder.build_store(slot, array).unwrap();
        let element_size = self
            .llvm_type(&element)
            .size_of()
            .ok_or_else(|| format!("error: array element type `{element}` has no native size"))?;
        let function = self.declare_libc_fn(
            "tyrus_array_unique",
            &[pointer.into(), i64_type.into(), i32_type.into()],
            Some(i64_type.into()),
        );
        Ok(Some(
            self.builder
                .build_call(
                    function,
                    &[
                        slot.into(),
                        element_size.into(),
                        i32_type
                            .const_int(self.array_element_kind(&element), false)
                            .into(),
                    ],
                    "validator.unique",
                )
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap()
                .into_int_value(),
        ))
    }

    fn requires_generated_unique_predicate(&self, ty: &Type) -> bool {
        match transparent_codegen_type(&self.resolve_type(ty)) {
            Type::Tuple(elements) => elements
                .iter()
                .all(|element| self.supports_structural_equality(element)),
            Type::Object(fields) => fields
                .iter()
                .all(|field| self.supports_structural_equality(&field_storage_type(field))),
            Type::Named(name) => self.interfaces.get(name.as_str()).is_some_and(|fields| {
                fields
                    .iter()
                    .all(|field| self.supports_structural_equality(&field_storage_type(field)))
            }),
            _ => false,
        }
    }

    fn supports_structural_equality(&self, ty: &Type) -> bool {
        match transparent_codegen_type(&self.resolve_type(ty)) {
            Type::Bool | Type::Str => true,
            ty if is_integer_type(ty) || is_float_type(ty) => true,
            Type::Tuple(elements) => elements
                .iter()
                .all(|element| self.supports_structural_equality(element)),
            Type::Object(fields) => fields
                .iter()
                .all(|field| self.supports_structural_equality(&field_storage_type(field))),
            Type::Named(name) => self.interfaces.get(name.as_str()).is_some_and(|fields| {
                fields
                    .iter()
                    .all(|field| self.supports_structural_equality(&field_storage_type(field)))
            }),
            _ => false,
        }
    }

    fn emit_structural_array_unique_predicate(
        &mut self,
        array: StructValue<'ctx>,
        element: &Type,
    ) -> Result<IntValue<'ctx>, String> {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .expect("internal: unique validator outside function");
        let i64_type = self.context.i64_type();
        let bool_type = self.context.bool_type();
        let element_type = self.llvm_type(element);
        let data = self
            .builder
            .build_extract_value(array, 0, "validator.unique_data")
            .map_err(|error| error.to_string())?
            .into_pointer_value();
        let length = self
            .builder
            .build_extract_value(array, 1, "validator.unique_len")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let i_slot = self
            .builder
            .build_alloca(i64_type, "validator.unique_i")
            .unwrap();
        let j_slot = self
            .builder
            .build_alloca(i64_type, "validator.unique_j")
            .unwrap();
        let duplicate_slot = self
            .builder
            .build_alloca(bool_type, "validator.unique_duplicate")
            .unwrap();
        self.builder
            .build_store(i_slot, i64_type.const_zero())
            .unwrap();
        self.builder
            .build_store(duplicate_slot, bool_type.const_zero())
            .unwrap();

        let outer_cond = self
            .context
            .append_basic_block(function, "validator.unique.outer");
        let outer_body = self
            .context
            .append_basic_block(function, "validator.unique.outer_body");
        let inner_cond = self
            .context
            .append_basic_block(function, "validator.unique.inner");
        let inner_body = self
            .context
            .append_basic_block(function, "validator.unique.inner_body");
        let duplicate = self
            .context
            .append_basic_block(function, "validator.unique.duplicate");
        let inner_next = self
            .context
            .append_basic_block(function, "validator.unique.inner_next");
        let outer_next = self
            .context
            .append_basic_block(function, "validator.unique.outer_next");
        let done = self
            .context
            .append_basic_block(function, "validator.unique.done");

        self.builder.build_unconditional_branch(outer_cond).unwrap();

        self.builder.position_at_end(outer_cond);
        let i_value = self
            .builder
            .build_load(i64_type, i_slot, "validator.unique_i")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let duplicate_value = self
            .builder
            .build_load(bool_type, duplicate_slot, "validator.unique_duplicate")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let i_in_bounds = self
            .builder
            .build_int_compare(
                IntPredicate::SLT,
                i_value,
                length,
                "validator.unique_i_bounds",
            )
            .unwrap();
        let no_duplicate = self
            .builder
            .build_not(duplicate_value, "validator.unique_no_duplicate")
            .unwrap();
        let keep_outer = self
            .builder
            .build_and(i_in_bounds, no_duplicate, "validator.unique_keep_outer")
            .unwrap();
        self.builder
            .build_conditional_branch(keep_outer, outer_body, done)
            .unwrap();

        self.builder.position_at_end(outer_body);
        let first_j = self
            .builder
            .build_int_add(
                i_value,
                i64_type.const_int(1, false),
                "validator.unique_j_start",
            )
            .unwrap();
        self.builder.build_store(j_slot, first_j).unwrap();
        self.builder.build_unconditional_branch(inner_cond).unwrap();

        self.builder.position_at_end(inner_cond);
        let j_value = self
            .builder
            .build_load(i64_type, j_slot, "validator.unique_j")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let duplicate_value = self
            .builder
            .build_load(bool_type, duplicate_slot, "validator.unique_duplicate")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let j_in_bounds = self
            .builder
            .build_int_compare(
                IntPredicate::SLT,
                j_value,
                length,
                "validator.unique_j_bounds",
            )
            .unwrap();
        let no_duplicate = self
            .builder
            .build_not(duplicate_value, "validator.unique_no_duplicate")
            .unwrap();
        let keep_inner = self
            .builder
            .build_and(j_in_bounds, no_duplicate, "validator.unique_keep_inner")
            .unwrap();
        self.builder
            .build_conditional_branch(keep_inner, inner_body, outer_next)
            .unwrap();

        self.builder.position_at_end(inner_body);
        let left_ptr = unsafe {
            self.builder
                .build_gep(element_type, data, &[i_value], "validator.unique_left")
                .unwrap()
        };
        let right_ptr = unsafe {
            self.builder
                .build_gep(element_type, data, &[j_value], "validator.unique_right")
                .unwrap()
        };
        let left = self
            .builder
            .build_load(element_type, left_ptr, "validator.unique_left_value")
            .map_err(|error| error.to_string())?;
        let right = self
            .builder
            .build_load(element_type, right_ptr, "validator.unique_right_value")
            .map_err(|error| error.to_string())?;
        let equal = self.emit_structural_equality(left, right, element)?;
        self.builder
            .build_conditional_branch(equal, duplicate, inner_next)
            .unwrap();

        self.builder.position_at_end(duplicate);
        self.builder
            .build_store(duplicate_slot, bool_type.const_int(1, false))
            .unwrap();
        self.builder.build_unconditional_branch(done).unwrap();

        self.builder.position_at_end(inner_next);
        let next_j = self
            .builder
            .build_int_add(
                j_value,
                i64_type.const_int(1, false),
                "validator.unique_next_j",
            )
            .unwrap();
        self.builder.build_store(j_slot, next_j).unwrap();
        self.builder.build_unconditional_branch(inner_cond).unwrap();

        self.builder.position_at_end(outer_next);
        let next_i = self
            .builder
            .build_int_add(
                i_value,
                i64_type.const_int(1, false),
                "validator.unique_next_i",
            )
            .unwrap();
        self.builder.build_store(i_slot, next_i).unwrap();
        self.builder.build_unconditional_branch(outer_cond).unwrap();

        self.builder.position_at_end(done);
        let duplicate_value = self
            .builder
            .build_load(bool_type, duplicate_slot, "validator.unique_duplicate")
            .map_err(|error| error.to_string())?
            .into_int_value();
        Ok(self
            .builder
            .build_not(duplicate_value, "validator.unique")
            .unwrap())
    }

    pub(super) fn emit_structural_equality(
        &self,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
        ty: &Type,
    ) -> Result<IntValue<'ctx>, String> {
        let resolved = self.resolve_type(ty);
        let ty = transparent_codegen_type(&resolved);
        let equal = match ty {
            Type::Bool => self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    left.into_int_value(),
                    right.into_int_value(),
                    "validator.unique_eq_bool",
                )
                .unwrap(),
            ty if is_integer_type(ty) => self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    left.into_int_value(),
                    right.into_int_value(),
                    "validator.unique_eq_int",
                )
                .unwrap(),
            ty if is_float_type(ty) => self
                .builder
                .build_float_compare(
                    inkwell::FloatPredicate::OEQ,
                    left.into_float_value(),
                    right.into_float_value(),
                    "validator.unique_eq_float",
                )
                .unwrap(),
            Type::Str => self.emit_validator_string_equality(
                left.into_struct_value(),
                right.into_struct_value(),
                "validator.unique_eq_string",
            ),
            Type::Decimal => self.emit_decimal_structural_equality(left, right),
            Type::Optional(inner) => self.emit_optional_structural_equality(left, right, inner)?,
            Type::Tuple(elements) => {
                self.emit_structural_field_equality(left, right, elements, "tuple")?
            }
            Type::Array(element) => self.emit_array_structural_equality(left, right, element)?,
            Type::Object(fields) => {
                let field_types = fields.iter().map(field_storage_type).collect::<Vec<_>>();
                self.emit_structural_field_equality(left, right, &field_types, "object")?
            }
            Type::Named(name) => {
                let fields = self
                    .interfaces
                    .get(name.as_str())
                    .ok_or_else(|| format!("error: unknown interface `{name}`"))?;
                let field_types = fields.iter().map(field_storage_type).collect::<Vec<_>>();
                self.emit_structural_field_equality(left, right, &field_types, name)?
            }
            other => {
                return Err(format!(
                    "error: `@unique` does not support structural equality for `{other}`"
                ));
            }
        };
        Ok(equal)
    }

    fn emit_decimal_structural_equality(
        &self,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> IntValue<'ctx> {
        let decimal = self.llvm_type(&Type::Decimal);
        let left_slot = self
            .builder
            .build_alloca(decimal, "validator.unique_decimal_left")
            .expect("decimal equality allocation is always emittable");
        let right_slot = self
            .builder
            .build_alloca(decimal, "validator.unique_decimal_right")
            .expect("decimal equality allocation is always emittable");
        self.builder.build_store(left_slot, left).unwrap();
        self.builder.build_store(right_slot, right).unwrap();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let compare = self.declare_libc_fn(
            "tyrus_decimal_compare",
            &[pointer.into(), pointer.into()],
            Some(self.context.i32_type().into()),
        );
        let comparison = self
            .builder
            .build_call(
                compare,
                &[left_slot.into(), right_slot.into()],
                "validator.unique_decimal_compare",
            )
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        self.builder
            .build_int_compare(
                IntPredicate::EQ,
                comparison,
                self.context.i32_type().const_zero(),
                "validator.unique_eq_decimal",
            )
            .unwrap()
    }

    fn emit_optional_structural_equality(
        &self,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
        inner: &Type,
    ) -> Result<IntValue<'ctx>, String> {
        let left = left.into_pointer_value();
        let right = right.into_pointer_value();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: optional equality outside a function")?;
        let both_absent = self
            .context
            .append_basic_block(function, "optional.eq.both_absent");
        let one_absent = self
            .context
            .append_basic_block(function, "optional.eq.one_absent");
        let both_present = self
            .context
            .append_basic_block(function, "optional.eq.both_present");
        let merge = self
            .context
            .append_basic_block(function, "optional.eq.merge");
        let left_absent = self
            .builder
            .build_is_null(left, "optional.eq.left_absent")
            .map_err(|error| error.to_string())?;
        let right_absent = self
            .builder
            .build_is_null(right, "optional.eq.right_absent")
            .map_err(|error| error.to_string())?;
        let both_are_absent = self
            .builder
            .build_and(left_absent, right_absent, "optional.eq.both_are_absent")
            .map_err(|error| error.to_string())?;
        let either_absent = self
            .builder
            .build_or(left_absent, right_absent, "optional.eq.either_absent")
            .map_err(|error| error.to_string())?;
        let after_both_absent = self
            .context
            .append_basic_block(function, "optional.eq.after_both_absent");
        self.builder
            .build_conditional_branch(both_are_absent, both_absent, after_both_absent)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(after_both_absent);
        self.builder
            .build_conditional_branch(either_absent, one_absent, both_present)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(both_absent);
        let absent_block = self.builder.get_insert_block().unwrap();
        self.builder
            .build_unconditional_branch(merge)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(one_absent);
        let one_absent_block = self.builder.get_insert_block().unwrap();
        self.builder
            .build_unconditional_branch(merge)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(both_present);
        let left_value = self
            .builder
            .build_load(self.llvm_type(inner), left, "optional.eq.left_value")
            .map_err(|error| error.to_string())?;
        let right_value = self
            .builder
            .build_load(self.llvm_type(inner), right, "optional.eq.right_value")
            .map_err(|error| error.to_string())?;
        let present_equal = self.emit_structural_equality(left_value, right_value, inner)?;
        let present_block = self.builder.get_insert_block().unwrap();
        self.builder
            .build_unconditional_branch(merge)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(merge);
        let result = self
            .builder
            .build_phi(self.context.bool_type(), "optional.eq.result")
            .map_err(|error| error.to_string())?;
        result.add_incoming(&[
            (&self.context.bool_type().const_int(1, false), absent_block),
            (&self.context.bool_type().const_zero(), one_absent_block),
            (&present_equal, present_block),
        ]);
        Ok(result.as_basic_value().into_int_value())
    }

    fn emit_array_structural_equality(
        &self,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
        element: &Type,
    ) -> Result<IntValue<'ctx>, String> {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: array equality outside a function")?;
        let left = left.into_struct_value();
        let right = right.into_struct_value();
        let left_data = self
            .builder
            .build_extract_value(left, 0, "structural.array.left_data")
            .map_err(|error| error.to_string())?
            .into_pointer_value();
        let left_length = self
            .builder
            .build_extract_value(left, 1, "structural.array.left_length")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let right_data = self
            .builder
            .build_extract_value(right, 0, "structural.array.right_data")
            .map_err(|error| error.to_string())?
            .into_pointer_value();
        let right_length = self
            .builder
            .build_extract_value(right, 1, "structural.array.right_length")
            .map_err(|error| error.to_string())?
            .into_int_value();

        let index_type = left_length.get_type();
        let verdict = self
            .builder
            .build_alloca(self.context.bool_type(), "structural.array.verdict")
            .map_err(|error| error.to_string())?;
        let index_slot = self
            .builder
            .build_alloca(index_type, "structural.array.index")
            .map_err(|error| error.to_string())?;
        let same_length = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                left_length,
                right_length,
                "structural.array.same_length",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(verdict, same_length)
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(index_slot, index_type.const_zero())
            .map_err(|error| error.to_string())?;

        let check = self
            .context
            .append_basic_block(function, "structural.array.check");
        let body = self
            .context
            .append_basic_block(function, "structural.array.body");
        let next = self
            .context
            .append_basic_block(function, "structural.array.next");
        let done = self
            .context
            .append_basic_block(function, "structural.array.done");
        self.builder
            .build_conditional_branch(same_length, check, done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(check);
        let index = self
            .builder
            .build_load(index_type, index_slot, "structural.array.index_value")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let more = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                index,
                left_length,
                "structural.array.more",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(more, body, done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(body);
        let element_type = self.llvm_type(element);
        let index = self
            .builder
            .build_load(index_type, index_slot, "structural.array.body_index")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let left_element = unsafe {
            self.builder
                .build_gep(
                    element_type,
                    left_data,
                    &[index],
                    "structural.array.left_element",
                )
                .map_err(|error| error.to_string())?
        };
        let right_element = unsafe {
            self.builder
                .build_gep(
                    element_type,
                    right_data,
                    &[index],
                    "structural.array.right_element",
                )
                .map_err(|error| error.to_string())?
        };
        let left_value = self
            .builder
            .build_load(element_type, left_element, "structural.array.left_value")
            .map_err(|error| error.to_string())?;
        let right_value = self
            .builder
            .build_load(element_type, right_element, "structural.array.right_value")
            .map_err(|error| error.to_string())?;
        let equal = self.emit_structural_equality(left_value, right_value, element)?;
        self.builder
            .build_store(verdict, equal)
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(equal, next, done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(next);
        let index = self
            .builder
            .build_load(index_type, index_slot, "structural.array.next_index")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let advanced = self
            .builder
            .build_int_add(
                index,
                index_type.const_int(1, false),
                "structural.array.advance",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(index_slot, advanced)
            .map_err(|error| error.to_string())?;
        self.builder
            .build_unconditional_branch(check)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(done);
        self.builder
            .build_load(self.context.bool_type(), verdict, "structural.array.equal")
            .map_err(|error| error.to_string())
            .map(|value| value.into_int_value())
    }

    fn emit_structural_field_equality(
        &self,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
        field_types: &[Type],
        label: &str,
    ) -> Result<IntValue<'ctx>, String> {
        let bool_type = self.context.bool_type();
        let mut equal = bool_type.const_int(1, false);
        let left = left.into_struct_value();
        let right = right.into_struct_value();
        for (index, field_type) in field_types.iter().enumerate() {
            let left_field = self
                .builder
                .build_extract_value(
                    left,
                    index as u32,
                    &format!("validator.unique_{label}_left"),
                )
                .map_err(|error| error.to_string())?;
            let right_field = self
                .builder
                .build_extract_value(
                    right,
                    index as u32,
                    &format!("validator.unique_{label}_right"),
                )
                .map_err(|error| error.to_string())?;
            let field_equal = self.emit_structural_equality(left_field, right_field, field_type)?;
            equal = self
                .builder
                .build_and(equal, field_equal, "validator.unique_eq_field")
                .unwrap();
        }
        Ok(equal)
    }

    fn emit_validator_string_equality(
        &self,
        left: StructValue<'ctx>,
        right: StructValue<'ctx>,
        name: &str,
    ) -> IntValue<'ctx> {
        self.emit_string_equals(left, right, name)
            .expect("string equality is always emittable")
    }

    fn int_less_than_predicate(&self, signed: bool) -> IntPredicate {
        if signed {
            IntPredicate::SLT
        } else {
            IntPredicate::ULT
        }
    }

    fn int_greater_than_predicate(&self, signed: bool) -> IntPredicate {
        if signed {
            IntPredicate::SGT
        } else {
            IntPredicate::UGT
        }
    }

    fn int_less_equal_predicate(&self, signed: bool) -> IntPredicate {
        if signed {
            IntPredicate::SLE
        } else {
            IntPredicate::ULE
        }
    }

    fn int_greater_equal_predicate(&self, signed: bool) -> IntPredicate {
        if signed {
            IntPredicate::SGE
        } else {
            IntPredicate::UGE
        }
    }
}

fn sanitize_validator_symbol(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect()
}

pub(super) fn validators_imply_non_zero(decorators: &[ValidatorDecorator]) -> bool {
    decorators.iter().any(|decorator| {
        decorator.name == "notEqual"
            && matches!(
                decorator.args.first(),
                Some(Expr {
                    kind: ExprKind::IntLit(0),
                    ..
                })
            )
    })
}

pub(super) fn validator_parameter_type(param: &Param, concrete: &Type) -> Type {
    if param.optional
        && param.default_value.is_none()
        && !matches!(transparent_codegen_type(concrete), Type::Optional(_))
    {
        Type::Optional(Box::new(concrete.clone()))
    } else {
        concrete.clone()
    }
}
