use super::*;

#[path = "enum_patterns/enum_patterns.rs"]
mod enum_patterns;
#[path = "literal_patterns/literal_patterns.rs"]
mod literal_patterns;
#[path = "object_patterns/object_patterns.rs"]
mod object_patterns;
impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_while(
        &mut self,
        cond: &Expr,
        body: &[Stmt],
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let cond_bb = self.context.append_basic_block(fn_val, "while.cond");
        let body_bb = self.context.append_basic_block(fn_val, "while.body");
        let end_bb = self.context.append_basic_block(fn_val, "while.end");

        self.builder.build_unconditional_branch(cond_bb).unwrap();
        self.builder.position_at_end(cond_bb);
        let cond_val = self.emit_condition_value(cond)?;
        self.discard_owned_native_result();
        self.builder
            .build_conditional_branch(cond_val, body_bb, end_bb)
            .unwrap();

        self.builder.position_at_end(body_bb);
        self.push_scope();
        self.push_loop_control(cond_bb, end_bb);
        for stmt in body {
            if self.current_block_terminated() {
                break;
            }
            self.emit_stmt(stmt, fn_val)?;
        }
        self.pop_loop_control();
        self.pop_scope();
        if !self.current_block_terminated() {
            self.builder.build_unconditional_branch(cond_bb).unwrap();
        }

        self.builder.position_at_end(end_bb);
        Ok(())
    }

    pub(super) fn emit_for(
        &mut self,
        init: &Stmt,
        cond: &Expr,
        update: &Stmt,
        body: &[Stmt],
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        self.push_scope();
        self.for_index_hint = self.for_index_hint(init, cond);
        self.emit_stmt(init, fn_val)?;
        self.for_index_hint = None;

        let cond_bb = self.context.append_basic_block(fn_val, "for.cond");
        let body_bb = self.context.append_basic_block(fn_val, "for.body");
        let update_bb = self.context.append_basic_block(fn_val, "for.update");
        let end_bb = self.context.append_basic_block(fn_val, "for.end");

        self.builder.build_unconditional_branch(cond_bb).unwrap();
        self.builder.position_at_end(cond_bb);
        let cond_val = self.emit_condition_value(cond)?;
        self.discard_owned_native_result();
        self.builder
            .build_conditional_branch(cond_val, body_bb, end_bb)
            .unwrap();

        self.builder.position_at_end(body_bb);
        self.push_scope();
        self.push_loop_control(update_bb, end_bb);
        for stmt in body {
            if self.current_block_terminated() {
                break;
            }
            self.emit_stmt(stmt, fn_val)?;
        }
        self.pop_loop_control();
        self.pop_scope();
        if !self.current_block_terminated() {
            self.builder.build_unconditional_branch(update_bb).unwrap();
        }

        self.builder.position_at_end(update_bb);
        self.emit_stmt(update, fn_val)?;
        if !self.current_block_terminated() {
            self.builder.build_unconditional_branch(cond_bb).unwrap();
        }

        self.builder.position_at_end(end_bb);
        self.pop_scope();
        Ok(())
    }

    pub(super) fn emit_for_of(
        &mut self,
        name: &str,
        iterable: &Expr,
        body: &[Stmt],
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let array_type = self
            .expr_type_hint(iterable)
            .ok_or_else(|| "error: cannot infer array type for for...of".to_string())?;
        if let Type::Range(element) = transparent_codegen_type(&array_type) {
            let element = element.as_ref().clone();
            let range_value = self.emit_expr(iterable)?.into_struct_value();
            return self.emit_for_of_range(name, range_value, element, body, fn_val);
        }
        if let Some(slice) = self.tuple_slice(iterable, "forof_tuple")? {
            return self.emit_for_of_indexed(
                name,
                slice.data,
                slice.element_llvm,
                slice.element_type,
                slice.length,
                body,
                fn_val,
            );
        }
        // A set iterates its values; a map iterates its `[K, V]` entries. Both
        // are materialized into an array and iterated the same way.
        //
        // Either shape can hand back a fresh allocation nothing has claimed
        // yet, and the loop body below runs arbitrary statements of its own,
        // each with the ordinary per-statement sweep that frees whatever
        // temporary is still pending once it finishes. Left registered (the
        // `.keys()`/`.values()` method-call case, claimed the moment
        // `emit_expr` returns) that sweep fired on the *first* iteration's own
        // statements, freeing the array data/length this loop had already
        // captured as plain SSA values, a genuine use-after-free on every
        // iteration after the first, reproduced by `for (const item of
        // stock.keys())`. Left unregistered (a `Set`/`Map` iterated directly,
        // materialized fresh by `emit_collection_array`/`emit_map_entries`
        // with no registration of their own) it was simply never freed at
        // all. Both are closed the same way: take whatever registration
        // exists (forgetting it before the loop body can sweep it early), or
        // supply the correct one when there is none to take, and release it
        // exactly once, after the loop has fully finished with it.
        let (array, element_type, release_type, always_owned) =
            if let Some((array, element)) = self.emit_collection_iterable(iterable, &array_type)? {
                let release_type = Type::Array(Box::new(element.clone()));
                (array.into_struct_value(), element, release_type, true)
            } else {
                let Type::Array(element) = transparent_codegen_type(&array_type) else {
                    return Err(format!(
                        "error: cannot iterate non-array type `{array_type}`"
                    ));
                };
                let element_type = self.resolve_type(element);
                let array = self.emit_expr(iterable)?.into_struct_value();
                (array, element_type, array_type.clone(), false)
            };
        let release_plan = if always_owned {
            self.forget_owned_native_result(array.into());
            Some(crate::ReleasePlan::Deep)
        } else {
            self.take_owned_native_result(array.into())
        };
        let element_llvm = self.llvm_type(&element_type);
        let data = self
            .builder
            .build_extract_value(array, 0, "forof_data")
            .unwrap()
            .into_pointer_value();
        let length = self
            .builder
            .build_extract_value(array, 1, "forof_length")
            .unwrap()
            .into_int_value();
        if let Some(plan) = release_plan.clone() {
            self.for_of_iterable_cleanups
                .push((array.into(), release_type.clone(), plan));
        }
        let result =
            self.emit_for_of_indexed(name, data, element_llvm, element_type, length, body, fn_val);
        if release_plan.is_some() {
            self.for_of_iterable_cleanups.pop();
        }
        result?;
        if let Some(plan) = release_plan {
            self.emit_owned_native_release(array.into(), &release_type, &plan);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_for_of_indexed(
        &mut self,
        name: &str,
        data: PointerValue<'ctx>,
        element_llvm: BasicTypeEnum<'ctx>,
        element_type: Type,
        length: IntValue<'ctx>,
        body: &[Stmt],
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let index_type = length.get_type();
        let index_slot = self
            .builder
            .build_alloca(index_type, "forof_index")
            .unwrap();
        self.builder
            .build_store(index_slot, index_type.const_zero())
            .unwrap();

        let cond_bb = self.context.append_basic_block(fn_val, "forof.cond");
        let body_bb = self.context.append_basic_block(fn_val, "forof.body");
        let update_bb = self.context.append_basic_block(fn_val, "forof.update");
        let end_bb = self.context.append_basic_block(fn_val, "forof.end");

        self.builder.build_unconditional_branch(cond_bb).unwrap();

        self.builder.position_at_end(cond_bb);
        let index = self
            .builder
            .build_load(index_type, index_slot, "forof_index_value")
            .unwrap()
            .into_int_value();
        let has_next = self
            .builder
            .build_int_compare(IntPredicate::ULT, index, length, "forof_has_next")
            .unwrap();
        self.builder
            .build_conditional_branch(has_next, body_bb, end_bb)
            .unwrap();

        self.builder.position_at_end(body_bb);
        self.push_scope();
        let current_index = self
            .builder
            .build_load(index_type, index_slot, "forof_current_index")
            .unwrap()
            .into_int_value();
        let element_slot = unsafe {
            self.builder
                .build_gep(element_llvm, data, &[current_index], "forof_element_ptr")
                .unwrap()
        };
        let element_value = self
            .builder
            .build_load(element_llvm, element_slot, name)
            .map_err(|error| error.to_string())?;
        let local = self.builder.build_alloca(element_llvm, name).unwrap();
        self.builder.build_store(local, element_value).unwrap();
        self.store_var(name, local, element_type);
        self.push_loop_control(update_bb, end_bb);
        for stmt in body {
            if self.current_block_terminated() {
                break;
            }
            self.emit_stmt(stmt, fn_val)?;
        }
        self.pop_loop_control();
        self.pop_scope();
        if !self.current_block_terminated() {
            self.builder.build_unconditional_branch(update_bb).unwrap();
        }

        self.builder.position_at_end(update_bb);
        let index = self
            .builder
            .build_load(index_type, index_slot, "forof_update_index")
            .unwrap()
            .into_int_value();
        let next = self
            .builder
            .build_int_add(index, index_type.const_int(1, false), "forof_next_index")
            .unwrap();
        self.builder.build_store(index_slot, next).unwrap();
        self.builder.build_unconditional_branch(cond_bb).unwrap();

        self.builder.position_at_end(end_bb);
        Ok(())
    }

    pub(super) fn emit_for_of_range(
        &mut self,
        name: &str,
        range_value: inkwell::values::StructValue<'ctx>,
        element: Type,
        body: &[Stmt],
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let element_llvm = self.llvm_type(&element);
        let start = self
            .builder
            .build_extract_value(range_value, 0, "forof_range_start")
            .unwrap();

        let index_slot = self
            .builder
            .build_alloca(element_llvm, "forof_range_index")
            .unwrap();
        self.builder.build_store(index_slot, start).unwrap();

        let cond_bb = self.context.append_basic_block(fn_val, "forof_range.cond");
        let body_bb = self.context.append_basic_block(fn_val, "forof_range.body");
        let update_bb = self
            .context
            .append_basic_block(fn_val, "forof_range.update");
        let end_bb = self.context.append_basic_block(fn_val, "forof_range.end");

        self.builder.build_unconditional_branch(cond_bb).unwrap();

        self.builder.position_at_end(cond_bb);
        let current = self
            .builder
            .build_load(element_llvm, index_slot, "forof_range_current")
            .map_err(|error| error.to_string())?;
        let has_next = self
            .emit_range_contains(range_value, current, &element)?
            .into_int_value();
        self.builder
            .build_conditional_branch(has_next, body_bb, end_bb)
            .unwrap();

        self.builder.position_at_end(body_bb);
        self.push_scope();
        let current_value = self
            .builder
            .build_load(element_llvm, index_slot, name)
            .map_err(|error| error.to_string())?;
        let local = self.builder.build_alloca(element_llvm, name).unwrap();
        self.builder.build_store(local, current_value).unwrap();
        self.store_var(name, local, element.clone());
        self.push_loop_control(update_bb, end_bb);
        for stmt in body {
            if self.current_block_terminated() {
                break;
            }
            self.emit_stmt(stmt, fn_val)?;
        }
        self.pop_loop_control();
        self.pop_scope();
        if !self.current_block_terminated() {
            self.builder.build_unconditional_branch(update_bb).unwrap();
        }

        self.builder.position_at_end(update_bb);
        let current = self
            .builder
            .build_load(element_llvm, index_slot, "forof_range_update")
            .map_err(|error| error.to_string())?;
        let next: BasicValueEnum<'ctx> = if current.is_float_value() {
            self.builder
                .build_float_add(
                    current.into_float_value(),
                    element_llvm.into_float_type().const_float(1.0),
                    "forof_range_next",
                )
                .unwrap()
                .into()
        } else {
            self.builder
                .build_int_add(
                    current.into_int_value(),
                    element_llvm.into_int_type().const_int(1, false),
                    "forof_range_next",
                )
                .unwrap()
                .into()
        };
        self.builder.build_store(index_slot, next).unwrap();
        self.builder.build_unconditional_branch(cond_bb).unwrap();

        self.builder.position_at_end(end_bb);
        Ok(())
    }

    pub(super) fn emit_for_in(
        &mut self,
        name: &str,
        object: &Expr,
        body: &[Stmt],
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let i64_ty = self.context.i64_type();

        let obj_ty = self
            .expr_type_hint(object)
            .ok_or_else(|| "error: cannot infer object type for for...in".to_string())?;
        let obj_ty = self.resolve_type(&obj_ty);
        let fields: Vec<String> = match &obj_ty {
            Type::Named(n) => self
                .interfaces
                .get(n.as_str())
                .map(|f| f.iter().map(|field| field.name.clone()).collect())
                .unwrap_or_default(),
            Type::Object(fields) => fields.iter().map(|f| f.name.clone()).collect(),
            _ => {
                return Err(format!(
                    "error: for...in requires an object type, found `{obj_ty}`"
                ));
            }
        };

        let count = fields.len() as u64;
        if count == 0 {
            return Ok(());
        }

        let key_ty = self.string_abi_type();
        let keys: Vec<StructValue<'ctx>> = fields
            .iter()
            .map(|f| self.const_string_abi(f, "forin.key").into_struct_value())
            .collect();

        let arr_ty = key_ty.array_type(count as u32);
        let arr_slot = self.builder.build_alloca(arr_ty, "forin.arr").unwrap();
        for (i, key) in keys.iter().enumerate() {
            let gep = unsafe {
                self.builder
                    .build_gep(
                        key_ty,
                        arr_slot,
                        &[i64_ty.const_int(i as u64, false)],
                        "forin.slot",
                    )
                    .unwrap()
            };
            self.builder.build_store(gep, *key).unwrap();
        }

        let index_slot = self.builder.build_alloca(i64_ty, "forin.index").unwrap();
        self.builder
            .build_store(index_slot, i64_ty.const_zero())
            .unwrap();

        let cond_bb = self.context.append_basic_block(fn_val, "forin.cond");
        let body_bb = self.context.append_basic_block(fn_val, "forin.body");
        let update_bb = self.context.append_basic_block(fn_val, "forin.update");
        let end_bb = self.context.append_basic_block(fn_val, "forin.end");

        self.builder.build_unconditional_branch(cond_bb).unwrap();

        self.builder.position_at_end(cond_bb);
        let index = self
            .builder
            .build_load(i64_ty, index_slot, "forin.idx")
            .unwrap()
            .into_int_value();
        let has_next = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                index,
                i64_ty.const_int(count, false),
                "forin.has_next",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(has_next, body_bb, end_bb)
            .unwrap();

        self.builder.position_at_end(body_bb);
        self.push_scope();
        let current_index = self
            .builder
            .build_load(i64_ty, index_slot, "forin.cur_idx")
            .unwrap()
            .into_int_value();
        let key_gep = unsafe {
            self.builder
                .build_gep(key_ty, arr_slot, &[current_index], "forin.key_gep")
                .unwrap()
        };
        let key_val = self.builder.build_load(key_ty, key_gep, name).unwrap();
        let key_slot = self.builder.build_alloca(key_ty, name).unwrap();
        self.builder.build_store(key_slot, key_val).unwrap();
        self.store_var(name, key_slot, Type::Str);

        self.push_loop_control(update_bb, end_bb);
        for stmt in body {
            if self.current_block_terminated() {
                break;
            }
            self.emit_stmt(stmt, fn_val)?;
        }
        self.pop_loop_control();
        self.pop_scope();
        if !self.current_block_terminated() {
            self.builder.build_unconditional_branch(update_bb).unwrap();
        }

        self.builder.position_at_end(update_bb);
        let idx = self
            .builder
            .build_load(i64_ty, index_slot, "forin.update_idx")
            .unwrap()
            .into_int_value();
        let next = self
            .builder
            .build_int_add(idx, i64_ty.const_int(1, false), "forin.next_idx")
            .unwrap();
        self.builder.build_store(index_slot, next).unwrap();
        self.builder.build_unconditional_branch(cond_bb).unwrap();

        self.builder.position_at_end(end_bb);
        Ok(())
    }

    pub(crate) fn emit_loop_expr(
        &mut self,
        expr: &Expr,
        body: &[Stmt],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: loop expression outside function")?;

        let cond_bb = self.context.append_basic_block(current_fn, "loop.cond");
        let body_bb = self.context.append_basic_block(current_fn, "loop.body");
        let end_bb = self.context.append_basic_block(current_fn, "loop.end");

        let value_type = self.expr_type_hint(expr).map(|ty| self.resolve_type(&ty));
        // Reserved in the entry block, not here. An `alloca` runs every time
        // control reaches it and its memory is only reclaimed when the function
        // returns, so a `loop` expression inside an enclosing loop was claiming
        // a fresh slot per iteration: `const v: i32 = loop { break i; };` under
        // a three-million-iteration `for` exhausted the stack and died of
        // SIGSEGV. The store that zeroes the slot stays in the flow, because
        // each execution of the loop must start from a clean value.
        let slot = value_type
            .map(|ty| -> Result<_, String> {
                let slot = self.entry_alloca(self.llvm_type(&ty), "loop.value")?;
                self.builder
                    .build_store(slot, self.llvm_type(&ty).const_zero())
                    .map_err(|error| error.to_string())?;
                Ok((slot, ty))
            })
            .transpose()?;

        self.builder.build_unconditional_branch(cond_bb).unwrap();
        self.builder.position_at_end(cond_bb);
        self.builder.build_unconditional_branch(body_bb).unwrap();

        self.builder.position_at_end(body_bb);
        self.push_scope();
        if let Some((slot, ty)) = &slot {
            self.push_value_loop_control(cond_bb, end_bb, *slot, ty.clone());
        } else {
            self.push_loop_control(cond_bb, end_bb);
        }
        for stmt in body {
            if self.current_block_terminated() {
                break;
            }
            self.emit_stmt(stmt, current_fn)?;
        }
        self.pop_loop_control();
        self.pop_scope();
        if !self.current_block_terminated() {
            self.builder.build_unconditional_branch(cond_bb).unwrap();
        }

        self.builder.position_at_end(end_bb);
        match slot {
            Some((slot, ty)) => {
                let value = self
                    .builder
                    .build_load(self.llvm_type(&ty), slot, "loop.value.load")
                    .map_err(|error| error.to_string())?;
                if self.async_owned_value_supported(&ty) {
                    self.record_owned_native_result_with(
                        value,
                        ty.clone(),
                        crate::ReleasePlan::Deep,
                    );
                }
                Ok(value)
            }
            None => Ok(self.context.i32_type().const_int(0, false).into()),
        }
    }

    pub(super) fn switch_jump_table_labels(
        &mut self,
        subject_type: &Type,
        cases: &[SwitchCase],
    ) -> Result<Option<Vec<IntValue<'ctx>>>, String> {
        if cases.is_empty() {
            return Ok(None);
        }
        if !cases
            .iter()
            .all(|case| self.switch_label_is_constant(&case.value))
        {
            return Ok(None);
        }

        let mut labels = Vec::with_capacity(cases.len());
        let mut seen = Vec::with_capacity(cases.len());
        for case in cases {
            let value = if is_integer_type(subject_type) {
                self.emit_expr_as(&case.value, subject_type)?
            } else {
                self.emit_expr(&case.value)?
            };
            let BasicValueEnum::IntValue(value) = value else {
                return Ok(None);
            };
            let Some(bits) = value.get_zero_extended_constant() else {
                return Ok(None);
            };
            if seen.contains(&bits) {
                return Ok(None);
            }
            seen.push(bits);
            labels.push(value);
        }

        Ok(Some(labels))
    }

    pub(super) fn switch_label_is_constant(&self, expr: &Expr) -> bool {
        match expr.kind() {
            ExprKind::IntLit(_) | ExprKind::UIntLit(_) | ExprKind::BoolLit(_) => true,
            ExprKind::Neg(inner) | ExprKind::BitNot(inner) | ExprKind::Not(inner) => {
                self.switch_label_is_constant(inner)
            }
            ExprKind::TypeAssert { value, .. } => self.switch_label_is_constant(value),
            ExprKind::FieldAccess { object, field } => {
                let ExprKind::Ident(name) = object.kind() else {
                    return false;
                };
                numeric_constant_type(name, field).is_some()
                    || self
                        .enums
                        .get(name)
                        .is_some_and(|info| info.members.iter().any(|(member, _)| member == field))
            }
            _ => false,
        }
    }

    pub(super) fn emit_string_switch(
        &mut self,
        expr: &Expr,
        cases: &[SwitchCase],
        default: &Option<Vec<Stmt>>,
        default_index: Option<usize>,
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let subject = self.emit_expr(expr)?;
        self.forget_owned_native_result(subject);
        self.discard_owned_native_result();
        let subject_ptr = subject.into_struct_value();

        let end_block = self.context.append_basic_block(fn_val, "switch.end");
        let case_blocks = (0..cases.len())
            .map(|index| {
                self.context
                    .append_basic_block(fn_val, &format!("switch.case.{index}"))
            })
            .collect::<Vec<_>>();
        let default_block = default
            .as_ref()
            .map(|_| self.context.append_basic_block(fn_val, "switch.default"));

        for (i, case) in cases.iter().enumerate() {
            let next_comparison = if i + 1 < cases.len() {
                self.context
                    .append_basic_block(fn_val, &format!("switch.next.{i}"))
            } else {
                default_block.unwrap_or(end_block)
            };

            let case_val = self.emit_expr(&case.value)?;
            let eq = self.emit_string_equality(subject_ptr, case_val.into_struct_value(), true)?;
            self.emit_async_cleanup_source_expression(case_val, &Type::Str, &case.value)?;
            self.builder
                .build_conditional_branch(eq.into_int_value(), case_blocks[i], next_comparison)
                .unwrap();
            if i + 1 < cases.len() {
                self.builder.position_at_end(next_comparison);
            }
        }
        if cases.is_empty() {
            self.builder
                .build_unconditional_branch(default_block.unwrap_or(end_block))
                .unwrap();
        }

        self.push_break_target(end_block);
        for (i, case) in cases.iter().enumerate() {
            self.builder.position_at_end(case_blocks[i]);
            self.push_scope();
            for stmt in &case.body {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(stmt, fn_val)?;
            }
            self.pop_scope();
            if !self.current_block_terminated() {
                let fallthrough = if default_index == Some(i + 1) {
                    default_block.expect("switch default index must have a block")
                } else {
                    case_blocks.get(i + 1).copied().unwrap_or(end_block)
                };
                self.builder
                    .build_unconditional_branch(fallthrough)
                    .unwrap();
            }
        }

        if let (Some(default_body), Some(default_block)) = (default, default_block) {
            self.builder.position_at_end(default_block);
            self.push_scope();
            for stmt in default_body {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(stmt, fn_val)?;
            }
            self.pop_scope();
            if !self.current_block_terminated() {
                let fallthrough = case_blocks
                    .get(default_index.expect("switch default must have an index"))
                    .copied()
                    .unwrap_or(end_block);
                self.builder
                    .build_unconditional_branch(fallthrough)
                    .unwrap();
            }
        }
        self.pop_break_target();

        self.builder.position_at_end(end_block);
        Ok(())
    }

    pub(super) fn emit_match_condition(
        &mut self,
        pattern: &MatchPattern,
        guard: &Option<Expr>,
        subject_expr: &Expr,
        subject: BasicValueEnum<'ctx>,
        is_string_match: bool,
        index: usize,
    ) -> Result<IntValue<'ctx>, String> {
        match pattern {
            MatchPattern::Lit { .. }
            | MatchPattern::Wildcard
            | MatchPattern::Binding { .. }
            | MatchPattern::ResultVariant { .. } => self.emit_literal_pattern_condition(
                pattern,
                guard,
                subject_expr,
                subject,
                is_string_match,
                index,
            ),
            MatchPattern::Array { .. } => {
                Err("error: match array pattern not yet supported in codegen".to_string())
            }
            MatchPattern::Tuple(_) => Ok(self.context.bool_type().const_all_ones()),
            MatchPattern::Or(_) => {
                Err("error: match OR pattern not yet supported in codegen".to_string())
            }
            MatchPattern::Range {
                start,
                end,
                inclusive,
            } => {
                let subject_type = self.expr_type_hint(subject_expr).ok_or_else(|| {
                    "error: cannot infer type for match range pattern".to_string()
                })?;
                let resolved = self.resolve_type(&subject_type);
                let unsigned = is_unsigned_int_type(transparent_codegen_type(&resolved));
                let (ge_pred, le_pred, lt_pred) = if unsigned {
                    (IntPredicate::UGE, IntPredicate::ULE, IntPredicate::ULT)
                } else {
                    (IntPredicate::SGE, IntPredicate::SLE, IntPredicate::SLT)
                };
                let ge = match start {
                    Some(start) => {
                        let start_val = self.emit_expr_as(start, &resolved)?;
                        self.emit_comparison(
                            subject,
                            start_val,
                            ge_pred,
                            inkwell::FloatPredicate::OGE,
                            &format!("match.range.ge.{index}"),
                        )?
                        .into_int_value()
                    }
                    None => self.context.bool_type().const_all_ones(),
                };
                let end_ok = match end {
                    Some(end) => {
                        let end_val = self.emit_expr_as(end, &resolved)?;
                        let (int_pred, float_pred) = if *inclusive {
                            (le_pred, inkwell::FloatPredicate::OLE)
                        } else {
                            (lt_pred, inkwell::FloatPredicate::OLT)
                        };
                        self.emit_comparison(
                            subject,
                            end_val,
                            int_pred,
                            float_pred,
                            &format!("match.range.end.{index}"),
                        )?
                        .into_int_value()
                    }
                    None => self.context.bool_type().const_all_ones(),
                };
                let range_ok = self
                    .builder
                    .build_and(ge, end_ok, &format!("match.range.{index}"))
                    .unwrap();
                if let Some(guard_expr) = guard {
                    let guard_val = self.emit_expr(guard_expr)?.into_int_value();
                    Ok(self
                        .builder
                        .build_and(range_ok, guard_val, &format!("match.guard.{index}"))
                        .unwrap())
                } else {
                    Ok(range_ok)
                }
            }
            MatchPattern::EnumVariant { .. } => self.emit_enum_variant_condition(
                pattern,
                guard,
                subject_expr,
                subject,
                is_string_match,
                index,
            ),
            MatchPattern::Object { .. } => {
                self.emit_object_pattern_condition(pattern, guard, subject_expr, subject, index)
            }
        }
    }

    pub(crate) fn push_loop_control(
        &mut self,
        continue_block: BasicBlock<'ctx>,
        break_block: BasicBlock<'ctx>,
    ) {
        self.push_loop_control_with_finally_depth(
            continue_block,
            break_block,
            self.finally_scopes.len(),
        );
    }

    pub(super) fn push_value_loop_control(
        &mut self,
        continue_block: BasicBlock<'ctx>,
        break_block: BasicBlock<'ctx>,
        value_slot: PointerValue<'ctx>,
        value_type: Type,
    ) {
        let body_scope = self
            .owned_native_vars
            .len()
            .checked_sub(1)
            .expect("internal: loop control pushed without a body scope");
        self.loop_stack
            .push((continue_block, break_block, body_scope));
        let finally_depth = self.finally_scopes.len();
        self.break_targets.push(BreakTarget {
            block: break_block,
            finally_depth,
            validation_depth: self.validation_cleanup_scopes.len(),
            scope_depth: body_scope,
            value_slot: Some((value_slot, value_type)),
        });
        self.break_targets
            .last_mut()
            .expect("internal: loop break target was not pushed")
            .finally_depth = finally_depth;
    }

    pub(crate) fn push_loop_control_with_finally_depth(
        &mut self,
        continue_block: BasicBlock<'ctx>,
        break_block: BasicBlock<'ctx>,
        finally_depth: usize,
    ) {
        let body_scope = self
            .owned_native_vars
            .len()
            .checked_sub(1)
            .expect("internal: loop control pushed without a body scope");
        self.loop_stack
            .push((continue_block, break_block, body_scope));
        self.push_break_target_at(break_block, body_scope);
        self.break_targets
            .last_mut()
            .expect("internal: loop break target was not pushed")
            .finally_depth = finally_depth;
    }

    pub(crate) fn pop_loop_control(&mut self) {
        self.loop_stack
            .pop()
            .expect("internal: missing loop control target");
        self.break_targets
            .pop()
            .expect("internal: missing loop break target");
    }

    pub(crate) fn push_break_target(&mut self, block: BasicBlock<'ctx>) {
        self.push_break_target_at(block, self.owned_native_vars.len());
    }

    pub(super) fn push_break_target_at(&mut self, block: BasicBlock<'ctx>, scope_depth: usize) {
        self.break_targets.push(BreakTarget {
            block,
            finally_depth: self.finally_scopes.len(),
            validation_depth: self.validation_cleanup_scopes.len(),
            scope_depth,
            value_slot: None,
        });
    }

    pub(crate) fn pop_break_target(&mut self) {
        self.break_targets
            .pop()
            .expect("internal: missing break target");
    }

    pub(crate) fn emit_deferred_finalizers_in_current_scope(
        &mut self,
        reason: ScopeExitReason,
    ) -> Result<(), String> {
        let depth = self
            .deferred_scopes
            .len()
            .checked_sub(1)
            .ok_or("internal: deferred finalizer scope stack is empty")?;
        if self.deferred_scopes[depth].is_empty() {
            return Ok(());
        }
        let fn_val = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: deferred finalizer outside a function")?;
        self.emit_deferred_finalizers_since(fn_val, depth, reason)
    }

    pub(crate) fn emit_deferred_finalizers_since(
        &mut self,
        fn_val: FunctionValue<'ctx>,
        depth: usize,
        reason: ScopeExitReason,
    ) -> Result<(), String> {
        let saved = self.deferred_scopes.clone();
        let exiting_scopes = self.deferred_scopes[depth..].to_vec();
        let result = (|| {
            for scope in exiting_scopes.into_iter().rev() {
                if reason.runs_errdefer() {
                    for finalizer in scope
                        .iter()
                        .rev()
                        .filter(|finalizer| matches!(finalizer.kind, DeferKind::OnError))
                    {
                        for stmt in &finalizer.body {
                            if self.current_block_terminated() {
                                return Ok(());
                            }
                            self.emit_stmt(stmt, fn_val)?;
                        }
                    }
                }
                for finalizer in scope
                    .iter()
                    .rev()
                    .filter(|finalizer| matches!(finalizer.kind, DeferKind::Always))
                {
                    for stmt in &finalizer.body {
                        if self.current_block_terminated() {
                            return Ok(());
                        }
                        self.emit_stmt(stmt, fn_val)?;
                    }
                }
            }
            Ok(())
        })();
        self.deferred_scopes = saved;
        result
    }
}
