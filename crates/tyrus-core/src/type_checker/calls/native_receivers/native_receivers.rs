use super::*;

impl TypeChecker {
    #[allow(clippy::too_many_lines)]
    pub(super) fn check_native_receiver_method(
        &mut self,
        object: &Expr,
        method: &str,
        method_span: Option<Span>,
        type_args: &[Type],
        args: &[Expr],
        receiver: MethodReceiver<'_>,
    ) -> Option<ValueType> {
        let MethodReceiver {
            ty: object_ty,
            optional: optional_receiver,
        } = receiver;
        if matches!(object_ty, ValueType::Known(Type::Named(name)) if name == "HttpServer") {
            if !self.interfaces.contains_key("HttpServer") {
                self.error(
                    "E0412",
                    "cannot find type `HttpServer` in this scope".to_string(),
                    Some(method),
                );
            }
            let return_type = match method {
                "close" => Some(Type::Apply {
                    name: "Promise".to_string(),
                    args: vec![Type::Void],
                }),
                "port" => Some(Type::I32),
                _ => None,
            };
            let Some(return_type) = return_type else {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for type `HttpServer`"),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return Some(ValueType::Error);
            };
            if !args.is_empty() {
                self.error(
                    "E0061",
                    format!(
                        "`{method}` expects 0 argument(s), but {} were provided",
                        args.len()
                    ),
                    Some(method),
                );
            }
            for arg in args {
                self.check_expr(arg);
            }
            return Some(ValueType::Known(return_type));
        }
        if matches!(object_ty, ValueType::Known(Type::Named(name)) if name == "WebSocket") {
            let promise_of = |payload: Type| Type::Apply {
                name: "Promise".to_string(),
                args: vec![payload],
            };
            let http_result_of = |success: Type| Type::Result {
                success: Box::new(success),
                error: Box::new(Type::Named("HttpError".to_string())),
            };
            let (expected, result, rejects) = match method {
                "stream" => (
                    Vec::new(),
                    promise_of(http_result_of(Type::Apply {
                        name: "DuplexStream".to_string(),
                        args: vec![
                            Type::Array(Box::new(Type::U8)),
                            Type::Array(Box::new(Type::U8)),
                        ],
                    })),
                    false,
                ),
                "receive" => (
                    Vec::new(),
                    promise_of(http_result_of(Type::Optional(Box::new(Type::Named(
                        "WebSocketMessage".to_string(),
                    ))))),
                    false,
                ),
                "sendText" => (
                    vec![Type::Str],
                    promise_of(http_result_of(Type::Void)),
                    false,
                ),
                "sendBytes" => (
                    vec![Type::Array(Box::new(Type::U8))],
                    promise_of(http_result_of(Type::Void)),
                    false,
                ),
                "close" => (
                    vec![Type::I32, Type::Str],
                    promise_of(http_result_of(Type::Void)),
                    false,
                ),
                "protocol" => (Vec::new(), Type::Str, false),
                "closeCode" => (Vec::new(), Type::Optional(Box::new(Type::I32)), false),
                _ => {
                    self.error(
                        "E0599",
                        format!("no method named `{method}` for type `WebSocket`"),
                        Some(method),
                    );
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return Some(ValueType::Error);
                }
            };
            if method == "receive" && !self.interfaces.contains_key("WebSocketMessage") {
                self.error(
                    "E0412",
                    "cannot find type `WebSocketMessage` in this scope".to_string(),
                    Some(method),
                );
            }
            let minimum = if method == "close" { 0 } else { expected.len() };
            if args.len() < minimum || args.len() > expected.len() {
                self.error(
                    "E0061",
                    if minimum == expected.len() {
                        format!(
                            "`{method}` expects {} argument(s), but {} were provided",
                            expected.len(),
                            args.len()
                        )
                    } else {
                        format!(
                            "`{method}` expects {minimum} to {} argument(s), but {} were provided",
                            expected.len(),
                            args.len()
                        )
                    },
                    Some(method),
                );
            }
            for (index, arg) in args.iter().enumerate() {
                let actual = self.check_expr(arg);
                if let Some(expected) = expected.get(index) {
                    self.require_assignable(expected, &actual, method, "E0308");
                }
            }
            if rejects {
                self.check_unhandled_effect(
                    &Type::Named("HttpError".to_string()),
                    &format!("call to `{method}`"),
                    method_span.unwrap_or(object.span),
                    true,
                );
            }
            return Some(optional_value_type(
                ValueType::Known(result),
                optional_receiver,
            ));
        }
        if matches!(object_ty, ValueType::Known(Type::Named(name)) if name == "ChildProcess") {
            let return_type = match method {
                "wait" => Type::Apply {
                    name: "Promise".to_string(),
                    args: vec![Type::Result {
                        success: Box::new(Type::I32),
                        error: Box::new(Type::Named("ProcessError".to_string())),
                    }],
                },
                "kill" => Type::Apply {
                    name: "Promise".to_string(),
                    args: vec![Type::Result {
                        success: Box::new(Type::Void),
                        error: Box::new(Type::Named("ProcessError".to_string())),
                    }],
                },
                _ => {
                    self.error(
                        "E0599",
                        format!("no method named `{method}` for type `ChildProcess`"),
                        Some(method),
                    );
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return Some(ValueType::Error);
                }
            };
            if !args.is_empty() {
                self.error(
                    "E0061",
                    format!(
                        "`{method}` expects 0 argument(s), but {} were provided",
                        args.len()
                    ),
                    Some(method),
                );
            }
            for arg in args {
                self.check_expr(arg);
            }
            return Some(ValueType::Known(return_type));
        }
        if matches!(object_ty, ValueType::Known(Type::Named(name)) if name == "HttpRouter") {
            if method == "fallback" {
                if args.len() != 1 {
                    self.error(
                        "E0061",
                        format!(
                            "method `fallback` expects 1 argument, but {} were provided",
                            args.len()
                        ),
                        Some(method),
                    );
                }
                for arg in args {
                    self.check_expr(arg);
                }
                return Some(ValueType::Known(Type::Result {
                    success: Box::new(Type::Void),
                    error: Box::new(Type::Named("HttpError".to_string())),
                }));
            }
            let is_route = matches!(
                method,
                "get"
                    | "post"
                    | "put"
                    | "patch"
                    | "delete"
                    | "head"
                    | "options"
                    | "connect"
                    | "query"
                    | "trace"
            );
            if is_route {
                self.check_http_route_options(method, args);
                return Some(ValueType::Known(Type::Result {
                    success: Box::new(Type::Void),
                    error: Box::new(Type::Named("HttpError".to_string())),
                }));
            }
            if method == "ws" {
                self.check_http_websocket_route(args);
                return Some(ValueType::Known(Type::Result {
                    success: Box::new(Type::Void),
                    error: Box::new(Type::Named("HttpError".to_string())),
                }));
            }
            if method == "docs" {
                self.check_http_docs(args);
                return Some(ValueType::Known(Type::Result {
                    success: Box::new(Type::Void),
                    error: Box::new(Type::Named("HttpError".to_string())),
                }));
            }
            if method == "onResponse" {
                if !self.interfaces.contains_key("HttpAccessLog") {
                    self.error(
                        "E0412",
                        "cannot find type `HttpAccessLog` in this scope".to_string(),
                        Some(method),
                    );
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return Some(ValueType::Error);
                }
                if args.len() != 1 {
                    self.error(
                        "E0061",
                        format!(
                            "`onResponse` expects 1 argument(s), but {} were provided",
                            args.len()
                        ),
                        Some(method),
                    );
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return Some(ValueType::Known(Type::Void));
                }
                let actual = self.check_expr(&args[0]);
                if !matches!(args[0].kind(), ExprKind::Ident(_)) {
                    self.error(
                        "E0308",
                        "HTTP response hooks must be named functions".to_string(),
                        Some("onResponse"),
                    );
                }
                let expected = Type::Function {
                    params: vec![Type::Named("HttpAccessLog".to_string())],
                    ret: Box::new(Type::Void),
                };
                if !self.is_assignable(&expected, &actual) {
                    self.error(
                        "E0308",
                        "HTTP response hooks must accept `HttpAccessLog` and return `void`"
                            .to_string(),
                        Some("onResponse"),
                    );
                }
                return Some(ValueType::Known(Type::Result {
                    success: Box::new(Type::Void),
                    error: Box::new(Type::Named("HttpError".to_string())),
                }));
            }
            if method == "cors" && !self.interfaces.contains_key("CorsOptions") {
                self.error(
                    "E0412",
                    "cannot find type `CorsOptions` in this scope".to_string(),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return Some(ValueType::Error);
            }
            if method == "csrf" && !self.interfaces.contains_key("CsrfOptions") {
                self.error(
                    "E0412",
                    "cannot find type `CsrfOptions` in this scope".to_string(),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return Some(ValueType::Error);
            }
            if method == "rateLimit" {
                if args.len() != 1 {
                    self.error(
                        "E0061",
                        format!(
                            "`rateLimit` expects 1 argument(s), but {} were provided",
                            args.len()
                        ),
                        Some(method),
                    );
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return Some(ValueType::Known(Type::Void));
                }
                self.check_http_rate_limit(&args[0], "server");
                return Some(ValueType::Known(Type::Result {
                    success: Box::new(Type::Void),
                    error: Box::new(Type::Named("HttpError".to_string())),
                }));
            }
            if method == "listen" {
                self.check_http_listen(args);
                return Some(ValueType::Known(Type::Result {
                    success: Box::new(Type::Named("HttpServer".to_string())),
                    error: Box::new(Type::Named("HttpError".to_string())),
                }));
            }
            let expected = match method {
                "cors" => Some(vec![Type::Named("CorsOptions".to_string())]),
                "csrf" => Some(vec![Type::Named("CsrfOptions".to_string())]),
                "fetchPolicy" => Some(vec![Type::Named("FetchPolicyOptions".to_string())]),
                "trustedProxies" => Some(vec![
                    Type::Array(Box::new(Type::Str)),
                    Type::Optional(Box::new(Type::Str)),
                ]),
                _ => None,
            };
            if method == "trustedProxies" {
                if let Some(arg) = args.first() {
                    self.check_http_trusted_proxy_literals(arg);
                }
                if let Some(arg) = args.get(1) {
                    self.check_http_trusted_proxy_identity_header_literal(arg);
                }
            }
            let Some(expected) = expected else {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for type `HttpRouter`"),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return Some(ValueType::Error);
            };
            let minimum = expected
                .iter()
                .rposition(|parameter| !matches!(parameter, Type::Optional(_)))
                .map_or(0, |index| index + 1);
            if args.len() < minimum || args.len() > expected.len() {
                self.error(
                    "E0061",
                    format!(
                        "method `{method}` expects {} to {} argument(s), but {} were provided",
                        minimum,
                        expected.len(),
                        args.len()
                    ),
                    Some(method),
                );
            }
            let expected_len = expected.len();
            for (index, arg) in args.iter().enumerate() {
                let actual = match expected.get(index) {
                    Some(parameter)
                        if matches!(method, "cors" | "csrf" | "fetchPolicy" | "trustedProxies") =>
                    {
                        self.check_expr_as(arg, parameter)
                    }
                    _ => self.check_expr(arg),
                };
                if let Some(expected) = expected.get(index) {
                    if index == expected_len - 1
                        && matches!(
                            method,
                            "get"
                                | "post"
                                | "put"
                                | "patch"
                                | "delete"
                                | "head"
                                | "options"
                                | "connect"
                                | "query"
                                | "trace"
                                | "fallback"
                        )
                        && self.is_legacy_http_schema_handler(&actual)
                    {
                        continue;
                    }
                    self.require_assignable(expected, &actual, method, "E0308");
                }
            }
            return Some(ValueType::Known(
                if matches!(method, "cors" | "csrf" | "fetchPolicy" | "trustedProxies") {
                    Type::Result {
                        success: Box::new(Type::Void),
                        error: Box::new(Type::Named("HttpError".to_string())),
                    }
                } else {
                    Type::Void
                },
            ));
        }
        if matches!(object_ty, ValueType::Known(Type::Named(name)) if name == "FSWatcher") {
            let expected: Option<Vec<Type>> = match method {
                "close" => Some(vec![]),
                _ => None,
            };
            let Some(expected) = expected else {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for type `FSWatcher`"),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return Some(ValueType::Error);
            };
            let minimum = expected
                .iter()
                .rposition(|parameter| !matches!(parameter, Type::Optional(_)))
                .map_or(0, |index| index + 1);
            if args.len() < minimum || args.len() > expected.len() {
                self.error(
                    "E0061",
                    format!(
                        "method `{method}` expects {} to {} argument(s), but {} were provided",
                        minimum,
                        expected.len(),
                        args.len()
                    ),
                    Some(method),
                );
            }
            for arg in args {
                self.check_expr(arg);
            }
            return Some(optional_value_type(
                ValueType::Known(Type::Void),
                optional_receiver,
            ));
        }
        if matches!(object_ty, ValueType::Known(Type::Named(name)) if name == "HttpClientResponse")
        {
            if method == "json" {
                return Some(self.check_http_client_response_json(type_args, method, args));
            }
            let expected: Option<Vec<Type>> = match method {
                "status" | "ok" | "text" | "bytes" | "close" => Some(Vec::new()),
                "header" => Some(vec![Type::Str]),
                _ => None,
            };
            let Some(expected) = expected else {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for type `HttpClientResponse`"),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return Some(ValueType::Error);
            };
            if args.len() != expected.len() {
                self.error(
                    "E0061",
                    format!(
                        "method `{method}` expects {} argument(s), but {} were provided",
                        expected.len(),
                        args.len()
                    ),
                    Some(method),
                );
            }
            for (index, arg) in args.iter().enumerate() {
                let actual = self.check_expr(arg);
                if let Some(expected) = expected.get(index) {
                    self.require_assignable(expected, &actual, method, "E0308");
                }
            }
            let result = match method {
                "status" => Type::I32,
                "ok" => Type::Bool,
                "text" => Type::Str,
                "bytes" => Type::Array(Box::new(Type::U8)),
                "header" => Type::Optional(Box::new(Type::Str)),
                _ => Type::Void,
            };
            return Some(optional_value_type(
                ValueType::Known(result),
                optional_receiver,
            ));
        }
        if matches!(object_ty, ValueType::Known(Type::Named(name)) if name == "HttpClientStreamResponse")
        {
            let expected: Option<Vec<Type>> = match method {
                "status" | "ok" | "stream" | "close" => Some(Vec::new()),
                "header" => Some(vec![Type::Str]),
                _ => None,
            };
            let Some(expected) = expected else {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for type `HttpClientStreamResponse`"),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return Some(ValueType::Error);
            };
            if args.len() != expected.len() {
                self.error(
                    "E0061",
                    format!(
                        "method `{method}` expects {} argument(s), but {} were provided",
                        expected.len(),
                        args.len()
                    ),
                    Some(method),
                );
            }
            for (index, arg) in args.iter().enumerate() {
                let actual = self.check_expr(arg);
                if let Some(expected) = expected.get(index) {
                    self.require_assignable(expected, &actual, method, "E0308");
                }
            }
            let result = match method {
                "status" => Type::I32,
                "ok" => Type::Bool,
                "header" => Type::Optional(Box::new(Type::Str)),
                "stream" => Type::Apply {
                    name: "Promise".to_string(),
                    args: vec![Type::Apply {
                        name: "ReadableStream".to_string(),
                        args: vec![Type::Array(Box::new(Type::U8))],
                    }],
                },
                _ => Type::Void,
            };
            return Some(optional_value_type(
                ValueType::Known(result),
                optional_receiver,
            ));
        }
        if matches!(object_ty, ValueType::Known(Type::Named(name)) if name == "TcpStream") {
            let net_promise = |success| Type::Apply {
                name: "Promise".to_string(),
                args: vec![Type::Result {
                    success: Box::new(success),
                    error: Box::new(Type::Named("NetError".to_string())),
                }],
            };
            let expected = match method {
                "read" => Some(vec![Type::U32, Type::Optional(Box::new(Type::U64))]),
                "write" => Some(vec![
                    Type::Array(Box::new(Type::U8)),
                    Type::Optional(Box::new(Type::U64)),
                ]),
                "shutdown" => Some(Vec::new()),
                "localAddress" => Some(Vec::new()),
                "peerAddress" => Some(Vec::new()),
                "close" => Some(Vec::new()),
                "stream" => Some(Vec::new()),
                _ => None,
            };
            let Some(expected) = expected else {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for type `TcpStream`"),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return Some(ValueType::Error);
            };
            let minimum = expected
                .iter()
                .rposition(|parameter| !matches!(parameter, Type::Optional(_)))
                .map_or(0, |index| index + 1);
            if args.len() < minimum || args.len() > expected.len() {
                self.error(
                    "E0061",
                    format!(
                        "method `{method}` expects {} to {} argument(s), but {} were provided",
                        minimum,
                        expected.len(),
                        args.len()
                    ),
                    Some(method),
                );
            }
            for (index, arg) in args.iter().enumerate() {
                let actual = self.check_expr(arg);
                if let Some(expected) = expected.get(index) {
                    self.require_assignable(expected, &actual, method, "E0308");
                }
            }
            let result = match method {
                "read" => net_promise(Type::Array(Box::new(Type::U8))),
                "write" => net_promise(Type::U64),
                "shutdown" => net_promise(Type::Void),
                "localAddress" | "peerAddress" => Type::Str,
                "stream" => net_promise(Type::Apply {
                    name: "DuplexStream".to_string(),
                    args: vec![
                        Type::Array(Box::new(Type::U8)),
                        Type::Array(Box::new(Type::U8)),
                    ],
                }),
                _ => Type::Void,
            };
            return Some(optional_value_type(
                ValueType::Known(result),
                optional_receiver,
            ));
        }
        if matches!(object_ty, ValueType::Known(Type::Named(name)) if name == "TcpListener") {
            let net_promise = |success| Type::Apply {
                name: "Promise".to_string(),
                args: vec![Type::Result {
                    success: Box::new(success),
                    error: Box::new(Type::Named("NetError".to_string())),
                }],
            };
            let expected = match method {
                "accept" => Some(vec![Type::Optional(Box::new(Type::U64))]),
                "localAddress" => Some(Vec::<Type>::new()),
                "close" => Some(Vec::<Type>::new()),
                _ => None,
            };
            let Some(expected) = expected else {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for type `TcpListener`"),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return Some(ValueType::Error);
            };
            let minimum = expected
                .iter()
                .rposition(|parameter| !matches!(parameter, Type::Optional(_)))
                .map_or(0, |index| index + 1);
            if args.len() < minimum || args.len() > expected.len() {
                self.error(
                    "E0061",
                    format!(
                        "method `{method}` expects {} to {} argument(s), but {} were provided",
                        minimum,
                        expected.len(),
                        args.len()
                    ),
                    Some(method),
                );
            }
            for arg in args {
                self.check_expr(arg);
            }
            let result = match method {
                "accept" => net_promise(Type::Named("TcpStream".to_string())),
                "localAddress" => Type::Str,
                _ => Type::Void,
            };
            return Some(optional_value_type(
                ValueType::Known(result),
                optional_receiver,
            ));
        }
        if matches!(object_ty, ValueType::Known(Type::Named(name)) if name == "UdpSocket") {
            let net_promise = |success| Type::Apply {
                name: "Promise".to_string(),
                args: vec![Type::Result {
                    success: Box::new(success),
                    error: Box::new(Type::Named("NetError".to_string())),
                }],
            };
            let expected = match method {
                "sendTo" => Some(vec![
                    Type::Array(Box::new(Type::U8)),
                    Type::Str,
                    Type::Optional(Box::new(Type::U64)),
                ]),
                "receiveFrom" => Some(vec![Type::U32, Type::Optional(Box::new(Type::U64))]),
                "setBroadcast" => Some(vec![Type::Bool]),
                "setTtl" => Some(vec![Type::U32]),
                "localAddress" => Some(Vec::new()),
                "close" => Some(Vec::new()),
                _ => None,
            };
            let Some(expected) = expected else {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for type `UdpSocket`"),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return Some(ValueType::Error);
            };
            let minimum = expected
                .iter()
                .rposition(|parameter| !matches!(parameter, Type::Optional(_)))
                .map_or(0, |index| index + 1);
            if args.len() < minimum || args.len() > expected.len() {
                self.error(
                    "E0061",
                    format!(
                        "method `{method}` expects {} to {} argument(s), but {} were provided",
                        minimum,
                        expected.len(),
                        args.len()
                    ),
                    Some(method),
                );
            }
            for (index, arg) in args.iter().enumerate() {
                let actual = self.check_expr(arg);
                if let Some(expected) = expected.get(index) {
                    self.require_assignable(expected, &actual, method, "E0308");
                }
            }
            let result = match method {
                "sendTo" => net_promise(Type::U64),
                "receiveFrom" => net_promise(Type::Named("UdpPacket".to_string())),
                "setBroadcast" | "setTtl" => net_promise(Type::Void),
                "localAddress" => Type::Str,
                _ => Type::Void,
            };
            return Some(optional_value_type(
                ValueType::Known(result),
                optional_receiver,
            ));
        }
        if let Some(reply_result) =
            self.check_http_reply_method(object_ty, method, args, optional_receiver)
        {
            return Some(reply_result);
        }
        if matches!(&object_ty, ValueType::Known(Type::Named(name)) if name == "DigestNamespace")
            && method == "md5"
        {
            self.warning_at(
                "E3278",
                "`digest.md5` is cryptographically broken and must be restricted to legacy compatibility".to_string(),
                method_span.unwrap_or(object.span),
            );
        }
        None
    }
}
