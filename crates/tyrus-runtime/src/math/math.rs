//! Mathematical primitives for `tyrus:math`.
//!
//! This module wraps Rust's intrinsic floating-point methods and a few
//! integer helpers so the codegen can emit direct `extern "C"` calls without
//! depending on libm or any extra runtime allocation.

use crate::abi::TyrusArray;

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_abs(x: f64) -> f64 {
    x.abs()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_sign(x: f64) -> f64 {
    x.signum()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_sqrt(x: f64) -> f64 {
    x.sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_cbrt(x: f64) -> f64 {
    x.cbrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_floor(x: f64) -> f64 {
    x.floor()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_ceil(x: f64) -> f64 {
    x.ceil()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_trunc(x: f64) -> f64 {
    x.trunc()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_round(x: f64) -> f64 {
    x.round()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_sin(x: f64) -> f64 {
    x.sin()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_cos(x: f64) -> f64 {
    x.cos()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_tan(x: f64) -> f64 {
    x.tan()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_asin(x: f64) -> f64 {
    x.asin()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_acos(x: f64) -> f64 {
    x.acos()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_atan(x: f64) -> f64 {
    x.atan()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_sinh(x: f64) -> f64 {
    x.sinh()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_cosh(x: f64) -> f64 {
    x.cosh()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_tanh(x: f64) -> f64 {
    x.tanh()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_exp(x: f64) -> f64 {
    x.exp()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_log(x: f64) -> f64 {
    x.ln()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_log10(x: f64) -> f64 {
    x.log10()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_log2(x: f64) -> f64 {
    x.log2()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_pow(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_max(a: f64, b: f64) -> f64 {
    a.max(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_min(a: f64, b: f64) -> f64 {
    a.min(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_imul(a: i32, b: i32) -> i32 {
    ((a as u32).wrapping_mul(b as u32)) as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_clz32(x: i32) -> i32 {
    (x as u32).leading_zeros() as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_math_approx_equal(a: f64, b: f64, tolerance: *const f64) -> bool {
    let tol = if tolerance.is_null() {
        f64::EPSILON * a.abs().max(b.abs())
    } else {
        unsafe { *tolerance }
    };
    (a - b).abs() <= tol
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_clamp(value: f64, min: f64, max: f64) -> f64 {
    value.clamp(min, max)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_clamp_min(value: f64, min: f64) -> f64 {
    value.max(min)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_clamp_max(value: f64, max: f64) -> f64 {
    value.min(max)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_lerp(start: f64, end: f64, amount: f64) -> f64 {
    start + (end - start) * amount
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_inverse_lerp(start: f64, end: f64, value: f64) -> f64 {
    if end == start {
        return 0.0;
    }
    (value - start) / (end - start)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_remap(
    value: f64,
    in_min: f64,
    in_max: f64,
    out_min: f64,
    out_max: f64,
) -> f64 {
    if in_max == in_min {
        return out_min;
    }
    out_min + (out_max - out_min) * ((value - in_min) / (in_max - in_min))
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_deg_to_rad(degrees: f64) -> f64 {
    degrees * std::f64::consts::PI / 180.0
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_rad_to_deg(radians: f64) -> f64 {
    radians * 180.0 / std::f64::consts::PI
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_lcm(a: u32, b: u32) -> u32 {
    if a == 0 || b == 0 {
        return 0;
    }
    a / tyrus_math_gcd(a, b) * b
}

#[cfg(feature = "os-entropy")]
#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_random_range(min: f64, max: f64) -> f64 {
    if min.partial_cmp(&max) != Some(std::cmp::Ordering::Less) {
        unsafe { crate::tyrus_panic(c"randomRange: min must be less than max".as_ptr()) }
    }
    use rand::RngExt;
    let mut rng = rand::rng();
    rng.random_range(min..max)
}

#[cfg(not(feature = "os-entropy"))]
#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_random_range(min: f64, max: f64) -> f64 {
    use std::sync::atomic::{AtomicU64, Ordering};

    if min.partial_cmp(&max) != Some(std::cmp::Ordering::Less) {
        unsafe { crate::tyrus_panic(c"randomRange: min must be less than max".as_ptr()) }
    }

    static CALL_COUNTER: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);

    let stack_marker: u8 = 0;
    let address_bits = &stack_marker as *const u8 as usize as u64;

    let counter = CALL_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut state =
        counter ^ address_bits ^ 0xD1B5_4A32_D192_ED03u64.wrapping_mul(counter.wrapping_add(1));
    if state == 0 {
        state = 0x9E37_79B9_7F4A_7C15;
    }

    state ^= state << 13;
    state ^= state >> 7;
    state ^= state << 17;

    let uniform = (state as f64) / (u64::MAX as f64);
    min + uniform * (max - min)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_round_to(x: f64, decimals: i32) -> f64 {
    let factor = 10_f64.powi(decimals);
    (x * factor).round() / factor
}

/// Returns the arithmetic mean of the `length` `f64` values at `data`.
/// Returns `0.0` for an empty array.
///
/// # Safety
///
/// `data` must point to `length` valid, initialized `f64` values.
///
/// Takes `(data, length)` rather than a `TyrusArray` by value: a 24-byte
/// aggregate is classified `MEMORY` under the x86-64 System V ABI, and
/// nothing in this codegen backend performs that classification for a
/// struct-by-value call, passing `TyrusArray` directly here previously
/// desynced the caller and callee's view of the struct's fields, reading
/// garbage lengths and segfaulting. The two-scalar form matches every other
/// native call in this codebase that borrows an array's contents (see
/// `emit_bytes_as_ptr_len` in `tyrus-codegen/src/crypto.rs`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_math_mean(data: *const f64, length: usize) -> f64 {
    if length == 0 {
        return 0.0;
    }
    let slice = unsafe { std::slice::from_raw_parts(data, length) };
    let sum: f64 = slice.iter().sum();
    sum / slice.len() as f64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_math_median(data: *const f64, length: usize) -> f64 {
    if length == 0 {
        return 0.0;
    }
    let mut slice = unsafe { std::slice::from_raw_parts(data, length) }.to_vec();
    slice.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = slice.len() / 2;
    if slice.len() % 2 == 0 {
        f64::midpoint(slice[mid - 1], slice[mid])
    } else {
        slice[mid]
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_math_mode(data: *const f64, length: usize) -> f64 {
    if length == 0 {
        return 0.0;
    }
    let slice = unsafe { std::slice::from_raw_parts(data, length) };
    use std::collections::HashMap;
    let mut counts: HashMap<u64, (f64, usize)> = HashMap::new();
    let mut best_value = slice[0];
    let mut best_count = 0;
    for &value in slice {
        let key = value.to_bits();
        let entry = counts.entry(key).or_insert((value, 0));
        entry.1 += 1;
        if entry.1 > best_count || (entry.1 == best_count && value < best_value) {
            best_count = entry.1;
            best_value = entry.0;
        }
    }
    best_value
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_math_variance(data: *const f64, length: usize) -> f64 {
    if length == 0 {
        return 0.0;
    }
    let slice = unsafe { std::slice::from_raw_parts(data, length) };
    let mean = slice.iter().sum::<f64>() / slice.len() as f64;
    slice.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / slice.len() as f64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_math_std_dev(data: *const f64, length: usize) -> f64 {
    unsafe { tyrus_math_variance(data, length) }.sqrt()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_math_percentile(p: f64, data: *const f64, length: usize) -> f64 {
    if length == 0 {
        return 0.0;
    }
    let mut slice = unsafe { std::slice::from_raw_parts(data, length) }.to_vec();
    slice.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let rank = (p / 100.0) * (slice.len() - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    if lower == upper {
        slice[lower]
    } else {
        let frac = rank - lower as f64;
        slice[lower] * (1.0 - frac) + slice[upper] * frac
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_factorial(n: u128) -> u128 {
    let mut result: u128 = 1;
    for i in 2..=n {
        result = result
            .checked_mul(i)
            .unwrap_or_else(|| unsafe { crate::tyrus_panic(c"factorial overflow".as_ptr()) });
    }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_permutation(n: u128, k: u128) -> u128 {
    if k > n {
        return 0;
    }
    let mut result: u128 = 1;
    for i in (n - k + 1)..=n {
        result = result
            .checked_mul(i)
            .unwrap_or_else(|| unsafe { crate::tyrus_panic(c"permutation overflow".as_ptr()) });
    }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_combination(n: u128, k: u128) -> u128 {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut result: u128 = 1;
    for i in 1..=k {
        result = result
            .checked_mul(n - k + i)
            .unwrap_or_else(|| unsafe { crate::tyrus_panic(c"combination overflow".as_ptr()) })
            / i;
    }
    result
}

const SMALL_PRIMES: [u64; 12] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];
const MILLER_RABIN_BASES: [u64; 7] = [2, 325, 9_375, 28_178, 450_775, 9_780_504, 1_795_265_022];

#[inline]
fn mul_mod(a: u64, b: u64, modulus: u64) -> u64 {
    ((a as u128 * b as u128) % modulus as u128) as u64
}

fn pow_mod(mut base: u64, mut exponent: u64, modulus: u64) -> u64 {
    let mut result = 1;
    base %= modulus;
    while exponent != 0 {
        if exponent & 1 != 0 {
            result = mul_mod(result, base, modulus);
        }
        base = mul_mod(base, base, modulus);
        exponent >>= 1;
    }
    result
}

fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    for prime in SMALL_PRIMES {
        if n.is_multiple_of(prime) {
            return n == prime;
        }
    }

    let mut d = n - 1;
    let s = d.trailing_zeros();
    d >>= s;

    'witness: for base in MILLER_RABIN_BASES {
        if base % n == 0 {
            continue;
        }
        let mut x = pow_mod(base, d, n);
        if x == 1 || x == n - 1 {
            continue;
        }
        for _ in 1..s {
            x = mul_mod(x, x, n);
            if x == n - 1 {
                continue 'witness;
            }
        }
        return false;
    }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_is_prime(n: u64) -> bool {
    is_prime(n)
}

#[inline]
fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

#[inline]
fn add_mod(a: u64, b: u64, modulus: u64) -> u64 {
    debug_assert!(a < modulus && b < modulus);
    if a >= modulus - b {
        a - (modulus - b)
    } else {
        a + b
    }
}

#[inline]
fn pollard_step(x: u64, increment: u64, modulus: u64) -> u64 {
    add_mod(mul_mod(x, x, modulus), increment, modulus)
}

fn pollard_rho(n: u64) -> u64 {
    debug_assert!(n > 1 && !n.is_multiple_of(2) && !is_prime(n));
    let mut increment = 1;
    loop {
        let mut tortoise = 2;
        let mut hare = 2;
        let mut factor = 1;
        while factor == 1 {
            tortoise = pollard_step(tortoise, increment, n);
            hare = pollard_step(pollard_step(hare, increment, n), increment, n);
            factor = gcd(tortoise.abs_diff(hare), n);
        }
        if factor != n {
            return factor;
        }
        increment += 1;
    }
}

fn factorize(n: u64, factors: &mut Vec<u64>) {
    if n == 1 {
        return;
    }
    if is_prime(n) {
        factors.push(n);
        return;
    }
    let divisor = pollard_rho(n);
    factorize(divisor, factors);
    factorize(n / divisor, factors);
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_next_prime(n: u64) -> u64 {
    if n < 2 {
        return 2;
    }
    if n < 3 {
        return 3;
    }
    if n < 5 {
        return 5;
    }
    let mut candidate = n
        .checked_add(1)
        .unwrap_or_else(|| unsafe { crate::tyrus_panic(c"nextPrime overflow".as_ptr()) });
    if candidate.is_multiple_of(2) {
        candidate = candidate
            .checked_add(1)
            .unwrap_or_else(|| unsafe { crate::tyrus_panic(c"nextPrime overflow".as_ptr()) });
    }
    while candidate.is_multiple_of(3) || candidate.is_multiple_of(5) {
        candidate = candidate
            .checked_add(2)
            .unwrap_or_else(|| unsafe { crate::tyrus_panic(c"nextPrime overflow".as_ptr()) });
    }
    while !is_prime(candidate) {
        let step = match candidate % 30 {
            1 | 7 | 13 | 19 => 4,
            11 | 17 | 23 | 29 => 2,
            _ => unreachable!("candidate must be coprime to 30"),
        };
        candidate = candidate
            .checked_add(step)
            .unwrap_or_else(|| unsafe { crate::tyrus_panic(c"nextPrime overflow".as_ptr()) });
    }
    candidate
}

/// Writes the prime factors of `n` as a `u64[]` into `*out_array`.
///
/// # Safety
///
/// `out_array` must be either null (in which case this is a no-op) or a
/// valid, writable pointer to a `TyrusArray`.
///
/// Writes through an out-parameter rather than returning `TyrusArray` by
/// value, mirroring `tyrus_process_argv` (`tyrus-runtime/src/process.rs`):
/// a 24-byte aggregate is classified `MEMORY` under the x86-64 System V
/// ABI, and this codegen backend does not perform that classification for
/// a struct-by-value return, returning `TyrusArray` directly previously
/// desynced the caller and callee's view of the struct, corrupting memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_math_prime_factors(n: u64, out_array: *mut TyrusArray) {
    if out_array.is_null() {
        return;
    }
    if n < 2 {
        unsafe {
            (*out_array).data = std::ptr::null_mut();
            (*out_array).length = 0;
            (*out_array).capacity = 0;
        }
        return;
    }
    let mut factors = Vec::new();
    let mut remainder = n;
    for prime in SMALL_PRIMES {
        while remainder.is_multiple_of(prime) {
            factors.push(prime);
            remainder /= prime;
        }
    }
    factorize(remainder, &mut factors);
    factors.sort_unstable();
    let mut data = factors;
    let ptr = data.as_mut_ptr() as *mut u8;
    let len = data.len();
    let cap = data.capacity();
    std::mem::forget(data);
    unsafe {
        (*out_array).data = ptr;
        (*out_array).length = len;
        (*out_array).capacity = cap;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_fibonacci(n: u64) -> u64 {
    if n > 93 {
        unsafe { crate::tyrus_panic(c"fibonacci overflow".as_ptr()) }
    }
    if n == 0 {
        return 0;
    }
    if n == 1 {
        return 1;
    }
    let mut a: u64 = 0;
    let mut b: u64 = 1;
    for _ in 2..=n {
        let c = a.wrapping_add(b);
        a = b;
        b = c;
    }
    b
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_hypot(x: f64, y: f64) -> f64 {
    x.hypot(y)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_copysign(x: f64, y: f64) -> f64 {
    x.copysign(y)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_fmod(x: f64, y: f64) -> f64 {
    x % y
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_log_base(x: f64, base: f64) -> f64 {
    x.log(base)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_powi(x: f64, exp: i32) -> f64 {
    x.powi(exp)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_to_polar(x: f64, y: f64) -> PolarCoords {
    PolarCoords {
        r: x.hypot(y),
        theta: y.atan2(x),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_math_to_cartesian(r: f64, theta: f64) -> CartesianCoords {
    CartesianCoords {
        x: r * theta.cos(),
        y: r * theta.sin(),
    }
}

#[repr(C)]
pub struct PolarCoords {
    pub r: f64,
    pub theta: f64,
}

#[repr(C)]
pub struct CartesianCoords {
    pub x: f64,
    pub y: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_prime_handles_u64_boundary_cases() {
        for prime in [2, 3, 5, 97, 1_000_000_007, 18_446_744_073_709_551_557] {
            assert!(is_prime(prime), "{prime} must be prime");
        }
        for composite in [
            0,
            1,
            4,
            341_550_071_728_321,
            3_825_123_056_546_413_051,
            u64::MAX,
        ] {
            assert!(!is_prime(composite), "{composite} must be composite");
        }
    }

    #[test]
    fn next_prime_skips_non_wheel_candidates() {
        assert_eq!(tyrus_math_next_prime(0), 2);
        assert_eq!(tyrus_math_next_prime(2), 3);
        assert_eq!(tyrus_math_next_prime(3), 5);
        assert_eq!(tyrus_math_next_prime(5), 7);
        assert_eq!(tyrus_math_next_prime(1_000_000_000), 1_000_000_007);
    }

    #[test]
    fn prime_factors_factor_large_semiprimes_in_order() {
        let mut factors = Vec::new();
        factorize(1_000_000_007 * 1_000_000_009, &mut factors);
        factors.sort_unstable();
        assert_eq!(factors, [1_000_000_007, 1_000_000_009]);

        let mut repeated_factors = Vec::new();
        let mut remainder = 2_u64.pow(5) * 3_u64.pow(3) * 1_000_000_007;
        for prime in SMALL_PRIMES {
            while remainder.is_multiple_of(prime) {
                repeated_factors.push(prime);
                remainder /= prime;
            }
        }
        factorize(remainder, &mut repeated_factors);
        repeated_factors.sort_unstable();
        assert_eq!(repeated_factors, [2, 2, 2, 2, 2, 3, 3, 3, 1_000_000_007]);
    }
}
