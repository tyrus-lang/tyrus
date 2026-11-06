use super::*;

impl TypeChecker {
    pub(super) fn is_legacy_http_schema_handler(&self, actual: &ValueType) -> bool {
        let ValueType::Known(Type::Function { params, ret }) = actual else {
            return false;
        };
        let [Type::Ref(request)] = params.as_slice() else {
            return false;
        };
        let Type::Named(request_name) = transparent_type(request.as_ref()) else {
            return false;
        };
        let Type::Named(response_name) = transparent_type(ret.as_ref()) else {
            return false;
        };
        (request_name == "HttpRequest" || self.is_supported_http_schema(request_name))
            && (response_name == "HttpResponse"
                || self.is_supported_http_response_schema(response_name))
    }

    pub(super) fn is_supported_http_schema(&self, name: &str) -> bool {
        self.is_supported_http_interface(name, HttpSchemaUse::Read, &mut HashSet::default())
    }

    pub(super) fn is_supported_http_response_schema(&self, name: &str) -> bool {
        self.is_supported_http_interface(name, HttpSchemaUse::Write, &mut HashSet::default())
    }

    pub(super) fn is_supported_http_interface(
        &self,
        name: &str,
        usage: HttpSchemaUse,
        visiting: &mut HashSet<String>,
    ) -> bool {
        self.http_schema_rejection(name, usage, "", visiting)
            .is_none()
    }

    pub(super) fn http_schema_rejection(
        &self,
        name: &str,
        usage: HttpSchemaUse,
        path: &str,
        visiting: &mut HashSet<String>,
    ) -> Option<(String, String)> {
        if !visiting.insert(name.to_string()) {
            return Some((
                path.to_string(),
                format!("`{name}` contains itself, so a value of it has no finite shape"),
            ));
        }
        let Some(fields) = self.interfaces.get(name) else {
            visiting.remove(name);
            return Some((
                path.to_string(),
                format!(
                    "`{name}` is not an interface or enum in this scope; a schema's field types must be imported where the route is declared"
                ),
            ));
        };
        let fields = fields.clone();
        let rejection = fields.iter().find_map(|field| {
            let where_ = if path.is_empty() {
                field.name.clone()
            } else {
                format!("{path}.{}", field.name)
            };
            if field.optional || matches!(transparent_type(&field.ty), Type::Optional(_)) {
                let payload = optional_payload_type(&field.ty);
                return (!usage.allows_optional(payload)).then(|| {
                    (
                        where_,
                        "an optional field of a request schema must hold a single scalar"
                            .to_string(),
                    )
                });
            }
            self.http_schema_field_rejection(&field.ty, usage, &where_, visiting)
        });
        visiting.remove(name);
        rejection
    }

    pub(super) fn http_schema_field_rejection(
        &self,
        ty: &Type,
        usage: HttpSchemaUse,
        path: &str,
        visiting: &mut HashSet<String>,
    ) -> Option<(String, String)> {
        match transparent_type(ty) {
            Type::Array(element) => {
                self.http_schema_field_rejection(element, usage, path, visiting)
            }
            Type::Named(name) => {
                if self.http_schema_enum_repr(name).is_some() {
                    return None;
                }
                if let Some(info) = self.enums.get(name) {
                    let reason = if info.is_algebraic {
                        format!(
                            "enum `{name}` has members carrying payloads, which have no single value to be on the wire"
                        )
                    } else {
                        format!(
                            "enum `{name}` is represented as `{}`, which JSON has no form for",
                            info.repr
                        )
                    };
                    return Some((path.to_string(), reason));
                }
                self.http_schema_rejection(name, usage, path, visiting)
            }
            Type::Optional(inner) => (!usage.allows_optional(inner)).then(|| {
                (
                    path.to_string(),
                    "an optional field of a request schema must hold a single scalar".to_string(),
                )
            }),
            Type::Union(variants) => {
                if !crate::automatic_data_methods::is_discriminated_union(variants) {
                    return Some((
                        path.to_string(),
                        "a union in a schema must be a discriminated union: every variant an object, with a shared field holding a distinct literal in each".to_string(),
                    ));
                }
                variants.iter().find_map(|variant| {
                    let Type::Object(fields) = transparent_type(variant) else {
                        return Some((
                            path.to_string(),
                            "every variant of a union in a schema must be an object".to_string(),
                        ));
                    };
                    fields.iter().find_map(|field| {
                        if matches!(transparent_type(&field.ty), Type::Lit(_)) {
                            return None;
                        }
                        let where_ = format!("{path}.{}", field.name);
                        self.http_schema_field_rejection(&field.ty, usage, &where_, visiting)
                    })
                })
            }
            other => (!is_supported_http_scalar_type(ty)).then(|| {
                (
                    path.to_string(),
                    format!("`{other}` is not a type an HTTP schema can carry"),
                )
            }),
        }
    }

    pub(crate) fn http_schema_enum_repr(&self, name: &str) -> Option<&Type> {
        let info = self.enums.get(name)?;
        if info.is_algebraic {
            return None;
        }
        is_supported_http_scalar_type(&info.repr).then_some(&info.repr)
    }

    pub(super) fn check_http_websocket_config(&mut self, value: &Expr) {
        let ExprKind::ObjectLit(fields) = value.kind() else {
            self.check_expr(value);
            self.error(
                "E0308",
                "WebSocket route `config` must be an object literal".to_string(),
                Some("config"),
            );
            return;
        };
        for (field, value) in fields {
            match field.as_str() {
                "rateLimit" => self.check_http_rate_limit(value, "WebSocket route config"),
                _ => {
                    self.check_expr(value);
                    self.error(
                        "E0609",
                        format!("unknown WebSocket route config option `{field}`"),
                        Some(field),
                    );
                }
            }
        }
    }

    pub(super) fn check_http_docs(&mut self, args: &[Expr]) {
        if args.len() > 1 {
            self.error(
                "E0061",
                format!(
                    "method `docs` expects 0 to 1 argument(s), but {} were provided",
                    args.len()
                ),
                Some("docs"),
            );
        }
        let Some(options) = args.first() else {
            return;
        };
        let ExprKind::ObjectLit(fields) = options.kind() else {
            self.check_expr(options);
            self.error(
                "E0308",
                "`docs` expects an options object".to_string(),
                Some("docs"),
            );
            return;
        };
        for (field, value) in fields {
            match field.as_str() {
                "path" | "title" | "version" | "description" | "css" => {
                    let actual = self.check_expr_as(value, &Type::Str);
                    self.require_assignable(&Type::Str, &actual, field, "E0308");
                }
                "auth" => self.check_http_docs_auth(value),
                "preHandler" => self.check_http_route_pre_handlers(value),
                _ => {
                    self.check_expr(value);
                    self.error(
                        "E0609",
                        format!("unknown `docs` option `{field}`"),
                        Some(field),
                    );
                }
            }
        }
    }

    pub(super) fn check_http_docs_auth(&mut self, value: &Expr) {
        let ExprKind::ObjectLit(fields) = value.kind() else {
            self.check_expr(value);
            self.error(
                "E0308",
                "`docs` option `auth` expects an object literal naming its schemes: `bearer`, `apiKey` or `oauth2`".to_string(),
                Some("auth"),
            );
            return;
        };
        let literal_string = |expr: &Expr| matches!(expr.kind(), ExprKind::StrLit(_));
        let mut bearer = false;
        let mut bearer_format = false;
        let mut schemes = 0;
        for (key, entry) in fields {
            match key.as_str() {
                "bearer" => match entry.kind() {
                    ExprKind::BoolLit(enabled) => {
                        bearer = *enabled;
                        if *enabled {
                            schemes += 1;
                        }
                    }
                    _ => {
                        self.check_expr(entry);
                        self.error(
                            "E0308",
                            "`auth.bearer` must be written as `true` or `false`: the document is built from it during compilation".to_string(),
                            Some(key),
                        );
                    }
                },
                "bearerFormat" => {
                    bearer_format = true;
                    if !literal_string(entry) {
                        self.check_expr(entry);
                        self.error(
                            "E0308",
                            "`auth.bearerFormat` must be written as a string literal, such as `\"JWT\"`".to_string(),
                            Some(key),
                        );
                    }
                }
                "apiKey" => {
                    schemes += 1;
                    self.check_http_docs_auth_object(
                        key,
                        entry,
                        &[("name", true), ("in", true)],
                        |checker, name, value| {
                            if name == "in"
                                && let ExprKind::StrLit(location) = value.kind()
                                    && !matches!(location.as_str(), "header" | "query" | "cookie") {
                                        checker.error(
                                            "E0308",
                                            format!("`auth.apiKey.in` is `{location}`; an API key travels in `\"header\"`, `\"query\"` or `\"cookie\"`"),
                                            Some(name),
                                        );
                                    }
                        },
                    );
                }
                "oauth2" => {
                    schemes += 1;
                    self.check_http_docs_auth_object(
                        key,
                        entry,
                        &[
                            ("authorizationUrl", false),
                            ("tokenUrl", true),
                            ("scopes", false),
                        ],
                        |_, _, _| {},
                    );
                }
                _ => {
                    self.check_expr(entry);
                    self.error(
                        "E0609",
                        format!("unknown `auth` option `{key}`; `auth` names `bearer`, `bearerFormat`, `apiKey` and `oauth2`"),
                        Some(key),
                    );
                }
            }
        }
        if bearer_format && !bearer {
            self.error(
                "E0308",
                "`auth.bearerFormat` describes the bearer scheme, which `auth` does not enable; add `bearer: true`".to_string(),
                Some("bearerFormat"),
            );
        }
        if schemes == 0 {
            self.error(
                "E0308",
                "`auth` names no scheme; add `bearer: true`, `apiKey: { name, in }` or `oauth2: { tokenUrl }`".to_string(),
                Some("auth"),
            );
        }
    }

    pub(super) fn check_http_docs_auth_object(
        &mut self,
        scheme: &str,
        value: &Expr,
        allowed: &[(&str, bool)],
        extra: impl Fn(&mut Self, &str, &Expr),
    ) {
        let ExprKind::ObjectLit(fields) = value.kind() else {
            self.check_expr(value);
            self.error(
                "E0308",
                format!("`auth.{scheme}` expects an object literal"),
                Some(scheme),
            );
            return;
        };
        for (key, entry) in fields {
            let Some(_) = allowed.iter().find(|(name, _)| name == key) else {
                self.check_expr(entry);
                self.error(
                    "E0609",
                    format!("unknown `auth.{scheme}` option `{key}`"),
                    Some(key),
                );
                continue;
            };
            let literal = if key == "scopes" {
                matches!(entry.kind(), ExprKind::ArrayLit(items) if items.iter().all(|item| matches!(item.kind(), ExprKind::StrLit(_))))
            } else {
                matches!(entry.kind(), ExprKind::StrLit(_))
            };
            if !literal {
                self.check_expr(entry);
                self.error(
                    "E0308",
                    format!(
                        "`auth.{scheme}.{key}` must be written as {}: the document is built from it during compilation",
                        if key == "scopes" { "an array of string literals" } else { "a string literal" }
                    ),
                    Some(key),
                );
                continue;
            }
            extra(self, key, entry);
        }
        for (name, required) in allowed {
            if *required && !fields.iter().any(|(key, _)| key == name) {
                self.error(
                    "E2741",
                    format!("`auth.{scheme}` is missing required field `{name}`"),
                    Some(scheme),
                );
            }
        }
    }

    pub(super) fn check_http_websocket_route(&mut self, args: &[Expr]) {
        if !self.interfaces.contains_key("WebSocket") {
            self.error(
                "E0412",
                "cannot find type `WebSocket` in this scope".to_string(),
                Some("ws"),
            );
            for arg in args {
                self.check_expr(arg);
            }
            return;
        }
        if args.len() != 2 {
            self.error(
                "E0061",
                format!(
                    "method `ws` expects 2 argument(s), but {} were provided",
                    args.len()
                ),
                Some("ws"),
            );
        }
        if let Some(path) = args.first() {
            let actual = self.check_expr(path);
            self.require_assignable(&Type::Str, &actual, "ws", "E0308");
        }
        let Some(options) = args.get(1) else {
            return;
        };
        let ExprKind::ObjectLit(fields) = options.kind() else {
            self.check_expr(options);
            self.error(
                "E0308",
                "WebSocket routes expect an options object with a `handler` field".to_string(),
                Some("ws"),
            );
            return;
        };
        let mut pre_handlers = None;
        let mut has_handler = false;
        for (field, value) in fields {
            match field.as_str() {
                "handler" => {
                    has_handler = true;
                    let actual = self.check_expr(value);
                    let expected = Type::Function {
                        params: vec![Type::Named("WebSocket".to_string())],
                        ret: Box::new(Type::Apply {
                            name: "Promise".to_string(),
                            args: vec![Type::Void],
                        }),
                    };
                    if !matches!(actual, ValueType::Error)
                        && !self.is_assignable(&expected, &actual)
                    {
                        self.error(
                            "E0308",
                            "a WebSocket route handler must be `(socket: WebSocket) => Promise<void>`"
                                .to_string(),
                            Some("handler"),
                        );
                    }
                }
                "preHandler" => {
                    pre_handlers = Some(value);
                }
                "config" => {
                    self.check_http_websocket_config(value);
                }
                "origins" | "subprotocols" => {
                    let expected = Type::Array(Box::new(Type::Str));
                    let actual = self.check_expr_as(value, &expected);
                    self.require_assignable(&expected, &actual, field, "E0308");
                }
                "maxMessageBytes" => {
                    let actual = self.check_expr_as(value, &Type::I64);
                    self.require_assignable(&Type::I64, &actual, field, "E0308");
                }
                _ => {
                    self.check_expr(value);
                    self.error(
                        "E0609",
                        format!("unknown WebSocket route option `{field}`"),
                        Some(field),
                    );
                }
            }
        }
        if let Some(pre_handlers) = pre_handlers {
            self.check_http_route_pre_handlers(pre_handlers);
        }
        if !has_handler {
            self.error(
                "E2741",
                "WebSocket route options object is missing required field `handler`".to_string(),
                Some("ws"),
            );
        }
    }

    pub(super) fn check_http_route_options(&mut self, method: &str, args: &[Expr]) {
        if args.len() != 2 {
            self.error(
                "E0061",
                format!(
                    "method `{method}` expects 2 argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        let Some(options) = args.get(1) else {
            return;
        };
        self.check_http_route_options_inner(method, args.first(), options);
    }

    fn check_http_route_options_inner(
        &mut self,
        method: &str,
        path: Option<&Expr>,
        options: &Expr,
    ) {
        if let Some(path) = path {
            let actual = self.check_expr(path);
            self.require_assignable(&Type::Str, &actual, method, "E0308");
        }
        let path_params = path
            .and_then(|path| match path {
                Expr {
                    kind: ExprKind::StrLit(path),
                    ..
                } => Some(http_path_params(path)),
                _ => None,
            })
            .unwrap_or_default();
        let Expr {
            kind: ExprKind::ObjectLit(fields),
            ..
        } = options
        else {
            self.check_expr(options);
            self.error(
                "E0308",
                "HTTP routes expect a route options object with a `handler` field".to_string(),
                Some(method),
            );
            return;
        };
        let mut handler = None;
        let mut pre_handlers = None;
        let mut contract = HttpRouteContract::default();
        for (field, value) in fields {
            match field.as_str() {
                "handler" => {
                    handler = Some(value);
                }
                "preHandler" => {
                    pre_handlers = Some(value);
                }
                "body" | "params" | "query" | "headers" | "cookies" => {
                    if let Some(schema) = self.check_http_schema_reference(field, value) {
                        if field == "params"
                            && let Type::Named(name) = &schema
                        {
                            contract.params_schema = Some(name.clone());
                        }
                        let schema = match &schema {
                            Type::Named(name) => {
                                crate::http_body::request_channel_type(field, name)
                            }
                            other => other.clone(),
                        };
                        contract.request_fields.push(Field {
                            name: field.clone(),
                            ty: schema,
                            readonly: true,
                            optional: false,
                            decorators: Vec::new(),
                            span: Span::DUMMY,
                        });
                    }
                }
                "tags" => {
                    self.check_expr_as(value, &Type::Array(Box::new(Type::Str)));
                }
                "response" => {
                    contract.responses = self.check_http_response_schema_reference(value);
                }
                "config" => {
                    self.check_http_route_config(value);
                }
                _ => {
                    self.check_expr(value);
                    self.error(
                        "E0609",
                        format!("unknown HTTP route option `{field}`"),
                        Some(field),
                    );
                }
            }
        }
        self.check_http_path_params(method, &path_params, contract.params_schema.as_deref());
        if let Some(pre_handlers) = pre_handlers {
            self.check_http_route_pre_handlers(pre_handlers);
        }
        let Some(handler) = handler else {
            self.error(
                "E2741",
                "HTTP route options object is missing required field `handler`".to_string(),
                Some(method),
            );
            return;
        };
        if contract.responses.is_empty() {
            self.error(
                "E2741",
                "HTTP route options object is missing required field `response`".to_string(),
                Some(method),
            );
        }
        self.check_http_route_handler(handler, &contract);
    }

    pub(super) fn check_http_route_pre_handlers(&mut self, value: &Expr) {
        let ExprKind::ArrayLit(pre_handlers) = value.kind() else {
            self.check_expr(value);
            self.error(
                "E0308",
                "HTTP route `preHandler` must be an array literal of named functions".to_string(),
                Some("preHandler"),
            );
            return;
        };
        let expected = Type::Function {
            params: vec![Type::Named("HttpRequest".to_string())],
            ret: Box::new(Type::Union(vec![
                Type::Named("HttpResponse".to_string()),
                Type::Lit(Box::new(Expr::synthetic(ExprKind::BoolLit(true)))),
            ])),
        };
        for pre_handler in pre_handlers {
            let actual = self.check_expr(pre_handler);
            let ExprKind::Ident(name) = pre_handler.kind() else {
                self.error(
                    "E0308",
                    "HTTP route `preHandler` entries must be named functions".to_string(),
                    Some("preHandler"),
                );
                continue;
            };
            if !self.is_assignable(&expected, &actual) {
                self.error(
                    "E0308",
                    format!(
                        "HTTP route preHandler `{name}` must accept `HttpRequest` and return `HttpResponse | true`"
                    ),
                    Some(name),
                );
            }
        }
    }

    pub(super) fn check_http_route_config(&mut self, value: &Expr) {
        let Expr {
            kind: ExprKind::ObjectLit(fields),
            ..
        } = value
        else {
            self.check_expr(value);
            self.error(
                "E0308",
                "HTTP route `config` must be an object literal".to_string(),
                Some("config"),
            );
            return;
        };
        for (field, value) in fields {
            match field.as_str() {
                "bodyLimit" => {
                    self.check_http_config_body_limit(value);
                }
                "rateLimit" => {
                    self.check_http_rate_limit(value, "HTTP route config");
                }
                _ => {
                    self.check_expr(value);
                    self.error(
                        "E0609",
                        format!("unknown HTTP route config option `{field}`"),
                        Some(field),
                    );
                }
            }
        }
    }

    pub(super) fn check_http_rate_limit(&mut self, value: &Expr, subject: &str) {
        let Expr {
            kind: ExprKind::ObjectLit(fields),
            ..
        } = value
        else {
            self.check_expr(value);
            self.error_at_with_help(
                "E0308",
                format!("{subject} `rateLimit` must be written out here, not passed as a value"),
                value.span,
                vec![
                    "write the object at the call, as in `{ max: 100, timeWindow: \"1 minute\" }`: `timeWindow` is read during compilation, so it has to be visible here".to_string(),
                ],
            );
            return;
        };
        let mut seen_max = false;
        let mut seen_time_window = false;
        let mut seen_ban = false;
        let mut ban_window: Option<&Expr> = None;
        for (field, value) in fields {
            match field.as_str() {
                "max" => {
                    seen_max = true;
                    let actual = self.check_expr(value);
                    self.require_http_config_integer("max", &actual);
                    if const_int_value(value).is_some_and(|number| number <= 0) {
                        self.error_at_with_help(
                            "E0707",
                            format!("{subject} `rateLimit.max` must be greater than zero"),
                            value.span,
                            vec!["a limit of zero would refuse every request; leave `rateLimit` out to impose no limit".to_string()],
                        );
                    }
                }
                "globalMax" => {
                    let actual = self.check_expr(value);
                    self.require_http_config_integer("globalMax", &actual);
                    if const_int_value(value).is_some_and(|number| number <= 0) {
                        self.error_at_with_help(
                            "E0707",
                            format!("{subject} `rateLimit.globalMax` must be greater than zero"),
                            value.span,
                            vec!["leave `globalMax` out to limit each client identity without a shared server-wide cap".to_string()],
                        );
                    }
                }
                "timeWindow" => {
                    seen_time_window = true;
                    self.check_http_config_time_window_in(value, subject);
                }
                "ban" => {
                    seen_ban = true;
                    let actual = self.check_expr(value);
                    self.require_http_config_integer("ban", &actual);
                    if const_int_value(value).is_some_and(|number| number < 0) {
                        self.error_at_with_help(
                            "E0707",
                            format!("{subject} `rateLimit.ban` cannot be negative"),
                            value.span,
                            vec!["use `0` to ban an identity on its first exceeded request, or leave `ban` out".to_string()],
                        );
                    }
                }
                "banWindow" => {
                    ban_window = Some(value);
                    self.check_http_config_time_window_in(value, subject);
                }
                _ => {
                    self.check_expr(value);
                    self.error(
                        "E0609",
                        format!("unknown {subject} rateLimit option `{field}`"),
                        Some(field),
                    );
                }
            }
        }
        if !seen_max {
            self.error(
                "E2741",
                format!("{subject} `rateLimit` is missing required field `max`"),
                Some("rateLimit"),
            );
        }
        if !seen_time_window {
            self.error(
                "E2741",
                format!("{subject} `rateLimit` is missing required field `timeWindow`"),
                Some("rateLimit"),
            );
        }
        if let Some(ban_window) = ban_window
            && !seen_ban
        {
            self.error_at_with_help(
                "E2741",
                format!("{subject} `rateLimit.banWindow` requires `ban`"),
                ban_window.span,
                vec!["add `ban` or remove `banWindow`".to_string()],
            );
        }
    }

    pub(super) fn require_http_config_integer(&mut self, field: &str, actual: &ValueType) {
        let is_integer = matches!(
            actual,
            ValueType::IntegerLiteral | ValueType::UnsignedIntegerLiteral
        ) || value_type_to_type(actual)
            .is_some_and(|ty| is_integer_type(transparent_type(&ty)));
        if !is_integer && !matches!(actual, ValueType::Error | ValueType::Unknown) {
            self.error(
                "E0308",
                format!("HTTP route config `{field}` must be an integer"),
                Some(field),
            );
        }
    }

    pub(super) fn check_http_config_body_limit(&mut self, value: &Expr) {
        let Expr {
            kind: ExprKind::StrLit(text),
            ..
        } = value
        else {
            self.check_expr(value);
            self.error(
                "E0308",
                "HTTP route config `bodyLimit` must be a string literal like `\"5mb\"` or `\"off\"`"
                    .to_string(),
                Some("bodyLimit"),
            );
            return;
        };
        if let Err(message) = crate::http_config::parse_body_limit(text) {
            self.error(
                "E3114",
                format!("invalid `bodyLimit`: {message}"),
                Some("bodyLimit"),
            );
        }
    }

    pub(super) fn check_http_config_time_window_in(&mut self, value: &Expr, subject: &str) {
        let Expr {
            kind: ExprKind::StrLit(text),
            ..
        } = value
        else {
            self.check_expr(value);
            self.error_at(
                "E0308",
                format!(
                    "{subject} `timeWindow` must be a string literal like `\"1m\"` or `\"30s\"`"
                ),
                value.span,
            );
            return;
        };
        match crate::http_config::parse_time_window(text) {
            Ok(duration) if duration.is_zero() => self.error_at(
                "E0707",
                format!("{subject} `rateLimit.timeWindow` must be greater than zero"),
                value.span,
            ),
            Ok(_) => {}
            Err(message) => self.error_at(
                "E3115",
                format!("invalid `timeWindow`: {message}"),
                value.span,
            ),
        }
    }

    pub(super) fn check_http_path_params(
        &mut self,
        method: &str,
        path_params: &[String],
        params_schema: Option<&str>,
    ) {
        let mut seen = HashSet::default();
        for param in path_params {
            if !seen.insert(param) {
                self.error(
                    "E2300",
                    format!("duplicate HTTP path param `{param}`"),
                    Some(param),
                );
            }
        }
        let Some(schema_name) = params_schema else {
            if let Some(param) = path_params.first() {
                self.error(
                    "E2741",
                    format!(
                        "HTTP route `{method}` path declares `:{param}` but route options are missing `params` schema"
                    ),
                    Some(param),
                );
            }
            return;
        };
        let Some(fields) = self.interfaces.get(schema_name).cloned() else {
            return;
        };
        for param in path_params {
            if !fields.iter().any(|field| field.name == *param) {
                self.error(
                    "E2741",
                    format!(
                        "HTTP route path param `:{param}` is missing from params schema `{schema_name}`"
                    ),
                    Some(param),
                );
            }
        }
        for field in fields.iter().filter(|field| !field.optional) {
            if !path_params.iter().any(|param| param == &field.name) {
                self.error(
                    "E2741",
                    format!(
                        "required params schema field `{}` is missing from HTTP route path",
                        field.name
                    ),
                    Some(&field.name),
                );
            }
        }
    }

    pub(super) fn check_http_client_response_json(
        &mut self,
        type_args: &[Type],
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        if !args.is_empty() {
            self.error(
                "E0061",
                format!(
                    "method `json` expects 0 argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
            for arg in args {
                self.check_expr(arg);
            }
        }
        let [type_arg] = type_args else {
            self.error_with_help(
                "E0107",
                "`json` needs the type to read the body as".to_string(),
                Some(method),
                vec![
                    "write `response.json<User>()`, naming an interface; the decoder reads a document against that interface's fields".to_string(),
                ],
            );
            return ValueType::Error;
        };
        let Type::Named(name) = transparent_type(type_arg) else {
            self.error_with_help(
                "E0308",
                format!("`json` reads a body as an interface, and `{type_arg}` is not one"),
                Some(method),
                vec!["name an interface, as in `response.json<User>()`".to_string()],
            );
            return ValueType::Error;
        };
        if !self.interfaces.contains_key(name) {
            self.error_with_help(
                "E0412",
                format!("cannot find type `{name}` in this scope"),
                Some(method),
                vec![format!(
                    "import or declare `{name}` here; the decoder is built from its fields"
                )],
            );
            return ValueType::Error;
        }
        if !crate::automatic_data_methods::supports_json(
            &Type::Named(name.clone()),
            &self.interfaces,
            &self.data_interfaces,
        ) {
            self.error_with_help(
                "E0277",
                format!("`{name}` cannot be read from JSON"),
                Some(method),
                vec![format!(
                    "every field of `{name}` has to be a type JSON carries, which is the same rule `{name}.fromJson(...)` follows"
                )],
            );
            return ValueType::Error;
        }
        ValueType::Known(Type::Result {
            success: Box::new(Type::Named(name.clone())),
            error: Box::new(Type::Named("JsonError".to_string())),
        })
    }

    pub(super) fn check_http_route_handler(
        &mut self,
        handler: &Expr,
        contract: &HttpRouteContract,
    ) {
        let request_ty = Type::Object(contract.request_fields.clone());
        let response_ty = http_response_union_type(&contract.responses);
        let reply_ty = Type::Apply {
            name: "HttpReply".to_string(),
            args: vec![http_response_map_type(&contract.responses)],
        };
        let reply_expected = Type::Function {
            params: vec![request_ty.clone(), reply_ty],
            ret: Box::new(Type::Void),
        };
        let async_reply_expected = Type::Function {
            params: match &reply_expected {
                Type::Function { params, .. } => params.clone(),
                _ => unreachable!(),
            },
            ret: Box::new(promise_of(Type::Void)),
        };
        let direct_expected = Type::Function {
            params: vec![request_ty.clone()],
            ret: Box::new(response_ty),
        };
        let handler_subject: &str = match handler.kind() {
            ExprKind::Ident(name) => name,
            _ => "handler",
        };
        if self.takes_only_a_request(handler) {
            self.refuse_direct_http_route_handler(handler, handler_subject, contract);
            return;
        }
        let effects = self.expression_function_effects(handler);
        if !effects.is_empty() {
            let thrown = effects
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" | ");
            self.error_at_with_help(
                "E3298",
                format!(
                    "an HTTP route `handler` cannot throw `{thrown}`: the server calls it with no `catch` around it"
                ),
                handler.span,
                vec![format!(
                    "catch `{thrown}` inside the handler and answer with `reply.status(...)`"
                )],
            );
        }
        let declaring = |expected: &Type| {
            if effects.is_empty() {
                expected.clone()
            } else {
                Type::Throws {
                    function: Box::new(expected.clone()),
                    effects: effects.clone(),
                }
            }
        };
        let actual = match handler.kind() {
            ExprKind::ArrowFn {
                is_async: true,
                params,
                ..
            } if params.len() == 2 => {
                self.check_expr_as(handler, &declaring(&async_reply_expected))
            }
            ExprKind::ArrowFn { params, .. } if params.len() == 2 => {
                self.check_expr_as(handler, &declaring(&reply_expected))
            }
            ExprKind::Ident(_) => self.check_expr(handler),
            _ => self.check_expr_as(handler, &declaring(&reply_expected)),
        };
        if self.is_assignable(&direct_expected, &actual)
            || self.is_legacy_http_schema_handler(&actual)
        {
            self.refuse_direct_http_route_handler(handler, handler_subject, contract);
            return;
        }
        let inline_reply_handler =
            matches!(handler.kind(), ExprKind::ArrowFn { params, .. } if params.len() == 2);
        if inline_reply_handler
            || self.is_http_reply_handler_assignable(&reply_expected, &actual)
            || self.is_http_reply_handler_assignable(&async_reply_expected, &actual)
        {
            self.check_http_reply_flow(handler);
            return;
        }
        if let Some(unresolved) = self.unresolved_http_handler_type(&actual) {
            self.error(
                "E3110",
                format!(
                    "HTTP route `handler` names `{unresolved}`, which this module does not import"
                ),
                Some(handler_subject),
            );
            return;
        }
        self.error(
            "E0308",
            "HTTP route `handler` must accept the inferred route request plus `HttpReply` and return `void` or `Promise<void>`".to_string(),
            Some(handler_subject),
        );
    }

    pub(super) fn unresolved_http_handler_type(&self, actual: &ValueType) -> Option<String> {
        let ValueType::Known(Type::Function { params, ret }) = actual else {
            return None;
        };
        params
            .iter()
            .chain(std::iter::once(ret.as_ref()))
            .find_map(|ty| self.first_unresolved_type_name(ty))
    }

    pub(super) fn is_http_reply_handler_assignable(
        &self,
        expected: &Type,
        actual: &ValueType,
    ) -> bool {
        let Type::Function {
            params: expected_params,
            ret: expected_ret,
        } = expected
        else {
            return false;
        };
        let ValueType::Known(Type::Function {
            params: actual_params,
            ret: actual_ret,
        }) = actual
        else {
            return false;
        };
        if expected_params.len() != actual_params.len() || expected_params.len() != 2 {
            return false;
        }
        let request_matches = self.is_assignable(
            &actual_params[0],
            &ValueType::Known(expected_params[0].clone()),
        );
        let reply_matches =
            self.http_reply_parameter_matches(&expected_params[1], &actual_params[1]);
        let return_matches = transparent_type(expected_ret) == transparent_type(actual_ret);
        request_matches && reply_matches && return_matches
    }

    pub(super) fn http_reply_parameter_matches(&self, expected: &Type, actual: &Type) -> bool {
        if expected == actual {
            return true;
        }
        if self.is_assignable(expected, &ValueType::Known(actual.clone())) {
            return true;
        }
        let (Some(expected_payload), Some(actual_payload)) =
            (http_reply_payload(expected), http_reply_payload(actual))
        else {
            return false;
        };
        let Type::Object(statuses) = transparent_type(expected_payload) else {
            return false;
        };
        let declared = statuses
            .iter()
            .map(|field| transparent_type(&field.ty).clone())
            .collect::<Vec<_>>();
        let written = match transparent_type(actual_payload) {
            Type::Union(variants) => variants
                .iter()
                .map(|variant| transparent_type(variant).clone())
                .collect::<Vec<_>>(),
            single => vec![single.clone()],
        };
        declared.iter().all(|ty| written.contains(ty))
            && written.iter().all(|ty| declared.contains(ty))
    }

    pub(super) fn refuse_direct_http_route_handler(
        &mut self,
        handler: &Expr,
        handler_subject: &str,
        contract: &HttpRouteContract,
    ) {
        let status = match contract.responses.as_slice() {
            [(status, _)] => self
                .enums
                .get("HttpStatus")
                .and_then(|info| {
                    info.members.iter().find_map(|(name, value)| {
                        matches!(value.kind(), ExprKind::IntLit(code) if *code == i128::from(*status))
                            .then(|| format!("HttpStatus.{name}"))
                    })
                })
                .unwrap_or_else(|| "HttpStatus.X".to_string()),
            _ => "HttpStatus.X".to_string(),
        };
        let help = vec![format!(
            "take `(request, reply)` and replace `return value` with `reply.status({status}).send(value)`; a handler answers through the reply and returns nothing"
        )];
        let message = "HTTP route `handler` must use `(request, reply)` and send responses with `reply.status(HttpStatus.X).send(value)`".to_string();
        match handler.kind() {
            ExprKind::Ident(_) => {
                self.error_with_help("E3108", message, Some(handler_subject), help);
            }
            _ => self.error_at_with_help("E3108", message, handler.span, help),
        }
    }

    pub(super) fn check_http_reply_flow(&mut self, handler: &Expr) {
        let metadata = match handler.kind() {
            ExprKind::ArrowFn { params, body, .. } => Some((params.clone(), body.clone())),
            ExprKind::Ident(name) => self
                .fn_body_metadata
                .get(name)
                .map(|metadata| (metadata.params.clone(), metadata.body.clone())),
            _ => None,
        };
        let Some((params, body)) = metadata else {
            return;
        };
        let Some(reply_name) = params.get(1).and_then(http_named_param) else {
            return;
        };
        if http_reply_flow_contains_complex_control(&body) {
            self.warning(
                "E3109",
                "HTTP route handler uses complex control flow that should be reviewed by lint-level analysis".to_string(),
                Some(&reply_name),
            );
        }
        let paths = self.analyze_http_reply_flow_block(
            &body,
            &reply_name,
            vec![HttpReplyFlowPath {
                sent: false,
                streamed: false,
                open: true,
            }],
        );
        if paths.iter().any(|path| path.open && !path.sent) {
            self.error(
                "E3100",
                "HTTP route handler must send a response with `reply.status(...).send(...)` on every success path".to_string(),
                Some(&reply_name),
            );
        }
    }

    pub(super) fn check_http_listen(&mut self, args: &[Expr]) {
        let ([port] | [port, _]) = args else {
            self.error(
                "E0061",
                format!(
                    "`listen` expects 1 to 2 argument(s), but {} were provided",
                    args.len()
                ),
                Some("listen"),
            );
            for arg in args {
                self.check_expr(arg);
            }
            return;
        };
        let actual = self.check_expr_as(port, &Type::I32);
        if !self.is_assignable(&Type::I32, &actual) {
            self.error(
                "E0308",
                format!("`listen` port expects `i32`, found `{}`", actual.display()),
                expression_subject(port),
            );
        }
        if let ExprKind::IntLit(value) = port.kind()
            && !(0..=65535).contains(value)
        {
            self.error_at_with_help(
                "E3111",
                format!("`{value}` is not a valid port; a port is between 0 and 65535"),
                port.span,
                vec!["write a port in the range 0 to 65535, such as `3000`".to_string()],
            );
        }
        let Some(options) = args.get(1) else {
            return;
        };
        if !self.interfaces.contains_key("ServerOptions") {
            self.error(
                "E0412",
                "cannot find type `ServerOptions` in this scope".to_string(),
                Some("listen"),
            );
            self.check_expr(options);
            return;
        }
        if !self.require_http_listen_options_literal(options) {
            self.check_expr(options);
            return;
        }
        for (field, named) in [("tls", "TlsOptions")] {
            if self.http_listen_options_field(options, field).is_some()
                && !self.interfaces.contains_key(named)
            {
                let span = self.current_span();
                self.diagnostics.push(CheckDiagnostic {
                    code: "E0412",
                    severity: CheckSeverity::Error,
                    message: format!("cannot find type `{named}` in this scope"),
                    span,
                    subject: Some(field.to_string()),
                    help: vec![format!("add `{named}` to the import from `tyrus:http`")],
                    prefer_declaration: false,
                });
                self.check_expr(options);
                return;
            }
        }
        let expected = Type::Named("ServerOptions".to_string());
        let actual = self.check_expr_as(options, &expected);
        let before = self.diagnostics.len();
        self.check_http_server_option_literals(options);
        if self.diagnostics.len() == before && !self.is_assignable(&expected, &actual) {
            self.error(
                "E0308",
                format!(
                    "`listen` options expect `ServerOptions`, found `{}`",
                    actual.display()
                ),
                expression_subject(options),
            );
        }
    }

    pub(super) fn check_http_cors_literals(&mut self, fields: &[(String, Expr)]) {
        let credentials = fields
            .iter()
            .find(|(name, _)| name == "credentials")
            .filter(|(_, value)| matches!(value.kind(), ExprKind::BoolLit(true)));

        for (name, value) in fields {
            let ExprKind::ArrayLit(entries) = value.kind() else {
                continue;
            };
            match name.as_str() {
                "origin" | "allowedHeaders" if credentials.is_some() => {
                    if let Some(wildcard) = entries
                        .iter()
                        .find(|entry| matches!(entry.kind(), ExprKind::StrLit(text) if text == "*"))
                    {
                        self.error_at_with_help(
                                "E2742",
                                format!(
                                    "CORS `{name}` cannot be `\"*\"` while `credentials` is `true`"
                                ),
                                wildcard.span,
                                vec![
                                    format!(
                                        "a browser refuses this pair, so every credentialed request would fail; list the exact {} instead, or set `credentials: false`",
                                        if name == "origin" { "origins" } else { "headers" }
                                    ),
                                ],
                            );
                    }
                }
                _ => {}
            }
            match name.as_str() {
                "origin" => {
                    if entries.is_empty() {
                        self.error_at_with_help(
                            "E2743",
                            "CORS `origin` cannot be empty".to_string(),
                            value.span,
                            vec!["an empty list matches no origin at all, which refuses every cross-origin request; remove the `cors` call if that is what you want".to_string()],
                        );
                    }
                    for entry in entries {
                        let ExprKind::StrLit(origin) = entry.kind() else {
                            continue;
                        };
                        if let Some(reason) = invalid_cors_origin(origin) {
                            self.error_at_with_help(
                                "E2743",
                                format!("`{origin}` is not a valid CORS origin"),
                                entry.span,
                                vec![reason],
                            );
                        }
                    }
                }
                "allowedHeaders" | "exposedHeaders" => {
                    for entry in entries {
                        let ExprKind::StrLit(header) = entry.kind() else {
                            continue;
                        };
                        if header != "*" && !is_http_header_name(header) {
                            self.error_at_with_help(
                                "E2743",
                                format!("`{header}` is not a valid header name"),
                                entry.span,
                                vec!["a header name may contain letters, digits and `!#$%&'*+-.^_`|~`".to_string()],
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn http_listen_options_field<'a>(
        &self,
        options: &'a Expr,
        name: &str,
    ) -> Option<&'a Expr> {
        let ExprKind::ObjectLit(fields) = options.kind() else {
            return None;
        };
        fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value)
    }

    pub(super) fn require_http_listen_options_literal(&mut self, options: &Expr) -> bool {
        if matches!(options.kind(), ExprKind::ObjectLit(_)) {
            return true;
        }
        self.error_at(
            "E3113",
            "`listen` options must be written as an object literal".to_string(),
            options.span,
        );
        false
    }

    pub(super) fn check_http_server_option_literals(&mut self, options: &Expr) {
        let ExprKind::ObjectLit(fields) = options.kind() else {
            return;
        };
        for (name, value) in fields {
            match name.as_str() {
                "requestTimeout" | "shutdownTimeout" => {
                    let ExprKind::StrLit(window) = value.kind() else {
                        continue;
                    };
                    if let Err(reason) = crate::http_config::parse_time_window(window) {
                        self.error_at("E3112", reason, value.span);
                    }
                }
                "host" => {
                    let ExprKind::StrLit(host) = value.kind() else {
                        continue;
                    };
                    if host.trim().parse::<std::net::IpAddr>().is_err() {
                        self.error_at(
                            "E3112",
                            format!("`{host}` is not a valid host address"),
                            value.span,
                        );
                    }
                }
                "rateLimit" => {
                    self.check_http_rate_limit(value, "server");
                    self.error_at_with_help(
                        "E3294",
                        "`rateLimit` is a server policy, and `listen` options carry the address and the connection".to_string(),
                        value.span,
                        vec![
                            "move it to `app.rateLimit({ max: ..., timeWindow: ... })`, beside `app.cors(...)` and `app.trustedProxies(...)`; a route that declares `config.rateLimit` still counts against its own".to_string(),
                        ],
                    );
                }
                "headerTimeout" => {
                    let ExprKind::StrLit(window) = value.kind() else {
                        continue;
                    };
                    match crate::http_config::parse_time_window(window) {
                        Err(reason) => self.error_at("E3112", reason, value.span),
                        Ok(duration) if duration.is_zero() => self.error_at_with_help(
                            "E3112",
                            "`headerTimeout` must be greater than zero".to_string(),
                            value.span,
                            vec!["a zero clock would cut off every connection before it could send a request".to_string()],
                        ),
                        Ok(_) => {}
                    }
                }
                "maxHeaderSize" => {
                    let ExprKind::StrLit(size) = value.kind() else {
                        continue;
                    };
                    match crate::http_config::parse_body_limit(size) {
                        Err(_) | Ok(crate::http_config::BodyLimit::Off) => self.error_at_with_help(
                            "E3112",
                            format!("`{size}` is not a valid header size"),
                            value.span,
                            vec![
                                "use a byte size like `\"32kb\"` or `\"64kib\"`".to_string(),
                                "`\"off\"` is not accepted here: an unbounded header block is what a slowloris client sends".to_string(),
                            ],
                        ),
                        Ok(crate::http_config::BodyLimit::Bytes(bytes))
                            if bytes < crate::http_config::MIN_MAX_HEADER_BYTES =>
                        {
                            self.error_at_with_help(
                                "E3112",
                                format!(
                                    "`maxHeaderSize` must be at least {} bytes",
                                    crate::http_config::MIN_MAX_HEADER_BYTES
                                ),
                                value.span,
                                vec![
                                    "the HTTP/1 parser refuses to run below this, so a smaller value would stop the server on its first connection rather than tighten anything".to_string(),
                                    "write `\"8kib\"` for the smallest accepted header block".to_string(),
                                ],
                            );
                        }
                        Ok(_) => {}
                    }
                }
                "maxConcurrentRequests" | "maxConnections" => {
                    let ExprKind::IntLit(limit) = value.kind() else {
                        continue;
                    };
                    if *limit <= 0 {
                        self.error_at(
                            "E3112",
                            format!("`{name}` must be greater than zero"),
                            value.span,
                        );
                    }
                }
                "tls" => {
                    let ExprKind::ObjectLit(tls) = value.kind() else {
                        continue;
                    };
                    for (field, path) in tls {
                        let ExprKind::StrLit(path) = path.kind() else {
                            continue;
                        };
                        if path.trim().is_empty() {
                            self.error_at_with_help(
                                "E3112",
                                format!("`tls.{field}` must name a file"),
                                value.span,
                                vec![format!(
                                    "write the path to the PEM file holding the {}",
                                    if field == "cert" {
                                        "certificate chain"
                                    } else {
                                        "private key"
                                    }
                                )],
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn check_http_trusted_proxy_literals(&mut self, arg: &Expr) {
        let ExprKind::ArrayLit(values) = arg.kind() else {
            return;
        };
        for value in values {
            let ExprKind::StrLit(proxy) = value.kind() else {
                continue;
            };
            if parse_trusted_proxy(proxy).is_none() {
                self.error_at(
                    "E3116",
                    format!("`{proxy}` is not a valid trusted proxy address or network"),
                    value.span,
                );
            }
        }
    }

    pub(super) fn check_http_trusted_proxy_identity_header_literal(&mut self, arg: &Expr) {
        let ExprKind::StrLit(header) = arg.kind() else {
            return;
        };
        if !is_http_header_name(header) {
            self.error_at(
                "E3117",
                format!("`{header}` is not a valid HTTP header name"),
                arg.span,
            );
        }
    }

    pub(super) fn check_http_header_name_literal(&mut self, arg: &Expr, method: &str) {
        let ExprKind::StrLit(name) = arg.kind() else {
            return;
        };
        if !is_http_header_name(name) {
            self.error_at(
                "E3117",
                format!("`{name}` is not a valid HTTP header name"),
                arg.span,
            );
            return;
        }
        if is_runtime_owned_http_header(name) {
            self.error_at(
                "E3118",
                format!(
                    "`{}` is set by the runtime and cannot be changed by `reply.{method}`",
                    name.to_ascii_lowercase()
                ),
                arg.span,
            );
        }
    }

    pub(super) fn check_http_reply_chain_status(&mut self, expr: &Expr, reply_name: &str) {
        if http_reply_chain_sets_status(expr, reply_name) {
            return;
        }
        let message = "`reply.send(...)` or `reply.stream()` must be preceded by `reply.status(HttpStatus.X)`";
        match http_reply_send_span(expr, reply_name) {
            Some(span) => self.error_at("E3101", message.to_string(), span),
            None => self.error("E3101", message.to_string(), Some("send")),
        }
    }

    pub(super) fn analyze_http_reply_flow_block(
        &mut self,
        body: &[Stmt],
        reply_name: &str,
        mut paths: Vec<HttpReplyFlowPath>,
    ) -> Vec<HttpReplyFlowPath> {
        for statement in body {
            if paths.iter().all(|path| !path.open) {
                break;
            }
            match statement.kind() {
                StmtKind::Expr(expr)
                | StmtKind::VarDecl {
                    value: Some(expr), ..
                } if http_reply_finalization_chain_count(expr, reply_name) > 0 => {
                    let send_count = http_reply_finalization_chain_count(expr, reply_name);
                    self.check_http_reply_chain_status(expr, reply_name);
                    let mut reported = false;
                    for path in paths.iter_mut().filter(|path| path.open) {
                        if (path.sent || send_count > 1) && !reported {
                            if path.streamed {
                                self.error_at(
                                    "E3119",
                                    "`reply` cannot be used after `reply.stream()` transfers the response"
                                        .to_string(),
                                    http_reply_method_span(expr, reply_name).unwrap_or(expr.span),
                                );
                            } else {
                                self.error(
                                    "E3104",
                                    "HTTP response was already sent on this controller path"
                                        .to_string(),
                                    Some("send"),
                                );
                            }
                            reported = true;
                        }
                        path.sent = true;
                        path.streamed |= http_reply_chain_opens_writable_stream(expr, reply_name);
                    }
                }
                StmtKind::Expr(expr)
                | StmtKind::VarDecl {
                    value: Some(expr), ..
                } if paths.iter().any(|path| path.open && path.streamed)
                    && http_reply_chain_rooted_expr(expr, reply_name) =>
                {
                    self.error_at(
                        "E3119",
                        "`reply` cannot be used after `reply.stream()` transfers the response"
                            .to_string(),
                        http_reply_method_span(expr, reply_name).unwrap_or(expr.span),
                    );
                }
                StmtKind::Expr(expr) if http_reply_status_without_send_expr(expr, reply_name) => {
                    self.error(
                        "E3102",
                        "`reply.status(...)` must be completed with `.send(...)`".to_string(),
                        Some("status"),
                    );
                }
                StmtKind::Return(None) => {
                    let has_missing_response = paths.iter().any(|path| path.open && !path.sent);
                    if has_missing_response {
                        self.error(
                            "E3100",
                            "HTTP route handler returns before sending a response".to_string(),
                            Some("return"),
                        );
                    }
                    for path in paths.iter_mut().filter(|path| path.open) {
                        path.open = false;
                    }
                }
                StmtKind::Return(Some(expr)) => {
                    let returns_reply_send =
                        http_reply_finalization_chain_count(expr, reply_name) > 0;
                    let has_missing_response = paths
                        .iter()
                        .any(|path| path.open && !path.sent && !returns_reply_send);
                    if has_missing_response {
                        self.error(
                            "E3100",
                            "HTTP route handler returns before sending a response".to_string(),
                            Some("return"),
                        );
                    }
                    if returns_reply_send {
                        let send_count = http_reply_finalization_chain_count(expr, reply_name);
                        self.check_http_reply_chain_status(expr, reply_name);
                        let mut reported = false;
                        for path in paths.iter_mut().filter(|path| path.open) {
                            if (path.sent || send_count > 1) && !reported {
                                if path.streamed {
                                    self.error_at(
                                        "E3119",
                                        "`reply` cannot be used after `reply.stream()` transfers the response"
                                            .to_string(),
                                        http_reply_method_span(expr, reply_name)
                                            .unwrap_or(expr.span),
                                    );
                                } else {
                                    self.error(
                                        "E3104",
                                        "HTTP response was already sent on this controller path"
                                            .to_string(),
                                        Some("send"),
                                    );
                                }
                                reported = true;
                            }
                            path.sent = true;
                            path.streamed |=
                                http_reply_chain_opens_writable_stream(expr, reply_name);
                        }
                    }
                    for path in paths.iter_mut().filter(|path| path.open) {
                        path.open = false;
                    }
                }
                StmtKind::Throw(_) => {
                    for path in paths.iter_mut().filter(|path| path.open) {
                        path.open = false;
                    }
                }
                StmtKind::Expr(Expr {
                    kind:
                        ExprKind::If {
                            then_block,
                            else_block,
                            ..
                        },
                    ..
                }) => {
                    let open_paths = paths
                        .iter()
                        .copied()
                        .filter(|path| path.open)
                        .collect::<Vec<_>>();
                    let closed_paths = paths
                        .iter()
                        .copied()
                        .filter(|path| !path.open)
                        .collect::<Vec<_>>();
                    let mut next_paths = self.analyze_http_reply_flow_block(
                        then_block,
                        reply_name,
                        open_paths.clone(),
                    );
                    if let Some(else_block) = else_block {
                        next_paths.extend(
                            self.analyze_http_reply_flow_block(else_block, reply_name, open_paths),
                        );
                    } else {
                        next_paths.extend(open_paths);
                    }
                    next_paths.extend(closed_paths);
                    paths = merge_http_reply_flow_paths(next_paths);
                }
                _ => {}
            }
        }
        merge_http_reply_flow_paths(paths)
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn check_http_reply_method(
        &mut self,
        object_ty: &ValueType,
        method: &str,
        args: &[Expr],
        optional_receiver: bool,
    ) -> Option<ValueType> {
        match method {
            "setCookie" => {
                let object_ty = transparent_value_type(object_ty)?;
                http_reply_payload(object_ty)?;
                if !(2..=3).contains(&args.len()) {
                    self.error(
                        "E0061",
                        format!(
                            "`reply.setCookie` expects 2 to 3 argument(s), but {} were provided",
                            args.len()
                        ),
                        Some(method),
                    );
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return Some(ValueType::Known(object_ty.clone()));
                }
                for (index, arg) in args.iter().enumerate() {
                    let expected = if index == 2 {
                        if !self.interfaces.contains_key("CookieOptions") {
                            self.error(
                                "E0412",
                                "cannot find type `CookieOptions` in this scope".to_string(),
                                Some(method),
                            );
                            self.check_expr(arg);
                            continue;
                        }
                        Type::Named("CookieOptions".to_string())
                    } else {
                        Type::Str
                    };
                    let actual = self.check_expr_as(arg, &expected);
                    if !self.is_assignable(&expected, &actual) {
                        let label = match index {
                            0 => "name",
                            1 => "value",
                            _ => "options",
                        };
                        self.error(
                            "E0308",
                            format!(
                                "`reply.setCookie` {label} expects `{expected}`, found `{}`",
                                actual.display()
                            ),
                            expression_subject(arg),
                        );
                    }
                }
                Some(optional_value_type(
                    ValueType::Known(object_ty.clone()),
                    optional_receiver,
                ))
            }
            "setCookies" => {
                let object_ty = transparent_value_type(object_ty)?;
                http_reply_payload(object_ty)?;
                let [cookies] = args else {
                    self.error(
                        "E0061",
                        format!(
                            "`reply.setCookies` expects 1 argument(s), but {} were provided",
                            args.len()
                        ),
                        Some(method),
                    );
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return Some(ValueType::Known(object_ty.clone()));
                };
                if !self.interfaces.contains_key("HttpCookie") {
                    self.error(
                        "E0412",
                        "cannot find type `HttpCookie` in this scope".to_string(),
                        Some(method),
                    );
                    self.check_expr(cookies);
                    return Some(ValueType::Known(object_ty.clone()));
                }
                let expected = Type::Array(Box::new(Type::Named("HttpCookie".to_string())));
                let actual = self.check_expr_as(cookies, &expected);
                if !self.is_assignable(&expected, &actual) {
                    self.error(
                        "E0308",
                        format!(
                            "`reply.setCookies` cookies expects `{expected}`, found `{}`",
                            actual.display()
                        ),
                        expression_subject(cookies),
                    );
                }
                Some(optional_value_type(
                    ValueType::Known(object_ty.clone()),
                    optional_receiver,
                ))
            }
            "clearCookies" => {
                let object_ty = transparent_value_type(object_ty)?;
                http_reply_payload(object_ty)?;
                if !args.is_empty() {
                    self.error(
                        "E0061",
                        format!(
                            "`reply.clearCookies` expects 0 argument(s), but {} were provided",
                            args.len()
                        ),
                        Some(method),
                    );
                    for arg in args {
                        self.check_expr(arg);
                    }
                }
                Some(optional_value_type(
                    ValueType::Known(object_ty.clone()),
                    optional_receiver,
                ))
            }
            "clearCookie" => {
                let object_ty = transparent_value_type(object_ty)?;
                http_reply_payload(object_ty)?;
                if args.len() != 1 {
                    self.error(
                        "E0061",
                        format!(
                            "`reply.clearCookie` expects 1 argument(s), but {} were provided",
                            args.len()
                        ),
                        Some(method),
                    );
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return Some(ValueType::Known(object_ty.clone()));
                }
                if let Some(arg) = args.first() {
                    let actual = self.check_expr_as(arg, &Type::Str);
                    if !self.is_assignable(&Type::Str, &actual) {
                        self.error(
                            "E0308",
                            format!(
                                "`reply.clearCookie` name expects `string`, found `{}`",
                                actual.display()
                            ),
                            expression_subject(arg),
                        );
                    }
                }
                Some(optional_value_type(
                    ValueType::Known(object_ty.clone()),
                    optional_receiver,
                ))
            }
            "header" | "appendHeader" => {
                let object_ty = transparent_value_type(object_ty)?;
                http_reply_payload(object_ty)?;
                if args.len() != 2 {
                    self.error(
                        "E0061",
                        format!(
                            "`reply.{method}` expects 2 argument(s), but {} were provided",
                            args.len()
                        ),
                        Some(method),
                    );
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return Some(ValueType::Known(object_ty.clone()));
                }
                for (index, arg) in args.iter().enumerate() {
                    let actual = self.check_expr_as(arg, &Type::Str);
                    if !self.is_assignable(&Type::Str, &actual) {
                        let label = if index == 0 { "name" } else { "value" };
                        self.error(
                            "E0308",
                            format!(
                                "`reply.{method}` {label} expects `string`, found `{}`",
                                actual.display()
                            ),
                            expression_subject(arg),
                        );
                    }
                    if index == 0 {
                        self.check_http_header_name_literal(arg, method);
                    }
                }
                Some(optional_value_type(
                    ValueType::Known(object_ty.clone()),
                    optional_receiver,
                ))
            }
            "removeHeader" | "type" => {
                let object_ty = transparent_value_type(object_ty)?;
                http_reply_payload(object_ty)?;
                if args.len() != 1 {
                    self.error(
                        "E0061",
                        format!(
                            "`reply.{method}` expects 1 argument(s), but {} were provided",
                            args.len()
                        ),
                        Some(method),
                    );
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return Some(ValueType::Known(object_ty.clone()));
                }
                if let Some(arg) = args.first() {
                    let actual = self.check_expr_as(arg, &Type::Str);
                    if !self.is_assignable(&Type::Str, &actual) {
                        let label = if method == "removeHeader" {
                            "name"
                        } else {
                            "value"
                        };
                        self.error(
                            "E0308",
                            format!(
                                "`reply.{method}` {label} expects `string`, found `{}`",
                                actual.display()
                            ),
                            expression_subject(arg),
                        );
                    }
                    if method == "removeHeader" {
                        self.check_http_header_name_literal(arg, method);
                    }
                }
                Some(optional_value_type(
                    ValueType::Known(object_ty.clone()),
                    optional_receiver,
                ))
            }
            "status" => {
                let object_ty = transparent_value_type(object_ty)?;
                if http_reply_status_payload(object_ty).is_some() {
                    self.error(
                        "E3103",
                        "HTTP route handler can set the reply status only once on a controller path".to_string(),
                        Some(method),
                    );
                    return Some(ValueType::Error);
                }
                let reply = http_reply_payload(object_ty)?;
                if args.len() != 1 {
                    self.error(
                        "E0061",
                        format!(
                            "`reply.status` expects 1 argument(s), but {} were provided",
                            args.len()
                        ),
                        Some(method),
                    );
                }
                let status = args.first().and_then(|arg| self.http_status_code_arg(arg));
                if let Some(arg) = args.first()
                    && status.is_none()
                {
                    self.check_expr(arg);
                }
                let Some(status) = status else {
                    return Some(ValueType::Error);
                };
                let Some(schema) = http_reply_schema_for_status(reply, status) else {
                    self.error(
                        "E3106",
                        format!("HTTP response status `{status}` is not declared in the route `response` object"),
                        expression_subject(args.first().unwrap()),
                    );
                    return Some(ValueType::Error);
                };
                Some(optional_value_type(
                    ValueType::Known(Type::Apply {
                        name: "__HttpReplyStatus".to_string(),
                        args: vec![
                            Type::Lit(Box::new(Expr::synthetic(ExprKind::IntLit(status as i128)))),
                            schema,
                        ],
                    }),
                    optional_receiver,
                ))
            }
            "stream" => {
                let object_ty = transparent_value_type(object_ty)?;
                let (status, schema) = match http_reply_status_payload(object_ty) {
                    Some(payload) => payload,
                    None => (200, http_reply_payload(object_ty)?.clone()),
                };
                if !args.is_empty() {
                    self.error(
                        "E0061",
                        format!(
                            "`reply.stream` expects 0 argument(s), but {} were provided",
                            args.len()
                        ),
                        Some(method),
                    );
                    for arg in args {
                        self.check_expr(arg);
                    }
                }
                if http_status_forbids_body(status)
                    || !matches!(&schema, Type::Named(name) if name == "StreamBody")
                {
                    self.error(
                        "E0308",
                        "`reply.stream()` requires this response status to declare `StreamBody`"
                            .to_string(),
                        Some(method),
                    );
                    return Some(ValueType::Error);
                }
                Some(optional_value_type(
                    ValueType::Known(Type::Apply {
                        name: "WritableStream".to_string(),
                        args: vec![Type::Array(Box::new(Type::U8))],
                    }),
                    optional_receiver,
                ))
            }
            "send" => {
                let object_ty = transparent_value_type(object_ty)?;
                let (status, schema) = match http_reply_status_payload(object_ty) {
                    Some(payload) => payload,
                    None => (200, http_reply_payload(object_ty)?.clone()),
                };
                if http_status_forbids_body(status) {
                    if args.len() > 1 {
                        self.error(
                            "E0061",
                            format!(
                                "`reply.send` for HTTP status `{status}` expects 0 to 1 argument(s), but {} were provided",
                                args.len()
                            ),
                            Some(method),
                        );
                    }
                    if let Some(options) = args.first() {
                        self.check_http_reply_options(options, method);
                    }
                    return Some(ValueType::Known(Type::Void));
                }
                if !(1..=2).contains(&args.len()) {
                    self.error(
                        "E0061",
                        format!(
                            "`reply.send` expects 1 to 2 argument(s), but {} were provided",
                            args.len()
                        ),
                        Some(method),
                    );
                    return Some(ValueType::Known(Type::Void));
                }
                if let Some(arg) = args.first() {
                    let expected = match &schema {
                        Type::Named(name) => crate::http_body::shapeless_body_type(name)
                            .unwrap_or_else(|| schema.clone()),
                        _ => schema.clone(),
                    };
                    let actual = self.check_expr_as(arg, &expected);
                    if !self.is_assignable(&expected, &actual) {
                        self.error_at(
                            "E3105",
                            format!(
                                "`reply.send` expects `{}`, found `{}`",
                                expected,
                                actual.display()
                            ),
                            arg.span,
                        );
                    }
                }
                if let Some(options) = args.get(1) {
                    self.check_http_reply_options(options, method);
                }
                Some(ValueType::Known(Type::Void))
            }
            _ => None,
        }
    }

    pub(super) fn http_status_code_arg(&mut self, arg: &Expr) -> Option<u16> {
        self.http_status_code_arg_for(arg, "`reply.status` expects `HttpStatus.<Name>`")
    }

    pub(super) fn check_http_reply_options(&mut self, options: &Expr, method: &str) {
        let expected = Type::Named("HttpReplyOptions".to_string());
        if !self.interfaces.contains_key("HttpReplyOptions") {
            self.error(
                "E0412",
                "cannot find type `HttpReplyOptions` in this scope".to_string(),
                expression_subject(options),
            );
            self.check_expr(options);
            return;
        }
        let actual = self.check_expr_as(options, &expected);
        if !self.is_assignable(&expected, &actual) {
            self.error(
                "E0308",
                format!(
                    "`reply.{method}` options expect `HttpReplyOptions`, found `{}`",
                    actual.display()
                ),
                expression_subject(options),
            );
        }
    }

    pub(super) fn http_status_code_arg_for(
        &mut self,
        arg: &Expr,
        expected_message: &str,
    ) -> Option<u16> {
        match arg.kind() {
            ExprKind::FieldAccess { object, field } => {
                let Expr {
                    kind: ExprKind::Ident(enum_name),
                    ..
                } = object.as_ref()
                else {
                    self.error(
                        "E3107",
                        expected_message.to_string(),
                        expression_subject(arg),
                    );
                    return None;
                };
                let Some(enum_info) = self.enums.get(enum_name) else {
                    self.error(
                        "E3107",
                        expected_message.to_string(),
                        expression_subject(arg),
                    );
                    return None;
                };
                let value = enum_info.members.iter().find_map(|(member, value)| {
                    (member == field).then_some(value).and_then(|value| {
                        if let ExprKind::IntLit(value) = value.kind() {
                            u16::try_from(*value).ok()
                        } else {
                            None
                        }
                    })
                });
                self.mark_used(enum_name);
                let Some(value) = value else {
                    self.error(
                        "E0609",
                        format!("no integer member `{field}` on enum `{enum_name}`"),
                        Some(field),
                    );
                    return None;
                };
                Some(value)
            }
            ExprKind::IntLit(_) | ExprKind::UIntLit(_) => {
                self.error(
                    "E3107",
                    format!("{expected_message}, not a raw numeric status code"),
                    expression_subject(arg),
                );
                None
            }
            _ => {
                self.error(
                    "E3107",
                    expected_message.to_string(),
                    expression_subject(arg),
                );
                None
            }
        }
    }

    pub(super) fn check_http_schema_reference(
        &mut self,
        field: &str,
        value: &Expr,
    ) -> Option<Type> {
        let Expr {
            kind: ExprKind::Ident(name),
            ..
        } = value
        else {
            self.check_expr(value);
            self.error(
                "E0308",
                format!("HTTP route `{field}` schema must be a named interface"),
                Some(field),
            );
            return None;
        };
        if crate::http_body::is_shapeless_body(name) && !matches!(field, "body" | "response") {
            self.error_with_help(
                "E3293",
                format!("`{name}` describes a body, and `{field}` is not one"),
                Some(name),
                vec![format!(
                    "declare an interface for `{field}`, whose fields are the names the request carries; `{name}` can only be a route's `body` or `response`"
                )],
            );
            return None;
        }
        if crate::http_body::is_request_only_body(name) && field == "response" {
            self.error_with_help(
                "E3293",
                format!("`{name}` reads a request body, so a `response` cannot be one"),
                Some(name),
                vec![
                    "answer with an interface, `TextBody` or `BytesBody`; `MultipartBody` only appears as a route's `body`".to_string(),
                ],
            );
            return None;
        }
        let supported = if field == "response" {
            self.is_supported_http_response_schema(name)
        } else {
            self.is_supported_http_schema(name)
        };
        if !supported {
            let usage = if field == "response" {
                HttpSchemaUse::Write
            } else {
                HttpSchemaUse::Read
            };
            match self.http_schema_rejection(name, usage, "", &mut HashSet::default()) {
                Some((where_, reason)) if !where_.is_empty() => self.error_with_help(
                    "E0308",
                    format!("HTTP route `{field}` schema `{name}` cannot be used: field `{where_}` is not supported"),
                    Some(name),
                    vec![reason],
                ),
                Some((_, reason)) => self.error_with_help(
                    "E0308",
                    format!("HTTP route `{field}` schema `{name}` cannot be used"),
                    Some(name),
                    vec![reason],
                ),
                None => self.error(
                    "E0308",
                    format!("HTTP route `{field}` schema `{name}` is not a supported interface"),
                    Some(name),
                ),
            }
            return None;
        }
        Some(Type::Named(name.clone()))
    }

    fn check_http_response_schema_type(&mut self, value: &Expr) -> Option<Type> {
        let ty = match value.kind() {
            ExprKind::Ident(name) => match name.as_str() {
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
                _ => return self.check_http_schema_reference("response", value),
            },
            ExprKind::TypeAssert { ty, .. } => ty.clone(),
            _ => {
                self.check_expr(value);
                self.error(
                    "E0308",
                    "HTTP route `response` schema must be an interface, an array type, or a supported primitive".to_string(),
                    Some("response"),
                );
                return None;
            }
        };
        if let Some((where_, reason)) =
            self.http_schema_field_rejection(&ty, HttpSchemaUse::Write, "", &mut HashSet::default())
        {
            self.error_with_help(
                "E0308",
                if where_.is_empty() {
                    format!("HTTP route `response` schema `{ty}` cannot be used")
                } else {
                    format!("HTTP route `response` schema `{ty}` cannot be used: field `{where_}` is not supported")
                },
                Some("response"),
                vec![reason],
            );
            return None;
        }
        Some(ty)
    }

    pub(super) fn check_http_response_schema_reference(
        &mut self,
        value: &Expr,
    ) -> Vec<(u16, Type)> {
        match value.kind() {
            ExprKind::Ident(name) if name == "void" => Vec::new(),
            ExprKind::Ident(_) | ExprKind::TypeAssert { .. } => self
                .check_http_response_schema_type(value)
                .map(|ty| vec![(200, ty)])
                .unwrap_or_default(),
            ExprKind::ObjectLit(responses) => {
                if responses.is_empty() {
                    self.error(
                        "E2741",
                        "HTTP route `response` object must declare at least one status code"
                            .to_string(),
                        Some("response"),
                    );
                }
                let mut schemas = Vec::new();
                let mut seen = HashSet::default();
                for (status, schema) in responses {
                    let status_code = match status.parse::<u16>() {
                        Ok(status_code) => status_code,
                        Err(_) => {
                            self.error(
                                "E0308",
                                format!(
                                    "HTTP response key `{status}` must be a numeric status code"
                                ),
                                Some(status),
                            );
                            continue;
                        }
                    };
                    if !(100..=599).contains(&status_code) {
                        self.error(
                            "E0308",
                            format!("HTTP response status code `{status_code}` must be in range 100..=599"),
                            Some(status),
                        );
                        continue;
                    }
                    if !seen.insert(status_code) {
                        self.error(
                            "E2300",
                            format!("duplicate HTTP response status code `{status_code}`"),
                            Some(status),
                        );
                        continue;
                    }
                    if matches!(schema.kind(), ExprKind::Ident(name) if name == "void") {
                        if !http_status_forbids_body(status_code) {
                            self.error(
                                "E0308",
                                format!(
                                    "HTTP response status `{status_code}` may include a body and must declare a schema interface instead of `void`"
                                ),
                                Some("void"),
                            );
                        } else {
                            schemas.push((status_code, Type::Void));
                        }
                    } else if http_status_forbids_body(status_code) {
                        self.error(
                            "E0308",
                            format!(
                                "HTTP response status `{status_code}` does not allow a body and must declare `void`"
                            ),
                            Some(status),
                        );
                    } else if let Some(schema) = self.check_http_response_schema_type(schema) {
                        schemas.push((status_code, schema));
                    }
                }
                schemas
            }
            _ => {
                self.check_expr(value);
                self.error(
                    "E0308",
                    "HTTP route `response` must be a response type or a status-code object"
                        .to_string(),
                    Some("response"),
                );
                Vec::new()
            }
        }
    }
}
