use super::*;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_temporal_plain_time_call(
        &mut self,
        callee: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        match callee {
            "__tyrus_temporal_plainTime_from" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `plainTimeFrom` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let input = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let time_type = self.llvm_type(&Type::Named("PlainTime".to_string()));
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let result_slot = self
                    .builder
                    .build_alloca(time_type, "temporal.time.from.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_plain_time_from",
                    &[self.string_abi_type(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[input.into(), result_slot.into()],
                        "temporal.plain_time.from",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(time_type, result_slot, "temporal.time.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_plainTime_toString" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `plainTimeToString` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let time = self.emit_expr_as(&args[0], &Type::Named("PlainTime".to_string()))?;
                let time_type = self.llvm_type(&Type::Named("PlainTime".to_string()));
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let input_slot =
                    self.native_value_slot(time_type, time, "temporal.time.string.input")?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_plain_time_to_string",
                    &[pointer.into()],
                    Some(self.string_abi_type()),
                );
                let result = self
                    .builder
                    .build_call(function, &[input_slot.into()], "temporal.plain_time.string")
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| "internal: plainTimeToString returned no value".to_string())?;
                self.guard_temporal_runtime_error()?;
                self.record_owned_native_result(result, Type::Str);
                return Ok(Some(result));
            }
            "__tyrus_temporal_plainTime_equals" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `plainTimeEquals` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let time_type = self.llvm_type(&Type::Named("PlainTime".to_string()));
                let left = self.emit_expr_as(&args[0], &Type::Named("PlainTime".to_string()))?;
                let right = self.emit_expr_as(&args[1], &Type::Named("PlainTime".to_string()))?;
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let left_slot =
                    self.native_value_slot(time_type, left, "temporal.time.equals.left")?;
                let right_slot =
                    self.native_value_slot(time_type, right, "temporal.time.equals.right")?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_plain_time_equals",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let result = self
                    .builder
                    .build_call(
                        function,
                        &[left_slot.into(), right_slot.into()],
                        "temporal.plain_time.equals",
                    )
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| "internal: plainTimeEquals returned no value".to_string())?;
                return Ok(Some(result));
            }
            "__tyrus_temporal_plainTime_addSeconds" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `plainTimeAddSeconds` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let time_type = self.llvm_type(&Type::Named("PlainTime".to_string()));
                let time = self.emit_expr_as(&args[0], &Type::Named("PlainTime".to_string()))?;
                let seconds = self.emit_expr_as(&args[1], &Type::I64)?;
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let input_slot =
                    self.native_value_slot(time_type, time, "temporal.time.add.input")?;
                let result_slot = self
                    .builder
                    .build_alloca(time_type, "temporal.time.add.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_plain_time_add_seconds",
                    &[
                        pointer.into(),
                        self.context.i64_type().into(),
                        pointer.into(),
                    ],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[input_slot.into(), seconds.into(), result_slot.into()],
                        "temporal.plain_time.add_seconds",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(time_type, result_slot, "temporal.time.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_plainTime_secondsUntil" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `plainTimeSecondsUntil` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let time_type = self.llvm_type(&Type::Named("PlainTime".to_string()));
                let start = self.emit_expr_as(&args[0], &Type::Named("PlainTime".to_string()))?;
                let end = self.emit_expr_as(&args[1], &Type::Named("PlainTime".to_string()))?;
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let start_slot =
                    self.native_value_slot(time_type, start, "temporal.time.until.start")?;
                let end_slot = self.native_value_slot(time_type, end, "temporal.time.until.end")?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_plain_time_seconds_until",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.i64_type().into()),
                );
                let result = self
                    .builder
                    .build_call(
                        function,
                        &[start_slot.into(), end_slot.into()],
                        "temporal.plain_time.seconds_until",
                    )
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| {
                        "internal: plainTimeSecondsUntil returned no value".to_string()
                    })?;
                self.guard_temporal_runtime_error()?;
                return Ok(Some(result));
            }
            "__tyrus_temporal_plainTime_addNanoseconds" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `plainTimeAddNanoseconds` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let time_type = self.llvm_type(&Type::Named("PlainTime".to_string()));
                let time = self.emit_expr_as(&args[0], &Type::Named("PlainTime".to_string()))?;
                let nanoseconds = self.emit_expr_as(&args[1], &Type::I64)?;
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let input_slot =
                    self.native_value_slot(time_type, time, "temporal.time.add_ns.input")?;
                let result_slot = self
                    .builder
                    .build_alloca(time_type, "temporal.time.add_ns.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_plain_time_add_nanoseconds",
                    &[
                        pointer.into(),
                        self.context.i64_type().into(),
                        pointer.into(),
                    ],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[input_slot.into(), nanoseconds.into(), result_slot.into()],
                        "temporal.plain_time.add_nanoseconds",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(time_type, result_slot, "temporal.time.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_plainTime_nanosecondsUntil" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `plainTimeNanosecondsUntil` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let time_type = self.llvm_type(&Type::Named("PlainTime".to_string()));
                let start = self.emit_expr_as(&args[0], &Type::Named("PlainTime".to_string()))?;
                let end = self.emit_expr_as(&args[1], &Type::Named("PlainTime".to_string()))?;
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let start_slot =
                    self.native_value_slot(time_type, start, "temporal.time.until_ns.start")?;
                let end_slot =
                    self.native_value_slot(time_type, end, "temporal.time.until_ns.end")?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_plain_time_nanoseconds_until",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.i64_type().into()),
                );
                let result = self
                    .builder
                    .build_call(
                        function,
                        &[start_slot.into(), end_slot.into()],
                        "temporal.plain_time.nanoseconds_until",
                    )
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| {
                        "internal: plainTimeNanosecondsUntil returned no value".to_string()
                    })?;
                self.guard_temporal_runtime_error()?;
                return Ok(Some(result));
            }
            "__tyrus_temporal_duration_fromFields" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `durationFromFields` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let duration_type = self.llvm_type(&Type::Named("Duration".to_string()));
                let duration = self.emit_expr_as(&args[0], &Type::Named("Duration".to_string()))?;
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let input_slot = self.native_value_slot(
                    duration_type,
                    duration,
                    "temporal.duration.fields.input",
                )?;
                let result_slot = self
                    .builder
                    .build_alloca(duration_type, "temporal.duration.fields.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_duration_from_fields",
                    &[pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[input_slot.into(), result_slot.into()],
                        "temporal.duration.from_fields",
                    )
                    .map_err(|error| error.to_string())?;
                return (self
                    .builder
                    .build_load(duration_type, result_slot, "temporal.duration.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_duration_total" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `durationTotal` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let duration_type = self.llvm_type(&Type::Named("Duration".to_string()));
                let duration = self.emit_expr_as(&args[0], &Type::Named("Duration".to_string()))?;
                let unit = self.emit_expr_as(&args[1], &Type::Str)?.into_struct_value();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let input_slot = self.native_value_slot(
                    duration_type,
                    duration,
                    "temporal.duration.total.input",
                )?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_duration_total",
                    &[pointer.into(), self.string_abi_type()],
                    Some(self.context.i64_type().into()),
                );
                let result = self
                    .builder
                    .build_call(
                        function,
                        &[input_slot.into(), unit.into()],
                        "temporal.duration.total",
                    )
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| "internal: durationTotal returned no value".to_string())?;
                self.guard_temporal_runtime_error()?;
                return Ok(Some(result));
            }
            "__tyrus_temporal_duration_round" => {
                if args.len() != 5 {
                    return (Err(format!(
                        "error: `durationRound` expects 5 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let duration_type = self.llvm_type(&Type::Named("Duration".to_string()));
                let date_type = self.llvm_type(&Type::Named("PlainDate".to_string()));
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let duration = self.emit_expr_as(&args[0], &Type::Named("Duration".to_string()))?;
                let unit = self.emit_expr_as(&args[1], &Type::Str)?;
                let increment = self.emit_expr_as(&args[2], &Type::I64)?;
                let mode = self.emit_expr_as(&args[3], &Type::Str)?;
                let relative_to =
                    self.emit_expr_as(&args[4], &Type::Named("PlainDate".to_string()))?;
                let duration_slot = self.native_value_slot(
                    duration_type,
                    duration,
                    "temporal.duration.round.input",
                )?;
                let relative_slot = self.native_value_slot(
                    date_type,
                    relative_to,
                    "temporal.duration.round.relative",
                )?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_duration_round",
                    &[
                        pointer.into(),
                        pointer.into(),
                        self.context.i64_type().into(),
                        pointer.into(),
                        pointer.into(),
                        pointer.into(),
                    ],
                    None,
                );
                let result_slot = self
                    .builder
                    .build_alloca(duration_type, "temporal.duration.round.result")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_call(
                        function,
                        &[
                            duration_slot.into(),
                            unit.into(),
                            increment.into(),
                            mode.into(),
                            relative_slot.into(),
                            result_slot.into(),
                        ],
                        "temporal.duration.round",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(duration_type, result_slot, "temporal.duration.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_plainDate_addDuration" | "__tyrus_temporal_plainTime_addDuration" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `{}` expects 2 arguments, found {}",
                        if callee == "__tyrus_temporal_plainDate_addDuration" {
                            "plainDateAddDuration"
                        } else {
                            "plainTimeAddDuration"
                        },
                        args.len()
                    )))
                    .map(Some);
                }
                let name = if callee == "__tyrus_temporal_plainDate_addDuration" {
                    "PlainDate"
                } else {
                    "PlainTime"
                };
                let runtime_name = if callee == "__tyrus_temporal_plainDate_addDuration" {
                    "tyrus_temporal_plain_date_add_duration"
                } else {
                    "tyrus_temporal_plain_time_add_duration"
                };
                let value_type = self.llvm_type(&Type::Named(name.to_string()));
                let duration_type = self.llvm_type(&Type::Named("Duration".to_string()));
                let value = self.emit_expr_as(&args[0], &Type::Named(name.to_string()))?;
                let duration = self.emit_expr_as(&args[1], &Type::Named("Duration".to_string()))?;
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let input_slot =
                    self.native_value_slot(value_type, value, "temporal.add_duration.input")?;
                let duration_slot =
                    self.native_value_slot(duration_type, duration, "temporal.add_duration.value")?;
                let result_slot = self
                    .builder
                    .build_alloca(value_type, "temporal.add_duration.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    runtime_name,
                    &[pointer.into(), pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[input_slot.into(), duration_slot.into(), result_slot.into()],
                        "temporal.add_duration",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(value_type, result_slot, "temporal.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            _ => {}
        }
        Ok(None)
    }
}
