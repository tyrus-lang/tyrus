use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_integer_cast_range_check(
        &self,
        value: IntValue<'ctx>,
        source: Option<&Type>,
        target: &Type,
    ) {
        let source = match source {
            Some(source) if is_integer_type(source) => transparent_codegen_type(source).clone(),
            _ => match value.get_type().get_bit_width() {
                8 => Type::I8,
                16 => Type::I16,
                32 => Type::I32,
                64 => Type::I64,
                128 => Type::I128,
                _ => return,
            },
        };
        let Some(fits) = self.numeric_cast_fits(value.into(), &source, target) else {
            return;
        };
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .expect("internal: range check outside function");
        let error_block = self
            .context
            .append_basic_block(function, "cast.range.error");
        let ok_block = self.context.append_basic_block(function, "cast.range.ok");
        self.builder
            .build_conditional_branch(fits, ok_block, error_block)
            .unwrap();

        self.builder.position_at_end(error_block);
        let message = format!("tyrus runtime error: value out of range for `{target}`");
        self.emit_runtime_panic(&message);
        self.builder.position_at_end(ok_block);
    }

    pub(super) fn emit_enum_variant(
        &mut self,
        type_name: &str,
        variant_name: &str,
        enum_info: &tyrus_core::ast::ResolvedEnum,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let variant_index = enum_info
            .members
            .iter()
            .position(|(name, _)| name == variant_name)
            .ok_or_else(|| format!("enum variant `{variant_name}` not found"))?;

        let enum_ty = self.llvm_type(&Type::Named(type_name.to_string()));
        let struct_ty = enum_ty.into_struct_type();

        let tag_ty = self.context.i8_type();
        let tag_value = tag_ty.const_int(variant_index as u64, false);

        let payload_ty = struct_ty
            .get_field_type_at_index(1)
            .unwrap()
            .into_array_type();
        let payload_value = payload_ty.const_zero();

        Ok(struct_ty
            .const_named_struct(&[tag_value.into(), payload_value.into()])
            .into())
    }

    pub(super) fn emit_enum_variant_constructor(
        &mut self,
        type_name: &str,
        _variant_name: &str,
        variant_index: usize,
        enum_info: &tyrus_core::ast::ResolvedEnum,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let enum_ty = self.llvm_type(&Type::Named(type_name.to_string()));
        let struct_ty = enum_ty.into_struct_type();

        let tag_ty = self.context.i8_type();
        let tag_value = tag_ty.const_int(variant_index as u64, false);

        let payload_ty = struct_ty
            .get_field_type_at_index(1)
            .unwrap()
            .into_array_type();
        let byte_ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());

        let alloca = self
            .builder
            .build_alloca(payload_ty, "enum.payload")
            .unwrap();
        let byte_ptr = self
            .builder
            .build_pointer_cast(alloca, byte_ptr_ty, "enum.payload.byteptr")
            .map_err(|e| format!("pointer cast failed: {e}"))?;

        match &enum_info.member_kinds[variant_index] {
            EnumPayload::Tuple(types) => {
                let mut offset: u64 = 0;
                for (arg, ty) in args.iter().zip(types) {
                    let value = self.emit_expr_as(arg, ty)?;
                    let llvm_ty = self.llvm_type(ty);
                    let size = self.type_layout(ty).0 as u64;
                    let arg_alloca = self.builder.build_alloca(llvm_ty, "enum.arg").unwrap();
                    self.builder.build_store(arg_alloca, value).unwrap();
                    let arg_byte_ptr = self
                        .builder
                        .build_pointer_cast(arg_alloca, byte_ptr_ty, "enum.arg.byteptr")
                        .map_err(|e| format!("pointer cast failed: {e}"))?;
                    let dest_ptr = unsafe {
                        self.builder
                            .build_gep(
                                self.context.i8_type(),
                                byte_ptr,
                                &[self.context.i64_type().const_int(offset, false)],
                                "enum.payload.offset",
                            )
                            .unwrap()
                    };
                    let size_val = self.context.i64_type().const_int(size, false);
                    self.builder
                        .build_memcpy(dest_ptr, 1, arg_byte_ptr, 1, size_val)
                        .map_err(|e| format!("memcpy failed: {e}"))?;
                    offset += size;
                }
            }
            EnumPayload::Struct(fields) => {
                if let Some(arg) = args.first() {
                    let arg_value = self.emit_expr_as(arg, &Type::Object(fields.clone()))?;
                    if let BasicValueEnum::StructValue(struct_val) = arg_value {
                        let mut offset: u64 = 0;
                        for (i, field) in fields.iter().enumerate() {
                            let field_val = self
                                .builder
                                .build_extract_value(
                                    struct_val,
                                    i as u32,
                                    &format!("enum.field.{}", field.name),
                                )
                                .map_err(|e| format!("extract_value failed: {e}"))?;
                            let llvm_ty = self.llvm_type(&field.ty);
                            let size = self.type_layout(&field.ty).0 as u64;
                            let field_alloca = self
                                .builder
                                .build_alloca(llvm_ty, "enum.field.alloca")
                                .unwrap();
                            self.builder.build_store(field_alloca, field_val).unwrap();
                            let field_byte_ptr = self
                                .builder
                                .build_pointer_cast(field_alloca, byte_ptr_ty, "enum.field.byteptr")
                                .map_err(|e| format!("pointer cast failed: {e}"))?;
                            let dest_ptr = unsafe {
                                self.builder
                                    .build_gep(
                                        self.context.i8_type(),
                                        byte_ptr,
                                        &[self.context.i64_type().const_int(offset, false)],
                                        "enum.payload.offset",
                                    )
                                    .unwrap()
                            };
                            let size_val = self.context.i64_type().const_int(size, false);
                            self.builder
                                .build_memcpy(dest_ptr, 1, field_byte_ptr, 1, size_val)
                                .map_err(|e| format!("memcpy failed: {e}"))?;
                            offset += size;
                        }
                    }
                }
            }
            EnumPayload::Unit => {}
        }

        let payload_value = self
            .builder
            .build_load(payload_ty, alloca, "enum.payload.loaded")
            .unwrap();

        let with_tag = self
            .builder
            .build_insert_value(struct_ty.get_undef(), tag_value, 0, "enum.tag")
            .map_err(|e| format!("insert enum tag failed: {e}"))?;
        let enum_value = self
            .builder
            .build_insert_value(with_tag, payload_value, 1, "enum.value")
            .map_err(|e| format!("insert enum payload failed: {e}"))?;

        Ok(enum_value.into_struct_value().into())
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn emit_expr_as(
        &mut self,
        expr: &Expr,
        expected: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let resolved_expected = self.resolve_type(expected);
        if &resolved_expected != expected {
            return self.emit_expr_as(expr, &resolved_expected);
        }
        match (transparent_codegen_type(expected), expr.kind()) {
            (Type::Set(element), ExprKind::NewSet { ty: None, initial }) => {
                let element = element.as_ref().clone();
                let set = self.emit_new_set(&element, initial.as_deref())?;
                self.record_owned_native_result(set, Type::Set(Box::new(element)));
                return Ok(set);
            }
            (
                Type::Map(key, value),
                ExprKind::NewMap {
                    key: None,
                    value: None,
                    initial,
                },
            ) => {
                let (key, value) = (key.as_ref().clone(), value.as_ref().clone());
                let map = self.emit_new_map(&key, &value, initial.as_deref())?;
                self.record_owned_native_result(map, Type::Map(Box::new(key), Box::new(value)));
                return Ok(map);
            }
            (
                Type::Array(element),
                ExprKind::MethodCall {
                    object,
                    method,
                    args,
                    ..
                },
            ) if matches!(object.as_ref().kind(), ExprKind::Ident(name) if name == "Array")
                && method == "withCapacity" =>
            {
                let element = element.as_ref().clone();
                return self.emit_array_with_capacity(args, &element);
            }
            _ => {}
        }
        // A tuple filling an array is copied into storage the array owns: the
        // tuple lives in a stack slot, and an array outlives the scope it was
        // built in. Borrowing the slot here would leave a dangling buffer.
        if let Type::Array(element) = transparent_codegen_type(expected) {
            let element = element.as_ref().clone();
            if matches!(
                self.expr_type_hint(expr)
                    .as_ref()
                    .map(transparent_codegen_type),
                Some(Type::Tuple(_))
            ) {
                return self.materialize_tuple_as_array(expr, &element);
            }
        }
        if matches!(transparent_codegen_type(expected), Type::Char)
            && let ExprKind::StrLit(text) = expr.kind()
            && let Some(character) = text.chars().next()
        {
            return Ok(self
                .context
                .i32_type()
                .const_int(u64::from(character as u32), false)
                .into());
        }
        if let Type::Result { success, error } = transparent_codegen_type(expected) {
            let variants = vec![success.as_ref().clone(), error.as_ref().clone()];
            let returns_native_result = match expr.kind() {
                ExprKind::FnCall { callee, .. } => {
                    callee == tyrus_core::memory::ZEROIZE_TARGET
                        || callee == "zeroize"
                        || tyrus_core::module_loader::original_global_name(callee) == "zeroize"
                }
                ExprKind::MethodCall { object, method, .. }
                | ExprKind::OptionalMethodCall { object, method, .. } => match object.kind() {
                    ExprKind::Ident(namespace) => {
                        tyrus_core::encoding::encoding_method_target(namespace, method)
                            .is_some_and(tyrus_core::encoding::encoding_target_returns_result)
                    }
                    _ => false,
                },
                _ => false,
            };
            if returns_native_result {
                return self.emit_expr(expr);
            }
            if let ExprKind::FnCall { callee, args, .. } = expr.kind() {
                if callee == "Ok" && args.is_empty() && matches!(success.as_ref(), Type::Void) {
                    let union_ty = self.union_llvm_type(&variants);
                    let value = union_ty.const_zero();
                    return self
                        .builder
                        .build_insert_value(
                            value,
                            self.context.i8_type().const_zero(),
                            0,
                            "result.explicit.void",
                        )
                        .map(|value| value.into_struct_value().into())
                        .map_err(|error| error.to_string());
                }
                if matches!(callee.as_str(), "Ok" | "Err") && args.len() == 1 {
                    let variant = if callee == "Ok" { 0 } else { 1 };
                    let payload_ty = &variants[variant];
                    let payload = self.emit_expr_as(&args[0], payload_ty)?;
                    self.forget_owned_native_result(payload);
                    self.forget_conditionally_owned_result(payload);
                    let union_ty = self.union_llvm_type(&variants);
                    let slot = self
                        .builder
                        .build_alloca(union_ty, "result.explicit")
                        .unwrap();
                    self.store_union_variant(&variants, slot, variant as u8, payload)?;
                    return self
                        .builder
                        .build_load(union_ty, slot, "result.explicit.value")
                        .map_err(|error| error.to_string());
                }
            }
            if self
                .expr_type_hint(expr)
                .is_some_and(|ty| matches!(self.resolve_type(&ty), Type::Result { .. }))
            {
                return self.emit_expr(expr);
            }
            return self
                .build_union_value(&variants, expr, "result.value")
                .map(|(value, _)| value);
        }
        if let Type::Union(variants) = transparent_codegen_type(expected) {
            if let Some(Type::Union(source)) = self
                .expr_type_hint(expr)
                .map(|ty| transparent_codegen_type(&self.resolve_type(&ty)).clone())
            {
                let value = self.emit_expr(expr)?;
                return self.emit_union_retag(value, &source, variants);
            }
            if let ExprKind::Ident(name) = expr.kind()
                && let Some((ptr, ty)) = self.load_var(name)
                && let Type::Union(source) = transparent_codegen_type(&ty)
            {
                let llvm_ty = self.llvm_type(&ty);
                let value = self
                    .builder
                    .build_load(llvm_ty, ptr, name)
                    .map_err(|error| error.to_string())?;
                return self.emit_union_retag(value, source, variants);
            }
            return self
                .build_union_value(variants, expr, "union.value")
                .map(|(value, _)| value);
        }
        match (expr, expected) {
            (
                Expr {
                    kind: ExprKind::ConstAssert(inner),
                    ..
                },
                expected,
            ) => self.emit_expr_as(inner, expected),
            (
                Expr {
                    kind: ExprKind::FnCall { callee, args, .. },
                    ..
                },
                Type::Optional(target),
            ) if args.len() == 1 && parse_runtime_fn(callee, target).is_some() => {
                let function = parse_runtime_fn(callee, target).unwrap();
                let ptr = self.context.ptr_type(inkwell::AddressSpace::default());
                let text = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let parse_fn =
                    self.declare_libc_fn(function, &[self.string_abi_type()], Some(ptr.into()));
                let result = self
                    .builder
                    .build_call(parse_fn, &[text.into()], callee)
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.record_owned_native_result_with(
                    result,
                    Type::Optional(target.clone()),
                    crate::ReleasePlan::Value,
                );
                Ok(result)
            }
            (
                Expr {
                    kind:
                        ExprKind::ArrowFn {
                            is_async,
                            params,
                            ret_type,
                            body,
                            ..
                        },
                    ..
                },
                Type::Function {
                    params: expected_params,
                    ret: expected_return,
                },
            ) => {
                let confined = std::mem::take(&mut self.confined_closure_pending);
                self.emit_contextual_arrow_fn(
                    params,
                    ret_type,
                    body,
                    expected_params,
                    expected_return,
                    *is_async,
                    !confined,
                )
            }
            (
                Expr {
                    kind:
                        ExprKind::MethodCall {
                            object,
                            method,
                            args,
                            ..
                        },
                    ..
                },
                expected,
            ) if is_functional_array_method(method)
                && matches!(
                    self.expr_type_hint(object)
                        .as_ref()
                        .map(transparent_codegen_type),
                    Some(Type::Array(_) | Type::Tuple(_))
                ) =>
            {
                let result =
                    self.emit_functional_array_method(object, method, args, Some(expected))?;
                if tyrus_core::async_types::array_method_allocates(method) {
                    let ty = self.resolve_type(expected);
                    match transparent_codegen_type(&ty) {
                        Type::Array(_) | Type::Tuple(_) => {
                            self.record_owned_native_result(result, ty);
                        }
                        Type::Optional(_)
                            if matches!(method.as_str(), "find" | "findLast" | "avg" | "avgBy") =>
                        {
                            self.record_owned_native_result_with(
                                result,
                                ty,
                                crate::ReleasePlan::Value,
                            );
                        }
                        _ => {}
                    }
                }
                Ok(result)
            }
            (
                Expr {
                    kind: ExprKind::ObjectLit(values),
                    ..
                },
                Type::Named(name),
            ) => self.emit_object_literal(name, values),
            (
                Expr {
                    kind: ExprKind::ObjectLit(values),
                    ..
                },
                Type::Object(fields),
            ) => self.emit_object_literal_with_fields(expected, fields, values),
            (
                Expr {
                    kind: ExprKind::ObjectLit(values),
                    ..
                },
                Type::Map(_, value_ty),
            ) => self.emit_map_literal(values, value_ty),
            (
                Expr {
                    kind: ExprKind::ObjectUpdate { .. },
                    ..
                },
                Type::Named(_),
            ) => self.emit_expr(expr),
            (
                Expr {
                    kind: ExprKind::ArrayLit(values),
                    ..
                },
                Type::Array(element),
            ) => self.emit_array_literal(values, element),
            (
                Expr {
                    kind: ExprKind::ArrayRepeat { value, count },
                    ..
                },
                Type::Array(element),
            ) => self.emit_array_repeat_literal(value, count, element),
            (
                Expr {
                    kind: ExprKind::ArrayRepeat { value, count },
                    ..
                },
                Type::Tuple(elements),
            ) => {
                let count_value = tyrus_core::type_checker::integer_constant_value(count)
                    .and_then(Result::ok)
                    .and_then(|count| usize::try_from(count).ok());
                if count_value != Some(elements.len()) {
                    return Err(format!(
                        "error: fixed array repeat count does not match its type length of {}",
                        elements.len()
                    ));
                }
                let repeated = vec![(**value).clone(); elements.len()];
                self.emit_tuple_literal(&repeated, elements)
            }
            (
                Expr {
                    kind: ExprKind::ArrayLit(values),
                    ..
                },
                Type::Tuple(elements),
            ) => self.emit_tuple_literal(values, elements),
            (
                Expr {
                    kind: ExprKind::ArrayLit(values),
                    ..
                },
                Type::Readonly(inner),
            ) => match inner.as_ref() {
                Type::Array(element) => self.emit_array_literal(values, element),
                Type::Tuple(elements) => self.emit_tuple_literal(values, elements),
                _ => self.emit_expr_as(expr, inner),
            },
            (
                Expr {
                    kind: ExprKind::ArrayRepeat { value, count },
                    ..
                },
                Type::Readonly(inner),
            ) => match inner.as_ref() {
                Type::Array(element) => self.emit_array_repeat_literal(value, count, element),
                Type::Tuple(elements) => {
                    let count_value = tyrus_core::type_checker::integer_constant_value(count)
                        .and_then(Result::ok)
                        .and_then(|count| usize::try_from(count).ok());
                    if count_value != Some(elements.len()) {
                        return Err(format!(
                            "error: fixed array repeat count does not match its type length of {}",
                            elements.len()
                        ));
                    }
                    let repeated = vec![(**value).clone(); elements.len()];
                    self.emit_tuple_literal(&repeated, elements)
                }
                _ => self.emit_expr_as(expr, inner),
            },
            (
                Expr {
                    kind: ExprKind::Neg(inner),
                    ..
                },
                expected,
            ) if is_integer_type(expected) || is_float_type(expected) => {
                if is_integer_type(expected)
                    && let ExprKind::IntLit(magnitude) = inner.kind()
                    && let Some(value) = magnitude.checked_neg()
                {
                    return self
                        .emit_expr_as(&Expr::new(ExprKind::IntLit(value), expr.span), expected);
                }
                let value = self.emit_expr_as(inner, expected)?;
                if value.is_int_value() {
                    Ok(self
                        .builder
                        .build_int_neg(value.into_int_value(), "neg")
                        .unwrap()
                        .into())
                } else {
                    Ok(self
                        .builder
                        .build_float_neg(value.into_float_value(), "neg")
                        .unwrap()
                        .into())
                }
            }
            (
                Expr {
                    kind: ExprKind::IntLit(value),
                    ..
                },
                Type::I32,
            ) => Ok(self
                .context
                .i32_type()
                .const_int(*value as u64, true)
                .into()),
            (
                Expr {
                    kind: ExprKind::IntLit(value),
                    ..
                },
                Type::I64,
            ) => Ok(self
                .context
                .i64_type()
                .const_int(*value as u64, true)
                .into()),
            (
                Expr {
                    kind: ExprKind::IntLit(value),
                    ..
                },
                Type::I128,
            ) => Ok(self.const_i128_value(*value).into()),
            (
                Expr {
                    kind: ExprKind::IntLit(value),
                    ..
                },
                Type::Isize,
            ) => Ok(self.usize_type().const_int(*value as u64, true).into()),
            (
                Expr {
                    kind: ExprKind::IntLit(value),
                    ..
                },
                Type::U32,
            ) => Ok(self
                .context
                .i32_type()
                .const_int(*value as u64, false)
                .into()),
            (
                Expr {
                    kind: ExprKind::IntLit(value),
                    ..
                },
                Type::U64,
            ) => Ok(self
                .context
                .i64_type()
                .const_int(*value as u64, false)
                .into()),
            (
                Expr {
                    kind: ExprKind::IntLit(value),
                    ..
                },
                Type::U128,
            ) => Ok(self.const_i128_value(*value).into()),
            (
                Expr {
                    kind: ExprKind::IntLit(value),
                    ..
                },
                Type::Usize,
            ) => Ok(self.usize_type().const_int(*value as u64, false).into()),
            (
                Expr {
                    kind: ExprKind::UIntLit(value),
                    ..
                },
                Type::U128,
            ) => Ok(self.const_u128_value(*value).into()),
            (
                Expr {
                    kind: ExprKind::UIntLit(value),
                    ..
                },
                Type::Usize,
            ) => Ok(self.usize_type().const_int(*value as u64, false).into()),
            (
                Expr {
                    kind: ExprKind::FloatLit(value),
                    ..
                },
                Type::Decimal,
            ) => {
                let (coefficient, exponent) = codegen_decimal_literal(value.text())?;
                let ty = self.llvm_type(&Type::Decimal).into_struct_type();
                Ok(ty
                    .const_named_struct(&[
                        self.const_i128_value(coefficient).into(),
                        self.context
                            .i32_type()
                            .const_int(exponent as u64, true)
                            .into(),
                    ])
                    .into())
            }
            (
                Expr {
                    kind: ExprKind::FloatLit(value),
                    ..
                },
                Type::F32,
            ) => Ok(const_float_literal(self.context.f32_type(), value).into()),
            (
                Expr {
                    kind: ExprKind::FloatLit(value),
                    ..
                },
                Type::F64,
            ) => Ok(const_float_literal(self.context.f64_type(), value).into()),
            (
                Expr {
                    kind: ExprKind::FloatLit(value),
                    ..
                },
                Type::F128,
            ) => Ok(const_float_literal(self.context.f128_type(), value).into()),
            (_, Type::Ref(inner) | Type::MutRef(inner) | Type::Readonly(inner)) => {
                self.emit_expr_as(expr, inner)
            }
            (
                Expr {
                    kind: ExprKind::Null | ExprKind::None,
                    ..
                },
                Type::Optional(_),
            ) => Ok(self
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .const_null()
                .into()),
            (_, Type::Optional(_))
                if self
                    .expr_type_hint(expr)
                    .as_ref()
                    .is_some_and(|actual| actual == expected) =>
            {
                self.emit_expr(expr)
            }
            (_, Type::Optional(inner))
                if !matches!(expr.kind(), ExprKind::Null | ExprKind::None) =>
            {
                let value = self.emit_expr_as(expr, inner)?;
                let inner_plan = self.take_owned_native_result(value);
                let storage = self.emit_optional_box(value, inner)?;
                if let Some(plan) = inner_plan {
                    self.record_owned_native_result_with(
                        storage.into(),
                        Type::Optional(inner.clone()),
                        crate::ReleasePlan::Box(Box::new(plan)),
                    );
                }
                Ok(storage.into())
            }
            _ => {
                let source = self.expr_type_hint(expr);
                let value = self.emit_expr(expr)?;
                self.coerce_value_with_source(expr, value, source.as_ref(), expected)
            }
        }
    }

    pub(super) fn coerce_value(
        &mut self,
        value: BasicValueEnum<'ctx>,
        expected: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        self.coerce_value_with_source(&Expr::synthetic(ExprKind::Null), value, None, expected)
    }

    pub(super) fn emit_fallible_numeric_cast(
        &mut self,
        expr: &Expr,
        source: Option<&Type>,
        target: &Type,
    ) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        let source = transparent_codegen_type(source?);
        let (source_optional, payload) = match source {
            Type::Optional(inner) => (true, transparent_codegen_type(inner).clone()),
            other => (false, other.clone()),
        };
        let castable = |ty: &Type| is_numeric_type(ty) || matches!(ty, Type::Char);
        if !castable(&payload) || !castable(target) {
            return None;
        }
        if source_optional && &payload == transparent_codegen_type(target) {
            return None;
        }
        Some(self.emit_fallible_numeric_cast_inner(expr, &payload, source_optional, target))
    }

    pub(super) fn emit_fallible_numeric_cast_inner(
        &mut self,
        expr: &Expr,
        payload: &Type,
        source_optional: bool,
        target: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if !source_optional {
            let value = self.emit_expr_as(expr, payload)?;
            return Ok(self
                .emit_checked_payload_cast(value, payload, target)?
                .into());
        }

        let pointer_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let optional = self.emit_expr(expr)?.into_pointer_value();
        let source_plan = self.take_owned_native_result(optional.into());
        let source_ty = Type::Optional(Box::new(payload.clone()));
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .expect("internal: fallible cast outside function");
        let present_block = self.context.append_basic_block(function, "cast.present");
        let absent_block = self.context.append_basic_block(function, "cast.absent");
        let done_block = self.context.append_basic_block(function, "cast.done");

        let is_absent = self
            .builder
            .build_is_null(optional, "cast.is_absent")
            .unwrap();
        self.builder
            .build_conditional_branch(is_absent, absent_block, present_block)
            .unwrap();

        self.builder.position_at_end(present_block);
        let loaded = self
            .builder
            .build_load(self.llvm_type(payload), optional, "cast.payload")
            .unwrap();
        if let Some(plan) = &source_plan {
            self.emit_owned_native_release(optional.into(), &source_ty, plan);
        }
        let converted = self.emit_checked_payload_cast(loaded, payload, target)?;
        let present_end = self.builder.get_insert_block().unwrap();
        self.builder.build_unconditional_branch(done_block).unwrap();

        self.builder.position_at_end(absent_block);
        if let Some(plan) = &source_plan {
            self.emit_owned_native_release(optional.into(), &source_ty, plan);
        }
        self.builder.build_unconditional_branch(done_block).unwrap();

        self.builder.position_at_end(done_block);
        let result = self.builder.build_phi(pointer_type, "cast.result").unwrap();
        result.add_incoming(&[
            (&converted, present_end),
            (&pointer_type.const_null(), absent_block),
        ]);
        Ok(result.as_basic_value())
    }

    pub(super) fn emit_checked_payload_cast(
        &mut self,
        value: BasicValueEnum<'ctx>,
        source: &Type,
        target: &Type,
    ) -> Result<PointerValue<'ctx>, String> {
        let Some(fits) = self.numeric_cast_fits(value, source, target) else {
            let converted = self.numeric_cast_convert(value, source, target)?;
            return self.emit_optional_box(converted, target);
        };

        let pointer_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .expect("internal: fallible cast outside function");
        let fits_block = self.context.append_basic_block(function, "cast.fits");
        let empty_block = self.context.append_basic_block(function, "cast.empty");
        let merge_block = self.context.append_basic_block(function, "cast.merge");
        self.builder
            .build_conditional_branch(fits, fits_block, empty_block)
            .unwrap();

        self.builder.position_at_end(fits_block);
        let converted = self.numeric_cast_convert(value, source, target)?;
        let boxed = self.emit_optional_box(converted, target)?;
        let fits_end = self.builder.get_insert_block().unwrap();
        self.builder
            .build_unconditional_branch(merge_block)
            .unwrap();

        self.builder.position_at_end(empty_block);
        self.builder
            .build_unconditional_branch(merge_block)
            .unwrap();

        self.builder.position_at_end(merge_block);
        let result = self.builder.build_phi(pointer_type, "cast.value").unwrap();
        result.add_incoming(&[
            (&boxed, fits_end),
            (&pointer_type.const_null(), empty_block),
        ]);
        Ok(result.as_basic_value().into_pointer_value())
    }

    pub(super) fn numeric_cast_fits(
        &self,
        value: BasicValueEnum<'ctx>,
        source: &Type,
        target: &Type,
    ) -> Option<IntValue<'ctx>> {
        let char_source = matches!(transparent_codegen_type(source), Type::Char);
        let source = if char_source { &Type::U32 } else { source };
        if matches!(transparent_codegen_type(target), Type::Char) && is_integer_type(source) {
            return Some(self.emit_char_fits(value.into_int_value(), source));
        }
        if is_integer_type(source) && is_integer_type(target) {
            let (source_min, source_max) = int_type_bounds(source)?;
            let (target_min, target_max) = int_type_bounds(target)?;
            let below = source_min < target_min;
            let above = match (source_max, target_max) {
                (_, None) => false,
                (None, Some(_)) => true,
                (Some(source_max), Some(target_max)) => source_max > target_max,
            };
            if !below && !above {
                return None;
            }
            let value = value.into_int_value();
            let unsigned = is_unsigned_int_type(source);
            let widened = if unsigned {
                self.builder
                    .build_int_z_extend(value, self.context.i128_type(), "cast.widened")
                    .unwrap()
            } else {
                self.builder
                    .build_int_s_extend(value, self.context.i128_type(), "cast.widened")
                    .unwrap()
            };
            let at_least = below.then(|| {
                self.builder
                    .build_int_compare(
                        IntPredicate::SGE,
                        widened,
                        self.const_i128(target_min),
                        "cast.min",
                    )
                    .unwrap()
            });
            let at_most = target_max.filter(|_| above).map(|target_max| {
                let predicate = if unsigned {
                    IntPredicate::ULE
                } else {
                    IntPredicate::SLE
                };
                self.builder
                    .build_int_compare(predicate, widened, self.const_i128(target_max), "cast.max")
                    .unwrap()
            });
            return match (at_least, at_most) {
                (Some(at_least), Some(at_most)) => Some(
                    self.builder
                        .build_and(at_least, at_most, "cast.fits")
                        .unwrap(),
                ),
                (Some(only), None) | (None, Some(only)) => Some(only),
                (None, None) => None,
            };
        }

        if is_float_type(source) && is_integer_type(target) {
            let bits = int_type_bits(target)?;
            let value = value.into_float_value();
            let float_type = value.get_type();
            let (low, high) = if is_unsigned_int_type(target) {
                (0.0, 2.0_f64.powi(bits as i32))
            } else {
                let magnitude = 2.0_f64.powi(bits as i32 - 1);
                (-magnitude, magnitude)
            };
            let at_least = self
                .builder
                .build_float_compare(
                    inkwell::FloatPredicate::OGE,
                    value,
                    float_type.const_float(low),
                    "cast.min",
                )
                .unwrap();
            let below_high = self
                .builder
                .build_float_compare(
                    inkwell::FloatPredicate::OLT,
                    value,
                    float_type.const_float(high),
                    "cast.max",
                )
                .unwrap();
            return Some(
                self.builder
                    .build_and(at_least, below_high, "cast.fits")
                    .unwrap(),
            );
        }

        None
    }

    pub(super) fn numeric_cast_convert(
        &mut self,
        value: BasicValueEnum<'ctx>,
        source: &Type,
        target: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let char_source = matches!(transparent_codegen_type(source), Type::Char);
        let source = if char_source { &Type::U32 } else { source };
        let char_target = matches!(transparent_codegen_type(target), Type::Char);
        let target = if char_target { &Type::U32 } else { target };
        let unsigned = is_unsigned_int_type(source);
        if is_integer_type(source) && is_integer_type(target) {
            let value = value.into_int_value();
            let target_type = self.llvm_type(target).into_int_type();
            let source_bits = value.get_type().get_bit_width();
            let target_bits = target_type.get_bit_width();
            if source_bits > target_bits {
                return Ok(self
                    .builder
                    .build_int_truncate(value, target_type, "cast.truncate")
                    .unwrap()
                    .into());
            }
            if source_bits < target_bits {
                return Ok(if unsigned {
                    self.builder
                        .build_int_z_extend(value, target_type, "cast.zextend")
                        .unwrap()
                        .into()
                } else {
                    self.builder
                        .build_int_s_extend(value, target_type, "cast.sextend")
                        .unwrap()
                        .into()
                });
            }
            return Ok(value.into());
        }

        if is_float_type(source) && is_integer_type(target) {
            let value = value.into_float_value();
            let target_type = self.llvm_type(target).into_int_type();
            return Ok(if is_unsigned_int_type(target) {
                self.builder
                    .build_float_to_unsigned_int(value, target_type, "cast.float_to_uint")
                    .unwrap()
                    .into()
            } else {
                self.builder
                    .build_float_to_signed_int(value, target_type, "cast.float_to_int")
                    .unwrap()
                    .into()
            });
        }

        self.coerce_value_with_source(
            &Expr::synthetic(ExprKind::Null),
            value,
            Some(source),
            target,
        )
    }

    pub(crate) fn coerce_value_with_source(
        &mut self,
        _expr: &Expr,
        value: BasicValueEnum<'ctx>,
        source: Option<&Type>,
        expected: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if let Some(source_ty) = source {
            if matches!(transparent_codegen_type(expected), Type::Decimal)
                && value.is_int_value()
                && is_integer_type(source_ty)
            {
                return Ok(self.emit_integer_to_decimal(value.into_int_value(), source_ty));
            }
            if is_integer_type(expected)
                && value.is_struct_value()
                && matches!(transparent_codegen_type(source_ty), Type::Decimal)
            {
                return self.emit_decimal_to_integer(value.into_struct_value(), expected);
            }
            let source_is_char = matches!(transparent_codegen_type(source_ty), Type::Char);
            if source_is_char && value.is_int_value() {
                if matches!(transparent_codegen_type(expected), Type::Str) {
                    let text = self.emit_format_char(value.into_int_value());
                    self.record_owned_native_result(text.into(), Type::Str);
                    return Ok(text.into());
                }
                if is_integer_type(expected) {
                    return self.coerce_value_with_source(_expr, value, Some(&Type::U32), expected);
                }
            }
            if matches!(transparent_codegen_type(expected), Type::Char)
                && value.is_int_value()
                && is_integer_type(source_ty)
            {
                self.emit_char_range_check(value.into_int_value(), source_ty);
                return self.numeric_cast_convert(value, source_ty, expected);
            }
        }
        let source_unsigned = source.is_some_and(is_unsigned_int_type);
        let target_unsigned = is_unsigned_int_type(expected);
        match expected {
            ty if is_integer_type(ty) && value.is_int_value() => {
                let value = value.into_int_value();
                let target = self.llvm_type(ty).into_int_type();
                let source_bits = value.get_type().get_bit_width();
                let target_bits = target.get_bit_width();
                self.emit_integer_cast_range_check(value, source, ty);
                if source_bits < target_bits {
                    if source_unsigned {
                        Ok(self
                            .builder
                            .build_int_z_extend(value, target, "int_zextend")
                            .unwrap()
                            .into())
                    } else {
                        Ok(self
                            .builder
                            .build_int_s_extend(value, target, "int_sextend")
                            .unwrap()
                            .into())
                    }
                } else if source_bits > target_bits {
                    Ok(self
                        .builder
                        .build_int_truncate(value, target, "int_truncate")
                        .unwrap()
                        .into())
                } else {
                    Ok(value.into())
                }
            }
            ty if is_float_type(ty) && value.is_float_value() => {
                let value = value.into_float_value();
                let target = self.llvm_type(ty).into_float_type();
                if value.get_type() == target {
                    Ok(value.into())
                } else if target.get_bit_width() > value.get_type().get_bit_width() {
                    Ok(self
                        .builder
                        .build_float_ext(value, target, "float_extend")
                        .unwrap()
                        .into())
                } else {
                    Ok(self
                        .builder
                        .build_float_trunc(value, target, "float_truncate")
                        .unwrap()
                        .into())
                }
            }
            ty if is_float_type(ty) && value.is_int_value() => {
                let int_val = value.into_int_value();
                let target = self.llvm_type(ty).into_float_type();
                if source_unsigned {
                    Ok(self
                        .builder
                        .build_unsigned_int_to_float(int_val, target, "uint_to_float")
                        .unwrap()
                        .into())
                } else {
                    Ok(self
                        .builder
                        .build_signed_int_to_float(int_val, target, "int_to_float")
                        .unwrap()
                        .into())
                }
            }
            ty if is_integer_type(ty) && value.is_float_value() => {
                let float_val = value.into_float_value();
                let target = self.llvm_type(ty).into_int_type();
                if target_unsigned {
                    Ok(self
                        .builder
                        .build_float_to_unsigned_int(float_val, target, "float_to_uint")
                        .unwrap()
                        .into())
                } else {
                    Ok(self
                        .builder
                        .build_float_to_signed_int(float_val, target, "float_to_int")
                        .unwrap()
                        .into())
                }
            }
            Type::Optional(inner) | Type::Ref(inner) | Type::MutRef(inner) => {
                self.coerce_value(value, inner)
            }
            _ => Ok(value),
        }
    }

    pub(super) fn emit_regex_field_access(
        &mut self,
        object: &Expr,
        field: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i64_type = self.context.i64_type();
        let bool_type = self.context.bool_type();
        let i32_type = self.context.i32_type();

        if let ExprKind::RegexLit { pattern, flags } = object.kind() {
            return match field {
                "source" => Ok(self
                    .const_string_abi(pattern, "regex_source")
                    .into_struct_value()
                    .into()),
                "flags" => Ok(self
                    .const_string_abi(flags, "regex_flags")
                    .into_struct_value()
                    .into()),
                "global" | "sticky" | "unicode" | "hasIndices" => Ok(bool_type.const_zero().into()),
                "ignoreCase" => {
                    let val = if flags.contains('i') { 1 } else { 0 };
                    Ok(bool_type.const_int(val, false).into())
                }
                "multiline" => {
                    let val = if flags.contains('m') { 1 } else { 0 };
                    Ok(bool_type.const_int(val, false).into())
                }
                "dotAll" => {
                    let val = if flags.contains('s') { 1 } else { 0 };
                    Ok(bool_type.const_int(val, false).into())
                }
                "lastIndex" => Ok(i32_type.const_zero().into()),
                _ => Err(format!("unknown regex property `{field}`")),
            };
        }

        let regex_ptr = self.emit_expr(object)?.into_struct_value();
        match field {
            "source" => {
                let fn_val = self.declare_libc_fn(
                    "tyrus_regex_get_source",
                    &[self.string_abi_type()],
                    Some(self.string_abi_type()),
                );
                let result = self
                    .builder
                    .build_call(fn_val, &[regex_ptr.into()], "regex_source")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.record_owned_native_result(result, Type::Str);
                Ok(result)
            }
            "flags" => {
                let fn_val = self.declare_libc_fn(
                    "tyrus_regex_get_flags",
                    &[self.string_abi_type()],
                    Some(self.string_abi_type()),
                );
                let result = self
                    .builder
                    .build_call(fn_val, &[regex_ptr.into()], "regex_flags")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.record_owned_native_result(result, Type::Str);
                Ok(result)
            }
            "global" | "sticky" | "unicode" | "hasIndices" => Ok(bool_type.const_zero().into()),
            "ignoreCase" => {
                let fn_val = self.declare_libc_fn(
                    "tyrus_regex_has_flag",
                    &[self.string_abi_type(), i64_type.into()],
                    Some(i64_type.into()),
                );
                let flag_char = i64_type.const_int(b'i' as u64, false);
                let result = self
                    .builder
                    .build_call(
                        fn_val,
                        &[regex_ptr.into(), flag_char.into()],
                        "regex_ignoreCase",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                let one = i64_type.const_int(1, false);
                let is_true = self
                    .builder
                    .build_int_compare(inkwell::IntPredicate::EQ, result, one, "is_ic")
                    .unwrap();
                Ok(self
                    .builder
                    .build_int_cast(is_true, bool_type, "ic_bool")
                    .unwrap()
                    .into())
            }
            "multiline" => {
                let fn_val = self.declare_libc_fn(
                    "tyrus_regex_has_flag",
                    &[self.string_abi_type(), i64_type.into()],
                    Some(i64_type.into()),
                );
                let flag_char = i64_type.const_int(b'm' as u64, false);
                let result = self
                    .builder
                    .build_call(
                        fn_val,
                        &[regex_ptr.into(), flag_char.into()],
                        "regex_multiline",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                let one = i64_type.const_int(1, false);
                let is_true = self
                    .builder
                    .build_int_compare(inkwell::IntPredicate::EQ, result, one, "is_ml")
                    .unwrap();
                Ok(self
                    .builder
                    .build_int_cast(is_true, bool_type, "ml_bool")
                    .unwrap()
                    .into())
            }
            "dotAll" => {
                let fn_val = self.declare_libc_fn(
                    "tyrus_regex_has_flag",
                    &[self.string_abi_type(), i64_type.into()],
                    Some(i64_type.into()),
                );
                let flag_char = i64_type.const_int(b's' as u64, false);
                let result = self
                    .builder
                    .build_call(
                        fn_val,
                        &[regex_ptr.into(), flag_char.into()],
                        "regex_dotAll",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                let one = i64_type.const_int(1, false);
                let is_true = self
                    .builder
                    .build_int_compare(inkwell::IntPredicate::EQ, result, one, "is_da")
                    .unwrap();
                Ok(self
                    .builder
                    .build_int_cast(is_true, bool_type, "da_bool")
                    .unwrap()
                    .into())
            }
            "lastIndex" => Ok(i32_type.const_zero().into()),
            _ => Err(format!("unknown regex property `{field}`")),
        }
    }

    pub(super) fn emit_place_ptr_object_field(
        &mut self,
        object: &Expr,
        field: &str,
    ) -> Result<(PointerValue<'ctx>, Type), String> {
        let (base_ptr, base_ty) = self.emit_place_ptr(object)?;
        let resolved = transparent_codegen_type(&base_ty).clone();
        let (index, field_ty) = match &resolved {
            Type::Named(name) => {
                let fields = self
                    .interfaces
                    .get(name)
                    .ok_or_else(|| format!("error: unknown type `{name}` for field assignment"))?;
                let index = fields
                    .iter()
                    .position(|candidate| candidate.name == *field)
                    .ok_or_else(|| format!("error: interface `{name}` has no field `{field}`"))?;
                (index, field_storage_type(&fields[index]))
            }
            Type::Object(fields) => {
                let index = self
                    .struct_field_index(&resolved, field)
                    .ok_or_else(|| format!("error: object has no field `{field}`"))?;
                let field_type = fields
                    .iter()
                    .find(|candidate| candidate.name == *field)
                    .map(field_storage_type)
                    .expect("object field index was resolved from the same type");
                (index as usize, field_type)
            }
            other => {
                return Err(format!(
                    "error: cannot assign to field `{field}` of `{other}`"
                ));
            }
        };
        let struct_ty = self.llvm_type(&resolved).into_struct_type();
        let field_ptr = self
            .builder
            .build_struct_gep(struct_ty, base_ptr, index as u32, field)
            .unwrap();
        Ok((field_ptr, field_ty))
    }

    pub(crate) fn emit_place_ptr(
        &mut self,
        expr: &Expr,
    ) -> Result<(PointerValue<'ctx>, Type), String> {
        match expr.kind() {
            ExprKind::Ident(name) => self
                .load_var(name)
                .ok_or_else(|| format!("internal: `{name}` not in codegen scope")),
            ExprKind::FieldAccess { object, field } if field == "value" => {
                let object_type = self.expr_type_hint(object).map(|ty| self.resolve_type(&ty));
                if let Some(Type::Apply { name, args }) = object_type {
                    if name == "MutexGuard" && args.len() == 1 {
                        let payload_ty = self.resolve_type(&args[0]);
                        let guard = self.emit_expr(object)?.into_pointer_value();
                        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                        let storage_fn = self.declare_libc_fn(
                            "tyrus_mutex_guard_storage",
                            &[pointer.into()],
                            Some(pointer.into()),
                        );
                        let storage = self
                            .builder
                            .build_call(storage_fn, &[guard.into()], "mutex.guard.storage")
                            .unwrap()
                            .try_as_basic_value()
                            .basic()
                            .unwrap()
                            .into_pointer_value();
                        return Ok((storage, payload_ty));
                    }
                    if name == "RwWriteGuard" && args.len() == 1 {
                        let payload_ty = self.resolve_type(&args[0]);
                        let guard = self.emit_expr(object)?.into_pointer_value();
                        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                        let storage_fn = self.declare_libc_fn(
                            "tyrus_rwlock_write_guard_storage",
                            &[pointer.into()],
                            Some(pointer.into()),
                        );
                        let storage = self
                            .builder
                            .build_call(storage_fn, &[guard.into()], "rwlock.write_guard.storage")
                            .unwrap()
                            .try_as_basic_value()
                            .basic()
                            .unwrap()
                            .into_pointer_value();
                        return Ok((storage, payload_ty));
                    }
                }
                self.emit_place_ptr_object_field(object, field)
            }
            ExprKind::FieldAccess { object, field } => {
                self.emit_place_ptr_object_field(object, field)
            }
            ExprKind::IndexAccess { object, index } => {
                let (base_ptr, base_ty) = self.emit_place_ptr(object)?;
                let resolved = transparent_codegen_type(&base_ty).clone();
                match &resolved {
                    Type::Array(element) => {
                        let element = element.as_ref().clone();
                        let array = self
                            .builder
                            .build_load(self.llvm_type(&resolved), base_ptr, "array")
                            .unwrap()
                            .into_struct_value();
                        let data = self
                            .builder
                            .build_extract_value(array, 0, "array_data")
                            .unwrap()
                            .into_pointer_value();
                        let length = self
                            .builder
                            .build_extract_value(array, 1, "array_length")
                            .unwrap()
                            .into_int_value();
                        let proved = self.checked_indexes.contains(index.span);
                        let index = self.emit_expr_as(index, &Type::Usize)?.into_int_value();
                        if !proved {
                            self.emit_array_bounds_check(index, length)?;
                        }
                        let element_ptr = unsafe {
                            self.builder
                                .build_gep(
                                    self.llvm_type(&element),
                                    data,
                                    &[index],
                                    "array_index_target",
                                )
                                .unwrap()
                        };
                        Ok((element_ptr, element))
                    }
                    Type::Tuple(elements) => {
                        let ExprKind::IntLit(literal) = index.kind() else {
                            return Err("error: tuple index must be an integer literal".to_string());
                        };
                        let position = usize::try_from(*literal)
                            .map_err(|_| "error: tuple index out of bounds".to_string())?;
                        let element = elements
                            .get(position)
                            .ok_or_else(|| "error: tuple index out of bounds".to_string())?
                            .clone();
                        let struct_ty = self.llvm_type(&resolved).into_struct_type();
                        let element_ptr = self
                            .builder
                            .build_struct_gep(
                                struct_ty,
                                base_ptr,
                                position as u32,
                                "tuple_index_target",
                            )
                            .unwrap();
                        Ok((element_ptr, element))
                    }
                    other => Err(format!("error: cannot index into type `{other}`")),
                }
            }
            _ => Err("error: invalid assignment target".to_string()),
        }
    }
}
