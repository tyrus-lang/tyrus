use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(crate) fn tuple_slice(
        &mut self,
        expr: &Expr,
        name: &str,
    ) -> Result<Option<TupleSlice<'ctx>>, String> {
        let Some(hint) = self.expr_type_hint(expr) else {
            return Ok(None);
        };
        let tuple_type = transparent_codegen_type(&hint).clone();
        let Type::Tuple(elements) = &tuple_type else {
            return Ok(None);
        };
        let element_type = self.tuple_iteration_element_type(elements)?;
        let element_llvm = self.llvm_type(&element_type);
        let length = self
            .context
            .i64_type()
            .const_int(elements.len() as u64, false);
        let tuple = self.emit_expr(expr)?.into_struct_value();
        let data = self
            .entry_alloca(self.llvm_type(&tuple_type), name)
            .unwrap();
        self.builder.build_store(data, tuple).unwrap();
        Ok(Some(TupleSlice {
            data,
            length,
            element_type,
            element_llvm,
        }))
    }

    pub(crate) fn emit_array_slice_into(
        &mut self,
        array_slot: PointerValue<'ctx>,
        output: PointerValue<'ctx>,
        start: IntValue<'ctx>,
        end: IntValue<'ctx>,
        element_size: IntValue<'ctx>,
        owned_string: bool,
    ) -> Result<IntValue<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i64_type = self.context.i64_type();
        let i32_type = self.context.i32_type();
        let result = if owned_string {
            let function = self.declare_libc_fn(
                "tyrus_array_slice_owned_strings",
                &[
                    pointer.into(),
                    i64_type.into(),
                    i64_type.into(),
                    pointer.into(),
                ],
                Some(i32_type.into()),
            );
            self.builder.build_call(
                function,
                &[array_slot.into(), start.into(), end.into(), output.into()],
                "array_copy",
            )
        } else {
            let function = self.declare_libc_fn(
                "tyrus_array_slice",
                &[
                    pointer.into(),
                    i64_type.into(),
                    i64_type.into(),
                    i64_type.into(),
                    pointer.into(),
                ],
                Some(i32_type.into()),
            );
            self.builder.build_call(
                function,
                &[
                    array_slot.into(),
                    start.into(),
                    end.into(),
                    element_size.into(),
                    output.into(),
                ],
                "array_copy",
            )
        };
        Ok(result
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value())
    }

    pub(super) fn emit_array_rotate(
        &mut self,
        array_slot: PointerValue<'ctx>,
        llvm_array_type: BasicTypeEnum<'ctx>,
        element: &Type,
        amount_expr: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let array = self
            .builder
            .build_load(llvm_array_type, array_slot, "array_rotate")
            .map_err(|error| error.to_string())?
            .into_struct_value();
        let data = self
            .builder
            .build_extract_value(array, 0, "array_rotate_data")
            .map_err(|error| error.to_string())?
            .into_pointer_value();
        let length = self
            .builder
            .build_extract_value(array, 1, "array_rotate_length")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let amount = self.emit_expr_as(amount_expr, &Type::I64)?.into_int_value();
        let i64_type = self.context.i64_type();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: array rotation outside function")?;
        let skip = self
            .context
            .append_basic_block(function, "array_rotate.skip");
        let normalize = self
            .context
            .append_basic_block(function, "array_rotate.normalize");
        let rotate = self
            .context
            .append_basic_block(function, "array_rotate.body");
        let finish = self
            .context
            .append_basic_block(function, "array_rotate.done");
        let small = self
            .builder
            .build_int_compare(
                IntPredicate::ULE,
                length,
                i64_type.const_int(1, false),
                "array_rotate_small",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(small, skip, normalize)
            .unwrap();
        self.builder.position_at_end(skip);
        self.builder.build_unconditional_branch(finish).unwrap();
        self.builder.position_at_end(normalize);
        let remainder = self
            .builder
            .build_int_signed_rem(amount, length, "array_rotate_remainder")
            .unwrap();
        let negative = self
            .builder
            .build_int_compare(
                IntPredicate::SLT,
                remainder,
                i64_type.const_zero(),
                "array_rotate_negative",
            )
            .unwrap();
        let positive_remainder = self
            .builder
            .build_int_add(remainder, length, "array_rotate_positive_remainder")
            .unwrap();
        let normalized = self
            .builder
            .build_select(
                negative,
                positive_remainder,
                remainder,
                "array_rotate_amount",
            )
            .unwrap()
            .into_int_value();
        let no_rotation = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                normalized,
                i64_type.const_zero(),
                "array_rotate_noop",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(no_rotation, finish, rotate)
            .unwrap();
        self.builder.position_at_end(rotate);
        self.emit_array_reverse_range(
            data,
            element,
            i64_type.const_zero(),
            normalized,
            "array_rotate.first",
        )?;
        self.emit_array_reverse_range(data, element, normalized, length, "array_rotate.second")?;
        self.emit_array_reverse_range(
            data,
            element,
            i64_type.const_zero(),
            length,
            "array_rotate.final",
        )?;
        self.builder.build_unconditional_branch(finish).unwrap();
        self.builder.position_at_end(finish);
        self.builder
            .build_load(llvm_array_type, array_slot, "array_rotate_result")
            .map_err(|error| error.to_string())
    }

    pub(super) fn emit_array_reverse_range(
        &mut self,
        data: PointerValue<'ctx>,
        element: &Type,
        start: IntValue<'ctx>,
        end: IntValue<'ctx>,
        _name: &str,
    ) -> Result<(), String> {
        let i64_type = self.context.i64_type();
        let element_llvm = self.llvm_type(element);
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: array reverse range outside function")?;
        let left = self
            .builder
            .build_alloca(i64_type, "array_rotate_left")
            .unwrap();
        let right = self
            .builder
            .build_alloca(i64_type, "array_rotate_right")
            .unwrap();
        let temporary = self
            .builder
            .build_alloca(element_llvm, "array_rotate_temporary")
            .unwrap();
        self.builder.build_store(left, start).unwrap();
        let last = self
            .builder
            .build_int_sub(end, i64_type.const_int(1, false), "array_rotate_last")
            .unwrap();
        self.builder.build_store(right, last).unwrap();
        let condition = self
            .context
            .append_basic_block(function, "array_rotate.reverse.condition");
        let body = self
            .context
            .append_basic_block(function, "array_rotate.reverse.body");
        let done = self
            .context
            .append_basic_block(function, "array_rotate.reverse.done");
        self.builder.build_unconditional_branch(condition).unwrap();
        self.builder.position_at_end(condition);
        let left_value = self
            .builder
            .build_load(i64_type, left, "array_rotate_left_value")
            .unwrap()
            .into_int_value();
        let right_value = self
            .builder
            .build_load(i64_type, right, "array_rotate_right_value")
            .unwrap()
            .into_int_value();
        let continue_loop = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                left_value,
                right_value,
                "array_rotate_continue",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(continue_loop, body, done)
            .unwrap();
        self.builder.position_at_end(body);
        let left_pointer = unsafe {
            self.builder
                .build_gep(
                    element_llvm,
                    data,
                    &[left_value],
                    "array_rotate_left_pointer",
                )
                .unwrap()
        };
        let right_pointer = unsafe {
            self.builder
                .build_gep(
                    element_llvm,
                    data,
                    &[right_value],
                    "array_rotate_right_pointer",
                )
                .unwrap()
        };
        let left_element = self
            .builder
            .build_load(element_llvm, left_pointer, "array_rotate_left_element")
            .map_err(|error| error.to_string())?;
        let right_element = self
            .builder
            .build_load(element_llvm, right_pointer, "array_rotate_right_element")
            .map_err(|error| error.to_string())?;
        self.builder.build_store(temporary, left_element).unwrap();
        self.builder
            .build_store(left_pointer, right_element)
            .unwrap();
        let temporary_value = self
            .builder
            .build_load(element_llvm, temporary, "array_rotate_temporary_value")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(right_pointer, temporary_value)
            .unwrap();
        let next_left = self
            .builder
            .build_int_add(
                left_value,
                i64_type.const_int(1, false),
                "array_rotate_next_left",
            )
            .unwrap();
        let next_right = self
            .builder
            .build_int_sub(
                right_value,
                i64_type.const_int(1, false),
                "array_rotate_next_right",
            )
            .unwrap();
        self.builder.build_store(left, next_left).unwrap();
        self.builder.build_store(right, next_right).unwrap();
        self.builder.build_unconditional_branch(condition).unwrap();
        self.builder.position_at_end(done);
        Ok(())
    }
}
