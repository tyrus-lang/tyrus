use super::*;
use tyrus_core::ast::Expr;

#[derive(Clone, Copy)]
enum SimdOperation {
    Add,
    Sub,
    Mul,
    Div,
}

impl SimdOperation {
    fn name(self) -> &'static str {
        match self {
            Self::Add => "simdAdd",
            Self::Sub => "simdSub",
            Self::Mul => "simdMul",
            Self::Div => "simdDiv",
        }
    }
}

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_simd_call(
        &mut self,
        callee: &str,
        type_args: &[Type],
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let operation = match callee {
            tyrus_core::simd::ADD_TARGET => SimdOperation::Add,
            tyrus_core::simd::SUB_TARGET => SimdOperation::Sub,
            tyrus_core::simd::MUL_TARGET => SimdOperation::Mul,
            tyrus_core::simd::DIV_TARGET => SimdOperation::Div,
            _ => return Ok(None),
        };
        if args.len() != 2 {
            return Err(format!(
                "error: `{}` expects 2 arguments, found {}",
                operation.name(),
                args.len()
            ));
        }
        let element = type_args
            .first()
            .cloned()
            .or_else(|| {
                self.expr_type_hint(&args[0])
                    .and_then(|ty| match self.resolve_type(&ty) {
                        Type::Array(element) => Some(*element),
                        Type::Tuple(elements) => elements.first().cloned(),
                        _ => None,
                    })
            })
            .ok_or_else(|| {
                format!(
                    "internal: `{}` element type was not resolved",
                    operation.name()
                )
            })?;
        let element = self.resolve_type(&element);
        if !matches!(
            &element,
            Type::I8
                | Type::I16
                | Type::I32
                | Type::I64
                | Type::U8
                | Type::U16
                | Type::U32
                | Type::U64
                | Type::F32
                | Type::F64
        ) {
            return Err(format!(
                "internal: `{}` does not support element type `{element}`",
                operation.name()
            ));
        }
        let is_float = matches!(&element, Type::F32 | Type::F64);

        let array_type = Type::Array(Box::new(element.clone()));
        let (left_data, left_length) = self.emit_simd_input(&args[0], &element, "left")?;
        let (right_data, right_length) = self.emit_simd_input(&args[1], &element, "right")?;

        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or_else(|| format!("internal: `{}` outside function", operation.name()))?;
        let mismatch = self
            .context
            .append_basic_block(function, "simd.add.mismatch");
        let allocate = self
            .context
            .append_basic_block(function, "simd.add.allocate");
        let allocation_failed = self
            .context
            .append_basic_block(function, "simd.add.allocation_failed");
        let loop_check = self
            .context
            .append_basic_block(function, "simd.add.vector_check");
        let vector_body = self
            .context
            .append_basic_block(function, "simd.add.vector_body");
        let vector_done = self
            .context
            .append_basic_block(function, "simd.add.vector_done");
        let tail_body = self
            .context
            .append_basic_block(function, "simd.add.tail_body");
        let tail_done = self
            .context
            .append_basic_block(function, "simd.add.tail_done");
        let merge = self.context.append_basic_block(function, "simd.add.merge");
        let validates_integer_division = matches!(operation, SimdOperation::Div) && !is_float;
        let signed_division = validates_integer_division && is_signed_integer(&element);
        let div_validate_check = validates_integer_division.then(|| {
            self.context
                .append_basic_block(function, "simd.div.validate.check")
        });
        let div_validate_body = validates_integer_division.then(|| {
            self.context
                .append_basic_block(function, "simd.div.validate.body")
        });
        let div_zero_error = validates_integer_division.then(|| {
            self.context
                .append_basic_block(function, "simd.div.zero_error")
        });
        let div_overflow_check = signed_division.then(|| {
            self.context
                .append_basic_block(function, "simd.div.overflow_check")
        });
        let div_overflow_error = signed_division.then(|| {
            self.context
                .append_basic_block(function, "simd.div.overflow_error")
        });
        let div_validate_next = validates_integer_division.then(|| {
            self.context
                .append_basic_block(function, "simd.div.validate.next")
        });
        let div_validate_done = validates_integer_division.then(|| {
            self.context
                .append_basic_block(function, "simd.div.validate.done")
        });
        let length_check_block = self
            .builder
            .get_insert_block()
            .ok_or_else(|| format!("internal: `{}` outside a basic block", operation.name()))?;
        let same_length = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                left_length,
                right_length,
                "simd.add.same_length",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(
                same_length,
                div_validate_check.unwrap_or(allocate),
                mismatch,
            )
            .map_err(|error| error.to_string())?;

        let variants = vec![array_type.clone(), Type::Named("SimdError".to_string())];
        let result_slot = self.entry_alloca(self.union_llvm_type(&variants), "simd.add.result")?;

        self.builder.position_at_end(mismatch);
        self.emit_simd_error_result(
            result_slot,
            &variants,
            1,
            "ERR_SIMD_LENGTH_MISMATCH",
            &format!("{} requires arrays of equal length", operation.name()),
        )?;
        self.builder
            .build_unconditional_branch(merge)
            .map_err(|error| error.to_string())?;

        if validates_integer_division {
            let validate_check = div_validate_check.unwrap();
            let validate_body = div_validate_body.unwrap();
            let zero_error = div_zero_error.unwrap();
            let validate_next = div_validate_next.unwrap();
            let validate_done = div_validate_done.unwrap();
            self.builder.position_at_end(validate_check);
            let index = self
                .builder
                .build_phi(self.usize_type(), "simd.div.validate.index")
                .map_err(|error| error.to_string())?;
            let zero = self.usize_type().const_zero();
            index.add_incoming(&[(&zero, length_check_block)]);
            let index_value = index.as_basic_value().into_int_value();
            let has_more = self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::ULT,
                    index_value,
                    left_length,
                    "simd.div.validate.has_more",
                )
                .map_err(|error| error.to_string())?;
            self.builder
                .build_conditional_branch(has_more, validate_body, validate_done)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(validate_body);
            let pointer_type = self.context.ptr_type(inkwell::AddressSpace::default());
            let left_ptr = self
                .builder
                .build_pointer_cast(left_data, pointer_type, "simd.div.validate.left")
                .map_err(|error| error.to_string())?;
            let right_ptr = self
                .builder
                .build_pointer_cast(right_data, pointer_type, "simd.div.validate.right")
                .map_err(|error| error.to_string())?;
            let left_element = unsafe {
                self.builder.build_gep(
                    self.llvm_type(&element),
                    left_ptr,
                    &[index_value],
                    "simd.div.validate.left.ptr",
                )
            }
            .map_err(|error| error.to_string())?;
            let right_element = unsafe {
                self.builder.build_gep(
                    self.llvm_type(&element),
                    right_ptr,
                    &[index_value],
                    "simd.div.validate.right.ptr",
                )
            }
            .map_err(|error| error.to_string())?;
            let left_value = self
                .builder
                .build_load(
                    self.llvm_type(&element),
                    left_element,
                    "simd.div.validate.left.value",
                )
                .map_err(|error| error.to_string())?
                .into_int_value();
            let right_value = self
                .builder
                .build_load(
                    self.llvm_type(&element),
                    right_element,
                    "simd.div.validate.right.value",
                )
                .map_err(|error| error.to_string())?
                .into_int_value();
            let divisor_is_zero = self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    right_value,
                    right_value.get_type().const_zero(),
                    "simd.div.validate.divisor_is_zero",
                )
                .map_err(|error| error.to_string())?;
            let valid_divisor = if signed_division {
                div_overflow_check.unwrap()
            } else {
                validate_next
            };
            self.builder
                .build_conditional_branch(divisor_is_zero, zero_error, valid_divisor)
                .map_err(|error| error.to_string())?;

            if signed_division {
                let overflow_check = div_overflow_check.unwrap();
                let overflow_error = div_overflow_error.unwrap();
                self.builder.position_at_end(overflow_check);
                let int_type = right_value.get_type();
                let min_value = int_type.const_int(1u64 << (int_type.get_bit_width() - 1), false);
                let minus_one = int_type.const_all_ones();
                let is_min = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::EQ,
                        left_value,
                        min_value,
                        "simd.div.validate.left_is_min",
                    )
                    .map_err(|error| error.to_string())?;
                let is_minus_one = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::EQ,
                        right_value,
                        minus_one,
                        "simd.div.validate.right_is_minus_one",
                    )
                    .map_err(|error| error.to_string())?;
                let signed_overflow = self
                    .builder
                    .build_and(is_min, is_minus_one, "simd.div.validate.signed_overflow")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_conditional_branch(signed_overflow, overflow_error, validate_next)
                    .map_err(|error| error.to_string())?;

                self.builder.position_at_end(overflow_error);
                self.emit_simd_error_result(
                    result_slot,
                    &variants,
                    1,
                    "ERR_SIMD_INTEGER_OVERFLOW",
                    "simdDiv cannot represent the signed minimum divided by -1",
                )?;
                self.builder
                    .build_unconditional_branch(merge)
                    .map_err(|error| error.to_string())?;
            }

            self.builder.position_at_end(zero_error);
            self.emit_simd_error_result(
                result_slot,
                &variants,
                1,
                "ERR_SIMD_DIVISION_BY_ZERO",
                "simdDiv cannot divide an integer lane by zero",
            )?;
            self.builder
                .build_unconditional_branch(merge)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(validate_next);
            let next_index = self
                .builder
                .build_int_add(
                    index_value,
                    self.usize_type().const_int(1, false),
                    "simd.div.validate.next_index",
                )
                .map_err(|error| error.to_string())?;
            let validate_next_end = self
                .builder
                .get_insert_block()
                .ok_or("internal: missing SIMD division validation block")?;
            self.builder
                .build_unconditional_branch(validate_check)
                .map_err(|error| error.to_string())?;
            index.add_incoming(&[(&next_index, validate_next_end)]);
            self.builder.position_at_end(validate_done);
            self.builder
                .build_unconditional_branch(allocate)
                .map_err(|error| error.to_string())?;
        }

        self.builder.position_at_end(allocate);
        let output_slot = self.entry_alloca(self.llvm_type(&array_type), "simd.add.output")?;
        let alloc_fn = self.declare_libc_fn(
            "tyrus_array_with_capacity",
            &[
                self.usize_type().into(),
                self.usize_type().into(),
                self.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into(),
            ],
            Some(self.context.i32_type().into()),
        );
        let allocation_status = self
            .builder
            .build_call(
                alloc_fn,
                &[
                    left_length.into(),
                    self.context
                        .i64_type()
                        .const_int(self.type_layout(&element).1 as u64, false)
                        .into(),
                    output_slot.into(),
                ],
                "simd.add.allocation_status",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: array allocation returned no status")?
            .into_int_value();
        let array_struct_type = self.llvm_type(&array_type).into_struct_type();
        let output_data_slot = self
            .builder
            .build_struct_gep(
                array_struct_type,
                output_slot,
                0,
                "simd.add.output.data.slot",
            )
            .map_err(|error| error.to_string())?;
        let output_data = self
            .builder
            .build_load(
                self.context.ptr_type(inkwell::AddressSpace::default()),
                output_data_slot,
                "simd.add.output.data",
            )
            .map_err(|error| error.to_string())?
            .into_pointer_value();
        let vector_pointer_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let vector_left = self
            .builder
            .build_pointer_cast(left_data, vector_pointer_type, "simd.add.vector.left")
            .map_err(|error| error.to_string())?;
        let vector_right = self
            .builder
            .build_pointer_cast(right_data, vector_pointer_type, "simd.add.vector.right")
            .map_err(|error| error.to_string())?;
        let vector_output = self
            .builder
            .build_pointer_cast(output_data, vector_pointer_type, "simd.add.vector.output")
            .map_err(|error| error.to_string())?;
        let scalar_pointer_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let scalar_left = self
            .builder
            .build_pointer_cast(left_data, scalar_pointer_type, "simd.add.scalar.left")
            .map_err(|error| error.to_string())?;
        let scalar_right = self
            .builder
            .build_pointer_cast(right_data, scalar_pointer_type, "simd.add.scalar.right")
            .map_err(|error| error.to_string())?;
        let scalar_output = self
            .builder
            .build_pointer_cast(output_data, scalar_pointer_type, "simd.add.scalar.output")
            .map_err(|error| error.to_string())?;

        let allocation_ok = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                allocation_status,
                self.context.i32_type().const_zero(),
                "simd.add.allocation_ok",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(allocation_ok, loop_check, allocation_failed)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(allocation_failed);
        self.emit_simd_error_result(
            result_slot,
            &variants,
            1,
            "ERR_SIMD_OUT_OF_MEMORY",
            &format!("{} could not allocate its result array", operation.name()),
        )?;
        self.builder
            .build_unconditional_branch(merge)
            .map_err(|error| error.to_string())?;

        let lanes = match &element {
            Type::I8 | Type::U8 => 16u64,
            Type::I16 | Type::U16 => 8,
            Type::I32 | Type::U32 | Type::F32 => 4,
            Type::I64 | Type::U64 | Type::F64 => 2,
            _ => unreachable!("SIMD element type was validated above"),
        };
        let scalar_type = self.llvm_type(&element);
        let vector_type = match &element {
            Type::I8 | Type::U8 => self.context.i8_type().vec_type(lanes as u32),
            Type::I16 | Type::U16 => self.context.i16_type().vec_type(lanes as u32),
            Type::I32 | Type::U32 => self.context.i32_type().vec_type(lanes as u32),
            Type::I64 | Type::U64 => self.context.i64_type().vec_type(lanes as u32),
            Type::F32 => self.context.f32_type().vec_type(lanes as u32),
            Type::F64 => self.context.f64_type().vec_type(lanes as u32),
            _ => unreachable!("SIMD element type was validated above"),
        };
        self.builder.position_at_end(loop_check);
        let vector_index = self
            .builder
            .build_phi(self.usize_type(), "simd.add.vector.index")
            .map_err(|error| error.to_string())?;
        let zero = self.usize_type().const_zero();
        vector_index.add_incoming(&[(&zero, allocate)]);
        let vector_index_value = vector_index.as_basic_value().into_int_value();
        let vector_block_index = self
            .builder
            .build_phi(self.usize_type(), "simd.add.vector.block_index")
            .map_err(|error| error.to_string())?;
        vector_block_index.add_incoming(&[(&zero, allocate)]);
        let vector_block_index_value = vector_block_index.as_basic_value().into_int_value();
        let remaining = self
            .builder
            .build_int_sub(left_length, vector_index_value, "simd.add.remaining")
            .map_err(|error| error.to_string())?;
        let vector_available = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::UGE,
                remaining,
                self.usize_type().const_int(lanes, false),
                "simd.add.vector_available",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(vector_available, vector_body, vector_done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(vector_body);
        let vector_left_ptr = unsafe {
            self.builder.build_gep(
                vector_type,
                vector_left,
                &[vector_block_index_value],
                "simd.add.vector.left.ptr",
            )
        }
        .map_err(|error| error.to_string())?;
        let vector_right_ptr = unsafe {
            self.builder.build_gep(
                vector_type,
                vector_right,
                &[vector_block_index_value],
                "simd.add.vector.right.ptr",
            )
        }
        .map_err(|error| error.to_string())?;
        let vector_output_ptr = unsafe {
            self.builder.build_gep(
                vector_type,
                vector_output,
                &[vector_block_index_value],
                "simd.add.vector.output.ptr",
            )
        }
        .map_err(|error| error.to_string())?;
        let vector_left_value = self
            .builder
            .build_load(vector_type, vector_left_ptr, "simd.add.vector.left.value")
            .map_err(|error| error.to_string())?;
        inkwell::values::BasicValue::as_instruction_value(&vector_left_value)
            .ok_or("internal: SIMD vector load did not produce an instruction")?
            .set_alignment(1)
            .map_err(|error| format!("SIMD vector load alignment: {error}"))?;
        let vector_right_value = self
            .builder
            .build_load(vector_type, vector_right_ptr, "simd.add.vector.right.value")
            .map_err(|error| error.to_string())?;
        inkwell::values::BasicValue::as_instruction_value(&vector_right_value)
            .ok_or("internal: SIMD vector load did not produce an instruction")?
            .set_alignment(1)
            .map_err(|error| format!("SIMD vector load alignment: {error}"))?;
        let vector_opcode = match (is_float, operation) {
            (true, SimdOperation::Add) => inkwell::values::InstructionOpcode::FAdd,
            (true, SimdOperation::Sub) => inkwell::values::InstructionOpcode::FSub,
            (true, SimdOperation::Mul) => inkwell::values::InstructionOpcode::FMul,
            (true, SimdOperation::Div) => inkwell::values::InstructionOpcode::FDiv,
            (false, SimdOperation::Add) => inkwell::values::InstructionOpcode::Add,
            (false, SimdOperation::Sub) => inkwell::values::InstructionOpcode::Sub,
            (false, SimdOperation::Mul) => inkwell::values::InstructionOpcode::Mul,
            (false, SimdOperation::Div) if is_signed_integer(&element) => {
                inkwell::values::InstructionOpcode::SDiv
            }
            (false, SimdOperation::Div) => inkwell::values::InstructionOpcode::UDiv,
        };
        let vector_result = self
            .builder
            .build_binop(
                vector_opcode,
                vector_left_value,
                vector_right_value,
                "simd.binary.vector.result",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(vector_output_ptr, vector_result)
            .map_err(|error| error.to_string())?
            .set_alignment(1)
            .map_err(|error| format!("SIMD vector store alignment: {error}"))?;
        let vector_next = self
            .builder
            .build_int_add(
                vector_index_value,
                self.usize_type().const_int(lanes, false),
                "simd.add.vector.next",
            )
            .map_err(|error| error.to_string())?;
        let vector_block_next = self
            .builder
            .build_int_add(
                vector_block_index_value,
                self.usize_type().const_int(1, false),
                "simd.add.vector.block_next",
            )
            .map_err(|error| error.to_string())?;
        let vector_body_end = self
            .builder
            .get_insert_block()
            .ok_or("internal: missing SIMD vector body")?;
        self.builder
            .build_unconditional_branch(loop_check)
            .map_err(|error| error.to_string())?;
        vector_index.add_incoming(&[(&vector_next, vector_body_end)]);
        vector_block_index.add_incoming(&[(&vector_block_next, vector_body_end)]);

        self.builder.position_at_end(vector_done);
        let tail_index = self
            .builder
            .build_phi(self.usize_type(), "simd.add.tail.index")
            .map_err(|error| error.to_string())?;
        tail_index.add_incoming(&[(&vector_index_value, loop_check)]);
        let tail_index_value = tail_index.as_basic_value().into_int_value();
        let tail_more = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::ULT,
                tail_index_value,
                left_length,
                "simd.add.tail.more",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(tail_more, tail_body, tail_done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(tail_body);
        let left_element_ptr = unsafe {
            self.builder.build_gep(
                scalar_type,
                scalar_left,
                &[tail_index_value],
                "simd.add.tail.left.ptr",
            )
        }
        .map_err(|error| error.to_string())?;
        let right_element_ptr = unsafe {
            self.builder.build_gep(
                scalar_type,
                scalar_right,
                &[tail_index_value],
                "simd.add.tail.right.ptr",
            )
        }
        .map_err(|error| error.to_string())?;
        let output_element_ptr = unsafe {
            self.builder.build_gep(
                scalar_type,
                scalar_output,
                &[tail_index_value],
                "simd.add.tail.output.ptr",
            )
        }
        .map_err(|error| error.to_string())?;
        let left_value = self
            .builder
            .build_load(scalar_type, left_element_ptr, "simd.add.tail.left")
            .map_err(|error| error.to_string())?;
        let right_value = self
            .builder
            .build_load(scalar_type, right_element_ptr, "simd.add.tail.right")
            .map_err(|error| error.to_string())?;
        let tail_result: BasicValueEnum<'ctx> = if is_float {
            let left_float = left_value.into_float_value();
            let right_float = right_value.into_float_value();
            match operation {
                SimdOperation::Add => {
                    self.builder
                        .build_float_add(left_float, right_float, "simd.tail.add")
                }
                SimdOperation::Sub => {
                    self.builder
                        .build_float_sub(left_float, right_float, "simd.tail.sub")
                }
                SimdOperation::Mul => {
                    self.builder
                        .build_float_mul(left_float, right_float, "simd.tail.mul")
                }
                SimdOperation::Div => {
                    self.builder
                        .build_float_div(left_float, right_float, "simd.tail.div")
                }
            }
            .map_err(|error| error.to_string())?
            .into()
        } else {
            let left_integer = left_value.into_int_value();
            let right_integer = right_value.into_int_value();
            let result = match operation {
                SimdOperation::Add => {
                    self.builder
                        .build_int_add(left_integer, right_integer, "simd.tail.add")
                }
                SimdOperation::Sub => {
                    self.builder
                        .build_int_sub(left_integer, right_integer, "simd.tail.sub")
                }
                SimdOperation::Mul => {
                    self.builder
                        .build_int_mul(left_integer, right_integer, "simd.tail.mul")
                }
                SimdOperation::Div if is_signed_integer(&element) => self
                    .builder
                    .build_int_signed_div(left_integer, right_integer, "simd.tail.div"),
                SimdOperation::Div => self.builder.build_int_unsigned_div(
                    left_integer,
                    right_integer,
                    "simd.tail.div",
                ),
            }
            .map_err(|error| error.to_string())?;
            result.into()
        };
        self.builder
            .build_store(output_element_ptr, tail_result)
            .map_err(|error| error.to_string())?;
        let tail_next = self
            .builder
            .build_int_add(
                tail_index_value,
                self.usize_type().const_int(1, false),
                "simd.add.tail.next",
            )
            .map_err(|error| error.to_string())?;
        let tail_body_end = self
            .builder
            .get_insert_block()
            .ok_or("internal: missing `simdAdd` tail body")?;
        self.builder
            .build_unconditional_branch(vector_done)
            .map_err(|error| error.to_string())?;
        tail_index.add_incoming(&[(&tail_next, tail_body_end)]);

        self.builder.position_at_end(tail_done);
        let array_struct = self.llvm_type(&array_type).into_struct_type();
        let length_slot = self
            .builder
            .build_struct_gep(array_struct, output_slot, 1, "simd.add.output.length")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(length_slot, left_length)
            .map_err(|error| error.to_string())?;
        let output = self
            .builder
            .build_load(array_struct, output_slot, "simd.add.output.value")
            .map_err(|error| error.to_string())?;
        self.store_union_variant(&variants, result_slot, 0, output)?;
        self.builder
            .build_unconditional_branch(merge)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(merge);
        let result = self
            .builder
            .build_load(
                self.union_llvm_type(&variants),
                result_slot,
                "simd.add.result.value",
            )
            .map_err(|error| error.to_string())?;
        self.record_owned_native_result(result, Type::Union(variants));
        Ok(Some(result))
    }

    fn emit_simd_input(
        &mut self,
        expr: &Expr,
        element: &Type,
        label: &str,
    ) -> Result<(PointerValue<'ctx>, inkwell::values::IntValue<'ctx>), String> {
        let hint = self
            .expr_type_hint(expr)
            .map(|ty| self.resolve_type(&ty))
            .ok_or_else(|| format!("internal: SIMD {label} argument type was not resolved"))?;
        match hint {
            Type::Array(_) => {
                let value = self
                    .emit_expr_as(expr, &Type::Array(Box::new(element.clone())))?
                    .into_struct_value();
                let data = self
                    .builder
                    .build_extract_value(value, 0, &format!("simd.{label}.data"))
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let length = self
                    .builder
                    .build_extract_value(value, 1, &format!("simd.{label}.length"))
                    .map_err(|error| error.to_string())?
                    .into_int_value();
                Ok((data, length))
            }
            Type::Tuple(elements) => {
                if elements.iter().any(|ty| self.resolve_type(ty) != *element) {
                    return Err(format!(
                        "internal: SIMD {label} fixed array has heterogeneous element types"
                    ));
                }
                let tuple = self.emit_expr(expr)?.into_struct_value();
                let llvm_element = self.llvm_type(element);
                let llvm_array = llvm_element.array_type(elements.len() as u32);
                let storage = self.entry_alloca(llvm_array, &format!("simd.{label}.fixed"))?;
                for index in 0..elements.len() {
                    let item = self
                        .builder
                        .build_extract_value(tuple, index as u32, "simd.fixed.element")
                        .map_err(|error| error.to_string())?;
                    let pointer = unsafe {
                        self.builder.build_gep(
                            llvm_array,
                            storage,
                            &[
                                self.usize_type().const_zero(),
                                self.usize_type().const_int(index as u64, false),
                            ],
                            "simd.fixed.element.ptr",
                        )
                    }
                    .map_err(|error| error.to_string())?;
                    self.builder
                        .build_store(pointer, item)
                        .map_err(|error| error.to_string())?;
                }
                let data = self
                    .builder
                    .build_pointer_cast(
                        storage,
                        self.context.ptr_type(inkwell::AddressSpace::default()),
                        &format!("simd.{label}.fixed.data"),
                    )
                    .map_err(|error| error.to_string())?;
                Ok((
                    data,
                    self.usize_type().const_int(elements.len() as u64, false),
                ))
            }
            other => Err(format!(
                "internal: SIMD {label} argument has unsupported type `{other}`"
            )),
        }
    }

    fn emit_simd_error_result(
        &mut self,
        result_slot: PointerValue<'ctx>,
        variants: &[Type],
        error_index: u8,
        code: &str,
        message: &str,
    ) -> Result<(), String> {
        let error_type = Type::Named("SimdError".to_string());
        let error = self.llvm_type(&error_type).into_struct_type().get_undef();
        let error = self
            .builder
            .build_insert_value(
                error,
                self.const_string_abi(code, "simd.error.code"),
                0,
                "simd.error.with_code",
            )
            .map_err(|error| error.to_string())?;
        let error = self
            .builder
            .build_insert_value(
                error,
                self.const_string_abi(message, "simd.error.message"),
                1,
                "simd.error.with_message",
            )
            .map_err(|error| error.to_string())?
            .into_struct_value();
        self.store_union_variant(variants, result_slot, error_index, error.into())?;
        Ok(())
    }
}

fn is_signed_integer(ty: &Type) -> bool {
    matches!(ty, Type::I8 | Type::I16 | Type::I32 | Type::I64)
}
