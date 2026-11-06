use super::*;

#[path = "frame_drop/frame_drop.rs"]
mod frame_drop;
use frame_drop::AsyncFrameDrop;

impl<'ctx> Codegen<'ctx> {
    pub(crate) fn async_frame_slot(&self, name: &str) -> Option<(PointerValue<'ctx>, Type)> {
        self.async_frame_slots
            .last()
            .and_then(|slots| slots.get(name))
            .cloned()
    }

    pub(super) fn bind_async_frame_locals(&mut self) {
        let slots = self
            .async_frame_slots
            .last()
            .into_iter()
            .flat_map(|slots| slots.iter())
            .map(|(name, (slot, ty))| (name.clone(), *slot, ty.clone()))
            .collect::<Vec<_>>();
        for (name, slot, ty) in slots {
            self.store_var(&name, slot, ty);
        }
    }

    pub(crate) fn async_frame_string_ownership(&self, name: &str) -> Option<PointerValue<'ctx>> {
        self.async_frame_string_ownership
            .last()
            .and_then(|slots| slots.get(name))
            .copied()
    }

    pub(crate) fn async_frame_tracks_copy_array(&self, name: &str) -> bool {
        self.async_frame_tracked_copy_arrays
            .last()
            .is_some_and(|arrays| arrays.contains(name))
    }

    pub(crate) fn async_frame_string_array_ownership(
        &mut self,
        name: &str,
    ) -> Option<PointerValue<'ctx>> {
        self.async_frame_string_array_ownership
            .last()
            .and_then(|slots| slots.get(name))
            .copied()
    }

    pub(crate) fn async_frame_owns_value(&self, name: &str) -> bool {
        self.async_frame_owned_values
            .last()
            .is_some_and(|values| values.contains(name))
    }

    pub(crate) fn mark_async_frame_value_initialized(&mut self, name: &str, initialized: bool) {
        let Some(slot) = self
            .async_frame_owned_initialization
            .last()
            .and_then(|slots| slots.get(name))
            .copied()
        else {
            return;
        };
        self.builder
            .build_store(
                slot,
                self.context
                    .bool_type()
                    .const_int(u64::from(initialized), false),
            )
            .unwrap();
    }

    pub(super) fn mark_async_frame_slot_uninitialized(&mut self, slot: PointerValue<'ctx>) {
        self.mark_async_frame_slot_initialized(slot, false);
    }

    pub(super) fn async_frame_slot_name(&self, slot: PointerValue<'ctx>) -> Option<String> {
        self.async_frame_slots.last().and_then(|slots| {
            slots
                .iter()
                .find(|(_, (candidate, _))| candidate.as_value_ref() == slot.as_value_ref())
                .map(|(name, _)| name.clone())
        })
    }

    pub(super) fn mark_async_frame_slot_initialized(
        &mut self,
        slot: PointerValue<'ctx>,
        initialized: bool,
    ) {
        let name = self.async_frame_slot_name(slot);
        if let Some(name) = name {
            self.mark_async_frame_value_initialized(&name, initialized);
        }
    }

    pub(crate) fn clear_async_frame_local_before_store(
        &mut self,
        function: FunctionValue<'ctx>,
        name: &str,
        slot: PointerValue<'ctx>,
        ty: &Type,
    ) -> Result<(), String> {
        let storage = if self
            .async_frame_owned_initialization
            .last()
            .is_some_and(|slots| slots.contains_key(name))
        {
            AsyncLocalStorage::OwnedValue
        } else if let Some(storage) = self
            .async_frame_local_storage
            .last()
            .and_then(|storage| storage.get(name))
            .copied()
        {
            storage
        } else {
            return Ok(());
        };
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        match storage {
            AsyncLocalStorage::OwnedString => {
                let value = self
                    .builder
                    .build_load(self.string_abi_type(), slot, "async.local.previous.string")
                    .unwrap()
                    .into_struct_value();
                self.emit_string_release(value)?;
            }
            AsyncLocalStorage::CopyArray | AsyncLocalStorage::StaticStringArray => {
                let value = self
                    .builder
                    .build_load(self.llvm_type(ty), slot, "async.local.previous.array")
                    .unwrap()
                    .into_struct_value();
                let data = self
                    .builder
                    .build_extract_value(value, 0, "async.local.previous.array.data")
                    .unwrap();
                let free = self.declare_libc_fn("free", &[pointer.into()], None);
                self.builder.build_call(free, &[data.into()], "").unwrap();
            }
            AsyncLocalStorage::OwnedStringArray => {
                let drop_contents = self.declare_libc_fn(
                    "tyrus_array_drop_string_contents",
                    &[pointer.into()],
                    None,
                );
                self.builder
                    .build_call(drop_contents, &[slot.into()], "")
                    .unwrap();
            }
            AsyncLocalStorage::OwnedValue => {
                let initialized = self
                    .async_frame_owned_initialization
                    .last()
                    .and_then(|slots| slots.get(name))
                    .copied()
                    .ok_or_else(|| {
                        format!("internal: async owned local `{name}` has no initialization flag")
                    })?;
                let present = self
                    .builder
                    .build_load(
                        self.context.bool_type(),
                        initialized,
                        "async.local.initialized",
                    )
                    .unwrap()
                    .into_int_value();
                let drop = self
                    .context
                    .append_basic_block(function, "async.local.drop");
                let next = self
                    .context
                    .append_basic_block(function, "async.local.drop.next");
                self.builder
                    .build_conditional_branch(present, drop, next)
                    .unwrap();
                self.builder.position_at_end(drop);
                self.emit_async_drop_value_in_place(function, slot, ty)?;
                if !self.current_block_terminated() {
                    self.builder.build_unconditional_branch(next).unwrap();
                }
                self.builder.position_at_end(next);
                self.mark_async_frame_value_initialized(name, false);
            }
            AsyncLocalStorage::StaticObject => {
                if let Some(fields) = self.async_object_fields(ty) {
                    let object_type = self.llvm_type(ty).into_struct_type();
                    for (index, field) in fields.iter().enumerate() {
                        let field_type = self.resolve_type(&field_storage_type(field));
                        if !async_string_array(&field_type) {
                            continue;
                        }
                        let field_slot = self
                            .builder
                            .build_struct_gep(
                                object_type,
                                slot,
                                index as u32,
                                "async.local.previous.object.array",
                            )
                            .unwrap();
                        let array = self
                            .builder
                            .build_load(
                                self.llvm_type(&field_type),
                                field_slot,
                                "async.local.previous.object.array.value",
                            )
                            .unwrap()
                            .into_struct_value();
                        let data = self
                            .builder
                            .build_extract_value(array, 0, "async.local.previous.object.array.data")
                            .unwrap();
                        let free = self.declare_libc_fn("free", &[pointer.into()], None);
                        self.builder.build_call(free, &[data.into()], "").unwrap();
                    }
                }
            }
            AsyncLocalStorage::TrackedString => {
                let ownership = self.async_frame_string_ownership(name).ok_or_else(|| {
                    format!("internal: async string `{name}` has no ownership flag")
                })?;
                self.replace_async_string_local(
                    function,
                    name,
                    slot,
                    ownership,
                    self.null_string(),
                    false,
                );
            }
            AsyncLocalStorage::TrackedCopyArray => {
                let value = self.llvm_type(ty).const_zero().into_struct_value();
                self.replace_async_copy_array_local(function, name, slot, value);
            }
            AsyncLocalStorage::TrackedStringArray => {
                let ownership = self
                    .async_frame_string_array_ownership(name)
                    .ok_or_else(|| {
                        format!("internal: async string array `{name}` has no ownership mode")
                    })?;
                let value = self.llvm_type(ty).const_zero().into_struct_value();
                self.replace_async_string_array_local(function, name, slot, ownership, value, 0);
            }
            AsyncLocalStorage::None => {}
        }
        Ok(())
    }

    pub(super) fn store_async_frame_parameter(
        &mut self,
        _function: FunctionValue<'ctx>,
        parameter: &AsyncFrameParam,
        slot: PointerValue<'ctx>,
        mut value: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        if async_promise_handle(&parameter.ty) {
            let retain = self.declare_libc_fn(
                "tyrus_promise_retain",
                &[pointer.into()],
                Some(pointer.into()),
            );
            self.builder
                .build_call(
                    retain,
                    &[value.into_pointer_value().into()],
                    "async.frame.param.retain",
                )
                .unwrap();
        } else if matches!(transparent_codegen_type(&parameter.ty), Type::Str) {
            value = self
                .emit_string_clone(value.into_struct_value(), "async.frame.param.string.clone")?;
        } else if let Type::Array(element) = transparent_codegen_type(&parameter.ty) {
            let string_elements = matches!(transparent_codegen_type(element), Type::Str);
            // A rest parameter of Copy elements used to be adopted by pointer
            // here (stored without cloning) while every other array
            // parameter below clones. That made two owners of one buffer: the
            // caller synthesizes the rest array as a temporary and releases it
            // right after the call (`async_argument_cleanups`), and the frame's
            // drop glue released it again. Worse than the double free, an async
            // callee's frame outlives the call, so reading `values[i]` after
            // the caller's release was a use-after-free. Cloning like every
            // sibling case gives the frame a buffer of its own.
            if !element.is_copy() && !string_elements {
                self.emit_async_clone_value_into(slot, value, &parameter.ty)?;
                return Ok(());
            }
            let array_type = self.llvm_type(&parameter.ty);
            let source = self
                .builder
                .build_alloca(array_type, "async.param.array.source")
                .unwrap();
            self.builder.build_store(source, value).unwrap();
            let status = if string_elements {
                let clone = self.declare_libc_fn(
                    "tyrus_array_clone_strings",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.i32_type().into()),
                );
                self.builder
                    .build_call(
                        clone,
                        &[source.into(), slot.into()],
                        "async.param.string.array.clone",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value()
            } else {
                let array = value.into_struct_value();
                let length = self
                    .builder
                    .build_extract_value(array, 1, "async.param.array.length")
                    .unwrap()
                    .into_int_value();
                let element_size = self.llvm_type(element).size_of().ok_or_else(|| {
                    "error: async array parameter element must be sized".to_string()
                })?;
                let clone = self.declare_libc_fn(
                    "tyrus_array_slice",
                    &[
                        pointer.into(),
                        self.context.i64_type().into(),
                        self.context.i64_type().into(),
                        self.context.i64_type().into(),
                        pointer.into(),
                    ],
                    Some(self.context.i32_type().into()),
                );
                self.builder
                    .build_call(
                        clone,
                        &[
                            source.into(),
                            self.context.i64_type().const_zero().into(),
                            length.into(),
                            element_size.into(),
                            slot.into(),
                        ],
                        "async.param.array.clone",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value()
            };
            self.emit_abort_if_negative(status, "async parameter", "async.param.array")?;
            return Ok(());
        } else if async_ref_counted_native_handle(&parameter.ty) {
            // A call borrows its arguments (`docs/language/ownership.md`), so
            // the frame takes a reference of its own for every parameter, not
            // only for captures. The caller keeps the one it passed and the
            // frame's drop glue releases this one. Adopting the caller's
            // reference instead made the call a hidden move: the caller's slot
            // had to be zeroed, and its next use read a null handle.
            self.emit_async_clone_value_into(slot, value, &parameter.ty)?;
            return Ok(());
        } else if matches!(transparent_codegen_type(&parameter.ty), Type::Optional(_))
            || self.async_object_fields(&parameter.ty).is_some()
        {
            self.emit_async_clone_value_into(slot, value, &parameter.ty)?;
            return Ok(());
        }
        self.builder.build_store(slot, value).unwrap();
        Ok(())
    }

    pub(super) fn bind_async_frame_parameter(
        &mut self,
        parameter: &AsyncFrameParam,
        slot: PointerValue<'ctx>,
    ) -> Result<(), String> {
        match &parameter.pattern {
            ParamPattern::Name(name) => {
                self.store_var(name, slot, parameter.ty.clone());
            }
            ParamPattern::Array(names) => {
                let Type::Array(element) = transparent_codegen_type(&parameter.ty) else {
                    return Err(
                        "error: async array destructure parameter must have array type".to_string(),
                    );
                };
                let array_type = self.llvm_type(&parameter.ty);
                let array = self
                    .builder
                    .build_load(array_type, slot, "async.param.array")
                    .map_err(|error| error.to_string())?
                    .into_struct_value();
                let data = self
                    .builder
                    .build_extract_value(array, 0, "async.param.array.data")
                    .unwrap()
                    .into_pointer_value();
                let length = self
                    .builder
                    .build_extract_value(array, 1, "async.param.array.length")
                    .unwrap()
                    .into_int_value();
                let element_type = self.resolve_type(element);
                let llvm_element = self.llvm_type(&element_type);
                for (index, name) in names.iter().enumerate() {
                    let index = self.context.i64_type().const_int(index as u64, false);
                    self.emit_array_bounds_check(index, length)?;
                    let binding = unsafe {
                        self.builder
                            .build_gep(llvm_element, data, &[index], name)
                            .unwrap()
                    };
                    self.store_var(name, binding, element_type.clone());
                }
            }
            ParamPattern::Object(names) => {
                let (fields, type_name) =
                    match transparent_codegen_type(&parameter.ty) {
                        Type::Named(name) => (
                            self.interfaces.get(name).cloned().unwrap_or_default(),
                            name.clone(),
                        ),
                        Type::Object(fields) => (fields.clone(), "{ ... }".to_string()),
                        _ => return Err(
                            "error: async object destructure parameter must have an object type"
                                .to_string(),
                        ),
                    };
                let object_type = self.llvm_type(&parameter.ty).into_struct_type();
                for name in names {
                    let index = fields
                        .iter()
                        .position(|field| field.name == *name)
                        .ok_or_else(|| {
                            format!("error: type `{type_name}` has no field `{name}`")
                        })?;
                    let field_type = self.resolve_type(&field_storage_type(&fields[index]));
                    let binding = self
                        .builder
                        .build_struct_gep(object_type, slot, index as u32, name)
                        .map_err(|error| error.to_string())?;
                    self.store_var(name, binding, field_type);
                }
            }
        }
        Ok(())
    }

    pub(super) fn async_frame_object(&self, ty: &Type) -> bool {
        self.async_object_fields(ty).is_some_and(|fields| {
            fields.iter().all(|field| {
                let field_type = self.resolve_type(&field_storage_type(field));
                self.is_copy_for_async(&field_type) || self.async_owned_value_supported(&field_type)
            })
        })
    }

    pub(super) fn emit_async_frame_drop(
        &mut self,
        name: &str,
        frame_type: inkwell::types::StructType<'ctx>,
        frame_params: &[AsyncFrameParam],
        frame_locals: &[(String, Type, AsyncLocalStorage)],
        expression_temporaries: &[(Span, Type)],
        for_of_field_count: usize,
    ) -> FunctionValue<'ctx> {
        let continuation_field_count = expression_temporaries.len() + for_of_field_count;
        let capture_param_count = self.async_capture_param_count;
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let drop = self.module.add_function(
            &format!("__tyrus_async_drop_{name}"),
            self.context.void_type().fn_type(&[pointer.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(drop, "entry");
        self.builder.position_at_end(entry);
        let frame = drop.get_first_param().unwrap().into_pointer_value();
        let layout = AsyncFrameDrop {
            drop,
            frame_type,
            frame,
            frame_params,
            frame_locals,
            expression_temporaries,
            continuation_field_count,
            capture_param_count,
            pointer,
        };
        self.emit_async_frame_drop_promises(layout);
        self.emit_async_frame_drop_locals(layout);
        self.emit_async_frame_drop_params(layout);
        let free = self.declare_libc_fn("free", &[pointer.into()], None);
        self.builder.build_call(free, &[frame.into()], "").unwrap();
        self.builder.build_return(None).unwrap();
        drop
    }
}
