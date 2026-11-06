use super::*;

impl<'ctx> Codegen<'ctx> {
    /// Recovering from a failed await in a `switch` handled by a `try`.
    pub(super) fn emit_async_switch_subject_failure(
        &mut self,
        failure: AsyncFailure<'_, 'ctx>,
        catch_binding: &str,
        catch_type: &Type,
        catch_body: &[Stmt],
        finally_body: &[Stmt],
    ) -> Result<Option<BasicBlock<'ctx>>, String> {
        let AsyncFailure {
            await_site,
            await_sites,
            catch_await_groups,
            finally_await_groups,
            catch_resume_blocks,
            async_try_blocks,
            async_if_blocks: _,
            async_if_branch_path_blocks: _,
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
        } = failure;
        Ok({
            if let Some(group) = catch_await_groups.iter().find(|group| {
                group.outer_index == await_site.outer_index && group.recursive_try_depth.is_none()
            }) {
                let (error_slot, _) =
                    self.async_frame_slot(&group.storage_name).ok_or_else(|| {
                        "internal: async switch catch error has no frame slot".to_string()
                    })?;
                self.push_scope();
                self.emit_async_forward_unhandled_rejection(
                    dependency,
                    dependency_pointer,
                    async_result,
                    &group.catch_type,
                    &group.finally_body,
                    self.async_pending_finally(
                        await_site.outer_index,
                        None,
                        await_sites,
                        catch_await_groups,
                        finally_await_groups,
                        poll,
                        frame_type,
                        frame,
                        state_pointer,
                    ),
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
                if !group.finally_body.is_empty() {
                    self.finally_scopes.push(FinallyScope {
                        body: group.finally_body.clone(),
                        catches_throw: true,
                        loop_depth: self.loop_stack.len(),
                    });
                }
                for statement in &group.sites[0].before {
                    self.emit_stmt(statement, poll)?;
                }
                if !group.finally_body.is_empty() {
                    self.finally_scopes.pop();
                }
                self.pop_scope();
                if self.current_block_terminated() {
                    None
                } else {
                    let preceding_catch_sites = catch_await_groups
                        .iter()
                        .take_while(|candidate| candidate.storage_name != group.storage_name)
                        .map(|candidate| candidate.sites.len())
                        .sum::<usize>();
                    let catch_state = await_sites.len() + preceding_catch_sites + 1;
                    self.emit_async_suspend_dependency(
                        poll,
                        frame_type,
                        frame,
                        state_pointer,
                        catch_state as u64,
                        &group.sites[0].dependency,
                    )?;
                    None
                }
            } else {
                let awaiting_finally = finally_await_groups.iter().find(|group| {
                    group.outer_index == await_site.outer_index && group.recursive_try_id.is_none()
                });
                if awaiting_finally.is_none() && !finally_body.is_empty() {
                    self.finally_scopes.push(FinallyScope {
                        body: finally_body.to_vec(),
                        catches_throw: true,
                        loop_depth: self.loop_stack.len(),
                    });
                }
                self.push_scope();
                self.emit_async_forward_unhandled_rejection(
                    dependency,
                    dependency_pointer,
                    async_result,
                    catch_type,
                    finally_body,
                    self.async_pending_finally(
                        await_site.outer_index,
                        None,
                        await_sites,
                        catch_await_groups,
                        finally_await_groups,
                        poll,
                        frame_type,
                        frame,
                        state_pointer,
                    ),
                )?;
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
                }
                self.emit_async_catch_body_exit(catch_join)?;
                if !self.current_block_terminated() {
                    self.emit_async_error_cleanup(&error_cleanup);
                }
                self.async_error_cleanups.pop();
                self.pop_scope();
                if awaiting_finally.is_none() && !finally_body.is_empty() {
                    self.finally_scopes.pop();
                }
                if let Some(finally_group) = awaiting_finally {
                    if !self.current_block_terminated() {
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
                    }
                    return Ok(None);
                }
                if !self.current_block_terminated() {
                    for statement in finally_body {
                        self.emit_stmt(statement, poll)?;
                    }
                }
                if !self.current_block_terminated() {
                    self.emit_async_cancelled_exit(poll, cancelled_flag, async_result)?;
                }
                if !self.current_block_terminated() {
                    let join = async_try_blocks
                        .get(&await_site.outer_index)
                        .copied()
                        .ok_or_else(|| {
                            "internal: nested async switch try continuation is missing".to_string()
                        })?;
                    self.builder.build_unconditional_branch(join).unwrap();
                }
                None
            }
        })
    }
}
