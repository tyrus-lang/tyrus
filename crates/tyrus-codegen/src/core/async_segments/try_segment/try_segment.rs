use super::*;

impl<'ctx> Codegen<'ctx> {
    /// Resuming inside a `try`, with its `catch` and `finally` still to run.
    pub(super) fn emit_async_try_segment(
        &mut self,
        segment: AsyncSegment<'_, 'ctx>,
        blocks: &mut AsyncBlockRegistry<'ctx>,
    ) -> Result<(), String> {
        let AsyncSegment {
            await_site,
            await_sites,
            catch_await_groups,
            finally_await_groups,
            catch_resume_blocks,
            poll,
            frame_type,
            frame,
            state_pointer,
            recovered,
            ..
        } = segment;
        if let AsyncAwaitKind::Try {
            after,
            catch_binding,
            catch_type,
            catch_body,
            finally_body,
            final_in_try,
            ..
        } = &await_site.kind
            && *final_in_try
        {
            let synchronous_catch =
                self.emit_async_synchronous_catch_entry(await_site, catch_await_groups, poll)?;
            if !finally_body.is_empty() {
                self.finally_scopes.push(FinallyScope {
                    body: finally_body.clone(),
                    catches_throw: true,
                    loop_depth: self.loop_stack.len(),
                });
            }
            for statement in after {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
            if !finally_body.is_empty() {
                self.finally_scopes.pop();
            }
            if let Some(entry) = &synchronous_catch {
                self.end_async_synchronous_catch_stretch(entry);
            }
            if !self.current_block_terminated() {
                for statement in finally_body {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, poll)?;
                }
            }
            let pending_handler = synchronous_catch.map(|entry| {
                (
                    entry,
                    catch_binding.clone(),
                    catch_type.clone(),
                    catch_body.clone(),
                    finally_body.clone(),
                )
            });
            if let Some(recovered) = blocks
                .try_blocks
                .get(&await_site.outer_index)
                .copied()
                .or(recovered)
            {
                if !self.current_block_terminated() {
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
                    } else {
                        self.builder.build_unconditional_branch(recovered).unwrap();
                    }
                }
                if let Some((entry, binding, ty, body, finally)) = &pending_handler {
                    self.emit_async_synchronous_catch_handler(
                        entry,
                        binding,
                        ty,
                        body,
                        finally,
                        await_site.outer_index,
                        blocks,
                        poll,
                    )?;
                }
                self.builder.position_at_end(recovered);
            }
        }
        Ok(())
    }
}
