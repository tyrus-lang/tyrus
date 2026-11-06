use super::*;

/// The enclosing function's `try`, `catch`, `finally` and loop bookkeeping.
/// It holds values and blocks of that function, so a nested function must not
/// see it: a `return` inside a closure written in a `try` used to restore the
/// outer `try` context from it, which referred to an instruction of another
/// function and made the whole module invalid.
pub(super) struct EnclosingTryState<'ctx> {
    catch_context_restores: Vec<(PointerValue<'ctx>, PointerValue<'ctx>, usize)>,
    catch_binding_types: Vec<Type>,
    catch_error_owned_slots: Vec<(String, PointerValue<'ctx>)>,
    catch_binding_releases: Vec<crate::statements::CatchBindingRelease<'ctx>>,
    async_error_cleanups: Vec<(String, crate::core::AsyncErrorCleanup<'ctx>)>,
    catch_error_subtypes: Vec<crate::statements::ErrorSubtypeStorage<'ctx>>,
    finally_scopes: Vec<crate::FinallyScope>,
    loop_stack: Vec<(BasicBlock<'ctx>, BasicBlock<'ctx>, usize)>,
}

impl<'ctx> Codegen<'ctx> {
    pub(super) fn take_enclosing_try_state(&mut self) -> EnclosingTryState<'ctx> {
        EnclosingTryState {
            catch_context_restores: std::mem::take(&mut self.catch_context_restores),
            catch_binding_types: std::mem::take(&mut self.catch_binding_types),
            catch_error_owned_slots: std::mem::take(&mut self.catch_error_owned_slots),
            catch_binding_releases: std::mem::take(&mut self.catch_binding_releases),
            async_error_cleanups: std::mem::take(&mut self.async_error_cleanups),
            catch_error_subtypes: std::mem::take(&mut self.catch_error_subtypes),
            finally_scopes: std::mem::take(&mut self.finally_scopes),
            loop_stack: std::mem::take(&mut self.loop_stack),
        }
    }

    pub(super) fn restore_enclosing_try_state(&mut self, state: EnclosingTryState<'ctx>) {
        self.catch_context_restores = state.catch_context_restores;
        self.catch_binding_types = state.catch_binding_types;
        self.catch_error_owned_slots = state.catch_error_owned_slots;
        self.catch_binding_releases = state.catch_binding_releases;
        self.async_error_cleanups = state.async_error_cleanups;
        self.catch_error_subtypes = state.catch_error_subtypes;
        self.finally_scopes = state.finally_scopes;
        self.loop_stack = state.loop_stack;
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_arrow_fn(
        &mut self,
        params: &[Param],
        ret_type: &Type,
        body: &[Stmt],
        is_escaping: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if params
            .iter()
            .any(|parameter| matches!(parameter.ty, Type::Infer))
            || matches!(ret_type, Type::Infer)
        {
            return Err(
                "error: arrow function requires a contextual type for inferred parameters or return type"
                    .to_string(),
            );
        }
        let symbol = format!("__tyrus_arrow_{}", self.arrow_fn_count);
        self.arrow_fn_count += 1;

        let param_types: Vec<_> = params
            .iter()
            .map(|param| {
                if param.optional && param.default_value.is_none() && !param.rest {
                    Type::Optional(Box::new(param.ty.clone()))
                } else {
                    param.ty.clone()
                }
            })
            .collect();
        let capture_names = closure_captures(params, body);
        let mut captures = capture_names
            .into_iter()
            .filter_map(|name| self.load_var(&name).map(|(slot, ty)| (name, slot, ty)))
            .collect::<Vec<_>>();
        captures.sort_by(|left, right| left.0.cmp(&right.0));

        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());

        let mut env_drop: Option<FunctionValue<'ctx>> = None;
        self.closure_env_on_stack = false;
        let (environment_type, environment) = if captures.is_empty() {
            (self.context.struct_type(&[], false), pointer.const_null())
        } else if is_escaping {
            let i64_type = self.context.i64_type();
            let mut field_types = vec![i64_type.into(), pointer.into()];
            field_types.extend(captures.iter().map(|(_, _, ty)| self.llvm_type(ty)));
            let env_ty = self.context.struct_type(&field_types, false);

            env_drop = self.emit_escaping_env_drop(&symbol, env_ty, &captures)?;

            let size = env_ty.size_of().unwrap();
            let (malloc, size) = self.declare_malloc(size);
            let env = self
                .builder
                .build_call(malloc, &[size.into()], "closure_environment")
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap()
                .into_pointer_value();

            let count_slot = self
                .builder
                .build_struct_gep(env_ty, env, 0, "closure_env_refcount")
                .unwrap();
            self.builder
                .build_store(count_slot, i64_type.const_int(1, false))
                .unwrap();
            let drop_slot = self
                .builder
                .build_struct_gep(env_ty, env, 1, "closure_env_drop")
                .unwrap();
            let drop_ptr = match env_drop {
                Some(drop) => drop.as_global_value().as_pointer_value(),
                None => pointer.const_null(),
            };
            self.builder.build_store(drop_slot, drop_ptr).unwrap();

            for (index, (_, slot, ty)) in captures.iter().enumerate() {
                let field = self
                    .builder
                    .build_struct_gep(env_ty, env, index as u32 + 2, "capture")
                    .unwrap();
                let value = self
                    .builder
                    .build_load(self.llvm_type(ty), *slot, "capture_value")
                    .unwrap();
                if self.escaping_capture_is_owned(ty) {
                    self.emit_async_clone_value_into(field, value, ty)?;
                } else if self.capture_is_closure(ty) {
                    self.builder.build_store(field, value).unwrap();
                    self.emit_closure_retain(value);
                } else {
                    self.builder.build_store(field, value).unwrap();
                }
            }
            (env_ty, env)
        } else {
            let ptr_fields: Vec<inkwell::types::BasicTypeEnum<'ctx>> =
                captures.iter().map(|_| pointer.into()).collect();
            let env_ty = self.context.struct_type(&ptr_fields, false);
            let env = self
                .builder
                .build_alloca(env_ty, "closure_env_stack")
                .unwrap();
            self.closure_env_on_stack = true;
            for (index, (_, slot, _)) in captures.iter().enumerate() {
                let field = self
                    .builder
                    .build_struct_gep(env_ty, env, index as u32, "capture_ptr")
                    .unwrap();
                self.builder.build_store(field, *slot).unwrap();
            }
            (env_ty, env)
        };

        let fn_type = self.llvm_closure_fn_type(&param_types, ret_type);
        let fn_val = self.module.add_function(&symbol, fn_type, None);
        let entry = self.context.append_basic_block(fn_val, "entry");

        let outer_block = self.builder.get_insert_block();
        let outer_vars = std::mem::replace(&mut self.vars, vec![HashMap::default()]);
        let outer_owned_closures =
            std::mem::replace(&mut self.owned_closure_vars, vec![HashSet::default()]);
        let outer_owned_closure_drops =
            std::mem::replace(&mut self.owned_closure_drop_fns, vec![HashMap::default()]);
        let outer_owned_native =
            std::mem::replace(&mut self.owned_native_vars, vec![HashMap::default()]);
        let outer_native_results = std::mem::take(&mut self.pending_owned_native_results);
        let outer_for_of_iterables = std::mem::take(&mut self.for_of_iterable_cleanups);
        let outer_conditionally_owned_vars =
            std::mem::replace(&mut self.conditionally_owned_vars, vec![HashMap::default()]);
        let outer_conditionally_owned_results =
            std::mem::take(&mut self.pending_conditionally_owned_results);
        let outer_mutable_string_ownership =
            std::mem::replace(&mut self.mutable_string_ownership, vec![HashMap::default()]);
        let outer_string_array_ownership =
            std::mem::replace(&mut self.string_array_ownership, vec![HashMap::default()]);
        let outer_async_results = std::mem::take(&mut self.async_results);
        let outer_deferred_scopes = std::mem::replace(&mut self.deferred_scopes, vec![Vec::new()]);
        let outer_try_state = self.take_enclosing_try_state();
        self.return_types.push(ret_type.clone());
        self.push_throw_frame(Vec::new());
        self.builder.position_at_end(entry);

        let result: Result<(), String> = (|| {
            let env_param = fn_val.get_nth_param(0).unwrap().into_pointer_value();
            if is_escaping {
                for (index, (name, _, ty)) in captures.iter().enumerate() {
                    let field = self
                        .builder
                        .build_struct_gep(environment_type, env_param, index as u32 + 2, name)
                        .unwrap();
                    let slot = self.builder.build_alloca(self.llvm_type(ty), name).unwrap();
                    let value = self
                        .builder
                        .build_load(self.llvm_type(ty), field, name)
                        .unwrap();
                    self.builder.build_store(slot, value).unwrap();
                    self.store_var(name, slot, ty.clone());
                }
            } else {
                for (index, (name, _, ty)) in captures.iter().enumerate() {
                    let field = self
                        .builder
                        .build_struct_gep(environment_type, env_param, index as u32, name)
                        .unwrap();
                    let outer_slot = self
                        .builder
                        .build_load(pointer, field, name)
                        .unwrap()
                        .into_pointer_value();
                    self.store_var(name, outer_slot, ty.clone());
                }
            }
            for (index, param) in params.iter().enumerate() {
                let storage_type = if param.optional && param.default_value.is_none() && !param.rest
                {
                    Type::Optional(Box::new(param.ty.clone()))
                } else {
                    param.ty.clone()
                };
                let param_idx = index as u32 + 1;
                match &param.pattern {
                    ParamPattern::Name(name) => {
                        let llvm_ty = self.llvm_type(&storage_type);
                        let alloca = self.builder.build_alloca(llvm_ty, name).unwrap();
                        let value = fn_val.get_nth_param(param_idx).unwrap();
                        self.builder.build_store(alloca, value).unwrap();
                        self.store_var(name, alloca, storage_type);
                    }
                    ParamPattern::Array(names) => {
                        let llvm_ty = self.llvm_type(&storage_type);
                        let alloca = self
                            .builder
                            .build_alloca(llvm_ty, "arrow_param_array")
                            .unwrap();
                        let value = fn_val.get_nth_param(param_idx).unwrap();
                        self.builder.build_store(alloca, value).unwrap();

                        let array_val = self
                            .builder
                            .build_load(llvm_ty, alloca, "arrow_param_array_loaded")
                            .map_err(|e| e.to_string())?
                            .into_struct_value();

                        let data = self
                            .builder
                            .build_extract_value(array_val, 0, "arrow_param_data")
                            .unwrap()
                            .into_pointer_value();
                        let length = self
                            .builder
                            .build_extract_value(array_val, 1, "arrow_param_len")
                            .unwrap()
                            .into_int_value();

                        let Type::Array(element) = &storage_type else {
                            return Err(
                                "error: arrow fn array destructure param must have array type"
                                    .to_string(),
                            );
                        };
                        let resolved_element = self.resolve_type(element);
                        let element_llvm = self.llvm_type(&resolved_element);
                        let i64_type = self.context.i64_type();
                        for (j, binding_name) in names.iter().enumerate() {
                            let idx = i64_type.const_int(j as u64, false);
                            self.emit_array_bounds_check(idx, length)?;
                            let slot = unsafe {
                                self.builder
                                    .build_gep(element_llvm, data, &[idx], binding_name)
                                    .unwrap()
                            };
                            let val = self
                                .builder
                                .build_load(element_llvm, slot, binding_name)
                                .map_err(|e| e.to_string())?;
                            let binding_alloca = self
                                .builder
                                .build_alloca(element_llvm, binding_name)
                                .unwrap();
                            self.builder.build_store(binding_alloca, val).unwrap();
                            self.store_var(binding_name, binding_alloca, resolved_element.clone());
                        }
                    }
                    ParamPattern::Object(names) => {
                        let llvm_ty = self.llvm_type(&storage_type);
                        let alloca = self
                            .builder
                            .build_alloca(llvm_ty, "arrow_param_obj")
                            .unwrap();
                        let value = fn_val.get_nth_param(param_idx).unwrap();
                        self.builder.build_store(alloca, value).unwrap();

                        let struct_val = self
                            .builder
                            .build_load(llvm_ty, alloca, "arrow_param_obj_loaded")
                            .map_err(|e| e.to_string())?
                            .into_struct_value();

                        let (interface, type_name) = match &storage_type {
                            Type::Named(name) => (
                                self.interfaces
                                    .get(name.as_str())
                                    .cloned()
                                    .unwrap_or_default(),
                                name.clone(),
                            ),
                            Type::Object(fields) => (fields.clone(), "{ ... }".to_string()),
                            _ => {
                                return Err(
                                    "error: arrow fn object destructure param must have a named or inline object type"
                                        .to_string(),
                                );
                            }
                        };
                        for binding_name in names {
                            let index = interface
                                .iter()
                                .position(|c| c.name == *binding_name)
                                .ok_or_else(|| {
                                    format!(
                                        "error: type `{type_name}` has no field `{binding_name}`"
                                    )
                                })?;
                            let field_val = self
                                .builder
                                .build_extract_value(struct_val, index as u32, binding_name)
                                .map_err(|e| e.to_string())?;
                            let field_ty = field_storage_type(&interface[index]);
                            let resolved_ty = self.resolve_type(&field_ty);
                            let llvm_field = self.llvm_type(&resolved_ty);
                            let binding_alloca =
                                self.builder.build_alloca(llvm_field, binding_name).unwrap();
                            self.builder.build_store(binding_alloca, field_val).unwrap();
                            self.store_var(binding_name, binding_alloca, resolved_ty);
                        }
                    }
                }
            }

            self.push_scope();

            for stmt in body {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(stmt, fn_val)?;
            }

            if !self.current_block_terminated() {
                self.emit_deferred_finalizers_in_current_scope(ScopeExitReason::Normal)?;
                self.emit_owned_closure_cleanups(None)?;
                if matches!(ret_type, Type::Void) {
                    self.builder.build_return(None).unwrap();
                } else {
                    self.builder.build_unreachable().unwrap();
                }
            }
            Ok(())
        })();

        self.deferred_scopes = outer_deferred_scopes;
        self.restore_enclosing_try_state(outer_try_state);
        self.vars = outer_vars;
        self.owned_closure_vars = outer_owned_closures;
        self.owned_closure_drop_fns = outer_owned_closure_drops;
        self.owned_native_vars = outer_owned_native;
        self.pending_owned_native_results = outer_native_results;
        self.for_of_iterable_cleanups = outer_for_of_iterables;
        self.conditionally_owned_vars = outer_conditionally_owned_vars;
        self.pending_conditionally_owned_results = outer_conditionally_owned_results;
        self.mutable_string_ownership = outer_mutable_string_ownership;
        self.string_array_ownership = outer_string_array_ownership;
        self.async_results = outer_async_results;
        self.return_types.pop();
        self.pop_throw_frame();
        if let Some(block) = outer_block {
            self.builder.position_at_end(block);
        }
        result?;

        let closure_type = self
            .llvm_type(&Type::Function {
                params: param_types,
                ret: Box::new(ret_type.clone()),
            })
            .into_struct_type();
        let mut closure = closure_type.get_undef();
        closure = self
            .builder
            .build_insert_value(
                closure,
                fn_val.as_global_value().as_pointer_value(),
                0,
                "closure_function",
            )
            .unwrap()
            .into_struct_value();
        closure = self
            .builder
            .build_insert_value(closure, environment, 1, "closure.env")
            .unwrap()
            .into_struct_value();
        let _ = env_drop;
        Ok(closure.into())
    }

    /// Predicate shared by the clone into an escaping environment and the drop
    /// that releases it. They must agree exactly: cloning a field the drop does
    /// not release leaks it, and releasing a field the clone did not own frees
    /// borrowed memory, so both read this one function.
    pub(super) fn escaping_capture_is_owned(&self, ty: &Type) -> bool {
        !ty.is_copy() && self.async_owned_value_supported(ty)
    }

    pub(super) fn capture_is_closure(&self, ty: &Type) -> bool {
        matches!(
            transparent_codegen_type(&self.resolve_type(ty)),
            Type::Function { .. }
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the arguments mirror contextual arrow metadata and borrowed slices avoid cloning"
    )]
    pub(crate) fn emit_contextual_arrow_fn(
        &mut self,
        params: &[Param],
        declared_return: &Type,
        body: &[Stmt],
        expected_params: &[Type],
        expected_return: &Type,
        is_async: bool,
        is_escaping: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let params = params
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                let mut parameter = parameter.clone();
                let param_name = parameter
                    .pattern
                    .binding_names()
                    .first()
                    .copied()
                    .unwrap_or("param")
                    .to_string();
                if matches!(parameter.ty, Type::Infer) {
                    parameter.ty = expected_params.get(index).cloned().ok_or_else(|| {
                        format!("error: cannot infer type of callback parameter `{param_name}`",)
                    })?;
                }
                Ok(parameter)
            })
            .collect::<Result<Vec<_>, String>>()?;
        let return_type = if matches!(declared_return, Type::Infer) {
            expected_return.clone()
        } else {
            declared_return.clone()
        };
        if matches!(return_type, Type::Infer) {
            return Err(
                "error: cannot infer callback return type during code generation".to_string(),
            );
        }
        if is_async {
            self.emit_async_arrow_fn(&params, &return_type, body)
        } else {
            self.emit_arrow_fn(&params, &return_type, body, is_escaping)
        }
    }

    pub(crate) fn emit_closure_call(
        &mut self,
        closure: BasicValueEnum<'ctx>,
        params: &[Type],
        ret: &Type,
        args: &[BasicValueEnum<'ctx>],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let closure = closure.into_struct_value();
        let function = self
            .builder
            .build_extract_value(closure, 0, "callback_function")
            .unwrap()
            .into_pointer_value();
        let environment = self
            .builder
            .build_extract_value(closure, 1, "callback_environment")
            .unwrap();
        let mut call_args = vec![environment.into()];
        call_args.extend(args.iter().copied().map(BasicMetadataValueEnum::from));
        let call = self
            .builder
            .build_indirect_call(
                self.llvm_closure_fn_type(params, ret),
                function,
                &call_args,
                "callback_call",
            )
            .unwrap();
        Ok(call.try_as_basic_value().basic())
    }

    pub(crate) fn release_synchronous_closure(
        &self,
        closure: BasicValueEnum<'ctx>,
        owned: bool,
    ) -> Result<(), String> {
        if !owned {
            return Ok(());
        }
        let environment = self
            .builder
            .build_extract_value(
                closure.into_struct_value(),
                1,
                "callback_environment_release",
            )
            .unwrap()
            .into_pointer_value();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let free = self.declare_libc_fn("free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[environment.into()], "")
            .unwrap();
        Ok(())
    }
}
