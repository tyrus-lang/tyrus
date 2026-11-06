use super::*;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_network_handle_method(
        &mut self,
        object: &Expr,
        method: &str,
        type_args: &[Type],
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        if matches!(self.expr_type_hint(object), Some(Type::Named(ref name)) if name == "HttpClientResponse")
        {
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let response = self.emit_expr(object)?.into_pointer_value();
            match method {
                "status" if args.is_empty() => {
                    let status = self.declare_libc_fn(
                        "tyrus_http_client_response_status",
                        &[pointer.into()],
                        Some(self.context.i32_type().into()),
                    );
                    return Ok(Some(
                        self.builder
                            .build_call(status, &[response.into()], "http.response.status")
                            .map_err(|error| error.to_string())?
                            .try_as_basic_value()
                            .basic()
                            .ok_or("internal: response status returned nothing")?,
                    ));
                }
                "ok" if args.is_empty() => {
                    let ok = self.declare_libc_fn(
                        "tyrus_http_client_response_ok",
                        &[pointer.into()],
                        Some(self.context.bool_type().into()),
                    );
                    return Ok(Some(
                        self.builder
                            .build_call(ok, &[response.into()], "http.response.ok")
                            .map_err(|error| error.to_string())?
                            .try_as_basic_value()
                            .basic()
                            .ok_or("internal: response ok returned nothing")?,
                    ));
                }
                "text" if args.is_empty() => {
                    let text = self.declare_libc_fn(
                        "tyrus_http_client_response_text",
                        &[pointer.into()],
                        Some(self.string_abi_type()),
                    );
                    let result = self
                        .builder
                        .build_call(text, &[response.into()], "http.response.text")
                        .map_err(|error| error.to_string())?
                        .try_as_basic_value()
                        .basic()
                        .ok_or("internal: response text returned nothing")?;
                    self.record_owned_native_result(result, Type::Str);
                    return Ok(Some(result));
                }
                "header" if args.len() == 1 => {
                    let name = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                    let header = self.declare_libc_fn(
                        "tyrus_http_client_response_header",
                        &[pointer.into(), self.string_abi_type()],
                        Some(pointer.into()),
                    );
                    let result = self
                        .builder
                        .build_call(
                            header,
                            &[response.into(), name.into()],
                            "http.response.header",
                        )
                        .map_err(|error| error.to_string())?
                        .try_as_basic_value()
                        .basic()
                        .ok_or("internal: response header returned nothing")?;
                    self.record_owned_native_result(result, Type::Optional(Box::new(Type::Str)));
                    return Ok(Some(result));
                }
                "bytes" if args.is_empty() => {
                    let array_type = self
                        .llvm_type(&Type::Array(Box::new(Type::U8)))
                        .into_struct_type();
                    let output = self.entry_alloca(array_type, "http.response.bytes")?;
                    let bytes = self.declare_libc_fn(
                        "tyrus_http_client_response_bytes",
                        &[pointer.into(), pointer.into()],
                        None,
                    );
                    self.builder
                        .build_call(
                            bytes,
                            &[response.into(), output.into()],
                            "http.response.bytes.copy",
                        )
                        .map_err(|error| error.to_string())?;
                    return (self
                        .builder
                        .build_load(array_type, output, "http.response.bytes.value")
                        .map_err(|error| error.to_string()))
                    .map(Some);
                }
                "json" => {
                    let [Type::Named(type_name)] = type_args else {
                        return (Err(
                            "internal: `json` reaches code generation with its interface named"
                                .to_string(),
                        ))
                        .map(Some);
                    };
                    let text = self.declare_libc_fn(
                        "tyrus_http_client_response_text",
                        &[pointer.into()],
                        Some(self.string_abi_type()),
                    );
                    let body = self
                        .builder
                        .build_call(text, &[response.into()], "http.response.json.text")
                        .map_err(|error| error.to_string())?
                        .try_as_basic_value()
                        .basic()
                        .ok_or("internal: response text returned nothing")?
                        .into_struct_value();
                    let decoded = self.emit_automatic_json_decode_value(type_name, body)?;
                    self.emit_string_release(body)?;
                    self.record_owned_native_result(
                        decoded,
                        Type::Result {
                            success: Box::new(Type::Named(type_name.clone())),
                            error: Box::new(Type::Named("JsonError".to_string())),
                        },
                    );
                    return Ok(Some(decoded));
                }
                "close" if args.is_empty() => {
                    let free = self.declare_libc_fn(
                        "tyrus_http_client_response_free",
                        &[pointer.into()],
                        None,
                    );
                    self.builder
                        .build_call(free, &[response.into()], "http.response.close")
                        .map_err(|error| error.to_string())?;
                    if let ExprKind::Ident(name) = object.kind()
                        && let Some((slot, _)) = self.async_frame_slot(name)
                    {
                        self.builder
                            .build_store(slot, pointer.const_null())
                            .map_err(|error| error.to_string())?;
                        self.mark_async_frame_value_initialized(name, false);
                    }
                    return Ok(Some(self.context.i32_type().const_zero().into()));
                }
                _ => {}
            }
        }

        if matches!(self.expr_type_hint(object), Some(Type::Named(ref name)) if name == "HttpClientStreamResponse")
        {
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let response = self.emit_expr(object)?.into_pointer_value();
            match method {
                "status" if args.is_empty() => {
                    let status = self.declare_libc_fn(
                        "tyrus_http_client_stream_response_status",
                        &[pointer.into()],
                        Some(self.context.i32_type().into()),
                    );
                    return Ok(Some(
                        self.builder
                            .build_call(status, &[response.into()], "http.stream_response.status")
                            .map_err(|error| error.to_string())?
                            .try_as_basic_value()
                            .basic()
                            .ok_or("internal: streamed response status returned nothing")?,
                    ));
                }
                "ok" if args.is_empty() => {
                    let ok = self.declare_libc_fn(
                        "tyrus_http_client_stream_response_ok",
                        &[pointer.into()],
                        Some(self.context.bool_type().into()),
                    );
                    return Ok(Some(
                        self.builder
                            .build_call(ok, &[response.into()], "http.stream_response.ok")
                            .map_err(|error| error.to_string())?
                            .try_as_basic_value()
                            .basic()
                            .ok_or("internal: streamed response ok returned nothing")?,
                    ));
                }
                "header" if args.len() == 1 => {
                    let name = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                    let header = self.declare_libc_fn(
                        "tyrus_http_client_stream_response_header",
                        &[pointer.into(), self.string_abi_type()],
                        Some(pointer.into()),
                    );
                    let result = self
                        .builder
                        .build_call(
                            header,
                            &[response.into(), name.into()],
                            "http.stream_response.header",
                        )
                        .map_err(|error| error.to_string())?
                        .try_as_basic_value()
                        .basic()
                        .ok_or("internal: streamed response header returned nothing")?;
                    self.record_owned_native_result(result, Type::Optional(Box::new(Type::Str)));
                    return Ok(Some(result));
                }
                "stream" if args.is_empty() => {
                    let stream = self.declare_libc_fn(
                        "tyrus_http_client_stream_response_stream",
                        &[pointer.into()],
                        Some(pointer.into()),
                    );
                    return (self
                        .builder
                        .build_call(stream, &[response.into()], "http.stream_response.stream")
                        .map_err(|error| error.to_string())?
                        .try_as_basic_value()
                        .basic()
                        .ok_or("internal: streamed response body returned nothing".to_string()))
                    .map(Some);
                }
                "close" if args.is_empty() => {
                    let free = self.declare_libc_fn(
                        "tyrus_http_client_stream_response_free",
                        &[pointer.into()],
                        None,
                    );
                    self.builder
                        .build_call(free, &[response.into()], "http.stream_response.close")
                        .map_err(|error| error.to_string())?;
                    if let ExprKind::Ident(name) = object.kind()
                        && let Some((slot, _)) = self.async_frame_slot(name)
                    {
                        self.builder
                            .build_store(slot, pointer.const_null())
                            .map_err(|error| error.to_string())?;
                        self.mark_async_frame_value_initialized(name, false);
                    }
                    return Ok(Some(self.context.i32_type().const_zero().into()));
                }
                _ => {}
            }
        }

        if matches!(self.expr_type_hint(object), Some(Type::Named(ref name)) if name == "TcpListener")
        {
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let listener = self.emit_expr(object)?.into_pointer_value();
            match method {
                "accept" if args.is_empty() || args.len() == 1 => {
                    let timeout = if args.len() == 1 {
                        self.emit_expr_as(&args[0], &Type::Optional(Box::new(Type::U64)))?
                    } else {
                        pointer.const_null().into()
                    };
                    let accept = self.declare_libc_fn(
                        "tyrus_net_tcp_listener_accept",
                        &[pointer.into(), pointer.into()],
                        Some(pointer.into()),
                    );
                    let result = self
                        .builder
                        .build_call(
                            accept,
                            &[listener.into(), timeout.into()],
                            "net.tcpListener.accept",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    if args.len() == 1 {
                        self.release_borrowed_optional_argument(&args[0], timeout);
                    }
                    return Ok(Some(result));
                }
                "localAddress" if args.is_empty() => {
                    let address = self.declare_libc_fn(
                        "tyrus_net_tcp_listener_local_address",
                        &[pointer.into()],
                        Some(self.string_abi_type()),
                    );
                    let result = self
                        .builder
                        .build_call(address, &[listener.into()], "net.tcpListener.localAddress")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    self.record_owned_native_result(result, Type::Str);
                    return Ok(Some(result));
                }
                "close" if args.is_empty() => {
                    let close = self.declare_libc_fn(
                        "tyrus_net_tcp_listener_close",
                        &[pointer.into()],
                        None,
                    );
                    self.builder
                        .build_call(close, &[listener.into()], "net.tcpListener.close")
                        .unwrap();
                    return Ok(Some(self.context.bool_type().const_zero().into()));
                }
                _ => {}
            }
        }

        if matches!(self.expr_type_hint(object), Some(Type::Named(ref name)) if name == "UdpSocket")
        {
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let socket = self.emit_expr(object)?.into_pointer_value();
            match method {
                "sendTo" if args.len() == 2 || args.len() == 3 => {
                    let array_ty = self.llvm_type(&Type::Array(Box::new(Type::U8)));
                    let slot = self
                        .builder
                        .build_alloca(array_ty, "net.udp.send.data")
                        .unwrap();
                    let data = self.emit_expr_as(&args[0], &Type::Array(Box::new(Type::U8)))?;
                    self.builder.build_store(slot, data).unwrap();
                    let address = self.emit_expr_as(&args[1], &Type::Str)?.into_struct_value();
                    let timeout = if args.len() == 3 {
                        self.emit_expr_as(&args[2], &Type::Optional(Box::new(Type::U64)))?
                    } else {
                        pointer.const_null().into()
                    };
                    let send = self.declare_libc_fn(
                        "tyrus_net_udp_send_to",
                        &[
                            pointer.into(),
                            pointer.into(),
                            self.string_abi_type(),
                            pointer.into(),
                        ],
                        Some(pointer.into()),
                    );
                    let result = self
                        .builder
                        .build_call(
                            send,
                            &[socket.into(), slot.into(), address.into(), timeout.into()],
                            "net.udp.sendTo",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    if args.len() == 3 {
                        self.release_borrowed_optional_argument(&args[2], timeout);
                    }
                    return Ok(Some(result));
                }
                "receiveFrom" if args.len() == 1 || args.len() == 2 => {
                    let max_bytes = self.emit_expr_as(&args[0], &Type::U32)?;
                    let timeout = if args.len() == 2 {
                        self.emit_expr_as(&args[1], &Type::Optional(Box::new(Type::U64)))?
                    } else {
                        pointer.const_null().into()
                    };
                    let receive = self.declare_libc_fn(
                        "tyrus_net_udp_receive_from",
                        &[
                            pointer.into(),
                            self.context.i32_type().into(),
                            pointer.into(),
                        ],
                        Some(pointer.into()),
                    );
                    let result = self
                        .builder
                        .build_call(
                            receive,
                            &[socket.into(), max_bytes.into(), timeout.into()],
                            "net.udp.receiveFrom",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    if args.len() == 2 {
                        self.release_borrowed_optional_argument(&args[1], timeout);
                    }
                    return Ok(Some(result));
                }
                "setBroadcast" if args.len() == 1 => {
                    let enabled = self.emit_expr_as(&args[0], &Type::Bool)?;
                    let set_broadcast = self.declare_libc_fn(
                        "tyrus_net_udp_set_broadcast",
                        &[pointer.into(), self.context.bool_type().into()],
                        Some(pointer.into()),
                    );
                    return Ok(Some(
                        self.builder
                            .build_call(
                                set_broadcast,
                                &[socket.into(), enabled.into()],
                                "net.udp.setBroadcast",
                            )
                            .unwrap()
                            .try_as_basic_value()
                            .basic()
                            .unwrap(),
                    ));
                }
                "setTtl" if args.len() == 1 => {
                    let ttl = self.emit_expr_as(&args[0], &Type::U32)?;
                    let set_ttl = self.declare_libc_fn(
                        "tyrus_net_udp_set_ttl",
                        &[pointer.into(), self.context.i32_type().into()],
                        Some(pointer.into()),
                    );
                    return Ok(Some(
                        self.builder
                            .build_call(set_ttl, &[socket.into(), ttl.into()], "net.udp.setTtl")
                            .unwrap()
                            .try_as_basic_value()
                            .basic()
                            .unwrap(),
                    ));
                }
                "localAddress" if args.is_empty() => {
                    let address = self.declare_libc_fn(
                        "tyrus_net_udp_local_address",
                        &[pointer.into()],
                        Some(self.string_abi_type()),
                    );
                    let result = self
                        .builder
                        .build_call(address, &[socket.into()], "net.udp.localAddress")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    self.record_owned_native_result(result, Type::Str);
                    return Ok(Some(result));
                }
                "close" if args.is_empty() => {
                    let close =
                        self.declare_libc_fn("tyrus_net_udp_close", &[pointer.into()], None);
                    self.builder
                        .build_call(close, &[socket.into()], "net.udp.close")
                        .unwrap();
                    return Ok(Some(self.context.bool_type().const_zero().into()));
                }
                _ => {}
            }
        }

        Ok(None)
    }
}
