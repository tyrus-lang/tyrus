use super::*;

#[path = "builtin_tables/builtin_tables.rs"]
mod builtin_tables;

pub(crate) use builtin_tables::{fs_builtins, math_builtins};

#[allow(clippy::too_many_lines)]
pub(crate) fn http_builtins() -> HashMap<String, ExportInfo> {
    [
        (
            "createRouter".to_string(),
            ExportInfo {
                target: "__tyrus_http_createRouter".to_string(),
                ty: Type::Function {
                    params: Vec::new(),
                    ret: Box::new(Type::Named("HttpRouter".to_string())),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "HttpStatus".to_string(),
            ExportInfo {
                target: "HttpStatus".to_string(),
                ty: Type::Named("HttpStatus".to_string()),
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: Some(http_status_enum()),
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "HttpReply".to_string(),
            ExportInfo {
                target: "HttpReply".to_string(),
                ty: Type::Named("HttpReply".to_string()),
                interface_fields: Some(Vec::new()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "HttpRouter".to_string(),
            ExportInfo {
                target: "HttpRouter".to_string(),
                ty: Type::Named("HttpRouter".to_string()),
                interface_fields: Some(Vec::new()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "HttpRequest".to_string(),
            ExportInfo {
                target: "HttpRequest".to_string(),
                ty: Type::Named("HttpRequest".to_string()),
                interface_fields: Some(crate::builtin_interfaces::http_request_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "HttpAccessLog".to_string(),
            ExportInfo {
                target: "HttpAccessLog".to_string(),
                ty: Type::Named("HttpAccessLog".to_string()),
                interface_fields: Some(crate::builtin_interfaces::http_access_log_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "HttpResponse".to_string(),
            ExportInfo {
                target: "HttpResponse".to_string(),
                ty: Type::Named("HttpResponse".to_string()),
                interface_fields: Some(crate::builtin_interfaces::http_response_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "HttpCacheOptions".to_string(),
            ExportInfo {
                target: "HttpCacheOptions".to_string(),
                ty: Type::Named("HttpCacheOptions".to_string()),
                interface_fields: Some(crate::builtin_interfaces::http_cache_options_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "HttpReplyOptions".to_string(),
            ExportInfo {
                target: "HttpReplyOptions".to_string(),
                ty: Type::Named("HttpReplyOptions".to_string()),
                interface_fields: Some(crate::builtin_interfaces::http_reply_options_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "WebSocket".to_string(),
            ExportInfo {
                target: "WebSocket".to_string(),
                ty: Type::Named("WebSocket".to_string()),
                interface_fields: Some(Vec::new()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "WebSocketMessage".to_string(),
            ExportInfo {
                target: "WebSocketMessage".to_string(),
                ty: Type::Named("WebSocketMessage".to_string()),
                interface_fields: Some(crate::builtin_interfaces::websocket_message_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "connectWebSocket".to_string(),
            ExportInfo {
                target: "__tyrus_http_connectWebSocket".to_string(),
                ty: Type::Function {
                    params: vec![
                        Type::Str,
                        Type::Optional(Box::new(Type::Array(Box::new(Type::Str)))),
                    ],
                    ret: Box::new(Type::Apply {
                        name: "Promise".to_string(),
                        args: vec![Type::Result {
                            success: Box::new(Type::Named("WebSocket".to_string())),
                            error: Box::new(Type::Named("HttpError".to_string())),
                        }],
                    }),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "HttpServer".to_string(),
            ExportInfo {
                target: "HttpServer".to_string(),
                ty: Type::Named("HttpServer".to_string()),
                interface_fields: Some(Vec::new()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "HttpError".to_string(),
            ExportInfo {
                target: "HttpError".to_string(),
                ty: Type::Named("HttpError".to_string()),
                interface_fields: Some(crate::builtin_interfaces::http_error_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "ServerOptions".to_string(),
            ExportInfo {
                target: "ServerOptions".to_string(),
                ty: Type::Named("ServerOptions".to_string()),
                interface_fields: Some(crate::builtin_interfaces::server_options_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "RateLimit".to_string(),
            ExportInfo {
                target: "RateLimit".to_string(),
                ty: Type::Named("RateLimit".to_string()),
                interface_fields: Some(crate::builtin_interfaces::rate_limit_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "TextBody".to_string(),
            ExportInfo {
                target: "TextBody".to_string(),
                ty: Type::Named("TextBody".to_string()),
                interface_fields: Some(crate::builtin_interfaces::text_body_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "BytesBody".to_string(),
            ExportInfo {
                target: "BytesBody".to_string(),
                ty: Type::Named("BytesBody".to_string()),
                interface_fields: Some(crate::builtin_interfaces::bytes_body_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "StreamBody".to_string(),
            ExportInfo {
                target: "StreamBody".to_string(),
                ty: Type::Named("StreamBody".to_string()),
                interface_fields: Some(crate::builtin_interfaces::stream_body_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "fetch".to_string(),
            ExportInfo {
                target: "__tyrus_http_fetch".to_string(),
                ty: Type::Function {
                    params: vec![
                        Type::Str,
                        Type::Optional(Box::new(Type::Named("FetchOptions".to_string()))),
                    ],
                    ret: Box::new(Type::Apply {
                        name: "Promise".to_string(),
                        args: vec![Type::Named("HttpClientResponse".to_string())],
                    }),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "HttpClientResponse".to_string(),
            ExportInfo {
                target: "HttpClientResponse".to_string(),
                ty: Type::Named("HttpClientResponse".to_string()),
                interface_fields: Some(Vec::new()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "HttpClientStreamResponse".to_string(),
            ExportInfo {
                target: "HttpClientStreamResponse".to_string(),
                ty: Type::Named("HttpClientStreamResponse".to_string()),
                interface_fields: Some(Vec::new()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "FetchOptions".to_string(),
            ExportInfo {
                target: "FetchOptions".to_string(),
                ty: Type::Named("FetchOptions".to_string()),
                interface_fields: Some(crate::builtin_interfaces::fetch_options_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "HttpHeader".to_string(),
            ExportInfo {
                target: "HttpHeader".to_string(),
                ty: Type::Named("HttpHeader".to_string()),
                interface_fields: Some(crate::builtin_interfaces::http_header_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "MultipartBody".to_string(),
            ExportInfo {
                target: "MultipartBody".to_string(),
                ty: Type::Named("MultipartBody".to_string()),
                interface_fields: Some(crate::builtin_interfaces::multipart_body_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "MultipartPart".to_string(),
            ExportInfo {
                target: "MultipartPart".to_string(),
                ty: Type::Named("MultipartPart".to_string()),
                interface_fields: Some(crate::builtin_interfaces::multipart_part_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "TlsOptions".to_string(),
            ExportInfo {
                target: "TlsOptions".to_string(),
                ty: Type::Named("TlsOptions".to_string()),
                interface_fields: Some(crate::builtin_interfaces::tls_options_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "CorsOptions".to_string(),
            ExportInfo {
                target: "CorsOptions".to_string(),
                ty: Type::Named("CorsOptions".to_string()),
                interface_fields: Some(crate::builtin_interfaces::cors_options_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "CsrfOptions".to_string(),
            ExportInfo {
                target: "CsrfOptions".to_string(),
                ty: Type::Named("CsrfOptions".to_string()),
                interface_fields: Some(crate::builtin_interfaces::csrf_options_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "FetchPolicyOptions".to_string(),
            ExportInfo {
                target: "FetchPolicyOptions".to_string(),
                ty: Type::Named("FetchPolicyOptions".to_string()),
                interface_fields: Some(crate::builtin_interfaces::fetch_policy_options_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "CookieOptions".to_string(),
            ExportInfo {
                target: "CookieOptions".to_string(),
                ty: Type::Named("CookieOptions".to_string()),
                interface_fields: Some(crate::builtin_interfaces::cookie_options_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "HttpCookie".to_string(),
            ExportInfo {
                target: "HttpCookie".to_string(),
                ty: Type::Named("HttpCookie".to_string()),
                interface_fields: Some(crate::builtin_interfaces::http_cookie_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
    ]
    .into_iter()
    .collect()
}

pub(crate) fn stream_builtins() -> HashMap<String, ExportInfo> {
    let mut exports = [
        "ReadableStream",
        "WritableStream",
        "DuplexStream",
        "TransformStream",
    ]
    .into_iter()
    .map(|name| {
        (
            name.to_string(),
            ExportInfo {
                target: name.to_string(),
                ty: Type::Named(name.to_string()),
                interface_fields: Some(Vec::new()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        )
    })
    .collect::<HashMap<_, _>>();
    exports.insert(
        "StreamError".to_string(),
        ExportInfo {
            target: "StreamError".to_string(),
            ty: Type::Named("StreamError".to_string()),
            interface_fields: Some(crate::builtin_interfaces::stream_error_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: Vec::new(),
            db_model: None,
        },
    );
    exports
}

pub(crate) fn process_builtins() -> HashMap<String, ExportInfo> {
    [
        (
            "exec".to_string(),
            ExportInfo {
                target: "__tyrus_process_exec".to_string(),
                ty: Type::Function {
                    params: vec![
                        Type::Str,
                        Type::Optional(Box::new(Type::Array(Box::new(Type::Str)))),
                    ],
                    ret: Box::new(Type::Result {
                        success: Box::new(Type::Named("ChildProcess".to_string())),
                        error: Box::new(Type::Named("ProcessError".to_string())),
                    }),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "ChildProcess".to_string(),
            ExportInfo {
                target: "ChildProcess".to_string(),
                ty: Type::Named("ChildProcess".to_string()),
                interface_fields: Some(crate::builtin_interfaces::child_process_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "ProcessError".to_string(),
            ExportInfo {
                target: "ProcessError".to_string(),
                ty: Type::Named("ProcessError".to_string()),
                interface_fields: Some(crate::builtin_interfaces::process_error_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
    ]
    .into_iter()
    .collect()
}

pub(crate) fn utils_builtins() -> HashMap<String, ExportInfo> {
    [
        (
            "styleText".to_string(),
            ExportInfo {
                target: "styleText".to_string(),
                ty: Type::Named(crate::style_text::STYLE_TEXT_MARKER.to_string()),
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "figlet".to_string(),
            ExportInfo {
                target: "figlet".to_string(),
                ty: Type::Named(crate::figlet::FIGLET_MARKER.to_string()),
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
    ]
    .into_iter()
    .collect()
}

pub(crate) fn cli_builtins() -> HashMap<String, ExportInfo> {
    let mut builtins: HashMap<String, ExportInfo> = crate::cli::prompt_builtins()
        .iter()
        .map(|builtin| {
            (
                builtin.name.to_string(),
                ExportInfo {
                    target: builtin.target.to_string(),
                    ty: crate::cli::prompt_type(builtin),
                    interface_fields: None,
                    behavior_info: None,
                    type_alias_ty: None,
                    enum_info: None,
                    throws: Vec::new(),
                    db_model: None,
                },
            )
        })
        .collect();
    builtins.insert(
        "isTTY".to_string(),
        ExportInfo {
            target: crate::cli::IS_TTY_TARGET.to_string(),
            ty: crate::cli::is_tty_type(),
            interface_fields: None,
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: Vec::new(),
            db_model: None,
        },
    );
    builtins
}

/// The export table of one `tyrus:*` module, built at most once per process.
///
/// Every lookup below used to rebuild the whole table: `native_target_return_type`
/// rebuilt all sixteen of them to answer one question about one target, and it
/// runs per call expression during code generation. The immutable registry owns
/// its tables for the process lifetime, so callers can receive stable references
/// without leaking heap allocations. `tyrus:test` is intentionally absent from
/// `NATIVE_MODULES` so production builds can reject it, but its table still has
/// to be available while the module loader enforces that rule.
pub(crate) fn native_builtins(source: &str) -> Option<&'static HashMap<String, ExportInfo>> {
    type Registry = HashMap<&'static str, HashMap<String, ExportInfo>>;
    static REGISTRY: std::sync::OnceLock<Registry> = std::sync::OnceLock::new();
    REGISTRY
        .get_or_init(|| {
            NATIVE_MODULES
                .iter()
                .copied()
                .chain(std::iter::once("tyrus:test"))
                .map(|module| {
                    let builtins = build_native_builtins(module)
                        .expect("every native module must have an export table");
                    (module, builtins)
                })
                .collect()
        })
        .get(source)
}

fn build_native_builtins(source: &str) -> Option<HashMap<String, ExportInfo>> {
    match source {
        "tyrus:algorithm" => Some(crate::algorithm::algorithm_builtins()),
        "tyrus:archive" => Some(crate::archive::archive_builtins()),
        "tyrus:compress" => Some(crate::compress::compress_builtins()),
        "tyrus:fs" => Some(fs_builtins()),
        "tyrus:http" => Some(http_builtins()),
        "tyrus:stream" => Some(stream_builtins()),
        "tyrus:process" => Some(process_builtins()),
        "tyrus:utils" => Some(utils_builtins()),
        "tyrus:cli" => Some(cli_builtins()),
        "tyrus:test" => Some(crate::test::test_builtins()),
        "tyrus:task" => Some(
            [
                (
                    "isCancelled".to_string(),
                    ExportInfo {
                        target: crate::task::IS_CANCELLED_TARGET.to_string(),
                        ty: crate::task::is_cancelled_type(),
                        interface_fields: None,
                        behavior_info: None,
                        type_alias_ty: None,
                        enum_info: None,
                        throws: Vec::new(),
                        db_model: None,
                    },
                ),
                (
                    "cancel".to_string(),
                    ExportInfo {
                        target: crate::task::CANCEL_TARGET.to_string(),
                        ty: crate::task::cancel_type(),
                        interface_fields: None,
                        behavior_info: None,
                        type_alias_ty: None,
                        enum_info: None,
                        throws: Vec::new(),
                        db_model: None,
                    },
                ),
                (
                    "sleep".to_string(),
                    ExportInfo {
                        target: crate::task::SLEEP_TARGET.to_string(),
                        ty: crate::task::sleep_type(),
                        interface_fields: None,
                        behavior_info: None,
                        type_alias_ty: None,
                        enum_info: None,
                        throws: Vec::new(),
                        db_model: None,
                    },
                ),
                (
                    "sleepAsync".to_string(),
                    ExportInfo {
                        target: crate::task::SLEEP_ASYNC_TARGET.to_string(),
                        ty: crate::task::sleep_async_type(),
                        interface_fields: None,
                        behavior_info: None,
                        type_alias_ty: None,
                        enum_info: None,
                        throws: Vec::new(),
                        db_model: None,
                    },
                ),
                (
                    "spawn".to_string(),
                    ExportInfo {
                        target: crate::task::SPAWN_TARGET.to_string(),
                        ty: crate::task::spawn_type(),
                        interface_fields: None,
                        behavior_info: None,
                        type_alias_ty: None,
                        enum_info: None,
                        throws: Vec::new(),
                        db_model: None,
                    },
                ),
                (
                    "channel".to_string(),
                    ExportInfo {
                        target: crate::task::CHANNEL_TARGET.to_string(),
                        ty: crate::task::channel_type(),
                        interface_fields: None,
                        behavior_info: None,
                        type_alias_ty: None,
                        enum_info: None,
                        throws: Vec::new(),
                        db_model: None,
                    },
                ),
                (
                    "Mutex".to_string(),
                    ExportInfo {
                        target: crate::task::MUTEX_TARGET.to_string(),
                        ty: crate::task::mutex_type(),
                        interface_fields: None,
                        behavior_info: None,
                        type_alias_ty: None,
                        enum_info: None,
                        throws: Vec::new(),
                        db_model: None,
                    },
                ),
                (
                    "RwLock".to_string(),
                    ExportInfo {
                        target: crate::task::RWLOCK_TARGET.to_string(),
                        ty: crate::task::rwlock_type(),
                        interface_fields: None,
                        behavior_info: None,
                        type_alias_ty: None,
                        enum_info: None,
                        throws: Vec::new(),
                        db_model: None,
                    },
                ),
                (
                    "Semaphore".to_string(),
                    ExportInfo {
                        target: crate::task::SEMAPHORE_TARGET.to_string(),
                        ty: crate::task::semaphore_type(),
                        interface_fields: None,
                        behavior_info: None,
                        type_alias_ty: None,
                        enum_info: None,
                        throws: Vec::new(),
                        db_model: None,
                    },
                ),
                (
                    "joinAll".to_string(),
                    ExportInfo {
                        target: crate::task::JOIN_ALL_TARGET.to_string(),
                        ty: crate::task::join_all_type(),
                        interface_fields: None,
                        behavior_info: None,
                        type_alias_ty: None,
                        enum_info: None,
                        throws: Vec::new(),
                        db_model: None,
                    },
                ),
                (
                    "race".to_string(),
                    ExportInfo {
                        target: crate::task::RACE_TARGET.to_string(),
                        ty: crate::task::race_type(),
                        interface_fields: None,
                        behavior_info: None,
                        type_alias_ty: None,
                        enum_info: None,
                        throws: Vec::new(),
                        db_model: None,
                    },
                ),
                (
                    "timeout".to_string(),
                    ExportInfo {
                        target: crate::task::TIMEOUT_TARGET.to_string(),
                        ty: crate::task::timeout_type(),
                        interface_fields: None,
                        behavior_info: None,
                        type_alias_ty: None,
                        enum_info: None,
                        throws: Vec::new(),
                        db_model: None,
                    },
                ),
                task_handle_type_export("JoinHandle"),
                task_handle_type_export("Sender"),
                task_handle_type_export("Receiver"),
            ]
            .into_iter()
            .collect(),
        ),
        "tyrus:crypto" => Some(crate::crypto::crypto_builtins()),
        "tyrus:db" => Some(crate::db::db_builtins()),
        "tyrus:encoding" => Some(crate::encoding::encoding_builtins()),
        "tyrus:memory" => Some(crate::memory::memory_builtins()),
        "tyrus:math" => Some(math_builtins()),
        "tyrus:net" => Some(crate::net::net_builtins()),
        "tyrus:os" => Some(crate::os::os_builtins()),
        "tyrus:simd" => Some(crate::simd::simd_builtins()),
        "tyrus:temporal" => Some(crate::temporal::temporal_builtins()),
        _ => None,
    }
}

pub fn native_interface_module(name: &str) -> Option<&'static str> {
    NATIVE_MODULES.iter().copied().find(|module| {
        native_builtins(module)
            .and_then(|builtins| builtins.get(name).map(|info| info.ty.clone()))
            .is_some_and(|ty| matches!(&ty, Type::Named(named) if named == name))
    })
}

pub fn native_export_info(
    source: &str,
    name: &str,
) -> Option<(Type, Option<Vec<Field>>, Option<Type>)> {
    native_builtins(source).and_then(|builtins| {
        builtins.get(name).map(|info| {
            (
                info.ty.clone(),
                info.interface_fields.clone(),
                info.type_alias_ty.clone(),
            )
        })
    })
}

#[derive(Debug, Clone)]
pub struct NativeImportAnalysisInfo {
    pub ty: Type,
    pub interface_fields: Option<Vec<Field>>,
    pub behavior_info: Option<BehaviorInterfaceInfo>,
    pub type_alias_ty: Option<Type>,
    pub enum_info: Option<ResolvedEnum>,
    pub throws: Vec<Type>,
    pub db_model: Option<crate::ast::DbModelMetadata>,
}

pub fn native_export_analysis_info(source: &str, name: &str) -> Option<NativeImportAnalysisInfo> {
    native_builtins(source).and_then(|builtins| {
        builtins.get(name).map(|info| NativeImportAnalysisInfo {
            ty: info.ty.clone(),
            interface_fields: info.interface_fields.clone(),
            behavior_info: info.behavior_info.clone(),
            type_alias_ty: info.type_alias_ty.clone(),
            enum_info: info.enum_info.clone(),
            throws: info.throws.clone(),
            db_model: info.db_model.clone(),
        })
    })
}

pub fn native_export_target(source: &str, name: &str) -> Option<String> {
    native_builtins(source).and_then(|builtins| builtins.get(name).map(|info| info.target.clone()))
}

pub fn native_module_exporting(name: &str) -> Option<&'static str> {
    NATIVE_MODULES
        .iter()
        .copied()
        .find(|source| native_builtins(source).is_some_and(|builtins| builtins.contains_key(name)))
}

pub fn native_target_return_type(target: &str) -> Option<Type> {
    for source in NATIVE_MODULES {
        let Some(builtins) = native_builtins(source) else {
            continue;
        };
        for info in builtins.values() {
            if info.target != target {
                continue;
            }
            if let Type::Function { ret, .. } = &info.ty {
                return Some((**ret).clone());
            }
        }
    }
    None
}

pub fn native_export_names(source: &str) -> Vec<String> {
    let mut names = native_builtins(source)
        .map(|builtins| builtins.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    names.sort();
    names
}

pub fn native_export_enum(source: &str, name: &str) -> Option<ResolvedEnum> {
    native_builtins(source).and_then(|builtins| {
        builtins
            .get(name)
            .and_then(|info| info.enum_info.as_ref().cloned())
    })
}

pub fn native_export_behavior(source: &str, name: &str) -> Option<BehaviorInterfaceInfo> {
    native_builtins(source).and_then(|builtins| {
        builtins
            .get(name)
            .and_then(|info| info.behavior_info.as_ref().cloned())
    })
}
