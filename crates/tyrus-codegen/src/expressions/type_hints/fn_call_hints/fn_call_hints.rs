use super::*;

impl Codegen<'_> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn declared_fn_call_type_hint(&self, expr: &Expr) -> Option<Type> {
        match expr.kind() {
            ExprKind::FnCall {
                callee,
                type_args,
                args,
                ..
            } => crate::cli::cli_prompt_return_type(callee)
                .or_else(|| {
                    (callee == tyrus_core::memory::ZEROIZE_TARGET
                        || callee == "zeroize"
                        || tyrus_core::module_loader::original_global_name(callee) == "zeroize")
                        .then_some(Type::Result {
                            success: Box::new(Type::Void),
                            error: Box::new(Type::Named("MemoryError".to_string())),
                        })
                })
                .or_else(|| {
                    (callee == tyrus_core::crypto::DIGEST_CREATE_TARGET).then_some(Type::Result {
                        success: Box::new(Type::Named("DigestStream".to_string())),
                        error: Box::new(Type::Named("CryptoError".to_string())),
                    })
                })
                .or_else(|| {
                    (callee == tyrus_core::db::QUERY_TARGET)
                        .then(|| self.db_query_result_type(type_args, args))
                        .flatten()
                })
                .or_else(|| {
                    (callee == tyrus_core::db::FIND_MANY_TARGET)
                        .then(|| self.db_find_many_result_type(type_args))
                        .flatten()
                })
                .or_else(|| {
                    (callee == tyrus_core::db::FIND_ONE_TARGET)
                        .then(|| self.db_find_one_result_type(type_args))
                        .flatten()
                })
                .or_else(|| {
                    (callee == tyrus_core::db::COUNT_TARGET).then(|| self.db_count_result_type())
                })
                .or_else(|| {
                    (callee == tyrus_core::simd::ADD_TARGET
                        || callee == tyrus_core::simd::SUB_TARGET
                        || callee == tyrus_core::simd::MUL_TARGET
                        || callee == tyrus_core::simd::DIV_TARGET)
                        .then(|| {
                            type_args.first().cloned().or_else(|| {
                                args.first()
                                    .and_then(|array| self.expr_type_hint(array))
                                    .and_then(|array| match self.resolve_type(&array) {
                                        Type::Array(element) => Some(*element),
                                        Type::Tuple(elements) => elements.first().cloned(),
                                        _ => None,
                                    })
                            })
                        })
                        .flatten()
                        .map(|element| Type::Result {
                            success: Box::new(Type::Array(Box::new(element))),
                            error: Box::new(Type::Named("SimdError".to_string())),
                        })
                })
                .or_else(|| (callee == tyrus_core::task::CANCEL_TARGET).then_some(Type::Bool))
                .or_else(|| (callee == tyrus_core::task::IS_CANCELLED_TARGET).then_some(Type::Bool))
                .or_else(|| (callee == "__tyrus_net_supportsIcmp").then_some(Type::Bool))
                .or_else(|| (callee == tyrus_core::task::SLEEP_TARGET).then_some(Type::Void))
                .or_else(|| {
                    (callee == tyrus_core::task::SLEEP_ASYNC_TARGET).then_some(Type::Apply {
                        name: "Promise".to_string(),
                        args: vec![Type::Void],
                    })
                })
                .or_else(|| {
                    (callee == tyrus_core::task::SPAWN_TARGET)
                        .then(|| args.first())
                        .flatten()
                        .and_then(|task| match task.kind() {
                            ExprKind::ArrowFn {
                                params, ret_type, ..
                            } => Some(self.resolve_type(&Type::Function {
                                params: params.iter().map(|p| p.ty.clone()).collect(),
                                ret: Box::new(ret_type.clone()),
                            })),
                            _ => self
                                .expr_type_hint(task)
                                .map(|task_ty| self.resolve_type(&task_ty)),
                        })
                        .and_then(|task_ty| match task_ty {
                            Type::Function { ret, .. } => {
                                crate::core::promise_payload_type(&ret).cloned()
                            }
                            _ => None,
                        })
                        .map(|payload| Type::Apply {
                            name: "JoinHandle".to_string(),
                            args: vec![payload],
                        })
                })
                .or_else(|| {
                    (callee == tyrus_core::task::CHANNEL_TARGET)
                        .then(|| type_args.first())
                        .flatten()
                        .map(|payload| {
                            Type::Tuple(vec![
                                Type::Apply {
                                    name: "Sender".to_string(),
                                    args: vec![payload.clone()],
                                },
                                Type::Apply {
                                    name: "Receiver".to_string(),
                                    args: vec![payload.clone()],
                                },
                            ])
                        })
                })
                .or_else(|| {
                    (callee == tyrus_core::task::JOIN_ALL_TARGET)
                        .then(|| {
                            type_args.first().cloned().or_else(|| {
                                args.first()
                                    .and_then(|handles| self.expr_type_hint(handles))
                                    .and_then(|iterable| {
                                        match transparent_codegen_type(&iterable) {
                                            Type::Array(element) => {
                                                crate::core::join_handle_payload(element).cloned()
                                            }
                                            _ => None,
                                        }
                                    })
                            })
                        })
                        .flatten()
                        .map(|payload| Type::Apply {
                            name: "Promise".to_string(),
                            args: vec![Type::Array(Box::new(payload))],
                        })
                })
                .or_else(|| {
                    (callee == tyrus_core::task::RACE_TARGET)
                        .then(|| {
                            type_args.first().cloned().or_else(|| {
                                args.first()
                                    .and_then(|handles| self.expr_type_hint(handles))
                                    .and_then(|iterable| {
                                        match transparent_codegen_type(&iterable) {
                                            Type::Array(element) => {
                                                crate::core::join_handle_payload(element).cloned()
                                            }
                                            _ => None,
                                        }
                                    })
                            })
                        })
                        .flatten()
                        .map(|payload| Type::Apply {
                            name: "Promise".to_string(),
                            args: vec![payload],
                        })
                })
                .or_else(|| {
                    (callee == tyrus_core::task::TIMEOUT_TARGET)
                        .then(|| {
                            type_args.first().cloned().or_else(|| {
                                args.get(1)
                                    .and_then(|task| self.expr_type_hint(task))
                                    .and_then(|promise| {
                                        crate::core::promise_payload_type(transparent_codegen_type(
                                            &promise,
                                        ))
                                        .cloned()
                                    })
                            })
                        })
                        .flatten()
                        .map(|payload| Type::Apply {
                            name: "Promise".to_string(),
                            args: vec![payload],
                        })
                })
                .or_else(|| {
                    (callee == tyrus_core::task::MUTEX_TARGET)
                        .then(|| {
                            type_args.first().cloned().or_else(|| {
                                args.first().and_then(|value| self.expr_type_hint(value))
                            })
                        })
                        .flatten()
                        .map(|payload| Type::Apply {
                            name: "Mutex".to_string(),
                            args: vec![payload],
                        })
                })
                .or_else(|| {
                    (callee == tyrus_core::task::RWLOCK_TARGET)
                        .then(|| {
                            type_args.first().cloned().or_else(|| {
                                args.first().and_then(|value| self.expr_type_hint(value))
                            })
                        })
                        .flatten()
                        .map(|payload| Type::Apply {
                            name: "RwLock".to_string(),
                            args: vec![payload],
                        })
                })
                .or_else(|| {
                    (callee == tyrus_core::task::SEMAPHORE_TARGET)
                        .then_some(Type::Named("Semaphore".to_string()))
                })
                .or_else(|| {
                    let exported = tyrus_core::algorithm::algorithm_target_return_type(callee);
                    match exported {
                        Some(Type::Result { .. }) => exported,
                        other => self.algorithm_constructor_type(callee, type_args).or(other),
                    }
                })
                .or_else(|| {
                    (callee == "__tyrus_net_resolve").then_some(Type::Apply {
                        name: "Promise".to_string(),
                        args: vec![Type::Result {
                            success: Box::new(Type::Array(Box::new(Type::Str))),
                            error: Box::new(Type::Named("NetError".to_string())),
                        }],
                    })
                })
                .or_else(|| {
                    (callee == "__tyrus_net_reverseLookup").then_some(Type::Apply {
                        name: "Promise".to_string(),
                        args: vec![Type::Result {
                            success: Box::new(Type::Str),
                            error: Box::new(Type::Named("NetError".to_string())),
                        }],
                    })
                })
                .or_else(|| {
                    (callee == "__tyrus_net_ping").then_some(Type::Apply {
                        name: "Promise".to_string(),
                        args: vec![Type::Result {
                            success: Box::new(Type::U64),
                            error: Box::new(Type::Named("NetError".to_string())),
                        }],
                    })
                })
                .or_else(|| {
                    (callee == "__tyrus_net_traceroute").then_some(Type::Apply {
                        name: "Promise".to_string(),
                        args: vec![Type::Result {
                            success: Box::new(Type::Array(Box::new(Type::Str))),
                            error: Box::new(Type::Named("NetError".to_string())),
                        }],
                    })
                })
                .or_else(|| {
                    (callee == "__tyrus_http_connectWebSocket").then_some(Type::Apply {
                        name: "Promise".to_string(),
                        args: vec![Type::Result {
                            success: Box::new(Type::Named("WebSocket".to_string())),
                            error: Box::new(Type::Named("HttpError".to_string())),
                        }],
                    })
                })
                .or_else(|| {
                    (callee == "__tyrus_net_tcpConnect").then_some(Type::Apply {
                        name: "Promise".to_string(),
                        args: vec![Type::Result {
                            success: Box::new(Type::Named("TcpStream".to_string())),
                            error: Box::new(Type::Named("NetError".to_string())),
                        }],
                    })
                })
                .or_else(|| {
                    (callee == "__tyrus_net_tcpListen").then_some(Type::Apply {
                        name: "Promise".to_string(),
                        args: vec![Type::Result {
                            success: Box::new(Type::Named("TcpListener".to_string())),
                            error: Box::new(Type::Named("NetError".to_string())),
                        }],
                    })
                })
                .or_else(|| {
                    (callee == "__tyrus_net_udpBind").then_some(Type::Apply {
                        name: "Promise".to_string(),
                        args: vec![Type::Named("UdpSocket".to_string())],
                    })
                })
                .or_else(|| tyrus_core::module_loader::native_target_return_type(callee))
                .or_else(|| self.fn_signatures.get(callee).map(|(_, ret)| ret.clone()))
                .or_else(|| {
                    self.fn_signatures
                        .get(tyrus_core::module_loader::original_global_name(callee))
                        .map(|(_, ret)| ret.clone())
                })
                .or_else(|| {
                    self.load_var(callee)
                        .map(|(_, ty)| ty)
                        .or_else(|| self.async_prescan_types.get(callee).cloned())
                        .and_then(|ty| match transparent_codegen_type(&ty) {
                            Type::Function { ret, .. } => Some((**ret).clone()),
                            _ => None,
                        })
                })
                .or_else(|| {
                    if self.fn_signatures.contains_key(callee) || self.load_var(callee).is_some() {
                        return None;
                    }
                    match callee.as_str() {
                        tyrus_core::algorithm::QUICKSELECT_TARGET => {
                            Some(Type::Optional(Box::new(Type::I32)))
                        }
                        tyrus_core::algorithm::BINARY_SEARCH_TARGET
                        | tyrus_core::algorithm::INTERPOLATION_SEARCH_TARGET
                        | tyrus_core::algorithm::EXPONENTIAL_SEARCH_TARGET => {
                            Some(Type::Optional(Box::new(Type::Usize)))
                        }
                        tyrus_core::algorithm::KMP_SEARCH_TARGET
                        | tyrus_core::algorithm::Z_ALGORITHM_TARGET
                        | tyrus_core::algorithm::RABIN_KARP_TARGET
                        | tyrus_core::algorithm::BOYER_MOORE_TARGET => {
                            Some(Type::Array(Box::new(Type::Usize)))
                        }
                        tyrus_core::algorithm::AHO_CORASICK_TARGET => {
                            Some(Type::Array(Box::new(Type::Tuple(vec![
                                Type::Usize,
                                Type::Usize,
                            ]))))
                        }
                        tyrus_core::algorithm::LEVENSHTEIN_TARGET => Some(Type::Usize),
                        "String" => Some(Type::Str),
                        "isNaN" => Some(Type::Bool),
                        "parseInt" => Some(Type::Optional(Box::new(Type::I64))),
                        "parseDecimal" => Some(Type::Optional(Box::new(Type::Decimal))),
                        "parseFloat" | "Number" => Some(Type::Optional(Box::new(Type::F64))),
                        "__tyrus_process_exec" => Some(Type::Result {
                            success: Box::new(Type::Named("ChildProcess".to_string())),
                            error: Box::new(Type::Named("ProcessError".to_string())),
                        }),
                        "__tyrus_fs_readFileSync" => Some(Type::Result {
                            success: Box::new(Type::Str),
                            error: Box::new(Type::Named("FsError".to_string())),
                        }),
                        "__tyrus_fs_readdir" => Some(Type::Result {
                            success: Box::new(Type::Array(Box::new(Type::Named(
                                "DirEntry".to_string(),
                            )))),
                            error: Box::new(Type::Named("FsError".to_string())),
                        }),
                        "__tyrus_fs_mkdir"
                        | "__tyrus_fs_rm"
                        | "__tyrus_fs_cp"
                        | "__tyrus_fs_mv"
                        | "__tyrus_fs_removeAll"
                        | "__tyrus_fs_chmod"
                        | "__tyrus_fs_chown"
                        | "__tyrus_fs_symlink"
                        | "__tyrus_fs_symlink_dir"
                        | "__tyrus_fs_hard_link"
                        | "__tyrus_fs_atomic_write" => Some(Type::Result {
                            success: Box::new(Type::Void),
                            error: Box::new(Type::Named("FsError".to_string())),
                        }),
                        "__tyrus_fs_readlink" => Some(Type::Result {
                            success: Box::new(Type::Str),
                            error: Box::new(Type::Named("FsError".to_string())),
                        }),
                        "__tyrus_fs_canonicalize" | "__tyrus_fs_temp_file" => Some(Type::Result {
                            success: Box::new(Type::Str),
                            error: Box::new(Type::Named("FsError".to_string())),
                        }),
                        "__tyrus_fs_stat" => Some(Type::Result {
                            success: Box::new(Type::Named("FileStats".to_string())),
                            error: Box::new(Type::Named("FsError".to_string())),
                        }),
                        "__tyrus_fs_metadata" => Some(Type::Result {
                            success: Box::new(Type::Named("Metadata".to_string())),
                            error: Box::new(Type::Named("FsError".to_string())),
                        }),
                        "__tyrus_fs_disk_usage" => Some(Type::Result {
                            success: Box::new(Type::Named("DiskUsage".to_string())),
                            error: Box::new(Type::Named("FsError".to_string())),
                        }),
                        "__tyrus_fs_diff" => Some(Type::Result {
                            success: Box::new(Type::Named("DiffResult".to_string())),
                            error: Box::new(Type::Named("FsError".to_string())),
                        }),
                        "__tyrus_fs_temp_dir" => Some(Type::Named("TempDir".to_string())),
                        "__tyrus_fs_watch" => Some(Type::Result {
                            success: Box::new(Type::Named("FSWatcher".to_string())),
                            error: Box::new(Type::Named("FsError".to_string())),
                        }),
                        "__tyrus_fs_search" => Some(Type::Result {
                            success: Box::new(Type::Array(Box::new(Type::Str))),
                            error: Box::new(Type::Named("FsError".to_string())),
                        }),
                        "__tyrus_fs_glob" => Some(Type::Result {
                            success: Box::new(Type::Array(Box::new(Type::Str))),
                            error: Box::new(Type::Named("FsError".to_string())),
                        }),
                        "__tyrus_fs_mime" => Some(Type::Optional(Box::new(Type::Str))),
                        "__tyrus_os_hostname"
                        | "__tyrus_os_release"
                        | "__tyrus_os_systemName"
                        | "__tyrus_os_locale"
                        | "__tyrus_os_language"
                        | "__tyrus_os_userName"
                        | "__tyrus_os_homeDir"
                        | "__tyrus_os_cacheDir"
                        | "__tyrus_os_configDir"
                        | "__tyrus_os_dataDir"
                        | "__tyrus_os_tempDir"
                        | "__tyrus_os_appDataDir" => Some(Type::Str),
                        "__tyrus_os_desktopDir"
                        | "__tyrus_os_documentsDir"
                        | "__tyrus_os_whoami" => Some(Type::Optional(Box::new(Type::Str))),
                        "__tyrus_os_clipboardRead" => Some(Type::Result {
                            success: Box::new(Type::Optional(Box::new(Type::Str))),
                            error: Box::new(Type::Named("OsError".to_string())),
                        }),
                        "__tyrus_os_userInfo" => Some(Type::Named("UserInfo".to_string())),
                        "__tyrus_os_cpus" => {
                            Some(Type::Array(Box::new(Type::Named("CpuInfo".to_string()))))
                        }
                        "__tyrus_os_networkInterfaces" => Some(Type::Result {
                            success: Box::new(Type::Array(Box::new(Type::Named(
                                "NetworkInterface".to_string(),
                            )))),
                            error: Box::new(Type::Named("OsError".to_string())),
                        }),
                        "__tyrus_os_totalMem" | "__tyrus_os_freeMem" | "__tyrus_os_uptime" => {
                            Some(Type::U64)
                        }
                        "__tyrus_os_loadAvg" => Some(Type::Array(Box::new(Type::F64))),
                        "__tyrus_os_battery" => Some(Type::Optional(Box::new(Type::Named(
                            "BatteryInfo".to_string(),
                        )))),
                        "__tyrus_os_cpuTemp" => Some(Type::Optional(Box::new(Type::F32))),
                        "__tyrus_net_parseIpAddress" => Some(Type::Optional(Box::new(
                            Type::Named("IpAddress".to_string()),
                        ))),
                        "__tyrus_net_parseSocketAddress" => Some(Type::Optional(Box::new(
                            Type::Named("SocketAddress".to_string()),
                        ))),
                        "__tyrus_os_requestAttention" => Some(Type::Void),
                        "__tyrus_os_notify"
                        | "__tyrus_os_clipboardWrite"
                        | "__tyrus_os_openUrl"
                        | "__tyrus_os_trash"
                        | "__tyrus_os_revealInFolder"
                        | "__tyrus_os_preventSleep"
                        | "__tyrus_os_allowSleep" => Some(Type::Result {
                            success: Box::new(Type::Void),
                            error: Box::new(Type::Named("OsError".to_string())),
                        }),
                        "__tyrus_os_isDarkMode"
                        | "__tyrus_os_isScreenLocked"
                        | "__tyrus_os_isSessionLocked"
                        | "__tyrus_os_accessibleColorContrast"
                        | "__tyrus_os_hasCapability" => Some(Type::Bool),
                        "__tyrus_os_systemFonts" => {
                            Some(Type::Array(Box::new(Type::Named("FontInfo".to_string()))))
                        }
                        "__tyrus_os_terminalSize" => {
                            Some(Type::Optional(Box::new(Type::Tuple(vec![
                                Type::U16,
                                Type::U16,
                            ]))))
                        }
                        "__tyrus_os_platform" => Some(Type::Named("OsPlatform".to_string())),
                        "__tyrus_os_arch" => Some(Type::Named("OsArch".to_string())),
                        "__tyrus_temporal_timezone_from" => {
                            Some(Type::Named("TimeZone".to_string()))
                        }
                        "__tyrus_temporal_instant_now"
                        | "__tyrus_temporal_instant_fromEpochMilliseconds" => {
                            Some(Type::Named("Instant".to_string()))
                        }
                        "__tyrus_temporal_zonedDateTime_from" => {
                            Some(Type::Named("ZonedDateTime".to_string()))
                        }
                        "__tyrus_temporal_zonedDateTime_toInstant" => {
                            Some(Type::Named("Instant".to_string()))
                        }
                        "__tyrus_temporal_zonedDateTime_toString" => Some(Type::Str),
                        "__tyrus_temporal_instant_toZonedDateTime" => {
                            Some(Type::Named("ZonedDateTime".to_string()))
                        }
                        "__tyrus_temporal_plainDate_fromFields"
                        | "__tyrus_temporal_plainDate_with"
                        | "__tyrus_temporal_plainDate_subtractDuration" => {
                            Some(Type::Named("PlainDate".to_string()))
                        }
                        "__tyrus_temporal_plainTime_fromFields"
                        | "__tyrus_temporal_plainTime_subtractDuration" => {
                            Some(Type::Named("PlainTime".to_string()))
                        }
                        "__tyrus_temporal_plainDate_until"
                        | "__tyrus_temporal_plainDate_since"
                        | "__tyrus_temporal_plainTime_until"
                        | "__tyrus_temporal_plainTime_since"
                        | "__tyrus_temporal_zonedDateTime_until" => {
                            Some(Type::Named("Duration".to_string()))
                        }
                        "__tyrus_temporal_zonedDateTime_equals" => Some(Type::Bool),
                        "__tyrus_temporal_zonedDateTime_addDuration" => {
                            Some(Type::Named("ZonedDateTime".to_string()))
                        }
                        "__tyrus_temporal_zonedDateTime_format" => Some(Type::Str),
                        "__tyrus_temporal_plainDateTime_from"
                        | "__tyrus_temporal_plainDateTime_fromFields"
                        | "__tyrus_temporal_plainDateTime_with"
                        | "__tyrus_temporal_plainDateTime_addDuration"
                        | "__tyrus_temporal_plainDateTime_subtractDuration" => {
                            Some(Type::Named("PlainDateTime".to_string()))
                        }
                        "__tyrus_temporal_plainDateTime_toString" => Some(Type::Str),
                        "__tyrus_temporal_plainDateTime_equals" => Some(Type::Bool),
                        "__tyrus_temporal_plainDateTime_until"
                        | "__tyrus_temporal_plainDateTime_since" => {
                            Some(Type::Named("Duration".to_string()))
                        }
                        "__tyrus_temporal_plainDate_untilWithOptions"
                        | "__tyrus_temporal_plainDate_sinceWithOptions"
                        | "__tyrus_temporal_plainTime_untilWithOptions"
                        | "__tyrus_temporal_plainTime_sinceWithOptions"
                        | "__tyrus_temporal_plainDateTime_untilWithOptions"
                        | "__tyrus_temporal_plainDateTime_sinceWithOptions" => {
                            Some(Type::Named("Duration".to_string()))
                        }
                        "__tyrus_temporal_plainYearMonth_from"
                        | "__tyrus_temporal_plainYearMonth_fromFields"
                        | "__tyrus_temporal_plainYearMonth_add" => {
                            Some(Type::Named("PlainYearMonth".to_string()))
                        }
                        "__tyrus_temporal_plainYearMonth_toString" => Some(Type::Str),
                        "__tyrus_temporal_plainMonthDay_from"
                        | "__tyrus_temporal_plainMonthDay_fromFields" => {
                            Some(Type::Named("PlainMonthDay".to_string()))
                        }
                        "__tyrus_temporal_plainMonthDay_toString" => Some(Type::Str),
                        "__tyrus_temporal_plainMonthDay_toPlainDate" => {
                            Some(Type::Named("PlainDate".to_string()))
                        }
                        "__tyrus_temporal_instant_toEpochMilliseconds" => Some(Type::I64),
                        "__tyrus_temporal_instant_toString" => Some(Type::Str),
                        "__tyrus_temporal_instant_addDuration"
                        | "__tyrus_temporal_instant_subtractDuration" => {
                            Some(Type::Named("Instant".to_string()))
                        }
                        "__tyrus_temporal_plainDate_from" => {
                            Some(Type::Named("PlainDate".to_string()))
                        }
                        "__tyrus_temporal_plainDate_toString" => Some(Type::Str),
                        "__tyrus_temporal_plainDate_equals" => Some(Type::Bool),
                        "__tyrus_temporal_plainDate_addDays" => {
                            Some(Type::Named("PlainDate".to_string()))
                        }
                        "__tyrus_temporal_plainDate_dayOfWeek"
                        | "__tyrus_temporal_plainDate_dayOfYear" => Some(Type::U32),
                        "__tyrus_temporal_plainDate_addMonths"
                        | "__tyrus_temporal_plainDate_addYears" => {
                            Some(Type::Named("PlainDate".to_string()))
                        }
                        "__tyrus_temporal_plainDate_daysUntil" => Some(Type::I64),
                        "__tyrus_temporal_plainTime_from" => {
                            Some(Type::Named("PlainTime".to_string()))
                        }
                        "__tyrus_temporal_plainTime_toString" => Some(Type::Str),
                        "__tyrus_temporal_plainTime_equals" => Some(Type::Bool),
                        "__tyrus_temporal_plainTime_addSeconds" => {
                            Some(Type::Named("PlainTime".to_string()))
                        }
                        "__tyrus_temporal_plainTime_secondsUntil" => Some(Type::I64),
                        "__tyrus_temporal_plainTime_addNanoseconds" => {
                            Some(Type::Named("PlainTime".to_string()))
                        }
                        "__tyrus_temporal_plainTime_nanosecondsUntil" => Some(Type::I64),
                        "__tyrus_temporal_duration_fromFields" => {
                            Some(Type::Named("Duration".to_string()))
                        }
                        "__tyrus_temporal_duration_total" => Some(Type::I64),
                        "__tyrus_temporal_duration_round" => {
                            Some(Type::Named("Duration".to_string()))
                        }
                        "__tyrus_temporal_plainDate_addDuration" => {
                            Some(Type::Named("PlainDate".to_string()))
                        }
                        "__tyrus_temporal_plainTime_addDuration" => {
                            Some(Type::Named("PlainTime".to_string()))
                        }
                        tyrus_core::crypto::TIMING_SAFE_EQUAL_TARGET => Some(Type::Bool),
                        "tyrus_math_is_prime" | "tyrus_math_approx_equal" => Some(Type::Bool),
                        _ => None,
                    }
                }),
            _ => None,
        }
    }
}
