use super::*;
use tyrus_core::ast::{ExprKind, expr_display, lit_display};
use tyrus_core::doc_comments::DocComment;

#[allow(clippy::too_many_lines)]
pub(super) fn native_import_type(module: &str, name: &str) -> Option<Type> {
    if module == "tyrus:archive"
        || module == "tyrus:compress"
        || module == "tyrus:crypto"
        || module == "tyrus:encoding"
        || module == "tyrus:math"
        || module == "tyrus:net"
        || module == "tyrus:process"
        || module == "tyrus:temporal"
        || module == "tyrus:test"
    {
        return tyrus_core::module_loader::native_export_info(module, name).map(|(ty, _, _)| ty);
    }
    match (module, name) {
        ("tyrus:fs", "readFileSync") => Some(Type::Function {
            params: vec![Type::Str, Type::Str],
            ret: Box::new(Type::Str),
        }),
        ("tyrus:fs", "writeFileSync") => Some(Type::Function {
            params: vec![Type::Str, Type::Str, Type::Str],
            ret: Box::new(Type::Void),
        }),
        ("tyrus:http", "createRouter") => Some(Type::Function {
            params: Vec::new(),
            ret: Box::new(Type::Named("HttpRouter".to_string())),
        }),
        ("tyrus:http", "HttpStatus") => Some(Type::Named("HttpStatus".to_string())),
        ("tyrus:http", "HttpReply") => Some(Type::Named("HttpReply".to_string())),
        ("tyrus:http", "HttpRouter") => Some(Type::Named("HttpRouter".to_string())),
        ("tyrus:http", "HttpRequest") => Some(Type::Named("HttpRequest".to_string())),
        ("tyrus:http", "HttpResponse") => Some(Type::Named("HttpResponse".to_string())),
        ("tyrus:http", "HttpCacheOptions") => Some(Type::Named("HttpCacheOptions".to_string())),
        ("tyrus:http", "HttpReplyOptions") => Some(Type::Named("HttpReplyOptions".to_string())),
        ("tyrus:http", "HttpError") => Some(Type::Named("HttpError".to_string())),
        ("tyrus:http", "HttpServer") => Some(Type::Named("HttpServer".to_string())),
        ("tyrus:http", "WebSocket") => Some(Type::Named("WebSocket".to_string())),
        ("tyrus:http", "WebSocketMessage") => Some(Type::Named("WebSocketMessage".to_string())),
        ("tyrus:http", "ServerOptions") => Some(Type::Named("ServerOptions".to_string())),
        ("tyrus:http", "RateLimit") => Some(Type::Named("RateLimit".to_string())),
        ("tyrus:http", "TlsOptions") => Some(Type::Named("TlsOptions".to_string())),
        ("tyrus:http", "TextBody") => Some(Type::Named("TextBody".to_string())),
        ("tyrus:http", "BytesBody") => Some(Type::Named("BytesBody".to_string())),
        ("tyrus:http", "StreamBody") => Some(Type::Named("StreamBody".to_string())),
        ("tyrus:http", "MultipartBody") => Some(Type::Named("MultipartBody".to_string())),
        ("tyrus:http", "MultipartPart") => Some(Type::Named("MultipartPart".to_string())),
        ("tyrus:http", "HttpClientResponse") => Some(Type::Named("HttpClientResponse".to_string())),
        ("tyrus:http", "HttpClientStreamResponse") => {
            Some(Type::Named("HttpClientStreamResponse".to_string()))
        }
        ("tyrus:http", "FetchOptions") => Some(Type::Named("FetchOptions".to_string())),
        ("tyrus:http", "HttpHeader") => Some(Type::Named("HttpHeader".to_string())),
        ("tyrus:http", "CorsOptions") => Some(Type::Named("CorsOptions".to_string())),
        ("tyrus:http", "CsrfOptions") => Some(Type::Named("CsrfOptions".to_string())),
        ("tyrus:http", "HttpAccessLog") => Some(Type::Named("HttpAccessLog".to_string())),
        ("tyrus:http", "FetchPolicyOptions") => Some(Type::Named("FetchPolicyOptions".to_string())),
        ("tyrus:http", "CookieOptions") => Some(Type::Named("CookieOptions".to_string())),
        ("tyrus:http", "HttpCookie") => Some(Type::Named("HttpCookie".to_string())),
        ("tyrus:utils", "styleText") => Some(Type::Named(
            tyrus_core::style_text::STYLE_TEXT_MARKER.to_string(),
        )),
        ("tyrus:utils", "figlet") => {
            Some(Type::Named(tyrus_core::figlet::FIGLET_MARKER.to_string()))
        }
        ("tyrus:cli", "isTTY") => Some(tyrus_core::cli::is_tty_type()),
        ("tyrus:cli", name) => tyrus_core::cli::prompt_builtins()
            .iter()
            .find(|builtin| builtin.name == name)
            .map(tyrus_core::cli::prompt_type),
        ("tyrus:task", "cancel") => Some(tyrus_core::task::cancel_type()),
        ("tyrus:task", "sleep") => Some(tyrus_core::task::sleep_type()),
        ("tyrus:task", "sleepAsync") => Some(tyrus_core::task::sleep_async_type()),
        ("tyrus:task", "spawn") => Some(tyrus_core::task::spawn_type()),
        ("tyrus:os", "EOL") => Some(Type::Str),
        ("tyrus:os", "platform") => Some(Type::Function {
            params: Vec::new(),
            ret: Box::new(Type::Named("OsPlatform".to_string())),
        }),
        ("tyrus:os", "OsPlatform") => Some(Type::Named("OsPlatform".to_string())),
        ("tyrus:os", "arch") => Some(Type::Function {
            params: Vec::new(),
            ret: Box::new(Type::Named("OsArch".to_string())),
        }),
        ("tyrus:os", "OsArch") => Some(Type::Named("OsArch".to_string())),
        (
            "tyrus:os",
            "hostname" | "release" | "systemName" | "locale" | "language" | "userName" | "homeDir"
            | "cacheDir" | "configDir" | "dataDir" | "tempDir",
        ) => Some(Type::Function {
            params: Vec::new(),
            ret: Box::new(Type::Str),
        }),
        ("tyrus:os", "whoami") => Some(Type::Function {
            params: Vec::new(),
            ret: Box::new(Type::Optional(Box::new(Type::Str))),
        }),
        ("tyrus:os", "desktopDir" | "documentsDir") => Some(Type::Function {
            params: Vec::new(),
            ret: Box::new(Type::Optional(Box::new(Type::Str))),
        }),
        ("tyrus:os", "appDataDir") => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Str),
        }),
        ("tyrus:os", "userInfo") => Some(Type::Function {
            params: Vec::new(),
            ret: Box::new(Type::Named("UserInfo".to_string())),
        }),
        ("tyrus:os", "cpus") => Some(Type::Function {
            params: Vec::new(),
            ret: Box::new(Type::Array(Box::new(Type::Named("CpuInfo".to_string())))),
        }),
        ("tyrus:os", "networkInterfaces") => Some(Type::Function {
            params: Vec::new(),
            ret: Box::new(Type::Array(Box::new(Type::Named(
                "NetworkInterface".to_string(),
            )))),
        }),
        ("tyrus:os", "totalMem" | "freeMem" | "uptime") => Some(Type::Function {
            params: Vec::new(),
            ret: Box::new(Type::U64),
        }),
        ("tyrus:os", "loadAvg") => Some(Type::Function {
            params: Vec::new(),
            ret: Box::new(Type::Array(Box::new(Type::F64))),
        }),
        ("tyrus:os", "battery") => Some(Type::Function {
            params: Vec::new(),
            ret: Box::new(Type::Optional(Box::new(Type::Named(
                "BatteryInfo".to_string(),
            )))),
        }),
        ("tyrus:os", "cpuTemp") => Some(Type::Function {
            params: vec![Type::Union(vec![
                Type::Lit(Box::new(Expr::synthetic(ExprKind::StrLit(
                    "celsius".to_string(),
                )))),
                Type::Lit(Box::new(Expr::synthetic(ExprKind::StrLit(
                    "fahrenheit".to_string(),
                )))),
            ])],
            ret: Box::new(Type::Optional(Box::new(Type::F32))),
        }),
        ("tyrus:os", "notify") => Some(Type::Function {
            params: vec![
                Type::Str,
                Type::Str,
                Type::Named("NotifyOptions".to_string()),
            ],
            ret: Box::new(Type::Void),
        }),
        ("tyrus:os", "NotifyUrgency") => Some(Type::Named("NotifyUrgency".to_string())),
        ("tyrus:os", "requestAttention") => Some(Type::Function {
            params: vec![Type::Bool],
            ret: Box::new(Type::Void),
        }),
        ("tyrus:os", "revealInFolder") => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Void),
        }),
        ("tyrus:os", "openUrl" | "trash") => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Result {
                success: Box::new(Type::Void),
                error: Box::new(Type::Named("OsError".to_string())),
            }),
        }),
        ("tyrus:os", "clipboardWrite") => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Void),
        }),
        ("tyrus:os", "clipboardRead") => Some(Type::Function {
            params: Vec::new(),
            ret: Box::new(Type::Optional(Box::new(Type::Str))),
        }),
        ("tyrus:os", "isDarkMode" | "isScreenLocked" | "isSessionLocked") => Some(Type::Function {
            params: Vec::new(),
            ret: Box::new(Type::Bool),
        }),
        ("tyrus:os", "terminalSize") => Some(Type::Function {
            params: Vec::new(),
            ret: Box::new(Type::Optional(Box::new(Type::Tuple(vec![
                Type::U16,
                Type::U16,
            ])))),
        }),
        ("tyrus:os", "preventSleep" | "allowSleep") => Some(Type::Function {
            params: Vec::new(),
            ret: Box::new(Type::Result {
                success: Box::new(Type::Void),
                error: Box::new(Type::Named("OsError".to_string())),
            }),
        }),
        ("tyrus:os", "FontInfo") => Some(Type::Named("FontInfo".to_string())),
        ("tyrus:os", "systemFonts") => Some(Type::Function {
            params: Vec::new(),
            ret: Box::new(Type::Array(Box::new(Type::Named("FontInfo".to_string())))),
        }),
        ("tyrus:os", "Color") => Some(Type::Named("Color".to_string())),
        ("tyrus:os", "accessibleColorContrast") => Some(Type::Function {
            params: vec![
                Type::Named("Color".to_string()),
                Type::Named("Color".to_string()),
            ],
            ret: Box::new(Type::Bool),
        }),
        ("tyrus:os", "SystemCapability") => Some(Type::Named("SystemCapability".to_string())),
        ("tyrus:os", "hasCapability") => Some(Type::Function {
            params: vec![Type::Named("SystemCapability".to_string())],
            ret: Box::new(Type::Bool),
        }),
        ("tyrus:temporal", "TimeZone") => Some(Type::Named("TimeZone".to_string())),
        ("tyrus:temporal", "TemporalError") => Some(Type::Named("TemporalError".to_string())),
        ("tyrus:temporal", "Instant") => Some(Type::Named("Instant".to_string())),
        ("tyrus:temporal", "ZonedDateTime") => Some(Type::Named("ZonedDateTime".to_string())),
        ("tyrus:temporal", "PlainDate") => Some(Type::Named("PlainDate".to_string())),
        ("tyrus:temporal", "PlainTime") => Some(Type::Named("PlainTime".to_string())),
        ("tyrus:temporal", "Duration") => Some(Type::Named("Duration".to_string())),
        ("tyrus:temporal", "PlainDateTime") => Some(Type::Named("PlainDateTime".to_string())),
        ("tyrus:temporal", "PlainYearMonth") => Some(Type::Named("PlainYearMonth".to_string())),
        ("tyrus:temporal", "PlainMonthDay") => Some(Type::Named("PlainMonthDay".to_string())),
        ("tyrus:temporal", "timeZone") => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Named("TimeZone".to_string())),
        }),
        ("tyrus:temporal", "instantNow") => Some(Type::Function {
            params: Vec::new(),
            ret: Box::new(Type::Named("Instant".to_string())),
        }),
        ("tyrus:temporal", "zonedDateTimeFrom") => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Named("ZonedDateTime".to_string())),
        }),
        ("tyrus:temporal", "zonedDateTimeToInstant") => Some(Type::Function {
            params: vec![Type::Named("ZonedDateTime".to_string())],
            ret: Box::new(Type::Named("Instant".to_string())),
        }),
        ("tyrus:temporal", "zonedDateTimeToString") => Some(Type::Function {
            params: vec![Type::Named("ZonedDateTime".to_string())],
            ret: Box::new(Type::Str),
        }),
        ("tyrus:temporal", "instantToZonedDateTime") => Some(Type::Function {
            params: vec![
                Type::Named("Instant".to_string()),
                Type::Named("TimeZone".to_string()),
            ],
            ret: Box::new(Type::Named("ZonedDateTime".to_string())),
        }),
        ("tyrus:temporal", "zonedDateTimeEquals") => Some(Type::Function {
            params: vec![
                Type::Named("ZonedDateTime".to_string()),
                Type::Named("ZonedDateTime".to_string()),
            ],
            ret: Box::new(Type::Bool),
        }),
        ("tyrus:temporal", "zonedDateTimeAddDuration") => Some(Type::Function {
            params: vec![
                Type::Named("ZonedDateTime".to_string()),
                Type::Named("Duration".to_string()),
            ],
            ret: Box::new(Type::Named("ZonedDateTime".to_string())),
        }),
        ("tyrus:temporal", "zonedDateTimeUntil") => Some(Type::Function {
            params: vec![
                Type::Named("ZonedDateTime".to_string()),
                Type::Named("ZonedDateTime".to_string()),
            ],
            ret: Box::new(Type::Named("Duration".to_string())),
        }),
        ("tyrus:temporal", "zonedDateTimeFormat") => Some(Type::Function {
            params: vec![Type::Named("ZonedDateTime".to_string()), Type::Str],
            ret: Box::new(Type::Str),
        }),
        ("tyrus:temporal", "instantFrom") => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Named("Instant".to_string())),
        }),
        ("tyrus:temporal", "instantFromEpochMilliseconds") => Some(Type::Function {
            params: vec![Type::I64],
            ret: Box::new(Type::Named("Instant".to_string())),
        }),
        ("tyrus:temporal", "instantToEpochMilliseconds") => Some(Type::Function {
            params: vec![Type::Named("Instant".to_string())],
            ret: Box::new(Type::I64),
        }),
        ("tyrus:temporal", "instantToString") => Some(Type::Function {
            params: vec![Type::Named("Instant".to_string())],
            ret: Box::new(Type::Str),
        }),
        ("tyrus:temporal", "instantAddDuration" | "instantSubtractDuration") => {
            Some(Type::Function {
                params: vec![
                    Type::Named("Instant".to_string()),
                    Type::Named("Duration".to_string()),
                ],
                ret: Box::new(Type::Named("Instant".to_string())),
            })
        }
        ("tyrus:temporal", "plainDateFrom") => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Named("PlainDate".to_string())),
        }),
        ("tyrus:temporal", "plainDateFromFields") => Some(Type::Function {
            params: vec![Type::Named("PlainDate".to_string())],
            ret: Box::new(Type::Named("PlainDate".to_string())),
        }),
        ("tyrus:temporal", "plainDateWith") => Some(Type::Function {
            params: vec![
                Type::Named("PlainDate".to_string()),
                Type::Named("PlainDate".to_string()),
            ],
            ret: Box::new(Type::Named("PlainDate".to_string())),
        }),
        ("tyrus:temporal", "plainDateSubtractDuration") => Some(Type::Function {
            params: vec![
                Type::Named("PlainDate".to_string()),
                Type::Named("Duration".to_string()),
            ],
            ret: Box::new(Type::Named("PlainDate".to_string())),
        }),
        ("tyrus:temporal", "plainDateUntil" | "plainDateSince") => Some(Type::Function {
            params: vec![
                Type::Named("PlainDate".to_string()),
                Type::Named("PlainDate".to_string()),
            ],
            ret: Box::new(Type::Named("Duration".to_string())),
        }),
        ("tyrus:temporal", "plainDateToString") => Some(Type::Function {
            params: vec![Type::Named("PlainDate".to_string())],
            ret: Box::new(Type::Str),
        }),
        ("tyrus:temporal", "plainDateEquals") => Some(Type::Function {
            params: vec![
                Type::Named("PlainDate".to_string()),
                Type::Named("PlainDate".to_string()),
            ],
            ret: Box::new(Type::Bool),
        }),
        ("tyrus:temporal", "plainDateAddDays") => Some(Type::Function {
            params: vec![Type::Named("PlainDate".to_string()), Type::I64],
            ret: Box::new(Type::Named("PlainDate".to_string())),
        }),
        ("tyrus:temporal", "plainDateDayOfWeek" | "plainDateDayOfYear") => Some(Type::Function {
            params: vec![Type::Named("PlainDate".to_string())],
            ret: Box::new(Type::U32),
        }),
        ("tyrus:temporal", "plainDateAddMonths" | "plainDateAddYears") => Some(Type::Function {
            params: vec![Type::Named("PlainDate".to_string()), Type::I64],
            ret: Box::new(Type::Named("PlainDate".to_string())),
        }),
        ("tyrus:temporal", "plainDateDaysUntil") => Some(Type::Function {
            params: vec![
                Type::Named("PlainDate".to_string()),
                Type::Named("PlainDate".to_string()),
            ],
            ret: Box::new(Type::I64),
        }),
        ("tyrus:temporal", "plainTimeFrom") => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Named("PlainTime".to_string())),
        }),
        ("tyrus:temporal", "plainTimeFromFields") => Some(Type::Function {
            params: vec![Type::Named("PlainTime".to_string())],
            ret: Box::new(Type::Named("PlainTime".to_string())),
        }),
        ("tyrus:temporal", "plainTimeSubtractDuration") => Some(Type::Function {
            params: vec![
                Type::Named("PlainTime".to_string()),
                Type::Named("Duration".to_string()),
            ],
            ret: Box::new(Type::Named("PlainTime".to_string())),
        }),
        ("tyrus:temporal", "plainTimeUntil" | "plainTimeSince") => Some(Type::Function {
            params: vec![
                Type::Named("PlainTime".to_string()),
                Type::Named("PlainTime".to_string()),
            ],
            ret: Box::new(Type::Named("Duration".to_string())),
        }),
        ("tyrus:temporal", "plainTimeToString") => Some(Type::Function {
            params: vec![Type::Named("PlainTime".to_string())],
            ret: Box::new(Type::Str),
        }),
        ("tyrus:temporal", "plainTimeEquals") => Some(Type::Function {
            params: vec![
                Type::Named("PlainTime".to_string()),
                Type::Named("PlainTime".to_string()),
            ],
            ret: Box::new(Type::Bool),
        }),
        ("tyrus:temporal", "plainTimeAddSeconds") => Some(Type::Function {
            params: vec![Type::Named("PlainTime".to_string()), Type::I64],
            ret: Box::new(Type::Named("PlainTime".to_string())),
        }),
        ("tyrus:temporal", "plainTimeSecondsUntil") => Some(Type::Function {
            params: vec![
                Type::Named("PlainTime".to_string()),
                Type::Named("PlainTime".to_string()),
            ],
            ret: Box::new(Type::I64),
        }),
        ("tyrus:temporal", "plainTimeAddNanoseconds") => Some(Type::Function {
            params: vec![Type::Named("PlainTime".to_string()), Type::I64],
            ret: Box::new(Type::Named("PlainTime".to_string())),
        }),
        ("tyrus:temporal", "plainTimeNanosecondsUntil") => Some(Type::Function {
            params: vec![
                Type::Named("PlainTime".to_string()),
                Type::Named("PlainTime".to_string()),
            ],
            ret: Box::new(Type::I64),
        }),
        ("tyrus:temporal", "durationFromFields") => Some(Type::Function {
            params: vec![Type::Named("Duration".to_string())],
            ret: Box::new(Type::Named("Duration".to_string())),
        }),
        ("tyrus:temporal", "durationTotal") => Some(Type::Function {
            params: vec![Type::Named("Duration".to_string()), Type::Str],
            ret: Box::new(Type::I64),
        }),
        ("tyrus:temporal", "durationRound") => Some(Type::Function {
            params: vec![
                Type::Named("Duration".to_string()),
                Type::Str,
                Type::I64,
                Type::Str,
                Type::Named("PlainDate".to_string()),
            ],
            ret: Box::new(Type::Named("Duration".to_string())),
        }),
        ("tyrus:temporal", "plainDateAddDuration") => Some(Type::Function {
            params: vec![
                Type::Named("PlainDate".to_string()),
                Type::Named("Duration".to_string()),
            ],
            ret: Box::new(Type::Named("PlainDate".to_string())),
        }),
        ("tyrus:temporal", "plainTimeAddDuration") => Some(Type::Function {
            params: vec![
                Type::Named("PlainTime".to_string()),
                Type::Named("Duration".to_string()),
            ],
            ret: Box::new(Type::Named("PlainTime".to_string())),
        }),
        ("tyrus:temporal", "plainDateTimeFrom") => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Named("PlainDateTime".to_string())),
        }),
        ("tyrus:temporal", "plainDateTimeFromFields") => Some(Type::Function {
            params: vec![Type::Named("PlainDateTime".to_string())],
            ret: Box::new(Type::Named("PlainDateTime".to_string())),
        }),
        ("tyrus:temporal", "plainDateTimeToString") => Some(Type::Function {
            params: vec![Type::Named("PlainDateTime".to_string())],
            ret: Box::new(Type::Str),
        }),
        ("tyrus:temporal", "plainDateTimeEquals") => Some(Type::Function {
            params: vec![
                Type::Named("PlainDateTime".to_string()),
                Type::Named("PlainDateTime".to_string()),
            ],
            ret: Box::new(Type::Bool),
        }),
        ("tyrus:temporal", "plainDateTimeWith") => Some(Type::Function {
            params: vec![
                Type::Named("PlainDateTime".to_string()),
                Type::Named("PlainDateTime".to_string()),
            ],
            ret: Box::new(Type::Named("PlainDateTime".to_string())),
        }),
        ("tyrus:temporal", "plainDateTimeAddDuration" | "plainDateTimeSubtractDuration") => {
            Some(Type::Function {
                params: vec![
                    Type::Named("PlainDateTime".to_string()),
                    Type::Named("Duration".to_string()),
                ],
                ret: Box::new(Type::Named("PlainDateTime".to_string())),
            })
        }
        ("tyrus:temporal", "plainDateTimeUntil" | "plainDateTimeSince") => Some(Type::Function {
            params: vec![
                Type::Named("PlainDateTime".to_string()),
                Type::Named("PlainDateTime".to_string()),
            ],
            ret: Box::new(Type::Named("Duration".to_string())),
        }),
        ("tyrus:temporal", "plainDateUntilWithOptions" | "plainDateSinceWithOptions") => {
            Some(Type::Function {
                params: vec![
                    Type::Named("PlainDate".to_string()),
                    Type::Named("PlainDate".to_string()),
                    Type::Str,
                    Type::Str,
                    Type::I64,
                    Type::Str,
                ],
                ret: Box::new(Type::Named("Duration".to_string())),
            })
        }
        ("tyrus:temporal", "plainTimeUntilWithOptions" | "plainTimeSinceWithOptions") => {
            Some(Type::Function {
                params: vec![
                    Type::Named("PlainTime".to_string()),
                    Type::Named("PlainTime".to_string()),
                    Type::Str,
                    Type::Str,
                    Type::I64,
                    Type::Str,
                ],
                ret: Box::new(Type::Named("Duration".to_string())),
            })
        }
        ("tyrus:temporal", "plainDateTimeUntilWithOptions" | "plainDateTimeSinceWithOptions") => {
            Some(Type::Function {
                params: vec![
                    Type::Named("PlainDateTime".to_string()),
                    Type::Named("PlainDateTime".to_string()),
                    Type::Str,
                    Type::Str,
                    Type::I64,
                    Type::Str,
                ],
                ret: Box::new(Type::Named("Duration".to_string())),
            })
        }
        ("tyrus:temporal", "plainYearMonthFrom") => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Named("PlainYearMonth".to_string())),
        }),
        ("tyrus:temporal", "plainYearMonthFromFields") => Some(Type::Function {
            params: vec![Type::Named("PlainYearMonth".to_string())],
            ret: Box::new(Type::Named("PlainYearMonth".to_string())),
        }),
        ("tyrus:temporal", "plainYearMonthToString") => Some(Type::Function {
            params: vec![Type::Named("PlainYearMonth".to_string())],
            ret: Box::new(Type::Str),
        }),
        ("tyrus:temporal", "plainYearMonthAdd") => Some(Type::Function {
            params: vec![Type::Named("PlainYearMonth".to_string()), Type::I64],
            ret: Box::new(Type::Named("PlainYearMonth".to_string())),
        }),
        ("tyrus:temporal", "plainMonthDayFrom") => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Named("PlainMonthDay".to_string())),
        }),
        ("tyrus:temporal", "plainMonthDayFromFields") => Some(Type::Function {
            params: vec![Type::Named("PlainMonthDay".to_string())],
            ret: Box::new(Type::Named("PlainMonthDay".to_string())),
        }),
        ("tyrus:temporal", "plainMonthDayToString") => Some(Type::Function {
            params: vec![Type::Named("PlainMonthDay".to_string())],
            ret: Box::new(Type::Str),
        }),
        ("tyrus:temporal", "plainMonthDayToPlainDate") => Some(Type::Function {
            params: vec![Type::Named("PlainMonthDay".to_string()), Type::I32],
            ret: Box::new(Type::Named("PlainDate".to_string())),
        }),
        (
            "tyrus:math",
            "abs" | "sign" | "sqrt" | "cbrt" | "floor" | "ceil" | "trunc" | "round" | "sin" | "cos"
            | "tan" | "asin" | "acos" | "atan" | "sinh" | "cosh" | "tanh" | "exp" | "log" | "log10"
            | "log2",
        ) => Some(Type::Function {
            params: vec![Type::F64],
            ret: Box::new(Type::F64),
        }),
        ("tyrus:math", "atan2") => Some(Type::Function {
            params: vec![Type::F64, Type::F64],
            ret: Box::new(Type::F64),
        }),
        ("tyrus:math", "pow") => Some(Type::Function {
            params: vec![Type::F64, Type::F64],
            ret: Box::new(Type::F64),
        }),
        ("tyrus:math", "max" | "min") => Some(Type::Function {
            params: vec![Type::F64, Type::F64],
            ret: Box::new(Type::F64),
        }),
        ("tyrus:math", "imul") => Some(Type::Function {
            params: vec![Type::I32, Type::I32],
            ret: Box::new(Type::I32),
        }),
        ("tyrus:math", "clz32") => Some(Type::Function {
            params: vec![Type::I32],
            ret: Box::new(Type::I32),
        }),
        ("tyrus:algorithm", "suffixArray") => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Array(Box::new(Type::Usize))),
        }),
        ("tyrus:algorithm", "suffixArrayInfo") => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Named("SuffixArrayInfo".to_string())),
        }),
        ("tyrus:algorithm", "SuffixArrayInfo") => Some(Type::Named("SuffixArrayInfo".to_string())),
        ("tyrus:algorithm", "suffixTreeUkkonen") => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Named("SuffixTree".to_string())),
        }),
        ("tyrus:algorithm", "SuffixTree") => Some(Type::Named("SuffixTree".to_string())),
        ("tyrus:algorithm", "lcaBinaryLifting") => Some(Type::Function {
            params: vec![
                Type::Array(Box::new(Type::Array(Box::new(Type::Usize)))),
                Type::Usize,
            ],
            ret: Box::new(Type::Named("Lca".to_string())),
        }),
        ("tyrus:algorithm", "Lca") => Some(Type::Named("Lca".to_string())),
        ("tyrus:algorithm", "heavyLightDecomposition") => Some(Type::Function {
            params: vec![
                Type::Array(Box::new(Type::Array(Box::new(Type::Usize)))),
                Type::Usize,
            ],
            ret: Box::new(Type::Named("Hld".to_string())),
        }),
        ("tyrus:algorithm", "Hld") => Some(Type::Named("Hld".to_string())),
        ("tyrus:algorithm", "centroidDecomposition") => Some(Type::Function {
            params: vec![Type::Array(Box::new(Type::Array(Box::new(Type::Usize))))],
            ret: Box::new(Type::Named("Centroid".to_string())),
        }),
        ("tyrus:algorithm", "Centroid") => Some(Type::Named("Centroid".to_string())),
        _ => tyrus_core::module_loader::native_export_info(module, name).map(|(ty, _, _)| ty),
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn native_import_detail(module: &str, name: &str) -> Option<&'static str> {
    match (module, name) {
        ("tyrus:simd", "simdAdd" | "simdSub" | "simdMul" | "simdDiv") => {
            Some("<T>(left: T[], right: T[]) => Result<T[], SimdError>")
        }
        ("tyrus:fs", "readFileSync") => Some("(path: string, encoding: string) => string"),
        ("tyrus:fs", "writeFileSync") => {
            Some("(path: string, data: string, encoding: string) => void")
        }
        ("tyrus:http", "createRouter") => Some("() => HttpRouter"),
        ("tyrus:http", "HttpStatus") => Some("enum HttpStatus"),
        ("tyrus:http", "HttpReply") => Some("type HttpReply<T>"),
        ("tyrus:http", "HttpRouter") => Some("type HttpRouter"),
        ("tyrus:http", "HttpRequest") => Some("interface HttpRequest"),
        ("tyrus:http", "HttpResponse") => Some("interface HttpResponse"),
        ("tyrus:http", "HttpCacheOptions") => Some("interface HttpCacheOptions"),
        ("tyrus:http", "HttpReplyOptions") => Some("interface HttpReplyOptions"),
        ("tyrus:http", "HttpError") => Some("interface HttpError"),
        ("tyrus:http", "HttpServer") => Some("type HttpServer"),
        ("tyrus:http", "WebSocket") => Some("type WebSocket"),
        ("tyrus:http", "WebSocketMessage") => Some(
            "interface WebSocketMessage { readonly binary: boolean, readonly text: string, readonly bytes: u8[] }",
        ),
        ("tyrus:http", "connectWebSocket") => {
            Some("(url: string, protocols?: string[]) => Promise<Result<WebSocket, HttpError>>")
        }
        ("tyrus:http", "ServerOptions") => Some("interface ServerOptions"),
        ("tyrus:http", "RateLimit") => Some("interface RateLimit"),
        ("tyrus:http", "TlsOptions") => Some("interface TlsOptions"),
        ("tyrus:http", "TextBody") => Some("type TextBody"),
        ("tyrus:http", "BytesBody") => Some("type BytesBody"),
        ("tyrus:http", "StreamBody") => Some("type StreamBody"),
        ("tyrus:http", "MultipartBody") => Some("type MultipartBody"),
        ("tyrus:http", "fetch") => Some(
            "(url: string, options?: FetchOptions) => Promise<HttpClientResponse>; streaming: fetch.stream(url, options?) => Promise<HttpClientStreamResponse>; verbs: fetch.get, fetch.post, fetch.put, fetch.patch, fetch.delete, fetch.head, fetch.options, fetch.connect, fetch.query, fetch.trace",
        ),
        ("tyrus:http", "HttpClientResponse") => Some("type HttpClientResponse"),
        ("tyrus:http", "HttpClientStreamResponse") => Some("type HttpClientStreamResponse"),
        ("tyrus:http", "HttpHeader") => {
            Some("interface HttpHeader { name: string, value: string }")
        }
        ("tyrus:http", "FetchOptions") => Some(
            "interface FetchOptions { headers?: HttpHeader[], body?: string, bodyStream?: ReadableStream<u8[]>, timeout?: string, maxBodySize?: string, redirects?: i32, decompress?: boolean }",
        ),
        ("tyrus:http", "MultipartPart") => Some(
            "interface MultipartPart { name: string, fileName: string?, contentType: string?, bytes: u8[] }",
        ),
        ("tyrus:http", "CorsOptions") => Some("interface CorsOptions"),
        ("tyrus:http", "CsrfOptions") => {
            Some("interface CsrfOptions { cookie: string, header: string }")
        }
        ("tyrus:http", "HttpAccessLog") => Some(
            "interface HttpAccessLog { readonly requestId: string, readonly method: string, readonly path: string, readonly status: HttpStatus, readonly duration: Duration }",
        ),
        ("tyrus:http", "FetchPolicyOptions") => Some("interface FetchPolicyOptions"),
        ("tyrus:http", "CookieOptions") => Some("interface CookieOptions"),
        ("tyrus:http", "HttpCookie") => Some("interface HttpCookie"),
        ("tyrus:stream", "ReadableStream") => Some("type ReadableStream<T>"),
        ("tyrus:stream", "WritableStream") => Some("type WritableStream<T>"),
        ("tyrus:stream", "DuplexStream") => Some("type DuplexStream<TRead, TWrite>"),
        ("tyrus:stream", "TransformStream") => Some("type TransformStream<TInput, TOutput>"),
        ("tyrus:stream", "StreamError") => {
            Some("interface StreamError { readonly code: string, readonly message: string }")
        }
        ("tyrus:process", "exec") => {
            Some("(command: string, args?: string[]) => Result<ChildProcess, ProcessError>")
        }
        ("tyrus:process", "ChildProcess") => Some("type ChildProcess"),
        ("tyrus:process", "ProcessError") => {
            Some("interface ProcessError { readonly code: string, readonly message: string }")
        }
        ("tyrus:net", "udpBind") => {
            Some("(address: string) => Promise<Result<UdpSocket, NetError>>")
        }
        ("tyrus:net", "UdpSocket") => Some("type UdpSocket"),
        ("tyrus:net", "UdpPacket") => Some("type UdpPacket { data: u8[], address: string }"),
        ("tyrus:net", "ping") => {
            Some("(address: string, timeoutMs?: u64) => Promise<Result<u64, NetError>>")
        }
        ("tyrus:net", "traceroute") => {
            Some("(address: string, timeoutMs?: u64) => Promise<Result<string[], NetError>>")
        }
        ("tyrus:net", "supportsIcmp") => Some("() => boolean"),
        ("tyrus:net", "tcpListen") => {
            Some("(address: string, timeoutMs?: u64) => Promise<Result<TcpListener, NetError>>")
        }
        ("tyrus:net", "TcpListener") => Some("type TcpListener"),
        ("tyrus:net", "tcpConnect") => {
            Some("(address: string, timeoutMs?: u64) => Promise<Result<TcpStream, NetError>>")
        }
        ("tyrus:net", "TcpStream") => Some("type TcpStream"),
        ("tyrus:net", "resolve") => {
            Some("(host: string, timeoutMs?: u64) => Promise<Result<string[], NetError>>")
        }
        ("tyrus:net", "reverseLookup") => {
            Some("(address: string, timeoutMs?: u64) => Promise<Result<string, NetError>>")
        }
        ("tyrus:net", "parseIpAddress") => Some("(value: string) => IpAddress?"),
        ("tyrus:net", "parseSocketAddress") => Some("(value: string) => SocketAddress?"),
        ("tyrus:net", "NetError") => Some("type NetError { code: string, message: string }"),
        ("tyrus:archive", "pack") => Some(
            "(source: string, destination: string, options?: PackOptions) => Promise<Result<ArchiveSummary, ArchiveError>>",
        ),
        ("tyrus:archive", "list") => Some(
            "(archive: string, options?: UnpackOptions) => Promise<Result<ArchiveEntry[], ArchiveError>>",
        ),
        ("tyrus:archive", "unpack") => Some(
            "(archive: string, destination: string, options?: UnpackOptions) => Promise<Result<ArchiveSummary, ArchiveError>>",
        ),
        ("tyrus:archive", "packStream") => Some(
            "(source: string, format: ArchiveFormat, options?: PackOptions) => ReadableStream<u8[]>",
        ),
        ("tyrus:archive", "unpackStream") => Some(
            "(archive: ReadableStream<u8[]>, destination: string, options?: UnpackOptions) => Promise<Result<ArchiveSummary, ArchiveError>>",
        ),
        ("tyrus:archive", "ArchiveFormat") => Some("enum ArchiveFormat { Zip, Tar, TarGz }"),
        ("tyrus:archive", "ArchiveEntryKind") => {
            Some("enum ArchiveEntryKind { File, Directory, Symlink, HardLink, Other }")
        }
        ("tyrus:archive", "ArchiveEntry") => Some(
            "interface ArchiveEntry { readonly name: string, readonly kind: ArchiveEntryKind, readonly size: u64, readonly compressedSize: u64? }",
        ),
        ("tyrus:archive", "ArchiveSummary") => {
            Some("interface ArchiveSummary { readonly entries: u64, readonly bytes: u64 }")
        }
        ("tyrus:archive", "ArchiveError") => Some(
            "interface ArchiveError { readonly code: string, readonly message: string, readonly entry: string? }",
        ),
        ("tyrus:archive", "PackOptions") => Some(
            "interface PackOptions { level?: i32, overwrite?: boolean, exclude?: string[], prefix?: string, reproducible?: boolean, timeoutMs?: u64 }",
        ),
        ("tyrus:archive", "UnpackOptions") => Some(
            "interface UnpackOptions { overwrite?: boolean, stripComponents?: u32, maxEntries?: u64, maxBytes?: u64, maxRatio?: u32, timeoutMs?: u64 }",
        ),
        ("tyrus:compress", "compress") => Some(
            "(data: u8[] | string, format: CompressFormat, options?: CompressOptions) => Promise<Result<u8[], CompressError>>",
        ),
        ("tyrus:compress", "decompress") => Some(
            "(data: u8[], options?: DecompressOptions) => Promise<Result<u8[], CompressError>>",
        ),
        ("tyrus:compress", "compressionStream") => Some(
            "(format: CompressFormat, options?: CompressOptions) => TransformStream<u8[], u8[]>",
        ),
        ("tyrus:compress", "decompressionStream") => {
            Some("(options?: DecompressOptions) => TransformStream<u8[], u8[]>")
        }
        ("tyrus:compress", "CompressFormat") => {
            Some("enum CompressFormat { Gzip, Deflate, DeflateRaw, Brotli }")
        }
        ("tyrus:compress", "CompressError") => {
            Some("interface CompressError { readonly code: string, readonly message: string }")
        }
        ("tyrus:compress", "CompressOptions") => Some("interface CompressOptions { level?: i32 }"),
        ("tyrus:compress", "DecompressOptions") => Some(
            "interface DecompressOptions { format?: CompressFormat, maxOutputBytes?: u64, maxRatio?: u32 }",
        ),
        ("tyrus:net", "IpAddress") => Some("type IpAddress"),
        ("tyrus:net", "SocketAddress") => Some("type SocketAddress"),
        ("tyrus:utils", "styleText") => {
            Some("styleText(format, text) | styleText.<style>...(text), ANSI text styling")
        }
        ("tyrus:utils", "figlet") => {
            Some("(text: string, font?: \"standard\" | \"small\" | \"big\" | \"slant\") => string")
        }
        ("tyrus:cli", "input") => Some("(options: InputOptions) => string"),
        ("tyrus:cli", "password") => Some("(options: PasswordOptions) => string"),
        ("tyrus:cli", "confirm") => Some("(options: ConfirmOptions) => boolean"),
        ("tyrus:cli", "select") => Some("(options: SelectOptions) => string"),
        ("tyrus:cli", "multiselect") => Some("(options: MultiSelectOptions) => string[]"),
        ("tyrus:cli", "number") => Some("(options: NumberOptions) => f64"),
        ("tyrus:cli", "integer") => Some("(options: IntegerOptions) => i64"),
        ("tyrus:cli", "isTTY") => {
            Some("() => boolean - true when stdout is an interactive terminal")
        }
        ("tyrus:task", "cancel") => Some("<T>(promise: Promise<T>, reason: string) => boolean"),
        ("tyrus:task", "sleep") => Some("(ms: u64) => void"),
        ("tyrus:task", "sleepAsync") => Some("(ms: u64) => Promise<void>"),
        ("tyrus:task", "spawn") => Some("<T>(task: () => Promise<T>) => JoinHandle<T>"),
        ("tyrus:task", "channel") => Some("<T>(capacity?: usize) => [Sender<T>, Receiver<T>]"),
        ("tyrus:task", "Mutex") => Some("<T>(value: T) => Mutex<T>"),
        ("tyrus:task", "RwLock") => Some("<T>(value: T) => RwLock<T>"),
        ("tyrus:task", "Semaphore") => Some("(permits: usize) => Semaphore"),
        ("tyrus:task", "joinAll") => Some("<T>(handles: JoinHandle<T>[]) => Promise<T[]>"),
        ("tyrus:task", "race") => Some("<T>(handles: JoinHandle<T>[]) => Promise<T>"),
        ("tyrus:task", "timeout") => Some("<T>(ms: u64, task: Promise<T>) => Promise<T>"),
        ("tyrus:task", "JoinHandle") => Some("type JoinHandle<T>"),
        ("tyrus:task", "Sender") => Some("type Sender<T>"),
        ("tyrus:task", "Receiver") => Some("type Receiver<T>"),
        ("tyrus:os", "EOL") => Some("string"),
        ("tyrus:os", "platform") => Some("() => OsPlatform"),
        ("tyrus:os", "OsPlatform") => Some("enum OsPlatform"),
        ("tyrus:os", "arch") => Some("() => OsArch"),
        ("tyrus:os", "OsArch") => Some("enum OsArch"),
        ("tyrus:os", "hostname") => Some("() => string"),
        ("tyrus:os", "release") => Some("() => string"),
        ("tyrus:os", "systemName") => Some("() => string"),
        ("tyrus:os", "locale") => Some("() => string"),
        ("tyrus:os", "language") => Some("() => string"),
        ("tyrus:os", "userInfo") => Some("() => UserInfo"),
        ("tyrus:os", "userName") => Some("() => string"),
        ("tyrus:os", "homeDir") => Some("() => string"),
        ("tyrus:os", "cacheDir") => Some("() => string"),
        ("tyrus:os", "configDir") => Some("() => string"),
        ("tyrus:os", "dataDir") => Some("() => string"),
        ("tyrus:os", "tempDir") => Some("() => string"),
        ("tyrus:os", "whoami") => Some("() => string?"),
        ("tyrus:os", "desktopDir") => Some("() => string?"),
        ("tyrus:os", "documentsDir") => Some("() => string?"),
        ("tyrus:os", "appDataDir") => Some("(app: string) => string"),
        ("tyrus:os", "cpus") => Some("() => CpuInfo[]"),
        ("tyrus:os", "networkInterfaces") => Some("() => NetworkInterface[]"),
        ("tyrus:os", "totalMem") => Some("() => u64"),
        ("tyrus:os", "freeMem") => Some("() => u64"),
        ("tyrus:os", "uptime") => Some("() => u64"),
        ("tyrus:os", "loadAvg") => Some("() => f64[]"),
        ("tyrus:os", "battery") => Some("() => BatteryInfo?"),
        ("tyrus:os", "cpuTemp") => Some("(unit?: \"celsius\" | \"fahrenheit\") => f32?"),
        ("tyrus:os", "notify") => {
            Some("(title: string, body: string, options?: NotifyOptions) => void")
        }
        ("tyrus:os", "NotifyUrgency") => Some("enum NotifyUrgency"),
        ("tyrus:os", "requestAttention") => Some("(critical: boolean) => void"),
        ("tyrus:os", "revealInFolder") => Some("(path: string) => void"),
        ("tyrus:os", "openUrl") => Some("(url: string) => Result<void, OsError>"),
        ("tyrus:os", "trash") => Some("(path: string) => Result<void, OsError>"),
        ("tyrus:os", "clipboardWrite") => Some("(text: string) => void"),
        ("tyrus:os", "clipboardRead") => Some("() => string?"),
        ("tyrus:os", "isDarkMode") => Some("() => boolean"),
        ("tyrus:os", "terminalSize") => Some("() => [u16, u16]?"),
        ("tyrus:os", "isScreenLocked") => Some("() => boolean"),
        ("tyrus:os", "isSessionLocked") => Some("() => boolean"),
        ("tyrus:os", "preventSleep") => Some("() => Result<void, OsError>"),
        ("tyrus:os", "allowSleep") => Some("() => Result<void, OsError>"),
        ("tyrus:os", "FontInfo") => Some("{ name: string, style: string, path: string }"),
        ("tyrus:os", "systemFonts") => Some("() => FontInfo[]"),
        ("tyrus:os", "Color") => Some("{ r: u8, g: u8, b: u8 }"),
        ("tyrus:os", "accessibleColorContrast") => Some("(fg: Color, bg: Color) => boolean"),
        ("tyrus:os", "SystemCapability") => Some("enum SystemCapability"),
        ("tyrus:os", "hasCapability") => Some("(cap: SystemCapability) => boolean"),
        ("tyrus:temporal", "TimeZone") => Some("enum TimeZone"),
        ("tyrus:temporal", "TemporalError") => Some("{ code: string, message: string }"),
        ("tyrus:temporal", "Instant") => Some("{ seconds: i64, nanoseconds: u32 }"),
        ("tyrus:temporal", "ZonedDateTime") => {
            Some("{ seconds: i64, nanoseconds: u32, timeZone: TimeZone }")
        }
        ("tyrus:temporal", "PlainDate") => Some("{ year: i32, month: u32, day: u32 }"),
        ("tyrus:temporal", "PlainTime") => {
            Some("{ hour: u32, minute: u32, second: u32, nanosecond: u32 }")
        }
        ("tyrus:temporal", "Duration") => Some(
            "{ years: i64, months: i64, weeks: i64, days: i64, hours: i64, minutes: i64, seconds: i64, milliseconds: i64, microseconds: i64, nanoseconds: i64 }",
        ),
        ("tyrus:temporal", "PlainDateTime") => Some("{ date: PlainDate, time: PlainTime }"),
        ("tyrus:temporal", "PlainYearMonth") => Some("{ year: i32, month: u32 }"),
        ("tyrus:temporal", "PlainMonthDay") => Some("{ month: u32, day: u32 }"),
        ("tyrus:temporal", "timeZone") => Some("(name: string) => Result<TimeZone, TemporalError>"),
        ("tyrus:temporal", "instantNow") => Some("() => Instant"),
        ("tyrus:temporal", "zonedDateTimeFrom") => {
            Some("(value: string) => Result<ZonedDateTime, TemporalError>")
        }
        ("tyrus:temporal", "zonedDateTimeToInstant") => {
            Some("(value: ZonedDateTime) => Result<Instant, TemporalError>")
        }
        ("tyrus:temporal", "zonedDateTimeToString") => {
            Some("(value: ZonedDateTime) => Result<string, TemporalError>")
        }
        ("tyrus:temporal", "instantToZonedDateTime") => {
            Some("(instant: Instant, timeZone: TimeZone) => Result<ZonedDateTime, TemporalError>")
        }
        ("tyrus:temporal", "zonedDateTimeEquals") => {
            Some("(left: ZonedDateTime, right: ZonedDateTime) => boolean")
        }
        ("tyrus:temporal", "zonedDateTimeAddDuration") => Some(
            "(value: ZonedDateTime, duration: Duration) => Result<ZonedDateTime, TemporalError>",
        ),
        ("tyrus:temporal", "zonedDateTimeUntil") => {
            Some("(start: ZonedDateTime, end: ZonedDateTime) => Result<Duration, TemporalError>")
        }
        ("tyrus:temporal", "zonedDateTimeFormat") => {
            Some("(value: ZonedDateTime, pattern: string) => Result<string, TemporalError>")
        }
        ("tyrus:temporal", "instantFrom") => {
            Some("(value: string) => Result<Instant, TemporalError>")
        }
        ("tyrus:temporal", "instantFromEpochMilliseconds") => {
            Some("(milliseconds: i64) => Result<Instant, TemporalError>")
        }
        ("tyrus:temporal", "instantToEpochMilliseconds") => {
            Some("(instant: Instant) => Result<i64, TemporalError>")
        }
        ("tyrus:temporal", "instantToString") => {
            Some("(instant: Instant) => Result<string, TemporalError>")
        }
        ("tyrus:temporal", "instantAddDuration" | "instantSubtractDuration") => {
            Some("(instant: Instant, duration: Duration) => Result<Instant, TemporalError>")
        }
        ("tyrus:temporal", "plainDateFrom") => {
            Some("(value: string) => Result<PlainDate, TemporalError>")
        }
        ("tyrus:temporal", "plainDateFromFields") => {
            Some("(fields: PlainDate) => Result<PlainDate, TemporalError>")
        }
        ("tyrus:temporal", "plainDateWith") => {
            Some("(date: PlainDate, fields: PlainDate) => Result<PlainDate, TemporalError>")
        }
        ("tyrus:temporal", "plainDateSubtractDuration") => {
            Some("(date: PlainDate, duration: Duration) => Result<PlainDate, TemporalError>")
        }
        ("tyrus:temporal", "plainDateUntil" | "plainDateSince") => {
            Some("(start: PlainDate, end: PlainDate) => Result<Duration, TemporalError>")
        }
        ("tyrus:temporal", "plainDateUntilWithOptions" | "plainDateSinceWithOptions") => Some(
            "(start: PlainDate, end: PlainDate, largestUnit: string, smallestUnit: string, roundingIncrement: i64, roundingMode: string) => Result<Duration, TemporalError>",
        ),
        ("tyrus:temporal", "plainDateToString") => {
            Some("(date: PlainDate) => Result<string, TemporalError>")
        }
        ("tyrus:temporal", "plainDateEquals") => {
            Some("(left: PlainDate, right: PlainDate) => boolean")
        }
        ("tyrus:temporal", "plainDateAddDays") => {
            Some("(date: PlainDate, days: i64) => Result<PlainDate, TemporalError>")
        }
        ("tyrus:temporal", "plainDateDayOfWeek") => {
            Some("(date: PlainDate) => Result<u32, TemporalError>")
        }
        ("tyrus:temporal", "plainDateDayOfYear") => {
            Some("(date: PlainDate) => Result<u32, TemporalError>")
        }
        ("tyrus:temporal", "plainDateAddMonths") => {
            Some("(date: PlainDate, months: i64) => Result<PlainDate, TemporalError>")
        }
        ("tyrus:temporal", "plainDateAddYears") => {
            Some("(date: PlainDate, years: i64) => Result<PlainDate, TemporalError>")
        }
        ("tyrus:temporal", "plainDateDaysUntil") => {
            Some("(start: PlainDate, end: PlainDate) => Result<i64, TemporalError>")
        }
        ("tyrus:temporal", "plainTimeFrom") => {
            Some("(value: string) => Result<PlainTime, TemporalError>")
        }
        ("tyrus:temporal", "plainTimeFromFields") => {
            Some("(fields: PlainTime) => Result<PlainTime, TemporalError>")
        }
        ("tyrus:temporal", "plainTimeSubtractDuration") => {
            Some("(time: PlainTime, duration: Duration) => Result<PlainTime, TemporalError>")
        }
        ("tyrus:temporal", "plainTimeUntil" | "plainTimeSince") => {
            Some("(start: PlainTime, end: PlainTime) => Result<Duration, TemporalError>")
        }
        ("tyrus:temporal", "plainTimeUntilWithOptions" | "plainTimeSinceWithOptions") => Some(
            "(start: PlainTime, end: PlainTime, largestUnit: string, smallestUnit: string, roundingIncrement: i64, roundingMode: string) => Result<Duration, TemporalError>",
        ),
        ("tyrus:temporal", "plainTimeToString") => {
            Some("(time: PlainTime) => Result<string, TemporalError>")
        }
        ("tyrus:temporal", "plainTimeEquals") => {
            Some("(left: PlainTime, right: PlainTime) => boolean")
        }
        ("tyrus:temporal", "plainTimeAddSeconds") => {
            Some("(time: PlainTime, seconds: i64) => Result<PlainTime, TemporalError>")
        }
        ("tyrus:temporal", "plainTimeSecondsUntil") => {
            Some("(start: PlainTime, end: PlainTime) => Result<i64, TemporalError>")
        }
        ("tyrus:temporal", "plainTimeAddNanoseconds") => {
            Some("(time: PlainTime, nanoseconds: i64) => Result<PlainTime, TemporalError>")
        }
        ("tyrus:temporal", "plainTimeNanosecondsUntil") => {
            Some("(start: PlainTime, end: PlainTime) => Result<i64, TemporalError>")
        }
        ("tyrus:temporal", "durationFromFields") => Some("(fields: Duration) => Duration"),
        ("tyrus:temporal", "durationTotal") => {
            Some("(duration: Duration, unit: string) => Result<i64, TemporalError>")
        }
        ("tyrus:temporal", "durationRound") => Some(
            "(duration: Duration, unit: string, increment: i64, mode: string, relativeTo: PlainDate) => Result<Duration, TemporalError>",
        ),
        ("tyrus:temporal", "plainDateAddDuration") => {
            Some("(date: PlainDate, duration: Duration) => Result<PlainDate, TemporalError>")
        }
        ("tyrus:temporal", "plainTimeAddDuration") => {
            Some("(time: PlainTime, duration: Duration) => Result<PlainTime, TemporalError>")
        }
        ("tyrus:temporal", "plainDateTimeFrom") => {
            Some("(value: string) => Result<PlainDateTime, TemporalError>")
        }
        ("tyrus:temporal", "plainDateTimeFromFields") => {
            Some("(fields: PlainDateTime) => Result<PlainDateTime, TemporalError>")
        }
        ("tyrus:temporal", "plainDateTimeToString") => {
            Some("(value: PlainDateTime) => Result<string, TemporalError>")
        }
        ("tyrus:temporal", "plainDateTimeEquals") => {
            Some("(left: PlainDateTime, right: PlainDateTime) => boolean")
        }
        ("tyrus:temporal", "plainDateTimeWith") => Some(
            "(value: PlainDateTime, replacement: PlainDateTime) => Result<PlainDateTime, TemporalError>",
        ),
        ("tyrus:temporal", "plainDateTimeAddDuration" | "plainDateTimeSubtractDuration") => Some(
            "(value: PlainDateTime, duration: Duration) => Result<PlainDateTime, TemporalError>",
        ),
        ("tyrus:temporal", "plainDateTimeUntil" | "plainDateTimeSince") => {
            Some("(start: PlainDateTime, end: PlainDateTime) => Result<Duration, TemporalError>")
        }
        ("tyrus:temporal", "plainDateTimeUntilWithOptions" | "plainDateTimeSinceWithOptions") => {
            Some(
                "(start: PlainDateTime, end: PlainDateTime, largestUnit: string, smallestUnit: string, roundingIncrement: i64, roundingMode: string) => Result<Duration, TemporalError>",
            )
        }
        ("tyrus:temporal", "plainYearMonthFrom") => {
            Some("(value: string) => Result<PlainYearMonth, TemporalError>")
        }
        ("tyrus:temporal", "plainYearMonthFromFields") => {
            Some("(fields: PlainYearMonth) => Result<PlainYearMonth, TemporalError>")
        }
        ("tyrus:temporal", "plainYearMonthToString") => {
            Some("(value: PlainYearMonth) => Result<string, TemporalError>")
        }
        ("tyrus:temporal", "plainYearMonthAdd") => {
            Some("(value: PlainYearMonth, months: i64) => Result<PlainYearMonth, TemporalError>")
        }
        ("tyrus:temporal", "plainMonthDayFrom") => {
            Some("(value: string) => Result<PlainMonthDay, TemporalError>")
        }
        ("tyrus:temporal", "plainMonthDayFromFields") => {
            Some("(fields: PlainMonthDay) => Result<PlainMonthDay, TemporalError>")
        }
        ("tyrus:temporal", "plainMonthDayToString") => {
            Some("(value: PlainMonthDay) => Result<string, TemporalError>")
        }
        ("tyrus:temporal", "plainMonthDayToPlainDate") => {
            Some("(value: PlainMonthDay, year: i32) => Result<PlainDate, TemporalError>")
        }
        ("tyrus:crypto", "timingSafeEqual") => Some(
            "(a: string | u8[], b: string | u8[]) => boolean, constant-time for equal lengths; length is not concealed",
        ),
        ("tyrus:crypto", "digest") => {
            Some("namespace, create, sha256, sha384, sha512, md5 (legacy checksums only)")
        }
        ("tyrus:crypto", "hmac") => Some("namespace, sha256, sha384, sha512"),
        ("tyrus:crypto", "cipher") => Some(
            "namespace, versioned AES-256-GCM envelopes, 32-byte keys, and optional authenticated AAD",
        ),
        ("tyrus:crypto", "rand") => {
            Some("namespace, uuidv7, uuidv4, ulid, randomBytes, randomInt, randomFloat")
        }
        ("tyrus:crypto", "argon2") => Some(
            "namespace, bounded asynchronous Argon2id hash/verify and work-factor migration checks",
        ),
        ("tyrus:crypto", "jwt") => Some(
            "namespace, allowlisted HMAC, RSA, ECDSA, and EdDSA signing/verification plus hardened JWK/JWKS verification; decodeUnverified returns attacker-controlled claims",
        ),
        ("tyrus:encoding", "utf8") => {
            Some("namespace: UTF-8 byte encoding and strict or lossy decoding")
        }
        ("tyrus:encoding", "utf16") => Some(
            "namespace: UTF-16 code units and explicit little-endian or big-endian byte conversion",
        ),
        ("tyrus:encoding", "hex") => {
            Some("namespace: canonical lowercase hexadecimal encoding and strict decoding")
        }
        ("tyrus:encoding", "base64") => {
            Some("namespace: standard base64 encoding with RFC 4648 padding and strict decoding")
        }
        ("tyrus:encoding", "base64Url") => {
            Some("namespace: unpadded URL-safe base64 encoding (`-`/`_`) and strict decoding")
        }
        ("tyrus:encoding", "base32") => {
            Some("namespace: canonical uppercase base32 encoding with padding and strict decoding")
        }
        ("tyrus:encoding", "radix") => {
            Some("namespace: integer formatting and parsing in bases 2, 8, 10, and 16")
        }
        ("tyrus:encoding", "EncodingError") => Some("{ code: string, message: string }"),
        ("tyrus:math", "PI") => Some("f64, ratio of circumference to diameter"),
        ("tyrus:math", "E") => Some("f64, base of natural logarithms"),
        ("tyrus:math", "LN2") => Some("f64, natural logarithm of 2"),
        ("tyrus:math", "LN10") => Some("f64, natural logarithm of 10"),
        ("tyrus:math", "LOG2E") => Some("f64, base-2 logarithm of E"),
        ("tyrus:math", "LOG10E") => Some("f64, base-10 logarithm of E"),
        ("tyrus:math", "SQRT2") => Some("f64, square root of 2"),
        ("tyrus:math", "EPSILON") => {
            Some("f64, smallest difference between 1 and the next representable f64")
        }
        ("tyrus:math", "TAU") => Some("f64, two times PI (full circle in radians)"),
        ("tyrus:math", "PHI") => Some("f64, the golden ratio, (1 + sqrt(5)) / 2"),
        ("tyrus:math", "abs") => Some("(x: f64) => f64"),
        ("tyrus:math", "sign") => Some("(x: f64) => f64"),
        ("tyrus:math", "sqrt") => Some("(x: f64) => f64"),
        ("tyrus:math", "cbrt") => Some("(x: f64) => f64"),
        ("tyrus:math", "floor") => Some("(x: f64) => f64"),
        ("tyrus:math", "ceil") => Some("(x: f64) => f64"),
        ("tyrus:math", "trunc") => Some("(x: f64) => f64"),
        ("tyrus:math", "round") => Some("(x: f64) => f64"),
        ("tyrus:math", "sin") => Some("(x: f64) => f64"),
        ("tyrus:math", "cos") => Some("(x: f64) => f64"),
        ("tyrus:math", "tan") => Some("(x: f64) => f64"),
        ("tyrus:math", "asin") => Some("(x: f64) => f64"),
        ("tyrus:math", "acos") => Some("(x: f64) => f64"),
        ("tyrus:math", "atan") => Some("(x: f64) => f64"),
        ("tyrus:math", "atan2") => Some("(y: f64, x: f64) => f64"),
        ("tyrus:math", "sinh") => Some("(x: f64) => f64"),
        ("tyrus:math", "cosh") => Some("(x: f64) => f64"),
        ("tyrus:math", "tanh") => Some("(x: f64) => f64"),
        ("tyrus:math", "exp") => Some("(x: f64) => f64"),
        ("tyrus:math", "log") => Some("(x: f64) => f64"),
        ("tyrus:math", "log10") => Some("(x: f64) => f64"),
        ("tyrus:math", "log2") => Some("(x: f64) => f64"),
        ("tyrus:math", "pow") => Some("(base: f64, exp: f64) => f64"),
        ("tyrus:math", "max") => Some("(a: f64, b: f64) => f64"),
        ("tyrus:math", "min") => Some("(a: f64, b: f64) => f64"),
        ("tyrus:math", "imul") => Some("(a: i32, b: i32) => i32"),
        ("tyrus:math", "clz32") => Some("(x: i32) => i32"),
        ("tyrus:math", "approxEqual") => Some("(a: f64, b: f64, tolerance?: f64) => boolean"),
        ("tyrus:math", "clamp") => Some("(value: f64, min: f64, max: f64) => f64"),
        ("tyrus:math", "clampMin") => Some("(value: f64, min: f64) => f64"),
        ("tyrus:math", "clampMax") => Some("(value: f64, max: f64) => f64"),
        ("tyrus:math", "lerp") => Some("(start: f64, end: f64, amount: f64) => f64"),
        ("tyrus:math", "inverseLerp") => Some("(start: f64, end: f64, value: f64) => f64"),
        ("tyrus:math", "remap") => {
            Some("(value: f64, inMin: f64, inMax: f64, outMin: f64, outMax: f64) => f64")
        }
        ("tyrus:math", "degToRad") => Some("(degrees: f64) => f64"),
        ("tyrus:math", "radToDeg") => Some("(radians: f64) => f64"),
        ("tyrus:math", "gcd") => Some("(a: u32, b: u32) => u32"),
        ("tyrus:math", "lcm") => Some("(a: u32, b: u32) => u32"),
        ("tyrus:math", "randomRange") => Some("(min: f64, max: f64) => f64"),
        ("tyrus:math", "smoothstep") => Some("(edge0: f64, edge1: f64, x: f64) => f64"),
        ("tyrus:math", "roundTo") => Some("(x: f64, decimals: i32) => f64"),
        ("tyrus:math", "mean") => Some("(values: f64[]) => f64"),
        ("tyrus:math", "median") => Some("(values: f64[]) => f64"),
        ("tyrus:math", "mode") => Some("(values: f64[]) => f64"),
        ("tyrus:math", "variance") => Some("(values: f64[]) => f64"),
        ("tyrus:math", "stdDev") => Some("(values: f64[]) => f64"),
        ("tyrus:math", "percentile") => Some("(p: f64, values: f64[]) => f64"),
        ("tyrus:math", "factorial") => Some("(n: u128) => u128"),
        ("tyrus:math", "permutation") => Some("(n: u128, k: u128) => u128"),
        ("tyrus:math", "combination") => Some("(n: u128, k: u128) => u128"),
        ("tyrus:math", "isPrime") => Some("(n: u64) => boolean"),
        ("tyrus:math", "nextPrime") => Some("(n: u64) => u64"),
        ("tyrus:math", "primeFactors") => Some("(n: u64) => u64[]"),
        ("tyrus:math", "fibonacci") => Some("(n: u64) => u64"),
        ("tyrus:math", "hypot") => Some("(x: f64, y: f64) => f64"),
        ("tyrus:math", "copysign") => Some("(x: f64, y: f64) => f64"),
        ("tyrus:math", "fmod") => Some("(x: f64, y: f64) => f64"),
        ("tyrus:math", "logBase") => Some("(x: f64, base: f64) => f64"),
        ("tyrus:math", "powi") => Some("(x: f64, exp: i32) => f64"),
        ("tyrus:math", "toPolar") => Some("(x: f64, y: f64) => [f64, f64]"),
        ("tyrus:math", "toCartesian") => Some("(r: f64, theta: f64) => [f64, f64]"),
        ("tyrus:algorithm", "suffixArray") => Some(
            "(text: string) => usize[], starting indices of every suffix sorted lexicographically (direct O(n² log n) construction)",
        ),
        ("tyrus:algorithm", "suffixArrayInfo") => Some(
            "(text: string) => SuffixArrayInfo, deterministic suffix sorting rounds and comparison metrics",
        ),
        ("tyrus:algorithm", "SuffixArrayInfo") => Some("interface SuffixArrayInfo"),
        ("tyrus:algorithm", "suffixTreeUkkonen") => Some(
            "(text: string) => SuffixTree, suffix tree over a string built with Ukkonen's linear-time algorithm (getSuffixArray)",
        ),
        ("tyrus:algorithm", "SuffixTree") => Some("type SuffixTree"),
        ("tyrus:algorithm", "lcaBinaryLifting") => Some(
            "(adj: usize[][], root: usize) => Lca, lowest common ancestor tables built with binary lifting (query)",
        ),
        ("tyrus:algorithm", "Lca") => Some("type Lca"),
        ("tyrus:algorithm", "heavyLightDecomposition") => Some(
            "(adj: usize[][], root: usize) => Hld, heavy-light decomposition over a tree (queryPath, position, head)",
        ),
        ("tyrus:algorithm", "Hld") => Some("type Hld"),
        ("tyrus:algorithm", "centroidDecomposition") => Some(
            "(adj: usize[][]) => Centroid, centroid decomposition over a tree (parentOf, depth)",
        ),
        ("tyrus:algorithm", "Centroid") => Some("type Centroid"),
        ("tyrus:algorithm", "bubbleSortInfo") => Some(
            "(values: i32[]) => BubbleSortInfo, sorted and original snapshots with deterministic Bubble Sort metrics",
        ),
        ("tyrus:algorithm", "binarySearchInfo") => {
            Some("(values: i32[], target: i32) => SearchInfo, binary search snapshot and metrics")
        }
        ("tyrus:algorithm", "interpolationSearchInfo") => Some(
            "(values: i32[], target: i32) => InterpolationSearchInfo, interpolation probes and estimates",
        ),
        ("tyrus:algorithm", "exponentialSearchInfo") => Some(
            "(values: i32[], target: i32) => ExponentialSearchInfo, exponential range and binary search metrics",
        ),
        ("tyrus:algorithm", "quickselectInfo") => Some(
            "(values: i32[], rank: usize) => SelectionInfo, quickselect snapshot and partition metrics",
        ),
        ("tyrus:algorithm", "SearchInfo") => Some("interface SearchInfo"),
        ("tyrus:algorithm", "InterpolationSearchInfo") => {
            Some("interface InterpolationSearchInfo extends SearchInfo")
        }
        ("tyrus:algorithm", "ExponentialSearchInfo") => {
            Some("interface ExponentialSearchInfo extends SearchInfo")
        }
        ("tyrus:algorithm", "SelectionInfo") => Some("interface SelectionInfo"),
        ("tyrus:algorithm", "bfsInfo") => Some(
            "(adj: usize[][] | Graph, start: usize) => TraversalInfo, breadth-first traversal metrics",
        ),
        ("tyrus:algorithm", "dfsInfo") => Some(
            "(adj: usize[][] | Graph, start: usize) => TraversalInfo, depth-first traversal metrics",
        ),
        ("tyrus:algorithm", "iddfsInfo") => Some(
            "(adj: usize[][] | Graph, start: usize, maxDepth: usize) => IddfsInfo, iterative-deepening traversal metrics",
        ),
        ("tyrus:algorithm", "TraversalInfo") => Some("interface TraversalInfo"),
        ("tyrus:algorithm", "IddfsInfo") => Some("interface IddfsInfo extends TraversalInfo"),
        ("tyrus:algorithm", "dijkstraInfo") => Some(
            "(adj: [usize, i32][][] | Graph, start: usize, end: usize) => DijkstraInfo, Dijkstra shortest-path metrics",
        ),
        ("tyrus:algorithm", "aStarInfo") => Some(
            "(adj: [usize, i32][][] | Graph, start: usize, end: usize, heuristic: i32[]) => AStarInfo, A* shortest-path metrics",
        ),
        ("tyrus:algorithm", "bellmanFordInfo") => Some(
            "(adj: [usize, i32][][] | Graph, start: usize) => BellmanFordInfo, Bellman-Ford passes, relaxations and negative-cycle evidence",
        ),
        ("tyrus:algorithm", "spfaInfo") => Some(
            "(adj: [usize, i32][][] | Graph, start: usize) => SpfaInfo, SPFA queue and shortest-path metrics",
        ),
        ("tyrus:algorithm", "johnsonInfo") => Some(
            "(adj: [usize, i32][][] | Graph) => JohnsonInfo, Johnson all-pairs distances and execution metrics",
        ),
        ("tyrus:algorithm", "floydWarshallInfo") => Some(
            "(matrix: i32?[][] | Graph) => FloydWarshallInfo, Floyd-Warshall all-pairs distances and update metrics",
        ),
        ("tyrus:algorithm", "ShortestPathInfo") => Some("interface ShortestPathInfo"),
        ("tyrus:algorithm", "DijkstraInfo") => {
            Some("interface DijkstraInfo extends ShortestPathInfo")
        }
        ("tyrus:algorithm", "AStarInfo") => Some("interface AStarInfo extends ShortestPathInfo"),
        ("tyrus:algorithm", "BellmanFordInfo") => {
            Some("interface BellmanFordInfo extends ShortestPathInfo")
        }
        ("tyrus:algorithm", "SpfaInfo") => Some("interface SpfaInfo extends ShortestPathInfo"),
        ("tyrus:algorithm", "JohnsonInfo") => Some("interface JohnsonInfo"),
        ("tyrus:algorithm", "FloydWarshallInfo") => Some("interface FloydWarshallInfo"),
        ("tyrus:algorithm", "kruskalInfo") => Some(
            "(edges: [usize, usize, i32][], vertexCount: usize) => KruskalInfo, Kruskal construction metrics",
        ),
        ("tyrus:algorithm", "primInfo") => Some(
            "(adjacency: [usize, i32][][], origin: usize) => PrimInfo, Prim construction metrics",
        ),
        ("tyrus:algorithm", "boruvkaInfo") => Some(
            "(edges: [usize, usize, i32][], vertexCount: usize) => BoruvkaInfo, Boruvka phase metrics",
        ),
        ("tyrus:algorithm", "steinerTreeInfo") => Some(
            "(edges: [usize, usize, i32][], vertexCount: usize, terminals: usize[]) => SteinerTreeInfo, Steiner tree result and problem limits",
        ),
        ("tyrus:algorithm", "SpanningTreeInfo") => Some("interface SpanningTreeInfo"),
        ("tyrus:algorithm", "KruskalInfo") => {
            Some("interface KruskalInfo extends SpanningTreeInfo")
        }
        ("tyrus:algorithm", "edmondsKarpInfo") => Some(
            "(capacity: i32[][], source: usize, sink: usize) => EdmondsKarpInfo, maximum-flow snapshot and BFS metrics",
        ),
        ("tyrus:algorithm", "dinicInfo") => Some(
            "(capacity: i32[][], source: usize, sink: usize) => DinicInfo, maximum-flow snapshot and level-graph metrics",
        ),
        ("tyrus:algorithm", "pushRelabelInfo") => Some(
            "(capacity: i32[][], source: usize, sink: usize) => PushRelabelInfo, maximum-flow snapshot and discharge metrics",
        ),
        ("tyrus:algorithm", "minCutInfo") => Some(
            "(capacity: i32[][], source: usize, sink: usize) => MinCutInfo, cut partitions and flow snapshot",
        ),
        ("tyrus:algorithm", "minCostMaxFlowInfo") => Some(
            "(edges: [usize, usize, i32, i32][], vertices: usize, source: usize, sink: usize) => MinCostMaxFlowInfo, flow and cost metrics",
        ),
        ("tyrus:algorithm", "FlowInfo") => Some("interface FlowInfo"),
        ("tyrus:algorithm", "EdmondsKarpInfo") => {
            Some("interface EdmondsKarpInfo extends FlowInfo")
        }
        ("tyrus:algorithm", "DinicInfo") => Some("interface DinicInfo extends FlowInfo"),
        ("tyrus:algorithm", "PushRelabelInfo") => {
            Some("interface PushRelabelInfo extends FlowInfo")
        }
        ("tyrus:algorithm", "MinCutInfo") => Some("interface MinCutInfo"),
        ("tyrus:algorithm", "MinCostMaxFlowInfo") => Some("interface MinCostMaxFlowInfo"),
        ("tyrus:algorithm", "fleuryEulerianInfo") => Some(
            "(adj: usize[][] | Graph) => FleuryEulerianInfo, Eulerian path and bridge-choice metrics",
        ),
        ("tyrus:algorithm", "hierholzerEulerianInfo") => Some(
            "(adj: usize[][] | Graph) => HierholzerEulerianInfo, Eulerian path and stack metrics",
        ),
        ("tyrus:algorithm", "EulerianInfo") => Some("interface EulerianInfo"),
        ("tyrus:algorithm", "FleuryEulerianInfo") => {
            Some("interface FleuryEulerianInfo extends EulerianInfo")
        }
        ("tyrus:algorithm", "HierholzerEulerianInfo") => {
            Some("interface HierholzerEulerianInfo extends EulerianInfo")
        }
        ("tyrus:algorithm", "PrimInfo") => Some("interface PrimInfo extends SpanningTreeInfo"),
        ("tyrus:algorithm", "BoruvkaInfo") => {
            Some("interface BoruvkaInfo extends SpanningTreeInfo")
        }
        ("tyrus:algorithm", "SteinerTreeInfo") => Some("interface SteinerTreeInfo"),
        ("tyrus:algorithm", "SortInfo") => Some("interface SortInfo"),
        ("tyrus:algorithm", "BubbleSortInfo") => Some("interface BubbleSortInfo extends SortInfo"),
        ("tyrus:algorithm", "findBridgesInfo") => {
            Some("(adj: usize[][] | Graph) => FindBridgesInfo, bridge result and low-link metrics")
        }
        ("tyrus:algorithm", "findCutVerticesInfo") => Some(
            "(adj: usize[][] | Graph) => FindCutVerticesInfo, articulation result and low-link metrics",
        ),
        ("tyrus:algorithm", "biconnectedComponentsInfo") => Some(
            "(adj: usize[][] | Graph) => BiconnectedComponentsInfo, component result and low-link metrics",
        ),
        ("tyrus:algorithm", "ConnectivityInfo") => Some("interface ConnectivityInfo"),
        ("tyrus:algorithm", "FindBridgesInfo") => {
            Some("interface FindBridgesInfo extends ConnectivityInfo")
        }
        ("tyrus:algorithm", "FindCutVerticesInfo") => {
            Some("interface FindCutVerticesInfo extends ConnectivityInfo")
        }
        ("tyrus:algorithm", "BiconnectedComponentsInfo") => {
            Some("interface BiconnectedComponentsInfo extends ConnectivityInfo")
        }
        ("tyrus:algorithm", "edmondsBlossomInfo") => Some(
            "(adj: usize[][] | Graph) => EdmondsBlossomInfo, matching and augmenting-path metrics",
        ),
        ("tyrus:algorithm", "hungarianInfo") => {
            Some("(costMatrix: i32[][]) => HungarianInfo, assignment and matrix metrics")
        }
        ("tyrus:algorithm", "MatchingInfo") => Some("interface MatchingInfo"),
        ("tyrus:algorithm", "EdmondsBlossomInfo") => {
            Some("interface EdmondsBlossomInfo extends MatchingInfo")
        }
        ("tyrus:algorithm", "HungarianInfo") => Some("interface HungarianInfo"),
        ("tyrus:algorithm", "kmpSearchInfo") => {
            Some("(text: string, pattern: string) => KmpSearchInfo, prefix-function search metrics")
        }
        ("tyrus:algorithm", "StringSearchInfo") => Some("interface StringSearchInfo"),
        ("tyrus:algorithm", "KmpSearchInfo") => {
            Some("interface KmpSearchInfo extends StringSearchInfo")
        }
        ("tyrus:algorithm", "zAlgorithmInfo") => {
            Some("(input: string) => ZAlgorithmInfo, Z-table construction metrics")
        }
        ("tyrus:algorithm", "ZAlgorithmInfo") => Some("interface ZAlgorithmInfo"),
        ("tyrus:algorithm", "rabinKarpInfo") => {
            Some("(text: string, pattern: string) => RabinKarpInfo, rolling-hash search metrics")
        }
        ("tyrus:algorithm", "RabinKarpInfo") => {
            Some("interface RabinKarpInfo extends StringSearchInfo")
        }
        ("tyrus:algorithm", "boyerMooreInfo") => {
            Some("(text: string, pattern: string) => BoyerMooreInfo, bad-character search metrics")
        }
        ("tyrus:algorithm", "BoyerMooreInfo") => {
            Some("interface BoyerMooreInfo extends StringSearchInfo")
        }
        ("tyrus:algorithm", "ahoCorasickInfo") => Some(
            "(text: string, patterns: string[]) => AhoCorasickInfo, multi-pattern automaton metrics",
        ),
        ("tyrus:algorithm", "AhoCorasickInfo") => Some("interface AhoCorasickInfo"),
        ("tyrus:algorithm", "levenshteinInfo") => {
            Some("(left: string, right: string) => EditDistanceInfo, edit-distance metrics")
        }
        ("tyrus:algorithm", "EditDistanceInfo") => Some("interface EditDistanceInfo"),
        _ => None,
    }
}

pub(super) fn native_parameter_names(module: &str, name: &str) -> Option<Vec<String>> {
    let parameters = match (module, name) {
        ("tyrus:simd", "simdAdd" | "simdSub" | "simdMul" | "simdDiv") => &["left", "right"][..],
        ("tyrus:fs", "readFileSync") => &["path", "encoding"][..],
        ("tyrus:fs", "writeFileSync") => &["path", "data", "encoding"][..],
        ("tyrus:http", "createRouter") => &[][..],
        ("tyrus:http", "HttpStatus") => &[][..],
        ("tyrus:http", "HttpReply") => &[][..],
        ("tyrus:http", "HttpRouter") => &[][..],
        (
            "tyrus:http",
            "HttpRequest"
            | "HttpResponse"
            | "CorsOptions"
            | "CsrfOptions"
            | "HttpAccessLog"
            | "CookieOptions"
            | "HttpCookie"
            | "HttpCacheOptions"
            | "HttpReplyOptions"
            | "ServerOptions"
            | "RateLimit"
            | "TlsOptions"
            | "TextBody"
            | "BytesBody"
            | "StreamBody"
            | "MultipartBody"
            | "MultipartPart"
            | "HttpError"
            | "HttpServer"
            | "WebSocket"
            | "WebSocketMessage"
            | "HttpClientResponse"
            | "HttpClientStreamResponse"
            | "FetchOptions"
            | "FetchPolicyOptions"
            | "HttpHeader",
        ) => &[][..],
        ("tyrus:http", "fetch") => &["url", "options"][..],
        ("tyrus:http", "connectWebSocket") => &["url", "protocols"][..],
        (
            "tyrus:stream",
            "ReadableStream" | "WritableStream" | "DuplexStream" | "TransformStream",
        ) => &[][..],
        ("tyrus:stream", "StreamError") => &[][..],
        ("tyrus:process", "exec") => &["command", "args"][..],
        ("tyrus:process", "ChildProcess" | "ProcessError") => &[][..],
        ("tyrus:net", "udpBind") => &["address"][..],
        ("tyrus:net", "ping" | "traceroute") => &["address", "timeoutMs"][..],
        ("tyrus:net", "supportsIcmp") => &[][..],
        ("tyrus:net", "tcpListen" | "tcpConnect") => &["address", "timeoutMs"][..],
        ("tyrus:net", "resolve") => &["host", "timeoutMs"][..],
        ("tyrus:archive", "pack") => &["source", "destination", "options"][..],
        ("tyrus:archive", "list") => &["archive", "options"][..],
        ("tyrus:archive", "unpack") => &["archive", "destination", "options"][..],
        ("tyrus:archive", "packStream") => &["source", "format", "options"][..],
        ("tyrus:archive", "unpackStream") => &["archive", "destination", "options"][..],
        (
            "tyrus:archive",
            "ArchiveEntryKind" | "ArchiveFormat" | "ArchiveEntry" | "ArchiveSummary"
            | "ArchiveError" | "PackOptions" | "UnpackOptions",
        ) => &[][..],
        ("tyrus:compress", "compress") => &["data", "format", "options"][..],
        ("tyrus:compress", "decompress") => &["data", "options"][..],
        ("tyrus:compress", "compressionStream") => &["format", "options"][..],
        ("tyrus:compress", "decompressionStream") => &["options"][..],
        (
            "tyrus:compress",
            "CompressFormat" | "CompressError" | "CompressOptions" | "DecompressOptions",
        ) => &[][..],
        ("tyrus:net", "reverseLookup") => &["address", "timeoutMs"][..],
        ("tyrus:net", "parseIpAddress" | "parseSocketAddress") => &["value"][..],
        (
            "tyrus:net",
            "UdpSocket" | "UdpPacket" | "TcpListener" | "TcpStream" | "NetError" | "IpAddress"
            | "SocketAddress",
        ) => &[][..],
        ("tyrus:utils", "styleText") => &[][..],
        ("tyrus:utils", "figlet") => &["text", "font"][..],
        (
            "tyrus:cli",
            "input" | "password" | "confirm" | "select" | "multiselect" | "number" | "integer",
        ) => &["options"][..],
        ("tyrus:cli", "isTTY") => &[][..],
        ("tyrus:task", "cancel") => &["promise", "reason"][..],
        ("tyrus:task", "sleep") => &["ms"][..],
        ("tyrus:task", "sleepAsync") => &["ms"][..],
        ("tyrus:task", "spawn") => &["task"][..],
        ("tyrus:task", "JoinHandle" | "Sender" | "Receiver") => &[][..],
        ("tyrus:os", "EOL") => &[][..],
        ("tyrus:os", "platform") => &[][..],
        ("tyrus:os", "OsPlatform") => &[][..],
        ("tyrus:os", "arch") => &[][..],
        ("tyrus:os", "OsArch") => &[][..],
        ("tyrus:os", "hostname") => &[][..],
        ("tyrus:os", "release") => &[][..],
        ("tyrus:os", "systemName") => &[][..],
        ("tyrus:os", "locale") => &[][..],
        ("tyrus:os", "language") => &[][..],
        ("tyrus:os", "userInfo") => &[][..],
        ("tyrus:os", "userName") => &[][..],
        ("tyrus:os", "homeDir") => &[][..],
        ("tyrus:os", "cacheDir") => &[][..],
        ("tyrus:os", "configDir") => &[][..],
        ("tyrus:os", "dataDir") => &[][..],
        ("tyrus:os", "tempDir") => &[][..],
        ("tyrus:os", "whoami") => &[][..],
        ("tyrus:os", "desktopDir") => &[][..],
        ("tyrus:os", "documentsDir") => &[][..],
        ("tyrus:os", "appDataDir") => &["app"][..],
        ("tyrus:os", "cpus") => &[][..],
        ("tyrus:os", "networkInterfaces") => &[][..],
        ("tyrus:os", "totalMem") => &[][..],
        ("tyrus:os", "freeMem") => &[][..],
        ("tyrus:os", "uptime") => &[][..],
        ("tyrus:os", "loadAvg") => &[][..],
        ("tyrus:os", "battery") => &[][..],
        ("tyrus:os", "cpuTemp") => &["unit"][..],
        ("tyrus:os", "NotifyUrgency") => &[][..],
        ("tyrus:os", "notify") => &["title", "body", "options"][..],
        ("tyrus:os", "requestAttention") => &["critical"][..],
        ("tyrus:os", "revealInFolder") => &["path"][..],
        ("tyrus:os", "openUrl") => &["url"][..],
        ("tyrus:os", "trash") => &["path"][..],
        ("tyrus:os", "clipboardWrite") => &["text"][..],
        ("tyrus:os", "clipboardRead") => &[][..],
        ("tyrus:os", "isDarkMode") => &[][..],
        ("tyrus:os", "terminalSize") => &[][..],
        ("tyrus:os", "isScreenLocked") => &[][..],
        ("tyrus:os", "isSessionLocked") => &[][..],
        ("tyrus:os", "preventSleep") => &[][..],
        ("tyrus:os", "allowSleep") => &[][..],
        ("tyrus:os", "FontInfo") => &[][..],
        ("tyrus:os", "systemFonts") => &[][..],
        ("tyrus:os", "Color") => &[][..],
        ("tyrus:os", "accessibleColorContrast") => &["fg", "bg"][..],
        ("tyrus:os", "SystemCapability") => &[][..],
        ("tyrus:os", "hasCapability") => &["cap"][..],
        ("tyrus:crypto", "timingSafeEqual") => &["a", "b"][..],
        ("tyrus:crypto", "digest" | "hmac" | "cipher" | "rand" | "argon2" | "jwt") => &[][..],
        (
            "tyrus:encoding",
            "utf8" | "utf16" | "hex" | "base64" | "base64Url" | "base32" | "radix"
            | "EncodingError",
        ) => &[][..],
        (
            "tyrus:math",
            "PI" | "E" | "LN2" | "LN10" | "LOG2E" | "LOG10E" | "SQRT2" | "EPSILON" | "TAU" | "PHI",
        ) => &[][..],
        (
            "tyrus:math",
            "abs" | "sign" | "sqrt" | "cbrt" | "floor" | "ceil" | "trunc" | "round" | "sin" | "cos"
            | "tan" | "asin" | "acos" | "atan" | "sinh" | "cosh" | "tanh" | "exp" | "log" | "log10"
            | "log2" | "clz32" | "degToRad" | "radToDeg",
        ) => &["x"][..],
        (
            "tyrus:math",
            "atan2" | "pow" | "max" | "min" | "imul" | "gcd" | "lcm" | "randomRange",
        ) => &["a", "b"][..],
        ("tyrus:math", "approxEqual" | "clamp" | "lerp" | "inverseLerp" | "smoothstep") => {
            &["a", "b", "c"][..]
        }
        ("tyrus:math", "clampMin" | "clampMax") => &["value", "limit"][..],
        ("tyrus:math", "remap") => &["value", "inMin", "inMax", "outMin", "outMax"][..],
        ("tyrus:math", "roundTo") => &["x", "decimals"][..],
        ("tyrus:math", "mean" | "median" | "mode" | "variance" | "stdDev") => &["values"][..],
        ("tyrus:math", "percentile") => &["p", "values"][..],
        ("tyrus:math", "factorial") => &["n"][..],
        ("tyrus:math", "permutation" | "combination") => &["n", "k"][..],
        ("tyrus:math", "isPrime" | "nextPrime" | "primeFactors" | "fibonacci") => &["n"][..],
        ("tyrus:math", "hypot" | "copysign" | "fmod") => &["x", "y"][..],
        ("tyrus:math", "logBase") => &["x", "base"][..],
        ("tyrus:math", "powi") => &["x", "exp"][..],
        ("tyrus:math", "toPolar") => &["x", "y"][..],
        ("tyrus:math", "toCartesian") => &["r", "theta"][..],
        ("tyrus:algorithm", "suffixArray") => &["text"][..],
        ("tyrus:algorithm", "suffixArrayInfo") => &["text"][..],
        ("tyrus:algorithm", "SuffixArrayInfo") => &[][..],
        ("tyrus:algorithm", "suffixTreeUkkonen") => &["text"][..],
        ("tyrus:algorithm", "SuffixTree") => &[][..],
        ("tyrus:algorithm", "lcaBinaryLifting") => &["adj", "root"][..],
        ("tyrus:algorithm", "Lca") => &[][..],
        ("tyrus:algorithm", "heavyLightDecomposition") => &["adj", "root"][..],
        ("tyrus:algorithm", "Hld") => &[][..],
        ("tyrus:algorithm", "centroidDecomposition") => &["adj"][..],
        ("tyrus:algorithm", "Centroid") => &[][..],
        ("tyrus:algorithm", "bubbleSortInfo") => &["values"][..],
        (
            "tyrus:algorithm",
            "binarySearchInfo" | "interpolationSearchInfo" | "exponentialSearchInfo",
        ) => &["values", "target"][..],
        ("tyrus:algorithm", "quickselectInfo") => &["values", "rank"][..],
        ("tyrus:algorithm", "bfsInfo" | "dfsInfo") => &["adj", "start"][..],
        ("tyrus:algorithm", "iddfsInfo") => &["adj", "start", "maxDepth"][..],
        ("tyrus:algorithm", "dijkstraInfo") => &["adj", "start", "end"][..],
        ("tyrus:algorithm", "aStarInfo") => &["adj", "start", "end", "heuristic"][..],
        ("tyrus:algorithm", "bellmanFordInfo" | "spfaInfo") => &["adj", "start"][..],
        ("tyrus:algorithm", "johnsonInfo") => &["adj"][..],
        ("tyrus:algorithm", "floydWarshallInfo") => &["matrix"][..],
        ("tyrus:algorithm", "kruskalInfo" | "boruvkaInfo") => &["edges", "vertexCount"][..],
        ("tyrus:algorithm", "primInfo") => &["adjacency", "origin"][..],
        ("tyrus:algorithm", "steinerTreeInfo") => &["edges", "vertexCount", "terminals"][..],
        ("tyrus:algorithm", "edmondsKarpInfo" | "dinicInfo" | "pushRelabelInfo" | "minCutInfo") => {
            &["capacity", "source", "sink"][..]
        }
        ("tyrus:algorithm", "minCostMaxFlowInfo") => &["edges", "vertices", "source", "sink"][..],
        ("tyrus:algorithm", "fleuryEulerianInfo" | "hierholzerEulerianInfo") => &["adj"][..],
        (
            "tyrus:algorithm",
            "findBridgesInfo" | "findCutVerticesInfo" | "biconnectedComponentsInfo",
        ) => &["adj"][..],
        ("tyrus:algorithm", "edmondsBlossomInfo") => &["adj"][..],
        ("tyrus:algorithm", "hungarianInfo") => &["costMatrix"][..],
        ("tyrus:algorithm", "kmpSearchInfo" | "rabinKarpInfo" | "boyerMooreInfo") => {
            &["text", "pattern"][..]
        }
        ("tyrus:algorithm", "zAlgorithmInfo") => &["input"][..],
        ("tyrus:algorithm", "ahoCorasickInfo") => &["text", "patterns"][..],
        ("tyrus:algorithm", "levenshteinInfo") => &["left", "right"][..],
        (
            "tyrus:algorithm",
            "ShortestPathInfo" | "DijkstraInfo" | "AStarInfo" | "BellmanFordInfo" | "SpfaInfo"
            | "JohnsonInfo" | "FloydWarshallInfo" | "SpanningTreeInfo" | "KruskalInfo" | "PrimInfo"
            | "BoruvkaInfo" | "SteinerTreeInfo",
        ) => &[][..],
        (
            "tyrus:algorithm",
            "SortInfo"
            | "BubbleSortInfo"
            | "SearchInfo"
            | "InterpolationSearchInfo"
            | "ExponentialSearchInfo"
            | "SelectionInfo"
            | "TraversalInfo"
            | "IddfsInfo"
            | "EulerianInfo"
            | "FleuryEulerianInfo"
            | "HierholzerEulerianInfo"
            | "StringSearchInfo"
            | "KmpSearchInfo"
            | "ZAlgorithmInfo"
            | "RabinKarpInfo"
            | "BoyerMooreInfo"
            | "AhoCorasickInfo"
            | "EditDistanceInfo",
        ) => &[][..],
        _ => return None,
    };
    Some(
        parameters
            .iter()
            .map(|parameter| (*parameter).to_string())
            .collect(),
    )
}

pub(super) fn native_import_example(module: &str, imported: &str, local: &str) -> Option<String> {
    match (module, imported) {
        ("tyrus:simd", "simdAdd") => Some(format!(
            "const result: Result<i32[], SimdError> = {local}([1, 2], [3, 4]);"
        )),
        ("tyrus:simd", "simdSub") => Some(format!(
            "const result: Result<i32[], SimdError> = {local}([4, 3], [1, 2]);"
        )),
        ("tyrus:simd", "simdMul") => Some(format!(
            "const result: Result<i32[], SimdError> = {local}([2, 3], [4, 5]);"
        )),
        ("tyrus:simd", "simdDiv") => Some(format!(
            "const result: Result<i32[], SimdError> = {local}([8, 9], [2, 3]);"
        )),
        ("tyrus:fs", "readFileSync") => Some(format!(
            "const content: string = {local}(\"file.txt\", \"utf8\");"
        )),
        ("tyrus:fs", "writeFileSync") => Some(format!(
            "{local}(\"file.txt\", \"Hello from tyrus\", \"utf8\");"
        )),
        ("tyrus:http", "createRouter") => Some(format!("const app: HttpRouter = {local}();")),
        ("tyrus:http", "HttpStatus") => {
            Some(format!("const status: {local} = {local}.Ok;"))
        }
        ("tyrus:stream", "ReadableStream") => Some(format!("type Input = {local}<u8[]>;")),
        ("tyrus:task", "JoinHandle") => Some(format!("const handle: {local}<i32> = spawn(async (): Promise<i32> => 1);")),
        ("tyrus:task", "Sender") => Some(format!("async function produce(tx: {local}<i32>): Promise<void> {{ await tx.send(1); }}")),
        ("tyrus:task", "Receiver") => Some(format!("async function drain(rx: {local}<i32>): Promise<i32> {{ const value = await rx.recv(); return value ?? 0; }}")),
        ("tyrus:stream", "WritableStream") => Some(format!("type Output = {local}<u8[]>;")),
        ("tyrus:stream", "DuplexStream") => {
            Some(format!("type Connection = {local}<u8[], u8[]>;"))
        }
        ("tyrus:stream", "TransformStream") => {
            Some(format!("type Codec = {local}<string, u8[]>;"))
        }
        ("tyrus:stream", "StreamError") => Some(format!(
            "catch (error: {local}) {{ console.error(error.code); }}"
        )),
        ("tyrus:process", "exec") => Some(format!(
            "const result: Result<ChildProcess, ProcessError> = {local}(\"tool\", [\"--version\"]);"
        )),
        ("tyrus:process", "ChildProcess") => Some(format!(
            "const result: Result<{local}, ProcessError> = exec(\"tool\");"
        )),
        ("tyrus:process", "ProcessError") => Some(format!("catch (error: {local}) {{ console.error(error.code); }}")),
        ("tyrus:http", "HttpReply") => Some(format!("reply: {local}<UserResponse>")),
        ("tyrus:http", "HttpRouter") => Some(format!("const app: {local} = createRouter();")),
        ("tyrus:http", "HttpRequest") => Some(format!("const request: {local} = request;")),
        ("tyrus:http", "HttpResponse") => Some(format!("const response: {local} = {{ statusCode: HttpStatus.Ok, body: \"OK\", cookies: \"\" }};")),
        ("tyrus:http", "HttpCacheOptions") => Some(format!(
            "const cache: {local} = {{ etag: \"users-v1\", control: \"private, max-age=60\" }};"
        )),
        ("tyrus:http", "HttpReplyOptions") => {
            Some(format!("const options: {local} = {{ cache }};"))
        }
        ("tyrus:http", "HttpServer") => Some(format!(
            "const server: {local} = app.listen(3000) onfail {{ return; }}; await server.close();"
        )),
        ("tyrus:http", "HttpError") => Some("const server = app.listen(3000) onfail { console.error(server.code); return; };".to_string()),
        ("tyrus:http", "ServerOptions") => Some(format!(
            "const options: {local} = {{ host: \"127.0.0.1\", requestTimeout: \"30s\", maxConcurrentRequests: 1024 }};"
        )),
        ("tyrus:http", "RateLimit") => Some(format!(
            "const limit: {local} = {{ max: 100, timeWindow: \"1 minute\" }};"
        )),
        ("tyrus:http", "TlsOptions") => Some(format!(
            "const tls: {local} = {{ cert: \"certs/server.crt\", key: \"certs/server.key\" }};"
        )),
        ("tyrus:http", "TextBody") => Some(format!(
            "app.get(\"/robots.txt\", {{ response: {{ 200: {local} }}, handler: robots }});"
        )),
        ("tyrus:http", "BytesBody") => Some(format!(
            "app.get(\"/favicon.ico\", {{ response: {{ 200: {local} }}, handler: icon }});"
        )),
        ("tyrus:http", "StreamBody") => Some(format!(
            "app.get(\"/download\", {{ response: {{ 200: {local} }}, handler: download }});"
        )),
        ("tyrus:http", "MultipartBody") => Some(format!(
            "app.post(\"/uploads\", {{ body: {local}, response: {{ 200: Uploaded }}, handler: receive }});"
        )),
        ("tyrus:http", "MultipartPart") => Some(format!(
            "const parts: {local}[] = request.body.copy();"
        )),
        ("tyrus:http", "fetch") => Some(format!(
            "const answer: HttpClientResponse = await {local}(\"https://example.com\");"
        )),
        ("tyrus:http", "connectWebSocket") => Some(format!(
            "const socket: WebSocket = await {local}(\"wss://example.com/chat\");"
        )),
        ("tyrus:http", "WebSocket") => Some(format!(
            "const socket: {local} = await connectWebSocket(\"wss://example.com/chat\"); await socket.sendText(\"hello\");"
        )),
        ("tyrus:http", "WebSocketMessage") => Some(format!(
            "const message: {local}? = await socket.receive();"
        )),
        ("tyrus:http", "HttpClientResponse") => Some(format!(
            "const answer: {local} = await fetch(\"https://example.com\");"
        )),
        ("tyrus:http", "HttpClientStreamResponse") => Some(format!(
            "const answer: {local} = await fetch.stream(\"https://example.com/video\");"
        )),
        ("tyrus:http", "FetchOptions") => Some(format!(
            "const answer: HttpClientResponse = await fetch(url, {{ method: \"POST\", timeout: \"10s\" }}); // options: {local}"
        )),
        ("tyrus:http", "HttpHeader") => Some(format!(
            "const headers: {local}[] = [{{ name: \"accept\", value: \"application/json\" }}];"
        )),
        ("tyrus:http", "CorsOptions") => Some(format!("const cors: {local} = {{ origin: [], methods: [], allowedHeaders: [], exposedHeaders: [], credentials: false, maxAge: 0 }};")),
        ("tyrus:http", "CsrfOptions") => Some(format!("const csrf: {local} = {{ cookie: \"csrf_token\", header: \"x-csrf-token\" }};")),
        ("tyrus:http", "HttpAccessLog") => Some(format!("const logAccess = (entry: {local}): void => {{ console.log(entry.method, entry.path); }};")),
        ("tyrus:http", "FetchPolicyOptions") => Some(format!("const policy: {local} = {{ allow: [], denyPrivate: true }};")),
        ("tyrus:http", "CookieOptions") => Some(format!("const options: {local} = {{ httpOnly: true, path: \"/\", sameSite: \"lax\" }};")),
        ("tyrus:http", "HttpCookie") => Some(format!("const cookie: {local} = {{ name: \"session\", value: \"token\" }};")),
        ("tyrus:utils", "figlet") => Some(format!("const banner: string = {local}(\"HELLO\", \"slant\");")),
        ("tyrus:cli", "input") => Some(format!(
            "const name: string = {local}({{ message: \"Project name?\", pattern: /^[a-z][a-z0-9-]*$/, patternMessage: \"lowercase, start with a letter\" }});"
        )),
        ("tyrus:cli", "password") => Some(format!(
            "const secret: string = {local}({{ message: \"Type a password\", confirm: true, confirmMessage: \"Confirm password\" }});"
        )),
        ("tyrus:cli", "confirm") => Some(format!(
            "const ok: boolean = {local}({{ message: \"Continue?\", default: true, yes: \"Yes\", no: \"No\" }});"
        )),
        ("tyrus:cli", "isTTY") => {
            Some(format!("const interactive: boolean = {local}();"))
        }
        ("tyrus:task", "cancel") => Some(format!("const cancelled: boolean = {local}(operation, \"Cancelled by caller\");")),
        ("tyrus:task", "sleep") => Some(format!("{local}(500);")),
        ("tyrus:task", "sleepAsync") => Some(format!("await {local}(500);")),
        ("tyrus:task", "spawn") => Some(format!(
            "const handle = {local}(async (): Promise<i32> => {{ return 42; }});"
        )),
        ("tyrus:archive", "pack") => Some(format!(
            "const summary: ArchiveSummary = await {local}(\"./release\", \"./dist/release.tar.gz\")?;"
        )),
        ("tyrus:archive", "list") => Some(format!(
            "const entries: ArchiveEntry[] = await {local}(\"./incoming/upload.zip\")?;"
        )),
        ("tyrus:archive", "unpack") => Some(format!(
            "const summary: ArchiveSummary = await {local}(\"./incoming/upload.zip\", \"./workspace\")?;"
        )),
        ("tyrus:archive", "packStream") => Some(format!(
            "await {local}(\"./site\", ArchiveFormat.TarGz).pipeTo(writeFileStream(\"./site.tgz\"));"
        )),
        ("tyrus:archive", "unpackStream") => Some(format!(
            "const summary: ArchiveSummary = await {local}(readFileStream(\"./site.tgz\"), \"./site\")?;"
        )),
        ("tyrus:archive", "ArchiveFormat") => {
            Some(format!("const format: {local} = {local}.TarGz;"))
        }
        ("tyrus:archive", "ArchiveEntryKind") => {
            Some(format!("const kind: {local} = {local}.File;"))
        }
        ("tyrus:archive", "ArchiveEntry") => {
            Some(format!("const entries: {local}[] = await list(\"./a.zip\")?;"))
        }
        ("tyrus:archive", "ArchiveSummary") => Some(format!(
            "const summary: {local} = await pack(\"./src\", \"./dist/src.zip\")?;"
        )),
        ("tyrus:archive", "ArchiveError") => Some(format!(
            "match (await unpack(\"./a.zip\", \"./out\")) {{ Ok(summary) => {{ print(summary.entries); }} Err(error) => {{ const failure: {local} = error; printError(failure.code); }} }}"
        )),
        ("tyrus:archive", "PackOptions") => Some(format!(
            "const options: {local} = {{ level: 9, exclude: [\"**/*.map\"] }};"
        )),
        ("tyrus:archive", "UnpackOptions") => Some(format!(
            "const options: {local} = {{ stripComponents: 1, maxEntries: 10_000 }};"
        )),
        ("tyrus:compress", "compress") => Some(format!(
            "const packed: u8[] = await {local}(\"Hello, Tyrus!\", CompressFormat.Gzip)?;"
        )),
        ("tyrus:compress", "decompress") => Some(format!(
            "const original: u8[] = await {local}(packed, {{ maxOutputBytes: 64_000_000 }})?;"
        )),
        ("tyrus:compress", "compressionStream") => Some(format!(
            "await readFileStream(\"./app.log\").pipeTo({local}(CompressFormat.Gzip).writable);"
        )),
        ("tyrus:compress", "decompressionStream") => Some(format!(
            "const unpacker: TransformStream<u8[], u8[]> = {local}({{ maxOutputBytes: 64_000_000 }});"
        )),
        ("tyrus:compress", "CompressFormat") => {
            Some(format!("const format: {local} = {local}.Brotli;"))
        }
        ("tyrus:compress", "CompressError") => Some(format!(
            "match (await decompress(packed)) {{ Ok(bytes) => {{ print(bytes.length); }} Err(error) => {{ const failure: {local} = error; printError(failure.code); }} }}"
        )),
        ("tyrus:compress", "CompressOptions") => {
            Some(format!("const options: {local} = {{ level: 9 }};"))
        }
        ("tyrus:compress", "DecompressOptions") => Some(format!(
            "const options: {local} = {{ format: CompressFormat.Brotli, maxRatio: 1_000 }};"
        )),
        ("tyrus:os", "EOL") => Some(format!("console.log(`line one${{{local}}}line two`);")),
        ("tyrus:os", "platform") => Some(format!("const platform: OsPlatform = {local}();")),
        ("tyrus:os", "OsPlatform") => Some(format!("const platform: {local} = {local}.Linux;")),
        ("tyrus:os", "arch") => Some(format!("const arch: OsArch = {local}();")),
        ("tyrus:os", "OsArch") => Some(format!("const arch: {local} = {local}.X86_64;")),
        ("tyrus:os", "hostname") => Some(format!("const name: string = {local}();")),
        ("tyrus:os", "release") => Some(format!("const kernel: string = {local}();")),
        ("tyrus:os", "systemName") => Some(format!("const name: string = {local}();")),
        ("tyrus:os", "locale") => Some(format!("const locale: string = {local}();")),
        ("tyrus:os", "language") => Some(format!("const language: string = {local}();")),
        ("tyrus:os", "userInfo") => Some(format!("const info: UserInfo = {local}();")),
        ("tyrus:os", "userName") => Some(format!("const user: string = {local}();")),
        ("tyrus:os", "homeDir") => Some(format!("const home: string = {local}();")),
        ("tyrus:os", "cacheDir") => Some(format!("const cache: string = {local}();")),
        ("tyrus:os", "configDir") => Some(format!("const config: string = {local}();")),
        ("tyrus:os", "dataDir") => Some(format!("const data: string = {local}();")),
        ("tyrus:os", "tempDir") => Some(format!("const temp: string = {local}();")),
        ("tyrus:os", "whoami") => Some(format!("const name: string? = {local}();")),
        ("tyrus:os", "desktopDir") => Some(format!("const desktop: string? = {local}();")),
        ("tyrus:os", "documentsDir") => Some(format!("const documents: string? = {local}();")),
        ("tyrus:os", "appDataDir") => {
            Some(format!("const dir: string = {local}(\"my-app\");"))
        }
        ("tyrus:os", "cpus") => Some(format!("const cores: CpuInfo[] = {local}();")),
        ("tyrus:os", "networkInterfaces") => Some(format!(
            "const interfaces: NetworkInterface[] = {local}();"
        )),
        ("tyrus:os", "totalMem") => Some(format!("const total: u64 = {local}();")),
        ("tyrus:os", "freeMem") => Some(format!("const free: u64 = {local}();")),
        ("tyrus:os", "uptime") => Some(format!("const seconds: u64 = {local}();")),
        ("tyrus:os", "loadAvg") => Some(format!("const load: f64[] = {local}();")),
        ("tyrus:os", "battery") => Some(format!(
            "const battery: BatteryInfo? = {local}();\nif (battery) {{\n    console.log(battery.level);\n}}"
        )),
        ("tyrus:os", "cpuTemp") => Some(format!("const celsius: f32? = {local}();")),
        ("tyrus:os", "NotifyUrgency") => Some(format!(
            "const urgency: {local} = {local}.Critical;"
        )),
        ("tyrus:os", "notify") => Some(format!(
            "{local}(\"Build complete\", \"All 42 tests passed.\");"
        )),
        ("tyrus:os", "requestAttention") => Some(format!("{local}(false);")),
        ("tyrus:os", "revealInFolder") => Some(format!("{local}(\"./report.txt\");")),
        ("tyrus:os", "openUrl") => Some(format!("{local}(\"https://example.com\");")),
        ("tyrus:os", "trash") => Some(format!("{local}(\"./stale-report.txt\");")),
        ("tyrus:os", "clipboardWrite") => Some(format!("{local}(\"copied text\");")),
        ("tyrus:os", "clipboardRead") => Some(format!("const clipboard: string? = {local}();")),
        ("tyrus:os", "isDarkMode") => Some(format!("const dark: boolean = {local}();")),
        ("tyrus:os", "terminalSize") => Some(format!(
            "const size: [u16, u16]? = {local}();\nif (size) {{\n    console.log(size[0], size[1]);\n}}"
        )),
        ("tyrus:os", "isScreenLocked") => Some(format!("const locked: boolean = {local}();")),
        ("tyrus:os", "isSessionLocked") => Some(format!("const locked: boolean = {local}();")),
        ("tyrus:os", "preventSleep") => Some(format!("{local}();")),
        ("tyrus:os", "allowSleep") => Some(format!("{local}();")),
        ("tyrus:os", "systemFonts") => Some(format!("const fonts: FontInfo[] = {local}();")),
        ("tyrus:os", "accessibleColorContrast") => Some(format!(
            "const readable: boolean = {local}({{ r: 0, g: 0, b: 0 }}, {{ r: 255, g: 255, b: 255 }});"
        )),
        ("tyrus:os", "SystemCapability") => Some(format!(
            "const cap: {local} = {local}.Clipboard;"
        )),
        ("tyrus:os", "hasCapability") => Some(format!(
            "const canClip: boolean = {local}(SystemCapability.Clipboard);"
        )),
        ("tyrus:crypto", "timingSafeEqual") => Some(format!("const equal: boolean = {local}(a, b);")),
        ("tyrus:crypto", "digest") => Some(format!("const hash: u8[] = {local}.sha256(\"data\");")),
        ("tyrus:crypto", "hmac") => Some(format!("const mac: u8[] = {local}.sha256(\"data\", \"key\");")),
        ("tyrus:crypto", "cipher") => Some(format!("const enc: u8[] = {local}.aes.encrypt(\"text\", key);")),
        ("tyrus:crypto", "rand") => Some(format!("const id: string = {local}.uuidv7();")),
        ("tyrus:crypto", "argon2") => Some(format!("const hash: string = await {local}.hash(\"password\");\nconst migrate: boolean = {local}.needsRehash(hash);")),
        ("tyrus:crypto", "jwt") => Some(format!("const token: string = {local}.sign(payload, \"0123456789abcdef0123456789abcdef\");")),
        ("tyrus:encoding", "utf8") => Some(format!("const bytes: u8[] = {local}.encode(\"hello\");")),
        ("tyrus:encoding", "utf16") => Some(format!("const units: u16[] = {local}.encode(\"hello\");")),
        ("tyrus:encoding", "hex") => Some(format!("const text: string = {local}.encode(bytes);")),
        ("tyrus:encoding", "base64") => Some(format!("const text: string = {local}.encode(bytes);")),
        ("tyrus:encoding", "base64Url") => {
            Some(format!("const text: string = {local}.encode(bytes);"))
        }
        ("tyrus:encoding", "base32") => Some(format!("const text: string = {local}.encode(bytes);")),
        ("tyrus:encoding", "radix") => Some(format!("const text: string = {local}.format(255, 16);")),
        ("tyrus:cli", "select") => Some(format!(
            "const target: string = {local}({{ message: \"Target?\", choices: [\"script\", \"cli\"] }});"
        )),
        ("tyrus:cli", "multiselect") => Some(format!(
            "const features: string[] = {local}({{ message: \"Features?\", choices: [\"a\", \"b\"] }});"
        )),
        ("tyrus:cli", "number") => Some(format!(
            "const ratio: f64 = {local}({{ message: \"Ratio?\", default: 1.5 }});"
        )),
        ("tyrus:cli", "integer") => Some(format!(
            "const port: i64 = {local}({{ message: \"Port?\", default: 3000, min: 1, max: 65535 }});"
        )),
        ("tyrus:math", "PI" | "E" | "LN2" | "LN10" | "LOG2E" | "LOG10E" | "SQRT2" | "EPSILON" | "TAU" | "PHI") => Some(format!("const value: f64 = {local};")),
        ("tyrus:algorithm", "suffixArray") => Some("const indices: usize[] = suffixArray(\"banana\");".to_string()),
        ("tyrus:algorithm", "suffixArrayInfo") => Some("const info: SuffixArrayInfo = suffixArrayInfo(\"banana\");".to_string()),
        ("tyrus:algorithm", "suffixTreeUkkonen") => Some("const tree: SuffixTree = suffixTreeUkkonen(\"banana\");".to_string()),
        ("tyrus:algorithm", "lcaBinaryLifting") => Some("const lca: Lca = lcaBinaryLifting([[1], [0, 2], [1]], 0);".to_string()),
        ("tyrus:algorithm", "heavyLightDecomposition") => Some("const hld: Hld = heavyLightDecomposition([[1], [0, 2], [1]], 0);".to_string()),
        ("tyrus:algorithm", "centroidDecomposition") => Some("const ct: Centroid = centroidDecomposition([[1], [0, 2], [1]]);".to_string()),
        ("tyrus:algorithm", "bubbleSortInfo") => Some("const info: BubbleSortInfo = bubbleSortInfo([4, 1, 3, 2]);".to_string()),
        ("tyrus:algorithm", "dijkstraInfo") => Some("const info: DijkstraInfo = dijkstraInfo([[[1, 2]], []], 0, 1);".to_string()),
        ("tyrus:algorithm", "aStarInfo") => Some("const info: AStarInfo = aStarInfo([[[1, 2]], []], 0, 1, [2, 0]);".to_string()),
        ("tyrus:algorithm", "bellmanFordInfo") => Some("const info: BellmanFordInfo = bellmanFordInfo([[[1, 2]], []], 0);".to_string()),
        ("tyrus:algorithm", "spfaInfo") => Some("const info: SpfaInfo = spfaInfo([[[1, 2]], []], 0);".to_string()),
        ("tyrus:algorithm", "johnsonInfo") => Some("const info: JohnsonInfo = johnsonInfo([[[1, 2]], []]);".to_string()),
        ("tyrus:algorithm", "floydWarshallInfo") => Some("const info: FloydWarshallInfo = floydWarshallInfo([[0, 2], [2, 0]]);".to_string()),
        ("tyrus:algorithm", "kruskalInfo") => Some("const info: KruskalInfo = kruskalInfo([[0, 1, 2]], 2);".to_string()),
        ("tyrus:algorithm", "primInfo") => Some("const info: PrimInfo = primInfo([[[1, 2]], [[0, 2]]], 0);".to_string()),
        ("tyrus:algorithm", "boruvkaInfo") => Some("const info: BoruvkaInfo = boruvkaInfo([[0, 1, 2]], 2);".to_string()),
        ("tyrus:algorithm", "steinerTreeInfo") => Some("const info: SteinerTreeInfo = steinerTreeInfo([[0, 1, 2]], 2, [0, 1]);".to_string()),
        ("tyrus:algorithm", "fleuryEulerianInfo") => Some("const info: FleuryEulerianInfo = fleuryEulerianInfo([[1, 2], [0, 2], [0, 1]]);".to_string()),
        ("tyrus:algorithm", "hierholzerEulerianInfo") => Some("const info: HierholzerEulerianInfo = hierholzerEulerianInfo([[1, 2], [0, 2], [0, 1]]);".to_string()),
        ("tyrus:algorithm", "kmpSearchInfo") => Some("const info: KmpSearchInfo = kmpSearchInfo(\"ababa\", \"aba\");".to_string()),
        ("tyrus:algorithm", "zAlgorithmInfo") => Some("const info: ZAlgorithmInfo = zAlgorithmInfo(\"aabcaabxaaaz\");".to_string()),
        ("tyrus:algorithm", "rabinKarpInfo") => Some("const info: RabinKarpInfo = rabinKarpInfo(\"ababa\", \"aba\");".to_string()),
        ("tyrus:algorithm", "boyerMooreInfo") => Some("const info: BoyerMooreInfo = boyerMooreInfo(\"ababa\", \"aba\");".to_string()),
        ("tyrus:algorithm", "ahoCorasickInfo") => Some("const info: AhoCorasickInfo = ahoCorasickInfo(\"ababa\", [\"aba\", \"ba\"]);".to_string()),
        ("tyrus:algorithm", "levenshteinInfo") => Some("const info: EditDistanceInfo = levenshteinInfo(\"kitten\", \"sitting\");".to_string()),
        _ => None,
    }
}

pub(super) fn native_enum_member_names(module: &str, name: &str) -> Option<Vec<String>> {
    match (module, name) {
        ("tyrus:http", "HttpStatus") => Some(
            tyrus_core::module_loader::http_status_enum()
                .members
                .into_iter()
                .map(|(member, _)| member)
                .collect(),
        ),
        ("tyrus:archive", "ArchiveEntryKind") => Some(
            tyrus_core::archive::archive_entry_kind_enum()
                .members
                .into_iter()
                .map(|(member, _)| member)
                .collect(),
        ),
        ("tyrus:archive", "ArchiveFormat") => Some(
            tyrus_core::archive::archive_format_enum()
                .members
                .into_iter()
                .map(|(member, _)| member)
                .collect(),
        ),
        ("tyrus:compress", "CompressFormat") => Some(
            tyrus_core::compress::compress_format_enum()
                .members
                .into_iter()
                .map(|(member, _)| member)
                .collect(),
        ),
        ("tyrus:os", "OsPlatform") => Some(
            tyrus_core::os::os_platform_enum()
                .members
                .into_iter()
                .map(|(member, _)| member)
                .collect(),
        ),
        ("tyrus:os", "OsArch") => Some(
            tyrus_core::os::os_arch_enum()
                .members
                .into_iter()
                .map(|(member, _)| member)
                .collect(),
        ),
        ("tyrus:os", "NotifyUrgency") => Some(
            tyrus_core::os::notify_urgency_enum()
                .members
                .into_iter()
                .map(|(member, _)| member)
                .collect(),
        ),
        ("tyrus:os", "SystemCapability") => Some(
            tyrus_core::os::system_capability_enum()
                .members
                .into_iter()
                .map(|(member, _)| member)
                .collect(),
        ),
        ("tyrus:temporal", "TimeZone") => Some(
            tyrus_core::temporal::time_zone_enum()
                .members
                .into_iter()
                .map(|(member, _)| member)
                .collect(),
        ),
        _ => None,
    }
}

pub(super) fn native_static_member_names(
    module: &str,
    name: &str,
) -> Option<Vec<(String, String)>> {
    if module == "tyrus:crypto" || module == "tyrus:encoding" {
        let (_, fields, _) = tyrus_core::module_loader::native_export_info(module, name)?;
        return Some(
            fields?
                .into_iter()
                .map(|field| (field.name, field.ty.to_string()))
                .collect(),
        );
    }
    None
}

pub(super) fn native_namespace_member_documentation(
    module: &str,
    namespace: &str,
    member: &str,
) -> Option<&'static str> {
    match (module, namespace, member) {
        ("tyrus:encoding", "utf8", "encode") => {
            Some("Encodes the string as its UTF-8 byte sequence.")
        }
        ("tyrus:encoding", "utf8", "decode") => Some(
            "Decodes valid UTF-8 bytes. Invalid byte sequences return `Err(EncodingError)`; use `decodeLossy` to replace invalid sequences.",
        ),
        ("tyrus:encoding", "utf8", "decodeLossy") => Some(
            "Decodes UTF-8 bytes and replaces every invalid sequence with the Unicode replacement character.",
        ),
        ("tyrus:encoding", "utf16", "encode") => Some("Encodes the string as UTF-16 code units."),
        ("tyrus:encoding", "utf16", "encodeLE" | "encodeBE") => {
            Some("Encodes the string as UTF-16 bytes in the explicitly selected byte order.")
        }
        ("tyrus:encoding", "utf16", "decode" | "decodeLE" | "decodeBE") => Some(
            "Decodes valid UTF-16 input. Invalid surrogate sequences or odd byte lengths return `Err(EncodingError)`; use the matching lossy method to replace invalid sequences.",
        ),
        ("tyrus:encoding", "utf16", "decodeLossy" | "decodeLELossy" | "decodeBELossy") => Some(
            "Decodes UTF-16 input and replaces invalid surrogate sequences with the Unicode replacement character.",
        ),
        ("tyrus:encoding", "hex", "encode") => {
            Some("Encodes the bytes as a lowercase hexadecimal string with no `0x` prefix.")
        }
        ("tyrus:encoding", "hex", "decode") => Some(
            "Decodes a pair of ASCII hexadecimal digits to one byte. Odd lengths, whitespace, prefixes, and invalid characters return `Err(EncodingError)`.",
        ),
        ("tyrus:encoding", "base64", "encode" | "decode") => Some(
            "Encodes to the standard base64 alphabet (`+/`) with mandatory `=` padding, or decodes it back. Padding must be canonical, folded case is rejected, and any stray character returns `Err(EncodingError)` with `ERR_ENCODING_INVALID_BASE64`.",
        ),
        ("tyrus:encoding", "base64Url", "encode" | "decode") => Some(
            "Encodes to the URL-safe base64 alphabet (`-_`) without padding, or decodes it back. Padding and standard-alphabet characters are rejected, and malformed input returns `Err(EncodingError)` with `ERR_ENCODING_INVALID_BASE64`.",
        ),
        ("tyrus:encoding", "base32", "encode" | "decode") => Some(
            "Encodes to the canonical uppercase base32 alphabet (`A`-`Z`, `2`-`7`) with mandatory `=` padding, or decodes it back. Lowercase letters, stray padding, and non-zero trailing bits return `Err(EncodingError)` with `ERR_ENCODING_INVALID_BASE32`.",
        ),
        ("tyrus:encoding", "radix", "format") => Some(
            "Formats the integer in the given base with lowercase digits. Negative values keep a leading `-`; the signature is `format(value: i64, base: 2 | 8 | 10 | 16)`, so any other base is a type error and `format` never throws.",
        ),
        ("tyrus:encoding", "radix", "parseI64" | "parseU64") => Some(
            "Parses a canonical integer in the given base (2, 8, 10, or 16). Only lowercase digits are accepted, `parseI64` allows one leading `-` and rejects `+`, and `parseU64` rejects any sign. Syntax errors return `Err(EncodingError)` with `ERR_ENCODING_INVALID_RADIX`; overflow uses `ERR_ENCODING_INPUT_TOO_LARGE`.",
        ),
        ("tyrus:crypto", "digest", "md5") => Some(
            "Legacy compatibility only. MD5 is cryptographically broken and must not be used for signatures, password hashing, integrity, or security decisions.",
        ),
        ("tyrus:crypto", "digest", "create") => Some(
            "Creates an incremental digest handle. The handle is dropped automatically at lexical scope exit; call `dispose()` only for deterministic early release.",
        ),
        ("tyrus:crypto", "hmac", _) => Some(
            "Use an independently generated secret key with sufficient entropy. HMAC authenticates data; a plain digest does not.",
        ),
        ("tyrus:crypto", "cipher", _) => Some(
            "AES-256-GCM requires a 32-byte key and authenticates the versioned envelope plus optional AAD before exposing plaintext. Rotate keys and never reuse externally managed nonces.",
        ),
        ("tyrus:crypto", "argon2", "hash" | "verify") => Some(
            "Argon2id runs on a bounded blocking pool with explicit password, pepper, PHC, parameter, memory, and concurrency limits. A peppered hash carries a non-secret marker, so verify rejects a missing `secret` instead of silently returning false. Pepper material must be stored separately from password hashes.",
        ),
        ("tyrus:crypto", "argon2", "needsRehash") => Some(
            "Call only after successful password verification. This inexpensive check compares the stored PHC profile with the current Argon2id profile.",
        ),
        ("tyrus:crypto", "jwt", "decodeUnverified") => Some(
            "Danger: this generic method validates the requested claims layout but does not authenticate the token. Its attacker-controlled claims must never be used for authorization or trusted decisions.",
        ),
        ("tyrus:crypto", "jwt", "sign" | "verify") => Some(
            "Claims use typed objects; verification requires an explicit type argument and rejects missing, mismatched, or out-of-range fields. Algorithms are allowlisted and must match the key family. HMAC secrets require at least 32 bytes; asymmetric signing uses a private PEM key, while verification accepts public PEM or JWK JSON.",
        ),
        ("tyrus:crypto", "jwt", "fetchJwks") => Some(
            "Fetches a bounded JWKS document over HTTPS with validated public DNS addresses, no proxy or redirects, pinned resolution, timeouts, a size/key limit, bounded caching, and single-flight request coalescing.",
        ),
        ("tyrus:crypto", "jwt", "verifyWithJwks") => Some(
            "Selects exactly one verification key by `kid`, rejects duplicate identifiers and incompatible key metadata, enforces the configured algorithm allowlist, and validates the explicitly typed claims object.",
        ),
        ("tyrus:crypto", "rand", "randomBytes") => Some(
            "Returns operating-system-backed cryptographic random bytes and rejects requests above the configured resource limit.",
        ),
        _ => None,
    }
}

pub(super) fn native_namespace_member_hover_at(
    source: &str,
    offset: usize,
) -> Option<(std::ops::Range<usize>, String)> {
    let tokens = tyrus_core::lexer::lex_spanned(source).ok()?;
    for import in scan_imports(source) {
        if import.source != "tyrus:crypto" && import.source != "tyrus:encoding" {
            continue;
        }
        for (imported, local) in import.specifiers {
            let prefixes = if import.source == "tyrus:crypto" && imported == "cipher" {
                vec![(format!("{local}.aes."), "cipher")]
            } else {
                vec![(format!("{local}."), imported.as_str())]
            };
            for (prefix, namespace) in prefixes {
                for (start, _) in source.match_indices(&prefix) {
                    let member_start = start + prefix.len();
                    let member_end = source[member_start..]
                        .find(|character: char| {
                            !(character.is_ascii_alphanumeric() || character == '_')
                        })
                        .map_or(source.len(), |length| member_start + length);
                    let range = member_start..member_end;
                    if !(range.contains(&offset) || offset == range.end) || range.is_empty() {
                        continue;
                    }
                    if !tokens.iter().any(|(token, token_range)| {
                        matches!(token, tyrus_core::lexer::Token::Ident(_)) && token_range == &range
                    }) {
                        continue;
                    }
                    let member = &source[range.clone()];
                    let documentation =
                        native_namespace_member_documentation(&import.source, namespace, member)?;
                    let detail = if import.source == "tyrus:crypto" && namespace == "cipher" {
                        tyrus_core::crypto::cipher_aes_namespace_fields()
                            .into_iter()
                            .find(|field| field.name == member)
                            .map(|field| field.ty.to_string())
                    } else {
                        native_static_member_names(&import.source, namespace).and_then(|members| {
                            members
                                .into_iter()
                                .find(|(name, _)| name == member)
                                .map(|(_, detail)| detail)
                        })
                    }?;
                    return Some((
                        range,
                        format!("```tyrus\n{namespace}.{member}: {detail}\n```\n\n{documentation}"),
                    ));
                }
            }
        }
    }
    None
}

pub(super) fn function_usage_example(name: &str, symbol: &Symbol) -> Option<String> {
    if symbol.kind != SymbolKind::Function {
        return None;
    }
    let arguments = symbol
        .parameters
        .iter()
        .map(|parameter| {
            let param_name = parameter
                .pattern
                .binding_names()
                .first()
                .copied()
                .unwrap_or("param")
                .to_string();
            match &parameter.ty {
                Type::Infer => param_name,
                Type::I8
                | Type::I16
                | Type::I32
                | Type::I64
                | Type::I128
                | Type::Isize
                | Type::U8
                | Type::U16
                | Type::U32
                | Type::U64
                | Type::U128
                | Type::Usize => "0".to_string(),
                Type::F32 | Type::F64 | Type::F128 => "0.0".to_string(),
                Type::Decimal => "0d".to_string(),
                Type::Bool => "false".to_string(),
                Type::Char => "'a'".to_string(),
                Type::Regex => "/pattern/".to_string(),
                Type::Str => "\"text\"".to_string(),
                Type::Optional(_) => "None".to_string(),
                Type::Ref(_) => format!("&{param_name}"),
                Type::MutRef(_) => format!("&mut {param_name}"),
                Type::Function { .. } => "callback".to_string(),
                Type::Named(name) => name.to_ascii_lowercase(),
                Type::Generic(name) => name.to_ascii_lowercase(),
                Type::GenericFunction { .. } => "callback".to_string(),
                Type::Throws { .. } => "callback".to_string(),
                Type::Array(_) => "[]".to_string(),
                Type::Tuple(elements) => format!(
                    "[{}]",
                    elements
                        .iter()
                        .map(|_| "value")
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                Type::Readonly(inner) if matches!(inner.as_ref(), Type::Array(_)) => {
                    "[]".to_string()
                }
                Type::Readonly(_) => param_name,
                Type::Object(_) => "{{\"key\": value}}".to_string(),
                Type::Union(_) | Type::Result { .. } => param_name,
                Type::Unknown => param_name,
                Type::Never => param_name,
                Type::Lit(e) => lit_display(e),
                Type::Void => "value".to_string(),
                Type::TypeQuery(expr) => format!("typeof {}", expr_display(expr)),
                Type::Apply { .. }
                | Type::KeyOf(_)
                | Type::Conditional { .. }
                | Type::InferVar(_)
                | Type::Map(_, _)
                | Type::Set(_)
                | Type::BinaryHeap(_)
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
                | Type::Range(_) => param_name,
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!("{name}({arguments});"))
}

pub(super) fn hover_markdown(
    name: &str,
    detail: &str,
    example: Option<&str>,
    documentation: Option<&DocComment>,
) -> String {
    let mut sections = Vec::new();
    if let Some(doc) = documentation {
        let markdown = doc.to_markdown();
        if !markdown.is_empty() {
            sections.push(markdown);
        }
    }
    let signature = format!("```tyrus\n{name}: {detail}\n```");
    sections.push(signature);
    if let Some(example) = example {
        sections.push(format!("Example:\n\n```tyrus\n{example}\n```"));
    }
    sections.join("\n\n")
}

pub(super) fn interface_hover_markdown(
    name: &str,
    fields: &[Field],
    documentation: Option<&DocComment>,
    definition_link: Option<&str>,
) -> String {
    let mut sections = Vec::new();
    if let Some(doc) = documentation {
        let markdown = doc.to_markdown();
        if !markdown.is_empty() {
            sections.push(markdown);
        }
    }

    let mut body = format!("interface {name} {{");
    if fields.is_empty() {
        body.push('}');
    } else {
        for field in fields {
            let readonly = if field.readonly { "readonly " } else { "" };
            let optional = if field.optional { "?" } else { "" };
            body.push_str(&format!(
                "\n    {readonly}{}{optional}: {};",
                field.name, field.ty
            ));
        }
        body.push_str("\n}");
    }
    sections.push(format!("```tyrus\n{body}\n```"));

    if let Some(link) = definition_link {
        sections.push(format!("[Go to definition]({link})"));
    }
    sections.join("\n\n")
}

#[allow(clippy::too_many_lines)]
pub(super) fn native_exports() -> Vec<IndexedExport> {
    [
        ("tyrus:archive", "pack"),
        ("tyrus:archive", "list"),
        ("tyrus:archive", "unpack"),
        ("tyrus:archive", "packStream"),
        ("tyrus:archive", "unpackStream"),
        ("tyrus:archive", "ArchiveFormat"),
        ("tyrus:archive", "ArchiveEntryKind"),
        ("tyrus:archive", "ArchiveEntry"),
        ("tyrus:archive", "ArchiveSummary"),
        ("tyrus:archive", "ArchiveError"),
        ("tyrus:archive", "PackOptions"),
        ("tyrus:archive", "UnpackOptions"),
        ("tyrus:compress", "compress"),
        ("tyrus:compress", "decompress"),
        ("tyrus:compress", "compressionStream"),
        ("tyrus:compress", "decompressionStream"),
        ("tyrus:compress", "CompressFormat"),
        ("tyrus:compress", "CompressError"),
        ("tyrus:compress", "CompressOptions"),
        ("tyrus:compress", "DecompressOptions"),
        ("tyrus:fs", "readFileSync"),
        ("tyrus:fs", "writeFileSync"),
        ("tyrus:http", "createRouter"),
        ("tyrus:http", "HttpStatus"),
        ("tyrus:http", "HttpReply"),
        ("tyrus:http", "HttpRouter"),
        ("tyrus:http", "HttpRequest"),
        ("tyrus:http", "HttpResponse"),
        ("tyrus:http", "HttpError"),
        ("tyrus:http", "HttpServer"),
        ("tyrus:http", "WebSocket"),
        ("tyrus:http", "WebSocketMessage"),
        ("tyrus:http", "connectWebSocket"),
        ("tyrus:http", "ServerOptions"),
        ("tyrus:http", "RateLimit"),
        ("tyrus:http", "TlsOptions"),
        ("tyrus:http", "TextBody"),
        ("tyrus:http", "BytesBody"),
        ("tyrus:http", "StreamBody"),
        ("tyrus:http", "MultipartBody"),
        ("tyrus:http", "MultipartPart"),
        ("tyrus:http", "fetch"),
        ("tyrus:http", "HttpClientResponse"),
        ("tyrus:http", "HttpClientStreamResponse"),
        ("tyrus:http", "FetchOptions"),
        ("tyrus:http", "HttpHeader"),
        ("tyrus:http", "HttpCacheOptions"),
        ("tyrus:http", "HttpReplyOptions"),
        ("tyrus:http", "CorsOptions"),
        ("tyrus:http", "CsrfOptions"),
        ("tyrus:http", "HttpAccessLog"),
        ("tyrus:http", "FetchPolicyOptions"),
        ("tyrus:http", "CookieOptions"),
        ("tyrus:http", "HttpCookie"),
        ("tyrus:stream", "ReadableStream"),
        ("tyrus:stream", "WritableStream"),
        ("tyrus:stream", "DuplexStream"),
        ("tyrus:stream", "TransformStream"),
        ("tyrus:stream", "StreamError"),
        ("tyrus:process", "exec"),
        ("tyrus:process", "ChildProcess"),
        ("tyrus:process", "ProcessError"),
        ("tyrus:utils", "styleText"),
        ("tyrus:utils", "figlet"),
        ("tyrus:cli", "input"),
        ("tyrus:cli", "password"),
        ("tyrus:cli", "confirm"),
        ("tyrus:cli", "select"),
        ("tyrus:cli", "multiselect"),
        ("tyrus:cli", "number"),
        ("tyrus:cli", "integer"),
        ("tyrus:cli", "isTTY"),
        ("tyrus:task", "cancel"),
        ("tyrus:task", "sleep"),
        ("tyrus:task", "sleepAsync"),
        ("tyrus:task", "spawn"),
        ("tyrus:task", "JoinHandle"),
        ("tyrus:task", "Sender"),
        ("tyrus:task", "Receiver"),
        ("tyrus:simd", "simdAdd"),
        ("tyrus:simd", "simdSub"),
        ("tyrus:simd", "simdMul"),
        ("tyrus:simd", "simdDiv"),
        ("tyrus:crypto", "timingSafeEqual"),
        ("tyrus:crypto", "digest"),
        ("tyrus:crypto", "hmac"),
        ("tyrus:crypto", "cipher"),
        ("tyrus:crypto", "rand"),
        ("tyrus:crypto", "argon2"),
        ("tyrus:crypto", "jwt"),
        ("tyrus:encoding", "utf8"),
        ("tyrus:encoding", "utf16"),
        ("tyrus:encoding", "hex"),
        ("tyrus:encoding", "base64"),
        ("tyrus:encoding", "base64Url"),
        ("tyrus:encoding", "base32"),
        ("tyrus:encoding", "radix"),
        ("tyrus:encoding", "EncodingError"),
        ("tyrus:math", "PI"),
        ("tyrus:math", "E"),
        ("tyrus:math", "LN2"),
        ("tyrus:math", "LN10"),
        ("tyrus:math", "LOG2E"),
        ("tyrus:math", "LOG10E"),
        ("tyrus:math", "SQRT2"),
        ("tyrus:math", "EPSILON"),
        ("tyrus:math", "TAU"),
        ("tyrus:math", "PHI"),
        ("tyrus:math", "abs"),
        ("tyrus:math", "sign"),
        ("tyrus:math", "sqrt"),
        ("tyrus:math", "cbrt"),
        ("tyrus:math", "floor"),
        ("tyrus:math", "ceil"),
        ("tyrus:math", "trunc"),
        ("tyrus:math", "round"),
        ("tyrus:math", "sin"),
        ("tyrus:math", "cos"),
        ("tyrus:math", "tan"),
        ("tyrus:math", "asin"),
        ("tyrus:math", "acos"),
        ("tyrus:math", "atan"),
        ("tyrus:math", "atan2"),
        ("tyrus:math", "sinh"),
        ("tyrus:math", "cosh"),
        ("tyrus:math", "tanh"),
        ("tyrus:math", "exp"),
        ("tyrus:math", "log"),
        ("tyrus:math", "log10"),
        ("tyrus:math", "log2"),
        ("tyrus:math", "pow"),
        ("tyrus:math", "max"),
        ("tyrus:math", "min"),
        ("tyrus:math", "imul"),
        ("tyrus:math", "clz32"),
        ("tyrus:math", "approxEqual"),
        ("tyrus:math", "clamp"),
        ("tyrus:math", "clampMin"),
        ("tyrus:math", "clampMax"),
        ("tyrus:math", "lerp"),
        ("tyrus:math", "inverseLerp"),
        ("tyrus:math", "remap"),
        ("tyrus:math", "degToRad"),
        ("tyrus:math", "radToDeg"),
        ("tyrus:math", "gcd"),
        ("tyrus:math", "lcm"),
        ("tyrus:math", "randomRange"),
        ("tyrus:math", "smoothstep"),
        ("tyrus:math", "roundTo"),
        ("tyrus:math", "mean"),
        ("tyrus:math", "median"),
        ("tyrus:math", "mode"),
        ("tyrus:math", "variance"),
        ("tyrus:math", "stdDev"),
        ("tyrus:math", "percentile"),
        ("tyrus:math", "factorial"),
        ("tyrus:math", "permutation"),
        ("tyrus:math", "combination"),
        ("tyrus:math", "isPrime"),
        ("tyrus:math", "nextPrime"),
        ("tyrus:math", "primeFactors"),
        ("tyrus:math", "fibonacci"),
        ("tyrus:math", "hypot"),
        ("tyrus:math", "copysign"),
        ("tyrus:math", "fmod"),
        ("tyrus:math", "logBase"),
        ("tyrus:math", "powi"),
        ("tyrus:math", "toPolar"),
        ("tyrus:math", "toCartesian"),
        ("tyrus:os", "EOL"),
        ("tyrus:os", "platform"),
        ("tyrus:os", "OsPlatform"),
        ("tyrus:os", "arch"),
        ("tyrus:os", "OsArch"),
        ("tyrus:os", "hostname"),
        ("tyrus:os", "release"),
        ("tyrus:os", "systemName"),
        ("tyrus:os", "locale"),
        ("tyrus:os", "language"),
        ("tyrus:os", "userInfo"),
        ("tyrus:os", "userName"),
        ("tyrus:os", "homeDir"),
        ("tyrus:os", "cacheDir"),
        ("tyrus:os", "configDir"),
        ("tyrus:os", "dataDir"),
        ("tyrus:os", "tempDir"),
        ("tyrus:os", "whoami"),
        ("tyrus:os", "desktopDir"),
        ("tyrus:os", "documentsDir"),
        ("tyrus:os", "appDataDir"),
        ("tyrus:os", "cpus"),
        ("tyrus:os", "networkInterfaces"),
        ("tyrus:os", "totalMem"),
        ("tyrus:os", "freeMem"),
        ("tyrus:os", "uptime"),
        ("tyrus:os", "loadAvg"),
        ("tyrus:os", "battery"),
        ("tyrus:os", "cpuTemp"),
        ("tyrus:os", "notify"),
        ("tyrus:os", "NotifyUrgency"),
        ("tyrus:os", "requestAttention"),
        ("tyrus:os", "revealInFolder"),
        ("tyrus:os", "openUrl"),
        ("tyrus:os", "trash"),
        ("tyrus:os", "clipboardWrite"),
        ("tyrus:os", "clipboardRead"),
        ("tyrus:os", "isDarkMode"),
        ("tyrus:os", "terminalSize"),
        ("tyrus:os", "isScreenLocked"),
        ("tyrus:os", "isSessionLocked"),
        ("tyrus:os", "preventSleep"),
        ("tyrus:os", "allowSleep"),
        ("tyrus:os", "FontInfo"),
        ("tyrus:os", "systemFonts"),
        ("tyrus:os", "Color"),
        ("tyrus:os", "accessibleColorContrast"),
        ("tyrus:os", "SystemCapability"),
        ("tyrus:os", "hasCapability"),
        ("tyrus:algorithm", "suffixArray"),
        ("tyrus:algorithm", "suffixArrayInfo"),
        ("tyrus:algorithm", "SuffixArrayInfo"),
        ("tyrus:algorithm", "suffixTreeUkkonen"),
        ("tyrus:algorithm", "SuffixTree"),
        ("tyrus:algorithm", "lcaBinaryLifting"),
        ("tyrus:algorithm", "Lca"),
        ("tyrus:algorithm", "heavyLightDecomposition"),
        ("tyrus:algorithm", "Hld"),
        ("tyrus:algorithm", "centroidDecomposition"),
        ("tyrus:algorithm", "Centroid"),
        ("tyrus:algorithm", "bubbleSortInfo"),
        ("tyrus:algorithm", "binarySearchInfo"),
        ("tyrus:algorithm", "interpolationSearchInfo"),
        ("tyrus:algorithm", "exponentialSearchInfo"),
        ("tyrus:algorithm", "quickselectInfo"),
        ("tyrus:algorithm", "SearchInfo"),
        ("tyrus:algorithm", "InterpolationSearchInfo"),
        ("tyrus:algorithm", "ExponentialSearchInfo"),
        ("tyrus:algorithm", "SelectionInfo"),
        ("tyrus:algorithm", "bfsInfo"),
        ("tyrus:algorithm", "dfsInfo"),
        ("tyrus:algorithm", "iddfsInfo"),
        ("tyrus:algorithm", "TraversalInfo"),
        ("tyrus:algorithm", "IddfsInfo"),
        ("tyrus:algorithm", "SortInfo"),
        ("tyrus:algorithm", "BubbleSortInfo"),
        ("tyrus:algorithm", "dijkstraInfo"),
        ("tyrus:algorithm", "aStarInfo"),
        ("tyrus:algorithm", "bellmanFordInfo"),
        ("tyrus:algorithm", "spfaInfo"),
        ("tyrus:algorithm", "johnsonInfo"),
        ("tyrus:algorithm", "floydWarshallInfo"),
        ("tyrus:algorithm", "ShortestPathInfo"),
        ("tyrus:algorithm", "DijkstraInfo"),
        ("tyrus:algorithm", "AStarInfo"),
        ("tyrus:algorithm", "BellmanFordInfo"),
        ("tyrus:algorithm", "SpfaInfo"),
        ("tyrus:algorithm", "JohnsonInfo"),
        ("tyrus:algorithm", "FloydWarshallInfo"),
        ("tyrus:algorithm", "kruskalInfo"),
        ("tyrus:algorithm", "primInfo"),
        ("tyrus:algorithm", "boruvkaInfo"),
        ("tyrus:algorithm", "steinerTreeInfo"),
        ("tyrus:algorithm", "SpanningTreeInfo"),
        ("tyrus:algorithm", "KruskalInfo"),
        ("tyrus:algorithm", "PrimInfo"),
        ("tyrus:algorithm", "BoruvkaInfo"),
        ("tyrus:algorithm", "SteinerTreeInfo"),
        ("tyrus:algorithm", "fleuryEulerianInfo"),
        ("tyrus:algorithm", "hierholzerEulerianInfo"),
        ("tyrus:algorithm", "EulerianInfo"),
        ("tyrus:algorithm", "FleuryEulerianInfo"),
        ("tyrus:algorithm", "HierholzerEulerianInfo"),
        ("tyrus:algorithm", "kmpSearchInfo"),
        ("tyrus:algorithm", "zAlgorithmInfo"),
        ("tyrus:algorithm", "rabinKarpInfo"),
        ("tyrus:algorithm", "StringSearchInfo"),
        ("tyrus:algorithm", "KmpSearchInfo"),
        ("tyrus:algorithm", "ZAlgorithmInfo"),
        ("tyrus:algorithm", "RabinKarpInfo"),
        ("tyrus:algorithm", "boyerMooreInfo"),
        ("tyrus:algorithm", "BoyerMooreInfo"),
        ("tyrus:algorithm", "ahoCorasickInfo"),
        ("tyrus:algorithm", "AhoCorasickInfo"),
        ("tyrus:algorithm", "levenshteinInfo"),
        ("tyrus:algorithm", "EditDistanceInfo"),
    ]
    .into_iter()
    .map(|(module, name)| {
        let kind = match (module, name) {
            ("tyrus:http", "HttpStatus") => SymbolKind::Enum,
            ("tyrus:archive", "ArchiveEntryKind" | "ArchiveFormat") => SymbolKind::Enum,
            ("tyrus:compress", "CompressFormat") => SymbolKind::Enum,
            ("tyrus:compress", "CompressError" | "CompressOptions" | "DecompressOptions") => {
                SymbolKind::Interface
            }
            (
                "tyrus:archive",
                "ArchiveEntry" | "ArchiveSummary" | "ArchiveError" | "PackOptions"
                | "UnpackOptions",
            ) => SymbolKind::Interface,
            ("tyrus:os", "OsPlatform" | "OsArch" | "NotifyUrgency" | "SystemCapability") => {
                SymbolKind::Enum
            }
            ("tyrus:os", "EOL") => SymbolKind::Variable,
            ("tyrus:http", "HttpReply") => SymbolKind::TypeAlias,
            ("tyrus:http", "HttpRouter") => SymbolKind::TypeAlias,
            (
                "tyrus:stream",
                "ReadableStream" | "WritableStream" | "DuplexStream" | "TransformStream",
            ) => SymbolKind::TypeAlias,
            ("tyrus:stream", "StreamError") => SymbolKind::Interface,
            ("tyrus:process", "ChildProcess") => SymbolKind::TypeAlias,
            ("tyrus:http", "WebSocket") => SymbolKind::TypeAlias,
            ("tyrus:http", "WebSocketMessage") => SymbolKind::Interface,
            ("tyrus:task", "JoinHandle" | "Sender" | "Receiver") => SymbolKind::TypeAlias,
            ("tyrus:process", "ProcessError") => SymbolKind::Interface,
            (
                "tyrus:http",
                "HttpRequest" | "HttpResponse" | "CorsOptions" | "CsrfOptions" | "HttpAccessLog"
                | "FetchPolicyOptions" | "CookieOptions" | "HttpCookie",
            ) => SymbolKind::Interface,
            ("tyrus:algorithm", "SuffixTree") => SymbolKind::TypeAlias,
            ("tyrus:algorithm", "Lca") => SymbolKind::TypeAlias,
            ("tyrus:algorithm", "Hld") => SymbolKind::TypeAlias,
            ("tyrus:algorithm", "Centroid") => SymbolKind::TypeAlias,
            ("tyrus:algorithm", "SortInfo" | "BubbleSortInfo") => SymbolKind::Interface,
            (
                "tyrus:algorithm",
                "SearchInfo"
                | "InterpolationSearchInfo"
                | "ExponentialSearchInfo"
                | "SelectionInfo"
                | "TraversalInfo"
                | "IddfsInfo"
                | "ShortestPathInfo"
                | "DijkstraInfo"
                | "AStarInfo"
                | "BellmanFordInfo"
                | "SpfaInfo"
                | "JohnsonInfo"
                | "FloydWarshallInfo"
                | "SpanningTreeInfo"
                | "KruskalInfo"
                | "PrimInfo"
                | "BoruvkaInfo"
                | "SteinerTreeInfo"
                | "EulerianInfo"
                | "FleuryEulerianInfo"
                | "HierholzerEulerianInfo"
                | "StringSearchInfo"
                | "KmpSearchInfo"
                | "ZAlgorithmInfo"
                | "RabinKarpInfo"
                | "BoyerMooreInfo"
                | "AhoCorasickInfo"
                | "EditDistanceInfo"
                | "SuffixArrayInfo",
            ) => SymbolKind::Interface,
            ("tyrus:crypto", "digest" | "hmac" | "cipher" | "rand" | "argon2" | "jwt") => {
                SymbolKind::TypeAlias
            }
            (
                "tyrus:encoding",
                "utf8" | "utf16" | "hex" | "base64" | "base64Url" | "base32" | "radix",
            ) => SymbolKind::TypeAlias,
            ("tyrus:encoding", "EncodingError") => SymbolKind::Interface,
            (
                "tyrus:math",
                "PI" | "E" | "LN2" | "LN10" | "LOG2E" | "LOG10E" | "SQRT2" | "EPSILON" | "TAU",
            ) => SymbolKind::Variable,
            (
                "tyrus:math",
                "abs" | "sign" | "sqrt" | "cbrt" | "floor" | "ceil" | "trunc" | "round" | "sin"
                | "cos" | "tan" | "asin" | "acos" | "atan" | "atan2" | "sinh" | "cosh" | "tanh"
                | "exp" | "log" | "log10" | "log2" | "pow" | "max" | "min" | "imul" | "clz32",
            ) => SymbolKind::Function,
            _ => SymbolKind::Function,
        };
        IndexedExport {
            name: name.to_string(),
            is_default: false,
            module_source: None,
            ty: native_import_detail(module, name)
                .expect("native export must have signature details")
                .to_string(),
            parameters: native_parameter_names(module, name)
                .expect("native export must have parameter names"),
            kind,
            uri: Url::parse(module).expect("native module URI must be valid"),
            definition: None,
            is_protected: false,
            metadata: Arc::new(IndexedExportMetadata::Value { ty: None }),
        }
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_math_constant_is_visible_to_the_editor() {
        let indexed: Vec<String> = native_exports()
            .into_iter()
            .map(|export| export.name)
            .collect();
        for constant in tyrus_core::module_loader::math_constant_names() {
            assert_eq!(
                native_import_type("tyrus:math", constant),
                Some(Type::F64),
                "`{constant}` has no type for the editor"
            );
            assert!(
                native_import_detail("tyrus:math", constant).is_some(),
                "`{constant}` has no hover detail"
            );
            assert!(
                native_import_example("tyrus:math", constant, "value").is_some(),
                "`{constant}` has no usage example"
            );
            assert!(
                indexed.iter().any(|name| name == constant),
                "`{constant}` is missing from the completion index"
            );
        }
    }

    #[test]
    fn process_exec_exposes_its_result_contract_to_the_editor() {
        assert_eq!(
            native_import_type("tyrus:process", "exec"),
            Some(Type::Function {
                params: vec![
                    Type::Str,
                    Type::Optional(Box::new(Type::Array(Box::new(Type::Str)))),
                ],
                ret: Box::new(Type::Result {
                    success: Box::new(Type::Named("ChildProcess".to_string())),
                    error: Box::new(Type::Named("ProcessError".to_string())),
                }),
            })
        );
        assert_eq!(
            native_import_detail("tyrus:process", "exec"),
            Some("(command: string, args?: string[]) => Result<ChildProcess, ProcessError>")
        );
    }
}
