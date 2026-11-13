use super::*;

/// The basic blocks the stable merge sort jumps between.
#[derive(Clone, Copy)]
pub(super) struct MergeSortBlocks<'ctx> {
    pub(super) pass_condition: BasicBlock<'ctx>,
    pub(super) pass_body: BasicBlock<'ctx>,
    pub(super) left_condition: BasicBlock<'ctx>,
    pub(super) merge_setup: BasicBlock<'ctx>,
    pub(super) merge_condition: BasicBlock<'ctx>,
    pub(super) compare: BasicBlock<'ctx>,
    pub(super) take_left: BasicBlock<'ctx>,
    pub(super) take_right: BasicBlock<'ctx>,
    pub(super) drain_left_condition: BasicBlock<'ctx>,
    pub(super) drain_left: BasicBlock<'ctx>,
    pub(super) drain_right_condition: BasicBlock<'ctx>,
    pub(super) drain_right: BasicBlock<'ctx>,
    pub(super) copy_condition: BasicBlock<'ctx>,
    pub(super) copy_body: BasicBlock<'ctx>,
    pub(super) next_left: BasicBlock<'ctx>,
    pub(super) next_width: BasicBlock<'ctx>,
    pub(super) width_to_length: BasicBlock<'ctx>,
    pub(super) width_double: BasicBlock<'ctx>,
    pub(super) done: BasicBlock<'ctx>,
}

/// The buffers, cursors and blocks every phase of the merge sort reads. The
/// prologue fills it once so each phase can live in a function of its own.
#[derive(Clone, Copy)]
pub(super) struct MergeSort<'a, 'ctx> {
    pub(super) view: &'a ArrayView<'ctx>,
    pub(super) data: PointerValue<'ctx>,
    pub(super) scratch: PointerValue<'ctx>,
    pub(super) element_type: BasicTypeEnum<'ctx>,
    pub(super) i64_type: inkwell::types::IntType<'ctx>,
    pub(super) zero: inkwell::values::IntValue<'ctx>,
    pub(super) one: inkwell::values::IntValue<'ctx>,
    pub(super) width_slot: PointerValue<'ctx>,
    pub(super) left_slot: PointerValue<'ctx>,
    pub(super) left_cursor: PointerValue<'ctx>,
    pub(super) right_cursor: PointerValue<'ctx>,
    pub(super) output_cursor: PointerValue<'ctx>,
    pub(super) function: FunctionValue<'ctx>,
    pub(super) blocks: MergeSortBlocks<'ctx>,
}

/// The bounds of the run pair the current merge step is working on.
#[derive(Clone, Copy)]
pub(super) struct MergeSortRange<'ctx> {
    pub(super) mid: inkwell::values::IntValue<'ctx>,
    pub(super) right: inkwell::values::IntValue<'ctx>,
    pub(super) width: inkwell::values::IntValue<'ctx>,
}

impl<'ctx> Codegen<'ctx> {
    /// Starts a pass and sets up the next pair of runs to merge.
    pub(super) fn emit_merge_sort_passes(
        &mut self,
        sort: MergeSort<'_, 'ctx>,
    ) -> Result<MergeSortRange<'ctx>, String> {
        let MergeSort {
            view,
            data: _,
            scratch: _,
            element_type: _,
            i64_type,
            zero,
            one: _,
            width_slot,
            left_slot,
            left_cursor,
            right_cursor,
            output_cursor,
            function: _,
            blocks,
        } = sort;
        let MergeSortBlocks {
            pass_condition,
            pass_body,
            left_condition,
            merge_setup,
            merge_condition,
            compare: _,
            take_left: _,
            take_right: _,
            drain_left_condition: _,
            drain_left: _,
            drain_right_condition: _,
            drain_right: _,
            copy_condition: _,
            copy_body: _,
            next_left: _,
            next_width,
            width_to_length: _,
            width_double: _,
            done,
        } = blocks;
        self.builder.position_at_end(pass_condition);
        let width = self
            .builder
            .build_load(i64_type, width_slot, "stable_sort_width_value")
            .unwrap()
            .into_int_value();
        let has_pass = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                width,
                view.length,
                "stable_sort_has_pass",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(has_pass, pass_body, done)
            .unwrap();

        self.builder.position_at_end(pass_body);
        self.builder.build_store(left_slot, zero).unwrap();
        self.builder
            .build_unconditional_branch(left_condition)
            .unwrap();

        self.builder.position_at_end(left_condition);
        let left = self
            .builder
            .build_load(i64_type, left_slot, "stable_sort_left_value")
            .unwrap()
            .into_int_value();
        let has_left = self
            .builder
            .build_int_compare(IntPredicate::ULT, left, view.length, "stable_sort_has_left")
            .unwrap();
        self.builder
            .build_conditional_branch(has_left, merge_setup, next_width)
            .unwrap();

        self.builder.position_at_end(merge_setup);
        let remaining = self
            .builder
            .build_int_sub(view.length, left, "stable_sort_remaining")
            .unwrap();
        let left_width = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(
                        IntPredicate::ULT,
                        width,
                        remaining,
                        "stable_sort_left_has_width",
                    )
                    .unwrap(),
                width,
                remaining,
                "stable_sort_left_width",
            )
            .unwrap()
            .into_int_value();
        let mid = self
            .builder
            .build_int_add(left, left_width, "stable_sort_mid")
            .unwrap();
        let after_mid = self
            .builder
            .build_int_sub(view.length, mid, "stable_sort_after_mid")
            .unwrap();
        let right_width = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(
                        IntPredicate::ULT,
                        width,
                        after_mid,
                        "stable_sort_right_has_width",
                    )
                    .unwrap(),
                width,
                after_mid,
                "stable_sort_right_width",
            )
            .unwrap()
            .into_int_value();
        let right = self
            .builder
            .build_int_add(mid, right_width, "stable_sort_right")
            .unwrap();
        self.builder.build_store(left_cursor, left).unwrap();
        self.builder.build_store(right_cursor, mid).unwrap();
        self.builder.build_store(output_cursor, left).unwrap();
        self.builder
            .build_unconditional_branch(merge_condition)
            .unwrap();

        Ok(MergeSortRange { mid, right, width })
    }

    /// Merges the two runs, element by element, into the scratch buffer.
    pub(super) fn emit_merge_sort_merge(
        &mut self,
        sort: MergeSort<'_, 'ctx>,
        range: MergeSortRange<'ctx>,
        callback: &Option<(BasicValueEnum<'ctx>, Vec<Type>)>,
        keyed: &Option<(usize, Type, bool)>,
    ) -> Result<(), String> {
        let MergeSort {
            view,
            data,
            scratch,
            element_type,
            i64_type,
            zero: _,
            one,
            width_slot: _,
            left_slot: _,
            left_cursor,
            right_cursor,
            output_cursor,
            function,
            blocks,
        } = sort;
        let MergeSortBlocks {
            pass_condition: _,
            pass_body: _,
            left_condition: _,
            merge_setup: _,
            merge_condition,
            compare,
            take_left,
            take_right,
            drain_left_condition,
            drain_left: _,
            drain_right_condition,
            drain_right: _,
            copy_condition: _,
            copy_body: _,
            next_left: _,
            next_width: _,
            width_to_length: _,
            width_double: _,
            done: _,
        } = blocks;
        let MergeSortRange {
            mid,
            right,
            width: _,
        } = range;
        self.builder.position_at_end(merge_condition);
        let i = self
            .builder
            .build_load(i64_type, left_cursor, "stable_sort_i_value")
            .unwrap()
            .into_int_value();
        let j = self
            .builder
            .build_load(i64_type, right_cursor, "stable_sort_j_value")
            .unwrap()
            .into_int_value();
        let left_available = self
            .builder
            .build_int_compare(IntPredicate::ULT, i, mid, "stable_sort_left_available")
            .unwrap();
        let right_available = self
            .builder
            .build_int_compare(IntPredicate::ULT, j, right, "stable_sort_right_available")
            .unwrap();
        let both_available = self
            .builder
            .build_and(
                left_available,
                right_available,
                "stable_sort_both_available",
            )
            .unwrap();
        let drain_left_or_right = self
            .context
            .append_basic_block(function, "stable_sort.drain.select");
        self.builder
            .build_conditional_branch(both_available, compare, drain_left_or_right)
            .unwrap();

        self.builder.position_at_end(drain_left_or_right);
        self.builder
            .build_conditional_branch(left_available, drain_left_condition, drain_right_condition)
            .unwrap();

        self.builder.position_at_end(compare);
        let left_slot_value = unsafe {
            self.builder
                .build_gep(element_type, data, &[i], "stable_sort_left_slot")
                .unwrap()
        };
        let right_slot_value = unsafe {
            self.builder
                .build_gep(element_type, data, &[j], "stable_sort_right_slot")
                .unwrap()
        };
        let left_value = self
            .builder
            .build_load(
                element_type,
                left_slot_value,
                "stable_sort_left_value_loaded",
            )
            .unwrap();
        let right_value = self
            .builder
            .build_load(
                element_type,
                right_slot_value,
                "stable_sort_right_value_loaded",
            )
            .unwrap();
        let comparison = if let Some((closure, params)) = &callback {
            self.emit_closure_call(*closure, params, &Type::I32, &[left_value, right_value])?
                .unwrap()
                .into_int_value()
        } else if let Some((field_index, field_type, descending)) = &keyed {
            let left_key = self
                .builder
                .build_extract_value(
                    left_value.into_struct_value(),
                    *field_index as u32,
                    "stable_sort_key_left",
                )
                .map_err(|error| error.to_string())?;
            let right_key = self
                .builder
                .build_extract_value(
                    right_value.into_struct_value(),
                    *field_index as u32,
                    "stable_sort_key_right",
                )
                .map_err(|error| error.to_string())?;
            if *descending {
                self.emit_default_sort_comparison(field_type, right_key, left_key)?
            } else {
                self.emit_default_sort_comparison(field_type, left_key, right_key)?
            }
        } else {
            self.emit_default_sort_comparison(&view.element_type, left_value, right_value)?
        };
        let choose_right = self
            .builder
            .build_int_compare(
                IntPredicate::SGT,
                comparison,
                self.context.i32_type().const_zero(),
                "stable_sort_choose_right",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(choose_right, take_right, take_left)
            .unwrap();

        self.builder.position_at_end(take_left);
        let k = self
            .builder
            .build_load(i64_type, output_cursor, "stable_sort_k_left")
            .unwrap()
            .into_int_value();
        let temp_left = unsafe {
            self.builder
                .build_gep(element_type, scratch, &[k], "stable_sort_temp_left")
                .unwrap()
        };
        self.builder.build_store(temp_left, left_value).unwrap();
        self.builder
            .build_store(
                left_cursor,
                self.builder
                    .build_int_add(i, one, "stable_sort_i_next")
                    .unwrap(),
            )
            .unwrap();
        self.builder
            .build_store(
                output_cursor,
                self.builder
                    .build_int_add(k, one, "stable_sort_k_next")
                    .unwrap(),
            )
            .unwrap();
        self.builder
            .build_unconditional_branch(merge_condition)
            .unwrap();

        self.builder.position_at_end(take_right);
        let k = self
            .builder
            .build_load(i64_type, output_cursor, "stable_sort_k_right")
            .unwrap()
            .into_int_value();
        let temp_right = unsafe {
            self.builder
                .build_gep(element_type, scratch, &[k], "stable_sort_temp_right")
                .unwrap()
        };
        self.builder.build_store(temp_right, right_value).unwrap();
        self.builder
            .build_store(
                right_cursor,
                self.builder
                    .build_int_add(j, one, "stable_sort_j_next")
                    .unwrap(),
            )
            .unwrap();
        self.builder
            .build_store(
                output_cursor,
                self.builder
                    .build_int_add(k, one, "stable_sort_k_next")
                    .unwrap(),
            )
            .unwrap();
        self.builder
            .build_unconditional_branch(merge_condition)
            .unwrap();

        Ok(())
    }

    /// Copies whatever is left of either run into the scratch buffer.
    pub(super) fn emit_merge_sort_drain(
        &mut self,
        sort: MergeSort<'_, 'ctx>,
        range: MergeSortRange<'ctx>,
    ) -> Result<(), String> {
        let MergeSort {
            view: _,
            data,
            scratch,
            element_type,
            i64_type,
            zero: _,
            one,
            width_slot: _,
            left_slot: _,
            left_cursor,
            right_cursor,
            output_cursor,
            function: _,
            blocks,
        } = sort;
        let MergeSortBlocks {
            pass_condition: _,
            pass_body: _,
            left_condition: _,
            merge_setup: _,
            merge_condition: _,
            compare: _,
            take_left: _,
            take_right: _,
            drain_left_condition,
            drain_left,
            drain_right_condition,
            drain_right,
            copy_condition,
            copy_body: _,
            next_left: _,
            next_width: _,
            width_to_length: _,
            width_double: _,
            done: _,
        } = blocks;
        let MergeSortRange {
            mid,
            right,
            width: _,
        } = range;
        self.builder.position_at_end(drain_left_condition);
        let i = self
            .builder
            .build_load(i64_type, left_cursor, "stable_sort_drain_i")
            .unwrap()
            .into_int_value();
        let has_i = self
            .builder
            .build_int_compare(IntPredicate::ULT, i, mid, "stable_sort_drain_has_i")
            .unwrap();
        self.builder
            .build_conditional_branch(has_i, drain_left, drain_right_condition)
            .unwrap();
        self.builder.position_at_end(drain_left);
        let k = self
            .builder
            .build_load(i64_type, output_cursor, "stable_sort_drain_left_k")
            .unwrap()
            .into_int_value();
        let source = unsafe {
            self.builder
                .build_gep(element_type, data, &[i], "stable_sort_drain_left_source")
                .unwrap()
        };
        let destination = unsafe {
            self.builder
                .build_gep(
                    element_type,
                    scratch,
                    &[k],
                    "stable_sort_drain_left_destination",
                )
                .unwrap()
        };
        self.builder
            .build_store(
                destination,
                self.builder
                    .build_load(element_type, source, "stable_sort_drain_left_value")
                    .unwrap(),
            )
            .unwrap();
        self.builder
            .build_store(
                left_cursor,
                self.builder
                    .build_int_add(i, one, "stable_sort_drain_left_next_i")
                    .unwrap(),
            )
            .unwrap();
        self.builder
            .build_store(
                output_cursor,
                self.builder
                    .build_int_add(k, one, "stable_sort_drain_left_next_k")
                    .unwrap(),
            )
            .unwrap();
        self.builder
            .build_unconditional_branch(drain_left_condition)
            .unwrap();

        self.builder.position_at_end(drain_right_condition);
        let j = self
            .builder
            .build_load(i64_type, right_cursor, "stable_sort_drain_j")
            .unwrap()
            .into_int_value();
        let has_j = self
            .builder
            .build_int_compare(IntPredicate::ULT, j, right, "stable_sort_drain_has_j")
            .unwrap();
        self.builder
            .build_conditional_branch(has_j, drain_right, copy_condition)
            .unwrap();
        self.builder.position_at_end(drain_right);
        let k = self
            .builder
            .build_load(i64_type, output_cursor, "stable_sort_drain_right_k")
            .unwrap()
            .into_int_value();
        let source = unsafe {
            self.builder
                .build_gep(element_type, data, &[j], "stable_sort_drain_right_source")
                .unwrap()
        };
        let destination = unsafe {
            self.builder
                .build_gep(
                    element_type,
                    scratch,
                    &[k],
                    "stable_sort_drain_right_destination",
                )
                .unwrap()
        };
        self.builder
            .build_store(
                destination,
                self.builder
                    .build_load(element_type, source, "stable_sort_drain_right_value")
                    .unwrap(),
            )
            .unwrap();
        self.builder
            .build_store(
                right_cursor,
                self.builder
                    .build_int_add(j, one, "stable_sort_drain_right_next_j")
                    .unwrap(),
            )
            .unwrap();
        self.builder
            .build_store(
                output_cursor,
                self.builder
                    .build_int_add(k, one, "stable_sort_drain_right_next_k")
                    .unwrap(),
            )
            .unwrap();
        self.builder
            .build_unconditional_branch(drain_right_condition)
            .unwrap();

        Ok(())
    }

    /// Copies the merged run back and widens the pass.
    pub(super) fn emit_merge_sort_copy_back(
        &mut self,
        sort: MergeSort<'_, 'ctx>,
        range: MergeSortRange<'ctx>,
    ) -> Result<(), String> {
        let MergeSort {
            view,
            data,
            scratch,
            element_type,
            i64_type,
            zero: _,
            one,
            width_slot,
            left_slot,
            left_cursor: _,
            right_cursor: _,
            output_cursor: _,
            function: _,
            blocks,
        } = sort;
        let MergeSortBlocks {
            pass_condition,
            pass_body: _,
            left_condition,
            merge_setup: _,
            merge_condition: _,
            compare: _,
            take_left: _,
            take_right: _,
            drain_left_condition: _,
            drain_left: _,
            drain_right_condition: _,
            drain_right: _,
            copy_condition,
            copy_body,
            next_left,
            next_width,
            width_to_length,
            width_double,
            done: _,
        } = blocks;
        let MergeSortRange {
            mid: _,
            right,
            width,
        } = range;
        self.builder.position_at_end(copy_condition);
        let copy_index = self
            .builder
            .build_load(i64_type, left_slot, "stable_sort_copy_index")
            .unwrap()
            .into_int_value();
        let copy_end = right;
        let has_copy = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                copy_index,
                copy_end,
                "stable_sort_has_copy",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(has_copy, copy_body, next_left)
            .unwrap();
        self.builder.position_at_end(copy_body);
        let source = unsafe {
            self.builder
                .build_gep(
                    element_type,
                    scratch,
                    &[copy_index],
                    "stable_sort_copy_source",
                )
                .unwrap()
        };
        let destination = unsafe {
            self.builder
                .build_gep(
                    element_type,
                    data,
                    &[copy_index],
                    "stable_sort_copy_destination",
                )
                .unwrap()
        };
        self.builder
            .build_store(
                destination,
                self.builder
                    .build_load(element_type, source, "stable_sort_copy_value")
                    .unwrap(),
            )
            .unwrap();
        self.builder
            .build_store(
                left_slot,
                self.builder
                    .build_int_add(copy_index, one, "stable_sort_copy_next")
                    .unwrap(),
            )
            .unwrap();
        self.builder
            .build_unconditional_branch(copy_condition)
            .unwrap();

        self.builder.position_at_end(next_left);
        self.builder.build_store(left_slot, right).unwrap();
        self.builder
            .build_unconditional_branch(left_condition)
            .unwrap();

        self.builder.position_at_end(next_width);
        let half = self
            .builder
            .build_int_unsigned_div(
                view.length,
                i64_type.const_int(2, false),
                "stable_sort_half_length",
            )
            .unwrap();
        let width_large = self
            .builder
            .build_int_compare(IntPredicate::UGT, width, half, "stable_sort_width_large")
            .unwrap();
        self.builder
            .build_conditional_branch(width_large, width_to_length, width_double)
            .unwrap();
        self.builder.position_at_end(width_to_length);
        self.builder.build_store(width_slot, view.length).unwrap();
        self.builder
            .build_unconditional_branch(pass_condition)
            .unwrap();
        self.builder.position_at_end(width_double);
        self.builder
            .build_store(
                width_slot,
                self.builder
                    .build_int_mul(
                        width,
                        i64_type.const_int(2, false),
                        "stable_sort_width_next",
                    )
                    .unwrap(),
            )
            .unwrap();
        self.builder
            .build_unconditional_branch(pass_condition)
            .unwrap();

        Ok(())
    }
}
