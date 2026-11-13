use super::*;

impl<'ctx> Codegen<'ctx> {
    /// Resuming inside the body of a loop: `while`, `do`/`while`, `for` and `for`/`of`.
    pub(super) fn emit_async_loop_body_segment(
        &mut self,
        segment: AsyncSegment<'_, 'ctx>,
        blocks: &mut AsyncBlockRegistry<'ctx>,
    ) -> Result<(), String> {
        let AsyncSegment {
            await_site,
            poll,
            async_for_of_slots,
            ..
        } = segment;
        if let AsyncAwaitKind::WhileBody {
            after,
            final_in_body,
            ..
        } = &await_site.kind
            && *final_in_body
        {
            let (condition, loop_after) = blocks
                .while_body_blocks
                .get(&await_site.outer_index)
                .copied()
                .ok_or_else(|| {
                    "internal: async while-body continuation was not initialized".to_string()
                })?;
            self.push_loop_control(condition, loop_after);
            for statement in after {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
            self.pop_loop_control();
            self.pop_scope();
            if !self.current_block_terminated() {
                self.builder.build_unconditional_branch(condition).unwrap();
            }
            self.builder.position_at_end(loop_after);
        }
        if let AsyncAwaitKind::DoWhileBody {
            cond,
            after,
            final_in_body,
            ..
        } = &await_site.kind
            && *final_in_body
        {
            let (loop_body, condition, loop_after) = blocks
                .do_while_body_blocks
                .get(&await_site.outer_index)
                .copied()
                .ok_or_else(|| {
                    "internal: async do/while-body continuation was not initialized".to_string()
                })?;
            self.push_loop_control(condition, loop_after);
            for statement in after {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
            self.pop_loop_control();
            self.pop_scope();
            if !self.current_block_terminated() {
                self.builder.build_unconditional_branch(condition).unwrap();
            }
            self.builder.position_at_end(condition);
            let condition_value = self.emit_condition_value(cond)?;
            self.builder
                .build_conditional_branch(condition_value, loop_body, loop_after)
                .unwrap();
            self.builder.position_at_end(loop_after);
        }
        if let AsyncAwaitKind::ForBody {
            update,
            after,
            final_in_body,
            ..
        } = &await_site.kind
            && *final_in_body
        {
            let (_, update_block, condition, loop_after) = blocks
                .for_body_blocks
                .get(&await_site.outer_index)
                .copied()
                .ok_or_else(|| {
                    "internal: async for-body continuation was not initialized".to_string()
                })?;
            self.push_loop_control(update_block, loop_after);
            for statement in after {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
            self.pop_loop_control();
            self.pop_scope();
            if !self.current_block_terminated() {
                self.builder
                    .build_unconditional_branch(update_block)
                    .unwrap();
            }
            self.builder.position_at_end(update_block);
            self.emit_stmt(update, poll)?;
            if !self.current_block_terminated() {
                self.builder.build_unconditional_branch(condition).unwrap();
            }
            self.builder.position_at_end(loop_after);
            self.pop_scope();
        }
        if let AsyncAwaitKind::ForOfBody {
            after,
            final_in_body,
            ..
        } = &await_site.kind
            && *final_in_body
        {
            let (condition, update, loop_after) = blocks
                .for_of_blocks
                .get(&await_site.outer_index)
                .copied()
                .ok_or_else(|| {
                    "internal: async for...of continuation was not initialized".to_string()
                })?;
            self.push_loop_control(update, loop_after);
            for statement in after {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
            self.pop_loop_control();
            self.pop_scope();
            if !self.current_block_terminated() {
                self.builder.build_unconditional_branch(update).unwrap();
            }
            self.builder.position_at_end(update);
            let (_, _, index_slot) = async_for_of_slots
                .get(&AsyncForOfSlotKey::Statement(await_site.outer_index))
                .copied()
                .ok_or_else(|| "internal: async for...of frame slots are missing".to_string())?;
            let index = self
                .builder
                .build_load(
                    self.context.i64_type(),
                    index_slot,
                    "async.for_of.update.index",
                )
                .unwrap()
                .into_int_value();
            let next = self
                .builder
                .build_int_add(
                    index,
                    self.context.i64_type().const_int(1, false),
                    "async.for_of.next.index",
                )
                .unwrap();
            self.builder.build_store(index_slot, next).unwrap();
            self.builder.build_unconditional_branch(condition).unwrap();
            self.builder.position_at_end(loop_after);
        }
        Ok(())
    }
}
