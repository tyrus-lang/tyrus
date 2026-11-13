use super::*;
impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_memory_swap(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let [left, right] = args else {
            return Err("error: `swap` expects 2 arguments".to_string());
        };
        let ExprKind::Ident(left_name) = left.kind() else {
            return Err("error: `swap` requires mutable local bindings".to_string());
        };
        let ExprKind::Ident(right_name) = right.kind() else {
            return Err("error: `swap` requires mutable local bindings".to_string());
        };
        if left_name == right_name {
            return Err("error: `swap` requires two distinct mutable local bindings".to_string());
        }
        let (left_slot, left_ty) = self.load_var(left_name).ok_or_else(|| {
            format!("error: `swap` requires a local binding, found `{left_name}`")
        })?;
        let (right_slot, right_ty) = self.load_var(right_name).ok_or_else(|| {
            format!("error: `swap` requires a local binding, found `{right_name}`")
        })?;
        let supported =
            left_ty.is_copy() || matches!(transparent_codegen_type(&left_ty), Type::Str);
        if left_ty != right_ty || !supported {
            return Err(format!(
                "error: `swap` requires mutable bindings of the same `Copy` type or `string`, found `{left_ty}` and `{right_ty}`"
            ));
        }
        let llvm_ty = self.llvm_type(&left_ty);
        let left_value = self
            .builder
            .build_load(llvm_ty, left_slot, "memory.swap.left")
            .map_err(|error| error.to_string())?;
        let right_value = self
            .builder
            .build_load(llvm_ty, right_slot, "memory.swap.right")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(left_slot, right_value)
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(right_slot, left_value)
            .map_err(|error| error.to_string())?;
        Ok(self.context.i8_type().const_zero().into())
    }

    pub(super) fn emit_memory_replace(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let [destination, source] = args else {
            return Err("error: `replace` expects 2 arguments".to_string());
        };
        let ExprKind::Ident(name) = destination.kind() else {
            return Err(
                "error: `replace` requires a mutable local binding as its first argument"
                    .to_string(),
            );
        };
        let (slot, ty) = self
            .load_var(name)
            .ok_or_else(|| format!("error: `replace` requires a local binding, found `{name}`"))?;
        if matches!(transparent_codegen_type(&ty), Type::Str) {
            let ExprKind::Ident(source_name) = source.kind() else {
                return Err("error: `replace` requires a mutable local `string` source".to_string());
            };
            let (source_slot, source_ty) = self.load_var(source_name).ok_or_else(|| {
                format!("error: `replace` requires a local binding, found `{source_name}`")
            })?;
            if !matches!(transparent_codegen_type(&source_ty), Type::Str) {
                return Err(format!(
                    "error: `replace` requires matching types, found `{ty}` and `{source_ty}`"
                ));
            }
            let function = self
                .builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_parent)
                .ok_or_else(|| "error: `replace` is not inside a function".to_string())?;
            let destination_bit = self.mutable_string_ownership_slot(name).ok_or_else(|| {
                "error: `replace` requires a mutable local `string` destination".to_string()
            })?;
            let source_bit = self
                .mutable_string_ownership_slot(source_name)
                .ok_or_else(|| {
                    "error: `replace` requires a mutable local `string` source".to_string()
                })?;
            let old = self
                .builder
                .build_load(self.string_abi_type(), slot, "memory.replace.old")
                .map_err(|error| error.to_string())?;
            let old_owned = self
                .builder
                .build_load(
                    self.context.bool_type(),
                    destination_bit,
                    "memory.replace.old.owned",
                )
                .map_err(|error| error.to_string())?
                .into_int_value();
            let replacement = self
                .builder
                .build_load(self.string_abi_type(), source_slot, "memory.replace.source")
                .map_err(|error| error.to_string())?
                .into_struct_value();
            let source_owned = self
                .builder
                .build_load(
                    self.context.bool_type(),
                    source_bit,
                    "memory.replace.source.owned",
                )
                .map_err(|error| error.to_string())?
                .into_int_value();
            self.replace_mutable_string_ownership_with_bit(
                function,
                name,
                slot,
                destination_bit,
                replacement,
                source_owned,
            );
            self.builder
                .build_store(source_slot, self.string_abi_type().const_zero())
                .map_err(|error| error.to_string())?;
            self.builder
                .build_store(source_bit, self.context.bool_type().const_zero())
                .map_err(|error| error.to_string())?;
            self.record_conditionally_owned_result(old, Type::Str, old_owned);
            return Ok(old);
        }
        if !ty.is_copy() {
            return Err(format!(
                "error: `replace` requires a mutable binding of a `Copy` type, found `{ty}`"
            ));
        }
        let old = self
            .builder
            .build_load(self.llvm_type(&ty), slot, "memory.replace.old")
            .map_err(|error| error.to_string())?;
        let replacement = self.emit_expr_as(source, &ty)?;
        self.builder
            .build_store(slot, replacement)
            .map_err(|error| error.to_string())?;
        Ok(old)
    }

    pub(super) fn emit_memory_layout_query(
        &self,
        type_args: &[Type],
        args: &[Expr],
        alignment: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let operation = if alignment { "alignOf" } else { "sizeOf" };
        if !args.is_empty() || type_args.len() != 1 {
            return Err(format!(
                "error: `{operation}` expects exactly one type argument and no value arguments"
            ));
        }
        let ty = &type_args[0];
        if !tyrus_core::memory::has_stable_layout(ty) {
            return Err(format!(
                "error: `{operation}` requires a primitive type with stable layout, found `{ty}`"
            ));
        }
        let pointer_bytes = u64::from(self.usize_type().get_bit_width() / 8);
        let value = match ty {
            Type::I8 | Type::U8 | Type::Bool => 1,
            Type::I16 | Type::U16 => 2,
            Type::I32 | Type::U32 | Type::F32 | Type::Char => 4,
            Type::I64 | Type::U64 | Type::F64 => 8,
            Type::I128 | Type::U128 | Type::F128 => 16,
            Type::Isize | Type::Usize => pointer_bytes,
            _ => {
                return Err(format!(
                    "error: `{operation}` could not determine the layout of `{ty}`"
                ));
            }
        };
        Ok(self.usize_type().const_int(value, false).into())
    }

    pub(super) fn emit_memory_drop(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let [argument] = args else {
            return Err("error: `drop` expects 1 argument".to_string());
        };
        let ExprKind::Ident(name) = argument.kind() else {
            return Err("error: `drop` requires a local binding".to_string());
        };
        let Some((slot, ty)) = self.load_var(name) else {
            return Err(format!(
                "error: `drop` requires a local binding, found `{name}`"
            ));
        };
        if !tyrus_core::memory::supports_drop(transparent_codegen_type(&ty)) {
            return Err(format!(
                "error: `drop` does not support `{ty}`; expected an owned string, array, collection, or native handle"
            ));
        }
        let async_frame_slot = self
            .async_frame_slots
            .last()
            .and_then(|slots| slots.get(name))
            .is_some_and(|(candidate, _)| candidate.as_value_ref() == slot.as_value_ref());
        if async_frame_slot {
            let function = self
                .builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_parent)
                .ok_or_else(|| "error: `drop` is not inside a function".to_string())?;
            self.clear_async_frame_local_before_store(function, name, slot, &ty)?;
            self.mark_async_frame_value_initialized(name, false);
            self.dispose_owned_native_binding(name);
            self.builder
                .build_store(slot, self.llvm_type(&ty).const_zero())
                .map_err(|error| error.to_string())?;
            return Ok(self.context.i8_type().const_zero().into());
        }
        if matches!(transparent_codegen_type(&ty), Type::Str | Type::Regex) {
            let value = self
                .builder
                .build_load(self.string_abi_type(), slot, "memory.drop.string")
                .unwrap()
                .into_struct_value();
            self.emit_string_release(value)?;
        } else if matches!(transparent_codegen_type(&ty), Type::Array(_)) {
            if let Some(scope_index) = (0..self.vars.len())
                .rev()
                .find(|index| self.vars[*index].contains_key(name))
                && let Some(plan) = self.owned_native_vars[scope_index].get(name).cloned()
            {
                let value = self
                    .builder
                    .build_load(self.llvm_type(&ty), slot, "memory.drop.value")
                    .unwrap();
                self.emit_owned_native_release(value, &ty, &plan);
            }
        } else {
            let value = self
                .builder
                .build_load(self.llvm_type(&ty), slot, "memory.drop.native")
                .map_err(|error| error.to_string())?;
            self.emit_owned_native_release(value, &ty, &ReleasePlan::Deep);
        }
        self.dispose_owned_native_binding(name);
        self.builder
            .build_store(slot, self.llvm_type(&ty).const_zero())
            .map_err(|error| error.to_string())?;
        Ok(self.context.i8_type().const_zero().into())
    }

    pub(super) fn emit_memory_zeroize(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let [argument] = args else {
            return Err("error: `zeroize` expects 1 argument".to_string());
        };
        let ExprKind::Ident(name) = argument.kind() else {
            return Err("error: `zeroize` requires a local `string` or `u8[]` binding".to_string());
        };
        let Some((slot, ty)) = self.load_var(name) else {
            return Err(format!(
                "error: `zeroize` requires a local binding, found `{name}`"
            ));
        };
        let result_type = Type::Result {
            success: Box::new(Type::Void),
            error: Box::new(Type::Named("MemoryError".to_string())),
        };
        let result_llvm_type = self.llvm_type(&result_type);
        let result_slot = self
            .builder
            .build_alloca(result_llvm_type, "memory.zeroize.result")
            .map_err(|error| error.to_string())?;
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let (symbol, arguments) = match transparent_codegen_type(&ty) {
            Type::Str => {
                let value = self
                    .builder
                    .build_load(self.string_abi_type(), slot, "memory.zeroize.string")
                    .map_err(|error| error.to_string())?;
                ("tyrus_memory_zeroize_string_result", vec![value.into()])
            }
            Type::Array(element) if matches!(element.as_ref(), Type::U8) => {
                ("tyrus_memory_zeroize_u8_array_result", vec![slot.into()])
            }
            other => {
                return Err(format!(
                    "error: `zeroize` supports only `string` and `u8[]`, found `{other}`"
                ));
            }
        };
        let mut parameter_types = if symbol.ends_with("string_result") {
            vec![self.string_abi_type()]
        } else {
            vec![pointer.into()]
        };
        parameter_types.push(pointer.into());
        let function = self.declare_libc_fn(symbol, &parameter_types, None);
        let mut call_arguments = arguments;
        call_arguments.push(result_slot.into());
        self.builder
            .build_call(function, &call_arguments, "memory.zeroize.call")
            .map_err(|error| error.to_string())?;
        let result = self
            .builder
            .build_load(result_llvm_type, result_slot, "memory.zeroize.value")
            .map_err(|error| error.to_string())?;
        self.dispose_owned_native_binding(name);
        self.builder
            .build_store(slot, self.llvm_type(&ty).const_zero())
            .map_err(|error| error.to_string())?;
        self.record_owned_native_result(result, result_type);
        Ok(result)
    }
}
