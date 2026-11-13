use super::*;

fn owned(text: &str) -> TyrusString {
    TyrusString::from_str_owned(text).expect("within the length limit")
}

#[test]
fn reports_both_lengths_without_walking_the_buffer() {
    let value = TyrusString::from_static("héllo").unwrap();
    assert_eq!(value.char_len(), 5);
    assert_eq!(value.byte_len(), 6);
}

#[test]
fn counts_an_astral_character_as_one() {
    let value = TyrusString::from_static("a🚀b").unwrap();
    assert_eq!(value.char_len(), 3);
    assert_eq!(value.byte_len(), 6);
}

#[test]
fn handles_the_empty_string() {
    let value = TyrusString::from_static("").unwrap();
    assert_eq!(value.char_len(), 0);
    assert_eq!(value.byte_len(), 0);
    assert_eq!(unsafe { value.as_str() }, "");
}

#[test]
fn both_lengths_agree_on_ascii() {
    let value = TyrusString::from_static("hello").unwrap();
    assert_eq!(value.char_len(), value.byte_len());
}

#[test]
fn the_ownership_bit_does_not_disturb_the_length() {
    let value = owned("héllo");
    assert!(value.is_owned());
    assert_eq!(value.byte_len(), 6);
    assert_eq!(value.char_len(), 5);

    let borrowed = TyrusString::from_static("héllo").unwrap();
    assert!(!borrowed.is_owned());
    assert_eq!(borrowed.byte_len(), 6);
    assert_eq!(borrowed.char_len(), 5);

    unsafe { value.release() };
}

/// A static buffer must never report itself as freeable: mistaking one for a
/// heap buffer is a crash, not a leak.
#[test]
fn a_static_string_is_never_owned() {
    let value = TyrusString::from_static("literal").unwrap();
    assert!(!value.is_owned());
    assert_eq!(value.char_len(), 7);
}

#[test]
fn zeroize_refuses_static_text() {
    let value = TyrusString::from_static("literal").unwrap();
    let (error, value) = unsafe { value.zeroize_and_release() }.unwrap_err();
    assert_eq!(error, ZeroizeError::NotOwned);
    assert_eq!(unsafe { value.as_str() }, "literal");
}

#[test]
fn zeroize_refuses_a_shared_allocation_without_mutating_it() {
    let value = owned("secret");
    let shared = value.share();
    let (error, value) = unsafe { value.zeroize_and_release() }.unwrap_err();
    assert_eq!(error, ZeroizeError::Shared);
    assert_eq!(unsafe { value.as_str() }, "secret");
    assert_eq!(unsafe { shared.as_str() }, "secret");
    unsafe { shared.release() };
    unsafe { value.release() };
}

#[test]
fn zeroize_erases_and_releases_an_exclusive_allocation() {
    let value = owned("secret");
    let pointer = value.as_raw_ptr();
    let length = value.byte_len();
    assert!(unsafe { value.zeroize_and_release() }.is_ok());
    assert!(!pointer.is_null());
    assert_eq!(length, 6);
}

#[test]
fn holds_a_nul_like_any_other_character() {
    let value = TyrusString::from_static("a\u{0}b").unwrap();
    assert_eq!(value.char_len(), 3);
    assert_eq!(value.byte_len(), 3);
    assert_eq!(unsafe { value.as_str() }, "a\u{0}b");
}

#[test]
fn converts_to_a_c_string_and_refuses_an_interior_nul() {
    let value = TyrusString::from_static("héllo").unwrap();
    let c_string = unsafe { value.to_c_string() }.expect("no NUL is present");
    assert_eq!(c_string.to_bytes(), "héllo".as_bytes());
    assert_eq!(c_string.to_str().unwrap(), unsafe { value.as_str() });

    let with_nul = TyrusString::from_static("a\u{0}b").unwrap();
    let error = unsafe { with_nul.to_c_string() }.expect_err("a NUL must be refused");
    assert_eq!(error.position(), 1);
}

#[test]
fn rejects_a_null_pointer() {
    assert!(unsafe { TyrusString::borrowed(std::ptr::null(), 0) }.is_none());
}

#[test]
fn occupies_exactly_two_registers() {
    assert_eq!(std::mem::size_of::<TyrusString>(), 16);
    assert_eq!(std::mem::align_of::<TyrusString>(), 8);
}

/// The generated code describes a string as `{ptr, i64}`, not as three fields,
/// because that is what the C ABI coerces this struct to. This pins the two
/// views against each other: reading the sixteen bytes as a pointer and one word
/// must give back the same lengths the fields do.
///
/// Getting this wrong does not fail to compile and does not crash, it drops
/// `chars` into a register nobody reads and makes `length` answer with garbage.
#[test]
fn the_lengths_pack_into_the_word_the_abi_passes() {
    let value = TyrusString::from_static("héllo").unwrap();

    let packed = value.packed_lengths();
    assert_eq!(packed & 0xffff_ffff, 6, "the low half is the byte length");
    assert_eq!(packed >> 32, 5, "the high half is the character count");

    let raw = unsafe { std::ptr::read((&value as *const TyrusString).cast::<[u64; 2]>()) };
    assert_eq!(raw[1], packed);
}

#[test]
fn the_packed_word_carries_the_ownership_bit() {
    let value = TyrusString::from_str_owned("hi").expect("within the length limit");
    assert!(value.is_owned());
    assert_eq!(value.packed_lengths() & 0xffff_ffff, 2 | (1 << 31));
    assert_eq!(value.byte_len(), 2);
    unsafe { value.release() };
}

#[test]
fn owns_a_copy_it_can_release() {
    let value = TyrusString::from_str_owned("a\u{0}b🚀").expect("within the length limit");
    assert!(value.is_owned());
    assert_eq!(value.char_len(), 4);
    assert_eq!(value.byte_len(), 7);
    assert_eq!(unsafe { value.as_str() }, "a\u{0}b🚀");
    unsafe { value.release() };
}

#[test]
fn an_empty_string_owns_nothing() {
    let value = TyrusString::from_str_owned("").expect("within the length limit");
    assert!(!value.is_owned());
    assert_eq!(value.byte_len(), 0);
    assert!(!value.is_null(), "empty is a value, absence is not");
    unsafe { value.release() };
}

#[test]
fn releasing_a_static_string_does_nothing() {
    let value = TyrusString::from_static("literal").unwrap();
    unsafe { value.release() };
}

/// Lending is how a string crosses the ABI without transferring ownership. The
/// lent value reads the same text and cannot free it, so the one rule the whole
/// boundary runs on (the callee never releases) needs no exceptions.
#[test]
fn a_lent_string_reads_the_same_text_and_cannot_free_it() {
    let owner = TyrusString::from_str_owned("héllo").expect("within the length limit");
    assert!(owner.is_owned());

    let lent = owner.borrow();
    assert!(
        !lent.is_owned(),
        "a lent string must never claim the buffer"
    );
    assert_eq!(unsafe { lent.as_str() }, "héllo");
    assert_eq!(lent.char_len(), 5);
    assert_eq!(lent.byte_len(), 6);

    unsafe { lent.release() };
    assert_eq!(unsafe { owner.as_str() }, "héllo");

    assert!(owner.is_owned());
    unsafe { owner.release() };
}

#[test]
fn lending_a_static_string_is_the_same_string() {
    let value = TyrusString::from_static("literal").unwrap();
    let lent = value.borrow();
    assert!(!lent.is_owned());
    assert_eq!(unsafe { lent.as_str() }, "literal");
}

#[test]
fn absence_is_null_and_is_not_the_empty_string() {
    let absent = TyrusString::null();
    assert!(absent.is_null());
    assert_eq!(absent.byte_len(), 0);
    assert_eq!(unsafe { absent.as_bytes() }, b"");

    let empty = TyrusString::from_static("").unwrap();
    assert!(!empty.is_null());
}

#[test]
fn sharing_a_string_reads_the_same_text_from_both() {
    let original = owned("héllo, world");
    let shared = original.share();

    assert_eq!(unsafe { shared.as_str() }, "héllo, world");
    assert_eq!(shared.byte_len(), original.byte_len());
    assert_eq!(shared.char_len(), original.char_len());
    assert!(
        shared.is_owned(),
        "a share of an owned buffer owns a reference"
    );
    assert_eq!(
        shared.as_raw_ptr(),
        original.as_raw_ptr(),
        "sharing must not copy the text"
    );

    unsafe { original.release() };
    assert_eq!(unsafe { shared.as_str() }, "héllo, world");
    unsafe { shared.release() };
}

#[test]
fn sharing_a_static_string_copies_it_rather_than_aliasing() {
    let literal = TyrusString::literal("compiled in");
    let shared = literal.share();

    assert!(shared.is_owned(), "the copy owns what it may outlive");
    assert_eq!(unsafe { shared.as_str() }, "compiled in");
    assert_ne!(
        shared.as_raw_ptr(),
        literal.as_raw_ptr(),
        "aliasing a buffer the value does not own is what this must not do"
    );

    unsafe { shared.release() };
    assert_eq!(unsafe { literal.as_str() }, "compiled in");
}

#[test]
fn many_shares_release_in_any_order() {
    let original = owned("shared many times");
    let a = original.share();
    let b = original.share();
    let c = a.share();

    unsafe { original.release() };
    assert_eq!(unsafe { b.as_str() }, "shared many times");
    unsafe { b.release() };
    assert_eq!(unsafe { c.as_str() }, "shared many times");
    unsafe { c.release() };
    assert_eq!(unsafe { a.as_str() }, "shared many times");
    unsafe { a.release() };
}

#[test]
fn sharing_the_empty_string_allocates_nothing() {
    let empty = owned("");
    assert!(!empty.is_owned());
    let shared = empty.share();
    assert_eq!(unsafe { shared.as_str() }, "");
    unsafe { shared.release() };
    unsafe { empty.release() };
}
