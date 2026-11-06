use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_var_decl_stmt(
        &mut self,
        stmt: &Stmt,
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        match stmt.kind() {
            StmtKind::VarDecl {
                mutable,
                name,
                ty,
                value,
                onfail,
            } => {
                if let (Some(value), Some(onfail)) = (value, onfail) {
                    if matches!(
                        self.expr_type_hint(value).map(|ty| self.resolve_type(&ty)),
                        Some(Type::Result { .. })
                    ) {
                        return self
                            .emit_result_onfail(*mutable, name, ty, value, onfail, fn_val, false);
                    }
                    let exception = match value.kind() {
                        ExprKind::FnCall { callee, .. } => self
                            .fn_throws
                            .get(callee)
                            .filter(|effects| effects.len() == 1)
                            .map(|effects| effects[0].clone()),
                        _ => None,
                    }
                    .ok_or("internal: unsupported `onfail` source reached code generation")?;
                    return self.emit_throwing_onfail(name, ty, value, onfail, exception, fn_val);
                }
                let index_hint = self.for_index_hint.take();
                self.stack_array_pending = self.contained_arrays.contains(stmt.span);
                self.confined_closure_pending = self.confined_closures.contains(stmt.span);
                self.array_literal_on_stack = false;
                let Some(value) = value else {
                    let resolved = self.resolve_type(ty);
                    let alloca = if let Some((slot, _)) = self.async_frame_slot(name) {
                        slot
                    } else if let Type::Union(variants) =
                        transparent_codegen_type(&resolved).clone()
                    {
                        let union_ty = self.union_llvm_type(&variants);
                        self.entry_alloca(union_ty, name)?
                    } else {
                        let llvm_ty = self.llvm_type(&resolved);
                        let alloca = self.entry_alloca(llvm_ty, name)?;
                        if matches!(transparent_codegen_type(&resolved), Type::Optional(_)) {
                            let null = self
                                .context
                                .ptr_type(inkwell::AddressSpace::default())
                                .const_null();
                            self.builder.build_store(alloca, null).unwrap();
                        }
                        match transparent_codegen_type(&resolved).clone() {
                            Type::Map(key, value) => {
                                let map = self.emit_new_map(&key, &value, None)?;
                                self.builder.build_store(alloca, map).unwrap();
                                self.mark_owned_synchronous_value(name);
                            }
                            Type::Set(element) => {
                                let set = self.emit_new_set(&element, None)?;
                                self.builder.build_store(alloca, set).unwrap();
                                self.mark_owned_synchronous_value(name);
                            }
                            _ => {}
                        }
                        alloca
                    };
                    self.store_var(name, alloca, resolved);
                    return Ok(());
                };
                let ty = match index_hint {
                    Some(hint) if matches!(ty, Type::Infer) => self.resolve_type(&hint),
                    _ => self.resolve_binding_type(ty, Some(value))?,
                };
                if let Type::Union(variants) = transparent_codegen_type(&ty).clone() {
                    let union_ty = self.union_llvm_type(&variants);
                    let alloca = self
                        .async_frame_slot(name)
                        .map(|(slot, _)| slot)
                        .map_or_else(|| self.entry_alloca(union_ty, name), Ok)?;
                    let (val, concrete) = if self.expr_type_hint(value).is_some_and(|hint| {
                        matches!(
                            transparent_codegen_type(&self.resolve_type(&hint)),
                            Type::Union(_)
                        )
                    }) && !matches!(value.kind(), ExprKind::Ternary { .. })
                    {
                        (self.emit_expr_as(value, &ty)?, None)
                    } else {
                        let (val, concrete) = self.build_union_value(&variants, value, name)?;
                        (val, Some(concrete))
                    };
                    let nested_owned = self.has_nested_owned_native_results(val, &ty);
                    if self.async_frame_owns_value(name) {
                        self.emit_async_clone_value_into(alloca, val, &ty)?;
                        self.emit_async_cleanup_source_expression(val, &ty, value)?;
                        self.mark_async_frame_value_initialized(name, true);
                    } else if nested_owned {
                        self.emit_async_clone_value_into(alloca, val, &ty)?;
                        self.emit_async_cleanup_source_expression(val, &ty, value)?;
                        self.mark_owned_synchronous_value(name);
                    } else {
                        self.builder.build_store(alloca, val).unwrap();
                        self.transfer_owned_native_result(name, val);
                        self.transfer_conditionally_owned_result(name, val);
                    }
                    if let Some(concrete) = concrete {
                        self.store_concrete_type(name, concrete);
                    }
                    self.store_var(name, alloca, ty);
                } else {
                    let llvm_ty = self.llvm_type(&ty);
                    let alloca = self
                        .async_frame_slot(name)
                        .map(|(slot, _)| slot)
                        .map_or_else(|| self.entry_alloca(llvm_ty, name), Ok)?;
                    self.clear_async_frame_local_before_store(fn_val, name, alloca, &ty)?;
                    let val = self.emit_expr_as(value, &ty)?;
                    self.take_field_moved_by_declaration(name, value, val, &ty)?;
                    if *mutable
                        && matches!(transparent_codegen_type(&ty), Type::Str)
                        && self.async_results.is_empty()
                        && !self.async_frame_owns_value(name)
                        && self.async_frame_string_ownership(name).is_none()
                    {
                        self.builder.build_store(alloca, val).unwrap();
                        let is_owned = self
                            .pending_owned_native_results
                            .iter()
                            .any(|(pending, _, _)| pending.as_value_ref() == val.as_value_ref());
                        if is_owned {
                            self.forget_owned_native_result(val);
                        }
                        // A value whose ownership is only known at runtime
                        // (`reduce` with a string accumulator, for one) is
                        // tracked in a separate pending list under its own
                        // `i1` rather than a plan, reusing that bit directly
                        // instead of collapsing it to the constant `is_owned`
                        // above is what makes this correct: the constant
                        // would either free a borrowed value or leak an owned
                        // one, depending on which runtime path actually ran.
                        let conditional_bit = self
                            .pending_conditionally_owned_results
                            .iter()
                            .position(|(pending, _, _)| {
                                pending.as_value_ref() == val.as_value_ref()
                            })
                            .map(|index| {
                                let (_, _, bit) =
                                    self.pending_conditionally_owned_results.remove(index);
                                bit
                            });
                        let owned_bit = conditional_bit.unwrap_or_else(|| {
                            self.context.bool_type().const_int(is_owned as u64, false)
                        });
                        let bit_slot = self
                            .builder
                            .build_alloca(self.context.bool_type(), &format!("{name}.owned"))
                            .unwrap();
                        self.builder.build_store(bit_slot, owned_bit).unwrap();
                        self.register_mutable_string_ownership(name, bit_slot);
                        self.store_var(name, alloca, ty);
                        self.discard_owned_native_result();
                        return Ok(());
                    }
                    // A synchronous `string[]` local carries its ownership mode
                    // at run time, for the same reason the mutable `string`
                    // above carries an ownership bit: `parts.push(x)` inside a
                    // branch or a loop is one point in the AST that only
                    // sometimes runs, so whether the elements are still the
                    // literal's static `.rodata` pointers or clones the array
                    // owns is a fact about the execution. Getting it wrong here
                    // is worse than a leak, `free` on `.rodata`, so the mode
                    // is stored and read back rather than assumed. See
                    // `string_array_ownership`.
                    //
                    // Only a plan the initializer actually recorded may be
                    // tracked. `Buffer` means the elements are not this
                    // binding's to free (a static literal, an empty literal, or
                    // one built by spreading someone else's array) and `Deep`
                    // means every element is its own allocation; a mixed
                    // literal's `ArrayElements` has no mode that describes it
                    // and keeps its existing static plan. The absence of a plan
                    // is the load-bearing case: `const alias: string[] = other`
                    // owns nothing, and tracking it would release another
                    // binding's buffer twice.
                    if matches!(
                        transparent_codegen_type(&ty),
                        Type::Array(element)
                            if matches!(transparent_codegen_type(element), Type::Str)
                    ) && self.async_results.is_empty()
                        && !self.async_frame_owns_value(name)
                        && self.async_frame_string_array_ownership(name).is_none()
                    {
                        let mode = self
                            .pending_owned_native_results
                            .iter()
                            .position(|(pending, _, _)| {
                                pending.as_value_ref() == val.as_value_ref()
                            })
                            .and_then(|index| match &self.pending_owned_native_results[index].2 {
                                ReleasePlan::Buffer => Some((index, 1u64)),
                                ReleasePlan::Deep => Some((index, 2u64)),
                                _ => None,
                            });
                        if let Some((index, mode)) = mode {
                            self.pending_owned_native_results.remove(index);
                            self.builder.build_store(alloca, val).unwrap();
                            let mode_slot = self
                                .entry_alloca(self.context.i8_type(), &format!("{name}.array.mode"))
                                .unwrap();
                            self.builder
                                .build_store(
                                    mode_slot,
                                    self.context.i8_type().const_int(mode, false),
                                )
                                .unwrap();
                            self.register_string_array_ownership(name, mode_slot);
                            self.store_var(name, alloca, ty);
                            self.discard_owned_native_result();
                            return Ok(());
                        }
                    }
                    // A `T?` coerced from a bare value, a literal, a numeric
                    // cast, anything whose own type is not already `Optional`,
                    // passed through `emit_optional_box`, which never
                    // registers its own release: some of its callers use the
                    // box *inside* a branch a caller later merges into a phi,
                    // and registering there could reach past that merge and
                    // read a value that does not dominate the reference, two
                    // attempts each broke a different IR snapshot test with
                    // "does not dominate all uses". Registering here instead is
                    // sound regardless of what branching happened *inside*
                    // `emit_expr_as`: `val` is what it returned, and a callee's
                    // return value always dominates its own call site.
                    //
                    // Gated to values whose own type is not already `Optional`,
                    // an `OptionalMethodCall` or `OptionalFieldAccess` result
                    // registers its own release already, and a plain `Ident`
                    // read of an existing optional binding is a borrow this
                    // scope must not also claim.
                    //
                    // Also gated to nothing *else* pending: boxing an
                    // expression that itself allocates and separately
                    // registers, `"a".toUpperCase()` coerced into `string?`
                    // registers the *string*, distinct from the box wrapping
                    // it, left both entries live, and the existing
                    // nested-owned handling below (built for a different
                    // shape, an aggregate holding an already-owned part) does
                    // not know how to reconcile a `Value`-plan box with a
                    // separately owned payload inside it: it double freed.
                    // Declining here leaves the box leaking exactly as it did
                    // before this fix, which is what it takes to keep this to
                    // a leak and not a second corruption in the same file.
                    let value_is_boxed_here =
                        matches!(transparent_codegen_type(&ty), Type::Optional(_))
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
                            ty.clone(),
                            crate::ReleasePlan::Value,
                        );
                    }
                    let nested_owned = self.has_nested_owned_native_results(val, &ty);
                    if self.async_frame_owns_value(name) {
                        let moved_from_awaited_result = matches!(value.kind(),
                            ExprKind::Try(inner) if matches!(inner.kind(),
                                ExprKind::Ident(source) if source.starts_with("await#"))
                        );
                        let moved_from_awaited_handle = matches!(value.kind(), ExprKind::Await(_))
                            && crate::core::async_move_only_native_handle(&ty);
                        let moved_owned_optional =
                            matches!(transparent_codegen_type(&ty), Type::Optional(_))
                                && self.take_owned_native_result(val).is_some();
                        if moved_from_awaited_result
                            || moved_from_awaited_handle
                            || moved_owned_optional
                        {
                            // `?` disarms the hidden await-frame Result before
                            // returning its success payload. Store that payload
                            // directly: cloning it and cleaning the source
                            // would free move-only handles such as TcpListener.
                            self.builder.build_store(alloca, val).unwrap();
                        } else {
                            self.emit_async_clone_value_into(alloca, val, &ty)?;
                            self.emit_async_cleanup_source_expression(val, &ty, value)?;
                        }
                        self.mark_async_frame_value_initialized(name, true);
                        // A bare identifier initializer of a move-only handle
                        // is a move, and the borrow checker has already refused
                        // every later use of the source: naming `app` after
                        // `const alias: HttpRouter = app;` does not compile.
                        // A counted handle gets a reference of its own on the
                        // way into the slot, which is what lets the source keep
                        // a release of its own; this one gets none, so the two
                        // slots name a single allocation. Leaving the source's
                        // bit set had the frame drop release it through both
                        // names, reported as a use-after-free the moment
                        // `HttpRouter` gained drop glue at all.
                        if let ExprKind::Ident(source) = value.kind()
                            && crate::core::async_move_only_native_handle(&ty)
                        {
                            self.mark_async_frame_value_initialized(source, false);
                        }
                    } else if nested_owned {
                        self.emit_async_clone_value_into(alloca, val, &ty)?;
                        self.emit_async_cleanup_source_expression(val, &ty, value)?;
                        self.mark_owned_synchronous_value(name);
                    } else {
                        self.builder.build_store(alloca, val).unwrap();
                        // A bare identifier initializer on this path is a move:
                        // the header is copied and the buffer is not, so exactly
                        // one of the two names may release it. The release used
                        // to stay with the source, which is only harmless while
                        // both names keep pointing at the same block. A `push`
                        // that reallocates breaks that: the source's pointer is
                        // freed by `realloc`, the destination gets a new one,
                        // and scope exit frees the source's stale pointer while
                        // the destination's buffer leaks. ASan reports it as a
                        // double free, and it reproduced identically for a
                        // `const` local and for a `const` global.
                        //
                        // Restricted to this branch on purpose: the two above
                        // clone into the destination rather than move, so there
                        // the source still owns its own copy and must keep its
                        // release.
                        if let ExprKind::Ident(source) = value.kind() {
                            self.transfer_owned_binding(source, name);
                        }
                    }
                    if !nested_owned {
                        self.transfer_owned_native_result(name, val);
                        self.transfer_conditionally_owned_result(name, val);
                    }
                    if matches!(
                        value.kind(),
                        ExprKind::ArrayLit(_) | ExprKind::ArrayRepeat { .. }
                    ) && matches!(transparent_codegen_type(&ty), Type::Array(element) if element.is_copy())
                        && !self.array_literal_on_stack
                    {
                        self.mark_owned_synchronous_value(name);
                    }
                    if let Some(ownership) = self.async_frame_string_ownership(name) {
                        let owned = self.async_owned_string_expression(value);
                        self.builder
                            .build_store(
                                ownership,
                                self.context.bool_type().const_int(owned as u64, false),
                            )
                            .unwrap();
                    }
                    if let Some(ownership) = self.async_frame_string_array_ownership(name) {
                        let mode = self.async_string_array_ownership_mode(value).ok_or_else(|| {
                            format!(
                                "error: mutable async string-array local `{name}` requires an initializer with known element ownership"
                            )
                        })?;
                        self.builder
                            .build_store(ownership, self.context.i8_type().const_int(mode, false))
                            .unwrap();
                    }
                    self.store_var(name, alloca, ty);
                    if let ExprKind::ArrowFn { throws, .. } = value.kind()
                        && !throws.is_empty()
                    {
                        self.throwing_closures.insert(name.clone());
                    }
                    if matches!(value.kind(), ExprKind::ArrowFn { .. }) {
                        if !self.closure_env_on_stack {
                            self.mark_owned_closure(name);
                        }
                    } else if self
                        .expr_type_hint(value)
                        .map(|hint| self.resolve_type(&hint))
                        .is_some_and(|hint| {
                            matches!(transparent_codegen_type(&hint), Type::Function { .. })
                        })
                    {
                        if !matches!(
                            value.kind(),
                            ExprKind::FnCall { .. }
                                | ExprKind::MethodCall { .. }
                                | ExprKind::CallValue { .. }
                        ) {
                            self.emit_closure_retain(val);
                        }
                        self.mark_owned_dynamic_closure(name);
                    }
                }
                self.discard_owned_native_result();
                Ok(())
            }
            _ => Err("internal: not a variable declaration".to_string()),
        }
    }
}
