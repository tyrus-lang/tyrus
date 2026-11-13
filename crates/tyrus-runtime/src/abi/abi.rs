//! Layouts and helpers that both runtime targets must agree on, byte for byte.
//!
//! These live apart from the rest of `types` because the WASM runtime shares
//! this file verbatim. Generated code loads and stores at fixed offsets, so a
//! second declaration of `TyrusArray` in the other crate is a silent ABI break
//! waiting to happen, there is only one now.

use std::ffi::{c_char, c_void};
use std::mem::ManuallyDrop;
use tyrus_string::TyrusString;

/// A `string[]` as it crosses the ABI: a borrowed array of `string` values plus
/// its bounds.
///
/// The elements are the sixteen-byte values themselves, not pointers to them,
/// a `string` carries its buffer pointer and its lengths inline, so there is no
/// separate object to point at.
#[repr(C)]
pub struct TyrusStringArray {
    pub data: *const TyrusString,
    pub length: usize,
    pub capacity: usize,
}

#[repr(C)]
pub struct TyrusArray {
    pub data: *mut u8,
    pub length: usize,
    pub capacity: usize,
}

#[repr(C)]
pub struct UserInfo {
    pub username: TyrusString,
    pub homedir: TyrusString,
    pub shell: TyrusString,
}

#[repr(C)]
pub struct OsErrorTaken {
    pub code: TyrusString,
    pub message: TyrusString,
}

#[repr(C)]
#[derive(Default)]
pub struct DbExecTaken {
    pub rows_affected: u64,
    pub last_insert_rowid: i64,
}

#[repr(C)]
pub struct DbErrorTaken {
    pub code: TyrusString,
    pub message: TyrusString,
}

#[repr(C)]
pub struct HttpErrorTaken {
    pub code: TyrusString,
    pub message: TyrusString,
}

#[repr(C)]
pub struct FsErrorTaken {
    pub code: TyrusString,
    pub message: TyrusString,
}

#[repr(C)]
pub struct ProcessErrorTaken {
    pub code: TyrusString,
    pub message: TyrusString,
}

#[repr(C)]
pub struct ProcessErrorFfi {
    pub code: TyrusString,
    pub message: TyrusString,
}

#[repr(C)]
pub union ProcessExecPayload {
    pub child: ManuallyDrop<*const c_void>,
    pub error: ManuallyDrop<ProcessErrorFfi>,
    pub words: [u64; 4],
}

#[repr(C)]
pub struct ProcessExecResultFfi {
    pub tag: u8,
    pub payload: ProcessExecPayload,
}

const _: () = {
    assert!(std::mem::size_of::<ProcessErrorFfi>() == 2 * std::mem::size_of::<TyrusString>());
    assert!(std::mem::size_of::<ProcessExecResultFfi>() == 40);
    assert!(std::mem::offset_of!(ProcessExecResultFfi, payload) == 8);
};

#[repr(C)]
pub struct NotifyOptionsFfi {
    pub icon: *mut c_char,
    pub urgency: *mut c_char,
    pub timeout: *mut c_char,
}

#[repr(C)]
pub struct ColorFfi {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[repr(C)]
pub struct MemoryUsage {
    pub rss: u64,
}

#[repr(C)]
pub struct CpuUsage {
    pub user: u64,
    pub system: u64,
}

pub type TyrusPromiseDrop = Option<unsafe extern "C" fn(*mut c_void)>;

pub type TyrusPromiseCancelHook = unsafe extern "C" fn(*mut c_void);

pub type TyrusAsyncPoll =
    unsafe extern "C" fn(*mut c_void, *mut crate::TyrusPromise, *mut crate::TyrusAsyncTask) -> i32;

pub type TyrusAsyncDropFrame = Option<unsafe extern "C" fn(*mut c_void)>;
