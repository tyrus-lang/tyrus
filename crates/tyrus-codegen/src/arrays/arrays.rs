use super::*;
use tyrus_core::ast::ExprKind;

#[path = "array_methods/array_methods.rs"]
mod array_methods;
#[path = "array_ops/array_ops.rs"]
mod array_ops;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn array_element_kind(&self, element: &Type) -> u64 {
        match transparent_codegen_type(&self.resolve_type(element)) {
            Type::Str => 1,
            Type::F32 => 2,
            Type::F64 => 3,
            _ => 0,
        }
    }

    pub(super) fn emit_object_literal(
        &mut self,
        name: &str,
        values: &[(String, Expr)],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fields = self
            .declared_fields(&Type::Named(name.to_string()))
            .ok_or_else(|| format!("error: unknown interface `{name}`"))?;
        self.emit_object_literal_with_fields(&Type::Named(name.to_string()), &fields, values)
    }

    pub(super) fn emit_object_literal_with_fields(
        &mut self,
        ty: &Type,
        fields: &[Field],
        values: &[(String, Expr)],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let owns_collection = self.validation_depth == 0 && self.type_has_validators(ty);
        if owns_collection {
            self.emit_validation_issues_begin();
        }
        self.validation_depth += 1;
        let result = self.emit_object_literal_fields(ty, fields, values);
        self.validation_depth -= 1;
        let result = result?;
        if owns_collection {
            self.emit_validation_issues_finish();
        }
        Ok(result)
    }

    fn emit_object_literal_fields(
        &mut self,
        ty: &Type,
        fields: &[Field],
        values: &[(String, Expr)],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let struct_type = self.llvm_type(ty).into_struct_type();
        let mut result = struct_type.get_undef();
        let mut field_plans: Vec<(usize, crate::ReleasePlan)> = Vec::new();
        for (declared_index, field) in fields.iter().enumerate() {
            let index = self
                .struct_field_index(ty, &field.name)
                .unwrap_or(declared_index as u32) as usize;
            let storage_type = field_storage_type(field);
            let mut boxed_here = false;
            let mut cloned_here = false;
            let value = if let Some((_, expression)) = values
                .iter()
                .find(|(candidate, _)| candidate == &field.name)
            {
                self.validation_target_path.push(field.name.clone());
                let emitted = self.emit_expr_as(expression, &storage_type);
                self.validation_target_path.pop();
                let mut emitted = emitted?;
                boxed_here = matches!(transparent_codegen_type(&storage_type), Type::Optional(_))
                    && !matches!(expression.kind(), ExprKind::Null | ExprKind::None)
                    && !self
                        .expr_type_hint(expression)
                        .map(|hint| self.resolve_type(&hint))
                        .is_some_and(|hint| {
                            matches!(transparent_codegen_type(&hint), Type::Optional(_))
                        })
                    && self.pending_owned_native_results.is_empty();
                // A field reading a place (`{ options: options }`) copies the
                // value, and the copy is shallow: it carries the very pointers
                // the binding allocated and still releases at its own scope
                // exit. Whoever later frees this object walks its type and
                // frees them a second time, a double free rather than a leak,
                // and `const a: T = { ... }` with an optional field stored into
                // another literal is enough to reach it.
                //
                // The language settles which of copy and move this is: the
                // borrow checker refuses `const b: T = a;` as a move and asks
                // for `&a`, yet accepts `a` inside a literal and leaves the
                // binding usable afterwards. So it is a copy, and a copy of an
                // owned value has to be a deep one. This is the clone `return`
                // already takes when it hands an owned value to a caller, for
                // exactly the same reason.
                if self.object_field_copies_a_borrowed_place(expression, &storage_type)
                    && let Some(cloned) =
                        self.emit_owned_object_field_clone(emitted, &storage_type)?
                {
                    if boxed_here {
                        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                        let free = self.declare_libc_fn("free", &[pointer.into()], None);
                        self.builder
                            .build_call(
                                free,
                                &[emitted.into_pointer_value().into()],
                                "object.field.shallow.box",
                            )
                            .map_err(|error| error.to_string())?;
                    }
                    cloned_here = true;
                    emitted = cloned;
                }
                emitted
            } else {
                self.emit_zero(&storage_type)?
            };
            if boxed_here || cloned_here {
                self.record_owned_native_result_with(
                    value,
                    storage_type.clone(),
                    if cloned_here {
                        crate::ReleasePlan::Deep
                    } else {
                        crate::ReleasePlan::Value
                    },
                );
            }
            if !field.decorators.is_empty() {
                self.emit_validator_checks(value, &storage_type, &field.decorators, &field.name)?;
            }
            if let Some(plan) = self.take_owned_native_result(value) {
                let plan = if matches!(
                    transparent_codegen_type(&self.resolve_type(&storage_type)),
                    Type::Str
                ) {
                    crate::ReleasePlan::Value
                } else {
                    plan
                };
                field_plans.push((index, plan));
            }
            result = self
                .builder
                .build_insert_value(result, value, index as u32, &field.name)
                .map_err(|error| error.to_string())?
                .into_struct_value();
        }
        if !field_plans.is_empty() {
            self.record_owned_native_result_with(
                result.into(),
                ty.clone(),
                crate::ReleasePlan::Fields(field_plans),
            );
        }
        Ok(result.into())
    }

    /// Whether this field takes its value by reading a place that a binding
    /// owns, which makes the store a shallow copy of allocations that stay the
    /// binding's.
    ///
    /// Only a place expression aliases storage this way. A literal, a call or
    /// any other expression produces a fresh value the field can simply keep,
    /// and cloning one would allocate a second copy and leak the first.
    fn object_field_copies_a_borrowed_place(&mut self, expression: &Expr, storage: &Type) -> bool {
        if !matches!(
            expression.kind(),
            ExprKind::Ident(_) | ExprKind::FieldAccess { .. } | ExprKind::IndexAccess { .. }
        ) {
            return false;
        }
        let resolved = self.resolve_type(storage);
        if resolved.is_copy() || matches!(transparent_codegen_type(&resolved), Type::Str) {
            return false;
        }
        // Synchronous code only. An async function accounts for the same
        // object by a different mechanism that assumes the field copy is
        // shallow: the cleanup after an async `return` releases an optional
        // field by freeing its box alone, deliberately, because the box held
        // a value borrowed from a parameter or local that the frame still
        // owns. A clone here puts a counted reference in that box, and the
        // box free drops it on the floor. Measured: without this guard
        // `tests/leaks/clean/async-owned-payload-transfer.tyr` leaked the
        // shared string of its `note: label` field, 30 bytes per iteration.
        //
        // Nothing is left unfixed by stopping here. The corrupting shape put
        // inside an `async` function reports zero bytes and no invalid free,
        // because the frame's drop glue, not the plan recorded here, is what
        // releases an async local. An optional chain is excluded for the
        // plainer reason that `record_owned_native_result_with` discards a
        // registration made inside one, so the clone would own nothing.
        if !self.async_results.is_empty() || self.optional_chain_depth > 0 {
            return false;
        }
        self.async_owned_value_supported(&resolved)
    }

    fn emit_owned_object_field_clone(
        &mut self,
        value: BasicValueEnum<'ctx>,
        storage: &Type,
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        if !self.async_owned_value_supported(storage) {
            return Ok(None);
        }
        let storage_type = self.llvm_type(storage);
        let slot = self
            .builder
            .build_alloca(storage_type, "object.field.owned")
            .map_err(|error| error.to_string())?;
        self.emit_async_clone_value_into(slot, value, storage)?;
        let cloned = self
            .builder
            .build_load(storage_type, slot, "object.field.owned.value")
            .map_err(|error| error.to_string())?;
        Ok(Some(cloned))
    }

    pub(super) fn type_has_validators(&self, ty: &Type) -> bool {
        let mut seen = HashSet::default();
        self.type_has_validators_inner(ty, &mut seen)
    }

    fn type_has_validators_inner(&self, ty: &Type, seen: &mut HashSet<String>) -> bool {
        match transparent_codegen_type(&self.resolve_type(ty)) {
            Type::Named(name) => {
                if !seen.insert(name.clone()) {
                    return false;
                }
                self.interfaces.get(name.as_str()).is_some_and(|fields| {
                    fields.iter().any(|field| {
                        !field.decorators.is_empty()
                            || self.type_has_validators_inner(&field_storage_type(field), seen)
                    })
                })
            }
            Type::Object(fields) => fields.iter().any(|field| {
                !field.decorators.is_empty()
                    || self.type_has_validators_inner(&field_storage_type(field), seen)
            }),
            Type::Array(element) => self.type_has_validators_inner(element, seen),
            Type::Optional(inner) => self.type_has_validators_inner(inner, seen),
            _ => false,
        }
    }

    pub(super) fn new_set_element_type(&self, initial: Option<&Expr>) -> Option<Type> {
        match initial {
            Some(Expr {
                kind: ExprKind::ArrayLit(values),
                ..
            }) => self.infer_array_literal_type(values),
            _ => None,
        }
    }

    pub(super) fn new_map_entry_types(&self, initial: Option<&Expr>) -> Option<(Type, Type)> {
        let Some(Expr {
            kind: ExprKind::ArrayLit(entries),
            ..
        }) = initial
        else {
            return None;
        };
        let mut keys = Vec::with_capacity(entries.len());
        let mut values = Vec::with_capacity(entries.len());
        for entry in entries {
            let ExprKind::ArrayLit(pair) = entry.kind() else {
                return None;
            };
            if pair.len() != 2 {
                return None;
            }
            keys.push(pair[0].clone());
            values.push(pair[1].clone());
        }
        Some((
            self.infer_array_literal_type(&keys)?,
            self.infer_array_literal_type(&values)?,
        ))
    }

    pub(super) fn infer_array_literal_type(&self, values: &[Expr]) -> Option<Type> {
        let first = values.first()?;
        let inferred = match first.kind() {
            ExprKind::IntLit(_) => Type::I32,
            ExprKind::FloatLit(_) => Type::F64,
            _ => self.expr_type_hint(first)?,
        };
        if matches!(first.kind(), ExprKind::UIntLit(_)) {
            return None;
        }
        values
            .iter()
            .all(|value| {
                matches!(value.kind(), ExprKind::IntLit(_)) && matches!(inferred, Type::I32)
                    || matches!(value.kind(), ExprKind::FloatLit(_))
                        && matches!(inferred, Type::F64)
                    || self.expr_type_hint(value).as_ref() == Some(&inferred)
            })
            .then_some(inferred)
    }

    pub(super) fn emit_array_buffer(
        &mut self,
        element_size: IntValue<'ctx>,
        length: IntValue<'ctx>,
    ) -> Result<PointerValue<'ctx>, String> {
        let usize_type = self.usize_type();
        let pointer_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let element_size = self.truncate_to_usize(element_size);
        let allocation_size = self
            .builder
            .build_int_mul(element_size, length, "array_allocation_size")
            .map_err(|error| error.to_string())?;
        let (malloc, allocation_size) = self.declare_malloc(allocation_size);

        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: an array literal is emitted inside a function")?;
        let allocate_block = self.context.append_basic_block(function, "array.allocate");
        let ready_block = self.context.append_basic_block(function, "array.ready");

        let wants_bytes = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::NE,
                allocation_size,
                usize_type.const_zero(),
                "array_wants_bytes",
            )
            .map_err(|error| error.to_string())?;
        let empty_block = self
            .builder
            .get_insert_block()
            .ok_or("internal: the builder is positioned")?;
        self.builder
            .build_conditional_branch(wants_bytes, allocate_block, ready_block)
            .map_err(|error| error.to_string())?;

        // A branch rather than a select, because the point is not to *ignore*
        // the pointer for an empty array, it is not to ask for one. Selecting
        // over the result still calls `malloc(0)`, which allocates something
        // (one byte, under AddressSanitizer) that nothing then frees.
        self.builder.position_at_end(allocate_block);
        let allocated = self
            .builder
            .build_call(malloc, &[allocation_size.into()], "array_data")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: `malloc` returns a pointer")?
            .into_pointer_value();
        self.builder
            .build_unconditional_branch(ready_block)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(ready_block);
        let data_phi = self
            .builder
            .build_phi(pointer_type, "array_data")
            .map_err(|error| error.to_string())?;
        data_phi.add_incoming(&[
            (&pointer_type.const_null(), empty_block),
            (&allocated, allocate_block),
        ]);
        Ok(data_phi.as_basic_value().into_pointer_value())
    }

    pub(super) fn emit_array_literal(
        &mut self,
        values: &[Expr],
        element: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let contained = std::mem::take(&mut self.stack_array_pending);
        let element_type = self.llvm_type(element);
        let usize_type = self.usize_type();
        let element_size = element_type
            .size_of()
            .ok_or_else(|| format!("error: array element type `{element}` has no native size"))?;

        enum Segment<'v> {
            Value(BasicValueEnum<'v>),
            Spread {
                data: PointerValue<'v>,
                length: IntValue<'v>,
            },
        }
        let mut segments = Vec::with_capacity(values.len());
        let mut fixed_count: u64 = 0;
        for (index, value) in values.iter().enumerate() {
            if let ExprKind::Spread(inner) = value.kind() {
                let source = self.emit_expr(inner)?.into_struct_value();
                let data = self
                    .builder
                    .build_extract_value(source, 0, "spread_data")
                    .unwrap()
                    .into_pointer_value();
                let length = self
                    .builder
                    .build_extract_value(source, 1, "spread_length")
                    .unwrap()
                    .into_int_value();
                segments.push(Segment::Spread { data, length });
            } else {
                self.validation_target_path.push(format!("[{index}]"));
                let emitted = self.emit_expr_as(value, element);
                self.validation_target_path.pop();
                segments.push(Segment::Value(emitted?));
                fixed_count += 1;
            }
        }

        let mut length = usize_type.const_int(fixed_count, false);
        for segment in &segments {
            if let Segment::Spread {
                length: spread_length,
                ..
            } = segment
            {
                length = self
                    .builder
                    .build_int_add(length, *spread_length, "array_length")
                    .unwrap();
            }
        }

        let all_fixed = fixed_count as usize == values.len();
        let on_stack = contained && all_fixed && element.is_copy();

        let data = if on_stack {
            self.entry_alloca(
                element_type.array_type(fixed_count as u32),
                "array_data_stack",
            )?
        } else {
            self.emit_array_buffer(element_size, length)?
        };

        let has_spread = segments
            .iter()
            .any(|segment| matches!(segment, Segment::Spread { .. }));
        let mut owned_elements = 0usize;
        let mut value_elements = 0usize;
        let mut owned_indices = Vec::new();
        for (index, segment) in segments.iter_mut().enumerate() {
            if let Segment::Value(value) = segment {
                value_elements += 1;
                if self.forget_owned_native_result(*value) {
                    owned_elements += 1;
                    owned_indices.push(index);
                    continue;
                }
                if matches!(transparent_codegen_type(element), Type::Str)
                    && matches!(values[index].kind(), ExprKind::Ident(_))
                {
                    let shared =
                        self.emit_string_clone(value.into_struct_value(), "array.element.share")?;
                    *value = shared;
                    owned_elements += 1;
                    owned_indices.push(index);
                }
            }
        }
        let claims_elements = value_elements > 0 && owned_elements == value_elements;
        let claims_mixed_elements =
            !has_spread && owned_elements > 0 && owned_elements < value_elements;
        // A `Copy` element is not an allocation, but the buffer holding the
        // elements is one. Only a literal bound to a name was being released,
        // so a literal used as a temporary, `[1, 2, 3].map(...)`, leaked its
        // buffer.
        //
        // A literal of *static strings* also allocates a buffer nobody frees,
        // no element is an allocation, but the buffer holding them is one, same
        // as the `Copy` case above. Two earlier attempts at claiming it each
        // aborted every example in the integration suite with a double free, both
        // blamed on the buffer escaping into a native callee that frees it
        // (`Promise.all([...])` by name). Re-examined later: `Promise.all`'s
        // own runtime never frees the caller's buffer at all, it borrows it
        // and makes its own copy, and reconstructing both original crash
        // shapes measures zero leaks and zero invalid frees today. See
        // `tests/leaks/clean/array-of-static-strings.tyr`.
        //
        // An *empty* literal allocates a buffer too, and claiming it is what
        // distinguishes `let parts: string[] = []`, which owns a fresh, if
        // empty, allocation, from `const alias: string[] = other`, which owns
        // nothing and must never release anything. Both reach a binding with no
        // elements to inspect, and only the literal records a plan here, so the
        // binding can tell them apart by whether a plan arrived at all. Without
        // this the empty literal was claimed by nobody and leaked its whole
        // buffer, which grew with everything later pushed into it.
        let claims_buffer = element.is_copy()
            || (value_elements > 0 && owned_elements == 0)
            || (value_elements == 0 && !has_spread);

        let mut offset = usize_type.const_zero();
        for segment in segments {
            let slot = unsafe {
                self.builder
                    .build_gep(element_type, data, &[offset], "array_element")
                    .unwrap()
            };
            match segment {
                Segment::Value(value) => {
                    self.builder.build_store(slot, value).unwrap();
                    offset = self
                        .builder
                        .build_int_add(offset, usize_type.const_int(1, false), "array_offset")
                        .unwrap();
                }
                Segment::Spread {
                    data: source,
                    length: spread_length,
                } => {
                    let copy_size = self
                        .builder
                        .build_int_mul(element_size, spread_length, "spread_copy_size")
                        .unwrap();
                    self.builder
                        .build_memcpy(slot, 1, source, 1, copy_size)
                        .map_err(|error| error.to_string())?;
                    offset = self
                        .builder
                        .build_int_add(offset, spread_length, "array_offset")
                        .unwrap();
                }
            }
        }

        let array_type = self.llvm_type(&Type::Array(Box::new(element.clone())));
        let mut array = array_type.into_struct_type().get_undef();
        array = self
            .builder
            .build_insert_value(array, data, 0, "array_data_field")
            .unwrap()
            .into_struct_value();
        array = self
            .builder
            .build_insert_value(array, length, 1, "array_length_field")
            .unwrap()
            .into_struct_value();
        array = self
            .builder
            .build_insert_value(array, length, 2, "array_capacity_field")
            .unwrap()
            .into_struct_value();
        if !on_stack && (claims_elements || claims_buffer || claims_mixed_elements) {
            // A `string[]` is released deeply whatever its elements looked like
            // here, because a string says at run time whether it owns anything:
            // `TyrusString::release` returns immediately for a value over
            // `.rodata`, so a deep release of a literal of static strings frees
            // the buffer and touches nothing else. The static distinction this
            // plan exists to make (which elements are allocations) stopped
            // being answerable here once `push` could add one *later*, and
            // stopped being necessary at the same time, for the same reason.
            //
            // A plain local does not depend on this: mutating one promotes its
            // own runtime ownership mode (`string_array_ownership`), which
            // supersedes the plan. An array held by an object field has no such
            // slot, so the plan recorded here is the only answer it ever gets,
            // and `Buffer` left every element it was given by a later `push` to
            // leak.
            let element_is_string = matches!(transparent_codegen_type(element), Type::Str);
            let plan = if claims_elements || element_is_string {
                crate::ReleasePlan::Deep
            } else if claims_mixed_elements {
                crate::ReleasePlan::ArrayElements(owned_indices)
            } else {
                crate::ReleasePlan::Buffer
            };
            self.record_owned_native_result_with(
                array.into(),
                Type::Array(Box::new(element.clone())),
                plan,
            );
        }
        self.array_literal_on_stack = on_stack;
        Ok(array.into())
    }

    pub(super) fn emit_array_repeat_literal(
        &mut self,
        value: &Expr,
        count: &Expr,
        element: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let count_val = tyrus_core::type_checker::integer_constant_value(count)
            .and_then(Result::ok)
            .and_then(|value| usize::try_from(value).ok())
            .filter(|count| (1..=tyrus_core::type_checker::MAX_ARRAY_REPEAT_COUNT).contains(count))
            .ok_or_else(|| {
                format!(
                    "error: array repeat count must be a constant integer between 1 and {}",
                    tyrus_core::type_checker::MAX_ARRAY_REPEAT_COUNT
                )
            })?;
        let count_val = u64::try_from(count_val)
            .map_err(|_| "error: array repeat count does not fit the target size".to_string())?;

        let contained = std::mem::take(&mut self.stack_array_pending);

        let element_type = self.llvm_type(element);
        let emitted_value = self.emit_expr_as(value, element)?;

        let usize_type = self.usize_type();
        let element_size = element_type
            .size_of()
            .ok_or_else(|| format!("error: array element type `{element}` has no native size"))?;

        let length = usize_type.const_int(count_val, false);
        let on_stack = contained && element.is_copy();
        let data = if on_stack {
            self.entry_alloca(
                element_type.array_type(u32::try_from(count_val).map_err(|_| {
                    "error: array repeat count does not fit LLVM array storage".to_string()
                })?),
                "array_repeat_data_stack",
            )?
        } else {
            self.emit_array_buffer(element_size, length)?
        };

        let mut offset = usize_type.const_zero();
        for i in 0..count_val {
            let slot = unsafe {
                self.builder
                    .build_gep(element_type, data, &[offset], "array_element")
                    .unwrap()
            };
            self.builder.build_store(slot, emitted_value).unwrap();
            if i < count_val - 1 {
                offset = self
                    .builder
                    .build_int_add(offset, usize_type.const_int(1, false), "array_offset")
                    .unwrap();
            }
        }

        let array_type = self.llvm_type(&Type::Array(Box::new(element.clone())));
        let mut array = array_type.into_struct_type().get_undef();
        array = self
            .builder
            .build_insert_value(array, data, 0, "array_data_field")
            .unwrap()
            .into_struct_value();
        array = self
            .builder
            .build_insert_value(array, length, 1, "array_length_field")
            .unwrap()
            .into_struct_value();
        array = self
            .builder
            .build_insert_value(array, length, 2, "array_capacity_field")
            .unwrap()
            .into_struct_value();

        if !element.is_copy() && !matches!(transparent_codegen_type(element), Type::Str) {
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let clone_element = self.emit_array_element_clone(element)?;
            let array_struct_type = array_type.into_struct_type();
            let array_alloca = self
                .builder
                .build_alloca(array_struct_type, "array.repeat.own")
                .unwrap();
            self.builder.build_store(array_alloca, array).unwrap();
            let own_elements = self.declare_libc_fn(
                "tyrus_array_own_elements",
                &[
                    pointer.into(),
                    self.context.i64_type().into(),
                    pointer.into(),
                ],
                Some(self.context.i32_type().into()),
            );
            let status = self
                .builder
                .build_call(
                    own_elements,
                    &[
                        array_alloca.into(),
                        element_size.into(),
                        clone_element.as_global_value().as_pointer_value().into(),
                    ],
                    "array.repeat.own_elements",
                )
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap()
                .into_int_value();
            self.emit_abort_if_negative(status, "array repeat", "array.repeat.own")?;
            array = self
                .builder
                .build_load(array_struct_type, array_alloca, "array.repeat.owned")
                .unwrap()
                .into_struct_value();
        }

        if on_stack {
            self.array_literal_on_stack = true;
        } else if !element.is_copy() {
            self.record_owned_native_result_with(
                array.into(),
                Type::Array(Box::new(element.clone())),
                crate::ReleasePlan::Deep,
            );
        } else {
            self.record_owned_native_result_with(
                array.into(),
                Type::Array(Box::new(element.clone())),
                crate::ReleasePlan::Buffer,
            );
        }

        Ok(array.into())
    }

    pub(super) fn emit_tuple_literal(
        &mut self,
        values: &[Expr],
        elements: &[Type],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if values.len() != elements.len() {
            return Err(format!(
                "error: tuple literal has {} elements but tuple type has {}",
                values.len(),
                elements.len()
            ));
        }
        let tuple_type = Type::Tuple(elements.to_vec());
        let mut tuple = self.llvm_type(&tuple_type).into_struct_type().get_undef();
        for (index, (value, element)) in values.iter().zip(elements).enumerate() {
            let value = self.emit_expr_as(value, element)?;
            tuple = self
                .builder
                .build_insert_value(tuple, value, index as u32, "tuple_element")
                .map_err(|error| error.to_string())?
                .into_struct_value();
        }
        Ok(tuple.into())
    }

    pub(super) fn materialize_tuple_as_array(
        &mut self,
        expr: &Expr,
        element: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let slice = self
            .tuple_slice(expr, "tuple_to_array")?
            .ok_or_else(|| "internal: expected a tuple receiver to materialize".to_string())?;
        let length = slice.length;
        let view = self.borrowed_array_view(slice, "tuple_to_array_view")?;
        let array_type = Type::Array(Box::new(element.clone()));
        let llvm_array_type = self.llvm_type(&array_type);
        let element_size = self
            .llvm_type(element)
            .size_of()
            .ok_or_else(|| format!("error: array element type `{element}` has no native size"))?;
        let output = self
            .entry_alloca(llvm_array_type, "tuple_to_array_output")
            .unwrap();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i64_type = self.context.i64_type();
        let i32_type = self.context.i32_type();
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
        self.builder
            .build_call(
                function,
                &[
                    view.slot.into(),
                    i64_type.const_zero().into(),
                    length.into(),
                    element_size.into(),
                    output.into(),
                ],
                "tuple_to_array_copy",
            )
            .map_err(|error| error.to_string())?;
        let result = self
            .builder
            .build_load(llvm_array_type, output, "tuple_to_array_result")
            .map_err(|error| error.to_string())?;
        self.record_owned_native_result(result, array_type);
        Ok(result)
    }

    pub(super) fn emit_array_index(
        &mut self,
        object: &Expr,
        index: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let array_type = self
            .expr_type_hint(object)
            .ok_or_else(|| "error: cannot infer indexed array type".to_string())?;
        let Type::Array(element) = transparent_codegen_type(&array_type) else {
            return Err(format!("error: cannot index into type `{array_type}`"));
        };
        let element = element.as_ref().clone();
        let array_value = self.emit_expr(object)?;
        let array = array_value.into_struct_value();
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
        let slot = unsafe {
            self.builder
                .build_gep(self.llvm_type(&element), data, &[index], "array_index")
                .unwrap()
        };
        let element_value = self
            .builder
            .build_load(self.llvm_type(&element), slot, "array_value")
            .map_err(|error| error.to_string())?;

        if let Some((array_ty, plan)) = self.take_owned_native_result_with_type(array_value) {
            let kept_value = if element.is_copy() {
                element_value
            } else if matches!(transparent_codegen_type(&element), Type::Str) {
                let duplicated: BasicValueEnum<'ctx> =
                    self.emit_string_clone(element_value.into_struct_value(), "array_index.owned")?;
                self.record_owned_native_result(duplicated, element.clone());
                duplicated
            } else {
                self.record_owned_native_result_with(array_value, array_ty, plan);
                return Ok(element_value);
            };
            self.emit_owned_native_release(array_value, &array_ty, &plan);
            return Ok(kept_value);
        }
        Ok(element_value)
    }

    pub(super) fn emit_array_bounds_check(
        &self,
        index: IntValue<'ctx>,
        length: IntValue<'ctx>,
    ) -> Result<(), String> {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: array access outside function")?;
        let error = self
            .context
            .append_basic_block(function, "array.bounds.error");
        let valid = self
            .context
            .append_basic_block(function, "array.bounds.valid");
        let too_large = self
            .builder
            .build_int_compare(IntPredicate::UGE, index, length, "index_too_large")
            .unwrap();
        self.builder
            .build_conditional_branch(too_large, error, valid)
            .unwrap();

        self.builder.position_at_end(error);
        let message = self
            .builder
            .build_global_string_ptr(
                "error: array index out of bounds: tried to access index %lld, but the array has %lld element(s)\n",
                "array_bounds_message",
            )
            .unwrap();
        let dprintf = self.declare_dprintf();
        self.builder
            .build_call(
                dprintf,
                &[
                    self.context.i32_type().const_int(2, false).into(),
                    message.as_pointer_value().into(),
                    index.into(),
                    length.into(),
                ],
                "",
            )
            .unwrap();
        self.emit_exit_failure();
        self.builder.position_at_end(valid);
        Ok(())
    }

    fn emit_split_at_bounds_check(
        &self,
        index: IntValue<'ctx>,
        length: IntValue<'ctx>,
    ) -> Result<(), String> {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: array access outside function")?;
        let error = self
            .context
            .append_basic_block(function, "array.splitat.error");
        let valid = self
            .context
            .append_basic_block(function, "array.splitat.valid");
        let too_large = self
            .builder
            .build_int_compare(IntPredicate::UGT, index, length, "splitat_too_large")
            .unwrap();
        self.builder
            .build_conditional_branch(too_large, error, valid)
            .unwrap();

        self.builder.position_at_end(error);
        let message = self
            .builder
            .build_global_string_ptr(
                "error: splitAt index out of bounds: tried to split at index %lld, but the array has %lld element(s)\n",
                "array_splitat_message",
            )
            .unwrap();
        let dprintf = self.declare_dprintf();
        self.builder
            .build_call(
                dprintf,
                &[
                    self.context.i32_type().const_int(2, false).into(),
                    message.as_pointer_value().into(),
                    index.into(),
                    length.into(),
                ],
                "",
            )
            .unwrap();
        self.emit_exit_failure();
        self.builder.position_at_end(valid);
        Ok(())
    }

    fn emit_array_extreme(
        &mut self,
        array_slot: PointerValue<'ctx>,
        llvm_array_type: BasicTypeEnum<'ctx>,
        element: &Type,
        element_size: IntValue<'ctx>,
        last: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let _ = llvm_array_type;
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i64_type = self.context.i64_type();
        let array = self
            .builder
            .build_load(
                self.llvm_type(&Type::Array(Box::new(element.clone()))),
                array_slot,
                "array_extreme",
            )
            .map_err(|error| error.to_string())?
            .into_struct_value();
        let data = self
            .builder
            .build_extract_value(array, 0, "array_extreme_data")
            .map_err(|error| error.to_string())?
            .into_pointer_value();
        let length = self
            .builder
            .build_extract_value(array, 1, "array_extreme_length")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let result = self
            .builder
            .build_alloca(pointer, "array_extreme_result")
            .unwrap();
        self.builder
            .build_store(result, pointer.const_null())
            .unwrap();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: array extreme outside function")?;
        let present = self
            .context
            .append_basic_block(function, "array_extreme.present");
        let done = self
            .context
            .append_basic_block(function, "array_extreme.done");
        let empty = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                length,
                i64_type.const_zero(),
                "array_extreme_empty",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(empty, done, present)
            .unwrap();
        self.builder.position_at_end(present);
        let index = if last {
            self.builder
                .build_int_sub(length, i64_type.const_int(1, false), "array_last_index")
                .unwrap()
        } else {
            i64_type.const_zero()
        };
        let element_pointer = unsafe {
            self.builder
                .build_gep(
                    self.llvm_type(element),
                    data,
                    &[index],
                    "array_extreme_element",
                )
                .unwrap()
        };
        let value = self
            .builder
            .build_load(
                self.llvm_type(element),
                element_pointer,
                "array_extreme_value",
            )
            .map_err(|error| error.to_string())?;
        let (malloc, size) = self.declare_malloc(element_size);
        let storage = self
            .builder
            .build_call(malloc, &[size.into()], "array_extreme_storage")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .ok_or("internal: array extreme malloc returned void")?
            .into_pointer_value();
        self.builder.build_store(storage, value).unwrap();
        self.builder.build_store(result, storage).unwrap();
        self.builder.build_unconditional_branch(done).unwrap();
        self.builder.position_at_end(done);
        self.builder
            .build_load(pointer, result, "array_extreme_result")
            .map_err(|error| error.to_string())
    }

    fn emit_array_numeric_reduction(
        &mut self,
        array_slot: PointerValue<'ctx>,
        llvm_array_type: BasicTypeEnum<'ctx>,
        element: &Type,
        product: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let array = self
            .builder
            .build_load(llvm_array_type, array_slot, "array_numeric_reduction")
            .map_err(|error| error.to_string())?
            .into_struct_value();
        let data = self
            .builder
            .build_extract_value(array, 0, "array_numeric_data")
            .map_err(|error| error.to_string())?
            .into_pointer_value();
        let length = self
            .builder
            .build_extract_value(array, 1, "array_numeric_length")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let element_llvm = self.llvm_type(element);
        let accumulator = self
            .builder
            .build_alloca(element_llvm, "array_numeric_accumulator")
            .unwrap();
        let initial: BasicValueEnum<'ctx> = if element_llvm.is_float_type() {
            element_llvm
                .into_float_type()
                .const_float(if product { 1.0 } else { 0.0 })
                .into()
        } else {
            element_llvm
                .into_int_type()
                .const_int(if product { 1 } else { 0 }, false)
                .into()
        };
        self.builder.build_store(accumulator, initial).unwrap();
        let index = self
            .builder
            .build_alloca(self.context.i64_type(), "array_numeric_index")
            .unwrap();
        self.builder
            .build_store(index, self.context.i64_type().const_zero())
            .unwrap();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: numeric array reduction outside function")?;
        let condition = self
            .context
            .append_basic_block(function, "array_numeric.condition");
        let body = self
            .context
            .append_basic_block(function, "array_numeric.body");
        let done = self
            .context
            .append_basic_block(function, "array_numeric.done");
        self.builder.build_unconditional_branch(condition).unwrap();
        self.builder.position_at_end(condition);
        let current_index = self
            .builder
            .build_load(self.context.i64_type(), index, "array_numeric_index_value")
            .unwrap()
            .into_int_value();
        let more = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                current_index,
                length,
                "array_numeric_more",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(more, body, done)
            .unwrap();
        self.builder.position_at_end(body);
        let element_pointer = unsafe {
            self.builder
                .build_gep(
                    element_llvm,
                    data,
                    &[current_index],
                    "array_numeric_element",
                )
                .unwrap()
        };
        let value = self
            .builder
            .build_load(element_llvm, element_pointer, "array_numeric_value")
            .map_err(|error| error.to_string())?;
        let current = self
            .builder
            .build_load(element_llvm, accumulator, "array_numeric_current")
            .map_err(|error| error.to_string())?;
        let next: BasicValueEnum<'ctx> = if element_llvm.is_float_type() {
            let result = if product {
                self.builder
                    .build_float_mul(
                        current.into_float_value(),
                        value.into_float_value(),
                        "array_product_next",
                    )
                    .unwrap()
            } else {
                self.builder
                    .build_float_add(
                        current.into_float_value(),
                        value.into_float_value(),
                        "array_sum_next",
                    )
                    .unwrap()
            };
            result.into()
        } else {
            let result = if product {
                self.builder
                    .build_int_mul(
                        current.into_int_value(),
                        value.into_int_value(),
                        "array_product_next",
                    )
                    .unwrap()
            } else {
                self.builder
                    .build_int_add(
                        current.into_int_value(),
                        value.into_int_value(),
                        "array_sum_next",
                    )
                    .unwrap()
            };
            result.into()
        };
        self.builder.build_store(accumulator, next).unwrap();
        let next_index = self
            .builder
            .build_int_add(
                current_index,
                self.context.i64_type().const_int(1, false),
                "array_numeric_next",
            )
            .unwrap();
        self.builder.build_store(index, next_index).unwrap();
        self.builder.build_unconditional_branch(condition).unwrap();
        self.builder.position_at_end(done);
        self.builder
            .build_load(element_llvm, accumulator, "array_numeric_result")
            .map_err(|error| error.to_string())
    }

    fn emit_array_average(
        &mut self,
        array_slot: PointerValue<'ctx>,
        llvm_array_type: BasicTypeEnum<'ctx>,
        element: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let array = self
            .builder
            .build_load(llvm_array_type, array_slot, "array_average")
            .map_err(|error| error.to_string())?
            .into_struct_value();
        let data = self
            .builder
            .build_extract_value(array, 0, "array_average_data")
            .map_err(|error| error.to_string())?
            .into_pointer_value();
        let length = self
            .builder
            .build_extract_value(array, 1, "array_average_length")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let result_pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let output_type = if matches!(transparent_codegen_type(element), Type::F128) {
            self.context.f128_type()
        } else {
            self.context.f64_type()
        };
        let result = self
            .builder
            .build_alloca(result_pointer, "array_average_result")
            .unwrap();
        self.builder
            .build_store(result, result_pointer.const_null())
            .unwrap();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: array average outside function")?;
        let empty = self
            .context
            .append_basic_block(function, "array_average.empty");
        let present = self
            .context
            .append_basic_block(function, "array_average.present");
        let condition = self
            .context
            .append_basic_block(function, "array_average.condition");
        let body = self
            .context
            .append_basic_block(function, "array_average.body");
        let finish_value = self
            .context
            .append_basic_block(function, "array_average.finish_value");
        let done = self
            .context
            .append_basic_block(function, "array_average.done");
        let is_empty = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                length,
                self.context.i64_type().const_zero(),
                "array_average_empty",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(is_empty, empty, present)
            .unwrap();
        self.builder.position_at_end(empty);
        self.builder.build_unconditional_branch(done).unwrap();
        self.builder.position_at_end(present);
        let accumulator = self
            .builder
            .build_alloca(output_type, "array_average_accumulator")
            .unwrap();
        self.builder
            .build_store(accumulator, output_type.const_zero())
            .unwrap();
        let index = self
            .builder
            .build_alloca(self.context.i64_type(), "array_average_index")
            .unwrap();
        self.builder
            .build_store(index, self.context.i64_type().const_zero())
            .unwrap();
        self.builder.build_unconditional_branch(condition).unwrap();
        self.builder.position_at_end(condition);
        let current_index = self
            .builder
            .build_load(self.context.i64_type(), index, "array_average_index_value")
            .unwrap()
            .into_int_value();
        let more = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                current_index,
                length,
                "array_average_more",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(more, body, finish_value)
            .unwrap();
        self.builder.position_at_end(body);
        let element_llvm = self.llvm_type(element);
        let element_pointer = unsafe {
            self.builder
                .build_gep(
                    element_llvm,
                    data,
                    &[current_index],
                    "array_average_element",
                )
                .unwrap()
        };
        let value = self
            .builder
            .build_load(element_llvm, element_pointer, "array_average_value")
            .map_err(|error| error.to_string())?;
        let value = match transparent_codegen_type(element) {
            ty if is_float_type(ty) => {
                let value = value.into_float_value();
                if output_type.get_bit_width() == value.get_type().get_bit_width() {
                    value
                } else {
                    self.builder
                        .build_float_ext(value, output_type, "array_average_promote")
                        .unwrap()
                }
            }
            ty if is_unsigned_int_type(ty) => self
                .builder
                .build_unsigned_int_to_float(
                    value.into_int_value(),
                    output_type,
                    "array_average_convert",
                )
                .unwrap(),
            ty if is_integer_type(ty) => self
                .builder
                .build_signed_int_to_float(
                    value.into_int_value(),
                    output_type,
                    "array_average_convert",
                )
                .unwrap(),
            _ => {
                return Err(format!(
                    "error: array average is not supported for `{element}`"
                ));
            }
        };
        let current = self
            .builder
            .build_load(output_type, accumulator, "array_average_current")
            .unwrap()
            .into_float_value();
        let next = self
            .builder
            .build_float_add(current, value, "array_average_sum")
            .unwrap();
        self.builder.build_store(accumulator, next).unwrap();
        let next_index = self
            .builder
            .build_int_add(
                current_index,
                self.context.i64_type().const_int(1, false),
                "array_average_next",
            )
            .unwrap();
        self.builder.build_store(index, next_index).unwrap();
        self.builder.build_unconditional_branch(condition).unwrap();
        self.builder.position_at_end(finish_value);
        let total = self
            .builder
            .build_load(output_type, accumulator, "array_average_total")
            .unwrap()
            .into_float_value();
        let count = self
            .builder
            .build_unsigned_int_to_float(length, output_type, "array_average_count")
            .unwrap();
        let average = self
            .builder
            .build_float_div(total, count, "array_average_value")
            .unwrap();
        let (malloc, size) = self.declare_malloc(output_type.size_of());
        let storage = self
            .builder
            .build_call(malloc, &[size.into()], "array_average_storage")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .ok_or("internal: array average malloc returned void")?
            .into_pointer_value();
        self.builder.build_store(storage, average).unwrap();
        self.builder.build_store(result, storage).unwrap();
        self.builder.build_unconditional_branch(done).unwrap();
        self.builder.position_at_end(done);
        self.builder
            .build_load(result_pointer, result, "array_average_result_value")
            .map_err(|error| error.to_string())
    }

    fn emit_array_entries(
        &mut self,
        array_slot: PointerValue<'ctx>,
        array_type: BasicTypeEnum<'ctx>,
        element: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let usize_type = self.usize_type();
        let array = self
            .builder
            .build_load(array_type, array_slot, "entries_src")
            .unwrap()
            .into_struct_value();
        let data = self
            .builder
            .build_extract_value(array, 0, "entries_data")
            .unwrap()
            .into_pointer_value();
        let length = self
            .builder
            .build_extract_value(array, 1, "entries_len")
            .unwrap()
            .into_int_value();
        let element_llvm = self.llvm_type(element);
        let tuple_ty = Type::Tuple(vec![Type::Usize, element.clone()]);
        let tuple_llvm = self.llvm_type(&tuple_ty);
        let entry_size = tuple_llvm
            .size_of()
            .ok_or_else(|| "error: array entry type has no native size".to_string())?;
        let bytes = self
            .builder
            .build_int_mul(entry_size, length, "entries_bytes")
            .unwrap();
        let (malloc, bytes) = self.declare_malloc(bytes);
        let out_data = self
            .builder
            .build_call(malloc, &[bytes.into()], "entries_out")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();

        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or_else(|| "internal: entries loop outside a function".to_string())?;
        let index_slot = self.entry_alloca(usize_type, "entries_i").unwrap();
        self.builder
            .build_store(index_slot, usize_type.const_zero())
            .unwrap();
        let cond = self.context.append_basic_block(function, "aentries.cond");
        let body = self.context.append_basic_block(function, "aentries.body");
        let end = self.context.append_basic_block(function, "aentries.end");
        self.builder.build_unconditional_branch(cond).unwrap();

        self.builder.position_at_end(cond);
        let index = self
            .builder
            .build_load(usize_type, index_slot, "entries_idx")
            .unwrap()
            .into_int_value();
        let more = self
            .builder
            .build_int_compare(IntPredicate::ULT, index, length, "entries_more")
            .unwrap();
        self.builder
            .build_conditional_branch(more, body, end)
            .unwrap();

        self.builder.position_at_end(body);
        let element_slot = unsafe {
            self.builder
                .build_gep(element_llvm, data, &[index], "entries_elem_ptr")
                .unwrap()
        };
        let element_value = self
            .builder
            .build_load(element_llvm, element_slot, "entries_elem")
            .unwrap();
        let mut tuple = tuple_llvm.into_struct_type().get_undef();
        tuple = self
            .builder
            .build_insert_value(tuple, index, 0, "entries_tuple_i")
            .unwrap()
            .into_struct_value();
        tuple = self
            .builder
            .build_insert_value(tuple, element_value, 1, "entries_tuple_v")
            .unwrap()
            .into_struct_value();
        let dest = unsafe {
            self.builder
                .build_gep(tuple_llvm, out_data, &[index], "entries_dest")
                .unwrap()
        };
        self.builder.build_store(dest, tuple).unwrap();
        let next = self
            .builder
            .build_int_add(index, usize_type.const_int(1, false), "entries_next")
            .unwrap();
        self.builder.build_store(index_slot, next).unwrap();
        self.builder.build_unconditional_branch(cond).unwrap();

        self.builder.position_at_end(end);
        let out_type = self.llvm_type(&Type::Array(Box::new(tuple_ty)));
        let mut result = out_type.into_struct_type().get_undef();
        result = self
            .builder
            .build_insert_value(result, out_data, 0, "aentries_data")
            .unwrap()
            .into_struct_value();
        result = self
            .builder
            .build_insert_value(result, length, 1, "aentries_len")
            .unwrap()
            .into_struct_value();
        result = self
            .builder
            .build_insert_value(result, length, 2, "aentries_cap")
            .unwrap()
            .into_struct_value();
        Ok(result.into())
    }

    pub(super) fn array_length_from_slot(
        &self,
        slot: PointerValue<'ctx>,
        array_type: BasicTypeEnum<'ctx>,
    ) -> IntValue<'ctx> {
        self.builder
            .build_extract_value(
                self.builder
                    .build_load(array_type, slot, "array")
                    .unwrap()
                    .into_struct_value(),
                1,
                "array_length",
            )
            .unwrap()
            .into_int_value()
    }
}

pub(super) struct TupleSlice<'ctx> {
    pub(super) data: PointerValue<'ctx>,
    pub(super) length: IntValue<'ctx>,
    pub(super) element_type: Type,
    pub(super) element_llvm: BasicTypeEnum<'ctx>,
}
