use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_destructure_stmt(
        &mut self,
        stmt: &Stmt,
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        match stmt.kind() {
            StmtKind::ArrayDestructure {
                mutable: _,
                bindings,
                value,
            } => {
                let array_type = self
                    .expr_type_hint(value)
                    .ok_or_else(|| "error: cannot infer array type for destructure".to_string())?;
                if let Type::Tuple(elements) = transparent_codegen_type(&array_type) {
                    let tuple = self.emit_expr(value)?;
                    // A direct call's tuple return registers itself wholesale
                    // (`record_owned_native_result`) so *something* frees its
                    // owned elements; nothing here used to claim that
                    // registration, so it stayed pending past this statement.
                    // Whatever next called `discard_owned_native_result`,
                    // possibly, if this destructure was the last statement in
                    // an imported function, code emitted for an entirely
                    // different function afterward, froze a stale LLVM value
                    // reference into an unrelated free, an invalid
                    // cross-function reference the verifier rejects outright
                    // (a bound local read from a plain variable, never
                    // registered here, never took this path). Taking the plan
                    // now and redistributing it per element is what a field
                    // extracted from an object literal already does; each
                    // non-`Copy` element becomes its own owner, released at
                    // its own binding's scope exit instead of through a
                    // dangling reference to the whole tuple.
                    let tuple_owned = self.take_owned_native_result(tuple).is_some();
                    let tuple = tuple.into_struct_value();
                    for (index, binding) in bindings.iter().enumerate() {
                        let Some(element_ty) = elements.get(index) else {
                            return Err("error: tuple destructure index out of bounds".to_string());
                        };
                        let resolved_ty = self.resolve_type(element_ty);
                        let value = self
                            .builder
                            .build_extract_value(tuple, index as u32, &binding.name)
                            .map_err(|error| error.to_string())?;
                        let slot = self
                            .async_frame_slot(&binding.name)
                            .map(|(slot, _)| slot)
                            .map_or_else(
                                || {
                                    self.builder
                                        .build_alloca(self.llvm_type(&resolved_ty), &binding.name)
                                        .map_err(|error| error.to_string())
                                },
                                Ok,
                            )?;
                        self.clear_async_frame_local_before_store(
                            fn_val,
                            &binding.name,
                            slot,
                            &resolved_ty,
                        )?;
                        self.builder.build_store(slot, value).unwrap();
                        self.store_var(&binding.name, slot, resolved_ty.clone());
                        if self.async_frame_owns_value(&binding.name) {
                            self.mark_async_frame_value_initialized(&binding.name, true);
                        } else if tuple_owned && self.async_owned_value_supported(&resolved_ty) {
                            self.mark_owned_synchronous_value(&binding.name);
                        }
                    }
                    return Ok(());
                }
                let Type::Array(element) = transparent_codegen_type(&array_type) else {
                    return Err(format!(
                        "error: cannot destructure non-array type `{array_type}`"
                    ));
                };
                let resolved_element = self.resolve_type(element);
                let element_llvm = self.llvm_type(&resolved_element);
                let array = self.emit_expr(value)?.into_struct_value();
                let data = self
                    .builder
                    .build_extract_value(array, 0, "destr_data")
                    .unwrap()
                    .into_pointer_value();
                let length = self
                    .builder
                    .build_extract_value(array, 1, "destr_length")
                    .unwrap()
                    .into_int_value();
                let i64_type = self.context.i64_type();
                for (i, binding) in bindings.iter().enumerate() {
                    let idx = i64_type.const_int(i as u64, false);
                    self.emit_array_bounds_check(idx, length)?;
                    let slot = unsafe {
                        self.builder
                            .build_gep(element_llvm, data, &[idx], &binding.name)
                            .unwrap()
                    };
                    let val = self
                        .builder
                        .build_load(element_llvm, slot, &binding.name)
                        .map_err(|e| e.to_string())?;
                    let alloca = self
                        .builder
                        .build_alloca(element_llvm, &binding.name)
                        .unwrap();
                    self.builder.build_store(alloca, val).unwrap();
                    self.store_var(&binding.name, alloca, resolved_element.clone());
                }
                Ok(())
            }
            StmtKind::ObjectDestructure {
                mutable: _,
                bindings,
                value,
            } => {
                if self.is_process_env(value) {
                    for binding in bindings {
                        let ty = self.env_declared_type(&binding.name).ok_or_else(|| {
                            format!(
                                "internal: env variable `{}` missing from the env schema",
                                binding.name
                            )
                        })?;
                        let value = self.emit_env_get(&binding.name, &ty)?;
                        let resolved_ty = self.resolve_type(&ty);
                        let llvm_ty = self.llvm_type(&resolved_ty);
                        let alloca = self.builder.build_alloca(llvm_ty, &binding.name).unwrap();
                        self.builder.build_store(alloca, value).unwrap();
                        self.store_var(&binding.name, alloca, resolved_ty.clone());
                        if matches!(resolved_ty, Type::Str) {
                            self.mark_owned_synchronous_value(&binding.name);
                        }
                    }
                    return Ok(());
                }
                let object_type = self
                    .expr_type_hint(value)
                    .ok_or_else(|| "error: cannot infer object type for destructure".to_string())?;
                let Type::Named(name) = transparent_codegen_type(&object_type) else {
                    return Err(format!(
                        "error: cannot destructure non-object type `{object_type}`"
                    ));
                };
                let val = self.emit_expr(value)?;
                let interface = self
                    .interfaces
                    .get(name.as_str())
                    .cloned()
                    .unwrap_or_default();
                for binding in bindings {
                    let index = interface
                        .iter()
                        .position(|candidate| candidate.name == binding.name)
                        .ok_or_else(|| {
                            format!("error: type `{name}` has no field `{}`", binding.name)
                        })?;
                    let field_val = self
                        .builder
                        .build_extract_value(val.into_struct_value(), index as u32, &binding.name)
                        .map_err(|e| e.to_string())?;
                    let field_ty = field_storage_type(&interface[index]);
                    let resolved_ty = self.resolve_type(&field_ty);
                    let llvm_ty = self.llvm_type(&resolved_ty);
                    let alloca = self.builder.build_alloca(llvm_ty, &binding.name).unwrap();
                    self.builder.build_store(alloca, field_val).unwrap();
                    self.store_var(&binding.name, alloca, resolved_ty);
                }
                Ok(())
            }
            _ => Err("internal: not a destructuring statement".to_string()),
        }
    }
}
