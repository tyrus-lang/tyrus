use super::*;

impl<'ctx> Codegen<'ctx> {
    /// Emitting a switch case whose expression awaits.
    pub(super) fn emit_async_body_switch_case(
        &mut self,
        context: AsyncBody<'_, 'ctx>,
        blocks: &mut AsyncBlockRegistry<'ctx>,
    ) -> Result<bool, String> {
        let AsyncBody {
            await_site,
            await_sites,
            poll,
            frame_type,
            frame,
            state_pointer,
            async_expression_slots,
            async_for_of_slots,
            ..
        } = context;
        if let AsyncAwaitKind::SwitchCaseExpression {
            switch_id,
            expr,
            cases,
            default,
            default_index,
            subject_type,
            subject_storage,
            first_in_switch,
            ..
        } = &await_site.kind
        {
            if !first_in_switch {
                return Err(
                    "internal: async switch case continuation was reached before dispatch"
                        .to_string(),
                );
            }
            let subject = self.emit_expr(expr)?;
            // A borrowed place such as a parameter keeps its own copy, which the
            // frame drop releases; freeing it here as well was a double free.
            // A string temporary such as `subject.toUpperCase()` is cloned into
            // the frame too, and nothing else releases the original.
            let release_source = !tyrus_core::ast::is_borrowed_place(expr)
                && (self.async_owned_local_expression(subject_type, expr)
                    || (matches!(transparent_codegen_type(subject_type), Type::Str)
                        && self.async_owned_string_expression(expr)));
            self.store_async_switch_subject(
                poll,
                subject_storage,
                subject_type,
                subject,
                false,
                release_source,
            )?;
            return self
                .emit_async_switch_case_dispatch(
                    await_site.outer_index,
                    *switch_id,
                    subject_type,
                    subject_storage,
                    cases,
                    default,
                    *default_index,
                    0,
                    await_sites,
                    blocks,
                    poll,
                    frame_type,
                    frame,
                    state_pointer,
                    async_expression_slots,
                    async_for_of_slots,
                )
                .map(|()| true);
        }
        Ok(false)
    }
}
