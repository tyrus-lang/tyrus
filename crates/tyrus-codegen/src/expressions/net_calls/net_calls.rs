use super::*;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_net_fn_call(
        &mut self,
        callee: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        match callee {
            "__tyrus_net_resolve" => {
                if !(args.len() == 1 || args.len() == 2) {
                    return (Err(format!(
                        "error: `resolve` expects 1 or 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let host = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let timeout = if args.len() == 2 {
                    self.emit_expr_as(&args[1], &Type::Optional(Box::new(Type::U64)))?
                } else {
                    self.context
                        .ptr_type(inkwell::AddressSpace::default())
                        .const_null()
                        .into()
                };
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let resolve = self.declare_libc_fn(
                    "tyrus_net_resolve",
                    &[self.string_abi_type(), pointer.into()],
                    Some(pointer.into()),
                );
                let promise = self
                    .builder
                    .build_call(resolve, &[host.into(), timeout.into()], "net.resolve")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.emit_native_string_argument_release(host, &args[0])?;
                return Ok(Some(promise));
            }
            "__tyrus_net_reverseLookup" => {
                if !(args.len() == 1 || args.len() == 2) {
                    return (Err(format!(
                        "error: `reverseLookup` expects 1 or 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let address = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let timeout = if args.len() == 2 {
                    self.emit_expr_as(&args[1], &Type::Optional(Box::new(Type::U64)))?
                } else {
                    self.context
                        .ptr_type(inkwell::AddressSpace::default())
                        .const_null()
                        .into()
                };
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let reverse = self.declare_libc_fn(
                    "tyrus_net_reverse_lookup",
                    &[self.string_abi_type(), pointer.into()],
                    Some(pointer.into()),
                );
                let promise = self
                    .builder
                    .build_call(
                        reverse,
                        &[address.into(), timeout.into()],
                        "net.reverseLookup",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.emit_native_string_argument_release(address, &args[0])?;
                return Ok(Some(promise));
            }
            "__tyrus_http_fetch" => {
                if args.is_empty() || args.len() > 2 {
                    return (Err(format!(
                        "error: `fetch` expects 1 to 2 argument(s), found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let url = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let options = match args.get(1) {
                    Some(options) => Some(self.emit_fetch_options(Some(options), None)?),
                    None => None,
                };
                let fetch = self.declare_libc_fn(
                    "tyrus_http_fetch",
                    &[self.string_abi_type(), pointer.into()],
                    Some(pointer.into()),
                );
                let argument = options.unwrap_or_else(|| pointer.const_null());
                let promise = self
                    .builder
                    .build_call(fetch, &[url.into(), argument.into()], "http.fetch")
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("internal: `fetch` did not return a promise")?;
                self.emit_native_string_argument_release(url, &args[0])?;
                return Ok(Some(promise));
            }
            "__tyrus_http_connectWebSocket" => {
                if !(args.len() == 1 || args.len() == 2) {
                    return (Err(format!(
                        "error: `connectWebSocket` expects 1 to 2 argument(s), found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let url = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let protocols = if args.len() == 2 {
                    self.emit_expr_as(
                        &args[1],
                        &Type::Optional(Box::new(Type::Array(Box::new(Type::Str)))),
                    )?
                } else {
                    pointer.const_null().into()
                };
                let connect = self.declare_libc_fn(
                    "tyrus_http_websocket_connect",
                    &[self.string_abi_type(), pointer.into()],
                    Some(pointer.into()),
                );
                let promise = self
                    .builder
                    .build_call(
                        connect,
                        &[url.into(), protocols.into()],
                        "http.connectWebSocket",
                    )
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("internal: `connectWebSocket` did not return a promise")?;
                if args.len() == 2 {
                    self.release_borrowed_optional_array_argument(&args[1], protocols, &Type::Str)?;
                }
                self.emit_native_string_argument_release(url, &args[0])?;
                return Ok(Some(promise));
            }
            "__tyrus_net_tcpConnect" => {
                if !(args.len() == 1 || args.len() == 2) {
                    return (Err(format!(
                        "error: `tcpConnect` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let address = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let timeout = if args.len() == 2 {
                    self.emit_expr_as(&args[1], &Type::Optional(Box::new(Type::U64)))?
                } else {
                    self.context
                        .ptr_type(inkwell::AddressSpace::default())
                        .const_null()
                        .into()
                };
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let connect = self.declare_libc_fn(
                    "tyrus_net_tcp_connect",
                    &[self.string_abi_type(), pointer.into()],
                    Some(pointer.into()),
                );
                let promise = self
                    .builder
                    .build_call(connect, &[address.into(), timeout.into()], "net.tcpConnect")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                if args.len() == 2 {
                    self.release_borrowed_optional_argument(&args[1], timeout);
                }
                self.emit_native_string_argument_release(address, &args[0])?;
                return Ok(Some(promise));
            }
            "__tyrus_net_tcpListen" => {
                if !(args.len() == 1 || args.len() == 2) {
                    return (Err(format!(
                        "error: `tcpListen` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let address = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let timeout = if args.len() == 2 {
                    self.emit_expr_as(&args[1], &Type::Optional(Box::new(Type::U64)))?
                } else {
                    self.context
                        .ptr_type(inkwell::AddressSpace::default())
                        .const_null()
                        .into()
                };
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let listen = self.declare_libc_fn(
                    "tyrus_net_tcp_listen",
                    &[self.string_abi_type(), pointer.into()],
                    Some(pointer.into()),
                );
                let promise = self
                    .builder
                    .build_call(listen, &[address.into(), timeout.into()], "net.tcpListen")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                if args.len() == 2 {
                    self.release_borrowed_optional_argument(&args[1], timeout);
                }
                self.emit_native_string_argument_release(address, &args[0])?;
                return Ok(Some(promise));
            }
            "__tyrus_net_udpBind" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `udpBind` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let address = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let bind = self.declare_libc_fn(
                    "tyrus_net_udp_bind",
                    &[self.string_abi_type()],
                    Some(pointer.into()),
                );
                let promise = self
                    .builder
                    .build_call(bind, &[address.into()], "net.udpBind")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.emit_native_string_argument_release(address, &args[0])?;
                return Ok(Some(promise));
            }
            "__tyrus_net_ping" | "__tyrus_net_traceroute" => {
                if !(args.len() == 1 || args.len() == 2) {
                    return (Err(format!(
                        "error: network probe expects 1 or 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let address = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let timeout = if args.len() == 2 {
                    self.emit_expr_as(&args[1], &Type::Optional(Box::new(Type::U64)))?
                } else {
                    self.context
                        .ptr_type(inkwell::AddressSpace::default())
                        .const_null()
                        .into()
                };
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let symbol = if callee == "__tyrus_net_ping" {
                    "tyrus_net_ping"
                } else {
                    "tyrus_net_traceroute"
                };
                let probe = self.declare_libc_fn(
                    symbol,
                    &[self.string_abi_type(), pointer.into()],
                    Some(pointer.into()),
                );
                let promise = self
                    .builder
                    .build_call(probe, &[address.into(), timeout.into()], "net.probe")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.emit_native_string_argument_release(address, &args[0])?;
                return Ok(Some(promise));
            }
            "__tyrus_net_supportsIcmp" => {
                if !args.is_empty() {
                    return (Err(format!(
                        "error: `supportsIcmp` expects 0 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let symbol = self.declare_libc_fn(
                    "tyrus_net_supports_icmp",
                    &[],
                    Some(self.context.bool_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(symbol, &[], "net.supportsIcmp")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "__tyrus_http_createRouter" => return (self.emit_http_router_new(args)).map(Some),
            "__tyrus_net_parseIpAddress" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `parseIpAddress` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let input = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val = self.declare_libc_fn(
                    "tyrus_net_parse_ip_address",
                    &[self.string_abi_type()],
                    Some(pointer.into()),
                );
                let result = self
                    .builder
                    .build_call(fn_val, &[input.into()], "net.parseIpAddress")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.record_owned_native_result(
                    result,
                    Type::Optional(Box::new(Type::Named("IpAddress".to_string()))),
                );
                return Ok(Some(result));
            }
            "__tyrus_net_parseSocketAddress" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `parseSocketAddress` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let input = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val = self.declare_libc_fn(
                    "tyrus_net_parse_socket_address",
                    &[self.string_abi_type()],
                    Some(pointer.into()),
                );
                let result = self
                    .builder
                    .build_call(fn_val, &[input.into()], "net.parseSocketAddress")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.record_owned_native_result(
                    result,
                    Type::Optional(Box::new(Type::Named("SocketAddress".to_string()))),
                );
                return Ok(Some(result));
            }
            _ => {}
        }
        Ok(None)
    }
}
