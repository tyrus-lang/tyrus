use super::*;

impl<'ctx> Codegen<'ctx> {
    /// Recovering from a failed await inside an `if` branch under a `try`.
    pub(super) fn emit_async_if_branch_failure(
        &mut self,
        failure: AsyncFailure<'_, 'ctx>,
        context: &AsyncTryContext,
    ) -> Result<Option<BasicBlock<'ctx>>, String> {
        let AsyncFailure {
            await_site,
            await_sites,
            catch_await_groups,
            finally_await_groups,
            catch_resume_blocks: _,
            async_try_blocks: _,
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
            cancelled_flag: _,
        } = failure;
        Ok({
            if let Some(group) = catch_await_groups.iter().find(|group| {
                group.outer_index == await_site.outer_index && group.recursive_try_depth.is_none()
            }) {
                let (error_slot, _) = self
                    .async_frame_slot(&group.storage_name)
                    .ok_or_else(|| "internal: async catch error has no frame slot".to_string())?;
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
                return Ok(None);
            }
            if !context.finally_body.is_empty() {
                self.finally_scopes.push(FinallyScope {
                    body: context.finally_body.clone(),
                    catches_throw: true,
                    loop_depth: self.loop_stack.len(),
                });
            }
            self.push_scope();
            self.emit_async_forward_unhandled_rejection(
                dependency,
                dependency_pointer,
                async_result,
                &context.catch_type,
                &context.finally_body,
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
            let error_cleanup = self.emit_async_error_binding(
                dependency,
                &context.catch_binding,
                &context.catch_type,
                None,
            )?;
            self.async_error_cleanups
                .push((context.catch_binding.clone(), error_cleanup.clone()));
            self.builder
                .build_call(promise_free, &[dependency.into()], "")
                .unwrap();
            self.builder
                .build_store(dependency_pointer, pointer.const_null())
                .unwrap();
            for statement in &context.catch_body {
                self.emit_stmt(statement, poll)?;
            }
            if !self.current_block_terminated() {
                self.emit_async_error_cleanup(&error_cleanup);
            }
            self.async_error_cleanups.pop();
            self.pop_scope();
            if !context.finally_body.is_empty() {
                self.finally_scopes.pop();
            }
            if !self.current_block_terminated() {
                let join = match &await_site.kind {
                    AsyncAwaitKind::IfBothBranches { if_id, .. } => {
                        async_if_branch_path_blocks.get(if_id).copied()
                    }
                    _ => async_if_blocks.get(&await_site.outer_index).copied(),
                }
                .ok_or_else(|| "internal: async if continuation was not initialized".to_string())?;
                self.builder.build_unconditional_branch(join).unwrap();
            }
            None
        })
    }
}
