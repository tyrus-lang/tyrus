use crate::tyrus_panic;

fn out_of_bounds() -> ! {
    unsafe { tyrus_panic(c"tyrus runtime error: BitSet index is out of bounds".as_ptr()) }
}

pub struct TyrusBitSet {
    size: usize,
    words: Vec<u64>,
}

impl TyrusBitSet {
    fn words_for(size: usize) -> usize {
        size.div_ceil(64)
    }

    fn mask_top_word(&mut self) {
        if self.words.is_empty() {
            return;
        }
        let used = self.size % 64;
        if used != 0 {
            let last = self.words.len() - 1;
            self.words[last] &= (1u64 << used) - 1;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_bit_set_new(size: usize) -> *mut TyrusBitSet {
    Box::into_raw(Box::new(TyrusBitSet {
        size,
        words: vec![0u64; TyrusBitSet::words_for(size)],
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_bit_set_from_array(
    data: *const usize,
    length: usize,
) -> *mut TyrusBitSet {
    unsafe {
        let indices = std::slice::from_raw_parts(data, length);
        let size = indices.iter().copied().max().map_or(0, |max| max + 1);
        let mut bitset = Box::new(TyrusBitSet {
            size,
            words: vec![0u64; TyrusBitSet::words_for(size)],
        });
        for &index in indices {
            if index >= bitset.size {
                out_of_bounds();
            }
            bitset.words[index / 64] |= 1u64 << (index % 64);
        }
        Box::into_raw(bitset)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_bit_set_set(bitset: *mut TyrusBitSet, index: usize, value: bool) {
    unsafe {
        let bitset = &mut *bitset;
        if index >= bitset.size {
            out_of_bounds();
        }
        let mask = 1u64 << (index % 64);
        if value {
            bitset.words[index / 64] |= mask;
        } else {
            bitset.words[index / 64] &= !mask;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_bit_set_flip(bitset: *mut TyrusBitSet, index: usize) {
    unsafe {
        let bitset = &mut *bitset;
        if index >= bitset.size {
            out_of_bounds();
        }
        bitset.words[index / 64] ^= 1u64 << (index % 64);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_bit_set_clear(bitset: *mut TyrusBitSet) {
    unsafe {
        let bitset = &mut *bitset;
        bitset.words.fill(0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_bit_set_get(bitset: *const TyrusBitSet, index: usize) -> bool {
    unsafe {
        let bitset = &*bitset;
        if index >= bitset.size {
            out_of_bounds();
        }
        bitset.words[index / 64] & (1u64 << (index % 64)) != 0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_bit_set_count(bitset: *const TyrusBitSet) -> usize {
    unsafe {
        let bitset = &*bitset;
        bitset
            .words
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_bit_set_is_empty(bitset: *const TyrusBitSet) -> bool {
    unsafe {
        let bitset = &*bitset;
        bitset.words.iter().all(|word| *word == 0)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_bit_set_is_subset(
    bitset: *const TyrusBitSet,
    other: *const TyrusBitSet,
) -> bool {
    unsafe {
        let bitset = &*bitset;
        let other = &*other;
        let words = bitset.words.len().max(other.words.len());
        for index in 0..words {
            let a = bitset.words.get(index).copied().unwrap_or(0);
            let b = other.words.get(index).copied().unwrap_or(0);
            if a & !b != 0 {
                return false;
            }
        }
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_bit_set_is_superset(
    bitset: *const TyrusBitSet,
    other: *const TyrusBitSet,
) -> bool {
    unsafe { tyrus_bit_set_is_subset(other, bitset) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_bit_set_union(
    bitset: *const TyrusBitSet,
    other: *const TyrusBitSet,
) -> *mut TyrusBitSet {
    unsafe {
        let bitset = &*bitset;
        let other = &*other;
        let size = bitset.size.max(other.size);
        let mut result = Box::new(TyrusBitSet {
            size,
            words: vec![0u64; TyrusBitSet::words_for(size)],
        });
        for index in 0..result.words.len() {
            let a = bitset.words.get(index).copied().unwrap_or(0);
            let b = other.words.get(index).copied().unwrap_or(0);
            result.words[index] = a | b;
        }
        Box::into_raw(result)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_bit_set_intersection(
    bitset: *const TyrusBitSet,
    other: *const TyrusBitSet,
) -> *mut TyrusBitSet {
    unsafe {
        let bitset = &*bitset;
        let other = &*other;
        let size = bitset.size.max(other.size);
        let mut result = Box::new(TyrusBitSet {
            size,
            words: vec![0u64; TyrusBitSet::words_for(size)],
        });
        for index in 0..result.words.len() {
            let a = bitset.words.get(index).copied().unwrap_or(0);
            let b = other.words.get(index).copied().unwrap_or(0);
            result.words[index] = a & b;
        }
        Box::into_raw(result)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_bit_set_difference(
    bitset: *const TyrusBitSet,
    other: *const TyrusBitSet,
) -> *mut TyrusBitSet {
    unsafe {
        let bitset = &*bitset;
        let other = &*other;
        let size = bitset.size.max(other.size);
        let mut result = Box::new(TyrusBitSet {
            size,
            words: vec![0u64; TyrusBitSet::words_for(size)],
        });
        for index in 0..result.words.len() {
            let a = bitset.words.get(index).copied().unwrap_or(0);
            let b = other.words.get(index).copied().unwrap_or(0);
            result.words[index] = a & !b;
        }
        Box::into_raw(result)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_bit_set_complement(bitset: *const TyrusBitSet) -> *mut TyrusBitSet {
    unsafe {
        let bitset = &*bitset;
        let mut result = Box::new(TyrusBitSet {
            size: bitset.size,
            words: bitset.words.clone(),
        });
        for word in &mut result.words {
            *word = !*word;
        }
        result.mask_top_word();
        Box::into_raw(result)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_bit_set_free(bitset: *mut TyrusBitSet) {
    unsafe {
        if !bitset.is_null() {
            drop(Box::from_raw(bitset));
        }
    }
}
