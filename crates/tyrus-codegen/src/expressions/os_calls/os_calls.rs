use super::*;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_os_fn_call(
        &mut self,
        callee: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        match callee {
            "__tyrus_os_platform" => {
                let platform = self.declare_libc_fn(
                    "tyrus_os_platform",
                    &[],
                    Some(self.context.i32_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(platform, &[], "os.platform")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "__tyrus_os_arch" => {
                let arch = self.declare_libc_fn(
                    "tyrus_os_arch",
                    &[],
                    Some(self.context.i32_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(arch, &[], "os.arch")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
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
            | "__tyrus_os_tempDir" => {
                let c_fn_name = match callee {
                    "__tyrus_os_hostname" => "tyrus_os_hostname",
                    "__tyrus_os_release" => "tyrus_os_release",
                    "__tyrus_os_systemName" => "tyrus_os_system_name",
                    "__tyrus_os_locale" => "tyrus_os_locale",
                    "__tyrus_os_language" => "tyrus_os_language",
                    "__tyrus_os_userName" => "tyrus_os_user_name",
                    "__tyrus_os_homeDir" => "tyrus_os_home_dir",
                    "__tyrus_os_cacheDir" => "tyrus_os_cache_dir",
                    "__tyrus_os_configDir" => "tyrus_os_config_dir",
                    "__tyrus_os_dataDir" => "tyrus_os_data_dir",
                    _ => "tyrus_os_temp_dir",
                };
                let fn_val = self.declare_libc_fn(c_fn_name, &[], Some(self.string_abi_type()));
                let result = self
                    .builder
                    .build_call(fn_val, &[], "os.string")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.record_owned_native_result(result, Type::Str);
                return Ok(Some(result));
            }
            "__tyrus_os_desktopDir" | "__tyrus_os_documentsDir" | "__tyrus_os_whoami" => {
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let c_fn_name = match callee {
                    "__tyrus_os_desktopDir" => "tyrus_os_desktop_dir",
                    "__tyrus_os_documentsDir" => "tyrus_os_documents_dir",
                    _ => "tyrus_os_whoami",
                };
                let fn_val = self.declare_libc_fn(c_fn_name, &[], Some(pointer.into()));
                let result = self
                    .builder
                    .build_call(fn_val, &[], "os.optional_string")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.record_owned_native_result(result, Type::Optional(Box::new(Type::Str)));
                return Ok(Some(result));
            }
            "__tyrus_os_clipboardRead" => {
                let result_type = Type::Result {
                    success: Box::new(Type::Optional(Box::new(Type::Str))),
                    error: Box::new(Type::Named("OsError".to_string())),
                };
                let result_llvm_type = self.llvm_type(&result_type);
                let result_slot = self
                    .builder
                    .build_alloca(result_llvm_type, "os.clipboard_read.result")
                    .unwrap();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val =
                    self.declare_libc_fn("tyrus_os_clipboard_read_result", &[pointer.into()], None);
                self.builder
                    .build_call(fn_val, &[result_slot.into()], "")
                    .unwrap();
                let result = self
                    .builder
                    .build_load(
                        result_llvm_type,
                        result_slot,
                        "os.clipboard_read.result.value",
                    )
                    .unwrap();
                self.record_owned_native_result(result, result_type);
                return Ok(Some(result));
            }
            "__tyrus_os_appDataDir" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `appDataDir` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let app = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_os_app_data_dir",
                    &[self.string_abi_type()],
                    Some(self.string_abi_type()),
                );
                let result = self
                    .builder
                    .build_call(fn_val, &[app.into()], "os.appDataDir")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.record_owned_native_result(result, Type::Str);
                return Ok(Some(result));
            }
            "__tyrus_os_userInfo" => {
                let struct_ty = self.llvm_type(&Type::Named("UserInfo".to_string()));
                let slot = self.builder.build_alloca(struct_ty, "os.userInfo").unwrap();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val = self.declare_libc_fn("tyrus_os_user_info", &[pointer.into()], None);
                self.builder.build_call(fn_val, &[slot.into()], "").unwrap();
                let result = self
                    .builder
                    .build_load(struct_ty, slot, "os.userInfo.value")
                    .unwrap();
                self.record_owned_native_result(result, Type::Named("UserInfo".to_string()));
                return Ok(Some(result));
            }
            "__tyrus_os_cpus" => {
                let array_ty =
                    self.llvm_type(&Type::Array(Box::new(Type::Named("CpuInfo".to_string()))));
                let slot = self.builder.build_alloca(array_ty, "os.cpus").unwrap();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val = self.declare_libc_fn("tyrus_os_cpus", &[pointer.into()], None);
                self.builder.build_call(fn_val, &[slot.into()], "").unwrap();
                let result = self
                    .builder
                    .build_load(array_ty, slot, "os.cpus.value")
                    .unwrap();
                self.record_owned_native_result(
                    result,
                    Type::Array(Box::new(Type::Named("CpuInfo".to_string()))),
                );
                return Ok(Some(result));
            }
            "__tyrus_os_networkInterfaces" => {
                let result_type = Type::Result {
                    success: Box::new(Type::Array(Box::new(Type::Named(
                        "NetworkInterface".to_string(),
                    )))),
                    error: Box::new(Type::Named("OsError".to_string())),
                };
                let result_llvm_type = self.llvm_type(&result_type);
                let slot = self
                    .builder
                    .build_alloca(result_llvm_type, "os.networkInterfaces")
                    .unwrap();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val = self.declare_libc_fn(
                    "tyrus_os_network_interfaces_result",
                    &[pointer.into()],
                    None,
                );
                self.builder.build_call(fn_val, &[slot.into()], "").unwrap();
                let result = self
                    .builder
                    .build_load(result_llvm_type, slot, "os.networkInterfaces.value")
                    .unwrap();
                self.record_owned_native_result(result, result_type);
                return Ok(Some(result));
            }
            "__tyrus_os_totalMem" | "__tyrus_os_freeMem" | "__tyrus_os_uptime" => {
                let c_fn_name = match callee {
                    "__tyrus_os_totalMem" => "tyrus_os_total_mem",
                    "__tyrus_os_freeMem" => "tyrus_os_free_mem",
                    _ => "tyrus_os_uptime",
                };
                let fn_val =
                    self.declare_libc_fn(c_fn_name, &[], Some(self.context.i64_type().into()));
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[], "os.u64")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "__tyrus_os_loadAvg" => {
                let array_ty = self.llvm_type(&Type::Array(Box::new(Type::F64)));
                let slot = self.builder.build_alloca(array_ty, "os.loadAvg").unwrap();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val = self.declare_libc_fn("tyrus_os_load_avg", &[pointer.into()], None);
                self.builder.build_call(fn_val, &[slot.into()], "").unwrap();
                let result = self
                    .builder
                    .build_load(array_ty, slot, "os.loadAvg.value")
                    .unwrap();
                self.record_owned_native_result(result, Type::Array(Box::new(Type::F64)));
                return Ok(Some(result));
            }
            "__tyrus_os_battery" => {
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val = self.declare_libc_fn("tyrus_os_battery", &[], Some(pointer.into()));
                let result = self
                    .builder
                    .build_call(fn_val, &[], "os.battery")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.record_owned_native_result(
                    result,
                    Type::Optional(Box::new(Type::Named("BatteryInfo".to_string()))),
                );
                return Ok(Some(result));
            }
            "__tyrus_os_cpuTemp" => {
                if args.len() > 1 {
                    return (Err(format!(
                        "error: `cpuTemp` expects at most 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let default_unit = Expr::synthetic(ExprKind::StrLit("celsius".to_string()));
                let unit_arg = args.first().unwrap_or(&default_unit);
                let unit = self.emit_expr_as(unit_arg, &Type::Str)?.into_struct_value();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val = self.declare_libc_fn(
                    "tyrus_os_cpu_temp",
                    &[self.string_abi_type()],
                    Some(pointer.into()),
                );
                let result = self
                    .builder
                    .build_call(fn_val, &[unit.into()], "os.cpuTemp")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.record_owned_native_result(result, Type::Optional(Box::new(Type::F32)));
                return Ok(Some(result));
            }
            "__tyrus_os_notify" => {
                if !(2..=3).contains(&args.len()) {
                    return (Err(format!(
                        "error: `notify` expects 2 or 3 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let title = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let body = self.emit_expr_as(&args[1], &Type::Str)?.into_struct_value();
                let default_options = Expr::synthetic(ExprKind::ObjectLit(Vec::new()));
                let options_arg = args.get(2).unwrap_or(&default_options);
                let options_type = Type::Named("NotifyOptions".to_string());
                let options = self.emit_expr_as(options_arg, &options_type)?;
                let options_slot = self
                    .builder
                    .build_alloca(self.llvm_type(&options_type), "os.notify.options")
                    .unwrap();
                self.builder.build_store(options_slot, options).unwrap();
                let result_type = Type::Result {
                    success: Box::new(Type::Void),
                    error: Box::new(Type::Named("OsError".to_string())),
                };
                let result_llvm_type = self.llvm_type(&result_type);
                let result_slot = self
                    .builder
                    .build_alloca(result_llvm_type, "os.notify.result")
                    .unwrap();
                let fn_val = self.declare_libc_fn(
                    "tyrus_os_notify_result",
                    &[
                        self.string_abi_type(),
                        self.string_abi_type(),
                        pointer.into(),
                        pointer.into(),
                    ],
                    None,
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[
                            title.into(),
                            body.into(),
                            options_slot.into(),
                            result_slot.into(),
                        ],
                        "",
                    )
                    .unwrap();
                let result = self
                    .builder
                    .build_load(result_llvm_type, result_slot, "os.notify.result.value")
                    .unwrap();
                self.record_owned_native_result(result, result_type);
                return Ok(Some(result));
            }
            "__tyrus_os_requestAttention" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `requestAttention` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let critical = self.emit_expr_as(&args[0], &Type::Bool)?.into_int_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_os_request_attention",
                    &[self.context.bool_type().into()],
                    None,
                );
                self.builder
                    .build_call(fn_val, &[critical.into()], "")
                    .unwrap();
                return Ok(Some(self.context.i32_type().const_zero().into()));
            }
            "__tyrus_os_clipboardWrite" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `clipboardWrite` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let text = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let result_type = Type::Result {
                    success: Box::new(Type::Void),
                    error: Box::new(Type::Named("OsError".to_string())),
                };
                let result_llvm_type = self.llvm_type(&result_type);
                let result_slot = self
                    .builder
                    .build_alloca(result_llvm_type, "os.clipboard_write.result")
                    .unwrap();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val = self.declare_libc_fn(
                    "tyrus_os_clipboard_write_result",
                    &[self.string_abi_type(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(fn_val, &[text.into(), result_slot.into()], "")
                    .unwrap();
                let result = self
                    .builder
                    .build_load(
                        result_llvm_type,
                        result_slot,
                        "os.clipboard_write.result.value",
                    )
                    .unwrap();
                self.record_owned_native_result(result, result_type);
                return Ok(Some(result));
            }
            "__tyrus_os_openUrl" | "__tyrus_os_trash" | "__tyrus_os_revealInFolder" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `{}` expects 1 argument, found {}",
                        if callee == "__tyrus_os_openUrl" {
                            "openUrl"
                        } else if callee == "__tyrus_os_trash" {
                            "trash"
                        } else {
                            "revealInFolder"
                        },
                        args.len()
                    )))
                    .map(Some);
                }
                let target = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let c_fn_name = match callee {
                    "__tyrus_os_openUrl" => "tyrus_os_open_url_result",
                    "__tyrus_os_trash" => "tyrus_os_trash_result",
                    _ => "tyrus_os_reveal_in_folder_result",
                };
                let result_type = Type::Result {
                    success: Box::new(Type::Void),
                    error: Box::new(Type::Named("OsError".to_string())),
                };
                let result_llvm_type = self.llvm_type(&result_type);
                let result_slot = self
                    .builder
                    .build_alloca(result_llvm_type, "os.result")
                    .unwrap();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val = self.declare_libc_fn(
                    c_fn_name,
                    &[self.string_abi_type(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(fn_val, &[target.into(), result_slot.into()], "")
                    .unwrap();
                let result = self
                    .builder
                    .build_load(result_llvm_type, result_slot, "os.result.value")
                    .unwrap();
                self.record_owned_native_result(result, result_type);
                return Ok(Some(result));
            }
            "__tyrus_os_isDarkMode"
            | "__tyrus_os_isScreenLocked"
            | "__tyrus_os_isSessionLocked" => {
                let c_fn_name = match callee {
                    "__tyrus_os_isDarkMode" => "tyrus_os_is_dark_mode",
                    "__tyrus_os_isScreenLocked" => "tyrus_os_is_screen_locked",
                    _ => "tyrus_os_is_session_locked",
                };
                let fn_val =
                    self.declare_libc_fn(c_fn_name, &[], Some(self.context.bool_type().into()));
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[], "os.bool")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "__tyrus_os_terminalSize" => {
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val =
                    self.declare_libc_fn("tyrus_os_terminal_size", &[], Some(pointer.into()));
                let result = self
                    .builder
                    .build_call(fn_val, &[], "os.terminalSize")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.record_owned_native_result(
                    result,
                    Type::Optional(Box::new(Type::Tuple(vec![Type::U16, Type::U16]))),
                );
                return Ok(Some(result));
            }
            "__tyrus_os_preventSleep" | "__tyrus_os_allowSleep" => {
                let c_fn_name = if callee == "__tyrus_os_preventSleep" {
                    "tyrus_os_prevent_sleep_result"
                } else {
                    "tyrus_os_allow_sleep_result"
                };
                let result_type = Type::Result {
                    success: Box::new(Type::Void),
                    error: Box::new(Type::Named("OsError".to_string())),
                };
                let result_llvm_type = self.llvm_type(&result_type);
                let result_slot = self
                    .builder
                    .build_alloca(result_llvm_type, "os.result")
                    .unwrap();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val = self.declare_libc_fn(c_fn_name, &[pointer.into()], None);
                self.builder
                    .build_call(fn_val, &[result_slot.into()], "")
                    .unwrap();
                let result = self
                    .builder
                    .build_load(result_llvm_type, result_slot, "os.result.value")
                    .unwrap();
                self.record_owned_native_result(result, result_type);
                return Ok(Some(result));
            }
            "__tyrus_os_systemFonts" => {
                let array_ty =
                    self.llvm_type(&Type::Array(Box::new(Type::Named("FontInfo".to_string()))));
                let slot = self
                    .builder
                    .build_alloca(array_ty, "os.systemFonts")
                    .unwrap();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val = self.declare_libc_fn("tyrus_os_system_fonts", &[pointer.into()], None);
                self.builder.build_call(fn_val, &[slot.into()], "").unwrap();
                let result = self
                    .builder
                    .build_load(array_ty, slot, "os.systemFonts.value")
                    .unwrap();
                self.record_owned_native_result(
                    result,
                    Type::Array(Box::new(Type::Named("FontInfo".to_string()))),
                );
                return Ok(Some(result));
            }
            "__tyrus_os_accessibleColorContrast" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `accessibleColorContrast` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let color_type = Type::Named("Color".to_string());
                let color_llvm_type = self.llvm_type(&color_type);
                let fg = self.emit_expr_as(&args[0], &color_type)?;
                let fg_slot = self
                    .builder
                    .build_alloca(color_llvm_type, "os.accessibleColorContrast.fg")
                    .unwrap();
                self.builder.build_store(fg_slot, fg).unwrap();
                let bg = self.emit_expr_as(&args[1], &color_type)?;
                let bg_slot = self
                    .builder
                    .build_alloca(color_llvm_type, "os.accessibleColorContrast.bg")
                    .unwrap();
                self.builder.build_store(bg_slot, bg).unwrap();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val = self.declare_libc_fn(
                    "tyrus_os_accessible_color_contrast",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(
                            fn_val,
                            &[fg_slot.into(), bg_slot.into()],
                            "os.accessibleColorContrast",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "__tyrus_os_hasCapability" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `hasCapability` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let capability = self
                    .emit_expr_as(&args[0], &Type::Named("SystemCapability".to_string()))?
                    .into_int_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_os_has_capability",
                    &[self.context.i32_type().into()],
                    Some(self.context.bool_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[capability.into()], "os.hasCapability")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            _ => {}
        }
        Ok(None)
    }
}
