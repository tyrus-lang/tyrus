use std::ffi::CString;

use crate::tyrus_panic;
use tyrus_string::TyrusString;

fn panic_invalid(what: &str) -> ! {
    let message = CString::new(format!("tyrus runtime error: BloomFilter {what}")).unwrap();
    unsafe { tyrus_panic(message.as_ptr()) }
}

fn fnv1a(bytes: &[u8], offset: u64) -> u64 {
    let mut hash = offset;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

pub struct TyrusBloomFilter {
    k: usize,
    num_bits: usize,
    words: Vec<u64>,
}

impl TyrusBloomFilter {
    fn probes(&self, key: &[u8]) -> impl Iterator<Item = usize> + use<> {
        let k = self.k;
        let num_bits = self.num_bits;
        let h1 = fnv1a(key, 0xcbf2_9ce4_8422_2325);
        let h2 = fnv1a(key, 0x9e37_79b9_7f4a_7c15) | 1;
        (0..k).map(move |i| ((h1 as u128 + i as u128 * h2 as u128) % num_bits as u128) as usize)
    }

    fn insert_bytes(&mut self, key: &[u8]) {
        for bit in self.probes(key) {
            self.words[bit / 64] |= 1 << (bit % 64);
        }
    }

    fn contains_bytes(&self, key: &[u8]) -> bool {
        self.probes(key)
            .all(|bit| self.words[bit / 64] & (1 << (bit % 64)) != 0)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_bloom_filter_new(k: usize, m: usize) -> *mut TyrusBloomFilter {
    if k == 0 {
        panic_invalid("requires k > 0 hash functions");
    }
    if m == 0 {
        panic_invalid("requires m > 0 bits");
    }
    let words = vec![0u64; m.div_ceil(64)];
    Box::into_raw(Box::new(TyrusBloomFilter {
        k,
        num_bits: m,
        words,
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_bloom_filter_insert(bf: *mut TyrusBloomFilter, key: TyrusString) {
    unsafe {
        let bf = &mut *bf;
        if key.is_null() {
            panic_invalid("insert key is null");
        }
        let key = key.as_str().as_bytes();
        bf.insert_bytes(key);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_bloom_filter_contains(
    bf: *const TyrusBloomFilter,
    key: TyrusString,
) -> bool {
    unsafe {
        let bf = &*bf;
        if key.is_null() {
            panic_invalid("contains key is null");
        }
        let key = key.as_str().as_bytes();
        bf.contains_bytes(key)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_bloom_filter_free(bf: *mut TyrusBloomFilter) {
    unsafe {
        if !bf.is_null() {
            drop(Box::from_raw(bf));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_filter_reports_false() {
        let bf = TyrusBloomFilter {
            k: 4,
            num_bits: 1 << 16,
            words: vec![0; (1 << 16) / 64],
        };
        assert!(!bf.contains_bytes(b"anything"));
    }

    #[test]
    fn inserted_keys_are_always_found() {
        let bf = unsafe { tyrus_bloom_filter_new(4, 1 << 16) };
        let keys = ["apple", "app", "banana", "strawberry", "tyrus", ""];
        unsafe {
            for key in keys {
                tyrus_bloom_filter_insert(bf, TyrusString::literal(key));
            }
            for key in keys {
                assert!(tyrus_bloom_filter_contains(bf, TyrusString::literal(key)));
            }
            tyrus_bloom_filter_free(bf);
        }
    }

    #[test]
    fn absent_keys_hit_all_zero_bits() {
        let bf = unsafe { tyrus_bloom_filter_new(3, 1 << 20) };
        unsafe {
            for key in ["alpha", "beta", "gamma", "delta"] {
                tyrus_bloom_filter_insert(bf, TyrusString::literal(key));
            }
            for key in ["epsilon", "omega"] {
                assert!(!tyrus_bloom_filter_contains(bf, TyrusString::literal(key)));
            }
            tyrus_bloom_filter_free(bf);
        }
    }

    #[test]
    fn insert_is_idempotent() {
        let bf = unsafe { tyrus_bloom_filter_new(5, 512) };
        unsafe {
            let key = "repeat";
            let other = "absent";
            for _ in 0..10 {
                tyrus_bloom_filter_insert(bf, TyrusString::literal(key));
            }
            assert!(tyrus_bloom_filter_contains(bf, TyrusString::literal(key)));
            assert!(!tyrus_bloom_filter_contains(
                bf,
                TyrusString::literal(other)
            ));
            tyrus_bloom_filter_free(bf);
        }
    }

    #[test]
    fn probes_stay_in_bounds_for_small_filters() {
        let bf = unsafe { tyrus_bloom_filter_new(3, 1) };
        unsafe {
            let key = "x";
            tyrus_bloom_filter_insert(bf, TyrusString::literal(key));
            assert!(tyrus_bloom_filter_contains(bf, TyrusString::literal(key)));
            tyrus_bloom_filter_free(bf);
        }
    }

    #[test]
    fn empty_filter_in_word_math() {
        let bf = unsafe { tyrus_bloom_filter_new(2, 64) };
        unsafe {
            let key = "word";
            tyrus_bloom_filter_insert(bf, TyrusString::literal(key));
            assert!(tyrus_bloom_filter_contains(bf, TyrusString::literal(key)));
            tyrus_bloom_filter_free(bf);
        }
    }
}
