use super::*;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_flow_exit_stmt(
        &mut self,
        stmt: &Stmt,
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        match stmt.kind() {
            StmtKind::Return(expr) => {
                // A compact `match` arm is a `return` of its expression, so an
                // `async` function returning `void` reaches here with a `print`
                // call: it has no value to settle the promise with.
                if let Some(value) = expr.as_ref().filter(|_| {
                    !self.async_results.is_empty()
                        && self
                            .return_types
                            .last()
                            .is_some_and(|ty| matches!(self.resolve_type(ty), Type::Void))
                }) {
                    self.emit_expr_for_side_effects(value, fn_val)?;
                    self.discard_owned_native_result();
                    if self.current_block_terminated() {
                        return Ok(());
                    }
                    return self.emit_flow_exit_stmt(
                        &Stmt::new(StmtKind::Return(None), stmt.span),
                        fn_val,
                    );
                }
                let transferred_local =
                    expr.as_ref()
                        .and_then(|expression| match expression.kind() {
                            ExprKind::Ident(name) => Some(name.as_str()),
                            ExprKind::FnCall { callee, args, .. }
                                if matches!(callee.as_str(), "Ok" | "Err") && args.len() == 1 =>
                            {
                                let ExprKind::Ident(name) = args[0].kind() else {
                                    return None;
                                };
                                Some(name.as_str())
                            }
                            _ => None,
                        });
                if let Some(completion) = self.async_finally_completion_scopes.last().cloned() {
                    match (expr.as_ref(), completion.return_slot.as_ref()) {
                        (Some(value), Some((slot, expected))) => {
                            if matches!(
                                self.expr_type_hint(value).map(|ty| self.resolve_type(&ty)),
                                Some(Type::Apply { name, args }) if name == "Promise" && args.len() == 1
                            ) {
                                return Err("error: direct Promise return through a suspending `finally` is not implemented yet".to_string());
                            }
                            let emitted = self.emit_expr_as(value, expected)?;
                            if self.async_owned_value_supported(expected) {
                                self.emit_async_clone_value_into(*slot, emitted, expected)?;
                                if let Some(storage) = completion.return_storage.as_deref() {
                                    self.mark_async_frame_value_initialized(storage, true);
                                }
                                self.emit_async_cleanup_source_expression(
                                    emitted, expected, value,
                                )?;
                            } else {
                                self.builder.build_store(*slot, emitted).unwrap();
                            }
                        }
                        (None, None) => {}
                        (None, Some((slot, expected)))
                            if matches!(transparent_codegen_type(expected), Type::Optional(_)) =>
                        {
                            self.builder
                                .build_store(*slot, self.llvm_type(expected).const_zero())
                                .unwrap();
                        }
                        _ => {
                            return Err(
                                "internal: async finally return completion has an incompatible payload"
                                    .to_string(),
                            );
                        }
                    }
                    self.builder
                        .build_store(
                            completion.mode_slot,
                            self.context.i32_type().const_int(2, false),
                        )
                        .unwrap();
                    self.emit_active_async_catch_cleanup(fn_val)?;
                    self.emit_pending_validation_cleanups(None);
                    self.emit_owned_value_cleanups(None)?;
                    self.emit_owned_closure_cleanups(transferred_local)?;
                    self.builder
                        .build_unconditional_branch(completion.entry)
                        .unwrap();
                    return Ok(());
                }
                if let (Some(result), Some(value)) =
                    (self.async_results.last().copied(), expr.as_ref())
                {
                    let expected = self
                        .return_types
                        .last()
                        .cloned()
                        .ok_or("internal: async return payload type is missing")?;
                    if matches!(
                        self.expr_type_hint(value).map(|ty| self.resolve_type(&ty)),
                        Some(Type::Apply { name, args }) if name == "Promise" && args.len() == 1
                    ) {
                        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                        let source = self.emit_expr(value)?.into_pointer_value();
                        self.emit_deferred_finalizers_since(
                            fn_val,
                            0,
                            self.return_exit_reason(expr.as_ref()),
                        )?;
                        self.emit_pending_finally(fn_val, false, None)?;
                        if self.current_block_terminated() {
                            return Ok(());
                        }
                        self.emit_pending_validation_cleanups(None);
                        self.emit_owned_closure_cleanups(transferred_local)?;
                        let adopt = self.declare_libc_fn(
                            "tyrus_promise_adopt",
                            &[pointer.into(), pointer.into()],
                            Some(self.context.i32_type().into()),
                        );
                        self.builder
                            .build_call(
                                adopt,
                                &[source.into(), result.into()],
                                "async.return.adopt",
                            )
                            .unwrap();
                        if !matches!(value.kind(), ExprKind::Ident(_)) {
                            let free =
                                self.declare_libc_fn("tyrus_promise_free", &[pointer.into()], None);
                            self.builder
                                .build_call(free, &[source.into()], "async.return.release")
                                .unwrap();
                        }
                        self.builder
                            .build_return(Some(&self.context.i32_type().const_int(1, false)))
                            .unwrap();
                        return Ok(());
                    }
                    let emitted = self.emit_expr_as(value, &expected)?;
                    self.emit_deferred_finalizers_since(
                        fn_val,
                        0,
                        self.return_exit_reason(expr.as_ref()),
                    )?;
                    self.emit_pending_finally(fn_val, false, None)?;
                    if self.current_block_terminated() {
                        return Ok(());
                    }
                    if let Some(release) = self
                        .returned_catch_binding(value)
                        .filter(|_| self.async_owned_value_supported(&expected))
                    {
                        let slot = self
                            .builder
                            .build_alloca(self.llvm_type(&expected), "catch.async.return")
                            .unwrap();
                        self.emit_async_clone_value_into(slot, emitted, &expected)?;
                        let owned = self
                            .builder
                            .build_load(self.llvm_type(&expected), slot, "catch.async.return.value")
                            .unwrap();
                        self.emit_pending_validation_cleanups(None);
                        self.emit_conditional_catch_binding_release(
                            fn_val,
                            release.owned_slot,
                            release.error_slot,
                            &release.catch_type,
                        )?;
                        self.emit_owned_closure_cleanups(transferred_local)?;
                        self.emit_async_transferred_value_ready(result, owned, &expected);
                        return Ok(());
                    }
                    // Clone into the promise payload before any temporary is
                    // released: a `return Err(`...`)` keeps its freshly built
                    // string inside the `Result`, and releasing the pending
                    // temporaries first freed it before the clone read it.
                    if self.async_owned_value_supported(&expected) {
                        let slot = self
                            .builder
                            .build_alloca(self.llvm_type(&expected), "owned.async.return")
                            .unwrap();
                        self.emit_async_clone_value_into(slot, emitted, &expected)?;
                        let owned = self
                            .builder
                            .build_load(self.llvm_type(&expected), slot, "owned.async.return.value")
                            .unwrap();
                        self.emit_pending_validation_cleanups(None);
                        self.emit_async_cleanup_source_expression(emitted, &expected, value)?;
                        self.emit_owned_closure_cleanups(transferred_local)?;
                        self.discard_all_owned_native_results();
                        self.emit_async_transferred_value_ready(result, owned, &expected);
                    } else {
                        self.emit_pending_validation_cleanups(None);
                        self.transfer_owned_native_return(emitted);
                        self.emit_owned_closure_cleanups(transferred_local)?;
                        self.emit_async_value_ready(result, emitted, &expected, value);
                    }
                    return Ok(());
                }
                if expr.is_none()
                    && let Some(result) = self.async_results.last().copied()
                {
                    self.emit_deferred_finalizers_since(
                        fn_val,
                        0,
                        self.return_exit_reason(expr.as_ref()),
                    )?;
                    self.emit_pending_finally(fn_val, false, None)?;
                    if self.current_block_terminated() {
                        return Ok(());
                    }
                    self.emit_pending_validation_cleanups(None);
                    self.emit_owned_closure_cleanups(None)?;
                    match self.return_types.last().cloned() {
                        Some(payload)
                            if matches!(transparent_codegen_type(&payload), Type::Optional(_)) =>
                        {
                            self.emit_async_empty_optional_ready(result, &payload);
                        }
                        _ => self.emit_async_void_ready(result),
                    }
                    return Ok(());
                }
                match expr {
                    Some(e) => {
                        if matches!(self.return_types.last(), Some(Type::Void)) {
                            self.emit_expr(e)?;
                            self.discard_all_owned_native_results();
                            self.emit_deferred_finalizers_since(
                                fn_val,
                                0,
                                self.return_exit_reason(expr.as_ref()),
                            )?;
                            self.emit_pending_finally(fn_val, false, None)?;
                            if self.current_block_terminated() {
                                return Ok(());
                            }
                            self.emit_pending_validation_cleanups(None);
                            self.emit_owned_closure_cleanups(None)?;
                            if fn_val.get_name().to_bytes() == b"main" {
                                self.builder
                                    .build_return(Some(&self.context.i32_type().const_zero()))
                                    .unwrap();
                            } else {
                                self.builder.build_return(None).unwrap();
                            }
                            return Ok(());
                        }
                        let expected = self.return_types.last().cloned();
                        let val = if let Some(expected) = expected.as_ref() {
                            self.emit_expr_as(e, expected)?
                        } else {
                            self.emit_expr(e)?
                        };
                        self.emit_deferred_finalizers_since(
                            fn_val,
                            0,
                            self.return_exit_reason(expr.as_ref()),
                        )?;
                        self.emit_pending_finally(fn_val, false, None)?;
                        if self.current_block_terminated() {
                            return Ok(());
                        }
                        if let Some(expected) = expected
                            .as_ref()
                            .filter(|expected| self.async_owned_value_supported(expected))
                        {
                            let slot = self
                                .builder
                                .build_alloca(self.llvm_type(expected), "owned.return")
                                .unwrap();
                            self.emit_async_clone_value_into(slot, val, expected)?;
                            let owned = self
                                .builder
                                .build_load(self.llvm_type(expected), slot, "owned.return.value")
                                .unwrap();
                            self.emit_pending_validation_cleanups(None);
                            self.emit_async_cleanup_source_expression(val, expected, e)?;
                            self.emit_owned_closure_cleanups(None)?;
                            self.discard_all_owned_native_results();
                            self.emit_for_of_iterable_cleanups();
                            self.builder.build_return(Some(&owned)).unwrap();
                        } else {
                            self.emit_pending_validation_cleanups(None);
                            if expected.as_ref().is_some_and(|ty| {
                                matches!(transparent_codegen_type(ty), Type::Function { .. })
                            }) {
                                let is_fresh = matches!(
                                    e.kind(),
                                    ExprKind::ArrowFn { .. }
                                        | ExprKind::FnCall { .. }
                                        | ExprKind::MethodCall { .. }
                                        | ExprKind::CallValue { .. }
                                );
                                let is_owned_local = matches!(
                                    e.kind(),
                                    ExprKind::Ident(name) if self.is_owned_closure_binding(name)
                                );
                                if !is_fresh && !is_owned_local {
                                    self.emit_closure_retain(val);
                                }
                            }
                            self.transfer_owned_native_return(val);
                            self.emit_conditional_return_owned_bit(fn_val, e);
                            self.emit_owned_closure_cleanups(transferred_local)?;
                            self.emit_for_of_iterable_cleanups();
                            self.builder.build_return(Some(&val)).unwrap();
                        }
                    }
                    None => {
                        self.emit_deferred_finalizers_since(
                            fn_val,
                            0,
                            self.return_exit_reason(expr.as_ref()),
                        )?;
                        self.emit_pending_finally(fn_val, false, None)?;
                        if self.current_block_terminated() {
                            return Ok(());
                        }
                        self.emit_pending_validation_cleanups(None);
                        self.emit_owned_closure_cleanups(None)?;
                        self.emit_for_of_iterable_cleanups();
                        let returns_optional = matches!(
                            self.return_types.last(),
                            Some(ty) if matches!(transparent_codegen_type(ty), Type::Optional(_))
                        );
                        let returns_result_void = self.return_types.last().and_then(|ty| {
                            let Type::Result { success, error } = self.resolve_type(ty) else {
                                return None;
                            };
                            matches!(success.as_ref(), Type::Void).then_some(error)
                        });
                        if let Some(error) = returns_result_void {
                            let variants = vec![Type::Void, (*error).clone()];
                            let result_ty = self.union_llvm_type(&variants);
                            let result = result_ty.const_zero();
                            let result = self
                                .builder
                                .build_insert_value(
                                    result,
                                    self.context.i8_type().const_zero(),
                                    0,
                                    "result.success.void",
                                )
                                .map_err(|error| error.to_string())?;
                            self.builder.build_return(Some(&result)).unwrap();
                        } else if fn_val.get_name().to_bytes() == b"main" {
                            self.builder
                                .build_return(Some(&self.context.i32_type().const_zero()))
                                .unwrap();
                        } else if returns_optional {
                            let null = self
                                .context
                                .ptr_type(inkwell::AddressSpace::default())
                                .const_null();
                            self.builder.build_return(Some(&null)).unwrap();
                        } else {
                            self.builder.build_return(None).unwrap();
                        }
                    }
                }
                Ok(())
            }

            StmtKind::Break(value) => {
                let target = self
                    .break_targets
                    .last()
                    .cloned()
                    .ok_or("internal: break outside loop or switch")?;
                if let Some(value_expr) = value {
                    let (slot, expected) = target
                        .value_slot
                        .ok_or("internal: break with a value outside a loop expression")?;
                    if self.async_owned_value_supported(&expected) {
                        let emitted = self.emit_expr_as(value_expr, &expected)?;
                        self.emit_async_clone_value_into(slot, emitted, &expected)?;
                        self.emit_async_cleanup_source_expression(emitted, &expected, value_expr)?;
                    } else {
                        let emitted = self.emit_expr_as(value_expr, &expected)?;
                        self.builder.build_store(slot, emitted).unwrap();
                        self.transfer_owned_native_return(emitted);
                    }
                }
                if self
                    .async_catch_cleanup_scopes
                    .last()
                    .is_some_and(|scope| self.break_targets.len() <= scope.break_depth)
                {
                    self.emit_active_async_catch_cleanup(fn_val)?;
                }
                if let Some(completion) = self.async_finally_completion_scopes.last().cloned()
                    && completion
                        .break_target
                        .is_some_and(|completion_target| completion_target.block == target.block)
                {
                    self.builder
                        .build_store(
                            completion.mode_slot,
                            self.context.i32_type().const_int(3, false),
                        )
                        .unwrap();
                    self.emit_pending_validation_cleanups_since(target.validation_depth);
                    self.emit_owned_value_cleanups_to_depth(target.scope_depth, None)?;
                    self.builder
                        .build_unconditional_branch(completion.entry)
                        .unwrap();
                    return Ok(());
                }
                self.emit_deferred_finalizers_since(
                    fn_val,
                    target.scope_depth,
                    ScopeExitReason::Break,
                )?;
                self.emit_pending_finally_since(fn_val, target.finally_depth)?;
                if self.current_block_terminated() {
                    return Ok(());
                }
                self.emit_pending_validation_cleanups_since(target.validation_depth);
                self.emit_owned_value_cleanups_to_depth(target.scope_depth, None)?;
                self.builder
                    .build_unconditional_branch(target.block)
                    .unwrap();
                Ok(())
            }

            StmtKind::Continue => {
                let loop_depth = self.loop_stack.len();
                if self
                    .async_catch_cleanup_scopes
                    .last()
                    .is_some_and(|scope| loop_depth <= scope.loop_depth)
                {
                    self.emit_active_async_catch_cleanup(fn_val)?;
                }
                if let Some(completion) = self.async_finally_completion_scopes.last().cloned()
                    && completion.continue_target.is_some_and(|(target, _)| {
                        self.loop_stack
                            .last()
                            .is_some_and(|(current, _, _)| *current == target)
                    })
                {
                    self.builder
                        .build_store(
                            completion.mode_slot,
                            self.context.i32_type().const_int(4, false),
                        )
                        .unwrap();
                    self.emit_pending_validation_cleanups(Some(loop_depth));
                    let (_, _, scope_depth) = *self
                        .loop_stack
                        .last()
                        .ok_or("internal: continue outside loop")?;
                    self.emit_owned_value_cleanups_to_depth(scope_depth, None)?;
                    self.builder
                        .build_unconditional_branch(completion.entry)
                        .unwrap();
                    return Ok(());
                }
                let (_, _, scope_depth) = *self
                    .loop_stack
                    .last()
                    .ok_or("internal: continue outside loop")?;
                self.emit_deferred_finalizers_since(
                    fn_val,
                    scope_depth,
                    ScopeExitReason::Continue,
                )?;
                self.emit_pending_finally(fn_val, false, Some(loop_depth))?;
                if self.current_block_terminated() {
                    return Ok(());
                }
                self.emit_pending_validation_cleanups(Some(loop_depth));
                let (continue_bb, _, scope_depth) = *self
                    .loop_stack
                    .last()
                    .ok_or("internal: continue outside loop")?;
                self.emit_owned_value_cleanups_to_depth(scope_depth, None)?;
                self.builder
                    .build_unconditional_branch(continue_bb)
                    .unwrap();
                Ok(())
            }
            _ => Err("internal: not a return, break or continue statement".to_string()),
        }
    }
}

impl<'ctx> Codegen<'ctx> {
    /// The `Err` that `?` returns, as the enclosing function's own
    /// `Result<U, E>`: the propagated value is a `Result<T, E>` whose success
    /// side may differ in type and size. When `?` reads a place the scope
    /// still owns (a local, a field, an element), the value is cloned first,
    /// because the scope's finalizers release that place on the way out and
    /// the caller would otherwise receive an error that was already freed.
    /// A temporary is moved. Must run before those finalizers.
    pub(crate) fn propagated_error_result(
        &mut self,
        source_ty: &Type,
        value: BasicValueEnum<'ctx>,
        borrowed: bool,
    ) -> Result<(BasicValueEnum<'ctx>, Type), String> {
        let Type::Result { success, error } = source_ty else {
            return Err("internal: `?` propagates a value that is not a Result".to_string());
        };
        let expected = self
            .return_types
            .last()
            .cloned()
            .ok_or("internal: `?` has no enclosing return type")?;
        let Type::Result {
            success: returned_success,
            error: returned_error,
        } = transparent_codegen_type(&self.resolve_type(&expected)).clone()
        else {
            return Err(
                "error: `?` requires the enclosing function to return a Result".to_string(),
            );
        };
        let source_llvm = self.llvm_type(source_ty);
        let slot = if borrowed && self.async_owned_value_supported(source_ty) {
            let slot = self
                .builder
                .build_alloca(source_llvm, "result.try.owned")
                .map_err(|error| error.to_string())?;
            self.emit_async_clone_value_into(slot, value, source_ty)?;
            slot
        } else {
            self.native_value_slot(source_llvm, value, "result.try.error")?
        };
        let source_variants = vec![success.as_ref().clone(), error.as_ref().clone()];
        let error_payload =
            self.load_union_payload(&source_variants, slot, error, "result.error.value")?;
        let returned_variants = vec![*returned_success, *returned_error];
        let union_ty = self.union_llvm_type(&returned_variants);
        let target = self
            .builder
            .build_alloca(union_ty, "result.propagated")
            .map_err(|error| error.to_string())?;
        self.store_union_variant(&returned_variants, target, 1, error_payload)?;
        let propagated = self
            .builder
            .build_load(union_ty, target, "result.propagated.value")
            .map_err(|error| error.to_string())?;
        Ok((propagated, expected))
    }

    /// The error branch of `?` inside an `async` function. The function being
    /// emitted is the state machine's poll function, so a plain `ret` of the
    /// `Result` is not its return type; the `Err` is settled through the same
    /// path a `return Err(...)` takes.
    pub(crate) fn emit_async_try_propagation(
        &mut self,
        fn_val: FunctionValue<'ctx>,
        propagated: BasicValueEnum<'ctx>,
        expected: &Type,
    ) -> Result<(), String> {
        if let Some(completion) = self.async_finally_completion_scopes.last().cloned() {
            let Some((return_slot, _)) = completion.return_slot.as_ref() else {
                return Err(
                    "internal: async finally completion for `?` has no return slot".to_string(),
                );
            };
            self.builder
                .build_store(*return_slot, propagated)
                .map_err(|error| error.to_string())?;
            if let Some(storage) = completion.return_storage.as_deref() {
                self.mark_async_frame_value_initialized(storage, true);
            }
            self.builder
                .build_store(
                    completion.mode_slot,
                    self.context.i32_type().const_int(2, false),
                )
                .map_err(|error| error.to_string())?;
            self.emit_active_async_catch_cleanup(fn_val)?;
            self.emit_pending_validation_cleanups(None);
            self.emit_owned_value_cleanups(None)?;
            self.emit_owned_closure_cleanups(None)?;
            self.builder
                .build_unconditional_branch(completion.entry)
                .map_err(|error| error.to_string())?;
            return Ok(());
        }
        let result = self
            .async_results
            .last()
            .copied()
            .ok_or("internal: async `?` has no result promise")?;
        self.emit_deferred_finalizers_since(fn_val, 0, ScopeExitReason::TryPropagation)?;
        self.emit_pending_finally(fn_val, false, None)?;
        if self.current_block_terminated() {
            return Ok(());
        }
        self.emit_pending_validation_cleanups(None);
        self.emit_owned_closure_cleanups(None)?;
        self.emit_async_transferred_value_ready(result, propagated, expected);
        Ok(())
    }
}

impl<'ctx> Codegen<'ctx> {
    /// The value `?` yields on success. `Ok()` of a `Result<void, E>` carries
    /// nothing, so the expression's value is the same placeholder a call to a
    /// `void` function produces.
    pub(crate) fn try_success_value(
        &mut self,
        variants: &[Type],
        slot: PointerValue<'ctx>,
        success: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if matches!(success, Type::Void) {
            return Ok(self.context.i32_type().const_zero().into());
        }
        self.load_union_payload(variants, slot, success, "result.success.value")
    }
}
