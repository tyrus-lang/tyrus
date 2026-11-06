#[path = "body_finally/body_finally.rs"]
mod body_finally;
#[path = "body_loops/body_loops.rs"]
mod body_loops;
#[path = "body_switch/body_switch.rs"]
mod body_switch;
#[path = "body_switch_case/body_switch_case.rs"]
mod body_switch_case;
#[path = "body_try/body_try.rs"]
mod body_try;
#[path = "control_flow_segment/control_flow_segment.rs"]
mod control_flow_segment;
#[path = "finally_segment/finally_segment.rs"]
mod finally_segment;
#[path = "frame_binding/frame_binding.rs"]
mod frame_binding;
use frame_binding::AsyncFrameStorage;
#[path = "frame_locals/frame_locals.rs"]
mod frame_locals;
#[path = "if_both_segment/if_both_segment.rs"]
mod if_both_segment;
#[path = "if_branch_segment/if_branch_segment.rs"]
mod if_branch_segment;
#[path = "loop_segments/loop_segments.rs"]
mod loop_segments;
#[path = "state_machine/state_machine.rs"]
mod state_machine;
use state_machine::AsyncStateMachine;
#[path = "switch_segment/switch_segment.rs"]
mod switch_segment;
#[path = "task_wrapper/task_wrapper.rs"]
mod task_wrapper;
use task_wrapper::AsyncTaskWrapper;
#[path = "try_segment/try_segment.rs"]
mod try_segment;
use super::*;

/// Everything the tail of a resume segment shares. `emit_async_resume_segment`
/// fills it once and each `AsyncAwaitKind` handler reads what it needs, which
/// is what lets those handlers live in files of their own instead of behind a
/// parameter list nobody can hold in their head. The block registry stays out
/// of it because it is borrowed mutably; everything here is `Copy`.
#[derive(Clone, Copy)]
pub(super) struct AsyncSegment<'a, 'ctx> {
    pub(super) segment_index: usize,
    pub(super) await_site: &'a AsyncAwaitSite,
    pub(super) await_sites: &'a [AsyncAwaitSite],
    pub(super) catch_await_groups: &'a [AsyncCatchAwaitGroup],
    pub(super) finally_await_groups: &'a [AsyncFinallyAwaitGroup],
    pub(super) catch_resume_blocks: &'a [BasicBlock<'ctx>],
    pub(super) poll: FunctionValue<'ctx>,
    pub(super) frame_type: inkwell::types::StructType<'ctx>,
    pub(super) frame: PointerValue<'ctx>,
    pub(super) state_pointer: PointerValue<'ctx>,
    pub(super) async_result: PointerValue<'ctx>,
    pub(super) async_for_of_slots: &'a HashMap<
        AsyncForOfSlotKey,
        (PointerValue<'ctx>, PointerValue<'ctx>, PointerValue<'ctx>),
    >,
    pub(super) pointer: inkwell::types::PointerType<'ctx>,
    pub(super) recovered: Option<BasicBlock<'ctx>>,
    pub(super) switch_context_control: bool,
}

/// The inputs shared by the handlers that emit one segment of an async body.
/// Same idea as `AsyncSegment`, for the other half of the state machine: the
/// block registry stays out because it is borrowed mutably, everything here is
/// `Copy`.
#[derive(Clone, Copy)]
pub(super) struct AsyncBody<'a, 'ctx> {
    pub(super) segment_index: usize,
    pub(super) await_site: &'a AsyncAwaitSite,
    pub(super) await_sites: &'a [AsyncAwaitSite],
    pub(super) catch_await_groups: &'a [AsyncCatchAwaitGroup],
    pub(super) finally_await_groups: &'a [AsyncFinallyAwaitGroup],
    pub(super) poll: FunctionValue<'ctx>,
    pub(super) frame_type: inkwell::types::StructType<'ctx>,
    pub(super) frame: PointerValue<'ctx>,
    pub(super) state_pointer: PointerValue<'ctx>,
    pub(super) async_expression_slots: &'a HashMap<Span, PointerValue<'ctx>>,
    pub(super) async_for_of_slots: &'a HashMap<
        AsyncForOfSlotKey,
        (PointerValue<'ctx>, PointerValue<'ctx>, PointerValue<'ctx>),
    >,
    pub(super) pointer: inkwell::types::PointerType<'ctx>,
}

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_async_loop_condition(
        &mut self,
        condition: Option<&AsyncConditionExpr>,
        value: BasicValueEnum<'ctx>,
    ) -> Result<inkwell::values::IntValue<'ctx>, String> {
        let Some(condition) = condition else {
            return Ok(value.into_int_value());
        };
        let slot = self
            .builder
            .build_alloca(
                self.llvm_type(&condition.payload),
                "async.condition.payload",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(slot, value)
            .map_err(|error| error.to_string())?;
        self.async_await_values
            .last_mut()
            .ok_or_else(|| "internal: async condition emitted outside an async frame".to_string())?
            .insert(condition.span, (slot, condition.payload.clone()));
        let tested = self.emit_expr(&condition.expression);
        if let Some(values) = self.async_await_values.last_mut() {
            values.remove(&condition.span);
        }
        Ok(tested?.into_int_value())
    }

    pub(super) fn emit_async_suspend_dependency(
        &mut self,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
        state: u64,
        expression: &Expr,
    ) -> Result<(), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let dependency = self.emit_expr(expression)?.into_pointer_value();
        if matches!(expression.kind(), ExprKind::Ident(identifier) if self.load_var(identifier).is_some_and(|(_, ty)| async_promise_handle(&ty)))
        {
            let retain = self.declare_libc_fn(
                "tyrus_promise_retain",
                &[pointer.into()],
                Some(pointer.into()),
            );
            self.builder
                .build_call(retain, &[dependency.into()], "async.await.retain")
                .unwrap();
        }
        let dependency_pointer = self
            .builder
            .build_struct_gep(frame_type, frame, 1, "async.frame.dependency")
            .unwrap();
        self.builder
            .build_store(dependency_pointer, dependency)
            .unwrap();
        self.builder
            .build_store(
                state_pointer,
                self.context.i32_type().const_int(state, false),
            )
            .unwrap();
        let task = poll.get_nth_param(2).unwrap().into_pointer_value();
        let await_dependency = self.declare_libc_fn(
            "tyrus_async_task_await",
            &[pointer.into(), pointer.into()],
            None,
        );
        self.builder
            .build_call(
                await_dependency,
                &[task.into(), dependency.into()],
                "async.await.register",
            )
            .unwrap();
        self.builder
            .build_return(Some(&self.context.i32_type().const_zero()))
            .unwrap();
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_async_begin_finally(
        &mut self,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
        state: u64,
        group: &AsyncFinallyAwaitGroup,
        pending_failure: Option<PointerValue<'ctx>>,
    ) -> Result<(), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let (mode_slot, _) = self
            .async_frame_slot(&group.mode_storage)
            .ok_or_else(|| "internal: async finally mode has no frame slot".to_string())?;
        let (failure_slot, _) = self
            .async_frame_slot(&group.failure_storage)
            .ok_or_else(|| "internal: async finally failure has no frame slot".to_string())?;
        self.builder
            .build_store(
                mode_slot,
                self.context
                    .i32_type()
                    .const_int(u64::from(pending_failure.is_some()), false),
            )
            .unwrap();
        self.builder
            .build_store(
                failure_slot,
                pending_failure.unwrap_or_else(|| pointer.const_null()),
            )
            .unwrap();
        if pending_failure.is_some() {
            let dependency_slot = self
                .builder
                .build_struct_gep(frame_type, frame, 1, "async.frame.dependency")
                .unwrap();
            self.builder
                .build_store(dependency_slot, pointer.const_null())
                .unwrap();
        }
        for statement in &group.sites[0].before {
            if self.current_block_terminated() {
                break;
            }
            self.emit_stmt(statement, poll)?;
        }
        if !self.current_block_terminated() {
            self.emit_async_suspend_dependency(
                poll,
                frame_type,
                frame,
                state_pointer,
                state,
                &group.sites[0].dependency,
            )?;
        }
        Ok(())
    }

    pub(super) fn emit_async_catch_body_entry(
        &mut self,
        poll: FunctionValue<'ctx>,
        cancelled_flag: PointerValue<'ctx>,
    ) -> Result<BasicBlock<'ctx>, String> {
        let cancelled = self
            .builder
            .build_load(
                self.context.bool_type(),
                cancelled_flag,
                "async.cancel.flag",
            )
            .map_err(|error| error.to_string())?
            .into_int_value();
        let body = self.context.append_basic_block(poll, "async.catch.body");
        let join = self.context.append_basic_block(poll, "async.catch.join");
        self.builder
            .build_conditional_branch(cancelled, join, body)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(body);
        Ok(join)
    }

    pub(super) fn emit_async_catch_body_exit(
        &mut self,
        join: BasicBlock<'ctx>,
    ) -> Result<(), String> {
        if !self.current_block_terminated() {
            self.builder
                .build_unconditional_branch(join)
                .map_err(|error| error.to_string())?;
        }
        self.builder.position_at_end(join);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_async_resume_segment(
        &mut self,
        segment_index: usize,
        await_sites: &[AsyncAwaitSite],
        catch_await_groups: &[AsyncCatchAwaitGroup],
        finally_await_groups: &[AsyncFinallyAwaitGroup],
        catch_resume_blocks: &[BasicBlock<'ctx>],
        blocks: &mut AsyncBlockRegistry<'ctx>,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
        async_result: PointerValue<'ctx>,
        payload_type: &Type,
        async_expression_slots: &HashMap<Span, PointerValue<'ctx>>,
        async_for_of_slots: &HashMap<
            AsyncForOfSlotKey,
            (PointerValue<'ctx>, PointerValue<'ctx>, PointerValue<'ctx>),
        >,
    ) -> Result<AsyncSegmentFlow, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let await_site = &await_sites[segment_index - 1];
        let dependency_pointer = self
            .builder
            .build_struct_gep(frame_type, frame, 1, "async.frame.dependency")
            .unwrap();
        let dependency = self
            .builder
            .build_load(pointer, dependency_pointer, "async.await.dependency")
            .unwrap()
            .into_pointer_value();
        let promise_state = self.declare_libc_fn(
            "tyrus_promise_state",
            &[pointer.into()],
            Some(self.context.i32_type().into()),
        );
        let state = self
            .builder
            .build_call(promise_state, &[dependency.into()], "async.await.state")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        let fulfilled = self
            .context
            .append_basic_block(poll, "async.await.fulfilled");
        let failed = self.context.append_basic_block(poll, "async.await.failed");
        let is_fulfilled = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                state,
                self.context.i32_type().const_int(1, false),
                "async.await.is_fulfilled",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(is_fulfilled, fulfilled, failed)
            .unwrap();
        self.builder.position_at_end(failed);
        let promise_free = self.declare_libc_fn("tyrus_promise_free", &[pointer.into()], None);
        let recovered = self.emit_async_failure_recovery(
            await_site,
            await_sites,
            catch_await_groups,
            finally_await_groups,
            catch_resume_blocks,
            &blocks.try_blocks,
            &blocks.if_blocks,
            &blocks.if_branch_path_blocks,
            &blocks.recursive_control_blocks,
            poll,
            frame_type,
            frame,
            state_pointer,
            async_result,
            dependency,
            dependency_pointer,
            promise_free,
        )?;
        self.builder.position_at_end(fulfilled);
        let mut switch_context_control = false;
        if let AsyncAwaitKind::Switch {
            switch_id,
            previous,
            before,
            ..
        } = &await_site.kind
        {
            switch_context_control =
                self.push_async_switch_context_control(await_site.outer_index, *switch_id, blocks);
            let end = blocks
                .switch_blocks
                .get(switch_id)
                .map(|switch| switch.end)
                .ok_or_else(|| {
                    "internal: async switch continuation was not initialized".to_string()
                })?;
            self.push_break_target(end);
            self.push_scope();
            self.bind_async_switch_previous_locals(previous);
            self.bind_async_switch_previous_locals(before);
        }
        let returns_awaited = matches!(await_site.kind, AsyncAwaitKind::Direct { returns: true });
        let binding_type = match &await_site.kind {
            AsyncAwaitKind::Binding { ty, .. } => Some(self.resolve_type(ty)),
            AsyncAwaitKind::Assignment { name } => self
                .async_frame_slot(name)
                .map(|(_, ty)| self.resolve_type(&ty)),
            AsyncAwaitKind::Expression { ty, .. } => Some(self.resolve_type(ty)),
            AsyncAwaitKind::SwitchSubject { subject_type, .. } => {
                Some(self.resolve_type(subject_type))
            }
            AsyncAwaitKind::SwitchCaseExpression { payload_type, .. } => {
                Some(self.resolve_type(payload_type))
            }
            AsyncAwaitKind::WhileCondition { condition, .. }
            | AsyncAwaitKind::DoWhileCondition { condition, .. }
            | AsyncAwaitKind::ForCondition { condition, .. }
            | AsyncAwaitKind::IfCondition { condition, .. } => Some(
                condition
                    .as_ref()
                    .map_or(Type::Bool, |condition| condition.payload.clone()),
            ),
            AsyncAwaitKind::WhileBody { action, .. }
            | AsyncAwaitKind::DoWhileBody { action, .. }
            | AsyncAwaitKind::ForBody { action, .. }
            | AsyncAwaitKind::ForOfBody { action, .. }
            | AsyncAwaitKind::ControlFlow { action, .. }
            | AsyncAwaitKind::FinallyCompletion { action, .. }
            | AsyncAwaitKind::IfBranch { action, .. }
            | AsyncAwaitKind::Switch { action, .. }
            | AsyncAwaitKind::Try { action, .. } => match action {
                AsyncNestedAwaitAction::Discard => None,
                AsyncNestedAwaitAction::Binding { ty, .. } if !matches!(ty, Type::Infer) => {
                    Some(self.resolve_type(ty))
                }
                AsyncNestedAwaitAction::Binding { .. } => self
                    .expr_type_hint(&await_site.dependency)
                    .and_then(|promise| {
                        promise_payload_type(transparent_codegen_type(&promise)).cloned()
                    })
                    .map(|ty| self.resolve_type(&ty)),
                AsyncNestedAwaitAction::Assignment { name } => self
                    .async_frame_slot(name)
                    .map(|(_, ty)| self.resolve_type(&ty)),
            },
            AsyncAwaitKind::IfBothBranches { action, .. } => match action {
                AsyncNestedAwaitAction::Discard => None,
                AsyncNestedAwaitAction::Binding { ty, .. } if !matches!(ty, Type::Infer) => {
                    Some(self.resolve_type(ty))
                }
                AsyncNestedAwaitAction::Binding { .. } => self
                    .expr_type_hint(&await_site.dependency)
                    .and_then(|promise| {
                        promise_payload_type(transparent_codegen_type(&promise)).cloned()
                    })
                    .map(|ty| self.resolve_type(&ty)),
                AsyncNestedAwaitAction::Assignment { name } => self
                    .async_frame_slot(name)
                    .map(|(_, ty)| self.resolve_type(&ty)),
            },
            _ => None,
        };
        let await_ready = self.declare_libc_fn(
            if (returns_awaited && !matches!(payload_type, Type::Void)) || binding_type.is_some() {
                "tyrus_promise_take_value"
            } else {
                "tyrus_promise_await"
            },
            &[pointer.into()],
            Some(pointer.into()),
        );
        let awaited_payload = self
            .builder
            .build_call(await_ready, &[dependency.into()], "async.await.ready")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();
        let forwarded_db_call = match await_site.dependency.kind() {
            ExprKind::FnCall { callee, .. } => {
                self.fn_db_async_returns.contains(callee)
                    || self
                        .fn_db_async_returns
                        .contains(tyrus_core::module_loader::original_global_name(callee))
            }
            _ => false,
        };
        let awaited_db_result = self
            .expr_type_hint(&await_site.dependency)
            .and_then(|promise| match transparent_codegen_type(&promise) {
                Type::Apply { name, args }
                    if (name == "DbPromise" || (name == "Promise" && forwarded_db_call))
                        && args.len() == 1 =>
                {
                    args.first().cloned()
                }
                _ => None,
            })
            .or_else(|| {
                if !forwarded_db_call {
                    return None;
                }
                matches!(
                    binding_type.as_ref().map(transparent_codegen_type),
                    Some(Type::Result { .. })
                )
                .then(|| binding_type.clone())
                .flatten()
            })
            .map(|ty| self.resolve_type(&ty));
        if let Some(binding_type) = binding_type {
            let mut payload_type = self
                .expr_type_hint(&await_site.dependency)
                .and_then(|promise| {
                    promise_payload_type(transparent_codegen_type(&promise)).cloned()
                })
                .map_or_else(|| binding_type.clone(), |ty| self.resolve_type(&ty));
            let raw_payload_type = if awaited_db_result.is_some() {
                Type::Str
            } else {
                payload_type.clone()
            };
            let raw_value = self
                .builder
                .build_load(
                    self.llvm_type(&raw_payload_type),
                    awaited_payload,
                    "async.await.binding.payload",
                )
                .unwrap();
            let mut value = if let Some(result_type) = awaited_db_result.as_ref() {
                let raw_string = raw_value.into_struct_value();
                let decoded = self.emit_db_async_result_decode(raw_string, result_type)?;
                self.emit_string_release(raw_string)?;
                decoded
            } else {
                raw_value
            };
            let onfail_binding = match &await_site.kind {
                AsyncAwaitKind::Binding {
                    onfail: Some(onfail),
                    declared_ty: Some(declared_ty),
                    value: Some(declaration_value),
                    ..
                } => Some((onfail, declared_ty, declaration_value)),
                _ => None,
            };
            if let Some((onfail, declared_ty, declaration_value)) = onfail_binding {
                self.builder
                    .build_call(promise_free, &[dependency.into()], "")
                    .unwrap();
                self.builder
                    .build_store(dependency_pointer, pointer.const_null())
                    .unwrap();
                let free = self.declare_libc_fn("free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[awaited_payload.into()], "")
                    .unwrap();
                let result_ty = payload_type.clone();
                let result_slot =
                    self.entry_alloca(self.llvm_type(&result_ty), "async.onfail.await.result")?;
                self.builder.build_store(result_slot, value).unwrap();
                self.async_await_values
                    .last_mut()
                    .ok_or_else(|| {
                        "internal: async onfail emitted outside an async frame".to_string()
                    })?
                    .insert(declaration_value.span, (result_slot, result_ty));
                let name = match &await_site.kind {
                    AsyncAwaitKind::Binding { name, .. } => name,
                    _ => unreachable!(),
                };
                self.emit_result_onfail(
                    false,
                    name,
                    declared_ty,
                    declaration_value,
                    onfail,
                    poll,
                    true,
                )?;
                self.async_await_values
                    .last_mut()
                    .unwrap()
                    .remove(&declaration_value.span);
                if matches!(binding_type, Type::Void) {
                    value = self.context.bool_type().const_zero().into();
                } else {
                    let (slot, _) = self.async_frame_slot(name).ok_or_else(|| {
                        format!("internal: async binding `{name}` has no frame slot")
                    })?;
                    value = self
                        .builder
                        .build_load(self.llvm_type(&binding_type), slot, "async.onfail.success")
                        .unwrap();
                }
                payload_type = binding_type.clone();
            }
            let optional_inner = match transparent_codegen_type(&binding_type) {
                Type::Optional(inner)
                    if !matches!(transparent_codegen_type(&payload_type), Type::Optional(_)) =>
                {
                    Some(self.resolve_type(inner))
                }
                _ => None,
            };
            let value = if let Some(inner) = optional_inner {
                let converted = if inner == payload_type {
                    value
                } else {
                    self.coerce_value_with_source(
                        &await_site.dependency,
                        value,
                        Some(&payload_type),
                        &inner,
                    )?
                };
                self.emit_optional_box(converted, &inner)?.into()
            } else if payload_type == binding_type {
                value
            } else {
                self.coerce_value_with_source(
                    &await_site.dependency,
                    value,
                    Some(&payload_type),
                    &binding_type,
                )?
            };
            if onfail_binding.is_none() {
                self.builder
                    .build_call(promise_free, &[dependency.into()], "")
                    .unwrap();
                self.builder
                    .build_store(dependency_pointer, pointer.const_null())
                    .unwrap();
                let free = self.declare_libc_fn("free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[awaited_payload.into()], "")
                    .unwrap();
            }
            let nested_action = match &await_site.kind {
                AsyncAwaitKind::WhileBody { action, .. }
                | AsyncAwaitKind::DoWhileBody { action, .. }
                | AsyncAwaitKind::ForBody { action, .. }
                | AsyncAwaitKind::ForOfBody { action, .. }
                | AsyncAwaitKind::ControlFlow { action, .. }
                | AsyncAwaitKind::FinallyCompletion { action, .. }
                | AsyncAwaitKind::IfBranch { action, .. }
                | AsyncAwaitKind::IfBothBranches { action, .. }
                | AsyncAwaitKind::Switch { action, .. }
                | AsyncAwaitKind::Try { action, .. } => Some(action.clone()),
                _ => None,
            };
            if let Some(action) = nested_action.as_ref() {
                self.store_async_nested_await_action(poll, action, value, &binding_type)?;
            }
            match &await_site.kind {
                AsyncAwaitKind::Binding { name, onfail, .. } if onfail.is_some() => {
                    let _ = name;
                }
                AsyncAwaitKind::Binding { name, .. } => {
                    let (slot, _) = self.async_frame_slot(name).ok_or_else(|| {
                        format!("internal: async binding `{name}` has no frame slot")
                    })?;
                    self.clear_async_frame_local_before_store(poll, name, slot, &binding_type)?;
                    self.builder.build_store(slot, value).unwrap();
                    self.mark_async_frame_value_initialized(name, true);
                    self.store_var(name, slot, binding_type);
                }
                AsyncAwaitKind::Assignment { name } => {
                    let (slot, _) = self.async_frame_slot(name).ok_or_else(|| {
                        format!("internal: async assignment `{name}` has no frame slot")
                    })?;
                    self.clear_async_frame_local_before_store(poll, name, slot, &binding_type)?;
                    self.builder.build_store(slot, value).unwrap();
                    self.mark_async_frame_value_initialized(name, true);
                    if let Some(ownership) = self.async_frame_string_ownership(name) {
                        self.builder
                            .build_store(ownership, self.context.bool_type().const_int(1, false))
                            .unwrap();
                    }
                    if let Some(ownership) = self.async_frame_string_array_ownership(name) {
                        self.builder
                            .build_store(ownership, self.context.i8_type().const_int(2, false))
                            .unwrap();
                    }
                    self.store_var(name, slot, binding_type);
                }
                AsyncAwaitKind::SwitchSubject {
                    switch_id,
                    cases,
                    default,
                    default_index,
                    subject_type,
                    subject_storage,
                    context,
                } => {
                    if let Some(subject_storage) = subject_storage {
                        self.store_async_switch_subject(
                            poll,
                            subject_storage,
                            subject_type,
                            value,
                            true,
                            false,
                        )?;
                        self.emit_async_switch_case_dispatch(
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
                        )?;
                        return Ok(AsyncSegmentFlow::SkipRest);
                    }
                    let has_clause_suspension = await_sites.iter().any(|site| {
                        site.outer_index == await_site.outer_index
                            && matches!(site.kind, AsyncAwaitKind::Switch { switch_id: candidate, .. } if candidate == *switch_id)
                    });
                    self.emit_async_switch_dispatch_value(
                        await_site.outer_index,
                        *switch_id,
                        subject_type,
                        value,
                        !subject_type.is_copy(),
                        cases,
                        default,
                        *default_index,
                        await_sites,
                        blocks,
                        poll,
                        frame_type,
                        frame,
                        state_pointer,
                        async_for_of_slots,
                    )?;
                    if has_clause_suspension {
                        return Ok(AsyncSegmentFlow::SkipRest);
                    }
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
                }
                AsyncAwaitKind::SwitchCaseExpression {
                    switch_id,
                    cases,
                    default,
                    default_index,
                    subject_type,
                    subject_storage,
                    case_index,
                    span,
                    payload_type,
                    prefixes,
                    guard,
                    final_in_case_expression,
                    context,
                    ..
                } => {
                    self.emit_async_switch_case_resume(
                        await_site.outer_index,
                        *switch_id,
                        subject_type,
                        subject_storage,
                        cases,
                        default,
                        *default_index,
                        *case_index,
                        *span,
                        payload_type,
                        prefixes,
                        guard.as_ref(),
                        *final_in_case_expression,
                        value,
                        await_sites,
                        blocks,
                        poll,
                        frame_type,
                        frame,
                        state_pointer,
                        async_expression_slots,
                        async_for_of_slots,
                    )?;
                    let future_suspension = !final_in_case_expression
                        || await_sites.iter().any(|site| {
                            site.outer_index == await_site.outer_index
                                && match site.kind {
                                    AsyncAwaitKind::SwitchCaseExpression {
                                        case_index: candidate,
                                        switch_id: candidate_switch,
                                        ..
                                    } => candidate_switch == *switch_id && candidate > *case_index,
                                    AsyncAwaitKind::Switch {
                                        switch_id: candidate,
                                        ..
                                    } => candidate == *switch_id,
                                    _ => false,
                                }
                        });
                    if future_suspension {
                        return Ok(AsyncSegmentFlow::SkipRest);
                    }
                    let end = blocks
                        .switch_blocks
                        .get(switch_id)
                        .map(|switch| switch.end)
                        .ok_or_else(|| {
                            "internal: async switch continuation was not initialized".to_string()
                        })?;
                    self.builder.position_at_end(end);
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
                }
                AsyncAwaitKind::IfCondition {
                    then_block,
                    else_block,
                    condition,
                } => {
                    let tested = self.emit_async_loop_condition(condition.as_ref(), value)?;
                    let selected = self.context.append_basic_block(poll, "async.if.selected");
                    let alternative = self
                        .context
                        .append_basic_block(poll, "async.if.alternative");
                    let after = self.context.append_basic_block(poll, "async.if.after");
                    self.builder
                        .build_conditional_branch(tested, selected, alternative)
                        .unwrap();
                    for (block, statements) in [(selected, then_block), (alternative, else_block)] {
                        self.builder.position_at_end(block);
                        self.push_scope();
                        for statement in statements {
                            if self.current_block_terminated() {
                                break;
                            }
                            self.emit_stmt(statement, poll)?;
                        }
                        self.pop_scope();
                        if !self.current_block_terminated() {
                            self.builder.build_unconditional_branch(after).unwrap();
                        }
                    }
                    self.builder.position_at_end(after);
                }
                AsyncAwaitKind::WhileCondition { body, condition } => {
                    let tested = self.emit_async_loop_condition(condition.as_ref(), value)?;
                    let loop_body = self.context.append_basic_block(poll, "async.while.body");
                    let schedule = self
                        .context
                        .append_basic_block(poll, "async.while.schedule");
                    let after = condition
                        .as_ref()
                        .filter(|condition| condition.guard.is_some())
                        .and_then(|_| {
                            blocks
                                .expression_guard_blocks
                                .get(&(segment_index - 1))
                                .copied()
                        })
                        .unwrap_or_else(|| {
                            self.context.append_basic_block(poll, "async.while.after")
                        });
                    self.builder
                        .build_conditional_branch(tested, loop_body, after)
                        .unwrap();
                    self.builder.position_at_end(loop_body);
                    self.push_loop_control(schedule, after);
                    for statement in body {
                        if self.current_block_terminated() {
                            break;
                        }
                        self.emit_stmt(statement, poll)?;
                    }
                    self.pop_loop_control();
                    if !self.current_block_terminated() {
                        self.builder.build_unconditional_branch(schedule).unwrap();
                    }
                    self.builder.position_at_end(schedule);
                    if let Some(guard) = condition
                        .as_ref()
                        .and_then(|condition| condition.guard.as_ref())
                    {
                        let reschedule = self
                            .context
                            .append_basic_block(poll, "async.while.guard.schedule");
                        let guard = self.emit_condition_value(guard)?;
                        self.builder
                            .build_conditional_branch(guard, reschedule, after)
                            .unwrap();
                        self.builder.position_at_end(reschedule);
                    }
                    self.emit_async_suspend_dependency(
                        poll,
                        frame_type,
                        frame,
                        state_pointer,
                        segment_index as u64,
                        &await_site.dependency,
                    )?;
                    self.builder.position_at_end(after);
                }
                AsyncAwaitKind::DoWhileCondition { condition, .. } => {
                    let condition = self.emit_async_loop_condition(condition.as_ref(), value)?;
                    let (loop_body, _, after) = blocks
                        .loop_blocks
                        .get(&(segment_index - 1))
                        .copied()
                        .ok_or_else(|| {
                            "internal: async do/while blocks were not initialized".to_string()
                        })?;
                    self.builder
                        .build_conditional_branch(condition, loop_body, after)
                        .unwrap();
                    self.builder.position_at_end(after);
                }
                AsyncAwaitKind::ForCondition {
                    update,
                    body,
                    condition,
                    ..
                } => {
                    let condition = self.emit_async_loop_condition(condition.as_ref(), value)?;
                    let (loop_body, update_block, schedule, after) = blocks
                        .for_blocks
                        .get(&(segment_index - 1))
                        .copied()
                        .ok_or_else(|| {
                            "internal: async for blocks were not initialized".to_string()
                        })?;
                    self.builder
                        .build_conditional_branch(condition, loop_body, after)
                        .unwrap();
                    self.builder.position_at_end(loop_body);
                    self.push_scope();
                    self.push_loop_control(update_block, after);
                    for statement in body {
                        if self.current_block_terminated() {
                            break;
                        }
                        self.emit_stmt(statement, poll)?;
                    }
                    self.pop_loop_control();
                    if !self.current_block_terminated() {
                        self.builder
                            .build_unconditional_branch(update_block)
                            .unwrap();
                    }
                    self.builder.position_at_end(update_block);
                    self.emit_stmt(update, poll)?;
                    if !self.current_block_terminated() {
                        self.builder.build_unconditional_branch(schedule).unwrap();
                    }
                    self.builder.position_at_end(after);
                    self.pop_scope();
                }
                AsyncAwaitKind::Expression {
                    statement,
                    span,
                    guard,
                    final_in_statement,
                    ..
                } => {
                    let slot = *async_expression_slots.get(span).ok_or_else(|| {
                        "internal: async expression payload has no frame slot".to_string()
                    })?;
                    self.builder.build_store(slot, value).unwrap();
                    self.async_await_values
                        .last_mut()
                        .unwrap()
                        .insert(*span, (slot, binding_type));
                    if *final_in_statement {
                        self.emit_stmt(statement, poll)?;
                        if let Some(expression) = simple_async_statement_expression(statement) {
                            let mut awaits = Vec::new();
                            collect_await_expressions(expression, &mut awaits);
                            for awaited in awaits {
                                if let Some((slot, ty)) = self
                                    .async_await_values
                                    .last()
                                    .and_then(|values| values.get(&awaited.span))
                                    .cloned()
                                    && !ty.is_copy()
                                    && !self.current_block_terminated()
                                {
                                    self.emit_async_drop_value_in_place(poll, slot, &ty)?;
                                    self.mark_async_frame_slot_uninitialized(slot);
                                    self.builder
                                        .build_store(slot, self.llvm_type(&ty).const_zero())
                                        .unwrap();
                                }
                                self.async_await_values
                                    .last_mut()
                                    .unwrap()
                                    .remove(&awaited.span);
                            }
                        }
                        if guard.is_some() {
                            let join = blocks.expression_guard_blocks
                                            .get(&(segment_index - 1))
                                            .copied()
                                            .ok_or_else(|| {
                                                "internal: guarded async expression continuation was not initialized"
                                                    .to_string()
                                            })?;
                            if !self.current_block_terminated() {
                                self.builder.build_unconditional_branch(join).unwrap();
                            }
                            self.builder.position_at_end(join);
                        }
                    }
                }
                AsyncAwaitKind::WhileBody { .. }
                | AsyncAwaitKind::DoWhileBody { .. }
                | AsyncAwaitKind::ForBody { .. }
                | AsyncAwaitKind::ForOfBody { .. }
                | AsyncAwaitKind::ControlFlow { .. }
                | AsyncAwaitKind::FinallyCompletion { .. }
                | AsyncAwaitKind::IfBranch { .. }
                | AsyncAwaitKind::IfBothBranches { .. }
                | AsyncAwaitKind::Switch { .. }
                | AsyncAwaitKind::Try { .. } => {}
                kind => {
                    return Err(format!(
                        "internal: async continuation for `{}` reached the nested-action path",
                        async_await_kind_name(kind)
                    ));
                }
            }
        } else if returns_awaited {
            let awaited_result_type = awaited_db_result.as_ref().unwrap_or(payload_type);
            if matches!(awaited_result_type, Type::Void) {
                self.builder
                    .build_call(promise_free, &[dependency.into()], "")
                    .unwrap();
                self.builder
                    .build_store(dependency_pointer, pointer.const_null())
                    .unwrap();
                self.emit_async_void_ready(async_result);
            } else {
                let raw_payload_type = if awaited_db_result.is_some() {
                    Type::Str
                } else {
                    payload_type.clone()
                };
                let raw_value = self
                    .builder
                    .build_load(
                        self.llvm_type(&raw_payload_type),
                        awaited_payload,
                        "async.await.payload",
                    )
                    .unwrap();
                let value = if let Some(result_type) = awaited_db_result.as_ref() {
                    let raw_string = raw_value.into_struct_value();
                    let decoded = self.emit_db_async_result_decode(raw_string, result_type)?;
                    self.emit_string_release(raw_string)?;
                    decoded
                } else {
                    raw_value
                };
                self.builder
                    .build_call(promise_free, &[dependency.into()], "")
                    .unwrap();
                self.builder
                    .build_store(dependency_pointer, pointer.const_null())
                    .unwrap();
                let free = self.declare_libc_fn("free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[awaited_payload.into()], "")
                    .unwrap();
                self.emit_async_transferred_value_ready(async_result, value, awaited_result_type);
            }
            return Ok(AsyncSegmentFlow::SkipRest);
        } else {
            self.builder
                .build_call(promise_free, &[dependency.into()], "")
                .unwrap();
            self.builder
                .build_store(dependency_pointer, pointer.const_null())
                .unwrap();
        }
        let segment = AsyncSegment {
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
            async_result,
            async_for_of_slots,
            pointer,
            recovered,
            switch_context_control,
        };

        self.emit_async_try_segment(segment, blocks)?;
        self.emit_async_control_flow_segment(segment, blocks)?;
        self.emit_async_finally_completion_segment(segment, blocks)?;
        self.emit_async_if_branch_segment(segment, blocks)?;
        if let Some(flow) = self.emit_async_if_both_branches_segment(segment, blocks)? {
            return Ok(flow);
        }
        if let Some(flow) = self.emit_async_switch_segment(segment, blocks)? {
            return Ok(flow);
        }
        self.emit_async_loop_body_segment(segment, blocks)?;
        Ok(AsyncSegmentFlow::Proceed)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_async_switch_dispatch(
        &mut self,
        outer_index: usize,
        switch_id: Span,
        expr: &Expr,
        cases: &[SwitchCase],
        default: &Option<Vec<Stmt>>,
        default_index: Option<usize>,
        await_sites: &[AsyncAwaitSite],
        blocks: &mut AsyncBlockRegistry<'ctx>,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
        async_for_of_slots: &HashMap<
            AsyncForOfSlotKey,
            (PointerValue<'ctx>, PointerValue<'ctx>, PointerValue<'ctx>),
        >,
    ) -> Result<(), String> {
        let subject_type = self
            .expr_type_hint(expr)
            .ok_or_else(|| "error: cannot infer type for async switch expression".to_string())?;
        let subject = self.emit_expr(expr)?;
        self.emit_async_switch_dispatch_value(
            outer_index,
            switch_id,
            &subject_type,
            subject,
            false,
            cases,
            default,
            default_index,
            await_sites,
            blocks,
            poll,
            frame_type,
            frame,
            state_pointer,
            async_for_of_slots,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_async_switch_clause_guard_entry(
        &mut self,
        outer_index: usize,
        switch_id: Span,
        clause_index: usize,
        site_index: usize,
        dependency: &Expr,
        guard: &AsyncSwitchClauseGuard,
        fallthrough: BasicBlock<'ctx>,
        blocks: &mut AsyncBlockRegistry<'ctx>,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
        async_for_of_slots: &HashMap<
            AsyncForOfSlotKey,
            (PointerValue<'ctx>, PointerValue<'ctx>, PointerValue<'ctx>),
        >,
    ) -> Result<(), String> {
        let selected = self
            .context
            .append_basic_block(poll, "async.switch.nested.selected");
        let bypass = self
            .context
            .append_basic_block(poll, "async.switch.nested.bypass");
        let for_of = guard.for_of.as_ref();
        let for_of_slots = for_of
            .map(|_| {
                async_for_of_slots
                    .get(&AsyncForOfSlotKey::SwitchClause(outer_index, clause_index))
                    .copied()
                    .ok_or_else(|| {
                        "internal: nested switch for...of frame slots are missing".to_string()
                    })
            })
            .transpose()?;
        if let (Some(for_of), Some((data_slot, length_slot, index_slot))) = (for_of, for_of_slots) {
            let array = self.emit_expr(&for_of.iterable)?.into_struct_value();
            let data = self
                .builder
                .build_extract_value(array, 0, "async.switch.nested.for_of.data")
                .unwrap();
            let length = self
                .builder
                .build_extract_value(array, 1, "async.switch.nested.for_of.length")
                .unwrap();
            self.builder.build_store(data_slot, data).unwrap();
            self.builder.build_store(length_slot, length).unwrap();
            self.builder
                .build_store(index_slot, self.context.i64_type().const_zero())
                .unwrap();
        }
        let condition_block = guard.repeat.then(|| {
            self.context
                .append_basic_block(poll, "async.switch.nested.loop.condition")
        });
        if let Some(condition_block) = condition_block {
            let repeat_block = if guard.repeat_update.is_some() || for_of.is_some() {
                self.context
                    .append_basic_block(poll, "async.switch.nested.loop.update")
            } else {
                condition_block
            };
            blocks
                .switch_clause_loop_blocks
                .insert((switch_id, clause_index), (repeat_block, bypass));
            self.builder
                .build_unconditional_branch(if guard.enter_before_condition {
                    selected
                } else {
                    condition_block
                })
                .unwrap();
            if repeat_block != condition_block {
                self.builder.position_at_end(repeat_block);
                if let Some(update) = &guard.repeat_update {
                    self.emit_stmt(update, poll)?;
                } else if let Some((_, _, index_slot)) = for_of_slots {
                    let index = self
                        .builder
                        .build_load(
                            self.context.i64_type(),
                            index_slot,
                            "async.switch.nested.for_of.update.index",
                        )
                        .unwrap()
                        .into_int_value();
                    let next = self
                        .builder
                        .build_int_add(
                            index,
                            self.context.i64_type().const_int(1, false),
                            "async.switch.nested.for_of.next.index",
                        )
                        .unwrap();
                    self.builder.build_store(index_slot, next).unwrap();
                }
                if !self.current_block_terminated() {
                    self.builder
                        .build_unconditional_branch(condition_block)
                        .unwrap();
                }
            }
            self.builder.position_at_end(condition_block);
        }
        let condition = if let Some((_, length_slot, index_slot)) = for_of_slots {
            let index = self
                .builder
                .build_load(
                    self.context.i64_type(),
                    index_slot,
                    "async.switch.nested.for_of.index",
                )
                .unwrap()
                .into_int_value();
            let length = self
                .builder
                .build_load(
                    self.context.i64_type(),
                    length_slot,
                    "async.switch.nested.for_of.length",
                )
                .unwrap()
                .into_int_value();
            self.builder
                .build_int_compare(
                    IntPredicate::ULT,
                    index,
                    length,
                    "async.switch.nested.for_of.has_next",
                )
                .unwrap()
        } else {
            self.emit_condition_value(&guard.condition)?
        };
        let (then_block, else_block) = if guard.schedule_when_true {
            (selected, bypass)
        } else {
            (bypass, selected)
        };
        self.builder
            .build_conditional_branch(condition, then_block, else_block)
            .unwrap();
        self.builder.position_at_end(selected);
        if let (Some(for_of), Some((data_slot, _, index_slot))) = (for_of, for_of_slots) {
            let iterable_type = self
                .expr_type_hint(&for_of.iterable)
                .map(|ty| self.resolve_type(&ty))
                .ok_or_else(|| {
                    "internal: cannot infer nested switch for...of iterable type".to_string()
                })?;
            let Type::Array(element_type) = transparent_codegen_type(&iterable_type) else {
                return Err("internal: nested switch for...of iterable is not an array".to_string());
            };
            let element_type = self.resolve_type(element_type);
            if !element_type.is_copy() {
                return Err("internal: nested switch for...of element must be Copy".to_string());
            }
            let data = self
                .builder
                .build_load(
                    self.context.ptr_type(inkwell::AddressSpace::default()),
                    data_slot,
                    "async.switch.nested.for_of.data.load",
                )
                .unwrap()
                .into_pointer_value();
            let index = self
                .builder
                .build_load(
                    self.context.i64_type(),
                    index_slot,
                    "async.switch.nested.for_of.element.index",
                )
                .unwrap()
                .into_int_value();
            let element_pointer = unsafe {
                self.builder
                    .build_gep(
                        self.llvm_type(&element_type),
                        data,
                        &[index],
                        "async.switch.nested.for_of.element.pointer",
                    )
                    .unwrap()
            };
            let element = self
                .builder
                .build_load(
                    self.llvm_type(&element_type),
                    element_pointer,
                    "async.switch.nested.for_of.element",
                )
                .unwrap();
            let (binding_slot, _) = self.async_frame_slot(&for_of.name).ok_or_else(|| {
                "internal: nested switch for...of binding has no frame slot".to_string()
            })?;
            self.builder.build_store(binding_slot, element).unwrap();
            self.store_var(&for_of.name, binding_slot, element_type);
        }
        if condition_block.is_some() {
            let (repeat, _) = blocks.switch_clause_loop_blocks[&(switch_id, clause_index)];
            self.push_loop_control(repeat, bypass);
        }
        for statement in &guard.selected_before {
            if self.current_block_terminated() {
                break;
            }
            self.emit_stmt(statement, poll)?;
        }
        if condition_block.is_some() {
            self.pop_loop_control();
        }
        if !self.current_block_terminated() {
            self.emit_async_suspend_dependency(
                poll,
                frame_type,
                frame,
                state_pointer,
                (site_index + 1) as u64,
                dependency,
            )?;
        }
        self.builder.position_at_end(bypass);
        for statement in &guard.bypass {
            if self.current_block_terminated() {
                break;
            }
            self.emit_stmt(statement, poll)?;
        }
        if !self.current_block_terminated() {
            self.builder
                .build_unconditional_branch(fallthrough)
                .unwrap();
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_async_switch_dispatch_value(
        &mut self,
        outer_index: usize,
        switch_id: Span,
        subject_type: &Type,
        subject: BasicValueEnum<'ctx>,
        owned_subject: bool,
        cases: &[SwitchCase],
        default: &Option<Vec<Stmt>>,
        default_index: Option<usize>,
        await_sites: &[AsyncAwaitSite],
        blocks: &mut AsyncBlockRegistry<'ctx>,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
        async_for_of_slots: &HashMap<
            AsyncForOfSlotKey,
            (PointerValue<'ctx>, PointerValue<'ctx>, PointerValue<'ctx>),
        >,
    ) -> Result<(), String> {
        let end = self.context.append_basic_block(poll, "async.switch.end");
        let case_blocks = (0..cases.len())
            .map(|index| {
                self.context
                    .append_basic_block(poll, &format!("async.switch.case.{index}"))
            })
            .collect::<Vec<_>>();
        let default_block = default.as_ref().map(|_| {
            self.context
                .append_basic_block(poll, "async.switch.default")
        });
        let unmatched = default_block.unwrap_or(end);
        let unmatched_cleanup = (owned_subject && default.is_none()).then(|| {
            self.context
                .append_basic_block(poll, "async.switch.subject.drop.unmatched")
        });

        for (index, case) in cases.iter().enumerate() {
            let next_comparison = if index + 1 < cases.len() {
                self.context
                    .append_basic_block(poll, &format!("async.switch.next.{index}"))
            } else {
                unmatched_cleanup.unwrap_or(unmatched)
            };
            let equals = self.emit_async_switch_case_equality(
                subject_type,
                subject,
                &case.value,
                &format!("async.switch.cmp.{index}"),
            )?;
            self.builder
                .build_conditional_branch(equals, case_blocks[index], next_comparison)
                .unwrap();
            if index + 1 < cases.len() {
                self.builder.position_at_end(next_comparison);
            }
        }
        if cases.is_empty() {
            self.builder
                .build_unconditional_branch(unmatched_cleanup.unwrap_or(unmatched))
                .unwrap();
        }
        if let Some(cleanup) = unmatched_cleanup {
            self.builder.position_at_end(cleanup);
            self.emit_owned_native_value_free(subject, subject_type);
            self.builder.build_unconditional_branch(unmatched).unwrap();
        }

        let mut clauses = cases
            .iter()
            .enumerate()
            .map(|(index, case)| (case_blocks[index], case.body.as_slice()))
            .collect::<Vec<_>>();
        if let (Some(default_body), Some(default_block), Some(default_index)) =
            (default.as_ref(), default_block, default_index)
        {
            clauses.insert(default_index, (default_block, default_body));
        }
        let clause_fallthrough = (0..clauses.len())
            .map(|clause_index| {
                clauses
                    .get(clause_index + 1)
                    .map_or(end, |(block, _)| *block)
            })
            .collect::<Vec<_>>();
        blocks.switch_blocks.insert(
            switch_id,
            AsyncSwitchBlocks {
                end,
                case_blocks: case_blocks.clone(),
                unmatched,
                clause_fallthrough: clause_fallthrough.clone(),
            },
        );

        let first_sites = await_sites
            .iter()
            .enumerate()
            .filter_map(|(site_index, site)| match &site.kind {
                AsyncAwaitKind::Switch {
                    switch_id: candidate,
                    await_clause,
                    before,
                    entry_guard,
                    first_in_clause: true,
                    ..
                } if site.outer_index == outer_index && *candidate == switch_id => Some((
                    *await_clause,
                    (site_index, site, before.as_slice(), entry_guard.as_ref()),
                )),
                _ => None,
            })
            .collect::<HashMap<_, _>>();
        let context_control =
            self.push_async_switch_context_control(outer_index, switch_id, blocks);
        self.push_break_target(end);
        for (clause_index, (block, body)) in clauses.iter().enumerate() {
            self.builder.position_at_end(*block);
            self.push_scope();
            if owned_subject {
                let name = format!("<async switch subject {switch_id:?} {clause_index}>");
                let slot =
                    self.entry_alloca(self.llvm_type(subject_type), "async.switch.subject.owner")?;
                self.builder
                    .build_store(slot, subject)
                    .map_err(|error| error.to_string())?;
                self.store_var(&name, slot, subject_type.clone());
                self.owned_native_vars
                    .last_mut()
                    .ok_or("internal: async switch subject has no ownership scope")?
                    .insert(name, ReleasePlan::Deep);
            }
            let (statements, suspension) = if let Some((site_index, site, before, entry_guard)) =
                first_sites.get(&clause_index)
            {
                (*before, Some((*site_index, *site, *entry_guard)))
            } else {
                (*body, None)
            };
            for statement in statements {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
            if !self.current_block_terminated()
                && let Some((site_index, site, entry_guard)) = suspension
            {
                if let Some(entry_guard) = entry_guard {
                    self.emit_async_switch_clause_guard_entry(
                        outer_index,
                        switch_id,
                        clause_index,
                        site_index,
                        &site.dependency,
                        entry_guard,
                        clause_fallthrough[clause_index],
                        blocks,
                        poll,
                        frame_type,
                        frame,
                        state_pointer,
                        async_for_of_slots,
                    )?;
                } else {
                    self.emit_async_suspend_dependency(
                        poll,
                        frame_type,
                        frame,
                        state_pointer,
                        (site_index + 1) as u64,
                        &site.dependency,
                    )?;
                }
            }
            self.pop_scope();
            if !self.current_block_terminated() {
                self.builder
                    .build_unconditional_branch(clause_fallthrough[clause_index])
                    .unwrap();
            }
        }
        self.pop_break_target();
        if context_control {
            self.pop_loop_control();
        }
        if first_sites.is_empty() {
            self.builder.position_at_end(end);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_async_switch_case_dispatch(
        &mut self,
        outer_index: usize,
        switch_id: Span,
        subject_type: &Type,
        subject_storage: &str,
        cases: &[SwitchCase],
        default: &Option<Vec<Stmt>>,
        default_index: Option<usize>,
        start_case: usize,
        await_sites: &[AsyncAwaitSite],
        blocks: &mut AsyncBlockRegistry<'ctx>,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
        async_expression_slots: &HashMap<Span, PointerValue<'ctx>>,
        async_for_of_slots: &HashMap<
            AsyncForOfSlotKey,
            (PointerValue<'ctx>, PointerValue<'ctx>, PointerValue<'ctx>),
        >,
    ) -> Result<(), String> {
        self.initialize_async_switch_case_blocks(
            outer_index,
            switch_id,
            cases,
            default,
            default_index,
            await_sites,
            blocks,
            poll,
            frame_type,
            frame,
            state_pointer,
            async_for_of_slots,
        )?;
        let (subject_slot, _) = self.async_frame_slot(subject_storage).ok_or_else(|| {
            "internal: suspending switch subject has no async frame slot".to_string()
        })?;
        let subject = self
            .builder
            .build_load(
                self.llvm_type(subject_type),
                subject_slot,
                "async.switch.subject.load",
            )
            .unwrap();
        let switch = blocks
            .switch_blocks
            .get(&switch_id)
            .cloned()
            .ok_or_else(|| "internal: async switch blocks were not initialized".to_string())?;
        for (case_index, case) in cases.iter().enumerate().skip(start_case) {
            if let Some((site_index, site)) = await_sites.iter().enumerate().find(|(_, site)| {
                site.outer_index == outer_index
                    && matches!(
                        site.kind,
                        AsyncAwaitKind::SwitchCaseExpression {
                            case_index: candidate,
                            switch_id: candidate_switch,
                            ..
                        } if candidate_switch == switch_id && candidate == case_index
                    )
            }) {
                if let AsyncAwaitKind::SwitchCaseExpression {
                    span,
                    payload_type,
                    prefixes,
                    guard,
                    ..
                } = &site.kind
                {
                    self.store_async_expression_prefixes(prefixes, async_expression_slots)?;
                    if let Some(guard) = guard {
                        let schedule = self
                            .context
                            .append_basic_block(poll, "async.switch.case.guard.schedule");
                        let bypass = self
                            .context
                            .append_basic_block(poll, "async.switch.case.guard.bypass");
                        let condition = match &guard.kind {
                            AsyncExpressionGuardKind::Boolean { .. } => {
                                self.emit_condition_value(&guard.condition)?
                            }
                            AsyncExpressionGuardKind::Optional { .. } => {
                                let value = self.emit_expr(&guard.condition)?.into_pointer_value();
                                self.builder
                                    .build_is_null(value, "async.switch.case.guard.is_null")
                                    .unwrap()
                            }
                        };
                        let schedule_when_true = match &guard.kind {
                            AsyncExpressionGuardKind::Boolean { schedule_when_true } => {
                                *schedule_when_true
                            }
                            AsyncExpressionGuardKind::Optional { schedule_when_null } => {
                                *schedule_when_null
                            }
                        };
                        let (then_block, else_block) = if schedule_when_true {
                            (schedule, bypass)
                        } else {
                            (bypass, schedule)
                        };
                        self.builder
                            .build_conditional_branch(condition, then_block, else_block)
                            .unwrap();
                        self.builder.position_at_end(schedule);
                        self.emit_async_suspend_dependency(
                            poll,
                            frame_type,
                            frame,
                            state_pointer,
                            (site_index + 1) as u64,
                            &site.dependency,
                        )?;
                        self.builder.position_at_end(bypass);
                        let await_slot = *async_expression_slots.get(span).ok_or_else(|| {
                            "internal: guarded async switch case has no payload slot".to_string()
                        })?;
                        self.async_await_values
                            .last_mut()
                            .unwrap()
                            .insert(*span, (await_slot, payload_type.clone()));
                        let next = self.context.append_basic_block(
                            poll,
                            &format!("async.switch.guarded.next.{case_index}"),
                        );
                        let selected = self.context.append_basic_block(
                            poll,
                            &format!("async.switch.guarded.selected.{case_index}"),
                        );
                        let equals = self.emit_async_switch_case_equality(
                            subject_type,
                            subject,
                            &case.value,
                            &format!("async.switch.guarded.cmp.{case_index}"),
                        )?;
                        self.async_await_values.last_mut().unwrap().remove(span);
                        self.drop_async_switch_case_expression_temporaries(
                            poll,
                            case_index,
                            await_sites,
                            async_expression_slots,
                        )?;
                        self.builder
                            .build_conditional_branch(equals, selected, next)
                            .unwrap();
                        self.builder.position_at_end(selected);
                        self.builder
                            .build_unconditional_branch(switch.case_blocks[case_index])
                            .unwrap();
                        self.builder.position_at_end(next);
                        return self.emit_async_switch_case_dispatch(
                            outer_index,
                            switch_id,
                            subject_type,
                            subject_storage,
                            cases,
                            default,
                            default_index,
                            case_index + 1,
                            await_sites,
                            blocks,
                            poll,
                            frame_type,
                            frame,
                            state_pointer,
                            async_expression_slots,
                            async_for_of_slots,
                        );
                    }
                }
                return self.emit_async_suspend_dependency(
                    poll,
                    frame_type,
                    frame,
                    state_pointer,
                    (site_index + 1) as u64,
                    &site.dependency,
                );
            }
            let next = self
                .context
                .append_basic_block(poll, &format!("async.switch.next.{case_index}"));
            let selected = self.context.append_basic_block(
                poll,
                &format!("async.switch.subject.drop.case.{case_index}"),
            );
            let equals = self.emit_async_switch_case_equality(
                subject_type,
                subject,
                &case.value,
                &format!("async.switch.cmp.{case_index}"),
            )?;
            self.builder
                .build_conditional_branch(equals, selected, next)
                .unwrap();
            self.builder.position_at_end(selected);
            self.builder
                .build_unconditional_branch(switch.case_blocks[case_index])
                .unwrap();
            self.builder.position_at_end(next);
        }
        self.finish_async_switch_subject(poll, subject_storage, subject_type)?;
        self.builder
            .build_unconditional_branch(switch.unmatched)
            .unwrap();
        Ok(())
    }

    pub(super) fn emit_async_switch_case_equality(
        &mut self,
        subject_type: &Type,
        subject: BasicValueEnum<'ctx>,
        case_value: &Expr,
        name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        if *subject_type == Type::Str {
            let value = self.emit_expr(case_value)?;
            let equals = self
                .emit_string_equality(subject.into_struct_value(), value.into_struct_value(), true)?
                .into_int_value();
            self.emit_async_cleanup_source_expression(value, subject_type, case_value)?;
            Ok(equals)
        } else {
            let case_value = if is_integer_type(subject_type) {
                self.emit_expr_as(case_value, subject_type)?
            } else {
                self.emit_expr(case_value)?
            };
            Ok(self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    subject.into_int_value(),
                    case_value.into_int_value(),
                    name,
                )
                .unwrap())
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_async_switch_case_resume(
        &mut self,
        outer_index: usize,
        switch_id: Span,
        subject_type: &Type,
        subject_storage: &str,
        cases: &[SwitchCase],
        default: &Option<Vec<Stmt>>,
        default_index: Option<usize>,
        case_index: usize,
        span: Span,
        payload_type: &Type,
        _prefixes: &[(Expr, Type)],
        _guard: Option<&AsyncExpressionGuard>,
        final_in_case_expression: bool,
        case_value: BasicValueEnum<'ctx>,
        await_sites: &[AsyncAwaitSite],
        blocks: &mut AsyncBlockRegistry<'ctx>,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
        async_expression_slots: &HashMap<Span, PointerValue<'ctx>>,
        async_for_of_slots: &HashMap<
            AsyncForOfSlotKey,
            (PointerValue<'ctx>, PointerValue<'ctx>, PointerValue<'ctx>),
        >,
    ) -> Result<(), String> {
        self.initialize_async_switch_case_blocks(
            outer_index,
            switch_id,
            cases,
            default,
            default_index,
            await_sites,
            blocks,
            poll,
            frame_type,
            frame,
            state_pointer,
            async_for_of_slots,
        )?;
        let case_slot = *async_expression_slots
            .get(&span)
            .ok_or_else(|| "internal: async switch case payload has no frame slot".to_string())?;
        self.builder.build_store(case_slot, case_value).unwrap();
        self.async_await_values
            .last_mut()
            .unwrap()
            .insert(span, (case_slot, payload_type.clone()));
        if !final_in_case_expression {
            let current_site_index = await_sites
                .iter()
                .position(|site| {
                    site.outer_index == outer_index
                        && matches!(
                            site.kind,
                            AsyncAwaitKind::SwitchCaseExpression {
                                case_index: candidate,
                                span: candidate_span,
                                switch_id: candidate_switch,
                                ..
                            } if candidate_switch == switch_id && candidate == case_index && candidate_span == span
                        )
                })
                .ok_or_else(|| {
                    "internal: resumed async switch case has no await state".to_string()
                })?;
            let (site_index, next_site) = await_sites
                .iter()
                .enumerate()
                .skip(current_site_index + 1)
                .find(|(_, site)| {
                    site.outer_index == outer_index
                        && matches!(
                            site.kind,
                            AsyncAwaitKind::SwitchCaseExpression {
                                case_index: candidate,
                                switch_id: candidate_switch,
                                ..
                            } if candidate_switch == switch_id && candidate == case_index
                        )
                })
                .ok_or_else(|| {
                    "internal: async switch case continuation has no next await state".to_string()
                })?;
            if let AsyncAwaitKind::SwitchCaseExpression { prefixes, .. } = &next_site.kind {
                self.store_async_expression_prefixes(prefixes, async_expression_slots)?;
            }
            return self.emit_async_suspend_dependency(
                poll,
                frame_type,
                frame,
                state_pointer,
                (site_index + 1) as u64,
                &next_site.dependency,
            );
        }
        let (subject_slot, _) = self.async_frame_slot(subject_storage).ok_or_else(|| {
            "internal: suspending switch subject has no async frame slot".to_string()
        })?;
        let subject = self
            .builder
            .build_load(
                self.llvm_type(subject_type),
                subject_slot,
                "async.switch.subject.resume",
            )
            .unwrap();
        let equals = self.emit_async_switch_case_equality(
            subject_type,
            subject,
            &cases[case_index].value,
            &format!("async.switch.awaited.cmp.{case_index}"),
        )?;
        self.drop_async_switch_case_expression_temporaries(
            poll,
            case_index,
            await_sites,
            async_expression_slots,
        )?;
        let selected = self
            .context
            .append_basic_block(poll, "async.switch.awaited.selected");
        let next = self
            .context
            .append_basic_block(poll, "async.switch.awaited.next");
        self.builder
            .build_conditional_branch(equals, selected, next)
            .unwrap();
        self.builder.position_at_end(selected);
        let selected_case = blocks
            .switch_blocks
            .get(&switch_id)
            .and_then(|switch| switch.case_blocks.get(case_index))
            .copied()
            .ok_or_else(|| "internal: awaited switch case block is missing".to_string())?;
        self.builder
            .build_unconditional_branch(selected_case)
            .unwrap();
        self.builder.position_at_end(next);
        self.emit_async_switch_case_dispatch(
            outer_index,
            switch_id,
            subject_type,
            subject_storage,
            cases,
            default,
            default_index,
            case_index + 1,
            await_sites,
            blocks,
            poll,
            frame_type,
            frame,
            state_pointer,
            async_expression_slots,
            async_for_of_slots,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_async_segment_body(
        &mut self,
        segment_index: usize,
        body: &[Stmt],
        await_sites: &[AsyncAwaitSite],
        catch_await_groups: &[AsyncCatchAwaitGroup],
        finally_await_groups: &[AsyncFinallyAwaitGroup],
        blocks: &mut AsyncBlockRegistry<'ctx>,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
        async_expression_slots: &HashMap<Span, PointerValue<'ctx>>,
        async_for_of_slots: &HashMap<
            AsyncForOfSlotKey,
            (PointerValue<'ctx>, PointerValue<'ctx>, PointerValue<'ctx>),
        >,
    ) -> Result<(), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let end = await_sites
            .get(segment_index)
            .map_or(body.len(), |site| site.outer_index);
        let start = if segment_index == 0 {
            0
        } else {
            let previous = await_sites[segment_index - 1].outer_index;
            if previous == end { end } else { previous + 1 }
        };
        for (offset, stmt) in body[start..end].iter().enumerate() {
            let outer_index = start + offset;
            let catch_only = catch_await_groups
                .iter()
                .find(|group| {
                    group.outer_index == outer_index && group.recursive_try_depth.is_none()
                })
                .filter(|_| {
                    !await_sites.iter().any(|site| {
                        site.outer_index == outer_index
                            && matches!(site.kind, AsyncAwaitKind::Try { .. })
                    })
                });
            if let Some(group) = catch_only {
                let StmtKind::TryCatch { try_body, .. } = stmt.kind() else {
                    return Err(
                        "internal: async catch group is not attached to a try statement"
                            .to_string(),
                    );
                };
                self.emit_async_catch_only_try(
                    group,
                    try_body,
                    await_sites,
                    catch_await_groups,
                    blocks,
                    poll,
                    frame_type,
                    frame,
                    state_pointer,
                )?;
            } else {
                self.emit_stmt(stmt, poll)?;
            }
            if self.current_block_terminated() {
                break;
            }
        }
        if self.current_block_terminated() || segment_index == await_sites.len() {
            return Ok(());
        }

        let await_site = &await_sites[segment_index];
        let initial_switch_context = match &await_site.kind {
            AsyncAwaitKind::SwitchSubject {
                switch_id, context, ..
            } => Some((*switch_id, context.as_ref())),
            AsyncAwaitKind::SwitchCaseExpression {
                first_in_switch: true,
                switch_id,
                context,
                ..
            }
            | AsyncAwaitKind::Switch {
                first_in_switch: true,
                switch_id,
                context,
                ..
            } => Some((*switch_id, context.as_ref())),
            _ => None,
        };
        if let Some((switch_id, context)) = initial_switch_context {
            self.begin_async_switch_context(
                await_site.outer_index,
                switch_id,
                context,
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
        let context = AsyncBody {
            segment_index,
            await_site,
            await_sites,
            catch_await_groups,
            finally_await_groups,
            poll,
            frame_type,
            frame,
            state_pointer,
            async_expression_slots,
            async_for_of_slots,
            pointer,
        };

        if let AsyncAwaitKind::ControlFlow {
            contexts,
            before,
            enter_depth,
            ..
        } = &await_site.kind
        {
            if *enter_depth < contexts.len() {
                self.begin_recursive_control_path_from(
                    await_site.outer_index,
                    contexts,
                    *enter_depth,
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
            self.emit_recursive_control_statements(
                before,
                await_site.outer_index,
                contexts,
                blocks,
                poll,
            )?;
            if self.current_block_terminated() {
                return Ok(());
            }
        }
        if self.emit_async_body_finally(context, blocks)? {
            return Ok(());
        }
        if self.emit_async_body_switch_case(context, blocks)? {
            return Ok(());
        }
        if self.emit_async_body_switch(context, blocks)? {
            return Ok(());
        }
        if self.emit_async_body_try(context, blocks)? {
            return Ok(());
        }
        if self.emit_async_body_loop(context, blocks)? {
            return Ok(());
        }
        if let AsyncAwaitKind::Expression { prefixes, .. } = &await_site.kind {
            for (prefix, prefix_type) in prefixes {
                if self
                    .async_await_values
                    .last()
                    .is_some_and(|values| values.contains_key(&prefix.span))
                {
                    continue;
                }
                let value = self.emit_expr_as(prefix, prefix_type)?;
                let slot = *async_expression_slots.get(&prefix.span).ok_or_else(|| {
                    "internal: async evaluation prefix has no frame slot".to_string()
                })?;
                self.builder.build_store(slot, value).unwrap();
                self.async_await_values
                    .last_mut()
                    .unwrap()
                    .insert(prefix.span, (slot, prefix_type.clone()));
            }
        }
        if let AsyncAwaitKind::Expression {
            statement,
            span,
            ty,
            guard: Some(guard),
            ..
        } = &await_site.kind
        {
            let schedule = self
                .context
                .append_basic_block(poll, "async.expression.guard.schedule");
            let bypass = self
                .context
                .append_basic_block(poll, "async.expression.guard.bypass");
            let join = self
                .context
                .append_basic_block(poll, "async.expression.guard.after");
            blocks.expression_guard_blocks.insert(segment_index, join);
            let condition = match &guard.kind {
                AsyncExpressionGuardKind::Boolean { .. } => {
                    self.emit_condition_value(&guard.condition)?
                }
                AsyncExpressionGuardKind::Optional { .. } => {
                    let value = self.emit_expr(&guard.condition)?.into_pointer_value();
                    self.builder
                        .build_is_null(value, "async.expression.guard.is_null")
                        .unwrap()
                }
            };
            let schedule_when_true = match &guard.kind {
                AsyncExpressionGuardKind::Boolean { schedule_when_true } => *schedule_when_true,
                AsyncExpressionGuardKind::Optional { schedule_when_null } => *schedule_when_null,
            };
            let (then_block, else_block) = if schedule_when_true {
                (schedule, bypass)
            } else {
                (bypass, schedule)
            };
            self.builder
                .build_conditional_branch(condition, then_block, else_block)
                .unwrap();
            self.builder.position_at_end(bypass);
            let await_slot = *async_expression_slots.get(span).ok_or_else(|| {
                "internal: guarded async expression has no payload slot".to_string()
            })?;
            self.async_await_values
                .last_mut()
                .unwrap()
                .insert(*span, (await_slot, ty.clone()));
            self.emit_stmt(statement, poll)?;
            if !self.current_block_terminated() {
                self.builder.build_unconditional_branch(join).unwrap();
            }
            self.builder.position_at_end(schedule);
        }
        let loop_guard = match &await_site.kind {
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
            } => condition.guard.as_ref(),
            _ => None,
        };
        if let Some(guard) = loop_guard {
            let schedule = self
                .context
                .append_basic_block(poll, "async.loop.guard.schedule");
            let after = blocks
                .expression_guard_blocks
                .get(&segment_index)
                .copied()
                .ok_or_else(|| "internal: guarded async loop has no exit block".to_string())?;
            let condition = self.emit_condition_value(guard)?;
            self.builder
                .build_conditional_branch(condition, schedule, after)
                .unwrap();
            self.builder.position_at_end(schedule);
        }
        self.emit_async_suspend_dependency(
            poll,
            frame_type,
            frame,
            state_pointer,
            (segment_index + 1) as u64,
            &await_site.dependency,
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn emit_async_function(
        &mut self,
        name: &str,
        params: &[Param],
        ret_type: &Type,
        body: &[Stmt],
    ) -> Result<(), String> {
        let Some(payload_type) = promise_payload_type(ret_type) else {
            return Err("error: async codegen requires a `Promise<T>` return type".to_string());
        };
        let payload_type = self.resolve_type(payload_type);
        let previous_prescan = std::mem::take(&mut self.async_prescan_types);
        for param in params {
            if let ParamPattern::Name(storage_name) = &param.pattern {
                let declared_type = self.resolve_type(&param.ty);
                let param_type = if param.optional && param.default_value.is_none() {
                    Type::Optional(Box::new(declared_type))
                } else {
                    declared_type
                };
                self.async_prescan_types
                    .insert(storage_name.clone(), param_type);
            }
        }
        self.prescan_async_local_types(body);
        let lowered_body_result = self.lower_async_onfail_body(body, &payload_type);
        self.async_prescan_types = previous_prescan;
        let lowered_body = lowered_body_result?;
        let body = lowered_body.as_slice();
        if name == "main" && (!params.is_empty() || !matches!(payload_type, Type::Void)) {
            return Err(
                "error: `async main` must have no parameters and return `Promise<void>`"
                    .to_string(),
            );
        }
        if !self.async_payload_supported(&payload_type) {
            return Err(format!(
                "error: async return payload `{payload_type}` does not have an ownership-safe cleanup implementation yet"
            ));
        }
        let mut frame_params = Vec::new();
        for (index, param) in params.iter().enumerate() {
            let declared_type = self.resolve_type(&param.ty);
            let param_type = if param.optional && param.default_value.is_none() {
                Type::Optional(Box::new(declared_type))
            } else {
                declared_type
            };
            let storage_name = match &param.pattern {
                ParamPattern::Name(name) => name.clone(),
                ParamPattern::Array(_) | ParamPattern::Object(_) => {
                    format!("__tyrus_async_param_{index}")
                }
            };
            let supported = match &param.pattern {
                ParamPattern::Array(_) => {
                    matches!(transparent_codegen_type(&param_type), Type::Array(element) if element.is_copy() || self.async_owned_value_supported(element))
                }
                ParamPattern::Object(_) => self.async_frame_object(&param_type),
                ParamPattern::Name(_) => {
                    param_type.is_copy()
                        || async_promise_handle(&param_type)
                        || async_http_reply_handle(&param_type)
                        || self.async_owned_value_supported(&param_type)
                }
            };
            if !supported {
                return Err(format!(
                    "error: async parameter `{storage_name}` requires ownership-safe frame storage"
                ));
            }
            frame_params.push(AsyncFrameParam {
                storage_name,
                ty: param_type,
                pattern: param.pattern.clone(),
            });
        }

        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let poll_type = self
            .context
            .i32_type()
            .fn_type(&[pointer.into(), pointer.into(), pointer.into()], false);
        let poll = self
            .module
            .add_function(&format!("__tyrus_async_poll_{name}"), poll_type, None);
        let poll_entry = self.context.append_basic_block(poll, "entry");
        self.builder.position_at_end(poll_entry);
        self.push_scope();
        self.return_types.push(payload_type.clone());
        self.push_throw_frame(self.fn_throws.get(name).cloned().unwrap_or_default());
        let async_result = poll.get_nth_param(1).unwrap().into_pointer_value();
        self.async_results.push(async_result);
        self.async_await_values.push(HashMap::default());
        let previous_prescan = std::mem::take(&mut self.async_prescan_types);
        for parameter in &frame_params {
            self.async_prescan_types
                .insert(parameter.storage_name.clone(), parameter.ty.clone());
        }
        self.prescan_async_local_types(body);
        for statement in body {
            if let StmtKind::TryCatch {
                catch_binding,
                catch_type,
                ..
            } = statement.kind()
            {
                self.async_prescan_types
                    .entry(catch_binding.clone())
                    .or_insert_with(|| catch_type.clone());
            }
        }
        let await_sites = self.collect_async_await_sites(body);
        let catch_await_groups = self.collect_async_catch_await_groups(body, &await_sites);
        let finally_await_groups = self.collect_async_finally_await_groups(body, &await_sites);
        let throw_regions = self.collect_async_throw_regions(
            body,
            &await_sites,
            &catch_await_groups,
            &finally_await_groups,
        );
        let guards_throws = !throw_regions.is_empty();
        if guards_throws {
            self.begin_async_throw_regions(poll, throw_regions);
        }
        let mut frame_locals = Vec::new();
        let last_suspending_outer = await_sites
            .last()
            .map(|site| site.outer_index)
            .into_iter()
            .chain(catch_await_groups.iter().map(|group| group.outer_index))
            .chain(finally_await_groups.iter().map(|group| group.outer_index))
            .max();
        self.collect_async_frame_locals(body, last_suspending_outer, &mut frame_locals)?;
        for site in &await_sites {
            if let AsyncAwaitKind::FinallyCompletion {
                mode_storage,
                throw_storage,
                return_storage,
                first_in_finally: true,
                ..
            } = &site.kind
            {
                frame_locals.push((mode_storage.clone(), Type::I32, AsyncLocalStorage::None));
                frame_locals.push((throw_storage.clone(), Type::Str, AsyncLocalStorage::None));
                if let Some(return_storage) = return_storage {
                    frame_locals.push((
                        return_storage.clone(),
                        payload_type.clone(),
                        if payload_type.is_copy() {
                            AsyncLocalStorage::None
                        } else {
                            AsyncLocalStorage::OwnedValue
                        },
                    ));
                }
            }
            if let AsyncAwaitKind::SwitchCaseExpression {
                subject_storage,
                subject_type,
                ..
            } = &site.kind
            {
                let resolved = self.resolve_type(subject_type);
                if !resolved.is_copy() && !self.async_owned_value_supported(&resolved) {
                    return Err(format!(
                        "error: async switch subject of type `{resolved}` cannot be preserved across a suspending case expression"
                    ));
                }
                if !frame_locals
                    .iter()
                    .any(|(name, _, _)| name == subject_storage)
                {
                    frame_locals.push((
                        subject_storage.clone(),
                        resolved.clone(),
                        if resolved.is_copy() {
                            AsyncLocalStorage::None
                        } else {
                            AsyncLocalStorage::OwnedValue
                        },
                    ));
                }
            }
        }
        for group in &catch_await_groups {
            let catch_type = self.resolve_type(&group.catch_type);
            let binds_error = matches!(&catch_type, Type::Named(name) if name == "Error");
            frame_locals.push((
                group.storage_name.clone(),
                catch_type,
                AsyncLocalStorage::OwnedValue,
            ));
            if binds_error {
                frame_locals.push((
                    Self::kept_error_tag_local(&group.storage_name),
                    Type::I64,
                    AsyncLocalStorage::None,
                ));
                for name in self.kept_error_subtype_names() {
                    frame_locals.push((
                        Self::kept_error_subtype_local(&group.storage_name, &name),
                        Type::Named(name),
                        AsyncLocalStorage::OwnedValue,
                    ));
                }
            }
        }
        for group in &finally_await_groups {
            frame_locals.push((
                group.mode_storage.clone(),
                Type::I32,
                AsyncLocalStorage::None,
            ));
            frame_locals.push((
                group.failure_storage.clone(),
                Type::Apply {
                    name: "Promise".to_string(),
                    args: vec![Type::Void],
                },
                AsyncLocalStorage::None,
            ));
            if let Some(completion) = &group.completion {
                frame_locals.push((
                    completion.mode_storage.clone(),
                    Type::I32,
                    AsyncLocalStorage::None,
                ));
                frame_locals.push((
                    completion.throw_storage.clone(),
                    Type::Str,
                    AsyncLocalStorage::OwnedValue,
                ));
                if !matches!(transparent_codegen_type(&payload_type), Type::Void) {
                    frame_locals.push((
                        completion.return_storage.clone(),
                        payload_type.clone(),
                        if payload_type.is_copy() {
                            AsyncLocalStorage::None
                        } else {
                            AsyncLocalStorage::OwnedValue
                        },
                    ));
                }
            }
        }
        let mut expression_temporaries = Vec::new();
        for site in &await_sites {
            let (span, ty, prefixes) = match &site.kind {
                AsyncAwaitKind::Expression {
                    span, ty, prefixes, ..
                } => (span, ty, prefixes),
                AsyncAwaitKind::SwitchCaseExpression {
                    span,
                    payload_type,
                    prefixes,
                    ..
                } => (span, payload_type, prefixes),
                _ => continue,
            };
            for (expression, prefix_type) in prefixes {
                if !expression_temporaries
                    .iter()
                    .any(|(existing, _)| existing == &expression.span)
                {
                    expression_temporaries.push((expression.span, prefix_type.clone()));
                }
            }
            if !expression_temporaries
                .iter()
                .any(|(existing, _)| existing == span)
            {
                expression_temporaries.push((*span, ty.clone()));
            }
        }
        let mut for_of_awaits = Vec::new();
        for site in &await_sites {
            if let Some(contexts) = control_flow_contexts(site) {
                for (depth, _) in contexts
                    .iter()
                    .enumerate()
                    .filter(|(_, context)| matches!(context, AsyncControlContext::ForOf { .. }))
                {
                    let key = AsyncForOfSlotKey::RecursiveControl(site.outer_index, depth);
                    if !for_of_awaits.contains(&key) {
                        for_of_awaits.push(key);
                    }
                }
                continue;
            }
            let key = match &site.kind {
                AsyncAwaitKind::ForOfBody { .. } => {
                    Some(AsyncForOfSlotKey::Statement(site.outer_index))
                }
                AsyncAwaitKind::IfBranch {
                    control_context: Some(AsyncControlContext::ForOf { .. }),
                    ..
                } => Some(AsyncForOfSlotKey::Statement(site.outer_index)),
                AsyncAwaitKind::SwitchSubject { context, .. }
                | AsyncAwaitKind::SwitchCaseExpression { context, .. }
                | AsyncAwaitKind::Switch { context, .. } => context
                    .as_ref()
                    .filter(|context| matches!(context, AsyncSwitchContext::ForOf { .. }))
                    .map(|_| AsyncForOfSlotKey::SwitchContext(site.outer_index)),
                _ => None,
            };
            if let Some(key) = key
                && !for_of_awaits.contains(&key)
            {
                for_of_awaits.push(key);
            }
            if let AsyncAwaitKind::Switch {
                await_clause,
                entry_guard: Some(guard),
                ..
            } = &site.kind
                && guard.for_of.is_some()
            {
                let key = AsyncForOfSlotKey::SwitchClause(site.outer_index, *await_clause);
                if !for_of_awaits.contains(&key) {
                    for_of_awaits.push(key);
                }
            }
        }
        self.async_prescan_types = previous_prescan;
        let owns_frame_local = frame_locals
            .iter()
            .any(|(_, _, storage)| !matches!(storage, AsyncLocalStorage::None));
        let frame_type = (!await_sites.is_empty()
            || !catch_await_groups.is_empty()
            || !finally_await_groups.is_empty()
            || !frame_params.is_empty()
            || owns_frame_local)
            .then(|| {
                let mut fields: Vec<BasicTypeEnum<'ctx>> =
                    vec![self.context.i32_type().into(), pointer.into()];
                fields.extend(
                    frame_params
                        .iter()
                        .map(|parameter| self.llvm_type(&parameter.ty)),
                );
                fields.extend(frame_locals.iter().map(|(_, ty, _)| self.llvm_type(ty)));
                fields.extend(
                    expression_temporaries
                        .iter()
                        .map(|(_, ty)| self.llvm_type(ty)),
                );
                for _ in &for_of_awaits {
                    fields.push(pointer.into());
                    fields.push(self.context.i64_type().into());
                    fields.push(self.context.i64_type().into());
                }
                fields.extend(
                    frame_locals
                        .iter()
                        .filter(|(_, _, storage)| {
                            matches!(storage, AsyncLocalStorage::TrackedString)
                        })
                        .map(|_| BasicTypeEnum::IntType(self.context.bool_type())),
                );
                fields.extend(
                    frame_locals
                        .iter()
                        .filter(|(_, _, storage)| {
                            matches!(storage, AsyncLocalStorage::TrackedStringArray)
                        })
                        .map(|_| BasicTypeEnum::IntType(self.context.i8_type())),
                );
                fields.extend(
                    frame_locals
                        .iter()
                        .filter(|(_, _, storage)| matches!(storage, AsyncLocalStorage::OwnedValue))
                        .map(|_| BasicTypeEnum::IntType(self.context.bool_type())),
                );
                self.context.struct_type(&fields, false)
            });
        if let Some(frame_type) = frame_type {
            let AsyncFrameStorage {
                frame,
                async_expression_slots,
                async_for_of_slots,
            } = self.bind_async_frame_storage(
                poll,
                frame_type,
                &frame_params,
                &frame_locals,
                &expression_temporaries,
                &for_of_awaits,
            )?;
            self.emit_async_state_machine(AsyncStateMachine {
                name,
                params,
                body,
                payload_type: &payload_type,
                await_sites: &await_sites,
                catch_await_groups: &catch_await_groups,
                finally_await_groups: &finally_await_groups,
                poll,
                frame_type,
                frame,
                async_result,
                async_expression_slots: &async_expression_slots,
                async_for_of_slots: &async_for_of_slots,
            })?;
        } else {
            self.async_frame_slots.push(HashMap::default());
            self.async_frame_string_ownership.push(HashMap::default());
            self.async_frame_tracked_copy_arrays
                .push(HashSet::default());
            self.async_frame_string_array_ownership
                .push(HashMap::default());
            self.async_frame_owned_values.push(HashSet::default());
            self.async_frame_owned_initialization
                .push(HashMap::default());
            self.async_frame_local_storage.push(HashMap::default());
            for stmt in body {
                self.emit_stmt(stmt, poll)?;
            }
        }
        if !self.current_block_terminated() {
            self.emit_owned_closure_cleanups(None)?;
            if matches!(payload_type, Type::Void) {
                self.emit_async_void_ready(async_result);
            } else {
                self.builder.build_unreachable().unwrap();
            }
        }
        self.async_frame_slots.pop();
        self.async_frame_string_ownership.pop();
        self.async_frame_tracked_copy_arrays.pop();
        self.async_frame_string_array_ownership.pop();
        self.async_frame_owned_values.pop();
        self.async_frame_owned_initialization.pop();
        self.async_frame_local_storage.pop();
        self.async_results.pop();
        self.async_await_values.pop();
        self.return_types.pop();
        self.pop_throw_frame();
        self.pop_scope();
        self.emit_async_task_wrapper(AsyncTaskWrapper {
            name,
            guards_throws,
            poll,
            frame_type,
            pointer,
            frame_params: &frame_params,
            frame_locals: &frame_locals,
            expression_temporaries: &expression_temporaries,
            for_of_await_count: for_of_awaits.len(),
        })?;
        Ok(())
    }

    pub(crate) fn emit_async_void_ready(&mut self, result: PointerValue<'ctx>) {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let fulfill = self.declare_libc_fn(
            "tyrus_promise_fulfill",
            &[pointer.into(), pointer.into(), pointer.into()],
            Some(self.context.i32_type().into()),
        );
        self.builder
            .build_call(
                fulfill,
                &[
                    result.into(),
                    pointer.const_null().into(),
                    pointer.const_null().into(),
                ],
                "async.fulfill.void",
            )
            .unwrap();
        self.builder
            .build_return(Some(&self.context.i32_type().const_int(1, false)))
            .unwrap();
    }

    pub(crate) fn emit_async_value_ready(
        &mut self,
        result: PointerValue<'ctx>,
        value: BasicValueEnum<'ctx>,
        value_type: &Type,
        source: &Expr,
    ) {
        self.emit_async_value_ready_with_ownership(result, value, value_type, Some(source));
    }

    pub(crate) fn emit_async_transferred_value_ready(
        &mut self,
        result: PointerValue<'ctx>,
        value: BasicValueEnum<'ctx>,
        value_type: &Type,
    ) {
        self.emit_async_value_ready_with_ownership(result, value, value_type, None);
    }

    pub(crate) fn emit_async_empty_optional_ready(
        &mut self,
        result: PointerValue<'ctx>,
        payload_type: &Type,
    ) {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        self.emit_async_value_ready_with_ownership(
            result,
            pointer.const_null().into(),
            payload_type,
            None,
        );
    }

    pub(super) fn emit_async_value_ready_with_ownership(
        &mut self,
        result: PointerValue<'ctx>,
        value: BasicValueEnum<'ctx>,
        value_type: &Type,
        source: Option<&Expr>,
    ) {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let payload = self
            .heap_alloc(self.llvm_type(value_type), "async.payload")
            .unwrap();
        if let Some(source) = source {
            self.emit_async_clone_value_into(payload, value, value_type)
                .unwrap();
            self.emit_async_cleanup_source_expression(value, value_type, source)
                .unwrap();
        } else {
            self.builder.build_store(payload, value).unwrap();
        }
        let drop_symbol = match transparent_codegen_type(value_type) {
            Type::Str => "tyrus_drop_async_string",
            Type::Array(element) if element.is_copy() => "tyrus_drop_async_copy_array",
            Type::Array(element) if matches!(transparent_codegen_type(element), Type::Str) => {
                "tyrus_drop_async_string_array"
            }
            Type::Array(_)
            | Type::Optional(_)
            | Type::Result { .. }
            | Type::Named(_)
            | Type::Object(_) => {
                let drop_payload = self.emit_async_object_payload_drop(value_type).unwrap();
                // `_owned`, not the plain `tyrus_promise_fulfill`: a task
                // cancelled at the exact moment it settles (`race`/`timeout`
                // cancelling the loser right as it finishes, confirmed with
                // valgrind) makes `fulfill` a no-op, the promise is no
                // longer `Pending`, and the box built just above it would
                // otherwise never be freed. `_owned` frees it itself whenever
                // the transition does not go through.
                let fulfill = self.declare_libc_fn(
                    "tyrus_promise_fulfill_owned",
                    &[pointer.into(), pointer.into(), pointer.into()],
                    Some(self.context.i32_type().into()),
                );
                self.builder
                    .build_call(
                        fulfill,
                        &[
                            result.into(),
                            payload.into(),
                            drop_payload.as_global_value().as_pointer_value().into(),
                        ],
                        "async.fulfill.value",
                    )
                    .unwrap();
                self.builder
                    .build_return(Some(&self.context.i32_type().const_int(1, false)))
                    .unwrap();
                return;
            }
            _ => "tyrus_drop_async_box",
        };
        let drop_payload = self.declare_libc_fn(drop_symbol, &[pointer.into()], None);
        let fulfill = self.declare_libc_fn(
            "tyrus_promise_fulfill_owned",
            &[pointer.into(), pointer.into(), pointer.into()],
            Some(self.context.i32_type().into()),
        );
        self.builder
            .build_call(
                fulfill,
                &[
                    result.into(),
                    payload.into(),
                    drop_payload.as_global_value().as_pointer_value().into(),
                ],
                "async.fulfill.value",
            )
            .unwrap();
        self.builder
            .build_return(Some(&self.context.i32_type().const_int(1, false)))
            .unwrap();
    }
}
