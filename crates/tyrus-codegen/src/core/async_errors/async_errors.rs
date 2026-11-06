use super::*;

#[path = "if_branch_recovery/if_branch_recovery.rs"]
mod if_branch_recovery;
#[path = "switch_subject_recovery/switch_subject_recovery.rs"]
mod switch_subject_recovery;
#[path = "try_recovery/try_recovery.rs"]
mod try_recovery;

/// Everything the failure-recovery handlers share. `emit_async_failure_recovery`
/// fills it once and each `AsyncAwaitKind` arm reads what it needs, which is
/// what lets those arms live in files of their own. Every field is `Copy`.
#[derive(Clone, Copy)]
pub(super) struct AsyncFailure<'a, 'ctx> {
    pub(super) await_site: &'a AsyncAwaitSite,
    pub(super) await_sites: &'a [AsyncAwaitSite],
    pub(super) catch_await_groups: &'a [AsyncCatchAwaitGroup],
    pub(super) finally_await_groups: &'a [AsyncFinallyAwaitGroup],
    pub(super) catch_resume_blocks: &'a [BasicBlock<'ctx>],
    pub(super) async_try_blocks: &'a HashMap<usize, BasicBlock<'ctx>>,
    pub(super) async_if_blocks: &'a HashMap<usize, BasicBlock<'ctx>>,
    pub(super) async_if_branch_path_blocks: &'a HashMap<Span, BasicBlock<'ctx>>,
    pub(super) poll: FunctionValue<'ctx>,
    pub(super) frame_type: inkwell::types::StructType<'ctx>,
    pub(super) frame: PointerValue<'ctx>,
    pub(super) state_pointer: PointerValue<'ctx>,
    pub(super) async_result: PointerValue<'ctx>,
    pub(super) dependency: PointerValue<'ctx>,
    pub(super) dependency_pointer: PointerValue<'ctx>,
    pub(super) promise_free: FunctionValue<'ctx>,
    pub(super) pointer: inkwell::types::PointerType<'ctx>,
    pub(super) cancelled_flag: PointerValue<'ctx>,
}

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_async_cancelled_flag(
        &mut self,
        async_result: PointerValue<'ctx>,
    ) -> Result<PointerValue<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let requested = self.declare_libc_fn(
            "tyrus_promise_cancel_requested",
            &[pointer.into()],
            Some(self.context.i32_type().into()),
        );
        let state = self
            .builder
            .build_call(requested, &[async_result.into()], "async.cancel.requested")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: cancel request query did not return a value")?
            .into_int_value();
        let cancelled = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                state,
                self.context.i32_type().const_int(1, false),
                "async.cancel.is_cancelled",
            )
            .map_err(|error| error.to_string())?;
        let slot = self.entry_alloca(self.context.bool_type(), "async.cancelled")?;
        self.builder
            .build_store(slot, cancelled)
            .map_err(|error| error.to_string())?;
        Ok(slot)
    }

    pub(super) fn emit_async_cancelled_exit(
        &mut self,
        poll: FunctionValue<'ctx>,
        cancelled_flag: PointerValue<'ctx>,
        async_result: PointerValue<'ctx>,
    ) -> Result<(), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let cancelled = self
            .builder
            .build_load(
                self.context.bool_type(),
                cancelled_flag,
                "async.cancel.exit",
            )
            .map_err(|error| error.to_string())?
            .into_int_value();
        let leave = self.context.append_basic_block(poll, "async.cancel.leave");
        let carry_on = self
            .context
            .append_basic_block(poll, "async.cancel.carry_on");
        self.builder
            .build_conditional_branch(cancelled, leave, carry_on)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(leave);
        let fulfill = self.declare_libc_fn(
            "tyrus_promise_fulfill",
            &[pointer.into(), pointer.into(), pointer.into()],
            Some(self.context.i32_type().into()),
        );
        self.builder
            .build_call(
                fulfill,
                &[
                    async_result.into(),
                    pointer.const_null().into(),
                    pointer.const_null().into(),
                ],
                "async.cancel.settle",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_return(Some(&self.context.i32_type().const_int(1, false)))
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(carry_on);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_async_failure_recovery(
        &mut self,
        await_site: &AsyncAwaitSite,
        await_sites: &[AsyncAwaitSite],
        catch_await_groups: &[AsyncCatchAwaitGroup],
        finally_await_groups: &[AsyncFinallyAwaitGroup],
        catch_resume_blocks: &[BasicBlock<'ctx>],
        async_try_blocks: &HashMap<usize, BasicBlock<'ctx>>,
        async_if_blocks: &HashMap<usize, BasicBlock<'ctx>>,
        async_if_branch_path_blocks: &HashMap<Span, BasicBlock<'ctx>>,
        recursive_control_blocks: &HashMap<(usize, usize), AsyncRecursiveControlBlocks<'ctx>>,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
        async_result: PointerValue<'ctx>,
        dependency: PointerValue<'ctx>,
        dependency_pointer: PointerValue<'ctx>,
        promise_free: FunctionValue<'ctx>,
    ) -> Result<Option<BasicBlock<'ctx>>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let cancelled_flag = self.emit_async_cancelled_flag(async_result)?;
        if let AsyncAwaitKind::SwitchCaseExpression {
            subject_storage,
            subject_type,
            ..
        } = &await_site.kind
        {
            self.finish_async_switch_subject(poll, subject_storage, subject_type)?;
        }
        let failure = AsyncFailure {
            await_site,
            await_sites,
            catch_await_groups,
            finally_await_groups,
            catch_resume_blocks,
            async_try_blocks,
            async_if_blocks,
            async_if_branch_path_blocks,
            poll,
            frame_type,
            frame,
            state_pointer,
            async_result,
            dependency,
            dependency_pointer,
            promise_free,
            pointer,
            cancelled_flag,
        };
        let recovered = match &await_site.kind {
            _ if control_flow_contexts(await_site).is_some_and(|contexts| {
                contexts
                    .iter()
                    .any(|context| matches!(context, AsyncControlContext::Try { .. }))
            }) =>
            {
                let depth = control_flow_contexts(await_site)
                    .and_then(|contexts| {
                        contexts
                            .iter()
                            .rposition(|context| matches!(context, AsyncControlContext::Try { .. }))
                    })
                    .ok_or_else(|| {
                        "internal: recursive async try context is missing".to_string()
                    })?;
                let recovery = AsyncTryRecovery {
                    await_site,
                    await_sites,
                    catch_await_groups,
                    finally_await_groups,
                    recursive_control_blocks,
                    poll,
                    frame_type,
                    frame,
                    state_pointer,
                    async_result,
                    dependency,
                    dependency_pointer,
                    promise_free,
                    cancelled_flag,
                };
                self.emit_async_recursive_try_recovery(depth, &recovery)?;
                None
            }
            AsyncAwaitKind::Try {
                catch_binding,
                catch_type,
                catch_body,
                finally_body,
                final_in_try,
                ..
            } => self.emit_async_try_failure(
                failure,
                catch_binding,
                catch_type,
                catch_body,
                finally_body,
                *final_in_try,
            )?,
            AsyncAwaitKind::SwitchSubject {
                context:
                    Some(AsyncSwitchContext::Try {
                        catch_binding,
                        catch_type,
                        catch_body,
                        finally_body,
                        ..
                    }),
                ..
            }
            | AsyncAwaitKind::SwitchCaseExpression {
                context:
                    Some(AsyncSwitchContext::Try {
                        catch_binding,
                        catch_type,
                        catch_body,
                        finally_body,
                        ..
                    }),
                ..
            }
            | AsyncAwaitKind::Switch {
                context:
                    Some(AsyncSwitchContext::Try {
                        catch_binding,
                        catch_type,
                        catch_body,
                        finally_body,
                        ..
                    }),
                ..
            } => self.emit_async_switch_subject_failure(
                failure,
                catch_binding,
                catch_type,
                catch_body,
                finally_body,
            )?,
            AsyncAwaitKind::IfBranch {
                try_context: Some(context),
                ..
            }
            | AsyncAwaitKind::IfBothBranches {
                try_context: Some(context),
                ..
            } => self.emit_async_if_branch_failure(failure, context)?,
            AsyncAwaitKind::Direct { .. }
            | AsyncAwaitKind::Binding { .. }
            | AsyncAwaitKind::Assignment { .. }
            | AsyncAwaitKind::IfCondition { .. }
            | AsyncAwaitKind::WhileCondition { .. }
            | AsyncAwaitKind::WhileBody { .. }
            | AsyncAwaitKind::DoWhileCondition { .. }
            | AsyncAwaitKind::DoWhileBody { .. }
            | AsyncAwaitKind::ForCondition { .. }
            | AsyncAwaitKind::ForBody { .. }
            | AsyncAwaitKind::ForOfBody { .. }
            | AsyncAwaitKind::ControlFlow { .. }
            | AsyncAwaitKind::FinallyCompletion { .. }
            | AsyncAwaitKind::IfBranch { .. }
            | AsyncAwaitKind::IfBothBranches { .. }
            | AsyncAwaitKind::SwitchSubject { .. }
            | AsyncAwaitKind::SwitchCaseExpression { .. }
            | AsyncAwaitKind::Switch { .. }
            | AsyncAwaitKind::Expression { .. } => {
                let propagate = self.declare_libc_fn(
                    "tyrus_promise_propagate_failure",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.i32_type().into()),
                );
                self.builder
                    .build_call(
                        propagate,
                        &[dependency.into(), async_result.into()],
                        "async.await.propagate",
                    )
                    .unwrap();
                self.builder
                    .build_call(promise_free, &[dependency.into()], "")
                    .unwrap();
                self.builder
                    .build_store(dependency_pointer, pointer.const_null())
                    .unwrap();
                self.builder
                    .build_return(Some(&self.context.i32_type().const_int(1, false)))
                    .unwrap();
                None
            }
        };
        Ok(recovered)
    }

    pub(super) fn emit_async_synchronous_catch_entry(
        &mut self,
        await_site: &AsyncAwaitSite,
        catch_await_groups: &[AsyncCatchAwaitGroup],
        poll: FunctionValue<'ctx>,
    ) -> Result<Option<AsyncSynchronousCatch<'ctx>>, String> {
        let catch_suspends = catch_await_groups
            .iter()
            .any(|group| group.outer_index == await_site.outer_index);
        if catch_suspends {
            return Ok(None);
        }
        let AsyncAwaitKind::Try { catch_type, .. } = &await_site.kind else {
            return Ok(None);
        };
        if !self.catch_binding_has_a_layout(catch_type) {
            return Ok(None);
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let context_type = self.catch_ctx_type();
        let error_slot = self
            .builder
            .build_alloca(context_type, "async.sync.try.error.storage")
            .unwrap();
        let context_slot = self
            .builder
            .build_alloca(context_type, "async.sync.try.context")
            .unwrap();
        let owned_slot = self
            .builder
            .build_alloca(self.context.bool_type(), "async.sync.try.error.owned")
            .unwrap();
        self.builder
            .build_store(owned_slot, self.context.bool_type().const_zero())
            .unwrap();
        let owned_field = self
            .builder
            .build_struct_gep(context_type, context_slot, 3, "async.sync.try.owned")
            .unwrap();
        self.builder.build_store(owned_field, owned_slot).unwrap();
        let catch_top = self.get_or_declare_catch_top().as_pointer_value();
        let previous = self
            .builder
            .build_load(pointer, catch_top, "async.sync.try.previous")
            .unwrap()
            .into_pointer_value();
        let previous_field = self
            .builder
            .build_struct_gep(context_type, context_slot, 2, "async.sync.try.previous")
            .unwrap();
        self.builder.build_store(previous_field, previous).unwrap();
        let error_field = self
            .builder
            .build_struct_gep(context_type, context_slot, 1, "async.sync.try.error")
            .unwrap();
        self.builder.build_store(error_field, error_slot).unwrap();
        self.builder.build_store(catch_top, context_slot).unwrap();
        let jump_buffer = self
            .builder
            .build_struct_gep(context_type, context_slot, 0, "async.sync.try.jump")
            .unwrap();
        let jumped = self
            .builder
            .build_call(
                self.declare_setjmp(),
                &[jump_buffer.into()],
                "async.sync.try.setjmp",
            )
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        let caught = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::NE,
                jumped,
                self.context.i32_type().const_zero(),
                "async.sync.try.caught",
            )
            .unwrap();
        let body_block = self.context.append_basic_block(poll, "async.sync.try.body");
        let catch_block = self
            .context
            .append_basic_block(poll, "async.sync.try.catch");
        self.builder
            .build_conditional_branch(caught, catch_block, body_block)
            .unwrap();
        self.builder.position_at_end(body_block);
        let scope_depth = self.owned_native_vars.len().saturating_sub(1);
        self.catch_context_restores
            .push((catch_top, previous, scope_depth));
        self.push_throw_catch_target(catch_type);
        Ok(Some(AsyncSynchronousCatch {
            context_slot,
            catch_top,
            previous,
            error_slot,
            owned_slot,
            catch_block,
        }))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_async_synchronous_catch_handler(
        &mut self,
        entry: &AsyncSynchronousCatch<'ctx>,
        catch_binding: &str,
        catch_type: &Type,
        catch_body: &[Stmt],
        finally_body: &[Stmt],
        outer_index: usize,
        blocks: &mut AsyncBlockRegistry<'ctx>,
        poll: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        self.builder.position_at_end(entry.catch_block);
        self.builder
            .build_store(entry.catch_top, entry.previous)
            .unwrap();
        let forward_finally: &[Stmt] = if first_await_in_block(finally_body).is_some() {
            &[]
        } else {
            finally_body
        };
        let subtype_storage = self.error_subtype_storage(catch_binding, catch_type)?;
        self.emit_receive_thrown_cell(
            entry.context_slot,
            catch_type,
            entry.error_slot,
            forward_finally,
            poll,
            subtype_storage.as_ref(),
        )?;
        self.push_scope();
        self.store_var(catch_binding, entry.error_slot, catch_type.clone());
        let kept_subtypes = subtype_storage.map(|mut storage| {
            storage.scope_depth = self.vars.len();
            self.catch_error_subtypes.push(storage.clone());
            storage
        });
        let tracks_owned_error = self.thrown_value_can_be_cloned(catch_type);
        if tracks_owned_error {
            self.catch_error_owned_slots
                .push((catch_binding.to_string(), entry.owned_slot));
            self.push_catch_binding_release(
                poll,
                catch_binding,
                entry.owned_slot,
                entry.error_slot,
                catch_type,
            );
        }
        for statement in catch_body {
            if self.current_block_terminated() {
                break;
            }
            self.emit_stmt(statement, poll)?;
        }
        if let Some(storage) = kept_subtypes.as_ref()
            && !self.current_block_terminated()
        {
            self.emit_error_subtype_storage_release(poll, storage, Some(entry.owned_slot))?;
        }
        if !self.current_block_terminated() && tracks_owned_error {
            self.emit_conditional_catch_binding_release(
                poll,
                entry.owned_slot,
                entry.error_slot,
                catch_type,
            )?;
        }
        if tracks_owned_error {
            self.pop_catch_binding_release();
            self.catch_error_owned_slots.pop();
        }
        self.pop_scope();
        if !self.current_block_terminated() {
            for statement in finally_body {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
        }
        if !self.current_block_terminated() {
            let recovered = blocks
                .try_blocks
                .get(&outer_index)
                .copied()
                .ok_or_else(|| "internal: async try continuation is missing".to_string())?;
            self.builder.build_unconditional_branch(recovered).unwrap();
        }
        Ok(())
    }

    pub(super) fn end_async_synchronous_catch_stretch(
        &mut self,
        entry: &AsyncSynchronousCatch<'ctx>,
    ) {
        self.catch_context_restores.pop();
        self.pop_throw_catch_target();
        if !self.current_block_terminated() {
            self.builder
                .build_store(entry.catch_top, entry.previous)
                .unwrap();
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_async_catch_only_try(
        &mut self,
        group: &AsyncCatchAwaitGroup,
        try_body: &[Stmt],
        await_sites: &[AsyncAwaitSite],
        catch_await_groups: &[AsyncCatchAwaitGroup],
        blocks: &mut AsyncBlockRegistry<'ctx>,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
    ) -> Result<(), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let context_type = self.catch_ctx_type();
        let (error_frame_slot, error_type) = self
            .async_frame_slot(&group.storage_name)
            .ok_or_else(|| "internal: async catch error has no frame slot".to_string())?;
        let error_slot = self
            .builder
            .build_alloca(self.llvm_type(&error_type), "async.try.error")
            .unwrap();
        let context_slot = self
            .builder
            .build_alloca(context_type, "async.try.context")
            .unwrap();
        let owned_slot = self
            .builder
            .build_alloca(self.context.bool_type(), "async.try.error.owned")
            .unwrap();
        self.builder
            .build_store(owned_slot, self.context.bool_type().const_zero())
            .unwrap();
        let owned_field = self
            .builder
            .build_struct_gep(context_type, context_slot, 3, "async.try.context.owned")
            .unwrap();
        self.builder.build_store(owned_field, owned_slot).unwrap();
        let catch_top = self.get_or_declare_catch_top().as_pointer_value();
        let previous = self
            .builder
            .build_load(pointer, catch_top, "async.try.previous")
            .unwrap()
            .into_pointer_value();
        let previous_field = self
            .builder
            .build_struct_gep(context_type, context_slot, 2, "async.try.context.previous")
            .unwrap();
        self.builder.build_store(previous_field, previous).unwrap();
        let error_field = self
            .builder
            .build_struct_gep(context_type, context_slot, 1, "async.try.context.error")
            .unwrap();
        self.builder.build_store(error_field, error_slot).unwrap();
        self.builder.build_store(catch_top, context_slot).unwrap();
        let jump_buffer = self
            .builder
            .build_struct_gep(context_type, context_slot, 0, "async.try.jump.buffer")
            .unwrap();
        let jumped = self
            .builder
            .build_call(
                self.declare_setjmp(),
                &[jump_buffer.into()],
                "async.try.setjmp",
            )
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        let caught = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::NE,
                jumped,
                self.context.i32_type().const_zero(),
                "async.try.caught",
            )
            .unwrap();
        let try_block = self.context.append_basic_block(poll, "async.try.body");
        let catch_block = self
            .context
            .append_basic_block(poll, "async.try.catch.entry");
        let recovered = self.context.append_basic_block(poll, "async.try.recovered");
        blocks.try_blocks.insert(group.outer_index, recovered);
        self.builder
            .build_conditional_branch(caught, catch_block, try_block)
            .unwrap();

        self.builder.position_at_end(try_block);
        if !group.finally_body.is_empty() {
            self.finally_scopes.push(FinallyScope {
                body: group.finally_body.clone(),
                catches_throw: false,
                loop_depth: self.loop_stack.len(),
            });
        }
        self.push_scope();
        let catch_scope_depth = self.owned_native_vars.len().saturating_sub(1);
        self.catch_context_restores
            .push((catch_top, previous, catch_scope_depth));
        self.catch_binding_types.push(error_type.clone());
        self.push_throw_catch_target(&group.catch_type);
        for statement in try_body {
            if self.current_block_terminated() {
                break;
            }
            self.emit_stmt(statement, poll)?;
        }
        self.catch_binding_types.pop();
        self.catch_context_restores.pop();
        self.pop_throw_catch_target();
        self.pop_scope();
        if !group.finally_body.is_empty() {
            self.finally_scopes.pop();
        }
        if !self.current_block_terminated() {
            self.builder.build_store(catch_top, previous).unwrap();
            for statement in &group.finally_body {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
            if !self.current_block_terminated() {
                self.builder.build_unconditional_branch(recovered).unwrap();
            }
        }

        self.builder.position_at_end(catch_block);
        self.builder.build_store(catch_top, previous).unwrap();
        let forward_finally = if first_await_in_block(&group.finally_body).is_some() {
            Vec::new()
        } else {
            group.finally_body.clone()
        };
        let stack_subtypes = self.error_subtype_storage(&group.catch_binding, &error_type)?;
        self.emit_receive_thrown_cell(
            context_slot,
            &error_type,
            error_slot,
            &forward_finally,
            poll,
            stack_subtypes.as_ref(),
        )?;
        let error = self
            .builder
            .build_load(
                self.llvm_type(&error_type),
                error_slot,
                "async.try.caught.error",
            )
            .unwrap();
        self.emit_async_clone_value_into(error_frame_slot, error, &error_type)?;
        self.mark_async_frame_slot_initialized(error_frame_slot, true);
        let frame_subtypes = match stack_subtypes.as_ref() {
            Some(kept) => {
                match self.frame_error_subtype_storage(&group.storage_name, &group.catch_binding) {
                    Some(frame_kept) => {
                        self.emit_keep_thrown_error_subtype_in_frame(
                            poll,
                            context_slot,
                            kept,
                            &frame_kept,
                        )?;
                        Some(frame_kept)
                    }
                    None => None,
                }
            }
            None => None,
        };
        if let Some(storage) = stack_subtypes.as_ref() {
            self.emit_error_subtype_storage_release(poll, storage, Some(owned_slot))?;
        }
        self.emit_conditional_catch_binding_release(poll, owned_slot, error_slot, &error_type)?;
        if !group.finally_body.is_empty() {
            self.finally_scopes.push(FinallyScope {
                body: group.finally_body.clone(),
                catches_throw: true,
                loop_depth: self.loop_stack.len(),
            });
        }
        self.push_scope();
        self.store_var(
            &group.catch_binding,
            error_frame_slot,
            group.catch_type.clone(),
        );
        if let Some(mut storage) = frame_subtypes {
            storage.scope_depth = self.vars.len();
            self.catch_error_subtypes.push(storage);
        }
        for statement in &group.sites[0].before {
            if self.current_block_terminated() {
                break;
            }
            self.emit_stmt(statement, poll)?;
        }
        if !self.current_block_terminated() {
            let preceding = catch_await_groups
                .iter()
                .take_while(|candidate| candidate.storage_name != group.storage_name)
                .map(|candidate| candidate.sites.len())
                .sum::<usize>();
            let catch_state = await_sites.len() + preceding + 1;
            self.emit_async_suspend_dependency(
                poll,
                frame_type,
                frame,
                state_pointer,
                catch_state as u64,
                &group.sites[0].dependency,
            )?;
        }
        self.pop_scope();
        if !group.finally_body.is_empty() {
            self.finally_scopes.pop();
        }
        self.builder.position_at_end(recovered);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_async_catch_resume(
        &mut self,
        await_sites: &[AsyncAwaitSite],
        catch_await_groups: &[AsyncCatchAwaitGroup],
        finally_await_groups: &[AsyncFinallyAwaitGroup],
        catch_resume_blocks: &[BasicBlock<'ctx>],
        blocks: &AsyncBlockRegistry<'ctx>,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
        async_result: PointerValue<'ctx>,
    ) -> Result<(), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let mut catch_block_index = 0usize;
        for group in catch_await_groups {
            let (error_slot, error_type) = self
                .async_frame_slot(&group.storage_name)
                .ok_or_else(|| "internal: async catch error has no frame slot".to_string())?;
            let recovered = if let Some(try_id) = group.recursive_try_id {
                blocks
                    .recursive_try_blocks
                    .get(&try_id)
                    .copied()
                    .ok_or_else(|| {
                        "internal: recursive async catch continuation has no try merge block"
                            .to_string()
                    })?
            } else {
                blocks
                    .try_blocks
                    .get(&group.outer_index)
                    .copied()
                    .ok_or_else(|| {
                        "internal: async catch continuation has no try merge block".to_string()
                    })?
            };
            let recursive_loop_control = group.recursive_try_depth.and_then(|try_depth| {
                blocks
                    .recursive_control_blocks
                    .iter()
                    .filter_map(|(&(candidate_outer, depth), block)| {
                        if candidate_outer != group.outer_index || depth >= try_depth {
                            return None;
                        }
                        let AsyncRecursiveControlBlocks::Loop {
                            continue_target,
                            after,
                            finally_depth,
                            ..
                        } = block
                        else {
                            return None;
                        };
                        Some((depth, *continue_target, *after, *finally_depth))
                    })
                    .max_by_key(|(depth, _, _, _)| *depth)
                    .map(|(_, continue_target, after, finally_depth)| {
                        (continue_target, after, finally_depth)
                    })
            });
            for (site_index, site) in group.sites.iter().enumerate() {
                self.builder
                    .position_at_end(catch_resume_blocks[catch_block_index]);
                catch_block_index += 1;
                self.push_scope();
                if let Some((continue_target, after, finally_depth)) = recursive_loop_control {
                    self.push_loop_control_with_finally_depth(
                        continue_target,
                        after,
                        finally_depth,
                    );
                }
                self.store_var(&group.catch_binding, error_slot, group.catch_type.clone());
                if let Some(storage) =
                    self.frame_error_subtype_storage(&group.storage_name, &group.catch_binding)
                {
                    self.catch_error_subtypes.push(storage);
                }
                let dependency_pointer = self
                    .builder
                    .build_struct_gep(frame_type, frame, 1, "async.frame.dependency")
                    .unwrap();
                let dependency = self
                    .builder
                    .build_load(pointer, dependency_pointer, "async.catch.dependency")
                    .unwrap()
                    .into_pointer_value();
                let promise_state = self.declare_libc_fn(
                    "tyrus_promise_state",
                    &[pointer.into()],
                    Some(self.context.i32_type().into()),
                );
                let state = self
                    .builder
                    .build_call(promise_state, &[dependency.into()], "async.catch.state")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                let fulfilled = self
                    .context
                    .append_basic_block(poll, "async.catch.fulfilled");
                let failed = self.context.append_basic_block(poll, "async.catch.failed");
                let is_fulfilled = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::EQ,
                        state,
                        self.context.i32_type().const_int(1, false),
                        "async.catch.is_fulfilled",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(is_fulfilled, fulfilled, failed)
                    .unwrap();

                let promise_free =
                    self.declare_libc_fn("tyrus_promise_free", &[pointer.into()], None);
                self.builder.position_at_end(failed);
                self.clear_async_frame_local_before_store(
                    poll,
                    &group.storage_name,
                    error_slot,
                    &error_type,
                )?;
                self.builder
                    .build_store(error_slot, self.llvm_type(&error_type).const_zero())
                    .unwrap();
                for statement in &group.finally_body {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, poll)?;
                }
                if !self.current_block_terminated() {
                    if let Some(finally_group) = finally_await_groups.iter().find(|candidate| {
                        candidate.recursive_try_id == group.recursive_try_id
                            && candidate.outer_index == group.outer_index
                    }) {
                        let preceding = finally_await_groups
                            .iter()
                            .take_while(|candidate| {
                                candidate.mode_storage != finally_group.mode_storage
                            })
                            .map(|candidate| candidate.sites.len())
                            .sum::<usize>();
                        let finally_state =
                            await_sites.len() + catch_resume_blocks.len() + preceding + 1;
                        self.emit_async_begin_finally(
                            poll,
                            frame_type,
                            frame,
                            state_pointer,
                            finally_state as u64,
                            finally_group,
                            Some(dependency),
                        )?;
                    } else if self.emit_async_route_rejection_outward(
                        group.recursive_try_id,
                        group.outer_index,
                        dependency,
                        await_sites,
                        catch_await_groups,
                        finally_await_groups,
                        blocks,
                        poll,
                        frame_type,
                        frame,
                        state_pointer,
                        async_result,
                    )? {
                    } else {
                        let propagate = self.declare_libc_fn(
                            "tyrus_promise_propagate_failure",
                            &[pointer.into(), pointer.into()],
                            Some(self.context.i32_type().into()),
                        );
                        self.builder
                            .build_call(
                                propagate,
                                &[dependency.into(), async_result.into()],
                                "async.catch.propagate",
                            )
                            .unwrap();
                        self.builder
                            .build_call(promise_free, &[dependency.into()], "")
                            .unwrap();
                        self.builder
                            .build_store(dependency_pointer, pointer.const_null())
                            .unwrap();
                        self.builder
                            .build_return(Some(&self.context.i32_type().const_int(1, false)))
                            .unwrap();
                    }
                }

                self.builder.position_at_end(fulfilled);
                let payload_type = self
                    .expr_type_hint(&site.dependency)
                    .and_then(|promise| {
                        promise_payload_type(transparent_codegen_type(&promise)).cloned()
                    })
                    .map_or(Type::Void, |ty| self.resolve_type(&ty));
                match &site.action {
                    AsyncNestedAwaitAction::Discard => {
                        let await_ready = self.declare_libc_fn(
                            "tyrus_promise_await",
                            &[pointer.into()],
                            Some(pointer.into()),
                        );
                        self.builder
                            .build_call(await_ready, &[dependency.into()], "async.catch.ready")
                            .unwrap();
                        self.builder
                            .build_call(promise_free, &[dependency.into()], "")
                            .unwrap();
                        self.builder
                            .build_store(dependency_pointer, pointer.const_null())
                            .unwrap();
                    }
                    action => {
                        let take = self.declare_libc_fn(
                            "tyrus_promise_take_value",
                            &[pointer.into()],
                            Some(pointer.into()),
                        );
                        let payload = self
                            .builder
                            .build_call(take, &[dependency.into()], "async.catch.take")
                            .unwrap()
                            .try_as_basic_value()
                            .basic()
                            .unwrap()
                            .into_pointer_value();
                        let value = self
                            .builder
                            .build_load(
                                self.llvm_type(&payload_type),
                                payload,
                                "async.catch.payload",
                            )
                            .unwrap();
                        self.builder
                            .build_call(promise_free, &[dependency.into()], "")
                            .unwrap();
                        self.builder
                            .build_store(dependency_pointer, pointer.const_null())
                            .unwrap();
                        let free = self.declare_libc_fn("free", &[pointer.into()], None);
                        self.builder
                            .build_call(free, &[payload.into()], "")
                            .unwrap();
                        self.store_async_nested_await_action(poll, action, value, &payload_type)?;
                    }
                }
                if !group.finally_body.is_empty() {
                    self.finally_scopes.push(FinallyScope {
                        body: group.finally_body.clone(),
                        catches_throw: true,
                        loop_depth: self.loop_stack.len(),
                    });
                }
                let completion_scope = finally_await_groups
                    .iter()
                    .find(|candidate| {
                        candidate.recursive_try_id == group.recursive_try_id
                            && candidate.outer_index == group.outer_index
                    })
                    .and_then(|finally_group| {
                        finally_group
                            .completion
                            .as_ref()
                            .map(|completion| (finally_group, completion))
                    })
                    .map(|(finally_group, completion)| {
                        let completion_blocks = blocks
                            .finally_completion_blocks
                            .get(&finally_group.mode_storage)
                            .copied()
                            .ok_or_else(|| {
                                "internal: async catch completion blocks are missing".to_string()
                            })?;
                        let (mode_slot, _) = self
                            .async_frame_slot(&completion.mode_storage)
                            .ok_or_else(|| {
                                "internal: async catch completion mode has no frame slot"
                                    .to_string()
                            })?;
                        let (throw_slot, _) = self
                            .async_frame_slot(&completion.throw_storage)
                            .ok_or_else(|| {
                                "internal: async catch completion throw has no frame slot"
                                    .to_string()
                            })?;
                        let return_slot = self
                            .async_frame_slot(&completion.return_storage)
                            .map(|(slot, ty)| (slot, self.resolve_type(&ty)));
                        let return_storage = return_slot
                            .is_some()
                            .then(|| completion.return_storage.clone());
                        Ok::<_, String>(AsyncFinallyCompletionScope {
                            mode_slot,
                            return_slot,
                            return_storage,
                            entry: completion_blocks.entry,
                            throw_entry: None,
                            throw_storage: completion.throw_storage.clone(),
                            break_target: None,
                            continue_target: None,
                            throw_slot,
                            intercept_throw: true,
                        })
                    })
                    .transpose()?;
                if let Some(completion_scope) = completion_scope.as_ref() {
                    self.async_finally_completion_scopes
                        .push(completion_scope.clone());
                }
                let statements = if site.final_in_catch {
                    &site.after
                } else {
                    &group.sites[site_index + 1].before
                };
                self.async_catch_cleanup_scopes
                    .push(AsyncCatchCleanupScope {
                        slot: error_slot,
                        ty: error_type.clone(),
                        binding: group.catch_binding.clone(),
                        storage_name: group.storage_name.clone(),
                        break_depth: self.break_targets.len(),
                        loop_depth: self.loop_stack.len(),
                        transferred: false,
                    });
                for statement in statements {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, poll)?;
                }
                self.async_catch_cleanup_scopes.pop();
                if !group.finally_body.is_empty() {
                    self.finally_scopes.pop();
                }
                if completion_scope.is_some() {
                    self.async_finally_completion_scopes.pop();
                }
                if !self.current_block_terminated() {
                    if site.final_in_catch {
                        self.clear_async_frame_local_before_store(
                            poll,
                            &group.storage_name,
                            error_slot,
                            &error_type,
                        )?;
                        self.builder
                            .build_store(error_slot, self.llvm_type(&error_type).const_zero())
                            .unwrap();
                        if !group.finally_at_recovered {
                            for statement in &group.finally_body {
                                if self.current_block_terminated() {
                                    break;
                                }
                                self.emit_stmt(statement, poll)?;
                            }
                        }
                        if !self.current_block_terminated() {
                            if let Some(finally_group) =
                                finally_await_groups.iter().find(|candidate| {
                                    candidate.recursive_try_id == group.recursive_try_id
                                        && candidate.outer_index == group.outer_index
                                })
                            {
                                let preceding = finally_await_groups
                                    .iter()
                                    .take_while(|candidate| {
                                        candidate.mode_storage != finally_group.mode_storage
                                    })
                                    .map(|candidate| candidate.sites.len())
                                    .sum::<usize>();
                                let finally_state =
                                    await_sites.len() + catch_resume_blocks.len() + preceding + 1;
                                self.emit_async_begin_finally(
                                    poll,
                                    frame_type,
                                    frame,
                                    state_pointer,
                                    finally_state as u64,
                                    finally_group,
                                    None,
                                )?;
                            } else {
                                self.builder.build_unconditional_branch(recovered).unwrap();
                            }
                        }
                    } else {
                        let preceding = catch_await_groups
                            .iter()
                            .take_while(|candidate| candidate.storage_name != group.storage_name)
                            .map(|candidate| candidate.sites.len())
                            .sum::<usize>();
                        let next_state = await_sites.len() + preceding + site_index + 2;
                        self.emit_async_suspend_dependency(
                            poll,
                            frame_type,
                            frame,
                            state_pointer,
                            next_state as u64,
                            &group.sites[site_index + 1].dependency,
                        )?;
                    }
                }
                if recursive_loop_control.is_some() {
                    self.pop_loop_control();
                }
                self.pop_scope();
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_async_finally_resume(
        &mut self,
        await_sites: &[AsyncAwaitSite],
        catch_await_groups: &[AsyncCatchAwaitGroup],
        finally_await_groups: &[AsyncFinallyAwaitGroup],
        catch_resume_blocks: &[BasicBlock<'ctx>],
        finally_resume_blocks: &[BasicBlock<'ctx>],
        blocks: &AsyncBlockRegistry<'ctx>,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
        async_result: PointerValue<'ctx>,
    ) -> Result<(), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let mut finally_block_index = 0usize;
        for group in finally_await_groups {
            let (mode_slot, _) = self
                .async_frame_slot(&group.mode_storage)
                .ok_or_else(|| "internal: async finally mode has no frame slot".to_string())?;
            let (failure_slot, _) = self
                .async_frame_slot(&group.failure_storage)
                .ok_or_else(|| "internal: async finally failure has no frame slot".to_string())?;
            let recovered = if let Some(try_id) = group.recursive_try_id {
                blocks
                    .recursive_try_blocks
                    .get(&try_id)
                    .copied()
                    .ok_or_else(|| {
                        "internal: recursive async finally continuation has no try merge block"
                            .to_string()
                    })?
            } else {
                blocks
                    .try_blocks
                    .get(&group.outer_index)
                    .copied()
                    .ok_or_else(|| {
                        "internal: async finally continuation has no try merge block".to_string()
                    })?
            };
            for (site_index, site) in group.sites.iter().enumerate() {
                self.builder
                    .position_at_end(finally_resume_blocks[finally_block_index]);
                finally_block_index += 1;
                let dependency_pointer = self
                    .builder
                    .build_struct_gep(frame_type, frame, 1, "async.frame.dependency")
                    .unwrap();
                let dependency = self
                    .builder
                    .build_load(pointer, dependency_pointer, "async.finally.dependency")
                    .unwrap()
                    .into_pointer_value();
                let promise_state = self.declare_libc_fn(
                    "tyrus_promise_state",
                    &[pointer.into()],
                    Some(self.context.i32_type().into()),
                );
                let state = self
                    .builder
                    .build_call(promise_state, &[dependency.into()], "async.finally.state")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                let fulfilled = self
                    .context
                    .append_basic_block(poll, "async.finally.fulfilled");
                let failed = self
                    .context
                    .append_basic_block(poll, "async.finally.failed");
                let is_fulfilled = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::EQ,
                        state,
                        self.context.i32_type().const_int(1, false),
                        "async.finally.is_fulfilled",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(is_fulfilled, fulfilled, failed)
                    .unwrap();

                let promise_free =
                    self.declare_libc_fn("tyrus_promise_free", &[pointer.into()], None);
                self.builder.position_at_end(failed);
                let pending_failure = self
                    .builder
                    .build_load(pointer, failure_slot, "async.finally.pending.failure")
                    .unwrap()
                    .into_pointer_value();
                let pending_present = self
                    .builder
                    .build_is_not_null(pending_failure, "async.finally.pending.present")
                    .unwrap();
                let release_pending = self
                    .context
                    .append_basic_block(poll, "async.finally.release.pending");
                let propagate_finally = self
                    .context
                    .append_basic_block(poll, "async.finally.propagate.failure");
                self.builder
                    .build_conditional_branch(pending_present, release_pending, propagate_finally)
                    .unwrap();
                self.builder.position_at_end(release_pending);
                self.builder
                    .build_call(promise_free, &[pending_failure.into()], "")
                    .unwrap();
                self.builder
                    .build_store(failure_slot, pointer.const_null())
                    .unwrap();
                self.builder
                    .build_unconditional_branch(propagate_finally)
                    .unwrap();
                self.builder.position_at_end(propagate_finally);
                let propagate = self.declare_libc_fn(
                    "tyrus_promise_propagate_failure",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.i32_type().into()),
                );
                if !self.emit_async_route_rejection_outward(
                    group.recursive_try_id,
                    group.outer_index,
                    dependency,
                    await_sites,
                    catch_await_groups,
                    finally_await_groups,
                    blocks,
                    poll,
                    frame_type,
                    frame,
                    state_pointer,
                    async_result,
                )? {
                    self.builder
                        .build_call(
                            propagate,
                            &[dependency.into(), async_result.into()],
                            "async.finally.propagate",
                        )
                        .unwrap();
                    self.builder
                        .build_call(promise_free, &[dependency.into()], "")
                        .unwrap();
                    self.builder
                        .build_store(dependency_pointer, pointer.const_null())
                        .unwrap();
                    self.builder
                        .build_return(Some(&self.context.i32_type().const_int(1, false)))
                        .unwrap();
                }

                self.builder.position_at_end(fulfilled);
                let payload_type = self
                    .expr_type_hint(&site.dependency)
                    .and_then(|promise| {
                        promise_payload_type(transparent_codegen_type(&promise)).cloned()
                    })
                    .map_or(Type::Void, |ty| self.resolve_type(&ty));
                match &site.action {
                    AsyncNestedAwaitAction::Discard => {
                        let await_ready = self.declare_libc_fn(
                            "tyrus_promise_await",
                            &[pointer.into()],
                            Some(pointer.into()),
                        );
                        self.builder
                            .build_call(await_ready, &[dependency.into()], "async.finally.ready")
                            .unwrap();
                        self.builder
                            .build_call(promise_free, &[dependency.into()], "")
                            .unwrap();
                        self.builder
                            .build_store(dependency_pointer, pointer.const_null())
                            .unwrap();
                    }
                    action => {
                        let take = self.declare_libc_fn(
                            "tyrus_promise_take_value",
                            &[pointer.into()],
                            Some(pointer.into()),
                        );
                        let payload = self
                            .builder
                            .build_call(take, &[dependency.into()], "async.finally.take")
                            .unwrap()
                            .try_as_basic_value()
                            .basic()
                            .unwrap()
                            .into_pointer_value();
                        let value = self
                            .builder
                            .build_load(
                                self.llvm_type(&payload_type),
                                payload,
                                "async.finally.payload",
                            )
                            .unwrap();
                        self.builder
                            .build_call(promise_free, &[dependency.into()], "")
                            .unwrap();
                        self.builder
                            .build_store(dependency_pointer, pointer.const_null())
                            .unwrap();
                        let free = self.declare_libc_fn("free", &[pointer.into()], None);
                        self.builder
                            .build_call(free, &[payload.into()], "")
                            .unwrap();
                        self.store_async_nested_await_action(poll, action, value, &payload_type)?;
                    }
                }
                let statements = if site.final_in_finally {
                    &site.after
                } else {
                    &group.sites[site_index + 1].before
                };
                for statement in statements {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, poll)?;
                }
                if self.current_block_terminated() {
                    continue;
                }
                if site.final_in_finally {
                    let mode = self
                        .builder
                        .build_load(self.context.i32_type(), mode_slot, "async.finally.mode")
                        .unwrap()
                        .into_int_value();
                    let has_failure = self
                        .builder
                        .build_int_compare(
                            inkwell::IntPredicate::EQ,
                            mode,
                            self.context.i32_type().const_int(1, false),
                            "async.finally.has.failure",
                        )
                        .unwrap();
                    let propagate_pending = self
                        .context
                        .append_basic_block(poll, "async.finally.propagate.pending");
                    let completed = group.completion.as_ref().map(|_| {
                        self.context
                            .append_basic_block(poll, "async.switch.finally.completion.dispatch")
                    });
                    self.builder
                        .build_conditional_branch(
                            has_failure,
                            propagate_pending,
                            completed.unwrap_or(recovered),
                        )
                        .unwrap();
                    self.builder.position_at_end(propagate_pending);
                    let pending_failure = self
                        .builder
                        .build_load(pointer, failure_slot, "async.finally.pending.failure")
                        .unwrap()
                        .into_pointer_value();
                    self.builder
                        .build_store(failure_slot, pointer.const_null())
                        .unwrap();
                    let routed = self.emit_async_route_rejection_outward(
                        group.recursive_try_id,
                        group.outer_index,
                        pending_failure,
                        await_sites,
                        catch_await_groups,
                        finally_await_groups,
                        blocks,
                        poll,
                        frame_type,
                        frame,
                        state_pointer,
                        async_result,
                    )?;
                    if !routed {
                        self.builder
                            .build_call(
                                propagate,
                                &[pending_failure.into(), async_result.into()],
                                "async.finally.propagate.pending",
                            )
                            .unwrap();
                        self.builder
                            .build_call(promise_free, &[pending_failure.into()], "")
                            .unwrap();
                        self.builder
                            .build_return(Some(&self.context.i32_type().const_int(1, false)))
                            .unwrap();
                    }
                    if let (Some(completion), Some(completed)) = (&group.completion, completed) {
                        self.builder.position_at_end(completed);
                        let (completion_mode_slot, _) = self
                            .async_frame_slot(&completion.mode_storage)
                            .ok_or_else(|| {
                                "internal: async switch finally completion mode has no frame slot"
                                    .to_string()
                            })?;
                        let completion_mode = self
                            .builder
                            .build_load(
                                self.context.i32_type(),
                                completion_mode_slot,
                                "async.switch.finally.completion.mode",
                            )
                            .unwrap()
                            .into_int_value();
                        let return_block = self
                            .context
                            .append_basic_block(poll, "async.switch.finally.completion.return");
                        let transfer_block = self
                            .context
                            .append_basic_block(poll, "async.switch.finally.completion.transfer");
                        let is_return = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::EQ,
                                completion_mode,
                                self.context.i32_type().const_int(2, false),
                                "async.switch.finally.completion.is_return",
                            )
                            .unwrap();
                        self.builder
                            .build_conditional_branch(is_return, return_block, transfer_block)
                            .unwrap();
                        self.builder.position_at_end(return_block);
                        if let Some((return_slot, return_type)) = self
                            .async_frame_slot(&completion.return_storage)
                            .map(|(slot, ty)| (slot, self.resolve_type(&ty)))
                        {
                            let value = self
                                .builder
                                .build_load(
                                    self.llvm_type(&return_type),
                                    return_slot,
                                    "async.switch.finally.return.value",
                                )
                                .unwrap();
                            self.builder
                                .build_store(return_slot, self.llvm_type(&return_type).const_zero())
                                .unwrap();
                            self.mark_async_frame_value_initialized(
                                &completion.return_storage,
                                false,
                            );
                            self.emit_async_transferred_value_ready(
                                async_result,
                                value,
                                &return_type,
                            );
                        } else {
                            self.emit_async_void_ready(async_result);
                        }
                        self.builder.position_at_end(transfer_block);
                        let break_dispatch = self
                            .context
                            .append_basic_block(poll, "async.switch.finally.completion.break");
                        let continue_dispatch = self
                            .context
                            .append_basic_block(poll, "async.switch.finally.completion.continue");
                        let throw_dispatch = self
                            .context
                            .append_basic_block(poll, "async.switch.finally.completion.throw");
                        let is_break = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::EQ,
                                completion_mode,
                                self.context.i32_type().const_int(3, false),
                                "async.switch.finally.completion.is_break",
                            )
                            .unwrap();
                        self.builder
                            .build_conditional_branch(is_break, break_dispatch, continue_dispatch)
                            .unwrap();
                        let completion_blocks = blocks
                            .finally_completion_blocks
                            .get(&group.mode_storage)
                            .copied()
                            .ok_or_else(|| {
                                "internal: async switch finally completion blocks are missing"
                                    .to_string()
                            })?;
                        self.builder.position_at_end(break_dispatch);
                        self.builder
                            .build_unconditional_branch(
                                completion_blocks
                                    .break_target
                                    .unwrap_or(completion_blocks.normal),
                            )
                            .unwrap();
                        self.builder.position_at_end(continue_dispatch);
                        let is_continue = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::EQ,
                                completion_mode,
                                self.context.i32_type().const_int(4, false),
                                "async.switch.finally.completion.is_continue",
                            )
                            .unwrap();
                        self.builder
                            .build_conditional_branch(
                                is_continue,
                                completion_blocks
                                    .continue_target
                                    .unwrap_or(completion_blocks.normal),
                                throw_dispatch,
                            )
                            .unwrap();
                        self.builder.position_at_end(throw_dispatch);
                        let is_throw = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::EQ,
                                completion_mode,
                                self.context.i32_type().const_int(5, false),
                                "async.switch.finally.completion.is_throw",
                            )
                            .unwrap();
                        let throw_value = self.context.append_basic_block(
                            poll,
                            "async.switch.finally.completion.throw.value",
                        );
                        self.builder
                            .build_conditional_branch(
                                is_throw,
                                throw_value,
                                completion_blocks.normal,
                            )
                            .unwrap();
                        self.builder.position_at_end(throw_value);
                        let (throw_slot, _) = self
                            .async_frame_slot(&completion.throw_storage)
                            .ok_or_else(|| {
                                "internal: async switch finally completion throw has no frame slot"
                                    .to_string()
                            })?;
                        let message = self
                            .builder
                            .build_load(
                                self.string_abi_type(),
                                throw_slot,
                                "async.switch.finally.throw.value",
                            )
                            .unwrap();
                        let reject = self.declare_libc_fn(
                            "tyrus_promise_reject_pending",
                            &[pointer.into(), self.string_abi_type()],
                            Some(self.context.i32_type().into()),
                        );
                        self.builder
                            .build_call(
                                reject,
                                &[async_result.into(), message.into()],
                                "async.switch.finally.throw.reject",
                            )
                            .unwrap();
                        self.emit_string_release(message.into_struct_value())?;
                        self.builder
                            .build_store(throw_slot, self.null_string())
                            .unwrap();
                        self.mark_async_frame_value_initialized(&completion.throw_storage, false);
                        self.builder
                            .build_return(Some(&self.context.i32_type().const_int(1, false)))
                            .unwrap();
                    }
                } else {
                    let preceding = finally_await_groups
                        .iter()
                        .take_while(|candidate| candidate.mode_storage != group.mode_storage)
                        .map(|candidate| candidate.sites.len())
                        .sum::<usize>();
                    let next_state =
                        await_sites.len() + catch_resume_blocks.len() + preceding + site_index + 2;
                    self.emit_async_suspend_dependency(
                        poll,
                        frame_type,
                        frame,
                        state_pointer,
                        next_state as u64,
                        &group.sites[site_index + 1].dependency,
                    )?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn emit_async_recursive_try_recovery(
        &mut self,
        depth: usize,
        recovery: &AsyncTryRecovery<'_, 'ctx>,
    ) -> Result<(), String> {
        let AsyncTryRecovery {
            await_site,
            await_sites,
            catch_await_groups,
            finally_await_groups,
            recursive_control_blocks,
            poll,
            frame_type,
            frame,
            state_pointer,
            async_result,
            dependency,
            dependency_pointer,
            promise_free,
            cancelled_flag,
        } = *recovery;
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let contexts = control_flow_contexts(await_site)
            .ok_or_else(|| "internal: recursive async contexts are missing".to_string())?;
        let context = contexts
            .get(depth)
            .ok_or_else(|| "internal: recursive async try context is missing".to_string())?;
        let AsyncControlContext::Try {
            try_id,
            catch_binding,
            catch_type,
            catch_body,
            finally_body,
            ..
        } = context
        else {
            unreachable!()
        };
        let recursive_finally = finally_await_groups
            .iter()
            .find(|group| group.recursive_try_id == Some(*try_id));
        if let Some(group) = catch_await_groups.iter().find(|group| {
            group.outer_index == await_site.outer_index && group.recursive_try_id == Some(*try_id)
        }) {
            let (error_slot, _) = self.async_frame_slot(&group.storage_name).ok_or_else(|| {
                "internal: recursive async catch error has no frame slot".to_string()
            })?;
            let pushed_finally = !group.finally_body.is_empty()
                && !self
                    .finally_scopes
                    .last()
                    .is_some_and(|scope| Self::same_statements(&scope.body, &group.finally_body));
            if pushed_finally {
                let recursive_loop_depth = contexts[..depth]
                    .iter()
                    .filter(|context| {
                        matches!(
                            context,
                            AsyncControlContext::While { .. }
                                | AsyncControlContext::DoWhile { .. }
                                | AsyncControlContext::For { .. }
                                | AsyncControlContext::ForOf { .. }
                        )
                    })
                    .count();
                self.finally_scopes.push(FinallyScope {
                    body: group.finally_body.clone(),
                    catches_throw: true,
                    loop_depth: self.loop_stack.len() + recursive_loop_depth,
                });
            }
            self.push_scope();
            self.emit_async_recursive_rejection_filter(
                depth,
                &group.catch_type,
                &group.finally_body,
                recovery,
            )?;
            self.emit_async_error_binding(
                dependency,
                &group.catch_binding,
                &group.catch_type,
                Some(error_slot),
            )?;
            self.builder
                .build_call(promise_free, &[dependency.into()], "")
                .unwrap();
            self.builder
                .build_store(dependency_pointer, pointer.const_null())
                .unwrap();
            for statement in &group.sites[0].before {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
            self.pop_scope();
            if pushed_finally {
                self.finally_scopes.pop();
            }
            if !self.current_block_terminated() {
                let preceding = catch_await_groups
                    .iter()
                    .take_while(|candidate| candidate.storage_name != group.storage_name)
                    .map(|candidate| candidate.sites.len())
                    .sum::<usize>();
                let catch_state = await_sites.len() + preceding + 1;
                self.emit_async_suspend_dependency(
                    poll,
                    frame_type,
                    frame,
                    state_pointer,
                    catch_state as u64,
                    &group.sites[0].dependency,
                )?;
            }
            return Ok(());
        }
        let pushed_finally = !finally_body.is_empty()
            && recursive_finally.is_none()
            && !self
                .finally_scopes
                .last()
                .is_some_and(|scope| Self::same_statements(&scope.body, finally_body));
        if pushed_finally {
            let recursive_loop_depth = contexts[..depth]
                .iter()
                .filter(|context| {
                    matches!(
                        context,
                        AsyncControlContext::While { .. }
                            | AsyncControlContext::DoWhile { .. }
                            | AsyncControlContext::For { .. }
                            | AsyncControlContext::ForOf { .. }
                    )
                })
                .count();
            self.finally_scopes.push(FinallyScope {
                body: finally_body.clone(),
                catches_throw: true,
                loop_depth: self.loop_stack.len() + recursive_loop_depth,
            });
        }
        self.push_scope();
        self.emit_async_recursive_rejection_filter(depth, catch_type, finally_body, recovery)?;
        let error_cleanup =
            self.emit_async_error_binding(dependency, catch_binding, catch_type, None)?;
        self.async_error_cleanups
            .push((catch_binding.to_string(), error_cleanup.clone()));
        self.builder
            .build_call(promise_free, &[dependency.into()], "")
            .unwrap();
        self.builder
            .build_store(dependency_pointer, pointer.const_null())
            .unwrap();
        let catch_join = self.emit_async_catch_body_entry(poll, cancelled_flag)?;
        for statement in catch_body {
            self.emit_stmt(statement, poll)?;
            if self.current_block_terminated() {
                break;
            }
        }
        self.emit_async_catch_body_exit(catch_join)?;
        if !self.current_block_terminated() {
            self.emit_async_error_cleanup(&error_cleanup);
        }
        self.async_error_cleanups.pop();
        self.pop_scope();
        if pushed_finally {
            self.finally_scopes.pop();
        }
        if !self.current_block_terminated() && recursive_finally.is_none() {
            for statement in finally_body {
                self.emit_stmt(statement, poll)?;
                if self.current_block_terminated() {
                    break;
                }
            }
        }
        if !self.current_block_terminated() && recursive_finally.is_none() {
            self.emit_async_cancelled_exit(poll, cancelled_flag, async_result)?;
        }
        if !self.current_block_terminated() {
            let AsyncRecursiveControlBlocks::Try { join } = recursive_control_blocks
                .get(&(await_site.outer_index, depth))
                .copied()
                .ok_or_else(|| {
                    "internal: recursive async try recovery block is missing".to_string()
                })?
            else {
                return Err(
                    "internal: recursive async try recovery block kind is invalid".to_string(),
                );
            };
            if let Some(group) = recursive_finally {
                let preceding = finally_await_groups
                    .iter()
                    .take_while(|candidate| candidate.mode_storage != group.mode_storage)
                    .map(|candidate| candidate.sites.len())
                    .sum::<usize>();
                let state = await_sites.len()
                    + catch_await_groups
                        .iter()
                        .map(|candidate| candidate.sites.len())
                        .sum::<usize>()
                    + preceding
                    + 1;
                self.emit_async_begin_finally(
                    poll,
                    frame_type,
                    frame,
                    state_pointer,
                    state as u64,
                    group,
                    None,
                )?;
            } else {
                self.builder.build_unconditional_branch(join).unwrap();
            }
        }
        Ok(())
    }

    pub(super) fn emit_async_recursive_rejection_filter(
        &mut self,
        depth: usize,
        catch_type: &Type,
        finally_body: &[Stmt],
        recovery: &AsyncTryRecovery<'_, 'ctx>,
    ) -> Result<(), String> {
        let Some(forward) =
            self.emit_async_rejection_filter(recovery.dependency, catch_type, finally_body)?
        else {
            return Ok(());
        };
        let handle = self
            .builder
            .get_insert_block()
            .ok_or("internal: async catch has no insertion block")?;
        self.builder.position_at_end(forward);
        let try_id = control_flow_contexts(recovery.await_site).and_then(|contexts| match contexts
            .get(depth)
        {
            Some(AsyncControlContext::Try { try_id, .. }) => Some(*try_id),
            _ => None,
        });
        if let Some(pending) = try_id.and_then(|try_id| {
            self.async_pending_finally(
                recovery.await_site.outer_index,
                Some(try_id),
                recovery.await_sites,
                recovery.catch_await_groups,
                recovery.finally_await_groups,
                recovery.poll,
                recovery.frame_type,
                recovery.frame,
                recovery.state_pointer,
            )
        }) {
            pending.begin(self, recovery.dependency)?;
            self.builder.position_at_end(handle);
            return Ok(());
        }
        let outer = control_flow_contexts(recovery.await_site).and_then(|contexts| {
            contexts[..depth]
                .iter()
                .rposition(|context| matches!(context, AsyncControlContext::Try { .. }))
        });
        match outer {
            Some(outer) => {
                let saved = self.finally_scopes.clone();
                if self
                    .finally_scopes
                    .last()
                    .is_some_and(|scope| Self::same_statements(&scope.body, finally_body))
                {
                    self.finally_scopes.pop();
                }
                let result = self.emit_async_recursive_try_recovery(outer, recovery);
                self.finally_scopes = saved;
                result?;
            }
            None => self.emit_async_propagate_rejection(
                recovery.dependency,
                recovery.dependency_pointer,
                recovery.async_result,
            ),
        }
        self.builder.position_at_end(handle);
        Ok(())
    }

    pub(super) fn emit_async_forward_unhandled_rejection(
        &mut self,
        dependency: PointerValue<'ctx>,
        dependency_pointer: PointerValue<'ctx>,
        async_result: PointerValue<'ctx>,
        catch_type: &Type,
        finally_body: &[Stmt],
        pending_finally: Option<AsyncPendingFinally<'_, 'ctx>>,
    ) -> Result<(), String> {
        if let Some(forward) =
            self.emit_async_rejection_filter(dependency, catch_type, finally_body)?
        {
            let handle = self
                .builder
                .get_insert_block()
                .ok_or("internal: async catch has no insertion block")?;
            self.builder.position_at_end(forward);
            match pending_finally {
                Some(pending) => pending.begin(self, dependency)?,
                None => self.emit_async_propagate_rejection(
                    dependency,
                    dependency_pointer,
                    async_result,
                ),
            }
            self.builder.position_at_end(handle);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_async_route_rejection_outward(
        &mut self,
        try_id: Option<Span>,
        outer_index: usize,
        failure: PointerValue<'ctx>,
        await_sites: &[AsyncAwaitSite],
        catch_await_groups: &[AsyncCatchAwaitGroup],
        finally_await_groups: &[AsyncFinallyAwaitGroup],
        blocks: &AsyncBlockRegistry<'ctx>,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
        async_result: PointerValue<'ctx>,
    ) -> Result<bool, String> {
        let Some(try_id) = try_id else {
            return Ok(false);
        };
        let located = await_sites.iter().find_map(|site| {
            if site.outer_index != outer_index {
                return None;
            }
            let contexts = control_flow_contexts(site)?;
            let depth = contexts.iter().position(|context| {
                matches!(context, AsyncControlContext::Try { try_id: id, .. } if *id == try_id)
            })?;
            let outer = contexts[..depth]
                .iter()
                .rposition(|context| matches!(context, AsyncControlContext::Try { .. }))?;
            Some((site, outer))
        });
        let Some((site, outer)) = located else {
            return Ok(false);
        };
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let dependency_pointer = self
            .builder
            .build_struct_gep(frame_type, frame, 1, "async.frame.dependency")
            .unwrap();
        self.builder
            .build_store(dependency_pointer, failure)
            .unwrap();
        let cancelled_flag = self.emit_async_cancelled_flag(async_result)?;
        let promise_free = self.declare_libc_fn("tyrus_promise_free", &[pointer.into()], None);
        let recovery = AsyncTryRecovery {
            await_site: site,
            await_sites,
            catch_await_groups,
            finally_await_groups,
            recursive_control_blocks: &blocks.recursive_control_blocks,
            poll,
            frame_type,
            frame,
            state_pointer,
            async_result,
            dependency: failure,
            dependency_pointer,
            promise_free,
            cancelled_flag,
        };
        self.emit_async_recursive_try_recovery(outer, &recovery)?;
        Ok(true)
    }

    pub(super) fn emit_async_rejection_filter(
        &mut self,
        dependency: PointerValue<'ctx>,
        catch_type: &Type,
        finally_body: &[Stmt],
    ) -> Result<Option<BasicBlock<'ctx>>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i64_type = self.context.i64_type();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: async catch outside a function")?;
        let thrown_tag = self.declare_libc_fn(
            "tyrus_promise_error_thrown_tag",
            &[pointer.into()],
            Some(i64_type.into()),
        );
        let tag = self
            .builder
            .build_call(thrown_tag, &[dependency.into()], "async.catch.forward.tag")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        let resolved = self.resolve_type(catch_type);
        let variants = match &resolved {
            Type::Union(variants) => variants.clone(),
            other => vec![other.clone()],
        };
        let mut accepted = Vec::new();
        for variant in &variants {
            accepted.push(self.thrown_type_tag(variant));
            if matches!(variant, Type::Named(name) if name == "Error") {
                let mut subtypes = self
                    .interfaces
                    .keys()
                    .filter(|name| self.interface_extends_error(name))
                    .cloned()
                    .collect::<Vec<_>>();
                subtypes.sort();
                for subtype in subtypes {
                    accepted.push(self.thrown_type_tag(&Type::Named(subtype)));
                }
            }
        }
        let untyped = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                tag,
                i64_type.const_zero(),
                "async.catch.forward.untyped",
            )
            .unwrap();
        let receives_any_untyped = variants.iter().any(|variant| match variant {
            Type::Str => true,
            Type::Named(name) if name == "Error" || name == "AggregateError" => true,
            Type::Named(name) => {
                Self::native_error_code_prefix(name).is_none()
                    && self.is_structured_error_binding(variant)
            }
            _ => false,
        });
        let mut handled = if receives_any_untyped {
            untyped
        } else {
            let accepted_prefixes = variants
                .iter()
                .filter_map(|variant| match variant {
                    Type::Named(name) => Self::native_error_code_prefix(name),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let error_code = self.declare_libc_fn(
                "tyrus_promise_error_code",
                &[pointer.into()],
                Some(self.string_abi_type()),
            );
            let code = self
                .builder
                .build_call(error_code, &[dependency.into()], "async.catch.forward.code")
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap()
                .into_struct_value();
            let absent = self.string_is_null(code)?;
            let code = self
                .builder
                .build_select(
                    absent,
                    self.const_string_abi("", "async.catch.forward.code.empty")
                        .into_struct_value(),
                    code,
                    "async.catch.forward.code.or.empty",
                )
                .map_err(|error| error.to_string())?
                .into_struct_value();
            let mut other_prefixes = self
                .interfaces
                .keys()
                .filter_map(|name| Self::native_error_code_prefix(name))
                .filter(|prefix| !accepted_prefixes.contains(prefix))
                .collect::<Vec<_>>();
            other_prefixes.sort();
            other_prefixes.dedup();
            let mut foreign = self.context.bool_type().const_zero();
            for prefix in other_prefixes {
                let has_prefix = self.emit_code_has_prefix(code, &prefix);
                foreign = self
                    .builder
                    .build_or(foreign, has_prefix, "async.catch.forward.foreign")
                    .unwrap();
            }
            let keeps = self
                .builder
                .build_not(foreign, "async.catch.forward.keeps")
                .unwrap();
            self.builder
                .build_and(untyped, keeps, "async.catch.forward.untyped.kept")
                .unwrap()
        };
        for tag_value in accepted {
            let matches = self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    tag,
                    i64_type.const_int(tag_value, false),
                    "async.catch.forward.accepted",
                )
                .unwrap();
            handled = self
                .builder
                .build_or(handled, matches, "async.catch.forward.handled")
                .unwrap();
        }
        let handle = self
            .context
            .append_basic_block(function, "async.catch.handle");
        let forward = self
            .context
            .append_basic_block(function, "async.catch.forward");
        self.builder
            .build_conditional_branch(handled, handle, forward)
            .unwrap();
        self.builder.position_at_end(forward);
        if !finally_body.is_empty() && first_await_in_block(finally_body).is_none() {
            let saved = self.finally_scopes.clone();
            if self
                .finally_scopes
                .last()
                .is_some_and(|scope| Self::same_statements(&scope.body, finally_body))
            {
                self.finally_scopes.pop();
            }
            self.push_scope();
            for statement in finally_body {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, function)?;
            }
            self.pop_scope();
            self.finally_scopes = saved;
            if self.current_block_terminated() {
                self.builder.position_at_end(handle);
                return Ok(None);
            }
        }
        let forward_end = self
            .builder
            .get_insert_block()
            .ok_or("internal: async catch has no insertion block")?;
        self.builder.position_at_end(handle);
        Ok(Some(forward_end))
    }

    pub(super) fn emit_async_propagate_rejection(
        &mut self,
        dependency: PointerValue<'ctx>,
        dependency_pointer: PointerValue<'ctx>,
        async_result: PointerValue<'ctx>,
    ) {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let propagate = self.declare_libc_fn(
            "tyrus_promise_propagate_failure",
            &[pointer.into(), pointer.into()],
            Some(self.context.i32_type().into()),
        );
        self.builder
            .build_call(
                propagate,
                &[dependency.into(), async_result.into()],
                "async.catch.forward.propagate",
            )
            .unwrap();
        let promise_free = self.declare_libc_fn("tyrus_promise_free", &[pointer.into()], None);
        self.builder
            .build_call(promise_free, &[dependency.into()], "")
            .unwrap();
        self.builder
            .build_store(dependency_pointer, pointer.const_null())
            .unwrap();
        self.builder
            .build_return(Some(&self.context.i32_type().const_int(1, false)))
            .unwrap();
    }

    pub(super) fn emit_async_error_binding(
        &mut self,
        dependency: PointerValue<'ctx>,
        binding: &str,
        binding_type: &Type,
        destination: Option<PointerValue<'ctx>>,
    ) -> Result<AsyncErrorCleanup<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let resolved = self.resolve_type(binding_type);
        if matches!(&resolved, Type::Named(name) if name == "AggregateError") {
            let clone = self.declare_libc_fn(
                "tyrus_promise_clone_aggregate_error",
                &[pointer.into()],
                Some(pointer.into()),
            );
            let aggregate = self
                .builder
                .build_call(clone, &[dependency.into()], "async.aggregate.error.clone")
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap()
                .into_pointer_value();
            let value = self
                .builder
                .build_load(
                    self.llvm_type(&resolved),
                    aggregate,
                    "async.aggregate.error",
                )
                .unwrap();
            let slot = destination.unwrap_or_else(|| {
                self.builder
                    .build_alloca(self.llvm_type(&resolved), binding)
                    .unwrap()
            });
            self.builder.build_store(slot, value).unwrap();
            if destination.is_some() {
                self.mark_async_frame_slot_initialized(slot, true);
            }
            self.store_var(binding, slot, resolved);
            return Ok(AsyncErrorCleanup::Aggregate(aggregate));
        }
        if self.catch_binding_reads_thrown_cell(&resolved) {
            return self.emit_async_error_binding_from_cell(
                dependency,
                binding,
                &resolved,
                destination,
            );
        }
        let error_message = self.declare_libc_fn(
            "tyrus_promise_error_message",
            &[pointer.into()],
            Some(self.string_abi_type()),
        );
        let message = self
            .builder
            .build_call(error_message, &[dependency.into()], "async.error.message")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_struct_value();
        let owned_message = self.emit_string_clone(message, "async.error.message.clone")?;
        let structured = self.is_structured_error_binding(&resolved);
        let binds_error = matches!(&resolved, Type::Named(name) if name == "Error");
        let net_code = if structured || binds_error {
            let error_code = self.declare_libc_fn(
                "tyrus_promise_error_code",
                &[pointer.into()],
                Some(self.string_abi_type()),
            );
            let code = self
                .builder
                .build_call(error_code, &[dependency.into()], "async.error.code")
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap()
                .into_struct_value();
            let code = if binds_error {
                let absent = self.string_is_null(code)?;
                self.builder
                    .build_select(
                        absent,
                        self.const_string_abi("ERR_RUNTIME", "async.error.code.default")
                            .into_struct_value(),
                        code,
                        "async.error.code.or.default",
                    )
                    .map_err(|error| error.to_string())?
                    .into_struct_value()
            } else {
                code
            };
            Some(
                self.emit_string_clone(code, "async.error.code.clone")?
                    .into_struct_value(),
            )
        } else {
            None
        };
        let value: BasicValueEnum<'ctx> = match &resolved {
            Type::Named(name) if name == "Error" => {
                let error = self
                    .builder
                    .build_insert_value(
                        self.llvm_type(&resolved).into_struct_type().const_zero(),
                        owned_message,
                        0,
                        "async.error",
                    )
                    .map_err(|error| {
                        format!("error: failed to construct async `Error` binding: {error}")
                    })?;
                let code =
                    net_code.ok_or_else(|| "error: missing async `Error` code".to_string())?;
                self.builder
                    .build_insert_value(error, code, 1, "async.error.code")
                    .map_err(|error| error.to_string())?
                    .into_struct_value()
                    .into()
            }
            Type::Str => owned_message,
            Type::Named(_) if structured => {
                let code =
                    net_code.ok_or_else(|| "error: missing structured error code".to_string())?;
                let error_type = self.llvm_type(&resolved).into_struct_type();
                let error = self
                    .builder
                    .build_insert_value(error_type.const_zero(), code, 0, "async.error.code")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_insert_value(error, owned_message, 1, "async.error.message")
                    .map_err(|error| error.to_string())?
                    .into_struct_value()
                    .into()
            }
            _ => {
                return Err(format!(
                    "error: async catch binding `{binding}` must use `Error` until aggregate materialization is available"
                ));
            }
        };
        let slot = destination.unwrap_or_else(|| {
            self.builder
                .build_alloca(self.llvm_type(&resolved), binding)
                .unwrap()
        });
        self.builder.build_store(slot, value).unwrap();
        if destination.is_some() {
            self.mark_async_frame_slot_initialized(slot, true);
        }
        self.store_var(binding, slot, resolved.clone());
        if binds_error
            && let Some(storage) = destination
                .and_then(|slot| self.async_frame_slot_name(slot))
                .and_then(|storage_name| self.frame_error_subtype_storage(&storage_name, binding))
        {
            self.emit_async_keep_error_subtype(dependency, slot, &storage)?;
            self.catch_error_subtypes.push(storage);
        }
        if binds_error
            && destination.is_none()
            && let Some(mut storage) = self.error_subtype_storage(binding, &resolved)?
        {
            storage.scope_depth = self.vars.len();
            self.emit_async_keep_error_subtype(dependency, slot, &storage)?;
            self.catch_error_subtypes.push(storage.clone());
            return Ok(AsyncErrorCleanup::KeptError { slot, storage });
        }
        let owned_message = owned_message.into_struct_value();
        Ok(match net_code {
            Some(code) => AsyncErrorCleanup::NetError {
                code,
                message: owned_message,
            },
            None => AsyncErrorCleanup::Message(owned_message),
        })
    }

    pub(super) fn emit_async_keep_error_subtype(
        &mut self,
        dependency: PointerValue<'ctx>,
        error_slot: PointerValue<'ctx>,
        storage: &ErrorSubtypeStorage<'ctx>,
    ) -> Result<(), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i64_type = self.context.i64_type();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: async catch outside a function")?;
        let thrown_tag = self.declare_libc_fn(
            "tyrus_promise_error_thrown_tag",
            &[pointer.into()],
            Some(i64_type.into()),
        );
        let tag = self
            .builder
            .build_call(thrown_tag, &[dependency.into()], "async.catch.kept.tag")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        let thrown_data = self.declare_libc_fn(
            "tyrus_promise_error_thrown_data",
            &[pointer.into()],
            Some(pointer.into()),
        );
        let data = self
            .builder
            .build_call(thrown_data, &[dependency.into()], "async.catch.kept.data")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();
        self.builder
            .build_store(storage.tag_slot, i64_type.const_zero())
            .unwrap();
        for (name, slot) in storage.slots.clone() {
            if let Some(local) = self.async_frame_slot_name(slot) {
                self.clear_async_frame_local_before_store(
                    function,
                    &local,
                    slot,
                    &Type::Named(name),
                )?;
                self.mark_async_frame_value_initialized(&local, false);
            }
        }
        let untyped = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                tag,
                i64_type.const_zero(),
                "async.catch.kept.untyped",
            )
            .unwrap();
        let error_type = Type::Named("Error".to_string());
        let error_struct = self.llvm_type(&error_type).into_struct_type();
        let done = self
            .context
            .append_basic_block(function, "async.catch.kept.done");
        for (name, slot) in storage.slots.clone() {
            let subtype = Type::Named(name.clone());
            let subtype_tag = self.thrown_type_tag(&subtype);
            let thrown = self
                .context
                .append_basic_block(function, "async.catch.kept.thrown");
            let next = self
                .context
                .append_basic_block(function, "async.catch.kept.next");
            let is_subtype = self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    tag,
                    i64_type.const_int(subtype_tag, false),
                    "async.catch.kept.is",
                )
                .unwrap();
            self.builder
                .build_conditional_branch(is_subtype, thrown, next)
                .unwrap();
            self.builder.position_at_end(thrown);
            self.builder.build_store(storage.tag_slot, tag).unwrap();
            let value = self
                .builder
                .build_load(self.llvm_type(&subtype), data, "async.catch.kept.value")
                .unwrap();
            self.emit_async_clone_value_into(slot, value, &subtype)?;
            self.mark_async_frame_slot_initialized(slot, true);
            if let Some(code_index) = self.struct_field_index(&subtype, "code") {
                let kept = self
                    .builder
                    .build_load(self.llvm_type(&subtype), slot, "async.catch.kept.whole")
                    .unwrap()
                    .into_struct_value();
                let code = self
                    .builder
                    .build_extract_value(kept, code_index, "async.catch.kept.code")
                    .unwrap()
                    .into_struct_value();
                let code = self
                    .emit_string_clone(code, "async.catch.kept.code.clone")?
                    .into_struct_value();
                let binding_code = self
                    .builder
                    .build_struct_gep(error_struct, error_slot, 1, "async.catch.binding.code")
                    .unwrap();
                let previous = self
                    .builder
                    .build_load(
                        self.string_abi_type(),
                        binding_code,
                        "async.catch.binding.code.old",
                    )
                    .unwrap()
                    .into_struct_value();
                self.emit_string_release(previous)?;
                self.builder.build_store(binding_code, code).unwrap();
            }
            self.builder.build_unconditional_branch(done).unwrap();
            self.builder.position_at_end(next);
            let (Some(prefix), Some(code_index), Some(message_index)) = (
                Self::native_error_code_prefix(&name),
                self.struct_field_index(&subtype, "code"),
                self.struct_field_index(&subtype, "message"),
            ) else {
                continue;
            };
            if !self.is_structured_error_binding(&subtype) {
                continue;
            }
            let binding = self
                .builder
                .build_load(error_struct, error_slot, "async.catch.kept.binding")
                .unwrap()
                .into_struct_value();
            let message = self
                .builder
                .build_extract_value(binding, 0, "async.catch.kept.binding.message")
                .unwrap()
                .into_struct_value();
            let code = self
                .builder
                .build_extract_value(binding, 1, "async.catch.kept.binding.code")
                .unwrap()
                .into_struct_value();
            let has_prefix = self.emit_code_has_prefix(code, &prefix);
            let names_module = self
                .builder
                .build_and(untyped, has_prefix, "async.catch.kept.names_module")
                .unwrap();
            let shaped = self
                .context
                .append_basic_block(function, "async.catch.kept.native");
            let after = self
                .context
                .append_basic_block(function, "async.catch.kept.native.next");
            self.builder
                .build_conditional_branch(names_module, shaped, after)
                .unwrap();
            self.builder.position_at_end(shaped);
            self.builder
                .build_store(storage.tag_slot, i64_type.const_int(subtype_tag, false))
                .unwrap();
            let code = self
                .emit_string_clone(code, "async.catch.kept.native.code")?
                .into_struct_value();
            let message = self
                .emit_string_clone(message, "async.catch.kept.native.message")?
                .into_struct_value();
            let value = self
                .builder
                .build_insert_value(
                    self.llvm_type(&subtype).into_struct_type().const_zero(),
                    code,
                    code_index,
                    "async.catch.kept.native.with_code",
                )
                .map_err(|error| error.to_string())?;
            let value = self
                .builder
                .build_insert_value(
                    value,
                    message,
                    message_index,
                    "async.catch.kept.native.with_message",
                )
                .map_err(|error| error.to_string())?
                .into_struct_value();
            self.builder.build_store(slot, value).unwrap();
            self.mark_async_frame_slot_initialized(slot, true);
            self.builder.build_unconditional_branch(done).unwrap();
            self.builder.position_at_end(after);
        }
        self.builder.build_unconditional_branch(done).unwrap();
        self.builder.position_at_end(done);
        Ok(())
    }

    pub(super) fn emit_async_error_binding_from_cell(
        &mut self,
        dependency: PointerValue<'ctx>,
        binding: &str,
        resolved: &Type,
        destination: Option<PointerValue<'ctx>>,
    ) -> Result<AsyncErrorCleanup<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i64_type = self.context.i64_type();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: async catch outside a function")?;
        let slot = destination.unwrap_or_else(|| {
            self.builder
                .build_alloca(self.llvm_type(resolved), binding)
                .unwrap()
        });
        let thrown_tag = self.declare_libc_fn(
            "tyrus_promise_error_thrown_tag",
            &[pointer.into()],
            Some(i64_type.into()),
        );
        let tag = self
            .builder
            .build_call(thrown_tag, &[dependency.into()], "async.catch.tag")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        let thrown_data = self.declare_libc_fn(
            "tyrus_promise_error_thrown_data",
            &[pointer.into()],
            Some(pointer.into()),
        );
        let data = self
            .builder
            .build_call(thrown_data, &[dependency.into()], "async.catch.data")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();
        let variants = match resolved {
            Type::Union(variants) => Some(variants.clone()),
            _ => None,
        };
        let candidates = variants.clone().unwrap_or_else(|| vec![resolved.clone()]);
        let done = self
            .context
            .append_basic_block(function, "async.catch.bound");
        for (index, candidate) in candidates.iter().enumerate() {
            let matched = self
                .context
                .append_basic_block(function, "async.catch.thrown");
            let next = self
                .context
                .append_basic_block(function, "async.catch.thrown.next");
            let is_candidate = self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    tag,
                    i64_type.const_int(self.thrown_type_tag(candidate), false),
                    "async.catch.is_thrown",
                )
                .unwrap();
            self.builder
                .build_conditional_branch(is_candidate, matched, next)
                .unwrap();
            self.builder.position_at_end(matched);
            let value = self
                .builder
                .build_load(self.llvm_type(candidate), data, "async.catch.thrown.value")
                .unwrap();
            match &variants {
                Some(variants) => {
                    let copy = self
                        .builder
                        .build_alloca(self.llvm_type(candidate), "async.catch.thrown.copy")
                        .unwrap();
                    self.emit_async_clone_value_into(copy, value, candidate)?;
                    let copied = self
                        .builder
                        .build_load(self.llvm_type(candidate), copy, "async.catch.thrown.copied")
                        .unwrap();
                    self.store_union_variant(variants, slot, index as u8, copied)?;
                }
                None => self.emit_async_clone_value_into(slot, value, candidate)?,
            }
            self.builder.build_unconditional_branch(done).unwrap();
            self.builder.position_at_end(next);
        }
        let structured = candidates
            .iter()
            .enumerate()
            .filter(|(_, candidate)| self.is_structured_error_binding(candidate))
            .map(|(index, candidate)| (index, candidate.clone()))
            .collect::<Vec<_>>();
        match structured.first().cloned() {
            Some((default_index, default_candidate)) => {
                let error_message = self.declare_libc_fn(
                    "tyrus_promise_error_message",
                    &[pointer.into()],
                    Some(self.string_abi_type()),
                );
                let message = self
                    .builder
                    .build_call(error_message, &[dependency.into()], "async.catch.message")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_struct_value();
                let message = self
                    .emit_string_clone(message, "async.catch.message.clone")?
                    .into_struct_value();
                let error_code = self.declare_libc_fn(
                    "tyrus_promise_error_code",
                    &[pointer.into()],
                    Some(self.string_abi_type()),
                );
                let code = self
                    .builder
                    .build_call(error_code, &[dependency.into()], "async.catch.code")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_struct_value();
                let absent = self.string_is_null(code)?;
                let code = self
                    .builder
                    .build_select(
                        absent,
                        self.const_string_abi("ERR_RUNTIME", "async.catch.code.default")
                            .into_struct_value(),
                        code,
                        "async.catch.code.or.default",
                    )
                    .map_err(|error| error.to_string())?
                    .into_struct_value();
                let code = self
                    .emit_string_clone(code, "async.catch.code.clone")?
                    .into_struct_value();
                let mut choices = Vec::new();
                if structured.len() > 1 {
                    for (index, candidate) in &structured {
                        let Type::Named(name) = candidate else {
                            continue;
                        };
                        if let Some(prefix) = Self::native_error_code_prefix(name) {
                            let has_prefix = self.emit_code_has_prefix(code, &prefix);
                            let chosen = self
                                .context
                                .append_basic_block(function, "async.catch.shaped.named");
                            let next = self
                                .context
                                .append_basic_block(function, "async.catch.shaped.next");
                            self.builder
                                .build_conditional_branch(has_prefix, chosen, next)
                                .unwrap();
                            choices.push((chosen, *index, candidate.clone()));
                            self.builder.position_at_end(next);
                        }
                    }
                }
                let fallback = self
                    .builder
                    .get_insert_block()
                    .ok_or("internal: async catch has no insertion block")?;
                choices.push((fallback, default_index, default_candidate));
                for (block, index, candidate) in choices {
                    self.builder.position_at_end(block);
                    let shaped = self
                        .builder
                        .build_insert_value(
                            self.llvm_type(&candidate).into_struct_type().const_zero(),
                            code,
                            0,
                            "async.catch.shaped.code",
                        )
                        .map_err(|error| error.to_string())?;
                    let shaped = self
                        .builder
                        .build_insert_value(shaped, message, 1, "async.catch.shaped.message")
                        .map_err(|error| error.to_string())?
                        .into_struct_value();
                    match &variants {
                        Some(variants) => {
                            self.store_union_variant(variants, slot, index as u8, shaped.into())?;
                        }
                        None => {
                            self.builder.build_store(slot, shaped).unwrap();
                        }
                    }
                    self.builder.build_unconditional_branch(done).unwrap();
                }
            }
            None => {
                self.emit_runtime_panic(
                    "tyrus runtime error: a rejected value reached a `catch` that does not handle its type",
                );
                if !self.current_block_terminated() {
                    self.builder.build_unreachable().unwrap();
                }
            }
        }
        self.builder.position_at_end(done);
        if destination.is_some() {
            self.mark_async_frame_slot_initialized(slot, true);
        }
        self.store_var(binding, slot, resolved.clone());
        Ok(AsyncErrorCleanup::Value {
            slot,
            ty: resolved.clone(),
        })
    }

    pub(crate) fn emit_async_error_cleanup(&mut self, cleanup: &AsyncErrorCleanup<'ctx>) {
        if self.current_block_terminated() {
            return;
        }
        match cleanup {
            AsyncErrorCleanup::Message(message) => {
                self.emit_string_release(*message)
                    .expect("releasing a caught message is always emittable");
            }
            AsyncErrorCleanup::NetError { code, message } => {
                self.emit_string_release(*code)
                    .expect("releasing a caught error code is always emittable");
                self.emit_string_release(*message)
                    .expect("releasing a caught message is always emittable");
            }
            AsyncErrorCleanup::Aggregate(aggregate) => {
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let drop = self.declare_libc_fn(
                    "tyrus_drop_async_aggregate_error",
                    &[pointer.into()],
                    None,
                );
                self.builder
                    .build_call(drop, &[(*aggregate).into()], "")
                    .unwrap();
            }
            AsyncErrorCleanup::Value { slot, ty } => {
                let function = self
                    .builder
                    .get_insert_block()
                    .and_then(inkwell::basic_block::BasicBlock::get_parent)
                    .expect("a caught value is released inside a function");
                let ty = ty.clone();
                self.emit_async_drop_value_in_place(function, *slot, &ty)
                    .expect("releasing a caught value is always emittable");
            }
            AsyncErrorCleanup::KeptError { slot, storage } => {
                let function = self
                    .builder
                    .get_insert_block()
                    .and_then(inkwell::basic_block::BasicBlock::get_parent)
                    .expect("a caught error is released inside a function");
                self.emit_async_drop_value_in_place(
                    function,
                    *slot,
                    &Type::Named("Error".to_string()),
                )
                .expect("releasing a caught error is always emittable");
                let storage = storage.clone();
                self.emit_error_subtype_storage_release(function, &storage, None)
                    .expect("releasing a kept error subtype is always emittable");
            }
        }
    }

    pub(crate) fn emit_active_async_catch_cleanup(
        &mut self,
        poll: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let Some(scope) = self.async_catch_cleanup_scopes.last().cloned() else {
            return Ok(());
        };
        self.clear_async_frame_local_before_store(
            poll,
            &scope.storage_name,
            scope.slot,
            &scope.ty,
        )?;
        self.builder
            .build_store(scope.slot, self.llvm_type(&scope.ty).const_zero())
            .unwrap();
        Ok(())
    }
}
