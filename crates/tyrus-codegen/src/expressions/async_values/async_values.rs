use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_promise_all_array_drop(
        &mut self,
        payload: &Type,
    ) -> Result<FunctionValue<'ctx>, String> {
        let payload = self.resolve_type(payload);
        let symbol = format!("__tyrus_drop_promise_all_array_{}", mangle_type(&payload));
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
        let boxed = function.get_first_param().unwrap().into_pointer_value();
        self.emit_async_drop_value_in_place(function, boxed, &Type::Array(Box::new(payload)))?;
        let free = self.declare_libc_fn("free", &[pointer.into()], None);
        self.builder.build_call(free, &[boxed.into()], "").unwrap();
        self.builder.build_return(None).unwrap();
        if let Some(block) = outer_block {
            self.builder.position_at_end(block);
        }
        Ok(function)
    }

    pub(super) fn emit_promise_settled_array_drop(
        &mut self,
        payload: &Type,
        settled: &Type,
    ) -> Result<FunctionValue<'ctx>, String> {
        let Type::Union(variants) = settled else {
            return Err("internal: Promise.allSettled result is not a union".to_string());
        };
        let fulfilled_value_index = self
            .struct_field_index(&variants[0], "value")
            .expect("fulfilled result always has a value field");
        let rejected_reason_index = self
            .struct_field_index(&variants[1], "reason")
            .expect("rejected result always has a reason field");
        let error_message_index = self
            .struct_field_index(&Type::Named("Error".to_string()), "message")
            .expect("Error always has a message field");
        let symbol = format!("__tyrus_drop_settled_array_{}", self.arrow_fn_count);
        self.arrow_fn_count += 1;
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let function = self.module.add_function(
            &symbol,
            self.context.void_type().fn_type(&[pointer.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(function, "entry");
        let loop_check = self.context.append_basic_block(function, "loop.check");
        let loop_body = self.context.append_basic_block(function, "loop.body");
        let fulfilled = self.context.append_basic_block(function, "fulfilled");
        let rejected = self.context.append_basic_block(function, "rejected");
        let next = self.context.append_basic_block(function, "next");
        let done = self.context.append_basic_block(function, "done");
        let outer_block = self.builder.get_insert_block();
        self.builder.position_at_end(entry);
        let boxed = function.get_nth_param(0).unwrap().into_pointer_value();
        let array_type = self.llvm_type(&Type::Array(Box::new(settled.clone())));
        let array = self
            .builder
            .build_load(array_type, boxed, "settled.array")
            .map_err(|error| error.to_string())?
            .into_struct_value();
        let data = self
            .builder
            .build_extract_value(array, 0, "settled.data")
            .unwrap()
            .into_pointer_value();
        let len = self
            .builder
            .build_extract_value(array, 1, "settled.len")
            .unwrap()
            .into_int_value();
        let index = self
            .builder
            .build_alloca(self.usize_type(), "settled.index")
            .unwrap();
        self.builder
            .build_store(index, self.usize_type().const_zero())
            .unwrap();
        self.builder.build_unconditional_branch(loop_check).unwrap();
        self.builder.position_at_end(loop_check);
        let current = self
            .builder
            .build_load(self.usize_type(), index, "settled.index.current")
            .unwrap()
            .into_int_value();
        let has_next = self
            .builder
            .build_int_compare(IntPredicate::ULT, current, len, "settled.has.next")
            .unwrap();
        self.builder
            .build_conditional_branch(has_next, loop_body, done)
            .unwrap();
        self.builder.position_at_end(loop_body);
        let slot = unsafe {
            self.builder
                .build_gep(self.llvm_type(settled), data, &[current], "settled.item")
                .unwrap()
        };
        let tag = self.load_union_tag(variants, slot)?;
        let is_rejected = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                tag,
                self.context.i8_type().const_int(1, false),
                "settled.is.rejected",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(is_rejected, rejected, fulfilled)
            .unwrap();
        self.builder.position_at_end(fulfilled);
        let fulfilled_result =
            self.load_union_payload(variants, slot, &variants[0], "settled.fulfilled")?;
        let fulfilled_value = self
            .builder
            .build_extract_value(
                fulfilled_result.into_struct_value(),
                fulfilled_value_index,
                "settled.fulfilled.value",
            )
            .map_err(|error| error.to_string())?;
        let fulfilled_slot = self
            .builder
            .build_alloca(self.llvm_type(payload), "settled.fulfilled.slot")
            .unwrap();
        self.builder
            .build_store(fulfilled_slot, fulfilled_value)
            .unwrap();
        self.emit_async_drop_value_in_place(function, fulfilled_slot, payload)?;
        if !self.current_block_terminated() {
            self.builder.build_unconditional_branch(next).unwrap();
        }
        self.builder.position_at_end(rejected);
        let rejected_value = self
            .load_union_payload(variants, slot, &variants[1], "settled.rejected")?
            .into_struct_value();
        let reason = self
            .builder
            .build_extract_value(rejected_value, rejected_reason_index, "settled.reason")
            .unwrap()
            .into_struct_value();
        let message = self
            .builder
            .build_extract_value(reason, error_message_index, "settled.reason.message")
            .unwrap();
        self.emit_string_release(message.into_struct_value())?;
        let free = self.declare_libc_fn(
            "free",
            &[self
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into()],
            None,
        );
        self.builder.build_unconditional_branch(next).unwrap();
        self.builder.position_at_end(next);
        let incremented = self
            .builder
            .build_int_add(
                current,
                self.usize_type().const_int(1, false),
                "settled.index.next",
            )
            .unwrap();
        self.builder.build_store(index, incremented).unwrap();
        self.builder.build_unconditional_branch(loop_check).unwrap();
        self.builder.position_at_end(done);
        self.builder.build_call(free, &[data.into()], "").unwrap();
        self.builder.build_call(free, &[boxed.into()], "").unwrap();
        self.builder.build_return(None).unwrap();
        if let Some(block) = outer_block {
            self.builder.position_at_end(block);
        }
        Ok(function)
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_promise_static_method(
        &mut self,
        object: &Expr,
        method: &str,
        type_args: &[Type],
        args: &[Expr],
    ) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        if !matches!(object.kind(), ExprKind::Ident(name) if name == "Promise")
            || self.load_var("Promise").is_some()
        {
            return None;
        }
        Some((|| {
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let result = match method {
                "reject" => {
                    let Some(error) = args.first() else {
                        return Err(
                            "error: `Promise.reject` expects an `Error` or message".to_string()
                        );
                    };
                    let error_type = self.expr_type_hint(error).map(|ty| self.resolve_type(&ty));
                    if matches!(&error_type, Some(Type::Named(name)) if name == "Error")
                        || (error_type.is_none() && matches!(error.kind(), ExprKind::ObjectLit(_)))
                    {
                        let value = self
                            .emit_expr_as(error, &Type::Named("Error".to_string()))?
                            .into_struct_value();
                        let message = self
                            .builder
                            .build_extract_value(value, 0, "promise.reject.message")
                            .unwrap();
                        let code = self
                            .builder
                            .build_extract_value(value, 1, "promise.reject.code")
                            .unwrap();
                        let pending = self.declare_libc_fn(
                            "tyrus_promise_pending",
                            &[],
                            Some(pointer.into()),
                        );
                        let promise = self
                            .builder
                            .build_call(pending, &[], "promise.reject.pending")
                            .unwrap()
                            .try_as_basic_value()
                            .basic()
                            .unwrap();
                        let reject = self.declare_libc_fn(
                            "tyrus_promise_reject_pending_structured",
                            &[
                                pointer.into(),
                                self.string_abi_type(),
                                self.string_abi_type(),
                            ],
                            Some(self.context.i32_type().into()),
                        );
                        self.builder
                            .build_call(
                                reject,
                                &[promise.into(), code.into(), message.into()],
                                "promise.reject.structured",
                            )
                            .unwrap();
                        return Ok(promise);
                    }
                    let message = match error_type {
                        Some(Type::Named(name)) if name == "AggregateError" => {
                            let value = self
                                .emit_expr_as(error, &Type::Named(name))?
                                .into_struct_value();
                            self.builder
                                .build_extract_value(value, 0, "promise.reject.message")
                                .unwrap()
                                .into_struct_value()
                        }
                        _ => self.emit_expr_as(error, &Type::Str)?.into_struct_value(),
                    };
                    let reject = self.declare_libc_fn(
                        "tyrus_promise_reject",
                        &[self.string_abi_type()],
                        Some(pointer.into()),
                    );
                    self.builder
                        .build_call(reject, &[message.into()], "promise.reject")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                }
                "resolve" => {
                    let Some(value) = args.first() else {
                        let resolve = self.declare_libc_fn(
                            "tyrus_promise_resolve_void",
                            &[],
                            Some(pointer.into()),
                        );
                        return Ok(self
                            .builder
                            .build_call(resolve, &[], "promise.resolve.void")
                            .unwrap()
                            .try_as_basic_value()
                            .basic()
                            .unwrap());
                    };
                    if matches!(self.expr_type_hint(value).map(|ty| self.resolve_type(&ty)), Some(Type::Apply { name, .. }) if matches!(name.as_str(), "Promise" | "DbPromise"))
                    {
                        let promise = self.emit_expr(value)?.into_pointer_value();
                        let retain = self.declare_libc_fn(
                            "tyrus_promise_retain",
                            &[pointer.into()],
                            Some(pointer.into()),
                        );
                        self.builder
                            .build_call(retain, &[promise.into()], "promise.resolve.flatten")
                            .unwrap()
                            .try_as_basic_value()
                            .basic()
                            .unwrap()
                    } else {
                        let ty = type_args
                            .first()
                            .cloned()
                            .or_else(|| self.expr_type_hint(value))
                            .ok_or_else(|| {
                                "error: cannot infer `Promise.resolve` payload type".to_string()
                            })?;
                        let emitted = self.emit_expr_as(value, &ty)?;
                        let emitted = if matches!(transparent_codegen_type(&ty), Type::Str) {
                            self.emit_string_clone(
                                emitted.into_struct_value(),
                                "promise.resolve.string.clone",
                            )?
                        } else {
                            emitted
                        };
                        if !matches!(transparent_codegen_type(&ty), Type::Str) {
                            self.forget_owned_native_result(emitted);
                        }
                        let payload =
                            self.heap_alloc(self.llvm_type(&ty), "promise.resolve.payload")?;
                        self.builder.build_store(payload, emitted).unwrap();
                        let drop_symbol = match transparent_codegen_type(&ty) {
                            Type::Str => "tyrus_drop_async_string",
                            Type::Array(element) if element.is_copy() => {
                                "tyrus_drop_async_copy_array"
                            }
                            _ => "tyrus_drop_async_box",
                        };
                        let drop_payload =
                            self.declare_libc_fn(drop_symbol, &[pointer.into()], None);
                        let resolve = self.declare_libc_fn(
                            "tyrus_promise_resolve",
                            &[pointer.into(), pointer.into()],
                            Some(pointer.into()),
                        );
                        self.builder
                            .build_call(
                                resolve,
                                &[
                                    payload.into(),
                                    drop_payload.as_global_value().as_pointer_value().into(),
                                ],
                                "promise.resolve",
                            )
                            .unwrap()
                            .try_as_basic_value()
                            .basic()
                            .unwrap()
                    }
                }
                "all" | "race" | "any" | "allSettled" => {
                    let Some(values) = args.first() else {
                        return Err(format!("error: `Promise.{method}` expects an array"));
                    };
                    let payload = type_args.first().cloned().or_else(|| {
                        let Type::Array(element) = self.expr_type_hint(values)? else {
                            return None;
                        };
                        let Type::Apply { name, args } = element.as_ref() else {
                            return None;
                        };
                        (name == "Promise").then(|| args.first().cloned()).flatten()
                    });
                    let array = self.emit_expr(values)?.into_struct_value();
                    let data = self
                        .builder
                        .build_extract_value(array, 0, "promise.inputs.data")
                        .unwrap();
                    let len = self
                        .builder
                        .build_extract_value(array, 1, "promise.inputs.len")
                        .unwrap();
                    let symbol = match method {
                        "all" => "tyrus_promise_all_typed",
                        "race" => "tyrus_promise_race",
                        "any" => "tyrus_promise_any",
                        _ => "tyrus_promise_all_settled_typed",
                    };
                    let mut call_args = vec![data.into(), len.into()];
                    let param_types = if method == "all" {
                        let payload = payload.ok_or_else(|| {
                            "error: cannot infer `Promise.all` payload type".to_string()
                        })?;
                        let (element_size, _) = self.type_layout(&payload);
                        let void_payload = matches!(
                            transparent_codegen_type(&self.resolve_type(&payload)),
                            Type::Void
                        );
                        if element_size == 0 && !void_payload {
                            return Err(format!(
                                "error: `Promise.all` payload `{payload}` has no native size"
                            ));
                        }
                        if !void_payload
                            && !payload.is_copy()
                            && !self.async_owned_value_supported(&payload)
                        {
                            return Err(format!(
                                "error: `Promise.all` payload `{payload}` is not supported by native code generation"
                            ));
                        }
                        let drop_payload = if void_payload {
                            Type::U8
                        } else {
                            payload.clone()
                        };
                        let drop_array = self.emit_promise_all_array_drop(&drop_payload)?;
                        call_args.push(
                            self.usize_type()
                                .const_int(element_size as u64, false)
                                .into(),
                        );
                        call_args.push(drop_array.as_global_value().as_pointer_value().into());
                        vec![
                            pointer.into(),
                            self.usize_type().into(),
                            self.usize_type().into(),
                            pointer.into(),
                        ]
                    } else if method == "allSettled" {
                        let payload = payload.ok_or_else(|| {
                            "error: cannot infer `Promise.allSettled` payload type".to_string()
                        })?;
                        if !payload.is_copy() && !self.async_owned_value_supported(&payload) {
                            return Err(format!(
                                "error: `Promise.allSettled` payload `{payload}` is not supported by native code generation"
                            ));
                        }
                        let settled = promise_settled_codegen_type(payload.clone());
                        let Type::Union(variants) = &settled else {
                            unreachable!()
                        };
                        let (element_size, element_align) = self.type_layout(&payload);
                        let (result_size, _) = self.type_layout(&settled);
                        let (_, fulfilled_align) = self.type_layout(&variants[0]);
                        let payload_offset = 1_usize.div_ceil(fulfilled_align) * fulfilled_align;
                        let (status_size, _) = self.type_layout(&Type::Str);
                        let fulfilled_value_offset =
                            payload_offset + status_size.div_ceil(element_align) * element_align;
                        let (_, error_align) = self.type_layout(&Type::Named("Error".to_string()));
                        let rejected_reason_offset =
                            payload_offset + status_size.div_ceil(error_align) * error_align;
                        let rejected_code_offset = rejected_reason_offset
                            + status_size.div_ceil(error_align) * error_align;
                        let drop_array =
                            self.emit_promise_settled_array_drop(&payload, &settled)?;
                        for value in [
                            element_size,
                            result_size,
                            payload_offset,
                            fulfilled_value_offset,
                            rejected_reason_offset,
                            rejected_code_offset,
                        ] {
                            call_args.push(self.usize_type().const_int(value as u64, false).into());
                        }
                        call_args.push(drop_array.as_global_value().as_pointer_value().into());
                        vec![
                            pointer.into(),
                            self.usize_type().into(),
                            self.usize_type().into(),
                            self.usize_type().into(),
                            self.usize_type().into(),
                            self.usize_type().into(),
                            self.usize_type().into(),
                            self.usize_type().into(),
                            pointer.into(),
                        ]
                    } else {
                        vec![pointer.into(), self.usize_type().into()]
                    };
                    let combinator =
                        self.declare_libc_fn(symbol, &param_types, Some(pointer.into()));
                    let result = self
                        .builder
                        .build_call(combinator, &call_args, &format!("promise.{method}"))
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    if let ExprKind::ArrayLit(elements) = values.kind() {
                        let promise_free =
                            self.declare_libc_fn("tyrus_promise_free", &[pointer.into()], None);
                        for (index, element) in elements.iter().enumerate() {
                            if !self.owned_promise_expression(element) {
                                continue;
                            }
                            let element_pointer = unsafe {
                                self.builder
                                    .build_gep(
                                        pointer,
                                        data.into_pointer_value(),
                                        &[self.usize_type().const_int(index as u64, false)],
                                        "promise.input.owned",
                                    )
                                    .unwrap()
                            };
                            let promise = self
                                .builder
                                .build_load(pointer, element_pointer, "promise.input.release")
                                .unwrap();
                            self.builder
                                .build_call(promise_free, &[promise.into()], "")
                                .unwrap();
                        }
                        let free = self.declare_libc_fn("free", &[pointer.into()], None);
                        self.builder.build_call(free, &[data.into()], "").unwrap();
                    }
                    result
                }
                "withResolvers" => {
                    let payload = type_args.first().cloned().ok_or_else(|| {
                        "error: `Promise.withResolvers` requires an explicit payload type during native code generation"
                            .to_string()
                    })?;
                    let capability_fn = self.declare_libc_fn(
                        "tyrus_promise_with_resolvers",
                        &[],
                        Some(pointer.into()),
                    );
                    let capability = self
                        .builder
                        .build_call(capability_fn, &[], "promise.capability")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_pointer_value();
                    let promise_fn = self.declare_libc_fn(
                        "tyrus_promise_capability_promise",
                        &[pointer.into()],
                        Some(pointer.into()),
                    );
                    let promise = self
                        .builder
                        .build_call(
                            promise_fn,
                            &[capability.into()],
                            "promise.capability.promise",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    let capability_free = self.declare_libc_fn(
                        "tyrus_promise_capability_free",
                        &[pointer.into()],
                        None,
                    );
                    self.builder
                        .build_call(capability_free, &[capability.into()], "")
                        .unwrap();
                    let promise_pointer = promise.into_pointer_value();

                    let resolve_type = Type::Function {
                        params: vec![payload.clone()],
                        ret: Box::new(Type::Void),
                    };
                    let reject_type = Type::Function {
                        params: vec![Type::Named("Error".to_string())],
                        ret: Box::new(Type::Void),
                    };
                    let resolve_fn_type =
                        self.llvm_closure_fn_type(std::slice::from_ref(&payload), &Type::Void);
                    let resolve_name =
                        format!("__tyrus_capability_resolve_{}", self.arrow_fn_count);
                    self.arrow_fn_count += 1;
                    let resolve_fn = self
                        .module
                        .add_function(&resolve_name, resolve_fn_type, None);
                    let resolve_entry = self.context.append_basic_block(resolve_fn, "entry");
                    let outer_block = self.builder.get_insert_block();
                    self.builder.position_at_end(resolve_entry);
                    let resolve_promise = resolve_fn.get_nth_param(0).unwrap().into_pointer_value();
                    let mut resolve_value = resolve_fn.get_nth_param(1).unwrap();
                    if matches!(transparent_codegen_type(&payload), Type::Str) {
                        resolve_value = self.emit_string_clone(
                            resolve_value.into_struct_value(),
                            "promise.capability.string.clone",
                        )?;
                    }
                    let boxed =
                        self.heap_alloc(self.llvm_type(&payload), "promise.capability.payload")?;
                    self.builder.build_store(boxed, resolve_value).unwrap();
                    let drop_symbol = match transparent_codegen_type(&payload) {
                        Type::Str => "tyrus_drop_async_string",
                        Type::Array(element) if element.is_copy() => "tyrus_drop_async_copy_array",
                        _ if payload.is_copy() => "tyrus_drop_async_box",
                        _ => {
                            return Err(format!(
                                "error: `Promise.withResolvers` payload `{payload}` is not supported by native code generation"
                            ));
                        }
                    };
                    let drop_payload = self.declare_libc_fn(drop_symbol, &[pointer.into()], None);
                    let settle = self.declare_libc_fn(
                        "tyrus_promise_fulfill_owned",
                        &[pointer.into(), pointer.into(), pointer.into()],
                        Some(self.context.i32_type().into()),
                    );
                    self.builder
                        .build_call(
                            settle,
                            &[
                                resolve_promise.into(),
                                boxed.into(),
                                drop_payload.as_global_value().as_pointer_value().into(),
                            ],
                            "promise.capability.resolve",
                        )
                        .unwrap();
                    self.builder.build_return(None).unwrap();

                    let reject_fn_type =
                        self.llvm_closure_fn_type(&[Type::Named("Error".to_string())], &Type::Void);
                    let reject_name = format!("__tyrus_capability_reject_{}", self.arrow_fn_count);
                    self.arrow_fn_count += 1;
                    let reject_fn = self.module.add_function(&reject_name, reject_fn_type, None);
                    let reject_entry = self.context.append_basic_block(reject_fn, "entry");
                    self.builder.position_at_end(reject_entry);
                    let reject_promise = reject_fn.get_nth_param(0).unwrap().into_pointer_value();
                    let error = reject_fn.get_nth_param(1).unwrap().into_struct_value();
                    let message = self
                        .builder
                        .build_extract_value(error, 0, "promise.capability.error.message")
                        .unwrap();
                    let reject = self.declare_libc_fn(
                        "tyrus_promise_reject_pending",
                        &[pointer.into(), self.string_abi_type()],
                        Some(self.context.i32_type().into()),
                    );
                    self.builder
                        .build_call(
                            reject,
                            &[reject_promise.into(), message.into()],
                            "promise.capability.reject",
                        )
                        .unwrap();
                    self.builder.build_return(None).unwrap();
                    if let Some(block) = outer_block {
                        self.builder.position_at_end(block);
                    }

                    let mut resolve_closure = self
                        .llvm_type(&resolve_type)
                        .into_struct_type()
                        .const_zero();
                    resolve_closure = self
                        .builder
                        .build_insert_value(
                            resolve_closure,
                            resolve_fn.as_global_value().as_pointer_value(),
                            0,
                            "promise.capability.resolve.function",
                        )
                        .unwrap()
                        .into_struct_value();
                    resolve_closure = self
                        .builder
                        .build_insert_value(
                            resolve_closure,
                            promise_pointer,
                            1,
                            "promise.capability.resolve.environment",
                        )
                        .unwrap()
                        .into_struct_value();
                    let mut reject_closure =
                        self.llvm_type(&reject_type).into_struct_type().const_zero();
                    reject_closure = self
                        .builder
                        .build_insert_value(
                            reject_closure,
                            reject_fn.as_global_value().as_pointer_value(),
                            0,
                            "promise.capability.reject.function",
                        )
                        .unwrap()
                        .into_struct_value();
                    reject_closure = self
                        .builder
                        .build_insert_value(
                            reject_closure,
                            promise_pointer,
                            1,
                            "promise.capability.reject.environment",
                        )
                        .unwrap()
                        .into_struct_value();
                    let result_type = Type::Object(vec![
                        Field {
                            name: "promise".to_string(),
                            ty: Type::Apply {
                                name: "Promise".to_string(),
                                args: vec![payload],
                            },
                            readonly: false,
                            optional: false,
                            decorators: Vec::new(),
                            span: Span::DUMMY,
                        },
                        Field {
                            name: "resolve".to_string(),
                            ty: resolve_type,
                            readonly: false,
                            optional: false,
                            decorators: Vec::new(),
                            span: Span::DUMMY,
                        },
                        Field {
                            name: "reject".to_string(),
                            ty: reject_type,
                            readonly: false,
                            optional: false,
                            decorators: Vec::new(),
                            span: Span::DUMMY,
                        },
                    ]);
                    let mut result = self.llvm_type(&result_type).into_struct_type().const_zero();
                    result = self
                        .builder
                        .build_insert_value(result, promise, 0, "promise.capability.result.promise")
                        .unwrap()
                        .into_struct_value();
                    result = self
                        .builder
                        .build_insert_value(
                            result,
                            resolve_closure,
                            1,
                            "promise.capability.result.resolve",
                        )
                        .unwrap()
                        .into_struct_value();
                    self.builder
                        .build_insert_value(
                            result,
                            reject_closure,
                            2,
                            "promise.capability.result.reject",
                        )
                        .unwrap()
                        .into_struct_value()
                        .into()
                }
                _ => {
                    return Err(format!(
                        "error: Promise.{method} codegen is not implemented"
                    ));
                }
            };
            Ok(result)
        })())
    }

    pub(crate) fn emit_promise_instance_method(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        let object_type = self
            .expr_type_hint(object)
            .map(|ty| self.resolve_type(&ty))?;
        let Type::Apply {
            name,
            args: promise_args,
        } = object_type
        else {
            return None;
        };
        if !matches!(name.as_str(), "Promise" | "DbPromise") || promise_args.len() != 1 {
            return None;
        }
        let payload = promise_args[0].clone();
        if !matches!(method, "then" | "catch" | "finally") {
            return Some(Err(format!("error: unknown Promise method `{method}`")));
        }
        Some((|| {
            let callback = args
                .first()
                .ok_or_else(|| format!("error: `Promise.{method}` expects a callback argument"))?;
            let callback_params = match method {
                "then" if matches!(payload, Type::Void) => Vec::new(),
                "then" => vec![payload.clone()],
                "catch" => vec![Type::Named("Error".to_string())],
                _ => Vec::new(),
            };
            let callback_return = self
                .infer_callback_return_type(callback, &callback_params)
                .unwrap_or(Type::Void);
            let callback_type = Type::Function {
                params: callback_params.clone(),
                ret: Box::new(callback_return.clone()),
            };
            let closure = match callback.kind() {
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
                    &callback_params,
                    &callback_return,
                    *is_async,
                    true,
                )?,
                _ => self.emit_expr_as(callback, &callback_type)?,
            };
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let closure_type = self.llvm_type(&callback_type).into_struct_type();
            let context = self.heap_alloc(closure_type.into(), "promise.callback.context")?;
            self.builder
                .build_store(context, closure)
                .map_err(|error| error.to_string())?;

            let adapter_name = format!("__tyrus_promise_{method}_{}", self.arrow_fn_count);
            self.arrow_fn_count += 1;
            let adapter_type = match method {
                "finally" => pointer.fn_type(&[pointer.into()], false),
                "catch" => pointer.fn_type(&[self.string_abi_type().into(), pointer.into()], false),
                _ => pointer.fn_type(&[pointer.into(), pointer.into()], false),
            };
            let adapter = self.module.add_function(&adapter_name, adapter_type, None);
            let entry = self.context.append_basic_block(adapter, "entry");
            let outer_block = self.builder.get_insert_block();
            self.builder.position_at_end(entry);
            let context_param = adapter
                .get_nth_param(if method == "finally" { 0 } else { 1 })
                .unwrap()
                .into_pointer_value();
            let loaded_closure = self
                .builder
                .build_load(closure_type, context_param, "promise.callback")
                .map_err(|error| error.to_string())?
                .into_struct_value();
            let function = self
                .builder
                .build_extract_value(loaded_closure, 0, "promise.callback.function")
                .unwrap()
                .into_pointer_value();
            let environment = self
                .builder
                .build_extract_value(loaded_closure, 1, "promise.callback.environment")
                .unwrap();
            let mut callback_args = vec![environment.into()];
            if method == "then" && !matches!(payload, Type::Void) {
                let value_ptr = adapter.get_nth_param(0).unwrap().into_pointer_value();
                let value = self
                    .builder
                    .build_load(self.llvm_type(&payload), value_ptr, "promise.then.value")
                    .map_err(|error| error.to_string())?;
                callback_args.push(value.into());
            } else if method == "catch" {
                let message = adapter.get_nth_param(0).unwrap().into_struct_value();
                let error_type = self.llvm_type(&Type::Named("Error".to_string()));
                let error = self
                    .builder
                    .build_insert_value(
                        error_type.into_struct_type().const_zero(),
                        message,
                        0,
                        "promise.catch.error",
                    )
                    .map_err(|error| error.to_string())?;
                let error = self
                    .builder
                    .build_insert_value(
                        error,
                        self.const_string_abi("ERR_RUNTIME", "promise.catch.error.code"),
                        1,
                        "promise.catch.error.code",
                    )
                    .map_err(|error| error.to_string())?
                    .into_struct_value();
                callback_args.push(error.into());
            }
            let call = self
                .builder
                .build_indirect_call(
                    self.llvm_closure_fn_type(&callback_params, &callback_return),
                    function,
                    &callback_args,
                    "promise.callback.call",
                )
                .map_err(|error| error.to_string())?;
            let returned = if let Type::Apply { name, .. } = &callback_return {
                if matches!(name.as_str(), "Promise" | "DbPromise") {
                    call.try_as_basic_value()
                        .basic()
                        .ok_or("internal: Promise callback lost its returned Promise")?
                        .into_pointer_value()
                } else {
                    return Err(format!(
                        "error: unsupported Promise callback return type `{callback_return}`"
                    ));
                }
            } else if matches!(callback_return, Type::Void) {
                let resolve =
                    self.declare_libc_fn("tyrus_promise_resolve_void", &[], Some(pointer.into()));
                self.builder
                    .build_call(resolve, &[], "promise.callback.resolve.void")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_pointer_value()
            } else {
                let mut value = call
                    .try_as_basic_value()
                    .basic()
                    .ok_or("internal: Promise callback lost its return value")?;
                if matches!(transparent_codegen_type(&callback_return), Type::Str) {
                    value = self.emit_string_clone(
                        value.into_struct_value(),
                        "promise.callback.string.clone",
                    )?;
                }
                let boxed =
                    self.heap_alloc(self.llvm_type(&callback_return), "promise.callback.payload")?;
                self.builder
                    .build_store(boxed, value)
                    .map_err(|error| error.to_string())?;
                let drop_symbol = match transparent_codegen_type(&callback_return) {
                    Type::Str => "tyrus_drop_async_string",
                    Type::Array(element) if element.is_copy() => "tyrus_drop_async_copy_array",
                    _ if callback_return.is_copy() => "tyrus_drop_async_box",
                    _ => {
                        return Err(format!(
                            "error: Promise callback return `{callback_return}` is not supported by native code generation"
                        ));
                    }
                };
                let drop_payload = self.declare_libc_fn(drop_symbol, &[pointer.into()], None);
                let resolve = self.declare_libc_fn(
                    "tyrus_promise_resolve",
                    &[pointer.into(), pointer.into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(
                        resolve,
                        &[
                            boxed.into(),
                            drop_payload.as_global_value().as_pointer_value().into(),
                        ],
                        "promise.callback.resolve",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_pointer_value()
            };
            self.builder.build_return(Some(&returned)).unwrap();
            if let Some(block) = outer_block {
                self.builder.position_at_end(block);
            }

            let source = self.emit_expr(object)?.into_pointer_value();
            let runtime_symbol = match method {
                "then" => "tyrus_promise_then",
                "catch" => "tyrus_promise_catch",
                _ => "tyrus_promise_finally",
            };
            let runtime = self.declare_libc_fn(
                runtime_symbol,
                &[
                    pointer.into(),
                    pointer.into(),
                    pointer.into(),
                    pointer.into(),
                ],
                Some(pointer.into()),
            );
            let drop_context = self.declare_libc_fn(
                "tyrus_drop_promise_closure_context",
                &[pointer.into()],
                None,
            );
            let result = self
                .builder
                .build_call(
                    runtime,
                    &[
                        source.into(),
                        adapter.as_global_value().as_pointer_value().into(),
                        context.into(),
                        drop_context.as_global_value().as_pointer_value().into(),
                    ],
                    &format!("promise.{method}"),
                )
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap();
            if self.owned_promise_expression(object) {
                let free = self.declare_libc_fn("tyrus_promise_free", &[pointer.into()], None);
                self.builder.build_call(free, &[source.into()], "").unwrap();
            }
            Ok(result)
        })())
    }

    pub(crate) fn owned_promise_expression(&self, expression: &Expr) -> bool {
        matches!(
            expression.kind(),
            ExprKind::FnCall { .. } | ExprKind::MethodCall { .. }
        ) && self.expr_type_hint(expression).is_some_and(|ty| {
            matches!(
                self.resolve_type(&ty),
                Type::Apply { name, args } if matches!(name.as_str(), "Promise" | "DbPromise") && args.len() == 1
            )
        })
    }

    pub(crate) fn emit_stream_instance_method(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        let object_type = self
            .expr_type_hint(object)
            .map(|ty| self.resolve_type(&ty))?;
        let Type::Apply {
            name,
            args: stream_args,
        } = object_type
        else {
            return None;
        };
        let valid_stream = match name.as_str() {
            "ReadableStream" | "WritableStream" => stream_args.len() == 1,
            "DuplexStream" | "TransformStream" => stream_args.len() == 2,
            _ => false,
        };
        if !valid_stream {
            return None;
        }
        let payload_ty = stream_args[0].clone();
        Some((|| {
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let stream = self.emit_expr(object)?.into_pointer_value();
            let receiver_is_owned = self.forget_owned_native_result(stream.into())
                || self.owned_stream_capability_expression(object);
            let result = match (name.as_str(), method) {
                ("ReadableStream", "read") if args.is_empty() => {
                    let read = self.declare_libc_fn(
                        "tyrus_readable_stream_read",
                        &[pointer.into()],
                        Some(pointer.into()),
                    );
                    Ok(self
                        .builder
                        .build_call(read, &[stream.into()], "stream.read")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap())
                }
                ("ReadableStream", "pipeTo") if args.len() == 1 => {
                    let destination = self.emit_expr(&args[0])?.into_pointer_value();
                    let destination_is_owned = self.forget_owned_native_result(destination.into())
                        || self.owned_stream_capability_expression(&args[0]);
                    let pipe = self.declare_libc_fn(
                        "tyrus_readable_stream_pipe_to",
                        &[pointer.into(), pointer.into()],
                        Some(pointer.into()),
                    );
                    let result = self
                        .builder
                        .build_call(pipe, &[stream.into(), destination.into()], "stream.pipe_to")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    if destination_is_owned {
                        let free = self.declare_libc_fn(
                            "tyrus_writable_stream_free",
                            &[pointer.into()],
                            None,
                        );
                        self.builder
                            .build_call(free, &[destination.into()], "")
                            .unwrap();
                    }
                    Ok(result)
                }
                ("WritableStream", "write") if args.len() == 1 => {
                    let value = self.emit_expr_as(&args[0], &payload_ty)?;
                    let boxed =
                        self.heap_alloc(self.llvm_type(&payload_ty), "stream.write.payload")?;
                    let value = match transparent_codegen_type(&payload_ty) {
                        Type::Str => {
                            let clone = self.emit_string_clone(
                                value.into_struct_value(),
                                "stream.write.string.clone",
                            )?;
                            self.builder
                                .build_store(boxed, clone)
                                .map_err(|error| error.to_string())?;
                            self.forget_owned_native_result(clone);
                            clone
                        }
                        Type::Array(element) if element.is_copy() => {
                            self.emit_async_clone_value_into(boxed, value, &payload_ty)?;
                            self.emit_async_cleanup_source_expression(
                                value,
                                &payload_ty,
                                &args[0],
                            )?;
                            value
                        }
                        _ => {
                            self.builder
                                .build_store(boxed, value)
                                .map_err(|error| error.to_string())?;
                            value
                        }
                    };
                    let drop_symbol = match transparent_codegen_type(&payload_ty) {
                        Type::Str => "tyrus_drop_async_string",
                        Type::Array(element) if element.is_copy() => "tyrus_drop_async_copy_array",
                        _ if payload_ty.is_copy() => "tyrus_drop_async_box",
                        _ => {
                            return Err(format!(
                                "error: stream payload `{payload_ty}` is not supported by native code generation"
                            ));
                        }
                    };
                    let drop_payload = self.declare_libc_fn(drop_symbol, &[pointer.into()], None);
                    let bytes = match transparent_codegen_type(&payload_ty) {
                        Type::Str => {
                            let length = self.string_byte_len(value.into_struct_value())?;
                            self.builder
                                .build_int_cast(
                                    length,
                                    self.usize_type(),
                                    "stream.write.string_bytes",
                                )
                                .map_err(|error| error.to_string())?
                        }
                        Type::Array(element) if element.is_copy() => {
                            let array = value.into_struct_value();
                            let length = self
                                .builder
                                .build_extract_value(array, 1, "stream.write.array_length")
                                .map_err(|error| error.to_string())?
                                .into_int_value();
                            let length = self
                                .builder
                                .build_int_cast(
                                    length,
                                    self.usize_type(),
                                    "stream.write.array_length_native",
                                )
                                .map_err(|error| error.to_string())?;
                            let element_bytes = self
                                .llvm_type(element)
                                .size_of()
                                .unwrap_or_else(|| self.usize_type().const_int(1, false));
                            let element_bytes = self
                                .builder
                                .build_int_cast(
                                    element_bytes,
                                    self.usize_type(),
                                    "stream.write.element_bytes_native",
                                )
                                .map_err(|error| error.to_string())?;
                            self.builder
                                .build_int_mul(length, element_bytes, "stream.write.array_bytes")
                                .map_err(|error| error.to_string())?
                        }
                        _ => self
                            .llvm_type(&payload_ty)
                            .size_of()
                            .unwrap_or_else(|| self.usize_type().const_int(1, false)),
                    };
                    let write = self.declare_libc_fn(
                        "tyrus_writable_stream_write",
                        &[
                            pointer.into(),
                            pointer.into(),
                            self.usize_type().into(),
                            pointer.into(),
                        ],
                        Some(pointer.into()),
                    );
                    Ok(self
                        .builder
                        .build_call(
                            write,
                            &[
                                stream.into(),
                                boxed.into(),
                                bytes.into(),
                                drop_payload.as_global_value().as_pointer_value().into(),
                            ],
                            "stream.write",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap())
                }
                ("ReadableStream" | "WritableStream", "close") if args.is_empty() => {
                    let symbol = if name == "ReadableStream" {
                        "tyrus_readable_stream_close"
                    } else {
                        "tyrus_writable_stream_close"
                    };
                    let close =
                        self.declare_libc_fn(symbol, &[pointer.into()], Some(pointer.into()));
                    Ok(self
                        .builder
                        .build_call(close, &[stream.into()], "stream.close")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap())
                }
                ("DuplexStream" | "TransformStream", "close") if args.is_empty() => {
                    let symbol = if name == "DuplexStream" {
                        "tyrus_duplex_stream_close"
                    } else {
                        "tyrus_transform_stream_close"
                    };
                    let close =
                        self.declare_libc_fn(symbol, &[pointer.into()], Some(pointer.into()));
                    Ok(self
                        .builder
                        .build_call(close, &[stream.into()], "stream.close")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap())
                }
                _ => Err(format!("error: unknown {name} method `{method}`")),
            }?;
            if receiver_is_owned {
                let symbol = match name.as_str() {
                    "ReadableStream" => "tyrus_readable_stream_free",
                    "WritableStream" => "tyrus_writable_stream_free",
                    "DuplexStream" => "tyrus_duplex_stream_free",
                    "TransformStream" => "tyrus_transform_stream_free",
                    _ => return Err(format!("error: unknown stream handle `{name}`")),
                };
                let free = self.declare_libc_fn(symbol, &[pointer.into()], None);
                self.builder.build_call(free, &[stream.into()], "").unwrap();
            }
            Ok(result)
        })())
    }

    pub(crate) fn owned_stream_capability_expression(&self, expression: &Expr) -> bool {
        match expression.kind() {
            ExprKind::FieldAccess { object, field } => {
                let Some(object_type) = self.expr_type_hint(object) else {
                    return false;
                };
                matches!(
                    transparent_codegen_type(&object_type),
                    Type::Apply { name, args }
                        if matches!(name.as_str(), "DuplexStream" | "TransformStream")
                            && args.len() == 2
                            && matches!(field.as_str(), "readable" | "writable")
                ) || matches!(
                    transparent_codegen_type(&object_type),
                    Type::Named(name)
                        if name == "ChildProcess"
                            && matches!(field.as_str(), "stdin" | "stdout" | "stderr")
                )
            }
            ExprKind::FnCall { callee, .. } => matches!(
                callee.as_str(),
                "__tyrus_fs_readFileStream"
                    | "__tyrus_fs_writeFileStream"
                    | tyrus_core::archive::PACK_STREAM_TARGET
                    | tyrus_core::compress::COMPRESSION_STREAM_TARGET
                    | tyrus_core::compress::DECOMPRESSION_STREAM_TARGET
            ),
            _ => false,
        }
    }

    pub(super) fn emit_async_arrow_fn(
        &mut self,
        params: &[Param],
        ret_type: &Type,
        body: &[Stmt],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if params
            .iter()
            .any(|parameter| matches!(parameter.ty, Type::Infer))
            || matches!(ret_type, Type::Infer)
        {
            return Err(
                "error: async closure requires explicit parameter and return types during code generation"
                    .to_string(),
            );
        }
        let mut captures = closure_captures(params, body)
            .into_iter()
            .filter_map(|name| self.load_var(&name).map(|(slot, ty)| (name, slot, ty)))
            .collect::<Vec<_>>();
        captures.sort_by(|left, right| left.0.cmp(&right.0));
        let unsupported = captures
            .iter()
            .filter(|(_, _, ty)| !ty.is_copy() && !self.async_owned_value_supported(ty))
            .map(|(name, _, _)| name.as_str())
            .collect::<Vec<_>>();
        if !unsupported.is_empty() {
            return Err(format!(
                "error: async closure captures require a recursively owned representation; captured `{}`",
                unsupported.join("`, `")
            ));
        }
        let symbol = format!("__tyrus_async_arrow_{}", self.arrow_fn_count);
        self.arrow_fn_count += 1;
        let outer_block = self.builder.get_insert_block();
        let capture_params = captures.iter().map(|(name, _, ty)| Param {
            decorators: Vec::new(),
            pattern: ParamPattern::Name(name.clone()),
            ty: ty.clone(),
            mutable: false,
            optional: false,
            default_value: None,
            rest: false,
            span: tyrus_core::span::Span::DUMMY,
        });
        let all_params = capture_params
            .chain(params.iter().cloned())
            .collect::<Vec<_>>();
        self.forward_declare_fn(&symbol, &all_params, ret_type);
        let outer_vars = std::mem::replace(&mut self.vars, vec![HashMap::default()]);
        let outer_owned_closures =
            std::mem::replace(&mut self.owned_closure_vars, vec![HashSet::default()]);
        let outer_owned_closure_drops =
            std::mem::replace(&mut self.owned_closure_drop_fns, vec![HashMap::default()]);
        let outer_owned_native =
            std::mem::replace(&mut self.owned_native_vars, vec![HashMap::default()]);
        let outer_native_results = std::mem::take(&mut self.pending_owned_native_results);
        let outer_for_of_iterables = std::mem::take(&mut self.for_of_iterable_cleanups);
        let outer_conditionally_owned_vars =
            std::mem::replace(&mut self.conditionally_owned_vars, vec![HashMap::default()]);
        let outer_conditionally_owned_results =
            std::mem::take(&mut self.pending_conditionally_owned_results);
        let outer_mutable_string_ownership =
            std::mem::replace(&mut self.mutable_string_ownership, vec![HashMap::default()]);
        let outer_string_array_ownership =
            std::mem::replace(&mut self.string_array_ownership, vec![HashMap::default()]);
        let outer_capture_params =
            std::mem::replace(&mut self.async_capture_param_count, captures.len());
        let outer_try_state = self.take_enclosing_try_state();
        let emitted = self.emit_async_function(&symbol, &all_params, ret_type, body);
        self.restore_enclosing_try_state(outer_try_state);
        self.async_capture_param_count = outer_capture_params;
        self.vars = outer_vars;
        self.owned_closure_vars = outer_owned_closures;
        self.owned_closure_drop_fns = outer_owned_closure_drops;
        self.owned_native_vars = outer_owned_native;
        self.pending_owned_native_results = outer_native_results;
        self.for_of_iterable_cleanups = outer_for_of_iterables;
        self.conditionally_owned_vars = outer_conditionally_owned_vars;
        self.pending_conditionally_owned_results = outer_conditionally_owned_results;
        self.mutable_string_ownership = outer_mutable_string_ownership;
        self.string_array_ownership = outer_string_array_ownership;
        emitted?;
        if let Some(block) = outer_block {
            self.builder.position_at_end(block);
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let environment_type = self.context.struct_type(
            &captures
                .iter()
                .map(|(_, _, ty)| self.llvm_type(ty))
                .collect::<Vec<_>>(),
            false,
        );
        let environment_drop = if captures.is_empty() {
            None
        } else {
            let drop = self.module.add_function(
                &format!("{symbol}_environment_drop"),
                self.context.void_type().fn_type(&[pointer.into()], false),
                None,
            );
            let entry = self.context.append_basic_block(drop, "entry");
            self.builder.position_at_end(entry);
            let environment = drop.get_first_param().unwrap().into_pointer_value();
            for (index, (_, _, ty)) in captures.iter().enumerate() {
                if ty.is_copy() {
                    continue;
                }
                let field = self
                    .builder
                    .build_struct_gep(
                        environment_type,
                        environment,
                        index as u32,
                        "async.closure.capture.drop",
                    )
                    .unwrap();
                self.emit_async_drop_value_in_place(drop, field, ty)?;
            }
            let free = self.declare_libc_fn("free", &[pointer.into()], None);
            self.builder
                .build_call(free, &[environment.into()], "")
                .unwrap();
            self.builder.build_return(None).unwrap();
            if let Some(block) = outer_block {
                self.builder.position_at_end(block);
            }
            Some(drop)
        };
        let environment = if captures.is_empty() {
            pointer.const_null()
        } else {
            let (malloc, size) = self.declare_malloc(environment_type.size_of().unwrap());
            let environment = self
                .builder
                .build_call(malloc, &[size.into()], "async.closure.environment")
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap()
                .into_pointer_value();
            for (index, (_, slot, ty)) in captures.iter().enumerate() {
                let field = self
                    .builder
                    .build_struct_gep(
                        environment_type,
                        environment,
                        index as u32,
                        "async.closure.capture",
                    )
                    .unwrap();
                let value = self
                    .builder
                    .build_load(self.llvm_type(ty), *slot, "async.closure.capture.value")
                    .unwrap();
                if ty.is_copy() {
                    self.builder.build_store(field, value).unwrap();
                } else {
                    self.emit_async_clone_value_into(field, value, ty)?;
                }
            }
            environment
        };
        let param_types = params
            .iter()
            .map(|parameter| {
                if parameter.optional && parameter.default_value.is_none() && !parameter.rest {
                    Type::Optional(Box::new(parameter.ty.clone()))
                } else {
                    parameter.ty.clone()
                }
            })
            .collect::<Vec<_>>();
        let thunk = self.module.add_function(
            &format!("{symbol}_closure"),
            self.llvm_closure_fn_type(&param_types, ret_type),
            None,
        );
        let entry = self.context.append_basic_block(thunk, "entry");
        self.builder.position_at_end(entry);
        let env = thunk.get_first_param().unwrap().into_pointer_value();
        let mut arguments: Vec<BasicMetadataValueEnum<'ctx>> = Vec::with_capacity(all_params.len());
        for (index, (_, _, ty)) in captures.iter().enumerate() {
            let field = self
                .builder
                .build_struct_gep(environment_type, env, index as u32, "async.closure.capture")
                .unwrap();
            arguments.push(
                self.builder
                    .build_load(self.llvm_type(ty), field, "async.closure.capture.value")
                    .unwrap()
                    .into(),
            );
        }
        arguments.extend((0..params.len()).map(|index| {
            BasicMetadataValueEnum::from(thunk.get_nth_param(index as u32 + 1).unwrap())
        }));
        let function = *self
            .fns
            .get(&symbol)
            .ok_or_else(|| "internal: async closure wrapper was not registered".to_string())?;
        let promise = self
            .builder
            .build_call(function, &arguments, "async.closure.call")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "internal: async closure wrapper lost its Promise".to_string())?;
        self.builder.build_return(Some(&promise)).unwrap();
        if let Some(block) = outer_block {
            self.builder.position_at_end(block);
        }
        let closure_type = self
            .llvm_type(&Type::Function {
                params: param_types,
                ret: Box::new(ret_type.clone()),
            })
            .into_struct_type();
        let mut closure = closure_type.get_undef();
        closure = self
            .builder
            .build_insert_value(
                closure,
                thunk.as_global_value().as_pointer_value(),
                0,
                "async.closure.function",
            )
            .unwrap()
            .into_struct_value();
        closure = self
            .builder
            .build_insert_value(closure, environment, 1, "async.closure.env")
            .unwrap()
            .into_struct_value();
        self.pending_closure_drop_fn = environment_drop;
        Ok(closure.into())
    }

    pub(super) fn promise_combinator_payload(&self, args: &[Expr]) -> Option<Type> {
        let iterable = args.first().and_then(|arg| self.expr_type_hint(arg))?;
        let Type::Array(element) = transparent_codegen_type(&iterable) else {
            return None;
        };
        crate::core::promise_payload_type(transparent_codegen_type(element)).cloned()
    }
}
