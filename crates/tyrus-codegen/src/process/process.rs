use crate::Codegen;
use inkwell::values::BasicValueEnum;
use tyrus_core::ast::{Expr, ExprKind, Type};

impl<'ctx> Codegen<'ctx> {
    pub(super) fn is_process_namespace(&self, expr: &Expr) -> bool {
        matches!(expr.kind(), ExprKind::Ident(name) if name == "process")
            && self.load_var("process").is_none()
    }

    pub(super) fn emit_process_argv(&mut self) -> Result<BasicValueEnum<'ctx>, String> {
        let array_ty = self.llvm_type(&Type::Array(Box::new(Type::Str)));
        let slot = self.builder.build_alloca(array_ty, "process.argv").unwrap();
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let argv_fn = self.declare_libc_fn("tyrus_process_argv", &[ptr_ty.into()], None);
        self.builder
            .build_call(argv_fn, &[slot.into()], "")
            .unwrap();
        let value = self
            .builder
            .build_load(array_ty, slot, "process.argv.value")
            .unwrap();
        self.record_owned_native_result(value, Type::Array(Box::new(Type::Str)));
        Ok(value)
    }

    pub(super) fn emit_process_pid(&mut self) -> Result<BasicValueEnum<'ctx>, String> {
        let pid_fn = self.declare_libc_fn(
            "tyrus_process_pid",
            &[],
            Some(self.context.i32_type().into()),
        );
        Ok(self
            .builder
            .build_call(pid_fn, &[], "process.pid")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap())
    }

    pub(super) fn emit_process_ppid(&mut self) -> Result<BasicValueEnum<'ctx>, String> {
        let ppid_fn = self.declare_libc_fn(
            "tyrus_process_ppid",
            &[],
            Some(self.context.i32_type().into()),
        );
        Ok(self
            .builder
            .build_call(ppid_fn, &[], "process.ppid")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap())
    }

    pub(super) fn emit_process_cwd(&mut self) -> Result<BasicValueEnum<'ctx>, String> {
        let cwd_fn = self.declare_libc_fn("tyrus_process_cwd", &[], Some(self.string_abi_type()));
        let result = self
            .builder
            .build_call(cwd_fn, &[], "process.cwd")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap();
        self.record_owned_native_result(result, Type::Str);
        Ok(result)
    }

    pub(super) fn emit_process_memory_usage(&mut self) -> Result<BasicValueEnum<'ctx>, String> {
        let struct_ty = self.llvm_type(&Type::Named("MemoryInfo".to_string()));
        let slot = self
            .builder
            .build_alloca(struct_ty, "process.memoryUsage")
            .unwrap();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let memory_usage_fn =
            self.declare_libc_fn("tyrus_process_memory_usage", &[pointer.into()], None);
        self.builder
            .build_call(memory_usage_fn, &[slot.into()], "")
            .unwrap();
        Ok(self
            .builder
            .build_load(struct_ty, slot, "process.memoryUsage.value")
            .unwrap())
    }

    pub(super) fn emit_process_cpu_usage(&mut self) -> Result<BasicValueEnum<'ctx>, String> {
        let struct_ty = self.llvm_type(&Type::Named("CpuUsage".to_string()));
        let slot = self
            .builder
            .build_alloca(struct_ty, "process.cpuUsage")
            .unwrap();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let cpu_usage_fn = self.declare_libc_fn("tyrus_process_cpu_usage", &[pointer.into()], None);
        self.builder
            .build_call(cpu_usage_fn, &[slot.into()], "")
            .unwrap();
        Ok(self
            .builder
            .build_load(struct_ty, slot, "process.cpuUsage.value")
            .unwrap())
    }

    pub(super) fn emit_process_uptime(&mut self) -> Result<BasicValueEnum<'ctx>, String> {
        let uptime_fn = self.declare_libc_fn(
            "tyrus_process_uptime",
            &[],
            Some(self.context.i64_type().into()),
        );
        Ok(self
            .builder
            .build_call(uptime_fn, &[], "process.uptime")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap())
    }

    /// Emits `exec(command, args?)` without involving a shell. An omitted
    /// argument list crosses the ABI as an empty array, not an optional box.
    pub(super) fn emit_process_exec(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let command = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
        let argument_type = Type::Array(Box::new(Type::Str));
        let mut optional_arguments_cleanup = None;
        let arguments = match args.get(1) {
            Some(value)
                if matches!(
                    self.expr_type_hint(value).map(|ty| self.resolve_type(&ty)),
                    Some(Type::Optional(inner)) if *inner == argument_type
                ) =>
            {
                let optional = self.emit_expr(value)?.into_pointer_value();
                optional_arguments_cleanup = Some((
                    optional.into(),
                    self.expr_type_hint(value)
                        .map(|ty| self.resolve_type(&ty))
                        .unwrap_or_else(|| Type::Optional(Box::new(argument_type.clone()))),
                    value.clone(),
                ));
                let function = self
                    .builder
                    .get_insert_block()
                    .and_then(inkwell::basic_block::BasicBlock::get_parent)
                    .ok_or("internal: process exec outside function")?;
                let absent = self
                    .context
                    .append_basic_block(function, "process.args.absent");
                let present = self
                    .context
                    .append_basic_block(function, "process.args.present");
                let merge = self
                    .context
                    .append_basic_block(function, "process.args.merge");
                let is_absent = self
                    .builder
                    .build_is_null(optional, "process.args.is_absent")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_conditional_branch(is_absent, absent, present)
                    .map_err(|error| error.to_string())?;

                self.builder.position_at_end(absent);
                self.builder
                    .build_unconditional_branch(merge)
                    .map_err(|error| error.to_string())?;

                self.builder.position_at_end(present);
                let payload = self
                    .builder
                    .build_load(
                        self.llvm_type(&argument_type),
                        optional,
                        "process.args.payload",
                    )
                    .map_err(|error| error.to_string())?;
                let present_end = self.builder.get_insert_block().unwrap();
                self.builder
                    .build_unconditional_branch(merge)
                    .map_err(|error| error.to_string())?;

                self.builder.position_at_end(merge);
                let arguments = self
                    .builder
                    .build_phi(self.llvm_type(&argument_type), "process.args")
                    .map_err(|error| error.to_string())?;
                let empty = self.llvm_type(&argument_type).const_zero();
                arguments.add_incoming(&[(&empty, absent), (&payload, present_end)]);
                arguments.as_basic_value()
            }
            Some(value) => self.emit_expr_as(value, &argument_type)?,
            None => self.llvm_type(&argument_type).const_zero(),
        };
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let arguments_slot = self
            .builder
            .build_alloca(self.llvm_type(&argument_type), "process.args.abi")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(arguments_slot, arguments)
            .map_err(|error| error.to_string())?;
        let result_type = Type::Result {
            success: Box::new(Type::Named("ChildProcess".to_string())),
            error: Box::new(Type::Named("ProcessError".to_string())),
        };
        let result_llvm_type = self.llvm_type(&result_type);
        let result_slot = self
            .builder
            .build_alloca(result_llvm_type, "process.exec.result")
            .map_err(|error| error.to_string())?;
        let exec = self.declare_libc_fn(
            "tyrus_process_exec_result",
            &[self.string_abi_type(), pointer.into(), pointer.into()],
            None,
        );
        self.builder
            .build_call(
                exec,
                &[command.into(), arguments_slot.into(), result_slot.into()],
                "",
            )
            .map_err(|error| error.to_string())?;
        self.emit_async_cleanup_source_expression(command.into(), &Type::Str, &args[0])?;
        if let Some((value, ty, expression)) = optional_arguments_cleanup {
            self.emit_async_cleanup_source_expression(value, &ty, &expression)?;
        } else if let Some(expression) = args.get(1) {
            self.emit_async_cleanup_source_expression(arguments, &argument_type, expression)?;
        }
        let result = self
            .builder
            .build_load(result_llvm_type, result_slot, "process.exec.value")
            .map_err(|error| error.to_string())?;
        self.record_owned_native_result(result, result_type);
        Ok(result)
    }
}
