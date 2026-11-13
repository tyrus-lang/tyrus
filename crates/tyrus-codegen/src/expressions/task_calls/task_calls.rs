use super::*;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_task_fn_call(
        &mut self,
        callee: &str,
        type_args: &[Type],
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        match callee {
            "__tyrus_task_isCancelled" => {
                if !args.is_empty() {
                    return (Err(format!(
                        "error: `isCancelled` expects 0 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let Some(result) = self.async_results.last().copied() else {
                    return (Err(
                        "error: `isCancelled` can only be used inside an async function"
                            .to_string(),
                    ))
                    .map(Some);
                };
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let requested = self.declare_libc_fn(
                    "tyrus_promise_cancel_requested",
                    &[pointer.into()],
                    Some(self.context.i32_type().into()),
                );
                let state = self
                    .builder
                    .build_call(requested, &[result.into()], "tasks.is_cancelled")
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("internal: cancel request query returned no value")?
                    .into_int_value();
                return Ok(Some(
                    self.builder
                        .build_int_compare(
                            inkwell::IntPredicate::NE,
                            state,
                            self.context.i32_type().const_zero(),
                            "tasks.cancelled_now",
                        )
                        .map_err(|error| error.to_string())?
                        .into(),
                ));
            }
            "__tyrus_task_cancel" => {
                if args.len() < 2 || args.len() > 3 {
                    return (Err(format!(
                        "error: `cancel` expects 2 to 3 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let promise = self.emit_expr(&args[0])?.into_pointer_value();
                let reason = self.emit_expr_as(&args[1], &Type::Str)?.into_struct_value();
                let i64_type = self.context.i64_type();
                // A negative deadline is "no deadline". The runtime has
                // no way to spell an absent `u64` across the ABI, and a
                // sentinel outside the range a caller can write is the
                // convention every other optional count here uses.
                let grace = match args.get(2) {
                    Some(grace) => self.emit_expr_as(grace, &Type::U64)?.into_int_value(),
                    None => i64_type.const_int(u64::MAX, false),
                };
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let cancel = self.declare_libc_fn(
                    "tyrus_task_cancel",
                    &[pointer.into(), self.string_abi_type(), i64_type.into()],
                    Some(self.context.i32_type().into()),
                );
                let cancelled = self
                    .builder
                    .build_call(
                        cancel,
                        &[promise.into(), reason.into(), grace.into()],
                        "tasks.cancel",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                return Ok(Some(
                    self.builder
                        .build_int_compare(
                            inkwell::IntPredicate::NE,
                            cancelled,
                            self.context.i32_type().const_zero(),
                            "tasks.cancelled",
                        )
                        .unwrap()
                        .into(),
                ));
            }
            "__tyrus_task_sleep" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `sleep` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let milliseconds = self.emit_expr_as(&args[0], &Type::U64)?;
                let sleep =
                    self.declare_libc_fn("tyrus_sleep", &[self.context.i64_type().into()], None);
                self.builder
                    .build_call(sleep, &[milliseconds.into()], "task.sleep")
                    .unwrap();
                return Ok(Some(self.context.i32_type().const_zero().into()));
            }
            "__tyrus_task_sleepAsync" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `sleepAsync` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let milliseconds = self.emit_expr_as(&args[0], &Type::U64)?;
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let sleep = self.declare_libc_fn(
                    "tyrus_task_sleep_async",
                    &[self.context.i64_type().into()],
                    Some(pointer.into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(sleep, &[milliseconds.into()], "task.sleepAsync")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "__tyrus_task_spawn" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `spawn` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let task = &args[0];
                let task_type = match task.kind() {
                    ExprKind::ArrowFn {
                        params, ret_type, ..
                    } => self.resolve_type(&Type::Function {
                        params: params.iter().map(|p| p.ty.clone()).collect(),
                        ret: Box::new(ret_type.clone()),
                    }),
                    _ => self
                        .expr_type_hint(task)
                        .map(|ty| self.resolve_type(&ty))
                        .ok_or_else(|| {
                            "error: cannot resolve the type of the task passed to `spawn`"
                                .to_string()
                        })?,
                };
                let Type::Function { ret: task_ret, .. } = task_type else {
                    return (Err(
                        "error: `spawn` expects an async function with no parameters".to_string(),
                    ))
                    .map(Some);
                };
                let is_fresh_arrow = matches!(task.kind(), ExprKind::ArrowFn { .. });
                let closure = match task.kind() {
                    ExprKind::ArrowFn {
                        is_async,
                        params,
                        ret_type,
                        body,
                        ..
                    } => self.emit_contextual_arrow_fn(
                        params,
                        ret_type,
                        body,
                        &[],
                        &task_ret,
                        *is_async,
                        true,
                    )?,
                    _ => self.emit_expr_as(
                        task,
                        &Type::Function {
                            params: Vec::new(),
                            ret: task_ret.clone(),
                        },
                    )?,
                }
                .into_struct_value();
                let environment_drop = is_fresh_arrow
                    .then(|| self.pending_closure_drop_fn.take())
                    .flatten();
                let function = self
                    .builder
                    .build_extract_value(closure, 0, "tasks.spawn.function")
                    .unwrap()
                    .into_pointer_value();
                let environment = self
                    .builder
                    .build_extract_value(closure, 1, "tasks.spawn.environment")
                    .unwrap();
                let call = self
                    .builder
                    .build_indirect_call(
                        self.llvm_closure_fn_type(&[], &task_ret),
                        function,
                        &[environment.into()],
                        "tasks.spawn.call",
                    )
                    .map_err(|error| error.to_string())?;
                let result = call
                    .try_as_basic_value()
                    .basic()
                    .ok_or("internal: spawn task lost its returned Promise")?;
                if let Some(drop_fn) = environment_drop {
                    self.builder
                        .build_call(drop_fn, &[environment.into()], "")
                        .unwrap();
                }
                return Ok(Some(result));
            }
            "__tyrus_task_channel" => {
                if args.len() > 1 {
                    return (Err(format!(
                        "error: `channel` expects 0 or 1 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                // `capacity?: usize`: an `Optional` is always a
                // pointer (null for `None`), matching how
                // `__tyrus_net_resolve`'s own `timeoutMs?: u64`
                // already crosses the ABI, necessary here because a
                // provided argument can itself be a runtime-absent
                // `usize?` value, not just an omitted argument.
                let capacity = if let Some(arg) = args.first() {
                    self.emit_expr_as(arg, &Type::Optional(Box::new(Type::Usize)))?
                        .into_pointer_value()
                } else {
                    pointer.const_null()
                };
                let pair_type = self
                    .context
                    .struct_type(&[pointer.into(), pointer.into()], false);
                let out = self
                    .builder
                    .build_alloca(pair_type, "channel.pair")
                    .map_err(|error| error.to_string())?;
                let channel_new = self.declare_libc_fn(
                    "tyrus_channel_new",
                    &[pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(channel_new, &[capacity.into(), out.into()], "")
                    .unwrap();
                // `capacity?` crosses the ABI as a pointer to a boxed
                // `usize`, and the runtime copies the value out before
                // returning, so the box is dead the moment this call
                // is done. `emit_optional_box` deliberately registers
                // no release of its own, which left these eight bytes
                // leaking once per channel; freeing it here, where its
                // lifetime provably ends, needs none of the ownership
                // machinery that gap is about.
                if !args.is_empty() {
                    let free = self.declare_libc_fn("free", &[pointer.into()], None);
                    self.builder
                        .build_call(free, &[capacity.into()], "")
                        .unwrap();
                }
                return (self
                    .builder
                    .build_load(pair_type, out, "channel.pair.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_task_mutex_new" => {
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let Some(value) = args.first() else {
                    return (Err("error: `Mutex` expects a value argument".to_string())).map(Some);
                };
                let payload_ty = type_args
                    .first()
                    .cloned()
                    .or_else(|| self.expr_type_hint(value))
                    .ok_or_else(|| "error: cannot infer `Mutex` payload type".to_string())?;
                let emitted = self.emit_expr_as(value, &payload_ty)?;
                let emitted = if matches!(transparent_codegen_type(&payload_ty), Type::Str) {
                    self.emit_string_clone(emitted.into_struct_value(), "mutex.new.string.clone")?
                } else {
                    emitted
                };
                let boxed = self.heap_alloc(self.llvm_type(&payload_ty), "mutex.new.payload")?;
                self.builder
                    .build_store(boxed, emitted)
                    .map_err(|error| error.to_string())?;
                let drop_symbol = match transparent_codegen_type(&payload_ty) {
                    Type::Str => "tyrus_drop_async_string",
                    Type::Array(element) if element.is_copy() => "tyrus_drop_async_copy_array",
                    _ if payload_ty.is_copy() => "tyrus_drop_async_box",
                    _ => {
                        return (Err(format!(
                            "error: `Mutex<{payload_ty}>` payload is not supported by native code generation"
                        ))).map(Some);
                    }
                };
                let drop_payload = self.declare_libc_fn(drop_symbol, &[pointer.into()], None);
                let mutex_new = self.declare_libc_fn(
                    "tyrus_mutex_new",
                    &[pointer.into(), pointer.into()],
                    Some(pointer.into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(
                            mutex_new,
                            &[
                                boxed.into(),
                                drop_payload.as_global_value().as_pointer_value().into(),
                            ],
                            "mutex.new",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "__tyrus_task_rwlock_new" => {
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let Some(value) = args.first() else {
                    return (Err("error: `RwLock` expects a value argument".to_string())).map(Some);
                };
                let payload_ty = type_args
                    .first()
                    .cloned()
                    .or_else(|| self.expr_type_hint(value))
                    .ok_or_else(|| "error: cannot infer `RwLock` payload type".to_string())?;
                let emitted = self.emit_expr_as(value, &payload_ty)?;
                let emitted = if matches!(transparent_codegen_type(&payload_ty), Type::Str) {
                    self.emit_string_clone(emitted.into_struct_value(), "rwlock.new.string.clone")?
                } else {
                    emitted
                };
                let boxed = self.heap_alloc(self.llvm_type(&payload_ty), "rwlock.new.payload")?;
                self.builder
                    .build_store(boxed, emitted)
                    .map_err(|error| error.to_string())?;
                let drop_symbol = match transparent_codegen_type(&payload_ty) {
                    Type::Str => "tyrus_drop_async_string",
                    Type::Array(element) if element.is_copy() => "tyrus_drop_async_copy_array",
                    _ if payload_ty.is_copy() => "tyrus_drop_async_box",
                    _ => {
                        return (Err(format!(
                            "error: `RwLock<{payload_ty}>` payload is not supported by native code generation"
                        ))).map(Some);
                    }
                };
                let drop_payload = self.declare_libc_fn(drop_symbol, &[pointer.into()], None);
                let rwlock_new = self.declare_libc_fn(
                    "tyrus_rwlock_new",
                    &[pointer.into(), pointer.into()],
                    Some(pointer.into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(
                            rwlock_new,
                            &[
                                boxed.into(),
                                drop_payload.as_global_value().as_pointer_value().into(),
                            ],
                            "rwlock.new",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "__tyrus_task_semaphore_new" => {
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let Some(permits) = args.first() else {
                    return (Err("error: `Semaphore` expects a `permits` argument".to_string()))
                        .map(Some);
                };
                let permits = self.emit_expr_as(permits, &Type::Usize)?;
                let semaphore_new = self.declare_libc_fn(
                    "tyrus_semaphore_new",
                    &[self.usize_type().into()],
                    Some(pointer.into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(semaphore_new, &[permits.into()], "semaphore.new")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "__tyrus_task_joinAll" => {
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let Some(values) = args.first() else {
                    return (Err("error: `joinAll` expects an array".to_string())).map(Some);
                };
                let payload = type_args.first().cloned().or_else(|| {
                    let Type::Array(element) = self.expr_type_hint(values)? else {
                        return None;
                    };
                    crate::core::join_handle_payload(&element).cloned()
                });
                let payload = payload
                    .ok_or_else(|| "error: cannot infer `joinAll` payload type".to_string())?;
                let (element_size, _) = self.type_layout(&payload);
                if element_size == 0 {
                    return (Err(format!(
                        "error: `joinAll` payload `{payload}` has no native size"
                    )))
                    .map(Some);
                }
                if !payload.is_copy() && !self.async_owned_value_supported(&payload) {
                    return (Err(format!(
                        "error: `joinAll` payload `{payload}` is not supported by native code generation"
                    ))).map(Some);
                }
                let array = self.emit_expr(values)?.into_struct_value();
                let data = self
                    .builder
                    .build_extract_value(array, 0, "join_all.inputs.data")
                    .unwrap();
                let len = self
                    .builder
                    .build_extract_value(array, 1, "join_all.inputs.len")
                    .unwrap();
                let drop_array = self.emit_promise_all_array_drop(&payload)?;
                let combinator = self.declare_libc_fn(
                    "tyrus_promise_all_typed",
                    &[
                        pointer.into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                        pointer.into(),
                    ],
                    Some(pointer.into()),
                );
                let result = self
                    .builder
                    .build_call(
                        combinator,
                        &[
                            data.into(),
                            len.into(),
                            self.usize_type()
                                .const_int(element_size as u64, false)
                                .into(),
                            drop_array.as_global_value().as_pointer_value().into(),
                        ],
                        "tasks.join_all",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                if let ExprKind::ArrayLit(elements) = values.kind() {
                    let promise_free =
                        self.declare_libc_fn("tyrus_promise_free", &[pointer.into()], None);
                    for (index, element) in elements.iter().enumerate() {
                        let is_owned_ident = matches!(element.kind(), ExprKind::Ident(_))
                            && self
                                .expr_type_hint(element)
                                .map(|ty| self.resolve_type(&ty))
                                .is_some_and(|ty| {
                                    matches!(
                                        transparent_codegen_type(&ty),
                                        Type::Apply { name, args }
                                            if name == "JoinHandle" && args.len() == 1
                                    )
                                });
                        if !self.owned_join_handle_expression(element) && !is_owned_ident {
                            continue;
                        }
                        let element_pointer = unsafe {
                            self.builder
                                .build_gep(
                                    pointer,
                                    data.into_pointer_value(),
                                    &[self.usize_type().const_int(index as u64, false)],
                                    "join_all.input.owned",
                                )
                                .unwrap()
                        };
                        let promise = self
                            .builder
                            .build_load(pointer, element_pointer, "join_all.input.release")
                            .unwrap();
                        self.builder
                            .build_call(promise_free, &[promise.into()], "")
                            .unwrap();
                    }
                    let free = self.declare_libc_fn("free", &[pointer.into()], None);
                    self.builder.build_call(free, &[data.into()], "").unwrap();
                }
                return Ok(Some(result));
            }
            "__tyrus_task_race" => {
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let Some(values) = args.first() else {
                    return (Err("error: `race` expects an array".to_string())).map(Some);
                };
                let array = self.emit_expr(values)?.into_struct_value();
                let data = self
                    .builder
                    .build_extract_value(array, 0, "tasks.race.inputs.data")
                    .unwrap();
                let len = self
                    .builder
                    .build_extract_value(array, 1, "tasks.race.inputs.len")
                    .unwrap();
                let combinator = self.declare_libc_fn(
                    "tyrus_task_race",
                    &[pointer.into(), self.usize_type().into()],
                    Some(pointer.into()),
                );
                let result = self
                    .builder
                    .build_call(combinator, &[data.into(), len.into()], "tasks.race")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                if let ExprKind::ArrayLit(elements) = values.kind() {
                    let promise_free =
                        self.declare_libc_fn("tyrus_promise_free", &[pointer.into()], None);
                    for (index, element) in elements.iter().enumerate() {
                        let is_owned_ident = matches!(element.kind(), ExprKind::Ident(_))
                            && self
                                .expr_type_hint(element)
                                .map(|ty| self.resolve_type(&ty))
                                .is_some_and(|ty| {
                                    matches!(
                                        transparent_codegen_type(&ty),
                                        Type::Apply { name, args }
                                            if name == "JoinHandle" && args.len() == 1
                                    )
                                });
                        if !self.owned_join_handle_expression(element) && !is_owned_ident {
                            continue;
                        }
                        let element_pointer = unsafe {
                            self.builder
                                .build_gep(
                                    pointer,
                                    data.into_pointer_value(),
                                    &[self.usize_type().const_int(index as u64, false)],
                                    "tasks.race.input.owned",
                                )
                                .unwrap()
                        };
                        let promise = self
                            .builder
                            .build_load(pointer, element_pointer, "tasks.race.input.release")
                            .unwrap();
                        self.builder
                            .build_call(promise_free, &[promise.into()], "")
                            .unwrap();
                    }
                    let free = self.declare_libc_fn("free", &[pointer.into()], None);
                    self.builder.build_call(free, &[data.into()], "").unwrap();
                }
                return Ok(Some(result));
            }
            "__tyrus_task_timeout" => {
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `timeout` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let ms = self.emit_expr_as(&args[0], &Type::U64)?;
                let task = self.emit_expr(&args[1])?;
                let combinator = self.declare_libc_fn(
                    "tyrus_task_timeout",
                    &[self.context.i64_type().into(), pointer.into()],
                    Some(pointer.into()),
                );
                let result = self
                    .builder
                    .build_call(combinator, &[ms.into(), task.into()], "tasks.timeout")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                if self.owned_promise_expression(&args[1]) {
                    let free = self.declare_libc_fn("tyrus_promise_free", &[pointer.into()], None);
                    self.builder.build_call(free, &[task.into()], "").unwrap();
                }
                return Ok(Some(result));
            }
            _ => {}
        }
        Ok(None)
    }
}
