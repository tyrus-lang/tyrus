use super::*;

#[path = "zoned_calls/zoned_calls.rs"]
mod zoned_calls;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_temporal_fn_call(
        &mut self,
        callee: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        if let Some(result) = self.emit_temporal_zoned_call(callee, args)? {
            return Ok(Some(result));
        }
        if let Some(result) = self.emit_temporal_constructor_result(callee, args)? {
            return Ok(Some(result));
        }
        match callee {
            "__tyrus_temporal_timezone_from" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `timeZone` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let name = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let function = self.declare_libc_fn(
                    "tyrus_temporal_timezone_from",
                    &[self.string_abi_type()],
                    Some(self.context.i32_type().into()),
                );
                let ordinal = self
                    .builder
                    .build_call(function, &[name.into()], "temporal.timezone")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.guard_temporal_runtime_error()?;
                return Ok(Some(ordinal));
            }
            "__tyrus_temporal_instant_now" => {
                if !args.is_empty() {
                    return (Err(format!(
                        "error: `instantNow` expects 0 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let instant_type = self.llvm_type(&Type::Named("Instant".to_string()));
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let result_slot = self
                    .builder
                    .build_alloca(instant_type, "temporal.instant.now.result")
                    .map_err(|error| error.to_string())?;
                let function =
                    self.declare_libc_fn("tyrus_temporal_instant_now", &[pointer.into()], None);
                self.builder
                    .build_call(function, &[result_slot.into()], "temporal.instant.now")
                    .map_err(|error| error.to_string())?;
                return (self
                    .builder
                    .build_load(instant_type, result_slot, "temporal.instant.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_instant_fromEpochMilliseconds" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `instantFromEpochMilliseconds` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let milliseconds = self.emit_expr_as(&args[0], &Type::I64)?;
                let instant_type = self.llvm_type(&Type::Named("Instant".to_string()));
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let result_slot = self
                    .builder
                    .build_alloca(instant_type, "temporal.instant.from_ms.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_instant_from_epoch_milliseconds",
                    &[self.context.i64_type().into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[milliseconds.into(), result_slot.into()],
                        "temporal.instant.from_ms",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(instant_type, result_slot, "temporal.instant.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_instant_addDuration"
            | "__tyrus_temporal_instant_subtractDuration" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `{}` expects 2 arguments, found {}",
                        callee,
                        args.len()
                    )))
                    .map(Some);
                }
                let instant_type = self.llvm_type(&Type::Named("Instant".to_string()));
                let duration_type = self.llvm_type(&Type::Named("Duration".to_string()));
                let instant = self.emit_expr_as(&args[0], &Type::Named("Instant".to_string()))?;
                let duration = self.emit_expr_as(&args[1], &Type::Named("Duration".to_string()))?;
                let symbol = if callee.ends_with("addDuration") {
                    "tyrus_temporal_instant_add_duration"
                } else {
                    "tyrus_temporal_instant_subtract_duration"
                };
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let instant_slot = self
                    .builder
                    .build_alloca(instant_type, "temporal.instant.duration.input")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(instant_slot, instant)
                    .map_err(|error| error.to_string())?;
                let duration_slot = self.native_value_slot(
                    duration_type,
                    duration,
                    "temporal.instant.duration.value",
                )?;
                let result_slot = self
                    .builder
                    .build_alloca(instant_type, "temporal.instant.duration.result")
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    symbol,
                    &[pointer.into(), pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[
                            instant_slot.into(),
                            duration_slot.into(),
                            result_slot.into(),
                        ],
                        "temporal.instant.duration",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(instant_type, result_slot, "temporal.instant.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_plainDateTime_from"
            | "__tyrus_temporal_plainYearMonth_from"
            | "__tyrus_temporal_plainMonthDay_from" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `{}` expects 1 argument, found {}",
                        callee,
                        args.len()
                    )))
                    .map(Some);
                }
                let type_name = if callee.contains("plainDateTime") {
                    "PlainDateTime"
                } else if callee.contains("plainYearMonth") {
                    "PlainYearMonth"
                } else {
                    "PlainMonthDay"
                };
                let input = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let result_type = self.llvm_type(&Type::Named(type_name.to_string()));
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let symbol = if callee.contains("plainDateTime") {
                    "tyrus_temporal_plain_date_time_from"
                } else if callee.contains("plainYearMonth") {
                    "tyrus_temporal_plain_year_month_from"
                } else {
                    "tyrus_temporal_plain_month_day_from"
                };
                let result_slot = self
                    .builder
                    .build_alloca(result_type, "temporal.composite.from.result")
                    .map_err(|error| error.to_string())?;
                let function =
                    self.declare_libc_fn(symbol, &[self.string_abi_type(), pointer.into()], None);
                self.builder
                    .build_call(
                        function,
                        &[input.into(), result_slot.into()],
                        "temporal.composite.from",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(result_type, result_slot, "temporal.composite.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_plainDateTime_fromFields"
            | "__tyrus_temporal_plainYearMonth_fromFields"
            | "__tyrus_temporal_plainMonthDay_fromFields" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `{}` expects 1 argument, found {}",
                        callee,
                        args.len()
                    )))
                    .map(Some);
                }
                let type_name = if callee.contains("plainDateTime") {
                    "PlainDateTime"
                } else if callee.contains("plainYearMonth") {
                    "PlainYearMonth"
                } else {
                    "PlainMonthDay"
                };
                let value = self.emit_expr_as(&args[0], &Type::Named(type_name.to_string()))?;
                let value_type = self.llvm_type(&Type::Named(type_name.to_string()));
                let symbol = if callee.contains("plainDateTime") {
                    "tyrus_temporal_plain_date_time_from_fields"
                } else if callee.contains("plainYearMonth") {
                    "tyrus_temporal_plain_year_month_from_fields"
                } else {
                    "tyrus_temporal_plain_month_day_from_fields"
                };
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let input_slot = self
                    .builder
                    .build_alloca(value_type, "temporal.composite.fields.input")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(input_slot, value)
                    .map_err(|error| error.to_string())?;
                let result_slot = self
                    .builder
                    .build_alloca(value_type, "temporal.composite.fields.result")
                    .map_err(|error| error.to_string())?;
                let function =
                    self.declare_libc_fn(symbol, &[self.string_abi_type(), pointer.into()], None);
                self.builder
                    .build_call(
                        function,
                        &[input_slot.into(), result_slot.into()],
                        "temporal.composite.fields",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(value_type, result_slot, "temporal.composite.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            "__tyrus_temporal_plainDateTime_toString"
            | "__tyrus_temporal_plainYearMonth_toString"
            | "__tyrus_temporal_plainMonthDay_toString" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `{}` expects 1 argument, found {}",
                        callee,
                        args.len()
                    )))
                    .map(Some);
                }
                let type_name = if callee.contains("plainDateTime") {
                    "PlainDateTime"
                } else if callee.contains("plainYearMonth") {
                    "PlainYearMonth"
                } else {
                    "PlainMonthDay"
                };
                let value = self.emit_expr_as(&args[0], &Type::Named(type_name.to_string()))?;
                let value_type = self.llvm_type(&Type::Named(type_name.to_string()));
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let symbol = if callee.contains("plainDateTime") {
                    "tyrus_temporal_plain_date_time_to_string"
                } else if callee.contains("plainYearMonth") {
                    "tyrus_temporal_plain_year_month_to_string"
                } else {
                    "tyrus_temporal_plain_month_day_to_string"
                };
                let slot = self
                    .builder
                    .build_alloca(value_type, "temporal.composite.toString.input")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(slot, value)
                    .map_err(|error| error.to_string())?;
                let function =
                    self.declare_libc_fn(symbol, &[pointer.into()], Some(self.string_abi_type()));
                let result = self
                    .builder
                    .build_call(function, &[slot.into()], "temporal.composite.toString")
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| {
                        "internal: temporal composite formatter returned no value".to_string()
                    })?;
                self.guard_temporal_runtime_error()?;
                self.record_owned_native_result(result, Type::Str);
                return Ok(Some(result));
            }
            "__tyrus_temporal_plainDateTime_equals" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `plainDateTimeEquals` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let ty = self.llvm_type(&Type::Named("PlainDateTime".to_string()));
                let left =
                    self.emit_expr_as(&args[0], &Type::Named("PlainDateTime".to_string()))?;
                let right =
                    self.emit_expr_as(&args[1], &Type::Named("PlainDateTime".to_string()))?;
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let left_slot = self
                    .builder
                    .build_alloca(ty, "temporal.datetime.equals.left")
                    .map_err(|error| error.to_string())?;
                let right_slot = self
                    .builder
                    .build_alloca(ty, "temporal.datetime.equals.right")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(left_slot, left)
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(right_slot, right)
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_temporal_plain_date_time_equals",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let result = self
                    .builder
                    .build_call(
                        function,
                        &[left_slot.into(), right_slot.into()],
                        "temporal.datetime.equals",
                    )
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| "internal: plainDateTimeEquals returned no value".to_string())?;
                self.guard_temporal_runtime_error()?;
                return Ok(Some(result));
            }
            "__tyrus_temporal_plainDateTime_with"
            | "__tyrus_temporal_plainDateTime_addDuration"
            | "__tyrus_temporal_plainDateTime_subtractDuration" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `{}` expects 2 arguments, found {}",
                        callee,
                        args.len()
                    )))
                    .map(Some);
                }
                let value_type = self.llvm_type(&Type::Named("PlainDateTime".to_string()));
                let left =
                    self.emit_expr_as(&args[0], &Type::Named("PlainDateTime".to_string()))?;
                let right_type = if callee.ends_with("_with") {
                    "PlainDateTime"
                } else {
                    "Duration"
                };
                let right = self.emit_expr_as(&args[1], &Type::Named(right_type.to_string()))?;
                let symbol = if callee.ends_with("_with") {
                    "tyrus_temporal_plain_date_time_with"
                } else if callee.contains("subtract") {
                    "tyrus_temporal_plain_date_time_subtract_duration"
                } else {
                    "tyrus_temporal_plain_date_time_add_duration"
                };
                let right_llvm = self.llvm_type(&Type::Named(right_type.to_string()));
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let left_slot = self
                    .builder
                    .build_alloca(value_type, "temporal.datetime.operation.left")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(left_slot, left)
                    .map_err(|error| error.to_string())?;
                let right_slot =
                    self.native_value_slot(right_llvm, right, "temporal.datetime.operation.right")?;
                let result_slot = self
                    .builder
                    .build_alloca(value_type, "temporal.datetime.operation.result")
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
                        "temporal.datetime.operation",
                    )
                    .map_err(|error| error.to_string())?;
                self.guard_temporal_runtime_error()?;
                return (self
                    .builder
                    .build_load(value_type, result_slot, "temporal.datetime.operation.value")
                    .map_err(|error| error.to_string()))
                .map(Some);
            }
            _ => {}
        }
        Ok(None)
    }

    fn emit_temporal_constructor_result(
        &mut self,
        callee: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let (symbol, name, input_types, success_ty) = match callee {
            "__tyrus_temporal_timezone_from_result" => (
                "tyrus_temporal_timezone_from_result",
                "timeZone",
                vec![Type::Str],
                Type::Named("TimeZone".to_string()),
            ),
            "__tyrus_temporal_instant_from_result" => (
                "tyrus_temporal_instant_from_result",
                "instantFrom",
                vec![Type::Str],
                Type::Named("Instant".to_string()),
            ),
            "__tyrus_temporal_instant_fromEpochMilliseconds_result" => (
                "tyrus_temporal_instant_from_epoch_milliseconds_result",
                "instantFromEpochMilliseconds",
                vec![Type::I64],
                Type::Named("Instant".to_string()),
            ),
            "__tyrus_temporal_instant_toEpochMilliseconds_result" => (
                "tyrus_temporal_instant_to_epoch_milliseconds_result",
                "instantToEpochMilliseconds",
                vec![Type::Named("Instant".to_string())],
                Type::I64,
            ),
            "__tyrus_temporal_instant_toString_result" => (
                "tyrus_temporal_instant_to_string_result",
                "instantToString",
                vec![Type::Named("Instant".to_string())],
                Type::Str,
            ),
            "__tyrus_temporal_instant_addDuration_result" => (
                "tyrus_temporal_instant_add_duration_result",
                "instantAddDuration",
                vec![
                    Type::Named("Instant".to_string()),
                    Type::Named("Duration".to_string()),
                ],
                Type::Named("Instant".to_string()),
            ),
            "__tyrus_temporal_instant_subtractDuration_result" => (
                "tyrus_temporal_instant_subtract_duration_result",
                "instantSubtractDuration",
                vec![
                    Type::Named("Instant".to_string()),
                    Type::Named("Duration".to_string()),
                ],
                Type::Named("Instant".to_string()),
            ),
            "__tyrus_temporal_plainDate_from_result" => (
                "tyrus_temporal_plain_date_from_result",
                "plainDateFrom",
                vec![Type::Str],
                Type::Named("PlainDate".to_string()),
            ),
            "__tyrus_temporal_plainTime_from_result" => (
                "tyrus_temporal_plain_time_from_result",
                "plainTimeFrom",
                vec![Type::Str],
                Type::Named("PlainTime".to_string()),
            ),
            "__tyrus_temporal_plainDateTime_from_result" => (
                "tyrus_temporal_plain_date_time_from_result",
                "plainDateTimeFrom",
                vec![Type::Str],
                Type::Named("PlainDateTime".to_string()),
            ),
            "__tyrus_temporal_plainYearMonth_from_result" => (
                "tyrus_temporal_plain_year_month_from_result",
                "plainYearMonthFrom",
                vec![Type::Str],
                Type::Named("PlainYearMonth".to_string()),
            ),
            "__tyrus_temporal_plainMonthDay_from_result" => (
                "tyrus_temporal_plain_month_day_from_result",
                "plainMonthDayFrom",
                vec![Type::Str],
                Type::Named("PlainMonthDay".to_string()),
            ),
            "__tyrus_temporal_plainDate_fromFields_result" => (
                "tyrus_temporal_plain_date_from_fields_result",
                "plainDateFromFields",
                vec![Type::Named("PlainDate".to_string())],
                Type::Named("PlainDate".to_string()),
            ),
            "__tyrus_temporal_plainDate_with_result" => (
                "tyrus_temporal_plain_date_with_result",
                "plainDateWith",
                vec![
                    Type::Named("PlainDate".to_string()),
                    Type::Named("PlainDate".to_string()),
                ],
                Type::Named("PlainDate".to_string()),
            ),
            "__tyrus_temporal_plainDate_subtractDuration_result" => (
                "tyrus_temporal_plain_date_subtract_duration_result",
                "plainDateSubtractDuration",
                vec![
                    Type::Named("PlainDate".to_string()),
                    Type::Named("Duration".to_string()),
                ],
                Type::Named("PlainDate".to_string()),
            ),
            "__tyrus_temporal_plainDate_until_result" => (
                "tyrus_temporal_plain_date_until_result",
                "plainDateUntil",
                vec![
                    Type::Named("PlainDate".to_string()),
                    Type::Named("PlainDate".to_string()),
                ],
                Type::Named("Duration".to_string()),
            ),
            "__tyrus_temporal_plainDate_since_result" => (
                "tyrus_temporal_plain_date_since_result",
                "plainDateSince",
                vec![
                    Type::Named("PlainDate".to_string()),
                    Type::Named("PlainDate".to_string()),
                ],
                Type::Named("Duration".to_string()),
            ),
            "__tyrus_temporal_plainDate_toString_result" => (
                "tyrus_temporal_plain_date_to_string_result",
                "plainDateToString",
                vec![Type::Named("PlainDate".to_string())],
                Type::Str,
            ),
            "__tyrus_temporal_plainDate_addDays_result" => (
                "tyrus_temporal_plain_date_add_days_result",
                "plainDateAddDays",
                vec![Type::Named("PlainDate".to_string()), Type::I64],
                Type::Named("PlainDate".to_string()),
            ),
            "__tyrus_temporal_plainDate_dayOfWeek_result" => (
                "tyrus_temporal_plain_date_day_of_week_result",
                "plainDateDayOfWeek",
                vec![Type::Named("PlainDate".to_string())],
                Type::U32,
            ),
            "__tyrus_temporal_plainDate_dayOfYear_result" => (
                "tyrus_temporal_plain_date_day_of_year_result",
                "plainDateDayOfYear",
                vec![Type::Named("PlainDate".to_string())],
                Type::U32,
            ),
            "__tyrus_temporal_plainDate_addMonths_result" => (
                "tyrus_temporal_plain_date_add_months_result",
                "plainDateAddMonths",
                vec![Type::Named("PlainDate".to_string()), Type::I64],
                Type::Named("PlainDate".to_string()),
            ),
            "__tyrus_temporal_plainDate_addYears_result" => (
                "tyrus_temporal_plain_date_add_years_result",
                "plainDateAddYears",
                vec![Type::Named("PlainDate".to_string()), Type::I64],
                Type::Named("PlainDate".to_string()),
            ),
            "__tyrus_temporal_plainDate_daysUntil_result" => (
                "tyrus_temporal_plain_date_days_until_result",
                "plainDateDaysUntil",
                vec![
                    Type::Named("PlainDate".to_string()),
                    Type::Named("PlainDate".to_string()),
                ],
                Type::I64,
            ),
            "__tyrus_temporal_plainDate_addDuration_result" => (
                "tyrus_temporal_plain_date_add_duration_result",
                "plainDateAddDuration",
                vec![
                    Type::Named("PlainDate".to_string()),
                    Type::Named("Duration".to_string()),
                ],
                Type::Named("PlainDate".to_string()),
            ),
            "__tyrus_temporal_plainDate_untilWithOptions_result" => (
                "tyrus_temporal_plain_date_until_with_options_result",
                "plainDateUntilWithOptions",
                vec![
                    Type::Named("PlainDate".to_string()),
                    Type::Named("PlainDate".to_string()),
                    Type::Str,
                    Type::Str,
                    Type::I64,
                    Type::Str,
                ],
                Type::Named("Duration".to_string()),
            ),
            "__tyrus_temporal_plainDate_sinceWithOptions_result" => (
                "tyrus_temporal_plain_date_since_with_options_result",
                "plainDateSinceWithOptions",
                vec![
                    Type::Named("PlainDate".to_string()),
                    Type::Named("PlainDate".to_string()),
                    Type::Str,
                    Type::Str,
                    Type::I64,
                    Type::Str,
                ],
                Type::Named("Duration".to_string()),
            ),
            "__tyrus_temporal_plainTime_fromFields_result" => (
                "tyrus_temporal_plain_time_from_fields_result",
                "plainTimeFromFields",
                vec![Type::Named("PlainTime".to_string())],
                Type::Named("PlainTime".to_string()),
            ),
            "__tyrus_temporal_plainTime_subtractDuration_result" => (
                "tyrus_temporal_plain_time_subtract_duration_result",
                "plainTimeSubtractDuration",
                vec![
                    Type::Named("PlainTime".to_string()),
                    Type::Named("Duration".to_string()),
                ],
                Type::Named("PlainTime".to_string()),
            ),
            "__tyrus_temporal_plainTime_until_result" => (
                "tyrus_temporal_plain_time_until_result",
                "plainTimeUntil",
                vec![
                    Type::Named("PlainTime".to_string()),
                    Type::Named("PlainTime".to_string()),
                ],
                Type::Named("Duration".to_string()),
            ),
            "__tyrus_temporal_plainTime_since_result" => (
                "tyrus_temporal_plain_time_since_result",
                "plainTimeSince",
                vec![
                    Type::Named("PlainTime".to_string()),
                    Type::Named("PlainTime".to_string()),
                ],
                Type::Named("Duration".to_string()),
            ),
            "__tyrus_temporal_plainTime_toString_result" => (
                "tyrus_temporal_plain_time_to_string_result",
                "plainTimeToString",
                vec![Type::Named("PlainTime".to_string())],
                Type::Str,
            ),
            "__tyrus_temporal_plainTime_addSeconds_result" => (
                "tyrus_temporal_plain_time_add_seconds_result",
                "plainTimeAddSeconds",
                vec![Type::Named("PlainTime".to_string()), Type::I64],
                Type::Named("PlainTime".to_string()),
            ),
            "__tyrus_temporal_plainTime_secondsUntil_result" => (
                "tyrus_temporal_plain_time_seconds_until_result",
                "plainTimeSecondsUntil",
                vec![
                    Type::Named("PlainTime".to_string()),
                    Type::Named("PlainTime".to_string()),
                ],
                Type::I64,
            ),
            "__tyrus_temporal_plainTime_addNanoseconds_result" => (
                "tyrus_temporal_plain_time_add_nanoseconds_result",
                "plainTimeAddNanoseconds",
                vec![Type::Named("PlainTime".to_string()), Type::I64],
                Type::Named("PlainTime".to_string()),
            ),
            "__tyrus_temporal_plainTime_nanosecondsUntil_result" => (
                "tyrus_temporal_plain_time_nanoseconds_until_result",
                "plainTimeNanosecondsUntil",
                vec![
                    Type::Named("PlainTime".to_string()),
                    Type::Named("PlainTime".to_string()),
                ],
                Type::I64,
            ),
            "__tyrus_temporal_plainTime_addDuration_result" => (
                "tyrus_temporal_plain_time_add_duration_result",
                "plainTimeAddDuration",
                vec![
                    Type::Named("PlainTime".to_string()),
                    Type::Named("Duration".to_string()),
                ],
                Type::Named("PlainTime".to_string()),
            ),
            "__tyrus_temporal_plainTime_untilWithOptions_result" => (
                "tyrus_temporal_plain_time_until_with_options_result",
                "plainTimeUntilWithOptions",
                vec![
                    Type::Named("PlainTime".to_string()),
                    Type::Named("PlainTime".to_string()),
                    Type::Str,
                    Type::Str,
                    Type::I64,
                    Type::Str,
                ],
                Type::Named("Duration".to_string()),
            ),
            "__tyrus_temporal_plainTime_sinceWithOptions_result" => (
                "tyrus_temporal_plain_time_since_with_options_result",
                "plainTimeSinceWithOptions",
                vec![
                    Type::Named("PlainTime".to_string()),
                    Type::Named("PlainTime".to_string()),
                    Type::Str,
                    Type::Str,
                    Type::I64,
                    Type::Str,
                ],
                Type::Named("Duration".to_string()),
            ),
            "__tyrus_temporal_plainDateTime_fromFields_result" => (
                "tyrus_temporal_plain_date_time_from_fields_result",
                "plainDateTimeFromFields",
                vec![Type::Named("PlainDateTime".to_string())],
                Type::Named("PlainDateTime".to_string()),
            ),
            "__tyrus_temporal_plainDateTime_toString_result" => (
                "tyrus_temporal_plain_date_time_to_string_result",
                "plainDateTimeToString",
                vec![Type::Named("PlainDateTime".to_string())],
                Type::Str,
            ),
            "__tyrus_temporal_plainDateTime_with_result" => (
                "tyrus_temporal_plain_date_time_with_result",
                "plainDateTimeWith",
                vec![
                    Type::Named("PlainDateTime".to_string()),
                    Type::Named("PlainDateTime".to_string()),
                ],
                Type::Named("PlainDateTime".to_string()),
            ),
            "__tyrus_temporal_plainDateTime_addDuration_result" => (
                "tyrus_temporal_plain_date_time_add_duration_result",
                "plainDateTimeAddDuration",
                vec![
                    Type::Named("PlainDateTime".to_string()),
                    Type::Named("Duration".to_string()),
                ],
                Type::Named("PlainDateTime".to_string()),
            ),
            "__tyrus_temporal_plainDateTime_subtractDuration_result" => (
                "tyrus_temporal_plain_date_time_subtract_duration_result",
                "plainDateTimeSubtractDuration",
                vec![
                    Type::Named("PlainDateTime".to_string()),
                    Type::Named("Duration".to_string()),
                ],
                Type::Named("PlainDateTime".to_string()),
            ),
            "__tyrus_temporal_plainDateTime_until_result" => (
                "tyrus_temporal_plain_date_time_until_result",
                "plainDateTimeUntil",
                vec![
                    Type::Named("PlainDateTime".to_string()),
                    Type::Named("PlainDateTime".to_string()),
                ],
                Type::Named("Duration".to_string()),
            ),
            "__tyrus_temporal_plainDateTime_since_result" => (
                "tyrus_temporal_plain_date_time_since_result",
                "plainDateTimeSince",
                vec![
                    Type::Named("PlainDateTime".to_string()),
                    Type::Named("PlainDateTime".to_string()),
                ],
                Type::Named("Duration".to_string()),
            ),
            "__tyrus_temporal_plainDateTime_untilWithOptions_result" => (
                "tyrus_temporal_plain_date_time_until_with_options_result",
                "plainDateTimeUntilWithOptions",
                vec![
                    Type::Named("PlainDateTime".to_string()),
                    Type::Named("PlainDateTime".to_string()),
                    Type::Str,
                    Type::Str,
                    Type::I64,
                    Type::Str,
                ],
                Type::Named("Duration".to_string()),
            ),
            "__tyrus_temporal_plainDateTime_sinceWithOptions_result" => (
                "tyrus_temporal_plain_date_time_since_with_options_result",
                "plainDateTimeSinceWithOptions",
                vec![
                    Type::Named("PlainDateTime".to_string()),
                    Type::Named("PlainDateTime".to_string()),
                    Type::Str,
                    Type::Str,
                    Type::I64,
                    Type::Str,
                ],
                Type::Named("Duration".to_string()),
            ),
            "__tyrus_temporal_plainYearMonth_fromFields_result" => (
                "tyrus_temporal_plain_year_month_from_fields_result",
                "plainYearMonthFromFields",
                vec![Type::Named("PlainYearMonth".to_string())],
                Type::Named("PlainYearMonth".to_string()),
            ),
            "__tyrus_temporal_plainYearMonth_toString_result" => (
                "tyrus_temporal_plain_year_month_to_string_result",
                "plainYearMonthToString",
                vec![Type::Named("PlainYearMonth".to_string())],
                Type::Str,
            ),
            "__tyrus_temporal_plainYearMonth_add_result" => (
                "tyrus_temporal_plain_year_month_add_result",
                "plainYearMonthAdd",
                vec![Type::Named("PlainYearMonth".to_string()), Type::I64],
                Type::Named("PlainYearMonth".to_string()),
            ),
            "__tyrus_temporal_plainMonthDay_fromFields_result" => (
                "tyrus_temporal_plain_month_day_from_fields_result",
                "plainMonthDayFromFields",
                vec![Type::Named("PlainMonthDay".to_string())],
                Type::Named("PlainMonthDay".to_string()),
            ),
            "__tyrus_temporal_plainMonthDay_toString_result" => (
                "tyrus_temporal_plain_month_day_to_string_result",
                "plainMonthDayToString",
                vec![Type::Named("PlainMonthDay".to_string())],
                Type::Str,
            ),
            "__tyrus_temporal_plainMonthDay_toPlainDate_result" => (
                "tyrus_temporal_plain_month_day_to_plain_date_result",
                "plainMonthDayToPlainDate",
                vec![Type::Named("PlainMonthDay".to_string()), Type::I32],
                Type::Named("PlainDate".to_string()),
            ),
            "__tyrus_temporal_zonedDateTime_from_result" => (
                "tyrus_temporal_zoned_date_time_from_result",
                "zonedDateTimeFrom",
                vec![Type::Str],
                Type::Named("ZonedDateTime".to_string()),
            ),
            "__tyrus_temporal_zonedDateTime_toInstant_result" => (
                "tyrus_temporal_zoned_date_time_to_instant_result",
                "zonedDateTimeToInstant",
                vec![Type::Named("ZonedDateTime".to_string())],
                Type::Named("Instant".to_string()),
            ),
            "__tyrus_temporal_zonedDateTime_toString_result" => (
                "tyrus_temporal_zoned_date_time_to_string_result",
                "zonedDateTimeToString",
                vec![Type::Named("ZonedDateTime".to_string())],
                Type::Str,
            ),
            "__tyrus_temporal_zonedDateTime_addDuration_result" => (
                "tyrus_temporal_zoned_date_time_add_duration_result",
                "zonedDateTimeAddDuration",
                vec![
                    Type::Named("ZonedDateTime".to_string()),
                    Type::Named("Duration".to_string()),
                ],
                Type::Named("ZonedDateTime".to_string()),
            ),
            "__tyrus_temporal_zonedDateTime_until_result" => (
                "tyrus_temporal_zoned_date_time_until_result",
                "zonedDateTimeUntil",
                vec![
                    Type::Named("ZonedDateTime".to_string()),
                    Type::Named("ZonedDateTime".to_string()),
                ],
                Type::Named("Duration".to_string()),
            ),
            "__tyrus_temporal_zonedDateTime_format_result" => (
                "tyrus_temporal_zoned_date_time_format_result",
                "zonedDateTimeFormat",
                vec![Type::Named("ZonedDateTime".to_string()), Type::Str],
                Type::Str,
            ),
            "__tyrus_temporal_instant_toZonedDateTime_result" => (
                "tyrus_temporal_instant_to_zoned_date_time_result",
                "instantToZonedDateTime",
                vec![
                    Type::Named("Instant".to_string()),
                    Type::Named("TimeZone".to_string()),
                ],
                Type::Named("ZonedDateTime".to_string()),
            ),
            "__tyrus_temporal_duration_total_result" => (
                "tyrus_temporal_duration_total_result",
                "durationTotal",
                vec![Type::Named("Duration".to_string()), Type::Str],
                Type::I64,
            ),
            "__tyrus_temporal_duration_round_result" => (
                "tyrus_temporal_duration_round_result",
                "durationRound",
                vec![
                    Type::Named("Duration".to_string()),
                    Type::Str,
                    Type::I64,
                    Type::Str,
                    Type::Named("PlainDate".to_string()),
                ],
                Type::Named("Duration".to_string()),
            ),
            _ => return Ok(None),
        };
        if args.len() != input_types.len() {
            return Err(format!(
                "error: `{name}` expects {} argument(s), found {}",
                input_types.len(),
                args.len()
            ));
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let mut abi_params = Vec::with_capacity(input_types.len() + 1);
        let mut call_args = Vec::with_capacity(input_types.len() + 1);
        let mut strings = Vec::new();
        for (index, (argument, input_ty)) in args.iter().zip(&input_types).enumerate() {
            let value = self.emit_expr_as(argument, input_ty)?;
            if matches!(input_ty, Type::Named(_)) {
                let value_type = self.llvm_type(input_ty);
                let slot = self.native_value_slot(
                    value_type,
                    value,
                    &format!("temporal.result.argument.{index}"),
                )?;
                abi_params.push(pointer.into());
                call_args.push(slot.into());
            } else {
                abi_params.push(self.llvm_type(input_ty));
                call_args.push(value.into());
                if input_ty == &Type::Str {
                    strings.push((value.into_struct_value(), argument));
                }
            }
        }
        let result_ty = Type::Result {
            success: Box::new(success_ty),
            error: Box::new(Type::Named("TemporalError".to_string())),
        };
        let result_llvm_ty = self.llvm_type(&result_ty);
        let slot = self
            .builder
            .build_alloca(result_llvm_ty, "temporal.result")
            .map_err(|error| error.to_string())?;
        abi_params.push(pointer.into());
        call_args.push(slot.into());
        let function = self.declare_libc_fn(symbol, &abi_params, None);
        self.builder
            .build_call(function, &call_args, "temporal.result.call")
            .map_err(|error| error.to_string())?;
        for (value, argument) in strings {
            self.emit_native_string_argument_release(value, argument)?;
        }
        let result = self
            .builder
            .build_load(result_llvm_ty, slot, "temporal.result.value")
            .map_err(|error| error.to_string())?;
        self.record_owned_native_result(result, result_ty);
        Ok(Some(result))
    }
}
