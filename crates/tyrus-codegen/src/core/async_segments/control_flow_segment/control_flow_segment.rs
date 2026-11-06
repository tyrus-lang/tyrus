use super::*;

impl<'ctx> Codegen<'ctx> {
    /// Resuming at a `break` or `continue` that crossed an await.
    pub(super) fn emit_async_control_flow_segment(
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
            async_for_of_slots,
            ..
        } = segment;
        if let AsyncAwaitKind::ControlFlow {
            contexts,
            after,
            exit_depth,
            ..
        } = &await_site.kind
        {
            self.emit_recursive_control_statements(
                after,
                await_site.outer_index,
                contexts,
                blocks,
                poll,
            )?;
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
        Ok(())
    }
}
