//! Formal proofs for the string representation.
//!
//! Two of the three fields are derived from the buffer and one is a bit stolen
//! from a length field. Each of those is a place where a wrong answer is silent:
//! a character count that disagrees with the bytes makes `length` lie, and an
//! ownership bit that bleeds into the length makes a buffer be freed that should
//! not be. Examples cannot rule either out; these harnesses can, over the domain
//! they model.
//!
//! The proofs go through `from_bytes`, not through the pointer constructors.
//! That is not a shortcut: the constructors' extra job is finding the NUL, which
//! is `strlen`, a foreign C function the verifier cannot model at all (it
//! reports "call to foreign C function `strlen` is not currently supported").
//! Proving the pure core and testing the pointer boundary separately is what
//! `kani.md` prescribes for exactly this situation, and it draws the line in the
//! right place: `strlen` is the C ABI's business, the field arithmetic is ours.
//!
//! Domains are kept small on purpose. Measured on this project, widening a
//! byte-string domain from four to twelve bytes took a counter proof from
//! seconds to over forty-five minutes without converging, so `kani.md` adopted a
//! ceiling of roughly five minutes per harness. Nine bytes is the smallest
//! domain that still crosses the eight-byte word boundary the character counter
//! works in.

use crate::{MAX_BYTE_LEN, OWNED, TyrusString};

const MAX_MODELED: usize = 9;

#[kani::proof]
#[kani::unwind(11)]
fn a_buffer_reports_the_characters_it_was_built_from() {
    let first: char = kani::any();
    let second: char = kani::any();

    let mut storage = [0u8; 8];
    let head = first.encode_utf8(&mut storage).len();
    let mut tail = [0u8; 4];
    let tail_len = second.encode_utf8(&mut tail).len();
    storage[head..head + tail_len].copy_from_slice(&tail[..tail_len]);
    let bytes = &storage[..head + tail_len];

    let value = TyrusString::from_bytes(bytes).expect("a modeled buffer is within the limit");

    assert_eq!(value.char_len(), 2);
    assert_eq!(value.byte_len(), head + tail_len);
    assert!(value.char_len() <= value.byte_len());
}

#[kani::proof]
#[kani::unwind(11)]
fn the_fields_are_total_on_arbitrary_bytes() {
    let storage: [u8; MAX_MODELED] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= MAX_MODELED);

    let value = TyrusString::from_bytes(&storage[..len]).expect("within the limit");

    assert_eq!(value.byte_len(), len);
    assert!(value.char_len() <= len);
}

/// The ownership bit and the byte length share one field without disturbing
/// each other: a fresh value is not owned, marking it owned leaves the length
/// readable, and the character count is untouched either way.
///
/// This is the invariant that keeps a `.rodata` literal from being freed. If a
/// length could push a bit into `OWNED`, a static string would start reporting
/// itself as freeable, and freeing a literal is a crash rather than a leak.
#[kani::proof]
#[kani::unwind(11)]
fn ownership_and_length_do_not_collide() {
    let storage: [u8; MAX_MODELED] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= MAX_MODELED);
    let bytes = &storage[..len];

    let borrowed = TyrusString::from_bytes(bytes).expect("within the limit");
    assert!(!borrowed.is_owned());
    assert_eq!(borrowed.byte_len(), len);

    let text = std::str::from_utf8(bytes).unwrap_or("");
    let owned = TyrusString::from_str_owned(text).expect("within the limit");
    kani::assume(owned.is_owned());
    assert_eq!(
        owned.byte_len(),
        text.len(),
        "the flag must not corrupt the length"
    );
}

#[kani::proof]
fn the_length_limit_leaves_the_ownership_bit_alone() {
    let len: usize = kani::any();
    kani::assume(len <= MAX_BYTE_LEN);

    assert_eq!(len as u32 & OWNED, 0);
}

/// The word the C ABI packs both lengths into describes the same value the
/// fields do, with the ownership bit landing where the byte length keeps it.
///
/// This is the invariant behind the whole ABI decision: the generated code sees
/// `{ptr, i64}` and recovers `bytes` from the low half and `chars` from the
/// high one. A packing that disagreed with the fields would not fail to compile
/// and would not crash, it would make `length` answer with the wrong number,
/// which is why it is proved rather than eyeballed.
#[kani::proof]
#[kani::unwind(11)]
fn the_packed_word_and_the_fields_describe_the_same_value() {
    let storage: [u8; MAX_MODELED] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= MAX_MODELED);
    let owned: bool = kani::any();

    let value = if owned {
        let text = std::str::from_utf8(&storage[..len]).unwrap_or("");
        TyrusString::from_str_owned(text).expect("within the limit")
    } else {
        TyrusString::from_bytes(&storage[..len]).expect("within the limit")
    };

    let packed = value.packed_lengths();
    assert_eq!(
        (packed & 0xffff_ffff) as u32 & !OWNED,
        value.byte_len() as u32
    );
    assert_eq!((packed >> 32) as u32, value.char_len() as u32);
    assert_eq!(packed & u64::from(OWNED) != 0, value.is_owned());
}

/// A lent string describes the same text and can never free it, whatever the
/// lender's own ownership was.
///
/// This is the invariant the whole FFI boundary rests on. The C ABI cannot
/// express `&`, so "the callee is only reading this" is enforced by the bit
/// being clear rather than by a contract someone has to remember, and a
/// borrowed value that could report ownership would be a double free waiting for
/// the callee to act on it.
#[kani::proof]
#[kani::unwind(11)]
fn a_borrowed_string_describes_the_same_text_and_never_owns_it() {
    let storage: [u8; MAX_MODELED] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= MAX_MODELED);
    let owned: bool = kani::any();

    let lender = if owned {
        let text = std::str::from_utf8(&storage[..len]).unwrap_or("");
        TyrusString::from_str_owned(text).expect("within the limit")
    } else {
        TyrusString::from_bytes(&storage[..len]).expect("within the limit")
    };

    let borrowed = lender.borrow();

    assert!(
        !borrowed.is_owned(),
        "a lent string can never free the buffer"
    );
    assert_eq!(borrowed.byte_len(), lender.byte_len());
    assert_eq!(borrowed.char_len(), lender.char_len());
    assert_eq!(borrowed.as_raw_ptr(), lender.as_raw_ptr());
    assert_eq!(lender.is_owned(), owned);
    assert!(!borrowed.borrow().is_owned());
}

/// Converting to a C string refuses exactly the buffers that hold a NUL, and
/// accepts exactly the ones that do not.
///
/// This is the invariant that replaced "the buffer always ends in NUL". The
/// representation no longer promises a terminator, so the C ABI's restriction
/// has to be enforced at the one place that needs it, and enforced totally,
/// since a buffer wrongly accepted would be handed to C and read past its end.
#[kani::proof]
#[kani::unwind(11)]
fn converting_to_a_c_string_refuses_exactly_the_buffers_holding_a_nul() {
    let storage: [u8; MAX_MODELED] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= MAX_MODELED);
    let bytes = &storage[..len];

    let value = TyrusString::from_bytes(bytes).expect("within the limit");
    let converted = unsafe { value.to_c_string() };

    let holds_a_nul = bytes.iter().any(|byte| *byte == 0);
    assert_eq!(converted.is_err(), holds_a_nul);
    if let Err(error) = converted {
        assert!(error.position() < len);
        assert_eq!(bytes[error.position()], 0);
        assert!(bytes[..error.position()].iter().all(|byte| *byte != 0));
    }
}

#[kani::proof]
#[kani::unwind(6)]
fn a_shared_buffer_is_freed_exactly_once() {
    let shares: usize = kani::any();
    kani::assume(shares <= 4);

    let original = TyrusString::from_str_owned("abcd").expect("within the limit");
    let mut holders = vec![original];
    for _ in 0..shares {
        let next = holders[0].share();
        holders.push(next);
    }
    assert_eq!(holders.len(), shares + 1);

    let mut remaining = holders.len();
    while remaining > 0 {
        let index: usize = kani::any();
        kani::assume(index < remaining);
        let value = holders.swap_remove(index);
        unsafe { value.release() };
        remaining -= 1;
        for held in &holders {
            assert_eq!(held.byte_len(), 4);
            assert_eq!(unsafe { held.as_bytes() }, b"abcd");
        }
    }
}

#[kani::proof]
#[kani::unwind(12)]
fn sharing_preserves_the_text_and_both_lengths() {
    let storage: [u8; MAX_MODELED] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= MAX_MODELED && len > 0);
    let bytes = &storage[..len];
    kani::assume(bytes.iter().all(|byte| *byte < 0x80));
    let text = std::str::from_utf8(bytes).expect("ASCII is UTF-8");

    let original = TyrusString::from_str_owned(text).expect("within the limit");
    let shared = original.share();

    assert!(shared.is_owned());
    assert_eq!(shared.byte_len(), original.byte_len());
    assert_eq!(shared.char_len(), original.char_len());
    assert_eq!(unsafe { shared.as_bytes() }, bytes);
    assert_eq!(shared.as_raw_ptr(), original.as_raw_ptr());

    unsafe { original.release() };
    assert_eq!(unsafe { shared.as_bytes() }, bytes);
    unsafe { shared.release() };
}
