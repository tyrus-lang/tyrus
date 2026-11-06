//! Global allocator that turns out-of-memory into a readable Tyrus runtime
//! error instead of a bare WASM trap.
//!
//! Without this, a failed allocation reaches Rust's `handle_alloc_error`, which
//! aborts with no message, the host only sees `unreachable`, which reads like
//! the sandbox itself broke. Worse, code paths that check the returned pointer
//! for null (`types::realloc` and its C-ABI callers) would silently continue
//! with a null buffer and produce corrupt values rather than stopping.

use core::alloc::{GlobalAlloc, Layout};
use core::fmt::Write;

const OOM_MESSAGE_CAPACITY: usize = 160;

struct FixedBuf {
    buf: [u8; OOM_MESSAGE_CAPACITY],
    len: usize,
}

impl FixedBuf {
    const fn new() -> Self {
        Self {
            buf: [0; OOM_MESSAGE_CAPACITY],
            len: 0,
        }
    }

    fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }
}

impl Write for FixedBuf {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let space = self.buf.len() - self.len;
        let take = s.len().min(space);
        self.buf[self.len..self.len + take].copy_from_slice(&s.as_bytes()[..take]);
        self.len += take;
        Ok(())
    }
}

fn oom_message(size: usize) -> FixedBuf {
    let mut msg = FixedBuf::new();
    let _ = write!(
        msg,
        "out of memory: failed to allocate {size} bytes; the sandbox memory limit was reached"
    );
    msg
}

#[cold]
#[inline(never)]
fn report_oom(size: usize) -> ! {
    let msg = oom_message(size);

    // SAFETY: WASM is single-threaded and the buffer is a static array, this
    // writes no heap memory, which is what makes it usable while OOM.
    unsafe {
        crate::output::write_stderr(msg.as_bytes());
        crate::output::write_stderr(b"\n");
    }

    #[cfg(target_arch = "wasm32")]
    {
        core::arch::wasm32::unreachable()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::process::abort()
    }
}

pub struct OomReporting;

// SAFETY: every method forwards to `dlmalloc::GlobalDlmalloc` with the same
// contract; the only added behavior is diverging (never returning) when the
// inner allocator reports failure.
unsafe impl GlobalAlloc for OomReporting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            let ptr = dlmalloc::GlobalDlmalloc.alloc(layout);
            if ptr.is_null() {
                report_oom(layout.size());
            }
            ptr
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        unsafe {
            let ptr = dlmalloc::GlobalDlmalloc.alloc_zeroed(layout);
            if ptr.is_null() {
                report_oom(layout.size());
            }
            ptr
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        unsafe {
            let new_ptr = dlmalloc::GlobalDlmalloc.realloc(ptr, layout, new_size);
            if new_ptr.is_null() {
                report_oom(new_size);
            }
            new_ptr
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            dlmalloc::GlobalDlmalloc.dealloc(ptr, layout);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_the_failed_allocation_size_and_the_limit() {
        let msg = oom_message(2_097_152);
        let text = std::str::from_utf8(msg.as_bytes()).unwrap();
        assert_eq!(
            text,
            "out of memory: failed to allocate 2097152 bytes; the sandbox memory limit was reached"
        );
    }

    #[test]
    fn the_largest_possible_size_still_fits_the_fixed_buffer() {
        let msg = oom_message(usize::MAX);
        let text = std::str::from_utf8(msg.as_bytes()).unwrap();
        assert!(
            text.ends_with("the sandbox memory limit was reached"),
            "message was truncated: {text}"
        );
    }

    #[test]
    fn writing_past_the_capacity_truncates_instead_of_panicking() {
        let mut buf = FixedBuf::new();
        for _ in 0..10 {
            write!(buf, "{}", "x".repeat(OOM_MESSAGE_CAPACITY)).unwrap();
        }
        assert_eq!(buf.as_bytes().len(), OOM_MESSAGE_CAPACITY);
    }
}
