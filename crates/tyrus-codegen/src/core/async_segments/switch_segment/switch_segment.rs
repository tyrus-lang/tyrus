use super::*;

impl<'ctx> Codegen<'ctx> {
    /// Resuming inside a `switch` case.
    pub(super) fn emit_async_switch_segment(
        &mut self,
        segment: AsyncSegment<'_, 'ctx>,
        blocks: &mut AsyncBlockRegistry<'ctx>,
    ) -> Result<Option<AsyncSegmentFlow>, String> {
        let AsyncSegment {
            await_site,
            await_sites,
            catch_await_groups,
            finally_await_groups,
            poll,
            frame_type,
            frame,
            state_pointer,
            async_for_of_slots,
            switch_context_control,
            ..
        } = segment;
        if let AsyncAwaitKind::Switch {
            switch_id,
            await_clause,
            after,
            final_in_clause,
            final_in_switch,
            entry_guard,
            context,
            ..
        } = &await_site.kind
        {
            let switch = blocks
                .switch_blocks
                .get(switch_id)
                .cloned()
                .ok_or_else(|| {
                    "internal: async switch continuation was not initialized".to_string()
                })?;
            if *final_in_clause {
                self.bind_async_switch_guard_for_of(entry_guard.as_ref())?;
                let nested_loop =
                    entry_guard
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
                for statement in after {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, poll)?;
                }
                if nested_loop.is_some() {
                    self.pop_loop_control();
                }
                if !self.current_block_terminated() {
                    let fallthrough = nested_loop
                        .map(|(condition, _)| condition)
                        .or_else(|| switch.clause_fallthrough.get(*await_clause).copied())
                        .ok_or_else(|| {
                            "internal: async switch clause continuation is missing".to_string()
                        })?;
                    self.builder
                        .build_unconditional_branch(fallthrough)
                        .unwrap();
                }
            }
            self.pop_scope();
            self.pop_break_target();
            if switch_context_control {
                self.pop_loop_control();
            }
            if *final_in_switch {
                self.builder.position_at_end(switch.end);
                self.finish_async_switch_context(
                    await_site.outer_index,
                    *switch_id,
                    context.as_ref(),
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
            } else if *final_in_clause {
                return Ok(Some(AsyncSegmentFlow::SkipRest));
            }
        }
        Ok(None)
    }
}
