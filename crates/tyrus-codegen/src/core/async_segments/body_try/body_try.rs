use super::*;

impl<'ctx> Codegen<'ctx> {
    /// Emitting a `try` whose body awaits.
    pub(super) fn emit_async_body_try(
        &mut self,
        context: AsyncBody<'_, 'ctx>,
        blocks: &mut AsyncBlockRegistry<'ctx>,
    ) -> Result<bool, String> {
        let AsyncBody {
            await_site,
            catch_await_groups,
            poll,
            ..
        } = context;
        if let AsyncAwaitKind::Try {
            before,
            catch_binding,
            catch_type,
            catch_body,
            finally_body,
            ..
        } = &await_site.kind
        {
            blocks
                .try_blocks
                .entry(await_site.outer_index)
                .or_insert_with(|| self.context.append_basic_block(poll, "async.try.recovered"));
            let synchronous_catch =
                self.emit_async_synchronous_catch_entry(await_site, catch_await_groups, poll)?;
            if !finally_body.is_empty() {
                self.finally_scopes.push(FinallyScope {
                    body: finally_body.clone(),
                    catches_throw: true,
                    loop_depth: self.loop_stack.len(),
                });
            }
            for statement in before {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
            if !finally_body.is_empty() {
                self.finally_scopes.pop();
            }
            if let Some(entry) = &synchronous_catch {
                let resume = self
                    .context
                    .append_basic_block(poll, "async.sync.try.continue");
                self.end_async_synchronous_catch_stretch(entry);
                if !self.current_block_terminated() {
                    self.builder.build_unconditional_branch(resume).unwrap();
                }
                self.emit_async_synchronous_catch_handler(
                    entry,
                    catch_binding,
                    catch_type,
                    catch_body,
                    finally_body,
                    await_site.outer_index,
                    blocks,
                    poll,
                )?;
                self.builder.position_at_end(resume);
            }
            if self.current_block_terminated() {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
