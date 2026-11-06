use super::*;
use crate::core::promise_payload_type;
use tyrus_core::ast::ExprKind;
use tyrus_core::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HttpServerOptionKind {
    Duration,
    ByteSize,
    Count,
    Flag,
}

#[derive(Debug, Clone, Copy)]
struct HttpServerNumericOption {
    name: &'static str,
    kind: HttpServerOptionKind,
    example: &'static str,
}

/// The numeric options, in the order the runtime struct declares them.
///
/// This order is the ABI: field `n + 1` of the struct is entry `n` here, and
/// the runtime reads them positionally. `rateLimit` occupies two entries
/// because one source object lowers to two numbers.
const HTTP_SERVER_NUMERIC_OPTIONS: &[HttpServerNumericOption] = &[
    HttpServerNumericOption {
        name: "requestTimeout",
        kind: HttpServerOptionKind::Duration,
        example: "30s",
    },
    HttpServerNumericOption {
        name: "maxConcurrentRequests",
        kind: HttpServerOptionKind::Count,
        example: "1024",
    },
    HttpServerNumericOption {
        name: "shutdownTimeout",
        kind: HttpServerOptionKind::Duration,
        example: "10s",
    },
    HttpServerNumericOption {
        name: "headerTimeout",
        kind: HttpServerOptionKind::Duration,
        example: "10s",
    },
    HttpServerNumericOption {
        name: "maxHeaderSize",
        kind: HttpServerOptionKind::ByteSize,
        example: "32kb",
    },
    HttpServerNumericOption {
        name: "maxConnections",
        kind: HttpServerOptionKind::Count,
        example: "512",
    },
    HttpServerNumericOption {
        name: "keepAlive",
        kind: HttpServerOptionKind::Flag,
        example: "true",
    },
    HttpServerNumericOption {
        name: "logger",
        kind: HttpServerOptionKind::Flag,
        example: "true",
    },
];

#[derive(Debug, Clone)]
struct HttpRouteSchema {
    part: String,
    schema: String,
}

#[derive(Debug, Clone)]
struct HttpRouteContract {
    request_schemas: Vec<HttpRouteSchema>,
    responses: Vec<(u16, Type)>,
    tags: Vec<String>,
}

impl HttpRouteContract {
    fn request_type(&self) -> Type {
        Type::Object(
            self.request_schemas
                .iter()
                .map(|schema| Field {
                    name: schema.part.clone(),
                    ty: tyrus_core::http_body::request_channel_type(&schema.part, &schema.schema),
                    readonly: true,
                    optional: false,
                    decorators: Vec::new(),
                    span: Span::DUMMY,
                })
                .collect(),
        )
    }

    fn reply_type(&self) -> Type {
        Type::Apply {
            name: "HttpReply".to_string(),
            args: vec![Type::Object(
                self.responses
                    .iter()
                    .map(|(status, schema)| Field {
                        name: status.to_string(),
                        ty: schema.clone(),
                        readonly: true,
                        optional: false,
                        decorators: Vec::new(),
                        span: Span::DUMMY,
                    })
                    .collect(),
            )],
        }
    }
}

enum HttpRouteHandler<'a> {
    Named(&'a str),
    Inline {
        is_async: bool,
        params: &'a [Param],
        ret_type: &'a Type,
        body: &'a [Stmt],
    },
}

struct HttpRouteOptions<'a> {
    contract: HttpRouteContract,
    handler: HttpRouteHandler<'a>,
    pre_handlers: Vec<&'a str>,
    body_limit: i64,
    rate_limit: Option<HttpRouteRateLimit<'a>>,
}

struct HttpRouteRateLimit<'a> {
    max: &'a Expr,
    global_max: Option<&'a Expr>,
    ban: Option<&'a Expr>,
    window_millis: i64,
    ban_window_millis: i64,
}

fn http_route_body_limit(fields: &[(String, Expr)]) -> i64 {
    let Some((
        _,
        Expr {
            kind: ExprKind::ObjectLit(config),
            ..
        },
    )) = fields.iter().find(|(field, _)| field == "config")
    else {
        return -1;
    };
    let Some((
        _,
        Expr {
            kind: ExprKind::StrLit(text),
            ..
        },
    )) = config.iter().find(|(field, _)| field == "bodyLimit")
    else {
        return -1;
    };
    match tyrus_core::http_config::parse_body_limit(text) {
        Ok(tyrus_core::http_config::BodyLimit::Off) => 0,
        Ok(tyrus_core::http_config::BodyLimit::Bytes(bytes)) => {
            i64::try_from(bytes).unwrap_or(i64::MAX)
        }
        Err(_) => -1,
    }
}

fn http_route_rate_limit(fields: &[(String, Expr)]) -> Option<HttpRouteRateLimit<'_>> {
    let (
        _,
        Expr {
            kind: ExprKind::ObjectLit(config),
            ..
        },
    ) = fields.iter().find(|(field, _)| field == "config")?
    else {
        return None;
    };
    let (
        _,
        Expr {
            kind: ExprKind::ObjectLit(rate_limit),
            ..
        },
    ) = config.iter().find(|(field, _)| field == "rateLimit")?
    else {
        return None;
    };
    let (_, max) = rate_limit.iter().find(|(field, _)| field == "max")?;
    let (
        _,
        Expr {
            kind: ExprKind::StrLit(window),
            ..
        },
    ) = rate_limit.iter().find(|(field, _)| field == "timeWindow")?
    else {
        return None;
    };
    let window_millis = tyrus_core::http_config::parse_time_window(window)
        .ok()?
        .as_millis()
        .try_into()
        .unwrap_or(i64::MAX);
    let global_max = rate_limit
        .iter()
        .find(|(field, _)| field == "globalMax")
        .map(|(_, value)| value);
    let ban = rate_limit
        .iter()
        .find(|(field, _)| field == "ban")
        .map(|(_, value)| value);
    let ban_window_millis = rate_limit
        .iter()
        .find(|(field, _)| field == "banWindow")
        .and_then(|(_, value)| match value.kind() {
            ExprKind::StrLit(window) => tyrus_core::http_config::parse_time_window(window)
                .ok()?
                .as_millis()
                .try_into()
                .ok(),
            _ => None,
        })
        .unwrap_or(window_millis);
    Some(HttpRouteRateLimit {
        max,
        global_max,
        ban,
        window_millis,
        ban_window_millis,
    })
}

#[path = "http_json/http_json.rs"]
mod http_json;
#[path = "http_routes/http_routes.rs"]
mod http_routes;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_http_reply_method(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        let object_ty = self.expr_type_hint(object)?;
        let is_reply = matches!(
            transparent_codegen_type(&object_ty),
            Type::Apply { name, .. } if name == "HttpReply"
        );
        let is_reply_status = matches!(
            transparent_codegen_type(&object_ty),
            Type::Apply { name, .. } if name == "__HttpReplyStatus"
        );
        if !is_reply && !is_reply_status {
            return None;
        }
        Some(match method {
            "setCookie" if is_reply => self.emit_http_reply_set_cookie(object, args),
            "clearCookie" if is_reply => self.emit_http_reply_clear_cookie(object, args),
            "setCookies" if is_reply => self.emit_http_reply_set_cookies(object, args),
            "clearCookies" if is_reply => self.emit_http_reply_clear_cookies(object, args),
            "header" if is_reply => self.emit_http_reply_header(object, args, false),
            "appendHeader" if is_reply => self.emit_http_reply_header(object, args, true),
            "removeHeader" if is_reply => self.emit_http_reply_remove_header(object, args),
            "type" if is_reply => self.emit_http_reply_type(object, args),
            "status" => self.emit_http_reply_status(object, args),
            "stream" => self.emit_http_reply_writable_stream(object, args),
            "send" => self.emit_http_reply_send(object, args),
            _ => Err(format!("error: unsupported HttpReply method `{method}`")),
        })
    }

    pub(super) fn emit_http_server_method(
        &mut self,
        object: &Expr,
        method: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let server = self.emit_expr(object)?.into_pointer_value();
        let symbol = match method {
            "close" => "tyrus_http_server_close",
            "port" => "tyrus_http_server_port",
            _ => return Err(format!("error: unsupported HttpServer method `{method}`")),
        };
        let returns = if method == "port" {
            self.context.i32_type().into()
        } else {
            pointer.into()
        };
        let function = self.declare_libc_fn(symbol, &[pointer.into()], Some(returns));
        self.builder
            .build_call(function, &[server.into()], &format!("http_server_{method}"))
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| format!("internal: `{method}` did not return a value"))
    }

    fn emit_http_server_options(
        &mut self,
        options: Option<&Expr>,
    ) -> Result<PointerValue<'ctx>, String> {
        let i64_type = self.context.i64_type();
        let string_type = self.string_abi_type();
        let mut field_types: Vec<inkwell::types::BasicTypeEnum<'ctx>> = vec![string_type];
        let numeric: inkwell::types::BasicTypeEnum<'ctx> = i64_type.into();
        field_types.extend(std::iter::repeat_n(
            numeric,
            HTTP_SERVER_NUMERIC_OPTIONS.len(),
        ));
        field_types.push(string_type);
        field_types.push(string_type);
        let options_type = self.context.struct_type(&field_types, false);
        let slot = self
            .builder
            .build_alloca(options_type, "http.server.options")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(slot, options_type.const_zero())
            .map_err(|error| error.to_string())?;
        let fields = match options {
            Some(options) => {
                let ExprKind::ObjectLit(fields) = options.kind() else {
                    return Err(
                        "error: `listen` options must be an object literal so their formats can be validated"
                            .to_string(),
                    );
                };
                fields.as_slice()
            }
            None => &[],
        };
        let mut numbers = vec![-1i64; HTTP_SERVER_NUMERIC_OPTIONS.len()];
        for (index, option) in HTTP_SERVER_NUMERIC_OPTIONS.iter().enumerate() {
            if let Some((_, value)) = fields.iter().find(|(name, _)| name == option.name) {
                numbers[index] = self.http_server_numeric_option(option, value)?;
            }
        }
        let host = match fields.iter().find(|(name, _)| name == "host") {
            Some((_, value)) => self.emit_expr_as(value, &Type::Str)?.into_struct_value(),
            None => self
                .const_string_abi("", "http.server.host")
                .into_struct_value(),
        };
        let (cert, key) = match fields.iter().find(|(name, _)| name == "tls") {
            Some((_, value)) => {
                let ExprKind::ObjectLit(tls) = value.kind() else {
                    return Err(
                        "error: `tls` must be an object literal like `{ cert: \"server.crt\", key: \"server.key\" }`"
                            .to_string(),
                    );
                };
                let mut path = |field: &str| -> Result<StructValue<'ctx>, String> {
                    let Some((_, value)) = tls.iter().find(|(name, _)| name == field) else {
                        return Err(format!("error: `tls.{field}` is required"));
                    };
                    Ok(self.emit_expr_as(value, &Type::Str)?.into_struct_value())
                };
                (path("cert")?, path("key")?)
            }
            None => (
                self.const_string_abi("", "http.server.tls.cert")
                    .into_struct_value(),
                self.const_string_abi("", "http.server.tls.key")
                    .into_struct_value(),
            ),
        };
        let mut value = self
            .builder
            .build_insert_value(
                options_type.const_zero(),
                host,
                0,
                "http.server.options.host",
            )
            .map_err(|error| error.to_string())?;
        for (index, number) in numbers.iter().enumerate() {
            value = self
                .builder
                .build_insert_value(
                    value,
                    i64_type.const_int(*number as u64, true),
                    (index + 1) as u32,
                    &format!(
                        "http.server.options.{}",
                        HTTP_SERVER_NUMERIC_OPTIONS[index].name
                    ),
                )
                .map_err(|error| error.to_string())?;
        }
        let cert_index = (HTTP_SERVER_NUMERIC_OPTIONS.len() + 1) as u32;
        let value = self
            .builder
            .build_insert_value(value, cert, cert_index, "http.server.options.tls_cert")
            .map_err(|error| error.to_string())?;
        let value = self
            .builder
            .build_insert_value(value, key, cert_index + 1, "http.server.options.tls_key")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(slot, value.into_struct_value())
            .map_err(|error| error.to_string())?;
        Ok(slot)
    }

    fn http_server_numeric_option(
        &mut self,
        option: &HttpServerNumericOption,
        value: &Expr,
    ) -> Result<i64, String> {
        match option.kind {
            HttpServerOptionKind::Duration => {
                let ExprKind::StrLit(window) = value.kind() else {
                    return Err(format!(
                        "error: `{}` must be a string literal like `\"{}\"`",
                        option.name, option.example
                    ));
                };
                Ok(tyrus_core::http_config::parse_time_window(window)
                    .map_err(|reason| format!("error: {reason}"))?
                    .as_millis()
                    .try_into()
                    .unwrap_or(i64::MAX))
            }
            HttpServerOptionKind::ByteSize => {
                let ExprKind::StrLit(size) = value.kind() else {
                    return Err(format!(
                        "error: `{}` must be a string literal like `\"{}\"`",
                        option.name, option.example
                    ));
                };
                match tyrus_core::http_config::parse_body_limit(size)
                    .map_err(|reason| format!("error: {reason}"))?
                {
                    tyrus_core::http_config::BodyLimit::Bytes(bytes) => {
                        Ok(i64::try_from(bytes).unwrap_or(i64::MAX))
                    }
                    tyrus_core::http_config::BodyLimit::Off => Err(format!(
                        "error: `{}` cannot be `\"off\"`; an unbounded header block is what a slowloris client sends",
                        option.name
                    )),
                }
            }
            HttpServerOptionKind::Count => match value.kind() {
                ExprKind::IntLit(limit) => Ok(*limit as i64),
                _ => Err(format!(
                    "error: `{}` must be an integer literal",
                    option.name
                )),
            },
            HttpServerOptionKind::Flag => match value.kind() {
                ExprKind::BoolLit(flag) => Ok(i64::from(*flag)),
                _ => Err(format!(
                    "error: `{}` must be `true` or `false`",
                    option.name
                )),
            },
        }
    }

    fn emit_http_reply_set_cookie(
        &mut self,
        object: &Expr,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let reply = self.emit_http_reply_receiver(object)?;
        self.emit_http_reply_set_cookie_on(reply, args)?;
        Ok(reply.into())
    }

    fn emit_http_reply_set_cookie_on(
        &mut self,
        reply: PointerValue<'ctx>,
        args: &[Expr],
    ) -> Result<(), String> {
        let ([name, value] | [name, value, _]) = args else {
            return Err(format!(
                "error: `reply.setCookie` expects 2 to 3 argument(s), found {}",
                args.len()
            ));
        };
        let name = self.emit_expr_as(name, &Type::Str)?.into_struct_value();
        let value = self.emit_expr_as(value, &Type::Str)?.into_struct_value();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_type = self.context.i32_type();
        if let Some(options) = args.get(2) {
            let options_type = Type::Named("CookieOptions".to_string());
            let options = self.emit_expr_as(options, &options_type)?;
            let options_slot = self
                .builder
                .build_alloca(self.llvm_type(&options_type), "http.cookie.options")
                .map_err(|error| error.to_string())?;
            self.builder.build_store(options_slot, options).unwrap();
            let function = self.declare_libc_fn(
                "tyrus_http_reply_set_cookie_with_options",
                &[
                    pointer.into(),
                    self.string_abi_type(),
                    self.string_abi_type(),
                    pointer.into(),
                ],
                Some(i32_type.into()),
            );
            self.builder
                .build_call(
                    function,
                    &[reply.into(), name.into(), value.into(), options_slot.into()],
                    "http.reply.set_cookie_with_options",
                )
                .map_err(|error| error.to_string())?;
            return Ok(());
        }
        let function = self.declare_libc_fn(
            "tyrus_http_reply_set_cookie",
            &[
                pointer.into(),
                self.string_abi_type(),
                self.string_abi_type(),
            ],
            Some(i32_type.into()),
        );
        self.builder
            .build_call(
                function,
                &[reply.into(), name.into(), value.into()],
                "http.reply.set_cookie",
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn emit_http_reply_set_cookies(
        &mut self,
        object: &Expr,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let reply = self.emit_http_reply_receiver(object)?;
        self.emit_http_reply_set_cookies_on(reply, args)?;
        Ok(reply.into())
    }

    fn emit_http_reply_set_cookies_on(
        &mut self,
        reply: PointerValue<'ctx>,
        args: &[Expr],
    ) -> Result<(), String> {
        let [cookies] = args else {
            return Err(format!(
                "error: `reply.setCookies` expects 1 argument(s), found {}",
                args.len()
            ));
        };
        let cookies_type = Type::Array(Box::new(Type::Named("HttpCookie".to_string())));
        let cookies = self.emit_expr_as(cookies, &cookies_type)?;
        let cookies_slot = self
            .builder
            .build_alloca(self.llvm_type(&cookies_type), "http.cookies")
            .map_err(|error| error.to_string())?;
        self.builder.build_store(cookies_slot, cookies).unwrap();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let function = self.declare_libc_fn(
            "tyrus_http_reply_set_cookies",
            &[pointer.into(), pointer.into()],
            Some(self.context.i32_type().into()),
        );
        self.builder
            .build_call(
                function,
                &[reply.into(), cookies_slot.into()],
                "http.reply.set_cookies",
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn emit_http_reply_clear_cookies(
        &mut self,
        object: &Expr,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let reply = self.emit_http_reply_receiver(object)?;
        self.emit_http_reply_clear_cookies_on(reply, args)?;
        Ok(reply.into())
    }

    fn emit_http_reply_clear_cookies_on(
        &mut self,
        reply: PointerValue<'ctx>,
        args: &[Expr],
    ) -> Result<(), String> {
        if !args.is_empty() {
            return Err(format!(
                "error: `reply.clearCookies` expects 0 argument(s), found {}",
                args.len()
            ));
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let function = self.declare_libc_fn(
            "tyrus_http_reply_clear_cookies",
            &[pointer.into()],
            Some(self.context.i32_type().into()),
        );
        self.builder
            .build_call(function, &[reply.into()], "http.reply.clear_cookies")
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn emit_http_reply_clear_cookie(
        &mut self,
        object: &Expr,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let reply = self.emit_http_reply_receiver(object)?;
        self.emit_http_reply_clear_cookie_on(reply, args)?;
        Ok(reply.into())
    }

    fn emit_http_reply_clear_cookie_on(
        &mut self,
        reply: PointerValue<'ctx>,
        args: &[Expr],
    ) -> Result<(), String> {
        let [name] = args else {
            return Err(format!(
                "error: `reply.clearCookie` expects 1 argument(s), found {}",
                args.len()
            ));
        };
        let name = self.emit_expr_as(name, &Type::Str)?.into_struct_value();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_type = self.context.i32_type();
        let function = self.declare_libc_fn(
            "tyrus_http_reply_clear_cookie",
            &[pointer.into(), self.string_abi_type()],
            Some(i32_type.into()),
        );
        self.builder
            .build_call(
                function,
                &[reply.into(), name.into()],
                "http.reply.clear_cookie",
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn emit_http_reply_header(
        &mut self,
        object: &Expr,
        args: &[Expr],
        append: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let reply = self.emit_http_reply_receiver(object)?;
        self.emit_http_reply_header_on(reply, args, append)?;
        Ok(reply.into())
    }

    fn emit_http_reply_header_on(
        &mut self,
        reply: PointerValue<'ctx>,
        args: &[Expr],
        append: bool,
    ) -> Result<(), String> {
        let [name, value] = args else {
            let method = if append { "appendHeader" } else { "header" };
            return Err(format!(
                "error: `reply.{method}` expects 2 argument(s), found {}",
                args.len()
            ));
        };
        let name = self.emit_expr_as(name, &Type::Str)?.into_struct_value();
        let value = self.emit_expr_as(value, &Type::Str)?.into_struct_value();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_type = self.context.i32_type();
        let function = self.declare_libc_fn(
            if append {
                "tyrus_http_reply_append_header"
            } else {
                "tyrus_http_reply_header"
            },
            &[
                pointer.into(),
                self.string_abi_type(),
                self.string_abi_type(),
            ],
            Some(i32_type.into()),
        );
        self.builder
            .build_call(
                function,
                &[reply.into(), name.into(), value.into()],
                "http.reply.header",
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn emit_http_reply_remove_header(
        &mut self,
        object: &Expr,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let reply = self.emit_http_reply_receiver(object)?;
        self.emit_http_reply_remove_header_on(reply, args)?;
        Ok(reply.into())
    }

    fn emit_http_reply_remove_header_on(
        &mut self,
        reply: PointerValue<'ctx>,
        args: &[Expr],
    ) -> Result<(), String> {
        let [name] = args else {
            return Err(format!(
                "error: `reply.removeHeader` expects 1 argument(s), found {}",
                args.len()
            ));
        };
        let name = self.emit_expr_as(name, &Type::Str)?.into_struct_value();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_type = self.context.i32_type();
        let function = self.declare_libc_fn(
            "tyrus_http_reply_remove_header",
            &[pointer.into(), self.string_abi_type()],
            Some(i32_type.into()),
        );
        self.builder
            .build_call(
                function,
                &[reply.into(), name.into()],
                "http.reply.remove_header",
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn emit_http_reply_type(
        &mut self,
        object: &Expr,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let reply = self.emit_http_reply_receiver(object)?;
        self.emit_http_reply_type_on(reply, args)?;
        Ok(reply.into())
    }

    fn emit_http_reply_type_on(
        &mut self,
        reply: PointerValue<'ctx>,
        args: &[Expr],
    ) -> Result<(), String> {
        let [value] = args else {
            return Err(format!(
                "error: `reply.type` expects 1 argument(s), found {}",
                args.len()
            ));
        };
        let value = self.emit_expr_as(value, &Type::Str)?.into_struct_value();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_type = self.context.i32_type();
        let function = self.declare_libc_fn(
            "tyrus_http_reply_type",
            &[pointer.into(), self.string_abi_type()],
            Some(i32_type.into()),
        );
        self.builder
            .build_call(function, &[reply.into(), value.into()], "http.reply.type")
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn emit_http_reply_status(
        &mut self,
        object: &Expr,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let reply = self.emit_http_reply_receiver(object)?;
        self.emit_http_reply_status_on(reply, args)?;
        Ok(reply.into())
    }

    fn emit_http_reply_status_on(
        &mut self,
        reply: PointerValue<'ctx>,
        args: &[Expr],
    ) -> Result<(), String> {
        let [status] = args else {
            return Err(format!(
                "error: `reply.status` expects 1 argument, found {}",
                args.len()
            ));
        };
        let status = self.emit_expr_as(status, &Type::I32)?.into_int_value();
        let response_type = self
            .llvm_type(&Type::Named("HttpResponse".to_string()))
            .into_struct_type();
        let status_ptr = self
            .builder
            .build_struct_gep(response_type, reply, 0, "http.reply.status.ptr")
            .map_err(|error| error.to_string())?;
        self.builder.build_store(status_ptr, status).unwrap();
        Ok(())
    }

    fn emit_http_reply_send(
        &mut self,
        object: &Expr,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let reply = self.emit_http_reply_receiver(object)?;
        let body_forbidden = self
            .http_reply_send_status(object)
            .is_some_and(|status| matches!(status, 100..=199 | 204 | 205 | 304));
        let (body_argument, options) = match args {
            [] => (None, None),
            [options] if body_forbidden => (None, Some(options)),
            [body] => (Some(body), None),
            [body, options] => (Some(body), Some(options)),
            _ => {
                return Err(format!(
                    "error: `reply.send` expects 0 to 2 argument(s), found {}",
                    args.len()
                ));
            }
        };
        if let Some(options) = options {
            self.emit_http_reply_send_options(reply, options)?;
        }
        let body = if let Some(value) = body_argument {
            let (_status, schema) = self
                .http_reply_send_contract(object)
                .ok_or_else(|| "error: cannot infer HttpReply response schema".to_string())?;
            let schema = self.http_response_schema_for_value(schema, value);
            if matches!(transparent_codegen_type(&schema), Type::Named(name) if name == "TextBody")
            {
                let text = self.emit_expr_as(value, &Type::Str)?.into_struct_value();
                // The response outlives the handler and releases this body
                // itself, so it needs a reference of its own. Without one, a
                // body the handler owns is freed twice: once by the handler's
                // own cleanup on the way out, and again by `finalize_response`,
                // which read it in between. That read is a use-after-free, and
                // AddressSanitizer stopped the process in the middle of
                // answering the request, which is why the symptom was a route
                // that never replied rather than a crash anyone could place.
                //
                // Cheap enough to do unconditionally: a `string` shares its
                // buffer by counting, so this is an increment for an allocated
                // body and nothing at all for one over `.rodata`. The JSON path
                // below deliberately does not do this: the string it stores is
                // built for the response and has no other owner.
                let shared = self
                    .emit_string_clone(text, "http.reply.text.body")?
                    .into_struct_value();
                self.emit_http_reply_body(reply, shared, "text/plain; charset=utf-8")?;
                return Ok(self.context.i32_type().const_zero().into());
            }
            if matches!(transparent_codegen_type(&schema), Type::Named(name) if name == "BytesBody")
            {
                let bytes = self
                    .emit_expr_as(value, &Type::Array(Box::new(Type::U8)))?
                    .into_struct_value();
                self.emit_http_reply_bytes_body(reply, bytes)?;
                return Ok(self.context.i32_type().const_zero().into());
            }
            if matches!(transparent_codegen_type(&schema), Type::Named(name) if name == "StreamBody")
            {
                let stream = self
                    .emit_expr_as(
                        value,
                        &Type::Apply {
                            name: "ReadableStream".to_string(),
                            args: vec![Type::Array(Box::new(Type::U8))],
                        },
                    )?
                    .into_pointer_value();
                self.emit_http_reply_stream_body(reply, stream)?;
                return Ok(self.context.i32_type().const_zero().into());
            }
            let response = self.emit_expr_as(value, &schema)?;
            match transparent_codegen_type(&schema) {
                Type::Named(name) => {
                    let response = response.into_struct_value();
                    let validator = self.ensure_named_object_validation_predicate_function(name)?;
                    let valid = self
                        .builder
                        .build_call(validator, &[response.into()], "http.reply.response.valid")
                        .map_err(|error| error.to_string())?
                        .try_as_basic_value()
                        .basic()
                        .ok_or_else(|| {
                            "internal: HTTP response validator did not return a value".to_string()
                        })?
                        .into_int_value();
                    self.emit_http_response_error_if_invalid(
                        valid,
                        reply,
                        500,
                        "the response body does not satisfy the declared schema",
                    );
                    self.emit_http_schema_response_json(name, response)?
                }
                Type::Array(element) => {
                    self.emit_automatic_json_array_encode(response.into_struct_value(), element)?
                }
                Type::Object(_) => {
                    self.emit_automatic_json_encode(response.into_struct_value(), &schema, false)?
                }
                Type::Union(_) => self.emit_automatic_json_value_encode(response, &schema)?,
                _ => self.emit_automatic_json_value_encode(response, &schema)?,
            }
        } else {
            self.const_string_abi("", "http.reply.empty.body")
                .into_struct_value()
        };
        self.emit_http_reply_body(reply, body, "application/json; charset=utf-8")?;
        Ok(self.context.i32_type().const_zero().into())
    }

    fn emit_http_reply_body(
        &mut self,
        reply: PointerValue<'ctx>,
        body: StructValue<'ctx>,
        content_type: &str,
    ) -> Result<(), String> {
        let response_type = self
            .llvm_type(&Type::Named("HttpResponse".to_string()))
            .into_struct_type();
        let body_variants = vec![Type::Str, Type::Array(Box::new(Type::U8))];
        let body_slot =
            self.entry_alloca(self.union_llvm_type(&body_variants), "http.reply.body")?;
        self.store_union_variant(&body_variants, body_slot, 0, body.into())?;
        let body = self
            .builder
            .build_load(
                self.union_llvm_type(&body_variants),
                body_slot,
                "http.reply.body.value",
            )
            .map_err(|error| error.to_string())?;
        let body_ptr = self
            .builder
            .build_struct_gep(response_type, reply, 1, "http.reply.body.ptr")
            .map_err(|error| error.to_string())?;
        self.builder.build_store(body_ptr, body).unwrap();
        self.emit_http_reply_content_type(reply, content_type)
    }

    fn emit_http_reply_bytes_body(
        &mut self,
        reply: PointerValue<'ctx>,
        bytes: StructValue<'ctx>,
    ) -> Result<(), String> {
        let array_type = self
            .llvm_type(&Type::Array(Box::new(Type::U8)))
            .into_struct_type();
        let slot = self.entry_alloca(array_type, "http.reply.bytes")?;
        self.builder.build_store(slot, bytes).unwrap();
        let data_ptr = self
            .builder
            .build_struct_gep(array_type, slot, 0, "http.reply.bytes.data.ptr")
            .map_err(|error| error.to_string())?;
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let data = self
            .builder
            .build_load(pointer, data_ptr, "http.reply.bytes.data")
            .map_err(|error| error.to_string())?;
        let length_ptr = self
            .builder
            .build_struct_gep(array_type, slot, 1, "http.reply.bytes.length.ptr")
            .map_err(|error| error.to_string())?;
        let length = self
            .builder
            .build_load(self.usize_type(), length_ptr, "http.reply.bytes.length")
            .map_err(|error| error.to_string())?;
        let function = self.declare_libc_fn(
            "tyrus_http_reply_bytes",
            &[pointer.into(), pointer.into(), self.usize_type().into()],
            Some(self.context.i32_type().into()),
        );
        self.builder
            .build_call(
                function,
                &[reply.into(), data.into(), length.into()],
                "http.reply.bytes.store",
            )
            .map_err(|error| error.to_string())?;
        self.emit_http_reply_content_type(reply, "application/octet-stream")
    }

    fn emit_http_reply_stream_body(
        &mut self,
        reply: PointerValue<'ctx>,
        stream: PointerValue<'ctx>,
    ) -> Result<(), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let function = self.declare_libc_fn(
            "tyrus_http_reply_stream",
            &[pointer.into(), pointer.into()],
            Some(self.context.i32_type().into()),
        );
        self.builder
            .build_call(
                function,
                &[reply.into(), stream.into()],
                "http.reply.stream.store",
            )
            .map_err(|error| error.to_string())?;
        self.emit_http_reply_content_type(reply, "application/octet-stream")
    }

    fn emit_http_reply_writable_stream(
        &mut self,
        object: &Expr,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if !args.is_empty() {
            return Err(format!(
                "error: `reply.stream` expects 0 argument(s), found {}",
                args.len()
            ));
        }
        let reply = self.emit_http_reply_receiver(object)?;
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let out = self.entry_alloca(pointer, "http.reply.writable_stream")?;
        self.builder
            .build_store(out, pointer.const_null())
            .map_err(|error| error.to_string())?;
        let function = self.declare_libc_fn(
            "tyrus_http_reply_writable_stream",
            &[pointer.into(), pointer.into()],
            Some(self.context.i32_type().into()),
        );
        self.builder
            .build_call(
                function,
                &[reply.into(), out.into()],
                "http.reply.writable_stream.open",
            )
            .map_err(|error| error.to_string())?;
        self.emit_http_reply_content_type(reply, "application/octet-stream")?;
        self.builder
            .build_load(pointer, out, "http.reply.writable_stream.value")
            .map_err(|error| error.to_string())
    }

    fn emit_http_reply_content_type(
        &mut self,
        reply: PointerValue<'ctx>,
        content_type: &str,
    ) -> Result<(), String> {
        let response_type = self
            .llvm_type(&Type::Named("HttpResponse".to_string()))
            .into_struct_type();
        let content_type = self.emit_optional_box(
            self.const_string_abi(content_type, "http.reply.content_type"),
            &Type::Str,
        )?;
        let content_type_ptr = self
            .builder
            .build_struct_gep(response_type, reply, 2, "http.reply.content_type.ptr")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(content_type_ptr, content_type)
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn emit_http_reply_send_options(
        &mut self,
        reply: PointerValue<'ctx>,
        options: &Expr,
    ) -> Result<(), String> {
        let options_type = Type::Named("HttpReplyOptions".to_string());
        let options = self.emit_expr_as(options, &options_type)?;
        let options_slot = self
            .builder
            .build_alloca(self.llvm_type(&options_type), "http.reply.options")
            .map_err(|error| error.to_string())?;
        self.builder.build_store(options_slot, options).unwrap();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let function = self.declare_libc_fn(
            "tyrus_http_reply_apply_options",
            &[pointer.into(), pointer.into()],
            Some(self.context.i32_type().into()),
        );
        self.builder
            .build_call(
                function,
                &[reply.into(), options_slot.into()],
                "http.reply.apply_options",
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    /// Lowers a fluent reply chain iteratively. Calling `emit_expr` for every
    /// receiver made one full `emit_expr` frame nest per method; ASan's red
    /// zones turned the otherwise valid four-method HTTP chain into a test
    /// thread stack overflow. The chain is lexical and side-effect ordered, so
    /// collecting it and replaying it from the base reply preserves evaluation
    /// order without recursion or extra runtime state.
    fn emit_http_reply_receiver(&mut self, object: &Expr) -> Result<PointerValue<'ctx>, String> {
        let mut receiver = object;
        let mut chain: Vec<(&str, &[Expr])> = Vec::new();
        while let ExprKind::MethodCall {
            object,
            method,
            args,
            ..
        } = receiver.kind()
        {
            if !matches!(
                method.as_str(),
                "setCookie"
                    | "clearCookie"
                    | "setCookies"
                    | "clearCookies"
                    | "header"
                    | "appendHeader"
                    | "removeHeader"
                    | "type"
                    | "status"
            ) {
                break;
            }
            chain.push((method.as_str(), args.as_slice()));
            receiver = object;
        }

        let reply = self.emit_expr(receiver)?.into_pointer_value();
        for (method, args) in chain.into_iter().rev() {
            match method {
                "setCookie" => self.emit_http_reply_set_cookie_on(reply, args)?,
                "clearCookie" => self.emit_http_reply_clear_cookie_on(reply, args)?,
                "setCookies" => self.emit_http_reply_set_cookies_on(reply, args)?,
                "clearCookies" => self.emit_http_reply_clear_cookies_on(reply, args)?,
                "header" => self.emit_http_reply_header_on(reply, args, false)?,
                "appendHeader" => self.emit_http_reply_header_on(reply, args, true)?,
                "removeHeader" => self.emit_http_reply_remove_header_on(reply, args)?,
                "type" => self.emit_http_reply_type_on(reply, args)?,
                "status" => self.emit_http_reply_status_on(reply, args)?,
                _ => unreachable!("reply chain methods are filtered above"),
            }
        }
        Ok(reply)
    }

    fn http_reply_send_contract(&self, object: &Expr) -> Option<(u16, Type)> {
        match self.expr_type_hint(object)? {
            Type::Apply { name, args } if name == "__HttpReplyStatus" && args.len() == 2 => {
                let Type::Lit(status) = args.first()? else {
                    return None;
                };
                let ExprKind::IntLit(status) = status.kind() else {
                    return None;
                };
                let status = u16::try_from(*status).ok()?;
                http_response_schema_name(args.get(1)?, status).map(|schema| (status, schema))
            }
            Type::Apply { name, args } if name == "HttpReply" && args.len() == 1 => {
                match args.first()? {
                    Type::Object(fields) if fields.len() == 1 => {
                        let status = fields[0].name.parse().ok()?;
                        http_response_schema_name(&fields[0].ty, status)
                            .map(|schema| (status, schema))
                    }
                    schema => http_response_schema_name(schema, 200).map(|schema| (200, schema)),
                }
            }
            _ => None,
        }
    }

    fn http_response_schema_for_value(&self, schema: Type, value: &Expr) -> Type {
        let Type::Union(variants) = transparent_codegen_type(&schema) else {
            return schema;
        };
        if let Some(value_type) = self.expr_type_hint(value) {
            let matching = variants
                .iter()
                .filter(|variant| self.types_assignable(&value_type, variant))
                .cloned()
                .collect::<Vec<_>>();
            if let Some(variant) = self.unambiguous_response_variant(&matching) {
                return variant;
            }
        }
        match value.kind() {
            ExprKind::ArrayLit(_) | ExprKind::ArrayRepeat { .. } => variants
                .iter()
                .find(|variant| matches!(transparent_codegen_type(variant), Type::Array(_)))
                .cloned()
                .unwrap_or(schema),
            ExprKind::ObjectLit(fields) => {
                let keys = fields
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .collect::<Vec<_>>();
                let matching = variants
                    .iter()
                    .filter(|variant| {
                        let Some(fields) = self.declared_fields_with_index(variant) else {
                            return false;
                        };
                        keys.iter()
                            .all(|key| fields.iter().any(|(_, field)| field.name == *key))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                if let Some(variant) = self.unambiguous_response_variant(&matching) {
                    variant
                } else {
                    schema
                }
            }
            ExprKind::StrLit(_) => variants
                .iter()
                .find(|variant| matches!(transparent_codegen_type(variant), Type::Str))
                .cloned()
                .unwrap_or(schema),
            ExprKind::BoolLit(_) => variants
                .iter()
                .find(|variant| matches!(transparent_codegen_type(variant), Type::Bool))
                .cloned()
                .unwrap_or(schema),
            _ => schema,
        }
    }

    fn unambiguous_response_variant(&self, variants: &[Type]) -> Option<Type> {
        let first = variants.first()?;
        variants
            .iter()
            .skip(1)
            .all(|variant| self.response_variants_have_same_shape(first, variant))
            .then(|| first.clone())
    }

    fn response_variants_have_same_shape(&self, left: &Type, right: &Type) -> bool {
        let (Some(left_fields), Some(right_fields)) =
            (self.declared_fields(left), self.declared_fields(right))
        else {
            return false;
        };
        left_fields.len() == right_fields.len()
            && left_fields.iter().all(|left_field| {
                right_fields
                    .iter()
                    .find(|right_field| right_field.name == left_field.name)
                    .is_some_and(|right_field| {
                        left_field.ty == right_field.ty
                            && left_field.readonly == right_field.readonly
                            && left_field.optional == right_field.optional
                            && left_field.decorators.len() == right_field.decorators.len()
                            && left_field
                                .decorators
                                .iter()
                                .zip(&right_field.decorators)
                                .all(|(left, right)| {
                                    left.name == right.name
                                        && left.display_name == right.display_name
                                        && left.args == right.args
                                })
                    })
            })
    }

    fn http_reply_send_status(&self, object: &Expr) -> Option<u16> {
        match self.expr_type_hint(object)? {
            Type::Apply { name, args } if name == "__HttpReplyStatus" && args.len() == 2 => {
                let Type::Lit(status) = args.first()? else {
                    return None;
                };
                let ExprKind::IntLit(status) = status.kind() else {
                    return None;
                };
                u16::try_from(*status).ok()
            }
            Type::Apply { name, .. } if name == "HttpReply" => Some(200),
            _ => None,
        }
    }

    pub(super) fn emit_fetch_options(
        &mut self,
        options: Option<&Expr>,
        verb_method: Option<&str>,
    ) -> Result<PointerValue<'ctx>, String> {
        let empty: Vec<(String, Expr)> = Vec::new();
        let fields: &[(String, Expr)] = match options {
            None => &empty,
            Some(options) => {
                let ExprKind::ObjectLit(fields) = options.kind() else {
                    return Err(
                        "error: `fetch` options must be written out at the call, as in `{ method: \"POST\", timeout: \"10s\" }`"
                            .to_string(),
                    );
                };
                fields
            }
        };
        let options_type = Type::Named("FetchOptions".to_string());
        let struct_type = self.llvm_type(&options_type).into_struct_type();
        let mut value = struct_type.const_zero();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());

        let boxed = |this: &mut Self, name: &str, ty: Type| -> Result<_, String> {
            match fields.iter().find(|(field, _)| field == name) {
                Some((_, expr)) => {
                    let inner = this.emit_expr_as(expr, &ty)?;
                    this.emit_optional_box(inner, &ty)
                }
                None => Ok(pointer.const_null()),
            }
        };
        if let Some(verb_method) = verb_method {
            let literal = self.const_string_abi(verb_method, "http.fetch.method");
            let boxed = self.emit_optional_box(literal, &Type::Str)?;
            value = self
                .builder
                .build_insert_value(value, boxed, 0, "http.fetch.method")
                .map_err(|error| error.to_string())?
                .into_struct_value();
        }
        for (index, (name, ty)) in [
            ("method", Type::Str),
            (
                "headers",
                Type::Array(Box::new(Type::Named("HttpHeader".to_string()))),
            ),
            ("body", Type::Str),
        ]
        .into_iter()
        .enumerate()
        {
            if index == 0 && verb_method.is_some() {
                continue;
            }
            let field = boxed(self, name, ty)?;
            value = self
                .builder
                .build_insert_value(value, field, index as u32, &format!("http.fetch.{name}"))
                .map_err(|error| error.to_string())?
                .into_struct_value();
        }

        let i64_type = self.context.i64_type();
        let numbers: [(&str, i64); 3] = [
            (
                "timeout",
                match fields.iter().find(|(field, _)| field == "timeout") {
                    Some((_, value)) => {
                        let ExprKind::StrLit(text) = value.kind() else {
                            return Err(
                                "error: `fetch` option `timeout` must be a string literal like `\"10s\"`"
                                    .to_string(),
                            );
                        };
                        i64::try_from(
                            tyrus_core::http_config::parse_time_window(text)
                                .map_err(|reason| format!("error: {reason}"))?
                                .as_millis(),
                        )
                        .unwrap_or(i64::MAX)
                    }
                    None => -1,
                },
            ),
            (
                "maxBodySize",
                match fields.iter().find(|(field, _)| field == "maxBodySize") {
                    Some((_, value)) => {
                        let ExprKind::StrLit(text) = value.kind() else {
                            return Err(
                                "error: `fetch` option `maxBodySize` must be a string literal like `\"2mb\"`"
                                    .to_string(),
                            );
                        };
                        match tyrus_core::http_config::parse_body_limit(text)
                            .map_err(|reason| format!("error: {reason}"))?
                        {
                            tyrus_core::http_config::BodyLimit::Bytes(bytes) => {
                                i64::try_from(bytes).unwrap_or(i64::MAX)
                            }
                            tyrus_core::http_config::BodyLimit::Off => i64::MAX,
                        }
                    }
                    None => -1,
                },
            ),
            (
                "redirects",
                match fields.iter().find(|(field, _)| field == "redirects") {
                    Some((_, value)) => match value.kind() {
                        ExprKind::IntLit(number) => i64::try_from(*number).unwrap_or(i64::MAX),
                        ExprKind::UIntLit(number) => i64::try_from(*number).unwrap_or(i64::MAX),
                        _ => {
                            return Err(
                                "error: `fetch` option `redirects` must be an integer literal"
                                    .to_string(),
                            );
                        }
                    },
                    None => -1,
                },
            ),
        ];
        for (offset, (name, number)) in numbers.iter().enumerate() {
            value = self
                .builder
                .build_insert_value(
                    value,
                    i64_type.const_int(*number as u64, true),
                    (offset + 3) as u32,
                    &format!("http.fetch.{name}"),
                )
                .map_err(|error| error.to_string())?
                .into_struct_value();
        }
        let body_stream = match fields.iter().find(|(field, _)| field == "bodyStream") {
            Some((_, source)) => {
                let source = self
                    .emit_expr_as(
                        source,
                        &Type::Apply {
                            name: "ReadableStream".to_string(),
                            args: vec![Type::Array(Box::new(Type::U8))],
                        },
                    )?
                    .into_pointer_value();
                let retain = self.declare_libc_fn(
                    "tyrus_readable_stream_retain",
                    &[pointer.into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(retain, &[source.into()], "http.fetch.body_stream.retain")
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("internal: retaining fetch body stream returned nothing")?
                    .into_pointer_value()
            }
            None => pointer.const_null(),
        };
        value = self
            .builder
            .build_insert_value(value, body_stream, 6, "http.fetch.body_stream")
            .map_err(|error| error.to_string())?
            .into_struct_value();
        // `-1` when omitted, so the runtime keeps its default of decoding.
        let decompress = match fields.iter().find(|(field, _)| field == "decompress") {
            Some((_, flag)) => {
                let flag = self.emit_expr_as(flag, &Type::Bool)?.into_int_value();
                self.builder
                    .build_int_z_extend(flag, i64_type, "http.fetch.decompress")
                    .map_err(|error| error.to_string())?
            }
            None => i64_type.const_all_ones(),
        };
        value = self
            .builder
            .build_insert_value(value, decompress, 7, "http.fetch.decompress")
            .map_err(|error| error.to_string())?
            .into_struct_value();
        let slot = self.entry_alloca(struct_type, "http.fetch.options")?;
        self.builder
            .build_store(slot, value)
            .map_err(|error| error.to_string())?;
        Ok(slot)
    }

    fn emit_http_docs(
        &mut self,
        router: PointerValue<'ctx>,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let fields: &[(String, Expr)] = match args.first().map(tyrus_core::ast::Expr::kind) {
            Some(ExprKind::ObjectLit(fields)) => fields,
            Some(_) => return Err("error: `docs` options must be an object literal".to_string()),
            None => &[],
        };
        let field = |name: &str| {
            fields
                .iter()
                .find(|(field, _)| field == name)
                .map(|(_, value)| value)
        };
        let mut strings = Vec::with_capacity(6);
        for name in ["path", "title", "version", "description", "css"] {
            let value = match field(name) {
                Some(value) => (
                    self.emit_expr_as(value, &Type::Str)?.into_struct_value(),
                    Some(value),
                ),
                None => (
                    self.llvm_type(&Type::Str).into_struct_type().const_zero(),
                    None,
                ),
            };
            strings.push(value);
        }
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
        let auth = match field("auth").and_then(tyrus_core::openapi::docs_auth_description) {
            Some(description) => self
                .emit_expr_as(&Expr::synthetic(ExprKind::StrLit(description)), &Type::Str)?
                .into_struct_value(),
            None => self.llvm_type(&Type::Str).into_struct_type().const_zero(),
        };
        let string_type = self.string_abi_type();
        let options_type = self.context.struct_type(&[string_type; 6], false);
        let options = self
            .builder
            .build_alloca(options_type, "http.docs.options")
            .map_err(|error| error.to_string())?;
        let values = [
            strings[0].0,
            strings[1].0,
            strings[2].0,
            strings[3].0,
            auth,
            strings[4].0,
        ];
        for (index, value) in values.into_iter().enumerate() {
            let slot = self
                .builder
                .build_struct_gep(options_type, options, index as u32, "http.docs.option")
                .map_err(|error| error.to_string())?;
            self.builder
                .build_store(slot, value)
                .map_err(|error| error.to_string())?;
        }
        let usize_type = self.usize_type();
        let i32_type = self.context.i32_type();
        let function = self.declare_libc_fn(
            "tyrus_http_router_docs",
            &[
                pointer.into(),
                pointer.into(),
                pointer.into(),
                usize_type.into(),
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
                    options.into(),
                    pre_handlers.into(),
                    usize_type.const_int(pre_handlers_len as u64, false).into(),
                ],
                "http_docs",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: docs did not return a status")?
            .into_int_value();
        let result = self.emit_http_status_result(status, "http.docs")?;
        for (value, expression) in strings {
            if let Some(expression) = expression {
                self.emit_native_string_argument_release(value, expression)?;
            }
        }
        Ok(result)
    }

    fn emit_http_response_hook_wrapper(
        &mut self,
        handler_name: &str,
    ) -> Result<FunctionValue<'ctx>, String> {
        let wrapper_name = format!("__tyrus_http_response_hook_{handler_name}");
        if let Some(wrapper) = self.module.get_function(&wrapper_name) {
            return Ok(wrapper);
        }
        let handler =
            self.fns.get(handler_name).copied().ok_or_else(|| {
                format!("error: HTTP response hook `{handler_name}` was not found")
            })?;
        let (params, ret) = self
            .fn_signatures
            .get(handler_name)
            .cloned()
            .ok_or_else(|| format!("error: HTTP response hook `{handler_name}` was not found"))?;
        if params != vec![Type::Named("HttpAccessLog".to_string())] || ret != Type::Void {
            return Err(format!(
                "error: HTTP response hook `{handler_name}` must accept `HttpAccessLog` and return `void`"
            ));
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let function_type = self.context.void_type().fn_type(&[pointer.into()], false);
        let wrapper = self.module.add_function(&wrapper_name, function_type, None);
        let previous_block = self.builder.get_insert_block();
        let result = (|| -> Result<(), String> {
            let entry = self.context.append_basic_block(wrapper, "entry");
            self.builder.position_at_end(entry);
            let access_pointer = wrapper.get_nth_param(0).unwrap().into_pointer_value();
            let access_type = self.llvm_type(&Type::Named("HttpAccessLog".to_string()));
            let access = self
                .builder
                .build_load(access_type, access_pointer, "access")
                .map_err(|error| error.to_string())?;
            self.builder
                .build_call(handler, &[access.into()], "response_hook")
                .map_err(|error| error.to_string())?;
            self.builder
                .build_return(None)
                .map_err(|error| error.to_string())?;
            Ok(())
        })();
        if let Some(block) = previous_block {
            self.builder.position_at_end(block);
        }
        result?;
        Ok(wrapper)
    }

    fn http_wrapper_request_type(&self, params: &[Type], contract: &HttpRouteContract) -> Type {
        match params.first() {
            Some(declared) if self.declared_fields(declared).is_some() => declared.clone(),
            _ => contract.request_type(),
        }
    }

    fn emit_http_callback_wrapper(
        &mut self,
        handler_name: &str,
        contract: &HttpRouteContract,
    ) -> Result<FunctionValue<'ctx>, String> {
        let handler = self
            .fns
            .get(handler_name)
            .copied()
            .ok_or_else(|| format!("error: HTTP handler `{handler_name}` was not found"))?;
        let (params, ret) = self
            .fn_signatures
            .get(handler_name)
            .cloned()
            .ok_or_else(|| format!("error: HTTP handler `{handler_name}` was not found"))?;
        self.emit_http_callback_wrapper_for_function(handler_name, handler, &params, &ret, contract)
    }

    fn emit_http_callback_wrapper_for_function(
        &mut self,
        handler_name: &str,
        handler: FunctionValue<'ctx>,
        params: &[Type],
        ret: &Type,
        contract: &HttpRouteContract,
    ) -> Result<FunctionValue<'ctx>, String> {
        let wrapper_name = format!("__tyrus_http_callback_{handler_name}");
        if let Some(wrapper) = self.module.get_function(&wrapper_name) {
            return Ok(wrapper);
        }
        let is_async = self.http_reply_handler_is_async(params, ret)?;
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let function_type = pointer.fn_type(&[pointer.into(), pointer.into()], false);
        let wrapper = self.module.add_function(&wrapper_name, function_type, None);
        let previous_block = self.builder.get_insert_block();
        let previous_async_results = std::mem::take(&mut self.async_results);
        let entry = self.context.append_basic_block(wrapper, "entry");
        self.builder.position_at_end(entry);
        let request_pointer = wrapper.get_nth_param(0).unwrap().into_pointer_value();
        let response_pointer = wrapper.get_nth_param(1).unwrap().into_pointer_value();
        let raw_request_type = self.llvm_type(&Type::Named("HttpRequest".to_string()));
        let raw_request = self
            .builder
            .build_load(raw_request_type, request_pointer, "request")
            .unwrap();
        let request_ty = self.http_wrapper_request_type(params, contract);
        let request_value = self.emit_http_declarative_request(
            &request_ty,
            raw_request.into_struct_value(),
            response_pointer,
        )?;
        let request_slot = self.entry_alloca(
            self.llvm_type(&request_ty).into_struct_type(),
            "http.request.owned",
        )?;
        self.builder
            .build_store(request_slot, request_value)
            .map_err(|error| error.to_string())?;
        let result = self
            .builder
            .build_call(
                handler,
                &[request_value.into(), response_pointer.into()],
                "response",
            )
            .unwrap();
        self.emit_async_drop_value_in_place(wrapper, request_slot, &request_ty)?;
        let promise = if is_async {
            result
                .try_as_basic_value()
                .basic()
                .ok_or_else(|| "internal: async HTTP handler did not return a Promise".to_string())?
                .into_pointer_value()
        } else {
            pointer.const_null()
        };
        self.builder.build_return(Some(&promise)).unwrap();
        self.async_results = previous_async_results;
        if let Some(block) = previous_block {
            self.builder.position_at_end(block);
        }
        Ok(wrapper)
    }

    fn emit_http_response_error_if_invalid(
        &mut self,
        valid: IntValue<'ctx>,
        response_pointer: PointerValue<'ctx>,
        status: i32,
        message: &str,
    ) {
        self.emit_http_failure_if_invalid(
            valid,
            response_pointer,
            status,
            message,
            "tyrus_http_set_response_error",
        );
    }

    fn emit_http_validation_response_if_invalid(
        &mut self,
        valid: IntValue<'ctx>,
        response_pointer: PointerValue<'ctx>,
        status: i32,
        message: &str,
    ) {
        self.emit_http_failure_if_invalid(
            valid,
            response_pointer,
            status,
            message,
            "tyrus_http_set_validation_error",
        );
    }

    fn emit_http_failure_if_invalid(
        &mut self,
        valid: IntValue<'ctx>,
        response_pointer: PointerValue<'ctx>,
        status: i32,
        message: &str,
        symbol: &str,
    ) {
        let current_function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .expect("HTTP validation response emitted outside function");
        let invalid_bb = self
            .context
            .append_basic_block(current_function, "http.validation.invalid");
        let ok_bb = self
            .context
            .append_basic_block(current_function, "http.validation.ok");
        self.builder
            .build_conditional_branch(valid, ok_bb, invalid_bb)
            .unwrap();
        self.builder.position_at_end(invalid_bb);
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let function = self.declare_libc_fn(
            symbol,
            &[
                pointer.into(),
                self.context.i32_type().into(),
                self.string_abi_type(),
            ],
            None,
        );
        let message = self
            .const_string_abi(message, "http.validation.message")
            .into_struct_value();
        self.builder
            .build_call(
                function,
                &[
                    response_pointer.into(),
                    self.context
                        .i32_type()
                        .const_int(status as u64, false)
                        .into(),
                    message.into(),
                ],
                "http.validation.error",
            )
            .unwrap();
        if let Some(result) = self.async_results.last().copied() {
            self.emit_async_void_ready(result);
        } else {
            match current_function.get_type().get_return_type() {
                None => {
                    self.builder.build_return(None).unwrap();
                }
                Some(BasicTypeEnum::PointerType(pointer)) => {
                    self.builder
                        .build_return(Some(&pointer.const_null()))
                        .unwrap();
                }
                Some(BasicTypeEnum::IntType(integer)) => {
                    self.builder
                        .build_return(Some(&integer.const_zero()))
                        .unwrap();
                }
                Some(return_type) => {
                    self.builder
                        .build_return(Some(&return_type.const_zero()))
                        .unwrap();
                }
            }
        }
        self.builder.position_at_end(ok_bb);
    }

    fn http_raw_request_source(
        &self,
        raw_request: StructValue<'ctx>,
        source: &str,
    ) -> Result<StructValue<'ctx>, String> {
        if !matches!(source, "body" | "params" | "query" | "headers" | "cookies") {
            return Err(format!("error: unsupported HTTP request source `{source}`"));
        }
        let index = http_request_abi_index(source)?;
        self.builder
            .build_extract_value(raw_request, index, &format!("http.request.{source}"))
            .map_err(|error| error.to_string())
            .map(|value| value.into_struct_value())
    }

    fn emit_http_declarative_request(
        &mut self,
        request_ty: &Type,
        raw_request: StructValue<'ctx>,
        response_pointer: PointerValue<'ctx>,
    ) -> Result<StructValue<'ctx>, String> {
        let fields = self
            .declared_fields_with_index(request_ty)
            .ok_or_else(|| "error: declarative HTTP request must be an object type".to_string())?;
        let mut result = self.llvm_type(request_ty).into_struct_type().get_undef();
        for (index, field) in &fields {
            let Type::Named(schema) = &field.ty else {
                let value = self.emit_http_shapeless_body(&field.ty, raw_request)?;
                result = self
                    .builder
                    .build_insert_value(result, value, *index, &field.name)
                    .map_err(|error| error.to_string())?
                    .into_struct_value();
                continue;
            };
            let source = self.http_raw_request_source(raw_request, &field.name)?;
            let value = self.emit_http_schema_request_from_json(
                schema,
                source,
                response_pointer,
                &field.name,
            )?;
            self.emit_validation_issues_begin();
            let validator = self.ensure_named_object_validation_predicate_function(schema)?;
            let valid = self
                .builder
                .build_call(validator, &[value.into()], "http.request.part.valid")
                .map_err(|error| error.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or_else(|| {
                    "internal: HTTP request validator did not return a value".to_string()
                })?
                .into_int_value();
            self.emit_http_validation_response_if_invalid(
                valid,
                response_pointer,
                400,
                &format!("Invalid request {}", field.name),
            );
            result = self
                .builder
                .build_insert_value(result, value, *index, &field.name)
                .map_err(|error| error.to_string())?
                .into_struct_value();
        }
        Ok(result)
    }

    fn emit_http_shapeless_body(
        &mut self,
        ty: &Type,
        raw_request: StructValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if matches!(ty, Type::Str) {
            return Ok(self.http_raw_request_source(raw_request, "body")?.into());
        }
        if matches!(ty, Type::Apply { name, args } if name == "ReadableStream" && args.as_slice() == [Type::Array(Box::new(Type::U8))])
        {
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let stream = self
                .builder
                .build_extract_value(
                    raw_request,
                    http_request_abi_index("bodyStream")?,
                    "http.request.body.stream.source",
                )
                .map_err(|error| error.to_string())?
                .into_pointer_value();
            let retain = self.declare_libc_fn(
                "tyrus_readable_stream_retain",
                &[pointer.into()],
                Some(pointer.into()),
            );
            return self
                .builder
                .build_call(retain, &[stream.into()], "http.request.body.stream.retain")
                .map_err(|error| error.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or_else(|| {
                    "internal: readable stream retain did not return a handle".to_string()
                });
        }
        let (data_index, length_index, element, copier, label) = match ty {
            Type::Array(element) if matches!(**element, Type::U8) => (
                http_request_abi_index("rawBody")?,
                http_request_abi_index("rawBodyLength")?,
                Type::U8,
                "tyrus_http_request_body_bytes",
                "http.request.body.bytes",
            ),
            Type::Array(element) if matches!(&**element, Type::Named(name) if name == "MultipartPart") => {
                (
                    http_request_abi_index("parts")?,
                    http_request_abi_index("partsLength")?,
                    Type::Named("MultipartPart".to_string()),
                    "tyrus_http_request_multipart_parts",
                    "http.request.body.parts",
                )
            }
            _ => {
                return Err(format!(
                    "internal: HTTP request body has no shapeless form for `{ty}`"
                ));
            }
        };
        let data = self
            .builder
            .build_extract_value(raw_request, data_index, &format!("{label}.source"))
            .map_err(|error| error.to_string())?;
        let length = self
            .builder
            .build_extract_value(raw_request, length_index, &format!("{label}.length"))
            .map_err(|error| error.to_string())?;
        let array_type = self
            .llvm_type(&Type::Array(Box::new(element)))
            .into_struct_type();
        let output = self.entry_alloca(array_type, label)?;
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let function = self.declare_libc_fn(
            copier,
            &[pointer.into(), self.usize_type().into(), pointer.into()],
            None,
        );
        self.builder
            .build_call(
                function,
                &[data.into(), length.into(), output.into()],
                &format!("{label}.copy"),
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_load(array_type, output, &format!("{label}.value"))
            .map_err(|error| error.to_string())
    }

    pub(super) fn http_schema_enum_repr(&self, name: &str) -> Option<Type> {
        let info = self.enums.get(name)?;
        if info.is_algebraic {
            return None;
        }
        matches!(
            info.repr,
            Type::Str
                | Type::Bool
                | Type::F32
                | Type::F64
                | Type::Decimal
                | Type::I32
                | Type::I64
                | Type::Isize
                | Type::U32
                | Type::U64
                | Type::Usize
        )
        .then(|| info.repr.clone())
    }

    pub(super) fn emit_abort_if_negative(
        &self,
        value: IntValue<'ctx>,
        operation: &str,
        block_name: &str,
    ) -> Result<(), String> {
        let current_function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: native operation outside function")?;
        let error_block = self
            .context
            .append_basic_block(current_function, &format!("{block_name}.error"));
        let continue_block = self
            .context
            .append_basic_block(current_function, &format!("{block_name}.ok"));
        let failed = self
            .builder
            .build_int_compare(
                IntPredicate::SLT,
                value,
                value.get_type().const_zero(),
                "native_failed",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(failed, error_block, continue_block)
            .unwrap();

        self.builder.position_at_end(error_block);
        let label = self
            .builder
            .build_global_string_ptr(operation, "native_operation")
            .unwrap();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let perror = self.declare_libc_fn("perror", &[ptr_type.into()], None);
        self.builder
            .build_call(perror, &[label.as_pointer_value().into()], "")
            .unwrap();
        let exit = self.declare_libc_fn("exit", &[self.context.i32_type().into()], None);
        self.builder
            .build_call(
                exit,
                &[self.context.i32_type().const_int(1, false).into()],
                "",
            )
            .unwrap();
        self.builder.build_unreachable().unwrap();

        self.builder.position_at_end(continue_block);
        Ok(())
    }

    pub(super) fn emit_exit_failure(&self) {
        let i32_type = self.context.i32_type();
        let exit = self.declare_libc_fn("exit", &[i32_type.into()], None);
        self.builder
            .build_call(exit, &[i32_type.const_int(1, false).into()], "")
            .unwrap();
        self.builder.build_unreachable().unwrap();
    }
}

fn http_json_scalar_getter(ty: &Type) -> Option<&'static str> {
    Some(match ty {
        Type::Str => "tyrus_json_document_get_string",
        Type::Bool => "tyrus_json_document_get_bool",
        Type::F32 => "tyrus_json_document_get_f32",
        Type::F64 => "tyrus_json_document_get_f64",
        Type::Decimal => "tyrus_json_document_get_decimal",
        Type::I32 => "tyrus_json_document_get_i32",
        Type::I64 | Type::Isize => "tyrus_json_document_get_i64",
        Type::U32 => "tyrus_json_document_get_u32",
        Type::U64 | Type::Usize => "tyrus_json_document_get_u64",
        _ => return None,
    })
}

fn http_invalid_channel_message(channel: &str) -> String {
    format!("Invalid request {channel}")
}

fn http_response_schema_name(schema: &Type, status: u16) -> Option<Type> {
    match transparent_codegen_type(schema) {
        Type::Object(fields) => fields
            .iter()
            .find(|field| field.name == status.to_string())
            .map(|field| field.ty.clone()),
        Type::Union(_) => {
            let _ = status;
            Some(schema.clone())
        }
        Type::Named(_)
        | Type::Array(_)
        | Type::Str
        | Type::Bool
        | Type::F32
        | Type::F64
        | Type::Decimal
        | Type::I32
        | Type::I64
        | Type::Isize
        | Type::U32
        | Type::U64
        | Type::Usize
        | Type::Void => Some(schema.clone()),
        _ => None,
    }
}

/// Where a field of the raw request the runtime lends sits in its struct.
///
/// The runtime's `HttpRequest` is `#[repr(C)]` in the order of
/// `http_request_abi_fields`, so the position is read from that list rather
/// than written as a number. Numbers written by hand drifted once already:
/// `id` was added as the first field and every channel, the byte body, the
/// multipart parts and the body stream were read one field too early.
fn http_request_abi_index(name: &str) -> Result<u32, String> {
    tyrus_core::builtin_interfaces::http_request_abi_fields()
        .iter()
        .position(|field| field.name == name)
        .map(|index| index as u32)
        .ok_or_else(|| format!("internal: the HTTP request ABI has no field `{name}`"))
}

#[cfg(test)]
mod http_request_abi_tests {
    use super::http_request_abi_index;

    #[test]
    fn every_raw_request_field_codegen_reads_has_its_runtime_position() {
        let expected = [
            ("id", 0),
            ("body", 3),
            ("params", 4),
            ("query", 5),
            ("headers", 6),
            ("cookies", 7),
            ("rawBody", 8),
            ("rawBodyLength", 9),
            ("parts", 10),
            ("partsLength", 11),
            ("bodyStream", 12),
        ];
        for (name, index) in expected {
            assert_eq!(http_request_abi_index(name), Ok(index), "field `{name}`");
        }
    }
}
