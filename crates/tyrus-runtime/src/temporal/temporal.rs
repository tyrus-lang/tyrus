//! Runtime services for `tyrus:temporal`.
//!
//! Provides timezone lookup and temporal primitives exposed to generated
//! Tyrus programs through the C ABI.

use super::strings::alloc_string;
use jiff::{
    RoundMode, Span, SpanRound, Timestamp, Unit, Zoned,
    civil::{Date, DateDifference, DateTime, DateTimeDifference, Time, TimeDifference},
    tz::TimeZone,
};
use std::ffi::CStr;
use std::mem::ManuallyDrop;
use tyrus_string::TyrusString;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instant {
    pub seconds: i64,
    pub nanoseconds: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZonedDateTime {
    pub seconds: i64,
    pub nanoseconds: u32,
    pub time_zone: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlainDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlainTime {
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub nanosecond: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Duration {
    pub years: i64,
    pub months: i64,
    pub weeks: i64,
    pub days: i64,
    pub hours: i64,
    pub minutes: i64,
    pub seconds: i64,
    pub milliseconds: i64,
    pub microseconds: i64,
    pub nanoseconds: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlainDateTime {
    pub date: PlainDate,
    pub time: PlainTime,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlainYearMonth {
    pub year: i32,
    pub month: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlainMonthDay {
    pub month: u32,
    pub day: u32,
}

thread_local! {
    static LAST_TEMPORAL_ERROR: std::cell::Cell<Option<(&'static CStr, &'static CStr)>> =
        const { std::cell::Cell::new(None) };
}

fn set_temporal_error(code: &'static CStr, message: &'static CStr) {
    LAST_TEMPORAL_ERROR.with(|slot| slot.set(Some((code, message))));
}

#[repr(C)]
pub struct TemporalErrorTaken {
    pub code: TyrusString,
    pub message: TyrusString,
}

#[repr(C)]
pub struct TemporalResultFfi<T> {
    pub tag: u8,
    pub payload: TemporalResultPayload<T>,
}

#[repr(C)]
pub union TemporalResultPayload<T> {
    pub ok: ManuallyDrop<T>,
    pub err: ManuallyDrop<TemporalErrorTaken>,
    words: [u64; 4],
}

const _: () = {
    assert!(std::mem::size_of::<TemporalErrorTaken>() == 2 * std::mem::size_of::<TyrusString>());
    assert!(std::mem::size_of::<TemporalResultFfi<Instant>>() == 40);
    assert!(std::mem::size_of::<TemporalResultFfi<i32>>() == 40);
    assert!(std::mem::size_of::<TemporalResultFfi<u32>>() == 40);
    assert!(std::mem::size_of::<TemporalResultFfi<i64>>() == 40);
    assert!(std::mem::size_of::<TemporalResultFfi<TyrusString>>() == 40);
    assert!(std::mem::size_of::<TemporalResultFfi<Duration>>() == 88);
    assert!(std::mem::size_of::<TemporalResultFfi<PlainDate>>() == 40);
    assert!(std::mem::size_of::<TemporalResultFfi<PlainTime>>() == 40);
    assert!(std::mem::size_of::<TemporalResultFfi<PlainDateTime>>() == 40);
    assert!(std::mem::size_of::<TemporalResultFfi<PlainYearMonth>>() == 40);
    assert!(std::mem::size_of::<TemporalResultFfi<PlainMonthDay>>() == 40);
    assert!(std::mem::offset_of!(TemporalResultFfi<Instant>, payload) == 8);
    assert!(std::mem::offset_of!(TemporalResultFfi<Duration>, payload) == 8);
};

fn begin_temporal_result() {
    LAST_TEMPORAL_ERROR.with(|slot| slot.set(None));
}

unsafe fn write_temporal_result<T>(out: *mut TemporalResultFfi<T>, value: T) {
    let error = LAST_TEMPORAL_ERROR.with(std::cell::Cell::take);
    if out.is_null() {
        return;
    }
    let result = if let Some((code, message)) = error {
        TemporalResultFfi {
            tag: 1,
            payload: TemporalResultPayload {
                err: ManuallyDrop::new(TemporalErrorTaken {
                    code: TyrusString::literal(code.to_str().expect("static UTF-8 code")),
                    message: TyrusString::literal(message.to_str().expect("static UTF-8 message")),
                }),
            },
        }
    } else {
        TemporalResultFfi {
            tag: 0,
            payload: TemporalResultPayload {
                ok: ManuallyDrop::new(value),
            },
        }
    };
    unsafe { out.write(result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_timezone_from_result(
    name: TyrusString,
    out: *mut TemporalResultFfi<i32>,
) {
    begin_temporal_result();
    let value = tyrus_temporal_timezone_from(name);
    unsafe { write_temporal_result(out, value) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_instant_from_result(
    input: TyrusString,
    out: *mut TemporalResultFfi<Instant>,
) {
    begin_temporal_result();
    let value = tyrus_temporal_instant_from(input);
    unsafe { write_temporal_result(out, value) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_instant_from_epoch_milliseconds_result(
    milliseconds: i64,
    out: *mut TemporalResultFfi<Instant>,
) {
    begin_temporal_result();
    let value = tyrus_temporal_instant_from_epoch_milliseconds(milliseconds);
    unsafe { write_temporal_result(out, value) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_instant_to_epoch_milliseconds_result(
    instant: *const Instant,
    out: *mut TemporalResultFfi<i64>,
) {
    begin_temporal_result();
    let value = unsafe {
        read_ffi_value(
            instant,
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant pointer must not be null",
        )
    }
    .map_or(0, |value| {
        tyrus_temporal_instant_to_epoch_milliseconds(value)
    });
    unsafe { write_temporal_result(out, value) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_instant_to_string_result(
    instant: *const Instant,
    out: *mut TemporalResultFfi<TyrusString>,
) {
    begin_temporal_result();
    let value = unsafe {
        read_ffi_value(
            instant,
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant pointer must not be null",
        )
    }
    .map_or_else(TyrusString::null, |value| {
        tyrus_temporal_instant_to_string(value)
    });
    unsafe { write_temporal_result(out, value) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_instant_add_duration_result(
    instant: *const Instant,
    duration: *const Duration,
    out: *mut TemporalResultFfi<Instant>,
) {
    begin_temporal_result();
    let value = unsafe {
        read_ffi_value(
            instant,
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant pointer must not be null",
        )
    }
    .zip(unsafe {
        read_ffi_value(
            duration,
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration pointer must not be null",
        )
    })
    .map_or(
        Instant {
            seconds: 0,
            nanoseconds: 0,
        },
        |(instant, duration)| tyrus_temporal_instant_add_duration(instant, duration),
    );
    unsafe { write_temporal_result(out, value) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_instant_subtract_duration_result(
    instant: *const Instant,
    duration: *const Duration,
    out: *mut TemporalResultFfi<Instant>,
) {
    begin_temporal_result();
    let value = unsafe {
        read_ffi_value(
            instant,
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant pointer must not be null",
        )
    }
    .zip(unsafe {
        read_ffi_value(
            duration,
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration pointer must not be null",
        )
    })
    .map_or(
        Instant {
            seconds: 0,
            nanoseconds: 0,
        },
        |(instant, duration)| tyrus_temporal_instant_subtract_duration(instant, duration),
    );
    unsafe { write_temporal_result(out, value) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_date_from_result(
    input: TyrusString,
    out: *mut TemporalResultFfi<PlainDate>,
) {
    begin_temporal_result();
    let value = tyrus_temporal_plain_date_from(input);
    unsafe { write_temporal_result(out, value) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_time_from_result(
    input: TyrusString,
    out: *mut TemporalResultFfi<PlainTime>,
) {
    begin_temporal_result();
    let value = tyrus_temporal_plain_time_from(input);
    unsafe { write_temporal_result(out, value) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_date_time_from_result(
    input: TyrusString,
    out: *mut TemporalResultFfi<PlainDateTime>,
) {
    begin_temporal_result();
    let mut value = std::mem::MaybeUninit::<PlainDateTime>::uninit();
    unsafe { tyrus_temporal_plain_date_time_from(input, value.as_mut_ptr()) };
    unsafe { write_temporal_result(out, value.assume_init()) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_year_month_from_result(
    input: TyrusString,
    out: *mut TemporalResultFfi<PlainYearMonth>,
) {
    begin_temporal_result();
    let mut value = std::mem::MaybeUninit::<PlainYearMonth>::uninit();
    unsafe { tyrus_temporal_plain_year_month_from(input, value.as_mut_ptr()) };
    unsafe { write_temporal_result(out, value.assume_init()) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_month_day_from_result(
    input: TyrusString,
    out: *mut TemporalResultFfi<PlainMonthDay>,
) {
    begin_temporal_result();
    let mut value = std::mem::MaybeUninit::<PlainMonthDay>::uninit();
    unsafe { tyrus_temporal_plain_month_day_from(input, value.as_mut_ptr()) };
    unsafe { write_temporal_result(out, value.assume_init()) };
}

macro_rules! temporal_date_unary_result {
    ($name:ident, $output:ty, $function:path, $fallback:expr) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            date: *const PlainDate,
            out: *mut TemporalResultFfi<$output>,
        ) {
            begin_temporal_result();
            let date = unsafe {
                read_ffi_value(
                    date,
                    c"ERR_TEMPORAL_INVALID_DATE",
                    c"date pointer must not be null",
                )
            }
            .unwrap_or($fallback);
            let value = $function(date);
            unsafe { write_temporal_result(out, value) };
        }
    };
}

macro_rules! temporal_date_binary_result {
    ($name:ident, $second:ty, $output:ty, $function:path, $fallback:expr) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            date: *const PlainDate,
            second: *const $second,
            out: *mut TemporalResultFfi<$output>,
        ) {
            begin_temporal_result();
            let date = unsafe {
                read_ffi_value(
                    date,
                    c"ERR_TEMPORAL_INVALID_DATE",
                    c"date pointer must not be null",
                )
            };
            let second = unsafe {
                read_ffi_value(
                    second,
                    c"ERR_TEMPORAL_INVALID_TEMPORAL_VALUE",
                    c"temporal value pointer must not be null",
                )
            };
            let value = date
                .zip(second)
                .map_or($fallback, |(date, second)| $function(date, second));
            unsafe { write_temporal_result(out, value) };
        }
    };
}

macro_rules! temporal_date_integer_result {
    ($name:ident, $output:ty, $function:path, $fallback:expr) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            date: *const PlainDate,
            value: i64,
            out: *mut TemporalResultFfi<$output>,
        ) {
            begin_temporal_result();
            let date = unsafe {
                read_ffi_value(
                    date,
                    c"ERR_TEMPORAL_INVALID_DATE",
                    c"date pointer must not be null",
                )
            }
            .unwrap_or(PlainDate {
                year: 0,
                month: 1,
                day: 1,
            });
            let result = $function(date, value);
            unsafe { write_temporal_result(out, result) };
        }
    };
}

temporal_date_unary_result!(
    tyrus_temporal_plain_date_to_string_result,
    TyrusString,
    tyrus_temporal_plain_date_to_string,
    PlainDate {
        year: 0,
        month: 1,
        day: 1
    }
);
temporal_date_unary_result!(
    tyrus_temporal_plain_date_day_of_week_result,
    u32,
    tyrus_temporal_plain_date_day_of_week,
    PlainDate {
        year: 0,
        month: 1,
        day: 1
    }
);
temporal_date_unary_result!(
    tyrus_temporal_plain_date_day_of_year_result,
    u32,
    tyrus_temporal_plain_date_day_of_year,
    PlainDate {
        year: 0,
        month: 1,
        day: 1
    }
);
temporal_date_integer_result!(
    tyrus_temporal_plain_date_add_days_result,
    PlainDate,
    tyrus_temporal_plain_date_add_days,
    PlainDate {
        year: 0,
        month: 1,
        day: 1
    }
);
temporal_date_integer_result!(
    tyrus_temporal_plain_date_add_months_result,
    PlainDate,
    tyrus_temporal_plain_date_add_months,
    PlainDate {
        year: 0,
        month: 1,
        day: 1
    }
);
temporal_date_integer_result!(
    tyrus_temporal_plain_date_add_years_result,
    PlainDate,
    tyrus_temporal_plain_date_add_years,
    PlainDate {
        year: 0,
        month: 1,
        day: 1
    }
);
temporal_date_unary_result!(
    tyrus_temporal_plain_date_from_fields_result,
    PlainDate,
    tyrus_temporal_plain_date_from_fields,
    PlainDate {
        year: 0,
        month: 1,
        day: 1
    }
);
temporal_date_binary_result!(
    tyrus_temporal_plain_date_with_result,
    PlainDate,
    PlainDate,
    tyrus_temporal_plain_date_with,
    PlainDate {
        year: 0,
        month: 1,
        day: 1
    }
);
temporal_date_binary_result!(
    tyrus_temporal_plain_date_subtract_duration_result,
    Duration,
    PlainDate,
    tyrus_temporal_plain_date_subtract_duration,
    PlainDate {
        year: 0,
        month: 1,
        day: 1
    }
);
temporal_date_binary_result!(
    tyrus_temporal_plain_date_until_result,
    PlainDate,
    Duration,
    tyrus_temporal_plain_date_until,
    Duration {
        years: 0,
        months: 0,
        weeks: 0,
        days: 0,
        hours: 0,
        minutes: 0,
        seconds: 0,
        milliseconds: 0,
        microseconds: 0,
        nanoseconds: 0
    }
);
temporal_date_binary_result!(
    tyrus_temporal_plain_date_since_result,
    PlainDate,
    Duration,
    tyrus_temporal_plain_date_since,
    Duration {
        years: 0,
        months: 0,
        weeks: 0,
        days: 0,
        hours: 0,
        minutes: 0,
        seconds: 0,
        milliseconds: 0,
        microseconds: 0,
        nanoseconds: 0
    }
);
temporal_date_binary_result!(
    tyrus_temporal_plain_date_days_until_result,
    PlainDate,
    i64,
    tyrus_temporal_plain_date_days_until,
    0
);
temporal_date_binary_result!(
    tyrus_temporal_plain_date_add_duration_result,
    Duration,
    PlainDate,
    tyrus_temporal_plain_date_add_duration,
    PlainDate {
        year: 0,
        month: 1,
        day: 1
    }
);

macro_rules! temporal_time_unary_result {
    ($name:ident, $output:ty, $function:path, $fallback:expr) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            time: *const PlainTime,
            out: *mut TemporalResultFfi<$output>,
        ) {
            begin_temporal_result();
            let time = unsafe {
                read_ffi_value(
                    time,
                    c"ERR_TEMPORAL_INVALID_TIME",
                    c"time pointer must not be null",
                )
            }
            .unwrap_or($fallback);
            let value = $function(time);
            unsafe { write_temporal_result(out, value) };
        }
    };
}

macro_rules! temporal_time_binary_result {
    ($name:ident, $second:ty, $output:ty, $function:path, $fallback:expr) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            time: *const PlainTime,
            second: *const $second,
            out: *mut TemporalResultFfi<$output>,
        ) {
            begin_temporal_result();
            let time = unsafe {
                read_ffi_value(
                    time,
                    c"ERR_TEMPORAL_INVALID_TIME",
                    c"time pointer must not be null",
                )
            };
            let second = unsafe {
                read_ffi_value(
                    second,
                    c"ERR_TEMPORAL_INVALID_TEMPORAL_VALUE",
                    c"temporal value pointer must not be null",
                )
            };
            let value = time
                .zip(second)
                .map_or($fallback, |(time, second)| $function(time, second));
            unsafe { write_temporal_result(out, value) };
        }
    };
}

macro_rules! temporal_time_integer_result {
    ($name:ident, $output:ty, $function:path, $fallback:expr) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            time: *const PlainTime,
            value: i64,
            out: *mut TemporalResultFfi<$output>,
        ) {
            begin_temporal_result();
            let time = unsafe {
                read_ffi_value(
                    time,
                    c"ERR_TEMPORAL_INVALID_TIME",
                    c"time pointer must not be null",
                )
            }
            .unwrap_or(PlainTime {
                hour: 0,
                minute: 0,
                second: 0,
                nanosecond: 0,
            });
            let result = $function(time, value);
            unsafe { write_temporal_result(out, result) };
        }
    };
}

macro_rules! temporal_difference_options_result {
    ($name:ident, $value:ty, $function:path, $code:literal, $message:literal) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            start: *const $value,
            end: *const $value,
            largest: TyrusString,
            smallest: TyrusString,
            increment: i64,
            mode: TyrusString,
            out: *mut TemporalResultFfi<Duration>,
        ) {
            begin_temporal_result();
            let start = unsafe { read_ffi_value(start, $code, $message) };
            let end = unsafe { read_ffi_value(end, $code, $message) };
            let duration = start
                .zip(end)
                .map_or_else(invalid_duration, |(start, end)| {
                    $function(start, end, largest, smallest, increment, mode)
                });
            unsafe { write_temporal_result(out, duration) };
        }
    };
}

temporal_time_unary_result!(
    tyrus_temporal_plain_time_from_fields_result,
    PlainTime,
    tyrus_temporal_plain_time_from_fields,
    PlainTime {
        hour: 0,
        minute: 0,
        second: 0,
        nanosecond: 0
    }
);
temporal_time_unary_result!(
    tyrus_temporal_plain_time_to_string_result,
    TyrusString,
    tyrus_temporal_plain_time_to_string,
    PlainTime {
        hour: 0,
        minute: 0,
        second: 0,
        nanosecond: 0
    }
);
temporal_time_integer_result!(
    tyrus_temporal_plain_time_add_seconds_result,
    PlainTime,
    tyrus_temporal_plain_time_add_seconds,
    PlainTime {
        hour: 0,
        minute: 0,
        second: 0,
        nanosecond: 0
    }
);
temporal_time_integer_result!(
    tyrus_temporal_plain_time_add_nanoseconds_result,
    PlainTime,
    tyrus_temporal_plain_time_add_nanoseconds,
    PlainTime {
        hour: 0,
        minute: 0,
        second: 0,
        nanosecond: 0
    }
);
temporal_time_binary_result!(
    tyrus_temporal_plain_time_subtract_duration_result,
    Duration,
    PlainTime,
    tyrus_temporal_plain_time_subtract_duration,
    PlainTime {
        hour: 0,
        minute: 0,
        second: 0,
        nanosecond: 0
    }
);
temporal_time_binary_result!(
    tyrus_temporal_plain_time_until_result,
    PlainTime,
    Duration,
    tyrus_temporal_plain_time_until,
    invalid_duration()
);
temporal_time_binary_result!(
    tyrus_temporal_plain_time_since_result,
    PlainTime,
    Duration,
    tyrus_temporal_plain_time_since,
    invalid_duration()
);
temporal_time_binary_result!(
    tyrus_temporal_plain_time_seconds_until_result,
    PlainTime,
    i64,
    tyrus_temporal_plain_time_seconds_until,
    0
);
temporal_time_binary_result!(
    tyrus_temporal_plain_time_nanoseconds_until_result,
    PlainTime,
    i64,
    tyrus_temporal_plain_time_nanoseconds_until,
    0
);
temporal_time_binary_result!(
    tyrus_temporal_plain_time_add_duration_result,
    Duration,
    PlainTime,
    tyrus_temporal_plain_time_add_duration,
    PlainTime {
        hour: 0,
        minute: 0,
        second: 0,
        nanosecond: 0
    }
);
temporal_difference_options_result!(
    tyrus_temporal_plain_date_until_with_options_result,
    PlainDate,
    tyrus_temporal_plain_date_until_with_options,
    c"ERR_TEMPORAL_INVALID_DATE",
    c"date pointer must not be null"
);
temporal_difference_options_result!(
    tyrus_temporal_plain_date_since_with_options_result,
    PlainDate,
    tyrus_temporal_plain_date_since_with_options,
    c"ERR_TEMPORAL_INVALID_DATE",
    c"date pointer must not be null"
);
temporal_difference_options_result!(
    tyrus_temporal_plain_time_until_with_options_result,
    PlainTime,
    tyrus_temporal_plain_time_until_with_options,
    c"ERR_TEMPORAL_INVALID_TIME",
    c"time pointer must not be null"
);
temporal_difference_options_result!(
    tyrus_temporal_plain_time_since_with_options_result,
    PlainTime,
    tyrus_temporal_plain_time_since_with_options,
    c"ERR_TEMPORAL_INVALID_TIME",
    c"time pointer must not be null"
);

macro_rules! temporal_aggregate_unary_result {
    ($name:ident, $input:ty, $output:ty, $function:path, $fallback:expr, $code:literal, $message:literal) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(input: *const $input, out: *mut TemporalResultFfi<$output>) {
            begin_temporal_result();
            let input = unsafe { read_ffi_value(input, $code, $message) }.unwrap_or($fallback);
            let mut value = $fallback;
            unsafe { $function(&input, &mut value) };
            unsafe { write_temporal_result(out, value) };
        }
    };
}

macro_rules! temporal_aggregate_string_result {
    ($name:ident, $input:ty, $function:path, $fallback:expr, $code:literal, $message:literal) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            input: *const $input,
            out: *mut TemporalResultFfi<TyrusString>,
        ) {
            begin_temporal_result();
            let input = unsafe { read_ffi_value(input, $code, $message) }.unwrap_or($fallback);
            let value = unsafe { $function(&input) };
            unsafe { write_temporal_result(out, value) };
        }
    };
}

macro_rules! temporal_aggregate_binary_result {
    ($name:ident, $first:ty, $second:ty, $output:ty, $function:path, $fallback:expr, $code:literal, $message:literal) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            first: *const $first,
            second: *const $second,
            out: *mut TemporalResultFfi<$output>,
        ) {
            begin_temporal_result();
            let first = unsafe { read_ffi_value(first, $code, $message) };
            let second = unsafe { read_ffi_value(second, $code, $message) };
            let mut value = $fallback;
            if let Some((first, second)) = first.zip(second) {
                unsafe { $function(&first, &second, &mut value) };
            }
            unsafe { write_temporal_result(out, value) };
        }
    };
}

macro_rules! temporal_aggregate_integer_result {
    ($name:ident, $input:ty, $output:ty, $integer:ty, $function:path, $fallback:expr, $code:literal, $message:literal) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            input: *const $input,
            integer: $integer,
            out: *mut TemporalResultFfi<$output>,
        ) {
            begin_temporal_result();
            let input = unsafe { read_ffi_value(input, $code, $message) };
            let mut value = $fallback;
            if let Some(input) = input {
                unsafe { $function(&input, integer, &mut value) };
            }
            unsafe { write_temporal_result(out, value) };
        }
    };
}

macro_rules! temporal_aggregate_options_result {
    ($name:ident, $input:ty, $function:path, $code:literal, $message:literal) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            start: *const $input,
            end: *const $input,
            largest: TyrusString,
            smallest: TyrusString,
            increment: i64,
            mode: TyrusString,
            out: *mut TemporalResultFfi<Duration>,
        ) {
            begin_temporal_result();
            let start = unsafe { read_ffi_value(start, $code, $message) };
            let end = unsafe { read_ffi_value(end, $code, $message) };
            let mut value = invalid_duration();
            if let Some((start, end)) = start.zip(end) {
                unsafe { $function(&start, &end, largest, smallest, increment, mode, &mut value) };
            }
            unsafe { write_temporal_result(out, value) };
        }
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_month_day_to_plain_date_result(
    value: *const PlainMonthDay,
    year: i32,
    out: *mut TemporalResultFfi<PlainDate>,
) {
    begin_temporal_result();
    let value = unsafe {
        read_ffi_value(
            value,
            c"ERR_TEMPORAL_INVALID_MONTH_DAY",
            c"month-day pointer must not be null",
        )
    }
    .unwrap_or(PlainMonthDay { month: 1, day: 1 });
    let date = tyrus_temporal_plain_month_day_to_plain_date(value, year);
    unsafe { write_temporal_result(out, date) };
}

temporal_aggregate_unary_result!(
    tyrus_temporal_plain_date_time_from_fields_result,
    PlainDateTime,
    PlainDateTime,
    tyrus_temporal_plain_date_time_from_fields,
    invalid_plain_date_time(),
    c"ERR_TEMPORAL_INVALID_DATE_TIME",
    c"date-time pointer must not be null"
);
temporal_aggregate_string_result!(
    tyrus_temporal_plain_date_time_to_string_result,
    PlainDateTime,
    tyrus_temporal_plain_date_time_to_string,
    invalid_plain_date_time(),
    c"ERR_TEMPORAL_INVALID_DATE_TIME",
    c"date-time pointer must not be null"
);
temporal_aggregate_binary_result!(
    tyrus_temporal_plain_date_time_with_result,
    PlainDateTime,
    PlainDateTime,
    PlainDateTime,
    tyrus_temporal_plain_date_time_with,
    invalid_plain_date_time(),
    c"ERR_TEMPORAL_INVALID_DATE_TIME",
    c"date-time pointer must not be null"
);
temporal_aggregate_binary_result!(
    tyrus_temporal_plain_date_time_add_duration_result,
    PlainDateTime,
    Duration,
    PlainDateTime,
    tyrus_temporal_plain_date_time_add_duration,
    invalid_plain_date_time(),
    c"ERR_TEMPORAL_INVALID_DATE_TIME",
    c"date-time pointer must not be null"
);
temporal_aggregate_binary_result!(
    tyrus_temporal_plain_date_time_subtract_duration_result,
    PlainDateTime,
    Duration,
    PlainDateTime,
    tyrus_temporal_plain_date_time_subtract_duration,
    invalid_plain_date_time(),
    c"ERR_TEMPORAL_INVALID_DATE_TIME",
    c"date-time pointer must not be null"
);
temporal_aggregate_binary_result!(
    tyrus_temporal_plain_date_time_until_result,
    PlainDateTime,
    PlainDateTime,
    Duration,
    tyrus_temporal_plain_date_time_until,
    invalid_duration(),
    c"ERR_TEMPORAL_INVALID_DATE_TIME",
    c"date-time pointer must not be null"
);
temporal_aggregate_binary_result!(
    tyrus_temporal_plain_date_time_since_result,
    PlainDateTime,
    PlainDateTime,
    Duration,
    tyrus_temporal_plain_date_time_since,
    invalid_duration(),
    c"ERR_TEMPORAL_INVALID_DATE_TIME",
    c"date-time pointer must not be null"
);
temporal_aggregate_options_result!(
    tyrus_temporal_plain_date_time_until_with_options_result,
    PlainDateTime,
    tyrus_temporal_plain_date_time_until_with_options,
    c"ERR_TEMPORAL_INVALID_DATE_TIME",
    c"date-time pointer must not be null"
);
temporal_aggregate_options_result!(
    tyrus_temporal_plain_date_time_since_with_options_result,
    PlainDateTime,
    tyrus_temporal_plain_date_time_since_with_options,
    c"ERR_TEMPORAL_INVALID_DATE_TIME",
    c"date-time pointer must not be null"
);
temporal_aggregate_unary_result!(
    tyrus_temporal_plain_year_month_from_fields_result,
    PlainYearMonth,
    PlainYearMonth,
    tyrus_temporal_plain_year_month_from_fields,
    PlainYearMonth { year: 0, month: 1 },
    c"ERR_TEMPORAL_INVALID_YEAR_MONTH",
    c"year-month pointer must not be null"
);
temporal_aggregate_string_result!(
    tyrus_temporal_plain_year_month_to_string_result,
    PlainYearMonth,
    tyrus_temporal_plain_year_month_to_string,
    PlainYearMonth { year: 0, month: 1 },
    c"ERR_TEMPORAL_INVALID_YEAR_MONTH",
    c"year-month pointer must not be null"
);
temporal_aggregate_integer_result!(
    tyrus_temporal_plain_year_month_add_result,
    PlainYearMonth,
    PlainYearMonth,
    i64,
    tyrus_temporal_plain_year_month_add,
    PlainYearMonth { year: 0, month: 1 },
    c"ERR_TEMPORAL_INVALID_YEAR_MONTH",
    c"year-month pointer must not be null"
);
temporal_aggregate_unary_result!(
    tyrus_temporal_plain_month_day_from_fields_result,
    PlainMonthDay,
    PlainMonthDay,
    tyrus_temporal_plain_month_day_from_fields,
    PlainMonthDay { month: 1, day: 1 },
    c"ERR_TEMPORAL_INVALID_MONTH_DAY",
    c"month-day pointer must not be null"
);
temporal_aggregate_string_result!(
    tyrus_temporal_plain_month_day_to_string_result,
    PlainMonthDay,
    tyrus_temporal_plain_month_day_to_string,
    PlainMonthDay { month: 1, day: 1 },
    c"ERR_TEMPORAL_INVALID_MONTH_DAY",
    c"month-day pointer must not be null"
);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_zoned_date_time_from_result(
    input: TyrusString,
    out: *mut TemporalResultFfi<ZonedDateTime>,
) {
    begin_temporal_result();
    let value = tyrus_temporal_zoned_date_time_from(input);
    unsafe { write_temporal_result(out, value) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_zoned_date_time_to_instant_result(
    value: *const ZonedDateTime,
    out: *mut TemporalResultFfi<Instant>,
) {
    begin_temporal_result();
    let value = unsafe {
        read_ffi_value(
            value,
            c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
            c"zoned date-time pointer must not be null",
        )
    }
    .unwrap_or(ZonedDateTime {
        seconds: 0,
        nanoseconds: 0,
        time_zone: 0,
    });
    unsafe { write_temporal_result(out, tyrus_temporal_zoned_date_time_to_instant(value)) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_zoned_date_time_to_string_result(
    value: *const ZonedDateTime,
    out: *mut TemporalResultFfi<TyrusString>,
) {
    begin_temporal_result();
    let value = unsafe {
        read_ffi_value(
            value,
            c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
            c"zoned date-time pointer must not be null",
        )
    }
    .unwrap_or(ZonedDateTime {
        seconds: 0,
        nanoseconds: 0,
        time_zone: 0,
    });
    unsafe { write_temporal_result(out, tyrus_temporal_zoned_date_time_to_string(value)) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_zoned_date_time_add_duration_result(
    value: *const ZonedDateTime,
    duration: *const Duration,
    out: *mut TemporalResultFfi<ZonedDateTime>,
) {
    begin_temporal_result();
    let value = unsafe {
        read_ffi_value(
            value,
            c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
            c"zoned date-time pointer must not be null",
        )
    };
    let duration = unsafe {
        read_ffi_value(
            duration,
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration pointer must not be null",
        )
    };
    let result = value.zip(duration).map_or(
        ZonedDateTime {
            seconds: 0,
            nanoseconds: 0,
            time_zone: 0,
        },
        |(value, duration)| tyrus_temporal_zoned_date_time_add_duration(value, duration),
    );
    unsafe { write_temporal_result(out, result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_zoned_date_time_until_result(
    start: *const ZonedDateTime,
    end: *const ZonedDateTime,
    out: *mut TemporalResultFfi<Duration>,
) {
    begin_temporal_result();
    let start = unsafe {
        read_ffi_value(
            start,
            c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
            c"zoned date-time pointer must not be null",
        )
    };
    let end = unsafe {
        read_ffi_value(
            end,
            c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
            c"zoned date-time pointer must not be null",
        )
    };
    let result = start
        .zip(end)
        .map_or_else(invalid_duration, |(start, end)| {
            tyrus_temporal_zoned_date_time_until(start, end)
        });
    unsafe { write_temporal_result(out, result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_zoned_date_time_format_result(
    value: *const ZonedDateTime,
    pattern: TyrusString,
    out: *mut TemporalResultFfi<TyrusString>,
) {
    begin_temporal_result();
    let value = unsafe {
        read_ffi_value(
            value,
            c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
            c"zoned date-time pointer must not be null",
        )
    };
    let result = value.map_or_else(TyrusString::null, |value| {
        tyrus_temporal_zoned_date_time_format(value, pattern)
    });
    unsafe { write_temporal_result(out, result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_instant_to_zoned_date_time_result(
    instant: *const Instant,
    time_zone: *const i32,
    out: *mut TemporalResultFfi<ZonedDateTime>,
) {
    begin_temporal_result();
    let instant = unsafe {
        read_ffi_value(
            instant,
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant pointer must not be null",
        )
    };
    let time_zone = unsafe {
        read_ffi_value(
            time_zone,
            c"ERR_TEMPORAL_INVALID_TIME_ZONE",
            c"time-zone pointer must not be null",
        )
    };
    let result = instant.zip(time_zone).map_or(
        ZonedDateTime {
            seconds: 0,
            nanoseconds: 0,
            time_zone: 0,
        },
        |(instant, time_zone)| tyrus_temporal_instant_to_zoned_date_time(instant, time_zone),
    );
    unsafe { write_temporal_result(out, result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_duration_total_result(
    duration: *const Duration,
    unit: TyrusString,
    out: *mut TemporalResultFfi<i64>,
) {
    begin_temporal_result();
    let duration = unsafe {
        read_ffi_value(
            duration,
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration pointer must not be null",
        )
    };
    let total = duration.map_or(0, |duration| unsafe {
        tyrus_temporal_duration_total(&duration, unit)
    });
    unsafe { write_temporal_result(out, total) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_duration_round_result(
    duration: *const Duration,
    unit: TyrusString,
    increment: i64,
    mode: TyrusString,
    relative_to: *const PlainDate,
    out: *mut TemporalResultFfi<Duration>,
) {
    begin_temporal_result();
    let duration = unsafe {
        read_ffi_value(
            duration,
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration pointer must not be null",
        )
    };
    let relative_to = unsafe {
        read_ffi_value(
            relative_to,
            c"ERR_TEMPORAL_INVALID_DATE",
            c"relative date pointer must not be null",
        )
    };
    let result =
        duration
            .zip(relative_to)
            .map_or_else(invalid_duration, |(duration, relative_to)| {
                tyrus_temporal_duration_round(duration, unit, increment, mode, relative_to)
            });
    unsafe { write_temporal_result(out, result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_take_error(out: *mut TemporalErrorTaken) {
    LAST_TEMPORAL_ERROR.with(|slot| {
        if let Some((code, message)) = slot.take()
            && !out.is_null()
        {
            unsafe {
                (*out).code =
                    TyrusString::literal(code.to_str().expect("a static error literal is UTF-8"));
                (*out).message = TyrusString::literal(
                    message.to_str().expect("a static error literal is UTF-8"),
                );
            }
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_temporal_timezone_from(name: TyrusString) -> i32 {
    if name.is_null() {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_TIME_ZONE",
            c"time zone must not be null",
        );
        return 0;
    }
    let name = unsafe { name.as_str() };
    let Some(ordinal) = tyrus_temporal_data::ordinal(name) else {
        set_temporal_error(c"ERR_TEMPORAL_INVALID_TIME_ZONE", c"unknown IANA time zone");
        return 0;
    };
    if TimeZone::get(name).is_err() {
        set_temporal_error(
            c"ERR_TEMPORAL_UNAVAILABLE_TIME_ZONE",
            c"IANA time zone is not available in the runtime database",
        );
        return 0;
    }
    ordinal
}

fn timezone_name(ordinal: i32) -> Option<&'static str> {
    tyrus_temporal_data::name(ordinal)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_temporal_timezone_name(ordinal: i32) -> TyrusString {
    let Some(name) = timezone_name(ordinal) else {
        return TyrusString::null();
    };
    alloc_string(name)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_temporal_timezone_offset(ordinal: i32, timestamp: i64) -> i32 {
    let Some(name) = timezone_name(ordinal) else {
        return 0;
    };
    let Ok(tz) = TimeZone::get(name) else {
        return 0;
    };
    let Ok(instant) = Timestamp::from_second(timestamp) else {
        return 0;
    };
    let zoned = instant.to_zoned(tz);
    zoned.offset().seconds()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_temporal_timezone_count() -> i32 {
    tyrus_temporal_data::TIME_ZONE_COUNT as i32
}

fn instant_from_timestamp(timestamp: Timestamp) -> Instant {
    let subsecond = timestamp.subsec_nanosecond();
    let (seconds, nanoseconds) = if subsecond < 0 {
        (
            timestamp.as_second() - 1,
            (subsecond + 1_000_000_000) as u32,
        )
    } else {
        (timestamp.as_second(), subsecond as u32)
    };
    Instant {
        seconds,
        nanoseconds,
    }
}

fn timestamp_from_instant(instant: Instant) -> Result<Timestamp, ()> {
    let nanoseconds = i32::try_from(instant.nanoseconds).map_err(|_| ())?;
    Timestamp::new(instant.seconds, nanoseconds).map_err(|_| ())
}

fn zoned_from_timestamp(timestamp: Timestamp, time_zone: i32) -> ZonedDateTime {
    let instant = instant_from_timestamp(timestamp);
    ZonedDateTime {
        seconds: instant.seconds,
        nanoseconds: instant.nanoseconds,
        time_zone,
    }
}

fn timestamp_from_zoned(value: ZonedDateTime) -> Result<Timestamp, ()> {
    timestamp_from_instant(Instant {
        seconds: value.seconds,
        nanoseconds: value.nanoseconds,
    })
}

fn duration_to_span(duration: Duration) -> Result<Span, ()> {
    Span::new()
        .try_years(duration.years)
        .and_then(|span| span.try_months(duration.months))
        .and_then(|span| span.try_weeks(duration.weeks))
        .and_then(|span| span.try_days(duration.days))
        .and_then(|span| span.try_hours(duration.hours))
        .and_then(|span| span.try_minutes(duration.minutes))
        .and_then(|span| span.try_seconds(duration.seconds))
        .and_then(|span| span.try_milliseconds(duration.milliseconds))
        .and_then(|span| span.try_microseconds(duration.microseconds))
        .and_then(|span| span.try_nanoseconds(duration.nanoseconds))
        .map_err(|_| ())
}

fn negate_duration(duration: Duration) -> Option<Duration> {
    Some(Duration {
        years: duration.years.checked_neg()?,
        months: duration.months.checked_neg()?,
        weeks: duration.weeks.checked_neg()?,
        days: duration.days.checked_neg()?,
        hours: duration.hours.checked_neg()?,
        minutes: duration.minutes.checked_neg()?,
        seconds: duration.seconds.checked_neg()?,
        milliseconds: duration.milliseconds.checked_neg()?,
        microseconds: duration.microseconds.checked_neg()?,
        nanoseconds: duration.nanoseconds.checked_neg()?,
    })
}

fn duration_from_span(span: Span) -> Duration {
    Duration {
        years: i64::from(span.get_years()),
        months: i64::from(span.get_months()),
        weeks: i64::from(span.get_weeks()),
        days: i64::from(span.get_days()),
        hours: i64::from(span.get_hours()),
        minutes: span.get_minutes(),
        seconds: span.get_seconds(),
        milliseconds: span.get_milliseconds(),
        microseconds: span.get_microseconds(),
        nanoseconds: span.get_nanoseconds(),
    }
}

fn temporal_unit(value: &str) -> Option<Unit> {
    match value {
        "year" | "years" => Some(Unit::Year),
        "month" | "months" => Some(Unit::Month),
        "week" | "weeks" => Some(Unit::Week),
        "day" | "days" => Some(Unit::Day),
        "hour" | "hours" => Some(Unit::Hour),
        "minute" | "minutes" => Some(Unit::Minute),
        "second" | "seconds" => Some(Unit::Second),
        "millisecond" | "milliseconds" => Some(Unit::Millisecond),
        "microsecond" | "microseconds" => Some(Unit::Microsecond),
        "nanosecond" | "nanoseconds" => Some(Unit::Nanosecond),
        _ => None,
    }
}

fn temporal_round_mode(value: &str) -> Option<RoundMode> {
    match value {
        "ceil" => Some(RoundMode::Ceil),
        "floor" => Some(RoundMode::Floor),
        "expand" => Some(RoundMode::Expand),
        "trunc" | "truncate" => Some(RoundMode::Trunc),
        "halfCeil" => Some(RoundMode::HalfCeil),
        "halfFloor" => Some(RoundMode::HalfFloor),
        "halfExpand" => Some(RoundMode::HalfExpand),
        "halfTrunc" => Some(RoundMode::HalfTrunc),
        "halfEven" => Some(RoundMode::HalfEven),
        _ => None,
    }
}

fn difference_options(
    largest: TyrusString,
    smallest: TyrusString,
    increment: i64,
    mode: TyrusString,
) -> Result<(Option<Unit>, Option<Unit>, i64, RoundMode), ()> {
    unsafe fn read(value: &TyrusString) -> Result<Option<&str>, ()> {
        if value.is_null() {
            return Ok(None);
        }
        let text = unsafe { value.as_str() };
        if text.is_empty() {
            Ok(None)
        } else {
            Ok(Some(text))
        }
    }
    let largest = unsafe { read(&largest) }?.map(|value| temporal_unit(value).ok_or(()));
    let largest = largest.transpose()?;
    let smallest = unsafe { read(&smallest) }?.map(|value| temporal_unit(value).ok_or(()));
    let smallest = smallest.transpose()?;
    let mode = unsafe { read(&mode) }?
        .map(|value| temporal_round_mode(value).ok_or(()))
        .transpose()?
        .unwrap_or(RoundMode::Trunc);
    if increment <= 0 {
        return Err(());
    }
    Ok((largest, smallest, increment, mode))
}

fn jiff_from_plain_date_time(value: PlainDateTime) -> Result<DateTime, ()> {
    let date = jiff_from_plain_date(value.date)?;
    let time = jiff_from_plain_time(value.time)?;
    Ok(DateTime::from_parts(date, time))
}

fn plain_date_time_from_jiff(value: DateTime) -> PlainDateTime {
    let date = value.date();
    let time = value.time();
    PlainDateTime {
        date: PlainDate {
            year: i32::from(date.year()),
            month: u32::try_from(date.month()).unwrap_or(0),
            day: u32::try_from(date.day()).unwrap_or(0),
        },
        time: PlainTime {
            hour: u32::try_from(time.hour()).unwrap_or(0),
            minute: u32::try_from(time.minute()).unwrap_or(0),
            second: u32::try_from(time.second()).unwrap_or(0),
            nanosecond: u32::try_from(time.subsec_nanosecond()).unwrap_or(0),
        },
    }
}

fn invalid_plain_date_time() -> PlainDateTime {
    PlainDateTime {
        date: PlainDate {
            year: 0,
            month: 1,
            day: 1,
        },
        time: PlainTime {
            hour: 0,
            minute: 0,
            second: 0,
            nanosecond: 0,
        },
    }
}

unsafe fn read_plain_date_time(value: *const PlainDateTime) -> Option<PlainDateTime> {
    unsafe {
        read_ffi_value(
            value,
            c"ERR_TEMPORAL_INVALID_DATE_TIME",
            c"date-time pointer must not be null",
        )
    }
}

unsafe fn write_plain_date_time(out: *mut PlainDateTime, value: PlainDateTime) {
    unsafe {
        write_ffi_value(
            out,
            value,
            c"ERR_TEMPORAL_INVALID_DATE_TIME",
            c"date-time output pointer must not be null",
        );
    }
}

unsafe fn read_ffi_value<T: Copy>(
    value: *const T,
    code: &'static CStr,
    message: &'static CStr,
) -> Option<T> {
    let Some(value) = (unsafe { value.as_ref() }) else {
        set_temporal_error(code, message);
        return None;
    };
    Some(*value)
}

unsafe fn write_ffi_value<T>(out: *mut T, value: T, code: &'static CStr, message: &'static CStr) {
    let Some(out) = (unsafe { out.as_mut() }) else {
        set_temporal_error(code, message);
        return;
    };
    *out = value;
}

fn invalid_duration() -> Duration {
    Duration {
        years: 0,
        months: 0,
        weeks: 0,
        days: 0,
        hours: 0,
        minutes: 0,
        seconds: 0,
        milliseconds: 0,
        microseconds: 0,
        nanoseconds: 0,
    }
}

fn duration_from_nanoseconds(value: i128) -> Duration {
    let sign = if value < 0 { -1 } else { 1 };
    let mut remainder = value.abs();
    let days = remainder / 86_400_000_000_000;
    remainder %= 86_400_000_000_000;
    let hours = remainder / 3_600_000_000_000;
    remainder %= 3_600_000_000_000;
    let minutes = remainder / 60_000_000_000;
    remainder %= 60_000_000_000;
    let seconds = remainder / 1_000_000_000;
    let nanoseconds = remainder % 1_000_000_000;
    Duration {
        years: 0,
        months: 0,
        weeks: 0,
        days: (days as i64) * sign,
        hours: (hours as i64) * sign,
        minutes: (minutes as i64) * sign,
        seconds: (seconds as i64) * sign,
        milliseconds: 0,
        microseconds: 0,
        nanoseconds: (nanoseconds as i64) * sign,
    }
}

pub extern "C" fn tyrus_temporal_zoned_date_time_from(input: TyrusString) -> ZonedDateTime {
    let invalid = ZonedDateTime {
        seconds: 0,
        nanoseconds: 0,
        time_zone: 0,
    };
    let Some(text) = (!input.is_null()).then(|| unsafe { input.as_str() }) else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
            c"zoned date-time must be valid UTF-8",
        );
        return invalid;
    };
    let Some(open) = text.rfind('[') else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
            c"zoned date-time requires an IANA time zone",
        );
        return invalid;
    };
    let Some(zone_name) = text
        .strip_suffix(']')
        .and_then(|value| value.get(open + 1..))
    else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
            c"invalid zoned date-time time zone",
        );
        return invalid;
    };
    let Some(time_zone) = tyrus_temporal_data::ordinal(zone_name) else {
        set_temporal_error(c"ERR_TEMPORAL_INVALID_TIME_ZONE", c"unknown IANA time zone");
        return invalid;
    };
    let Ok(zoned) = text.parse::<Zoned>() else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
            c"invalid ISO zoned date-time or offset mismatch",
        );
        return invalid;
    };
    zoned_from_timestamp(zoned.timestamp(), time_zone)
}

pub extern "C" fn tyrus_temporal_zoned_date_time_to_instant(value: ZonedDateTime) -> Instant {
    match timestamp_from_zoned(value) {
        Ok(timestamp) => instant_from_timestamp(timestamp),
        Err(()) => {
            set_temporal_error(
                c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
                c"invalid zoned date-time value",
            );
            Instant {
                seconds: 0,
                nanoseconds: 0,
            }
        }
    }
}

pub extern "C" fn tyrus_temporal_zoned_date_time_to_string(value: ZonedDateTime) -> TyrusString {
    let Some(name) = timezone_name(value.time_zone) else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_TIME_ZONE",
            c"invalid IANA time zone ordinal",
        );
        return TyrusString::null();
    };
    let Ok(time_zone) = TimeZone::get(name) else {
        set_temporal_error(
            c"ERR_TEMPORAL_UNAVAILABLE_TIME_ZONE",
            c"IANA time zone is unavailable",
        );
        return TyrusString::null();
    };
    let Ok(timestamp) = timestamp_from_zoned(value) else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
            c"invalid zoned date-time value",
        );
        return TyrusString::null();
    };
    let _ = name;
    alloc_string(&timestamp.to_zoned(time_zone).to_string())
}

pub extern "C" fn tyrus_temporal_instant_to_zoned_date_time(
    instant: Instant,
    time_zone: i32,
) -> ZonedDateTime {
    if timezone_name(time_zone).is_none() {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_TIME_ZONE",
            c"invalid IANA time zone ordinal",
        );
        return ZonedDateTime {
            seconds: 0,
            nanoseconds: 0,
            time_zone: 0,
        };
    }
    let Ok(timestamp) = timestamp_from_instant(instant) else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant fields are outside the supported range",
        );
        return ZonedDateTime {
            seconds: 0,
            nanoseconds: 0,
            time_zone,
        };
    };
    zoned_from_timestamp(timestamp, time_zone)
}

pub extern "C" fn tyrus_temporal_zoned_date_time_equals(
    left: ZonedDateTime,
    right: ZonedDateTime,
) -> bool {
    left == right
}

pub extern "C" fn tyrus_temporal_zoned_date_time_add_duration(
    value: ZonedDateTime,
    duration: Duration,
) -> ZonedDateTime {
    let Some(name) = timezone_name(value.time_zone) else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_TIME_ZONE",
            c"invalid IANA time zone ordinal",
        );
        return value;
    };
    let Ok(zone) = TimeZone::get(name) else {
        set_temporal_error(
            c"ERR_TEMPORAL_UNAVAILABLE_TIME_ZONE",
            c"IANA time zone is unavailable",
        );
        return value;
    };
    let Ok(timestamp) = timestamp_from_zoned(value) else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
            c"invalid zoned date-time value",
        );
        return value;
    };
    let Ok(span) = duration_to_span(duration) else {
        set_temporal_error(
            c"ERR_TEMPORAL_DURATION_OVERFLOW",
            c"duration fields exceed the supported range",
        );
        return value;
    };
    match timestamp.to_zoned(zone).checked_add(span) {
        Ok(result) => zoned_from_timestamp(result.timestamp(), value.time_zone),
        Err(_) => {
            set_temporal_error(
                c"ERR_TEMPORAL_ZONED_DATE_TIME_OVERFLOW",
                c"zoned date-time arithmetic overflow",
            );
            value
        }
    }
}

pub extern "C" fn tyrus_temporal_zoned_date_time_until(
    start: ZonedDateTime,
    end: ZonedDateTime,
) -> Duration {
    match (timestamp_from_zoned(start), timestamp_from_zoned(end)) {
        (Ok(left), Ok(right)) => duration_from_nanoseconds(left.duration_until(right).as_nanos()),
        _ => {
            set_temporal_error(
                c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
                c"invalid zoned date-time value",
            );
            Duration {
                years: 0,
                months: 0,
                weeks: 0,
                days: 0,
                hours: 0,
                minutes: 0,
                seconds: 0,
                milliseconds: 0,
                microseconds: 0,
                nanoseconds: 0,
            }
        }
    }
}

pub extern "C" fn tyrus_temporal_zoned_date_time_format(
    value: ZonedDateTime,
    pattern: TyrusString,
) -> TyrusString {
    let Some(pattern) = (!pattern.is_null()).then(|| unsafe { pattern.as_str() }) else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_FORMAT",
            c"format pattern must be valid UTF-8",
        );
        return TyrusString::null();
    };
    let Some(name) = timezone_name(value.time_zone) else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_TIME_ZONE",
            c"invalid IANA time zone ordinal",
        );
        return TyrusString::null();
    };
    let Ok(zone) = TimeZone::get(name) else {
        set_temporal_error(
            c"ERR_TEMPORAL_UNAVAILABLE_TIME_ZONE",
            c"IANA time zone is unavailable",
        );
        return TyrusString::null();
    };
    let Ok(timestamp) = timestamp_from_zoned(value) else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
            c"invalid zoned date-time value",
        );
        return TyrusString::null();
    };
    let translated = pattern
        .replace("yyyy", "%Y")
        .replace("MM", "%m")
        .replace("dd", "%d")
        .replace("HH", "%H")
        .replace("mm", "%M")
        .replace("ss", "%S");
    alloc_string(&timestamp.to_zoned(zone).strftime(&translated).to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn now_timestamp() -> Timestamp {
    Timestamp::now()
}

#[cfg(target_arch = "wasm32")]
fn now_timestamp() -> Timestamp {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        fn tyrus_host_now_millis() -> f64;
    }
    let millis = unsafe { tyrus_host_now_millis() };
    if !millis.is_finite() {
        return Timestamp::UNIX_EPOCH;
    }
    Timestamp::from_millisecond(millis as i64).unwrap_or(Timestamp::UNIX_EPOCH)
}

pub extern "C" fn tyrus_temporal_instant_now() -> Instant {
    instant_from_timestamp(now_timestamp())
}

pub extern "C" fn tyrus_temporal_instant_from(input: TyrusString) -> Instant {
    let Some(input) = (!input.is_null()).then(|| unsafe { input.as_str() }) else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant must be valid UTF-8",
        );
        return Instant {
            seconds: 0,
            nanoseconds: 0,
        };
    };
    match input.parse::<Timestamp>() {
        Ok(timestamp) => instant_from_timestamp(timestamp),
        Err(_) => {
            set_temporal_error(c"ERR_TEMPORAL_INVALID_INSTANT", c"invalid ISO instant");
            Instant {
                seconds: 0,
                nanoseconds: 0,
            }
        }
    }
}

pub extern "C" fn tyrus_temporal_instant_from_epoch_milliseconds(milliseconds: i64) -> Instant {
    match Timestamp::from_millisecond(milliseconds) {
        Ok(timestamp) => instant_from_timestamp(timestamp),
        Err(_) => {
            set_temporal_error(
                c"ERR_TEMPORAL_INVALID_INSTANT",
                c"epoch milliseconds are outside the supported instant range",
            );
            Instant {
                seconds: 0,
                nanoseconds: 0,
            }
        }
    }
}

pub extern "C" fn tyrus_temporal_instant_to_epoch_milliseconds(instant: Instant) -> i64 {
    let Ok(timestamp) = timestamp_from_instant(instant) else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant fields are outside the supported range",
        );
        return 0;
    };
    timestamp.as_millisecond()
}

pub extern "C" fn tyrus_temporal_instant_to_string(instant: Instant) -> TyrusString {
    let Ok(timestamp) = timestamp_from_instant(instant) else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant fields are outside the supported range",
        );
        return TyrusString::null();
    };
    alloc_string(&timestamp.to_string())
}

fn instant_duration_operation(instant: Instant, duration: Duration, subtract: bool) -> Instant {
    if duration.years != 0 || duration.months != 0 || duration.weeks != 0 || duration.days != 0 {
        set_temporal_error(
            c"ERR_TEMPORAL_INSTANT_CALENDAR_DURATION",
            c"instant arithmetic requires fixed-length time units",
        );
        return instant;
    }
    let Ok(timestamp) = timestamp_from_instant(instant) else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant fields are invalid",
        );
        return instant;
    };
    let Ok(span) = duration_to_span(duration) else {
        set_temporal_error(
            c"ERR_TEMPORAL_DURATION_OVERFLOW",
            c"duration fields exceed the supported range",
        );
        return instant;
    };
    let result = if subtract {
        timestamp.checked_sub(span)
    } else {
        timestamp.checked_add(span)
    };
    match result {
        Ok(value) => instant_from_timestamp(value),
        Err(_) => {
            set_temporal_error(
                c"ERR_TEMPORAL_INSTANT_OVERFLOW",
                c"instant arithmetic overflowed",
            );
            instant
        }
    }
}

pub extern "C" fn tyrus_temporal_instant_add_duration(
    instant: Instant,
    duration: Duration,
) -> Instant {
    instant_duration_operation(instant, duration, false)
}

pub extern "C" fn tyrus_temporal_instant_subtract_duration(
    instant: Instant,
    duration: Duration,
) -> Instant {
    instant_duration_operation(instant, duration, true)
}

#[unsafe(export_name = "tyrus_temporal_instant_now")]
pub unsafe extern "C" fn tyrus_temporal_instant_now_ffi(out: *mut Instant) {
    unsafe {
        write_ffi_value(
            out,
            tyrus_temporal_instant_now(),
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant output pointer must not be null",
        );
    }
}

#[unsafe(export_name = "tyrus_temporal_instant_from")]
pub unsafe extern "C" fn tyrus_temporal_instant_from_ffi(input: TyrusString, out: *mut Instant) {
    unsafe {
        write_ffi_value(
            out,
            tyrus_temporal_instant_from(input),
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant output pointer must not be null",
        );
    }
}

#[unsafe(export_name = "tyrus_temporal_instant_from_epoch_milliseconds")]
pub unsafe extern "C" fn tyrus_temporal_instant_from_epoch_milliseconds_ffi(
    milliseconds: i64,
    out: *mut Instant,
) {
    unsafe {
        write_ffi_value(
            out,
            tyrus_temporal_instant_from_epoch_milliseconds(milliseconds),
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant output pointer must not be null",
        );
    }
}

#[unsafe(export_name = "tyrus_temporal_instant_to_epoch_milliseconds")]
pub unsafe extern "C" fn tyrus_temporal_instant_to_epoch_milliseconds_ffi(
    instant: *const Instant,
) -> i64 {
    unsafe {
        let Some(instant) = read_ffi_value(
            instant,
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant pointer must not be null",
        ) else {
            return 0;
        };
        tyrus_temporal_instant_to_epoch_milliseconds(instant)
    }
}

#[unsafe(export_name = "tyrus_temporal_instant_to_string")]
pub unsafe extern "C" fn tyrus_temporal_instant_to_string_ffi(
    instant: *const Instant,
) -> TyrusString {
    unsafe {
        let Some(instant) = read_ffi_value(
            instant,
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant pointer must not be null",
        ) else {
            return TyrusString::null();
        };
        tyrus_temporal_instant_to_string(instant)
    }
}

#[unsafe(export_name = "tyrus_temporal_instant_add_duration")]
pub unsafe extern "C" fn tyrus_temporal_instant_add_duration_ffi(
    instant: *const Instant,
    duration: *const Duration,
    out: *mut Instant,
) {
    unsafe {
        let Some(instant) = read_ffi_value(
            instant,
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant pointer must not be null",
        ) else {
            return;
        };
        let Some(duration) = read_ffi_value(
            duration,
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration pointer must not be null",
        ) else {
            return;
        };
        write_ffi_value(
            out,
            tyrus_temporal_instant_add_duration(instant, duration),
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant output pointer must not be null",
        );
    }
}

#[unsafe(export_name = "tyrus_temporal_instant_subtract_duration")]
pub unsafe extern "C" fn tyrus_temporal_instant_subtract_duration_ffi(
    instant: *const Instant,
    duration: *const Duration,
    out: *mut Instant,
) {
    unsafe {
        let Some(instant) = read_ffi_value(
            instant,
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant pointer must not be null",
        ) else {
            return;
        };
        let Some(duration) = read_ffi_value(
            duration,
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration pointer must not be null",
        ) else {
            return;
        };
        write_ffi_value(
            out,
            tyrus_temporal_instant_subtract_duration(instant, duration),
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant output pointer must not be null",
        );
    }
}

#[unsafe(export_name = "tyrus_temporal_zoned_date_time_from")]
pub unsafe extern "C" fn tyrus_temporal_zoned_date_time_from_ffi(
    input: TyrusString,
    out: *mut ZonedDateTime,
) {
    unsafe {
        write_ffi_value(
            out,
            tyrus_temporal_zoned_date_time_from(input),
            c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
            c"zoned date-time output pointer must not be null",
        );
    }
}

#[unsafe(export_name = "tyrus_temporal_zoned_date_time_to_instant")]
pub unsafe extern "C" fn tyrus_temporal_zoned_date_time_to_instant_ffi(
    value: *const ZonedDateTime,
    out: *mut Instant,
) {
    unsafe {
        let Some(value) = read_ffi_value(
            value,
            c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
            c"zoned date-time pointer must not be null",
        ) else {
            return;
        };
        write_ffi_value(
            out,
            tyrus_temporal_zoned_date_time_to_instant(value),
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant output pointer must not be null",
        );
    }
}

#[unsafe(export_name = "tyrus_temporal_zoned_date_time_to_string")]
pub unsafe extern "C" fn tyrus_temporal_zoned_date_time_to_string_ffi(
    value: *const ZonedDateTime,
) -> TyrusString {
    unsafe {
        let Some(value) = read_ffi_value(
            value,
            c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
            c"zoned date-time pointer must not be null",
        ) else {
            return TyrusString::null();
        };
        tyrus_temporal_zoned_date_time_to_string(value)
    }
}

#[unsafe(export_name = "tyrus_temporal_instant_to_zoned_date_time")]
pub unsafe extern "C" fn tyrus_temporal_instant_to_zoned_date_time_ffi(
    instant: *const Instant,
    time_zone: i32,
    out: *mut ZonedDateTime,
) {
    unsafe {
        let Some(instant) = read_ffi_value(
            instant,
            c"ERR_TEMPORAL_INVALID_INSTANT",
            c"instant pointer must not be null",
        ) else {
            return;
        };
        write_ffi_value(
            out,
            tyrus_temporal_instant_to_zoned_date_time(instant, time_zone),
            c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
            c"zoned date-time output pointer must not be null",
        );
    }
}

#[unsafe(export_name = "tyrus_temporal_zoned_date_time_equals")]
pub unsafe extern "C" fn tyrus_temporal_zoned_date_time_equals_ffi(
    left: *const ZonedDateTime,
    right: *const ZonedDateTime,
) -> bool {
    unsafe {
        let (Some(left), Some(right)) = (
            read_ffi_value(
                left,
                c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
                c"zoned date-time pointer must not be null",
            ),
            read_ffi_value(
                right,
                c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
                c"zoned date-time pointer must not be null",
            ),
        ) else {
            return false;
        };
        tyrus_temporal_zoned_date_time_equals(left, right)
    }
}

#[unsafe(export_name = "tyrus_temporal_zoned_date_time_add_duration")]
pub unsafe extern "C" fn tyrus_temporal_zoned_date_time_add_duration_ffi(
    value: *const ZonedDateTime,
    duration: *const Duration,
    out: *mut ZonedDateTime,
) {
    unsafe {
        let Some(value) = read_ffi_value(
            value,
            c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
            c"zoned date-time pointer must not be null",
        ) else {
            return;
        };
        let Some(duration) = read_ffi_value(
            duration,
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration pointer must not be null",
        ) else {
            return;
        };
        write_ffi_value(
            out,
            tyrus_temporal_zoned_date_time_add_duration(value, duration),
            c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
            c"zoned date-time output pointer must not be null",
        );
    }
}

#[unsafe(export_name = "tyrus_temporal_zoned_date_time_until")]
pub unsafe extern "C" fn tyrus_temporal_zoned_date_time_until_ffi(
    start: *const ZonedDateTime,
    end: *const ZonedDateTime,
    out: *mut Duration,
) {
    unsafe {
        let (Some(start), Some(end)) = (
            read_ffi_value(
                start,
                c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
                c"zoned date-time pointer must not be null",
            ),
            read_ffi_value(
                end,
                c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
                c"zoned date-time pointer must not be null",
            ),
        ) else {
            return;
        };
        write_ffi_value(
            out,
            tyrus_temporal_zoned_date_time_until(start, end),
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration output pointer must not be null",
        );
    }
}

#[unsafe(export_name = "tyrus_temporal_zoned_date_time_format")]
pub unsafe extern "C" fn tyrus_temporal_zoned_date_time_format_ffi(
    value: *const ZonedDateTime,
    pattern: TyrusString,
) -> TyrusString {
    unsafe {
        let Some(value) = read_ffi_value(
            value,
            c"ERR_TEMPORAL_INVALID_ZONED_DATE_TIME",
            c"zoned date-time pointer must not be null",
        ) else {
            return TyrusString::null();
        };
        tyrus_temporal_zoned_date_time_format(value, pattern)
    }
}

fn plain_date_from_jiff(date: Date) -> PlainDate {
    PlainDate {
        year: i32::from(date.year()),
        month: date.month() as u32,
        day: date.day() as u32,
    }
}

fn jiff_from_plain_date(date: PlainDate) -> Result<Date, ()> {
    Date::new(
        i16::try_from(date.year).map_err(|_| ())?,
        i8::try_from(date.month).map_err(|_| ())?,
        i8::try_from(date.day).map_err(|_| ())?,
    )
    .map_err(|_| ())
}

fn plain_time_from_jiff(time: Time) -> PlainTime {
    PlainTime {
        hour: time.hour() as u32,
        minute: time.minute() as u32,
        second: time.second() as u32,
        nanosecond: time.subsec_nanosecond() as u32,
    }
}

fn jiff_from_plain_time(time: PlainTime) -> Result<Time, ()> {
    Time::new(
        i8::try_from(time.hour).map_err(|_| ())?,
        i8::try_from(time.minute).map_err(|_| ())?,
        i8::try_from(time.second).map_err(|_| ())?,
        i32::try_from(time.nanosecond).map_err(|_| ())?,
    )
    .map_err(|_| ())
}

fn jiff_from_plain_year_month(value: PlainYearMonth) -> Result<Date, ()> {
    Date::new(
        i16::try_from(value.year).map_err(|_| ())?,
        i8::try_from(value.month).map_err(|_| ())?,
        1,
    )
    .map_err(|_| ())
}

fn jiff_from_plain_month_day(value: PlainMonthDay) -> Result<Date, ()> {
    Date::new(
        1972,
        i8::try_from(value.month).map_err(|_| ())?,
        i8::try_from(value.day).map_err(|_| ())?,
    )
    .map_err(|_| ())
}

pub extern "C" fn tyrus_temporal_plain_date_from(input: TyrusString) -> PlainDate {
    let Some(input) = (!input.is_null()).then(|| unsafe { input.as_str() }) else {
        set_temporal_error(c"ERR_TEMPORAL_INVALID_DATE", c"date must not be null");
        return PlainDate {
            year: 0,
            month: 1,
            day: 1,
        };
    };
    match input.parse::<Date>() {
        Ok(date) => plain_date_from_jiff(date),
        Err(_) => {
            set_temporal_error(c"ERR_TEMPORAL_INVALID_DATE", c"invalid ISO calendar date");
            PlainDate {
                year: 0,
                month: 1,
                day: 1,
            }
        }
    }
}

pub extern "C" fn tyrus_temporal_plain_date_to_string(date: PlainDate) -> TyrusString {
    let Ok(date) = jiff_from_plain_date(date) else {
        set_temporal_error(c"ERR_TEMPORAL_INVALID_DATE", c"date fields are invalid");
        return TyrusString::null();
    };
    alloc_string(&date.to_string())
}

pub extern "C" fn tyrus_temporal_plain_date_equals(left: PlainDate, right: PlainDate) -> bool {
    left == right
}

pub extern "C" fn tyrus_temporal_plain_date_add_days(date: PlainDate, days: i64) -> PlainDate {
    let Ok(date) = jiff_from_plain_date(date) else {
        set_temporal_error(c"ERR_TEMPORAL_INVALID_DATE", c"date fields are invalid");
        return date;
    };
    let Ok(span) = Span::new().try_days(days) else {
        set_temporal_error(
            c"ERR_TEMPORAL_DATE_OVERFLOW",
            c"date arithmetic exceeds the supported calendar range",
        );
        return plain_date_from_jiff(date);
    };
    match date.checked_add(span) {
        Ok(date) => plain_date_from_jiff(date),
        Err(_) => {
            set_temporal_error(
                c"ERR_TEMPORAL_DATE_OVERFLOW",
                c"date arithmetic exceeds the supported calendar range",
            );
            plain_date_from_jiff(date)
        }
    }
}

pub extern "C" fn tyrus_temporal_plain_date_day_of_week(date: PlainDate) -> u32 {
    let Ok(date) = jiff_from_plain_date(date) else {
        set_temporal_error(c"ERR_TEMPORAL_INVALID_DATE", c"date fields are invalid");
        return 0;
    };
    date.weekday().to_monday_one_offset() as u32
}

pub extern "C" fn tyrus_temporal_plain_date_day_of_year(date: PlainDate) -> u32 {
    let Ok(date) = jiff_from_plain_date(date) else {
        set_temporal_error(c"ERR_TEMPORAL_INVALID_DATE", c"date fields are invalid");
        return 0;
    };
    date.day_of_year() as u32
}

fn plain_date_add_span(date: PlainDate, span: Span) -> PlainDate {
    let Ok(date_value) = jiff_from_plain_date(date) else {
        set_temporal_error(c"ERR_TEMPORAL_INVALID_DATE", c"date fields are invalid");
        return date;
    };
    match date_value.checked_add(span) {
        Ok(result) => plain_date_from_jiff(result),
        Err(_) => {
            set_temporal_error(
                c"ERR_TEMPORAL_DATE_OVERFLOW",
                c"date arithmetic exceeds the supported calendar range",
            );
            date
        }
    }
}

pub extern "C" fn tyrus_temporal_plain_date_add_months(date: PlainDate, months: i64) -> PlainDate {
    let Ok(span) = Span::new().try_months(months) else {
        set_temporal_error(
            c"ERR_TEMPORAL_DATE_OVERFLOW",
            c"date arithmetic exceeds the supported calendar range",
        );
        return date;
    };
    plain_date_add_span(date, span)
}

pub extern "C" fn tyrus_temporal_plain_date_add_years(date: PlainDate, years: i64) -> PlainDate {
    let Ok(span) = Span::new().try_years(years) else {
        set_temporal_error(
            c"ERR_TEMPORAL_DATE_OVERFLOW",
            c"date arithmetic exceeds the supported calendar range",
        );
        return date;
    };
    plain_date_add_span(date, span)
}

pub extern "C" fn tyrus_temporal_plain_date_days_until(start: PlainDate, end: PlainDate) -> i64 {
    let (Ok(start), Ok(end)) = (jiff_from_plain_date(start), jiff_from_plain_date(end)) else {
        set_temporal_error(c"ERR_TEMPORAL_INVALID_DATE", c"date fields are invalid");
        return 0;
    };
    end.duration_since(start).as_secs() / 86_400
}

pub extern "C" fn tyrus_temporal_plain_date_from_fields(date: PlainDate) -> PlainDate {
    if jiff_from_plain_date(date).is_err() {
        set_temporal_error(c"ERR_TEMPORAL_INVALID_DATE", c"date fields are invalid");
    }
    date
}

pub extern "C" fn tyrus_temporal_plain_date_with(
    _date: PlainDate,
    replacement: PlainDate,
) -> PlainDate {
    tyrus_temporal_plain_date_from_fields(replacement)
}

pub extern "C" fn tyrus_temporal_plain_date_subtract_duration(
    date: PlainDate,
    duration: Duration,
) -> PlainDate {
    let Some(negated) = negate_duration(duration) else {
        set_temporal_error(
            c"ERR_TEMPORAL_DURATION_OVERFLOW",
            c"duration subtraction overflow",
        );
        return date;
    };
    tyrus_temporal_plain_date_add_duration(date, negated)
}

pub extern "C" fn tyrus_temporal_plain_date_until(start: PlainDate, end: PlainDate) -> Duration {
    duration_from_nanoseconds(
        i128::from(tyrus_temporal_plain_date_days_until(start, end)) * 86_400_000_000_000,
    )
}

pub extern "C" fn tyrus_temporal_plain_date_since(start: PlainDate, end: PlainDate) -> Duration {
    tyrus_temporal_plain_date_until(end, start)
}

pub extern "C" fn tyrus_temporal_plain_time_from(input: TyrusString) -> PlainTime {
    let Some(input) = (!input.is_null()).then(|| unsafe { input.as_str() }) else {
        set_temporal_error(c"ERR_TEMPORAL_INVALID_TIME", c"time must not be null");
        return PlainTime {
            hour: 0,
            minute: 0,
            second: 0,
            nanosecond: 0,
        };
    };
    match input.parse::<Time>() {
        Ok(time) => plain_time_from_jiff(time),
        Err(_) => {
            set_temporal_error(c"ERR_TEMPORAL_INVALID_TIME", c"invalid ISO clock time");
            PlainTime {
                hour: 0,
                minute: 0,
                second: 0,
                nanosecond: 0,
            }
        }
    }
}

pub extern "C" fn tyrus_temporal_plain_time_to_string(time: PlainTime) -> TyrusString {
    let Ok(time) = jiff_from_plain_time(time) else {
        set_temporal_error(c"ERR_TEMPORAL_INVALID_TIME", c"time fields are invalid");
        return TyrusString::null();
    };
    alloc_string(&time.to_string())
}

pub extern "C" fn tyrus_temporal_plain_time_equals(left: PlainTime, right: PlainTime) -> bool {
    left == right
}

pub extern "C" fn tyrus_temporal_plain_time_add_seconds(
    time: PlainTime,
    seconds: i64,
) -> PlainTime {
    let Ok(time) = jiff_from_plain_time(time) else {
        set_temporal_error(c"ERR_TEMPORAL_INVALID_TIME", c"time fields are invalid");
        return time;
    };
    let total_seconds = i128::from(time.hour()) * 3_600
        + i128::from(time.minute()) * 60
        + i128::from(time.second())
        + i128::from(seconds);
    let wrapped_seconds = total_seconds.rem_euclid(86_400);
    let hour = (wrapped_seconds / 3_600) as i8;
    let minute = ((wrapped_seconds % 3_600) / 60) as i8;
    let second = (wrapped_seconds % 60) as i8;
    match Time::new(hour, minute, second, time.subsec_nanosecond()) {
        Ok(time) => plain_time_from_jiff(time),
        Err(_) => {
            set_temporal_error(c"ERR_TEMPORAL_INVALID_TIME", c"time arithmetic failed");
            PlainTime {
                hour: 0,
                minute: 0,
                second: 0,
                nanosecond: 0,
            }
        }
    }
}

pub extern "C" fn tyrus_temporal_plain_time_seconds_until(start: PlainTime, end: PlainTime) -> i64 {
    let (Ok(start), Ok(end)) = (jiff_from_plain_time(start), jiff_from_plain_time(end)) else {
        set_temporal_error(c"ERR_TEMPORAL_INVALID_TIME", c"time fields are invalid");
        return 0;
    };
    end.second() as i64 - start.second() as i64
        + (end.minute() as i64 - start.minute() as i64) * 60
        + (end.hour() as i64 - start.hour() as i64) * 3_600
}

pub extern "C" fn tyrus_temporal_plain_time_add_nanoseconds(
    time: PlainTime,
    nanoseconds: i64,
) -> PlainTime {
    let original = time;
    let Ok(time) = jiff_from_plain_time(time) else {
        set_temporal_error(c"ERR_TEMPORAL_INVALID_TIME", c"time fields are invalid");
        return original;
    };
    let total_nanoseconds = (i128::from(time.hour()) * 3_600
        + i128::from(time.minute()) * 60
        + i128::from(time.second()))
        * 1_000_000_000
        + i128::from(time.subsec_nanosecond())
        + i128::from(nanoseconds);
    let wrapped_nanoseconds = total_nanoseconds.rem_euclid(86_400_000_000_000);
    let hour = (wrapped_nanoseconds / 3_600_000_000_000) as i8;
    let minute = ((wrapped_nanoseconds % 3_600_000_000_000) / 60_000_000_000) as i8;
    let second = ((wrapped_nanoseconds % 60_000_000_000) / 1_000_000_000) as i8;
    let nanosecond = (wrapped_nanoseconds % 1_000_000_000) as i32;
    match Time::new(hour, minute, second, nanosecond) {
        Ok(time) => plain_time_from_jiff(time),
        Err(_) => {
            set_temporal_error(c"ERR_TEMPORAL_INVALID_TIME", c"time arithmetic failed");
            original
        }
    }
}

pub extern "C" fn tyrus_temporal_plain_time_nanoseconds_until(
    start: PlainTime,
    end: PlainTime,
) -> i64 {
    let (Ok(start), Ok(end)) = (jiff_from_plain_time(start), jiff_from_plain_time(end)) else {
        set_temporal_error(c"ERR_TEMPORAL_INVALID_TIME", c"time fields are invalid");
        return 0;
    };
    let start = (i64::from(start.hour()) * 3_600
        + i64::from(start.minute()) * 60
        + i64::from(start.second()))
        * 1_000_000_000
        + i64::from(start.subsec_nanosecond());
    let end =
        (i64::from(end.hour()) * 3_600 + i64::from(end.minute()) * 60 + i64::from(end.second()))
            * 1_000_000_000
            + i64::from(end.subsec_nanosecond());
    end - start
}

pub extern "C" fn tyrus_temporal_plain_time_from_fields(time: PlainTime) -> PlainTime {
    if jiff_from_plain_time(time).is_err() {
        set_temporal_error(c"ERR_TEMPORAL_INVALID_TIME", c"time fields are invalid");
    }
    time
}

pub extern "C" fn tyrus_temporal_plain_time_subtract_duration(
    time: PlainTime,
    duration: Duration,
) -> PlainTime {
    let Some(negated) = negate_duration(duration) else {
        set_temporal_error(
            c"ERR_TEMPORAL_DURATION_OVERFLOW",
            c"duration subtraction overflow",
        );
        return time;
    };
    tyrus_temporal_plain_time_add_duration(time, negated)
}

pub extern "C" fn tyrus_temporal_plain_time_until(start: PlainTime, end: PlainTime) -> Duration {
    duration_from_nanoseconds(i128::from(tyrus_temporal_plain_time_nanoseconds_until(
        start, end,
    )))
}

pub extern "C" fn tyrus_temporal_plain_time_since(start: PlainTime, end: PlainTime) -> Duration {
    tyrus_temporal_plain_time_until(end, start)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_date_time_from(
    input: TyrusString,
    out: *mut PlainDateTime,
) {
    unsafe {
        let invalid = PlainDateTime {
            date: PlainDate {
                year: 0,
                month: 1,
                day: 1,
            },
            time: PlainTime {
                hour: 0,
                minute: 0,
                second: 0,
                nanosecond: 0,
            },
        };
        let Some(text) = (!input.is_null()).then(|| input.as_str()) else {
            set_temporal_error(
                c"ERR_TEMPORAL_INVALID_DATE_TIME",
                c"date-time must be valid UTF-8",
            );
            write_plain_date_time(out, invalid);
            return;
        };
        let result = match text.parse::<DateTime>() {
            Ok(value) => plain_date_time_from_jiff(value),
            Err(_) => {
                set_temporal_error(c"ERR_TEMPORAL_INVALID_DATE_TIME", c"invalid ISO date-time");
                invalid
            }
        };
        write_plain_date_time(out, result);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_date_time_from_fields(
    value: *const PlainDateTime,
    out: *mut PlainDateTime,
) {
    unsafe {
        let Some(value) = read_plain_date_time(value) else {
            write_plain_date_time(out, invalid_plain_date_time());
            return;
        };
        if jiff_from_plain_date_time(value).is_err() {
            set_temporal_error(
                c"ERR_TEMPORAL_INVALID_DATE_TIME",
                c"date-time fields are invalid",
            );
        }
        write_plain_date_time(out, value);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_date_time_to_string(
    value: *const PlainDateTime,
) -> TyrusString {
    unsafe {
        let Some(value) = read_plain_date_time(value) else {
            return TyrusString::null();
        };
        let Ok(value) = jiff_from_plain_date_time(value) else {
            set_temporal_error(
                c"ERR_TEMPORAL_INVALID_DATE_TIME",
                c"date-time fields are invalid",
            );
            return TyrusString::null();
        };
        let text = value.to_string().replace(' ', "T");
        alloc_string(&text)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_date_time_equals(
    left: *const PlainDateTime,
    right: *const PlainDateTime,
) -> bool {
    unsafe {
        let (Some(left), Some(right)) = (read_plain_date_time(left), read_plain_date_time(right))
        else {
            return false;
        };
        left == right
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_date_time_with(
    value: *const PlainDateTime,
    replacement: *const PlainDateTime,
    out: *mut PlainDateTime,
) {
    unsafe {
        let (Some(value), Some(replacement)) = (
            read_plain_date_time(value),
            read_plain_date_time(replacement),
        ) else {
            write_plain_date_time(out, invalid_plain_date_time());
            return;
        };
        if jiff_from_plain_date_time(replacement).is_err() {
            set_temporal_error(
                c"ERR_TEMPORAL_INVALID_DATE_TIME",
                c"replacement date-time fields are invalid",
            );
        }
        let _ = value;
        write_plain_date_time(out, replacement);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_date_time_add_duration(
    value: *const PlainDateTime,
    duration: *const Duration,
    out: *mut PlainDateTime,
) {
    unsafe {
        let Some(value) = read_plain_date_time(value) else {
            write_plain_date_time(out, invalid_plain_date_time());
            return;
        };
        let Some(duration) = read_ffi_value(
            duration,
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration pointer must not be null",
        ) else {
            return;
        };
        let Ok(value) = jiff_from_plain_date_time(value) else {
            set_temporal_error(
                c"ERR_TEMPORAL_INVALID_DATE_TIME",
                c"date-time fields are invalid",
            );
            write_plain_date_time(out, value);
            return;
        };
        let Ok(span) = duration_to_span(duration) else {
            set_temporal_error(
                c"ERR_TEMPORAL_DURATION_OVERFLOW",
                c"duration fields exceed the supported range",
            );
            write_plain_date_time(out, plain_date_time_from_jiff(value));
            return;
        };
        let result = match value.checked_add(span) {
            Ok(result) => plain_date_time_from_jiff(result),
            Err(_) => {
                set_temporal_error(
                    c"ERR_TEMPORAL_DATE_TIME_OVERFLOW",
                    c"date-time arithmetic overflow",
                );
                plain_date_time_from_jiff(value)
            }
        };
        write_plain_date_time(out, result);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_date_time_subtract_duration(
    value: *const PlainDateTime,
    duration: *const Duration,
    out: *mut PlainDateTime,
) {
    unsafe {
        let Some(value) = read_plain_date_time(value) else {
            write_plain_date_time(out, invalid_plain_date_time());
            return;
        };
        let Some(duration) = read_ffi_value(
            duration,
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration pointer must not be null",
        ) else {
            return;
        };
        let Some(negated) = negate_duration(duration) else {
            set_temporal_error(
                c"ERR_TEMPORAL_DURATION_OVERFLOW",
                c"duration subtraction overflow",
            );
            write_plain_date_time(out, value);
            return;
        };
        tyrus_temporal_plain_date_time_add_duration(&value, &negated, out);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_date_time_until(
    start: *const PlainDateTime,
    end: *const PlainDateTime,
    out: *mut Duration,
) {
    unsafe {
        let (Some(start), Some(end)) = (read_plain_date_time(start), read_plain_date_time(end))
        else {
            return;
        };
        write_ffi_value(
            out,
            plain_date_time_until_value(start, end),
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration output pointer must not be null",
        );
    }
}

fn plain_date_time_until_value(start: PlainDateTime, end: PlainDateTime) -> Duration {
    let (Ok(start), Ok(end)) = (
        jiff_from_plain_date_time(start),
        jiff_from_plain_date_time(end),
    ) else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_DATE_TIME",
            c"date-time fields are invalid",
        );
        return invalid_duration();
    };
    match start.until(end) {
        Ok(span) => duration_from_span(span),
        Err(_) => {
            set_temporal_error(
                c"ERR_TEMPORAL_DATE_TIME_DIFFERENCE",
                c"date-time difference failed",
            );
            invalid_duration()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_date_time_since(
    start: *const PlainDateTime,
    end: *const PlainDateTime,
    out: *mut Duration,
) {
    unsafe {
        tyrus_temporal_plain_date_time_until(end, start, out);
    }
}

pub extern "C" fn tyrus_temporal_plain_date_until_with_options(
    start: PlainDate,
    end: PlainDate,
    largest: TyrusString,
    smallest: TyrusString,
    increment: i64,
    mode: TyrusString,
) -> Duration {
    let (Ok(start), Ok(end)) = (jiff_from_plain_date(start), jiff_from_plain_date(end)) else {
        set_temporal_error(c"ERR_TEMPORAL_INVALID_DATE", c"date fields are invalid");
        return invalid_duration();
    };
    let Ok((largest, smallest, increment, mode)) =
        difference_options(largest, smallest, increment, mode)
    else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_DIFFERENCE_OPTIONS",
            c"invalid date difference options",
        );
        return invalid_duration();
    };
    let mut options = DateDifference::new(end).increment(increment).mode(mode);
    if let Some(unit) = largest {
        options = options.largest(unit);
    }
    if let Some(unit) = smallest {
        options = options.smallest(unit);
    }
    match start.until(options) {
        Ok(span) => duration_from_span(span),
        Err(_) => {
            set_temporal_error(
                c"ERR_TEMPORAL_DATE_DIFFERENCE",
                c"date difference options are incompatible",
            );
            invalid_duration()
        }
    }
}

pub extern "C" fn tyrus_temporal_plain_date_since_with_options(
    start: PlainDate,
    end: PlainDate,
    largest: TyrusString,
    smallest: TyrusString,
    increment: i64,
    mode: TyrusString,
) -> Duration {
    tyrus_temporal_plain_date_until_with_options(end, start, largest, smallest, increment, mode)
}

pub extern "C" fn tyrus_temporal_plain_time_until_with_options(
    start: PlainTime,
    end: PlainTime,
    largest: TyrusString,
    smallest: TyrusString,
    increment: i64,
    mode: TyrusString,
) -> Duration {
    let (Ok(start), Ok(end)) = (jiff_from_plain_time(start), jiff_from_plain_time(end)) else {
        set_temporal_error(c"ERR_TEMPORAL_INVALID_TIME", c"time fields are invalid");
        return invalid_duration();
    };
    let Ok((largest, smallest, increment, mode)) =
        difference_options(largest, smallest, increment, mode)
    else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_DIFFERENCE_OPTIONS",
            c"invalid time difference options",
        );
        return invalid_duration();
    };
    let mut options = TimeDifference::new(end).increment(increment).mode(mode);
    if let Some(unit) = largest {
        options = options.largest(unit);
    }
    if let Some(unit) = smallest {
        options = options.smallest(unit);
    }
    match start.until(options) {
        Ok(span) => duration_from_span(span),
        Err(_) => {
            set_temporal_error(
                c"ERR_TEMPORAL_TIME_DIFFERENCE",
                c"time difference options are incompatible",
            );
            invalid_duration()
        }
    }
}

pub extern "C" fn tyrus_temporal_plain_time_since_with_options(
    start: PlainTime,
    end: PlainTime,
    largest: TyrusString,
    smallest: TyrusString,
    increment: i64,
    mode: TyrusString,
) -> Duration {
    tyrus_temporal_plain_time_until_with_options(end, start, largest, smallest, increment, mode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_date_time_until_with_options(
    start: *const PlainDateTime,
    end: *const PlainDateTime,
    largest: TyrusString,
    smallest: TyrusString,
    increment: i64,
    mode: TyrusString,
    out: *mut Duration,
) {
    unsafe {
        let (Some(start), Some(end)) = (read_plain_date_time(start), read_plain_date_time(end))
        else {
            return;
        };
        let result = plain_date_time_until_with_options_value(
            start, end, largest, smallest, increment, mode,
        );
        write_ffi_value(
            out,
            result,
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration output pointer must not be null",
        );
    }
}

fn plain_date_time_until_with_options_value(
    start: PlainDateTime,
    end: PlainDateTime,
    largest: TyrusString,
    smallest: TyrusString,
    increment: i64,
    mode: TyrusString,
) -> Duration {
    let (Ok(start), Ok(end)) = (
        jiff_from_plain_date_time(start),
        jiff_from_plain_date_time(end),
    ) else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_DATE_TIME",
            c"date-time fields are invalid",
        );
        return invalid_duration();
    };
    let Ok((largest, smallest, increment, mode)) =
        difference_options(largest, smallest, increment, mode)
    else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_DIFFERENCE_OPTIONS",
            c"invalid date-time difference options",
        );
        return invalid_duration();
    };
    let mut options = DateTimeDifference::new(end).increment(increment).mode(mode);
    if let Some(unit) = largest {
        options = options.largest(unit);
    }
    if let Some(unit) = smallest {
        options = options.smallest(unit);
    }
    match start.until(options) {
        Ok(span) => duration_from_span(span),
        Err(_) => {
            set_temporal_error(
                c"ERR_TEMPORAL_DATE_TIME_DIFFERENCE",
                c"date-time difference options are incompatible",
            );
            invalid_duration()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_date_time_since_with_options(
    start: *const PlainDateTime,
    end: *const PlainDateTime,
    largest: TyrusString,
    smallest: TyrusString,
    increment: i64,
    mode: TyrusString,
    out: *mut Duration,
) {
    unsafe {
        tyrus_temporal_plain_date_time_until_with_options(
            end, start, largest, smallest, increment, mode, out,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_year_month_from(
    input: TyrusString,
    out: *mut PlainYearMonth,
) {
    unsafe {
        let invalid = PlainYearMonth { year: 0, month: 1 };
        let Some(text) = (!input.is_null()).then(|| input.as_str()) else {
            set_temporal_error(
                c"ERR_TEMPORAL_INVALID_YEAR_MONTH",
                c"year-month must be valid UTF-8",
            );
            write_ffi_value(
                out,
                invalid,
                c"ERR_TEMPORAL_INVALID_YEAR_MONTH",
                c"year-month output pointer must not be null",
            );
            return;
        };
        let Ok(date) = format!("{text}-01").parse::<Date>() else {
            set_temporal_error(
                c"ERR_TEMPORAL_INVALID_YEAR_MONTH",
                c"invalid ISO year-month",
            );
            write_ffi_value(
                out,
                invalid,
                c"ERR_TEMPORAL_INVALID_YEAR_MONTH",
                c"year-month output pointer must not be null",
            );
            return;
        };
        write_ffi_value(
            out,
            PlainYearMonth {
                year: i32::from(date.year()),
                month: u32::try_from(date.month()).unwrap_or(0),
            },
            c"ERR_TEMPORAL_INVALID_YEAR_MONTH",
            c"year-month output pointer must not be null",
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_year_month_from_fields(
    value: *const PlainYearMonth,
    out: *mut PlainYearMonth,
) {
    unsafe {
        let Some(value) = read_ffi_value(
            value,
            c"ERR_TEMPORAL_INVALID_YEAR_MONTH",
            c"year-month pointer must not be null",
        ) else {
            return;
        };
        if jiff_from_plain_year_month(value).is_err() {
            set_temporal_error(
                c"ERR_TEMPORAL_INVALID_YEAR_MONTH",
                c"year-month fields are invalid",
            );
        }
        write_ffi_value(
            out,
            value,
            c"ERR_TEMPORAL_INVALID_YEAR_MONTH",
            c"year-month output pointer must not be null",
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_year_month_to_string(
    value: *const PlainYearMonth,
) -> TyrusString {
    unsafe {
        let Some(value) = read_ffi_value(
            value,
            c"ERR_TEMPORAL_INVALID_YEAR_MONTH",
            c"year-month pointer must not be null",
        ) else {
            return TyrusString::null();
        };
        if jiff_from_plain_year_month(value).is_err() {
            set_temporal_error(
                c"ERR_TEMPORAL_INVALID_YEAR_MONTH",
                c"year-month fields are invalid",
            );
            return TyrusString::null();
        }
        alloc_string(&format!("{:04}-{:02}", value.year, value.month))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_year_month_add(
    value: *const PlainYearMonth,
    months: i64,
    out: *mut PlainYearMonth,
) {
    unsafe {
        let Some(value) = read_ffi_value(
            value,
            c"ERR_TEMPORAL_INVALID_YEAR_MONTH",
            c"year-month pointer must not be null",
        ) else {
            return;
        };
        if jiff_from_plain_year_month(value).is_err() {
            set_temporal_error(
                c"ERR_TEMPORAL_INVALID_YEAR_MONTH",
                c"year-month fields are invalid",
            );
            write_ffi_value(
                out,
                value,
                c"ERR_TEMPORAL_INVALID_YEAR_MONTH",
                c"year-month output pointer must not be null",
            );
            return;
        }
        let total = i128::from(value.year) * 12 + i128::from(value.month) - 1 + i128::from(months);
        let year = total.div_euclid(12);
        let month = total.rem_euclid(12) + 1;
        let Ok(year) = i32::try_from(year) else {
            set_temporal_error(
                c"ERR_TEMPORAL_YEAR_MONTH_OVERFLOW",
                c"year-month arithmetic overflow",
            );
            write_ffi_value(
                out,
                value,
                c"ERR_TEMPORAL_INVALID_YEAR_MONTH",
                c"year-month output pointer must not be null",
            );
            return;
        };
        let result = PlainYearMonth {
            year,
            month: u32::try_from(month).unwrap_or(1),
        };
        if jiff_from_plain_year_month(result).is_err() {
            set_temporal_error(
                c"ERR_TEMPORAL_INVALID_YEAR_MONTH",
                c"year-month fields are invalid",
            );
        }
        write_ffi_value(
            out,
            result,
            c"ERR_TEMPORAL_INVALID_YEAR_MONTH",
            c"year-month output pointer must not be null",
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_month_day_from(
    input: TyrusString,
    out: *mut PlainMonthDay,
) {
    unsafe {
        let invalid = PlainMonthDay { month: 1, day: 1 };
        let Some(text) = (!input.is_null()).then(|| input.as_str()) else {
            set_temporal_error(
                c"ERR_TEMPORAL_INVALID_MONTH_DAY",
                c"month-day must be valid UTF-8",
            );
            write_ffi_value(
                out,
                invalid,
                c"ERR_TEMPORAL_INVALID_MONTH_DAY",
                c"month-day output pointer must not be null",
            );
            return;
        };
        let Ok(date) = format!("1972-{text}").parse::<Date>() else {
            set_temporal_error(c"ERR_TEMPORAL_INVALID_MONTH_DAY", c"invalid ISO month-day");
            write_ffi_value(
                out,
                invalid,
                c"ERR_TEMPORAL_INVALID_MONTH_DAY",
                c"month-day output pointer must not be null",
            );
            return;
        };
        write_ffi_value(
            out,
            PlainMonthDay {
                month: u32::try_from(date.month()).unwrap_or(0),
                day: u32::try_from(date.day()).unwrap_or(0),
            },
            c"ERR_TEMPORAL_INVALID_MONTH_DAY",
            c"month-day output pointer must not be null",
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_month_day_from_fields(
    value: *const PlainMonthDay,
    out: *mut PlainMonthDay,
) {
    unsafe {
        let Some(value) = read_ffi_value(
            value,
            c"ERR_TEMPORAL_INVALID_MONTH_DAY",
            c"month-day pointer must not be null",
        ) else {
            return;
        };
        if jiff_from_plain_month_day(value).is_err() {
            set_temporal_error(
                c"ERR_TEMPORAL_INVALID_MONTH_DAY",
                c"month-day fields are invalid",
            );
        }
        write_ffi_value(
            out,
            value,
            c"ERR_TEMPORAL_INVALID_MONTH_DAY",
            c"month-day output pointer must not be null",
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_plain_month_day_to_string(
    value: *const PlainMonthDay,
) -> TyrusString {
    unsafe {
        let Some(value) = read_ffi_value(
            value,
            c"ERR_TEMPORAL_INVALID_MONTH_DAY",
            c"month-day pointer must not be null",
        ) else {
            return TyrusString::null();
        };
        if jiff_from_plain_month_day(value).is_err() {
            set_temporal_error(
                c"ERR_TEMPORAL_INVALID_MONTH_DAY",
                c"month-day fields are invalid",
            );
            return TyrusString::null();
        }
        alloc_string(&format!("{:02}-{:02}", value.month, value.day))
    }
}

pub extern "C" fn tyrus_temporal_plain_month_day_to_plain_date(
    value: PlainMonthDay,
    year: i32,
) -> PlainDate {
    let date = PlainDate {
        year,
        month: value.month,
        day: value.day,
    };
    tyrus_temporal_plain_date_from_fields(date)
}

pub extern "C" fn tyrus_temporal_duration_round(
    duration: Duration,
    unit: TyrusString,
    increment: i64,
    mode: TyrusString,
    relative_to: PlainDate,
) -> Duration {
    let Some(unit) = (!unit.is_null()).then(|| unsafe { unit.as_str() }) else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_DURATION_UNIT",
            c"duration unit must not be null",
        );
        return invalid_duration();
    };
    let Some(unit) = temporal_unit(unit) else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_DURATION_UNIT",
            c"unsupported duration rounding unit",
        );
        return invalid_duration();
    };
    let Some(mode) = (!mode.is_null()).then(|| unsafe { mode.as_str() }) else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_ROUNDING_MODE",
            c"rounding mode must not be null",
        );
        return invalid_duration();
    };
    let Some(mode) = temporal_round_mode(mode) else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_ROUNDING_MODE",
            c"unsupported duration rounding mode",
        );
        return invalid_duration();
    };
    let Ok(date) = jiff_from_plain_date(relative_to) else {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_DATE",
            c"relativeTo date fields are invalid",
        );
        return invalid_duration();
    };
    if increment <= 0 {
        set_temporal_error(
            c"ERR_TEMPORAL_INVALID_ROUNDING_INCREMENT",
            c"rounding increment must be positive",
        );
        return invalid_duration();
    }
    let round = SpanRound::new()
        .smallest(unit)
        .increment(increment)
        .mode(mode)
        .relative(date);
    let Ok(span) = duration_to_span(duration) else {
        set_temporal_error(
            c"ERR_TEMPORAL_DURATION_OVERFLOW",
            c"duration fields exceed the supported range",
        );
        return invalid_duration();
    };
    match span.round(round) {
        Ok(value) => duration_from_span(value),
        Err(_) => {
            set_temporal_error(
                c"ERR_TEMPORAL_DURATION_ROUNDING",
                c"duration rounding failed for the selected unit",
            );
            invalid_duration()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_duration_from_fields(
    duration: *const Duration,
    out: *mut Duration,
) {
    unsafe {
        let Some(duration) = read_ffi_value(
            duration,
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration pointer must not be null",
        ) else {
            return;
        };
        write_ffi_value(
            out,
            duration,
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration output pointer must not be null",
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_temporal_duration_total(
    duration: *const Duration,
    unit: TyrusString,
) -> i64 {
    unsafe {
        let Some(duration) = read_ffi_value(
            duration,
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration pointer must not be null",
        ) else {
            return 0;
        };
        let Some(unit) = (!unit.is_null()).then(|| unit.as_str()) else {
            set_temporal_error(
                c"ERR_TEMPORAL_INVALID_DURATION_UNIT",
                c"duration unit must not be null",
            );
            return 0;
        };
        if duration.years != 0 || duration.months != 0 {
            set_temporal_error(
                c"ERR_TEMPORAL_VARIABLE_DURATION_UNIT",
                c"years and months require a calendar reference",
            );
            return 0;
        }
        let nanoseconds = i128::from(duration.weeks) * 604_800_000_000_000
            + i128::from(duration.days) * 86_400_000_000_000
            + i128::from(duration.hours) * 3_600_000_000_000
            + i128::from(duration.minutes) * 60_000_000_000
            + i128::from(duration.seconds) * 1_000_000_000
            + i128::from(duration.milliseconds) * 1_000_000
            + i128::from(duration.microseconds) * 1_000
            + i128::from(duration.nanoseconds);
        let divisor = match unit {
            "weeks" => 604_800_000_000_000,
            "days" => 86_400_000_000_000,
            "hours" => 3_600_000_000_000,
            "minutes" => 60_000_000_000,
            "seconds" => 1_000_000_000,
            "milliseconds" => 1_000_000,
            "microseconds" => 1_000,
            "nanoseconds" => 1,
            _ => {
                set_temporal_error(
                    c"ERR_TEMPORAL_INVALID_DURATION_UNIT",
                    c"unsupported duration unit",
                );
                return 0;
            }
        };
        match i64::try_from(nanoseconds / divisor) {
            Ok(total) => total,
            Err(_) => {
                set_temporal_error(
                    c"ERR_TEMPORAL_DURATION_OVERFLOW",
                    c"duration total exceeds i64",
                );
                0
            }
        }
    }
}

pub extern "C" fn tyrus_temporal_plain_date_add_duration(
    date: PlainDate,
    duration: Duration,
) -> PlainDate {
    if duration.hours != 0
        || duration.minutes != 0
        || duration.seconds != 0
        || duration.milliseconds != 0
        || duration.microseconds != 0
        || duration.nanoseconds != 0
    {
        set_temporal_error(
            c"ERR_TEMPORAL_DATE_CLOCK_DURATION",
            c"date arithmetic cannot contain clock units",
        );
        return date;
    }
    let Ok(date_value) = jiff_from_plain_date(date) else {
        set_temporal_error(c"ERR_TEMPORAL_INVALID_DATE", c"date fields are invalid");
        return date;
    };
    let Ok(span) = Span::new()
        .try_years(duration.years)
        .and_then(|span| span.try_months(duration.months))
        .and_then(|span| span.try_weeks(duration.weeks))
        .and_then(|span| span.try_days(duration.days))
    else {
        set_temporal_error(
            c"ERR_TEMPORAL_DURATION_OVERFLOW",
            c"duration fields exceed the supported range",
        );
        return date;
    };
    match date_value.checked_add(span) {
        Ok(result) => plain_date_from_jiff(result),
        Err(_) => {
            set_temporal_error(
                c"ERR_TEMPORAL_DATE_OVERFLOW",
                c"date arithmetic exceeds the supported calendar range",
            );
            date
        }
    }
}

pub extern "C" fn tyrus_temporal_plain_time_add_duration(
    time: PlainTime,
    duration: Duration,
) -> PlainTime {
    if duration.years != 0 || duration.months != 0 {
        set_temporal_error(
            c"ERR_TEMPORAL_TIME_CALENDAR_DURATION",
            c"time arithmetic cannot contain years or months",
        );
        return time;
    }
    let total_nanoseconds = i128::from(duration.weeks) * 604_800_000_000_000
        + i128::from(duration.days) * 86_400_000_000_000
        + i128::from(duration.hours) * 3_600_000_000_000
        + i128::from(duration.minutes) * 60_000_000_000
        + i128::from(duration.seconds) * 1_000_000_000
        + i128::from(duration.milliseconds) * 1_000_000
        + i128::from(duration.microseconds) * 1_000
        + i128::from(duration.nanoseconds);
    let Ok(total_nanoseconds) = i64::try_from(total_nanoseconds) else {
        set_temporal_error(
            c"ERR_TEMPORAL_DURATION_OVERFLOW",
            c"time duration exceeds i64 nanoseconds",
        );
        return time;
    };
    tyrus_temporal_plain_time_add_nanoseconds(time, total_nanoseconds)
}

#[cfg(test)]
mod tests {
    use super::{
        Duration, Instant, TemporalResultFfi, tyrus_temporal_duration_from_fields,
        tyrus_temporal_duration_total, tyrus_temporal_instant_from_epoch_milliseconds,
        tyrus_temporal_instant_to_epoch_milliseconds, tyrus_temporal_instant_to_string,
        tyrus_temporal_instant_to_zoned_date_time, tyrus_temporal_plain_date_add_days,
        tyrus_temporal_plain_date_add_duration, tyrus_temporal_plain_date_add_months,
        tyrus_temporal_plain_date_add_years, tyrus_temporal_plain_date_day_of_week,
        tyrus_temporal_plain_date_day_of_year, tyrus_temporal_plain_date_days_until,
        tyrus_temporal_plain_date_equals, tyrus_temporal_plain_date_from,
        tyrus_temporal_plain_date_from_fields, tyrus_temporal_plain_date_since,
        tyrus_temporal_plain_date_subtract_duration, tyrus_temporal_plain_date_to_string,
        tyrus_temporal_plain_date_until, tyrus_temporal_plain_date_with,
        tyrus_temporal_plain_time_add_duration, tyrus_temporal_plain_time_add_nanoseconds,
        tyrus_temporal_plain_time_add_seconds, tyrus_temporal_plain_time_equals,
        tyrus_temporal_plain_time_from, tyrus_temporal_plain_time_from_fields,
        tyrus_temporal_plain_time_nanoseconds_until, tyrus_temporal_plain_time_seconds_until,
        tyrus_temporal_plain_time_since, tyrus_temporal_plain_time_subtract_duration,
        tyrus_temporal_plain_time_to_string, tyrus_temporal_plain_time_until,
        tyrus_temporal_timezone_count, tyrus_temporal_timezone_name,
        tyrus_temporal_zoned_date_time_add_duration, tyrus_temporal_zoned_date_time_equals,
        tyrus_temporal_zoned_date_time_format, tyrus_temporal_zoned_date_time_from,
        tyrus_temporal_zoned_date_time_to_instant, tyrus_temporal_zoned_date_time_to_string,
        tyrus_temporal_zoned_date_time_until,
    };
    use tyrus_string::TyrusString;

    unsafe fn assert_temporal_result_error<T>(result: TemporalResultFfi<T>, expected_code: &str) {
        assert_eq!(result.tag, 1);
        let error = unsafe { std::mem::ManuallyDrop::into_inner(result.payload.err) };
        assert_eq!(unsafe { error.code.as_str() }, expected_code);
        unsafe {
            error.code.release();
            error.message.release();
        }
    }

    #[test]
    fn temporal_result_constructors_return_ok_and_err_payloads() {
        let zone_name = TyrusString::literal("Europe/Paris");
        let mut zone = std::mem::MaybeUninit::<TemporalResultFfi<i32>>::uninit();
        unsafe { super::tyrus_temporal_timezone_from_result(zone_name, zone.as_mut_ptr()) };
        let zone = unsafe { zone.assume_init() };
        assert_eq!(zone.tag, 0);
        assert_ne!(unsafe { *zone.payload.ok }, 0);

        let invalid_name = TyrusString::literal("Not/A_Real_Zone");
        let mut invalid_zone = std::mem::MaybeUninit::<TemporalResultFfi<i32>>::uninit();
        unsafe {
            super::tyrus_temporal_timezone_from_result(invalid_name, invalid_zone.as_mut_ptr())
        };
        let invalid_zone = unsafe { invalid_zone.assume_init() };
        assert_eq!(invalid_zone.tag, 1);
        let error = unsafe { std::mem::ManuallyDrop::into_inner(invalid_zone.payload.err) };
        assert_eq!(
            unsafe { error.code.as_str() },
            "ERR_TEMPORAL_INVALID_TIME_ZONE"
        );
        unsafe {
            error.code.release();
            error.message.release();
        }

        let mut epoch = std::mem::MaybeUninit::<TemporalResultFfi<Instant>>::uninit();
        unsafe {
            super::tyrus_temporal_instant_from_epoch_milliseconds_result(0, epoch.as_mut_ptr())
        };
        let epoch = unsafe { epoch.assume_init() };
        assert_eq!(epoch.tag, 0);
        assert_eq!(unsafe { (*epoch.payload.ok).seconds }, 0);

        let mut invalid_epoch = std::mem::MaybeUninit::<TemporalResultFfi<Instant>>::uninit();
        unsafe {
            super::tyrus_temporal_instant_from_epoch_milliseconds_result(
                i64::MAX,
                invalid_epoch.as_mut_ptr(),
            )
        };
        let invalid_epoch = unsafe { invalid_epoch.assume_init() };
        assert_eq!(invalid_epoch.tag, 1);
        let error = unsafe { std::mem::ManuallyDrop::into_inner(invalid_epoch.payload.err) };
        assert_eq!(
            unsafe { error.code.as_str() },
            "ERR_TEMPORAL_INVALID_INSTANT"
        );
        unsafe {
            error.code.release();
            error.message.release();
        }
    }

    #[test]
    fn plain_date_result_operations_preserve_success_and_error_variants() {
        let date = super::PlainDate {
            year: 2026,
            month: 7,
            day: 9,
        };
        let mut next = std::mem::MaybeUninit::<TemporalResultFfi<super::PlainDate>>::uninit();
        unsafe { super::tyrus_temporal_plain_date_add_days_result(&date, 1, next.as_mut_ptr()) };
        let next = unsafe { next.assume_init() };
        assert_eq!(next.tag, 0);
        let next = unsafe { std::mem::ManuallyDrop::into_inner(next.payload.ok) };
        assert_eq!(next.year, 2026);
        assert_eq!(next.month, 7);
        assert_eq!(next.day, 10);

        let mut overflow = std::mem::MaybeUninit::<TemporalResultFfi<super::PlainDate>>::uninit();
        unsafe {
            super::tyrus_temporal_plain_date_add_days_result(&date, i64::MAX, overflow.as_mut_ptr())
        };
        let overflow = unsafe { overflow.assume_init() };
        unsafe { assert_temporal_result_error(overflow, "ERR_TEMPORAL_DATE_OVERFLOW") };
    }

    #[test]
    fn plain_time_result_operations_preserve_success_and_error_variants() {
        let time = super::PlainTime {
            hour: 14,
            minute: 30,
            second: 15,
            nanosecond: 500_000_000,
        };
        let mut text = std::mem::MaybeUninit::<TemporalResultFfi<TyrusString>>::uninit();
        unsafe { super::tyrus_temporal_plain_time_to_string_result(&time, text.as_mut_ptr()) };
        let text = unsafe { text.assume_init() };
        assert_eq!(text.tag, 0);
        let text = unsafe { std::mem::ManuallyDrop::into_inner(text.payload.ok) };
        assert_eq!(unsafe { text.as_str() }, "14:30:15.5");
        unsafe { text.release() };

        let invalid = super::PlainTime {
            hour: 24,
            minute: 0,
            second: 0,
            nanosecond: 0,
        };
        let mut invalid_text = std::mem::MaybeUninit::<TemporalResultFfi<TyrusString>>::uninit();
        unsafe {
            super::tyrus_temporal_plain_time_to_string_result(&invalid, invalid_text.as_mut_ptr())
        };
        let invalid_text = unsafe { invalid_text.assume_init() };
        unsafe { assert_temporal_result_error(invalid_text, "ERR_TEMPORAL_INVALID_TIME") };

        let end = super::PlainTime {
            hour: 15,
            minute: 0,
            second: 0,
            nanosecond: 0,
        };
        let mut difference = std::mem::MaybeUninit::<TemporalResultFfi<Duration>>::uninit();
        unsafe {
            super::tyrus_temporal_plain_time_until_with_options_result(
                &time,
                &end,
                TyrusString::literal("hours"),
                TyrusString::literal("minutes"),
                1,
                TyrusString::literal("trunc"),
                difference.as_mut_ptr(),
            )
        };
        let difference = unsafe { difference.assume_init() };
        assert_eq!(difference.tag, 0);
        let _difference = unsafe { std::mem::ManuallyDrop::into_inner(difference.payload.ok) };

        let mut invalid_options = std::mem::MaybeUninit::<TemporalResultFfi<Duration>>::uninit();
        unsafe {
            super::tyrus_temporal_plain_time_until_with_options_result(
                &time,
                &end,
                TyrusString::literal("invalid"),
                TyrusString::literal("minutes"),
                1,
                TyrusString::literal("trunc"),
                invalid_options.as_mut_ptr(),
            )
        };
        let invalid_options = unsafe { invalid_options.assume_init() };
        unsafe {
            assert_temporal_result_error(invalid_options, "ERR_TEMPORAL_INVALID_DIFFERENCE_OPTIONS")
        };
    }

    #[test]
    fn civil_calendar_result_operations_return_typed_values_and_errors() {
        let date_time = super::PlainDateTime {
            date: super::PlainDate {
                year: 2026,
                month: 7,
                day: 9,
            },
            time: super::PlainTime {
                hour: 23,
                minute: 30,
                second: 0,
                nanosecond: 0,
            },
        };
        let mut text = std::mem::MaybeUninit::<TemporalResultFfi<TyrusString>>::uninit();
        unsafe {
            super::tyrus_temporal_plain_date_time_to_string_result(&date_time, text.as_mut_ptr())
        };
        let text = unsafe { text.assume_init() };
        assert_eq!(text.tag, 0);
        let text = unsafe { std::mem::ManuallyDrop::into_inner(text.payload.ok) };
        assert_eq!(unsafe { text.as_str() }, "2026-07-09T23:30:00");
        unsafe { text.release() };

        let duration = Duration {
            years: 0,
            months: 0,
            weeks: 0,
            days: 0,
            hours: 1,
            minutes: 0,
            seconds: 0,
            milliseconds: 0,
            microseconds: 0,
            nanoseconds: 0,
        };
        let mut next_date_time =
            std::mem::MaybeUninit::<TemporalResultFfi<super::PlainDateTime>>::uninit();
        unsafe {
            super::tyrus_temporal_plain_date_time_add_duration_result(
                &date_time,
                &duration,
                next_date_time.as_mut_ptr(),
            )
        };
        let next_date_time = unsafe { next_date_time.assume_init() };
        assert_eq!(next_date_time.tag, 0);
        let next_date_time =
            unsafe { std::mem::ManuallyDrop::into_inner(next_date_time.payload.ok) };
        assert_eq!(next_date_time.date.day, 10);
        assert_eq!(next_date_time.time.hour, 0);
        assert_eq!(next_date_time.time.minute, 30);

        let mut date_time_difference =
            std::mem::MaybeUninit::<TemporalResultFfi<Duration>>::uninit();
        unsafe {
            super::tyrus_temporal_plain_date_time_until_with_options_result(
                &date_time,
                &next_date_time,
                TyrusString::literal("hours"),
                TyrusString::literal("minutes"),
                30,
                TyrusString::literal("halfExpand"),
                date_time_difference.as_mut_ptr(),
            )
        };
        let date_time_difference = unsafe { date_time_difference.assume_init() };
        assert_eq!(date_time_difference.tag, 0);
        let _date_time_difference =
            unsafe { std::mem::ManuallyDrop::into_inner(date_time_difference.payload.ok) };

        let year_month = super::PlainYearMonth {
            year: 2026,
            month: 7,
        };
        let mut next_month =
            std::mem::MaybeUninit::<TemporalResultFfi<super::PlainYearMonth>>::uninit();
        unsafe {
            super::tyrus_temporal_plain_year_month_add_result(
                &year_month,
                6,
                next_month.as_mut_ptr(),
            )
        };
        let next_month = unsafe { next_month.assume_init() };
        assert_eq!(next_month.tag, 0);
        let next_month = unsafe { std::mem::ManuallyDrop::into_inner(next_month.payload.ok) };
        assert_eq!(next_month.year, 2027);
        assert_eq!(next_month.month, 1);

        let mut overflow =
            std::mem::MaybeUninit::<TemporalResultFfi<super::PlainYearMonth>>::uninit();
        unsafe {
            super::tyrus_temporal_plain_year_month_add_result(
                &year_month,
                i64::MAX,
                overflow.as_mut_ptr(),
            )
        };
        let overflow = unsafe { overflow.assume_init() };
        unsafe { assert_temporal_result_error(overflow, "ERR_TEMPORAL_YEAR_MONTH_OVERFLOW") };

        let birthday = super::PlainMonthDay { month: 2, day: 29 };
        let mut date = std::mem::MaybeUninit::<TemporalResultFfi<super::PlainDate>>::uninit();
        unsafe {
            super::tyrus_temporal_plain_month_day_to_plain_date_result(
                &birthday,
                2028,
                date.as_mut_ptr(),
            )
        };
        let date = unsafe { date.assume_init() };
        assert_eq!(date.tag, 0);
        let date = unsafe { std::mem::ManuallyDrop::into_inner(date.payload.ok) };
        assert_eq!((date.year, date.month, date.day), (2028, 2, 29));

        let mut invalid_date =
            std::mem::MaybeUninit::<TemporalResultFfi<super::PlainDate>>::uninit();
        unsafe {
            super::tyrus_temporal_plain_month_day_to_plain_date_result(
                &birthday,
                2027,
                invalid_date.as_mut_ptr(),
            )
        };
        let invalid_date = unsafe { invalid_date.assume_init() };
        unsafe { assert_temporal_result_error(invalid_date, "ERR_TEMPORAL_INVALID_DATE") };
    }

    #[test]
    fn zoned_and_duration_result_operations_return_typed_values_and_errors() {
        let mut zoned = std::mem::MaybeUninit::<TemporalResultFfi<super::ZonedDateTime>>::uninit();
        unsafe {
            super::tyrus_temporal_zoned_date_time_from_result(
                TyrusString::literal("2026-07-09T14:30:00-03:00[America/Sao_Paulo]"),
                zoned.as_mut_ptr(),
            )
        };
        let zoned = unsafe { zoned.assume_init() };
        assert_eq!(zoned.tag, 0);
        let zoned = unsafe { std::mem::ManuallyDrop::into_inner(zoned.payload.ok) };

        let mut text = std::mem::MaybeUninit::<TemporalResultFfi<TyrusString>>::uninit();
        unsafe {
            super::tyrus_temporal_zoned_date_time_to_string_result(&zoned, text.as_mut_ptr())
        };
        let text = unsafe { text.assume_init() };
        assert_eq!(text.tag, 0);
        let text = unsafe { std::mem::ManuallyDrop::into_inner(text.payload.ok) };
        assert_eq!(
            unsafe { text.as_str() },
            "2026-07-09T14:30:00-03:00[America/Sao_Paulo]"
        );
        unsafe { text.release() };

        let mut formatted = std::mem::MaybeUninit::<TemporalResultFfi<TyrusString>>::uninit();
        unsafe {
            super::tyrus_temporal_zoned_date_time_format_result(
                &zoned,
                TyrusString::literal("yyyy-MM-dd HH:mm"),
                formatted.as_mut_ptr(),
            )
        };
        let formatted = unsafe { formatted.assume_init() };
        assert_eq!(formatted.tag, 0);
        let formatted = unsafe { std::mem::ManuallyDrop::into_inner(formatted.payload.ok) };
        assert_eq!(unsafe { formatted.as_str() }, "2026-07-09 14:30");
        unsafe { formatted.release() };

        let duration = Duration {
            years: 0,
            months: 0,
            weeks: 0,
            days: 0,
            hours: 1,
            minutes: 30,
            seconds: 0,
            milliseconds: 0,
            microseconds: 0,
            nanoseconds: 0,
        };
        let mut total = std::mem::MaybeUninit::<TemporalResultFfi<i64>>::uninit();
        unsafe {
            super::tyrus_temporal_duration_total_result(
                &duration,
                TyrusString::literal("minutes"),
                total.as_mut_ptr(),
            )
        };
        let total = unsafe { total.assume_init() };
        assert_eq!(total.tag, 0);
        assert_eq!(unsafe { *total.payload.ok }, 90);

        let relative_to = super::PlainDate {
            year: 2026,
            month: 7,
            day: 9,
        };
        let mut rounded = std::mem::MaybeUninit::<TemporalResultFfi<Duration>>::uninit();
        unsafe {
            super::tyrus_temporal_duration_round_result(
                &duration,
                TyrusString::literal("minutes"),
                15,
                TyrusString::literal("halfExpand"),
                &relative_to,
                rounded.as_mut_ptr(),
            )
        };
        let rounded = unsafe { rounded.assume_init() };
        assert_eq!(rounded.tag, 0);
        let _rounded = unsafe { std::mem::ManuallyDrop::into_inner(rounded.payload.ok) };

        let mut variable_total = std::mem::MaybeUninit::<TemporalResultFfi<i64>>::uninit();
        let year_duration = Duration {
            years: 1,
            ..duration
        };
        unsafe {
            super::tyrus_temporal_duration_total_result(
                &year_duration,
                TyrusString::literal("days"),
                variable_total.as_mut_ptr(),
            )
        };
        let variable_total = unsafe { variable_total.assume_init() };
        unsafe {
            assert_temporal_result_error(variable_total, "ERR_TEMPORAL_VARIABLE_DURATION_UNIT")
        };
    }

    #[test]
    fn instant_result_operations_return_owned_success_and_typed_errors() {
        let instant = Instant {
            seconds: 0,
            nanoseconds: 0,
        };
        let mut text = std::mem::MaybeUninit::<TemporalResultFfi<TyrusString>>::uninit();
        unsafe { super::tyrus_temporal_instant_to_string_result(&instant, text.as_mut_ptr()) };
        let text = unsafe { text.assume_init() };
        assert_eq!(text.tag, 0);
        let text = unsafe { std::mem::ManuallyDrop::into_inner(text.payload.ok) };
        assert_eq!(unsafe { text.as_str() }, "1970-01-01T00:00:00Z");
        unsafe { text.release() };

        let mut milliseconds = std::mem::MaybeUninit::<TemporalResultFfi<i64>>::uninit();
        unsafe {
            super::tyrus_temporal_instant_to_epoch_milliseconds_result(
                &instant,
                milliseconds.as_mut_ptr(),
            )
        };
        let milliseconds = unsafe { milliseconds.assume_init() };
        assert_eq!(milliseconds.tag, 0);
        assert_eq!(unsafe { *milliseconds.payload.ok }, 0);

        let second = Duration {
            years: 0,
            months: 0,
            weeks: 0,
            days: 0,
            hours: 0,
            minutes: 0,
            seconds: 1,
            milliseconds: 0,
            microseconds: 0,
            nanoseconds: 0,
        };
        let mut later = std::mem::MaybeUninit::<TemporalResultFfi<Instant>>::uninit();
        unsafe {
            super::tyrus_temporal_instant_add_duration_result(&instant, &second, later.as_mut_ptr())
        };
        let later = unsafe { later.assume_init() };
        assert_eq!(later.tag, 0);
        assert_eq!(unsafe { (*later.payload.ok).seconds }, 1);

        let calendar_duration = Duration { years: 1, ..second };
        let mut invalid = std::mem::MaybeUninit::<TemporalResultFfi<Instant>>::uninit();
        unsafe {
            super::tyrus_temporal_instant_add_duration_result(
                &instant,
                &calendar_duration,
                invalid.as_mut_ptr(),
            )
        };
        let invalid = unsafe { invalid.assume_init() };
        assert_eq!(invalid.tag, 1);
        let error = unsafe { std::mem::ManuallyDrop::into_inner(invalid.payload.err) };
        assert_eq!(
            unsafe { error.code.as_str() },
            "ERR_TEMPORAL_INSTANT_CALENDAR_DURATION"
        );
        unsafe {
            error.code.release();
            error.message.release();
        }
    }

    #[test]
    fn civil_parsers_return_err_values_for_invalid_input() {
        let mut date = std::mem::MaybeUninit::<TemporalResultFfi<super::PlainDate>>::uninit();
        unsafe {
            super::tyrus_temporal_plain_date_from_result(
                TyrusString::literal("not-a-date"),
                date.as_mut_ptr(),
            )
        };
        unsafe { assert_temporal_result_error(date.assume_init(), "ERR_TEMPORAL_INVALID_DATE") };

        let mut time = std::mem::MaybeUninit::<TemporalResultFfi<super::PlainTime>>::uninit();
        unsafe {
            super::tyrus_temporal_plain_time_from_result(
                TyrusString::literal("not-a-time"),
                time.as_mut_ptr(),
            )
        };
        unsafe { assert_temporal_result_error(time.assume_init(), "ERR_TEMPORAL_INVALID_TIME") };

        let mut date_time =
            std::mem::MaybeUninit::<TemporalResultFfi<super::PlainDateTime>>::uninit();
        unsafe {
            super::tyrus_temporal_plain_date_time_from_result(
                TyrusString::literal("not-a-date-time"),
                date_time.as_mut_ptr(),
            )
        };
        unsafe {
            assert_temporal_result_error(date_time.assume_init(), "ERR_TEMPORAL_INVALID_DATE_TIME")
        };

        let mut year_month =
            std::mem::MaybeUninit::<TemporalResultFfi<super::PlainYearMonth>>::uninit();
        unsafe {
            super::tyrus_temporal_plain_year_month_from_result(
                TyrusString::literal("not-a-month"),
                year_month.as_mut_ptr(),
            )
        };
        unsafe {
            assert_temporal_result_error(
                year_month.assume_init(),
                "ERR_TEMPORAL_INVALID_YEAR_MONTH",
            )
        };

        let mut month_day =
            std::mem::MaybeUninit::<TemporalResultFfi<super::PlainMonthDay>>::uninit();
        unsafe {
            super::tyrus_temporal_plain_month_day_from_result(
                TyrusString::literal("not-a-day"),
                month_day.as_mut_ptr(),
            )
        };
        unsafe {
            assert_temporal_result_error(month_day.assume_init(), "ERR_TEMPORAL_INVALID_MONTH_DAY")
        };
    }

    #[test]
    fn exposes_paris_from_the_shared_catalog() {
        let mut ordinal = 0..tyrus_temporal_timezone_count();
        let paris = ordinal.find(|ordinal| {
            let pointer = tyrus_temporal_timezone_name(*ordinal);
            if pointer.is_null() {
                return false;
            }
            let name = unsafe { pointer.as_str() }.to_owned();
            unsafe { pointer.release() };
            name == "Europe/Paris"
        });

        assert!(paris.is_some());
    }

    #[test]
    fn rejects_unknown_timezone_with_a_stable_error() {
        let name = TyrusString::literal("Europe/NotARealZone");
        assert_eq!(super::tyrus_temporal_timezone_from(name.borrow()), 0);

        let mut taken = super::TemporalErrorTaken {
            code: TyrusString::null(),
            message: TyrusString::null(),
        };
        unsafe { super::tyrus_temporal_take_error(&mut taken) };
        assert_eq!(
            unsafe { taken.code.as_str() },
            "ERR_TEMPORAL_INVALID_TIME_ZONE"
        );
        assert_eq!(unsafe { taken.message.as_str() }, "unknown IANA time zone");
    }

    #[test]
    fn round_trips_epoch_milliseconds_without_losing_the_millisecond() {
        let instant = tyrus_temporal_instant_from_epoch_milliseconds(-1);
        assert_eq!(tyrus_temporal_instant_to_epoch_milliseconds(instant), -1);
    }

    #[test]
    fn formats_an_instant_as_utc_iso_text() {
        let instant = tyrus_temporal_instant_from_epoch_milliseconds(0);
        let pointer = tyrus_temporal_instant_to_string(instant);
        let text = unsafe { pointer.as_str() };
        assert_eq!(text, "1970-01-01T00:00:00Z");
        unsafe { pointer.release() };
    }

    #[test]
    fn parses_an_iso_instant() {
        let input = TyrusString::literal("2026-07-09T15:30:00Z");
        let instant = super::tyrus_temporal_instant_from(input.borrow());
        assert_eq!(instant.seconds, 1_783_611_000);
        assert_eq!(instant.nanoseconds, 0);
    }

    #[test]
    fn parses_and_formats_a_zoned_date_time() {
        let input = TyrusString::literal("2026-07-09T14:30:00-03:00[America/Sao_Paulo]");
        let zoned = tyrus_temporal_zoned_date_time_from(input.borrow());
        let instant = tyrus_temporal_zoned_date_time_to_instant(zoned);
        assert_eq!(instant.seconds, 1_783_618_200);
        let pointer = tyrus_temporal_zoned_date_time_to_string(zoned);
        let text = unsafe { pointer.as_str() };
        assert_eq!(text, "2026-07-09T14:30:00-03:00[America/Sao_Paulo]");
        unsafe { pointer.release() };
        let converted = tyrus_temporal_instant_to_zoned_date_time(instant, zoned.time_zone);
        assert!(tyrus_temporal_zoned_date_time_equals(zoned, converted));
        let duration = Duration {
            years: 0,
            months: 0,
            weeks: 0,
            days: 0,
            hours: 1,
            minutes: 0,
            seconds: 0,
            milliseconds: 0,
            microseconds: 0,
            nanoseconds: 0,
        };
        let later = tyrus_temporal_zoned_date_time_add_duration(zoned, duration);
        assert_eq!(tyrus_temporal_zoned_date_time_until(zoned, later).hours, 1);
        let formatted =
            tyrus_temporal_zoned_date_time_format(later, TyrusString::literal("yyyy-MM-dd HH:mm"));
        assert_eq!(unsafe { formatted.as_str() }, "2026-07-09 15:30");
        unsafe { formatted.release() };
    }

    #[test]
    fn parses_and_formats_plain_dates() {
        let input = TyrusString::literal("2026-07-09");
        let date = tyrus_temporal_plain_date_from(input.borrow());
        assert_eq!((date.year, date.month, date.day), (2026, 7, 9));
        let pointer = tyrus_temporal_plain_date_to_string(date);
        let text = unsafe { pointer.as_str() };
        assert_eq!(text, "2026-07-09");
        unsafe { pointer.release() };
        assert!(tyrus_temporal_plain_date_equals(date, date));
        let next = tyrus_temporal_plain_date_add_days(date, 1);
        assert_eq!((next.year, next.month, next.day), (2026, 7, 10));
        assert_eq!(tyrus_temporal_plain_date_day_of_week(date), 4);
        assert_eq!(tyrus_temporal_plain_date_day_of_year(date), 190);
        let month_end = tyrus_temporal_plain_date_add_months(date, 1);
        assert_eq!(
            (month_end.year, month_end.month, month_end.day),
            (2026, 8, 9)
        );
        let january_end = TyrusString::literal("2026-01-31");
        let january_end = tyrus_temporal_plain_date_from(january_end.borrow());
        let february = tyrus_temporal_plain_date_add_months(january_end, 1);
        assert_eq!((february.year, february.month, february.day), (2026, 2, 28));
        let leap_day = tyrus_temporal_plain_date_add_years(date, -2);
        assert_eq!((leap_day.year, leap_day.month, leap_day.day), (2024, 7, 9));
        assert_eq!(tyrus_temporal_plain_date_days_until(date, next), 1);
        assert_eq!(tyrus_temporal_plain_date_days_until(next, date), -1);
        assert_eq!(tyrus_temporal_plain_date_from_fields(date), date);
        assert_eq!(tyrus_temporal_plain_date_with(date, next), next);
        let one_day = Duration {
            years: 0,
            months: 0,
            weeks: 0,
            days: 1,
            hours: 0,
            minutes: 0,
            seconds: 0,
            milliseconds: 0,
            microseconds: 0,
            nanoseconds: 0,
        };
        assert_eq!(
            tyrus_temporal_plain_date_subtract_duration(next, one_day),
            date
        );
        assert_eq!(tyrus_temporal_plain_date_until(date, next).days, 1);
        assert_eq!(tyrus_temporal_plain_date_since(next, date).days, 1);
        let input = TyrusString::literal("14:30:15.500");
        let time = tyrus_temporal_plain_time_from(input.borrow());
        assert_eq!(
            (time.hour, time.minute, time.second, time.nanosecond),
            (14, 30, 15, 500_000_000)
        );
        let pointer = tyrus_temporal_plain_time_to_string(time);
        let text = unsafe { pointer.as_str() };
        assert_eq!(text, "14:30:15.5");
        unsafe { pointer.release() };
        assert!(tyrus_temporal_plain_time_equals(time, time));
        let wrapped = tyrus_temporal_plain_time_add_seconds(time, 36_000);
        assert_eq!((wrapped.hour, wrapped.minute, wrapped.second), (0, 30, 15));
        let earlier = tyrus_temporal_plain_time_add_seconds(time, -60);
        assert_eq!(tyrus_temporal_plain_time_seconds_until(earlier, time), 60);
        let shifted_time = tyrus_temporal_plain_time_add_nanoseconds(time, 750_000_000);
        assert_eq!(
            (shifted_time.second, shifted_time.nanosecond),
            (16, 250_000_000)
        );
        assert_eq!(
            tyrus_temporal_plain_time_nanoseconds_until(time, shifted_time),
            750_000_000
        );
        assert_eq!(tyrus_temporal_plain_time_from_fields(time), time);
        assert_eq!(
            tyrus_temporal_plain_time_subtract_duration(shifted_time, one_day),
            shifted_time
        );
        assert_eq!(
            tyrus_temporal_plain_time_until(time, shifted_time).nanoseconds,
            750_000_000
        );
        assert_eq!(
            tyrus_temporal_plain_time_since(shifted_time, time).nanoseconds,
            750_000_000
        );
        let duration_fields = Duration {
            years: 0,
            months: 0,
            weeks: 0,
            days: 0,
            hours: 1,
            minutes: 30,
            seconds: 0,
            milliseconds: 0,
            microseconds: 0,
            nanoseconds: 0,
        };
        let mut duration = duration_fields;
        unsafe { tyrus_temporal_duration_from_fields(&duration_fields, &mut duration) };
        let unit = TyrusString::literal("minutes");
        assert_eq!(
            unsafe { tyrus_temporal_duration_total(&duration, unit.borrow()) },
            90
        );
        let date_with_duration = tyrus_temporal_plain_date_add_duration(date, one_day);
        assert_eq!(
            (
                date_with_duration.year,
                date_with_duration.month,
                date_with_duration.day
            ),
            (2026, 7, 10)
        );
        let time_with_duration = tyrus_temporal_plain_time_add_duration(time, duration);
        assert_eq!(
            (time_with_duration.hour, time_with_duration.minute),
            (16, 0)
        );
    }

    #[test]
    fn supports_advanced_difference_options_and_fixed_instant_arithmetic() {
        let start_text = TyrusString::literal("2026-01-01");
        let end_text = TyrusString::literal("2026-03-01");
        let start = tyrus_temporal_plain_date_from(start_text.borrow());
        let end = tyrus_temporal_plain_date_from(end_text.borrow());
        let largest = TyrusString::literal("months");
        let smallest = TyrusString::literal("months");
        let mode = TyrusString::literal("trunc");
        let difference = super::tyrus_temporal_plain_date_until_with_options(
            start,
            end,
            largest.borrow(),
            smallest.borrow(),
            1,
            mode.borrow(),
        );
        assert_eq!(difference.months, 2);

        let instant = tyrus_temporal_instant_from_epoch_milliseconds(0);
        let one_second = Duration {
            years: 0,
            months: 0,
            weeks: 0,
            days: 0,
            hours: 0,
            minutes: 0,
            seconds: 1,
            milliseconds: 0,
            microseconds: 0,
            nanoseconds: 0,
        };
        let later = super::tyrus_temporal_instant_add_duration(instant, one_second);
        assert_eq!(tyrus_temporal_instant_to_epoch_milliseconds(later), 1000);
        assert_eq!(
            super::tyrus_temporal_instant_subtract_duration(later, one_second),
            instant
        );
    }

    #[test]
    fn supports_composite_and_partial_calendar_values() {
        let date_time_text = TyrusString::literal("2026-07-09T14:30:15.500");
        let mut date_time = super::invalid_plain_date_time();
        unsafe {
            super::tyrus_temporal_plain_date_time_from(date_time_text.borrow(), &mut date_time);
        };
        assert_eq!(
            date_time.date,
            super::PlainDate {
                year: 2026,
                month: 7,
                day: 9
            }
        );
        assert_eq!(
            date_time.time,
            super::PlainTime {
                hour: 14,
                minute: 30,
                second: 15,
                nanosecond: 500_000_000
            }
        );
        let text = unsafe { super::tyrus_temporal_plain_date_time_to_string(&date_time) };
        assert_eq!(unsafe { text.as_str() }, "2026-07-09T14:30:15.5");
        unsafe { text.release() };
        let duration = super::Duration {
            years: 0,
            months: 0,
            weeks: 0,
            days: 1,
            hours: 1,
            minutes: 0,
            seconds: 0,
            milliseconds: 0,
            microseconds: 0,
            nanoseconds: 0,
        };
        let mut later = super::invalid_plain_date_time();
        unsafe {
            super::tyrus_temporal_plain_date_time_add_duration(&date_time, &duration, &mut later);
        };
        assert_eq!((later.date.day, later.time.hour), (10, 15));
        let mut difference = super::invalid_duration();
        unsafe { super::tyrus_temporal_plain_date_time_until(&date_time, &later, &mut difference) };
        assert_eq!(difference.days, 1);

        let year_month_text = TyrusString::literal("2026-07");
        let mut year_month = super::PlainYearMonth { year: 0, month: 1 };
        unsafe {
            super::tyrus_temporal_plain_year_month_from(year_month_text.borrow(), &mut year_month);
        };
        let mut next_month = year_month;
        unsafe { super::tyrus_temporal_plain_year_month_add(&year_month, 6, &mut next_month) };
        assert_eq!(
            next_month,
            super::PlainYearMonth {
                year: 2027,
                month: 1
            }
        );
        let month_day_text = TyrusString::literal("02-29");
        let mut month_day = super::PlainMonthDay { month: 1, day: 1 };
        unsafe {
            super::tyrus_temporal_plain_month_day_from(month_day_text.borrow(), &mut month_day);
        };
        assert_eq!(
            super::tyrus_temporal_plain_month_day_to_plain_date(month_day, 2028),
            super::PlainDate {
                year: 2028,
                month: 2,
                day: 29
            }
        );
    }

    #[test]
    fn rounds_duration_with_a_calendar_anchor() {
        let duration = super::Duration {
            years: 0,
            months: 0,
            weeks: 0,
            days: 0,
            hours: 1,
            minutes: 31,
            seconds: 0,
            milliseconds: 0,
            microseconds: 0,
            nanoseconds: 0,
        };
        let unit = TyrusString::literal("hours");
        let mode = TyrusString::literal("halfExpand");
        let date = super::PlainDate {
            year: 2026,
            month: 7,
            day: 9,
        };
        let rounded =
            super::tyrus_temporal_duration_round(duration, unit.borrow(), 1, mode.borrow(), date);
        assert_eq!((rounded.hours, rounded.minutes), (2, 0));
    }

    #[test]
    fn rejects_truncating_fields_and_extreme_subtraction_without_panicking() {
        fn take_error_code() -> String {
            let mut error = super::TemporalErrorTaken {
                code: TyrusString::null(),
                message: TyrusString::null(),
            };
            unsafe { super::tyrus_temporal_take_error(&mut error) };
            assert!(!error.code.is_null());
            unsafe { error.code.as_str() }.to_string()
        }

        let invalid_date = super::PlainDate {
            year: 65_536,
            month: 1,
            day: 1,
        };
        super::tyrus_temporal_plain_date_from_fields(invalid_date);
        assert_eq!(take_error_code(), "ERR_TEMPORAL_INVALID_DATE");
        assert_eq!(
            super::tyrus_temporal_plain_date_day_of_week(invalid_date),
            0
        );
        assert_eq!(take_error_code(), "ERR_TEMPORAL_INVALID_DATE");

        let invalid_time = super::PlainTime {
            hour: 256,
            minute: 0,
            second: 0,
            nanosecond: 0,
        };
        super::tyrus_temporal_plain_time_from_fields(invalid_time);
        assert_eq!(take_error_code(), "ERR_TEMPORAL_INVALID_TIME");

        let invalid_year_month = super::PlainYearMonth {
            year: 2026,
            month: 13,
        };
        let mut invalid_result = invalid_year_month;
        unsafe {
            super::tyrus_temporal_plain_year_month_add(&invalid_year_month, 0, &mut invalid_result);
        };
        assert_eq!(invalid_result, invalid_year_month);
        assert_eq!(take_error_code(), "ERR_TEMPORAL_INVALID_YEAR_MONTH");

        let invalid_instant = super::Instant {
            seconds: 0,
            nanoseconds: 1_000_000_000,
        };
        super::tyrus_temporal_instant_to_zoned_date_time(invalid_instant, 0);
        assert_eq!(take_error_code(), "ERR_TEMPORAL_INVALID_INSTANT");

        let minimum_duration = super::Duration {
            years: i64::MIN,
            months: 0,
            weeks: 0,
            days: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
            milliseconds: 0,
            microseconds: 0,
            nanoseconds: 0,
        };
        let date = super::PlainDate {
            year: 2026,
            month: 8,
            day: 7,
        };
        assert_eq!(
            super::tyrus_temporal_plain_date_subtract_duration(date, minimum_duration),
            date
        );
        assert_eq!(take_error_code(), "ERR_TEMPORAL_DURATION_OVERFLOW");

        assert_eq!(
            super::tyrus_temporal_plain_date_add_days(date, i64::MAX),
            date
        );
        assert_eq!(take_error_code(), "ERR_TEMPORAL_DATE_OVERFLOW");

        let maximum_duration = super::Duration {
            years: i64::MAX,
            months: 0,
            weeks: 0,
            days: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
            milliseconds: 0,
            microseconds: 0,
            nanoseconds: 0,
        };
        assert_eq!(
            super::tyrus_temporal_plain_date_add_duration(date, maximum_duration),
            date
        );
        assert_eq!(take_error_code(), "ERR_TEMPORAL_DURATION_OVERFLOW");

        let instant = super::Instant {
            seconds: 0,
            nanoseconds: 0,
        };
        assert_eq!(
            super::tyrus_temporal_instant_add_duration(instant, maximum_duration),
            instant
        );
        assert_eq!(take_error_code(), "ERR_TEMPORAL_INSTANT_CALENDAR_DURATION");

        let clock_overflow = super::Duration {
            years: 0,
            hours: i64::MAX,
            ..maximum_duration
        };
        assert_eq!(
            super::tyrus_temporal_instant_add_duration(instant, clock_overflow),
            instant
        );
        assert_eq!(take_error_code(), "ERR_TEMPORAL_DURATION_OVERFLOW");
    }

    #[test]
    fn rejects_a_zoned_date_time_when_its_offset_disagrees_with_the_zone() {
        let input = TyrusString::literal("2026-08-07T14:30:00+09:00[America/Sao_Paulo]");
        let result = super::tyrus_temporal_zoned_date_time_from(input.borrow());
        assert_eq!(result.seconds, 0);

        let mut error = super::TemporalErrorTaken {
            code: TyrusString::null(),
            message: TyrusString::null(),
        };
        unsafe { super::tyrus_temporal_take_error(&mut error) };
        assert_eq!(
            unsafe { error.code.as_str() },
            "ERR_TEMPORAL_INVALID_ZONED_DATE_TIME"
        );
    }
}

macro_rules! temporal_ffi_from_text {
    ($wrapper:ident, $symbol:literal, $ty:ty, $function:ident, $code:expr_2021, $label:expr_2021) => {
        #[unsafe(export_name = $symbol)]
        pub unsafe extern "C" fn $wrapper(input: TyrusString, out: *mut $ty) {
            unsafe {
                write_ffi_value(out, $function(input), $code, $label);
            }
        }
    };
}

macro_rules! temporal_ffi_to_string {
    ($wrapper:ident, $symbol:literal, $ty:ty, $function:ident, $code:expr_2021, $input:expr_2021) => {
        #[unsafe(export_name = $symbol)]
        pub unsafe extern "C" fn $wrapper(value: *const $ty) -> TyrusString {
            unsafe {
                let Some(value) = read_ffi_value(value, $code, $input) else {
                    return TyrusString::null();
                };
                $function(value)
            }
        }
    };
}

macro_rules! temporal_ffi_equals {
    ($wrapper:ident, $symbol:literal, $ty:ty, $function:ident, $code:expr_2021, $input:expr_2021) => {
        #[unsafe(export_name = $symbol)]
        pub unsafe extern "C" fn $wrapper(left: *const $ty, right: *const $ty) -> bool {
            unsafe {
                let (Some(left), Some(right)) = (
                    read_ffi_value(left, $code, $input),
                    read_ffi_value(right, $code, $input),
                ) else {
                    return false;
                };
                $function(left, right)
            }
        }
    };
}

macro_rules! temporal_ffi_amount_output {
    ($wrapper:ident, $symbol:literal, $ty:ty, $function:ident, $code:expr_2021, $input:expr_2021, $output:expr_2021) => {
        #[unsafe(export_name = $symbol)]
        pub unsafe extern "C" fn $wrapper(value: *const $ty, amount: i64, out: *mut $ty) {
            unsafe {
                let Some(value) = read_ffi_value(value, $code, $input) else {
                    return;
                };
                write_ffi_value(out, $function(value, amount), $code, $output);
            }
        }
    };
}

macro_rules! temporal_ffi_duration_output {
    ($wrapper:ident, $symbol:literal, $ty:ty, $function:ident, $code:expr_2021, $input:expr_2021, $output:expr_2021) => {
        #[unsafe(export_name = $symbol)]
        pub unsafe extern "C" fn $wrapper(
            value: *const $ty,
            duration: *const Duration,
            out: *mut $ty,
        ) {
            unsafe {
                let Some(value) = read_ffi_value(value, $code, $input) else {
                    return;
                };
                let Some(duration) = read_ffi_value(
                    duration,
                    c"ERR_TEMPORAL_INVALID_DURATION",
                    c"duration pointer must not be null",
                ) else {
                    return;
                };
                write_ffi_value(out, $function(value, duration), $code, $output);
            }
        }
    };
}

macro_rules! temporal_ffi_binary_duration {
    ($wrapper:ident, $symbol:literal, $ty:ty, $function:ident, $code:expr_2021, $input:expr_2021) => {
        #[unsafe(export_name = $symbol)]
        pub unsafe extern "C" fn $wrapper(start: *const $ty, end: *const $ty, out: *mut Duration) {
            unsafe {
                let (Some(start), Some(end)) = (
                    read_ffi_value(start, $code, $input),
                    read_ffi_value(end, $code, $input),
                ) else {
                    return;
                };
                write_ffi_value(
                    out,
                    $function(start, end),
                    c"ERR_TEMPORAL_INVALID_DURATION",
                    c"duration output pointer must not be null",
                );
            }
        }
    };
}

temporal_ffi_from_text!(
    tyrus_temporal_plain_date_from_ffi,
    "tyrus_temporal_plain_date_from",
    PlainDate,
    tyrus_temporal_plain_date_from,
    c"ERR_TEMPORAL_INVALID_DATE",
    c"date output pointer must not be null"
);
temporal_ffi_to_string!(
    tyrus_temporal_plain_date_to_string_ffi,
    "tyrus_temporal_plain_date_to_string",
    PlainDate,
    tyrus_temporal_plain_date_to_string,
    c"ERR_TEMPORAL_INVALID_DATE",
    c"date pointer must not be null"
);
temporal_ffi_equals!(
    tyrus_temporal_plain_date_equals_ffi,
    "tyrus_temporal_plain_date_equals",
    PlainDate,
    tyrus_temporal_plain_date_equals,
    c"ERR_TEMPORAL_INVALID_DATE",
    c"date pointer must not be null"
);
temporal_ffi_amount_output!(
    tyrus_temporal_plain_date_add_days_ffi,
    "tyrus_temporal_plain_date_add_days",
    PlainDate,
    tyrus_temporal_plain_date_add_days,
    c"ERR_TEMPORAL_INVALID_DATE",
    c"date pointer must not be null",
    c"date output pointer must not be null"
);
temporal_ffi_amount_output!(
    tyrus_temporal_plain_date_add_months_ffi,
    "tyrus_temporal_plain_date_add_months",
    PlainDate,
    tyrus_temporal_plain_date_add_months,
    c"ERR_TEMPORAL_INVALID_DATE",
    c"date pointer must not be null",
    c"date output pointer must not be null"
);
temporal_ffi_amount_output!(
    tyrus_temporal_plain_date_add_years_ffi,
    "tyrus_temporal_plain_date_add_years",
    PlainDate,
    tyrus_temporal_plain_date_add_years,
    c"ERR_TEMPORAL_INVALID_DATE",
    c"date pointer must not be null",
    c"date output pointer must not be null"
);
temporal_ffi_duration_output!(
    tyrus_temporal_plain_date_add_duration_ffi,
    "tyrus_temporal_plain_date_add_duration",
    PlainDate,
    tyrus_temporal_plain_date_add_duration,
    c"ERR_TEMPORAL_INVALID_DATE",
    c"date pointer must not be null",
    c"date output pointer must not be null"
);
temporal_ffi_duration_output!(
    tyrus_temporal_plain_date_subtract_duration_ffi,
    "tyrus_temporal_plain_date_subtract_duration",
    PlainDate,
    tyrus_temporal_plain_date_subtract_duration,
    c"ERR_TEMPORAL_INVALID_DATE",
    c"date pointer must not be null",
    c"date output pointer must not be null"
);
temporal_ffi_binary_duration!(
    tyrus_temporal_plain_date_until_ffi,
    "tyrus_temporal_plain_date_until",
    PlainDate,
    tyrus_temporal_plain_date_until,
    c"ERR_TEMPORAL_INVALID_DATE",
    c"date pointer must not be null"
);
temporal_ffi_binary_duration!(
    tyrus_temporal_plain_date_since_ffi,
    "tyrus_temporal_plain_date_since",
    PlainDate,
    tyrus_temporal_plain_date_since,
    c"ERR_TEMPORAL_INVALID_DATE",
    c"date pointer must not be null"
);

temporal_ffi_from_text!(
    tyrus_temporal_plain_time_from_ffi,
    "tyrus_temporal_plain_time_from",
    PlainTime,
    tyrus_temporal_plain_time_from,
    c"ERR_TEMPORAL_INVALID_TIME",
    c"time output pointer must not be null"
);
temporal_ffi_to_string!(
    tyrus_temporal_plain_time_to_string_ffi,
    "tyrus_temporal_plain_time_to_string",
    PlainTime,
    tyrus_temporal_plain_time_to_string,
    c"ERR_TEMPORAL_INVALID_TIME",
    c"time pointer must not be null"
);
temporal_ffi_equals!(
    tyrus_temporal_plain_time_equals_ffi,
    "tyrus_temporal_plain_time_equals",
    PlainTime,
    tyrus_temporal_plain_time_equals,
    c"ERR_TEMPORAL_INVALID_TIME",
    c"time pointer must not be null"
);
temporal_ffi_amount_output!(
    tyrus_temporal_plain_time_add_seconds_ffi,
    "tyrus_temporal_plain_time_add_seconds",
    PlainTime,
    tyrus_temporal_plain_time_add_seconds,
    c"ERR_TEMPORAL_INVALID_TIME",
    c"time pointer must not be null",
    c"time output pointer must not be null"
);
temporal_ffi_amount_output!(
    tyrus_temporal_plain_time_add_nanoseconds_ffi,
    "tyrus_temporal_plain_time_add_nanoseconds",
    PlainTime,
    tyrus_temporal_plain_time_add_nanoseconds,
    c"ERR_TEMPORAL_INVALID_TIME",
    c"time pointer must not be null",
    c"time output pointer must not be null"
);
temporal_ffi_duration_output!(
    tyrus_temporal_plain_time_add_duration_ffi,
    "tyrus_temporal_plain_time_add_duration",
    PlainTime,
    tyrus_temporal_plain_time_add_duration,
    c"ERR_TEMPORAL_INVALID_TIME",
    c"time pointer must not be null",
    c"time output pointer must not be null"
);
temporal_ffi_duration_output!(
    tyrus_temporal_plain_time_subtract_duration_ffi,
    "tyrus_temporal_plain_time_subtract_duration",
    PlainTime,
    tyrus_temporal_plain_time_subtract_duration,
    c"ERR_TEMPORAL_INVALID_TIME",
    c"time pointer must not be null",
    c"time output pointer must not be null"
);
temporal_ffi_binary_duration!(
    tyrus_temporal_plain_time_until_ffi,
    "tyrus_temporal_plain_time_until",
    PlainTime,
    tyrus_temporal_plain_time_until,
    c"ERR_TEMPORAL_INVALID_TIME",
    c"time pointer must not be null"
);
temporal_ffi_binary_duration!(
    tyrus_temporal_plain_time_since_ffi,
    "tyrus_temporal_plain_time_since",
    PlainTime,
    tyrus_temporal_plain_time_since,
    c"ERR_TEMPORAL_INVALID_TIME",
    c"time pointer must not be null"
);

#[unsafe(export_name = "tyrus_temporal_plain_date_day_of_week")]
pub unsafe extern "C" fn tyrus_temporal_plain_date_day_of_week_ffi(value: *const PlainDate) -> u32 {
    unsafe {
        let Some(value) = read_ffi_value(
            value,
            c"ERR_TEMPORAL_INVALID_DATE",
            c"date pointer must not be null",
        ) else {
            return 0;
        };
        tyrus_temporal_plain_date_day_of_week(value)
    }
}

#[unsafe(export_name = "tyrus_temporal_plain_date_day_of_year")]
pub unsafe extern "C" fn tyrus_temporal_plain_date_day_of_year_ffi(value: *const PlainDate) -> u32 {
    unsafe {
        let Some(value) = read_ffi_value(
            value,
            c"ERR_TEMPORAL_INVALID_DATE",
            c"date pointer must not be null",
        ) else {
            return 0;
        };
        tyrus_temporal_plain_date_day_of_year(value)
    }
}

#[unsafe(export_name = "tyrus_temporal_plain_date_days_until")]
pub unsafe extern "C" fn tyrus_temporal_plain_date_days_until_ffi(
    start: *const PlainDate,
    end: *const PlainDate,
) -> i64 {
    unsafe {
        let (Some(start), Some(end)) = (
            read_ffi_value(
                start,
                c"ERR_TEMPORAL_INVALID_DATE",
                c"date pointer must not be null",
            ),
            read_ffi_value(
                end,
                c"ERR_TEMPORAL_INVALID_DATE",
                c"date pointer must not be null",
            ),
        ) else {
            return 0;
        };
        tyrus_temporal_plain_date_days_until(start, end)
    }
}

#[unsafe(export_name = "tyrus_temporal_plain_time_seconds_until")]
pub unsafe extern "C" fn tyrus_temporal_plain_time_seconds_until_ffi(
    start: *const PlainTime,
    end: *const PlainTime,
) -> i64 {
    unsafe {
        let (Some(start), Some(end)) = (
            read_ffi_value(
                start,
                c"ERR_TEMPORAL_INVALID_TIME",
                c"time pointer must not be null",
            ),
            read_ffi_value(
                end,
                c"ERR_TEMPORAL_INVALID_TIME",
                c"time pointer must not be null",
            ),
        ) else {
            return 0;
        };
        tyrus_temporal_plain_time_seconds_until(start, end)
    }
}

#[unsafe(export_name = "tyrus_temporal_plain_time_nanoseconds_until")]
pub unsafe extern "C" fn tyrus_temporal_plain_time_nanoseconds_until_ffi(
    start: *const PlainTime,
    end: *const PlainTime,
) -> i64 {
    unsafe {
        let (Some(start), Some(end)) = (
            read_ffi_value(
                start,
                c"ERR_TEMPORAL_INVALID_TIME",
                c"time pointer must not be null",
            ),
            read_ffi_value(
                end,
                c"ERR_TEMPORAL_INVALID_TIME",
                c"time pointer must not be null",
            ),
        ) else {
            return 0;
        };
        tyrus_temporal_plain_time_nanoseconds_until(start, end)
    }
}

#[unsafe(export_name = "tyrus_temporal_plain_date_from_fields")]
pub unsafe extern "C" fn tyrus_temporal_plain_date_from_fields_ffi(
    value: *const PlainDate,
    out: *mut PlainDate,
) {
    unsafe {
        let Some(value) = read_ffi_value(
            value,
            c"ERR_TEMPORAL_INVALID_DATE",
            c"date pointer must not be null",
        ) else {
            return;
        };
        write_ffi_value(
            out,
            tyrus_temporal_plain_date_from_fields(value),
            c"ERR_TEMPORAL_INVALID_DATE",
            c"date output pointer must not be null",
        );
    }
}

#[unsafe(export_name = "tyrus_temporal_plain_time_from_fields")]
pub unsafe extern "C" fn tyrus_temporal_plain_time_from_fields_ffi(
    value: *const PlainTime,
    out: *mut PlainTime,
) {
    unsafe {
        let Some(value) = read_ffi_value(
            value,
            c"ERR_TEMPORAL_INVALID_TIME",
            c"time pointer must not be null",
        ) else {
            return;
        };
        write_ffi_value(
            out,
            tyrus_temporal_plain_time_from_fields(value),
            c"ERR_TEMPORAL_INVALID_TIME",
            c"time output pointer must not be null",
        );
    }
}

#[unsafe(export_name = "tyrus_temporal_plain_date_with")]
pub unsafe extern "C" fn tyrus_temporal_plain_date_with_ffi(
    value: *const PlainDate,
    replacement: *const PlainDate,
    out: *mut PlainDate,
) {
    unsafe {
        let (Some(value), Some(replacement)) = (
            read_ffi_value(
                value,
                c"ERR_TEMPORAL_INVALID_DATE",
                c"date pointer must not be null",
            ),
            read_ffi_value(
                replacement,
                c"ERR_TEMPORAL_INVALID_DATE",
                c"date pointer must not be null",
            ),
        ) else {
            return;
        };
        write_ffi_value(
            out,
            tyrus_temporal_plain_date_with(value, replacement),
            c"ERR_TEMPORAL_INVALID_DATE",
            c"date output pointer must not be null",
        );
    }
}

#[unsafe(export_name = "tyrus_temporal_plain_date_until_with_options")]
pub unsafe extern "C" fn tyrus_temporal_plain_date_until_with_options_ffi(
    start: *const PlainDate,
    end: *const PlainDate,
    largest: TyrusString,
    smallest: TyrusString,
    increment: i64,
    mode: TyrusString,
    out: *mut Duration,
) {
    unsafe {
        let (Some(start), Some(end)) = (
            read_ffi_value(
                start,
                c"ERR_TEMPORAL_INVALID_DATE",
                c"date pointer must not be null",
            ),
            read_ffi_value(
                end,
                c"ERR_TEMPORAL_INVALID_DATE",
                c"date pointer must not be null",
            ),
        ) else {
            return;
        };
        write_ffi_value(
            out,
            tyrus_temporal_plain_date_until_with_options(
                start, end, largest, smallest, increment, mode,
            ),
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration output pointer must not be null",
        );
    }
}

#[unsafe(export_name = "tyrus_temporal_plain_date_since_with_options")]
pub unsafe extern "C" fn tyrus_temporal_plain_date_since_with_options_ffi(
    start: *const PlainDate,
    end: *const PlainDate,
    largest: TyrusString,
    smallest: TyrusString,
    increment: i64,
    mode: TyrusString,
    out: *mut Duration,
) {
    unsafe {
        let (Some(start), Some(end)) = (
            read_ffi_value(
                start,
                c"ERR_TEMPORAL_INVALID_DATE",
                c"date pointer must not be null",
            ),
            read_ffi_value(
                end,
                c"ERR_TEMPORAL_INVALID_DATE",
                c"date pointer must not be null",
            ),
        ) else {
            return;
        };
        write_ffi_value(
            out,
            tyrus_temporal_plain_date_since_with_options(
                start, end, largest, smallest, increment, mode,
            ),
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration output pointer must not be null",
        );
    }
}

#[unsafe(export_name = "tyrus_temporal_plain_time_until_with_options")]
pub unsafe extern "C" fn tyrus_temporal_plain_time_until_with_options_ffi(
    start: *const PlainTime,
    end: *const PlainTime,
    largest: TyrusString,
    smallest: TyrusString,
    increment: i64,
    mode: TyrusString,
    out: *mut Duration,
) {
    unsafe {
        let (Some(start), Some(end)) = (
            read_ffi_value(
                start,
                c"ERR_TEMPORAL_INVALID_TIME",
                c"time pointer must not be null",
            ),
            read_ffi_value(
                end,
                c"ERR_TEMPORAL_INVALID_TIME",
                c"time pointer must not be null",
            ),
        ) else {
            return;
        };
        write_ffi_value(
            out,
            tyrus_temporal_plain_time_until_with_options(
                start, end, largest, smallest, increment, mode,
            ),
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration output pointer must not be null",
        );
    }
}

#[unsafe(export_name = "tyrus_temporal_plain_time_since_with_options")]
pub unsafe extern "C" fn tyrus_temporal_plain_time_since_with_options_ffi(
    start: *const PlainTime,
    end: *const PlainTime,
    largest: TyrusString,
    smallest: TyrusString,
    increment: i64,
    mode: TyrusString,
    out: *mut Duration,
) {
    unsafe {
        let (Some(start), Some(end)) = (
            read_ffi_value(
                start,
                c"ERR_TEMPORAL_INVALID_TIME",
                c"time pointer must not be null",
            ),
            read_ffi_value(
                end,
                c"ERR_TEMPORAL_INVALID_TIME",
                c"time pointer must not be null",
            ),
        ) else {
            return;
        };
        write_ffi_value(
            out,
            tyrus_temporal_plain_time_since_with_options(
                start, end, largest, smallest, increment, mode,
            ),
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration output pointer must not be null",
        );
    }
}

#[unsafe(export_name = "tyrus_temporal_plain_month_day_to_plain_date")]
pub unsafe extern "C" fn tyrus_temporal_plain_month_day_to_plain_date_ffi(
    value: *const PlainMonthDay,
    year: i32,
    out: *mut PlainDate,
) {
    unsafe {
        let Some(value) = read_ffi_value(
            value,
            c"ERR_TEMPORAL_INVALID_MONTH_DAY",
            c"month-day pointer must not be null",
        ) else {
            return;
        };
        write_ffi_value(
            out,
            tyrus_temporal_plain_month_day_to_plain_date(value, year),
            c"ERR_TEMPORAL_INVALID_DATE",
            c"date output pointer must not be null",
        );
    }
}

#[unsafe(export_name = "tyrus_temporal_duration_round")]
pub unsafe extern "C" fn tyrus_temporal_duration_round_ffi(
    duration: *const Duration,
    unit: TyrusString,
    increment: i64,
    mode: TyrusString,
    relative_to: *const PlainDate,
    out: *mut Duration,
) {
    unsafe {
        let Some(duration) = read_ffi_value(
            duration,
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration pointer must not be null",
        ) else {
            return;
        };
        let Some(relative_to) = read_ffi_value(
            relative_to,
            c"ERR_TEMPORAL_INVALID_DATE",
            c"date pointer must not be null",
        ) else {
            return;
        };
        write_ffi_value(
            out,
            tyrus_temporal_duration_round(duration, unit, increment, mode, relative_to),
            c"ERR_TEMPORAL_INVALID_DURATION",
            c"duration output pointer must not be null",
        );
    }
}
