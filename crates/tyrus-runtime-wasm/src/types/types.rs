//! Allocation helpers used by the WASM-only stubs.
//!
//! The layouts themselves live in [`crate::abi`], shared with the native
//! runtime. Nothing target-specific belongs here beyond what the stubs need to
//! hand a C string back to generated code.

use tyrus_string::TyrusString;

pub(crate) fn text(value: &TyrusString) -> Result<String, ()> {
    if value.is_null() {
        return Err(());
    }
    Ok(unsafe { value.as_str() }.to_owned())
}

pub(crate) fn alloc_value_string(s: &str) -> TyrusString {
    crate::strings::alloc_string(s)
}
