use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(crate) fn emit_result_onfail(
        &mut self,
        _mutable: bool,
        name: &str,
        ty: &Type,
        value: &Expr,
        onfail: &OnFail,
        fn_val: FunctionValue<'ctx>,
        owned_awaited_result: bool,
    ) -> Result<(), String> {
        let source_ty = self
            .expr_type_hint(value)
            .map(|ty| self.resolve_type(&ty))
            .ok_or("internal: cannot determine the Result type used by `onfail`")?;
        let Type::Result { success, error } = source_ty.clone() else {
            return Err("internal: non-Result `onfail` reached code generation".to_string());
        };
        let success_ty = if matches!(ty, Type::Infer) {
            success.as_ref().clone()
        } else {
            self.resolve_binding_type(ty, Some(value))?
        };
        let variants = vec![success.as_ref().clone(), error.as_ref().clone()];
        let result = self.emit_expr_as(value, &source_ty)?;
        let result_release = self
            .take_owned_native_result(result)
            .or_else(|| owned_awaited_result.then_some(ReleasePlan::Deep));
        let result_slot =
            self.native_value_slot(self.llvm_type(&source_ty), result, "onfail.result")?;
        let tag = self.load_union_tag(&variants, result_slot)?;
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: `onfail` outside a function")?;
        let success_block = self.context.append_basic_block(function, "onfail.success");
        let error_block = self.context.append_basic_block(function, "onfail.error");
        let is_success = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                tag,
                self.context.i8_type().const_zero(),
                "onfail.is_success",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(is_success, success_block, error_block)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(error_block);
        self.push_scope();
        let error_value =
            self.load_union_payload(&variants, result_slot, error.as_ref(), "onfail.error.value")?;
        let error_slot = self.entry_alloca(self.llvm_type(error.as_ref()), name)?;
        self.builder.build_store(error_slot, error_value).unwrap();
        self.store_var(name, error_slot, error.as_ref().clone());
        if self.async_owned_value_supported(error.as_ref())
            && let Some(plan) = result_release.clone()
        {
            self.mark_owned_synchronous_value_with(name, plan);
        }
        match &onfail.handler {
            OnFailHandler::Expr(handler) => {
                self.emit_expr_for_side_effects(handler, fn_val)?;
                self.discard_owned_native_result();
            }
            OnFailHandler::Block(handler) => {
                self.push_scope();
                for statement in handler {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, fn_val)?;
                }
                self.pop_scope();
            }
        }
        if !self.current_block_terminated() {
            let returns_void = self
                .return_types
                .last()
                .is_some_and(|ty| matches!(self.resolve_type(ty), Type::Void));
            let implicit_error_return = if returns_void {
                Stmt::synthetic(StmtKind::Return(None))
            } else {
                Stmt::synthetic(StmtKind::Return(Some(Expr::synthetic(ExprKind::FnCall {
                    callee: "Err".to_string(),
                    type_args: Vec::new(),
                    args: vec![Expr::synthetic(ExprKind::Ident(name.to_string()))],
                }))))
            };
            self.emit_stmt_kind(&implicit_error_return, fn_val)?;
        }
        self.pop_scope();

        self.builder.position_at_end(success_block);
        if matches!(success_ty, Type::Void) {
            return Ok(());
        }
        let success_value = self.load_union_payload(
            &variants,
            result_slot,
            success.as_ref(),
            "onfail.success.value",
        )?;
        if let Some((frame_slot, _)) = self
            .async_frame_slot(name)
            .filter(|(_, frame_ty)| self.resolve_type(frame_ty) == success_ty)
        {
            self.clear_async_frame_local_before_store(fn_val, name, frame_slot, &success_ty)?;
            if self.async_frame_owns_value(name) {
                if result_release.is_some() {
                    // The source was an owned temporary: its success moves in.
                    self.builder.build_store(frame_slot, success_value).unwrap();
                } else {
                    // The awaited `Result` stays owned by its hidden frame
                    // local, which releases it with the frame, so the binding
                    // gets a copy.
                    self.emit_async_clone_value_into(frame_slot, success_value, &success_ty)?;
                }
                self.mark_async_frame_value_initialized(name, true);
            } else {
                self.builder.build_store(frame_slot, success_value).unwrap();
            }
            self.store_var(name, frame_slot, success_ty);
            return Ok(());
        }
        let success_slot = self.entry_alloca(self.llvm_type(&success_ty), name)?;
        self.builder
            .build_store(success_slot, success_value)
            .unwrap();
        self.store_var(name, success_slot, success_ty);
        if self.async_owned_value_supported(success.as_ref())
            && let Some(plan) = result_release
        {
            self.mark_owned_synchronous_value_with(name, plan);
        }
        Ok(())
    }

    pub(super) fn emit_throwing_onfail(
        &mut self,
        name: &str,
        ty: &Type,
        value: &Expr,
        onfail: &OnFail,
        exception: Type,
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        if !self.async_results.is_empty() {
            return Err("error: `onfail` for async exceptions is not lowered yet".to_string());
        }
        let binding_ty = self.resolve_binding_type(ty, Some(value))?;
        let binding_slot = self.entry_alloca(self.llvm_type(&binding_ty), name)?;
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let ctx_ty = self.catch_ctx_type();
        let error_ty = self.llvm_type(&exception);
        let error_slot = self
            .builder
            .build_alloca(error_ty, "onfail.throw.error")
            .unwrap();
        let ctx_slot = self
            .builder
            .build_alloca(ctx_ty, "onfail.throw.context")
            .unwrap();
        let owned_slot = self
            .builder
            .build_alloca(self.context.bool_type(), "onfail.throw.error.owned")
            .unwrap();
        self.builder
            .build_store(owned_slot, self.context.bool_type().const_zero())
            .unwrap();
        let catch_top = self.get_or_declare_catch_top().as_pointer_value();
        let previous = self
            .builder
            .build_load(ptr_ty, catch_top, "onfail.throw.previous")
            .unwrap();
        let previous_field = self
            .builder
            .build_struct_gep(ctx_ty, ctx_slot, 2, "onfail.throw.previous.field")
            .unwrap();
        self.builder.build_store(previous_field, previous).unwrap();
        let error_field = self
            .builder
            .build_struct_gep(ctx_ty, ctx_slot, 1, "onfail.throw.error.field")
            .unwrap();
        self.builder.build_store(error_field, error_slot).unwrap();
        let owned_field = self
            .builder
            .build_struct_gep(ctx_ty, ctx_slot, 3, "onfail.throw.owned.field")
            .unwrap();
        self.builder.build_store(owned_field, owned_slot).unwrap();
        self.builder.build_store(catch_top, ctx_slot).unwrap();
        let jump_buffer = self
            .builder
            .build_struct_gep(ctx_ty, ctx_slot, 0, "onfail.throw.jmp")
            .unwrap();
        let resumed = self
            .builder
            .build_call(
                self.declare_setjmp(),
                &[jump_buffer.into()],
                "onfail.throw.setjmp",
            )
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        let caught = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                resumed,
                self.context.i32_type().const_zero(),
                "onfail.throw.caught",
            )
            .unwrap();
        let success_block = self
            .context
            .append_basic_block(fn_val, "onfail.throw.success");
        let error_block = self
            .context
            .append_basic_block(fn_val, "onfail.throw.error");
        let after_block = self
            .context
            .append_basic_block(fn_val, "onfail.throw.after");
        self.builder
            .build_conditional_branch(caught, error_block, success_block)
            .unwrap();

        self.builder.position_at_end(success_block);
        self.push_throw_catch_target(&exception);
        let success = self.emit_expr_as(value, &binding_ty);
        self.pop_throw_catch_target();
        let success = success?;
        self.builder.build_store(catch_top, previous).unwrap();
        self.builder.build_store(binding_slot, success).unwrap();
        self.builder
            .build_unconditional_branch(after_block)
            .unwrap();

        self.builder.position_at_end(error_block);
        self.builder.build_store(catch_top, previous).unwrap();
        self.emit_receive_thrown_cell(ctx_slot, &exception, error_slot, &[], fn_val, None)?;
        self.push_scope();
        let error = self.builder.build_load(error_ty, error_slot, name).unwrap();
        let handler_slot = self.entry_alloca(error_ty, name)?;
        self.builder.build_store(handler_slot, error).unwrap();
        self.store_var(name, handler_slot, exception.clone());
        let tracks_owned_error = self.thrown_value_can_be_cloned(&exception);
        if tracks_owned_error {
            self.catch_error_owned_slots
                .push((name.to_string(), owned_slot));
        }
        match &onfail.handler {
            OnFailHandler::Expr(handler) => {
                self.emit_expr_for_side_effects(handler, fn_val)?;
                self.discard_owned_native_result();
            }
            OnFailHandler::Block(handler) => {
                self.push_scope();
                for statement in handler {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, fn_val)?;
                }
                self.pop_scope();
            }
        }
        if !self.current_block_terminated() {
            let implicit_exit = if matches!(self.return_types.last(), Some(Type::Void)) {
                if tracks_owned_error {
                    self.emit_conditional_catch_binding_release(
                        fn_val,
                        owned_slot,
                        handler_slot,
                        &exception,
                    )?;
                }
                Stmt::synthetic(StmtKind::Return(None))
            } else {
                Stmt::synthetic(StmtKind::Throw(Expr::synthetic(ExprKind::Ident(
                    name.to_string(),
                ))))
            };
            self.emit_stmt_kind(&implicit_exit, fn_val)?;
        }
        if tracks_owned_error {
            self.catch_error_owned_slots.pop();
        }
        self.pop_scope();

        self.builder.position_at_end(after_block);
        self.store_var(name, binding_slot, binding_ty);
        Ok(())
    }

    pub(super) fn result_error_type(&self, name: &str) -> Option<Type> {
        let (_, ty) = self.load_var(name)?;
        match self.resolve_type(&ty) {
            Type::Result { error, .. } => Some(*error),
            _ => None,
        }
    }

    pub(crate) fn error_subtype_storage(
        &mut self,
        binding: &str,
        catch_type: &Type,
    ) -> Result<Option<ErrorSubtypeStorage<'ctx>>, String> {
        if !matches!(self.resolve_type(catch_type), Type::Named(name) if name == "Error") {
            return Ok(None);
        }
        let tag_slot = self.entry_alloca(self.context.i64_type(), "catch.error.tag")?;
        let mut slots = Vec::new();
        for name in self.kept_error_subtype_names() {
            let subtype = Type::Named(name.clone());
            let slot = self.entry_alloca(self.llvm_type(&subtype), "catch.error.subtype")?;
            slots.push((name, slot));
        }
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: catch storage outside a function")?;
        Ok(Some(ErrorSubtypeStorage {
            binding: binding.to_string(),
            tag_slot,
            slots,
            scope_depth: self.vars.len(),
            function,
        }))
    }

    pub(crate) fn kept_error_subtype_names(&self) -> Vec<String> {
        let mut names = self
            .interfaces
            .keys()
            .filter(|name| self.interface_extends_error(name))
            .cloned()
            .collect::<Vec<_>>();
        names.sort();
        names.retain(|name| {
            let subtype = Type::Named(name.clone());
            self.struct_field_index(&subtype, "message").is_some()
                && self.struct_field_index(&subtype, "code").is_some()
        });
        names
    }

    pub(crate) fn kept_error_tag_local(storage_name: &str) -> String {
        format!("{storage_name}.error.tag")
    }

    pub(crate) fn kept_error_subtype_local(storage_name: &str, subtype: &str) -> String {
        format!("{storage_name}.error.{subtype}")
    }

    pub(crate) fn frame_error_subtype_storage(
        &self,
        storage_name: &str,
        binding: &str,
    ) -> Option<ErrorSubtypeStorage<'ctx>> {
        let (tag_slot, _) = self.async_frame_slot(&Self::kept_error_tag_local(storage_name))?;
        let slots = self
            .kept_error_subtype_names()
            .into_iter()
            .filter_map(|name| {
                let (slot, _) =
                    self.async_frame_slot(&Self::kept_error_subtype_local(storage_name, &name))?;
                Some((name, slot))
            })
            .collect();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)?;
        Some(ErrorSubtypeStorage {
            binding: binding.to_string(),
            tag_slot,
            slots,
            scope_depth: self.vars.len(),
            function,
        })
    }

    pub(super) fn kept_error_subtypes(&self, name: &str) -> Option<ErrorSubtypeStorage<'ctx>> {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)?;
        self.catch_error_subtypes
            .iter()
            .rev()
            .find(|storage| storage.binding == name)
            .filter(|storage| storage.function == function)
            .cloned()
    }

    pub(crate) fn emit_error_subtype_storage_release(
        &mut self,
        fn_val: FunctionValue<'ctx>,
        storage: &ErrorSubtypeStorage<'ctx>,
        owned_slot: Option<PointerValue<'ctx>>,
    ) -> Result<(), String> {
        if storage.slots.is_empty() || self.current_block_terminated() {
            return Ok(());
        }
        let done = self
            .context
            .append_basic_block(fn_val, "catch.error.subtype.release.done");
        if let Some(owned_slot) = owned_slot {
            let owned = self
                .builder
                .build_load(
                    self.context.bool_type(),
                    owned_slot,
                    "catch.error.subtype.owned",
                )
                .unwrap()
                .into_int_value();
            let release = self
                .context
                .append_basic_block(fn_val, "catch.error.subtype.release");
            self.builder
                .build_conditional_branch(owned, release, done)
                .unwrap();
            self.builder.position_at_end(release);
        }
        let tag = self
            .builder
            .build_load(
                self.context.i64_type(),
                storage.tag_slot,
                "catch.error.subtype.kept",
            )
            .unwrap()
            .into_int_value();
        for (name, slot) in &storage.slots {
            let subtype = Type::Named(name.clone());
            let matched = self
                .context
                .append_basic_block(fn_val, "catch.error.subtype.drop");
            let next = self
                .context
                .append_basic_block(fn_val, "catch.error.subtype.drop.next");
            let is_subtype = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    tag,
                    self.context
                        .i64_type()
                        .const_int(self.thrown_type_tag(&subtype), false),
                    "catch.error.subtype.is",
                )
                .unwrap();
            self.builder
                .build_conditional_branch(is_subtype, matched, next)
                .unwrap();
            self.builder.position_at_end(matched);
            if owned_slot.is_some() {
                self.emit_error_subtype_extra_fields_release(fn_val, &subtype, *slot)?;
            } else {
                self.emit_async_drop_value_in_place(fn_val, *slot, &subtype)?;
            }
            if !self.current_block_terminated() {
                self.builder.build_unconditional_branch(next).unwrap();
            }
            self.builder.position_at_end(next);
        }
        self.builder.build_unconditional_branch(done).unwrap();
        self.builder.position_at_end(done);
        Ok(())
    }

    pub(super) fn emit_error_subtype_extra_fields_release(
        &mut self,
        fn_val: FunctionValue<'ctx>,
        subtype: &Type,
        value: PointerValue<'ctx>,
    ) -> Result<(), String> {
        let Type::Named(name) = subtype else {
            return Ok(());
        };
        let Some(fields) = self.interfaces.get(name).cloned() else {
            return Ok(());
        };
        let struct_type = self.llvm_type(subtype).into_struct_type();
        for field in fields {
            if field.name == "message" || field.name == "code" || field.ty.is_copy() {
                continue;
            }
            let Some(index) = self.struct_field_index(subtype, &field.name) else {
                continue;
            };
            let pointer = self
                .builder
                .build_struct_gep(struct_type, value, index, "catch.error.subtype.field")
                .unwrap();
            self.emit_async_drop_value_in_place(fn_val, pointer, &field.ty)?;
        }
        Ok(())
    }

    pub(crate) fn emit_error_subtype_is(
        &mut self,
        name: &str,
        target: &Type,
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let binds_error = matches!(
            self.load_var(name).map(|(_, ty)| self.resolve_type(&ty)),
            Some(Type::Named(bound)) if bound == "Error"
        );
        if !binds_error {
            return Ok(None);
        }
        let resolved = self.resolve_type(target);
        if matches!(&resolved, Type::Named(target) if target == "Error") {
            return Ok(Some(self.context.bool_type().const_int(1, false).into()));
        }
        let Some(storage) = self.kept_error_subtypes(name) else {
            return Err(format!(
                "error: `{name} is {resolved}` needs the value `catch ({name})` keeps in this function; catch the union of the error types instead"
            ));
        };
        let tag = self
            .builder
            .build_load(self.context.i64_type(), storage.tag_slot, "is.error.tag")
            .unwrap()
            .into_int_value();
        let matches = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                tag,
                self.context
                    .i64_type()
                    .const_int(self.thrown_type_tag(&resolved), false),
                "is.error.subtype",
            )
            .unwrap();
        Ok(Some(matches.into()))
    }

    pub(super) fn apply_error_subtype_narrowing(
        &mut self,
        name: &str,
        target: &Type,
        holds: bool,
    ) -> bool {
        if !holds {
            return false;
        }
        let binds_error = matches!(
            self.load_var(name).map(|(_, ty)| self.resolve_type(&ty)),
            Some(Type::Named(bound)) if bound == "Error"
        );
        if !binds_error {
            return false;
        }
        let Some(storage) = self.kept_error_subtypes(name) else {
            return false;
        };
        let resolved = self.resolve_type(target);
        let Type::Named(target_name) = &resolved else {
            return false;
        };
        let Some((_, slot)) = storage.slots.iter().find(|(kept, _)| kept == target_name) else {
            return false;
        };
        self.store_var(name, *slot, resolved.clone());
        true
    }

    pub(crate) fn catch_ctx_type(&self) -> StructType<'ctx> {
        let jbuf_ty = self.context.i64_type().array_type(25);
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        self.context.struct_type(
            &[
                jbuf_ty.into(),
                ptr_ty.into(),
                ptr_ty.into(),
                ptr_ty.into(),
                self.context.i64_type().into(),
                ptr_ty.into(),
            ],
            false,
        )
    }

    pub(crate) fn get_or_declare_catch_top(&self) -> GlobalValue<'ctx> {
        if let Some(g) = self.module.get_global("tyrus.catch.top") {
            return g;
        }
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let g = self.module.add_global(ptr_ty, None, "tyrus.catch.top");
        g.set_initializer(&ptr_ty.const_null());
        g.set_thread_local(true);
        g
    }

    pub(crate) fn emit_throw(
        &mut self,
        expr: &Expr,
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let thrown_ty = self.thrown_expr_type(expr);
        let mut value =
            if matches!(expr.kind(), ExprKind::ObjectLit(_)) && !matches!(thrown_ty, Type::Str) {
                self.emit_expr_as(expr, &thrown_ty)?
            } else {
                self.emit_expr(expr)?
            };
        self.forget_owned_native_result(value);
        let thrown = Self::thrown_value_kind(&thrown_ty);
        if let Some(completion) = self
            .async_finally_completion_scopes
            .last()
            .filter(|completion| completion.intercept_throw)
            .cloned()
        {
            let message = match thrown {
                ThrownValue::Str => value,
                ThrownValue::ErrorMessage => {
                    self.builder
                        .build_extract_value(value.into_struct_value(), 0, "async.finally.throw.message")
                        .map_err(|error| {
                            format!("error: failed to preserve the message thrown through a suspending `finally`: {error}")
                        })?
                }
                _ => {
                    return Err("error: this thrown value cannot be preserved through a suspending `finally` yet".to_string());
                }
            };
            let preserved =
                self.emit_string_clone(message.into_struct_value(), "async.finally.throw.clone")?;
            self.builder
                .build_store(completion.throw_slot, preserved)
                .unwrap();
            self.mark_async_frame_value_initialized(&completion.throw_storage, true);
            self.emit_active_async_error_cleanups(None);
            self.builder
                .build_store(
                    completion.mode_slot,
                    self.context.i32_type().const_int(5, false),
                )
                .unwrap();
            self.emit_active_async_catch_cleanup(fn_val)?;
            self.emit_pending_validation_cleanups_for_throw(expr);
            let thrown_type = self
                .expr_type_hint(expr)
                .map_or(Type::Str, |ty| self.resolve_type(&ty));
            self.emit_async_cleanup_source_expression(value, &thrown_type, expr)?;
            self.emit_catch_binding_releases_on_exit(None)?;
            self.emit_owned_value_cleanups(None)?;
            self.builder
                .build_unconditional_branch(completion.throw_entry.unwrap_or(completion.entry))
                .unwrap();
            return Ok(());
        }
        self.transfer_active_async_catch_binding(expr);
        self.emit_pending_finally(fn_val, true, None)?;
        if self.current_block_terminated() {
            return Ok(());
        }
        self.emit_pending_validation_cleanups_for_throw(expr);
        let mut thrown_local = match expr.kind() {
            ExprKind::Ident(name) => Some(name.as_str()),
            _ => None,
        };
        let kept_error_subtype = if self.async_results.is_empty()
            && matches!(self.resolve_type(&thrown_ty), Type::Named(ref name) if name == "Error")
        {
            thrown_local.and_then(|name| self.kept_error_subtypes(name))
        } else {
            None
        };
        let mut owned = self.emit_thrown_value_owned_bit(thrown, expr);
        let borrowed_named_error = matches!(expr.kind(), ExprKind::Ident(name)
            if !self.match_result_bindings.iter().rev().any(|bindings| bindings.contains(name)))
            && matches!(transparent_codegen_type(&self.resolve_type(&thrown_ty)), Type::Named(name)
            if self.interfaces.get(name).is_some_and(|fields| {
                fields.iter().any(|field| field.name == "code" && matches!(field.ty, Type::Str))
                    && fields.iter().any(|field| field.name == "message" && matches!(field.ty, Type::Str))
            }))
            && !owned
                .get_zero_extended_constant()
                .is_some_and(|value| value != 0);
        let is_caught_binding = matches!(expr.kind(), ExprKind::Ident(name) if self
            .catch_error_owned_slots
            .iter()
            .rev()
            .any(|(binding, _)| binding == name));
        if !is_caught_binding
            && !owned
                .get_zero_extended_constant()
                .is_some_and(|value| value != 0)
            && !borrowed_named_error
            && !thrown_ty.is_copy()
            && self.async_owned_value_supported(&thrown_ty)
            && self.thrown_value_can_be_cloned(&thrown_ty)
            && kept_error_subtype.is_none()
        {
            let slot = self.entry_alloca(self.llvm_type(&thrown_ty), "throw.value.clone")?;
            self.emit_async_clone_value_into(slot, value, &thrown_ty)?;
            value = self
                .builder
                .build_load(self.llvm_type(&thrown_ty), slot, "throw.value.clone.loaded")
                .map_err(|error| error.to_string())?;
            owned = self.context.bool_type().const_all_ones();
            thrown_local = None;
        }
        self.emit_active_async_error_cleanups(thrown_local);
        self.emit_owned_value_cleanups_to_depth(self.enclosing_catch_scope_depth(), thrown_local)?;
        if let Some(storage) = kept_error_subtype {
            self.emit_rethrow_kept_error_subtype(&storage, fn_val, owned)?;
        }
        if let Type::Union(variants) = &thrown_ty
            && !Self::union_is_payload_free(variants)
        {
            return self.emit_throw_union_variants(value, variants, fn_val, owned);
        }
        let async_throw = self.async_throw_rejection(&thrown_ty, value, thrown)?;
        self.emit_throw_value_in_async(value, thrown, fn_val, async_throw, owned, &thrown_ty)
    }

    pub(crate) fn thrown_value_can_be_cloned(&self, ty: &Type) -> bool {
        let resolved = self.resolve_type(ty);
        let ty = transparent_codegen_type(&resolved);
        match ty {
            Type::Str => true,
            Type::Named(name) if name == "Error" => true,
            Type::Named(name) if tyrus_core::native_handles::is_opaque_handle_name(name) => false,
            Type::Named(_) | Type::Object(_) => {
                self.async_object_fields(ty).is_some_and(|fields| {
                    fields
                        .iter()
                        .all(|field| self.thrown_value_can_be_cloned(&field_storage_type(field)))
                })
            }
            Type::Result { success, error } => {
                self.thrown_value_can_be_cloned(success) && self.thrown_value_can_be_cloned(error)
            }
            Type::Union(variants) | Type::Tuple(variants) => variants
                .iter()
                .all(|variant| self.thrown_value_can_be_cloned(variant)),
            Type::Array(element) | Type::Optional(element) => {
                self.thrown_value_can_be_cloned(element)
            }
            _ => ty.is_copy(),
        }
    }

    pub(crate) fn emit_conditional_catch_binding_release(
        &mut self,
        fn_val: FunctionValue<'ctx>,
        owned_slot: PointerValue<'ctx>,
        error_slot: PointerValue<'ctx>,
        catch_type: &Type,
    ) -> Result<(), String> {
        if let Type::Union(variants) = transparent_codegen_type(catch_type)
            && tyrus_core::typeof_category::is_unknown_runtime_union(transparent_codegen_type(
                catch_type,
            ))
        {
            let variants = variants.clone();
            return self
                .emit_conditional_catch_unknown_release(fn_val, owned_slot, error_slot, &variants);
        }
        if !self.thrown_value_can_be_cloned(catch_type) {
            return Ok(());
        }
        let owned = self
            .builder
            .build_load(self.context.bool_type(), owned_slot, "catch.value.owned")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let release = self
            .context
            .append_basic_block(fn_val, "catch.value.release");
        let done = self.context.append_basic_block(fn_val, "catch.value.done");
        self.builder
            .build_conditional_branch(owned, release, done)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(release);
        let value = self
            .builder
            .build_load(self.llvm_type(catch_type), error_slot, "catch.value.load")
            .map_err(|error| error.to_string())?;
        self.emit_owned_native_value_free(value, catch_type);
        if !self.current_block_terminated() {
            self.builder
                .build_unconditional_branch(done)
                .map_err(|error| error.to_string())?;
        }
        self.builder.position_at_end(done);
        Ok(())
    }

    pub(crate) fn emit_active_async_error_cleanups(&mut self, exclude: Option<&str>) {
        let cleanups = self
            .async_error_cleanups
            .iter()
            .rev()
            .filter(|(binding, _)| exclude != Some(binding.as_str()))
            .map(|(_, cleanup)| cleanup.clone())
            .collect::<Vec<_>>();
        for cleanup in cleanups {
            self.emit_async_error_cleanup(&cleanup);
        }
    }

    pub(super) fn emit_conditional_catch_unknown_release(
        &mut self,
        fn_val: FunctionValue<'ctx>,
        owned_slot: PointerValue<'ctx>,
        error_slot: PointerValue<'ctx>,
        variants: &[Type],
    ) -> Result<(), String> {
        let Some(string_index) = variants
            .iter()
            .position(|variant| matches!(variant, Type::Str))
        else {
            return Ok(());
        };
        let owned = self
            .builder
            .build_load(self.context.bool_type(), owned_slot, "catch.unknown.owned")
            .map_err(|error| format!("catch unknown owned load: {error}"))?
            .into_int_value();
        let check = self
            .context
            .append_basic_block(fn_val, "catch.unknown.check");
        let release = self
            .context
            .append_basic_block(fn_val, "catch.unknown.free");
        let done = self
            .context
            .append_basic_block(fn_val, "catch.unknown.done");
        self.builder
            .build_conditional_branch(owned, check, done)
            .map_err(|error| format!("catch unknown branch: {error}"))?;

        self.builder.position_at_end(check);
        let tag = self.load_union_tag(variants, error_slot)?;
        let is_string = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                tag,
                self.context.i8_type().const_int(string_index as u64, false),
                "catch.unknown.is.string",
            )
            .map_err(|error| format!("catch unknown tag compare: {error}"))?;
        self.builder
            .build_conditional_branch(is_string, release, done)
            .map_err(|error| format!("catch unknown branch: {error}"))?;

        self.builder.position_at_end(release);
        let value = self
            .load_union_payload(variants, error_slot, &Type::Str, "catch.unknown.value")?
            .into_struct_value();
        self.emit_string_release(value)?;
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| format!("catch unknown branch: {error}"))?;

        self.builder.position_at_end(done);
        Ok(())
    }

    pub(crate) fn emit_catch_binding_releases_on_exit(
        &mut self,
        transferred: Option<&str>,
    ) -> Result<(), String> {
        let Some(function) = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
        else {
            return Ok(());
        };
        let releases = self
            .catch_binding_releases
            .iter()
            .rev()
            .filter(|release| {
                release.function == function && transferred != Some(release.name.as_str())
            })
            .cloned()
            .collect::<Vec<_>>();
        for release in releases {
            self.emit_conditional_catch_binding_release(
                function,
                release.owned_slot,
                release.error_slot,
                &release.catch_type,
            )?;
        }
        Ok(())
    }

    pub(crate) fn push_catch_binding_release(
        &mut self,
        function: FunctionValue<'ctx>,
        name: &str,
        owned_slot: PointerValue<'ctx>,
        error_slot: PointerValue<'ctx>,
        catch_type: &Type,
    ) {
        self.catch_binding_releases.push(CatchBindingRelease {
            function,
            name: name.to_string(),
            owned_slot,
            error_slot,
            catch_type: catch_type.clone(),
        });
    }

    pub(crate) fn pop_catch_binding_release(&mut self) {
        self.catch_binding_releases.pop();
    }

    pub(super) fn returned_catch_binding(&self, value: &Expr) -> Option<CatchBindingRelease<'ctx>> {
        let ExprKind::Ident(name) = value.kind() else {
            return None;
        };
        let (slot, _) = self.load_var(name)?;
        self.catch_binding_releases
            .iter()
            .rev()
            .find(|release| release.name == *name && release.error_slot == slot)
            .cloned()
    }

    pub(crate) fn thrown_value_kind(ty: &Type) -> ThrownValue {
        match ty {
            Type::Str => ThrownValue::Str,
            Type::Named(name) if name == "ValidationError" => ThrownValue::Validation,
            Type::Named(name) if name == "Error" => ThrownValue::ErrorMessage,
            Type::Named(name) => {
                native_coded_error(name).map_or(ThrownValue::Other, ThrownValue::Coded)
            }
            _ => ThrownValue::Other,
        }
    }

    pub(crate) fn interface_extends_error(&self, name: &str) -> bool {
        name != "Error"
            && (tyrus_core::builtin_interfaces::native_error_extends_error(name)
                || self.error_subtypes.contains(name))
    }

    pub(crate) fn native_error_code_prefix(name: &str) -> Option<String> {
        if !tyrus_core::builtin_interfaces::native_error_extends_error(name) {
            return None;
        }
        let module = name.strip_suffix("Error")?;
        Some(format!("ERR_{}_", module.to_ascii_uppercase()))
    }

    pub(crate) fn thrown_type_tag(&self, ty: &Type) -> u64 {
        thrown_type_tag_of(&self.resolve_type(ty).to_string())
    }

    pub(crate) fn push_throw_frame(&mut self, throws: Vec<Type>) {
        self.throw_frames.push(ThrowFrame {
            throws,
            catches: Vec::new(),
        });
    }

    pub(crate) fn pop_throw_frame(&mut self) {
        self.throw_frames.pop();
    }

    pub(crate) fn push_throw_catch_target(&mut self, ty: &Type) {
        if let Some(frame) = self.throw_frames.last_mut() {
            frame.catches.push(ty.clone());
        }
    }

    pub(crate) fn pop_throw_catch_target(&mut self) {
        if let Some(frame) = self.throw_frames.last_mut() {
            frame.catches.pop();
        }
    }

    pub(super) fn throw_target(&self) -> Option<Type> {
        let frame = self.throw_frames.last()?;
        if let Some(target) = frame.catches.last() {
            return Some(self.resolve_type(target));
        }
        match frame.throws.as_slice() {
            [] => None,
            [only] => Some(self.resolve_type(only)),
            many => Some(Type::Union(
                many.iter().map(|ty| self.resolve_type(ty)).collect(),
            )),
        }
    }

    pub(super) fn thrown_expr_type(&self, expr: &Expr) -> Type {
        let hint = self.expr_type_hint(expr).map(|ty| self.resolve_type(&ty));
        let object_literal = matches!(expr.kind(), ExprKind::ObjectLit(_));
        if hint.is_none()
            && let Some(target) = self.object_literal_throw_target(expr)
        {
            return target;
        }
        match (hint, self.throw_target()) {
            (Some(hint), _) => hint,
            (None, Some(Type::Union(variants))) if object_literal => variants
                .iter()
                .find(|variant| matches!(variant, Type::Named(_) | Type::Object(_)))
                .cloned()
                .unwrap_or(Type::Str),
            (None, Some(target)) if object_literal => target,
            (None, _) => Type::Str,
        }
    }

    pub(super) fn object_literal_throw_target(&self, expr: &Expr) -> Option<Type> {
        let ExprKind::ObjectLit(fields) = expr.kind() else {
            return None;
        };
        let mut names = fields
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>();
        names.sort_unstable();
        let frame = self.throw_frames.last()?;
        for candidate in frame.catches.iter().rev().chain(frame.throws.iter()) {
            let resolved = self.resolve_type(candidate);
            let variants = match &resolved {
                Type::Union(variants) => variants.clone(),
                other => vec![other.clone()],
            };
            for variant in variants {
                let Type::Named(name) = &variant else {
                    continue;
                };
                let Some(declared) = self.interfaces.get(name) else {
                    continue;
                };
                let mut declared = declared
                    .iter()
                    .map(|field| field.name.as_str())
                    .collect::<Vec<_>>();
                declared.sort_unstable();
                if declared == names {
                    return Some(variant);
                }
            }
        }
        None
    }

    pub(super) fn emit_rethrow_kept_error_subtype(
        &mut self,
        storage: &ErrorSubtypeStorage<'ctx>,
        fn_val: FunctionValue<'ctx>,
        owned: IntValue<'ctx>,
    ) -> Result<(), String> {
        let tag = self
            .builder
            .build_load(
                self.context.i64_type(),
                storage.tag_slot,
                "rethrow.kept.tag",
            )
            .unwrap()
            .into_int_value();
        for (name, slot) in &storage.slots {
            let subtype = Type::Named(name.clone());
            let matched = self
                .context
                .append_basic_block(fn_val, "rethrow.kept.subtype");
            let next = self
                .context
                .append_basic_block(fn_val, "rethrow.kept.subtype.next");
            let is_subtype = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    tag,
                    self.context
                        .i64_type()
                        .const_int(self.thrown_type_tag(&subtype), false),
                    "rethrow.kept.is_subtype",
                )
                .unwrap();
            self.builder
                .build_conditional_branch(is_subtype, matched, next)
                .unwrap();
            self.builder.position_at_end(matched);
            let value = self
                .builder
                .build_load(self.llvm_type(&subtype), *slot, "rethrow.kept.value")
                .unwrap();
            self.emit_throw_value_in_async(
                value,
                ThrownValue::Other,
                fn_val,
                None,
                owned,
                &subtype,
            )?;
            self.builder.position_at_end(next);
        }
        Ok(())
    }

    pub(super) fn emit_throw_union_variants(
        &mut self,
        value: BasicValueEnum<'ctx>,
        variants: &[Type],
        fn_val: FunctionValue<'ctx>,
        owned: IntValue<'ctx>,
    ) -> Result<(), String> {
        let union_ty = self.union_llvm_type(variants);
        let slot = self.builder.build_alloca(union_ty, "throw.union").unwrap();
        self.builder.build_store(slot, value).unwrap();
        let tag = self.load_union_tag(variants, slot)?;
        for (index, variant) in variants.iter().enumerate() {
            let matched = self
                .context
                .append_basic_block(fn_val, "throw.union.variant");
            let next = self.context.append_basic_block(fn_val, "throw.union.next");
            let is_variant = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    tag,
                    self.context.i8_type().const_int(index as u64, false),
                    "throw.union.is",
                )
                .unwrap();
            self.builder
                .build_conditional_branch(is_variant, matched, next)
                .unwrap();
            self.builder.position_at_end(matched);
            let payload =
                self.load_union_payload(variants, slot, variant, "throw.union.payload")?;
            let thrown = Self::thrown_value_kind(variant);
            let async_throw = self.async_throw_rejection(variant, payload, thrown)?;
            self.emit_throw_value_in_async(payload, thrown, fn_val, async_throw, owned, variant)?;
            self.builder.position_at_end(next);
        }
        self.builder.build_unreachable().unwrap();
        Ok(())
    }

    pub(super) fn emit_thrown_value_owned_bit(
        &self,
        thrown: ThrownValue,
        expr: &Expr,
    ) -> IntValue<'ctx> {
        let bool_ty = self.context.bool_type();
        if let ExprKind::Ident(name) = expr.kind() {
            if self
                .async_catch_cleanup_scopes
                .iter()
                .rev()
                .any(|scope| scope.binding == *name && scope.transferred)
            {
                return bool_ty.const_int(1, false);
            }
            if let Some((_, slot)) = self
                .catch_error_owned_slots
                .iter()
                .rev()
                .find(|(bound, _)| bound == name)
            {
                return self
                    .builder
                    .build_load(bool_ty, *slot, "throw.rethrow.owned")
                    .unwrap()
                    .into_int_value();
            }
        }
        let owned = if matches!(thrown, ThrownValue::Str) {
            self.thrown_value_is_owned(expr)
        } else if let ExprKind::Ident(name) = expr.kind() {
            self.owned_native_vars
                .iter()
                .rev()
                .find_map(|scope| scope.get(name))
                .is_some()
        } else {
            false
        };
        bool_ty.const_int(u64::from(owned), false)
    }

    /// Whether `expr`, thrown as a `string`, is provably an allocation the
    /// catch handler may free, as opposed to a literal, a parameter, or
    /// anything else this pass cannot vouch for.
    ///
    /// Two shapes qualify: a fresh allocation at the throw site itself
    /// (`throw "a".toUpperCase()`), and a local bound from one earlier in this
    /// scope (`throw held`), found by checking the same `owned_native_vars`
    /// bookkeeping that decides whether this scope's own cleanup would have
    /// freed it. Anything else defaults to false, which only costs a leak.
    pub(super) fn thrown_value_is_owned(&self, expr: &Expr) -> bool {
        if self.async_owned_string_expression(expr) {
            return true;
        }
        let ExprKind::Ident(name) = expr.kind() else {
            return false;
        };
        self.owned_native_vars
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .is_some_and(|plan| *plan == ReleasePlan::Deep)
    }

    pub(crate) fn async_throw_rejection(
        &mut self,
        travels_as: &Type,
        value: BasicValueEnum<'ctx>,
        thrown: ThrownValue,
    ) -> Result<Option<(PointerValue<'ctx>, AsyncRejection<'ctx>)>, String> {
        let Some(async_result) = self.async_results.last().copied() else {
            return Ok(None);
        };
        let thrown_type = Some(self.resolve_type(travels_as));
        let rejection = match thrown {
            ThrownValue::Str => AsyncRejection::Message(value),
            ThrownValue::ErrorMessage => AsyncRejection::Structured {
                code_index: 1,
                message_index: 0,
            },
            ThrownValue::Other | ThrownValue::Coded(_)
                if thrown_type
                    .as_ref()
                    .is_some_and(|ty| self.is_structured_error_binding(ty)) =>
            {
                AsyncRejection::Structured {
                    code_index: 0,
                    message_index: 1,
                }
            }
            ThrownValue::Other | ThrownValue::Coded(_) if matches!(&thrown_type, Some(Type::Named(name)) if self.interfaces.contains_key(name)) =>
            {
                let thrown_type = thrown_type.clone().expect("matched a named type");
                let message_index =
                    self.struct_field_index(&thrown_type, "message")
                        .filter(|index| {
                            matches!(&thrown_type, Type::Named(name) if self
                            .interfaces
                            .get(name)
                            .and_then(|fields| fields.get(*index as usize))
                            .is_some_and(|field| matches!(field.ty, Type::Str)))
                        });
                let message = match message_index {
                    Some(index) => self
                        .builder
                        .build_extract_value(
                            value.into_struct_value(),
                            index,
                            "async.throw.message",
                        )
                        .map_err(|error| error.to_string())?,
                    None => self.const_string_abi("a thrown value", "async.throw.message.default"),
                };
                AsyncRejection::Message(message)
            }
            _ => return Ok(None),
        };
        Ok(Some((async_result, rejection)))
    }

    pub(crate) fn emit_throw_value(
        &mut self,
        value: BasicValueEnum<'ctx>,
        thrown: ThrownValue,
        fn_val: FunctionValue<'ctx>,
        thrown_type: &Type,
    ) -> Result<(), String> {
        self.emit_throw_value_in_async(
            value,
            thrown,
            fn_val,
            None,
            self.context.bool_type().const_zero(),
            thrown_type,
        )
    }

    pub(crate) fn emit_throw_value_in_async(
        &mut self,
        value: BasicValueEnum<'ctx>,
        thrown: ThrownValue,
        fn_val: FunctionValue<'ctx>,
        async_throw: Option<(PointerValue<'ctx>, AsyncRejection<'ctx>)>,
        owned: IntValue<'ctx>,
        thrown_type: &Type,
    ) -> Result<(), String> {
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let catch_top_global = self.get_or_declare_catch_top();

        let catch_top = self
            .builder
            .build_load(ptr_ty, catch_top_global.as_pointer_value(), "throw.ctx")
            .unwrap()
            .into_pointer_value();

        let is_null = self
            .builder
            .build_is_null(catch_top, "throw.is_null")
            .unwrap();

        let do_throw_bb = self.context.append_basic_block(fn_val, "throw.longjmp");
        let uncaught_bb = self.context.append_basic_block(fn_val, "throw.uncaught");
        self.builder
            .build_conditional_branch(is_null, uncaught_bb, do_throw_bb)
            .unwrap();

        self.builder.position_at_end(do_throw_bb);
        let value_size = value
            .get_type()
            .size_of()
            .ok_or("internal: a thrown value has no size")?;
        let (malloc, size) = self.declare_malloc(value_size);
        let cell = self
            .builder
            .build_call(malloc, &[size.into()], "throw.cell")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .ok_or("internal: malloc returned no value")?
            .into_pointer_value();
        self.builder.build_store(cell, value).unwrap();
        let tag = self
            .context
            .i64_type()
            .const_int(self.thrown_type_tag(thrown_type), false);
        self.emit_jump_to_catch(catch_top, tag, cell, owned);

        self.builder.position_at_end(uncaught_bb);
        if let Some((async_result, rejection)) = async_throw {
            self.emit_attach_thrown_cell(async_result, value, thrown_type)?;
            self.emit_async_reject_call(async_result, rejection, value)?;
            let release = self
                .context
                .append_basic_block(fn_val, "async.throw.release");
            let completed = self
                .context
                .append_basic_block(fn_val, "async.throw.completed");
            self.builder
                .build_conditional_branch(owned, release, completed)
                .unwrap();
            self.builder.position_at_end(release);
            if matches!(thrown, ThrownValue::Str) {
                self.emit_string_release(value.into_struct_value())?;
            } else if matches!(thrown, ThrownValue::Validation) {
                let free = self.declare_libc_fn("free", &[ptr_ty.into()], None);
                self.builder.build_call(free, &[value.into()], "").unwrap();
            } else {
                let value_slot =
                    self.entry_alloca(self.llvm_type(thrown_type), "async.throw.owned.value")?;
                self.builder.build_store(value_slot, value).unwrap();
                self.emit_async_drop_value_in_place(fn_val, value_slot, thrown_type)?;
            }
            self.builder.build_unconditional_branch(completed).unwrap();
            self.builder.position_at_end(completed);
            self.builder
                .build_return(Some(&i32_ty.const_int(1, false)))
                .unwrap();
            return Ok(());
        }
        let dprintf = self.declare_dprintf();
        let msg: BasicValueEnum<'ctx> = match thrown {
            ThrownValue::Str if self.is_string_value(value) => value,
            ThrownValue::Str => self.const_string_abi("<error>", "throw.generic"),
            ThrownValue::Validation => {
                let error = value.into_struct_value();
                let message = self
                    .builder
                    .build_extract_value(error, 1, "throw.validation.message")
                    .unwrap();
                let issues = self
                    .builder
                    .build_extract_value(error, 2, "throw.validation.issues")
                    .unwrap()
                    .into_struct_value();
                let data = self
                    .builder
                    .build_extract_value(issues, 0, "throw.validation.issues.data")
                    .unwrap();
                let count = self
                    .builder
                    .build_extract_value(issues, 1, "throw.validation.issues.len")
                    .unwrap();
                let count = self.truncate_to_usize(count.into_int_value());
                let describe = self.declare_libc_fn(
                    "tyrus_validation_error_describe",
                    &[
                        self.string_abi_type(),
                        ptr_ty.into(),
                        self.usize_type().into(),
                    ],
                    Some(self.string_abi_type()),
                );
                self.builder
                    .build_call(
                        describe,
                        &[message.into(), data.into(), count.into()],
                        "throw.validation.describe",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| {
                        "internal: tyrus_validation_error_describe did not return a value"
                            .to_string()
                    })?
            }
            ThrownValue::Other => self.const_string_abi("<error>", "throw.generic"),
            ThrownValue::ErrorMessage => self
                .builder
                .build_extract_value(value.into_struct_value(), 0, "throw.error.message")
                .map_err(|error| error.to_string())?,
            ThrownValue::Coded(_) => self
                .builder
                .build_extract_value(value.into_struct_value(), 1, "throw.coded.message")
                .map_err(|error| error.to_string())?,
        };
        let bounded =
            |text: BasicValueEnum<'ctx>| -> Result<[BasicMetadataValueEnum<'ctx>; 2], String> {
                let text = text.into_struct_value();
                let length = self.string_byte_len(text)?;
                let precision = self
                    .builder
                    .build_int_truncate(length, i32_ty, "throw.print.precision")
                    .map_err(|error| error.to_string())?;
                Ok([precision.into(), self.string_data_ptr(text)?.into()])
            };
        let format = match thrown {
            ThrownValue::Coded(name) => format!("uncaught exception: {name} (%.*s): %.*s\n"),
            ThrownValue::ErrorMessage => "uncaught exception: Error: %.*s\n".to_string(),
            _ => "uncaught exception: %.*s\n".to_string(),
        };
        let fmt_str = self
            .builder
            .build_global_string_ptr(&format, "throw.fmt")
            .unwrap();
        let mut args: Vec<BasicMetadataValueEnum<'ctx>> = vec![
            i32_ty.const_int(2, false).into(),
            fmt_str.as_pointer_value().into(),
        ];
        if let ThrownValue::Coded(_) = thrown {
            let code = self
                .builder
                .build_extract_value(value.into_struct_value(), 0, "throw.coded.code")
                .map_err(|error| error.to_string())?;
            args.extend(bounded(code)?);
        }
        args.extend(bounded(msg)?);
        self.builder.build_call(dprintf, &args, "").unwrap();
        let exit = self.declare_libc_fn("exit", &[i32_ty.into()], None);
        self.builder
            .build_call(exit, &[i32_ty.const_int(1, false).into()], "")
            .unwrap();
        self.builder.build_unreachable().unwrap();

        let _ = fn_val;
        Ok(())
    }

    pub(super) fn emit_validation_error_cleanup_slot(&self, error_slot: PointerValue<'ctx>) {
        let error_type = self
            .llvm_type(&Type::Named("ValidationError".to_string()))
            .into_struct_type();
        let error = self
            .builder
            .build_load(error_type, error_slot, "validation.cleanup.error")
            .unwrap()
            .into_struct_value();
        let issues = self
            .builder
            .build_extract_value(error, 2, "validation.cleanup.issues")
            .unwrap()
            .into_struct_value();
        let data = self
            .builder
            .build_extract_value(issues, 0, "validation.cleanup.data")
            .unwrap()
            .into_pointer_value();
        let count = self
            .builder
            .build_extract_value(issues, 1, "validation.cleanup.count")
            .unwrap()
            .into_int_value();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let free = self.declare_libc_fn(
            "tyrus_validation_issues_free",
            &[ptr_type.into(), self.usize_type().into()],
            None,
        );
        self.builder
            .build_call(free, &[data.into(), count.into()], "")
            .unwrap();
    }

    pub(super) fn emit_pending_validation_cleanups_for_throw(&self, expr: &Expr) {
        let transferred = match expr.kind() {
            ExprKind::Ident(name) => self.load_var(name).map(|(slot, _)| slot),
            _ => None,
        };
        for scope in self.validation_cleanup_scopes.iter().rev() {
            if scope.transferred {
                continue;
            }
            if transferred.is_some_and(|slot| slot == scope.error_slot) {
                continue;
            }
            self.emit_validation_error_cleanup_slot(scope.error_slot);
        }
    }

    pub(crate) fn emit_attach_thrown_cell(
        &mut self,
        promise: PointerValue<'ctx>,
        value: BasicValueEnum<'ctx>,
        thrown_type: &Type,
    ) -> Result<(), String> {
        let resolved = self.resolve_type(thrown_type);
        let carries_shape = match &resolved {
            Type::Str => true,
            Type::Named(name) => name != "Error" && self.interfaces.contains_key(name),
            Type::Union(variants) => !Self::union_is_payload_free(variants),
            _ => false,
        };
        if !carries_shape {
            return Ok(());
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let size = value
            .get_type()
            .size_of()
            .ok_or("internal: a thrown value has no size")?;
        let (malloc, size) = self.declare_malloc(size);
        let cell = self
            .builder
            .build_call(malloc, &[size.into()], "async.throw.cell")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .ok_or("internal: malloc returned no value")?
            .into_pointer_value();
        self.emit_async_clone_value_into(cell, value, &resolved)?;
        let drop = self.emit_async_object_payload_drop(&resolved)?;
        let attach = self.declare_libc_fn(
            "tyrus_promise_attach_thrown",
            &[
                pointer.into(),
                self.context.i64_type().into(),
                pointer.into(),
                pointer.into(),
            ],
            None,
        );
        let tag = self
            .context
            .i64_type()
            .const_int(self.thrown_type_tag(&resolved), false);
        self.builder
            .build_call(
                attach,
                &[
                    promise.into(),
                    tag.into(),
                    cell.into(),
                    drop.as_global_value().as_pointer_value().into(),
                ],
                "",
            )
            .unwrap();
        Ok(())
    }

    pub(super) fn emit_jump_to_catch(
        &self,
        catch_top: PointerValue<'ctx>,
        tag: IntValue<'ctx>,
        cell: PointerValue<'ctx>,
        owned: IntValue<'ctx>,
    ) {
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let ctx_ty = self.catch_ctx_type();
        let tag_field = self
            .builder
            .build_struct_gep(ctx_ty, catch_top, 4, "throw.tag_field")
            .unwrap();
        self.builder.build_store(tag_field, tag).unwrap();
        let cell_field = self
            .builder
            .build_struct_gep(ctx_ty, catch_top, 5, "throw.cell_field")
            .unwrap();
        self.builder.build_store(cell_field, cell).unwrap();
        let owned_ptr_field = self
            .builder
            .build_struct_gep(ctx_ty, catch_top, 3, "throw.owned_ptr_field")
            .unwrap();
        let owned_ptr = self
            .builder
            .build_load(ptr_ty, owned_ptr_field, "throw.owned_ptr")
            .unwrap()
            .into_pointer_value();
        self.builder.build_store(owned_ptr, owned).unwrap();
        let jbuf = self
            .builder
            .build_struct_gep(ctx_ty, catch_top, 0, "throw.jbuf")
            .unwrap();
        let longjmp = self.declare_libc_fn("longjmp", &[ptr_ty.into(), i32_ty.into()], None);
        self.builder
            .build_call(
                longjmp,
                &[jbuf.into(), i32_ty.const_int(1, false).into()],
                "",
            )
            .unwrap();
        self.builder.build_unreachable().unwrap();
    }

    /// Installs a landing context around a call that may throw, when the
    /// scopes between the call and the `catch` that will receive the throw own
    /// something. The callee delivers a throw with `longjmp` straight to
    /// `tyrus.catch.top`, which skips the releases a `throw` written at the call
    /// site would emit: the landing receives it first, then forwards it exactly
    /// as such a `throw` would. Outside a `try` of this function the landing is
    /// installed only when some caller has a `catch`: with none, the thrower
    /// reports the uncaught value itself and the process exits, so there is
    /// nothing worth releasing. Returns the previous context, which
    /// `end_throwing_call_landing` restores after the call.
    pub(crate) fn begin_throwing_call_landing(
        &mut self,
    ) -> Result<Option<PointerValue<'ctx>>, String> {
        let inside_try = !self.catch_context_restores.is_empty();
        if !self.async_results.is_empty()
            || (!inside_try && self.current_emitting_function_is_main())
            || !self.has_owned_value_cleanups_to_depth(self.enclosing_catch_scope_depth())
        {
            return Ok(None);
        }
        let fn_val = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: a call landing needs an enclosing function")?;
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let ctx_ty = self.catch_ctx_type();
        let ctx_slot = self.entry_alloca(ctx_ty, "call.landing.ctx")?;
        let owned_slot = self.entry_alloca(self.context.bool_type(), "call.landing.owned")?;
        self.builder
            .build_store(owned_slot, self.context.bool_type().const_zero())
            .unwrap();
        let catch_top_ptr = self.get_or_declare_catch_top().as_pointer_value();
        let prev = self
            .builder
            .build_load(ptr_ty, catch_top_ptr, "call.landing.prev")
            .unwrap()
            .into_pointer_value();
        let prev_field = self
            .builder
            .build_struct_gep(ctx_ty, ctx_slot, 2, "call.landing.prev_field")
            .unwrap();
        self.builder.build_store(prev_field, prev).unwrap();
        let owned_ptr_field = self
            .builder
            .build_struct_gep(ctx_ty, ctx_slot, 3, "call.landing.owned_ptr")
            .unwrap();
        self.builder
            .build_store(owned_ptr_field, owned_slot)
            .unwrap();
        let installed = if inside_try {
            ctx_slot
        } else {
            let no_catch = self
                .builder
                .build_is_null(prev, "call.landing.no_catch")
                .unwrap();
            self.builder
                .build_select(no_catch, ptr_ty.const_null(), ctx_slot, "call.landing.top")
                .unwrap()
                .into_pointer_value()
        };
        self.builder.build_store(catch_top_ptr, installed).unwrap();
        let jbuf = self
            .builder
            .build_struct_gep(ctx_ty, ctx_slot, 0, "call.landing.jbuf")
            .unwrap();
        let setjmp = self.declare_setjmp();
        let returned = self
            .builder
            .build_call(setjmp, &[jbuf.into()], "call.landing.setjmp")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        let caught = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                returned,
                i32_ty.const_zero(),
                "call.landing.caught",
            )
            .unwrap();
        let landing = self.context.append_basic_block(fn_val, "call.landing");
        let call = self.context.append_basic_block(fn_val, "call.landing.call");
        self.builder
            .build_conditional_branch(caught, landing, call)
            .unwrap();

        self.builder.position_at_end(landing);
        self.builder.build_store(catch_top_ptr, prev).unwrap();
        let tag_field = self
            .builder
            .build_struct_gep(ctx_ty, ctx_slot, 4, "call.landing.tag_field")
            .unwrap();
        let tag = self
            .builder
            .build_load(self.context.i64_type(), tag_field, "call.landing.tag")
            .unwrap()
            .into_int_value();
        let cell_field = self
            .builder
            .build_struct_gep(ctx_ty, ctx_slot, 5, "call.landing.cell_field")
            .unwrap();
        let cell = self
            .builder
            .build_load(ptr_ty, cell_field, "call.landing.cell")
            .unwrap()
            .into_pointer_value();
        let owned = self
            .builder
            .build_load(
                self.context.bool_type(),
                owned_slot,
                "call.landing.is_owned",
            )
            .unwrap()
            .into_int_value();
        self.emit_forward_thrown_cell(tag, cell, owned, fn_val)?;
        if !self.current_block_terminated() {
            self.builder.build_unreachable().unwrap();
        }

        self.builder.position_at_end(call);
        Ok(Some(prev))
    }

    pub(crate) fn end_throwing_call_landing(&mut self, prev: Option<PointerValue<'ctx>>) {
        if let Some(prev) = prev {
            let catch_top_ptr = self.get_or_declare_catch_top().as_pointer_value();
            self.builder.build_store(catch_top_ptr, prev).unwrap();
        }
    }

    pub(crate) fn emit_receive_thrown_cell(
        &mut self,
        context_slot: PointerValue<'ctx>,
        catch_type: &Type,
        error_slot: PointerValue<'ctx>,
        forward_finally: &[Stmt],
        fn_val: FunctionValue<'ctx>,
        subtype_storage: Option<&ErrorSubtypeStorage<'ctx>>,
    ) -> Result<(), String> {
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let ctx_ty = self.catch_ctx_type();
        let tag_field = self
            .builder
            .build_struct_gep(ctx_ty, context_slot, 4, "catch.tag_field")
            .unwrap();
        let tag = self
            .builder
            .build_load(i64_ty, tag_field, "catch.tag")
            .unwrap()
            .into_int_value();
        if let Some(storage) = subtype_storage {
            self.builder.build_store(storage.tag_slot, tag).unwrap();
        }
        let cell_field = self
            .builder
            .build_struct_gep(ctx_ty, context_slot, 5, "catch.cell_field")
            .unwrap();
        let cell = self
            .builder
            .build_load(ptr_ty, cell_field, "catch.cell")
            .unwrap()
            .into_pointer_value();
        let accepted = self.context.append_basic_block(fn_val, "catch.accepted");
        let forward = self.context.append_basic_block(fn_val, "catch.forward");
        let resolved = self.resolve_type(catch_type);
        match &resolved {
            Type::Union(variants) if !Self::union_is_payload_free(variants) => {
                for (index, variant) in variants.iter().enumerate() {
                    let matched = self.context.append_basic_block(fn_val, "catch.variant");
                    let next = self
                        .context
                        .append_basic_block(fn_val, "catch.variant.next");
                    let is_variant = self
                        .builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            tag,
                            i64_ty.const_int(self.thrown_type_tag(variant), false),
                            "catch.is_variant",
                        )
                        .unwrap();
                    self.builder
                        .build_conditional_branch(is_variant, matched, next)
                        .unwrap();
                    self.builder.position_at_end(matched);
                    let payload = self
                        .builder
                        .build_load(self.llvm_type(variant), cell, "catch.variant.value")
                        .unwrap();
                    self.store_union_variant(variants, error_slot, index as u8, payload)?;
                    self.builder.build_unconditional_branch(accepted).unwrap();
                    self.builder.position_at_end(next);
                }
                self.builder.build_unconditional_branch(forward).unwrap();
            }
            _ => {
                if matches!(&resolved, Type::Named(name) if name == "Error") {
                    let mut subtypes = self
                        .interfaces
                        .keys()
                        .filter(|name| self.interface_extends_error(name))
                        .cloned()
                        .collect::<Vec<_>>();
                    subtypes.sort();
                    for subtype in subtypes {
                        let subtype = Type::Named(subtype);
                        let (Some(message_index), Some(code_index)) = (
                            self.struct_field_index(&subtype, "message"),
                            self.struct_field_index(&subtype, "code"),
                        ) else {
                            continue;
                        };
                        let projected = self
                            .context
                            .append_basic_block(fn_val, "catch.error.subtype");
                        let next = self
                            .context
                            .append_basic_block(fn_val, "catch.error.subtype.next");
                        let is_subtype = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                tag,
                                i64_ty.const_int(self.thrown_type_tag(&subtype), false),
                                "catch.is_error_subtype",
                            )
                            .unwrap();
                        self.builder
                            .build_conditional_branch(is_subtype, projected, next)
                            .unwrap();
                        self.builder.position_at_end(projected);
                        let value = self
                            .builder
                            .build_load(self.llvm_type(&subtype), cell, "catch.error.subtype.value")
                            .unwrap()
                            .into_struct_value();
                        if let (Some(storage), Type::Named(subtype_name)) =
                            (subtype_storage, &subtype)
                            && let Some((_, slot)) =
                                storage.slots.iter().find(|(name, _)| name == subtype_name)
                        {
                            self.builder.build_store(*slot, value).unwrap();
                        }
                        let message = self
                            .builder
                            .build_extract_value(
                                value,
                                message_index,
                                "catch.error.subtype.message",
                            )
                            .unwrap();
                        let code = self
                            .builder
                            .build_extract_value(value, code_index, "catch.error.subtype.code")
                            .unwrap();
                        let error = self
                            .builder
                            .build_insert_value(
                                self.llvm_type(&resolved).into_struct_type().const_zero(),
                                message,
                                0,
                                "catch.error.projected.message",
                            )
                            .unwrap();
                        let error = self
                            .builder
                            .build_insert_value(error, code, 1, "catch.error.projected.code")
                            .unwrap();
                        self.builder.build_store(error_slot, error).unwrap();
                        if subtype_storage.is_none() {
                            let owned_ptr_field = self
                                .builder
                                .build_struct_gep(
                                    ctx_ty,
                                    context_slot,
                                    3,
                                    "catch.subtype.owned_ptr_field",
                                )
                                .unwrap();
                            let owned_ptr = self
                                .builder
                                .build_load(ptr_ty, owned_ptr_field, "catch.subtype.owned_ptr")
                                .unwrap()
                                .into_pointer_value();
                            let owned = self
                                .builder
                                .build_load(
                                    self.context.bool_type(),
                                    owned_ptr,
                                    "catch.subtype.owned",
                                )
                                .unwrap()
                                .into_int_value();
                            let release = self
                                .context
                                .append_basic_block(fn_val, "catch.subtype.release");
                            let released = self
                                .context
                                .append_basic_block(fn_val, "catch.subtype.released");
                            self.builder
                                .build_conditional_branch(owned, release, released)
                                .unwrap();
                            self.builder.position_at_end(release);
                            let whole = self
                                .builder
                                .build_alloca(self.llvm_type(&subtype), "catch.subtype.whole")
                                .unwrap();
                            self.builder.build_store(whole, value).unwrap();
                            self.emit_error_subtype_extra_fields_release(fn_val, &subtype, whole)?;
                            self.builder.build_unconditional_branch(released).unwrap();
                            self.builder.position_at_end(released);
                        }
                        self.builder.build_unconditional_branch(accepted).unwrap();
                        self.builder.position_at_end(next);
                    }
                }
                let matched = self.context.append_basic_block(fn_val, "catch.exact");
                let is_type = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        tag,
                        i64_ty.const_int(self.thrown_type_tag(&resolved), false),
                        "catch.is_type",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(is_type, matched, forward)
                    .unwrap();
                self.builder.position_at_end(matched);
                let value = self
                    .builder
                    .build_load(self.llvm_type(&resolved), cell, "catch.value")
                    .unwrap();
                self.builder.build_store(error_slot, value).unwrap();
                self.builder.build_unconditional_branch(accepted).unwrap();
            }
        }

        self.builder.position_at_end(forward);
        let owned_ptr_field = self
            .builder
            .build_struct_gep(ctx_ty, context_slot, 3, "catch.forward.owned_ptr_field")
            .unwrap();
        let owned_ptr = self
            .builder
            .build_load(ptr_ty, owned_ptr_field, "catch.forward.owned_ptr")
            .unwrap()
            .into_pointer_value();
        let owned = self
            .builder
            .build_load(self.context.bool_type(), owned_ptr, "catch.forward.owned")
            .unwrap()
            .into_int_value();
        self.push_scope();
        for statement in forward_finally {
            if self.current_block_terminated() {
                break;
            }
            self.emit_stmt(statement, fn_val)?;
        }
        self.pop_scope();
        if !self.current_block_terminated() {
            self.emit_forward_thrown_cell(tag, cell, owned, fn_val)?;
        }

        self.builder.position_at_end(accepted);
        let free = self.declare_libc_fn("free", &[ptr_ty.into()], None);
        self.builder.build_call(free, &[cell.into()], "").unwrap();
        Ok(())
    }

    pub(super) fn emit_forward_thrown_cell(
        &mut self,
        tag: IntValue<'ctx>,
        cell: PointerValue<'ctx>,
        owned: IntValue<'ctx>,
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        self.emit_pending_finally(fn_val, true, None)?;
        if self.current_block_terminated() {
            return Ok(());
        }
        self.emit_owned_value_cleanups_to_depth(self.enclosing_catch_scope_depth(), None)?;
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let catch_top = self
            .builder
            .build_load(
                ptr_ty,
                self.get_or_declare_catch_top().as_pointer_value(),
                "forward.ctx",
            )
            .unwrap()
            .into_pointer_value();
        let is_null = self
            .builder
            .build_is_null(catch_top, "forward.is_null")
            .unwrap();
        let jump = self.context.append_basic_block(fn_val, "forward.longjmp");
        let uncaught = self.context.append_basic_block(fn_val, "forward.uncaught");
        self.builder
            .build_conditional_branch(is_null, uncaught, jump)
            .unwrap();
        self.builder.position_at_end(jump);
        self.emit_jump_to_catch(catch_top, tag, cell, owned);
        self.builder.position_at_end(uncaught);
        const MESSAGE: &str = "a thrown value reached no `catch` that handles its type";
        if let Some(promise) = self.async_results.last().copied() {
            let declared = self
                .throw_frames
                .last()
                .map(|frame| frame.throws.clone())
                .unwrap_or_default();
            for declared_type in declared {
                let resolved = self.resolve_type(&declared_type);
                let thrown_kind = match &resolved {
                    Type::Str => ThrownValue::Str,
                    Type::Named(name) if name == "ValidationError" => continue,
                    _ => ThrownValue::Other,
                };
                let matched = self.context.append_basic_block(fn_val, "forward.declared");
                let next = self
                    .context
                    .append_basic_block(fn_val, "forward.declared.next");
                let is_declared = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        tag,
                        self.context
                            .i64_type()
                            .const_int(self.thrown_type_tag(&resolved), false),
                        "forward.is_declared",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(is_declared, matched, next)
                    .unwrap();
                self.builder.position_at_end(matched);
                let value = self
                    .builder
                    .build_load(self.llvm_type(&resolved), cell, "forward.declared.value")
                    .unwrap();
                match self.async_throw_rejection(&resolved, value, thrown_kind)? {
                    Some(rejection) => {
                        let free = self.declare_libc_fn("free", &[ptr_ty.into()], None);
                        self.builder.build_call(free, &[cell.into()], "").unwrap();
                        self.emit_throw_value_in_async(
                            value,
                            thrown_kind,
                            fn_val,
                            Some(rejection),
                            owned,
                            &resolved,
                        )?;
                    }
                    None => {
                        self.builder.build_unconditional_branch(next).unwrap();
                    }
                }
                self.builder.position_at_end(next);
            }
            let reject = self.declare_libc_fn(
                "tyrus_promise_reject_pending",
                &[ptr_ty.into(), self.string_abi_type()],
                Some(i32_ty.into()),
            );
            let message = self.const_string_abi(MESSAGE, "forward.uncaught.message");
            self.builder
                .build_call(reject, &[promise.into(), message.into()], "forward.reject")
                .unwrap();
            self.builder
                .build_return(Some(&i32_ty.const_int(1, false)))
                .unwrap();
        } else {
            let dprintf = self.declare_dprintf();
            let format = self
                .builder
                .build_global_string_ptr(&format!("uncaught exception: {MESSAGE}\n"), "forward.fmt")
                .unwrap();
            self.builder
                .build_call(
                    dprintf,
                    &[
                        i32_ty.const_int(2, false).into(),
                        format.as_pointer_value().into(),
                    ],
                    "",
                )
                .unwrap();
            let exit = self.declare_libc_fn("exit", &[i32_ty.into()], None);
            self.builder
                .build_call(exit, &[i32_ty.const_int(1, false).into()], "")
                .unwrap();
            self.builder.build_unreachable().unwrap();
        }
        Ok(())
    }

    pub(crate) fn emit_try_catch(
        &mut self,
        try_body: &[Stmt],
        catch_binding: &str,
        catch_type: &Type,
        catch_body: &[Stmt],
        finally_body: Option<&[Stmt]>,
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let ctx_ty = self.catch_ctx_type();
        let err_llvm_ty = self.llvm_type(catch_type);

        let err_slot = self.builder.build_alloca(err_llvm_ty, "try.err").unwrap();
        let ctx_slot = self.builder.build_alloca(ctx_ty, "try.ctx").unwrap();
        let owned_slot = self
            .builder
            .build_alloca(self.context.bool_type(), "try.err.owned")
            .unwrap();
        self.builder
            .build_store(owned_slot, self.context.bool_type().const_zero())
            .unwrap();

        let catch_top_global = self.get_or_declare_catch_top();
        let catch_top_ptr = catch_top_global.as_pointer_value();

        let prev = self
            .builder
            .build_load(ptr_ty, catch_top_ptr, "try.prev")
            .unwrap();
        let prev_field = self
            .builder
            .build_struct_gep(ctx_ty, ctx_slot, 2, "try.ctx.prev")
            .unwrap();
        self.builder.build_store(prev_field, prev).unwrap();

        let err_ptr_field = self
            .builder
            .build_struct_gep(ctx_ty, ctx_slot, 1, "try.ctx.err_ptr")
            .unwrap();
        self.builder.build_store(err_ptr_field, err_slot).unwrap();

        let owned_ptr_field = self
            .builder
            .build_struct_gep(ctx_ty, ctx_slot, 3, "try.ctx.owned_ptr")
            .unwrap();
        self.builder
            .build_store(owned_ptr_field, owned_slot)
            .unwrap();

        self.builder.build_store(catch_top_ptr, ctx_slot).unwrap();

        let jbuf = self
            .builder
            .build_struct_gep(ctx_ty, ctx_slot, 0, "try.jbuf")
            .unwrap();
        let setjmp = self.declare_setjmp();
        let setjmp_ret = self
            .builder
            .build_call(setjmp, &[jbuf.into()], "try.setjmp")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();

        let caught = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                setjmp_ret,
                i32_ty.const_zero(),
                "try.caught",
            )
            .unwrap();

        let try_body_bb = self.context.append_basic_block(fn_val, "try.body");
        let catch_body_bb = self.context.append_basic_block(fn_val, "try.catch.body");
        let after_bb = self.context.append_basic_block(fn_val, "try.after");
        let merge_bb = if finally_body.is_some() {
            self.context.append_basic_block(fn_val, "try.merge")
        } else {
            after_bb
        };

        self.builder
            .build_conditional_branch(caught, catch_body_bb, try_body_bb)
            .unwrap();

        self.builder.position_at_end(try_body_bb);
        self.finally_scopes.push(FinallyScope {
            body: finally_body.unwrap_or_default().to_vec(),
            catches_throw: false,
            loop_depth: self.loop_stack.len(),
        });
        self.push_scope();
        let catch_scope_depth = self.owned_native_vars.len().saturating_sub(1);
        self.push_throw_catch_target(catch_type);
        self.catch_context_restores.push((
            catch_top_ptr,
            prev.into_pointer_value(),
            catch_scope_depth,
        ));
        self.catch_binding_types.push(catch_type.clone());
        for stmt in try_body {
            if self.current_block_terminated() {
                break;
            }
            self.emit_stmt(stmt, fn_val)?;
        }
        self.catch_binding_types.pop();
        self.catch_context_restores.pop();
        self.pop_throw_catch_target();
        self.pop_scope();
        self.finally_scopes.pop();
        if !self.current_block_terminated() {
            self.builder.build_store(catch_top_ptr, prev).unwrap();
            self.builder.build_unconditional_branch(after_bb).unwrap();
        }

        self.builder.position_at_end(catch_body_bb);
        self.builder.build_store(catch_top_ptr, prev).unwrap();
        let subtype_storage = self.error_subtype_storage(catch_binding, catch_type)?;
        self.emit_receive_thrown_cell(
            ctx_slot,
            catch_type,
            err_slot,
            finally_body.unwrap_or_default(),
            fn_val,
            subtype_storage.as_ref(),
        )?;
        if let Some(body) = finally_body {
            self.finally_scopes.push(FinallyScope {
                body: body.to_vec(),
                catches_throw: true,
                loop_depth: self.loop_stack.len(),
            });
        }
        self.push_scope();
        let e_val = self
            .builder
            .build_load(err_llvm_ty, err_slot, catch_binding)
            .unwrap();
        let e_slot = self
            .builder
            .build_alloca(err_llvm_ty, catch_binding)
            .unwrap();
        self.builder.build_store(e_slot, e_val).unwrap();
        self.store_var(catch_binding, e_slot, catch_type.clone());
        let keeps_subtypes = subtype_storage.is_some();
        let kept_subtypes = subtype_storage.clone();
        if let Some(storage) = subtype_storage {
            self.catch_error_subtypes.push(storage);
        }
        let owns_validation_issues =
            matches!(catch_type, Type::Named(name) if name == "ValidationError");
        if owns_validation_issues {
            self.validation_cleanup_scopes.push(ValidationCleanupScope {
                error_slot: e_slot,
                loop_depth: self.loop_stack.len(),
                transferred: false,
            });
        }
        if let Some(completion) = self.async_finally_completion_scopes.last_mut() {
            completion.intercept_throw = true;
        }
        // Lets a `throw catch_binding;` inside this body forward the runtime
        // ownership bit this catch received. A catch binding is not a
        // scope-owned local, so rechecking its ownership from local metadata
        // would report false and lose ownership of the thrown payload.
        let tracks_owned_rethrow =
            !self.resolve_type(catch_type).is_copy() && self.thrown_value_can_be_cloned(catch_type);
        if tracks_owned_rethrow {
            self.catch_error_owned_slots
                .push((catch_binding.to_string(), owned_slot));
            self.catch_binding_releases.push(CatchBindingRelease {
                function: fn_val,
                name: catch_binding.to_string(),
                owned_slot,
                error_slot: e_slot,
                catch_type: catch_type.clone(),
            });
        }
        for stmt in catch_body {
            if self.current_block_terminated() {
                break;
            }
            let transfers_binding = match stmt.kind() {
                StmtKind::Return(Some(expression)) | StmtKind::Throw(expression) => {
                    matches!(expression.kind(), ExprKind::Ident(name) if name == catch_binding)
                }
                _ => false,
            };
            if !transfers_binding
                && matches!(
                    stmt.kind(),
                    StmtKind::Throw(_) | StmtKind::Break(_) | StmtKind::Continue
                )
            {
                self.emit_conditional_catch_binding_release(
                    fn_val, owned_slot, e_slot, catch_type,
                )?;
            }
            let rethrows_subtype = transfers_binding
                && matches!(stmt.kind(), StmtKind::Throw(_))
                && self.async_results.is_empty();
            if let Some(storage) = kept_subtypes.as_ref().filter(|_| {
                !rethrows_subtype
                    && matches!(
                        stmt.kind(),
                        StmtKind::Return(_)
                            | StmtKind::Throw(_)
                            | StmtKind::Break(_)
                            | StmtKind::Continue
                    )
            }) {
                self.emit_error_subtype_storage_release(fn_val, storage, Some(owned_slot))?;
            }
            self.emit_stmt(stmt, fn_val)?;
        }
        if tracks_owned_rethrow {
            self.catch_binding_releases.pop();
            self.catch_error_owned_slots.pop();
        }
        if keeps_subtypes {
            self.catch_error_subtypes.pop();
        }
        if let Some(completion) = self.async_finally_completion_scopes.last_mut() {
            completion.intercept_throw = false;
        }
        let validation_cleanup =
            owns_validation_issues.then(|| self.validation_cleanup_scopes.pop().unwrap());
        self.pop_scope();
        if finally_body.is_some() {
            self.finally_scopes.pop();
        }
        if !self.current_block_terminated() {
            if validation_cleanup.is_some_and(|scope| !scope.transferred) {
                self.emit_validation_error_cleanup_slot(e_slot);
            } else if tracks_owned_rethrow {
                self.emit_conditional_catch_binding_release(
                    fn_val, owned_slot, e_slot, catch_type,
                )?;
            }
            if let Some(storage) = kept_subtypes.as_ref() {
                self.emit_error_subtype_storage_release(fn_val, storage, Some(owned_slot))?;
            }
            self.builder.build_unconditional_branch(after_bb).unwrap();
        }

        self.builder.position_at_end(after_bb);
        if let Some(stmts) = finally_body {
            for stmt in stmts {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(stmt, fn_val)?;
            }
            if !self.current_block_terminated() {
                self.builder.build_unconditional_branch(merge_bb).unwrap();
            }
            self.builder.position_at_end(merge_bb);
        }

        Ok(())
    }
}
