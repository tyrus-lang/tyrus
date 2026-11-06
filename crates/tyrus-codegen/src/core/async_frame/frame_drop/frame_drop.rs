use super::*;

/// The frame layout the drop function walks. Every field is `Copy`, so each
/// group of releases can live in a function of its own.
#[derive(Clone, Copy)]
pub(super) struct AsyncFrameDrop<'a, 'ctx> {
    pub(super) drop: FunctionValue<'ctx>,
    pub(super) frame_type: inkwell::types::StructType<'ctx>,
    pub(super) frame: PointerValue<'ctx>,
    pub(super) frame_params: &'a [AsyncFrameParam],
    pub(super) frame_locals: &'a [(String, Type, AsyncLocalStorage)],
    pub(super) expression_temporaries: &'a [(Span, Type)],
    pub(super) continuation_field_count: usize,
    pub(super) capture_param_count: usize,
    pub(super) pointer: inkwell::types::PointerType<'ctx>,
}

impl<'ctx> Codegen<'ctx> {
    /// Releases the promises and the await temporaries the frame holds.
    pub(super) fn emit_async_frame_drop_promises(&mut self, layout: AsyncFrameDrop<'_, 'ctx>) {
        let AsyncFrameDrop {
            drop,
            frame_type,
            frame,
            frame_params,
            frame_locals,
            expression_temporaries,
            continuation_field_count: _,
            capture_param_count: _,
            pointer,
        } = layout;
        let mut promise_fields = vec![(1usize, "dependency")];
        promise_fields.extend(
            frame_params
                .iter()
                .enumerate()
                .filter(|(_, parameter)| async_promise_handle(&parameter.ty))
                .map(|(index, parameter)| (index + 2, parameter.storage_name.as_str())),
        );
        promise_fields.extend(
            frame_locals
                .iter()
                .enumerate()
                .filter(|(_, (_, ty, _))| async_promise_handle(ty))
                .map(|(index, (field, _, _))| (index + frame_params.len() + 2, field.as_str())),
        );
        let promise_free = self.declare_libc_fn("tyrus_promise_free", &[pointer.into()], None);
        for (index, field_name) in promise_fields {
            let slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    index as u32,
                    &format!("async.drop.{field_name}.slot"),
                )
                .unwrap();
            let value = self
                .builder
                .build_load(pointer, slot, &format!("async.drop.{field_name}"))
                .unwrap()
                .into_pointer_value();
            let release = self.context.append_basic_block(drop, "release.promise");
            let next = self.context.append_basic_block(drop, "next.promise");
            let present = self
                .builder
                .build_is_not_null(value, &format!("async.drop.{field_name}.present"))
                .unwrap();
            self.builder
                .build_conditional_branch(present, release, next)
                .unwrap();
            self.builder.position_at_end(release);
            self.builder
                .build_call(promise_free, &[value.into()], "")
                .unwrap();
            self.builder.build_unconditional_branch(next).unwrap();
            self.builder.position_at_end(next);
        }
        for (index, (_, ty)) in expression_temporaries.iter().enumerate() {
            if ty.is_copy() {
                continue;
            }
            let slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    (index + frame_params.len() + frame_locals.len() + 2) as u32,
                    "async.drop.expression.temporary",
                )
                .unwrap();
            self.emit_async_drop_value_in_place(drop, slot, ty)
                .expect("async expression temporary drop glue must be supported");
        }
        for (index, parameter) in frame_params
            .iter()
            .enumerate()
            .filter(|(_, parameter)| matches!(transparent_codegen_type(&parameter.ty), Type::Str))
        {
            let field_name = &parameter.storage_name;
            let slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    (index + 2) as u32,
                    &format!("async.drop.{field_name}.slot"),
                )
                .unwrap();
            let value = self
                .builder
                .build_load(
                    self.string_abi_type(),
                    slot,
                    &format!("async.drop.{field_name}"),
                )
                .unwrap()
                .into_struct_value();
            self.emit_string_release(value)
                .expect("releasing a captured string is always emittable");
        }
        for (index, (local_name, _, _storage)) in
            frame_locals
                .iter()
                .enumerate()
                .filter(|(_, (_, ty, storage))| {
                    matches!(storage, AsyncLocalStorage::OwnedString)
                        && matches!(transparent_codegen_type(ty), Type::Str)
                })
        {
            let slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    (index + frame_params.len() + 2) as u32,
                    &format!("async.drop.{local_name}.slot"),
                )
                .unwrap();
            let value = self
                .builder
                .build_load(
                    self.string_abi_type(),
                    slot,
                    &format!("async.drop.{local_name}"),
                )
                .unwrap()
                .into_struct_value();
            self.emit_string_release(value)
                .expect("releasing an async frame local is always emittable");
        }
    }

    /// Releases the locals the frame owns, by ownership mode.
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_async_frame_drop_locals(&mut self, layout: AsyncFrameDrop<'_, 'ctx>) {
        let AsyncFrameDrop {
            drop,
            frame_type,
            frame,
            frame_params,
            frame_locals,
            expression_temporaries: _,
            continuation_field_count,
            capture_param_count: _,
            pointer,
        } = layout;
        let mut ownership_index =
            frame_params.len() + frame_locals.len() + continuation_field_count + 2;
        for (index, (local_name, _, storage)) in frame_locals.iter().enumerate() {
            if !matches!(storage, AsyncLocalStorage::TrackedString) {
                continue;
            }
            let slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    (index + frame_params.len() + 2) as u32,
                    &format!("async.drop.{local_name}.slot"),
                )
                .unwrap();
            let ownership = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    ownership_index as u32,
                    &format!("async.drop.{local_name}.owned.slot"),
                )
                .unwrap();
            ownership_index += 1;
            let owned = self
                .builder
                .build_load(
                    self.context.bool_type(),
                    ownership,
                    &format!("async.drop.{local_name}.owned"),
                )
                .unwrap()
                .into_int_value();
            let release = self
                .context
                .append_basic_block(drop, "release.tracked.local.string");
            let next = self
                .context
                .append_basic_block(drop, "next.tracked.local.string");
            self.builder
                .build_conditional_branch(owned, release, next)
                .unwrap();
            self.builder.position_at_end(release);
            let value = self
                .builder
                .build_load(
                    self.string_abi_type(),
                    slot,
                    &format!("async.drop.{local_name}"),
                )
                .unwrap()
                .into_struct_value();
            self.emit_string_release(value)
                .expect("releasing a tracked frame local is always emittable");
            self.builder.build_unconditional_branch(next).unwrap();
            self.builder.position_at_end(next);
        }
        for (index, (local_name, local_type, storage)) in frame_locals.iter().enumerate() {
            if !matches!(
                storage,
                AsyncLocalStorage::CopyArray
                    | AsyncLocalStorage::TrackedCopyArray
                    | AsyncLocalStorage::StaticStringArray
            ) {
                continue;
            }
            let slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    (index + frame_params.len() + 2) as u32,
                    &format!("async.drop.{local_name}.slot"),
                )
                .unwrap();
            let array = self
                .builder
                .build_load(
                    self.llvm_type(local_type),
                    slot,
                    &format!("async.drop.{local_name}"),
                )
                .unwrap()
                .into_struct_value();
            let data = self
                .builder
                .build_extract_value(array, 0, &format!("async.drop.{local_name}.data"))
                .unwrap()
                .into_pointer_value();
            let release = self.context.append_basic_block(drop, "release.local.array");
            let next = self.context.append_basic_block(drop, "next.local.array");
            let present = self
                .builder
                .build_is_not_null(data, &format!("async.drop.{local_name}.present"))
                .unwrap();
            self.builder
                .build_conditional_branch(present, release, next)
                .unwrap();
            self.builder.position_at_end(release);
            let free = self.declare_libc_fn("free", &[pointer.into()], None);
            self.builder.build_call(free, &[data.into()], "").unwrap();
            self.builder.build_unconditional_branch(next).unwrap();
            self.builder.position_at_end(next);
        }
        let drop_owned_string_array =
            self.declare_libc_fn("tyrus_array_drop_string_contents", &[pointer.into()], None);
        for (index, (local_name, _, storage)) in frame_locals.iter().enumerate() {
            if !matches!(storage, AsyncLocalStorage::OwnedStringArray) {
                continue;
            }
            let slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    (index + frame_params.len() + 2) as u32,
                    &format!("async.drop.{local_name}.slot"),
                )
                .unwrap();
            self.builder
                .build_call(drop_owned_string_array, &[slot.into()], "")
                .unwrap();
        }
        for (index, (local_name, local_type, storage)) in frame_locals.iter().enumerate() {
            if !matches!(storage, AsyncLocalStorage::TrackedStringArray) {
                continue;
            }
            let slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    (index + frame_params.len() + 2) as u32,
                    &format!("async.drop.{local_name}.slot"),
                )
                .unwrap();
            let mode_slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    ownership_index as u32,
                    &format!("async.drop.{local_name}.array.ownership.slot"),
                )
                .unwrap();
            ownership_index += 1;
            let mode = self
                .builder
                .build_load(
                    self.context.i8_type(),
                    mode_slot,
                    &format!("async.drop.{local_name}.array.ownership"),
                )
                .unwrap()
                .into_int_value();
            let release_deep = self
                .context
                .append_basic_block(drop, "release.tracked.local.string.array.deep");
            let check_shallow = self
                .context
                .append_basic_block(drop, "check.tracked.local.string.array.shallow");
            let release_shallow = self
                .context
                .append_basic_block(drop, "release.tracked.local.string.array.shallow");
            let next = self
                .context
                .append_basic_block(drop, "next.tracked.local.string.array");
            let deep = self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    mode,
                    self.context.i8_type().const_int(2, false),
                    &format!("async.drop.{local_name}.array.deep"),
                )
                .unwrap();
            self.builder
                .build_conditional_branch(deep, release_deep, check_shallow)
                .unwrap();
            self.builder.position_at_end(release_deep);
            self.builder
                .build_call(drop_owned_string_array, &[slot.into()], "")
                .unwrap();
            self.builder.build_unconditional_branch(next).unwrap();
            self.builder.position_at_end(check_shallow);
            let shallow = self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    mode,
                    self.context.i8_type().const_int(1, false),
                    &format!("async.drop.{local_name}.array.shallow"),
                )
                .unwrap();
            self.builder
                .build_conditional_branch(shallow, release_shallow, next)
                .unwrap();
            self.builder.position_at_end(release_shallow);
            let array = self
                .builder
                .build_load(
                    self.llvm_type(local_type),
                    slot,
                    &format!("async.drop.{local_name}"),
                )
                .unwrap()
                .into_struct_value();
            let data = self
                .builder
                .build_extract_value(array, 0, &format!("async.drop.{local_name}.data"))
                .unwrap()
                .into_pointer_value();
            let free = self.declare_libc_fn("free", &[pointer.into()], None);
            self.builder.build_call(free, &[data.into()], "").unwrap();
            self.builder.build_unconditional_branch(next).unwrap();
            self.builder.position_at_end(next);
        }
        for (index, (local_name, local_type, storage)) in frame_locals.iter().enumerate() {
            if !matches!(storage, AsyncLocalStorage::StaticObject) {
                continue;
            }
            let Some(fields) = self.async_object_fields(local_type) else {
                continue;
            };
            let object_type = self.llvm_type(local_type).into_struct_type();
            let object_slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    (index + frame_params.len() + 2) as u32,
                    &format!("async.drop.{local_name}.slot"),
                )
                .unwrap();
            for (field_index, field) in fields.iter().enumerate() {
                let field_type = self.resolve_type(&field_storage_type(field));
                if !async_string_array(&field_type) {
                    continue;
                }
                let field_slot = self
                    .builder
                    .build_struct_gep(
                        object_type,
                        object_slot,
                        field_index as u32,
                        &format!("async.drop.{local_name}.{}.slot", field.name),
                    )
                    .unwrap();
                let array = self
                    .builder
                    .build_load(
                        self.llvm_type(&field_type),
                        field_slot,
                        &format!("async.drop.{local_name}.{}", field.name),
                    )
                    .unwrap()
                    .into_struct_value();
                let data = self
                    .builder
                    .build_extract_value(
                        array,
                        0,
                        &format!("async.drop.{local_name}.{}.data", field.name),
                    )
                    .unwrap()
                    .into_pointer_value();
                let release = self
                    .context
                    .append_basic_block(drop, "release.static.object.string.array");
                let next = self
                    .context
                    .append_basic_block(drop, "next.static.object.string.array");
                let present = self
                    .builder
                    .build_is_not_null(
                        data,
                        &format!("async.drop.{local_name}.{}.present", field.name),
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(present, release, next)
                    .unwrap();
                self.builder.position_at_end(release);
                let free = self.declare_libc_fn("free", &[pointer.into()], None);
                self.builder.build_call(free, &[data.into()], "").unwrap();
                self.builder.build_unconditional_branch(next).unwrap();
                self.builder.position_at_end(next);
            }
        }
        for (index, (local_name, local_type, storage)) in frame_locals.iter().enumerate() {
            if !matches!(storage, AsyncLocalStorage::OwnedValue) {
                continue;
            }
            let slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    (index + frame_params.len() + 2) as u32,
                    &format!("async.drop.{local_name}.slot"),
                )
                .unwrap();
            let initialized_slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    ownership_index as u32,
                    &format!("async.drop.{local_name}.initialized.slot"),
                )
                .unwrap();
            ownership_index += 1;
            let initialized = self
                .builder
                .build_load(
                    self.context.bool_type(),
                    initialized_slot,
                    &format!("async.drop.{local_name}.initialized"),
                )
                .unwrap()
                .into_int_value();
            let release = self
                .context
                .append_basic_block(drop, "release.initialized.owned.local");
            let next = self
                .context
                .append_basic_block(drop, "next.initialized.owned.local");
            self.builder
                .build_conditional_branch(initialized, release, next)
                .unwrap();
            self.builder.position_at_end(release);
            self.emit_async_drop_value_in_place(drop, slot, local_type)
                .unwrap();
            if !self.current_block_terminated() {
                self.builder.build_unconditional_branch(next).unwrap();
            }
            self.builder.position_at_end(next);
        }
    }

    /// Releases the parameters the frame owns.
    pub(super) fn emit_async_frame_drop_params(&mut self, layout: AsyncFrameDrop<'_, 'ctx>) {
        let AsyncFrameDrop {
            drop,
            frame_type,
            frame,
            frame_params,
            frame_locals: _,
            expression_temporaries: _,
            continuation_field_count: _,
            capture_param_count,
            pointer,
        } = layout;
        for (index, parameter) in frame_params
            .iter()
            .enumerate()
            .filter(|(_, parameter)| async_optional_copy(&parameter.ty))
        {
            let field_name = &parameter.storage_name;
            let slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    (index + 2) as u32,
                    &format!("async.drop.{field_name}.slot"),
                )
                .unwrap();
            let value = self
                .builder
                .build_load(pointer, slot, &format!("async.drop.{field_name}"))
                .unwrap()
                .into_pointer_value();
            let release = self.context.append_basic_block(drop, "release.optional");
            let next = self.context.append_basic_block(drop, "next.optional");
            let present = self
                .builder
                .build_is_not_null(value, &format!("async.drop.{field_name}.present"))
                .unwrap();
            self.builder
                .build_conditional_branch(present, release, next)
                .unwrap();
            self.builder.position_at_end(release);
            let free = self.declare_libc_fn("free", &[pointer.into()], None);
            self.builder.build_call(free, &[value.into()], "").unwrap();
            self.builder.build_unconditional_branch(next).unwrap();
            self.builder.position_at_end(next);
        }
        for (index, parameter) in frame_params
            .iter()
            .enumerate()
            .filter(|(_, parameter)| async_copy_array(&parameter.ty))
        {
            let field_name = &parameter.storage_name;
            let slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    (index + 2) as u32,
                    &format!("async.drop.{field_name}.slot"),
                )
                .unwrap();
            let array = self
                .builder
                .build_load(
                    self.llvm_type(&parameter.ty),
                    slot,
                    &format!("async.drop.{field_name}"),
                )
                .unwrap()
                .into_struct_value();
            let data = self
                .builder
                .build_extract_value(array, 0, &format!("async.drop.{field_name}.data"))
                .unwrap()
                .into_pointer_value();
            let release = self.context.append_basic_block(drop, "release.array");
            let next = self.context.append_basic_block(drop, "next.array");
            let present = self
                .builder
                .build_is_not_null(data, &format!("async.drop.{field_name}.present"))
                .unwrap();
            self.builder
                .build_conditional_branch(present, release, next)
                .unwrap();
            self.builder.position_at_end(release);
            let free = self.declare_libc_fn("free", &[pointer.into()], None);
            self.builder.build_call(free, &[data.into()], "").unwrap();
            self.builder.build_unconditional_branch(next).unwrap();
            self.builder.position_at_end(next);
        }
        let drop_string_array =
            self.declare_libc_fn("tyrus_array_drop_string_contents", &[pointer.into()], None);
        for (index, parameter) in frame_params
            .iter()
            .enumerate()
            .filter(|(_, parameter)| async_string_array(&parameter.ty))
        {
            let slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    (index + 2) as u32,
                    &format!("async.drop.{}.slot", parameter.storage_name),
                )
                .unwrap();
            self.builder
                .build_call(drop_string_array, &[slot.into()], "")
                .unwrap();
        }
        for (parameter_index, parameter) in frame_params.iter().enumerate() {
            let generic_owned = matches!(
                transparent_codegen_type(&parameter.ty),
                Type::Array(element) if !element.is_copy() && !matches!(transparent_codegen_type(element), Type::Str)
            ) || matches!(
                transparent_codegen_type(&parameter.ty),
                Type::Optional(inner) if !inner.is_copy()
            ) || matches!(
                transparent_codegen_type(&parameter.ty),
                Type::Apply { name, args }
                    if matches!(
                        name.as_str(),
                        "Sender"
                            | "Receiver"
                            | "Mutex"
                            | "MutexGuard"
                            | "RwLock"
                            | "RwReadGuard"
                            | "RwWriteGuard"
                    ) && args.len() == 1
            ) || matches!(
                transparent_codegen_type(&parameter.ty),
                Type::Apply { name, args }
                    if matches!(name.as_str(), "DuplexStream" | "TransformStream") && args.len() == 2
            ) || matches!(
                transparent_codegen_type(&parameter.ty),
                Type::Apply { name, args }
                    if matches!(name.as_str(), "ReadableStream" | "WritableStream") && args.len() == 1
            ) || matches!(
                transparent_codegen_type(&parameter.ty),
                Type::Named(name) if matches!(name.as_str(), "Semaphore" | "SemaphorePermit" | "ChildProcess" | "WebSocket")
            ) || self.async_object_fields(&parameter.ty).is_some();
            if !generic_owned {
                continue;
            }
            // A capture is borrowed, not owned: the closure's environment drop
            // releases it. Releasing it here as well frees a reference nobody
            // took, measured as a double free of `Arc<TyrusMutex>` that valgrind
            // caught in `__tyrus_async_drop_*` and that showed up as heap
            // corruption far from here.
            if parameter_index < capture_param_count
                && !async_ref_counted_native_handle(&parameter.ty)
            {
                continue;
            }
            let object_slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    (parameter_index + 2) as u32,
                    &format!("async.drop.{}.slot", parameter.storage_name),
                )
                .unwrap();
            self.emit_async_drop_value_in_place(drop, object_slot, &parameter.ty)
                .unwrap();
        }
    }
}
