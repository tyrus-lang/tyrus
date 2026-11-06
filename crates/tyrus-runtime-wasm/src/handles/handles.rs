//! Handles and layouts the sandbox stubs name but never build.
//!
//! Their real definitions live in native modules that cannot compile for wasm,
//! `promise` carries a `Mutex` and a `Condvar`, `executor` an `AtomicU8` and a
//! Tokio runtime, `types::HttpRouter` an axum `Router`. Generated code only ever
//! holds a *pointer* to these, so the sandbox needs the name and not the layout.
//!
//! They deliberately sit here rather than in the shared `abi` module: `abi` is
//! compiled into both crates, and declaring them there would collide with the
//! real definitions on the native side.

use crate::abi::TyrusStringArray;
use std::ffi::c_char;

#[repr(C)]
pub struct TyrusPromise {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct TyrusAsyncTask {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct TyrusAsyncAggregateError {
    _opaque: [u8; 0],
}

pub struct TyrusReadableStream {
    _opaque: [u8; 0],
}

pub struct TyrusWritableStream {
    _opaque: [u8; 0],
}

pub struct TyrusDuplexStream {
    _opaque: [u8; 0],
}

pub struct TyrusTransformStream {
    _opaque: [u8; 0],
}

pub struct TyrusChildProcess {
    _opaque: [u8; 0],
}

/// Option layouts of `tyrus:archive` and `tyrus:compress`: the sandbox stubs
/// abort before reading one, so only the names the native signatures use.
pub struct PackOptionsFfi {
    _opaque: [u8; 0],
}

pub struct UnpackOptionsFfi {
    _opaque: [u8; 0],
}

pub struct CompressOptionsFfi {
    _opaque: [u8; 0],
}

pub struct DecompressOptionsFfi {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct TyrusStreamPair {
    pub readable: *const TyrusReadableStream,
    pub writable: *const TyrusWritableStream,
}

pub struct HttpRouter {
    _opaque: [u8; 0],
}

pub struct TyrusHttpServer {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct HttpRequest {
    pub method: *const c_char,
    pub path: *const c_char,
    pub body: *const c_char,
    pub params: *const c_char,
    pub query: *const c_char,
    pub headers: *const c_char,
    pub cookies: *const c_char,
}

#[repr(C)]
pub struct HttpResponse {
    pub status: i32,
    pub body: *const c_char,
    pub cookies: *const c_char,
}

#[repr(C)]
pub struct CorsOptions {
    pub origin: TyrusStringArray,
    pub methods: TyrusStringArray,
    pub allowed_headers: TyrusStringArray,
    pub exposed_headers: TyrusStringArray,
    pub credentials: bool,
    pub max_age: i32,
}

/// Server settings passed to `tyrus_http_router_listen`.
///
/// Only the pointer type is used: the sandbox has no network, so the stub that
/// receives this aborts without reading a field. The layout is deliberately
/// **not** kept in step with `types::HttpServerOptions`, which is the real one;
/// mirroring it here would be a second copy of an ABI with nothing to keep it
/// honest, and the first field to drift would drift silently.
#[repr(C)]
pub struct HttpServerOptions {
    pub host: tyrus_string::TyrusString,
    pub request_timeout_millis: i64,
    pub max_concurrent_requests: i64,
}

pub type HttpHandler =
    unsafe extern "C" fn(*const HttpRequest, *mut HttpResponse) -> *mut TyrusPromise;

pub type HttpPreHandler = unsafe extern "C" fn(*const HttpRequest, *mut HttpResponse) -> i32;

#[repr(C)]
pub struct WebSocketHandle {
    _private: [u8; 0],
}

pub type WebSocketHandler = unsafe extern "C" fn(*const WebSocketHandle) -> *mut TyrusPromise;
