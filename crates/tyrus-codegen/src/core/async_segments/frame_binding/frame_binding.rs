use super::*;

/// The frame pointer and the slot maps that the state machine reads from it.
pub(super) struct AsyncFrameStorage<'ctx> {
    pub(super) frame: PointerValue<'ctx>,
    pub(super) async_expression_slots: HashMap<Span, PointerValue<'ctx>>,
    pub(super) async_for_of_slots:
        HashMap<AsyncForOfSlotKey, (PointerValue<'ctx>, PointerValue<'ctx>, PointerValue<'ctx>)>,
}

impl<'ctx> Codegen<'ctx> {
    /// Binds the parameters and locals that live in the async frame and pushes
    /// the ownership bookkeeping the body emission pops again at the end.
    pub(super) fn bind_async_frame_storage(
        &mut self,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame_params: &[AsyncFrameParam],
        frame_locals: &[(String, Type, AsyncLocalStorage)],
        expression_temporaries: &[(Span, Type)],
        for_of_awaits: &[AsyncForOfSlotKey],
    ) -> Result<AsyncFrameStorage<'ctx>, String> {
        let frame = poll.get_nth_param(0).unwrap().into_pointer_value();
        for (index, parameter) in frame_params.iter().enumerate() {
            let slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    (index + 2) as u32,
                    &format!("async.frame.param.{}", parameter.storage_name),
                )
                .unwrap();
            self.bind_async_frame_parameter(parameter, slot)?;
        }
        let mut frame_slots = HashMap::default();
        for (index, (local_name, local_type, _)) in frame_locals.iter().enumerate() {
            let slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    (index + frame_params.len() + 2) as u32,
                    &format!("async.frame.local.{local_name}"),
                )
                .unwrap();
            frame_slots.insert(local_name.clone(), (slot, local_type.clone()));
        }
        self.async_frame_slots.push(frame_slots);
        let async_expression_slots = expression_temporaries
            .iter()
            .enumerate()
            .map(|(index, (span, _))| {
                let slot = self
                    .builder
                    .build_struct_gep(
                        frame_type,
                        frame,
                        (index + frame_params.len() + frame_locals.len() + 2) as u32,
                        "async.frame.await.value",
                    )
                    .unwrap();
                (*span, slot)
            })
            .collect::<HashMap<_, _>>();
        let for_of_slot_base =
            frame_params.len() + frame_locals.len() + expression_temporaries.len() + 2;
        let async_for_of_slots = for_of_awaits
            .iter()
            .enumerate()
            .map(|(offset, slot_key)| {
                let field = for_of_slot_base + offset * 3;
                let data = self
                    .builder
                    .build_struct_gep(frame_type, frame, field as u32, "async.for_of.data")
                    .unwrap();
                let length = self
                    .builder
                    .build_struct_gep(frame_type, frame, (field + 1) as u32, "async.for_of.length")
                    .unwrap();
                let index = self
                    .builder
                    .build_struct_gep(frame_type, frame, (field + 2) as u32, "async.for_of.index")
                    .unwrap();
                (*slot_key, (data, length, index))
            })
            .collect::<HashMap<_, _>>();
        let mut string_ownership = HashMap::default();
        let mut ownership_index = frame_params.len()
            + frame_locals.len()
            + expression_temporaries.len()
            + for_of_awaits.len() * 3
            + 2;
        for (local_name, _, storage) in frame_locals {
            if !matches!(storage, AsyncLocalStorage::TrackedString) {
                continue;
            }
            let slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    ownership_index as u32,
                    &format!("async.frame.local.{local_name}.owned"),
                )
                .unwrap();
            string_ownership.insert(local_name.clone(), slot);
            ownership_index += 1;
        }
        self.async_frame_string_ownership.push(string_ownership);
        let mut string_array_ownership = HashMap::default();
        for (local_name, _, storage) in frame_locals {
            if !matches!(storage, AsyncLocalStorage::TrackedStringArray) {
                continue;
            }
            let slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    ownership_index as u32,
                    &format!("async.frame.local.{local_name}.array.ownership"),
                )
                .unwrap();
            string_array_ownership.insert(local_name.clone(), slot);
            ownership_index += 1;
        }
        self.async_frame_string_array_ownership
            .push(string_array_ownership);
        let mut owned_initialization = HashMap::default();
        for (local_name, _, storage) in frame_locals {
            if !matches!(storage, AsyncLocalStorage::OwnedValue) {
                continue;
            }
            let slot = self
                .builder
                .build_struct_gep(
                    frame_type,
                    frame,
                    ownership_index as u32,
                    &format!("async.frame.local.{local_name}.initialized"),
                )
                .unwrap();
            owned_initialization.insert(local_name.clone(), slot);
            ownership_index += 1;
        }
        self.async_frame_owned_initialization
            .push(owned_initialization);
        self.async_frame_tracked_copy_arrays.push(
            frame_locals
                .iter()
                .filter(|(_, _, storage)| matches!(storage, AsyncLocalStorage::TrackedCopyArray))
                .map(|(name, _, _)| name.clone())
                .collect(),
        );
        self.async_frame_owned_values.push(
            frame_locals
                .iter()
                .filter(|(_, _, storage)| matches!(storage, AsyncLocalStorage::OwnedValue))
                .map(|(name, _, _)| name.clone())
                .collect(),
        );
        self.async_frame_local_storage.push(
            frame_locals
                .iter()
                .map(|(name, _, storage)| (name.clone(), *storage))
                .collect(),
        );
        Ok(AsyncFrameStorage {
            frame,
            async_expression_slots,
            async_for_of_slots,
        })
    }
}
