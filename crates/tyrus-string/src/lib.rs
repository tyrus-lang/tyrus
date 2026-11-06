//! The native representation of a `string`.
//!
//! A `string` is a pointer plus a length, the same shape as Rust's `&str`,
//! carrying its byte length and its character count in sixteen bytes. Nothing
//! here is wired into the compiler yet; see
//! `../tyrus-roadmaps/string-representation.md` for the phases that do that.
//!
//! # Why the buffer is not NUL-terminated
//!
//! The first version of this crate required a terminator, so that a `string`
//! could be handed to `printf` and to the C ABI at no cost. That was the wrong
//! trade, and it is worth recording why, because the reasoning generalizes.
//!
//! The saving was real but narrow, file paths for libc calls, and output,
//! while the cost fell on everything:
//!
//! * **A `strlen` per call.** Finding a terminator is a scan of the whole
//!   buffer. Every string function in the runtime paid one just to learn where
//!   its input ended. A stored length removes that scan from all of them.
//! * **Slices could never be free.** Returning `"ell"` as a window into
//!   `"hello"` would need a NUL after the second `l`, in the middle of the
//!   original buffer. So `slice` and `substring` were forced to allocate and
//!   copy. Saving one copy at the C boundary cost a copy on every slice.
//! * **U+0000 could never be held**, though it is an ordinary character
//!   everywhere else: Rust, Python and JavaScript all store it without fuss.
//!
//! What remains is a genuine cost, paid where it belongs: code that needs a
//! `char*` converts, and the conversion is the one operation that can fail on
//! an interior NUL. That is exactly Rust's arrangement, `String` holds
//! anything, `CString::new` refuses, and [`TyrusString::to_c_string`] is this
//! crate's `CString::new`.
//!
//! # Why not the rest of Rust's layout
//!
//! Rust's `String` is twenty-four bytes: pointer, capacity, length.
//!
//! *Capacity* exists so a buffer can grow in place. Tyrus strings are
//! immutable (every operation returns a new string) so nothing grows and
//! there is no capacity to track. Dropping that field is what leaves room for
//! a character count inside sixteen bytes, and sixteen is the System V
//! threshold: at that size the value travels in two registers, and at
//! twenty-four it goes through memory.
//!
//! *A character count* is something Rust does not keep at all, `String::len()`
//! is a byte count, and `chars().count()` stays linear. Tyrus defines `length`
//! as characters, so it is stored, and the immutability that removed capacity
//! is also what makes it cheap: written once at construction, never invalidated.
//!
//! # Ownership
//!
//! The high bit of the byte length says whether the buffer belongs to this
//! value. A static buffer (a literal in `.rodata`) must never be freed and a
//! heap buffer must be freed exactly once, and confusing the two is not a leak
//! but a crash. That bit is the single source of truth for the question.
//!
//! The type is deliberately neither `Copy` nor `Clone`: two values that both
//! believe they own one buffer is the shape of a double free.
//!
//! # How the value crosses the C ABI
//!
//! Sixteen bytes is under the System V threshold, so the value travels in two
//! registers, but *which* two is a frontend decision, and LLVM IR does not make
//! it for you. A declaration written as the literal struct, `@f({ptr, i32,
//! i32})`, puts the three fields in three registers, while a `#[repr(C)]` Rust
//! function expects two: the pointer, then both lengths packed into one word.
//! The verifier accepts either, so the mismatch does not fail to compile or
//! crash, it silently drops [`chars`](Self::char_len) and makes `length` return
//! a wrong number.
//!
//! The generated code therefore describes a string as `{ptr, i64}`, which is
//! what a C frontend emits for this struct and is bit-for-bit the same object:
//! eight bytes of pointer, then the two `u32` fields contiguous. See
//! [`packed_lengths`](Self::packed_lengths) for the one place that view is
//! written down on this side, and the test that pins it.
//!
//! This assumes a little-endian target, where the low half of that word is
//! `bytes` and the high half is `chars`. Both targets this language builds for
//! are little-endian; a big-endian one would swap the halves.

use std::ffi::{CString, c_char};
use std::sync::atomic::{AtomicU32, Ordering};

#[cfg(kani)]
#[path = "kani_proofs/kani_proofs.rs"]
mod kani_proofs;

const OWNED: u32 = 1 << 31;

pub const MAX_BYTE_LEN: usize = (OWNED - 1) as usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InteriorNul {
    position: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroizeError {
    NotOwned,
    Shared,
}

impl InteriorNul {
    pub fn position(&self) -> usize {
        self.position
    }
}

/// A `string` as the generated code sees it.
///
/// `#[repr(C)]` because this is an ABI type: the field order is a contract with
/// the code the compiler emits, not an implementation detail.
#[repr(C)]
#[derive(Debug)]
pub struct TyrusString {
    ptr: *const u8,
    bytes: u32,
    chars: u32,
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut core::ffi::c_void;
    fn free(ptr: *mut core::ffi::c_void);
}

impl TyrusString {
    pub fn from_static(text: &'static str) -> Option<Self> {
        Self::from_bytes(text.as_bytes())
    }

    /// The absent string, which is how a fallible native function reports
    /// failure across the ABI.
    ///
    /// A null pointer, as the old representation used for the same job: the
    /// generated code already tests the pointer it gets back, and keeping that
    /// convention means the failure paths do not have to be redesigned along
    /// with everything else. It is not the empty string, `""` is a real value
    /// with a non-null pointer and no bytes.
    pub const fn null() -> Self {
        Self {
            ptr: std::ptr::null(),
            bytes: 0,
            chars: 0,
        }
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }

    pub fn from_str_owned(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        let (bytes_field, chars_field) = Self::length_fields(bytes)?;
        if bytes.is_empty() {
            return Self::from_static("");
        }
        let buffer = Self::alloc_buffer(bytes.len());
        // SAFETY: `buffer` is a fresh allocation with room for exactly
        // `bytes.len()` bytes after the header, and cannot overlap the source.
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, bytes.len()) };
        Some(Self {
            ptr: buffer,
            bytes: bytes_field | OWNED,
            chars: chars_field,
        })
    }

    /// Releases the buffer if this value owns it, and does nothing otherwise.
    ///
    /// The ownership bit is the whole decision. Freeing a `.rodata` literal is
    /// not a leak but a crash, which is why the bit exists and why nothing here
    /// tries to infer the answer from the pointer.
    ///
    /// # Safety
    ///
    /// The buffer must not have been released already, and nothing else may
    /// hold it. Consuming `self` is what makes that checkable at a glance:
    /// there is no second value left to release.
    pub unsafe fn release(self) {
        if !self.is_owned() {
            return;
        }
        if self.byte_len() == 0 {
            return;
        }
        // SAFETY: ownership was checked, so the buffer carries a count.
        let count = unsafe { self.reference_count() };
        if count.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }
        std::sync::atomic::fence(Ordering::Acquire);
        // SAFETY: the count reached zero, so this is the last reference, and
        // the allocation began `HEADER` bytes before the text.
        unsafe { free(self.ptr.sub(Self::HEADER) as *mut core::ffi::c_void) };
    }

    /// Overwrites an exclusively owned heap buffer with zeroes and releases it.
    ///
    /// The writes are volatile and followed by a compiler fence so an optimizer
    /// cannot remove them as dead stores before the allocation is freed. A
    /// shared buffer is refused: erasing it would mutate text still observable
    /// through another value. Failure returns both the reason and the unchanged
    /// value so the caller can keep using or release its reference.
    ///
    /// # Safety
    ///
    /// The value must be live and must not have been released already.
    pub unsafe fn zeroize_and_release(self) -> Result<(), (ZeroizeError, Self)> {
        if self.byte_len() == 0 {
            return Ok(());
        }
        if !self.is_owned() {
            return Err((ZeroizeError::NotOwned, self));
        }
        // SAFETY: ownership was checked, so the buffer carries a count.
        let count = unsafe { self.reference_count() };
        if count.load(Ordering::Acquire) != 1 {
            return Err((ZeroizeError::Shared, self));
        }
        // SAFETY: a count of one means this value is the only live owner, and
        // its allocation contains exactly `byte_len` writable bytes.
        for index in 0..self.byte_len() {
            unsafe { std::ptr::write_volatile(self.ptr.cast_mut().add(index), 0) };
        }
        std::sync::atomic::compiler_fence(Ordering::SeqCst);
        // SAFETY: this remains the only reference after the erase.
        unsafe { self.release() };
        Ok(())
    }

    pub const HEADER: usize = 8;

    unsafe fn reference_count(&self) -> &AtomicU32 {
        debug_assert!(self.is_owned(), "only an owned buffer carries a count");
        // SAFETY: `from_str_owned` allocated `HEADER + len` and returned the
        // pointer advanced past the header, so this lands on the count.
        unsafe { &*self.ptr.sub(Self::HEADER).cast::<AtomicU32>() }
    }

    pub fn alloc_buffer(bytes: usize) -> *mut u8 {
        if bytes == 0 {
            return std::ptr::null_mut();
        }
        let allocation = unsafe { malloc(Self::HEADER + bytes) }.cast::<u8>();
        if allocation.is_null() {
            std::alloc::handle_alloc_error(
                std::alloc::Layout::from_size_align(Self::HEADER + bytes, Self::HEADER)
                    .expect("a header-aligned layout of a checked length"),
            );
        }
        // SAFETY: the allocation is at least `HEADER` bytes and aligned for it.
        unsafe { allocation.cast::<AtomicU32>().write(AtomicU32::new(1)) };
        // SAFETY: the allocation has room for the header plus `bytes`.
        unsafe { allocation.add(Self::HEADER) }
    }

    /// Drops one reference to the buffer behind `text`, freeing it at zero.
    ///
    /// # Safety
    ///
    /// `text` must be a pointer returned by [`alloc_buffer`](Self::alloc_buffer)
    /// that has not been released already, or null, which is a no-op.
    pub unsafe fn release_buffer(text: *const u8) {
        if text.is_null() {
            return;
        }
        // SAFETY: the pointer came from `alloc_buffer`, so the count sits
        // `HEADER` bytes behind it.
        let count = unsafe { &*text.sub(Self::HEADER).cast::<AtomicU32>() };
        if count.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }
        std::sync::atomic::fence(Ordering::Acquire);
        // SAFETY: the count reached zero, so this is the last reference.
        unsafe { free(text.sub(Self::HEADER) as *mut core::ffi::c_void) };
    }

    pub fn share(&self) -> Self {
        if !self.is_owned() {
            // SAFETY: a non-owned value's buffer is live for this call, which
            // is the contract every caller of the sharing path already meets.
            let text = unsafe { self.as_str() };
            return Self::from_str_owned(text).unwrap_or_else(Self::null);
        }
        // SAFETY: ownership was just checked.
        let count = unsafe { self.reference_count() };
        let previous = count.fetch_add(1, Ordering::Relaxed);
        assert!(
            previous < u32::MAX,
            "reference count overflow on a shared string"
        );
        Self {
            ptr: self.ptr,
            bytes: self.bytes,
            chars: self.chars,
        }
    }

    /// Both length fields as the single word the C ABI packs them into.
    ///
    /// The generated code sees a string as `{ptr, i64}` rather than as three
    /// fields, for the reasons in the module documentation. This is that view,
    /// written on this side so the two can be compared in a test instead of
    /// only in a comment.
    #[inline]
    pub fn packed_lengths(&self) -> u64 {
        u64::from(self.bytes) | (u64::from(self.chars) << 32)
    }

    pub fn literal(text: &'static str) -> Self {
        Self::from_static(text).expect("a literal is far below the length limit")
    }

    /// A value over this string's text that the caller may keep.
    ///
    /// This copied the bytes until the buffer started carrying a reference
    /// count, it was the replacement for `strdup`, which found the end at a
    /// NUL. Now it is [`share`](Self::share) under the name the runtime already
    /// called it by: the promise was never "a private copy", it was "something
    /// I may outlive the source with", and a counted reference keeps that
    /// promise without moving any bytes.
    ///
    /// # Safety
    ///
    /// The buffer must still be live.
    pub unsafe fn clone_owned(&self) -> Option<Self> {
        Some(self.share())
    }

    /// A view over Rust text, for handing a `&str` to a native function.
    ///
    /// The counterpart of [`borrow`](Self::borrow): that one lends a value this
    /// side already has, this one lends text the Rust side owns. Both produce a
    /// value with the ownership bit clear, so neither can free anything, and the
    /// callee's rule stays the same.
    ///
    /// # Safety
    ///
    /// The text must outlive the returned value. Every caller hands it straight
    /// to a call and drops it there, which is what makes that checkable at a
    /// glance.
    pub unsafe fn lend(text: &str) -> Option<Self> {
        Self::from_bytes(text.as_bytes())
    }

    /// The same text, as a value that can never free it.
    ///
    /// This is how a string is lent across the ABI. The C boundary passes
    /// sixteen bytes by value and cannot express `&`, so "the callee is only
    /// reading this" would otherwise be a comment, and a comment is what a
    /// double free is made of.
    ///
    /// Instead the lending is a value with the ownership bit cleared. The
    /// duplicate describes the same buffer and is *incapable* of releasing it,
    /// so the rule the whole FFI runs on needs no exceptions:
    ///
    /// > The callee never releases. If it needs to keep the text, it copies.
    /// > The caller owns, and the caller releases.
    ///
    /// That single rule is what makes a two-hundred-function boundary
    /// auditable: no per-function ownership contract to remember, and no
    /// signature that means one thing here and another there. It is the
    /// distinction Rust draws between `T` and `&T`, moved into the one place the
    /// language cannot enforce it.
    ///
    /// Taking `&self` is deliberate: the lender keeps its value, so the
    /// borrow cannot outlive a move it did not make.
    ///
    /// # `x.borrow().release()` frees nothing
    ///
    /// A borrow has its ownership bit cleared, so releasing one is a no-op by
    /// construction, and it compiles, because [`release`](Self::release) takes
    /// `self` and this type is `Copy`. That combination leaked the text of
    /// every fulfilled `Promise<string>` until it was caught by the leak
    /// corpus. To release a value that lives behind a reference, *take* it:
    ///
    /// ```ignore
    /// std::mem::replace(&mut *slot, TyrusString::null()).release();
    /// ```
    ///
    /// which also leaves the slot in a state where a second release is
    /// harmless.
    #[inline]
    pub fn borrow(&self) -> Self {
        Self {
            ptr: self.ptr,
            bytes: self.bytes & !OWNED,
            chars: self.chars,
        }
    }

    // `from_owned` used to live here: it took any live pointer and marked the
    // value its owner. That became unsound the moment ownership started meaning
    // "holds a reference", the count sits `HEADER` bytes before the text, so a
    // buffer that did not come from `from_str_owned` has no count there, and
    // adopting one would read and later free memory that was never part of it.
    //
    // Owned buffers now have exactly one source on this side, which is what
    // makes the header an invariant rather than a convention. The generated
    // code has its own allocation site and reproduces the same layout; the test
    // that compares the two is what keeps them honest.

    /// A string over a buffer owned by someone else, which this value will
    /// never free.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null and valid for reads of `bytes` bytes holding
    /// valid UTF-8, and the buffer must outlive the returned value.
    pub unsafe fn borrowed(ptr: *const u8, bytes: usize) -> Option<Self> {
        if ptr.is_null() {
            return None;
        }
        Self::from_bytes(unsafe { std::slice::from_raw_parts(ptr, bytes) })
    }

    #[inline]
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let (bytes_field, chars_field) = Self::length_fields(bytes)?;
        Some(Self {
            ptr: bytes.as_ptr(),
            bytes: bytes_field,
            chars: chars_field,
        })
    }

    #[inline]
    fn length_fields(bytes: &[u8]) -> Option<(u32, u32)> {
        if bytes.len() > MAX_BYTE_LEN {
            return None;
        }
        Some((
            bytes.len() as u32,
            tyrus_validators::char_count_bytes(bytes) as u32,
        ))
    }

    #[inline]
    pub fn char_len(&self) -> usize {
        self.chars as usize
    }

    #[inline]
    pub fn byte_len(&self) -> usize {
        (self.bytes & !OWNED) as usize
    }

    #[inline]
    pub fn is_owned(&self) -> bool {
        self.bytes & OWNED != 0
    }

    pub fn as_raw_ptr(&self) -> *const u8 {
        self.ptr
    }

    /// The bytes.
    ///
    /// # Safety
    ///
    /// The buffer must still be live.
    pub unsafe fn as_bytes(&self) -> &[u8] {
        if self.ptr.is_null() {
            return &[];
        }
        unsafe { std::slice::from_raw_parts(self.ptr, self.byte_len()) }
    }

    /// The text as a `&str`.
    ///
    /// # Safety
    ///
    /// The buffer must still be live, and must hold the valid UTF-8 the
    /// constructors require.
    pub unsafe fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(self.as_bytes()) }
    }

    /// A NUL-terminated copy, for handing to a C API.
    ///
    /// This is the boundary the representation pushes the C ABI's restriction
    /// out to, and the only operation that can fail because of a NUL, the same
    /// role `CString::new` plays in Rust. It allocates, because a terminator
    /// cannot be added to a buffer this value does not own the end of.
    ///
    /// # Safety
    ///
    /// The buffer must still be live.
    pub unsafe fn to_c_string(&self) -> Result<CString, InteriorNul> {
        let bytes = unsafe { self.as_bytes() };
        match bytes.iter().position(|byte| *byte == 0) {
            Some(position) => Err(InteriorNul { position }),
            None => Ok(CString::new(bytes).expect("no NUL is present")),
        }
    }
}

impl TyrusString {
    /// A string over a C-owned buffer, given its length.
    ///
    /// # Safety
    ///
    /// As [`borrowed`](Self::borrowed).
    pub unsafe fn borrowed_c(ptr: *const c_char, bytes: usize) -> Option<Self> {
        unsafe { Self::borrowed(ptr.cast::<u8>(), bytes) }
    }

    /// A string over a NUL-terminated buffer, finding the end with `strlen`.
    ///
    /// **A migration shim, not part of the design.** The whole point of storing
    /// a length is to stop scanning for the end, and this scans. It exists so
    /// the representation can be introduced behind the current FFI signatures,
    /// which pass a pointer and no length, and validated against the test
    /// corpus before those signatures change. It goes away when they do.
    ///
    /// A buffer holding an interior NUL is reported as ending there, which is
    /// the same truncation the C ABI imposes today: this shim cannot see more
    /// than the caller gave it a way to describe.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null, NUL-terminated and hold valid UTF-8, and the
    /// buffer must outlive the returned value.
    #[inline]
    pub unsafe fn borrowed_from_c_str(ptr: *const c_char) -> Option<Self> {
        if ptr.is_null() {
            return None;
        }
        let bytes = unsafe { std::ffi::CStr::from_ptr(ptr) }.to_bytes();
        Self::from_bytes(bytes)
    }
}

#[cfg(test)]
#[path = "../tests/unit/string.rs"]
mod tests;
