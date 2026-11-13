use super::*;

impl<'ctx> Codegen<'ctx> {
    /// Emitting a `switch` whose body awaits.
    pub(super) fn emit_async_body_switch(
        &mut self,
        context: AsyncBody<'_, 'ctx>,
        blocks: &mut AsyncBlockRegistry<'ctx>,
    ) -> Result<bool, String> {
        let AsyncBody {
            segment_index,
            await_site,
            await_sites,
            poll,
            frame_type,
            frame,
            state_pointer,
            async_for_of_slots,
            ..
        } = context;
        if let AsyncAwaitKind::Switch {
            switch_id,
            expr,
            cases,
            default,
            default_index,
            previous,
            before,
            await_clause,
            entry_guard,
            first_in_clause,
            first_in_switch,
            ..
        } = &await_site.kind
        {
            if *first_in_switch {
                return self
                    .emit_async_switch_dispatch(
                        await_site.outer_index,
                        *switch_id,
                        expr,
                        cases,
                        default,
                        *default_index,
                        await_sites,
                        blocks,
                        poll,
                        frame_type,
                        frame,
                        state_pointer,
                        async_for_of_slots,
                    )
                    .map(|()| true);
            }
            if *first_in_clause {
                return Err(
                    "internal: async switch clause entry was emitted more than once".to_string(),
                );
            }
            let end = blocks
                .switch_blocks
                .get(switch_id)
                .map(|switch| switch.end)
                .ok_or_else(|| {
                    "internal: async switch continuation was not initialized".to_string()
                })?;
            let context_control =
                self.push_async_switch_context_control(await_site.outer_index, *switch_id, blocks);
            self.push_break_target(end);
            self.push_scope();
            self.bind_async_switch_previous_locals(previous);
            self.bind_async_switch_guard_for_of(entry_guard.as_ref())?;
            let nested_loop = entry_guard
                .as_ref()
                .filter(|guard| guard.repeat)
                .and_then(|_| {
                    blocks
                        .switch_clause_loop_blocks
                        .get(&(*switch_id, *await_clause))
                        .copied()
                });
            if let Some((condition, bypass)) = nested_loop {
                self.push_loop_control(condition, bypass);
            }
            for statement in before {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
            if nested_loop.is_some() {
                self.pop_loop_control();
            }
            if !self.current_block_terminated() {
                self.emit_async_suspend_dependency(
                    poll,
                    frame_type,
                    frame,
                    state_pointer,
                    (segment_index + 1) as u64,
                    &await_site.dependency,
                )?;
            }
            self.pop_scope();
            self.pop_break_target();
            if context_control {
                self.pop_loop_control();
            }
            return Ok(true);
        }
        Ok(false)
    }
}
