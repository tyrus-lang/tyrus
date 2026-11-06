use super::*;

impl<'ctx> Codegen<'ctx> {
    /// Resuming when both branches of an `if` await.
    pub(super) fn emit_async_if_both_branches_segment(
        &mut self,
        segment: AsyncSegment<'_, 'ctx>,
        blocks: &mut AsyncBlockRegistry<'ctx>,
    ) -> Result<Option<AsyncSegmentFlow>, String> {
        let AsyncSegment {
            segment_index,
            await_site,
            await_sites,
            catch_await_groups,
            finally_await_groups,
            catch_resume_blocks,
            poll,
            frame_type,
            frame,
            state_pointer,
            async_for_of_slots,
            ..
        } = segment;
        if let AsyncAwaitKind::IfBothBranches {
            contexts,
            exit_depth,
            if_id,
            after,
            in_then,
            final_in_branch,
            try_context,
            ..
        } = &await_site.kind
        {
            let loop_targets =
                self.recursive_control_loop_targets(await_site.outer_index, contexts, blocks);
            if let Some((continue_target, after_target, finally_depth)) = loop_targets {
                self.push_loop_control_with_finally_depth(
                    continue_target,
                    after_target,
                    finally_depth,
                );
            }
            self.push_scope();
            for statement in after {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
            self.pop_scope();
            if loop_targets.is_some() {
                self.pop_loop_control();
            }
            if !final_in_branch {
                if !self.current_block_terminated() {
                    let (next_state, next_site) = await_sites
                        .iter()
                        .enumerate()
                        .skip(segment_index)
                        .find(|(_, candidate)| {
                            matches!(
                                candidate.kind,
                                AsyncAwaitKind::IfBothBranches {
                                    if_id: candidate_id,
                                    in_then: candidate_then,
                                    ..
                                } if candidate_id == *if_id && candidate_then == *in_then
                            )
                        })
                        .map(|(index, site)| (index + 1, site))
                        .ok_or_else(|| {
                            "internal: async branch continuation state is missing".to_string()
                        })?;
                    self.emit_async_suspend_dependency(
                        poll,
                        frame_type,
                        frame,
                        state_pointer,
                        next_state as u64,
                        &next_site.dependency,
                    )?;
                }
                return Ok(Some(AsyncSegmentFlow::SkipRest));
            }
            let join = blocks
                .if_branch_path_blocks
                .get(if_id)
                .copied()
                .ok_or_else(|| "internal: async if continuation was not initialized".to_string())?;
            if !self.current_block_terminated() {
                self.builder.build_unconditional_branch(join).unwrap();
            }
            if *in_then {
                return Ok(Some(AsyncSegmentFlow::SkipRest));
            }
            self.builder.position_at_end(join);
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
        Ok(None)
    }
}
