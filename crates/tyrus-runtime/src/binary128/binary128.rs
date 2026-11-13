//! Exact `f128` (IEEE binary128) to decimal text.
//!
//! Generated code cannot format an `f128` the way it formats the narrower
//! floats: Rust has no stable `f128`, so the value cannot cross the ABI as a
//! number this crate can do arithmetic on. It arrives as its two raw 64-bit
//! halves instead and is decoded here.
//!
//! The digits are produced exactly. The obvious approach, peel one digit at a
//! time by multiplying the remainder by ten in `f128` arithmetic, drifts,
//! because every multiply rounds: `0.000025` came out as
//! `2.499999999999999999e-5` even though the stored value was the correctly
//! rounded one. This module works in exact integers instead, and emits the
//! *shortest* digit string that still round-trips to the same bit pattern, the
//! same guarantee `f32`/`f64` printing gives.

use std::cmp::Ordering;

const EXPONENT_BIAS: i32 = 16383;
const FRACTION_BITS: u32 = 112;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Big {
    limbs: Vec<u32>,
}

impl Big {
    fn from_u128(mut value: u128) -> Self {
        let mut limbs = Vec::new();
        while value > 0 {
            limbs.push((value & 0xFFFF_FFFF) as u32);
            value >>= 32;
        }
        Self { limbs }
    }

    fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    fn trim(&mut self) {
        while self.limbs.last() == Some(&0) {
            self.limbs.pop();
        }
    }

    fn mul_small(&mut self, factor: u32) {
        if self.is_zero() || factor == 0 {
            if factor == 0 {
                self.limbs.clear();
            }
            return;
        }
        let mut carry = 0u64;
        for limb in &mut self.limbs {
            let product = u64::from(*limb) * u64::from(factor) + carry;
            *limb = (product & 0xFFFF_FFFF) as u32;
            carry = product >> 32;
        }
        while carry > 0 {
            self.limbs.push((carry & 0xFFFF_FFFF) as u32);
            carry >>= 32;
        }
    }

    fn shl_bits(&mut self, bits: u32) {
        if self.is_zero() || bits == 0 {
            return;
        }
        let whole = (bits / 32) as usize;
        let part = bits % 32;
        if part > 0 {
            let mut carry = 0u32;
            for limb in &mut self.limbs {
                let shifted = (u64::from(*limb) << part) | u64::from(carry);
                *limb = (shifted & 0xFFFF_FFFF) as u32;
                carry = (shifted >> 32) as u32;
            }
            if carry > 0 {
                self.limbs.push(carry);
            }
        }
        if whole > 0 {
            self.limbs.splice(0..0, std::iter::repeat_n(0u32, whole));
        }
    }

    #[cfg(test)]
    fn mul_pow5(&mut self, mut power: u32) {
        const CHUNK: u32 = 13;
        const CHUNK_VALUE: u32 = 1_220_703_125;
        while power >= CHUNK {
            self.mul_small(CHUNK_VALUE);
            power -= CHUNK;
        }
        if power > 0 {
            self.mul_small(5u32.pow(power));
        }
    }

    #[cfg(test)]
    fn mul_pow10(&mut self, power: u32) {
        self.mul_pow5(power);
        self.shl_bits(power);
    }

    fn cmp_big(&self, other: &Self) -> Ordering {
        if self.limbs.len() != other.limbs.len() {
            return self.limbs.len().cmp(&other.limbs.len());
        }
        for (left, right) in self.limbs.iter().rev().zip(other.limbs.iter().rev()) {
            match left.cmp(right) {
                Ordering::Equal => {}
                other => return other,
            }
        }
        Ordering::Equal
    }

    fn add_assign(&mut self, other: &Self) {
        if other.limbs.len() > self.limbs.len() {
            self.limbs.resize(other.limbs.len(), 0);
        }
        let mut carry = 0u64;
        for (index, limb) in self.limbs.iter_mut().enumerate() {
            let addend = other.limbs.get(index).copied().unwrap_or(0);
            let sum = u64::from(*limb) + u64::from(addend) + carry;
            *limb = (sum & 0xFFFF_FFFF) as u32;
            carry = sum >> 32;
        }
        if carry > 0 {
            self.limbs.push(carry as u32);
        }
    }

    fn sub_assign(&mut self, other: &Self) {
        let mut borrow = 0i64;
        for (index, limb) in self.limbs.iter_mut().enumerate() {
            let subtrahend = other.limbs.get(index).copied().unwrap_or(0);
            let diff = i64::from(*limb) - i64::from(subtrahend) - borrow;
            if diff < 0 {
                *limb = (diff + (1i64 << 32)) as u32;
                borrow = 1;
            } else {
                *limb = diff as u32;
                borrow = 0;
            }
        }
        debug_assert_eq!(borrow, 0, "sub_assign underflowed");
        self.trim();
    }

    fn sum(&self, other: &Self) -> Self {
        let mut result = self.clone();
        result.add_assign(other);
        result
    }
}

enum Decoded {
    Zero {
        negative: bool,
    },
    Finite {
        negative: bool,
        digits: Vec<u8>,
        exponent: i32,
    },
    Infinite {
        negative: bool,
    },
    NaN,
}

fn decode(low: u64, high: u64) -> Decoded {
    let negative = high >> 63 == 1;
    let biased = ((high >> 48) & 0x7FFF) as i32;
    let fraction = (u128::from(high & 0xFFFF_FFFF_FFFF) << 64) | u128::from(low);

    if biased == 0x7FFF {
        return if fraction == 0 {
            Decoded::Infinite { negative }
        } else {
            Decoded::NaN
        };
    }
    if biased == 0 && fraction == 0 {
        return Decoded::Zero { negative };
    }

    let (significand, exponent) = if biased == 0 {
        (fraction, 1 - EXPONENT_BIAS - FRACTION_BITS as i32)
    } else {
        (
            fraction | (1u128 << FRACTION_BITS),
            biased - EXPONENT_BIAS - FRACTION_BITS as i32,
        )
    };
    let uneven_gaps = fraction == 0 && biased > 1;
    let (digits, exponent) = shortest_digits(significand, exponent, uneven_gaps);
    Decoded::Finite {
        negative,
        digits,
        exponent,
    }
}

fn shortest_digits(significand: u128, exponent: i32, uneven_gaps: bool) -> (Vec<u8>, i32) {
    let mut numerator = Big::from_u128(significand);
    let mut denominator = Big::from_u128(1);
    let mut gap_high = Big::from_u128(1);
    let mut gap_low = Big::from_u128(1);

    if exponent >= 0 {
        let shift = exponent as u32;
        if uneven_gaps {
            numerator.shl_bits(shift + 2);
            denominator = Big::from_u128(4);
            gap_high.shl_bits(shift + 1);
            gap_low.shl_bits(shift);
        } else {
            numerator.shl_bits(shift + 1);
            denominator = Big::from_u128(2);
            gap_high.shl_bits(shift);
            gap_low.shl_bits(shift);
        }
    } else {
        let shift = (-exponent) as u32;
        if uneven_gaps {
            numerator.shl_bits(2);
            denominator.shl_bits(shift + 2);
            gap_high = Big::from_u128(2);
            gap_low = Big::from_u128(1);
        } else {
            numerator.shl_bits(1);
            denominator.shl_bits(shift + 1);
            gap_high = Big::from_u128(1);
            gap_low = Big::from_u128(1);
        }
    }

    let mut k = 0i32;
    loop {
        let high_edge = numerator.sum(&gap_high);
        if high_edge.cmp_big(&denominator) == Ordering::Greater {
            denominator.mul_small(10);
            k += 1;
        } else {
            break;
        }
    }
    loop {
        let mut high_edge = numerator.sum(&gap_high);
        high_edge.mul_small(10);
        if high_edge.cmp_big(&denominator) != Ordering::Greater {
            numerator.mul_small(10);
            gap_high.mul_small(10);
            gap_low.mul_small(10);
            k -= 1;
        } else {
            break;
        }
    }

    let mut digits = Vec::new();
    loop {
        numerator.mul_small(10);
        gap_high.mul_small(10);
        gap_low.mul_small(10);

        let mut digit = 0u8;
        while numerator.cmp_big(&denominator) != Ordering::Less {
            numerator.sub_assign(&denominator);
            digit += 1;
        }

        let below = numerator.cmp_big(&gap_low) == Ordering::Less;
        let above = numerator.sum(&gap_high).cmp_big(&denominator) == Ordering::Greater;

        match (below, above) {
            (false, false) => digits.push(digit),
            (true, false) => {
                digits.push(digit);
                break;
            }
            (false, true) => {
                digits.push(digit + 1);
                break;
            }
            (true, true) => {
                let mut doubled = numerator.clone();
                doubled.shl_bits(1);
                digits.push(match doubled.cmp_big(&denominator) {
                    Ordering::Less => digit,
                    _ => digit + 1,
                });
                break;
            }
        }
    }

    (digits, k)
}

fn render(negative: bool, digits: &[u8], exponent: i32) -> String {
    const FIXED_LOW: i32 = -5;
    const FIXED_HIGH: i32 = 21;

    let sign = if negative { "-" } else { "" };
    let text = digits
        .iter()
        .map(|digit| char::from(b'0' + digit))
        .collect::<String>();

    if exponent > FIXED_LOW && exponent <= FIXED_HIGH {
        let count = text.len() as i32;
        if exponent <= 0 {
            let zeros = "0".repeat((-exponent) as usize);
            return format!("{sign}0.{zeros}{text}");
        }
        if exponent >= count {
            let zeros = "0".repeat((exponent - count) as usize);
            return format!("{sign}{text}{zeros}.0");
        }
        let (integer, fraction) = text.split_at(exponent as usize);
        return format!("{sign}{integer}.{fraction}");
    }

    let (first, rest) = text.split_at(1);
    let fraction = if rest.is_empty() { "0" } else { rest };
    format!("{sign}{first}.{fraction}e{:+}", exponent - 1)
}

pub fn format(low: u64, high: u64) -> String {
    match decode(low, high) {
        Decoded::NaN => "NaN".to_string(),
        Decoded::Infinite { negative } => {
            if negative {
                "-Infinity".to_string()
            } else {
                "Infinity".to_string()
            }
        }
        Decoded::Zero { negative } => {
            if negative {
                "-0.0".to_string()
            } else {
                "0.0".to_string()
            }
        }
        Decoded::Finite {
            negative,
            digits,
            exponent,
        } => render(negative, &digits, exponent),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bits(negative: bool, biased_exponent: u32, fraction: u128) -> (u64, u64) {
        let high = (u64::from(negative) << 63)
            | (u64::from(biased_exponent) << 48)
            | ((fraction >> 64) as u64 & 0xFFFF_FFFF_FFFF);
        (fraction as u64, high)
    }

    fn exact(significand: u128, exponent: i32) -> (u64, u64) {
        let mut significand = significand;
        let mut exponent = exponent;
        while significand >> FRACTION_BITS == 0 {
            significand <<= 1;
            exponent -= 1;
        }
        while significand >> FRACTION_BITS > 1 {
            significand >>= 1;
            exponent += 1;
        }
        let biased = (exponent + EXPONENT_BIAS + FRACTION_BITS as i32) as u32;
        bits(false, biased, significand & ((1u128 << FRACTION_BITS) - 1))
    }

    #[test]
    fn formats_exactly_representable_values() {
        let (low, high) = exact(1, 0);
        assert_eq!(format(low, high), "1.0");
        let (low, high) = exact(3, -1);
        assert_eq!(format(low, high), "1.5");
        let (low, high) = exact(5, 1);
        assert_eq!(format(low, high), "10.0");
        let (low, high) = exact(10_000_000_000, 0);
        assert_eq!(format(low, high), "10000000000.0");
    }

    #[test]
    fn picks_the_shortest_text_that_still_identifies_the_value() {
        let tenth = nearest_decimal(1, 1);
        assert_eq!(format(tenth.0, tenth.1), "0.1");
        let small = nearest_decimal(25, 6);
        assert_eq!(format(small.0, small.1), "0.000025");
        let third = nearest_decimal(2, 1);
        assert_eq!(format(third.0, third.1), "0.2");
    }

    fn nearest_decimal(mantissa: u128, scale: u32) -> (u64, u64) {
        let mut numerator = Big::from_u128(mantissa);
        let mut denominator = Big::from_u128(1);
        denominator.mul_pow10(scale);
        let mut exponent = 0i32;
        loop {
            let mut probe = denominator.clone();
            probe.shl_bits(FRACTION_BITS);
            match numerator.cmp_big(&probe) {
                Ordering::Less => {
                    numerator.shl_bits(1);
                    exponent -= 1;
                }
                _ => break,
            }
        }
        loop {
            let mut probe = denominator.clone();
            probe.shl_bits(FRACTION_BITS + 1);
            match numerator.cmp_big(&probe) {
                Ordering::Less => break,
                _ => {
                    denominator.shl_bits(1);
                    exponent += 1;
                }
            }
        }
        let mut significand = 0u128;
        let mut remainder = numerator.clone();
        let mut bit = FRACTION_BITS as i32;
        while bit >= 0 {
            let mut scaled = denominator.clone();
            scaled.shl_bits(bit as u32);
            if remainder.cmp_big(&scaled) != Ordering::Less {
                remainder.sub_assign(&scaled);
                significand |= 1u128 << bit;
            }
            bit -= 1;
        }
        let mut doubled = remainder.clone();
        doubled.shl_bits(1);
        if doubled.cmp_big(&denominator) != Ordering::Less {
            significand += 1;
        }
        exact(significand, exponent)
    }

    #[test]
    fn formats_zero_infinity_and_nan() {
        assert_eq!(format(0, 0), "0.0");
        let (low, high) = bits(true, 0, 0);
        assert_eq!(format(low, high), "-0.0");
        let (low, high) = bits(false, 0x7FFF, 0);
        assert_eq!(format(low, high), "Infinity");
        let (low, high) = bits(true, 0x7FFF, 0);
        assert_eq!(format(low, high), "-Infinity");
        let (low, high) = bits(false, 0x7FFF, 1);
        assert_eq!(format(low, high), "NaN");
    }

    #[test]
    fn formats_negative_values() {
        let (low, mut high) = exact(3, -1);
        high |= 1 << 63;
        assert_eq!(format(low, high), "-1.5");
    }

    #[test]
    fn switches_to_scientific_notation_outside_the_readable_range() {
        let (low, high) = bits(false, 0x7FFE, (1u128 << FRACTION_BITS) - 1);
        let text = format(low, high);
        assert!(text.ends_with("e+4932"), "unexpected text: {text}");
        assert!(
            text.len() < 48,
            "scientific notation should stay short: {text}"
        );
        let text = format(1, 0);
        assert!(text.ends_with("e-4966"), "unexpected text: {text}");
    }

    #[test]
    fn every_shortest_text_round_trips_to_its_own_value() {
        for scale in 0..12u32 {
            for mantissa in [1u128, 3, 7, 25, 1234, 999_999] {
                let (low, high) = nearest_decimal(mantissa, scale);
                let text = format(low, high);
                let reparsed = parse_shortest(&text);
                assert_eq!(
                    reparsed,
                    (low, high),
                    "`{text}` did not round-trip (mantissa {mantissa}, scale {scale})"
                );
            }
        }
    }

    fn parse_shortest(text: &str) -> (u64, u64) {
        let text = text.strip_prefix('-').unwrap_or(text);
        let (mantissa_text, power) = match text.split_once('e') {
            Some((mantissa, exponent)) => (mantissa, exponent.parse::<i32>().unwrap()),
            None => (text, 0),
        };
        let (digits, fraction_len) = match mantissa_text.split_once('.') {
            Some((integer, fraction)) => (format!("{integer}{fraction}"), fraction.len() as i32),
            None => (mantissa_text.to_string(), 0),
        };
        let mut mantissa = digits.parse::<u128>().unwrap();
        let mut scale = fraction_len - power;
        while scale < 0 {
            mantissa *= 10;
            scale += 1;
        }
        nearest_decimal(mantissa, scale as u32)
    }
}
