use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(crate) fn emit_http_clear_error(&self) -> Result<(), String> {
        let function = self.declare_libc_fn("tyrus_http_clear_error", &[], None);
        self.builder
            .build_call(function, &[], "")
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn emit_http_listen_result(
        &mut self,
        handle: inkwell::values::PointerValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let result_type = Type::Result {
            success: Box::new(Type::Named("HttpServer".to_string())),
            error: Box::new(Type::Named("HttpError".to_string())),
        };
        let variants = vec![
            Type::Named("HttpServer".to_string()),
            Type::Named("HttpError".to_string()),
        ];
        let slot = self.entry_alloca(self.union_llvm_type(&variants), "http.listen.result")?;
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: `listen` result emitted outside a function")?;
        let failed = self
            .context
            .append_basic_block(function, "http.listen.error");
        let succeeded = self
            .context
            .append_basic_block(function, "http.listen.success");
        let done = self
            .context
            .append_basic_block(function, "http.listen.done");
        let is_null = self
            .builder
            .build_is_null(handle, "http.listen.failed")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(is_null, failed, succeeded)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(succeeded);
        self.store_union_variant(&variants, slot, 0, handle.into())?;
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(failed);
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let string = self.string_abi_type();
        let taken_type = self.context.struct_type(&[string, string], false);
        let taken_slot = self
            .builder
            .build_alloca(taken_type, "http.listen.error.taken")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(taken_slot, taken_type.const_zero())
            .map_err(|error| error.to_string())?;
        let take_error = self.declare_libc_fn("tyrus_http_take_error", &[pointer.into()], None);
        self.builder
            .build_call(take_error, &[taken_slot.into()], "")
            .map_err(|error| error.to_string())?;
        let fields = self
            .declared_fields_with_index(&Type::Named("HttpError".to_string()))
            .ok_or("internal: HttpError interface is unavailable")?;
        let mut error = self
            .llvm_type(&Type::Named("HttpError".to_string()))
            .into_struct_type()
            .const_zero();
        for (index, field) in fields {
            let source_index = match field.name.as_str() {
                "code" => 0,
                "message" => 1,
                _ => continue,
            };
            let value = self
                .builder
                .build_struct_gep(
                    taken_type,
                    taken_slot,
                    source_index,
                    "http.listen.error.field",
                )
                .map_err(|error| error.to_string())?;
            let value = self
                .builder
                .build_load(string, value, "http.listen.error.value")
                .map_err(|error| error.to_string())?;
            error = self
                .builder
                .build_insert_value(error, value, index, "http.listen.error.object")
                .map_err(|error| error.to_string())?
                .into_struct_value();
        }
        self.store_union_variant(&variants, slot, 1, error.into())?;
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(done);
        let result = self
            .builder
            .build_load(
                self.union_llvm_type(&variants),
                slot,
                "http.listen.result.value",
            )
            .map_err(|error| error.to_string())?;
        self.record_owned_native_result(result, result_type);
        Ok(result)
    }

    pub(crate) fn emit_http_status_result(
        &mut self,
        status: inkwell::values::IntValue<'ctx>,
        label: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let result_type = Type::Result {
            success: Box::new(Type::Void),
            error: Box::new(Type::Named("HttpError".to_string())),
        };
        let variants = vec![Type::Void, Type::Named("HttpError".to_string())];
        let union_type = self.union_llvm_type(&variants);
        let slot = self.entry_alloca(union_type, &format!("{label}.result"))?;
        self.builder
            .build_store(slot, union_type.const_zero())
            .map_err(|error| error.to_string())?;
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: HTTP result emitted outside a function")?;
        let failed = self
            .context
            .append_basic_block(function, &format!("{label}.failed"));
        let succeeded = self
            .context
            .append_basic_block(function, &format!("{label}.succeeded"));
        let done = self
            .context
            .append_basic_block(function, &format!("{label}.done"));
        let is_failure = self
            .builder
            .build_int_compare(
                IntPredicate::SLT,
                status,
                self.context.i32_type().const_zero(),
                &format!("{label}.is_failure"),
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(is_failure, failed, succeeded)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(succeeded);
        self.store_union_variant_tag(union_type, slot, 0)?;
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(failed);
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let string = self.string_abi_type();
        let taken_type = self.context.struct_type(&[string, string], false);
        let taken_slot = self.entry_alloca(taken_type, &format!("{label}.error"))?;
        self.builder
            .build_store(taken_slot, taken_type.const_zero())
            .map_err(|error| error.to_string())?;
        let take_error = self.declare_libc_fn("tyrus_http_take_error", &[pointer.into()], None);
        self.builder
            .build_call(take_error, &[taken_slot.into()], "")
            .map_err(|error| error.to_string())?;
        let fields = self
            .declared_fields_with_index(&Type::Named("HttpError".to_string()))
            .ok_or("internal: HttpError interface is unavailable")?;
        let mut error = self
            .llvm_type(&Type::Named("HttpError".to_string()))
            .into_struct_type()
            .const_zero();
        for (index, field) in fields {
            let source_index = match field.name.as_str() {
                "code" => 0,
                "message" => 1,
                _ => continue,
            };
            let value_ptr = self
                .builder
                .build_struct_gep(
                    taken_type,
                    taken_slot,
                    source_index,
                    &format!("{label}.error.field"),
                )
                .map_err(|error| error.to_string())?;
            let value = self
                .builder
                .build_load(string, value_ptr, &format!("{label}.error.value"))
                .map_err(|error| error.to_string())?;
            error = self
                .builder
                .build_insert_value(error, value, index, &format!("{label}.error.object"))
                .map_err(|error| error.to_string())?
                .into_struct_value();
        }
        self.store_union_variant(&variants, slot, 1, error.into())?;
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(done);
        let result = self
            .builder
            .build_load(union_type, slot, &format!("{label}.result.value"))
            .map_err(|error| error.to_string())?;
        self.record_owned_native_result(result, result_type);
        Ok(result)
    }

    pub(crate) fn emit_http_router_new(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if !args.is_empty() {
            return Err("error: `createRouter` does not accept arguments".to_string());
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let function = self.declare_libc_fn("tyrus_http_router_new", &[], Some(pointer.into()));
        let router = self
            .builder
            .build_call(function, &[], "http_router")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap();
        for route_export in self.auto_route_exports.clone() {
            let route_fn = self.fns.get(&route_export).copied().ok_or_else(|| {
                format!(
                    "internal: auto-discovered route function `{route_export}` was not declared"
                )
            })?;
            self.builder
                .build_call(route_fn, &[router.into()], "http_auto_route")
                .map_err(|error| error.to_string())?;
        }
        if self.async_results.is_empty() {
            self.record_owned_native_result(router, Type::Named("HttpRouter".to_string()));
        }
        Ok(router)
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn emit_http_router_method(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_type = self.context.i32_type();
        let router = self.emit_expr(object)?.into_pointer_value();
        if method == "ws" {
            return self.emit_http_websocket_route(router, args);
        }
        if method == "docs" {
            return self.emit_http_docs(router, args);
        }
        if method == "onResponse" {
            let ExprKind::Ident(handler_name) = args[0].kind() else {
                return Err("error: HTTP response hooks must be named functions".to_string());
            };
            let callback = self.emit_http_response_hook_wrapper(handler_name)?;
            let function = self.declare_libc_fn(
                "tyrus_http_router_on_response",
                &[pointer.into(), pointer.into()],
                Some(i32_type.into()),
            );
            self.emit_http_clear_error()?;
            let status = self
                .builder
                .build_call(
                    function,
                    &[
                        router.into(),
                        callback.as_global_value().as_pointer_value().into(),
                    ],
                    "http_on_response",
                )
                .map_err(|error| error.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or("internal: onResponse did not return a status")?
                .into_int_value();
            return self.emit_http_status_result(status, "http.on_response");
        }
        if method == "cors" {
            let options_type = Type::Named("CorsOptions".to_string());
            let options = self.emit_expr_as(&args[0], &options_type)?;
            let options_slot = self
                .builder
                .build_alloca(self.llvm_type(&options_type), "cors_options")
                .unwrap();
            self.builder.build_store(options_slot, options).unwrap();
            let function = self.declare_libc_fn(
                "tyrus_http_router_cors",
                &[pointer.into(), pointer.into()],
                Some(i32_type.into()),
            );
            self.emit_http_clear_error()?;
            let status = self
                .builder
                .build_call(function, &[router.into(), options_slot.into()], "http_cors")
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap()
                .into_int_value();
            return self.emit_http_status_result(status, "http.cors");
        }
        if method == "csrf" {
            let options_type = Type::Named("CsrfOptions".to_string());
            let options = self.emit_expr_as(&args[0], &options_type)?;
            let options_slot = self
                .builder
                .build_alloca(self.llvm_type(&options_type), "csrf_options")
                .map_err(|error| error.to_string())?;
            self.builder
                .build_store(options_slot, options)
                .map_err(|error| error.to_string())?;
            let function = self.declare_libc_fn(
                "tyrus_http_router_csrf",
                &[pointer.into(), pointer.into()],
                Some(i32_type.into()),
            );
            self.emit_http_clear_error()?;
            let status = self
                .builder
                .build_call(function, &[router.into(), options_slot.into()], "http_csrf")
                .map_err(|error| error.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or("internal: csrf did not return a status")?
                .into_int_value();
            return self.emit_http_status_result(status, "http.csrf");
        }
        if method == "fetchPolicy" {
            let options_type = Type::Named("FetchPolicyOptions".to_string());
            let options = self.emit_expr_as(&args[0], &options_type)?;
            let options_slot = self
                .builder
                .build_alloca(self.llvm_type(&options_type), "fetch_policy_options")
                .map_err(|error| error.to_string())?;
            self.builder
                .build_store(options_slot, options)
                .map_err(|error| error.to_string())?;
            let function = self.declare_libc_fn(
                "tyrus_http_router_fetch_policy",
                &[pointer.into(), pointer.into()],
                Some(i32_type.into()),
            );
            self.emit_http_clear_error()?;
            let status = self
                .builder
                .build_call(
                    function,
                    &[router.into(), options_slot.into()],
                    "http_fetch_policy",
                )
                .map_err(|error| error.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or("internal: fetchPolicy did not return a status")?
                .into_int_value();
            return self.emit_http_status_result(status, "http.fetch_policy");
        }
        if method == "rateLimit" {
            let ExprKind::ObjectLit(fields) = args[0].kind() else {
                return Err(
                    "error: `rateLimit` must be an object literal like `{ max: 100, timeWindow: \"1 minute\" }`"
                        .to_string(),
                );
            };
            let Some((_, ExprKind::IntLit(max))) = fields
                .iter()
                .find(|(name, _)| name == "max")
                .map(|(name, value)| (name, value.kind()))
            else {
                return Err("error: `rateLimit.max` must be an integer literal".to_string());
            };
            let Some((_, ExprKind::StrLit(window))) = fields
                .iter()
                .find(|(name, _)| name == "timeWindow")
                .map(|(name, value)| (name, value.kind()))
            else {
                return Err(
                    "error: `rateLimit.timeWindow` must be a string literal like `\"1 minute\"`"
                        .to_string(),
                );
            };
            let global_max = match fields
                .iter()
                .find(|(name, _)| name == "globalMax")
                .map(|(_, value)| value.kind())
            {
                Some(ExprKind::IntLit(global_max)) => *global_max,
                Some(_) => {
                    return Err(
                        "error: `rateLimit.globalMax` must be an integer literal".to_string()
                    );
                }
                None => -1,
            };
            let window_millis = tyrus_core::http_config::parse_time_window(window)
                .map_err(|reason| format!("error: {reason}"))?
                .as_millis();
            let ban = match fields
                .iter()
                .find(|(name, _)| name == "ban")
                .map(|(_, value)| value.kind())
            {
                Some(ExprKind::IntLit(ban)) => *ban,
                Some(_) => {
                    return Err("error: `rateLimit.ban` must be an integer literal".to_string());
                }
                None => -1,
            };
            let ban_window_millis = match fields
                .iter()
                .find(|(name, _)| name == "banWindow")
                .map(|(_, value)| value.kind())
            {
                Some(ExprKind::StrLit(window)) => {
                    tyrus_core::http_config::parse_time_window(window)
                        .map_err(|reason| format!("error: {reason}"))?
                        .as_millis()
                }
                Some(_) => {
                    return Err("error: `rateLimit.banWindow` must be a string literal".to_string());
                }
                None => window_millis,
            };
            let i64_type = self.context.i64_type();
            let function = self.declare_libc_fn(
                "tyrus_http_router_rate_limit",
                &[
                    pointer.into(),
                    i64_type.into(),
                    i64_type.into(),
                    i64_type.into(),
                    i64_type.into(),
                    i64_type.into(),
                ],
                Some(i32_type.into()),
            );
            self.emit_http_clear_error()?;
            let status = self
                .builder
                .build_call(
                    function,
                    &[
                        router.into(),
                        i64_type
                            .const_int(i64::try_from(*max).unwrap_or(i64::MAX) as u64, true)
                            .into(),
                        i64_type
                            .const_int(
                                i64::try_from(window_millis).unwrap_or(i64::MAX) as u64,
                                true,
                            )
                            .into(),
                        i64_type
                            .const_int(i64::try_from(global_max).unwrap_or(i64::MAX) as u64, true)
                            .into(),
                        i64_type
                            .const_int(i64::try_from(ban).unwrap_or(i64::MAX) as u64, true)
                            .into(),
                        i64_type
                            .const_int(
                                i64::try_from(ban_window_millis).unwrap_or(i64::MAX) as u64,
                                true,
                            )
                            .into(),
                    ],
                    "http_rate_limit",
                )
                .map_err(|error| error.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or("internal: rateLimit did not return a status")?
                .into_int_value();
            return self.emit_http_status_result(status, "http.rate_limit");
        }
        if method == "trustedProxies" {
            let proxies_type = Type::Array(Box::new(Type::Str));
            let proxies = self.emit_expr_as(&args[0], &proxies_type)?;
            let proxies_slot = self
                .builder
                .build_alloca(self.llvm_type(&proxies_type), "http.trusted_proxies")
                .map_err(|error| error.to_string())?;
            self.builder.build_store(proxies_slot, proxies).unwrap();
            let identity_header = args
                .get(1)
                .map(|header| {
                    self.emit_expr_as(header, &Type::Str)
                        .map(|value| value.into_struct_value())
                })
                .transpose()?;
            let identity_header_slot = identity_header
                .map(|header| {
                    let slot = self
                        .builder
                        .build_alloca(
                            self.llvm_type(&Type::Str),
                            "http.trusted_proxy_identity_header",
                        )
                        .map_err(|error| error.to_string())?;
                    self.builder.build_store(slot, header).unwrap();
                    Ok::<_, String>(slot)
                })
                .transpose()?;
            let function = self.declare_libc_fn(
                "tyrus_http_router_trusted_proxies",
                &[pointer.into(), pointer.into(), pointer.into()],
                Some(i32_type.into()),
            );
            self.emit_http_clear_error()?;
            let status = self
                .builder
                .build_call(
                    function,
                    &[
                        router.into(),
                        proxies_slot.into(),
                        identity_header_slot
                            .map_or_else(|| pointer.const_null().into(), std::convert::Into::into),
                    ],
                    "http_trusted_proxies",
                )
                .map_err(|error| error.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or("internal: trustedProxies did not return a status")?
                .into_int_value();
            return self.emit_http_status_result(status, "http.trusted_proxies");
        }
        if method == "listen" {
            let port = self.emit_expr_as(&args[0], &Type::I32)?.into_int_value();
            let options_slot = self.emit_http_server_options(args.get(1))?;
            let function = self.declare_libc_fn(
                "tyrus_http_router_listen",
                &[pointer.into(), i32_type.into(), pointer.into()],
                Some(pointer.into()),
            );
            let handle = self
                .builder
                .build_call(
                    function,
                    &[router.into(), port.into(), options_slot.into()],
                    "http_listen",
                )
                .map_err(|error| error.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or("internal: listen did not return a server handle")?
                .into_pointer_value();
            return self.emit_http_listen_result(handle);
        }

        let (path, handler_index, symbol) = match method {
            "get" | "post" | "put" | "patch" | "delete" | "head" | "options" | "connect"
            | "query" | "trace" => (
                Some(self.emit_expr(&args[0])?.into_struct_value()),
                1,
                format!("tyrus_http_router_{method}"),
            ),
            "fallback" => (None, 0, "tyrus_http_router_fallback".to_string()),
            _ => return Err(format!("error: unsupported HttpRouter method `{method}`")),
        };
        let route = self.http_route_options(method, &args[handler_index])?;
        let callback = match route.handler {
            HttpRouteHandler::Named(handler_name) => {
                self.emit_http_callback_wrapper(handler_name, &route.contract)?
            }
            HttpRouteHandler::Inline {
                is_async,
                params,
                ret_type,
                body,
            } => {
                let (handler_name, handler, param_types, ret_type) = self
                    .emit_http_inline_handler(is_async, params, ret_type, body, &route.contract)?;
                self.emit_http_callback_wrapper_for_function(
                    &handler_name,
                    handler,
                    &param_types,
                    &ret_type,
                    &route.contract,
                )?
            }
        };
        let i64_type = self.context.i64_type();
        let usize_type = self.usize_type();
        let (
            rate_limit_max,
            rate_limit_window_millis,
            rate_limit_global_max,
            rate_limit_ban,
            rate_limit_ban_window_millis,
        ) = match &route.rate_limit {
            Some(rate_limit) => (
                self.emit_expr_as(rate_limit.max, &Type::I64)?
                    .into_int_value(),
                i64_type.const_int(rate_limit.window_millis as u64, false),
                match rate_limit.global_max {
                    Some(global_max) => self.emit_expr_as(global_max, &Type::I64)?.into_int_value(),
                    None => i64_type.const_int((-1_i64) as u64, true),
                },
                match rate_limit.ban {
                    Some(ban) => self.emit_expr_as(ban, &Type::I64)?.into_int_value(),
                    None => i64_type.const_int((-1_i64) as u64, true),
                },
                if rate_limit.ban.is_some() {
                    i64_type.const_int(rate_limit.ban_window_millis as u64, false)
                } else {
                    i64_type.const_int((-1_i64) as u64, true)
                },
            ),
            None => (
                i64_type.const_int((-1_i64) as u64, true),
                i64_type.const_int((-1_i64) as u64, true),
                i64_type.const_int((-1_i64) as u64, true),
                i64_type.const_int((-1_i64) as u64, true),
                i64_type.const_int((-1_i64) as u64, true),
            ),
        };
        let (pre_handlers_ptr, pre_handlers_len) = self.emit_http_pre_handlers(&route)?;
        let params = if path.is_some() {
            vec![
                pointer.into(),
                self.string_abi_type(),
                pointer.into(),
                i64_type.into(),
                i64_type.into(),
                i64_type.into(),
                i64_type.into(),
                i64_type.into(),
                i64_type.into(),
                i64_type.into(),
                pointer.into(),
                usize_type.into(),
            ]
        } else {
            vec![
                pointer.into(),
                pointer.into(),
                pointer.into(),
                usize_type.into(),
            ]
        };
        let function = self.declare_libc_fn(&symbol, &params, Some(i32_type.into()));
        let mut values: Vec<BasicMetadataValueEnum<'ctx>> = vec![router.into()];
        if let Some(path) = path {
            values.push(path.into());
        }
        values.push(callback.as_global_value().as_pointer_value().into());
        if path.is_some() {
            values.push(i64_type.const_int(route.body_limit as u64, true).into());
            values.push(rate_limit_max.into());
            values.push(rate_limit_window_millis.into());
            values.push(rate_limit_global_max.into());
            values.push(rate_limit_ban.into());
            values.push(rate_limit_ban_window_millis.into());
            let body_schema = route
                .contract
                .request_schemas
                .iter()
                .find(|schema| schema.part == "body")
                .map(|schema| schema.schema.as_str());
            let route_flags = match body_schema {
                Some(tyrus_core::http_body::MULTIPART_BODY) => 2i64,
                Some(tyrus_core::http_body::STREAM_BODY) => 4i64,
                Some(schema) if !tyrus_core::http_body::is_shapeless_body(schema) => 1i64,
                _ => 0i64,
            };
            values.push(i64_type.const_int(route_flags as u64, true).into());
            values.push(pre_handlers_ptr.into());
            values.push(usize_type.const_int(pre_handlers_len as u64, false).into());
        } else {
            values.push(pre_handlers_ptr.into());
            values.push(usize_type.const_int(pre_handlers_len as u64, false).into());
        }
        self.emit_http_clear_error()?;
        let status = self
            .builder
            .build_call(function, &values, "http_route")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: HTTP route registration returned nothing")?
            .into_int_value();
        let result = self.emit_http_status_result(status, "http.route")?;
        if let (Some(ExprKind::StrLit(route_path)), Some(_)) =
            (args.first().map(tyrus_core::ast::Expr::kind), path)
        {
            let tag = self
                .builder
                .build_extract_value(result.into_struct_value(), 0, "http.route.result.tag")
                .map_err(|error| error.to_string())?
                .into_int_value();
            let registered = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    tag,
                    self.context.i8_type().const_zero(),
                    "http.route.registered",
                )
                .map_err(|error| error.to_string())?;
            let function = self
                .builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_parent)
                .ok_or("internal: HTTP route description emitted outside a function")?;
            let describe = self
                .context
                .append_basic_block(function, "http.route.describe");
            let done = self.context.append_basic_block(function, "http.route.done");
            self.builder
                .build_conditional_branch(registered, describe, done)
                .map_err(|error| error.to_string())?;
            self.builder.position_at_end(describe);
            self.emit_http_route_description(router, method, route_path, &route.contract)?;
            self.builder
                .build_unconditional_branch(done)
                .map_err(|error| error.to_string())?;
            self.builder.position_at_end(done);
        }
        Ok(result)
    }

    pub(super) fn emit_http_route_description(
        &mut self,
        router: PointerValue<'ctx>,
        method: &str,
        route_path: &str,
        contract: &HttpRouteContract,
    ) -> Result<(), String> {
        let fragment = {
            let tables = tyrus_core::openapi::SchemaTables {
                interfaces: &self.interfaces,
                enums: &self.enums,
            };
            let description = tyrus_core::openapi::RouteDescription {
                method,
                path: route_path,
                tags: contract.tags.clone(),
                request: contract
                    .request_schemas
                    .iter()
                    .map(|schema| (schema.part.as_str(), schema.schema.as_str()))
                    .collect(),
                responses: contract.responses.clone(),
            };
            tyrus_core::openapi::route_fragment(&description, &tables)
        };
        let fragment = self
            .emit_expr_as(&Expr::synthetic(ExprKind::StrLit(fragment)), &Type::Str)?
            .into_struct_value();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let function = self.declare_libc_fn(
            "tyrus_http_router_describe",
            &[pointer.into(), self.string_abi_type()],
            Some(self.context.i32_type().into()),
        );
        self.builder
            .build_call(function, &[router.into(), fragment.into()], "http_describe")
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub(super) fn emit_http_pre_handlers(
        &mut self,
        route: &HttpRouteOptions,
    ) -> Result<(PointerValue<'ctx>, usize), String> {
        let names = route.pre_handlers.clone();
        self.emit_http_pre_handler_table(&names)
    }

    pub(super) fn emit_http_pre_handler_table(
        &mut self,
        names: &[&str],
    ) -> Result<(PointerValue<'ctx>, usize), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        if names.is_empty() {
            return Ok((pointer.const_null(), 0));
        }
        let mut callbacks = Vec::with_capacity(names.len());
        for handler_name in names {
            let callback = self.emit_http_pre_handler_wrapper(handler_name)?;
            callbacks.push(callback.as_global_value().as_pointer_value());
        }
        let table = pointer.const_array(&callbacks);
        let global = self.module.add_global(
            table.get_type(),
            None,
            &format!("__tyrus_http_pre_handlers_{}", self.arrow_fn_count),
        );
        self.arrow_fn_count += 1;
        global.set_initializer(&table);
        global.set_constant(true);
        global.set_linkage(inkwell::module::Linkage::Private);
        global.set_unnamed_addr(true);
        Ok((global.as_pointer_value(), callbacks.len()))
    }

    pub(super) fn http_route_options<'a>(
        &self,
        method: &str,
        options: &'a Expr,
    ) -> Result<HttpRouteOptions<'a>, String> {
        let Expr {
            kind: ExprKind::ObjectLit(fields),
            ..
        } = options
        else {
            return Err(format!(
                "error: `{method}` route options must be an object literal"
            ));
        };
        let Some((_, handler)) = fields.iter().find(|(field, _)| field == "handler") else {
            return Err(format!(
                "error: `{method}` route options must include a `handler` field"
            ));
        };
        let handler = match handler.kind() {
            ExprKind::Ident(handler_name) => HttpRouteHandler::Named(handler_name),
            ExprKind::ArrowFn {
                is_async,
                params,
                ret_type,
                body,
                ..
            } => HttpRouteHandler::Inline {
                is_async: *is_async,
                params,
                ret_type,
                body,
            },
            _ => {
                return Err(format!(
                    "error: `{method}` route handler must be a function identifier or inline arrow function"
                ));
            }
        };
        let pre_handlers = fields
            .iter()
            .find(|(field, _)| field == "preHandler")
            .map(|(_, value)| match value.kind() {
                ExprKind::ArrayLit(handlers) => handlers
                    .iter()
                    .filter_map(|handler| match handler.kind() {
                        ExprKind::Ident(name) => Some(name.as_str()),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            })
            .unwrap_or_default();
        Ok(HttpRouteOptions {
            contract: self.http_route_contract(fields)?,
            handler,
            pre_handlers,
            body_limit: http_route_body_limit(fields),
            rate_limit: http_route_rate_limit(fields),
        })
    }

    /// Adapts a Tyrus `HttpResponse | true` pre-handler to the compact native
    /// ABI. A nonzero return means that a response was written and the runtime
    /// must stop the chain; `true` returns zero and lets the route continue.
    pub(super) fn emit_http_pre_handler_wrapper(
        &mut self,
        handler_name: &str,
    ) -> Result<FunctionValue<'ctx>, String> {
        let wrapper_name = format!("__tyrus_http_pre_handler_{handler_name}");
        if let Some(wrapper) = self.module.get_function(&wrapper_name) {
            return Ok(wrapper);
        }
        let handler = self
            .fns
            .get(handler_name)
            .copied()
            .ok_or_else(|| format!("error: HTTP preHandler `{handler_name}` was not found"))?;
        let (params, ret) = self
            .fn_signatures
            .get(handler_name)
            .cloned()
            .ok_or_else(|| format!("error: HTTP preHandler `{handler_name}` was not found"))?;
        let Type::Union(variants) = self.resolve_type(&ret) else {
            return Err(format!(
                "error: HTTP preHandler `{handler_name}` must return `HttpResponse | true`"
            ));
        };
        let response_type = Type::Named("HttpResponse".to_string());
        let response_tag = variants
            .iter()
            .position(|variant| *variant == response_type)
            .ok_or_else(|| {
                format!("error: HTTP preHandler `{handler_name}` must return `HttpResponse | true`")
            })?;
        if params != vec![Type::Named("HttpRequest".to_string())] {
            return Err(format!(
                "error: HTTP preHandler `{handler_name}` must accept `HttpRequest`"
            ));
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let function_type = self
            .context
            .i32_type()
            .fn_type(&[pointer.into(), pointer.into()], false);
        let wrapper = self.module.add_function(&wrapper_name, function_type, None);
        let previous_block = self.builder.get_insert_block();
        let result = (|| -> Result<(), String> {
            let entry = self.context.append_basic_block(wrapper, "entry");
            let passed = self.context.append_basic_block(wrapper, "passed");
            let blocked = self.context.append_basic_block(wrapper, "blocked");
            self.builder.position_at_end(entry);
            let request_pointer = wrapper.get_nth_param(0).unwrap().into_pointer_value();
            let response_pointer = wrapper.get_nth_param(1).unwrap().into_pointer_value();
            let raw_request_type = self.llvm_type(&Type::Named("HttpRequest".to_string()));
            let request = self
                .builder
                .build_load(raw_request_type, request_pointer, "request")
                .unwrap();
            let result = self
                .builder
                .build_call(handler, &[request.into()], "pre_handler.result")
                .map_err(|error| error.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or_else(|| "internal: HTTP preHandler did not return a value".to_string())?;
            let result_slot = self
                .builder
                .build_alloca(self.union_llvm_type(&variants), "pre_handler.result.slot")
                .map_err(|error| error.to_string())?;
            self.builder
                .build_store(result_slot, result)
                .map_err(|error| error.to_string())?;
            let tag = self.load_union_tag(&variants, result_slot)?;
            let is_response = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    tag,
                    self.context.i8_type().const_int(response_tag as u64, false),
                    "pre_handler.has_response",
                )
                .map_err(|error| error.to_string())?;
            self.builder
                .build_conditional_branch(is_response, blocked, passed)
                .map_err(|error| error.to_string())?;
            self.builder.position_at_end(blocked);
            let response = self.load_union_payload(
                &variants,
                result_slot,
                &response_type,
                "pre_handler.response",
            )?;
            self.builder
                .build_store(response_pointer, response)
                .map_err(|error| error.to_string())?;
            self.builder
                .build_return(Some(&self.context.i32_type().const_int(1, false)))
                .map_err(|error| error.to_string())?;
            self.builder.position_at_end(passed);
            self.builder
                .build_return(Some(&self.context.i32_type().const_zero()))
                .map_err(|error| error.to_string())?;
            Ok(())
        })();
        if let Some(block) = previous_block {
            self.builder.position_at_end(block);
        }
        result?;
        Ok(wrapper)
    }

    /// Adapts a named `HttpAccessLog => void` function to the native response
    /// hook ABI. The access event exists only for this call, so the wrapper
    /// passes it by value and cannot let a borrowed runtime record escape.
    /// `app.ws(path, { handler, origins?, subprotocols?, maxMessageBytes? })`.
    pub(super) fn emit_http_websocket_route(
        &mut self,
        router: PointerValue<'ctx>,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_type = self.context.i32_type();
        let i64_type = self.context.i64_type();
        let [path, options] = args else {
            return Err(format!(
                "error: `ws` expects 2 argument(s), found {}",
                args.len()
            ));
        };
        let ExprKind::ObjectLit(fields) = options.kind() else {
            return Err("error: `ws` route options must be an object literal".to_string());
        };
        let field = |name: &str| {
            fields
                .iter()
                .find(|(field, _)| field == name)
                .map(|(_, value)| value)
        };
        let handler =
            field("handler").ok_or("error: `ws` route options must include a `handler` field")?;
        let handler = match handler.kind() {
            ExprKind::Ident(handler_name) => self.http_websocket_named_handler(handler_name)?,
            ExprKind::ArrowFn {
                is_async,
                params,
                ret_type,
                body,
                ..
            } => self.emit_http_websocket_inline_handler(*is_async, params, ret_type, body)?,
            _ => {
                return Err(
                    "error: a WebSocket route handler must be a function identifier or inline arrow function"
                        .to_string(),
                )
            }
        };
        let callback = self.emit_http_websocket_callback(handler)?;
        let path = self.emit_expr_as(path, &Type::Str)?.into_struct_value();
        let string_array = Type::Array(Box::new(Type::Str));
        type ListArgument<'ctx> = (PointerValue<'ctx>, Option<BasicValueEnum<'ctx>>);
        let list_slot = |this: &mut Self, name: &str| -> Result<ListArgument<'ctx>, String> {
            let Some(value) = field(name) else {
                return Ok((pointer.const_null(), None));
            };
            let value = this.emit_expr_as(value, &string_array)?;
            let slot = this
                .builder
                .build_alloca(this.llvm_type(&string_array), &format!("http.ws.{name}"))
                .map_err(|error| error.to_string())?;
            this.builder
                .build_store(slot, value)
                .map_err(|error| error.to_string())?;
            Ok((slot, Some(value)))
        };
        let (origins, origins_value) = list_slot(self, "origins")?;
        let (protocols, protocols_value) = list_slot(self, "subprotocols")?;
        let pre_handler_names: Vec<&str> =
            match field("preHandler").map(tyrus_core::ast::Expr::kind) {
                Some(ExprKind::ArrayLit(handlers)) => handlers
                    .iter()
                    .filter_map(|handler| match handler.kind() {
                        ExprKind::Ident(name) => Some(name.as_str()),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
        let (pre_handlers, pre_handlers_len) =
            self.emit_http_pre_handler_table(&pre_handler_names)?;
        let usize_type = self.usize_type();
        let (
            rate_limit_max,
            rate_limit_window_millis,
            rate_limit_global_max,
            rate_limit_ban,
            rate_limit_ban_window_millis,
        ) = match http_route_rate_limit(fields) {
            Some(rate_limit) => (
                self.emit_expr_as(rate_limit.max, &Type::I64)?
                    .into_int_value(),
                i64_type.const_int(rate_limit.window_millis as u64, false),
                match rate_limit.global_max {
                    Some(global_max) => self.emit_expr_as(global_max, &Type::I64)?.into_int_value(),
                    None => i64_type.const_int((-1_i64) as u64, true),
                },
                match rate_limit.ban {
                    Some(ban) => self.emit_expr_as(ban, &Type::I64)?.into_int_value(),
                    None => i64_type.const_int((-1_i64) as u64, true),
                },
                if rate_limit.ban.is_some() {
                    i64_type.const_int(rate_limit.ban_window_millis as u64, false)
                } else {
                    i64_type.const_int((-1_i64) as u64, true)
                },
            ),
            None => (
                i64_type.const_int((-1_i64) as u64, true),
                i64_type.const_int((-1_i64) as u64, true),
                i64_type.const_int((-1_i64) as u64, true),
                i64_type.const_int((-1_i64) as u64, true),
                i64_type.const_int((-1_i64) as u64, true),
            ),
        };
        let max_message_bytes = match field("maxMessageBytes") {
            Some(value) => self.emit_expr_as(value, &Type::I64)?.into_int_value(),
            None => i64_type.const_int((-1_i64) as u64, true),
        };
        let function = self.declare_libc_fn(
            "tyrus_http_router_ws",
            &[
                pointer.into(),
                self.string_abi_type(),
                pointer.into(),
                pointer.into(),
                pointer.into(),
                i64_type.into(),
                pointer.into(),
                usize_type.into(),
                i64_type.into(),
                i64_type.into(),
                i64_type.into(),
                i64_type.into(),
                i64_type.into(),
            ],
            Some(i32_type.into()),
        );
        self.emit_http_clear_error()?;
        let status = self
            .builder
            .build_call(
                function,
                &[
                    router.into(),
                    path.into(),
                    callback.as_global_value().as_pointer_value().into(),
                    origins.into(),
                    protocols.into(),
                    max_message_bytes.into(),
                    pre_handlers.into(),
                    usize_type.const_int(pre_handlers_len as u64, false).into(),
                    rate_limit_max.into(),
                    rate_limit_window_millis.into(),
                    rate_limit_global_max.into(),
                    rate_limit_ban.into(),
                    rate_limit_ban_window_millis.into(),
                ],
                "http_ws",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: ws did not return a status")?
            .into_int_value();
        let result = self.emit_http_status_result(status, "http.ws")?;
        for (name, value) in [
            ("origins", origins_value),
            ("subprotocols", protocols_value),
        ] {
            if let (Some(value), Some(expression)) = (value, field(name)) {
                self.emit_async_cleanup_source_expression(value, &string_array, expression)?;
            }
        }
        if let Some(ExprKind::StrLit(route_path)) = args.first().map(tyrus_core::ast::Expr::kind) {
            let fragment = tyrus_core::openapi::websocket_fragment(route_path);
            let fragment = self
                .emit_expr_as(&Expr::synthetic(ExprKind::StrLit(fragment)), &Type::Str)?
                .into_struct_value();
            let describe = self.declare_libc_fn(
                "tyrus_http_router_describe",
                &[pointer.into(), self.string_abi_type()],
                Some(i32_type.into()),
            );
            self.builder
                .build_call(
                    describe,
                    &[router.into(), fragment.into()],
                    "http_ws_describe",
                )
                .map_err(|error| error.to_string())?;
        }
        Ok(result)
    }

    pub(super) fn http_websocket_named_handler(
        &self,
        handler_name: &str,
    ) -> Result<FunctionValue<'ctx>, String> {
        let handler =
            self.fns.get(handler_name).copied().ok_or_else(|| {
                format!("error: WebSocket handler `{handler_name}` was not found")
            })?;
        let (params, ret) = self
            .fn_signatures
            .get(handler_name)
            .cloned()
            .ok_or_else(|| format!("error: WebSocket handler `{handler_name}` was not found"))?;
        let takes_socket = matches!(
            params.as_slice(),
            [parameter] if matches!(transparent_codegen_type(parameter), Type::Named(name) if name == "WebSocket")
        );
        let returns_void_promise = promise_payload_type(&ret)
            .is_some_and(|payload| matches!(transparent_codegen_type(payload), Type::Void));
        if !takes_socket || !returns_void_promise || !self.async_functions.contains(handler_name) {
            return Err(format!(
                "error: WebSocket handler `{handler_name}` must be `async` and accept `(socket: WebSocket)`, returning `Promise<void>`"
            ));
        }
        Ok(handler)
    }

    pub(super) fn emit_http_websocket_inline_handler(
        &mut self,
        is_async: bool,
        params: &[Param],
        ret_type: &Type,
        body: &[Stmt],
    ) -> Result<FunctionValue<'ctx>, String> {
        if !is_async || params.len() != 1 {
            return Err(
                "error: an inline WebSocket handler must be `async (socket: WebSocket): Promise<void> => { ... }`"
                    .to_string(),
            );
        }
        let symbol = format!("__tyrus_http_websocket_handler_{}", self.arrow_fn_count);
        self.arrow_fn_count += 1;
        let response_ty = if matches!(ret_type, Type::Infer) {
            Type::Apply {
                name: "Promise".to_string(),
                args: vec![Type::Void],
            }
        } else {
            self.resolve_type(ret_type)
        };
        let mut resolved_params = params.to_vec();
        resolved_params[0].ty = Type::Named("WebSocket".to_string());
        self.forward_declare_fn(&symbol, &resolved_params, &response_ty);
        self.async_functions.insert(symbol.clone());
        // The same stacks an inline HTTP handler swaps, for the same reason: the
        // route is registered from inside another function, whose blocks and
        // owned values must not leak into this body.
        let previous_block = self.builder.get_insert_block();
        let previous_owned_native =
            std::mem::replace(&mut self.owned_native_vars, vec![HashMap::default()]);
        let previous_native_results = std::mem::take(&mut self.pending_owned_native_results);
        let previous_for_of_iterables = std::mem::take(&mut self.for_of_iterable_cleanups);
        let emitted = self.emit_async_function(&symbol, &resolved_params, &response_ty, body);
        self.owned_native_vars = previous_owned_native;
        self.pending_owned_native_results = previous_native_results;
        self.for_of_iterable_cleanups = previous_for_of_iterables;
        if let Some(block) = previous_block {
            self.builder.position_at_end(block);
        }
        emitted?;
        self.fns
            .get(&symbol)
            .copied()
            .ok_or_else(|| "internal: inline WebSocket handler was not emitted".to_string())
    }

    pub(super) fn emit_http_websocket_callback(
        &mut self,
        handler: FunctionValue<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let handler_name = handler.get_name().to_string_lossy().into_owned();
        let wrapper_name = format!("__tyrus_http_websocket_callback_{handler_name}");
        if let Some(wrapper) = self.module.get_function(&wrapper_name) {
            return Ok(wrapper);
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let wrapper = self.module.add_function(
            &wrapper_name,
            pointer.fn_type(&[pointer.into()], false),
            None,
        );
        let previous_block = self.builder.get_insert_block();
        let result = (|| -> Result<(), String> {
            let entry = self.context.append_basic_block(wrapper, "entry");
            self.builder.position_at_end(entry);
            let socket = wrapper
                .get_nth_param(0)
                .ok_or("internal: WebSocket callback has no socket")?;
            let promise = self
                .builder
                .build_call(handler, &[socket.into()], "websocket_handler")
                .map_err(|error| error.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or("internal: WebSocket handler did not return a Promise")?;
            self.builder
                .build_return(Some(&promise))
                .map_err(|error| error.to_string())?;
            Ok(())
        })();
        if let Some(block) = previous_block {
            self.builder.position_at_end(block);
        }
        result?;
        Ok(wrapper)
    }

    pub(super) fn http_route_contract(
        &self,
        fields: &[(String, Expr)],
    ) -> Result<HttpRouteContract, String> {
        let mut request_schemas = Vec::new();
        let mut responses = Vec::new();
        let mut tags = Vec::new();
        for (field, value) in fields {
            match field.as_str() {
                "tags" => {
                    let ExprKind::ArrayLit(values) = value.kind() else {
                        return Err(
                            "error: HTTP route `tags` must be an array of string literals"
                                .to_string(),
                        );
                    };
                    for value in values {
                        let ExprKind::StrLit(tag) = value.kind() else {
                            return Err(
                                "error: HTTP route `tags` must contain only string literals"
                                    .to_string(),
                            );
                        };
                        tags.push(tag.clone());
                    }
                }
                "body" | "params" | "query" | "headers" | "cookies" => {
                    let Expr {
                        kind: ExprKind::Ident(schema),
                        ..
                    } = value
                    else {
                        return Err(format!(
                            "error: HTTP route `{field}` schema must be a named interface"
                        ));
                    };
                    request_schemas.push(HttpRouteSchema {
                        part: field.clone(),
                        schema: schema.clone(),
                    });
                }
                "response" => match value {
                    Expr {
                        kind: ExprKind::Ident(schema),
                        ..
                    } if schema == "void" => responses.push((200, Type::Void)),
                    Expr {
                        kind: ExprKind::Ident(schema),
                        ..
                    } => responses.push((200, http_response_type_from_name(schema))),
                    Expr {
                        kind: ExprKind::TypeAssert { ty, .. },
                        ..
                    } => responses.push((200, ty.clone())),
                    Expr {
                        kind: ExprKind::ObjectLit(entries),
                        ..
                    } => {
                        for (status, schema) in entries {
                            let status = status.parse::<u16>().map_err(|_| {
                                format!("error: HTTP response key `{status}` must be numeric")
                            })?;
                            let ty = match schema.kind() {
                                ExprKind::Ident(schema) => http_response_type_from_name(schema),
                                ExprKind::TypeAssert { ty, .. } => ty.clone(),
                                _ => {
                                    return Err(
                                        "error: HTTP response schema must be an interface, array type, supported primitive, or `void`"
                                            .to_string(),
                                    );
                                }
                            };
                            responses.push((status, ty));
                        }
                    }
                    _ => {
                        return Err(
                            "error: HTTP response must be a schema or status map".to_string()
                        );
                    }
                },
                "handler" => {}
                _ => {}
            }
        }
        Ok(HttpRouteContract {
            request_schemas,
            responses,
            tags,
        })
    }

    pub(super) fn emit_http_inline_handler(
        &mut self,
        is_async: bool,
        params: &[Param],
        ret_type: &Type,
        body: &[Stmt],
        contract: &HttpRouteContract,
    ) -> Result<(String, FunctionValue<'ctx>, Vec<Type>, Type), String> {
        if params.len() != 2 {
            return Err(
                "internal: an inline HTTP handler reaching code generation takes `(request, reply)`"
                    .to_string(),
            );
        }
        let ParamPattern::Name(param_name) = &params[0].pattern else {
            return Err(
                "error: inline HTTP handler request parameter must be a simple name".to_string(),
            );
        };
        let request_ty = if matches!(params[0].ty, Type::Infer) {
            contract.request_type()
        } else {
            self.resolve_type(&params[0].ty)
        };
        let ParamPattern::Name(reply_name) = &params[1].pattern else {
            return Err(
                "error: inline HTTP handler reply parameter must be a simple name".to_string(),
            );
        };
        let reply_name = reply_name.clone();
        let reply_ty = if matches!(params[1].ty, Type::Infer) {
            contract.reply_type()
        } else {
            self.resolve_type(&params[1].ty)
        };
        let response_ty = if matches!(ret_type, Type::Infer) {
            Type::Void
        } else {
            self.resolve_type(ret_type)
        };
        let param_types = vec![request_ty.clone(), reply_ty.clone()];
        let symbol = format!("__tyrus_http_inline_handler_{}", self.arrow_fn_count);
        self.arrow_fn_count += 1;
        if is_async {
            let response_ty = if matches!(ret_type, Type::Infer) {
                Type::Apply {
                    name: "Promise".to_string(),
                    args: vec![Type::Void],
                }
            } else {
                self.resolve_type(ret_type)
            };
            let mut resolved_params = params.to_vec();
            for (parameter, resolved) in resolved_params.iter_mut().zip(&param_types) {
                parameter.ty = resolved.clone();
            }
            self.forward_declare_fn(&symbol, &resolved_params, &response_ty);
            self.async_functions.insert(symbol.clone());
            let previous_block = self.builder.get_insert_block();
            let previous_owned_native =
                std::mem::replace(&mut self.owned_native_vars, vec![HashMap::default()]);
            let previous_native_results = std::mem::take(&mut self.pending_owned_native_results);
            let previous_for_of_iterables = std::mem::take(&mut self.for_of_iterable_cleanups);
            let emitted = self.emit_async_function(&symbol, &resolved_params, &response_ty, body);
            self.owned_native_vars = previous_owned_native;
            self.pending_owned_native_results = previous_native_results;
            self.for_of_iterable_cleanups = previous_for_of_iterables;
            if let Some(block) = previous_block {
                self.builder.position_at_end(block);
            }
            emitted?;
            let function =
                self.fns.get(&symbol).copied().ok_or_else(|| {
                    "internal: inline async HTTP handler was not emitted".to_string()
                })?;
            return Ok((symbol, function, param_types, response_ty));
        }
        let function =
            self.module
                .add_function(&symbol, self.llvm_fn_type(&param_types, &response_ty), None);

        let previous_block = self.builder.get_insert_block();
        let global_scope = self.vars.first().cloned().unwrap_or_default();
        let previous_vars = std::mem::replace(&mut self.vars, vec![global_scope]);
        let previous_owned_closures =
            std::mem::replace(&mut self.owned_closure_vars, vec![HashSet::default()]);
        let previous_owned_closure_drops =
            std::mem::replace(&mut self.owned_closure_drop_fns, vec![HashMap::default()]);
        let previous_owned_native =
            std::mem::replace(&mut self.owned_native_vars, vec![HashMap::default()]);
        let previous_native_results = std::mem::take(&mut self.pending_owned_native_results);
        let previous_for_of_iterables = std::mem::take(&mut self.for_of_iterable_cleanups);
        let previous_string_array_ownership =
            std::mem::replace(&mut self.string_array_ownership, vec![HashMap::default()]);
        let previous_concrete =
            std::mem::replace(&mut self.var_concrete_types, vec![HashMap::default()]);
        let previous_non_zero =
            std::mem::replace(&mut self.non_zero_vars, vec![HashSet::default()]);
        let previous_async_results = std::mem::take(&mut self.async_results);

        let result = (|| -> Result<(), String> {
            let entry = self.context.append_basic_block(function, "entry");
            self.builder.position_at_end(entry);
            self.push_scope();
            self.return_types.push(response_ty.clone());
            self.push_throw_frame(Vec::new());
            let alloca = self
                .builder
                .build_alloca(self.llvm_type(&request_ty), param_name)
                .unwrap();
            let value = function.get_nth_param(0).unwrap();
            self.builder.build_store(alloca, value).unwrap();
            self.store_var(param_name, alloca, request_ty.clone());
            let alloca = self
                .builder
                .build_alloca(self.llvm_type(&reply_ty), &reply_name)
                .unwrap();
            let value = function.get_nth_param(1).unwrap();
            self.builder.build_store(alloca, value).unwrap();
            self.store_var(&reply_name, alloca, reply_ty.clone());
            for stmt in body {
                self.emit_stmt(stmt, function)?;
            }
            if self
                .builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_terminator)
                .is_none()
            {
                self.emit_owned_closure_cleanups(None)?;
                if matches!(response_ty, Type::Void) {
                    self.builder.build_return(None).unwrap();
                } else {
                    return Err(
                        "error: inline HTTP handler must return a response value".to_string()
                    );
                }
            }
            self.return_types.pop();
            self.pop_throw_frame();
            self.pop_scope();
            Ok(())
        })();

        self.vars = previous_vars;
        self.owned_closure_vars = previous_owned_closures;
        self.owned_closure_drop_fns = previous_owned_closure_drops;
        self.owned_native_vars = previous_owned_native;
        self.pending_owned_native_results = previous_native_results;
        self.for_of_iterable_cleanups = previous_for_of_iterables;
        self.string_array_ownership = previous_string_array_ownership;
        self.var_concrete_types = previous_concrete;
        self.non_zero_vars = previous_non_zero;
        self.async_results = previous_async_results;
        if let Some(block) = previous_block {
            self.builder.position_at_end(block);
        }
        result?;
        Ok((symbol, function, param_types, response_ty))
    }

    pub(super) fn http_reply_handler_is_async(
        &self,
        params: &[Type],
        ret: &Type,
    ) -> Result<bool, String> {
        if params.len() == 2 && matches!(transparent_codegen_type(ret), Type::Void) {
            return Ok(false);
        }
        if params.len() == 2
            && promise_payload_type(ret)
                .is_some_and(|payload| matches!(transparent_codegen_type(payload), Type::Void))
        {
            return Ok(true);
        }
        Err("error: HTTP route handler must accept `(request, reply)` and return `void` or `Promise<void>`".to_string())
    }
}

fn http_response_type_from_name(name: &str) -> Type {
    match name {
        "void" => Type::Void,
        "string" => Type::Str,
        "boolean" => Type::Bool,
        "i32" => Type::I32,
        "i64" => Type::I64,
        "isize" => Type::Isize,
        "u32" => Type::U32,
        "u64" => Type::U64,
        "usize" => Type::Usize,
        "f32" => Type::F32,
        "f64" => Type::F64,
        "decimal" => Type::Decimal,
        _ => Type::Named(name.to_string()),
    }
}
