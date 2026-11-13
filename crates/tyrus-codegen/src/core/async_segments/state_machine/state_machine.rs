use super::*;

/// Everything the async state machine needs once the frame is bound.
#[derive(Clone, Copy)]
pub(super) struct AsyncStateMachine<'a, 'ctx> {
    pub(super) name: &'a str,
    pub(super) params: &'a [Param],
    pub(super) body: &'a [Stmt],
    pub(super) payload_type: &'a Type,
    pub(super) await_sites: &'a [AsyncAwaitSite],
    pub(super) catch_await_groups: &'a [AsyncCatchAwaitGroup],
    pub(super) finally_await_groups: &'a [AsyncFinallyAwaitGroup],
    pub(super) poll: FunctionValue<'ctx>,
    pub(super) frame_type: inkwell::types::StructType<'ctx>,
    pub(super) frame: PointerValue<'ctx>,
    pub(super) async_result: PointerValue<'ctx>,
    pub(super) async_expression_slots: &'a HashMap<Span, PointerValue<'ctx>>,
    pub(super) async_for_of_slots: &'a HashMap<
        AsyncForOfSlotKey,
        (PointerValue<'ctx>, PointerValue<'ctx>, PointerValue<'ctx>),
    >,
}

impl<'ctx> Codegen<'ctx> {
    /// Builds the resume dispatch and emits every segment of the async body.
    pub(super) fn emit_async_state_machine(
        &mut self,
        context: AsyncStateMachine<'_, 'ctx>,
    ) -> Result<(), String> {
        let AsyncStateMachine {
            name,
            params,
            body,
            payload_type,
            await_sites,
            catch_await_groups,
            finally_await_groups,
            poll,
            frame_type,
            frame,
            async_result,
            async_expression_slots,
            async_for_of_slots,
        } = context;
        let state_pointer = self
            .builder
            .build_struct_gep(frame_type, frame, 0, "async.frame.state")
            .unwrap();
        let state = self
            .builder
            .build_load(
                self.context.i32_type(),
                state_pointer,
                "async.frame.state.load",
            )
            .unwrap()
            .into_int_value();
        let segments: Vec<_> = (0..=await_sites.len())
            .map(|index| {
                self.context
                    .append_basic_block(poll, &format!("async.state.{index}"))
            })
            .collect();
        let mut catch_resume_blocks = Vec::new();
        for group in catch_await_groups {
            for index in 0..group.sites.len() {
                catch_resume_blocks.push(self.context.append_basic_block(
                    poll,
                    &format!("async.catch.{}.state.{index}", group.outer_index),
                ));
            }
        }
        let mut finally_resume_blocks = Vec::new();
        for group in finally_await_groups {
            for index in 0..group.sites.len() {
                finally_resume_blocks.push(self.context.append_basic_block(
                    poll,
                    &format!("async.finally.{}.state.{index}", group.outer_index),
                ));
            }
        }
        let resumable_count =
            await_sites.len() + catch_resume_blocks.len() + finally_resume_blocks.len();
        let outward_routes = self.assign_async_throw_outward_states(poll, resumable_count + 1);
        let outward_blocks = outward_routes
            .iter()
            .map(|_| self.context.append_basic_block(poll, "async.throw.outward"))
            .collect::<Vec<_>>();
        let dispatch_count = resumable_count + outward_routes.len();
        if dispatch_count == 0 {
            self.builder
                .build_unconditional_branch(segments[0])
                .unwrap();
        }
        for next_state in 1..=dispatch_count {
            let matches_state = self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    state,
                    self.context.i32_type().const_int(next_state as u64, false),
                    &format!("async.is_state.{next_state}"),
                )
                .unwrap();
            let destination = if next_state <= await_sites.len() {
                segments[next_state]
            } else if next_state <= await_sites.len() + catch_resume_blocks.len() {
                catch_resume_blocks[next_state - await_sites.len() - 1]
            } else if next_state <= resumable_count {
                finally_resume_blocks
                    [next_state - await_sites.len() - catch_resume_blocks.len() - 1]
            } else {
                outward_blocks[next_state - resumable_count - 1]
            };
            let fallback = if next_state == dispatch_count {
                segments[0]
            } else {
                self.context
                    .append_basic_block(poll, &format!("async.dispatch.{next_state}"))
            };
            self.builder
                .build_conditional_branch(matches_state, destination, fallback)
                .unwrap();
            if next_state != dispatch_count {
                self.builder.position_at_end(fallback);
            }
        }

        let mut blocks = AsyncBlockRegistry::default();
        for (site_index, site) in await_sites.iter().enumerate() {
            let guarded = match &site.kind {
                AsyncAwaitKind::WhileCondition {
                    condition: Some(condition),
                    ..
                }
                | AsyncAwaitKind::DoWhileCondition {
                    condition: Some(condition),
                    ..
                }
                | AsyncAwaitKind::ForCondition {
                    condition: Some(condition),
                    ..
                } => condition.guard.is_some(),
                _ => false,
            };
            if guarded {
                blocks.expression_guard_blocks.insert(
                    site_index,
                    self.context
                        .append_basic_block(poll, "async.loop.guard.after"),
                );
            }
        }
        for (segment_index, segment) in segments.iter().enumerate() {
            self.builder.position_at_end(*segment);
            if segment_index == 0 {
                let collects_parameter_issues =
                    params.iter().any(|param| !param.decorators.is_empty());
                if collects_parameter_issues {
                    self.emit_validation_issues_begin();
                }
                for param in params {
                    let ParamPattern::Name(param_name) = &param.pattern else {
                        continue;
                    };
                    if param.decorators.is_empty() {
                        continue;
                    }
                    let (slot, storage_type) = self.load_var(param_name).ok_or_else(|| {
                        format!("internal: async parameter `{param_name}` is missing")
                    })?;
                    let value = self
                        .builder
                        .build_load(self.llvm_type(&storage_type), slot, param_name)
                        .unwrap();
                    self.emit_parameter_validator_call(
                        name,
                        param_name,
                        value,
                        &storage_type,
                        &param.decorators,
                    )?;
                }
                if collects_parameter_issues {
                    self.emit_validation_issues_finish();
                }
            }
            if segment_index > 0
                && matches!(
                    self.emit_async_resume_segment(
                        segment_index,
                        await_sites,
                        catch_await_groups,
                        finally_await_groups,
                        &catch_resume_blocks,
                        &mut blocks,
                        poll,
                        frame_type,
                        frame,
                        state_pointer,
                        async_result,
                        payload_type,
                        async_expression_slots,
                        async_for_of_slots,
                    )?,
                    AsyncSegmentFlow::SkipRest
                )
            {
                continue;
            }

            self.emit_async_segment_body(
                segment_index,
                body,
                await_sites,
                catch_await_groups,
                finally_await_groups,
                &mut blocks,
                poll,
                frame_type,
                frame,
                state_pointer,
                async_expression_slots,
                async_for_of_slots,
            )?;
        }
        if (!catch_await_groups.is_empty() || !finally_await_groups.is_empty())
            && !self.current_block_terminated()
        {
            self.emit_owned_closure_cleanups(None)?;
            if matches!(payload_type, Type::Void) {
                self.emit_async_void_ready(async_result);
            } else {
                self.builder.build_unreachable().unwrap();
            }
        }
        self.emit_async_catch_resume(
            await_sites,
            catch_await_groups,
            finally_await_groups,
            &catch_resume_blocks,
            &blocks,
            poll,
            frame_type,
            frame,
            state_pointer,
            async_result,
        )?;
        self.emit_async_finally_resume(
            await_sites,
            catch_await_groups,
            finally_await_groups,
            &catch_resume_blocks,
            &finally_resume_blocks,
            &blocks,
            poll,
            frame_type,
            frame,
            state_pointer,
            async_result,
        )?;
        self.emit_async_throw_outward_routes(
            &outward_routes,
            &outward_blocks,
            await_sites,
            catch_await_groups,
            finally_await_groups,
            &blocks,
            poll,
            frame_type,
            frame,
            state_pointer,
            async_result,
        )?;
        Ok(())
    }
}
