use super::*;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_assignment_stmt(
        &mut self,
        stmt: &Stmt,
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        match stmt.kind() {
            StmtKind::Assign { name, op, value } => {
                let string_ownership = self.async_frame_string_ownership(name);
                let tracked_copy_array = self.async_frame_tracks_copy_array(name);
                let string_array_ownership = self.async_frame_string_array_ownership(name);
                let mutable_string_ownership = self.mutable_string_ownership_slot(name);
                let sync_string_array_ownership = self.string_array_ownership_slot(name);
                if string_ownership.is_some()
                    && !matches!(op, AssignOp::Assign | AssignOp::AddAssign)
                {
                    return Err(format!(
                        "error: compound assignment for mutable async string local `{name}` does not have ownership-safe lowering yet"
                    ));
                }
                if mutable_string_ownership.is_some() && !matches!(op, AssignOp::Assign) {
                    return Err(format!(
                        "error: compound assignment for mutable string local `{name}` does not have ownership-safe lowering yet"
                    ));
                }
                if tracked_copy_array && !matches!(op, AssignOp::Assign) {
                    return Err(format!(
                        "error: compound assignment for mutable async array local `{name}` does not have ownership-safe lowering yet"
                    ));
                }
                if string_array_ownership.is_some() && !matches!(op, AssignOp::Assign) {
                    return Err(format!(
                        "error: compound assignment for mutable async string-array local `{name}` does not have ownership-safe lowering yet"
                    ));
                }
                if matches!(
                    op,
                    AssignOp::AndAssign | AssignOp::OrAssign | AssignOp::NullCoalesceAssign
                ) {
                    return self.emit_logical_assignment(name, op, value, fn_val);
                }
                let (ptr, target_ty) = self
                    .load_var(name)
                    .ok_or_else(|| format!("internal: variable `{name}` not found in codegen"))?;
                let resolved_target = transparent_codegen_type(&target_ty).clone();
                if let Type::Union(variants) = &resolved_target {
                    if !matches!(op, AssignOp::Assign) {
                        return Err(
                            "error: compound assignment is not supported for union types"
                                .to_string(),
                        );
                    }
                    if self.expr_type_hint(value).is_some_and(|hint| {
                        matches!(
                            transparent_codegen_type(&self.resolve_type(&hint)),
                            Type::Union(_)
                        )
                    }) && !matches!(value.kind(), ExprKind::Ternary { .. })
                    {
                        let val = self.emit_expr_as(value, &target_ty)?;
                        self.builder.build_store(ptr, val).unwrap();
                        self.clear_concrete_type(name);
                    } else {
                        let (val, concrete) = self.build_union_value(variants, value, name)?;
                        self.builder.build_store(ptr, val).unwrap();
                        self.store_concrete_type(name, concrete);
                    }
                    return Ok(());
                }
                let val = match op {
                    AssignOp::Assign => self.emit_expr_as(value, &target_ty)?,
                    AssignOp::AddAssign => self.emit_binop(
                        &BinOp::Add,
                        &Expr::synthetic(ExprKind::Ident(name.clone())),
                        value,
                    )?,
                    AssignOp::SubAssign => self.emit_binop(
                        &BinOp::Sub,
                        &Expr::synthetic(ExprKind::Ident(name.clone())),
                        value,
                    )?,
                    AssignOp::MulAssign => self.emit_binop(
                        &BinOp::Mul,
                        &Expr::synthetic(ExprKind::Ident(name.clone())),
                        value,
                    )?,
                    AssignOp::DivAssign => self.emit_binop(
                        &BinOp::Div,
                        &Expr::synthetic(ExprKind::Ident(name.clone())),
                        value,
                    )?,
                    AssignOp::ModAssign => self.emit_binop(
                        &BinOp::Mod,
                        &Expr::synthetic(ExprKind::Ident(name.clone())),
                        value,
                    )?,
                    AssignOp::PowAssign => self.emit_binop(
                        &BinOp::Pow,
                        &Expr::synthetic(ExprKind::Ident(name.clone())),
                        value,
                    )?,
                    AssignOp::BitAndAssign => self.emit_binop(
                        &BinOp::BitAnd,
                        &Expr::synthetic(ExprKind::Ident(name.clone())),
                        value,
                    )?,
                    AssignOp::BitOrAssign => self.emit_binop(
                        &BinOp::BitOr,
                        &Expr::synthetic(ExprKind::Ident(name.clone())),
                        value,
                    )?,
                    AssignOp::BitXorAssign => self.emit_binop(
                        &BinOp::BitXor,
                        &Expr::synthetic(ExprKind::Ident(name.clone())),
                        value,
                    )?,
                    AssignOp::ShlAssign => self.emit_binop(
                        &BinOp::Shl,
                        &Expr::synthetic(ExprKind::Ident(name.clone())),
                        value,
                    )?,
                    AssignOp::ShrAssign => self.emit_binop(
                        &BinOp::Shr,
                        &Expr::synthetic(ExprKind::Ident(name.clone())),
                        value,
                    )?,
                    AssignOp::UShrAssign => self.emit_binop(
                        &BinOp::UShr,
                        &Expr::synthetic(ExprKind::Ident(name.clone())),
                        value,
                    )?,
                    AssignOp::AndAssign | AssignOp::OrAssign | AssignOp::NullCoalesceAssign => {
                        unreachable!()
                    }
                };
                if matches!(op, AssignOp::Assign) {
                    // A `T?` reassigned from a bare value, a literal, a
                    // numeric cast, anything whose own type is not already
                    // `Optional`, passes through `emit_optional_box` inside
                    // `emit_expr_as` above, which never registers its own
                    // release. `VarDecl` already guards this exact shape for
                    // an initializer; a later reassignment of the same
                    // binding needs the identical registration; otherwise
                    // `prepare_owned_native_assignment` below has no pending
                    // entry to find and the box leaks every time.
                    let value_is_boxed_here =
                        matches!(transparent_codegen_type(&target_ty), Type::Optional(_))
                            && !matches!(value.kind(), ExprKind::Null | ExprKind::None)
                            && !self
                                .expr_type_hint(value)
                                .map(|hint| self.resolve_type(&hint))
                                .is_some_and(|hint| {
                                    matches!(transparent_codegen_type(&hint), Type::Optional(_))
                                })
                            && self.pending_owned_native_results.is_empty();
                    if value_is_boxed_here {
                        self.record_owned_native_result_with(
                            val,
                            target_ty.clone(),
                            crate::ReleasePlan::Value,
                        );
                    }
                    self.prepare_owned_native_assignment(name, ptr, &target_ty, val);
                    self.prepare_conditionally_owned_assignment(name, ptr, &target_ty, val);
                }
                if self.async_frame_owns_value(name) && matches!(op, AssignOp::Assign) {
                    let temporary = self
                        .builder
                        .build_alloca(self.llvm_type(&target_ty), "async.local.replacement")
                        .unwrap();
                    self.builder
                        .build_store(temporary, self.llvm_type(&target_ty).const_zero())
                        .unwrap();
                    self.emit_async_clone_value_into(temporary, val, &target_ty)?;
                    self.emit_async_cleanup_source_expression(val, &target_ty, value)?;
                    self.clear_async_frame_local_before_store(fn_val, name, ptr, &target_ty)?;
                    let replacement = self
                        .builder
                        .build_load(
                            self.llvm_type(&target_ty),
                            temporary,
                            "async.local.replacement.value",
                        )
                        .unwrap();
                    self.builder.build_store(ptr, replacement).unwrap();
                    self.mark_async_frame_value_initialized(name, true);
                } else if let Some(ownership) = string_ownership {
                    let new_owned = if matches!(op, AssignOp::AddAssign) {
                        true
                    } else if matches!(value.kind(), ExprKind::StrLit(_)) {
                        false
                    } else if self.async_owned_string_expression(value) {
                        true
                    } else {
                        return Err(format!(
                            "error: assignment to mutable async string local `{name}` requires a value with known ownership"
                        ));
                    };
                    self.forget_owned_native_result(val);
                    self.replace_async_string_local(
                        fn_val,
                        name,
                        ptr,
                        ownership,
                        val.into_struct_value(),
                        new_owned,
                    );
                } else if tracked_copy_array {
                    if !self.async_owned_copy_array_expression(value) {
                        return Err(format!(
                            "error: assignment to mutable async array local `{name}` requires a fresh owned buffer"
                        ));
                    }
                    self.forget_owned_native_result(val);
                    self.replace_async_copy_array_local(fn_val, name, ptr, val.into_struct_value());
                } else if let Some(ownership) = string_array_ownership {
                    let new_mode = self.async_string_array_ownership_mode(value).ok_or_else(|| {
                        format!(
                            "error: assignment to mutable async string-array local `{name}` requires known element ownership"
                        )
                    })?;
                    self.forget_owned_native_result(val);
                    self.replace_async_string_array_local(
                        fn_val,
                        name,
                        ptr,
                        ownership,
                        val.into_struct_value(),
                        new_mode,
                    );
                } else if let Some(mode_slot) = sync_string_array_ownership {
                    let stored_type = self
                        .load_var(name)
                        .map_or_else(|| Type::Array(Box::new(Type::Str)), |(_, ty)| ty);
                    self.replace_string_array_ownership(
                        fn_val,
                        name,
                        ptr,
                        mode_slot,
                        val,
                        &stored_type,
                    );
                } else if let Some(bit_slot) = mutable_string_ownership {
                    let is_owned = self
                        .pending_owned_native_results
                        .iter()
                        .any(|(pending, _, _)| pending.as_value_ref() == val.as_value_ref());
                    if is_owned {
                        self.forget_owned_native_result(val);
                    }
                    self.replace_mutable_string_ownership(
                        fn_val,
                        name,
                        ptr,
                        bit_slot,
                        val.into_struct_value(),
                        is_owned,
                    );
                } else {
                    self.builder.build_store(ptr, val).unwrap();
                }
                self.discard_owned_native_result();
                Ok(())
            }

            StmtKind::IndexAssign { name, index, value } => {
                if let Some(ownership) = self.async_frame_string_array_ownership(name) {
                    let (slot, ty) = self.async_frame_slot(name).ok_or_else(|| {
                        format!("internal: async string-array local `{name}` has no frame slot")
                    })?;
                    self.normalize_async_string_array_for_mutation(
                        fn_val, name, slot, ownership, &ty,
                    )?;
                }
                let (array_slot, array_type) = self
                    .load_var(name)
                    .ok_or_else(|| format!("internal: `{name}` not found in codegen"))?;
                if let ExprKind::StrLit(field) = index.kind()
                    && let Some(field_index) = self.struct_field_index(&array_type, field)
                {
                    let field_ptr = self
                        .builder
                        .build_struct_gep(
                            self.llvm_type(&array_type),
                            array_slot,
                            field_index,
                            field,
                        )
                        .map_err(|error| error.to_string())?;
                    let field_type = self
                        .struct_field_type(&array_type, field)
                        .ok_or_else(|| format!("internal: field `{field}` has no type"))?;
                    let new_value = self.emit_expr_as(value, &field_type)?;
                    self.builder
                        .build_store(field_ptr, new_value)
                        .map_err(|error| error.to_string())?;
                    return Ok(());
                }
                if let Type::Map(key_ty, value_ty) = transparent_codegen_type(&array_type).clone() {
                    let handle = self
                        .builder
                        .build_load(self.llvm_type(&array_type), array_slot, name)
                        .unwrap()
                        .into_pointer_value();
                    return self.emit_map_insert(handle, index, &key_ty, value, &value_ty);
                }
                if let Type::Tuple(elements) = transparent_codegen_type(&array_type).clone() {
                    let Expr {
                        kind: ExprKind::IntLit(literal),
                        ..
                    } = index
                    else {
                        return Err("error: tuple index must be an integer literal".to_string());
                    };
                    let position = usize::try_from(*literal)
                        .map_err(|_| "error: tuple index out of bounds".to_string())?;
                    let element = elements
                        .get(position)
                        .ok_or_else(|| "error: tuple index out of bounds".to_string())?;
                    let struct_type = self.llvm_type(&array_type).into_struct_type();
                    let field_ptr = self
                        .builder
                        .build_struct_gep(
                            struct_type,
                            array_slot,
                            position as u32,
                            "tuple_index_target",
                        )
                        .unwrap();
                    let value = self.emit_expr_as(value, element)?;
                    self.builder.build_store(field_ptr, value).unwrap();
                    return Ok(());
                }
                let Type::Array(element) = transparent_codegen_type(&array_type) else {
                    return Err(format!("error: cannot index into type `{array_type}`"));
                };
                let array = self
                    .builder
                    .build_load(self.llvm_type(&array_type), array_slot, name)
                    .unwrap()
                    .into_struct_value();
                let data = self
                    .builder
                    .build_extract_value(array, 0, "array_data")
                    .unwrap()
                    .into_pointer_value();
                let length = self
                    .builder
                    .build_extract_value(array, 1, "array_length")
                    .unwrap()
                    .into_int_value();
                let proved = self.checked_indexes.contains(index.span);
                let index = self.emit_expr_as(index, &Type::Usize)?.into_int_value();
                if !proved {
                    self.emit_array_bounds_check(index, length)?;
                }
                let target = unsafe {
                    self.builder
                        .build_gep(
                            self.llvm_type(element),
                            data,
                            &[index],
                            "array_index_target",
                        )
                        .unwrap()
                };
                let source_value = self.emit_expr_as(value, element)?;
                let mut stored_value = source_value;
                if self.async_frame_string_array_ownership(name).is_some()
                    && matches!(transparent_codegen_type(element), Type::Str)
                {
                    stored_value = self.emit_string_clone(
                        source_value.into_struct_value(),
                        "async.string.array.element.clone",
                    )?;
                    let previous = self
                        .builder
                        .build_load(
                            self.string_abi_type(),
                            target,
                            "async.string.array.element.previous",
                        )
                        .unwrap()
                        .into_struct_value();
                    self.emit_string_release(previous)?;
                    self.emit_async_cleanup_source_expression(source_value, element, value)?;
                }
                self.builder.build_store(target, stored_value).unwrap();
                Ok(())
            }

            StmtKind::FieldAssign {
                object,
                field,
                op,
                value,
            } => {
                let target = Expr::synthetic(ExprKind::FieldAccess {
                    object: Box::new(object.clone()),
                    field: field.clone(),
                });
                let (field_ptr, field_ty) = self.emit_place_ptr(&target)?;
                if !transparent_codegen_type(&field_ty).is_copy() {
                    if !matches!(op, AssignOp::Assign) {
                        return Err(format!(
                            "error: compound assignment `{op:?}` is not supported for owned field `{field}`"
                        ));
                    }
                    if !self.async_owned_value_supported(&field_ty) {
                        return Err(format!(
                            "error: assignment to owned field `{field}` requires supported clone and drop glue"
                        ));
                    }
                    let replacement = self.emit_expr_as(value, &field_ty)?;
                    let temporary = self
                        .builder
                        .build_alloca(self.llvm_type(&field_ty), "owned.field.replacement")
                        .unwrap();
                    self.builder
                        .build_store(temporary, self.llvm_type(&field_ty).const_zero())
                        .unwrap();
                    self.emit_async_clone_value_into(temporary, replacement, &field_ty)?;
                    self.emit_async_cleanup_source_expression(replacement, &field_ty, value)?;
                    self.emit_async_drop_value_in_place(fn_val, field_ptr, &field_ty)?;
                    let replacement = self
                        .builder
                        .build_load(
                            self.llvm_type(&field_ty),
                            temporary,
                            "owned.field.replacement.value",
                        )
                        .unwrap();
                    self.emit_assigned_field_validator_checks(object, field, replacement)?;
                    self.builder.build_store(field_ptr, replacement).unwrap();
                    if let Some(root) = tyrus_core::ast::place_root_ident(object) {
                        self.promote_owned_synchronous_binding(root);
                    }
                    return Ok(());
                }
                let stored = match op {
                    AssignOp::Assign => self.emit_expr_as(value, &field_ty)?,
                    _ => {
                        let binop = assign_op_binop(op).ok_or_else(|| {
                            format!(
                                "error: compound assignment `{op:?}` is not supported for field `{field}`"
                            )
                        })?;
                        self.emit_binop(&binop, &target, value)?
                    }
                };
                self.emit_assigned_field_validator_checks(object, field, stored)?;
                self.builder.build_store(field_ptr, stored).unwrap();
                Ok(())
            }
            _ => Err("internal: not an assignment statement".to_string()),
        }
    }
}
