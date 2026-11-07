//! Native ABI, printing, and error translation for the language `decimal` primitive.

use std::ffi::c_int;
use tyrus_decimal::{Decimal as CoreDecimal, DecimalError};
use tyrus_string::TyrusString;

fn decimal_text(mut coefficient: i128, mut exponent: i32) -> String {
    if coefficient == 0 {
        return "0".to_string();
    }
    while coefficient % 10 == 0 {
        coefficient /= 10;
        exponent = exponent.saturating_add(1);
    }
    let sign = if coefficient < 0 { "-" } else { "" };
    let digits = coefficient.unsigned_abs().to_string();
    if exponent >= 0 {
        return format!("{sign}{digits}{}", "0".repeat(exponent as usize));
    }
    let scale = -(i64::from(exponent));
    if scale >= digits.len() as i64 {
        format!(
            "{sign}0.{}{}",
            "0".repeat((scale as usize).saturating_sub(digits.len())),
            digits
        )
    } else {
        let point = digits.len() - scale as usize;
        format!("{sign}{}.{}", &digits[..point], &digits[point..])
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn write_decimal_bytes(fd: c_int, mut bytes: &[u8]) {
    while !bytes.is_empty() {
        #[cfg(windows)]
        let count = bytes.len().min(u32::MAX as usize) as u32;
        #[cfg(not(windows))]
        let count = bytes.len();
        let written = unsafe { libc::write(fd, bytes.as_ptr().cast(), count) };
        if written <= 0 {
            return;
        }
        bytes = &bytes[written as usize..];
    }
}

#[cfg(target_arch = "wasm32")]
fn write_decimal_bytes(fd: c_int, bytes: &[u8]) {
    unsafe {
        if fd == 2 {
            crate::output::write_stderr(bytes);
        } else {
            crate::output::write_stdout(bytes);
        }
    }
}

fn write_decimal_zeros(fd: c_int, mut count: usize) {
    const ZEROS: [u8; 64] = [b'0'; 64];
    while count != 0 {
        let chunk = count.min(ZEROS.len());
        write_decimal_bytes(fd, &ZEROS[..chunk]);
        count -= chunk;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_decimal_print(mut coefficient: i128, mut exponent: i32, fd: c_int) {
    if coefficient == 0 {
        write_decimal_bytes(fd, b"0");
        return;
    }
    while coefficient % 10 == 0 {
        coefficient /= 10;
        exponent = exponent.saturating_add(1);
    }
    if coefficient < 0 {
        write_decimal_bytes(fd, b"-");
    }
    let mut reversed = [0_u8; 40];
    let mut value = coefficient.unsigned_abs();
    let mut digits = 0;
    while value != 0 {
        reversed[digits] = b'0' + (value % 10) as u8;
        value /= 10;
        digits += 1;
    }
    let mut text = [0_u8; 40];
    for index in 0..digits {
        text[index] = reversed[digits - index - 1];
    }
    if exponent >= 0 {
        write_decimal_bytes(fd, &text[..digits]);
        write_decimal_zeros(fd, exponent as usize);
        return;
    }
    let scale = -(i64::from(exponent));
    if scale >= digits as i64 {
        write_decimal_bytes(fd, b"0.");
        write_decimal_zeros(fd, (scale as usize).saturating_sub(digits));
        write_decimal_bytes(fd, &text[..digits]);
    } else {
        let point = digits - scale as usize;
        write_decimal_bytes(fd, &text[..point]);
        write_decimal_bytes(fd, b".");
        write_decimal_bytes(fd, &text[point..digits]);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_decimal_to_string(coefficient: i128, exponent: i32) -> TyrusString {
    crate::strings::alloc_string(&decimal_text(coefficient, exponent))
}

fn decimal_value_overflow() -> ! {
    unsafe { crate::tyrus_panic(c"decimal result is out of range".as_ptr()) }
}

fn decimal_division_by_zero() -> ! {
    unsafe { crate::tyrus_panic(c"decimal division by zero".as_ptr()) }
}

fn decimal_exponent_must_be_a_non_negative_integer() -> ! {
    unsafe { crate::tyrus_panic(c"decimal exponent must be a non-negative integer".as_ptr()) }
}

fn decimal_has_fractional_component() -> ! {
    unsafe { crate::tyrus_panic(c"decimal value has a fractional component".as_ptr()) }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TyrusDecimal {
    pub coefficient: i128,
    pub exponent: i32,
}

impl From<TyrusDecimal> for CoreDecimal {
    fn from(value: TyrusDecimal) -> Self {
        Self {
            coefficient: value.coefficient,
            exponent: value.exponent,
        }
    }
}

impl From<CoreDecimal> for TyrusDecimal {
    fn from(value: CoreDecimal) -> Self {
        Self {
            coefficient: value.coefficient,
            exponent: value.exponent,
        }
    }
}

impl TyrusDecimal {
    pub(crate) fn parse(text: &str) -> Option<Self> {
        CoreDecimal::parse(text).map(Into::into)
    }

    pub(crate) fn json_number_text(self) -> String {
        let value = CoreDecimal::from(self)
            .normalized()
            .unwrap_or_else(|_| decimal_value_overflow());
        decimal_text(value.coefficient, value.exponent)
    }
}

fn guard_decimal_result(value: Result<CoreDecimal, DecimalError>) -> TyrusDecimal {
    match value {
        Ok(value) => value.into(),
        Err(DecimalError::DivisionByZero) => decimal_division_by_zero(),
        Err(DecimalError::InvalidExponent) => decimal_exponent_must_be_a_non_negative_integer(),
        Err(DecimalError::FractionalComponent) => decimal_has_fractional_component(),
        Err(DecimalError::OutOfRange) => decimal_value_overflow(),
    }
}

unsafe fn decimal_operands<'a>(
    out: *mut TyrusDecimal,
    left: *const TyrusDecimal,
    right: *const TyrusDecimal,
) -> (&'a mut TyrusDecimal, CoreDecimal, CoreDecimal) {
    let (Some(out), Some(left), Some(right)) =
        (unsafe { out.as_mut() }, unsafe { left.as_ref() }, unsafe {
            right.as_ref()
        })
    else {
        decimal_value_overflow();
    };
    (out, (*left).into(), (*right).into())
}

macro_rules! decimal_binary_abi {
    ($name:ident, $operation:ident) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            out: *mut TyrusDecimal,
            left: *const TyrusDecimal,
            right: *const TyrusDecimal,
        ) {
            let (out, left, right) = unsafe { decimal_operands(out, left, right) };
            *out = guard_decimal_result(left.$operation(right));
        }
    };
}

decimal_binary_abi!(tyrus_decimal_add, add);
decimal_binary_abi!(tyrus_decimal_sub, sub);
decimal_binary_abi!(tyrus_decimal_mul, mul);
decimal_binary_abi!(tyrus_decimal_div, div);
decimal_binary_abi!(tyrus_decimal_pow, pow);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_decimal_to_i128(value: *const TyrusDecimal) -> i128 {
    let Some(value) = (unsafe { value.as_ref() }) else {
        decimal_value_overflow();
    };
    match CoreDecimal::from(*value).to_i128_exact() {
        Ok(value) => value,
        Err(DecimalError::FractionalComponent) => decimal_has_fractional_component(),
        Err(_) => decimal_value_overflow(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_parse_decimal(text: TyrusString) -> *mut TyrusDecimal {
    if text.is_null() {
        return std::ptr::null_mut();
    }
    let text = unsafe { text.as_str() };
    TyrusDecimal::parse(text).map_or(std::ptr::null_mut(), |value| Box::into_raw(Box::new(value)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_decimal_compare(
    left: *const TyrusDecimal,
    right: *const TyrusDecimal,
) -> i32 {
    let (Some(left), Some(right)) = (unsafe { left.as_ref() }, unsafe { right.as_ref() }) else {
        decimal_value_overflow();
    };
    match CoreDecimal::from(*left)
        .compare((*right).into())
        .unwrap_or_else(|_| decimal_value_overflow())
    {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rendered_decimal(coefficient: i128, exponent: i32) -> String {
        let mut fds = [0; 2];
        #[cfg(not(windows))]
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
        #[cfg(windows)]
        assert_eq!(
            unsafe { libc::pipe(fds.as_mut_ptr(), 64 * 1024, libc::O_BINARY) },
            0
        );
        tyrus_decimal_print(coefficient, exponent, fds[1]);
        assert_eq!(unsafe { libc::close(fds[1]) }, 0);
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 64];
        loop {
            #[cfg(not(windows))]
            let read = unsafe { libc::read(fds[0], buffer.as_mut_ptr().cast(), buffer.len()) };
            #[cfg(windows)]
            let read = unsafe {
                libc::read(
                    fds[0],
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as libc::c_uint,
                )
            };
            if read <= 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read as usize]);
        }
        assert_eq!(unsafe { libc::close(fds[0]) }, 0);
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn decimal_print_is_canonical_and_does_not_allocate_a_language_string() {
        assert_eq!(rendered_decimal(100, -2), "1");
        assert_eq!(rendered_decimal(1, -3), "0.001");
        assert_eq!(rendered_decimal(-1230, -2), "-12.3");
        assert_eq!(rendered_decimal(12, 3), "12000");
    }

    #[test]
    fn decimal_text_matches_the_canonical_print_representation() {
        assert_eq!(decimal_text(100, -2), "1");
        assert_eq!(decimal_text(1, -3), "0.001");
        assert_eq!(decimal_text(-1230, -2), "-12.3");
        assert_eq!(decimal_text(5, 2), "500");
    }

    #[test]
    fn decimal_abi_uses_the_shared_arithmetic_core() {
        let left = TyrusDecimal {
            coefficient: 1,
            exponent: -1,
        };
        let right = TyrusDecimal {
            coefficient: 2,
            exponent: -1,
        };
        let mut result = TyrusDecimal {
            coefficient: 0,
            exponent: 0,
        };
        unsafe { tyrus_decimal_add(&mut result, &left, &right) };
        assert_eq!(
            result,
            TyrusDecimal {
                coefficient: 3,
                exponent: -1,
            }
        );
    }

    #[test]
    fn decimal_parser_and_json_use_the_shared_normalized_value() {
        assert_eq!(
            TyrusDecimal::parse("-1_250.00e-2"),
            Some(TyrusDecimal {
                coefficient: -125,
                exponent: -1,
            })
        );
        assert_eq!(TyrusDecimal::parse("19.3d"), None);
        assert_eq!(
            TyrusDecimal {
                coefficient: 1_230,
                exponent: -2,
            }
            .json_number_text(),
            "12.3"
        );
        for (coefficient, exponent, expected) in [
            (1_250_i128, -2_i32, "12.5"),
            (-1_250, -2, "-12.5"),
            (5, -3, "0.005"),
            (12, 2, "1200"),
            (0, -5, "0"),
        ] {
            let value = TyrusDecimal {
                coefficient,
                exponent,
            };
            assert_eq!(value.json_number_text(), expected);
            assert_eq!(decimal_text(coefficient, exponent), expected);
        }
    }
}
