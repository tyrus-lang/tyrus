use super::*;

impl<'ctx> Codegen<'ctx> {
    /// Emitting a segment whose await sits after a finally block.
    pub(super) fn emit_async_body_finally(
        &mut self,
        context: AsyncBody<'_, 'ctx>,
        blocks: &mut AsyncBlockRegistry<'ctx>,
    ) -> Result<bool, String> {
        let AsyncBody {
            await_site,
            await_sites,
            catch_await_groups,
            finally_await_groups,
            poll,
            frame_type,
            frame,
            state_pointer,
            async_for_of_slots,
            ..
        } = context;
        if let AsyncAwaitKind::FinallyCompletion {
            contexts,
            enter_depth,
            try_body,
            catch_binding,
            catch_type,
            catch_body,
            before,
            mode_storage,
            throw_storage,
            return_storage,
            first_in_finally,
            ..
        } = &await_site.kind
        {
            if *first_in_finally {
                if *enter_depth < contexts.len() {
                    self.begin_recursive_control_path_from(
                        await_site.outer_index,
                        contexts,
                        *enter_depth,
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
                let entry = self
                    .context
                    .append_basic_block(poll, "async.finally.completion.entry");
                let normal = self
                    .context
                    .append_basic_block(poll, "async.finally.completion.normal");
                let break_target = self.break_targets.last().map(|target| target.block);
                let continue_target = self.loop_stack.last().map(|(target, _, _)| *target);
                blocks.finally_completion_blocks.insert(
                    mode_storage.clone(),
                    AsyncFinallyCompletionBlocks {
                        entry,
                        normal,
                        break_target,
                        continue_target,
                    },
                );
                let (mode_slot, _) = self.async_frame_slot(mode_storage).ok_or_else(|| {
                    "internal: async finally completion mode has no frame slot".to_string()
                })?;
                let return_slot = return_storage
                    .as_ref()
                    .map(|storage| {
                        self.async_frame_slot(storage)
                            .map(|(slot, ty)| (slot, self.resolve_type(&ty)))
                            .ok_or_else(|| {
                                "internal: async finally return value has no frame slot".to_string()
                            })
                    })
                    .transpose()?;
                let (throw_slot, _) = self.async_frame_slot(throw_storage).ok_or_else(|| {
                    "internal: async finally throw value has no frame slot".to_string()
                })?;
                let loop_targets =
                    self.recursive_control_loop_targets(await_site.outer_index, contexts, blocks);
                if let Some((continue_target, break_target, finally_depth)) = loop_targets {
                    self.push_loop_control_with_finally_depth(
                        continue_target,
                        break_target,
                        finally_depth,
                    );
                }
                self.async_finally_completion_scopes
                    .push(AsyncFinallyCompletionScope {
                        mode_slot,
                        return_slot,
                        return_storage: return_storage.clone(),
                        entry,
                        throw_entry: None,
                        throw_storage: throw_storage.clone(),
                        break_target: self.break_targets.last().cloned(),
                        continue_target: self
                            .loop_stack
                            .last()
                            .map(|(target, _, depth)| (*target, *depth)),
                        throw_slot,
                        intercept_throw: false,
                    });
                self.emit_try_catch(try_body, catch_binding, catch_type, catch_body, None, poll)?;
                self.async_finally_completion_scopes.pop();
                if loop_targets.is_some() {
                    self.pop_loop_control();
                }
                if !self.current_block_terminated() {
                    self.builder
                        .build_store(mode_slot, self.context.i32_type().const_zero())
                        .unwrap();
                    self.builder.build_unconditional_branch(entry).unwrap();
                }
                self.builder.position_at_end(entry);
            }
            for statement in before {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
            if self.current_block_terminated() {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
