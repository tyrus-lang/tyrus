use super::*;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_temporal_zoned_call(
        &mut self,
        callee: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        match callee {
            "__tyrus_temporal_plainDateTime_until" | "__tyrus_temporal_plainDateTime_since" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `{}` expects 2 arguments, found {}",
                        callee,
                        args.len()
                    )))
                    .map(Some);
                }
                let ty = self.llvm_type(&Type::Named("PlainDateTime".to_string()));
                let duration = self.llvm_type(&Type::Named("Duration".to_string()));
                let left =
                    self.emit_expr_as(&args[0], &Type::Named("PlainDateTime".to_string()))?;
                let right =
                    self.emit_expr_as(&args[1], &Type::Named("PlainDateTime".to_string()))?;
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let left_slot = self
                    .builder
                    .build_alloca(ty, "temporal.datetime.difference.left")
                    .map_err(|error| error.to_string())?;
                let right_slot = self
                    .builder
                    .build_alloca(ty, "temporal.datetime.difference.right")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(left_slot, left)
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(right_slot, right)
                    .map_err(|error| error.to_string())?;
                let result_slot = self
                    .builder
                    .build_alloca(duration, "temporal.datetime.difference.result")
                    .map_err(|error| error.to_string())?;
                let symbol = if callee.ends_with("_since") {
                    "tyrus_temporal_plain_date_time_since"
                } else {
                    "tyrus_temporal_plain_date_time_until"
                };
                let function = self.declare_libc_fn(
                    symbol,
                    &[pointer.into(), pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[left_slot.into(), right_slot.into(), result_slot.into()],
                        "temporal.datetime.difference",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(duration, result_slot, "temporal.duration.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_plainDate_untilWithOptions"
            | "__tyrus_temporal_plainDate_sinceWithOptions"
            | "__tyrus_temporal_plainTime_untilWithOptions"
            | "__tyrus_temporal_plainTime_sinceWithOptions"
            | "__tyrus_temporal_plainDateTime_untilWithOptions"
            | "__tyrus_temporal_plainDateTime_sinceWithOptions" => {
                if args.len() != 6 {
                    return (Err(format!(
                        "error: `{}` expects 6 arguments, found {}",
                        callee,
                        args.len()
                    )))
                    .map(Some);
                }
                let type_name = if callee.contains("plainDateTime") {
                    "PlainDateTime"
                } else if callee.contains("plainTime") {
                    "PlainTime"
                } else {
                    "PlainDate"
                };
                let value_type = self.llvm_type(&Type::Named(type_name.to_string()));
                let duration_type = self.llvm_type(&Type::Named("Duration".to_string()));
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let start = self.emit_expr_as(&args[0], &Type::Named(type_name.to_string()))?;
                let end = self.emit_expr_as(&args[1], &Type::Named(type_name.to_string()))?;
                let largest = self.emit_expr_as(&args[2], &Type::Str)?;
                let smallest = self.emit_expr_as(&args[3], &Type::Str)?;
                let increment = self.emit_expr_as(&args[4], &Type::I64)?;
                let mode = self.emit_expr_as(&args[5], &Type::Str)?;
                let symbol = match (type_name, callee.ends_with("SinceWithOptions")) {
                    ("PlainDate", true) => "tyrus_temporal_plain_date_since_with_options",
                    ("PlainDate", false) => "tyrus_temporal_plain_date_until_with_options",
                    ("PlainTime", true) => "tyrus_temporal_plain_time_since_with_options",
                    ("PlainTime", false) => "tyrus_temporal_plain_time_until_with_options",
                    ("PlainDateTime", true) => "tyrus_temporal_plain_date_time_since_with_options",
                    _ => "tyrus_temporal_plain_date_time_until_with_options",
                };
                let start_slot = self
                    .builder
                    .build_alloca(value_type, "temporal.difference.start")
                    .map_err(|error| error.to_string())?;
                let end_slot = self
                    .builder
                    .build_alloca(value_type, "temporal.difference.end")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(start_slot, start)
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(end_slot, end)
                    .map_err(|error| error.to_string())?;
                let result_slot = self
                    .builder
                    .build_alloca(duration_type, "temporal.difference.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    symbol,
                    &[
                        pointer.into(),
                        pointer.into(),
                        pointer.into(),
                        pointer.into(),
                        self.context.i64_type().into(),
                        pointer.into(),
                        pointer.into(),
                    ],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[
                            start_slot.into(),
                            end_slot.into(),
                            largest.into(),
                            smallest.into(),
                            increment.into(),
                            mode.into(),
                            result_slot.into(),
                        ],
                        "temporal.difference.options",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(duration_type, result_slot, "temporal.duration.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_plainYearMonth_add" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `plainYearMonthAdd` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let ty = self.llvm_type(&Type::Named("PlainYearMonth".to_string()));
                let value =
                    self.emit_expr_as(&args[0], &Type::Named("PlainYearMonth".to_string()))?;
                let months = self.emit_expr_as(&args[1], &Type::I64)?;
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let input_slot = self
                    .builder
                    .build_alloca(ty, "temporal.yearMonth.add.input")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(input_slot, value)
                    .map_err(|error| error.to_string())?;
                let result_slot = self
                    .builder
                    .build_alloca(ty, "temporal.yearMonth.add.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_plain_year_month_add",
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
                        &[input_slot.into(), months.into(), result_slot.into()],
                        "temporal.yearMonth.add",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(ty, result_slot, "temporal.yearMonth.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_plainMonthDay_toPlainDate" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `plainMonthDayToPlainDate` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let month_day = self.llvm_type(&Type::Named("PlainMonthDay".to_string()));
                let date = self.llvm_type(&Type::Named("PlainDate".to_string()));
                let value =
                    self.emit_expr_as(&args[0], &Type::Named("PlainMonthDay".to_string()))?;
                let year = self.emit_expr_as(&args[1], &Type::I32)?;
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let input_slot =
                    self.native_value_slot(month_day, value, "temporal.monthDay.toDate.input")?;
                let result_slot = self
                    .builder
                    .build_alloca(date, "temporal.monthDay.toDate.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_plain_month_day_to_plain_date",
                    &[
                        pointer.into(),
                        self.context.i32_type().into(),
                        pointer.into(),
                    ],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[input_slot.into(), year.into(), result_slot.into()],
                        "temporal.monthDay.toDate",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(date, result_slot, "temporal.date.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_instant_toZonedDateTime" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `instantToZonedDateTime` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let instant = self.emit_expr_as(&args[0], &Type::Named("Instant".to_string()))?;
                let zone = self.emit_expr_as(&args[1], &Type::Named("TimeZone".to_string()))?;
                let instant_type = self.llvm_type(&Type::Named("Instant".to_string()));
                let result_type = self.llvm_type(&Type::Named("ZonedDateTime".to_string()));
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let instant_slot = self
                    .builder
                    .build_alloca(instant_type, "temporal.instant.zoned.input")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(instant_slot, instant)
                    .map_err(|error| error.to_string())?;
                let result_slot = self
                    .builder
                    .build_alloca(result_type, "temporal.instant.zoned.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_instant_to_zoned_date_time",
                    &[
                        pointer.into(),
                        self.context.i32_type().into(),
                        pointer.into(),
                    ],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[instant_slot.into(), zone.into(), result_slot.into()],
                        "temporal.instant.zoned",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(result_type, result_slot, "temporal.zoned.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_plainDate_fromFields" | "__tyrus_temporal_plainTime_fromFields" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `{}` expects 1 argument, found {}",
                        callee,
                        args.len()
                    )))
                    .map(Some);
                }
                let type_name = if callee.contains("plainDate") {
                    "PlainDate"
                } else {
                    "PlainTime"
                };
                let value = self.emit_expr_as(&args[0], &Type::Named(type_name.to_string()))?;
                let value_type = self.llvm_type(&Type::Named(type_name.to_string()));
                let symbol = if callee.contains("plainDate") {
                    "tyrus_temporal_plain_date_from_fields"
                } else {
                    "tyrus_temporal_plain_time_from_fields"
                };
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let input_slot = self
                    .builder
                    .build_alloca(value_type, "temporal.fields.input")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(input_slot, value)
                    .map_err(|error| error.to_string())?;
                let result_slot = self
                    .builder
                    .build_alloca(value_type, "temporal.fields.result")
                    .map_err(|error| error.to_string())?;
                let function =
                    self.declare_libc_fn(symbol, &[self.string_abi_type(), pointer.into()], None);
                self.builder
                    .build_call(
                        function,
                        &[input_slot.into(), result_slot.into()],
                        "temporal.fields",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(value_type, result_slot, "temporal.fields.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_plainDate_with" => {
                let left = self.emit_expr_as(&args[0], &Type::Named("PlainDate".to_string()))?;
                let right = self.emit_expr_as(&args[1], &Type::Named("PlainDate".to_string()))?;
                let ty = self.llvm_type(&Type::Named("PlainDate".to_string()));
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let left_slot = self
                    .builder
                    .build_alloca(ty, "temporal.date.with.left")
                    .map_err(|error| error.to_string())?;
                let right_slot = self
                    .builder
                    .build_alloca(ty, "temporal.date.with.right")
                    .map_err(|error| error.to_string())?;
                let result_slot = self
                    .builder
                    .build_alloca(ty, "temporal.date.with.result")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(left_slot, left)
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(right_slot, right)
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_plain_date_with",
                    &[pointer.into(), pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[left_slot.into(), right_slot.into(), result_slot.into()],
                        "temporal.date.with",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(ty, result_slot, "temporal.date.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_plainDate_subtractDuration"
            | "__tyrus_temporal_plainTime_subtractDuration"
            | "__tyrus_temporal_zonedDateTime_addDuration" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `{}` expects 2 arguments, found {}",
                        callee,
                        args.len()
                    )))
                    .map(Some);
                }
                let type_name = if callee.contains("plainDate") {
                    "PlainDate"
                } else if callee.contains("plainTime") {
                    "PlainTime"
                } else {
                    "ZonedDateTime"
                };
                let value = self.emit_expr_as(&args[0], &Type::Named(type_name.to_string()))?;
                let duration = self.emit_expr_as(&args[1], &Type::Named("Duration".to_string()))?;
                let value_type = self.llvm_type(&Type::Named(type_name.to_string()));
                let duration_type = self.llvm_type(&Type::Named("Duration".to_string()));
                let symbol = if callee.contains("plainDate") {
                    "tyrus_temporal_plain_date_subtract_duration"
                } else if callee.contains("plainTime") {
                    "tyrus_temporal_plain_time_subtract_duration"
                } else {
                    "tyrus_temporal_zoned_date_time_add_duration"
                };
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let input_slot = self
                    .builder
                    .build_alloca(value_type, "temporal.duration.input")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(input_slot, value)
                    .map_err(|error| error.to_string())?;
                let duration_slot =
                    self.native_value_slot(duration_type, duration, "temporal.duration.argument")?;
                let result_slot = self
                    .builder
                    .build_alloca(value_type, "temporal.duration.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    symbol,
                    &[pointer.into(), pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[input_slot.into(), duration_slot.into(), result_slot.into()],
                        "temporal.duration.apply",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(value_type, result_slot, "temporal.duration.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_plainDate_until"
            | "__tyrus_temporal_plainDate_since"
            | "__tyrus_temporal_plainTime_until"
            | "__tyrus_temporal_plainTime_since"
            | "__tyrus_temporal_zonedDateTime_until" => {
                let type_name = if callee.contains("plainDate") {
                    "PlainDate"
                } else if callee.contains("plainTime") {
                    "PlainTime"
                } else {
                    "ZonedDateTime"
                };
                let left = self.emit_expr_as(&args[0], &Type::Named(type_name.to_string()))?;
                let right = self.emit_expr_as(&args[1], &Type::Named(type_name.to_string()))?;
                let value_type = self.llvm_type(&Type::Named(type_name.to_string()));
                let duration_type = self.llvm_type(&Type::Named("Duration".to_string()));
                let symbol = match callee {
                    "__tyrus_temporal_plainDate_until" => "tyrus_temporal_plain_date_until",
                    "__tyrus_temporal_plainDate_since" => "tyrus_temporal_plain_date_since",
                    "__tyrus_temporal_plainTime_until" => "tyrus_temporal_plain_time_until",
                    "__tyrus_temporal_plainTime_since" => "tyrus_temporal_plain_time_since",
                    _ => "tyrus_temporal_zoned_date_time_until",
                };
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let left_slot = self
                    .builder
                    .build_alloca(value_type, "temporal.until.left")
                    .map_err(|error| error.to_string())?;
                let right_slot = self
                    .builder
                    .build_alloca(value_type, "temporal.until.right")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(left_slot, left)
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(right_slot, right)
                    .map_err(|error| error.to_string())?;
                let result_slot = self
                    .builder
                    .build_alloca(duration_type, "temporal.until.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    symbol,
                    &[pointer.into(), pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[left_slot.into(), right_slot.into(), result_slot.into()],
                        "temporal.until",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(duration_type, result_slot, "temporal.duration.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            _ => {}
        }
        Ok(None)
    }
}
