use super::*;

#[derive(Clone)]
pub(crate) struct AsyncThrowRegion {
    try_span: Span,
    handlers: Vec<Span>,
    state: u32,
    catch_type: Type,
    route: Option<AsyncThrowRoute>,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum AsyncThrowRoute {
    Outward {
        try_id: Span,
        outer_index: usize,
    },
    Finally {
        outer_index: usize,
        try_id: Option<Span>,
    },
}

pub(crate) struct AsyncThrowRegions<'ctx> {
    poll: FunctionValue<'ctx>,
    regions: Vec<AsyncThrowRegion>,
}

fn covers(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn statements_span(body: &[Stmt]) -> Option<Span> {
    let first = body
        .iter()
        .find(|statement| statement.span != Span::DUMMY)?;
    let last = body
        .iter()
        .rev()
        .find(|statement| statement.span != Span::DUMMY)?;
    Some(Span::new(first.span.start, last.span.end))
}

fn innermost_uncovered_try<'a>(
    body: &'a [Stmt],
    site: &'a AsyncAwaitSite,
    catch_await_groups: &[AsyncCatchAwaitGroup],
) -> Option<(Span, &'a Type, &'a [Stmt], &'a [Stmt])> {
    if let Some(contexts) = control_flow_contexts(site)
        && let Some(AsyncControlContext::Try {
            try_id,
            catch_type,
            catch_body,
            finally_body,
            ..
        }) = contexts
            .iter()
            .rev()
            .find(|context| matches!(context, AsyncControlContext::Try { .. }))
    {
        return Some((*try_id, catch_type, catch_body, finally_body));
    }
    let top_level = matches!(
        &site.kind,
        AsyncAwaitKind::IfBranch {
            try_context: Some(_),
            ..
        } | AsyncAwaitKind::IfBothBranches {
            try_context: Some(_),
            ..
        } | AsyncAwaitKind::Switch {
            context: Some(AsyncSwitchContext::Try { .. }),
            ..
        }
    ) || (matches!(site.kind, AsyncAwaitKind::Try { .. })
        && catch_await_groups.iter().any(|group| {
            group.outer_index == site.outer_index && group.recursive_try_depth.is_none()
        }));
    if !top_level {
        return None;
    }
    let statement = body.get(site.outer_index)?;
    let StmtKind::TryCatch {
        catch_type,
        catch_body,
        finally_body,
        ..
    } = statement.kind()
    else {
        return None;
    };
    Some((
        statement.span,
        catch_type,
        catch_body,
        finally_body.as_deref().unwrap_or_default(),
    ))
}

impl<'ctx> Codegen<'ctx> {
    pub(super) fn collect_async_throw_regions(
        &self,
        body: &[Stmt],
        await_sites: &[AsyncAwaitSite],
        catch_await_groups: &[AsyncCatchAwaitGroup],
        finally_await_groups: &[AsyncFinallyAwaitGroup],
    ) -> Vec<AsyncThrowRegion> {
        let mut regions: Vec<AsyncThrowRegion> = Vec::new();
        for (index, site) in await_sites.iter().enumerate() {
            if let Some(contexts) = control_flow_contexts(site)
                .filter(|_| !matches!(site.kind, AsyncAwaitKind::FinallyCompletion { .. }))
            {
                for (depth, context) in contexts.iter().enumerate() {
                    let AsyncControlContext::Try {
                        try_id,
                        catch_body,
                        finally_body,
                        ..
                    } = context
                    else {
                        continue;
                    };
                    let Some(AsyncControlContext::Try {
                        catch_type: outer_catch_type,
                        ..
                    }) = contexts[..depth]
                        .iter()
                        .rev()
                        .find(|context| matches!(context, AsyncControlContext::Try { .. }))
                    else {
                        continue;
                    };
                    let route = AsyncThrowRoute::Outward {
                        try_id: *try_id,
                        outer_index: site.outer_index,
                    };
                    if regions
                        .iter()
                        .any(|region| matches!(region.route, Some(AsyncThrowRoute::Outward { try_id: id, .. }) if id == *try_id))
                    {
                        continue;
                    }
                    for handler in [catch_body.as_slice(), finally_body.as_slice()]
                        .into_iter()
                        .filter_map(statements_span)
                    {
                        regions.push(AsyncThrowRegion {
                            try_span: handler,
                            handlers: Vec::new(),
                            state: 0,
                            catch_type: outer_catch_type.clone(),
                            route: Some(route),
                        });
                    }
                }
            }
            if matches!(site.kind, AsyncAwaitKind::FinallyCompletion { .. }) {
                continue;
            }
            if matches!(
                site.kind,
                AsyncAwaitKind::SwitchSubject { .. } | AsyncAwaitKind::SwitchCaseExpression { .. }
            ) {
                continue;
            }
            let catch_suspends = catch_await_groups.iter().any(|group| {
                group.outer_index == site.outer_index && group.recursive_try_depth.is_none()
            });
            let finally_suspends = finally_await_groups.iter().any(|group| {
                group.outer_index == site.outer_index && group.recursive_try_id.is_none()
            });
            if matches!(site.kind, AsyncAwaitKind::Try { .. }) && !catch_suspends {
                if finally_suspends
                    && let Some(statement) = body.get(site.outer_index)
                    && let StmtKind::TryCatch {
                        catch_type,
                        finally_body: Some(finally_body),
                        ..
                    } = statement.kind()
                    && statement.span != Span::DUMMY
                    && !regions
                        .iter()
                        .any(|region| region.try_span == statement.span)
                {
                    regions.push(AsyncThrowRegion {
                        try_span: statement.span,
                        handlers: statements_span(finally_body).into_iter().collect(),
                        state: 0,
                        catch_type: catch_type.clone(),
                        route: Some(AsyncThrowRoute::Finally {
                            outer_index: site.outer_index,
                            try_id: None,
                        }),
                    });
                }
                continue;
            }
            let Some((try_span, catch_type, catch_body, finally_body)) =
                innermost_uncovered_try(body, site, catch_await_groups)
            else {
                continue;
            };
            if try_span == Span::DUMMY || regions.iter().any(|region| region.try_span == try_span) {
                continue;
            }
            regions.push(AsyncThrowRegion {
                try_span,
                handlers: [catch_body, finally_body]
                    .into_iter()
                    .filter_map(statements_span)
                    .collect(),
                state: (index + 1) as u32,
                catch_type: catch_type.clone(),
                route: None,
            });
        }
        regions
    }

    pub(super) fn begin_async_throw_regions(
        &mut self,
        poll: FunctionValue<'ctx>,
        regions: Vec<AsyncThrowRegion>,
    ) {
        self.async_throw_regions
            .push(AsyncThrowRegions { poll, regions });
    }

    pub(super) fn assign_async_throw_outward_states(
        &mut self,
        poll: FunctionValue<'ctx>,
        first_state: usize,
    ) -> Vec<(u32, AsyncThrowRoute)> {
        let mut routes: Vec<(u32, AsyncThrowRoute)> = Vec::new();
        let Some(active) = self
            .async_throw_regions
            .last_mut()
            .filter(|active| active.poll == poll)
        else {
            return routes;
        };
        for region in &mut active.regions {
            let Some(route) = region.route else {
                continue;
            };
            region.state = match routes.iter().find(|candidate| candidate.1 == route) {
                Some(candidate) => candidate.0,
                None => {
                    let state = (first_state + routes.len()) as u32;
                    routes.push((state, route));
                    state
                }
            };
        }
        routes
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_async_throw_outward_routes(
        &mut self,
        routes: &[(u32, AsyncThrowRoute)],
        route_blocks: &[BasicBlock<'ctx>],
        await_sites: &[AsyncAwaitSite],
        catch_await_groups: &[AsyncCatchAwaitGroup],
        finally_await_groups: &[AsyncFinallyAwaitGroup],
        blocks: &AsyncBlockRegistry<'ctx>,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
        async_result: PointerValue<'ctx>,
    ) -> Result<(), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let resume_at = self.builder.get_insert_block();
        for ((_, route), block) in routes.iter().zip(route_blocks) {
            self.builder.position_at_end(*block);
            let dependency_pointer = self
                .builder
                .build_struct_gep(frame_type, frame, 1, "async.throw.outward.dependency")
                .unwrap();
            let failure = self
                .builder
                .build_load(pointer, dependency_pointer, "async.throw.outward.failure")
                .unwrap()
                .into_pointer_value();
            let routed = match *route {
                AsyncThrowRoute::Outward {
                    try_id,
                    outer_index,
                } => self.emit_async_route_rejection_outward(
                    Some(try_id),
                    outer_index,
                    failure,
                    await_sites,
                    catch_await_groups,
                    finally_await_groups,
                    blocks,
                    poll,
                    frame_type,
                    frame,
                    state_pointer,
                    async_result,
                )?,
                AsyncThrowRoute::Finally {
                    outer_index,
                    try_id,
                } => {
                    match self.async_pending_finally(
                        outer_index,
                        try_id,
                        await_sites,
                        catch_await_groups,
                        finally_await_groups,
                        poll,
                        frame_type,
                        frame,
                        state_pointer,
                    ) {
                        Some(pending) => {
                            pending.begin(self, failure)?;
                            true
                        }
                        None => false,
                    }
                }
            };
            if !routed {
                self.emit_async_propagate_rejection(failure, dependency_pointer, async_result);
            }
        }
        if let Some(block) = resume_at {
            self.builder.position_at_end(block);
        }
        Ok(())
    }

    pub(super) fn end_async_throw_regions(&mut self) -> Option<AsyncThrowRegions<'ctx>> {
        self.async_throw_regions.pop()
    }

    fn get_or_declare_async_throw_region(&self) -> GlobalValue<'ctx> {
        if let Some(global) = self.module.get_global("tyrus.async.throw.region") {
            return global;
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let global = self
            .module
            .add_global(pointer, None, "tyrus.async.throw.region");
        global.set_initializer(&pointer.const_null());
        global.set_thread_local(true);
        global
    }

    pub(crate) fn emit_async_throw_region_marker(
        &mut self,
        span: Span,
        function: FunctionValue<'ctx>,
    ) {
        let Some(active) = self.async_throw_regions.last() else {
            return;
        };
        if active.poll != function || span == Span::DUMMY || self.current_block_terminated() {
            return;
        }
        let state = active
            .regions
            .iter()
            .filter(|region| {
                covers(region.try_span, span)
                    && !region.handlers.iter().any(|handler| covers(*handler, span))
            })
            .min_by_key(|region| region.try_span.end - region.try_span.start)
            .map_or(0, |region| region.state);
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let marker_global = self.get_or_declare_async_throw_region();
        let marker = self
            .builder
            .build_load(
                pointer,
                marker_global.as_pointer_value(),
                "async.throw.region",
            )
            .unwrap()
            .into_pointer_value();
        self.builder
            .build_store(
                marker,
                self.context.i32_type().const_int(u64::from(state), false),
            )
            .unwrap();
    }

    pub(super) fn emit_async_guarded_poll(
        &mut self,
        name: &str,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        guarded: &AsyncThrowRegions<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_type = self.context.i32_type();
        let context_type = self.catch_ctx_type();
        let wrapper = self.module.add_function(
            &format!("__tyrus_async_guarded_poll_{name}"),
            poll.get_type(),
            None,
        );
        let entry = self.context.append_basic_block(wrapper, "entry");
        let install = self.context.append_basic_block(wrapper, "guard.install");
        let run = self.context.append_basic_block(wrapper, "guard.run");
        let landing = self.context.append_basic_block(wrapper, "guard.landing");
        let uncaught = self.context.append_basic_block(wrapper, "guard.uncaught");
        let frame = wrapper.get_nth_param(0).unwrap().into_pointer_value();
        let promise = wrapper.get_nth_param(1).unwrap().into_pointer_value();

        self.builder.position_at_end(entry);
        let marker = self.builder.build_alloca(i32_type, "guard.marker").unwrap();
        self.builder
            .build_store(marker, i32_type.const_zero())
            .unwrap();
        let marker_global = self.get_or_declare_async_throw_region().as_pointer_value();
        let previous_marker = self
            .builder
            .build_load(pointer, marker_global, "guard.previous.marker")
            .unwrap()
            .into_pointer_value();
        self.builder.build_store(marker_global, marker).unwrap();
        let context = self
            .builder
            .build_alloca(context_type, "guard.context")
            .unwrap();
        let owned_slot = self
            .builder
            .build_alloca(self.context.bool_type(), "guard.owned")
            .unwrap();
        self.builder
            .build_store(owned_slot, self.context.bool_type().const_zero())
            .unwrap();
        let owned_field = self
            .builder
            .build_struct_gep(context_type, context, 3, "guard.owned.field")
            .unwrap();
        self.builder.build_store(owned_field, owned_slot).unwrap();
        let error_slot = self
            .builder
            .build_alloca(context_type, "guard.error.storage")
            .unwrap();
        let error_field = self
            .builder
            .build_struct_gep(context_type, context, 1, "guard.error.field")
            .unwrap();
        self.builder.build_store(error_field, error_slot).unwrap();
        let catch_top = self.get_or_declare_catch_top().as_pointer_value();
        let previous = self
            .builder
            .build_load(pointer, catch_top, "guard.previous")
            .unwrap()
            .into_pointer_value();
        let previous_field = self
            .builder
            .build_struct_gep(context_type, context, 2, "guard.previous.field")
            .unwrap();
        self.builder.build_store(previous_field, previous).unwrap();
        self.builder.build_unconditional_branch(install).unwrap();

        self.builder.position_at_end(install);
        self.builder.build_store(catch_top, context).unwrap();
        let jump_buffer = self
            .builder
            .build_struct_gep(context_type, context, 0, "guard.jump")
            .unwrap();
        let jumped = self
            .builder
            .build_call(self.declare_setjmp(), &[jump_buffer.into()], "guard.setjmp")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        let caught = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::NE,
                jumped,
                i32_type.const_zero(),
                "guard.caught",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(caught, landing, run)
            .unwrap();

        self.builder.position_at_end(run);
        let arguments = wrapper
            .get_param_iter()
            .map(std::convert::Into::into)
            .collect::<Vec<_>>();
        let status = self
            .builder
            .build_call(poll, &arguments, "guard.status")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap();
        self.builder.build_store(catch_top, previous).unwrap();
        self.builder
            .build_store(marker_global, previous_marker)
            .unwrap();
        self.builder.build_return(Some(&status)).unwrap();

        self.builder.position_at_end(landing);
        self.builder.build_store(catch_top, previous).unwrap();
        let tag_field = self
            .builder
            .build_struct_gep(context_type, context, 4, "guard.tag.field")
            .unwrap();
        let tag = self
            .builder
            .build_load(self.context.i64_type(), tag_field, "guard.tag")
            .unwrap()
            .into_int_value();
        let cell_field = self
            .builder
            .build_struct_gep(context_type, context, 5, "guard.cell.field")
            .unwrap();
        let cell = self
            .builder
            .build_load(pointer, cell_field, "guard.cell")
            .unwrap()
            .into_pointer_value();
        let owned = self
            .builder
            .build_load(self.context.bool_type(), owned_slot, "guard.owned.load")
            .unwrap()
            .into_int_value();
        let state = self
            .builder
            .build_load(i32_type, marker, "guard.marker.load")
            .unwrap()
            .into_int_value();
        let mut candidates = vec![Type::Str, Type::Named("Error".to_string())];
        let mut subtypes = self
            .interfaces
            .keys()
            .filter(|interface| self.interface_extends_error(interface))
            .cloned()
            .collect::<Vec<_>>();
        subtypes.sort();
        candidates.extend(subtypes.into_iter().map(Type::Named));
        candidates.extend(self.fn_throws.get(name).cloned().unwrap_or_default());
        let mut region_blocks = Vec::new();
        for region in &guarded.regions {
            candidates.push(region.catch_type.clone());
            if region_blocks
                .iter()
                .any(|(state, _)| *state == region.state)
            {
                continue;
            }
            region_blocks.push((
                region.state,
                self.context.append_basic_block(wrapper, "guard.region"),
            ));
        }
        let cases = region_blocks
            .iter()
            .map(|(state, block)| (i32_type.const_int(u64::from(*state), false), *block))
            .collect::<Vec<_>>();
        self.builder.build_switch(state, uncaught, &cases).unwrap();

        let dependency_type = frame_type
            .get_field_type_at_index(1)
            .ok_or("internal: async frame has no dependency slot")?;
        let state_type = frame_type
            .get_field_type_at_index(0)
            .ok_or("internal: async frame has no state slot")?;
        for (region_state, block) in region_blocks {
            self.builder.position_at_end(block);
            let pending = self.declare_libc_fn("tyrus_promise_pending", &[], Some(pointer.into()));
            let failure = self
                .builder
                .build_call(pending, &[], "guard.failure")
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap()
                .into_pointer_value();
            self.emit_async_reject_with_thrown_cell(
                wrapper,
                failure,
                tag,
                cell,
                owned,
                &candidates,
            )?;
            let dependency_slot = self
                .builder
                .build_struct_gep(frame_type, frame, 1, "guard.dependency")
                .unwrap();
            let stale = self
                .builder
                .build_load(dependency_type, dependency_slot, "guard.dependency.stale")
                .unwrap()
                .into_pointer_value();
            let release = self
                .context
                .append_basic_block(wrapper, "guard.dependency.release");
            let resume = self.context.append_basic_block(wrapper, "guard.resume");
            let is_null = self
                .builder
                .build_is_null(stale, "guard.dependency.empty")
                .unwrap();
            self.builder
                .build_conditional_branch(is_null, resume, release)
                .unwrap();
            self.builder.position_at_end(release);
            let promise_free = self.declare_libc_fn("tyrus_promise_free", &[pointer.into()], None);
            self.builder
                .build_call(promise_free, &[stale.into()], "")
                .unwrap();
            self.builder.build_unconditional_branch(resume).unwrap();
            self.builder.position_at_end(resume);
            self.builder.build_store(dependency_slot, failure).unwrap();
            let state_slot = self
                .builder
                .build_struct_gep(frame_type, frame, 0, "guard.state")
                .unwrap();
            self.builder
                .build_store(
                    state_slot,
                    state_type
                        .into_int_type()
                        .const_int(u64::from(region_state), false),
                )
                .unwrap();
            self.builder
                .build_store(marker, i32_type.const_zero())
                .unwrap();
            self.builder.build_unconditional_branch(install).unwrap();
        }

        self.builder.position_at_end(uncaught);
        self.emit_async_reject_with_thrown_cell(wrapper, promise, tag, cell, owned, &candidates)?;
        self.builder
            .build_store(marker_global, previous_marker)
            .unwrap();
        self.builder
            .build_return(Some(&i32_type.const_int(1, false)))
            .unwrap();
        Ok(wrapper)
    }

    fn emit_async_reject_with_thrown_cell(
        &mut self,
        function: FunctionValue<'ctx>,
        promise: PointerValue<'ctx>,
        tag: IntValue<'ctx>,
        cell: PointerValue<'ctx>,
        owned: IntValue<'ctx>,
        candidates: &[Type],
    ) -> Result<(), String> {
        const UNKNOWN: &str = "a thrown value reached no `catch` that handles its type";
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let free = self.declare_libc_fn("free", &[pointer.into()], None);
        let done = self.context.append_basic_block(function, "guard.rejected");
        let mut seen = HashSet::default();
        let flattened = candidates
            .iter()
            .map(|candidate| self.resolve_type(candidate))
            .flat_map(|candidate| match candidate {
                Type::Union(variants) => variants,
                other => vec![other],
            })
            .collect::<Vec<_>>();
        for candidate in flattened {
            let kind = Self::thrown_value_kind(&candidate);
            if matches!(kind, ThrownValue::Validation)
                || !self.catch_binding_has_a_layout(&candidate)
                || !seen.insert(self.thrown_type_tag(&candidate))
            {
                continue;
            }
            let matched = self.context.append_basic_block(function, "guard.candidate");
            let next = self
                .context
                .append_basic_block(function, "guard.candidate.next");
            let is_candidate = self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    tag,
                    self.context
                        .i64_type()
                        .const_int(self.thrown_type_tag(&candidate), false),
                    "guard.is_candidate",
                )
                .unwrap();
            self.builder
                .build_conditional_branch(is_candidate, matched, next)
                .unwrap();
            self.builder.position_at_end(matched);
            let value = self
                .builder
                .build_load(self.llvm_type(&candidate), cell, "guard.value")
                .unwrap();
            self.async_results.push(promise);
            let rejection = self.async_throw_rejection(&candidate, value, kind);
            self.async_results.pop();
            self.emit_attach_thrown_cell(promise, value, &candidate)?;
            match rejection? {
                Some((_, rejection)) => self.emit_async_reject_call(promise, rejection, value)?,
                None => {
                    let message = self.const_string_abi(UNKNOWN, "guard.unknown.message");
                    self.emit_async_reject_call(promise, AsyncRejection::Message(message), value)?;
                }
            }
            self.builder.build_call(free, &[cell.into()], "").unwrap();
            let release = self.context.append_basic_block(function, "guard.release");
            self.builder
                .build_conditional_branch(owned, release, done)
                .unwrap();
            self.builder.position_at_end(release);
            match kind {
                ThrownValue::Other | ThrownValue::ErrorMessage | ThrownValue::Coded(_)
                    if value.is_struct_value() =>
                {
                    let error = value.into_struct_value();
                    for index in 0..error.get_type().count_fields() {
                        let field = self
                            .builder
                            .build_extract_value(error, index, "guard.owned.field")
                            .map_err(|error| error.to_string())?;
                        if self.is_string_value(field) {
                            self.emit_string_release(field.into_struct_value())?;
                        }
                    }
                }
                _ => {
                    if self.is_string_value(value) {
                        self.emit_string_release(value.into_struct_value())?;
                    }
                }
            }
            self.builder.build_unconditional_branch(done).unwrap();
            self.builder.position_at_end(next);
        }
        let message = self.const_string_abi(UNKNOWN, "guard.unknown.message");
        self.emit_async_reject_call(promise, AsyncRejection::Message(message), message)?;
        self.builder.build_call(free, &[cell.into()], "").unwrap();
        self.builder.build_unconditional_branch(done).unwrap();
        self.builder.position_at_end(done);
        Ok(())
    }
}
