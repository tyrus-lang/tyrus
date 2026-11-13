use super::*;
use tyrus_core::ast::ExprKind;

#[derive(Clone, Copy, Eq, PartialEq)]
enum ConsoleOutput {
    Stdout,
    Stderr,
}

pub(super) fn static_object_projection_keys(
    argument: &Expr,
    type_hint: Option<Type>,
) -> Option<Vec<String>> {
    if let ExprKind::ArrayLit(elements) = argument.kind() {
        return elements
            .iter()
            .map(|element| match element.kind() {
                ExprKind::StrLit(value) => Some(value.clone()),
                _ => None,
            })
            .collect();
    }
    static_object_projection_key_type_names(type_hint.as_ref()?)
}

fn static_object_projection_key_type_names(ty: &Type) -> Option<Vec<String>> {
    match transparent_codegen_type(ty) {
        Type::Array(element) => static_object_projection_string_literal_names(element),
        _ => None,
    }
}

fn static_object_projection_string_literal_names(ty: &Type) -> Option<Vec<String>> {
    match transparent_codegen_type(ty) {
        Type::Never => Some(Vec::new()),
        Type::Lit(expression) => match expression.kind() {
            ExprKind::StrLit(value) => Some(vec![value.clone()]),
            _ => None,
        },
        Type::Union(variants) => variants.iter().try_fold(Vec::new(), |mut names, variant| {
            names.extend(static_object_projection_string_literal_names(variant)?);
            Some(names)
        }),
        _ => None,
    }
}

enum PrintPiece<'ctx> {
    Literal {
        text: String,
        name: &'static str,
    },
    Value {
        val: BasicValueEnum<'ctx>,
        type_hint: Type,
        arg_index: usize,
    },
}

#[path = "automatic_json/automatic_json.rs"]
mod automatic_json;

#[path = "native_handles/native_handles.rs"]
mod native_handles;
#[path = "network_handles/network_handles.rs"]
mod network_handles;
impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_regex_method(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match method {
            "test" => {
                let input = self.emit_expr(&args[0])?.into_struct_value();
                let pattern = self.emit_expr(object)?.into_struct_value();
                let i64_type = self.context.i64_type();
                let function = self.declare_libc_fn(
                    "tyrus_string_matches_regex",
                    &[self.string_abi_type(), self.string_abi_type()],
                    Some(i64_type.into()),
                );
                let result = self
                    .builder
                    .build_call(function, &[input.into(), pattern.into()], "regex.test")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                let is_true = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        result,
                        i64_type.const_int(0, false),
                        "regex.test",
                    )
                    .unwrap();
                Ok(is_true.into())
            }

            "exec" => {
                let pattern = self.emit_expr(object)?.into_struct_value();
                let input = self.emit_expr(&args[0])?.into_struct_value();
                let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                let i64_type = self.context.i64_type();

                let array_type = self.llvm_type(&Type::Array(Box::new(Type::Str)));
                let out_slot = self.entry_alloca(array_type, "regex.exec_out").unwrap();

                let function = self.declare_libc_fn(
                    "tyrus_regex_exec",
                    &[
                        self.string_abi_type(),
                        self.string_abi_type(),
                        ptr_type.into(),
                    ],
                    Some(i64_type.into()),
                );
                let matched = self
                    .builder
                    .build_call(
                        function,
                        &[pattern.into(), input.into(), out_slot.into()],
                        "regex.exec",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();

                let fn_val = self
                    .builder
                    .get_insert_block()
                    .and_then(inkwell::basic_block::BasicBlock::get_parent)
                    .ok_or("internal: regex.exec outside function")?;
                let present = self
                    .context
                    .append_basic_block(fn_val, "regex.exec_present");
                let absent = self.context.append_basic_block(fn_val, "regex.exec_absent");
                let merge = self.context.append_basic_block(fn_val, "regex.exec_merge");

                let is_zero = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        matched,
                        i64_type.const_int(0, false),
                        "regex.exec_matched",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(is_zero, absent, present)
                    .unwrap();

                self.builder.position_at_end(absent);
                let null_ptr = ptr_type.const_null();
                self.builder.build_unconditional_branch(merge).unwrap();
                let absent_block = self.builder.get_insert_block().unwrap();

                self.builder.position_at_end(present);
                let array_val = self
                    .builder
                    .build_load(array_type, out_slot, "regex.exec_array")
                    .unwrap();
                let size = array_type
                    .size_of()
                    .ok_or("error: regex exec array has no native size")?;
                let (malloc, size) = self.declare_malloc(size);
                let storage = self
                    .builder
                    .build_call(malloc, &[size.into()], "regex.exec_storage")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_pointer_value();
                self.builder.build_store(storage, array_val).unwrap();
                self.builder.build_unconditional_branch(merge).unwrap();
                let present_block = self.builder.get_insert_block().unwrap();

                self.builder.position_at_end(merge);
                let phi = self
                    .builder
                    .build_phi(ptr_type, "regex.exec_result")
                    .unwrap();
                phi.add_incoming(&[
                    (&null_ptr, absent_block),
                    (&(storage as PointerValue<'ctx>), present_block),
                ]);
                self.record_owned_native_result_with(
                    phi.as_basic_value(),
                    Type::Optional(Box::new(Type::Array(Box::new(Type::Str)))),
                    crate::ReleasePlan::Box(Box::new(crate::ReleasePlan::Deep)),
                );
                Ok(phi.as_basic_value())
            }

            "execAll" => {
                let pattern = self.emit_expr(object)?.into_struct_value();
                let input = self.emit_expr(&args[0])?.into_struct_value();
                let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                let i64_type = self.context.i64_type();

                let outer_array_type =
                    self.llvm_type(&Type::Array(Box::new(Type::Array(Box::new(Type::Str)))));
                let out_slot = self
                    .entry_alloca(outer_array_type, "regex.exec_all_out")
                    .unwrap();

                let function = self.declare_libc_fn(
                    "tyrus_regex_exec_all",
                    &[
                        self.string_abi_type(),
                        self.string_abi_type(),
                        ptr_type.into(),
                    ],
                    Some(i64_type.into()),
                );
                let matched = self
                    .builder
                    .build_call(
                        function,
                        &[pattern.into(), input.into(), out_slot.into()],
                        "regex.exec_all",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();

                let fn_val = self
                    .builder
                    .get_insert_block()
                    .and_then(inkwell::basic_block::BasicBlock::get_parent)
                    .ok_or("internal: regex.exec_all outside function")?;
                let present = self
                    .context
                    .append_basic_block(fn_val, "regex.exec_all_present");
                let absent = self
                    .context
                    .append_basic_block(fn_val, "regex.exec_all_absent");
                let merge = self
                    .context
                    .append_basic_block(fn_val, "regex.exec_all_merge");

                let is_zero = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        matched,
                        i64_type.const_int(0, false),
                        "regex.exec_all_matched",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(is_zero, absent, present)
                    .unwrap();

                self.builder.position_at_end(absent);
                let null_ptr = ptr_type.const_null();
                self.builder.build_unconditional_branch(merge).unwrap();
                let absent_block = self.builder.get_insert_block().unwrap();

                self.builder.position_at_end(present);
                let array_val = self
                    .builder
                    .build_load(outer_array_type, out_slot, "regex.exec_all_array")
                    .unwrap();
                let size = outer_array_type
                    .size_of()
                    .ok_or("error: regex exec_all array has no native size")?;
                let (malloc, size) = self.declare_malloc(size);
                let storage = self
                    .builder
                    .build_call(malloc, &[size.into()], "regex.exec_all_storage")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_pointer_value();
                self.builder.build_store(storage, array_val).unwrap();
                self.builder.build_unconditional_branch(merge).unwrap();
                let present_block = self.builder.get_insert_block().unwrap();

                self.builder.position_at_end(merge);
                let phi = self
                    .builder
                    .build_phi(ptr_type, "regex.exec_all_result")
                    .unwrap();
                phi.add_incoming(&[
                    (&null_ptr, absent_block),
                    (&(storage as PointerValue<'ctx>), present_block),
                ]);
                self.record_owned_native_result_with(
                    phi.as_basic_value(),
                    Type::Optional(Box::new(Type::Array(Box::new(Type::Array(Box::new(
                        Type::Str,
                    )))))),
                    crate::ReleasePlan::Box(Box::new(crate::ReleasePlan::Deep)),
                );
                Ok(phi.as_basic_value())
            }

            "toString" => {
                if let ExprKind::RegexLit { pattern, flags } = object.kind() {
                    let text = format!("/{pattern}/{flags}");
                    let global = self
                        .const_string_abi(&text, "regex.to_string")
                        .into_struct_value();
                    return Ok(global.into());
                }
                let fn_val = self.declare_libc_fn(
                    "tyrus_regex_to_string",
                    &[self.string_abi_type()],
                    Some(self.string_abi_type()),
                );
                let regex_ptr = self.emit_expr(object)?.into_struct_value();
                let result = self
                    .builder
                    .build_call(fn_val, &[regex_ptr.into()], "regex_to_string")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.record_owned_native_result(result, Type::Str);
                Ok(result)
            }

            _ => Err(format!("error: unknown Regex method `{method}`")),
        }
    }

    pub(super) fn emit_optional_field_access(
        &mut self,
        object: &Expr,
        field: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let Some(Type::Optional(receiver_type)) = self.declared_expr_type_hint(object) else {
            return Err("error: optional chaining requires an optional receiver".to_string());
        };
        let Some(result_type) = self.field_type_on(&receiver_type, field) else {
            return Err(format!(
                "error: cannot infer the type of optional field `{field}`"
            ));
        };
        if matches!(result_type, Type::Void) {
            return Err("error: optional chaining of void fields is not supported".to_string());
        }

        let receiver = self.emit_expr(object)?.into_pointer_value();
        let is_null = self
            .builder
            .build_is_null(receiver, "optional.field.is_null")
            .unwrap();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: optional field access outside function")?;
        let absent = self
            .context
            .append_basic_block(function, "optional.field.absent");
        let present = self
            .context
            .append_basic_block(function, "optional.field.present");
        let merge = self
            .context
            .append_basic_block(function, "optional.field.merge");
        self.builder
            .build_conditional_branch(is_null, absent, present)
            .unwrap();

        self.builder.position_at_end(absent);
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let null_result = pointer.const_null();
        self.builder.build_unconditional_branch(merge).unwrap();
        let absent_block = self.builder.get_insert_block().unwrap();

        self.builder.position_at_end(present);
        let receiver_value = self
            .builder
            .build_load(
                self.llvm_type(&receiver_type),
                receiver,
                "optional.field.receiver",
            )
            .unwrap();
        let receiver_slot = self
            .entry_alloca(
                self.llvm_type(&receiver_type),
                "optional.field.receiver.slot",
            )
            .unwrap();
        self.builder
            .build_store(receiver_slot, receiver_value)
            .unwrap();
        let temporary = format!("__tyrus_optional_receiver_{}", self.arrow_fn_count);
        self.arrow_fn_count += 1;
        self.push_scope();
        self.store_var(&temporary, receiver_slot, (*receiver_type).clone());
        let result = self.emit_expr(&Expr::synthetic(ExprKind::FieldAccess {
            object: Box::new(Expr::synthetic(ExprKind::Ident(temporary))),
            field: field.to_string(),
        }))?;
        self.pop_scope();

        if matches!(transparent_codegen_type(&result_type), Type::Optional(_)) {
            self.builder.build_unconditional_branch(merge).unwrap();
            let present_block = self.builder.get_insert_block().unwrap();

            self.builder.position_at_end(merge);
            let phi = self
                .builder
                .build_phi(pointer, "optional.field.result")
                .unwrap();
            phi.add_incoming(&[
                (&null_result, absent_block),
                (&result.into_pointer_value(), present_block),
            ]);
            return Ok(phi.as_basic_value());
        }

        let size = self
            .llvm_type(&result_type)
            .size_of()
            .ok_or_else(|| format!("error: optional type `{result_type}` has no native size"))?;
        let (malloc, size) = self.declare_malloc(size);
        let storage = self
            .builder
            .build_call(malloc, &[size.into()], "optional.field.storage")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();
        self.builder.build_store(storage, result).unwrap();
        self.builder.build_unconditional_branch(merge).unwrap();
        let present_block = self.builder.get_insert_block().unwrap();

        self.builder.position_at_end(merge);
        let phi = self
            .builder
            .build_phi(pointer, "optional.field.result")
            .unwrap();
        phi.add_incoming(&[(&null_result, absent_block), (&storage, present_block)]);
        // Unlike `emit_optional_method_call`, this box's own release was never
        // registered at all, found while closing `optional-chain`, as an
        // 8-byte-per-iteration leak neither a bound local nor a `??` ever
        // claimed. `Value`, not `Deep`: the field may hold a static string a
        // struct literal was built with, so only the header may be freed
        // unconditionally, freeing the payload here would risk exactly the
        // segfault that first narrowed the `??` fix to method calls only.
        self.record_owned_native_result_with(
            phi.as_basic_value(),
            Type::Optional(Box::new(result_type)),
            crate::ReleasePlan::Value,
        );
        Ok(phi.as_basic_value())
    }

    pub(super) fn emit_optional_method_call(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let Type::Optional(receiver_type) = self
            .expr_type_hint(object)
            .ok_or("error: cannot infer optional receiver type")?
        else {
            return Err("error: optional chaining requires an optional receiver".to_string());
        };
        let Type::Optional(result_type) = self
            .expr_type_hint(&Expr::synthetic(ExprKind::OptionalMethodCall {
                object: Box::new(object.clone()),
                method: method.to_string(),
                type_args: Vec::new(),
                args: args.to_vec(),
            }))
            .ok_or_else(|| format!("error: cannot infer optional method `{method}` result"))?
        else {
            return Err(format!(
                "error: optional method `{method}` must return an optional value"
            ));
        };
        if matches!(result_type.as_ref(), Type::Void) {
            return Err(
                "error: optional chaining of void methods is not supported yet".to_string(),
            );
        }

        let receiver = self.emit_expr(object)?.into_pointer_value();
        let is_null = self
            .builder
            .build_is_null(receiver, "optional.is_null")
            .unwrap();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: optional method call outside function")?;
        let absent = self.context.append_basic_block(function, "optional.absent");
        let present = self
            .context
            .append_basic_block(function, "optional.present");
        let merge = self.context.append_basic_block(function, "optional.merge");
        self.builder
            .build_conditional_branch(is_null, absent, present)
            .unwrap();

        self.builder.position_at_end(absent);
        let null_result = self
            .context
            .ptr_type(inkwell::AddressSpace::default())
            .const_null();
        self.builder.build_unconditional_branch(merge).unwrap();
        let absent_block = self.builder.get_insert_block().unwrap();

        self.builder.position_at_end(present);
        let receiver_value = self
            .builder
            .build_load(
                self.llvm_type(&receiver_type),
                receiver,
                "optional.receiver",
            )
            .unwrap();
        let receiver_slot = self
            .entry_alloca(self.llvm_type(&receiver_type), "optional.receiver.slot")
            .unwrap();
        self.builder
            .build_store(receiver_slot, receiver_value)
            .unwrap();
        let temporary = format!("__tyrus_optional_receiver_{}", self.arrow_fn_count);
        self.arrow_fn_count += 1;
        self.push_scope();
        self.store_var(&temporary, receiver_slot, (*receiver_type).clone());
        self.optional_chain_depth += 1;
        let result = self.emit_method_call(
            &Expr::synthetic(ExprKind::Ident(temporary)),
            method,
            &[],
            args,
        );
        self.optional_chain_depth -= 1;
        let result = result?;
        self.pop_scope();

        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let size = self
            .llvm_type(&result_type)
            .size_of()
            .ok_or_else(|| format!("error: optional type `{result_type}` has no native size"))?;
        let (malloc, size) = self.declare_malloc(size);
        let storage = self
            .builder
            .build_call(malloc, &[size.into()], "optional.result.storage")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();
        self.builder.build_store(storage, result).unwrap();
        self.builder.build_unconditional_branch(merge).unwrap();
        let present_block = self.builder.get_insert_block().unwrap();

        self.builder.position_at_end(merge);
        let phi = self.builder.build_phi(pointer, "optional.result").unwrap();
        phi.add_incoming(&[(&null_result, absent_block), (&storage, present_block)]);

        // The box is claimed here, on the merged value, and not where it was
        // malloc'd: a release recorded inside `present` does not dominate the
        // scope cleanup that runs after the branches rejoin, which is what made
        // recording it emit invalid IR before.
        //
        // `Deep` rather than a flat plan: every string-returning method reached
        // through `?.` genuinely allocates (the same reasoning
        // `async_owned_string_expression` already relies on elsewhere), so a
        // box this becomes the sole owner of, a `?.` result bound to a name
        // with no further `??`, has to recurse into its payload when released,
        // or that payload leaks forever. `Deep` on a `Type::Optional` already
        // does exactly that: it is what `emit_async_drop_value_in_place`'s own
        // `Type::Optional` arm performs, null-checked, for a plain scope-owned
        // optional local.
        //
        // The one place that is wrong is `??`: it hands the payload out *by
        // value* (cloned, so the merged result is independently owned), and
        // recursing here too would free memory the coalesce result still needs.
        // `emit_null_coalesce` accounts for this itself, it forgets this exact
        // registration and frees only the box's own header, bypassing this plan
        // entirely, once it has safely dealt with the payload on its own terms.
        self.record_owned_native_result_with(
            phi.as_basic_value(),
            Type::Optional(result_type.clone()),
            crate::ReleasePlan::Deep,
        );
        Ok(phi.as_basic_value())
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_method_call(
        &mut self,
        object: &Expr,
        method: &str,
        type_args: &[Type],
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if let ExprKind::Ident(model_name) = object.kind() {
            let model_name = self
                .db_models
                .contains_key(model_name)
                .then_some(model_name.as_str())
                .or_else(|| {
                    self.db_models
                        .keys()
                        .find(|name| name.ends_with(&format!("_{model_name}")))
                        .map(String::as_str)
                });
            if let Some(model_name) = model_name.filter(|_| method == "findMany") {
                let Some(result) = self.emit_db_call(
                    tyrus_core::db::FIND_MANY_TARGET,
                    &[Type::Named(model_name.to_string())],
                    args,
                )?
                else {
                    return Err("internal: model `findMany` dispatch was not handled".to_string());
                };
                return Ok(result);
            }
            if let Some(model_name) = model_name.filter(|_| method == "findOne") {
                let Some(result) = self.emit_db_call(
                    tyrus_core::db::FIND_ONE_TARGET,
                    &[Type::Named(model_name.to_string())],
                    args,
                )?
                else {
                    return Err("internal: model `findOne` dispatch was not handled".to_string());
                };
                return Ok(result);
            }
            if let (Some(model_name), Some(target)) =
                (model_name, tyrus_core::db::model_method_target(method))
            {
                let Some(result) =
                    self.emit_db_call(target, &[Type::Named(model_name.to_string())], args)?
                else {
                    return Err(format!(
                        "internal: model `{method}` dispatch was not handled"
                    ));
                };
                return Ok(result);
            }
        }
        if let ExprKind::Ident(obj_name) = object.kind() {
            match (obj_name.as_str(), method) {
                ("console", "log" | "error" | "warn") => {
                    return self.emit_console_log(args, obj_name, method);
                }
                ("process", "exit") if self.load_var("process").is_none() => {
                    let code = self.emit_expr_as(&args[0], &Type::I32)?.into_int_value();
                    let exit =
                        self.declare_libc_fn("exit", &[self.context.i32_type().into()], None);
                    self.builder.build_call(exit, &[code.into()], "").unwrap();
                    self.builder.build_unreachable().unwrap();
                    return Ok(self.context.i32_type().const_zero().into());
                }
                ("process", "cwd") if self.load_var("process").is_none() => {
                    return self.emit_process_cwd();
                }
                ("process", "uptime") if self.load_var("process").is_none() => {
                    return self.emit_process_uptime();
                }
                ("process", "memoryUsage") if self.load_var("process").is_none() => {
                    return self.emit_process_memory_usage();
                }
                ("process", "cpuUsage") if self.load_var("process").is_none() => {
                    return self.emit_process_cpu_usage();
                }
                _ => {}
            }
        }

        if let ExprKind::Ident(type_name) = object.kind()
            && method == "fromJson"
            && self.declared_interfaces.contains_key(type_name)
        {
            return self.emit_automatic_json_decode(type_name, &args[0]);
        }

        if let Some(symbol) = self
            .extension_method_for(object, method)
            .map(|extension| extension.symbol.clone())
        {
            let mut call_args = Vec::with_capacity(args.len() + 1);
            call_args.push(object.clone());
            call_args.extend(args.iter().cloned());
            let call = Expr::synthetic(ExprKind::FnCall {
                callee: symbol,
                type_args: type_args.to_vec(),
                args: call_args,
            });
            return self.emit_expr(&call);
        }

        if method == "copy"
            && args.is_empty()
            && let Some(receiver) = self.expr_type_hint(object)
        {
            let resolved = self.resolve_type(&receiver);
            if !resolved.is_copy() && self.async_owned_value_supported(&resolved) {
                let value = self.emit_expr_as(object, &resolved)?;
                let slot = self
                    .builder
                    .build_alloca(self.llvm_type(&resolved), "value.copy.result")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(slot, self.llvm_type(&resolved).const_zero())
                    .map_err(|error| error.to_string())?;
                self.emit_async_clone_value_into(slot, value, &resolved)?;
                let copied = self
                    .builder
                    .build_load(self.llvm_type(&resolved), slot, "value.copy.value")
                    .map_err(|error| error.to_string())?;
                self.record_owned_native_result(copied, resolved);
                return Ok(copied);
            }
        }

        if let Some((automatic, receiver)) = self.automatic_data_method_for(object, method) {
            match automatic {
                tyrus_core::automatic_data_methods::AutomaticDataMethod::Copy => {
                    let value = self.emit_expr_as(object, &receiver)?;
                    let slot = self
                        .builder
                        .build_alloca(self.llvm_type(&receiver), "automatic.copy.result")
                        .map_err(|error| error.to_string())?;
                    self.emit_async_clone_value_into(slot, value, &receiver)?;
                    let copied = self
                        .builder
                        .build_load(self.llvm_type(&receiver), slot, "automatic.copy.value")
                        .map_err(|error| error.to_string())?;
                    self.record_owned_native_result(copied, receiver);
                    return Ok(copied);
                }
                tyrus_core::automatic_data_methods::AutomaticDataMethod::Eq => {
                    let left = self.emit_expr_as(object, &receiver)?;
                    let right = self.emit_expr_as(&args[0], &receiver)?;
                    return Ok(self
                        .emit_structural_equality(left, right, &receiver)?
                        .into());
                }
                tyrus_core::automatic_data_methods::AutomaticDataMethod::Debug => {
                    let value = self.emit_expr_as(object, &receiver)?;
                    let result = self.emit_automatic_debug(value, &receiver)?;
                    self.record_owned_native_result(result.into(), Type::Str);
                    return Ok(result.into());
                }
                tyrus_core::automatic_data_methods::AutomaticDataMethod::JsonEncode => {
                    let value = self.emit_expr_as(object, &receiver)?.into_struct_value();
                    let pretty = matches!(args.first().map(tyrus_core::ast::Expr::kind), Some(ExprKind::ObjectLit(fields)) if fields.iter().any(|(name, value)| name == "pretty" && matches!(value.kind(), ExprKind::BoolLit(true))));
                    let result = self.emit_automatic_json_encode(value, &receiver, pretty)?;
                    self.record_owned_native_result(result.into(), Type::Str);
                    return Ok(result.into());
                }
                _ => {}
            }
        }

        if matches!(method, "pick" | "omit")
            && let Some(result) = self.emit_object_projection_method(object, method, args)?
        {
            return Ok(result);
        }

        if self
            .expr_type_hint(object)
            .is_some_and(|ty| matches!(transparent_codegen_type(&ty), Type::Char))
            && let Some(result) = self.emit_char_method(object, method)
        {
            return result;
        }

        if method == "toString"
            && args.is_empty()
            && let Some(ty) = self.expr_type_hint(object)
        {
            let scalar = transparent_codegen_type(&ty).clone();
            if matches!(scalar, Type::Str | Type::Bool)
                || is_integer_type(&scalar)
                || is_float_type(&scalar)
            {
                let value = self.emit_expr(object)?;
                let result = self.emit_template_value_string(value, Some(&ty))?;
                if self.numeric_to_string_allocates(object, method, args) {
                    self.record_owned_native_result(result.into(), Type::Str);
                }
                return Ok(result.into());
            }
        }

        if matches!(
            method,
            "algebraicAdd" | "algebraicSub" | "algebraicMul" | "algebraicDiv" | "algebraicRem"
        ) && self
            .expr_type_hint(object)
            .is_some_and(|ty| matches!(transparent_codegen_type(&ty), Type::F32 | Type::F64))
        {
            let ty = self.expr_type_hint(object).unwrap();
            let left = self.emit_expr_as(object, &ty)?.into_float_value();
            let right = self.emit_expr_as(&args[0], &ty)?.into_float_value();
            let result = match method {
                "algebraicAdd" => self.builder.build_float_add(left, right, "algebraic.fadd"),
                "algebraicSub" => self.builder.build_float_sub(left, right, "algebraic.fsub"),
                "algebraicMul" => self.builder.build_float_mul(left, right, "algebraic.fmul"),
                "algebraicDiv" => self.builder.build_float_div(left, right, "algebraic.fdiv"),
                "algebraicRem" => self.builder.build_float_rem(left, right, "algebraic.frem"),
                _ => unreachable!(),
            }
            .map_err(|error| error.to_string())?;
            let value = result.as_value_ref();
            if unsafe { llvm_sys::core::LLVMCanValueUseFastMathFlags(value) } != 0 {
                unsafe {
                    llvm_sys::core::LLVMSetFastMathFlags(value, llvm_sys::LLVMFastMathAllowReassoc);
                }
            }
            return Ok(result.into());
        }

        if let Some(result) = self.emit_promise_instance_method(object, method, args) {
            return result;
        }

        if let Some(result) = self.emit_join_handle_instance_method(object, method, args) {
            return result;
        }

        if let Some(result) = self.emit_sender_instance_method(object, method, args) {
            return result;
        }

        if let Some(result) = self.emit_receiver_instance_method(object, method, args) {
            return result;
        }

        if let Some(result) = self.emit_stream_instance_method(object, method, args) {
            return result;
        }

        if let Some(result) = self.emit_mutex_instance_method(object, method, args) {
            return result;
        }

        if let Some(result) = self.emit_rwlock_instance_method(object, method, args) {
            return result;
        }

        if let Some(result) = self.emit_semaphore_instance_method(object, method, args) {
            return result;
        }

        if let Some(Type::Map(key, value)) = self.expr_type_hint(object) {
            let result = self.emit_map_method(object, &key, &value, method, args)?;
            match method {
                "get" => {
                    self.record_owned_native_result_with(
                        result,
                        Type::Optional(value),
                        crate::ReleasePlan::Deep,
                    );
                }
                "keys" => {
                    self.record_owned_native_result(result, Type::Array(key));
                }
                "values" => {
                    self.record_owned_native_result(result, Type::Array(value));
                }
                "entries" => {
                    self.record_owned_native_result(
                        result,
                        Type::Array(Box::new(Type::Tuple(vec![*key.clone(), *value.clone()]))),
                    );
                }
                _ => {}
            }
            return Ok(result);
        }

        if let Some(Type::Set(element)) = self.expr_type_hint(object) {
            let result = self.emit_set_method(object, &element, method, args)?;
            if method == "values" {
                self.record_owned_native_result(result, Type::Array(element));
            }
            return Ok(result);
        }

        if method == "contains"
            && let Some(Type::Range(element)) =
                self.expr_type_hint(object).map(|ty| self.resolve_type(&ty))
        {
            let range_value = self.emit_expr(object)?.into_struct_value();
            let needle = self.emit_expr_as(&args[0], &element)?;
            return self.emit_range_contains(range_value, needle, &element);
        }

        if matches!(
            self.expr_type_hint(object)
                .as_ref()
                .map(transparent_codegen_type),
            Some(Type::Array(_) | Type::Tuple(_))
        ) {
            let result = self.emit_array_method(object, method, args)?;
            if tyrus_core::async_types::array_method_allocates(method) {
                let call = Expr::synthetic(ExprKind::MethodCall {
                    object: Box::new(object.clone()),
                    method: method.to_string(),
                    method_span: tyrus_core::span::Span::DUMMY,
                    type_args: type_args.to_vec(),
                    args: args.to_vec(),
                });
                if let Some(ty) = self.expr_type_hint(&call).map(|ty| self.resolve_type(&ty)) {
                    match transparent_codegen_type(&ty) {
                        Type::Array(_) | Type::Tuple(_) | Type::Str => {
                            let plan = if method == "entries" {
                                crate::ReleasePlan::Buffer
                            } else {
                                crate::ReleasePlan::Deep
                            };
                            self.record_owned_native_result_with(result, ty, plan);
                        }
                        Type::Optional(_) => {
                            // `at`/`find`/`findLast` never remove the
                            // element, the array still owns it afterward,
                            // so a `string` inside the box is an alias the
                            // array will free later, and `Deep` here would
                            // double free it (confirmed: freed a `.rodata`
                            // address, "Invalid free()", the exact class of
                            // bug `range-slicing-owned-strings` closed for
                            // `.slice()`). Always `Value` for these three:
                            // free the box, never what it points to.
                            //
                            // `pop`/`shift` genuinely remove the element, so
                            // ownership transfers into the box *when the
                            // receiver's own elements are themselves owned*,
                            // a `string[]` literal with no computed element
                            // (`["a", "b"]`) never `strdup`s its elements at
                            // all, so popping one and freeing it the same way
                            // would hit the identical double free. The
                            // receiver's own registered plan already answers
                            // this (`Deep`/`Fields` means its elements were
                            // proven owned at construction; anything else,
                            // including no plan at all for a receiver this
                            // function cannot trace, means `Value` is the
                            // only safe choice).
                            //
                            // A `string[]` tracked by `string_array_ownership`
                            // proves the same thing without a registered plan,
                            // and proves it unconditionally: `emit_array_method`
                            // above has already normalized the receiver to mode
                            // `2` for a removal, precisely so the element leaving
                            // the array is an allocation rather than one of the
                            // literal's `.rodata` pointers. The promotion runs
                            // before this decision, so by here there is no
                            // static-element case left to guard against.
                            //
                            // The async frame keeps that same mode byte in a
                            // registry of its own, because an async local does
                            // not live in the scope stack the synchronous one
                            // walks. Asking only the synchronous registry made
                            // every `pop`/`shift` inside an `async` function fall
                            // to `Value`, which frees the box the element was
                            // moved into and leaves the element's own allocation
                            // behind, one leaked string per call, measured at
                            // 2400 bytes over 200 iterations.
                            let removal_root = matches!(method, "pop" | "shift")
                                .then(|| tyrus_core::ast::place_root_ident(object))
                                .flatten();
                            let removal_transfers_element = match removal_root {
                                Some(name) => {
                                    matches!(
                                        self.owned_native_plan_for(name),
                                        Some(
                                            crate::ReleasePlan::Deep
                                                | crate::ReleasePlan::Fields(_)
                                        )
                                    ) || self.string_array_ownership_slot(name).is_some()
                                        || self.async_frame_string_array_ownership(name).is_some()
                                }
                                None => false,
                            };
                            let plan = if removal_transfers_element {
                                crate::ReleasePlan::Deep
                            } else {
                                crate::ReleasePlan::Value
                            };
                            self.record_owned_native_result_with(result, ty, plan);
                        }
                        _ => {}
                    }
                }
            }
            return Ok(result);
        }

        let receiver_is_string = matches!(self.expr_type_hint(object), Some(Type::Str))
            || self
                .expr_type_hint(object)
                .and_then(|ty| self.payload_free_union_underlying(&ty))
                .is_some_and(|(_, underlying)| matches!(underlying, Type::Str));
        if receiver_is_string {
            let result = self.emit_string_method(object, method, args)?;
            // A method that builds a fresh string hands its allocation to the
            // caller. Recording that here is what lets the value be released
            // when it goes out of scope; without it every such call leaked, and
            // in a loop the leak grew with the iteration count.
            //
            // The list is explicit rather than "everything returning a string",
            // because a method that returns a borrowed or static pointer must
            // never be freed.
            if string_method_allocates(method) {
                self.record_owned_native_result(result, Type::Str);
            }
            if matches!(method, "split" | "graphemes") {
                self.record_owned_native_result(result, Type::Array(Box::new(Type::Str)));
            }
            if method == "chars" {
                self.record_owned_native_result(result, Type::Array(Box::new(Type::Char)));
            }
            if method == "matchAll" {
                self.record_owned_native_result(
                    result,
                    Type::Array(Box::new(Type::Array(Box::new(Type::Str)))),
                );
            }
            return Ok(result);
        }

        if let Some(result) = self.emit_native_handle_method(object, method, args)? {
            return Ok(result);
        }
        if let ExprKind::Ident(name) = object.kind()
            && name == "__tyrus_http_fetch"
        {
            if method == "stream" {
                if args.is_empty() || args.len() > 2 {
                    return Err(format!(
                        "error: `fetch.stream` expects 1 to 2 argument(s), found {}",
                        args.len()
                    ));
                }
                let url = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let options = match args.get(1) {
                    Some(options) => self.emit_fetch_options(Some(options), None)?,
                    None => pointer.const_null(),
                };
                let fetch = self.declare_libc_fn(
                    "tyrus_http_fetch_stream",
                    &[self.string_abi_type(), pointer.into()],
                    Some(pointer.into()),
                );
                let promise = self
                    .builder
                    .build_call(fetch, &[url.into(), options.into()], "http.fetch.stream")
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("internal: `fetch.stream` did not return a promise")?;
                self.emit_native_string_argument_release(url, &args[0])?;
                return Ok(promise);
            }
            if let Some(verb_method) = tyrus_core::http_body::fetch_verb_method(method) {
                if args.is_empty() || args.len() > 2 {
                    return Err(format!(
                        "error: `fetch.{method}` expects 1 to 2 argument(s), found {}",
                        args.len()
                    ));
                }
                let url = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let options = self.emit_fetch_options(args.get(1), Some(verb_method))?;
                let fetch = self.declare_libc_fn(
                    "tyrus_http_fetch",
                    &[self.string_abi_type(), pointer.into()],
                    Some(pointer.into()),
                );
                let promise = self
                    .builder
                    .build_call(fetch, &[url.into(), options.into()], "http.fetch.verb")
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("internal: `fetch` did not return a promise")?;
                self.emit_native_string_argument_release(url, &args[0])?;
                return Ok(promise);
            }
        }

        if let Some(result) = self.emit_network_handle_method(object, method, type_args, args)? {
            return Ok(result);
        }
        if let Some(result) = self.emit_http_reply_method(object, method, args) {
            return result;
        }

        if matches!(self.expr_type_hint(object), Some(Type::Regex)) {
            return self.emit_regex_method(object, method, args);
        }

        if let Some(result) = self.emit_crypto_namespace_method(object, method, type_args, args) {
            return result;
        }

        if let Some(ty) = self.expr_type_hint(object) {
            let is_algorithm = matches!(
                transparent_codegen_type(&ty),
                Type::BinaryHeap(_)
                    | Type::BinomialHeap(_)
                    | Type::UnionFind(_)
                    | Type::Counter(_)
                    | Type::Deque(_)
                    | Type::FenwickTree
                    | Type::BubbleSort
                    | Type::SegmentTree
                    | Type::SparseTable
                    | Type::SegmentTreeLazy
                    | Type::BitSet
                    | Type::Trie
                    | Type::BloomFilter
                    | Type::SuffixTree
                    | Type::Lca
                    | Type::Hld
                    | Type::Centroid
                    | Type::LinkedList(_)
                    | Type::DoubleLinkedList(_)
                    | Type::AvlTree(_)
                    | Type::RedBlackTree(_)
                    | Type::SplayTree(_)
                    | Type::Treap(_)
                    | Type::SortedSet(_)
                    | Type::SkipList(_)
                    | Type::SortedMap(_, _)
                    | Type::Graph(_, _)
            );
            if is_algorithm
                && let Some(result) = self.emit_algorithm_method_call(object, method, args)
            {
                return result;
            }
        }

        if let Some((params, ret)) = self.method_field_signature(object, method) {
            let closure = self.emit_expr(&Expr::synthetic(ExprKind::FieldAccess {
                object: Box::new(object.clone()),
                field: method.to_string(),
            }))?;
            let mut arg_values = Vec::with_capacity(args.len());
            for arg in args {
                arg_values.push(self.emit_expr(arg)?);
            }
            let result = self.emit_closure_call(closure, &params, &ret, &arg_values)?;
            return Ok(result.unwrap_or_else(|| self.context.i32_type().const_zero().into()));
        }

        Err(format!("error: unknown method call `{object:?}.{method}`"))
    }

    fn emit_object_projection_method(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let Some(source_type) = self.expr_type_hint(object) else {
            return Ok(None);
        };
        let Some(source_fields) = self.declared_fields(&source_type) else {
            return Ok(None);
        };
        let Some(keys) = args.first().and_then(|argument| {
            static_object_projection_keys(argument, self.expr_type_hint(argument))
        }) else {
            return Err(format!(
                "internal: checked object `{method}` projection has no static keys"
            ));
        };
        let target_fields = source_fields
            .iter()
            .filter(|field| {
                let selected = keys.iter().any(|key| key == &field.name);
                if method == "pick" {
                    selected
                } else {
                    !selected
                }
            })
            .cloned()
            .collect::<Vec<_>>();
        let target_type = Type::Object(target_fields.clone());
        let target_llvm_type = self.llvm_type(&target_type).into_struct_type();
        let target_slot = self.entry_alloca(target_llvm_type, "object.projection")?;
        let source = self.emit_expr(object)?.into_struct_value();

        for target_field in &target_fields {
            let source_index = self
                .struct_field_index(&source_type, &target_field.name)
                .ok_or_else(|| {
                    format!(
                        "internal: object projection source has no field `{}`",
                        target_field.name
                    )
                })?;
            let target_index = self
                .struct_field_index(&target_type, &target_field.name)
                .ok_or_else(|| {
                    format!(
                        "internal: object projection target has no field `{}`",
                        target_field.name
                    )
                })?;
            let field_type = self.resolve_type(&field_storage_type(target_field));
            let field_value = self
                .builder
                .build_extract_value(source, source_index, "object.projection.field")
                .map_err(|error| error.to_string())?;
            let field_slot = self
                .builder
                .build_struct_gep(
                    target_llvm_type,
                    target_slot,
                    target_index,
                    "object.projection.field.slot",
                )
                .map_err(|error| error.to_string())?;
            self.emit_async_clone_value_into(field_slot, field_value, &field_type)?;
        }

        let result = self
            .builder
            .build_load(target_llvm_type, target_slot, "object.projection.result")
            .map_err(|error| error.to_string())?;
        if self.async_owned_value_supported(&target_type) {
            self.record_owned_native_result(result, target_type);
        }
        Ok(Some(result))
    }

    pub(super) fn extension_method_for(
        &self,
        object: &Expr,
        method: &str,
    ) -> Option<&ExtensionMethod> {
        let object_type = self
            .expr_type_hint(object)
            .map(|ty| self.resolve_type(&ty))?;
        self.extension_methods.iter().rev().find(|extension| {
            extension.name == method && self.resolve_type(&extension.target) == object_type
        })
    }

    pub(super) fn automatic_data_method_for(
        &self,
        object: &Expr,
        method: &str,
    ) -> Option<(
        tyrus_core::automatic_data_methods::AutomaticDataMethod,
        Type,
    )> {
        let automatic = tyrus_core::automatic_data_methods::AutomaticDataMethod::instance(method)?;
        if !matches!(
            automatic,
            tyrus_core::automatic_data_methods::AutomaticDataMethod::Copy
                | tyrus_core::automatic_data_methods::AutomaticDataMethod::Debug
                | tyrus_core::automatic_data_methods::AutomaticDataMethod::Eq
                | tyrus_core::automatic_data_methods::AutomaticDataMethod::JsonEncode
        ) {
            return None;
        }
        let receiver = self.resolve_type(&self.expr_type_hint(object)?);
        let Type::Named(name) = &receiver else {
            return None;
        };
        self.declared_interfaces
            .contains_key(name)
            .then_some((automatic, receiver))
    }

    fn emit_automatic_debug(
        &mut self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
    ) -> Result<StructValue<'ctx>, String> {
        let resolved = self.resolve_type(ty);
        let fields = match transparent_codegen_type(&resolved) {
            Type::Named(name) => self.declared_fields_with_index(&resolved).ok_or_else(|| {
                format!("internal: automatic debug cannot find interface `{name}`")
            })?,
            Type::Object(fields) => fields
                .iter()
                .enumerate()
                .map(|(index, field)| (index as u32, field.clone()))
                .collect(),
            _ => {
                return Err(format!(
                    "internal: automatic debug needs an object value, found `{ty}`"
                ));
            }
        };
        let object = value.into_struct_value();
        let mut pieces = vec![self.const_string("{ ")];
        let mut temporary_values = Vec::new();
        for (position, (index, field)) in fields.iter().enumerate() {
            if position > 0 {
                pieces.push(self.const_string(", "));
            }
            pieces.push(self.const_string(&format!("{}: ", field.name)));
            let field_value = self
                .builder
                .build_extract_value(object, *index, "automatic.debug.field")
                .map_err(|error| error.to_string())?;
            let field_ty = self.resolve_type(&field_storage_type(field));
            let (rendered, is_temporary) =
                self.emit_automatic_debug_value(field_value, &field_ty)?;
            pieces.push(rendered);
            if is_temporary {
                temporary_values.push(rendered);
            }
        }
        pieces.push(self.const_string(" }"));
        let result = self.emit_string_join(&pieces, "automatic.debug")?;
        for temporary in temporary_values {
            self.emit_string_release(temporary)?;
        }
        Ok(result)
    }

    fn emit_automatic_debug_value(
        &mut self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
    ) -> Result<(StructValue<'ctx>, bool), String> {
        if matches!(transparent_codegen_type(ty), Type::Str) {
            return self.emit_automatic_debug_string(value.into_struct_value());
        }
        if matches!(transparent_codegen_type(ty), Type::Decimal) {
            let value = value.into_struct_value();
            let coefficient = self
                .builder
                .build_extract_value(value, 0, "automatic.debug.decimal.coefficient")
                .map_err(|error| error.to_string())?;
            let exponent = self
                .builder
                .build_extract_value(value, 1, "automatic.debug.decimal.exponent")
                .map_err(|error| error.to_string())?;
            let coefficient_slot = self.entry_alloca(
                self.context.i128_type(),
                "automatic.debug.decimal.coefficient",
            )?;
            self.builder
                .build_store(coefficient_slot, coefficient)
                .map_err(|error| error.to_string())?;
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let format = self.declare_libc_fn(
                "tyrus_decimal_to_string",
                &[pointer.into(), self.context.i32_type().into()],
                Some(self.string_abi_type()),
            );
            let rendered = self
                .builder
                .build_call(
                    format,
                    &[coefficient_slot.into(), exponent.into()],
                    "automatic.debug.decimal",
                )
                .map_err(|error| error.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or("internal: decimal formatter did not return a string")?
                .into_struct_value();
            return Ok((rendered, true));
        }
        if let Type::Optional(inner) = transparent_codegen_type(ty) {
            return self.emit_automatic_optional_debug(value, inner);
        }
        if let Type::Array(element) = transparent_codegen_type(ty) {
            return self
                .emit_automatic_array_debug(value, element)
                .map(|text| (text, true));
        }
        if let Type::Union(variants) = transparent_codegen_type(ty) {
            return self
                .emit_automatic_union_debug(value, variants)
                .map(|text| (text, true));
        }
        if let Type::Tuple(elements) = transparent_codegen_type(ty) {
            return self
                .emit_automatic_tuple_debug(value, elements)
                .map(|text| (text, true));
        }
        if matches!(
            transparent_codegen_type(ty),
            Type::Named(_) | Type::Object(_)
        ) {
            return self
                .emit_automatic_debug(value, ty)
                .map(|text| (text, true));
        }
        let rendered = self.emit_template_value_string(value, Some(ty))?;
        Ok((rendered, !matches!(transparent_codegen_type(ty), Type::Str)))
    }

    fn emit_automatic_debug_string(
        &mut self,
        value: StructValue<'ctx>,
    ) -> Result<(StructValue<'ctx>, bool), String> {
        let escape = self.declare_libc_fn(
            "tyrus_string_escape_debug",
            &[self.string_abi_type()],
            Some(self.string_abi_type()),
        );
        let escaped = self
            .builder
            .build_call(escape, &[value.into()], "automatic.debug.string.escape")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: automatic debug string escape returned no value")?
            .into_struct_value();
        let result = self.emit_string_join(
            &[self.const_string("\""), escaped, self.const_string("\"")],
            "automatic.debug.string",
        )?;
        self.emit_string_release(escaped)?;
        Ok((result, true))
    }

    fn emit_automatic_optional_debug(
        &mut self,
        value: BasicValueEnum<'ctx>,
        inner: &Type,
    ) -> Result<(StructValue<'ctx>, bool), String> {
        let pointer = value.into_pointer_value();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: automatic optional debug outside a function")?;
        let present = self
            .context
            .append_basic_block(function, "automatic.debug.optional.present");
        let absent = self
            .context
            .append_basic_block(function, "automatic.debug.optional.absent");
        let merge = self
            .context
            .append_basic_block(function, "automatic.debug.optional.merge");
        let is_present = self
            .builder
            .build_is_not_null(pointer, "automatic.debug.optional.is_present")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(is_present, present, absent)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(present);
        let present_value = self
            .builder
            .build_load(
                self.llvm_type(inner),
                pointer,
                "automatic.debug.optional.value",
            )
            .map_err(|error| error.to_string())?;
        let (rendered, temporary) = self.emit_automatic_debug_value(present_value, inner)?;
        let present_text =
            self.emit_string_clone(rendered, "automatic.debug.optional.present.clone")?;
        if temporary {
            self.emit_string_release(rendered)?;
        }
        let present_block = self.builder.get_insert_block().unwrap();
        self.builder
            .build_unconditional_branch(merge)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(absent);
        let absent_text = self.emit_string_clone(
            self.const_string("None"),
            "automatic.debug.optional.absent.clone",
        )?;
        let absent_block = self.builder.get_insert_block().unwrap();
        self.builder
            .build_unconditional_branch(merge)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(merge);
        let result = self
            .builder
            .build_phi(self.string_type(), "automatic.debug.optional.result")
            .map_err(|error| error.to_string())?;
        result.add_incoming(&[(&present_text, present_block), (&absent_text, absent_block)]);
        Ok((result.as_basic_value().into_struct_value(), true))
    }

    fn emit_automatic_array_debug(
        &mut self,
        value: BasicValueEnum<'ctx>,
        element: &Type,
    ) -> Result<StructValue<'ctx>, String> {
        let array = value.into_struct_value();
        let data = self
            .builder
            .build_extract_value(array, 0, "automatic.debug.array.data")
            .map_err(|error| error.to_string())?
            .into_pointer_value();
        let length = self
            .builder
            .build_extract_value(array, 1, "automatic.debug.array.length")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: automatic array debug outside a function")?;
        let index = self
            .builder
            .build_alloca(self.context.i64_type(), "automatic.debug.array.index")
            .map_err(|error| error.to_string())?;
        let text = self
            .builder
            .build_alloca(self.string_type(), "automatic.debug.array.text")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(index, self.context.i64_type().const_zero())
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(text, self.const_string("["))
            .map_err(|error| error.to_string())?;

        let condition = self
            .context
            .append_basic_block(function, "automatic.debug.array.condition");
        let body = self
            .context
            .append_basic_block(function, "automatic.debug.array.body");
        let done = self
            .context
            .append_basic_block(function, "automatic.debug.array.done");
        self.builder
            .build_unconditional_branch(condition)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(condition);
        let current_index = self
            .builder
            .build_load(
                self.context.i64_type(),
                index,
                "automatic.debug.array.index.current",
            )
            .map_err(|error| error.to_string())?
            .into_int_value();
        let has_element = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                current_index,
                length,
                "automatic.debug.array.has_element",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(has_element, body, done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(body);
        let current_text = self
            .builder
            .build_load(
                self.string_type(),
                text,
                "automatic.debug.array.text.current",
            )
            .map_err(|error| error.to_string())?
            .into_struct_value();
        let element_slot = unsafe {
            self.builder
                .build_gep(
                    self.llvm_type(element),
                    data,
                    &[current_index],
                    "automatic.debug.array.element",
                )
                .map_err(|error| error.to_string())?
        };
        let element_value = self
            .builder
            .build_load(
                self.llvm_type(element),
                element_slot,
                "automatic.debug.array.element.value",
            )
            .map_err(|error| error.to_string())?;
        let (rendered, temporary) = self.emit_automatic_debug_value(element_value, element)?;
        let is_first = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                current_index,
                self.context.i64_type().const_zero(),
                "automatic.debug.array.is_first",
            )
            .map_err(|error| error.to_string())?;
        let separator = self
            .builder
            .build_select(
                is_first,
                self.const_string(""),
                self.const_string(", "),
                "automatic.debug.array.separator",
            )
            .map_err(|error| error.to_string())?
            .into_struct_value();
        let next_text = self.emit_string_join(
            &[current_text, separator, rendered],
            "automatic.debug.array.join",
        )?;
        self.emit_string_release(current_text)?;
        if temporary {
            self.emit_string_release(rendered)?;
        }
        self.builder
            .build_store(text, next_text)
            .map_err(|error| error.to_string())?;
        let next_index = self
            .builder
            .build_int_add(
                current_index,
                self.context.i64_type().const_int(1, false),
                "automatic.debug.array.index.next",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(index, next_index)
            .map_err(|error| error.to_string())?;
        self.builder
            .build_unconditional_branch(condition)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(done);
        let current_text = self
            .builder
            .build_load(self.string_type(), text, "automatic.debug.array.text.done")
            .map_err(|error| error.to_string())?
            .into_struct_value();
        let result = self.emit_string_join(
            &[current_text, self.const_string("]")],
            "automatic.debug.array.end",
        )?;
        self.emit_string_release(current_text)?;
        Ok(result)
    }

    fn emit_automatic_union_debug(
        &mut self,
        value: BasicValueEnum<'ctx>,
        variants: &[Type],
    ) -> Result<StructValue<'ctx>, String> {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: automatic union debug outside a function")?;
        let slot = self
            .builder
            .build_alloca(self.union_llvm_type(variants), "automatic.debug.union")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(slot, value)
            .map_err(|error| error.to_string())?;
        let tag = self.load_union_tag(variants, slot)?;
        let merge = self
            .context
            .append_basic_block(function, "automatic.debug.union.merge");
        let mut arms = Vec::with_capacity(variants.len() + 1);
        let mut next_test = self
            .context
            .append_basic_block(function, "automatic.debug.union.test.0");
        self.builder
            .build_unconditional_branch(next_test)
            .map_err(|error| error.to_string())?;

        for (index, variant) in variants.iter().enumerate() {
            let render = self
                .context
                .append_basic_block(function, &format!("automatic.debug.union.render.{index}"));
            let miss = if index + 1 == variants.len() {
                self.context
                    .append_basic_block(function, "automatic.debug.union.unreachable")
            } else {
                self.context.append_basic_block(
                    function,
                    &format!("automatic.debug.union.test.{}", index + 1),
                )
            };
            self.builder.position_at_end(next_test);
            let matches = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    tag,
                    self.context.i8_type().const_int(index as u64, false),
                    &format!("automatic.debug.union.is.{index}"),
                )
                .map_err(|error| error.to_string())?;
            self.builder
                .build_conditional_branch(matches, render, miss)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(render);
            let payload = self.load_union_payload(
                variants,
                slot,
                variant,
                &format!("automatic.debug.union.value.{index}"),
            )?;
            let (rendered, temporary) = self.emit_automatic_debug_value(payload, variant)?;
            let text = self.emit_string_clone(rendered, "automatic.debug.union.clone")?;
            if temporary {
                self.emit_string_release(rendered)?;
            }
            let incoming = self.builder.get_insert_block().unwrap();
            self.builder
                .build_unconditional_branch(merge)
                .map_err(|error| error.to_string())?;
            arms.push((text, incoming));
            next_test = miss;
        }

        self.builder.position_at_end(next_test);
        let fallback = self.const_string("None");
        self.builder
            .build_unconditional_branch(merge)
            .map_err(|error| error.to_string())?;
        arms.push((fallback.into(), next_test));

        self.builder.position_at_end(merge);
        let result = self
            .builder
            .build_phi(self.string_type(), "automatic.debug.union.result")
            .map_err(|error| error.to_string())?;
        for (text, block) in &arms {
            result.add_incoming(&[(text as &dyn inkwell::values::BasicValue, *block)]);
        }
        Ok(result.as_basic_value().into_struct_value())
    }

    fn emit_automatic_tuple_debug(
        &mut self,
        value: BasicValueEnum<'ctx>,
        elements: &[Type],
    ) -> Result<StructValue<'ctx>, String> {
        let tuple = value.into_struct_value();
        let mut pieces = vec![self.const_string("[")];
        let mut temporary_values = Vec::new();
        for (index, element) in elements.iter().enumerate() {
            if index > 0 {
                pieces.push(self.const_string(", "));
            }
            let element_value = self
                .builder
                .build_extract_value(tuple, index as u32, "automatic.debug.tuple.element")
                .map_err(|error| error.to_string())?;
            let (rendered, temporary) = self.emit_automatic_debug_value(element_value, element)?;
            pieces.push(rendered);
            if temporary {
                temporary_values.push(rendered);
            }
        }
        pieces.push(self.const_string("]"));
        let result = self.emit_string_join(&pieces, "automatic.debug.tuple")?;
        for temporary in temporary_values {
            self.emit_string_release(temporary)?;
        }
        Ok(result)
    }

    pub(super) fn extension_method_return_type(&self, object: &Expr, method: &str) -> Option<Type> {
        let symbol = self.extension_method_for(object, method)?.symbol.clone();
        self.fn_signatures
            .get(&symbol)
            .map(|(_, ret)| ret.clone())
            .or_else(|| {
                self.load_var(&symbol).map(|(_, ty)| ty).and_then(|ty| {
                    match transparent_codegen_type(&ty) {
                        Type::Function { ret, .. } => Some((**ret).clone()),
                        _ => None,
                    }
                })
            })
    }

    fn method_field_signature(&self, object: &Expr, method: &str) -> Option<(Vec<Type>, Type)> {
        let object_ty = self.resolve_type(&self.expr_type_hint(object)?);
        let fields = match object_ty {
            Type::Named(name) => self.interfaces.get(&name)?.clone(),
            Type::Object(fields) => fields,
            _ => return None,
        };
        match &fields.iter().find(|field| field.name == method)?.ty {
            Type::Function { params, ret } => Some((params.clone(), (**ret).clone())),
            _ => None,
        }
    }

    fn emit_crypto_namespace_method(
        &mut self,
        object: &Expr,
        method: &str,
        type_args: &[Type],
        args: &[Expr],
    ) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        if let ExprKind::Ident(namespace) = object.kind()
            && matches!(
                namespace.as_str(),
                "utf8" | "utf16" | "hex" | "base64" | "base64Url" | "base32" | "radix"
            )
            && let Some(target) = tyrus_core::encoding::encoding_method_target(namespace, method)
        {
            return Some(self.emit_encoding_runtime_call(target, args));
        }
        let obj_ty = self.expr_type_hint(object)?;
        let ty_name = match &obj_ty {
            Type::Named(name) => name.clone(),
            _ => return None,
        };

        if tyrus_core::crypto::is_crypto_namespace(&ty_name) {
            let import_name =
                tyrus_core::crypto::crypto_type_to_import_name(&ty_name).unwrap_or(&ty_name);
            if let Some(target) = tyrus_core::crypto::crypto_method_target(import_name, method) {
                return Some(self.emit_crypto_runtime_call_typed(target, type_args, args));
            }
        }

        if matches!(
            ty_name.as_str(),
            "Utf8Namespace"
                | "Utf16Namespace"
                | "HexNamespace"
                | "Base64Namespace"
                | "Base64UrlNamespace"
                | "Base32Namespace"
                | "RadixNamespace"
        ) {
            let namespace = match ty_name.as_str() {
                "Utf8Namespace" => "utf8",
                "Utf16Namespace" => "utf16",
                "HexNamespace" => "hex",
                "Base64Namespace" => "base64",
                "Base64UrlNamespace" => "base64Url",
                "Base32Namespace" => "base32",
                _ => "radix",
            };
            if let Some(target) = tyrus_core::encoding::encoding_method_target(namespace, method) {
                return Some(self.emit_encoding_runtime_call(target, args));
            }
        }

        if ty_name == "DigestStream"
            && let Some(target) = tyrus_core::crypto::digest_stream_method_target(method)
        {
            let mut full_args = vec![object.clone()];
            full_args.extend(args.iter().cloned());
            return Some(self.emit_crypto_runtime_call_typed(target, type_args, &full_args));
        }

        if let ExprKind::FieldAccess {
            object: inner_obj,
            field: inner_field,
        } = object.kind()
        {
            let inner_ty_name = self.expr_type_hint(inner_obj).and_then(|ty| match ty {
                Type::Named(name) => Some(name),
                _ => None,
            })?;
            if tyrus_core::crypto::is_crypto_namespace(&inner_ty_name)
                && let Some(inner_ns_name) =
                    tyrus_core::crypto::nested_crypto_inner_type_name(&inner_ty_name, inner_field)
                && let Some(target) =
                    tyrus_core::crypto::nested_crypto_method_target(inner_ns_name, method)
            {
                return Some(self.emit_crypto_runtime_call_typed(target, type_args, args));
            }
        }

        if let Some(result) = self.emit_algorithm_method_call(object, method, args) {
            return Some(result);
        }

        None
    }

    pub(super) fn emit_console_log(
        &mut self,
        args: &[Expr],
        _obj: &str,
        method: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let output = if matches!(method, "error" | "warn") {
            ConsoleOutput::Stderr
        } else {
            ConsoleOutput::Stdout
        };
        match method {
            "error" => self.emit_colored_console_line(args, output, "red")?,
            "warn" => self.emit_colored_console_line(args, output, "yellow")?,
            _ => self.emit_print_line(args, output, true)?,
        }
        Ok(self.context.i32_type().const_int(0, false).into())
    }

    fn emit_colored_console_line(
        &mut self,
        args: &[Expr],
        output: ConsoleOutput,
        color: &str,
    ) -> Result<(), String> {
        let open = tyrus_core::style_text::ansi_open(&[color.to_string()])
            .map_err(|name| format!("internal: unknown text style `{name}`"))?;
        self.builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or_else(|| "internal: colored console line outside a function".to_string())?;
        let color_check_fn = self.declare_libc_fn(
            "tyrus_console_stderr_color_enabled",
            &[],
            Some(self.context.bool_type().into()),
        );
        let enabled = self
            .builder
            .build_call(color_check_fn, &[], "console.color.enabled")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();

        if self.print_line_has_union_arg(args) {
            self.emit_color_branch(enabled, &open, "console.color.open.seq", output);
            self.emit_print_line_unlocked(args, output, false)?;
            self.emit_color_branch(
                enabled,
                tyrus_core::style_text::ANSI_RESET,
                "console.color.reset.seq",
                output,
            );
            self.emit_literal_to("\n", "console.newline", output);
            return Ok(());
        }

        let pieces = self.collect_print_pieces(args)?;
        self.emit_lock_begin(output);
        self.emit_color_branch(enabled, &open, "console.color.open.seq", output);
        self.emit_print_pieces(&pieces, args, output)?;
        self.emit_color_branch(
            enabled,
            tyrus_core::style_text::ANSI_RESET,
            "console.color.reset.seq",
            output,
        );
        self.emit_literal_to("\n", "console.newline", output);
        self.emit_lock_end(output);
        Ok(())
    }

    fn emit_color_branch(
        &mut self,
        enabled: IntValue<'ctx>,
        text: &str,
        name: &str,
        output: ConsoleOutput,
    ) {
        let Some(function) = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
        else {
            return;
        };
        let then_bb = self.context.append_basic_block(function, name);
        let after_bb = self
            .context
            .append_basic_block(function, &format!("{name}.after"));
        self.builder
            .build_conditional_branch(enabled, then_bb, after_bb)
            .unwrap();
        self.builder.position_at_end(then_bb);
        self.emit_literal_to(text, name, output);
        self.builder.build_unconditional_branch(after_bb).unwrap();
        self.builder.position_at_end(after_bb);
    }

    pub(super) fn emit_print_builtin(
        &mut self,
        callee: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        match callee {
            "print" => {
                self.emit_print_line(args, ConsoleOutput::Stdout, true)?;
            }
            "printError" => {
                self.emit_colored_console_line(args, ConsoleOutput::Stderr, "red")?;
            }
            "printWarn" => {
                self.emit_colored_console_line(args, ConsoleOutput::Stderr, "yellow")?;
            }
            "printRaw" => {
                self.emit_print_line(args, ConsoleOutput::Stdout, false)?;
                self.emit_stdout_flush();
            }
            "printCR" => {
                self.emit_print_cr(args)?;
            }
            _ => return Ok(None),
        }
        Ok(Some(self.context.i32_type().const_int(0, false).into()))
    }

    fn emit_print_cr(&mut self, args: &[Expr]) -> Result<(), String> {
        let is_terminal_fn = self.declare_libc_fn(
            "tyrus_stdout_is_terminal",
            &[],
            Some(self.context.bool_type().into()),
        );
        let is_terminal = self
            .builder
            .build_call(is_terminal_fn, &[], "printcr.is_terminal")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        self.emit_color_branch(
            is_terminal,
            "\r\x1B[K",
            "printcr.clear.seq",
            ConsoleOutput::Stdout,
        );
        self.emit_print_line(args, ConsoleOutput::Stdout, false)?;
        self.emit_stdout_flush();
        Ok(())
    }

    fn emit_stdout_flush(&mut self) {
        if cfg!(windows) && self.target == CompileTarget::Native {
            let i32_type = self.context.i32_type();
            let flush = self.declare_libc_fn(
                "tyrus_stdio_flush",
                &[i32_type.into()],
                Some(i32_type.into()),
            );
            self.builder
                .build_call(flush, &[i32_type.const_int(1, false).into()], "")
                .unwrap();
            return;
        }
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let f = self.declare_libc_fn(
            "fflush",
            &[ptr_ty.into()],
            Some(self.context.i32_type().into()),
        );
        let null = ptr_ty.const_null();
        self.builder.build_call(f, &[null.into()], "").unwrap();
    }

    fn emit_print_line(
        &mut self,
        args: &[Expr],
        output: ConsoleOutput,
        newline: bool,
    ) -> Result<(), String> {
        if self.print_line_has_union_arg(args) {
            return self.emit_print_line_unlocked(args, output, newline);
        }
        let pieces = self.collect_print_pieces(args)?;
        self.emit_lock_begin(output);
        self.emit_print_pieces(&pieces, args, output)?;
        if newline {
            self.emit_literal_to("\n", "print.newline", output);
        }
        self.emit_lock_end(output);
        Ok(())
    }

    fn print_line_has_union_arg(&self, args: &[Expr]) -> bool {
        args.iter().any(|arg| {
            if matches!(arg.kind(), ExprKind::StrLit(_)) {
                return false;
            }
            let type_hint = self
                .expr_type_hint(arg)
                .map(|hint| self.resolve_type(&hint));
            matches!(
                type_hint.as_ref().map(transparent_codegen_type).cloned(),
                Some(Type::Union(_))
            )
        })
    }

    fn emit_print_line_unlocked(
        &mut self,
        args: &[Expr],
        output: ConsoleOutput,
        newline: bool,
    ) -> Result<(), String> {
        let mut literal_run = String::new();
        for (index, arg) in args.iter().enumerate() {
            match arg.kind() {
                ExprKind::StrLit(s) => {
                    if index > 0 {
                        literal_run.push(' ');
                    }
                    literal_run.push_str(s);
                }
                _ => {
                    if !literal_run.is_empty() {
                        self.emit_literal_to(&literal_run, "print.str", output);
                        literal_run.clear();
                    }
                    if index > 0 {
                        self.emit_literal_to(" ", "print.arg.space", output);
                    }
                    let type_hint = self
                        .expr_type_hint(arg)
                        .map(|hint| self.resolve_type(&hint));
                    let val = self.emit_expr(arg)?;
                    self.emit_print_value_inline_to(
                        val,
                        type_hint.as_ref().unwrap_or(&Type::Unknown),
                        false,
                        output,
                    )?;
                    if !self.async_results.is_empty()
                        && let Some(ty) = type_hint
                            .as_ref()
                            .filter(|ty| self.async_owned_value_supported(ty))
                    {
                        self.emit_async_cleanup_source_expression(val, ty, arg)?;
                    }
                }
            }
        }
        if !literal_run.is_empty() {
            self.emit_literal_to(&literal_run, "print.str", output);
        }
        if newline {
            self.emit_literal_to("\n", "print.newline", output);
        }
        Ok(())
    }

    fn collect_print_pieces(&mut self, args: &[Expr]) -> Result<Vec<PrintPiece<'ctx>>, String> {
        let mut pieces = Vec::new();
        let mut literal_run = String::new();
        for (index, arg) in args.iter().enumerate() {
            match arg.kind() {
                ExprKind::StrLit(s) => {
                    if index > 0 {
                        literal_run.push(' ');
                    }
                    literal_run.push_str(s);
                }
                _ => {
                    if !literal_run.is_empty() {
                        pieces.push(PrintPiece::Literal {
                            text: std::mem::take(&mut literal_run),
                            name: "print.str",
                        });
                    }
                    if index > 0 {
                        pieces.push(PrintPiece::Literal {
                            text: " ".to_string(),
                            name: "print.arg.space",
                        });
                    }
                    let type_hint = self
                        .expr_type_hint(arg)
                        .map_or(Type::Unknown, |hint| self.resolve_type(&hint));
                    let val = self.emit_expr(arg)?;
                    pieces.push(PrintPiece::Value {
                        val,
                        type_hint,
                        arg_index: index,
                    });
                }
            }
        }
        if !literal_run.is_empty() {
            pieces.push(PrintPiece::Literal {
                text: literal_run,
                name: "print.str",
            });
        }
        Ok(pieces)
    }

    fn emit_print_pieces(
        &mut self,
        pieces: &[PrintPiece<'ctx>],
        args: &[Expr],
        output: ConsoleOutput,
    ) -> Result<(), String> {
        for piece in pieces {
            match piece {
                PrintPiece::Literal { text, name } => self.emit_literal_to(text, name, output),
                PrintPiece::Value {
                    val,
                    type_hint,
                    arg_index,
                } => {
                    self.emit_print_value_inline_to(*val, type_hint, false, output)?;
                    if !matches!(args[*arg_index].kind(), ExprKind::Ident(_))
                        && !self.async_results.is_empty()
                        && self.async_owned_value_supported(type_hint)
                    {
                        self.emit_async_cleanup_source_expression(
                            *val,
                            type_hint,
                            &args[*arg_index],
                        )?;
                    }
                }
            }
        }
        Ok(())
    }

    fn emit_lock_begin(&mut self, output: ConsoleOutput) {
        let name = match output {
            ConsoleOutput::Stdout => "tyrus_stdout_lock_begin",
            ConsoleOutput::Stderr => "tyrus_stderr_lock_begin",
        };
        let f = self.declare_libc_fn(name, &[], None);
        self.builder.build_call(f, &[], "").unwrap();
    }

    fn emit_lock_end(&mut self, output: ConsoleOutput) {
        let name = match output {
            ConsoleOutput::Stdout => "tyrus_stdout_lock_end",
            ConsoleOutput::Stderr => "tyrus_stderr_lock_end",
        };
        let f = self.declare_libc_fn(name, &[], None);
        self.builder.build_call(f, &[], "").unwrap();
    }

    fn emit_console_log_union(
        &mut self,
        union_ptr: PointerValue<'ctx>,
        variants: &[Type],
        output: ConsoleOutput,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let tag = self.load_union_tag(variants, union_ptr)?;
        let i8_ty = self.context.i8_type();

        let fn_val = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: console.log union outside function")?;

        let merge_bb = self.context.append_basic_block(fn_val, "log.merge");

        let first_cmp = self.context.append_basic_block(fn_val, "log.cmp.0");
        self.builder.build_unconditional_branch(first_cmp).unwrap();

        let mut prev_cmp_bb = first_cmp;

        for i in 0..variants.len() {
            let print_bb = self
                .context
                .append_basic_block(fn_val, &format!("log.print.{i}"));
            let miss_bb = if i < variants.len() - 1 {
                self.context
                    .append_basic_block(fn_val, &format!("log.cmp.{}", i + 1))
            } else {
                merge_bb
            };

            self.builder.position_at_end(prev_cmp_bb);
            let cmp = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    tag,
                    i8_ty.const_int(i as u64, false),
                    &format!("log.eq.{i}"),
                )
                .unwrap();
            self.builder
                .build_conditional_branch(cmp, print_bb, miss_bb)
                .unwrap();

            self.builder.position_at_end(print_bb);
            let payload = self.load_union_payload(
                variants,
                union_ptr,
                &variants[i],
                &format!("log.val.{i}"),
            )?;
            self.emit_print_value_inline_to(payload, &variants[i], false, output)?;
            self.builder.build_unconditional_branch(merge_bb).unwrap();

            prev_cmp_bb = miss_bb;
        }

        self.builder.position_at_end(merge_bb);
        Ok(self.context.i32_type().const_int(0, false).into())
    }

    fn emit_print_value_inline_to(
        &mut self,
        val: BasicValueEnum<'ctx>,
        hint: &Type,
        quote_strings: bool,
        output: ConsoleOutput,
    ) -> Result<(), String> {
        if val.is_struct_value() && val.into_struct_value().get_type() == self.string_type()
            || matches!(hint, Type::Regex)
        {
            if quote_strings {
                self.emit_print_string_quoted_to(val.into_struct_value(), output);
            } else {
                self.emit_print_string_to(val.into_struct_value(), output);
            }
        } else if matches!(hint, Type::Bool) {
            let true_value = self
                .const_string_abi("true", "bool.true")
                .into_struct_value();
            let false_value = self
                .const_string_abi("false", "bool.false")
                .into_struct_value();
            let text = self
                .builder
                .build_select(val.into_int_value(), true_value, false_value, "bool.text")
                .unwrap();
            self.emit_print_string_to(text.into_struct_value(), output);
        } else if matches!(transparent_codegen_type(hint), Type::Char) && val.is_int_value() {
            let text = self.emit_format_char(val.into_int_value());
            self.emit_print_string_to(text, output);
            self.emit_string_release(text)
                .expect("releasing a rendered string is always emittable");
        } else if val.is_int_value() {
            let is_unsigned = is_unsigned_int_type(hint);
            let int_val = self.promote_int_for_printf(val.into_int_value(), is_unsigned);
            let bits = int_val.get_type().get_bit_width();
            let (fmt_str, display_val): (&str, BasicValueEnum<'ctx>) = match (bits, is_unsigned) {
                (32, true) => ("%u", int_val.into()),
                (64, true) => ("%llu", int_val.into()),
                (128, true) => {
                    let print = self.declare_print_u128(output);
                    self.builder
                        .build_call(print, &[int_val.into()], "")
                        .unwrap();
                    return Ok(());
                }
                (32, false) => ("%d", int_val.into()),
                (64, false) => ("%lld", int_val.into()),
                (128, false) => {
                    let print = self.declare_print_i128(output);
                    self.builder
                        .build_call(print, &[int_val.into()], "")
                        .unwrap();
                    return Ok(());
                }
                _ => ("%d", int_val.into()),
            };
            self.emit_printf_to(fmt_str, &[display_val.into()], "fmt", output);
        } else if val.is_float_value() {
            let float = val.into_float_value();
            if matches!(transparent_codegen_type(hint), Type::F128) {
                let print = self.declare_print_f128(output);
                self.builder.build_call(print, &[float.into()], "").unwrap();
                return Ok(());
            }
            let text = self.emit_format_f64(float);
            self.emit_print_string_to(text, output);
            self.emit_string_release(text)
                .expect("releasing a rendered string is always emittable");
        } else if val.is_struct_value() {
            match transparent_codegen_type(hint).clone() {
                Type::Decimal => {
                    self.emit_stdout_flush();
                    let value = val.into_struct_value();
                    let coefficient = self
                        .builder
                        .build_extract_value(value, 0, "print.decimal.coefficient")
                        .unwrap();
                    let coefficient_slot = self
                        .entry_alloca(self.context.i128_type(), "print.decimal.coefficient.slot")?;
                    self.builder
                        .build_store(coefficient_slot, coefficient)
                        .unwrap();
                    let exponent = self
                        .builder
                        .build_extract_value(value, 1, "print.decimal.exponent")
                        .unwrap();
                    let fd = match output {
                        ConsoleOutput::Stdout => 1,
                        ConsoleOutput::Stderr => 2,
                    };
                    let print = self.declare_libc_fn(
                        "tyrus_decimal_print",
                        &[
                            self.context
                                .ptr_type(inkwell::AddressSpace::default())
                                .into(),
                            self.context.i32_type().into(),
                            self.context.i32_type().into(),
                        ],
                        None,
                    );
                    self.builder
                        .build_call(
                            print,
                            &[
                                coefficient_slot.into(),
                                exponent.into(),
                                self.context.i32_type().const_int(fd, false).into(),
                            ],
                            "",
                        )
                        .unwrap();
                }
                Type::Named(name) if self.interfaces.contains_key(name.as_str()) => {
                    self.emit_print_struct_to(val.into_struct_value(), &name, false, output);
                }
                Type::Object(fields) => {
                    self.emit_print_object_fields_to(
                        val.into_struct_value(),
                        &fields,
                        false,
                        output,
                    );
                }
                Type::Tuple(elements) => {
                    self.emit_print_tuple_to(val.into_struct_value(), &elements, false, output);
                }
                Type::Array(element) => {
                    self.emit_print_array_to(val.into_struct_value(), &element, false, output);
                }
                Type::Union(variants) => {
                    let slot = self
                        .builder
                        .build_alloca(self.union_llvm_type(&variants), "print.union")
                        .map_err(|error| error.to_string())?;
                    self.builder
                        .build_store(slot, val)
                        .map_err(|error| error.to_string())?;
                    self.emit_console_log_union(slot, &variants, output)?;
                }
                other => {
                    let name = match &other {
                        Type::Named(name) => {
                            tyrus_core::module_loader::original_global_name(name).to_string()
                        }
                        other => other.to_string(),
                    };
                    return Err(format!(
                        "error: `console.log` cannot render a value of type `{name}` yet; format the parts you want to see, or add a `toString()` to the type"
                    ));
                }
            }
        } else if val.is_pointer_value() {
            if quote_strings {
                self.emit_printf_to("\"%s\"", &[val.into()], "print.ptr.quoted", output);
            } else {
                self.emit_printf_to("%s", &[val.into()], "print.ptr", output);
            }
        }
        Ok(())
    }

    fn emit_print_tuple_to(
        &self,
        tuple_val: StructValue<'ctx>,
        elements: &[Type],
        newline: bool,
        output: ConsoleOutput,
    ) {
        let emit_literal = |text: &str, name: &str| {
            self.emit_literal_to(text, name, output);
        };
        emit_literal("[", "tuple.open");
        for (index, element) in elements.iter().enumerate() {
            if index > 0 {
                emit_literal(", ", "tuple.sep");
            }
            let value = self
                .builder
                .build_extract_value(tuple_val, index as u32, "tuple_element")
                .unwrap();
            self.emit_print_scalar_inline_to(value, element, output);
        }
        emit_literal(if newline { "]\n" } else { "]" }, "tuple.close");
    }

    fn emit_print_array_to(
        &self,
        array_val: StructValue<'ctx>,
        element: &Type,
        newline: bool,
        output: ConsoleOutput,
    ) {
        let emit_literal = |text: &str, name: &str| {
            self.emit_literal_to(text, name, output);
        };

        let Some(function) = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
        else {
            return;
        };

        let data = self
            .builder
            .build_extract_value(array_val, 0, "array.print.data")
            .unwrap()
            .into_pointer_value();
        let length = self
            .builder
            .build_extract_value(array_val, 1, "array.print.len")
            .unwrap()
            .into_int_value();

        let idx_type = self.usize_type();
        let length = self.truncate_to_usize(length);
        let element_llvm = self.llvm_type(element);
        let index_slot = self
            .entry_alloca(idx_type, "array.print.idx")
            .expect("an entry-block slot inside a function is always emittable");
        self.builder
            .build_store(index_slot, idx_type.const_zero())
            .unwrap();

        emit_literal("[", "array.print.open");

        let cond_bb = self
            .context
            .append_basic_block(function, "array.print.cond");
        let body_bb = self
            .context
            .append_basic_block(function, "array.print.body");
        let end_bb = self.context.append_basic_block(function, "array.print.end");
        self.builder.build_unconditional_branch(cond_bb).unwrap();

        self.builder.position_at_end(cond_bb);
        let index = self
            .builder
            .build_load(idx_type, index_slot, "array.print.i")
            .unwrap()
            .into_int_value();
        let in_bounds = self
            .builder
            .build_int_compare(IntPredicate::SLT, index, length, "array.print.cmp")
            .unwrap();
        self.builder
            .build_conditional_branch(in_bounds, body_bb, end_bb)
            .unwrap();

        self.builder.position_at_end(body_bb);
        let is_first = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                index,
                idx_type.const_zero(),
                "array.print.first",
            )
            .unwrap();
        let sep_bb = self.context.append_basic_block(function, "array.print.sep");
        let elem_bb = self
            .context
            .append_basic_block(function, "array.print.elem");
        self.builder
            .build_conditional_branch(is_first, elem_bb, sep_bb)
            .unwrap();

        self.builder.position_at_end(sep_bb);
        emit_literal(", ", "array.print.sep.text");
        self.builder.build_unconditional_branch(elem_bb).unwrap();

        self.builder.position_at_end(elem_bb);
        let slot = unsafe {
            self.builder
                .build_gep(element_llvm, data, &[index], "array.print.slot")
                .unwrap()
        };
        let element_value = self
            .builder
            .build_load(element_llvm, slot, "array.print.value")
            .unwrap();
        self.emit_print_scalar_inline_to(element_value, element, output);
        let next = self
            .builder
            .build_int_add(index, idx_type.const_int(1, false), "array.print.next")
            .unwrap();
        self.builder.build_store(index_slot, next).unwrap();
        self.builder.build_unconditional_branch(cond_bb).unwrap();

        self.builder.position_at_end(end_bb);
        emit_literal(if newline { "]\n" } else { "]" }, "array.print.close");

        if let Some(last) = function.get_last_basic_block()
            && last != end_bb
        {
            let _ = end_bb.move_after(last);
        }
    }

    pub(super) fn emit_format_f64(&self, float: FloatValue<'ctx>) -> StructValue<'ctx> {
        let f64_type = self.context.f64_type();
        if float.get_type() == self.context.f32_type() {
            let format_f32 = self.declare_libc_fn(
                "tyrus_format_f32",
                &[self.context.f32_type().into()],
                Some(self.string_abi_type()),
            );
            return self
                .builder
                .build_call(format_f32, &[float.into()], "float_fmt")
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap()
                .into_struct_value();
        }
        let f64_val = if float.get_type() == f64_type {
            float
        } else if float.get_type().get_bit_width() < f64_type.get_bit_width() {
            self.builder
                .build_float_ext(float, f64_type, "float_fmt_ext")
                .unwrap()
        } else {
            self.builder
                .build_float_trunc(float, f64_type, "float_fmt_trunc")
                .unwrap()
        };
        let format_fn = self.declare_libc_fn(
            "tyrus_format_f64",
            &[f64_type.into()],
            Some(self.string_abi_type()),
        );
        self.builder
            .build_call(format_fn, &[f64_val.into()], "float_fmt")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_struct_value()
    }

    pub(super) fn emit_format_f64_precision(
        &self,
        float: FloatValue<'ctx>,
        precision: u32,
    ) -> StructValue<'ctx> {
        let f64_type = self.context.f64_type();
        let f64_val = if float.get_type() == f64_type {
            float
        } else if float.get_type().get_bit_width() < f64_type.get_bit_width() {
            self.builder
                .build_float_ext(float, f64_type, "float_fmt_precision_ext")
                .unwrap()
        } else {
            self.builder
                .build_float_trunc(float, f64_type, "float_fmt_precision_trunc")
                .unwrap()
        };
        let i32_type = self.context.i32_type();
        let format_fn = self.declare_libc_fn(
            "tyrus_format_f64_precision",
            &[f64_type.into(), i32_type.into()],
            Some(self.string_abi_type()),
        );
        self.builder
            .build_call(
                format_fn,
                &[
                    f64_val.into(),
                    i32_type.const_int(precision as u64, false).into(),
                ],
                "float_fmt_precision",
            )
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_struct_value()
    }

    pub(super) fn emit_format_int_padded(
        &self,
        integer: IntValue<'ctx>,
        hint: Option<&Type>,
        width: u32,
    ) -> StructValue<'ctx> {
        let i128_type = self.context.i128_type();
        let unsigned = hint.is_some_and(is_unsigned_int_type);
        let (widened, formatter) = if unsigned {
            (
                self.builder
                    .build_int_z_extend(integer, i128_type, "format_padded.u128")
                    .unwrap(),
                "tyrus_format_u128_padded",
            )
        } else {
            (
                self.builder
                    .build_int_s_extend(integer, i128_type, "format_padded.i128")
                    .unwrap(),
                "tyrus_format_i128_padded",
            )
        };
        self.emit_format_integer_runtime(widened, formatter, Some(width))
    }

    pub(super) fn emit_format_integer_runtime(
        &self,
        widened: IntValue<'ctx>,
        formatter: &str,
        width: Option<u32>,
    ) -> StructValue<'ctx> {
        let i32_type = self.context.i32_type();
        let (formatter, first_type, first_value): (
            String,
            inkwell::types::BasicTypeEnum<'ctx>,
            inkwell::values::BasicMetadataValueEnum<'ctx>,
        ) = if cfg!(windows) && self.target == CompileTarget::Native {
            let slot = self
                .entry_alloca(self.context.i128_type(), "format.integer.argument")
                .unwrap();
            self.builder.build_store(slot, widened).unwrap();
            (
                format!("{formatter}_indirect"),
                self.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into(),
                slot.into(),
            )
        } else {
            (
                formatter.to_string(),
                widened.get_type().into(),
                widened.into(),
            )
        };
        let mut param_types = vec![first_type];
        let mut arguments = vec![first_value];
        if let Some(width) = width {
            param_types.push(i32_type.into());
            arguments.push(i32_type.const_int(u64::from(width), false).into());
        }
        let format_fn =
            self.declare_libc_fn(&formatter, &param_types, Some(self.string_abi_type()));
        self.builder
            .build_call(format_fn, &arguments, "format.integer")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_struct_value()
    }

    pub(super) fn emit_format_char(&self, code_point: IntValue<'ctx>) -> StructValue<'ctx> {
        let i32_type = self.context.i32_type();
        let format_fn = self.declare_libc_fn(
            "tyrus_format_char",
            &[i32_type.into()],
            Some(self.string_abi_type()),
        );
        self.builder
            .build_call(format_fn, &[code_point.into()], "char_fmt")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_struct_value()
    }

    fn emit_char_method(
        &mut self,
        object: &Expr,
        method: &str,
    ) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        let runtime = match method {
            "toUpperCase" => "tyrus_char_to_upper",
            "toLowerCase" => "tyrus_char_to_lower",
            "isDigit" => "tyrus_char_is_digit",
            "isAlphabetic" => "tyrus_char_is_alphabetic",
            "isAlphanumeric" => "tyrus_char_is_alphanumeric",
            "isWhitespace" => "tyrus_char_is_whitespace",
            "toString" => {
                let value = match self.emit_expr(object) {
                    Ok(value) => value,
                    Err(error) => return Some(Err(error)),
                };
                let text = self.emit_format_char(value.into_int_value());
                self.record_owned_native_result(text.into(), Type::Str);
                return Some(Ok(text.into()));
            }
            _ => return None,
        };
        let value = match self.emit_expr(object) {
            Ok(value) => value,
            Err(error) => return Some(Err(error)),
        };
        let i32_type = self.context.i32_type();
        let returns_bool = method.starts_with("is");
        let return_type: BasicTypeEnum<'ctx> = if returns_bool {
            self.context.bool_type().into()
        } else {
            i32_type.into()
        };
        let function = self.declare_libc_fn(runtime, &[i32_type.into()], Some(return_type));
        let result = self
            .builder
            .build_call(function, &[value.into()], "char.method")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap();
        Some(Ok(result))
    }

    fn emit_literal_to(&self, text: &str, name: &str, output: ConsoleOutput) {
        self.emit_printf_to(text, &[], name, output);
    }

    fn emit_print_string_to(&self, value: StructValue<'ctx>, output: ConsoleOutput) {
        let i32_type = self.context.i32_type();
        let print = self.declare_libc_fn(
            "tyrus_print_str",
            &[self.string_abi_type(), i32_type.into()],
            None,
        );
        let fd = match output {
            ConsoleOutput::Stdout => 1,
            ConsoleOutput::Stderr => 2,
        };
        self.builder
            .build_call(
                print,
                &[value.into(), i32_type.const_int(fd, false).into()],
                "",
            )
            .unwrap();
    }

    fn emit_print_string_quoted_to(&self, value: StructValue<'ctx>, output: ConsoleOutput) {
        self.emit_literal_to("\"", "print.str.open", output);
        self.emit_print_string_to(value, output);
        self.emit_literal_to("\"", "print.str.close", output);
    }

    fn emit_printf_to(
        &self,
        fmt: &str,
        args: &[BasicMetadataValueEnum<'ctx>],
        name: &str,
        output: ConsoleOutput,
    ) {
        let fmt_ptr = self.builder.build_global_string_ptr(fmt, name).unwrap();
        let mut call_args = Vec::with_capacity(args.len() + 2);
        match output {
            ConsoleOutput::Stdout if cfg!(windows) && self.target == CompileTarget::Native => {
                call_args.push(self.context.i32_type().const_int(1, false).into());
                call_args.push(fmt_ptr.as_pointer_value().into());
                call_args.extend_from_slice(args);
                self.builder
                    .build_call(self.declare_dprintf(), &call_args, "")
                    .unwrap();
            }
            ConsoleOutput::Stdout => {
                call_args.push(fmt_ptr.as_pointer_value().into());
                call_args.extend_from_slice(args);
                self.builder
                    .build_call(self.declare_printf(), &call_args, "")
                    .unwrap();
            }
            ConsoleOutput::Stderr => {
                call_args.push(self.context.i32_type().const_int(2, false).into());
                call_args.push(fmt_ptr.as_pointer_value().into());
                call_args.extend_from_slice(args);
                self.builder
                    .build_call(self.declare_dprintf(), &call_args, "")
                    .unwrap();
            }
        }
    }

    fn emit_print_struct_to(
        &self,
        struct_val: StructValue<'ctx>,
        type_name: &str,
        newline: bool,
        output: ConsoleOutput,
    ) {
        let Some(fields) = self.declared_fields_with_index(&Type::Named(type_name.to_string()))
        else {
            return;
        };
        self.emit_print_fields_to(struct_val, &fields, newline, output);
    }

    fn emit_print_object_fields_to(
        &self,
        struct_val: StructValue<'ctx>,
        fields: &[Field],
        newline: bool,
        output: ConsoleOutput,
    ) {
        let fields: Vec<(u32, Field)> = fields
            .iter()
            .enumerate()
            .map(|(index, field)| (index as u32, field.clone()))
            .collect();
        self.emit_print_fields_to(struct_val, &fields, newline, output);
    }

    fn emit_print_fields_to(
        &self,
        struct_val: StructValue<'ctx>,
        fields: &[(u32, Field)],
        newline: bool,
        output: ConsoleOutput,
    ) {
        let Some(function) = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
        else {
            return;
        };
        let bool_ty = self.context.bool_type();
        let printed = self.entry_alloca(bool_ty, "obj.printed").unwrap();
        self.builder
            .build_store(printed, bool_ty.const_zero())
            .unwrap();

        self.emit_literal_to("{", "obj.open", output);
        for (index, field) in fields {
            let storage = field_storage_type(field);
            let field_val = self
                .builder
                .build_extract_value(struct_val, *index, &field.name)
                .unwrap();
            if let Type::Optional(inner) = &storage {
                let ptr = field_val.into_pointer_value();
                let is_null = self.builder.build_is_null(ptr, "obj.opt.null").unwrap();
                let print_bb = self.context.append_basic_block(function, "obj.field.print");
                let cont_bb = self.context.append_basic_block(function, "obj.field.cont");
                self.builder
                    .build_conditional_branch(is_null, cont_bb, print_bb)
                    .unwrap();
                self.builder.position_at_end(print_bb);
                self.emit_obj_separator_to(function, printed, output);
                self.emit_literal_to(&format!("{}: ", field.name), "obj.label", output);
                let inner_ty = self.llvm_type(inner);
                let loaded = self
                    .builder
                    .build_load(inner_ty, ptr, "obj.opt.load")
                    .unwrap();
                self.emit_print_scalar_inline_to(loaded, inner, output);
                self.builder
                    .build_store(printed, bool_ty.const_int(1, false))
                    .unwrap();
                self.builder.build_unconditional_branch(cont_bb).unwrap();
                self.builder.position_at_end(cont_bb);
            } else {
                self.emit_obj_separator_to(function, printed, output);
                self.emit_literal_to(&format!("{}: ", field.name), "obj.label", output);
                self.emit_print_scalar_inline_to(field_val, &storage, output);
                self.builder
                    .build_store(printed, bool_ty.const_int(1, false))
                    .unwrap();
            }
        }

        let printed_val = self
            .builder
            .build_load(bool_ty, printed, "obj.printed.final")
            .unwrap()
            .into_int_value();
        let with_space = self.context.append_basic_block(function, "obj.close.space");
        let no_space = self.context.append_basic_block(function, "obj.close.empty");
        let done = self.context.append_basic_block(function, "obj.close.done");
        self.builder
            .build_conditional_branch(printed_val, with_space, no_space)
            .unwrap();
        self.builder.position_at_end(with_space);
        self.emit_literal_to(
            if newline { " }\n" } else { " }" },
            "obj.close.space.text",
            output,
        );
        self.builder.build_unconditional_branch(done).unwrap();
        self.builder.position_at_end(no_space);
        self.emit_literal_to(
            if newline { "}\n" } else { "}" },
            "obj.close.empty.text",
            output,
        );
        self.builder.build_unconditional_branch(done).unwrap();
        self.builder.position_at_end(done);
    }

    fn emit_obj_separator_to(
        &self,
        function: FunctionValue<'ctx>,
        printed: PointerValue<'ctx>,
        output: ConsoleOutput,
    ) {
        let bool_ty = self.context.bool_type();
        let printed_val = self
            .builder
            .build_load(bool_ty, printed, "obj.sep.printed")
            .unwrap()
            .into_int_value();
        let comma_bb = self.context.append_basic_block(function, "obj.sep.comma");
        let space_bb = self.context.append_basic_block(function, "obj.sep.space");
        let after_bb = self.context.append_basic_block(function, "obj.sep.after");
        self.builder
            .build_conditional_branch(printed_val, comma_bb, space_bb)
            .unwrap();
        let emit = |text: &str, name: &str| {
            self.emit_literal_to(text, name, output);
        };
        self.builder.position_at_end(comma_bb);
        emit(", ", "obj.sep.comma.text");
        self.builder.build_unconditional_branch(after_bb).unwrap();
        self.builder.position_at_end(space_bb);
        emit(" ", "obj.sep.space.text");
        self.builder.build_unconditional_branch(after_bb).unwrap();
        self.builder.position_at_end(after_bb);
    }

    fn emit_print_scalar_inline_to(
        &self,
        val: BasicValueEnum<'ctx>,
        hint: &Type,
        output: ConsoleOutput,
    ) {
        let printf_str = |fmt: &str, arg: BasicMetadataValueEnum<'ctx>, name: &str| {
            self.emit_printf_to(fmt, &[arg], name, output);
        };
        if val.is_struct_value() && val.into_struct_value().get_type() == self.string_type() {
            self.emit_print_string_quoted_to(val.into_struct_value(), output);
            return;
        }
        match transparent_codegen_type(hint) {
            Type::Bool => {
                let truthy = self
                    .const_string_abi("true", "obj.bool.true")
                    .into_struct_value();
                let falsy = self
                    .const_string_abi("false", "obj.bool.false")
                    .into_struct_value();
                let text = self
                    .builder
                    .build_select(val.into_int_value(), truthy, falsy, "obj.bool.text")
                    .unwrap();
                self.emit_print_string_to(text.into_struct_value(), output);
            }
            Type::Optional(inner) => {
                let ptr = val.into_pointer_value();
                let is_null = self.builder.build_is_null(ptr, "obj.opt.null").unwrap();
                let func = self
                    .builder
                    .get_insert_block()
                    .and_then(inkwell::basic_block::BasicBlock::get_parent);
                let Some(func) = func else { return };
                let null_bb = self.context.append_basic_block(func, "obj.opt.none");
                let some_bb = self.context.append_basic_block(func, "obj.opt.some");
                let cont_bb = self.context.append_basic_block(func, "obj.opt.cont");
                self.builder
                    .build_conditional_branch(is_null, null_bb, some_bb)
                    .unwrap();
                self.builder.position_at_end(null_bb);
                let undefined = self
                    .const_string_abi("undefined", "obj.opt.undef")
                    .into_struct_value();
                self.emit_print_string_to(undefined, output);
                self.builder.build_unconditional_branch(cont_bb).unwrap();
                self.builder.position_at_end(some_bb);
                let inner_ty = self.llvm_type(inner);
                let loaded = self
                    .builder
                    .build_load(inner_ty, ptr, "obj.opt.load")
                    .unwrap();
                self.emit_print_scalar_inline_to(loaded, inner, output);
                self.builder.build_unconditional_branch(cont_bb).unwrap();
                self.builder.position_at_end(cont_bb);
            }
            Type::Named(name) if self.interfaces.contains_key(name.as_str()) => {
                self.emit_print_struct_to(val.into_struct_value(), name, false, output);
            }
            Type::Object(fields) => {
                self.emit_print_object_fields_to(val.into_struct_value(), fields, false, output);
            }
            Type::Tuple(elements) => {
                self.emit_print_tuple_to(val.into_struct_value(), elements, false, output);
            }
            Type::Array(element) => {
                self.emit_print_array_to(val.into_struct_value(), element, false, output);
            }
            Type::Char if val.is_int_value() => {
                let text = self.emit_format_char(val.into_int_value());
                self.emit_print_string_to(text, output);
                self.emit_string_release(text)
                    .expect("releasing a rendered string is always emittable");
            }
            hint if val.is_int_value() => {
                let unsigned = is_unsigned_int_type(hint);
                let int_val = self.promote_int_for_printf(val.into_int_value(), unsigned);
                let bits = int_val.get_type().get_bit_width();
                let fmt = match (bits, unsigned) {
                    (128, true) => {
                        let print = self.declare_print_u128(output);
                        self.builder
                            .build_call(print, &[int_val.into()], "")
                            .unwrap();
                        return;
                    }
                    (128, false) => {
                        let print = self.declare_print_i128(output);
                        self.builder
                            .build_call(print, &[int_val.into()], "")
                            .unwrap();
                        return;
                    }
                    (64, true) => "%llu",
                    (64, false) => "%lld",
                    (_, true) => "%u",
                    (_, false) => "%d",
                };
                printf_str(fmt, int_val.into(), "obj.fmt.int");
            }
            _ if val.is_float_value() => {
                let text = self.emit_format_f64(val.into_float_value());
                self.emit_print_string_to(text, output);
                self.emit_string_release(text)
                    .expect("releasing a rendered string is always emittable");
            }
            _ => {}
        }
    }

    fn promote_int_for_printf(&self, value: IntValue<'ctx>, unsigned: bool) -> IntValue<'ctx> {
        let i32_type = self.context.i32_type();
        if value.get_type().get_bit_width() >= i32_type.get_bit_width() {
            return value;
        }
        if unsigned || value.get_type().get_bit_width() == 1 {
            self.builder
                .build_int_z_extend(value, i32_type, "print.zext")
                .unwrap()
        } else {
            self.builder
                .build_int_s_extend(value, i32_type, "print.sext")
                .unwrap()
        }
    }

    fn declare_putchar(&self) -> FunctionValue<'ctx> {
        if let Some(function) = self.module.get_function("putchar") {
            return function;
        }
        let i32_type = self.context.i32_type();
        self.module
            .add_function("putchar", i32_type.fn_type(&[i32_type.into()], false), None)
    }

    fn emit_putchar_to(&self, character: IntValue<'ctx>, output: ConsoleOutput) {
        match output {
            ConsoleOutput::Stdout => {
                self.builder
                    .build_call(self.declare_putchar(), &[character.into()], "")
                    .unwrap();
            }
            ConsoleOutput::Stderr => {
                self.emit_printf_to("%c", &[character.into()], "print.char.stderr", output);
            }
        }
    }

    fn declare_print_i128(&self, output: ConsoleOutput) -> FunctionValue<'ctx> {
        let name = match output {
            ConsoleOutput::Stdout => "__tyrus_print_i128",
            ConsoleOutput::Stderr => "__tyrus_print_i128_stderr",
        };
        if let Some(function) = self.module.get_function(name) {
            return function;
        }
        let i128_type = self.context.i128_type();
        let function = self.module.add_function(
            name,
            self.context.void_type().fn_type(&[i128_type.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(function, "entry");
        let negative = self.context.append_basic_block(function, "negative");
        let print = self.context.append_basic_block(function, "print");
        let outer = self.builder.get_insert_block();
        self.builder.position_at_end(entry);

        let value = function.get_first_param().unwrap().into_int_value();
        let is_negative = self
            .builder
            .build_int_compare(
                IntPredicate::SLT,
                value,
                i128_type.const_zero(),
                "is_negative",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(is_negative, negative, print)
            .unwrap();

        self.builder.position_at_end(negative);
        self.emit_putchar_to(self.context.i32_type().const_int('-' as u64, false), output);
        let abs = self
            .builder
            .build_int_sub(i128_type.const_zero(), value, "abs_i128")
            .unwrap();
        self.builder.build_unconditional_branch(print).unwrap();

        self.builder.position_at_end(print);
        let magnitude = self.builder.build_phi(i128_type, "magnitude").unwrap();
        magnitude.add_incoming(&[(&value, entry), (&abs, negative)]);
        self.emit_print_u128_body(
            function,
            magnitude.as_basic_value().into_int_value(),
            output,
        );

        if let Some(block) = outer {
            self.builder.position_at_end(block);
        }
        function
    }

    fn declare_print_u128(&self, output: ConsoleOutput) -> FunctionValue<'ctx> {
        let name = match output {
            ConsoleOutput::Stdout => "__tyrus_print_u128",
            ConsoleOutput::Stderr => "__tyrus_print_u128_stderr",
        };
        if let Some(function) = self.module.get_function(name) {
            return function;
        }
        let i128_type = self.context.i128_type();
        let function = self.module.add_function(
            name,
            self.context.void_type().fn_type(&[i128_type.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(function, "entry");
        let outer = self.builder.get_insert_block();
        self.builder.position_at_end(entry);
        let value = function.get_first_param().unwrap().into_int_value();
        self.emit_print_u128_body(function, value, output);
        if let Some(block) = outer {
            self.builder.position_at_end(block);
        }
        function
    }

    fn declare_print_f128(&self, output: ConsoleOutput) -> FunctionValue<'ctx> {
        let name = match output {
            ConsoleOutput::Stdout => "__tyrus_print_f128",
            ConsoleOutput::Stderr => "__tyrus_print_f128_stderr",
        };
        if let Some(function) = self.module.get_function(name) {
            return function;
        }
        let f128_type = self.context.f128_type();
        let i64_type = self.context.i64_type();
        let i128_type = self.context.i128_type();
        let function = self.module.add_function(
            name,
            self.context.void_type().fn_type(&[f128_type.into()], false),
            None,
        );
        let outer = self.builder.get_insert_block();
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        let value = function.get_first_param().unwrap().into_float_value();
        let bits = self
            .builder
            .build_bit_cast(value, i128_type, "f128.bits")
            .unwrap()
            .into_int_value();
        let low = self
            .builder
            .build_int_truncate(bits, i64_type, "f128.bits.low")
            .unwrap();
        let shifted = self
            .builder
            .build_right_shift(
                bits,
                i128_type.const_int(64, false),
                false,
                "f128.bits.shift",
            )
            .unwrap();
        let high = self
            .builder
            .build_int_truncate(shifted, i64_type, "f128.bits.high")
            .unwrap();
        let format_fn = self.declare_libc_fn(
            "tyrus_format_f128",
            &[i64_type.into(), i64_type.into()],
            Some(self.string_abi_type()),
        );
        let text = self
            .builder
            .build_call(format_fn, &[low.into(), high.into()], "f128.text")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_struct_value();
        let data = self
            .string_data_ptr(text)
            .expect("a string value always exposes its buffer");
        let length = self
            .string_byte_len(text)
            .expect("a string value always exposes its length");
        let precision = self
            .builder
            .build_int_truncate(length, self.context.i32_type(), "print.f128.precision")
            .unwrap();
        self.emit_printf_to(
            "%.*s",
            &[precision.into(), data.into()],
            "print.f128",
            output,
        );
        self.emit_string_release(text)
            .expect("releasing a rendered string is always emittable");
        self.builder.build_return(None).unwrap();

        if let Some(block) = outer {
            self.builder.position_at_end(block);
        }
        function
    }

    fn emit_print_u128_body(
        &self,
        function: FunctionValue<'ctx>,
        value: IntValue<'ctx>,
        output: ConsoleOutput,
    ) {
        let i8_type = self.context.i8_type();
        let i32_type = self.context.i32_type();
        let i128_type = self.context.i128_type();
        let zero = i128_type.const_zero();
        let ten = i128_type.const_int(10, false);
        let buffer_type = i8_type.array_type(39);
        let buffer = self.entry_alloca(buffer_type, "u128_digits").unwrap();
        let index = self.entry_alloca(i32_type, "u128_index").unwrap();
        let current = self.entry_alloca(i128_type, "u128_current").unwrap();
        self.builder
            .build_store(index, i32_type.const_int(38, false))
            .unwrap();
        self.builder.build_store(current, value).unwrap();

        let zero_block = self.context.append_basic_block(function, "u128.zero");
        let loop_block = self.context.append_basic_block(function, "u128.loop");
        let write_block = self.context.append_basic_block(function, "u128.write");
        let output_block = self.context.append_basic_block(function, "u128.output");
        let done = self.context.append_basic_block(function, "u128.done");

        let is_zero = self
            .builder
            .build_int_compare(IntPredicate::EQ, value, zero, "u128_is_zero")
            .unwrap();
        self.builder
            .build_conditional_branch(is_zero, zero_block, loop_block)
            .unwrap();

        self.builder.position_at_end(zero_block);
        self.emit_putchar_to(i32_type.const_int('0' as u64, false), output);
        self.builder.build_return(None).unwrap();

        self.builder.position_at_end(loop_block);
        let current_value = self
            .builder
            .build_load(i128_type, current, "u128_current_value")
            .unwrap()
            .into_int_value();
        let has_digits = self
            .builder
            .build_int_compare(IntPredicate::NE, current_value, zero, "u128_has_digits")
            .unwrap();
        self.builder
            .build_conditional_branch(has_digits, write_block, output_block)
            .unwrap();

        self.builder.position_at_end(write_block);
        let digit = self
            .builder
            .build_int_unsigned_rem(current_value, ten, "u128_digit")
            .unwrap();
        let digit_i8 = self
            .builder
            .build_int_truncate(digit, i8_type, "u128_digit_i8")
            .unwrap();
        let ascii = self
            .builder
            .build_int_add(digit_i8, i8_type.const_int('0' as u64, false), "u128_ascii")
            .unwrap();
        let index_value = self
            .builder
            .build_load(i32_type, index, "u128_index_value")
            .unwrap()
            .into_int_value();
        let slot = unsafe {
            self.builder
                .build_gep(
                    buffer_type,
                    buffer,
                    &[i32_type.const_zero(), index_value],
                    "u128_digit_slot",
                )
                .unwrap()
        };
        self.builder.build_store(slot, ascii).unwrap();
        let next_current = self
            .builder
            .build_int_unsigned_div(current_value, ten, "u128_next")
            .unwrap();
        self.builder.build_store(current, next_current).unwrap();
        let next_index = self
            .builder
            .build_int_sub(index_value, i32_type.const_int(1, false), "u128_next_index")
            .unwrap();
        self.builder.build_store(index, next_index).unwrap();
        self.builder.build_unconditional_branch(loop_block).unwrap();

        self.builder.position_at_end(output_block);
        let first_index = self
            .builder
            .build_load(i32_type, index, "u128_first_index")
            .unwrap()
            .into_int_value();
        let output_index = self.entry_alloca(i32_type, "u128_output_index").unwrap();
        let start_index = self
            .builder
            .build_int_add(
                first_index,
                i32_type.const_int(1, false),
                "u128_start_index",
            )
            .unwrap();
        self.builder.build_store(output_index, start_index).unwrap();
        let output_loop = self
            .context
            .append_basic_block(function, "u128.output.loop");
        let output_write = self
            .context
            .append_basic_block(function, "u128.output.write");
        self.builder
            .build_unconditional_branch(output_loop)
            .unwrap();

        self.builder.position_at_end(output_loop);
        let cursor = self
            .builder
            .build_load(i32_type, output_index, "u128_output_cursor")
            .unwrap()
            .into_int_value();
        let in_range = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                cursor,
                i32_type.const_int(39, false),
                "u128_output_has_more",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(in_range, output_write, done)
            .unwrap();

        self.builder.position_at_end(output_write);
        let slot = unsafe {
            self.builder
                .build_gep(
                    buffer_type,
                    buffer,
                    &[i32_type.const_zero(), cursor],
                    "u128_output_slot",
                )
                .unwrap()
        };
        let character = self
            .builder
            .build_load(i8_type, slot, "u128_output_char")
            .unwrap()
            .into_int_value();
        let character = self
            .builder
            .build_int_z_extend(character, i32_type, "u128_output_char_i32")
            .unwrap();
        self.emit_putchar_to(character, output);
        let next = self
            .builder
            .build_int_add(cursor, i32_type.const_int(1, false), "u128_output_next")
            .unwrap();
        self.builder.build_store(output_index, next).unwrap();
        self.builder
            .build_unconditional_branch(output_loop)
            .unwrap();

        self.builder.position_at_end(done);
        self.builder.build_return(None).unwrap();
    }
}

/// Whether a `string` method returns a newly allocated string the caller owns.
///
/// Anything not listed here either returns a scalar (`length`, `indexOf`) or a
/// pointer the callee still owns, and freeing those would be a double free.
fn string_method_allocates(method: &str) -> bool {
    matches!(
        method,
        "charAt"
            | "concat"
            | "toUpperCase"
            | "toLowerCase"
            | "trim"
            | "trimStart"
            | "trimEnd"
            | "slice"
            | "substring"
            | "replace"
            | "repeat"
            | "padStart"
            | "padEnd"
            | "toSlugCase"
            | "toDashCase"
            | "toKebabCase"
            | "toPascalCase"
            | "toCamelCase"
            | "toSnakeCase"
            | "truncate"
            | "truncateEllipsis"
            | "stripHtml"
            | "escapeHtml"
            | "urlEncode"
            | "urlDecode"
    )
}

pub(super) fn algorithm_method_allocates(method: &str) -> bool {
    matches!(method, "getSuffixArray" | "queryPath")
}
