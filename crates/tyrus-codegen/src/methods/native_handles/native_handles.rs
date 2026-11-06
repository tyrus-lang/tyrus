use super::*;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_native_handle_method(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        if matches!(self.expr_type_hint(object), Some(Type::Named(ref name)) if name == "HttpRouter")
        {
            return (self.emit_http_router_method(object, method, args)).map(Some);
        }

        if matches!(self.expr_type_hint(object), Some(Type::Named(ref name)) if name == "HttpServer")
        {
            return (self.emit_http_server_method(object, method)).map(Some);
        }

        if matches!(self.expr_type_hint(object), Some(Type::Named(ref name)) if name == "WebSocket")
        {
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let socket = self.emit_expr(object)?.into_pointer_value();
            match method {
                "stream" if args.is_empty() => {
                    let function = self.declare_libc_fn(
                        "tyrus_websocket_stream_duplex",
                        &[pointer.into()],
                        Some(pointer.into()),
                    );
                    return (self
                        .builder
                        .build_call(function, &[socket.into()], "websocket.stream")
                        .map_err(|error| error.to_string())?
                        .try_as_basic_value()
                        .basic()
                        .ok_or_else(|| "internal: WebSocket.stream returned nothing".to_string()))
                    .map(Some);
                }
                "receive" if args.is_empty() => {
                    let function = self.declare_libc_fn(
                        "tyrus_websocket_receive",
                        &[pointer.into()],
                        Some(pointer.into()),
                    );
                    return (self
                        .builder
                        .build_call(function, &[socket.into()], "websocket.receive")
                        .map_err(|error| error.to_string())?
                        .try_as_basic_value()
                        .basic()
                        .ok_or_else(|| {
                            "internal: WebSocket.receive did not return a value".to_string()
                        }))
                    .map(Some);
                }
                "close" if args.len() <= 2 => {
                    let i32_type = self.context.i32_type();
                    let code = match args.first() {
                        Some(code) => self.emit_expr_as(code, &Type::I32)?.into_int_value(),
                        None => i32_type.const_int((-1_i64) as u64, true),
                    };
                    let reason = match args.get(1) {
                        Some(reason) => self.emit_expr_as(reason, &Type::Str)?.into_struct_value(),
                        None => self.llvm_type(&Type::Str).into_struct_type().const_zero(),
                    };
                    let function = self.declare_libc_fn(
                        "tyrus_websocket_close",
                        &[pointer.into(), i32_type.into(), self.string_abi_type()],
                        Some(pointer.into()),
                    );
                    let promise = self
                        .builder
                        .build_call(
                            function,
                            &[socket.into(), code.into(), reason.into()],
                            "websocket.close",
                        )
                        .map_err(|error| error.to_string())?
                        .try_as_basic_value()
                        .basic()
                        .ok_or("internal: WebSocket.close did not return a promise")?;
                    if let Some(argument) = args.get(1) {
                        self.emit_native_string_argument_release(reason, argument)?;
                    }
                    return Ok(Some(promise));
                }
                "sendText" if args.len() == 1 => {
                    let text = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                    let function = self.declare_libc_fn(
                        "tyrus_websocket_send_text",
                        &[pointer.into(), self.string_abi_type()],
                        Some(pointer.into()),
                    );
                    let promise = self
                        .builder
                        .build_call(
                            function,
                            &[socket.into(), text.into()],
                            "websocket.sendText",
                        )
                        .map_err(|error| error.to_string())?
                        .try_as_basic_value()
                        .basic()
                        .ok_or("internal: WebSocket.sendText did not return a promise")?;
                    self.emit_native_string_argument_release(text, &args[0])?;
                    return Ok(Some(promise));
                }
                "sendBytes" if args.len() == 1 => {
                    let array_type = Type::Array(Box::new(Type::U8));
                    let slot = self
                        .builder
                        .build_alloca(self.llvm_type(&array_type), "websocket.sendBytes.data")
                        .map_err(|error| error.to_string())?;
                    let data = self.emit_expr_as(&args[0], &array_type)?;
                    self.builder
                        .build_store(slot, data)
                        .map_err(|error| error.to_string())?;
                    let function = self.declare_libc_fn(
                        "tyrus_websocket_send_bytes",
                        &[pointer.into(), pointer.into()],
                        Some(pointer.into()),
                    );
                    let promise = self
                        .builder
                        .build_call(
                            function,
                            &[socket.into(), slot.into()],
                            "websocket.sendBytes",
                        )
                        .map_err(|error| error.to_string())?
                        .try_as_basic_value()
                        .basic()
                        .ok_or("internal: WebSocket.sendBytes did not return a promise")?;
                    self.emit_async_cleanup_source_expression(data, &array_type, &args[0])?;
                    return Ok(Some(promise));
                }
                "protocol" if args.is_empty() => {
                    let function = self.declare_libc_fn(
                        "tyrus_websocket_protocol",
                        &[pointer.into()],
                        Some(self.string_abi_type()),
                    );
                    let protocol = self
                        .builder
                        .build_call(function, &[socket.into()], "websocket.protocol")
                        .map_err(|error| error.to_string())?
                        .try_as_basic_value()
                        .basic()
                        .ok_or("internal: WebSocket.protocol did not return a string")?;
                    self.record_owned_native_result(protocol, Type::Str);
                    return Ok(Some(protocol));
                }
                "closeCode" if args.is_empty() => {
                    let function = self.declare_libc_fn(
                        "tyrus_websocket_close_code",
                        &[pointer.into()],
                        Some(pointer.into()),
                    );
                    let code = self
                        .builder
                        .build_call(function, &[socket.into()], "websocket.closeCode")
                        .map_err(|error| error.to_string())?
                        .try_as_basic_value()
                        .basic()
                        .ok_or("internal: WebSocket.closeCode did not return a value")?;
                    self.record_owned_native_result(code, Type::Optional(Box::new(Type::I32)));
                    return Ok(Some(code));
                }
                _ => {
                    return (Err(format!("error: unsupported WebSocket method `{method}`")))
                        .map(Some);
                }
            }
        }

        if matches!(self.expr_type_hint(object), Some(Type::Named(ref name)) if name == "ChildProcess")
        {
            if !args.is_empty() {
                return (Err(format!("error: ChildProcess.{method} expects 0 arguments")))
                    .map(Some);
            }
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let child = self.emit_expr(object)?.into_pointer_value();
            let symbol = match method {
                "wait" => "tyrus_child_process_wait",
                "kill" => "tyrus_child_process_kill",
                _ => {
                    return (Err(format!("error: unsupported ChildProcess method `{method}`")))
                        .map(Some);
                }
            };
            let function = self.declare_libc_fn(symbol, &[pointer.into()], Some(pointer.into()));
            return (self
                .builder
                .build_call(function, &[child.into()], &format!("process.{method}"))
                .map_err(|error| error.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or_else(|| format!("internal: ChildProcess.{method} did not return a value")))
            .map(Some);
        }

        if matches!(&self.expr_type_hint(object), Some(Type::Named(name)) if name == "FSWatcher")
            && method == "close"
            && args.is_empty()
        {
            let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
            let watcher = self.emit_expr(object)?.into_pointer_value();
            let fn_val = self.declare_libc_fn("tyrus_fs_watcher_close", &[ptr_type.into()], None);
            self.builder
                .build_call(fn_val, &[watcher.into()], "fs_watcher_close")
                .unwrap();
            return Ok(Some(self.context.bool_type().const_zero().into()));
        }

        if matches!(self.expr_type_hint(object), Some(Type::Named(ref name)) if name == "TcpStream")
        {
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let stream = self.emit_expr(object)?.into_pointer_value();
            match method {
                "close" if args.is_empty() => {
                    let close =
                        self.declare_libc_fn("tyrus_net_tcp_stream_close", &[pointer.into()], None);
                    self.builder
                        .build_call(close, &[stream.into()], "net.tcpStream.close")
                        .unwrap();
                    return Ok(Some(self.context.bool_type().const_zero().into()));
                }
                "stream" if args.is_empty() => {
                    let duplex = self.declare_libc_fn(
                        "tyrus_net_tcp_stream_duplex",
                        &[pointer.into()],
                        Some(pointer.into()),
                    );
                    return (self
                        .builder
                        .build_call(duplex, &[stream.into()], "net.tcpStream.stream")
                        .map_err(|error| error.to_string())?
                        .try_as_basic_value()
                        .basic()
                        .ok_or_else(|| "internal: TcpStream.stream returned nothing".to_string()))
                    .map(Some);
                }
                "read" if args.len() == 1 || args.len() == 2 => {
                    let max_bytes = self.emit_expr_as(&args[0], &Type::U32)?;
                    let timeout = if args.len() == 2 {
                        self.emit_expr_as(&args[1], &Type::Optional(Box::new(Type::U64)))?
                    } else {
                        pointer.const_null().into()
                    };
                    let read = self.declare_libc_fn(
                        "tyrus_net_tcp_stream_read",
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
                            read,
                            &[stream.into(), max_bytes.into(), timeout.into()],
                            "net.tcpStream.read",
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
                "write" if args.len() == 1 || args.len() == 2 => {
                    let array_ty = self.llvm_type(&Type::Array(Box::new(Type::U8)));
                    let slot = self
                        .builder
                        .build_alloca(array_ty, "net.tcpStream.write.data")
                        .unwrap();
                    let data = self.emit_expr_as(&args[0], &Type::Array(Box::new(Type::U8)))?;
                    self.builder.build_store(slot, data).unwrap();
                    let timeout = if args.len() == 2 {
                        self.emit_expr_as(&args[1], &Type::Optional(Box::new(Type::U64)))?
                    } else {
                        pointer.const_null().into()
                    };
                    let write = self.declare_libc_fn(
                        "tyrus_net_tcp_stream_write",
                        &[pointer.into(), pointer.into(), pointer.into()],
                        Some(pointer.into()),
                    );
                    let result = self
                        .builder
                        .build_call(
                            write,
                            &[stream.into(), slot.into(), timeout.into()],
                            "net.tcpStream.write",
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
                "shutdown" if args.is_empty() => {
                    let shutdown = self.declare_libc_fn(
                        "tyrus_net_tcp_stream_shutdown",
                        &[pointer.into()],
                        Some(pointer.into()),
                    );
                    return Ok(Some(
                        self.builder
                            .build_call(shutdown, &[stream.into()], "net.tcpStream.shutdown")
                            .unwrap()
                            .try_as_basic_value()
                            .basic()
                            .unwrap(),
                    ));
                }
                "localAddress" | "peerAddress" if args.is_empty() => {
                    let symbol = if method == "localAddress" {
                        "tyrus_net_tcp_stream_local_address"
                    } else {
                        "tyrus_net_tcp_stream_peer_address"
                    };
                    let address = self.declare_libc_fn(
                        symbol,
                        &[pointer.into()],
                        Some(self.string_abi_type()),
                    );
                    let result = self
                        .builder
                        .build_call(address, &[stream.into()], "net.tcpStream.address")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    self.record_owned_native_result(result, Type::Str);
                    return Ok(Some(result));
                }
                _ => {}
            }
        }

        Ok(None)
    }
}
