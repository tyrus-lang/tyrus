use super::*;

impl<'ctx> Codegen<'ctx> {
    /// Resuming after a `finally` block has finished.
    pub(super) fn emit_async_finally_completion_segment(
        &mut self,
        segment: AsyncSegment<'_, 'ctx>,
        blocks: &mut AsyncBlockRegistry<'ctx>,
    ) -> Result<(), String> {
        let AsyncSegment {
            await_site,
            await_sites,
            catch_await_groups,
            finally_await_groups,
            poll,
            frame_type,
            frame,
            state_pointer,
            async_result,
            async_for_of_slots,
            pointer,
            ..
        } = segment;
        if let AsyncAwaitKind::FinallyCompletion {
            contexts,
            exit_depth,
            after,
            mode_storage,
            throw_storage,
            return_storage,
            final_in_finally,
            ..
        } = &await_site.kind
            && *final_in_finally
        {
            self.emit_recursive_control_statements(
                after,
                await_site.outer_index,
                contexts,
                blocks,
                poll,
            )?;
            if !self.current_block_terminated() {
                let completion = blocks
                    .finally_completion_blocks
                    .get(mode_storage)
                    .copied()
                    .ok_or_else(|| {
                        "internal: async finally completion blocks are missing".to_string()
                    })?;
                let (mode_slot, _) = self.async_frame_slot(mode_storage).ok_or_else(|| {
                    "internal: async finally completion mode has no frame slot".to_string()
                })?;
                let mode = self
                    .builder
                    .build_load(
                        self.context.i32_type(),
                        mode_slot,
                        "async.finally.completion",
                    )
                    .unwrap()
                    .into_int_value();
                let return_block = self
                    .context
                    .append_basic_block(poll, "async.finally.completion.return");
                let transfer_block = self
                    .context
                    .append_basic_block(poll, "async.finally.completion.transfer");
                let is_return = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::EQ,
                        mode,
                        self.context.i32_type().const_int(2, false),
                        "async.finally.completion.is_return",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(is_return, return_block, transfer_block)
                    .unwrap();
                self.builder.position_at_end(return_block);
                if let Some(return_storage) = return_storage {
                    let (return_slot, return_type) =
                        self.async_frame_slot(return_storage).ok_or_else(|| {
                            "internal: async finally return value has no frame slot".to_string()
                        })?;
                    let return_type = self.resolve_type(&return_type);
                    let value = self
                        .builder
                        .build_load(
                            self.llvm_type(&return_type),
                            return_slot,
                            "async.finally.return.value",
                        )
                        .unwrap();
                    self.builder
                        .build_store(return_slot, self.llvm_type(&return_type).const_zero())
                        .unwrap();
                    self.mark_async_frame_value_initialized(return_storage, false);
                    self.emit_async_transferred_value_ready(async_result, value, &return_type);
                } else {
                    self.emit_async_void_ready(async_result);
                }
                self.builder.position_at_end(transfer_block);
                let break_dispatch = self
                    .context
                    .append_basic_block(poll, "async.finally.completion.break");
                let continue_dispatch = self
                    .context
                    .append_basic_block(poll, "async.finally.completion.continue");
                let throw_dispatch = self
                    .context
                    .append_basic_block(poll, "async.finally.completion.throw");
                let is_break = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::EQ,
                        mode,
                        self.context.i32_type().const_int(3, false),
                        "async.finally.completion.is_break",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(is_break, break_dispatch, continue_dispatch)
                    .unwrap();
                self.builder.position_at_end(break_dispatch);
                self.builder
                    .build_unconditional_branch(
                        completion.break_target.unwrap_or(completion.normal),
                    )
                    .unwrap();
                self.builder.position_at_end(continue_dispatch);
                let is_continue = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::EQ,
                        mode,
                        self.context.i32_type().const_int(4, false),
                        "async.finally.completion.is_continue",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(
                        is_continue,
                        completion.continue_target.unwrap_or(completion.normal),
                        throw_dispatch,
                    )
                    .unwrap();
                self.builder.position_at_end(throw_dispatch);
                let is_throw = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::EQ,
                        mode,
                        self.context.i32_type().const_int(5, false),
                        "async.finally.completion.is_throw",
                    )
                    .unwrap();
                let throw_value = self
                    .context
                    .append_basic_block(poll, "async.finally.completion.throw.value");
                self.builder
                    .build_conditional_branch(is_throw, throw_value, completion.normal)
                    .unwrap();
                self.builder.position_at_end(throw_value);
                let (throw_slot, _) = self.async_frame_slot(throw_storage).ok_or_else(|| {
                    "internal: async finally throw value has no frame slot".to_string()
                })?;
                let message = self
                    .builder
                    .build_load(
                        self.string_abi_type(),
                        throw_slot,
                        "async.finally.throw.value",
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
                        "async.finally.throw.reject",
                    )
                    .unwrap();
                self.emit_string_release(message.into_struct_value())?;
                self.builder
                    .build_store(throw_slot, self.null_string())
                    .unwrap();
                self.mark_async_frame_value_initialized(throw_storage, false);
                self.builder
                    .build_return(Some(&self.context.i32_type().const_int(1, false)))
                    .unwrap();
                self.builder.position_at_end(completion.normal);
                if *exit_depth < contexts.len() {
                    self.finish_recursive_control_path_to(
                        await_site.outer_index,
                        contexts,
                        *exit_depth,
                        await_sites,
                        catch_await_groups,
                        finally_await_groups,
                        blocks,
                        poll,
                        frame_type,
                        frame,
                        state_pointer,
                        async_for_of_slots,
                    )?;
                }
            } else {
                let completion = blocks
                    .finally_completion_blocks
                    .get(mode_storage)
                    .copied()
                    .ok_or_else(|| {
                        "internal: async finally completion blocks are missing".to_string()
                    })?;
                self.builder.position_at_end(completion.normal);
                self.builder.build_unreachable().unwrap();
            }
        }
        Ok(())
    }
}
