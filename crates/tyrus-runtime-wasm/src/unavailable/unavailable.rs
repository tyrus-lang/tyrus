#![allow(
    unused_variables,
    reason = "a stub takes the native parameters and uses none of them"
)]

use crate::abi::*;
use tyrus_string::TyrusString;

use crate::handles::*;
use crate::panic::tyrus_panic;
use crate::sandbox::{FS_ERROR, OS_ERROR, unavailable};
use crate::stubs::BoxedPayload;
use std::ffi::{c_char, c_void};
use std::mem::ManuallyDrop;

#[repr(C)]
struct OsErrorResult {
    code: TyrusString,
    message: TyrusString,
}

#[repr(C)]
union OsVoidResultPayload {
    error: ManuallyDrop<OsErrorResult>,
    words: [u64; 4],
}

#[repr(C)]
struct OsVoidResult {
    tag: u8,
    payload: OsVoidResultPayload,
}

#[repr(C)]
union OsClipboardReadResultPayload {
    value: *mut c_void,
    error: ManuallyDrop<OsErrorTaken>,
    words: [u64; 4],
}

#[repr(C)]
pub struct OsClipboardReadResult {
    tag: u8,
    payload: OsClipboardReadResultPayload,
}

#[repr(C)]
struct FsFileStats {
    size: u64,
    is_file: bool,
    is_dir: bool,
    is_symlink: bool,
    modified: u64,
    created: u64,
    permissions: u32,
}

#[repr(C)]
union FsFileStatsResultPayload {
    error: ManuallyDrop<FsErrorTaken>,
    stats: ManuallyDrop<FsFileStats>,
    words: [u64; 5],
}

#[repr(C)]
pub struct FsFileStatsResult {
    tag: u8,
    payload: FsFileStatsResultPayload,
}

const _: () = {
    assert!(std::mem::size_of::<OsErrorResult>() == 2 * std::mem::size_of::<TyrusString>());
    assert!(std::mem::size_of::<OsVoidResult>() == 40);
    assert!(std::mem::offset_of!(OsVoidResult, payload) == 8);
    assert!(std::mem::size_of::<OsClipboardReadResult>() == 40);
    assert!(std::mem::offset_of!(OsClipboardReadResult, payload) == 8);
    assert!(std::mem::size_of::<FsFileStats>() == 40);
    assert!(std::mem::size_of::<FsFileStatsResult>() == 48);
    assert!(std::mem::offset_of!(FsFileStatsResult, payload) == 8);
};

unsafe fn finish_os_void_result(out: *mut u8) {
    if out.is_null() {
        return;
    }
    let mut error = OsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_os_take_error(&mut error) };
    let value = OsVoidResult {
        tag: 1,
        payload: OsVoidResultPayload {
            error: ManuallyDrop::new(OsErrorResult {
                code: if error.code.is_null() {
                    TyrusString::literal("ERR_OS_UNAVAILABLE")
                } else {
                    error.code
                },
                message: if error.message.is_null() {
                    TyrusString::literal("operation is unavailable in this runtime")
                } else {
                    error.message
                },
            }),
        },
    };
    unsafe { std::ptr::write(out.cast(), value) };
}

unsafe fn stream_unavailable() -> ! {
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `tyrus:stream` is not available in the playground sandbox"
                .as_ptr(),
        )
    }
}

unsafe fn process_unavailable() -> ! {
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `tyrus:process` is not available in the playground sandbox"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_tcp_listener_retain(handle: *mut c_void) -> *mut c_void {
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `tyrus:net` is not available in the playground sandbox".as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_tcp_stream_retain(handle: *mut c_void) -> *mut c_void {
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `tyrus:net` is not available in the playground sandbox".as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_process_exec(
    _command: TyrusString,
    _args: *const TyrusStringArray,
) -> *const TyrusChildProcess {
    unsafe { process_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_process_exec_result(
    _command: TyrusString,
    _args: *const TyrusStringArray,
    _out: *mut ProcessExecResultFfi,
) {
    unsafe { process_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_process_take_error(out: *mut ProcessErrorTaken) {
    if !out.is_null() {
        unsafe {
            *out = ProcessErrorTaken {
                code: TyrusString::null(),
                message: TyrusString::null(),
            }
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_child_process_retain(
    child: *const TyrusChildProcess,
) -> *const TyrusChildProcess {
    unsafe { process_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_child_process_free(child: *const TyrusChildProcess) {
    unsafe { process_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_child_process_pid(child: *const TyrusChildProcess) -> u32 {
    unsafe { process_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_child_process_stdin(
    child: *const TyrusChildProcess,
) -> *const TyrusWritableStream {
    unsafe { process_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_child_process_stdout(
    child: *const TyrusChildProcess,
) -> *const TyrusReadableStream {
    unsafe { process_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_child_process_stderr(
    child: *const TyrusChildProcess,
) -> *const TyrusReadableStream {
    unsafe { process_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_child_process_wait(
    child: *const TyrusChildProcess,
) -> *mut TyrusPromise {
    unsafe { process_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_child_process_kill(
    child: *const TyrusChildProcess,
) -> *mut TyrusPromise {
    unsafe { process_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_stream_new(capacity_bytes: usize, out: *mut TyrusStreamPair) {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_readable_stream_retain(
    stream: *const TyrusReadableStream,
) -> *const TyrusReadableStream {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_readable_stream_free(stream: *const TyrusReadableStream) {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_writable_stream_retain(
    stream: *const TyrusWritableStream,
) -> *const TyrusWritableStream {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_writable_stream_free(stream: *const TyrusWritableStream) {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_readable_stream_read(
    stream: *const TyrusReadableStream,
) -> *mut TyrusPromise {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_writable_stream_write(
    stream: *const TyrusWritableStream,
    value: *mut c_void,
    bytes: usize,
    drop_value: TyrusPromiseDrop,
) -> *mut TyrusPromise {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_readable_stream_close(
    stream: *const TyrusReadableStream,
) -> *mut TyrusPromise {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_writable_stream_close(
    stream: *const TyrusWritableStream,
) -> *mut TyrusPromise {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_readable_stream_pipe_to(
    source: *const TyrusReadableStream,
    destination: *const TyrusWritableStream,
) -> *mut TyrusPromise {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_duplex_stream_new(
    readable: *const TyrusReadableStream,
    writable: *const TyrusWritableStream,
) -> *const TyrusDuplexStream {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_duplex_stream_retain(
    stream: *const TyrusDuplexStream,
) -> *const TyrusDuplexStream {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_duplex_stream_free(stream: *const TyrusDuplexStream) {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_duplex_stream_readable(
    stream: *const TyrusDuplexStream,
) -> *const TyrusReadableStream {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_duplex_stream_writable(
    stream: *const TyrusDuplexStream,
) -> *const TyrusWritableStream {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_duplex_stream_close(
    stream: *const TyrusDuplexStream,
) -> *mut TyrusPromise {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_transform_stream_new(
    readable: *const TyrusReadableStream,
    writable: *const TyrusWritableStream,
) -> *const TyrusTransformStream {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_transform_stream_retain(
    stream: *const TyrusTransformStream,
) -> *const TyrusTransformStream {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_transform_stream_free(stream: *const TyrusTransformStream) {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_transform_stream_readable(
    stream: *const TyrusTransformStream,
) -> *const TyrusReadableStream {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_transform_stream_writable(
    stream: *const TyrusTransformStream,
) -> *const TyrusWritableStream {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_transform_stream_close(
    stream: *const TyrusTransformStream,
) -> *mut TyrusPromise {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_async_block_on(promise: *mut TyrusPromise) -> i32 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `blockOn` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_async_task_free(task: *mut TyrusAsyncTask) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `taskFree` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_async_task_new(
    poll: TyrusAsyncPoll,
    drop_frame: TyrusAsyncDropFrame,
    frame: *mut c_void,
    result: *mut TyrusPromise,
) -> *mut TyrusAsyncTask {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `taskNew` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_async_task_await(
    task: *mut TyrusAsyncTask,
    dependency: *mut TyrusPromise,
) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `taskAwait` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_async_task_schedule(task: *mut TyrusAsyncTask) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `taskSchedule` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_console_stderr_color_enabled() -> bool {
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_rand_random_bytes(length: u32, output: *mut TyrusArray) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `randRandomBytes` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_crypto_rand_random_int(min: i64, max: i64) -> i64 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `randRandomInt` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_timing_safe_equal(
    a_ptr: *const u8,
    a_len: usize,
    b_ptr: *const u8,
    b_len: usize,
) -> bool {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `timingSafeEqual` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_drop_async_aggregate_error(
    aggregate: *mut TyrusAsyncAggregateError,
) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `asyncAggregateError` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_env_get_bool(_name: TyrusString) -> bool {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `getBool` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_env_get_f64(_name: TyrusString) -> f64 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `getF64` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_env_get_i64(_name: TyrusString) -> i64 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `getI64` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_env_get_u64(_name: TyrusString) -> u64 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `getU64` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_env_require(
    _name: TyrusString,
    _type_code: u32,
    _message: TyrusString,
) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `require` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_figlet(_text: TyrusString, _font: TyrusString) -> TyrusString {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `figlet` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_take_error(out: *mut FsErrorTaken) {
    unsafe {
        if out.is_null() {
            return;
        }
        FS_ERROR.take_into(
            std::ptr::addr_of_mut!((*out).code),
            std::ptr::addr_of_mut!((*out).message),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_stat_result(_path: TyrusString, out: *mut FsFileStatsResult) {
    unavailable!(FS_ERROR, "ERR_FS_UNAVAILABLE", "stat");
    if out.is_null() {
        return;
    }
    let mut error = FsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_fs_take_error(&mut error) };
    let result = FsFileStatsResult {
        tag: 1,
        payload: FsFileStatsResultPayload {
            error: ManuallyDrop::new(FsErrorTaken {
                code: if error.code.is_null() {
                    TyrusString::literal("ERR_FS_UNAVAILABLE")
                } else {
                    error.code
                },
                message: if error.message.is_null() {
                    TyrusString::literal("stat is unavailable in this runtime")
                } else {
                    error.message
                },
            }),
        },
    };
    unsafe { std::ptr::write(out.cast(), result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_read_file_stream(
    _path: TyrusString,
) -> *const TyrusReadableStream {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_write_file_stream(
    _path: TyrusString,
) -> *const TyrusWritableStream {
    unsafe { stream_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_own_response_body(body: *const c_char) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `ownResponseBody` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_reply_clear_cookie(
    _response: *mut HttpResponse,
    _name: TyrusString,
) -> i32 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `replyClearCookie` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[repr(C)]
pub struct DocsOptions {
    _opaque: [u8; 0],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_reply_clear_cookies(_response: *mut HttpResponse) -> i32 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `replyClearCookies` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_reply_set_cookies(
    _response: *mut HttpResponse,
    _cookies: *const TyrusArray,
) -> i32 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `replySetCookies` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_reply_set_cookie(
    _response: *mut HttpResponse,
    _name: TyrusString,
    _value: TyrusString,
) -> i32 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `replySetCookie` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_router_trusted_proxies(
    router: *mut HttpRouter,
    proxies: *const TyrusStringArray,
    identity_header: *const TyrusString,
) -> i32 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `trustedProxies` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_router_cors(
    router: *mut HttpRouter,
    options: *const CorsOptions,
) -> i32 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `cors` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_router_fallback(
    router: *mut HttpRouter,
    handler: Option<HttpHandler>,
    pre_handlers: *const HttpPreHandler,
    pre_handlers_len: usize,
) -> i32 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `fallback` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_router_free(router: *mut HttpRouter) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `free` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_router_listen(
    router: *mut HttpRouter,
    port: i32,
    options: *const HttpServerOptions,
) -> *const TyrusHttpServer {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `listen` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_http_router_new() -> *mut HttpRouter {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `new` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_set_validation_error(
    _response: *mut HttpResponse,
    _status: i32,
    _message: TyrusString,
) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `setValidationError` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_ping(
    _address: TyrusString,
    _timeout_ms: *const u64,
) -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `ping` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_archive_pack(
    _source: TyrusString,
    _destination: TyrusString,
    _options: *const PackOptionsFfi,
) -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `pack` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_archive_list(
    _archive: TyrusString,
    _options: *const UnpackOptionsFfi,
) -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `list` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_archive_unpack(
    _archive: TyrusString,
    _destination: TyrusString,
    _options: *const UnpackOptionsFfi,
) -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `unpack` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_compress_compress(
    _data: *const u8,
    _length: usize,
    _format: i32,
    _options: *const CompressOptionsFfi,
) -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `compress` is not available in WebAssembly yet, which has no background thread pool to run it on"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_compress_decompress(
    _data: *const u8,
    _length: usize,
    _options: *const DecompressOptionsFfi,
) -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `decompress` is not available in WebAssembly yet, which has no background thread pool to run it on"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_compress_compression_stream(
    _format: i32,
    _options: *const CompressOptionsFfi,
) -> *const TyrusTransformStream {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `compressionStream` is not available in WebAssembly yet, which has no background thread pool to run it on"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_compress_decompression_stream(
    _options: *const DecompressOptionsFfi,
) -> *const TyrusTransformStream {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `decompressionStream` is not available in WebAssembly yet, which has no background thread pool to run it on"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_archive_pack_stream(
    _source: TyrusString,
    _format: i32,
    _options: *const PackOptionsFfi,
) -> *const TyrusReadableStream {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `packStream` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_archive_unpack_stream(
    _archive: *const TyrusReadableStream,
    _destination: TyrusString,
    _options: *const UnpackOptionsFfi,
) -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `unpackStream` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_resolve(
    _host: TyrusString,
    _timeout_ms: *const u64,
) -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `resolve` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_reverse_lookup(
    _address: TyrusString,
    _timeout_ms: *const u64,
) -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `reverseLookup` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_net_supports_icmp() -> bool {
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_traceroute(
    _address: TyrusString,
    _timeout_ms: *const u64,
) -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `traceroute` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_accessible_color_contrast(
    fg: *const ColorFfi,
    bg: *const ColorFfi,
) -> bool {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "accessibleColorContrast");
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_allow_sleep() -> bool {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "allowSleep");
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_allow_sleep_result(out: *mut u8) {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "allowSleep");
    unsafe { finish_os_void_result(out) };
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_arch() -> i32 {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "arch");
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_clipboard_write(_text: TyrusString) {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "clipboardWrite");
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_clipboard_read_result(out: *mut OsClipboardReadResult) {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "clipboardRead");
    if out.is_null() {
        return;
    }
    let mut error = OsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_os_take_error(&mut error) };
    let result = OsClipboardReadResult {
        tag: 1,
        payload: OsClipboardReadResultPayload {
            error: ManuallyDrop::new(OsErrorTaken {
                code: if error.code.is_null() {
                    TyrusString::literal("ERR_OS_UNAVAILABLE")
                } else {
                    error.code
                },
                message: if error.message.is_null() {
                    TyrusString::literal("clipboardRead is unavailable in this runtime")
                } else {
                    error.message
                },
            }),
        },
    };
    unsafe { std::ptr::write(out.cast(), result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_cpu_temp(_unit: TyrusString) -> BoxedPayload {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "cpuTemp");
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_cpus(out_array: *mut TyrusArray) {
    unsafe {
        unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "cpus");
        if let Some(out) = out_array.as_mut() {
            out.data = std::ptr::null_mut();
            out.length = 0;
            out.capacity = 0;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_has_capability(capability: i32) -> bool {
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_is_dark_mode() -> bool {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "isDarkMode");
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_is_screen_locked() -> bool {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "isScreenLocked");
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_is_session_locked() -> bool {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "isSessionLocked");
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_load_avg(out_array: *mut TyrusArray) {
    unsafe {
        unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "loadAvg");
        if let Some(out) = out_array.as_mut() {
            out.data = std::ptr::null_mut();
            out.length = 0;
            out.capacity = 0;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_network_interfaces(out_array: *mut TyrusArray) {
    unsafe {
        unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "networkInterfaces");
        if let Some(out) = out_array.as_mut() {
            out.data = std::ptr::null_mut();
            out.length = 0;
            out.capacity = 0;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_notify(
    _title: TyrusString,
    _body: TyrusString,
    _options: *const NotifyOptionsFfi,
) {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "notify");
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_open_url(_url: TyrusString) -> bool {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "openUrl");
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_open_url_result(_url: TyrusString, out: *mut u8) {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "openUrl");
    unsafe { finish_os_void_result(out) };
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_platform() -> i32 {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "platform");
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_prevent_sleep() -> bool {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "preventSleep");
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_prevent_sleep_result(out: *mut u8) {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "preventSleep");
    unsafe { finish_os_void_result(out) };
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_request_attention(critical: bool) {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "requestAttention");
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_reveal_in_folder(_path: TyrusString) -> bool {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "revealInFolder");
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_reveal_in_folder_result(_path: TyrusString, out: *mut u8) {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "revealInFolder");
    unsafe { finish_os_void_result(out) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_system_fonts(out_array: *mut TyrusArray) {
    unsafe {
        unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "systemFonts");
        if let Some(out) = out_array.as_mut() {
            out.data = std::ptr::null_mut();
            out.length = 0;
            out.capacity = 0;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_take_error(out: *mut OsErrorTaken) {
    unsafe {
        if out.is_null() {
            return;
        }
        OS_ERROR.take_into(
            std::ptr::addr_of_mut!((*out).code),
            std::ptr::addr_of_mut!((*out).message),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_trash(_path: TyrusString) -> bool {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "trash");
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_trash_result(_path: TyrusString, out: *mut u8) {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "trash");
    unsafe { finish_os_void_result(out) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_user_info(out: *mut UserInfo) {
    unavailable!(OS_ERROR, "ERR_OS_UNAVAILABLE", "userInfo");
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_process_argv(out_array: *mut TyrusArray) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `argv` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_process_cpu_usage(out: *mut CpuUsage) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `cpuUsage` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_process_memory_usage(out: *mut MemoryUsage) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `memoryUsage` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_process_pid() -> u32 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `pid` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_process_ppid() -> u32 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `ppid` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_process_uptime() -> u64 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `uptime` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_aggregate_error_at(
    _promise: *const TyrusPromise,
    _index: usize,
) -> TyrusString {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `aggregateErrorAt` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_aggregate_error_count(
    promise: *const TyrusPromise,
) -> usize {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `aggregateErrorCount` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_clone_aggregate_error(
    promise: *const TyrusPromise,
) -> *mut TyrusAsyncAggregateError {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `cloneAggregateError` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_attach_thrown(
    _promise: *mut TyrusPromise,
    _tag: u64,
    _data: *mut c_void,
    _drop_data: TyrusPromiseDrop,
) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `attachThrown` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_error_thrown_tag(_promise: *const TyrusPromise) -> u64 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `errorThrownTag` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_error_thrown_data(
    _promise: *const TyrusPromise,
) -> *const c_void {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `errorThrownData` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_error_code(_promise: *const TyrusPromise) -> TyrusString {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `errorCode` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_error_message(_promise: *const TyrusPromise) -> TyrusString {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `errorMessage` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_free(promise: *mut TyrusPromise) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `free` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_all(
    _inputs: *const *mut TyrusPromise,
    _len: usize,
) -> *mut TyrusPromise {
    unsafe { promise_combinators_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_race(
    _inputs: *const *mut TyrusPromise,
    _len: usize,
) -> *mut TyrusPromise {
    unsafe { promise_combinators_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_any(
    _inputs: *const *mut TyrusPromise,
    _len: usize,
) -> *mut TyrusPromise {
    unsafe { promise_combinators_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_all_settled(
    _inputs: *const *mut TyrusPromise,
    _len: usize,
) -> *mut TyrusPromise {
    unsafe { promise_combinators_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_all_typed(
    _inputs: *const *mut TyrusPromise,
    _len: usize,
    _element_size: usize,
    _drop_array: TyrusPromiseDrop,
) -> *mut TyrusPromise {
    unsafe { promise_combinators_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_all_settled_typed(
    _inputs: *const *mut TyrusPromise,
    _len: usize,
    _element_size: usize,
    _result_size: usize,
    _payload_offset: usize,
    _fulfilled_value_offset: usize,
    _rejected_reason_offset: usize,
    _rejected_code_offset: usize,
    _drop_array: TyrusPromiseDrop,
) -> *mut TyrusPromise {
    unsafe { promise_combinators_unavailable() }
}

unsafe fn promise_combinators_unavailable() -> ! {
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: Promise combinators are not available in the playground sandbox"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_fulfill(
    promise: *mut TyrusPromise,
    value: *mut c_void,
    drop_value: TyrusPromiseDrop,
) -> i32 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `fulfill` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_fulfill_owned(
    promise: *mut TyrusPromise,
    value: *mut c_void,
    drop_value: TyrusPromiseDrop,
) -> i32 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `fulfillOwned` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_promise_pending() -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `pending` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_propagate_failure(
    source: *const TyrusPromise,
    target: *mut TyrusPromise,
) -> i32 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `propagateFailure` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_reject(_message: TyrusString) -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `reject` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_reject_pending(
    _promise: *mut TyrusPromise,
    _message: TyrusString,
) -> i32 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `rejectPending` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_promise_resolve(
    value: *mut c_void,
    drop_value: TyrusPromiseDrop,
) -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `resolve` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_promise_resolve_void() -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `resolveVoid` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_retain(promise: *mut TyrusPromise) -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `retain` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_state(promise: *const TyrusPromise) -> i32 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `state` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_cancel_requested(promise: *const TyrusPromise) -> i32 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `cancel` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_request_cancel(
    promise: *mut TyrusPromise,
    reason: TyrusString,
) -> i32 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `cancel` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_on_cancel_request(
    promise: *mut TyrusPromise,
    hook: TyrusPromiseCancelHook,
    context: *mut c_void,
    drop_context: TyrusPromiseDrop,
) -> i32 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `cancel` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_take_value(promise: *mut TyrusPromise) -> *mut c_void {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `takeValue` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_value_ptr(promise: *const TyrusPromise) -> *mut c_void {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `valuePtr` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_stdout_is_terminal() -> bool {
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_style(
    _open: TyrusString,
    _text: TyrusString,
    _close: TyrusString,
) -> TyrusString {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `style` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_task_cancel(
    _promise: *mut TyrusPromise,
    _reason: TyrusString,
    _grace_millis: u64,
) -> i32 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `cancel` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_task_sleep_async(milliseconds: u64) -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `sleep` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_sleep(_milliseconds: u64) {
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `sleep` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        );
    }
}

pub struct TyrusChannelSender {
    _private: (),
}
pub struct TyrusChannelReceiver {
    _private: (),
}
#[repr(C)]
pub struct TyrusChannelPair {
    pub sender: *const TyrusChannelSender,
    pub receiver: *const TyrusChannelReceiver,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_channel_new(_capacity: *const usize, _out: *mut TyrusChannelPair) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `channel` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_channel_sender_retain(
    _sender: *const TyrusChannelSender,
) -> *const TyrusChannelSender {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `Sender` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_channel_sender_free(_sender: *const TyrusChannelSender) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `Sender` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_channel_receiver_retain(
    _receiver: *const TyrusChannelReceiver,
) -> *const TyrusChannelReceiver {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `Receiver` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_channel_receiver_free(_receiver: *const TyrusChannelReceiver) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `Receiver` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_channel_send(
    _sender: *const TyrusChannelSender,
    _value: *mut c_void,
    _drop_value: TyrusPromiseDrop,
) -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `send` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_channel_try_send(
    _sender: *const TyrusChannelSender,
    _value: *mut c_void,
    _drop_value: TyrusPromiseDrop,
) -> i32 {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `trySend` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_channel_recv(
    _receiver: *const TyrusChannelReceiver,
) -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `recv` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_channel_try_recv(
    _receiver: *const TyrusChannelReceiver,
) -> *mut c_void {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `tryRecv` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_channel_close(_receiver: *const TyrusChannelReceiver) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `close` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

pub struct TyrusMutex {
    _private: (),
}
pub struct TyrusMutexGuard {
    _private: (),
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_mutex_new(
    _value: *mut c_void,
    _drop_value: TyrusPromiseDrop,
) -> *const TyrusMutex {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `Mutex` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_mutex_retain(_mutex: *const TyrusMutex) -> *const TyrusMutex {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `Mutex` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_mutex_free(_mutex: *const TyrusMutex) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `Mutex` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_mutex_lock(_mutex: *const TyrusMutex) -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `lock` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_mutex_try_lock(_mutex: *const TyrusMutex) -> *mut c_void {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `tryLock` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_mutex_guard_storage(_guard: *const TyrusMutexGuard) -> *mut c_void {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `MutexGuard` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_mutex_guard_unlock(_guard: *mut TyrusMutexGuard) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `MutexGuard` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

pub struct TyrusRwLock {
    _private: (),
}
pub struct TyrusRwLockReadGuard {
    _private: (),
}
pub struct TyrusRwLockWriteGuard {
    _private: (),
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_rwlock_new(
    _value: *mut c_void,
    _drop_value: TyrusPromiseDrop,
) -> *const TyrusRwLock {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `RwLock` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_rwlock_retain(_rwlock: *const TyrusRwLock) -> *const TyrusRwLock {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `RwLock` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_rwlock_free(_rwlock: *const TyrusRwLock) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `RwLock` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_rwlock_read(_rwlock: *const TyrusRwLock) -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `read` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_rwlock_write(_rwlock: *const TyrusRwLock) -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `write` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_rwlock_try_read(_rwlock: *const TyrusRwLock) -> *mut c_void {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `tryRead` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_rwlock_try_write(_rwlock: *const TyrusRwLock) -> *mut c_void {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `tryWrite` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_rwlock_read_guard_storage(
    _guard: *const TyrusRwLockReadGuard,
) -> *mut c_void {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `RwReadGuard` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_rwlock_write_guard_storage(
    _guard: *const TyrusRwLockWriteGuard,
) -> *mut c_void {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `RwWriteGuard` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_rwlock_read_guard_unlock(_guard: *mut TyrusRwLockReadGuard) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `RwReadGuard` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_rwlock_write_guard_unlock(_guard: *mut TyrusRwLockWriteGuard) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `RwWriteGuard` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

pub struct TyrusSemaphore {
    _private: (),
}
pub struct TyrusSemaphorePermit {
    _private: (),
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_semaphore_new(_permits: usize) -> *const TyrusSemaphore {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `Semaphore` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_semaphore_retain(
    _semaphore: *const TyrusSemaphore,
) -> *const TyrusSemaphore {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `Semaphore` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_semaphore_free(_semaphore: *const TyrusSemaphore) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `Semaphore` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_semaphore_acquire(
    _semaphore: *const TyrusSemaphore,
) -> *mut TyrusPromise {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `acquire` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_semaphore_try_acquire(
    _semaphore: *const TyrusSemaphore,
) -> *mut c_void {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `tryAcquire` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_semaphore_permit_release(_permit: *mut TyrusSemaphorePermit) {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `SemaphorePermit` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

unsafe fn websocket_unavailable() -> ! {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `WebSocket` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_router_ws(
    router: *mut HttpRouter,
    path: TyrusString,
    handler: Option<WebSocketHandler>,
    origins: *const TyrusStringArray,
    protocols: *const TyrusStringArray,
    max_message_bytes: i64,
    pre_handlers: *const HttpPreHandler,
    pre_handlers_len: usize,
    rate_limit_max: i64,
    rate_limit_window_millis: i64,
    rate_limit_global_max: i64,
    rate_limit_ban: i64,
    rate_limit_ban_window_millis: i64,
) -> i32 {
    unsafe { websocket_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_websocket_connect(
    url: TyrusString,
    protocols: *const TyrusStringArray,
) -> *mut TyrusPromise {
    unsafe { websocket_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_websocket_receive(
    handle: *const WebSocketHandle,
) -> *mut TyrusPromise {
    unsafe { websocket_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_websocket_stream_duplex(
    handle: *const WebSocketHandle,
) -> *mut TyrusPromise {
    unsafe { websocket_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_websocket_send_text(
    handle: *const WebSocketHandle,
    message: TyrusString,
) -> *mut TyrusPromise {
    unsafe { websocket_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_websocket_send_bytes(
    handle: *const WebSocketHandle,
    data: *const TyrusArray,
) -> *mut TyrusPromise {
    unsafe { websocket_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_websocket_close(
    handle: *const WebSocketHandle,
    code: i32,
    reason: TyrusString,
) -> *mut TyrusPromise {
    unsafe { websocket_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_websocket_protocol(handle: *const WebSocketHandle) -> TyrusString {
    unsafe { websocket_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_websocket_retain(
    handle: *const WebSocketHandle,
) -> *const WebSocketHandle {
    handle
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_websocket_free(handle: *const WebSocketHandle) {}

unsafe fn docs_unavailable() -> ! {
    // SAFETY: the message is a static C string literal.
    unsafe {
        tyrus_panic(
            c"tyrus runtime error: `app.docs` is not available in the playground sandbox, which runs your program in the browser with no operating system, filesystem or network access"
                .as_ptr(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_router_describe(
    router: *mut HttpRouter,
    fragment: TyrusString,
) -> i32 {
    unsafe { docs_unavailable() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_router_docs(
    router: *mut HttpRouter,
    options: *const DocsOptions,
    pre_handlers: *const HttpPreHandler,
    pre_handlers_len: usize,
) -> i32 {
    unsafe { docs_unavailable() }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_runtime_configure_development_https(enabled: i32) {}
