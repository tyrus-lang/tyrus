use super::*;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn emit_async_clone_value_into(
        &mut self,
        destination: PointerValue<'ctx>,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
    ) -> Result<(), String> {
        let ty = self.resolve_type(ty);
        let result_union = match transparent_codegen_type(&ty) {
            Type::Result { success, error } => Some(Type::Union(vec![
                success.as_ref().clone(),
                error.as_ref().clone(),
            ])),
            _ => None,
        };
        let clone_ty = result_union.as_ref().unwrap_or(&ty);
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        match transparent_codegen_type(clone_ty) {
            Type::Named(name) if name == "DigestStream" => {
                self.builder.build_store(destination, value).unwrap();
            }
            Type::Named(name) if name == "TcpStream" => {
                let retain = self.declare_libc_fn(
                    "tyrus_net_tcp_stream_retain",
                    &[pointer.into()],
                    Some(pointer.into()),
                );
                let retained = self
                    .builder
                    .build_call(retain, &[value.into()], "async.tcp_stream.retain")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.builder.build_store(destination, retained).unwrap();
            }
            Type::Named(name) if name == "TcpListener" => {
                let retain = self.declare_libc_fn(
                    "tyrus_net_tcp_listener_retain",
                    &[pointer.into()],
                    Some(pointer.into()),
                );
                let retained = self
                    .builder
                    .build_call(retain, &[value.into()], "async.tcp_listener.retain")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.builder.build_store(destination, retained).unwrap();
            }
            Type::Named(name) if name == "UdpSocket" => {
                self.builder.build_store(destination, value).unwrap();
            }
            Type::Apply { name, args } if name == "Sender" && args.len() == 1 => {
                let retain = self.declare_libc_fn(
                    "tyrus_channel_sender_retain",
                    &[pointer.into()],
                    Some(pointer.into()),
                );
                let retained = self
                    .builder
                    .build_call(
                        retain,
                        &[value.into()],
                        "async.closure.capture.sender.retain",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.builder.build_store(destination, retained).unwrap();
            }
            Type::Apply { name, args } if name == "Receiver" && args.len() == 1 => {
                let retain = self.declare_libc_fn(
                    "tyrus_channel_receiver_retain",
                    &[pointer.into()],
                    Some(pointer.into()),
                );
                let retained = self
                    .builder
                    .build_call(
                        retain,
                        &[value.into()],
                        "async.closure.capture.receiver.retain",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.builder.build_store(destination, retained).unwrap();
            }
            Type::Apply { name, args }
                if matches!(name.as_str(), "ReadableStream" | "WritableStream")
                    && args.len() == 1 =>
            {
                let (symbol, label) = if name == "ReadableStream" {
                    ("tyrus_readable_stream_retain", "readable")
                } else {
                    ("tyrus_writable_stream_retain", "writable")
                };
                let retain = self.declare_libc_fn(symbol, &[pointer.into()], Some(pointer.into()));
                let retained = self
                    .builder
                    .build_call(
                        retain,
                        &[value.into()],
                        &format!("async.stream.{label}.retain"),
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.builder.build_store(destination, retained).unwrap();
            }
            Type::Apply { name, args }
                if matches!(name.as_str(), "DuplexStream" | "TransformStream")
                    && args.len() == 2 =>
            {
                let (symbol, label) = if name == "DuplexStream" {
                    ("tyrus_duplex_stream_retain", "duplex")
                } else {
                    ("tyrus_transform_stream_retain", "transform")
                };
                let retain = self.declare_libc_fn(symbol, &[pointer.into()], Some(pointer.into()));
                let retained = self
                    .builder
                    .build_call(
                        retain,
                        &[value.into()],
                        &format!("async.stream.{label}.retain"),
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.builder.build_store(destination, retained).unwrap();
            }
            Type::Apply { name, args } if name == "Mutex" && args.len() == 1 => {
                let retain = self.declare_libc_fn(
                    "tyrus_mutex_retain",
                    &[pointer.into()],
                    Some(pointer.into()),
                );
                let retained = self
                    .builder
                    .build_call(
                        retain,
                        &[value.into()],
                        "async.closure.capture.mutex.retain",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.builder.build_store(destination, retained).unwrap();
            }
            Type::Apply { name, args } if name == "MutexGuard" && args.len() == 1 => {
                self.builder.build_store(destination, value).unwrap();
            }
            Type::Apply { name, args } if name == "RwLock" && args.len() == 1 => {
                let retain = self.declare_libc_fn(
                    "tyrus_rwlock_retain",
                    &[pointer.into()],
                    Some(pointer.into()),
                );
                let retained = self
                    .builder
                    .build_call(
                        retain,
                        &[value.into()],
                        "async.closure.capture.rwlock.retain",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.builder.build_store(destination, retained).unwrap();
            }
            Type::Apply { name, args }
                if matches!(name.as_str(), "RwReadGuard" | "RwWriteGuard") && args.len() == 1 =>
            {
                self.builder.build_store(destination, value).unwrap();
            }
            Type::Named(name) if name == "Semaphore" => {
                let retain = self.declare_libc_fn(
                    "tyrus_semaphore_retain",
                    &[pointer.into()],
                    Some(pointer.into()),
                );
                let retained = self
                    .builder
                    .build_call(
                        retain,
                        &[value.into()],
                        "async.closure.capture.semaphore.retain",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.builder.build_store(destination, retained).unwrap();
            }
            Type::Named(name) if name == "HttpServer" => {
                let retain = self.declare_libc_fn(
                    "tyrus_http_server_retain",
                    &[pointer.into()],
                    Some(pointer.into()),
                );
                let retained = self
                    .builder
                    .build_call(
                        retain,
                        &[value.into()],
                        "async.closure.capture.http_server.retain",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.builder.build_store(destination, retained).unwrap();
            }
            Type::Named(name) if name == "ChildProcess" => {
                let retain = self.declare_libc_fn(
                    "tyrus_child_process_retain",
                    &[pointer.into()],
                    Some(pointer.into()),
                );
                let retained = self
                    .builder
                    .build_call(
                        retain,
                        &[value.into()],
                        "async.closure.capture.child_process.retain",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.builder.build_store(destination, retained).unwrap();
            }
            Type::Named(name) if name == "WebSocket" => {
                let retain = self.declare_libc_fn(
                    "tyrus_websocket_retain",
                    &[pointer.into()],
                    Some(pointer.into()),
                );
                let retained = self
                    .builder
                    .build_call(
                        retain,
                        &[value.into()],
                        "async.closure.capture.websocket.retain",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.builder.build_store(destination, retained).unwrap();
            }
            Type::Named(name) if name == "SemaphorePermit" => {
                self.builder.build_store(destination, value).unwrap();
            }
            Type::Lit(expr) => {
                let underlying = self.lit_underlying_type(expr);
                self.emit_async_clone_value_into(destination, value, &underlying)?;
            }
            Type::Str => {
                let cloned: BasicValueEnum<'ctx> = self
                    .emit_string_clone(value.into_struct_value(), "async.payload.string.clone")?;
                self.builder.build_store(destination, cloned).unwrap();
            }
            Type::Array(element) if matches!(transparent_codegen_type(element), Type::Str) => {
                let source = self
                    .builder
                    .build_alloca(self.llvm_type(&ty), "async.payload.string.array.source")
                    .unwrap();
                self.builder.build_store(source, value).unwrap();
                let clone = self.declare_libc_fn(
                    "tyrus_array_clone_strings",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.i32_type().into()),
                );
                let status = self
                    .builder
                    .build_call(
                        clone,
                        &[source.into(), destination.into()],
                        "async.payload.string.array.clone",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                self.emit_abort_if_negative(status, "async payload", "async.payload.string.array")?;
            }
            Type::Array(element) => {
                let source = self
                    .builder
                    .build_alloca(self.llvm_type(&ty), "async.payload.owned.array.source")
                    .unwrap();
                self.builder.build_store(source, value).unwrap();
                let clone_element = self.emit_async_array_element_clone(element)?;
                let element_size = self.llvm_type(element).size_of().ok_or_else(|| {
                    format!("error: async array element `{element}` must be sized")
                })?;
                let clone = self.declare_libc_fn(
                    "tyrus_array_clone_owned",
                    &[
                        pointer.into(),
                        pointer.into(),
                        self.context.i64_type().into(),
                        pointer.into(),
                    ],
                    Some(self.context.i32_type().into()),
                );
                let status = self
                    .builder
                    .build_call(
                        clone,
                        &[
                            source.into(),
                            destination.into(),
                            element_size.into(),
                            clone_element.as_global_value().as_pointer_value().into(),
                        ],
                        "async.payload.owned.array.clone",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                self.emit_abort_if_negative(status, "async payload", "async.payload.owned.array")?;
            }
            Type::Optional(inner) => {
                let source = value.into_pointer_value();
                if source.is_null() {
                    self.builder.build_store(destination, source).unwrap();
                    return Ok(());
                }
                let function = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let clone = self
                    .context
                    .append_basic_block(function, "async.optional.clone");
                let empty = self
                    .context
                    .append_basic_block(function, "async.optional.empty");
                let merge = self
                    .context
                    .append_basic_block(function, "async.optional.merge");
                let present = self
                    .builder
                    .build_is_not_null(source, "async.optional.present")
                    .unwrap();
                self.builder
                    .build_conditional_branch(present, clone, empty)
                    .unwrap();
                self.builder.position_at_end(clone);
                let cloned = self.heap_alloc(self.llvm_type(inner), "async.optional.value")?;
                let loaded = self
                    .builder
                    .build_load(self.llvm_type(inner), source, "async.optional.source")
                    .unwrap();
                self.emit_async_clone_value_into(cloned, loaded, inner)?;
                let cloned_block = self.builder.get_insert_block().unwrap();
                self.builder.build_unconditional_branch(merge).unwrap();
                self.builder.position_at_end(empty);
                self.builder.build_unconditional_branch(merge).unwrap();
                self.builder.position_at_end(merge);
                let result = self
                    .builder
                    .build_phi(pointer, "async.optional.result")
                    .unwrap();
                result.add_incoming(&[(&cloned, cloned_block), (&pointer.const_null(), empty)]);
                self.builder
                    .build_store(destination, result.as_basic_value())
                    .unwrap();
            }
            Type::Union(variants) => {
                let function = self
                    .builder
                    .get_insert_block()
                    .and_then(inkwell::basic_block::BasicBlock::get_parent)
                    .ok_or("internal: union clone outside a function")?;
                let source = self
                    .builder
                    .build_alloca(self.llvm_type(&ty), "async.union.source")
                    .unwrap();
                self.builder.build_store(source, value).unwrap();
                let tag = self.load_union_tag(variants, source)?;
                let union_type = self.union_llvm_type(variants);
                let destination_tag = self
                    .builder
                    .build_struct_gep(union_type, destination, 0, "async.union.destination.tag")
                    .map_err(|error| error.to_string())?;
                self.builder.build_store(destination_tag, tag).unwrap();
                if Self::union_is_payload_free(variants) {
                    return Ok(());
                }
                let merge = self
                    .context
                    .append_basic_block(function, "async.union.clone.merge");
                let mut next = self.builder.get_insert_block().unwrap();
                for (index, variant) in variants.iter().enumerate() {
                    let clone = self
                        .context
                        .append_basic_block(function, "async.union.clone.variant");
                    let otherwise = if index + 1 == variants.len() {
                        merge
                    } else {
                        self.context
                            .append_basic_block(function, "async.union.clone.next")
                    };
                    self.builder.position_at_end(next);
                    let is_variant = self
                        .builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            tag,
                            self.context.i8_type().const_int(index as u64, false),
                            "async.union.clone.matches",
                        )
                        .unwrap();
                    self.builder
                        .build_conditional_branch(is_variant, clone, otherwise)
                        .unwrap();
                    self.builder.position_at_end(clone);
                    if !matches!(variant, Type::Void) {
                        let source_value = self.load_union_payload(
                            variants,
                            source,
                            variant,
                            "async.union.clone.source",
                        )?;
                        let destination_value = self.union_payload_slot(
                            variants,
                            destination,
                            variant,
                            "async.union.clone.destination",
                        )?;
                        self.emit_async_clone_value_into(destination_value, source_value, variant)?;
                    }
                    if !self.current_block_terminated() {
                        self.builder.build_unconditional_branch(merge).unwrap();
                    }
                    next = otherwise;
                }
                self.builder.position_at_end(merge);
            }
            Type::Named(name) if tyrus_core::native_handles::is_opaque_handle_name(name) => {
                self.builder
                    .build_store(destination, value)
                    .map_err(|error| error.to_string())?;
            }
            Type::Named(_) | Type::Object(_) => {
                let fields = self.async_object_fields(&ty).ok_or_else(|| {
                    format!("internal: async object payload `{ty}` has no fields")
                })?;
                let object_type = self.llvm_type(&ty).into_struct_type();
                let object = value.into_struct_value();
                for (index, field) in fields.iter().enumerate() {
                    let field_type = self.resolve_type(&field_storage_type(field));
                    let field_value = self
                        .builder
                        .build_extract_value(
                            object,
                            index as u32,
                            &format!("async.payload.{}.value", field.name),
                        )
                        .map_err(|error| error.to_string())?;
                    let field_destination = self
                        .builder
                        .build_struct_gep(
                            object_type,
                            destination,
                            index as u32,
                            &format!("async.payload.{}.slot", field.name),
                        )
                        .map_err(|error| error.to_string())?;
                    self.emit_async_clone_value_into(field_destination, field_value, &field_type)?;
                }
            }
            Type::Tuple(elements) => {
                let tuple_type = self.llvm_type(&ty).into_struct_type();
                let tuple = value.into_struct_value();
                for (index, element_type) in elements.iter().enumerate() {
                    let element_type = self.resolve_type(element_type);
                    let element_value = self
                        .builder
                        .build_extract_value(
                            tuple,
                            index as u32,
                            &format!("async.payload.tuple.{index}.value"),
                        )
                        .map_err(|error| error.to_string())?;
                    let element_destination = self
                        .builder
                        .build_struct_gep(
                            tuple_type,
                            destination,
                            index as u32,
                            &format!("async.payload.tuple.{index}.slot"),
                        )
                        .map_err(|error| error.to_string())?;
                    self.emit_async_clone_value_into(
                        element_destination,
                        element_value,
                        &element_type,
                    )?;
                }
            }
            _ => {
                self.builder.build_store(destination, value).unwrap();
            }
        }
        Ok(())
    }

    pub(super) fn emit_async_array_element_clone(
        &mut self,
        element: &Type,
    ) -> Result<FunctionValue<'ctx>, String> {
        let element = self.resolve_type(element);
        let symbol = format!("__tyrus_clone_async_element_{}", mangle_type(&element));
        if let Some(function) = self.module.get_function(&symbol) {
            return Ok(function);
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let function = self.module.add_function(
            &symbol,
            self.context
                .void_type()
                .fn_type(&[pointer.into(), pointer.into()], false),
            None,
        );
        let outer = self.builder.get_insert_block();
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);
        let destination = function.get_nth_param(0).unwrap().into_pointer_value();
        let source = function.get_nth_param(1).unwrap().into_pointer_value();
        let value = self
            .builder
            .build_load(self.llvm_type(&element), source, "async.element.source")
            .unwrap();
        self.emit_async_clone_value_into(destination, value, &element)?;
        self.builder.build_return(None).unwrap();
        if let Some(outer) = outer {
            self.builder.position_at_end(outer);
        }
        Ok(function)
    }

    pub(super) fn emit_async_array_element_drop(
        &mut self,
        element: &Type,
    ) -> Result<FunctionValue<'ctx>, String> {
        let element = self.resolve_type(element);
        let symbol = format!("__tyrus_drop_async_element_{}", mangle_type(&element));
        if let Some(function) = self.module.get_function(&symbol) {
            return Ok(function);
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let function = self.module.add_function(
            &symbol,
            self.context.void_type().fn_type(&[pointer.into()], false),
            None,
        );
        let outer = self.builder.get_insert_block();
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);
        let slot = function.get_first_param().unwrap().into_pointer_value();
        self.emit_async_drop_value_in_place(function, slot, &element)?;
        if !self.current_block_terminated() {
            self.builder.build_return(None).unwrap();
        }
        if let Some(outer) = outer {
            self.builder.position_at_end(outer);
        }
        Ok(function)
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn emit_async_cleanup_source_expression(
        &mut self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
        expression: &Expr,
    ) -> Result<(), String> {
        if matches!(expression.kind(), ExprKind::Null | ExprKind::None) {
            return Ok(());
        }
        let ty = self.resolve_type(ty);
        if self.owned_http_server_expression(expression)
            && matches!(
                transparent_codegen_type(&ty),
                Type::Optional(inner)
                    if matches!(
                        transparent_codegen_type(inner),
                        Type::Named(name) if name == "HttpServer"
                    )
            )
        {
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let function = self
                .builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_parent)
                .ok_or("internal: optional server cleanup outside function")?;
            let box_pointer = value.into_pointer_value();
            let release = self
                .context
                .append_basic_block(function, "async.source.http_server.release");
            let next = self
                .context
                .append_basic_block(function, "async.source.http_server.next");
            let present = self
                .builder
                .build_is_not_null(box_pointer, "async.source.http_server.present")
                .unwrap();
            self.builder
                .build_conditional_branch(present, release, next)
                .unwrap();
            self.builder.position_at_end(release);
            let server = self
                .builder
                .build_load(pointer, box_pointer, "async.source.http_server")
                .map_err(|error| error.to_string())?
                .into_pointer_value();
            let free = self.declare_libc_fn("tyrus_http_server_free", &[pointer.into()], None);
            self.builder.build_call(free, &[server.into()], "").unwrap();
            self.builder.build_unconditional_branch(next).unwrap();
            self.builder.position_at_end(next);
        }
        if let Some(plan) = self.take_owned_native_result(value) {
            self.emit_owned_native_release(value, &ty, &plan);
            return Ok(());
        }
        if async_ref_counted_native_handle(&ty)
            && let ExprKind::Ident(_) = expression.kind()
        {
            return Ok(());
        }
        let removed_direct = false;
        if !removed_direct
            && (self.owned_user_function_call(expression)
                || self.async_owned_string_expression(expression)
                || self.owned_algorithm_method_call(expression))
            && let Some(index) = self
                .pending_owned_native_results
                .iter()
                .rposition(|(_, pending_type, _)| self.resolve_type(pending_type) == ty)
        {
            self.pending_owned_native_results.remove(index);
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let transfers_native_handle = match transparent_codegen_type(&ty) {
            Type::Apply { name, args }
                if args.len() == 1
                    && matches!(
                        name.as_str(),
                        "Sender"
                            | "Receiver"
                            | "Mutex"
                            | "MutexGuard"
                            | "RwLock"
                            | "RwReadGuard"
                            | "RwWriteGuard"
                    ) =>
            {
                true
            }
            Type::Named(name)
                if matches!(
                    name.as_str(),
                    "DigestStream"
                        | "TcpStream"
                        | "TcpListener"
                        | "UdpSocket"
                        | "HttpClientResponse"
                        | "HttpClientStreamResponse"
                        | "Semaphore"
                        | "SemaphorePermit"
                        | "WebSocket"
                ) =>
            {
                true
            }
            _ => false,
        };
        if transfers_native_handle && let ExprKind::Ident(name) = expression.kind() {
            if self.async_frame_owns_value(name) {
                self.mark_async_frame_value_initialized(name, false);
            } else if !self.async_results.is_empty() {
                let retain = match transparent_codegen_type(&ty) {
                    Type::Apply { name, args }
                        if args.len() == 1
                            || (args.len() == 2
                                && matches!(name.as_str(), "DuplexStream" | "TransformStream")) =>
                    {
                        match name.as_str() {
                            "Sender" => Some("tyrus_channel_sender_retain"),
                            "Receiver" => Some("tyrus_channel_receiver_retain"),
                            "ReadableStream" => Some("tyrus_readable_stream_retain"),
                            "WritableStream" => Some("tyrus_writable_stream_retain"),
                            "DuplexStream" => Some("tyrus_duplex_stream_retain"),
                            "TransformStream" => Some("tyrus_transform_stream_retain"),
                            "Mutex" => Some("tyrus_mutex_retain"),
                            "RwLock" => Some("tyrus_rwlock_retain"),
                            _ => None,
                        }
                    }
                    Type::Named(name) if name == "Semaphore" => Some("tyrus_semaphore_retain"),
                    Type::Named(name) if name == "ChildProcess" => {
                        Some("tyrus_child_process_retain")
                    }
                    Type::Named(name) if name == "WebSocket" => Some("tyrus_websocket_retain"),
                    _ => None,
                };
                if let Some(retain) = retain {
                    let retain =
                        self.declare_libc_fn(retain, &[pointer.into()], Some(pointer.into()));
                    self.builder
                        .build_call(retain, &[value.into_pointer_value().into()], "")
                        .unwrap();
                }
            } else {
                self.dispose_owned_native_binding(name);
            }
            return Ok(());
        }
        match transparent_codegen_type(&ty) {
            // The receiver of a stream takes a reference of its own, so a
            // stream built for the call, such as `readFileStream(...)` passed
            // straight to an async function, is released here or never.
            Type::Apply { name, args }
                if matches!(
                    (name.as_str(), args.len()),
                    ("ReadableStream" | "WritableStream", 1)
                        | ("DuplexStream" | "TransformStream", 2)
                ) && self.owned_stream_capability_expression(expression) =>
            {
                let release = match name.as_str() {
                    "ReadableStream" => "tyrus_readable_stream_free",
                    "WritableStream" => "tyrus_writable_stream_free",
                    "DuplexStream" => "tyrus_duplex_stream_free",
                    _ => "tyrus_transform_stream_free",
                };
                let free = self.declare_libc_fn(release, &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into_pointer_value().into()], "")
                    .unwrap();
            }
            Type::Apply { name, args }
                if (name == "Promise" || name == "DbPromise" || name == "JoinHandle")
                    && args.len() == 1
                    && (self.owned_promise_expression(expression)
                        || self.owned_join_handle_expression(expression)) =>
            {
                let free = self.declare_libc_fn("tyrus_promise_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into_pointer_value().into()], "")
                    .unwrap();
            }
            Type::Apply { name, args }
                if name == "Mutex"
                    && args.len() == 1
                    && self.owned_mutex_expression(expression) =>
            {
                let free = self.declare_libc_fn("tyrus_mutex_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into_pointer_value().into()], "")
                    .unwrap();
            }
            Type::Apply { name, args }
                if name == "RwLock"
                    && args.len() == 1
                    && self.owned_rwlock_expression(expression) =>
            {
                let free = self.declare_libc_fn("tyrus_rwlock_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into_pointer_value().into()], "")
                    .unwrap();
            }
            Type::Named(name)
                if name == "Semaphore" && self.owned_semaphore_expression(expression) =>
            {
                let free = self.declare_libc_fn("tyrus_semaphore_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into_pointer_value().into()], "")
                    .unwrap();
            }
            Type::Named(name)
                if name == "HttpServer" && self.owned_http_server_expression(expression) =>
            {
                let free = self.declare_libc_fn("tyrus_http_server_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into_pointer_value().into()], "")
                    .unwrap();
            }
            Type::Str if removed_direct || self.async_owned_string_expression(expression) => {
                self.emit_string_release(value.into_struct_value())
                    .expect("releasing an async temporary is always emittable");
            }
            // A native crypto call returning a claims aggregate
            // (`jwt.verify(...)`) is neither a user function nor an array
            // method, so neither predicate above recognizes it. It also never
            // leaves a pending registration to identity-match: inside an async
            // function `record_owned_native_result` is a no-op by design,
            // because a value *moved* into the frame must not be freed at its
            // source. This aggregate is not moved, `emit_async_clone_value_
            // into` `strdup`s each string field, so the original buffers from
            // `tyrus_json_document_get_string` leak unless freed right here.
            _ if (self.owned_user_function_call(expression)
                || self.owned_array_method_call(expression)
                || self.owned_crypto_claims_call(expression))
                && self.async_owned_value_supported(&ty) =>
            {
                self.emit_owned_native_value_free(value, &ty);
            }
            Type::Optional(_) if self.async_owned_array_removal(expression) => {
                self.emit_owned_native_value_free(value, &ty);
            }
            Type::Optional(_) if self.owned_channel_receive_expression(expression) => {
                self.emit_owned_native_value_free(value, &ty);
            }
            // `tryLock`/`tryRead`/`tryWrite`/`tryAcquire` return a fresh
            // optional box around a unique guard. The frame clone allocates a
            // new box and plain-stores that unique pointer into it. Free only
            // the now-empty source container: deep-dropping it would release
            // the guard that the destination frame has just taken ownership
            // of, while leaving it allocated leaks one box per acquisition.
            Type::Optional(_) if self.owned_optional_sync_guard_expression(expression) => {
                let free = self.declare_libc_fn("free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into_pointer_value().into()], "")
                    .unwrap();
            }
            Type::Optional(inner)
                if !matches!(
                    self.expr_type_hint(expression)
                        .map(|source| self.resolve_type(&source)),
                    Some(Type::Optional(_))
                ) =>
            {
                let source = value.into_pointer_value();
                let function = self
                    .builder
                    .get_insert_block()
                    .and_then(inkwell::basic_block::BasicBlock::get_parent)
                    .ok_or("internal: optional argument cleanup outside function")?;
                let release = self
                    .context
                    .append_basic_block(function, "async.argument.optional.release");
                let next = self
                    .context
                    .append_basic_block(function, "async.argument.optional.next");
                let present = self
                    .builder
                    .build_is_not_null(source, "async.argument.optional.present")
                    .unwrap();
                self.builder
                    .build_conditional_branch(present, release, next)
                    .unwrap();
                self.builder.position_at_end(release);
                let inner_type = self.resolve_type(inner);
                let inner_value = self
                    .builder
                    .build_load(
                        self.llvm_type(&inner_type),
                        source,
                        "async.argument.optional.inner",
                    )
                    .unwrap();
                self.emit_async_cleanup_source_expression(inner_value, &inner_type, expression)?;
                let free = self.declare_libc_fn("free", &[pointer.into()], None);
                self.builder.build_call(free, &[source.into()], "").unwrap();
                if !self.current_block_terminated() {
                    self.builder.build_unconditional_branch(next).unwrap();
                }
                self.builder.position_at_end(next);
            }
            Type::Array(element) if element.is_copy() => {
                if removed_direct || self.async_owned_copy_array_expression(expression) {
                    let data = self
                        .builder
                        .build_extract_value(
                            value.into_struct_value(),
                            0,
                            "async.return.source.array.data",
                        )
                        .unwrap();
                    let free = self.declare_libc_fn("free", &[pointer.into()], None);
                    self.builder.build_call(free, &[data.into()], "").unwrap();
                }
            }
            Type::Array(element) if matches!(transparent_codegen_type(element), Type::Str) => {
                if let Some(mode) = self.async_string_array_ownership_mode(expression) {
                    let slot = self
                        .builder
                        .build_alloca(self.llvm_type(&ty), "async.return.source.string.array")
                        .unwrap();
                    self.builder.build_store(slot, value).unwrap();
                    if mode == 1 {
                        let array = value.into_struct_value();
                        let data = self
                            .builder
                            .build_extract_value(array, 0, "async.return.source.array.data")
                            .unwrap();
                        let free = self.declare_libc_fn("free", &[pointer.into()], None);
                        self.builder.build_call(free, &[data.into()], "").unwrap();
                    } else {
                        let drop_contents = self.declare_libc_fn(
                            "tyrus_array_drop_string_contents",
                            &[pointer.into()],
                            None,
                        );
                        self.builder
                            .build_call(drop_contents, &[slot.into()], "")
                            .unwrap();
                    }
                }
            }
            Type::Array(element) => {
                let ExprKind::ArrayLit(elements) = expression.kind() else {
                    return Ok(());
                };
                let array = value.into_struct_value();
                let data = self
                    .builder
                    .build_extract_value(array, 0, "async.return.source.array.data")
                    .unwrap()
                    .into_pointer_value();
                let element_type = self.resolve_type(element);
                let llvm_element = self.llvm_type(&element_type);
                for (index, element_expression) in elements.iter().enumerate() {
                    let element_pointer = unsafe {
                        self.builder
                            .build_gep(
                                llvm_element,
                                data,
                                &[self.context.i64_type().const_int(index as u64, false)],
                                "async.return.source.array.element",
                            )
                            .unwrap()
                    };
                    let element_value = self
                        .builder
                        .build_load(
                            llvm_element,
                            element_pointer,
                            "async.return.source.array.element.value",
                        )
                        .unwrap();
                    self.emit_async_cleanup_source_expression(
                        element_value,
                        &element_type,
                        element_expression,
                    )?;
                }
                let free = self.declare_libc_fn("free", &[pointer.into()], None);
                self.builder.build_call(free, &[data.into()], "").unwrap();
            }
            Type::Union(variants) if matches!(expression.kind(), ExprKind::ObjectLit(_)) => {
                let concrete = self.union_variant_for_expr(expression, variants);
                let source = self
                    .builder
                    .build_alloca(self.llvm_type(&ty), "union.return.source")
                    .unwrap();
                self.builder.build_store(source, value).unwrap();
                let payload = self.load_union_payload(
                    variants,
                    source,
                    &concrete,
                    "union.return.source.payload",
                )?;
                if let Some(index) =
                    self.pending_owned_native_results
                        .iter()
                        .rposition(|(_, pending_type, _)| {
                            self.resolve_type(pending_type) == self.resolve_type(&concrete)
                        })
                {
                    let (_, _, plan) = self.pending_owned_native_results.remove(index);
                    self.emit_owned_native_release(payload, &concrete, &plan);
                }
            }
            Type::Result { .. }
                if matches!(
                    expression.kind(),
                    ExprKind::FnCall { callee, args, .. }
                        if matches!(callee.as_str(), "Ok" | "Err")
                            && args.first().is_some_and(|argument| {
                                !tyrus_core::ast::is_borrowed_place(argument)
                            })
                ) =>
            {
                let source = self
                    .builder
                    .build_alloca(self.llvm_type(&ty), "async.return.source.result")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(source, value)
                    .map_err(|error| error.to_string())?;
                let function = self
                    .builder
                    .get_insert_block()
                    .and_then(inkwell::basic_block::BasicBlock::get_parent)
                    .ok_or("internal: async Result cleanup outside function")?;
                self.emit_async_drop_value_in_place(function, source, &ty)?;
            }
            Type::Named(_) | Type::Object(_) => {
                let values = match expression.kind() {
                    ExprKind::ObjectLit(values) => values,
                    ExprKind::ObjectUpdate { fields, .. } => fields,
                    _ => return Ok(()),
                };
                let fields = self
                    .async_object_fields(&ty)
                    .ok_or_else(|| format!("internal: async return object `{ty}` has no fields"))?;
                let object = value.into_struct_value();
                for (index, field) in fields.iter().enumerate() {
                    let Some((_, field_expression)) =
                        values.iter().find(|(name, _)| name == &field.name)
                    else {
                        continue;
                    };
                    let field_type = self.resolve_type(&field_storage_type(field));
                    if field_type.is_copy() {
                        continue;
                    }
                    let field_value = self
                        .builder
                        .build_extract_value(
                            object,
                            index as u32,
                            &format!("async.return.source.{}", field.name),
                        )
                        .unwrap();
                    self.emit_async_cleanup_source_expression(
                        field_value,
                        &field_type,
                        field_expression,
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn emit_async_object_payload_drop(
        &mut self,
        ty: &Type,
    ) -> Result<FunctionValue<'ctx>, String> {
        let symbol = format!(
            "__tyrus_drop_async_payload_{}",
            mangle_type(&self.resolve_type(ty))
        );
        if let Some(function) = self.module.get_function(&symbol) {
            return Ok(function);
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let function = self.module.add_function(
            &symbol,
            self.context.void_type().fn_type(&[pointer.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(function, "entry");
        let outer_block = self.builder.get_insert_block();
        self.builder.position_at_end(entry);
        let payload = function.get_first_param().unwrap().into_pointer_value();
        self.emit_async_drop_value_in_place(function, payload, ty)?;
        let free = self.declare_libc_fn("free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[payload.into()], "")
            .unwrap();
        self.builder.build_return(None).unwrap();
        if let Some(block) = outer_block {
            self.builder.position_at_end(block);
        }
        Ok(function)
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn emit_async_drop_value_in_place(
        &mut self,
        function: FunctionValue<'ctx>,
        slot: PointerValue<'ctx>,
        ty: &Type,
    ) -> Result<(), String> {
        let ty = self.resolve_type(ty);
        let result_union = match transparent_codegen_type(&ty) {
            Type::Result { success, error } => Some(Type::Union(vec![
                success.as_ref().clone(),
                error.as_ref().clone(),
            ])),
            _ => None,
        };
        let drop_ty = result_union.as_ref().unwrap_or(&ty);
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        match transparent_codegen_type(drop_ty) {
            Type::Named(name) if name == "DigestStream" => {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.digest.stream")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let release = self
                    .context
                    .append_basic_block(function, "drop.digest.stream");
                let next = self
                    .context
                    .append_basic_block(function, "drop.digest.stream.next");
                let present = self
                    .builder
                    .build_is_not_null(value, "async.payload.drop.digest.stream.present")
                    .unwrap();
                self.builder
                    .build_conditional_branch(present, release, next)
                    .unwrap();
                self.builder.position_at_end(release);
                let free = self.declare_libc_fn(
                    "tyrus_crypto_digest_stream_free",
                    &[pointer.into()],
                    None,
                );
                self.builder.build_call(free, &[value.into()], "").unwrap();
                self.builder.build_unconditional_branch(next).unwrap();
                self.builder.position_at_end(next);
            }
            Type::Named(name) if name == "TcpStream" => {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.tcp.stream")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let release = self.context.append_basic_block(function, "drop.tcp.stream");
                let next = self
                    .context
                    .append_basic_block(function, "drop.tcp.stream.next");
                let present = self
                    .builder
                    .build_is_not_null(value, "async.payload.drop.tcp.stream.present")
                    .unwrap();
                self.builder
                    .build_conditional_branch(present, release, next)
                    .unwrap();
                self.builder.position_at_end(release);
                let free =
                    self.declare_libc_fn("tyrus_net_tcp_stream_free", &[pointer.into()], None);
                self.builder.build_call(free, &[value.into()], "").unwrap();
                self.builder.build_unconditional_branch(next).unwrap();
                self.builder.position_at_end(next);
            }
            Type::Apply { name, args } if name == "Sender" && args.len() == 1 => {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.sender")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let free =
                    self.declare_libc_fn("tyrus_channel_sender_free", &[pointer.into()], None);
                self.builder.build_call(free, &[value.into()], "").unwrap();
            }
            Type::Apply { name, args } if name == "Receiver" && args.len() == 1 => {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.receiver")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let free =
                    self.declare_libc_fn("tyrus_channel_receiver_free", &[pointer.into()], None);
                self.builder.build_call(free, &[value.into()], "").unwrap();
            }
            Type::Apply { name, args }
                if matches!(name.as_str(), "ReadableStream" | "WritableStream")
                    && args.len() == 1 =>
            {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.stream")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let symbol = if name == "ReadableStream" {
                    "tyrus_readable_stream_free"
                } else {
                    "tyrus_writable_stream_free"
                };
                let free = self.declare_libc_fn(symbol, &[pointer.into()], None);
                self.builder.build_call(free, &[value.into()], "").unwrap();
            }
            Type::Apply { name, args }
                if matches!(name.as_str(), "DuplexStream" | "TransformStream")
                    && args.len() == 2 =>
            {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.composite_stream")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let symbol = if name == "DuplexStream" {
                    "tyrus_duplex_stream_free"
                } else {
                    "tyrus_transform_stream_free"
                };
                let free = self.declare_libc_fn(symbol, &[pointer.into()], None);
                self.builder.build_call(free, &[value.into()], "").unwrap();
            }
            Type::Apply { name, args } if name == "Mutex" && args.len() == 1 => {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.mutex")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let free = self.declare_libc_fn("tyrus_mutex_free", &[pointer.into()], None);
                self.builder.build_call(free, &[value.into()], "").unwrap();
            }
            Type::Apply { name, args } if name == "MutexGuard" && args.len() == 1 => {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.mutex_guard")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let unlock =
                    self.declare_libc_fn("tyrus_mutex_guard_unlock", &[pointer.into()], None);
                self.builder
                    .build_call(unlock, &[value.into()], "")
                    .unwrap();
            }
            Type::Apply { name, args } if name == "RwLock" && args.len() == 1 => {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.rwlock")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let free = self.declare_libc_fn("tyrus_rwlock_free", &[pointer.into()], None);
                self.builder.build_call(free, &[value.into()], "").unwrap();
            }
            Type::Apply { name, args } if name == "RwReadGuard" && args.len() == 1 => {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.rwlock_read_guard")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let unlock =
                    self.declare_libc_fn("tyrus_rwlock_read_guard_unlock", &[pointer.into()], None);
                self.builder
                    .build_call(unlock, &[value.into()], "")
                    .unwrap();
            }
            Type::Apply { name, args } if name == "RwWriteGuard" && args.len() == 1 => {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.rwlock_write_guard")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let unlock = self.declare_libc_fn(
                    "tyrus_rwlock_write_guard_unlock",
                    &[pointer.into()],
                    None,
                );
                self.builder
                    .build_call(unlock, &[value.into()], "")
                    .unwrap();
            }
            Type::Named(name) if name == "Semaphore" => {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.semaphore")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let free = self.declare_libc_fn("tyrus_semaphore_free", &[pointer.into()], None);
                self.builder.build_call(free, &[value.into()], "").unwrap();
            }
            Type::Named(name) if name == "HttpServer" => {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.http_server")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let free = self.declare_libc_fn("tyrus_http_server_free", &[pointer.into()], None);
                self.builder.build_call(free, &[value.into()], "").unwrap();
            }
            Type::Named(name) if name == "ChildProcess" => {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.child_process")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let free =
                    self.declare_libc_fn("tyrus_child_process_free", &[pointer.into()], None);
                self.builder.build_call(free, &[value.into()], "").unwrap();
            }
            Type::Named(name) if name == "WebSocket" => {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.websocket")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let free = self.declare_libc_fn("tyrus_websocket_free", &[pointer.into()], None);
                self.builder.build_call(free, &[value.into()], "").unwrap();
            }
            Type::Named(name)
                if matches!(
                    name.as_str(),
                    "HttpClientResponse" | "HttpClientStreamResponse"
                ) =>
            {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.http_response")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let free = self.declare_libc_fn(
                    if name == "HttpClientResponse" {
                        "tyrus_http_client_response_free"
                    } else {
                        "tyrus_http_client_stream_response_free"
                    },
                    &[pointer.into()],
                    None,
                );
                self.builder.build_call(free, &[value.into()], "").unwrap();
            }
            Type::Named(name) if name == "HttpRouter" => {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.http_router")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let free = self.declare_libc_fn("tyrus_http_router_free", &[pointer.into()], None);
                self.builder.build_call(free, &[value.into()], "").unwrap();
            }
            Type::Named(name) if name == "SemaphorePermit" => {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.semaphore_permit")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let release =
                    self.declare_libc_fn("tyrus_semaphore_permit_release", &[pointer.into()], None);
                self.builder
                    .build_call(release, &[value.into()], "")
                    .unwrap();
            }
            Type::Named(name) if name == "TcpListener" => {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.tcp.listener")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let release = self
                    .context
                    .append_basic_block(function, "drop.tcp.listener");
                let next = self
                    .context
                    .append_basic_block(function, "drop.tcp.listener.next");
                let present = self
                    .builder
                    .build_is_not_null(value, "async.payload.drop.tcp.listener.present")
                    .unwrap();
                self.builder
                    .build_conditional_branch(present, release, next)
                    .unwrap();
                self.builder.position_at_end(release);
                let free =
                    self.declare_libc_fn("tyrus_net_tcp_listener_free", &[pointer.into()], None);
                self.builder.build_call(free, &[value.into()], "").unwrap();
                self.builder.build_unconditional_branch(next).unwrap();
                self.builder.position_at_end(next);
            }
            Type::Named(name) if name == "UdpSocket" => {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.udp.socket")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let release = self.context.append_basic_block(function, "drop.udp.socket");
                let next = self
                    .context
                    .append_basic_block(function, "drop.udp.socket.next");
                let present = self
                    .builder
                    .build_is_not_null(value, "async.payload.drop.udp.socket.present")
                    .unwrap();
                self.builder
                    .build_conditional_branch(present, release, next)
                    .unwrap();
                self.builder.position_at_end(release);
                let free =
                    self.declare_libc_fn("tyrus_net_udp_socket_free", &[pointer.into()], None);
                self.builder.build_call(free, &[value.into()], "").unwrap();
                self.builder.build_unconditional_branch(next).unwrap();
                self.builder.position_at_end(next);
            }
            // A literal type is stored as whatever its literal is, which
            // `llvm_type` and `type_layout` have always resolved through
            // `lit_underlying_type` while the two walkers here did not. So a
            // `kind: "failed"` field was laid out as a `string`, decoded into an
            // allocated one, and then released by nobody: one string per
            // discriminated union read back from JSON. `async_owned_value_
            // supported` already answers `true` for `Type::Lit`, which is what
            // made the omission a leak rather than a refusal.
            Type::Lit(expr) => {
                let underlying = self.lit_underlying_type(expr);
                self.emit_async_drop_value_in_place(function, slot, &underlying)?;
            }
            Type::Str => {
                let value = self
                    .builder
                    .build_load(self.string_abi_type(), slot, "async.payload.drop.string")
                    .map_err(|error| error.to_string())?
                    .into_struct_value();
                self.emit_string_release(value)?;
            }
            Type::Array(element) if matches!(transparent_codegen_type(element), Type::Str) => {
                let drop_array = self.declare_libc_fn(
                    "tyrus_array_drop_string_contents",
                    &[pointer.into()],
                    None,
                );
                self.builder
                    .build_call(drop_array, &[slot.into()], "")
                    .unwrap();
            }
            Type::Array(element) if element.is_copy() => {
                let array = self
                    .builder
                    .build_load(self.llvm_type(&ty), slot, "async.payload.drop.copy.array")
                    .unwrap()
                    .into_struct_value();
                let data = self
                    .builder
                    .build_extract_value(array, 0, "async.payload.drop.copy.array.data")
                    .unwrap();
                let free = self.declare_libc_fn("free", &[pointer.into()], None);
                self.builder.build_call(free, &[data.into()], "").unwrap();
            }
            Type::Array(element) => {
                let drop_element = self.emit_async_array_element_drop(element)?;
                let element_size = self.llvm_type(element).size_of().ok_or_else(|| {
                    format!("error: async array element `{element}` must be sized")
                })?;
                let drop_contents = self.declare_libc_fn(
                    "tyrus_array_drop_owned_contents",
                    &[
                        pointer.into(),
                        self.context.i64_type().into(),
                        pointer.into(),
                    ],
                    None,
                );
                self.builder
                    .build_call(
                        drop_contents,
                        &[
                            slot.into(),
                            element_size.into(),
                            drop_element.as_global_value().as_pointer_value().into(),
                        ],
                        "",
                    )
                    .unwrap();
            }
            Type::Optional(inner) => {
                let value = self
                    .builder
                    .build_load(pointer, slot, "async.payload.drop.optional")
                    .unwrap()
                    .into_pointer_value();
                let release = self.context.append_basic_block(function, "drop.optional");
                let next = self
                    .context
                    .append_basic_block(function, "drop.optional.next");
                let present = self
                    .builder
                    .build_is_not_null(value, "async.payload.drop.optional.present")
                    .unwrap();
                self.builder
                    .build_conditional_branch(present, release, next)
                    .unwrap();
                self.builder.position_at_end(release);
                self.emit_async_drop_value_in_place(function, value, inner)?;
                let free = self.declare_libc_fn("free", &[pointer.into()], None);
                self.builder.build_call(free, &[value.into()], "").unwrap();
                self.builder.build_unconditional_branch(next).unwrap();
                self.builder.position_at_end(next);
            }
            Type::Union(variants) if Self::union_is_payload_free(variants) => {}
            Type::Union(variants) => {
                let tag = self.load_union_tag(variants, slot)?;
                let merge = self
                    .context
                    .append_basic_block(function, "drop.union.merge");
                let mut next = self.builder.get_insert_block().unwrap();
                for (index, variant) in variants.iter().enumerate() {
                    let release = self
                        .context
                        .append_basic_block(function, "drop.union.variant");
                    let otherwise = if index + 1 == variants.len() {
                        merge
                    } else {
                        self.context.append_basic_block(function, "drop.union.next")
                    };
                    self.builder.position_at_end(next);
                    let is_variant = self
                        .builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            tag,
                            self.context.i8_type().const_int(index as u64, false),
                            "drop.union.matches",
                        )
                        .unwrap();
                    self.builder
                        .build_conditional_branch(is_variant, release, otherwise)
                        .unwrap();
                    self.builder.position_at_end(release);
                    if !matches!(variant, Type::Void) {
                        let variant_slot =
                            self.union_payload_slot(variants, slot, variant, "drop.union.payload")?;
                        self.emit_async_drop_value_in_place(function, variant_slot, variant)?;
                    }
                    if !self.current_block_terminated() {
                        self.builder.build_unconditional_branch(merge).unwrap();
                    }
                    next = otherwise;
                }
                self.builder.position_at_end(merge);
            }
            // Dropping a handle frees nothing here: what it points at belongs
            // to the runtime, and the program releases it explicitly, the way
            // `server.close()` does. This mirrors the frame that stores a bare
            // handle and releases none of it.
            //
            // Only for a handle the program can actually give up. The arms
            // above are the handles with no such call, and `HttpRouter` is the
            // one that taught the difference: it has no `close()`, so falling
            // through to here left it with no owner at all, which cost a leak
            // per router created in an `async` function. Reaching this arm is a
            // claim that some other call releases the handle, not a default.
            Type::Named(name) if tyrus_core::native_handles::is_opaque_handle_name(name) => {}
            Type::Named(_) | Type::Object(_) => {
                let fields = self.async_object_fields(&ty).ok_or_else(|| {
                    format!("internal: async object payload `{ty}` has no fields")
                })?;
                let object_type = self.llvm_type(&ty).into_struct_type();
                for (index, field) in fields.iter().enumerate() {
                    let field_type = self.resolve_type(&field_storage_type(field));
                    if self.is_copy_for_async(&field_type) {
                        continue;
                    }
                    let field_slot = self
                        .builder
                        .build_struct_gep(
                            object_type,
                            slot,
                            index as u32,
                            &format!("async.payload.drop.{}.slot", field.name),
                        )
                        .map_err(|error| error.to_string())?;
                    self.emit_async_drop_value_in_place(function, field_slot, &field_type)?;
                }
            }
            Type::Tuple(elements) => {
                let tuple_type = self.llvm_type(&ty).into_struct_type();
                for (index, element_type) in elements.iter().enumerate() {
                    let element_type = self.resolve_type(element_type);
                    if element_type.is_copy() {
                        continue;
                    }
                    let element_slot = self
                        .builder
                        .build_struct_gep(
                            tuple_type,
                            slot,
                            index as u32,
                            &format!("async.payload.drop.tuple.{index}.slot"),
                        )
                        .map_err(|error| error.to_string())?;
                    self.emit_async_drop_value_in_place(function, element_slot, &element_type)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
