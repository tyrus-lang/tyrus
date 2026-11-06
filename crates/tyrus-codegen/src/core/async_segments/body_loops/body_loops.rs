use super::*;

impl<'ctx> Codegen<'ctx> {
    /// Emitting the body of a loop that awaits inside it: `for`/`of`, `for`, `do`/`while` and `while`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_async_body_loop(
        &mut self,
        context: AsyncBody<'_, 'ctx>,
        blocks: &mut AsyncBlockRegistry<'ctx>,
    ) -> Result<bool, String> {
        let AsyncBody {
            segment_index,
            await_site,
            await_sites,
            catch_await_groups,
            finally_await_groups,
            poll,
            frame_type,
            frame,
            state_pointer,
            async_for_of_slots,
            pointer,
            ..
        } = context;
        if let AsyncAwaitKind::ForOfBody {
            name,
            iterable,
            element_type,
            before,
            first_in_body,
            ..
        } = &await_site.kind
        {
            let (condition, update, after) = if *first_in_body {
                let (data_slot, length_slot, index_slot) = async_for_of_slots
                    .get(&AsyncForOfSlotKey::Statement(await_site.outer_index))
                    .copied()
                    .ok_or_else(|| {
                        "internal: async for...of frame slots are missing".to_string()
                    })?;
                let array = self.emit_expr(iterable)?.into_struct_value();
                let data = self
                    .builder
                    .build_extract_value(array, 0, "async.for_of.source.data")
                    .unwrap();
                let length = self
                    .builder
                    .build_extract_value(array, 1, "async.for_of.source.length")
                    .unwrap();
                self.builder.build_store(data_slot, data).unwrap();
                self.builder.build_store(length_slot, length).unwrap();
                self.builder
                    .build_store(index_slot, self.context.i64_type().const_zero())
                    .unwrap();
                let condition = self
                    .context
                    .append_basic_block(poll, "async.for_of.condition");
                let loop_body = self
                    .context
                    .append_basic_block(poll, "async.for_of.selected");
                let update = self.context.append_basic_block(poll, "async.for_of.update");
                let after = self.context.append_basic_block(poll, "async.for_of.after");
                blocks
                    .for_of_blocks
                    .insert(await_site.outer_index, (condition, update, after));
                self.builder.build_unconditional_branch(condition).unwrap();
                self.builder.position_at_end(condition);
                let index = self
                    .builder
                    .build_load(
                        self.context.i64_type(),
                        index_slot,
                        "async.for_of.condition.index",
                    )
                    .unwrap()
                    .into_int_value();
                let length = self
                    .builder
                    .build_load(
                        self.context.i64_type(),
                        length_slot,
                        "async.for_of.condition.length",
                    )
                    .unwrap()
                    .into_int_value();
                let has_next = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::ULT,
                        index,
                        length,
                        "async.for_of.has_next",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(has_next, loop_body, after)
                    .unwrap();
                self.builder.position_at_end(loop_body);
                let data = self
                    .builder
                    .build_load(pointer, data_slot, "async.for_of.data.load")
                    .unwrap()
                    .into_pointer_value();
                let element_pointer = unsafe {
                    self.builder
                        .build_gep(
                            self.llvm_type(element_type),
                            data,
                            &[index],
                            "async.for_of.element.pointer",
                        )
                        .unwrap()
                };
                let element = self
                    .builder
                    .build_load(
                        self.llvm_type(element_type),
                        element_pointer,
                        "async.for_of.element",
                    )
                    .unwrap();
                let (binding_slot, _) = self.async_frame_slot(name).ok_or_else(|| {
                    format!("internal: async for...of binding `{name}` has no frame slot")
                })?;
                self.builder.build_store(binding_slot, element).unwrap();
                self.push_scope();
                self.store_var(name, binding_slot, element_type.clone());
                (condition, update, after)
            } else {
                blocks
                    .for_of_blocks
                    .get(&await_site.outer_index)
                    .copied()
                    .ok_or_else(|| {
                        "internal: async for...of continuation was not initialized".to_string()
                    })?
            };
            let _ = condition;
            self.push_loop_control(update, after);
            for statement in before {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
            self.pop_loop_control();
            if self.current_block_terminated() {
                return Ok(true);
            }
        } else if let AsyncAwaitKind::ForBody {
            init,
            cond,
            before,
            first_in_body,
            ..
        } = &await_site.kind
        {
            let (loop_body, update, _condition, after) = if *first_in_body {
                let loop_body = self
                    .context
                    .append_basic_block(poll, "async.for_body.selected");
                let update = self
                    .context
                    .append_basic_block(poll, "async.for_body.update");
                let condition = self
                    .context
                    .append_basic_block(poll, "async.for_body.condition");
                let after = self
                    .context
                    .append_basic_block(poll, "async.for_body.after");
                blocks.for_body_blocks.insert(
                    await_site.outer_index,
                    (loop_body, update, condition, after),
                );
                self.push_scope();
                self.emit_stmt(init, poll)?;
                if self.current_block_terminated() {
                    return Ok(true);
                }
                self.builder.build_unconditional_branch(condition).unwrap();
                self.builder.position_at_end(condition);
                let condition_value = self.emit_condition_value(cond)?;
                self.builder
                    .build_conditional_branch(condition_value, loop_body, after)
                    .unwrap();
                self.builder.position_at_end(loop_body);
                (loop_body, update, condition, after)
            } else {
                blocks
                    .for_body_blocks
                    .get(&await_site.outer_index)
                    .copied()
                    .ok_or_else(|| {
                        "internal: async for-body continuation was not initialized".to_string()
                    })?
            };
            let _ = loop_body;
            if *first_in_body {
                self.push_scope();
            }
            self.push_loop_control(update, after);
            for statement in before {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
            self.pop_loop_control();
            if self.current_block_terminated() {
                return Ok(true);
            }
        } else if let AsyncAwaitKind::DoWhileBody {
            before,
            first_in_body,
            ..
        } = &await_site.kind
        {
            let (_loop_body, condition, after) = if *first_in_body {
                let loop_body = self
                    .context
                    .append_basic_block(poll, "async.do_while_body.selected");
                let condition = self
                    .context
                    .append_basic_block(poll, "async.do_while_body.condition");
                let after = self
                    .context
                    .append_basic_block(poll, "async.do_while_body.after");
                blocks
                    .do_while_body_blocks
                    .insert(await_site.outer_index, (loop_body, condition, after));
                self.builder.build_unconditional_branch(loop_body).unwrap();
                self.builder.position_at_end(loop_body);
                (loop_body, condition, after)
            } else {
                blocks
                    .do_while_body_blocks
                    .get(&await_site.outer_index)
                    .copied()
                    .ok_or_else(|| {
                        "internal: async do/while-body continuation was not initialized".to_string()
                    })?
            };
            if *first_in_body {
                self.push_scope();
            }
            self.push_loop_control(condition, after);
            for statement in before {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
            self.pop_loop_control();
            if self.current_block_terminated() {
                return Ok(true);
            }
        } else if let AsyncAwaitKind::WhileBody {
            cond,
            before,
            first_in_body,
            ..
        } = &await_site.kind
        {
            let (condition, after) = if *first_in_body {
                let condition = self
                    .context
                    .append_basic_block(poll, "async.while_body.condition");
                let selected = self
                    .context
                    .append_basic_block(poll, "async.while_body.selected");
                let after = self
                    .context
                    .append_basic_block(poll, "async.while_body.after");
                blocks
                    .while_body_blocks
                    .insert(await_site.outer_index, (condition, after));
                self.builder.build_unconditional_branch(condition).unwrap();
                self.builder.position_at_end(condition);
                let condition_value = self.emit_condition_value(cond)?;
                self.builder
                    .build_conditional_branch(condition_value, selected, after)
                    .unwrap();
                self.builder.position_at_end(selected);
                (condition, after)
            } else {
                blocks
                    .while_body_blocks
                    .get(&await_site.outer_index)
                    .copied()
                    .ok_or_else(|| {
                        "internal: async while-body continuation was not initialized".to_string()
                    })?
            };
            if *first_in_body {
                self.push_scope();
            }
            self.push_loop_control(condition, after);
            for statement in before {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
            self.pop_loop_control();
            if self.current_block_terminated() {
                return Ok(true);
            }
        } else if let AsyncAwaitKind::IfBranch {
            cond,
            before,
            other,
            await_in_then,
            first_in_branch,
            try_context,
            control_context,
            ..
        } = &await_site.kind
        {
            if *first_in_branch {
                if let Some(context) = control_context {
                    let loop_body = self.context.append_basic_block(poll, "async.control.body");
                    let loop_condition = self
                        .context
                        .append_basic_block(poll, "async.control.condition");
                    let loop_continue = match context {
                        AsyncControlContext::For { .. } | AsyncControlContext::ForOf { .. } => self
                            .context
                            .append_basic_block(poll, "async.control.update"),
                        _ => loop_condition,
                    };
                    let loop_after = self.context.append_basic_block(poll, "async.control.after");
                    blocks.control_body_blocks.insert(
                        await_site.outer_index,
                        (loop_body, loop_continue, loop_condition, loop_after),
                    );
                    self.push_scope();
                    match context {
                        AsyncControlContext::While { condition, .. } => {
                            self.builder
                                .build_unconditional_branch(loop_condition)
                                .unwrap();
                            self.builder.position_at_end(loop_condition);
                            let value = self.emit_condition_value(condition)?;
                            self.builder
                                .build_conditional_branch(value, loop_body, loop_after)
                                .unwrap();
                        }
                        AsyncControlContext::DoWhile { .. } => {
                            self.builder.build_unconditional_branch(loop_body).unwrap();
                        }
                        AsyncControlContext::For {
                            init, condition, ..
                        } => {
                            self.emit_stmt(init, poll)?;
                            if self.current_block_terminated() {
                                return Ok(true);
                            }
                            self.bind_async_frame_locals();
                            self.builder
                                .build_unconditional_branch(loop_condition)
                                .unwrap();
                            self.builder.position_at_end(loop_condition);
                            let value = self.emit_condition_value(condition)?;
                            self.builder
                                .build_conditional_branch(value, loop_body, loop_after)
                                .unwrap();
                        }
                        AsyncControlContext::ForOf {
                            name,
                            iterable,
                            element_type,
                            ..
                        } => {
                            let (data_slot, length_slot, index_slot) = async_for_of_slots
                                .get(&AsyncForOfSlotKey::Statement(await_site.outer_index))
                                .copied()
                                .ok_or_else(|| {
                                    "internal: async nested for...of frame slots are missing"
                                        .to_string()
                                })?;
                            let array = self.emit_expr(iterable)?.into_struct_value();
                            let data = self
                                .builder
                                .build_extract_value(array, 0, "async.control.for_of.data")
                                .unwrap();
                            let length = self
                                .builder
                                .build_extract_value(array, 1, "async.control.for_of.length")
                                .unwrap();
                            self.builder.build_store(data_slot, data).unwrap();
                            self.builder.build_store(length_slot, length).unwrap();
                            self.builder
                                .build_store(index_slot, self.context.i64_type().const_zero())
                                .unwrap();
                            self.builder
                                .build_unconditional_branch(loop_condition)
                                .unwrap();
                            self.builder.position_at_end(loop_condition);
                            let index = self
                                .builder
                                .build_load(
                                    self.context.i64_type(),
                                    index_slot,
                                    "async.control.for_of.index",
                                )
                                .unwrap()
                                .into_int_value();
                            let length = self
                                .builder
                                .build_load(
                                    self.context.i64_type(),
                                    length_slot,
                                    "async.control.for_of.length",
                                )
                                .unwrap()
                                .into_int_value();
                            let has_next = self
                                .builder
                                .build_int_compare(
                                    inkwell::IntPredicate::ULT,
                                    index,
                                    length,
                                    "async.control.for_of.has_next",
                                )
                                .unwrap();
                            self.builder
                                .build_conditional_branch(has_next, loop_body, loop_after)
                                .unwrap();
                            self.builder.position_at_end(loop_body);
                            let data = self
                                .builder
                                .build_load(pointer, data_slot, "async.control.for_of.data.load")
                                .unwrap()
                                .into_pointer_value();
                            let element_pointer = unsafe {
                                self.builder
                                    .build_gep(
                                        self.llvm_type(element_type),
                                        data,
                                        &[index],
                                        "async.control.for_of.element.pointer",
                                    )
                                    .unwrap()
                            };
                            let element = self
                                .builder
                                .build_load(
                                    self.llvm_type(element_type),
                                    element_pointer,
                                    "async.control.for_of.element",
                                )
                                .unwrap();
                            let (binding_slot, _) = self.async_frame_slot(name).ok_or_else(|| {
                                format!("internal: async nested for...of binding `{name}` has no frame slot")
                            })?;
                            self.builder.build_store(binding_slot, element).unwrap();
                            self.store_var(name, binding_slot, element_type.clone());
                        }
                        AsyncControlContext::Block { .. }
                        | AsyncControlContext::If { .. }
                        | AsyncControlContext::Try { .. } => {
                            unreachable!(
                                "legacy async control context cannot contain blocks or ifs"
                            )
                        }
                    }
                    self.builder.position_at_end(loop_body);
                    self.push_loop_control(loop_continue, loop_after);
                    let outer_before = match context {
                        AsyncControlContext::While { before, .. }
                        | AsyncControlContext::DoWhile { before, .. }
                        | AsyncControlContext::For { before, .. }
                        | AsyncControlContext::ForOf { before, .. } => before,
                        AsyncControlContext::Block { .. }
                        | AsyncControlContext::If { .. }
                        | AsyncControlContext::Try { .. } => {
                            unreachable!(
                                "legacy async control context cannot contain blocks or ifs"
                            )
                        }
                    };
                    for statement in outer_before {
                        if self.current_block_terminated() {
                            break;
                        }
                        self.emit_stmt(statement, poll)?;
                    }
                    self.pop_loop_control();
                    if self.current_block_terminated() {
                        return Ok(true);
                    }
                }
                let selected = self.context.append_basic_block(poll, "async.if.selected");
                let alternative = self
                    .context
                    .append_basic_block(poll, "async.if.alternative");
                let join = self.context.append_basic_block(poll, "async.if.after");
                blocks.if_blocks.insert(await_site.outer_index, join);
                if try_context.is_some() {
                    let recovered = finally_await_groups
                        .iter()
                        .any(|group| group.outer_index == await_site.outer_index)
                        .then(|| self.context.append_basic_block(poll, "async.try.recovered"))
                        .unwrap_or(join);
                    blocks.try_blocks.insert(await_site.outer_index, recovered);
                }
                let condition = self.emit_condition_value(cond)?;
                let (then_block, else_block) = if *await_in_then {
                    (selected, alternative)
                } else {
                    (alternative, selected)
                };
                self.builder
                    .build_conditional_branch(condition, then_block, else_block)
                    .unwrap();
                self.builder.position_at_end(alternative);
                self.push_scope();
                self.apply_condition_narrowing(cond, !*await_in_then);
                if control_context.is_some() {
                    let (_, condition, _, after) = blocks
                        .control_body_blocks
                        .get(&await_site.outer_index)
                        .copied()
                        .ok_or_else(|| {
                            "internal: async nested control continuation was not initialized"
                                .to_string()
                        })?;
                    self.push_loop_control(condition, after);
                }
                for statement in other {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, poll)?;
                }
                if control_context.is_some() {
                    self.pop_loop_control();
                }
                self.pop_scope();
                if !self.current_block_terminated() {
                    self.builder.build_unconditional_branch(join).unwrap();
                }
                self.builder.position_at_end(selected);
            }
            if *first_in_branch {
                self.push_scope();
                self.apply_condition_narrowing(cond, *await_in_then);
            }
            if control_context.is_some() {
                let (_, condition, _, after) = blocks
                    .control_body_blocks
                    .get(&await_site.outer_index)
                    .copied()
                    .ok_or_else(|| {
                        "internal: async nested control continuation was not initialized"
                            .to_string()
                    })?;
                self.push_loop_control(condition, after);
            }
            for statement in before {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
            if control_context.is_some() {
                self.pop_loop_control();
            }
            if self.current_block_terminated() {
                return Ok(true);
            }
        } else if let AsyncAwaitKind::IfBothBranches {
            contexts,
            enter_depth,
            if_id,
            cond,
            first_in_if,
            try_context,
            ..
        } = &await_site.kind
        {
            if !first_in_if {
                return Ok(true);
            }
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
            let then_entry = self.context.append_basic_block(poll, "async.if.selected");
            let else_entry = self
                .context
                .append_basic_block(poll, "async.if.alternative");
            let join = self.context.append_basic_block(poll, "async.if.after");
            blocks.if_branch_path_blocks.insert(*if_id, join);
            if try_context.is_some() {
                let recovered = finally_await_groups
                    .iter()
                    .any(|group| group.outer_index == await_site.outer_index)
                    .then(|| self.context.append_basic_block(poll, "async.try.recovered"))
                    .unwrap_or(join);
                blocks.try_blocks.insert(await_site.outer_index, recovered);
            }
            let condition = self.emit_condition_value(cond)?;
            self.builder
                .build_conditional_branch(condition, then_entry, else_entry)
                .unwrap();
            for (entry, selected_then) in [(then_entry, true), (else_entry, false)] {
                let (site_state, selected_site) = await_sites
                    .iter()
                    .enumerate()
                    .find(|(_, candidate)| {
                        matches!(
                            candidate.kind,
                            AsyncAwaitKind::IfBothBranches {
                                if_id: candidate_id,
                                in_then,
                                first_in_branch: true,
                                ..
                            } if candidate_id == *if_id && in_then == selected_then
                        )
                    })
                    .map(|(index, site)| (index + 1, site))
                    .ok_or_else(|| {
                        "internal: async branch has no initial continuation state".to_string()
                    })?;
                let AsyncAwaitKind::IfBothBranches { before, .. } = &selected_site.kind else {
                    unreachable!()
                };
                self.builder.position_at_end(entry);
                self.push_scope();
                for statement in before {
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
                        site_state as u64,
                        &selected_site.dependency,
                    )?;
                }
                self.pop_scope();
            }
            return Ok(true);
        } else if let AsyncAwaitKind::DoWhileCondition {
            body, condition, ..
        } = &await_site.kind
        {
            let loop_body = self.context.append_basic_block(poll, "async.do_while.body");
            let schedule = self
                .context
                .append_basic_block(poll, "async.do_while.schedule");
            let after = condition
                .as_ref()
                .filter(|condition| condition.guard.is_some())
                .and_then(|_| blocks.expression_guard_blocks.get(&segment_index).copied())
                .unwrap_or_else(|| {
                    self.context
                        .append_basic_block(poll, "async.do_while.after")
                });
            blocks
                .loop_blocks
                .insert(segment_index, (loop_body, schedule, after));
            self.builder.build_unconditional_branch(loop_body).unwrap();
            self.builder.position_at_end(loop_body);
            self.push_scope();
            self.push_loop_control(schedule, after);
            for statement in body {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
            self.pop_loop_control();
            self.pop_scope();
            if !self.current_block_terminated() {
                self.builder.build_unconditional_branch(schedule).unwrap();
            }
            self.builder.position_at_end(schedule);
        } else if let AsyncAwaitKind::ForCondition {
            init, condition, ..
        } = &await_site.kind
        {
            let loop_body = self.context.append_basic_block(poll, "async.for.body");
            let update = self.context.append_basic_block(poll, "async.for.update");
            let schedule = self.context.append_basic_block(poll, "async.for.schedule");
            let after = condition
                .as_ref()
                .filter(|condition| condition.guard.is_some())
                .and_then(|_| blocks.expression_guard_blocks.get(&segment_index).copied())
                .unwrap_or_else(|| self.context.append_basic_block(poll, "async.for.after"));
            blocks
                .for_blocks
                .insert(segment_index, (loop_body, update, schedule, after));
            self.push_scope();
            self.emit_stmt(init, poll)?;
            if self.current_block_terminated() {
                return Ok(true);
            }
            self.builder.build_unconditional_branch(schedule).unwrap();
            self.builder.position_at_end(schedule);
        }
        Ok(false)
    }
}
