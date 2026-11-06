use super::*;

#[path = "plain_time_calls/plain_time_calls.rs"]
mod plain_time_calls;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_temporal_plain_fn_call(
        &mut self,
        callee: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        if let Some(result) = self.emit_temporal_plain_time_call(callee, args)? {
            return Ok(Some(result));
        }
        match callee {
            "__tyrus_temporal_zonedDateTime_equals" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `zonedDateTimeEquals` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let ty = self.llvm_type(&Type::Named("ZonedDateTime".to_string()));
                let left =
                    self.emit_expr_as(&args[0], &Type::Named("ZonedDateTime".to_string()))?;
                let right =
                    self.emit_expr_as(&args[1], &Type::Named("ZonedDateTime".to_string()))?;
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let left_slot = self
                    .builder
                    .build_alloca(ty, "temporal.zoned.equals.left")
                    .map_err(|error| error.to_string())?;
                let right_slot = self
                    .builder
                    .build_alloca(ty, "temporal.zoned.equals.right")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(left_slot, left)
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(right_slot, right)
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_zoned_date_time_equals",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let result = self
                    .builder
                    .build_call(
                        function,
                        &[left_slot.into(), right_slot.into()],
                        "temporal.zoned.equals",
                    )
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| "internal: zonedDateTimeEquals returned no value".to_string())?;
                self.guard_temporal_runtime_error()?;
                return Ok(Some(result));
            }
            "__tyrus_temporal_zonedDateTime_format" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `zonedDateTimeFormat` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let value =
                    self.emit_expr_as(&args[0], &Type::Named("ZonedDateTime".to_string()))?;
                let pattern = self.emit_expr_as(&args[1], &Type::Str)?;
                let value_type = self.llvm_type(&Type::Named("ZonedDateTime".to_string()));
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let input_slot = self
                    .builder
                    .build_alloca(value_type, "temporal.zoned.format.input")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(input_slot, value)
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_zoned_date_time_format",
                    &[pointer.into(), self.string_abi_type()],
                    Some(self.string_abi_type()),
                );
                let result = self
                    .builder
                    .build_call(
                        function,
                        &[input_slot.into(), pattern.into()],
                        "temporal.zoned.format",
                    )
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| "internal: zonedDateTimeFormat returned no value".to_string())?;
                self.guard_temporal_runtime_error()?;
                self.record_owned_native_result(result, Type::Str);
                return Ok(Some(result));
            }
            "__tyrus_temporal_instant_from" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `instantFrom` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let input = self.emit_expr_as(&args[0], &Type::Str)?;
                let instant_type = self.llvm_type(&Type::Named("Instant".to_string()));
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let result_slot = self
                    .builder
                    .build_alloca(instant_type, "temporal.instant.from.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_instant_from",
                    &[self.string_abi_type(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[input.into(), result_slot.into()],
                        "temporal.instant.from",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(instant_type, result_slot, "temporal.instant.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_zonedDateTime_from" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `zonedDateTimeFrom` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let input = self.emit_expr_as(&args[0], &Type::Str)?;
                let result_type = self.llvm_type(&Type::Named("ZonedDateTime".to_string()));
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let result_slot = self
                    .builder
                    .build_alloca(result_type, "temporal.zoned.from.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_zoned_date_time_from",
                    &[self.string_abi_type(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[input.into(), result_slot.into()],
                        "temporal.zoned.from",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(result_type, result_slot, "temporal.zoned.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_zonedDateTime_toInstant" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `zonedDateTimeToInstant` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let value =
                    self.emit_expr_as(&args[0], &Type::Named("ZonedDateTime".to_string()))?;
                let value_type = self.llvm_type(&Type::Named("ZonedDateTime".to_string()));
                let result_type = self.llvm_type(&Type::Named("Instant".to_string()));
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let input_slot = self
                    .builder
                    .build_alloca(value_type, "temporal.zoned.instant.input")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(input_slot, value)
                    .map_err(|error| error.to_string())?;
                let result_slot = self
                    .builder
                    .build_alloca(result_type, "temporal.zoned.instant.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_zoned_date_time_to_instant",
                    &[pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[input_slot.into(), result_slot.into()],
                        "temporal.zoned.instant",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(result_type, result_slot, "temporal.instant.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_zonedDateTime_toString" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `zonedDateTimeToString` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let value =
                    self.emit_expr_as(&args[0], &Type::Named("ZonedDateTime".to_string()))?;
                let value_type = self.llvm_type(&Type::Named("ZonedDateTime".to_string()));
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let input_slot = self
                    .builder
                    .build_alloca(value_type, "temporal.zoned.string.input")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(input_slot, value)
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_zoned_date_time_to_string",
                    &[pointer.into()],
                    Some(self.string_abi_type()),
                );
                let result = self
                    .builder
                    .build_call(function, &[input_slot.into()], "temporal.zoned.string")
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| {
                        "internal: zonedDateTimeToString returned no value".to_string()
                    })?;
                self.guard_temporal_runtime_error()?;
                self.record_owned_native_result(result, Type::Str);
                return Ok(Some(result));
            }
            "__tyrus_temporal_instant_toEpochMilliseconds" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `instantToEpochMilliseconds` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let instant = self.emit_expr_as(&args[0], &Type::Named("Instant".to_string()))?;
                let instant_type = self.llvm_type(&Type::Named("Instant".to_string()));
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let input_slot = self
                    .builder
                    .build_alloca(instant_type, "temporal.instant.to_ms.input")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(input_slot, instant)
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_instant_to_epoch_milliseconds",
                    &[pointer.into()],
                    Some(self.context.i64_type().into()),
                );
                let result = self
                    .builder
                    .build_call(function, &[input_slot.into()], "temporal.instant.to_ms")
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| {
                        "internal: instantToEpochMilliseconds returned no value".to_string()
                    })?;
                self.guard_temporal_runtime_error()?;
                return Ok(Some(result));
            }
            "__tyrus_temporal_instant_toString" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `instantToString` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let instant = self.emit_expr_as(&args[0], &Type::Named("Instant".to_string()))?;
                let instant_type = self.llvm_type(&Type::Named("Instant".to_string()));
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let input_slot = self
                    .builder
                    .build_alloca(instant_type, "temporal.instant.string.input")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(input_slot, instant)
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_instant_to_string",
                    &[pointer.into()],
                    Some(self.string_abi_type()),
                );
                let result = self
                    .builder
                    .build_call(function, &[input_slot.into()], "temporal.instant.string")
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| "internal: instantToString returned no value".to_string())?;
                self.guard_temporal_runtime_error()?;
                self.record_owned_native_result(result, Type::Str);
                return Ok(Some(result));
            }
            "__tyrus_temporal_plainDate_from" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `plainDateFrom` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let input = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let date_type = self.llvm_type(&Type::Named("PlainDate".to_string()));
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let result_slot = self
                    .builder
                    .build_alloca(date_type, "temporal.date.from.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_plain_date_from",
                    &[self.string_abi_type(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[input.into(), result_slot.into()],
                        "temporal.plain_date.from",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(date_type, result_slot, "temporal.date.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_plainDate_toString" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `plainDateToString` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let date = self.emit_expr_as(&args[0], &Type::Named("PlainDate".to_string()))?;
                let date_type = self.llvm_type(&Type::Named("PlainDate".to_string()));
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let input_slot =
                    self.native_value_slot(date_type, date, "temporal.date.string.input")?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_plain_date_to_string",
                    &[pointer.into()],
                    Some(self.string_abi_type()),
                );
                let result = self
                    .builder
                    .build_call(function, &[input_slot.into()], "temporal.plain_date.string")
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| "internal: plainDateToString returned no value".to_string())?;
                self.guard_temporal_runtime_error()?;
                self.record_owned_native_result(result, Type::Str);
                return Ok(Some(result));
            }
            "__tyrus_temporal_plainDate_equals" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `plainDateEquals` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let date_type = self.llvm_type(&Type::Named("PlainDate".to_string()));
                let left = self.emit_expr_as(&args[0], &Type::Named("PlainDate".to_string()))?;
                let right = self.emit_expr_as(&args[1], &Type::Named("PlainDate".to_string()))?;
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let left_slot =
                    self.native_value_slot(date_type, left, "temporal.date.equals.left")?;
                let right_slot =
                    self.native_value_slot(date_type, right, "temporal.date.equals.right")?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_plain_date_equals",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let result = self
                    .builder
                    .build_call(
                        function,
                        &[left_slot.into(), right_slot.into()],
                        "temporal.plain_date.equals",
                    )
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| "internal: plainDateEquals returned no value".to_string())?;
                return Ok(Some(result));
            }
            "__tyrus_temporal_plainDate_addDays" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `plainDateAddDays` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let date_type = self.llvm_type(&Type::Named("PlainDate".to_string()));
                let date = self.emit_expr_as(&args[0], &Type::Named("PlainDate".to_string()))?;
                let days = self.emit_expr_as(&args[1], &Type::I64)?;
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let input_slot =
                    self.native_value_slot(date_type, date, "temporal.date.add.input")?;
                let result_slot = self
                    .builder
                    .build_alloca(date_type, "temporal.date.add.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_plain_date_add_days",
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
                        &[input_slot.into(), days.into(), result_slot.into()],
                        "temporal.plain_date.add_days",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(date_type, result_slot, "temporal.date.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_plainDate_dayOfWeek" | "__tyrus_temporal_plainDate_dayOfYear" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `{}` expects 1 argument, found {}",
                        if callee == "__tyrus_temporal_plainDate_dayOfWeek" {
                            "plainDateDayOfWeek"
                        } else {
                            "plainDateDayOfYear"
                        },
                        args.len()
                    )))
                    .map(Some);
                }
                let date_type = self.llvm_type(&Type::Named("PlainDate".to_string()));
                let date = self.emit_expr_as(&args[0], &Type::Named("PlainDate".to_string()))?;
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let input_slot =
                    self.native_value_slot(date_type, date, "temporal.date.query.input")?;
                let runtime_name = if callee == "__tyrus_temporal_plainDate_dayOfWeek" {
                    "tyrus_temporal_plain_date_day_of_week"
                } else {
                    "tyrus_temporal_plain_date_day_of_year"
                };
                let function = self.declare_libc_fn(
                    runtime_name,
                    &[pointer.into()],
                    Some(self.context.i32_type().into()),
                );
                let result = self
                    .builder
                    .build_call(
                        function,
                        &[input_slot.into()],
                        "temporal.plain_date.calendar_field",
                    )
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| {
                        "internal: PlainDate calendar query returned no value".to_string()
                    })?;
                self.guard_temporal_runtime_error()?;
                return Ok(Some(result));
            }
            "__tyrus_temporal_plainDate_addMonths" | "__tyrus_temporal_plainDate_addYears" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `{}` expects 2 arguments, found {}",
                        if callee == "__tyrus_temporal_plainDate_addMonths" {
                            "plainDateAddMonths"
                        } else {
                            "plainDateAddYears"
                        },
                        args.len()
                    )))
                    .map(Some);
                }
                let date_type = self.llvm_type(&Type::Named("PlainDate".to_string()));
                let date = self.emit_expr_as(&args[0], &Type::Named("PlainDate".to_string()))?;
                let amount = self.emit_expr_as(&args[1], &Type::I64)?;
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let input_slot =
                    self.native_value_slot(date_type, date, "temporal.date.calendar.input")?;
                let result_slot = self
                    .builder
                    .build_alloca(date_type, "temporal.date.calendar.result")
                    .map_err(|error| error.to_string())?;
                let runtime_name = if callee == "__tyrus_temporal_plainDate_addMonths" {
                    "tyrus_temporal_plain_date_add_months"
                } else {
                    "tyrus_temporal_plain_date_add_years"
                };
                let function = self.declare_libc_fn(
                    runtime_name,
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
                        &[input_slot.into(), amount.into(), result_slot.into()],
                        "temporal.plain_date.add_calendar_units",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(date_type, result_slot, "temporal.date.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_plainDate_daysUntil" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `plainDateDaysUntil` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let date_type = self.llvm_type(&Type::Named("PlainDate".to_string()));
                let start = self.emit_expr_as(&args[0], &Type::Named("PlainDate".to_string()))?;
                let end = self.emit_expr_as(&args[1], &Type::Named("PlainDate".to_string()))?;
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let start_slot =
                    self.native_value_slot(date_type, start, "temporal.date.until.start")?;
                let end_slot = self.native_value_slot(date_type, end, "temporal.date.until.end")?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_plain_date_days_until",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.i64_type().into()),
                );
                let result = self
                    .builder
                    .build_call(
                        function,
                        &[start_slot.into(), end_slot.into()],
                        "temporal.plain_date.days_until",
                    )
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| "internal: plainDateDaysUntil returned no value".to_string())?;
                self.guard_temporal_runtime_error()?;
                return Ok(Some(result));
            }
            _ => {}
        }
        Ok(None)
    }
}
