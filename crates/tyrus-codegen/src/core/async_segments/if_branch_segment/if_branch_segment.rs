use super::*;

impl<'ctx> Codegen<'ctx> {
    /// Resuming inside one branch of an `if`.
    pub(super) fn emit_async_if_branch_segment(
        &mut self,
        segment: AsyncSegment<'_, 'ctx>,
        blocks: &mut AsyncBlockRegistry<'ctx>,
    ) -> Result<(), String> {
        let AsyncSegment {
            await_site,
            await_sites,
            finally_await_groups,
            catch_resume_blocks,
            poll,
            frame_type,
            frame,
            state_pointer,
            async_for_of_slots,
            ..
        } = segment;
        if let AsyncAwaitKind::IfBranch {
            after,
            final_in_branch,
            try_context,
            control_context,
            ..
        } = &await_site.kind
            && *final_in_branch
        {
            let join = blocks
                .if_blocks
                .get(&await_site.outer_index)
                .copied()
                .ok_or_else(|| "internal: async if continuation was not initialized".to_string())?;
            if control_context.is_some() {
                let (_, condition, _, loop_after) = blocks
                    .control_body_blocks
                    .get(&await_site.outer_index)
                    .copied()
                    .ok_or_else(|| {
                        "internal: async nested control continuation was not initialized"
                            .to_string()
                    })?;
                self.push_loop_control(condition, loop_after);
            }
            for statement in after {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
            if control_context.is_some() {
                self.pop_loop_control();
            }
            self.pop_scope();
            if !self.current_block_terminated() {
                self.builder.build_unconditional_branch(join).unwrap();
            }
            self.builder.position_at_end(join);
            if let Some(context) = control_context {
                let (loop_body, continue_target, condition_block, loop_after) = blocks
                    .control_body_blocks
                    .get(&await_site.outer_index)
                    .copied()
                    .ok_or_else(|| {
                        "internal: async nested control continuation was not initialized"
                            .to_string()
                    })?;
                self.push_loop_control(continue_target, loop_after);
                let after = match context {
                    AsyncControlContext::While { after, .. }
                    | AsyncControlContext::DoWhile { after, .. }
                    | AsyncControlContext::For { after, .. }
                    | AsyncControlContext::ForOf { after, .. } => after,
                    AsyncControlContext::Block { .. }
                    | AsyncControlContext::If { .. }
                    | AsyncControlContext::Try { .. } => {
                        unreachable!("legacy async control context cannot contain blocks or ifs")
                    }
                };
                self.bind_async_frame_locals();
                for statement in after {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, poll)?;
                }
                self.pop_loop_control();
                if !self.current_block_terminated() {
                    self.builder
                        .build_unconditional_branch(continue_target)
                        .unwrap();
                }
                self.bind_async_frame_locals();
                match context {
                    AsyncControlContext::While { .. } => {}
                    AsyncControlContext::DoWhile { condition, .. } => {
                        self.builder.position_at_end(continue_target);
                        let value = self.emit_condition_value(condition)?;
                        self.builder
                            .build_conditional_branch(value, loop_body, loop_after)
                            .unwrap();
                    }
                    AsyncControlContext::For { update, .. } => {
                        self.builder.position_at_end(continue_target);
                        self.emit_stmt(update, poll)?;
                        if !self.current_block_terminated() {
                            self.builder
                                .build_unconditional_branch(condition_block)
                                .unwrap();
                        }
                    }
                    AsyncControlContext::ForOf { .. } => {
                        self.builder.position_at_end(continue_target);
                        let (_, _, index_slot) = async_for_of_slots
                            .get(&AsyncForOfSlotKey::Statement(await_site.outer_index))
                            .copied()
                            .ok_or_else(|| {
                                "internal: async nested for...of frame slots are missing"
                                    .to_string()
                            })?;
                        let index = self
                            .builder
                            .build_load(
                                self.context.i64_type(),
                                index_slot,
                                "async.control.for_of.update.index",
                            )
                            .unwrap()
                            .into_int_value();
                        let next = self
                            .builder
                            .build_int_add(
                                index,
                                self.context.i64_type().const_int(1, false),
                                "async.control.for_of.next.index",
                            )
                            .unwrap();
                        self.builder.build_store(index_slot, next).unwrap();
                        self.builder
                            .build_unconditional_branch(condition_block)
                            .unwrap();
                    }
                    AsyncControlContext::Block { .. }
                    | AsyncControlContext::If { .. }
                    | AsyncControlContext::Try { .. } => {
                        unreachable!("legacy async control context cannot contain blocks or ifs")
                    }
                }
                self.pop_scope();
                self.bind_async_frame_locals();
                self.builder.position_at_end(loop_after);
            }
            if let Some(context) = try_context
                && !self.current_block_terminated()
            {
                if let Some(finally_group) = finally_await_groups
                    .iter()
                    .find(|group| group.outer_index == await_site.outer_index)
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
                    let recovered = blocks
                        .try_blocks
                        .get(&await_site.outer_index)
                        .copied()
                        .ok_or_else(|| {
                            "internal: async finally has no recovered block".to_string()
                        })?;
                    self.builder.position_at_end(recovered);
                } else if !context.finally_body.is_empty() {
                    for statement in &context.finally_body {
                        self.emit_stmt(statement, poll)?;
                    }
                }
            }
        }
        Ok(())
    }
}
