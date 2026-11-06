use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_logical_side_effect(
        &mut self,
        op: &BinOp,
        left: &Expr,
        right: &Expr,
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let left_value = self.emit_condition_value(left)?;
        let right_block = self
            .context
            .append_basic_block(fn_val, "logical.stmt.right");
        let merge_block = self
            .context
            .append_basic_block(fn_val, "logical.stmt.merge");
        match op {
            BinOp::And => self
                .builder
                .build_conditional_branch(left_value, right_block, merge_block)
                .unwrap(),
            BinOp::Or => self
                .builder
                .build_conditional_branch(left_value, merge_block, right_block)
                .unwrap(),
            _ => unreachable!(),
        };
        self.builder.position_at_end(right_block);
        self.push_scope();
        if matches!(op, BinOp::And)
            && let Some(name) = Self::detect_optional_presence_narrowing(left)
            && let Some(concrete) = self.optional_present_type(&name)
        {
            self.store_concrete_type(&name, concrete);
        }
        self.emit_expr_for_side_effects(right, fn_val)?;
        self.pop_scope();
        if !self.current_block_terminated() {
            self.builder
                .build_unconditional_branch(merge_block)
                .unwrap();
        }
        self.builder.position_at_end(merge_block);
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn emit_binop(
        &mut self,
        op: &BinOp,
        left: &Expr,
        right: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if matches!(op, BinOp::And | BinOp::Or) {
            return self.emit_short_circuit_boolean(op, left, right);
        }
        if matches!(op, BinOp::NullCoalesce) {
            return self.emit_null_coalesce(left, right);
        }
        if matches!(op, BinOp::Add)
            && let Some(parts) = self.collect_string_concat_chain(left, right)
        {
            return self.emit_string_concat_chain(&parts);
        }

        let hint = self
            .expr_type_hint(left)
            .or_else(|| self.expr_type_hint(right));
        if matches!(
            hint.as_ref().map(transparent_codegen_type),
            Some(Type::Decimal)
        ) && matches!(
            op,
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Pow
        ) {
            return self.emit_decimal_arithmetic(op, left, right);
        }
        if matches!(
            hint.as_ref().map(transparent_codegen_type),
            Some(Type::Decimal)
        ) && matches!(
            op,
            BinOp::Eq | BinOp::Neq | BinOp::Gt | BinOp::Lt | BinOp::Gte | BinOp::Lte
        ) {
            return self.emit_decimal_comparison(op, left, right);
        }
        let lv = if let Some(expected) = &hint {
            self.emit_expr_as(left, expected)?
        } else {
            self.emit_expr(left)?
        };
        let rv = if let Some(expected) = &hint {
            self.emit_expr_as(right, expected)?
        } else {
            self.emit_expr(right)?
        };

        let is_unsigned = hint.as_ref().is_some_and(is_unsigned_int_type);

        match op {
            BinOp::Add => {
                if lv.is_struct_value() && rv.is_struct_value() {
                    let concatenated =
                        self.emit_string_concat(lv.into_struct_value(), rv.into_struct_value())?;
                    if !self.async_results.is_empty() {
                        self.emit_async_cleanup_source_expression(lv, &Type::Str, left)?;
                        self.emit_async_cleanup_source_expression(rv, &Type::Str, right)?;
                    }
                    self.record_owned_native_result(concatenated, Type::Str);
                    Ok(concatenated)
                } else if lv.is_pointer_value() || rv.is_pointer_value() {
                    Err("error: string concatenation requires two string operands".to_string())
                } else if lv.is_int_value() && rv.is_int_value() {
                    let a = lv.into_int_value();
                    let b = rv.into_int_value();
                    let r = self.builder.build_int_add(a, b, "add").unwrap();
                    if !is_unsigned {
                        self.emit_signed_overflow_check(a, b, r, true, "+");
                    }
                    Ok(r.into())
                } else if lv.is_float_value() && rv.is_float_value() {
                    Ok(self
                        .builder
                        .build_float_add(lv.into_float_value(), rv.into_float_value(), "fadd")
                        .unwrap()
                        .into())
                } else {
                    Err("error: `+` requires operands of the same type".to_string())
                }
            }
            BinOp::Sub => {
                if lv.is_int_value() {
                    let a = lv.into_int_value();
                    let b = rv.into_int_value();
                    let r = self.builder.build_int_sub(a, b, "sub").unwrap();
                    if !is_unsigned {
                        self.emit_signed_overflow_check(a, b, r, false, "-");
                    }
                    Ok(r.into())
                } else {
                    Ok(self
                        .builder
                        .build_float_sub(lv.into_float_value(), rv.into_float_value(), "fsub")
                        .unwrap()
                        .into())
                }
            }
            BinOp::Mul => {
                if lv.is_int_value() {
                    Ok(self
                        .builder
                        .build_int_mul(lv.into_int_value(), rv.into_int_value(), "mul")
                        .unwrap()
                        .into())
                } else {
                    Ok(self
                        .builder
                        .build_float_mul(lv.into_float_value(), rv.into_float_value(), "fmul")
                        .unwrap()
                        .into())
                }
            }
            BinOp::Div => {
                if lv.is_int_value() {
                    let divisor = rv.into_int_value();
                    if !self.expr_is_known_non_zero(right) {
                        let message = "tyrus runtime error: integer division by zero".to_string();
                        self.emit_zero_check(divisor, &message);
                    }
                    if is_unsigned {
                        Ok(self
                            .builder
                            .build_int_unsigned_div(lv.into_int_value(), divisor, "udiv")
                            .unwrap()
                            .into())
                    } else {
                        Ok(self
                            .builder
                            .build_int_signed_div(lv.into_int_value(), divisor, "div")
                            .unwrap()
                            .into())
                    }
                } else {
                    Ok(self
                        .builder
                        .build_float_div(lv.into_float_value(), rv.into_float_value(), "fdiv")
                        .unwrap()
                        .into())
                }
            }
            BinOp::Mod => {
                if lv.is_int_value() && rv.is_int_value() {
                    if lv.into_int_value().get_type() != rv.into_int_value().get_type() {
                        return Err(
                            "error: `%` requires operands of the same numeric type".to_string()
                        );
                    }
                    let divisor = rv.into_int_value();
                    if !self.expr_is_known_non_zero(right) {
                        let message = "tyrus runtime error: integer remainder by zero".to_string();
                        self.emit_zero_check(divisor, &message);
                    }
                    if is_unsigned {
                        Ok(self
                            .builder
                            .build_int_unsigned_rem(lv.into_int_value(), divisor, "uremainder")
                            .unwrap()
                            .into())
                    } else {
                        Ok(self
                            .builder
                            .build_int_signed_rem(lv.into_int_value(), divisor, "remainder")
                            .unwrap()
                            .into())
                    }
                } else if lv.is_float_value() && rv.is_float_value() {
                    if lv.into_float_value().get_type() != rv.into_float_value().get_type() {
                        return Err(
                            "error: `%` requires operands of the same numeric type".to_string()
                        );
                    }
                    Ok(self
                        .builder
                        .build_float_rem(lv.into_float_value(), rv.into_float_value(), "fremainder")
                        .unwrap()
                        .into())
                } else {
                    Err("error: `%` requires operands of the same numeric type".to_string())
                }
            }
            BinOp::Pow => {
                if lv.is_int_value() && rv.is_int_value() {
                    let left = lv.into_int_value();
                    let right = rv.into_int_value();
                    if left.get_type() != right.get_type() {
                        return Err(
                            "error: `**` requires operands of the same numeric type".to_string()
                        );
                    }
                    let pow = self.declare_int_pow(left.get_type());
                    Ok(self
                        .builder
                        .build_call(pow, &[left.into(), right.into()], "power")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap())
                } else if lv.is_float_value() && rv.is_float_value() {
                    let left = lv.into_float_value();
                    let right = rv.into_float_value();
                    if left.get_type() != right.get_type() {
                        return Err(
                            "error: `**` requires operands of the same numeric type".to_string()
                        );
                    }
                    let pow = Intrinsic::find("llvm.pow")
                        .and_then(|intrinsic| {
                            intrinsic.get_declaration(&self.module, &[left.get_type().into()])
                        })
                        .ok_or("internal: LLVM `pow` intrinsic is unavailable")?;
                    Ok(self
                        .builder
                        .build_call(pow, &[left.into(), right.into()], "power")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap())
                } else {
                    Err("error: `**` requires operands of the same numeric type".to_string())
                }
            }
            BinOp::Eq => self.emit_value_equality(lv, rv, left, right),
            BinOp::Neq => {
                if lv.is_int_value() {
                    Ok(self
                        .builder
                        .build_int_compare(
                            IntPredicate::NE,
                            lv.into_int_value(),
                            rv.into_int_value(),
                            "neq",
                        )
                        .unwrap()
                        .into())
                } else if self.is_string_value(lv) && self.is_string_value(rv) {
                    self.emit_string_equality(lv.into_struct_value(), rv.into_struct_value(), false)
                } else if (lv.is_pointer_value() || self.is_string_value(lv))
                    && (rv.is_pointer_value() || self.is_string_value(rv))
                {
                    let operand = if matches!(right.kind(), ExprKind::Null) {
                        lv
                    } else if matches!(left.kind(), ExprKind::Null) {
                        rv
                    } else {
                        return Err("error: pointer inequality is only supported against `null`"
                            .to_string());
                    };
                    let is_null = if self.is_string_value(operand) {
                        self.string_is_null(operand.into_struct_value())?
                    } else {
                        self.builder
                            .build_is_null(operand.into_pointer_value(), "is_null")
                            .unwrap()
                    };
                    Ok(self
                        .builder
                        .build_not(is_null, "is_not_null")
                        .unwrap()
                        .into())
                } else {
                    Ok(self
                        .builder
                        .build_float_compare(
                            inkwell::FloatPredicate::UNE,
                            lv.into_float_value(),
                            rv.into_float_value(),
                            "fneq",
                        )
                        .unwrap()
                        .into())
                }
            }
            BinOp::Gt => {
                let (int_pred, float_pred) = if is_unsigned {
                    (IntPredicate::UGT, inkwell::FloatPredicate::OGT)
                } else {
                    (IntPredicate::SGT, inkwell::FloatPredicate::OGT)
                };
                self.emit_comparison(lv, rv, int_pred, float_pred, "gt")
            }
            BinOp::Lt => {
                let (int_pred, float_pred) = if is_unsigned {
                    (IntPredicate::ULT, inkwell::FloatPredicate::OLT)
                } else {
                    (IntPredicate::SLT, inkwell::FloatPredicate::OLT)
                };
                self.emit_comparison(lv, rv, int_pred, float_pred, "lt")
            }
            BinOp::Gte => {
                let (int_pred, float_pred) = if is_unsigned {
                    (IntPredicate::UGE, inkwell::FloatPredicate::OGE)
                } else {
                    (IntPredicate::SGE, inkwell::FloatPredicate::OGE)
                };
                self.emit_comparison(lv, rv, int_pred, float_pred, "gte")
            }
            BinOp::Lte => {
                let (int_pred, float_pred) = if is_unsigned {
                    (IntPredicate::ULE, inkwell::FloatPredicate::OLE)
                } else {
                    (IntPredicate::SLE, inkwell::FloatPredicate::OLE)
                };
                self.emit_comparison(lv, rv, int_pred, float_pred, "lte")
            }
            BinOp::BitAnd => {
                if lv.is_int_value() && rv.is_int_value() {
                    Ok(self
                        .builder
                        .build_and(lv.into_int_value(), rv.into_int_value(), "bitand")
                        .unwrap()
                        .into())
                } else {
                    Err("error: `&` requires integer operands".to_string())
                }
            }
            BinOp::BitOr => {
                if lv.is_int_value() && rv.is_int_value() {
                    Ok(self
                        .builder
                        .build_or(lv.into_int_value(), rv.into_int_value(), "bitor")
                        .unwrap()
                        .into())
                } else {
                    Err("error: `|` requires integer operands".to_string())
                }
            }
            BinOp::BitXor => {
                if lv.is_int_value() && rv.is_int_value() {
                    Ok(self
                        .builder
                        .build_xor(lv.into_int_value(), rv.into_int_value(), "bitxor")
                        .unwrap()
                        .into())
                } else {
                    Err("error: `^` requires integer operands".to_string())
                }
            }
            BinOp::Shl => {
                if lv.is_int_value() && rv.is_int_value() {
                    Ok(self
                        .builder
                        .build_left_shift(lv.into_int_value(), rv.into_int_value(), "shl")
                        .unwrap()
                        .into())
                } else {
                    Err("error: `<<` requires integer operands".to_string())
                }
            }
            BinOp::Shr => {
                if lv.is_int_value() && rv.is_int_value() {
                    let arithmetic = !is_unsigned;
                    Ok(self
                        .builder
                        .build_right_shift(
                            lv.into_int_value(),
                            rv.into_int_value(),
                            arithmetic,
                            "shr",
                        )
                        .unwrap()
                        .into())
                } else {
                    Err("error: `>>` requires integer operands".to_string())
                }
            }
            BinOp::UShr => {
                if lv.is_int_value() && rv.is_int_value() {
                    Ok(self
                        .builder
                        .build_right_shift(lv.into_int_value(), rv.into_int_value(), false, "ushr")
                        .unwrap()
                        .into())
                } else {
                    Err("error: `>>>` requires integer operands".to_string())
                }
            }
            BinOp::And | BinOp::Or | BinOp::NullCoalesce => {
                unreachable!("lazy operators are emitted before eager operands")
            }
        }
    }

    pub(super) fn emit_decimal_arithmetic(
        &mut self,
        op: &BinOp,
        left: &Expr,
        right: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let decimal = self.llvm_type(&Type::Decimal);
        if let Some(value) = codegen_constant_decimal(&Expr::synthetic(ExprKind::BinOp {
            op: op.clone(),
            left: Box::new(left.clone()),
            right: Box::new(right.clone()),
        })) {
            let value = value?;
            return Ok(decimal
                .into_struct_type()
                .const_named_struct(&[
                    self.const_i128_value(value.coefficient).into(),
                    self.context
                        .i32_type()
                        .const_int(value.exponent as u64, true)
                        .into(),
                ])
                .into());
        }
        let left_value = self.emit_expr_as(left, &Type::Decimal)?;
        let right_value = self.emit_expr_as(right, &Type::Decimal)?;
        let left_slot = self.builder.build_alloca(decimal, "decimal.left").unwrap();
        let right_slot = self.builder.build_alloca(decimal, "decimal.right").unwrap();
        let result_slot = self
            .builder
            .build_alloca(decimal, "decimal.result")
            .unwrap();
        self.builder.build_store(left_slot, left_value).unwrap();
        self.builder.build_store(right_slot, right_value).unwrap();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let function = self.declare_libc_fn(
            match op {
                BinOp::Add => "tyrus_decimal_add",
                BinOp::Sub => "tyrus_decimal_sub",
                BinOp::Mul => "tyrus_decimal_mul",
                BinOp::Div => "tyrus_decimal_div",
                BinOp::Pow => "tyrus_decimal_pow",
                _ => unreachable!("decimal arithmetic codegen received a different operator"),
            },
            &[pointer.into(), pointer.into(), pointer.into()],
            None,
        );
        self.builder
            .build_call(
                function,
                &[result_slot.into(), left_slot.into(), right_slot.into()],
                "",
            )
            .unwrap();
        Ok(self
            .builder
            .build_load(decimal, result_slot, "decimal.value")
            .unwrap())
    }

    pub(super) fn emit_decimal_comparison(
        &mut self,
        op: &BinOp,
        left: &Expr,
        right: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if let (Some(left), Some(right)) = (
            codegen_constant_decimal(left),
            codegen_constant_decimal(right),
        ) {
            let ordering = left?
                .compare(right?)
                .map_err(|error| format!("decimal constant evaluation failed: {error:?}"))?;
            let result = match op {
                BinOp::Eq => ordering.is_eq(),
                BinOp::Neq => !ordering.is_eq(),
                BinOp::Gt => ordering.is_gt(),
                BinOp::Lt => ordering.is_lt(),
                BinOp::Gte => !ordering.is_lt(),
                BinOp::Lte => !ordering.is_gt(),
                _ => unreachable!("decimal comparison codegen received a different operator"),
            };
            return Ok(self
                .context
                .bool_type()
                .const_int(u64::from(result), false)
                .into());
        }
        let decimal = self.llvm_type(&Type::Decimal);
        let left_value = self.emit_expr_as(left, &Type::Decimal)?;
        let right_value = self.emit_expr_as(right, &Type::Decimal)?;
        let left_slot = self
            .builder
            .build_alloca(decimal, "decimal.compare.left")
            .unwrap();
        let right_slot = self
            .builder
            .build_alloca(decimal, "decimal.compare.right")
            .unwrap();
        self.builder.build_store(left_slot, left_value).unwrap();
        self.builder.build_store(right_slot, right_value).unwrap();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let compare = self.declare_libc_fn(
            "tyrus_decimal_compare",
            &[pointer.into(), pointer.into()],
            Some(self.context.i32_type().into()),
        );
        let result = self
            .builder
            .build_call(
                compare,
                &[left_slot.into(), right_slot.into()],
                "decimal.compare",
            )
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        let predicate = match op {
            BinOp::Eq => IntPredicate::EQ,
            BinOp::Neq => IntPredicate::NE,
            BinOp::Gt => IntPredicate::SGT,
            BinOp::Lt => IntPredicate::SLT,
            BinOp::Gte => IntPredicate::SGE,
            BinOp::Lte => IntPredicate::SLE,
            _ => unreachable!("decimal comparison codegen received a different operator"),
        };
        Ok(self
            .builder
            .build_int_compare(
                predicate,
                result,
                self.context.i32_type().const_zero(),
                "decimal.comparison",
            )
            .unwrap()
            .into())
    }

    pub(super) fn emit_null_coalesce(
        &mut self,
        left: &Expr,
        right: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let Type::Optional(inner) = self
            .expr_type_hint(left)
            .ok_or("error: cannot infer nullish operand type")?
        else {
            return Err("error: left operand of `??` must be optional".to_string());
        };
        let inner = self.resolve_type(&inner);
        let owned_result = !inner.is_copy() && self.async_owned_value_supported(&inner);
        // Whether the box's payload is *provably* an allocation, as opposed to
        // merely being a non-`Copy` type. `owned_result` alone is not enough: an
        // `OptionalFieldAccess` box holds whatever the field currently has,
        // which can be a static string a struct literal was built with, and
        // freeing that is corruption, not a leak, confirmed with a segfault
        // before this was narrowed. An `OptionalMethodCall` is different: every
        // string-returning method reachable through `?.` genuinely allocates,
        // matching the same reasoning `async_owned_string_expression` already
        // relies on elsewhere. So does a direct call to one of the handful of
        // native builtins that return `Optional<Str>` already boxed on the
        // runtime side (`box_optional_pointer` in `os.rs`/`fs.rs`), `mime`,
        // `clipboardRead`, `desktopDir`, `documentsDir`, `whoami`, every
        // present payload there is a fresh `alloc_string`/native allocation,
        // never a borrowed or static string, matching `OptionalMethodCall`'s
        // guarantee exactly. A `left` that is none of these, an `Ident`
        // referring to an optional bound earlier, say, is conservatively left
        // alone too: its provenance is not visible here.
        let present_payload_is_provably_owned = owned_result
            && (matches!(left.kind(), ExprKind::OptionalMethodCall { .. })
                || self.async_owned_array_removal(left)
                || self.http_response_header_call(left)
                || matches!(
                    left.kind(),
                    ExprKind::FnCall { callee, .. }
                        if matches!(
                            callee.as_str(),
                            "__tyrus_fs_mime"
                                | "__tyrus_os_desktopDir"
                                | "__tyrus_os_documentsDir"
                                | "__tyrus_os_whoami"
                                | "__tyrus_os_clipboardRead"
                        )
                ));
        let left_value = self.emit_expr(left)?;
        let left_pointer = left_value.into_pointer_value();
        let async_fresh_box = if self.async_results.is_empty() {
            false
        } else {
            match left.kind() {
                ExprKind::OptionalMethodCall { .. } => true,
                ExprKind::OptionalFieldAccess { object, field } => {
                    let receiver = match self.expr_type_hint(object) {
                        Some(Type::Optional(receiver)) => Some(receiver),
                        _ => None,
                    };
                    receiver
                        .and_then(|receiver| self.field_type_on(&receiver, field))
                        .is_some_and(|field_type| {
                            !matches!(transparent_codegen_type(&field_type), Type::Optional(_))
                        })
                }
                _ => self.async_owned_array_removal(left),
            }
        };
        let owns_left_box = self.forget_owned_native_result(left_value) || async_fresh_box;
        let is_null = self.builder.build_is_null(left_pointer, "is_null").unwrap();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: nullish expression outside function")?;
        let fallback = self
            .context
            .append_basic_block(function, "coalesce.fallback");
        let present = self
            .context
            .append_basic_block(function, "coalesce.present");
        let merge = self.context.append_basic_block(function, "coalesce.merge");
        self.builder
            .build_conditional_branch(is_null, fallback, present)
            .unwrap();

        self.builder.position_at_end(fallback);
        if owns_left_box {
            self.emit_pointer_free(left_pointer);
        }
        let fallback_source = self.emit_expr_as(right, &inner)?;
        let fallback_value = if owned_result {
            let slot = self
                .entry_alloca(self.llvm_type(&inner), "coalesce.fallback.owned")
                .expect("a stack slot needs an enclosing function");
            self.emit_async_clone_value_into(slot, fallback_source, &inner)?;
            self.emit_async_cleanup_source_expression(fallback_source, &inner, right)?;
            self.builder
                .build_load(self.llvm_type(&inner), slot, "coalesce.fallback.value")
                .unwrap()
        } else {
            fallback_source
        };
        self.builder.build_unconditional_branch(merge).unwrap();
        let fallback_block = self.builder.get_insert_block().unwrap();

        self.builder.position_at_end(present);
        let present_source = self
            .builder
            .build_load(self.llvm_type(&inner), left_pointer, "optional_value")
            .unwrap();
        if owns_left_box {
            self.emit_pointer_free(left_pointer);
        }
        let present_value = if owned_result {
            let slot = self
                .entry_alloca(self.llvm_type(&inner), "coalesce.present.owned")
                .expect("a stack slot needs an enclosing function");
            self.emit_async_clone_value_into(slot, present_source, &inner)?;
            if present_payload_is_provably_owned {
                let discard_slot = self
                    .entry_alloca(self.llvm_type(&inner), "coalesce.present.discard")
                    .expect("a stack slot needs an enclosing function");
                self.builder
                    .build_store(discard_slot, present_source)
                    .unwrap();
                self.emit_async_drop_value_in_place(function, discard_slot, &inner)?;
            }
            self.builder
                .build_load(self.llvm_type(&inner), slot, "coalesce.present.value")
                .unwrap()
        } else {
            present_source
        };
        self.builder.build_unconditional_branch(merge).unwrap();
        let present_block = self.builder.get_insert_block().unwrap();

        self.builder.position_at_end(merge);
        let phi = self
            .builder
            .build_phi(self.llvm_type(&inner), "coalesce")
            .unwrap();
        phi.add_incoming(&[
            (&fallback_value, fallback_block),
            (&present_value, present_block),
        ]);
        let value = phi.as_basic_value();
        if owned_result {
            self.record_owned_native_result(value, inner);
        }
        Ok(value)
    }

    pub(super) fn emit_chained_comparison(
        &mut self,
        terms: &[Expr],
        ops: &[BinOp],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: chained comparison outside function")?;
        let merge_block = self.context.append_basic_block(function, "chaincmp.merge");

        let hint = terms.iter().find_map(|term| self.expr_type_hint(term));
        let is_unsigned = hint.as_ref().is_some_and(is_unsigned_int_type);
        let emit_term = |this: &mut Self, term: &Expr| -> Result<BasicValueEnum<'ctx>, String> {
            match &hint {
                Some(expected) => this.emit_expr_as(term, expected),
                None => this.emit_expr(term),
            }
        };

        let mut prev_value = emit_term(self, &terms[0])?;
        let mut incoming: Vec<(BasicValueEnum<'ctx>, BasicBlock<'ctx>)> =
            Vec::with_capacity(ops.len());
        for (i, op) in ops.iter().enumerate() {
            let next_value = emit_term(self, &terms[i + 1])?;
            let cmp = match op {
                BinOp::Gt => {
                    let (int_predicate, float_predicate) = if is_unsigned {
                        (IntPredicate::UGT, inkwell::FloatPredicate::OGT)
                    } else {
                        (IntPredicate::SGT, inkwell::FloatPredicate::OGT)
                    };
                    self.emit_comparison(
                        prev_value,
                        next_value,
                        int_predicate,
                        float_predicate,
                        "chaincmp.gt",
                    )?
                }
                BinOp::Lt => {
                    let (int_predicate, float_predicate) = if is_unsigned {
                        (IntPredicate::ULT, inkwell::FloatPredicate::OLT)
                    } else {
                        (IntPredicate::SLT, inkwell::FloatPredicate::OLT)
                    };
                    self.emit_comparison(
                        prev_value,
                        next_value,
                        int_predicate,
                        float_predicate,
                        "chaincmp.lt",
                    )?
                }
                BinOp::Gte => {
                    let (int_predicate, float_predicate) = if is_unsigned {
                        (IntPredicate::UGE, inkwell::FloatPredicate::OGE)
                    } else {
                        (IntPredicate::SGE, inkwell::FloatPredicate::OGE)
                    };
                    self.emit_comparison(
                        prev_value,
                        next_value,
                        int_predicate,
                        float_predicate,
                        "chaincmp.gte",
                    )?
                }
                BinOp::Lte => {
                    let (int_predicate, float_predicate) = if is_unsigned {
                        (IntPredicate::ULE, inkwell::FloatPredicate::OLE)
                    } else {
                        (IntPredicate::SLE, inkwell::FloatPredicate::OLE)
                    };
                    self.emit_comparison(
                        prev_value,
                        next_value,
                        int_predicate,
                        float_predicate,
                        "chaincmp.lte",
                    )?
                }
                BinOp::Eq => {
                    self.emit_value_equality(prev_value, next_value, &terms[i], &terms[i + 1])?
                }
                _ => unreachable!("chained comparisons only ever use <, <=, >, >=, =="),
            }
            .into_int_value();

            let current_block = self.builder.get_insert_block().unwrap();
            if i + 1 == ops.len() {
                self.builder
                    .build_unconditional_branch(merge_block)
                    .unwrap();
                incoming.push((cmp.into(), current_block));
            } else {
                let next_block = self.context.append_basic_block(function, "chaincmp.next");
                self.builder
                    .build_conditional_branch(cmp, next_block, merge_block)
                    .unwrap();
                incoming.push((cmp.into(), current_block));
                self.builder.position_at_end(next_block);
                prev_value = next_value;
            }
        }

        self.builder.position_at_end(merge_block);
        let phi = self
            .builder
            .build_phi(self.context.bool_type(), "chaincmp")
            .unwrap();
        for (value, block) in &incoming {
            phi.add_incoming(&[(value, *block)]);
        }
        Ok(phi.as_basic_value())
    }

    pub(crate) fn emit_comparison(
        &self,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
        int_predicate: IntPredicate,
        float_predicate: inkwell::FloatPredicate,
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if left.is_int_value() && right.is_int_value() {
            Ok(self
                .builder
                .build_int_compare(
                    int_predicate,
                    left.into_int_value(),
                    right.into_int_value(),
                    name,
                )
                .unwrap()
                .into())
        } else if left.is_float_value() && right.is_float_value() {
            Ok(self
                .builder
                .build_float_compare(
                    float_predicate,
                    left.into_float_value(),
                    right.into_float_value(),
                    name,
                )
                .unwrap()
                .into())
        } else {
            Err("error: comparison requires matching numeric operands".to_string())
        }
    }
}
