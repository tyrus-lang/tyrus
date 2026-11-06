use super::*;

pub(super) fn cli_prompt_return_type(callee: &str) -> Option<Type> {
    Some(match callee {
        "__tyrus_cli_input" | "__tyrus_cli_password" | "__tyrus_cli_select" => Type::Str,
        "__tyrus_cli_confirm" | tyrus_core::cli::IS_TTY_TARGET => Type::Bool,
        "__tyrus_cli_number" => Type::F64,
        "__tyrus_cli_integer" => Type::I64,
        "__tyrus_cli_multiselect" => Type::Array(Box::new(Type::Str)),
        _ => return None,
    })
}

enum PromptReturn {
    Str,
    Bool,
    Number,
    Integer,
    /// A `string[]` written to an out-parameter (`multiselect`).
    StringArray,
}

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_cli_prompt(
        &mut self,
        callee: &str,
        args: &[Expr],
    ) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        if callee == tyrus_core::cli::IS_TTY_TARGET {
            return Some(self.emit_cli_is_tty(args));
        }
        let (options_interface, ret) = match callee {
            "__tyrus_cli_input" => ("InputOptions", PromptReturn::Str),
            "__tyrus_cli_password" => ("PasswordOptions", PromptReturn::Str),
            "__tyrus_cli_confirm" => ("ConfirmOptions", PromptReturn::Bool),
            "__tyrus_cli_select" => ("SelectOptions", PromptReturn::Str),
            "__tyrus_cli_multiselect" => ("MultiSelectOptions", PromptReturn::StringArray),
            "__tyrus_cli_number" => ("NumberOptions", PromptReturn::Number),
            "__tyrus_cli_integer" => ("IntegerOptions", PromptReturn::Integer),
            _ => return None,
        };
        Some(self.emit_cli_prompt_inner(callee, options_interface, ret, args))
    }

    fn emit_cli_is_tty(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        if !args.is_empty() {
            return Err(format!(
                "error: `isTTY` takes no arguments, got {}",
                args.len()
            ));
        }
        let bool_type = self.context.bool_type();
        let function =
            self.declare_libc_fn(tyrus_core::cli::IS_TTY_TARGET, &[], Some(bool_type.into()));
        Ok(self
            .builder
            .build_call(function, &[], "cli_is_tty")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap())
    }

    fn emit_cli_prompt_inner(
        &mut self,
        callee: &str,
        options_interface: &str,
        ret: PromptReturn,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let Some(options_arg) = args.first() else {
            return Err(format!("error: `{callee}` requires an options object"));
        };
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let options_type = Type::Named(options_interface.to_string());
        let options = self.emit_expr_as(options_arg, &options_type)?;
        let options_slot = self
            .builder
            .build_alloca(self.llvm_type(&options_type), "cli_options")
            .unwrap();
        self.builder.build_store(options_slot, options).unwrap();

        match ret {
            PromptReturn::Str => {
                let function =
                    self.declare_libc_fn(callee, &[ptr_type.into()], Some(self.string_abi_type()));
                Ok(self
                    .builder
                    .build_call(function, &[options_slot.into()], "cli_prompt")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap())
            }
            PromptReturn::Bool => {
                let bool_type = self.context.bool_type();
                let function =
                    self.declare_libc_fn(callee, &[ptr_type.into()], Some(bool_type.into()));
                Ok(self
                    .builder
                    .build_call(function, &[options_slot.into()], "cli_prompt")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap())
            }
            PromptReturn::Number => {
                let f64_type = self.context.f64_type();
                let function =
                    self.declare_libc_fn(callee, &[ptr_type.into()], Some(f64_type.into()));
                Ok(self
                    .builder
                    .build_call(function, &[options_slot.into()], "cli_prompt")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap())
            }
            PromptReturn::Integer => {
                let i64_type = self.context.i64_type();
                let function =
                    self.declare_libc_fn(callee, &[ptr_type.into()], Some(i64_type.into()));
                Ok(self
                    .builder
                    .build_call(function, &[options_slot.into()], "cli_prompt")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap())
            }
            PromptReturn::StringArray => {
                let array_type = self.llvm_type(&Type::Array(Box::new(Type::Str)));
                let out_slot = self.builder.build_alloca(array_type, "cli_out").unwrap();
                let function =
                    self.declare_libc_fn(callee, &[ptr_type.into(), ptr_type.into()], None);
                self.builder
                    .build_call(
                        function,
                        &[options_slot.into(), out_slot.into()],
                        "cli_multiselect",
                    )
                    .unwrap();
                Ok(self
                    .builder
                    .build_load(array_type, out_slot, "cli_multiselect_result")
                    .unwrap())
            }
        }
    }
}
