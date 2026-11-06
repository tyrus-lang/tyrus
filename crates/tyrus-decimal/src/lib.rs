#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecimalError {
    OutOfRange,
    DivisionByZero,
    InvalidExponent,
    FractionalComponent,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decimal {
    pub coefficient: i128,
    pub exponent: i32,
}

impl Decimal {
    pub const MAX_SIGNIFICANT_DIGITS: u32 = 34;
    pub const MIN_EXPONENT: i32 = -6_176;
    pub const MAX_ADJUSTED_EXPONENT: i32 = 6_144;
    pub const MAX_COEFFICIENT: i128 = 9_999_999_999_999_999_999_999_999_999_999_999;

    pub const ZERO: Self = Self {
        coefficient: 0,
        exponent: 0,
    };
    pub const ONE: Self = Self {
        coefficient: 1,
        exponent: 0,
    };

    pub fn is_in_range(self) -> bool {
        if self.coefficient == 0 {
            return true;
        }
        let digits = self.coefficient.unsigned_abs().ilog10() + 1;
        self.exponent >= Self::MIN_EXPONENT
            && digits <= Self::MAX_SIGNIFICANT_DIGITS
            && self
                .exponent
                .checked_add(digits as i32 - 1)
                .is_some_and(|exponent| exponent <= Self::MAX_ADJUSTED_EXPONENT)
    }

    pub fn normalized(mut self) -> Result<Self, DecimalError> {
        if self.coefficient == 0 {
            return Ok(Self::ZERO);
        }
        while self.coefficient % 10 == 0 {
            self.coefficient /= 10;
            self.exponent = self
                .exponent
                .checked_add(1)
                .ok_or(DecimalError::OutOfRange)?;
        }
        self.is_in_range()
            .then_some(self)
            .ok_or(DecimalError::OutOfRange)
    }

    pub fn from_integer(value: i128) -> Self {
        Self {
            coefficient: value,
            exponent: 0,
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        if bytes.is_empty() {
            return None;
        }
        let (negative, unsigned) = match bytes[0] {
            b'+' => (false, &text[1..]),
            b'-' => (true, &text[1..]),
            _ => (false, text),
        };
        if unsigned.is_empty() {
            return None;
        }
        let exponent_index = unsigned
            .bytes()
            .position(|byte| matches!(byte, b'e' | b'E'));
        let (significand, explicit_exponent) = match exponent_index {
            Some(index) => (&unsigned[..index], parse_exponent(&unsigned[index + 1..])?),
            None => (unsigned, 0),
        };
        if significand.is_empty() || significand.bytes().any(|byte| matches!(byte, b'e' | b'E')) {
            return None;
        }

        let bytes = significand.as_bytes();
        let mut coefficient = 0_i128;
        let mut pending_zeros = 0_i32;
        let mut significant_digits = 0_u32;
        let mut fractional_digits = 0_i32;
        let mut digits_before_point = 0_u32;
        let mut digits_after_point = 0_u32;
        let mut seen_point = false;
        let mut seen_nonzero = false;

        for (index, byte) in bytes.iter().copied().enumerate() {
            match byte {
                b'.' if !seen_point => seen_point = true,
                b'0'..=b'9' => {
                    if seen_point {
                        fractional_digits = fractional_digits.checked_add(1)?;
                        digits_after_point = digits_after_point.checked_add(1)?;
                    } else {
                        digits_before_point = digits_before_point.checked_add(1)?;
                    }
                    let digit = i128::from(byte - b'0');
                    if !seen_nonzero && digit == 0 {
                        continue;
                    }
                    if digit == 0 {
                        pending_zeros = pending_zeros.checked_add(1)?;
                        continue;
                    }
                    for _ in 0..pending_zeros {
                        significant_digits = significant_digits.checked_add(1)?;
                        if significant_digits > Self::MAX_SIGNIFICANT_DIGITS {
                            return None;
                        }
                        coefficient = coefficient.checked_mul(10)?;
                    }
                    pending_zeros = 0;
                    significant_digits = significant_digits.checked_add(1)?;
                    if significant_digits > Self::MAX_SIGNIFICANT_DIGITS {
                        return None;
                    }
                    coefficient = coefficient.checked_mul(10)?.checked_add(digit)?;
                    seen_nonzero = true;
                }
                b'_' if index > 0
                    && index + 1 < bytes.len()
                    && bytes[index - 1].is_ascii_digit()
                    && bytes[index + 1].is_ascii_digit() => {}
                _ => return None,
            }
        }
        if digits_before_point == 0 || (seen_point && digits_after_point == 0) {
            return None;
        }
        if !seen_nonzero {
            return Some(Self::ZERO);
        }
        let exponent = explicit_exponent
            .checked_sub(fractional_digits)?
            .checked_add(pending_zeros)?;
        let coefficient = if negative { -coefficient } else { coefficient };
        let value = Self {
            coefficient,
            exponent,
        };
        value.is_in_range().then_some(value)
    }

    pub fn to_i128_exact(self) -> Result<i128, DecimalError> {
        let value = self.normalized()?;
        if value.exponent >= 0 {
            let power = u32::try_from(value.exponent).map_err(|_| DecimalError::OutOfRange)?;
            if power > 38 {
                return Err(DecimalError::OutOfRange);
            }
            return value
                .coefficient
                .checked_mul(10_i128.pow(power))
                .ok_or(DecimalError::OutOfRange);
        }
        let power = value.exponent.unsigned_abs();
        if power > 38 {
            return Err(DecimalError::FractionalComponent);
        }
        let divisor = 10_i128.pow(power);
        (value.coefficient % divisor == 0)
            .then_some(value.coefficient / divisor)
            .ok_or(DecimalError::FractionalComponent)
    }

    pub fn compare(self, other: Self) -> Result<std::cmp::Ordering, DecimalError> {
        let left = self.normalized()?;
        let right = other.normalized()?;
        if left.coefficient < 0 && right.coefficient >= 0 {
            return Ok(std::cmp::Ordering::Less);
        }
        if left.coefficient >= 0 && right.coefficient < 0 {
            return Ok(std::cmp::Ordering::Greater);
        }
        let left_digits = if left.coefficient == 0 {
            1
        } else {
            left.coefficient.unsigned_abs().ilog10() + 1
        };
        let right_digits = if right.coefficient == 0 {
            1
        } else {
            right.coefficient.unsigned_abs().ilog10() + 1
        };
        let left_magnitude = i64::from(left.exponent) + i64::from(left_digits);
        let right_magnitude = i64::from(right.exponent) + i64::from(right_digits);
        if left_magnitude != right_magnitude {
            let ordering = left_magnitude.cmp(&right_magnitude);
            return Ok(if left.coefficient < 0 {
                ordering.reverse()
            } else {
                ordering
            });
        }
        let exponent = left.exponent.min(right.exponent);
        let left_power = left
            .exponent
            .checked_sub(exponent)
            .ok_or(DecimalError::OutOfRange)?;
        let right_power = right
            .exponent
            .checked_sub(exponent)
            .ok_or(DecimalError::OutOfRange)?;
        Ok(
            scale_by_power_of_ten(left.coefficient, left_power as u32)?.cmp(
                &scale_by_power_of_ten(right.coefficient, right_power as u32)?,
            ),
        )
    }

    #[allow(clippy::should_implement_trait)]
    pub fn add(self, other: Self) -> Result<Self, DecimalError> {
        let exponent = self.exponent.min(other.exponent);
        let left_power = self
            .exponent
            .checked_sub(exponent)
            .ok_or(DecimalError::OutOfRange)?;
        let right_power = other
            .exponent
            .checked_sub(exponent)
            .ok_or(DecimalError::OutOfRange)?;
        Self {
            coefficient: scale_by_power_of_ten(self.coefficient, left_power as u32)?
                .checked_add(scale_by_power_of_ten(
                    other.coefficient,
                    right_power as u32,
                )?)
                .ok_or(DecimalError::OutOfRange)?,
            exponent,
        }
        .rounded_to_precision()
    }

    #[allow(clippy::should_implement_trait)]
    pub fn sub(self, other: Self) -> Result<Self, DecimalError> {
        self.add(Self {
            coefficient: other
                .coefficient
                .checked_neg()
                .ok_or(DecimalError::OutOfRange)?,
            exponent: other.exponent,
        })
    }

    pub fn rounded_to_precision(mut self) -> Result<Self, DecimalError> {
        if self.coefficient == 0 {
            return Ok(Self::ZERO);
        }
        let mut digits = self.coefficient.unsigned_abs().ilog10() + 1;
        while digits > Self::MAX_SIGNIFICANT_DIGITS {
            let remainder = self.coefficient % 10;
            self.coefficient /= 10;
            if remainder.unsigned_abs() > 5
                || (remainder.unsigned_abs() == 5 && self.coefficient % 2 != 0)
            {
                self.coefficient = self
                    .coefficient
                    .checked_add(if remainder.is_negative() { -1 } else { 1 })
                    .ok_or(DecimalError::OutOfRange)?;
            }
            self.exponent = self
                .exponent
                .checked_add(1)
                .ok_or(DecimalError::OutOfRange)?;
            digits = self.coefficient.unsigned_abs().ilog10() + 1;
        }
        self.normalized()
    }

    #[allow(clippy::should_implement_trait)]
    pub fn mul(self, other: Self) -> Result<Self, DecimalError> {
        if self.coefficient == 0 || other.coefficient == 0 {
            return Ok(Self::ZERO);
        }
        let negative = (self.coefficient < 0) != (other.coefficient < 0);
        let mut coefficient = U256::multiply(
            self.coefficient.unsigned_abs(),
            other.coefficient.unsigned_abs(),
        );
        let mut exponent = self
            .exponent
            .checked_add(other.exponent)
            .ok_or(DecimalError::OutOfRange)?;
        let mut sticky = false;
        let mut discarded = 0;
        while coefficient.decimal_digits() > Self::MAX_SIGNIFICANT_DIGITS {
            let digit = coefficient.divide_by_ten();
            sticky |= discarded != 0;
            discarded = digit;
            exponent = exponent.checked_add(1).ok_or(DecimalError::OutOfRange)?;
        }
        if discarded > 5 || (discarded == 5 && (sticky || coefficient.is_odd())) {
            coefficient.increment()?;
            if coefficient.decimal_digits() > Self::MAX_SIGNIFICANT_DIGITS {
                coefficient.divide_by_ten();
                exponent = exponent.checked_add(1).ok_or(DecimalError::OutOfRange)?;
            }
        }
        let coefficient =
            i128::try_from(coefficient.to_u128()?).map_err(|_| DecimalError::OutOfRange)?;
        Self {
            coefficient: if negative { -coefficient } else { coefficient },
            exponent,
        }
        .normalized()
    }

    #[allow(clippy::should_implement_trait)]
    pub fn div(self, other: Self) -> Result<Self, DecimalError> {
        if other.coefficient == 0 {
            return Err(DecimalError::DivisionByZero);
        }
        let negative = (self.coefficient < 0) != (other.coefficient < 0);
        let numerator = self.coefficient.unsigned_abs();
        let denominator = other.coefficient.unsigned_abs();
        let mut coefficient = numerator / denominator;
        let mut remainder = numerator % denominator;
        let mut fractional = 0_u32;
        while remainder != 0 && decimal_digits(coefficient) < Self::MAX_SIGNIFICANT_DIGITS + 1 {
            remainder = remainder.checked_mul(10).ok_or(DecimalError::OutOfRange)?;
            let digit = remainder / denominator;
            remainder %= denominator;
            coefficient = coefficient
                .checked_mul(10)
                .and_then(|value| value.checked_add(digit))
                .ok_or(DecimalError::OutOfRange)?;
            fractional += 1;
        }
        let mut exponent = self
            .exponent
            .checked_sub(other.exponent)
            .and_then(|value| value.checked_sub(fractional as i32))
            .ok_or(DecimalError::OutOfRange)?;
        if decimal_digits(coefficient) > Self::MAX_SIGNIFICANT_DIGITS {
            let discarded = coefficient % 10;
            coefficient /= 10;
            if discarded > 5
                || (discarded == 5 && (remainder != 0 || !coefficient.is_multiple_of(2)))
            {
                coefficient = coefficient.checked_add(1).ok_or(DecimalError::OutOfRange)?;
            }
            exponent = exponent.checked_add(1).ok_or(DecimalError::OutOfRange)?;
        }
        let coefficient = i128::try_from(coefficient).map_err(|_| DecimalError::OutOfRange)?;
        Self {
            coefficient: if negative { -coefficient } else { coefficient },
            exponent,
        }
        .normalized()
    }

    pub fn pow(self, exponent: Self) -> Result<Self, DecimalError> {
        let exponent = exponent.normalized()?;
        if exponent.coefficient < 0 || exponent.exponent < 0 || exponent.exponent > 38 {
            return Err(DecimalError::InvalidExponent);
        }
        let mut power = u32::try_from(
            exponent
                .coefficient
                .checked_mul(10_i128.pow(exponent.exponent as u32))
                .ok_or(DecimalError::InvalidExponent)?,
        )
        .map_err(|_| DecimalError::InvalidExponent)?;
        let mut result = Self::ONE;
        let mut base = self;
        while power != 0 {
            if power & 1 == 1 {
                result = result.mul(base)?;
            }
            power >>= 1;
            if power != 0 {
                base = base.mul(base)?;
            }
        }
        Ok(result)
    }
}

fn parse_exponent(text: &str) -> Option<i32> {
    if text.is_empty() {
        return None;
    }
    let (negative, digits) = match text.as_bytes()[0] {
        b'+' => (false, &text[1..]),
        b'-' => (true, &text[1..]),
        _ => (false, text),
    };
    if digits.is_empty() {
        return None;
    }
    let bytes = digits.as_bytes();
    let mut value = 0_i32;
    for (index, byte) in bytes.iter().copied().enumerate() {
        match byte {
            b'0'..=b'9' => value = value.checked_mul(10)?.checked_add(i32::from(byte - b'0'))?,
            b'_' if index > 0
                && index + 1 < bytes.len()
                && bytes[index - 1].is_ascii_digit()
                && bytes[index + 1].is_ascii_digit() => {}
            _ => return None,
        }
    }
    if negative {
        value.checked_neg()
    } else {
        Some(value)
    }
}

fn scale_by_power_of_ten(coefficient: i128, power: u32) -> Result<i128, DecimalError> {
    if power > 38 {
        return Err(DecimalError::OutOfRange);
    }
    coefficient
        .checked_mul(10_i128.pow(power))
        .ok_or(DecimalError::OutOfRange)
}
fn decimal_digits(value: u128) -> u32 {
    if value == 0 { 0 } else { value.ilog10() + 1 }
}

#[derive(Clone, Copy)]
struct U256 {
    limbs: [u64; 4],
}

impl U256 {
    fn multiply(left: u128, right: u128) -> Self {
        let left = [left as u64, (left >> 64) as u64];
        let right = [right as u64, (right >> 64) as u64];
        let mut result = Self { limbs: [0; 4] };
        for (i, a) in left.into_iter().enumerate() {
            let mut carry = 0_u128;
            for (j, b) in right.into_iter().enumerate() {
                let index = i + j;
                let product =
                    u128::from(result.limbs[index]) + u128::from(a) * u128::from(b) + carry;
                result.limbs[index] = product as u64;
                carry = product >> 64;
            }
            let mut index = i + 2;
            while carry != 0 {
                let sum = u128::from(result.limbs[index]) + carry;
                result.limbs[index] = sum as u64;
                carry = sum >> 64;
                index += 1;
            }
        }
        result
    }
    fn divide_by_ten(&mut self) -> u8 {
        let mut remainder = 0_u128;
        for limb in self.limbs.iter_mut().rev() {
            let dividend = (remainder << 64) | u128::from(*limb);
            *limb = (dividend / 10) as u64;
            remainder = dividend % 10;
        }
        remainder as u8
    }
    fn increment(&mut self) -> Result<(), DecimalError> {
        for limb in &mut self.limbs {
            let (next, overflow) = limb.overflowing_add(1);
            *limb = next;
            if !overflow {
                return Ok(());
            }
        }
        Err(DecimalError::OutOfRange)
    }
    fn decimal_digits(self) -> u32 {
        let mut value = self;
        let mut digits = 1;
        while value.limbs != [0; 4] {
            value.divide_by_ten();
            if value.limbs != [0; 4] {
                digits += 1;
            }
        }
        digits
    }
    fn is_odd(self) -> bool {
        self.limbs[0] & 1 != 0
    }
    fn to_u128(self) -> Result<u128, DecimalError> {
        if self.limbs[2] != 0 || self.limbs[3] != 0 {
            return Err(DecimalError::OutOfRange);
        }
        Ok(u128::from(self.limbs[0]) | (u128::from(self.limbs[1]) << 64))
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn accepted_values_respect_every_public_decimal_limit() {
        let coefficient: i128 = kani::any();
        let exponent: i32 = kani::any();
        let value = Decimal {
            coefficient,
            exponent,
        };
        kani::assume(value.is_in_range() && coefficient != 0);
        let digits = coefficient.unsigned_abs().ilog10() + 1;
        assert!(digits <= Decimal::MAX_SIGNIFICANT_DIGITS);
        assert!(exponent >= Decimal::MIN_EXPONENT);
        assert!(exponent + digits as i32 - 1 <= Decimal::MAX_ADJUSTED_EXPONENT);
    }

    #[kani::proof]
    #[kani::unwind(2)]
    fn rounding_one_discarded_digit_uses_half_even() {
        let retained_last_digit: u8 = kani::any();
        let discarded: u8 = kani::any();
        kani::assume((1..=8).contains(&retained_last_digit));
        kani::assume(discarded <= 9);
        let retained =
            1_000_000_000_000_000_000_000_000_000_000_000_i128 + i128::from(retained_last_digit);
        let discarded = i128::from(discarded);
        let coefficient = retained * 10 + discarded;
        let rounded = Decimal {
            coefficient,
            exponent: 0,
        }
        .rounded_to_precision()
        .unwrap();
        let increment = discarded > 5 || (discarded == 5 && retained % 2 != 0);
        assert_eq!(rounded.coefficient, retained + i128::from(increment));
        assert_eq!(rounded.exponent, 1);
    }

    #[kani::proof]
    #[kani::unwind(5)]
    fn u256_multiplication_preserves_every_bit_of_u8_products() {
        let left: u8 = kani::any();
        let right: u8 = kani::any();
        let product = u128::from(left) * u128::from(right);
        let wide = U256::multiply(u128::from(left), u128::from(right));
        assert_eq!(wide.limbs[0], product as u64);
        assert_eq!(wide.limbs[1], (product >> 64) as u64);
        assert_eq!(wide.limbs[2], 0);
        assert_eq!(wide.limbs[3], 0);
    }

    #[kani::proof]
    #[kani::unwind(2)]
    fn comparison_is_reflexive_for_bounded_normalized_values() {
        let coefficient: i32 = kani::any();
        let exponent: i8 = kani::any();
        kani::assume(coefficient != 0 && coefficient % 10 != 0);
        let value = Decimal {
            coefficient: i128::from(coefficient),
            exponent: i32::from(exponent),
        };
        assert_eq!(value.compare(value), Ok(std::cmp::Ordering::Equal));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decimal(coefficient: i128, exponent: i32) -> Decimal {
        Decimal {
            coefficient,
            exponent,
        }
    }

    #[test]
    fn parsing_uses_the_literal_decimal_grammar_without_a_suffix() {
        assert_eq!(Decimal::parse("19.3"), Some(decimal(193, -1)));
        assert_eq!(Decimal::parse("-1_250.00e-2"), Some(decimal(-125, -1)));
        assert_eq!(Decimal::parse("0.000"), Some(Decimal::ZERO));
        assert_eq!(Decimal::parse("19.3d"), None);
        assert_eq!(Decimal::parse("1e6145"), None);
        assert_eq!(Decimal::parse("1__2"), None);
        assert_eq!(Decimal::parse(".1"), None);
        assert_eq!(Decimal::parse("1."), None);
    }

    #[test]
    fn arithmetic_preserves_base_ten_values_and_half_even_rounding() {
        assert_eq!(decimal(1, -1).add(decimal(2, -1)), Ok(decimal(3, -1)));
        assert_eq!(decimal(1, -1).sub(decimal(2, -1)), Ok(decimal(-1, -1)));
        assert_eq!(decimal(1, -1).mul(decimal(2, -1)), Ok(decimal(2, -2)));
        assert_eq!(
            decimal(1, 0).div(decimal(3, 0)),
            Ok(decimal(3_333_333_333_333_333_333_333_333_333_333_333, -34))
        );
        assert_eq!(
            decimal(1, 0).div(decimal(6, 0)),
            Ok(decimal(1_666_666_666_666_666_666_666_666_666_666_667, -34))
        );
        assert_eq!(decimal(25, -1).pow(decimal(2, 0)), Ok(decimal(625, -2)));
    }

    #[test]
    fn multiplication_uses_the_full_256_bit_intermediate() {
        let maximum = decimal(9_999_999_999_999_999_999_999_999_999_999_999, 0);
        assert_eq!(
            maximum.mul(maximum),
            Ok(decimal(9_999_999_999_999_999_999_999_999_999_999_998, 34))
        );
    }

    #[test]
    fn comparison_and_integer_conversion_are_representation_independent() {
        assert_eq!(
            decimal(10, -1).compare(decimal(1, 0)),
            Ok(std::cmp::Ordering::Equal)
        );
        assert_eq!(
            decimal(1, -1).compare(decimal(2, -1)),
            Ok(std::cmp::Ordering::Less)
        );
        assert_eq!(decimal(1_200, -2).to_i128_exact(), Ok(12));
        assert_eq!(
            decimal(12, -1).to_i128_exact(),
            Err(DecimalError::FractionalComponent)
        );
    }

    #[test]
    fn errors_are_structured_for_consumers() {
        assert_eq!(
            decimal(1, 0).div(Decimal::ZERO),
            Err(DecimalError::DivisionByZero)
        );
        assert_eq!(
            decimal(2, 0).pow(decimal(-1, 0)),
            Err(DecimalError::InvalidExponent)
        );
    }
}
