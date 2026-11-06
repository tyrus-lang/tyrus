use super::*;

#[allow(clippy::too_many_lines)]
pub(crate) fn temporal_builtins() -> HashMap<String, ExportInfo> {
    [
        (
            "TimeZone".to_string(),
            ExportInfo {
                target: "TimeZone".to_string(),
                ty: Type::Named("TimeZone".to_string()),
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: Some(time_zone_enum()),
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "TemporalError".to_string(),
            ExportInfo {
                target: "TemporalError".to_string(),
                ty: Type::Named("TemporalError".to_string()),
                interface_fields: Some(crate::builtin_interfaces::temporal_error_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "Instant".to_string(),
            ExportInfo {
                target: "Instant".to_string(),
                ty: Type::Named("Instant".to_string()),
                interface_fields: Some(crate::builtin_interfaces::instant_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "ZonedDateTime".to_string(),
            ExportInfo {
                target: "ZonedDateTime".to_string(),
                ty: Type::Named("ZonedDateTime".to_string()),
                interface_fields: Some(crate::builtin_interfaces::zoned_date_time_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "zonedDateTimeFrom".to_string(),
            temporal_result_function(
                "__tyrus_temporal_zonedDateTime_from_result",
                vec![Type::Str],
                Type::Named("ZonedDateTime".to_string()),
            ),
        ),
        (
            "zonedDateTimeToInstant".to_string(),
            temporal_result_function(
                "__tyrus_temporal_zonedDateTime_toInstant_result",
                vec![Type::Named("ZonedDateTime".to_string())],
                Type::Named("Instant".to_string()),
            ),
        ),
        (
            "zonedDateTimeToString".to_string(),
            temporal_result_function(
                "__tyrus_temporal_zonedDateTime_toString_result",
                vec![Type::Named("ZonedDateTime".to_string())],
                Type::Str,
            ),
        ),
        (
            "zonedDateTimeEquals".to_string(),
            ExportInfo {
                target: "__tyrus_temporal_zonedDateTime_equals".to_string(),
                ty: Type::Function {
                    params: vec![
                        Type::Named("ZonedDateTime".to_string()),
                        Type::Named("ZonedDateTime".to_string()),
                    ],
                    ret: Box::new(Type::Bool),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "zonedDateTimeAddDuration".to_string(),
            temporal_result_function(
                "__tyrus_temporal_zonedDateTime_addDuration_result",
                vec![
                    Type::Named("ZonedDateTime".to_string()),
                    Type::Named("Duration".to_string()),
                ],
                Type::Named("ZonedDateTime".to_string()),
            ),
        ),
        (
            "zonedDateTimeUntil".to_string(),
            temporal_result_function(
                "__tyrus_temporal_zonedDateTime_until_result",
                vec![
                    Type::Named("ZonedDateTime".to_string()),
                    Type::Named("ZonedDateTime".to_string()),
                ],
                Type::Named("Duration".to_string()),
            ),
        ),
        (
            "zonedDateTimeFormat".to_string(),
            temporal_result_function(
                "__tyrus_temporal_zonedDateTime_format_result",
                vec![Type::Named("ZonedDateTime".to_string()), Type::Str],
                Type::Str,
            ),
        ),
        (
            "PlainDate".to_string(),
            ExportInfo {
                target: "PlainDate".to_string(),
                ty: Type::Named("PlainDate".to_string()),
                interface_fields: Some(crate::builtin_interfaces::plain_date_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "PlainTime".to_string(),
            ExportInfo {
                target: "PlainTime".to_string(),
                ty: Type::Named("PlainTime".to_string()),
                interface_fields: Some(crate::builtin_interfaces::plain_time_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "Duration".to_string(),
            ExportInfo {
                target: "Duration".to_string(),
                ty: Type::Named("Duration".to_string()),
                interface_fields: Some(crate::builtin_interfaces::duration_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "PlainDateTime".to_string(),
            ExportInfo {
                target: "PlainDateTime".to_string(),
                ty: Type::Named("PlainDateTime".to_string()),
                interface_fields: Some(crate::builtin_interfaces::plain_date_time_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "PlainYearMonth".to_string(),
            ExportInfo {
                target: "PlainYearMonth".to_string(),
                ty: Type::Named("PlainYearMonth".to_string()),
                interface_fields: Some(crate::builtin_interfaces::plain_year_month_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "PlainMonthDay".to_string(),
            ExportInfo {
                target: "PlainMonthDay".to_string(),
                ty: Type::Named("PlainMonthDay".to_string()),
                interface_fields: Some(crate::builtin_interfaces::plain_month_day_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "plainDateTimeFrom".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDateTime_from_result",
                vec![Type::Str],
                Type::Named("PlainDateTime".to_string()),
            ),
        ),
        (
            "plainDateTimeFromFields".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDateTime_fromFields_result",
                vec![Type::Named("PlainDateTime".to_string())],
                Type::Named("PlainDateTime".to_string()),
            ),
        ),
        (
            "plainDateTimeToString".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDateTime_toString_result",
                vec![Type::Named("PlainDateTime".to_string())],
                Type::Str,
            ),
        ),
        (
            "plainDateTimeEquals".to_string(),
            temporal_function(
                "__tyrus_temporal_plainDateTime_equals",
                vec![
                    Type::Named("PlainDateTime".to_string()),
                    Type::Named("PlainDateTime".to_string()),
                ],
                Type::Bool,
            ),
        ),
        (
            "plainDateTimeWith".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDateTime_with_result",
                vec![
                    Type::Named("PlainDateTime".to_string()),
                    Type::Named("PlainDateTime".to_string()),
                ],
                Type::Named("PlainDateTime".to_string()),
            ),
        ),
        (
            "plainDateTimeAddDuration".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDateTime_addDuration_result",
                vec![
                    Type::Named("PlainDateTime".to_string()),
                    Type::Named("Duration".to_string()),
                ],
                Type::Named("PlainDateTime".to_string()),
            ),
        ),
        (
            "plainDateTimeSubtractDuration".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDateTime_subtractDuration_result",
                vec![
                    Type::Named("PlainDateTime".to_string()),
                    Type::Named("Duration".to_string()),
                ],
                Type::Named("PlainDateTime".to_string()),
            ),
        ),
        (
            "plainDateTimeUntil".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDateTime_until_result",
                vec![
                    Type::Named("PlainDateTime".to_string()),
                    Type::Named("PlainDateTime".to_string()),
                ],
                Type::Named("Duration".to_string()),
            ),
        ),
        (
            "plainDateTimeSince".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDateTime_since_result",
                vec![
                    Type::Named("PlainDateTime".to_string()),
                    Type::Named("PlainDateTime".to_string()),
                ],
                Type::Named("Duration".to_string()),
            ),
        ),
        (
            "plainYearMonthFrom".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainYearMonth_from_result",
                vec![Type::Str],
                Type::Named("PlainYearMonth".to_string()),
            ),
        ),
        (
            "plainYearMonthFromFields".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainYearMonth_fromFields_result",
                vec![Type::Named("PlainYearMonth".to_string())],
                Type::Named("PlainYearMonth".to_string()),
            ),
        ),
        (
            "plainYearMonthToString".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainYearMonth_toString_result",
                vec![Type::Named("PlainYearMonth".to_string())],
                Type::Str,
            ),
        ),
        (
            "plainYearMonthAdd".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainYearMonth_add_result",
                vec![Type::Named("PlainYearMonth".to_string()), Type::I64],
                Type::Named("PlainYearMonth".to_string()),
            ),
        ),
        (
            "plainMonthDayFrom".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainMonthDay_from_result",
                vec![Type::Str],
                Type::Named("PlainMonthDay".to_string()),
            ),
        ),
        (
            "plainMonthDayFromFields".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainMonthDay_fromFields_result",
                vec![Type::Named("PlainMonthDay".to_string())],
                Type::Named("PlainMonthDay".to_string()),
            ),
        ),
        (
            "plainMonthDayToString".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainMonthDay_toString_result",
                vec![Type::Named("PlainMonthDay".to_string())],
                Type::Str,
            ),
        ),
        (
            "plainMonthDayToPlainDate".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainMonthDay_toPlainDate_result",
                vec![Type::Named("PlainMonthDay".to_string()), Type::I32],
                Type::Named("PlainDate".to_string()),
            ),
        ),
        (
            "durationRound".to_string(),
            temporal_result_function(
                "__tyrus_temporal_duration_round_result",
                vec![
                    Type::Named("Duration".to_string()),
                    Type::Str,
                    Type::I64,
                    Type::Str,
                    Type::Named("PlainDate".to_string()),
                ],
                Type::Named("Duration".to_string()),
            ),
        ),
        (
            "instantNow".to_string(),
            ExportInfo {
                target: "__tyrus_temporal_instant_now".to_string(),
                ty: Type::Function {
                    params: Vec::new(),
                    ret: Box::new(Type::Named("Instant".to_string())),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "instantFrom".to_string(),
            ExportInfo {
                target: "__tyrus_temporal_instant_from_result".to_string(),
                ty: Type::Function {
                    params: vec![Type::Str],
                    ret: Box::new(Type::Result {
                        success: Box::new(Type::Named("Instant".to_string())),
                        error: Box::new(Type::Named("TemporalError".to_string())),
                    }),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "instantFromEpochMilliseconds".to_string(),
            ExportInfo {
                target: "__tyrus_temporal_instant_fromEpochMilliseconds_result".to_string(),
                ty: Type::Function {
                    params: vec![Type::I64],
                    ret: Box::new(Type::Result {
                        success: Box::new(Type::Named("Instant".to_string())),
                        error: Box::new(Type::Named("TemporalError".to_string())),
                    }),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "instantToEpochMilliseconds".to_string(),
            temporal_result_function(
                "__tyrus_temporal_instant_toEpochMilliseconds_result",
                vec![Type::Named("Instant".to_string())],
                Type::I64,
            ),
        ),
        (
            "instantToString".to_string(),
            temporal_result_function(
                "__tyrus_temporal_instant_toString_result",
                vec![Type::Named("Instant".to_string())],
                Type::Str,
            ),
        ),
        (
            "instantAddDuration".to_string(),
            temporal_result_function(
                "__tyrus_temporal_instant_addDuration_result",
                vec![
                    Type::Named("Instant".to_string()),
                    Type::Named("Duration".to_string()),
                ],
                Type::Named("Instant".to_string()),
            ),
        ),
        (
            "instantSubtractDuration".to_string(),
            temporal_result_function(
                "__tyrus_temporal_instant_subtractDuration_result",
                vec![
                    Type::Named("Instant".to_string()),
                    Type::Named("Duration".to_string()),
                ],
                Type::Named("Instant".to_string()),
            ),
        ),
        (
            "plainDateFrom".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDate_from_result",
                vec![Type::Str],
                Type::Named("PlainDate".to_string()),
            ),
        ),
        (
            "instantToZonedDateTime".to_string(),
            temporal_result_function(
                "__tyrus_temporal_instant_toZonedDateTime_result",
                vec![
                    Type::Named("Instant".to_string()),
                    Type::Named("TimeZone".to_string()),
                ],
                Type::Named("ZonedDateTime".to_string()),
            ),
        ),
        (
            "plainDateFromFields".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDate_fromFields_result",
                vec![Type::Named("PlainDate".to_string())],
                Type::Named("PlainDate".to_string()),
            ),
        ),
        (
            "plainDateWith".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDate_with_result",
                vec![
                    Type::Named("PlainDate".to_string()),
                    Type::Named("PlainDate".to_string()),
                ],
                Type::Named("PlainDate".to_string()),
            ),
        ),
        (
            "plainDateSubtractDuration".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDate_subtractDuration_result",
                vec![
                    Type::Named("PlainDate".to_string()),
                    Type::Named("Duration".to_string()),
                ],
                Type::Named("PlainDate".to_string()),
            ),
        ),
        (
            "plainDateUntil".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDate_until_result",
                vec![
                    Type::Named("PlainDate".to_string()),
                    Type::Named("PlainDate".to_string()),
                ],
                Type::Named("Duration".to_string()),
            ),
        ),
        (
            "plainDateSince".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDate_since_result",
                vec![
                    Type::Named("PlainDate".to_string()),
                    Type::Named("PlainDate".to_string()),
                ],
                Type::Named("Duration".to_string()),
            ),
        ),
        (
            "plainTimeFromFields".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainTime_fromFields_result",
                vec![Type::Named("PlainTime".to_string())],
                Type::Named("PlainTime".to_string()),
            ),
        ),
        (
            "plainTimeSubtractDuration".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainTime_subtractDuration_result",
                vec![
                    Type::Named("PlainTime".to_string()),
                    Type::Named("Duration".to_string()),
                ],
                Type::Named("PlainTime".to_string()),
            ),
        ),
        (
            "plainTimeUntil".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainTime_until_result",
                vec![
                    Type::Named("PlainTime".to_string()),
                    Type::Named("PlainTime".to_string()),
                ],
                Type::Named("Duration".to_string()),
            ),
        ),
        (
            "plainTimeSince".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainTime_since_result",
                vec![
                    Type::Named("PlainTime".to_string()),
                    Type::Named("PlainTime".to_string()),
                ],
                Type::Named("Duration".to_string()),
            ),
        ),
        (
            "plainDateToString".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDate_toString_result",
                vec![Type::Named("PlainDate".to_string())],
                Type::Str,
            ),
        ),
        (
            "plainDateEquals".to_string(),
            ExportInfo {
                target: "__tyrus_temporal_plainDate_equals".to_string(),
                ty: Type::Function {
                    params: vec![
                        Type::Named("PlainDate".to_string()),
                        Type::Named("PlainDate".to_string()),
                    ],
                    ret: Box::new(Type::Bool),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "plainDateAddDays".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDate_addDays_result",
                vec![Type::Named("PlainDate".to_string()), Type::I64],
                Type::Named("PlainDate".to_string()),
            ),
        ),
        (
            "plainDateDayOfWeek".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDate_dayOfWeek_result",
                vec![Type::Named("PlainDate".to_string())],
                Type::U32,
            ),
        ),
        (
            "plainDateDayOfYear".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDate_dayOfYear_result",
                vec![Type::Named("PlainDate".to_string())],
                Type::U32,
            ),
        ),
        (
            "plainDateAddMonths".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDate_addMonths_result",
                vec![Type::Named("PlainDate".to_string()), Type::I64],
                Type::Named("PlainDate".to_string()),
            ),
        ),
        (
            "plainDateAddYears".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDate_addYears_result",
                vec![Type::Named("PlainDate".to_string()), Type::I64],
                Type::Named("PlainDate".to_string()),
            ),
        ),
        (
            "plainDateDaysUntil".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDate_daysUntil_result",
                vec![
                    Type::Named("PlainDate".to_string()),
                    Type::Named("PlainDate".to_string()),
                ],
                Type::I64,
            ),
        ),
        (
            "plainTimeFrom".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainTime_from_result",
                vec![Type::Str],
                Type::Named("PlainTime".to_string()),
            ),
        ),
        (
            "plainTimeToString".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainTime_toString_result",
                vec![Type::Named("PlainTime".to_string())],
                Type::Str,
            ),
        ),
        (
            "plainTimeEquals".to_string(),
            ExportInfo {
                target: "__tyrus_temporal_plainTime_equals".to_string(),
                ty: Type::Function {
                    params: vec![
                        Type::Named("PlainTime".to_string()),
                        Type::Named("PlainTime".to_string()),
                    ],
                    ret: Box::new(Type::Bool),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "plainTimeAddSeconds".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainTime_addSeconds_result",
                vec![Type::Named("PlainTime".to_string()), Type::I64],
                Type::Named("PlainTime".to_string()),
            ),
        ),
        (
            "plainTimeSecondsUntil".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainTime_secondsUntil_result",
                vec![
                    Type::Named("PlainTime".to_string()),
                    Type::Named("PlainTime".to_string()),
                ],
                Type::I64,
            ),
        ),
        (
            "plainTimeAddNanoseconds".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainTime_addNanoseconds_result",
                vec![Type::Named("PlainTime".to_string()), Type::I64],
                Type::Named("PlainTime".to_string()),
            ),
        ),
        (
            "plainTimeNanosecondsUntil".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainTime_nanosecondsUntil_result",
                vec![
                    Type::Named("PlainTime".to_string()),
                    Type::Named("PlainTime".to_string()),
                ],
                Type::I64,
            ),
        ),
        (
            "durationFromFields".to_string(),
            ExportInfo {
                target: "__tyrus_temporal_duration_fromFields".to_string(),
                ty: Type::Function {
                    params: vec![Type::Named("Duration".to_string())],
                    ret: Box::new(Type::Named("Duration".to_string())),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "durationTotal".to_string(),
            temporal_result_function(
                "__tyrus_temporal_duration_total_result",
                vec![Type::Named("Duration".to_string()), Type::Str],
                Type::I64,
            ),
        ),
        (
            "plainDateAddDuration".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainDate_addDuration_result",
                vec![
                    Type::Named("PlainDate".to_string()),
                    Type::Named("Duration".to_string()),
                ],
                Type::Named("PlainDate".to_string()),
            ),
        ),
        (
            "plainTimeAddDuration".to_string(),
            temporal_result_function(
                "__tyrus_temporal_plainTime_addDuration_result",
                vec![
                    Type::Named("PlainTime".to_string()),
                    Type::Named("Duration".to_string()),
                ],
                Type::Named("PlainTime".to_string()),
            ),
        ),
    ]
    .into_iter()
    .collect()
}
