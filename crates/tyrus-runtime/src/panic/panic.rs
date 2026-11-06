use std::ffi::{CStr, c_char};
use tyrus_string::TyrusString;

/// Report `message` on standard error (falling back to `fallback` when the
/// pointer is null) and terminate the process with a non-zero status.
///
/// Only the last two lines differ between targets: a WASM instance has no
/// process to exit and no file descriptors, so the text goes to the buffer the
/// playground worker reads and the instance traps. The message itself, which
/// is the part generated code and the tests care about, is produced once.
///
/// # Safety
///
/// `message` must be a valid `TyrusString`, the absent value, or one whose buffer stays live for the duration of the call. The buffer is not NUL-terminated and must not be read as a C string.
pub(crate) unsafe fn abort_with(message: TyrusString, fallback: &str) -> ! {
    let text = if message.is_null() {
        fallback.to_string()
    } else {
        unsafe { message.as_str() }.to_owned()
    };
    #[cfg(not(target_arch = "wasm32"))]
    {
        eprintln!("{text}");
        std::process::exit(1);
    }
    #[cfg(target_arch = "wasm32")]
    {
        // SAFETY: both slices remain live until their writes complete.
        unsafe {
            crate::output::write_stderr(text.as_bytes());
            crate::output::write_stderr(b"\n");
        }
        core::arch::wasm32::unreachable()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_panic(message: *const c_char) -> ! {
    unsafe {
        let lent = if message.is_null() {
            TyrusString::null()
        } else {
            { CStr::from_ptr(message) }
                .to_str()
                .ok()
                .and_then(|text| TyrusString::lend(text))
                .unwrap_or_else(TyrusString::null)
        };
        abort_with(lent, "tyrus runtime error")
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_panic_user(message: TyrusString) -> ! {
    unsafe { abort_with(message, "explicit panic") }
}
