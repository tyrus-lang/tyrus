use super::*;
use tyrus_core::ast::ExprKind;

fn env_type_code(ty: &Type) -> u32 {
    match ty {
        Type::Bool => 1,
        Type::I32 => 2,
        Type::I64 => 3,
        Type::U32 => 4,
        Type::U64 => 5,
        Type::F32 => 6,
        Type::F64 => 7,
        _ => 0,
    }
}

impl<'ctx> Codegen<'ctx> {
    pub(super) fn is_process_env(&self, expr: &Expr) -> bool {
        matches!(
            expr,
            Expr { kind: ExprKind::FieldAccess { object, field }, .. }
                if field == "env"
                    && matches!(object.as_ref().kind(), ExprKind::Ident(name) if name == "process")
        ) && self.load_var("process").is_none()
    }

    pub(super) fn env_declared_type(&self, name: &str) -> Option<Type> {
        self.env_var_types.get(name).cloned()
    }

    pub(super) fn emit_env_startup(&mut self) -> Result<(), String> {
        if let Some(threads) = self.worker_threads.take() {
            let i64_type = self.context.i64_type();
            let configure = self.declare_libc_fn(
                "tyrus_runtime_configure_worker_threads",
                &[i64_type.into()],
                None,
            );
            let value = i64_type.const_int(u64::from(threads), false);
            self.builder
                .build_call(configure, &[value.into()], "")
                .unwrap();
        }

        if std::mem::take(&mut self.development_https) {
            let i32_type = self.context.i32_type();
            let configure = self.declare_libc_fn(
                "tyrus_runtime_configure_development_https",
                &[i32_type.into()],
                None,
            );
            self.builder
                .build_call(configure, &[i32_type.const_int(1, false).into()], "")
                .unwrap();
        }

        let Some(startup) = self.env_startup.take() else {
            return Ok(());
        };
        let i32_type = self.context.i32_type();

        match &startup.init {
            EnvInit::File { path } => {
                let init_fn =
                    self.declare_libc_fn("tyrus_env_init", &[self.string_abi_type()], None);
                let file = self.const_string_abi(path, "env.file").into_struct_value();
                self.builder
                    .build_call(init_fn, &[file.into()], "")
                    .unwrap();
            }
            EnvInit::Sealed {
                project_name,
                fingerprint,
            } => {
                let init_fn = self.declare_libc_fn(
                    "tyrus_env_init_sealed",
                    &[self.string_abi_type(), self.string_abi_type()],
                    None,
                );
                let project = self
                    .const_string_abi(project_name, "env.project")
                    .into_struct_value();
                let fingerprint = self
                    .const_string_abi(fingerprint, "env.fingerprint")
                    .into_struct_value();
                self.builder
                    .build_call(init_fn, &[project.into(), fingerprint.into()], "")
                    .unwrap();
            }
        }

        let require_fn = self.declare_libc_fn(
            "tyrus_env_require",
            &[
                self.string_abi_type(),
                i32_type.into(),
                self.string_abi_type(),
            ],
            None,
        );
        for var in &startup.vars {
            let detail = var.message.clone().unwrap_or_else(|| {
                format!(
                    "environment variable `{}` must be set to a `{}` value",
                    var.name, var.ty
                )
            });
            let message = format!(
                "tyrus validation error: `@type({})` failed for `{}`: {detail}",
                var.ty, var.name
            );
            let name = self
                .const_string_abi(&var.name, "env.name")
                .into_struct_value();
            let message = self
                .const_string_abi(&message, "env.message")
                .into_struct_value();
            self.builder
                .build_call(
                    require_fn,
                    &[
                        name.into(),
                        i32_type
                            .const_int(env_type_code(&var.ty) as u64, false)
                            .into(),
                        message.into(),
                    ],
                    "",
                )
                .unwrap();
        }

        self.env_validation_mode = true;
        for var in &startup.vars {
            if var.decorators.is_empty() {
                continue;
            }
            let value = self.emit_env_get(&var.name, &var.ty)?;
            self.emit_validator_checks(value, &var.ty, &var.decorators, &var.name)?;
            if matches!(self.resolve_type(&var.ty), Type::Str) {
                self.emit_string_release(value.into_struct_value())?;
            }
        }
        self.env_validation_mode = false;
        Ok(())
    }

    pub(super) fn emit_env_get(
        &mut self,
        name: &str,
        ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let name_ptr = self
            .const_string_abi(name, "env.get.name")
            .into_struct_value();
        let call = |codegen: &Self, fn_name: &str, ret: BasicTypeEnum<'ctx>| {
            let getter = codegen.declare_libc_fn(fn_name, &[codegen.string_abi_type()], Some(ret));
            codegen
                .builder
                .build_call(getter, &[name_ptr.into()], "env.get")
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap()
        };
        let value = match ty {
            Type::Str => call(self, "tyrus_env_get_str", self.string_abi_type()),
            Type::Bool => call(self, "tyrus_env_get_bool", self.context.bool_type().into()),
            Type::I64 => call(self, "tyrus_env_get_i64", self.context.i64_type().into()),
            Type::U64 => call(self, "tyrus_env_get_u64", self.context.i64_type().into()),
            Type::I32 | Type::U32 => {
                let fn_name = if matches!(ty, Type::I32) {
                    "tyrus_env_get_i64"
                } else {
                    "tyrus_env_get_u64"
                };
                let wide = call(self, fn_name, self.context.i64_type().into());
                self.builder
                    .build_int_truncate(
                        wide.into_int_value(),
                        self.context.i32_type(),
                        "env.get.trunc",
                    )
                    .unwrap()
                    .into()
            }
            Type::F64 => call(self, "tyrus_env_get_f64", self.context.f64_type().into()),
            Type::F32 => {
                let wide = call(self, "tyrus_env_get_f64", self.context.f64_type().into());
                self.builder
                    .build_float_trunc(
                        wide.into_float_value(),
                        self.context.f32_type(),
                        "env.get.trunc",
                    )
                    .unwrap()
                    .into()
            }
            other => {
                return Err(format!(
                    "internal: unsupported env variable type `{other}` for `{name}`"
                ));
            }
        };
        Ok(value)
    }
}
