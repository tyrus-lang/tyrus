use super::abi::{
    CpuUsage, MemoryUsage, ProcessErrorFfi, ProcessErrorTaken, ProcessExecPayload,
    ProcessExecResultFfi, TyrusArray, TyrusStringArray,
};
use super::strings::alloc_string;
use crate::stream::{self, TyrusReadableStream, TyrusWritableStream};
use std::ffi::CStr;
use std::mem::ManuallyDrop;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command as TokioCommand;
use tokio::sync::{mpsc, oneshot, watch};
use tyrus_string::TyrusString;

const PROCESS_STREAM_CHUNK_BYTES: usize = 64 * 1024;
const PROCESS_STREAM_CAPACITY_BYTES: usize = 256 * 1024;

pub struct TyrusChildProcess {
    pid: u32,
    stdin: Arc<TyrusWritableStream>,
    stdout: Arc<TyrusReadableStream>,
    stderr: Arc<TyrusReadableStream>,
    kill_requests: mpsc::UnboundedSender<ProcessKillRequest>,
    exit: watch::Receiver<Option<Result<i32, ()>>>,
}

#[repr(C)]
union ProcessAsyncResultPayload {
    code: ManuallyDrop<i32>,
    error: ManuallyDrop<ProcessErrorFfi>,
    words: [u64; 4],
}

#[repr(C)]
struct ProcessAsyncResultFfi {
    tag: u8,
    payload: ProcessAsyncResultPayload,
}

const _: () = {
    assert!(std::mem::size_of::<ProcessAsyncResultFfi>() == 40);
    assert!(std::mem::offset_of!(ProcessAsyncResultFfi, payload) == 8);
};

struct ProcessKillRequest {
    response: oneshot::Sender<Result<(), ()>>,
    wait_for_exit: bool,
}

impl Drop for TyrusChildProcess {
    fn drop(&mut self) {
        if self.exit.borrow().is_some() {
            return;
        }
        let (response, receiver) = oneshot::channel();
        if self
            .kill_requests
            .send(ProcessKillRequest {
                response,
                wait_for_exit: true,
            })
            .is_err()
        {
            return;
        }
        match tokio::runtime::Handle::try_current() {
            Ok(handle) if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
                tokio::task::block_in_place(|| {
                    let _ = handle.block_on(receiver);
                });
            }
            Ok(_) => {}
            Err(_) => {
                let _ = receiver.blocking_recv();
            }
        }
    }
}

thread_local! {
    static LAST_PROCESS_ERROR: std::cell::Cell<Option<(&'static CStr, &'static CStr)>> =
        const { std::cell::Cell::new(None) };
}

fn set_process_error(code: &'static CStr, message: &'static CStr) {
    LAST_PROCESS_ERROR.with(|slot| slot.set(Some((code, message))));
}

fn fulfill_process_error(
    promise: *mut crate::TyrusPromise,
    code: &'static str,
    message: &'static str,
) {
    let result = Box::new(ProcessAsyncResultFfi {
        tag: 1,
        payload: ProcessAsyncResultPayload {
            error: ManuallyDrop::new(ProcessErrorFfi {
                code: TyrusString::literal(code),
                message: TyrusString::literal(message),
            }),
        },
    });
    unsafe {
        crate::tyrus_promise_fulfill_owned(
            promise,
            Box::into_raw(result).cast(),
            Some(drop_process_async_result),
        );
    }
}

unsafe extern "C" fn drop_process_async_result(value: *mut std::ffi::c_void) {
    unsafe {
        if value.is_null() {
            return;
        }
        let result = Box::from_raw(value.cast::<ProcessAsyncResultFfi>());
        if result.tag == 1 {
            let error = ManuallyDrop::into_inner(result.payload.error);
            error.code.release();
            error.message.release();
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_process_take_error(out: *mut ProcessErrorTaken) {
    if out.is_null() {
        return;
    }
    LAST_PROCESS_ERROR.with(|slot| {
        if let Some((code, message)) = slot.take() {
            unsafe {
                (*out).code =
                    TyrusString::literal(code.to_str().expect("a static error literal is UTF-8"));
                (*out).message = TyrusString::literal(
                    message.to_str().expect("a static error literal is UTF-8"),
                );
            }
        }
    });
}

fn process_args(args: &TyrusStringArray) -> Option<Vec<String>> {
    if args.length == 0 {
        return Some(Vec::new());
    }
    if args.data.is_null() {
        return None;
    }
    let values = unsafe { std::slice::from_raw_parts(args.data, args.length) };
    values
        .iter()
        .map(|value| (!value.is_null()).then(|| unsafe { value.as_str().to_owned() }))
        .collect()
}

/// Starts a process without a shell and wires its standard descriptors to the
/// bounded byte-stream adapters. A null return denotes invalid input or a
/// spawn failure reported through the process error slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_process_exec(
    command: TyrusString,
    args: *const TyrusStringArray,
) -> *const TyrusChildProcess {
    if command.is_null() {
        set_process_error(
            c"ERR_PROCESS_INVALID_COMMAND",
            c"command must be a valid string",
        );
        return std::ptr::null();
    }
    let Some(args) = (unsafe { args.as_ref() }).and_then(process_args) else {
        set_process_error(
            c"ERR_PROCESS_INVALID_ARGUMENT",
            c"arguments must be valid strings",
        );
        return std::ptr::null();
    };
    let mut command_builder = TokioCommand::new(unsafe { command.as_str() });
    command_builder
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let spawned = if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::spawn(move || {
            crate::executor::executor().block_on(async move { command_builder.spawn() })
        })
        .join()
        .unwrap_or_else(|_| Err(std::io::Error::other("process spawn worker panicked")))
    } else {
        crate::executor::executor().block_on(async { command_builder.spawn() })
    };
    let mut child = match spawned {
        Ok(child) => child,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            set_process_error(
                c"ERR_PROCESS_PERMISSION_DENIED",
                c"permission denied to execute the command",
            );
            return std::ptr::null();
        }
        Err(_) => {
            set_process_error(c"ERR_PROCESS_SPAWN_FAILED", c"failed to start process");
            return std::ptr::null();
        }
    };
    let Some(child_stdin) = child.stdin.take() else {
        set_process_error(c"ERR_PROCESS_PIPE_FAILED", c"failed to open process stdin");
        return std::ptr::null();
    };
    let Some(child_stdout) = child.stdout.take() else {
        set_process_error(c"ERR_PROCESS_PIPE_FAILED", c"failed to open process stdout");
        return std::ptr::null();
    };
    let Some(child_stderr) = child.stderr.take() else {
        set_process_error(c"ERR_PROCESS_PIPE_FAILED", c"failed to open process stderr");
        return std::ptr::null();
    };
    let (stdout_stream, stdout_producer) =
        stream::byte_readable_stream(PROCESS_STREAM_CAPACITY_BYTES);
    let (stderr_stream, stderr_producer) =
        stream::byte_readable_stream(PROCESS_STREAM_CAPACITY_BYTES);
    let (stdin_stream, stdin_consumer) =
        stream::byte_writable_stream(PROCESS_STREAM_CAPACITY_BYTES);
    let stdout = unsafe { Arc::from_raw(stdout_stream) };
    let stderr = unsafe { Arc::from_raw(stderr_stream) };
    let stdin = unsafe { Arc::from_raw(stdin_stream) };
    let pid = child.id().unwrap_or_default();
    let (kill_requests, mut kill_receiver) = mpsc::unbounded_channel::<ProcessKillRequest>();
    let (exit_sender, exit) = watch::channel(None);
    crate::executor::executor().spawn(async move {
        let mut reap_response: Option<oneshot::Sender<Result<(), ()>>> = None;
        let result = loop {
            tokio::select! {
                status = child.wait() => {
                    let result = status.map(|status| status.code().unwrap_or(-1)).map_err(|_| ());
                    if let Some(response) = reap_response.take() {
                        let _ = response.send(result.map(|_| ()));
                    }
                    break result;
                }
                request = kill_receiver.recv() => {
                    match request {
                        Some(request) => {
                            let result = child.start_kill().map_err(|_| ());
                            if request.wait_for_exit && result.is_ok() {
                                reap_response = Some(request.response);
                            } else {
                                let _ = request.response.send(result);
                            }
                        }
                        None => {
                            let _ = child.start_kill();
                        }
                    }
                }
            }
        };
        let _ = exit_sender.send(Some(result));
    });
    crate::executor::executor().spawn(async move {
        let mut reader = child_stdout;
        loop {
            let mut chunk = vec![0; PROCESS_STREAM_CHUNK_BYTES];
            let result = tokio::select! {
                result = reader.read(&mut chunk) => result,
                () = stdout_producer.writable_reader_gone() => return,
            };
            match result {
                Ok(0) => {
                    stdout_producer.close();
                    return;
                }
                Ok(length) => {
                    chunk.truncate(length);
                    if stdout_producer.write(chunk).await.is_err() {
                        return;
                    }
                }
                Err(_) => {
                    stdout_producer.fail();
                    return;
                }
            }
        }
    });
    crate::executor::executor().spawn(async move {
        let mut reader = child_stderr;
        loop {
            let mut chunk = vec![0; PROCESS_STREAM_CHUNK_BYTES];
            let result = tokio::select! {
                result = reader.read(&mut chunk) => result,
                () = stderr_producer.writable_reader_gone() => return,
            };
            match result {
                Ok(0) => {
                    stderr_producer.close();
                    return;
                }
                Ok(length) => {
                    chunk.truncate(length);
                    if stderr_producer.write(chunk).await.is_err() {
                        return;
                    }
                }
                Err(_) => {
                    stderr_producer.fail();
                    return;
                }
            }
        }
    });
    crate::executor::executor().spawn(async move {
        let mut writer = child_stdin;
        loop {
            match stdin_consumer.read().await {
                Ok(Some(chunk)) => {
                    if writer.write_all(&chunk).await.is_err() {
                        stdin_consumer.fail();
                        return;
                    }
                }
                Ok(None) => {
                    if writer.flush().await.is_err() || writer.shutdown().await.is_err() {
                        stdin_consumer.fail();
                    } else {
                        stdin_consumer.close();
                    }
                    return;
                }
                Err(_) => return,
            }
        }
    });
    Arc::into_raw(Arc::new(TyrusChildProcess {
        pid,
        stdin,
        stdout,
        stderr,
        kill_requests,
        exit,
    }))
}

/// Writes `Result<ChildProcess, ProcessError>` into compiler-owned storage.
/// Recoverable spawn failures are values, so this API never rejects or throws.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_process_exec_result(
    command: TyrusString,
    args: *const TyrusStringArray,
    out: *mut ProcessExecResultFfi,
) {
    if out.is_null() {
        return;
    }
    let child = unsafe { tyrus_process_exec(command, args) };
    let result = if !child.is_null() {
        ProcessExecResultFfi {
            tag: 0,
            payload: ProcessExecPayload {
                child: ManuallyDrop::new(child.cast()),
            },
        }
    } else {
        let mut error = ProcessErrorTaken {
            code: TyrusString::null(),
            message: TyrusString::null(),
        };
        unsafe { tyrus_process_take_error(&mut error) };
        if error.code.is_null() {
            error.code = TyrusString::literal("ERR_PROCESS_UNKNOWN");
        }
        if error.message.is_null() {
            error.message = TyrusString::literal("process execution failed");
        }
        ProcessExecResultFfi {
            tag: 1,
            payload: ProcessExecPayload {
                error: ManuallyDrop::new(ProcessErrorFfi {
                    code: error.code,
                    message: error.message,
                }),
            },
        }
    };
    unsafe { std::ptr::write(out, result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_child_process_wait(
    child: *const TyrusChildProcess,
) -> *mut crate::TyrusPromise {
    let promise = crate::tyrus_promise_pending();
    if child.is_null() {
        fulfill_process_error(promise, "ERR_PROCESS_CLOSED", "process handle is closed");
        return promise;
    }
    let child = std::mem::ManuallyDrop::new(unsafe { Arc::from_raw(child) });
    let child = Arc::clone(&child);
    unsafe { crate::tyrus_promise_retain(promise) };
    let promise_address = promise as usize;
    crate::executor::executor().spawn(async move {
        let mut exit = child.exit.clone();
        let result = loop {
            if let Some(result) = *exit.borrow() {
                break result;
            }
            if exit.changed().await.is_err() {
                break Err(());
            }
        };
        let promise = promise_address as *mut crate::TyrusPromise;
        match result {
            Ok(code) => unsafe {
                let result = Box::new(ProcessAsyncResultFfi {
                    tag: 0,
                    payload: ProcessAsyncResultPayload {
                        code: ManuallyDrop::new(code),
                    },
                });
                crate::tyrus_promise_fulfill_owned(
                    promise,
                    Box::into_raw(result).cast(),
                    Some(drop_process_async_result),
                );
            },
            Err(()) => fulfill_process_error(
                promise,
                "ERR_PROCESS_WAIT_FAILED",
                "failed to wait for process",
            ),
        }
        unsafe { crate::tyrus_promise_free(promise) };
    });
    promise
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_child_process_kill(
    child: *const TyrusChildProcess,
) -> *mut crate::TyrusPromise {
    let promise = crate::tyrus_promise_pending();
    if child.is_null() {
        fulfill_process_error(promise, "ERR_PROCESS_CLOSED", "process handle is closed");
        return promise;
    }
    let child = std::mem::ManuallyDrop::new(unsafe { Arc::from_raw(child) });
    let child = Arc::clone(&child);
    let (reply, response) = oneshot::channel();
    if child
        .kill_requests
        .send(ProcessKillRequest {
            response: reply,
            wait_for_exit: false,
        })
        .is_err()
    {
        fulfill_process_error(promise, "ERR_PROCESS_CLOSED", "process handle is closed");
        return promise;
    }
    unsafe { crate::tyrus_promise_retain(promise) };
    let promise_address = promise as usize;
    crate::executor::executor().spawn(async move {
        let result = response.await.unwrap_or(Err(()));
        let promise = promise_address as *mut crate::TyrusPromise;
        match result {
            Ok(()) => unsafe {
                let result = Box::new(ProcessAsyncResultFfi {
                    tag: 0,
                    payload: ProcessAsyncResultPayload { words: [0; 4] },
                });
                crate::tyrus_promise_fulfill_owned(
                    promise,
                    Box::into_raw(result).cast(),
                    Some(drop_process_async_result),
                );
            },
            Err(()) => fulfill_process_error(
                promise,
                "ERR_PROCESS_KILL_FAILED",
                "failed to terminate process",
            ),
        }
        unsafe { crate::tyrus_promise_free(promise) };
    });
    promise
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_child_process_retain(
    child: *const TyrusChildProcess,
) -> *const TyrusChildProcess {
    if !child.is_null() {
        unsafe { Arc::increment_strong_count(child) };
    }
    child
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_child_process_free(child: *const TyrusChildProcess) {
    if !child.is_null() {
        drop(unsafe { Arc::from_raw(child) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_child_process_pid(child: *const TyrusChildProcess) -> u32 {
    if child.is_null() {
        0
    } else {
        unsafe { (*child).pid }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_child_process_stdin(
    child: *const TyrusChildProcess,
) -> *const TyrusWritableStream {
    if child.is_null() {
        std::ptr::null()
    } else {
        Arc::into_raw(Arc::clone(unsafe { &(*child).stdin }))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_child_process_stdout(
    child: *const TyrusChildProcess,
) -> *const TyrusReadableStream {
    if child.is_null() {
        std::ptr::null()
    } else {
        Arc::into_raw(Arc::clone(unsafe { &(*child).stdout }))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_child_process_stderr(
    child: *const TyrusChildProcess,
) -> *const TyrusReadableStream {
    if child.is_null() {
        std::ptr::null()
    } else {
        Arc::into_raw(Arc::clone(unsafe { &(*child).stderr }))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_process_argv(out_array: *mut TyrusArray) {
    if out_array.is_null() {
        return;
    }
    let args: Vec<String> = std::env::args_os()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    if args.is_empty() {
        unsafe {
            (*out_array).data = std::ptr::null_mut();
            (*out_array).length = 0;
            (*out_array).capacity = 0;
        }
        return;
    }
    let mut data: Vec<TyrusString> = Vec::with_capacity(args.len());
    for arg in &args {
        data.push(alloc_string(arg));
    }
    let ptr = data.as_mut_ptr();
    let len = data.len();
    let cap = data.capacity();
    std::mem::forget(data);
    unsafe {
        (*out_array).data = ptr as *mut u8;
        (*out_array).length = len;
        (*out_array).capacity = cap;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_process_pid() -> u32 {
    std::process::id()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_process_ppid() -> u32 {
    let Ok(pid) = sysinfo::get_current_pid() else {
        return 0;
    };
    let mut system = sysinfo::System::new();
    system.refresh_process(pid);
    system
        .process(pid)
        .and_then(sysinfo::Process::parent)
        .map_or(0, sysinfo::Pid::as_u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_process_cwd() -> TyrusString {
    let cwd = std::env::current_dir()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    alloc_string(&cwd)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_process_uptime() -> u64 {
    let Ok(pid) = sysinfo::get_current_pid() else {
        return 0;
    };
    let mut system = sysinfo::System::new();
    system.refresh_process(pid);
    let Some(process) = system.process(pid) else {
        return 0;
    };
    let start_seconds = process.start_time();
    let now_seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(start_seconds, |elapsed| elapsed.as_secs());
    now_seconds.saturating_sub(start_seconds) * 1000
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_process_memory_usage(out: *mut MemoryUsage) {
    if out.is_null() {
        return;
    }
    let rss = sysinfo::get_current_pid().ok().map_or(0, |pid| {
        let mut system = sysinfo::System::new();
        system.refresh_process(pid);
        system.process(pid).map_or(0, sysinfo::Process::memory)
    });
    unsafe {
        (*out).rss = rss;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_process_cpu_usage(out: *mut CpuUsage) {
    if out.is_null() {
        return;
    }
    let (user, system) = platform_cpu_usage_micros();
    unsafe {
        (*out).user = user;
        (*out).system = system;
    }
}

#[cfg(unix)]
fn platform_cpu_usage_micros() -> (u64, u64) {
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    if result != 0 {
        return (0, 0);
    }
    let to_micros =
        |tv: libc::timeval| (tv.tv_sec as u64).saturating_mul(1_000_000) + tv.tv_usec as u64;
    (to_micros(usage.ru_utime), to_micros(usage.ru_stime))
}

#[cfg(not(unix))]
#[cfg(not(windows))]
fn platform_cpu_usage_micros() -> (u64, u64) {
    (0, 0)
}

#[cfg(windows)]
fn platform_cpu_usage_micros() -> (u64, u64) {
    use std::mem::MaybeUninit;
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

    let mut creation = MaybeUninit::<FILETIME>::uninit();
    let mut exit = MaybeUninit::<FILETIME>::uninit();
    let mut kernel = MaybeUninit::<FILETIME>::uninit();
    let mut user = MaybeUninit::<FILETIME>::uninit();
    let succeeded = unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            creation.as_mut_ptr(),
            exit.as_mut_ptr(),
            kernel.as_mut_ptr(),
            user.as_mut_ptr(),
        )
    } != 0;
    if !succeeded {
        return (0, 0);
    }
    let kernel = unsafe { kernel.assume_init() };
    let user = unsafe { user.assume_init() };
    (
        filetime_ticks_to_micros(user.dwHighDateTime, user.dwLowDateTime),
        filetime_ticks_to_micros(kernel.dwHighDateTime, kernel.dwLowDateTime),
    )
}

#[cfg(any(windows, test))]
fn filetime_ticks_to_micros(high: u32, low: u32) -> u64 {
    (u64::from(high) << 32 | u64::from(low)) / 10
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static CANCELLED_STDIN_WRITE_DROPS: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn windows_filetime_conversion_preserves_microsecond_units() {
        assert_eq!(filetime_ticks_to_micros(0, 10_000), 1_000);
        assert_eq!(filetime_ticks_to_micros(1, 0), (1_u64 << 32) / 10);
    }

    unsafe extern "C" fn drop_cancelled_stdin_payload(value: *mut std::ffi::c_void) {
        CANCELLED_STDIN_WRITE_DROPS.fetch_add(1, Ordering::Relaxed);
        unsafe { crate::async_payload::tyrus_drop_async_copy_array(value) };
    }

    fn owned_byte_payload(length: usize) -> *mut std::ffi::c_void {
        let data = unsafe { libc::calloc(length, 1) }.cast::<u8>();
        assert!(!data.is_null(), "test byte allocation must succeed");
        Box::into_raw(Box::new(TyrusArray {
            data,
            length,
            capacity: length,
        }))
        .cast()
    }

    unsafe fn settled_wait_code(promise: *mut crate::TyrusPromise) -> i32 {
        assert_eq!(unsafe { crate::promise::wait_until_settled(promise) }, 1);
        let result = unsafe { crate::tyrus_promise_await(promise).cast::<ProcessAsyncResultFfi>() };
        assert_eq!(unsafe { (*result).tag }, 0);
        unsafe { *(*result).payload.code }
    }

    unsafe fn assert_settled_kill_ok(promise: *mut crate::TyrusPromise) {
        assert_eq!(unsafe { crate::promise::wait_until_settled(promise) }, 1);
        let result = unsafe { crate::tyrus_promise_await(promise).cast::<ProcessAsyncResultFfi>() };
        assert_eq!(unsafe { (*result).tag }, 0);
    }

    unsafe fn assert_process_result_error(promise: *mut crate::TyrusPromise, expected_code: &str) {
        assert_eq!(unsafe { crate::promise::wait_until_settled(promise) }, 1);
        let result = unsafe { crate::tyrus_promise_await(promise).cast::<ProcessAsyncResultFfi>() };
        assert_eq!(unsafe { (*result).tag }, 1);
        let error = unsafe { &(*result).payload.error };
        assert_eq!(unsafe { error.code.as_str() }, expected_code);
        assert!(!unsafe { error.message.as_str() }.is_empty());
        unsafe { crate::tyrus_promise_free(promise) };
    }

    #[test]
    fn pid_is_this_process_own_id() {
        assert_eq!(tyrus_process_pid(), std::process::id());
    }

    #[test]
    fn ppid_is_a_different_process_than_pid() {
        let ppid = tyrus_process_ppid();
        assert_ne!(ppid, tyrus_process_pid());
    }

    #[test]
    fn cwd_matches_std_env_current_dir() {
        let expected = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let pointer = tyrus_process_cwd();
        assert!(!pointer.is_null());
        let actual = unsafe { pointer.as_str() }.to_owned();
        unsafe { pointer.release() };
        assert_eq!(actual, expected);
    }

    #[test]
    fn uptime_is_a_small_but_non_negative_number_of_milliseconds() {
        // Counted from the start of the test binary, not of this test: a serial
        // run under AddressSanitizer reached this test 264 s in, past the one
        // minute bound this used to assert. A day still rejects the epoch or a
        // wrong unit, which is what the bound is for.
        let uptime = tyrus_process_uptime();
        assert!(uptime < 86_400_000, "{uptime}");
        assert_eq!(uptime % 1000, 0, "{uptime}");
    }

    #[test]
    fn memory_usage_reports_a_positive_rss() {
        let mut usage = MemoryUsage { rss: 0 };
        unsafe { tyrus_process_memory_usage(&mut usage) };
        assert!(usage.rss > 0);
    }

    #[test]
    fn cpu_usage_reports_non_negative_user_and_system_micros() {
        let mut usage = CpuUsage { user: 0, system: 0 };
        unsafe { tyrus_process_cpu_usage(&mut usage) };
        assert!(usage.user > 0 || usage.system > 0);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn exec_preserves_stdout_bytes_and_reports_the_exit_code() {
        let command = unsafe { TyrusString::lend("sh") }.expect("static command is valid");
        let arguments = [
            unsafe { TyrusString::lend("-c") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("printf '\\000stream-bytes'; exit 7") }
                .expect("static argument is valid"),
        ];
        let child = unsafe {
            tyrus_process_exec(
                command,
                &TyrusStringArray {
                    data: arguments.as_ptr(),
                    length: arguments.len(),
                    capacity: arguments.len(),
                },
            )
        };
        assert!(!child.is_null());
        let stdout = unsafe { tyrus_child_process_stdout(child) };
        let mut output = Vec::new();
        while let Some(chunk) = unsafe { crate::stream::read_byte_stream(stdout).await.unwrap() } {
            output.extend_from_slice(&chunk);
        }
        let wait = unsafe { tyrus_child_process_wait(child) };
        unsafe {
            assert_eq!(settled_wait_code(wait), 7);
            crate::tyrus_promise_free(wait);
            crate::stream::tyrus_readable_stream_free(stdout);
            tyrus_child_process_free(child);
        }
        assert_eq!(output, b"\0stream-bytes");
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn exec_captures_windows_stdout_stderr_and_exit_code() {
        let command = unsafe { TyrusString::lend("cmd.exe") }.expect("static command is valid");
        let arguments = [
            unsafe { TyrusString::lend("/D") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("/S") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("/C") }.expect("static argument is valid"),
            unsafe {
                TyrusString::lend("echo tyrus-output & echo tyrus-diagnostic 1>&2 & exit /B 7")
            }
            .expect("static argument is valid"),
        ];
        let child = unsafe {
            tyrus_process_exec(
                command,
                &TyrusStringArray {
                    data: arguments.as_ptr(),
                    length: arguments.len(),
                    capacity: arguments.len(),
                },
            )
        };
        assert!(!child.is_null());
        let stdout = unsafe { tyrus_child_process_stdout(child) };
        let stderr = unsafe { tyrus_child_process_stderr(child) };
        let mut output = Vec::new();
        while let Some(chunk) = unsafe { crate::stream::read_byte_stream(stdout).await.unwrap() } {
            output.extend_from_slice(&chunk);
        }
        let mut diagnostics = Vec::new();
        while let Some(chunk) = unsafe { crate::stream::read_byte_stream(stderr).await.unwrap() } {
            diagnostics.extend_from_slice(&chunk);
        }
        let wait = unsafe { tyrus_child_process_wait(child) };
        unsafe {
            assert_eq!(settled_wait_code(wait), 7);
            crate::tyrus_promise_free(wait);
            crate::stream::tyrus_readable_stream_free(stdout);
            crate::stream::tyrus_readable_stream_free(stderr);
            tyrus_child_process_free(child);
        }
        assert_eq!(output, b"tyrus-output\r\n");
        assert_eq!(diagnostics, b"tyrus-diagnostic\r\n");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn kill_unblocks_wait_for_a_running_process() {
        let command = unsafe { TyrusString::lend("sh") }.expect("static command is valid");
        let arguments = [
            unsafe { TyrusString::lend("-c") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("sleep 30") }.expect("static argument is valid"),
        ];
        let child = unsafe {
            tyrus_process_exec(
                command,
                &TyrusStringArray {
                    data: arguments.as_ptr(),
                    length: arguments.len(),
                    capacity: arguments.len(),
                },
            )
        };
        assert!(!child.is_null());
        let kill = unsafe { tyrus_child_process_kill(child) };
        unsafe {
            assert_settled_kill_ok(kill);
            crate::tyrus_promise_free(kill);
        }
        let wait = unsafe { tyrus_child_process_wait(child) };
        unsafe {
            let code = settled_wait_code(wait);
            assert!(code <= 0, "terminated process must not report success");
            crate::tyrus_promise_free(wait);
            tyrus_child_process_free(child);
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn kill_unblocks_wait_for_a_running_process() {
        let command =
            unsafe { TyrusString::lend("powershell.exe") }.expect("static command is valid");
        let arguments = [
            unsafe { TyrusString::lend("-NoLogo") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("-NoProfile") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("-NonInteractive") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("-Command") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("Start-Sleep -Seconds 30") }
                .expect("static argument is valid"),
        ];
        let child = unsafe {
            tyrus_process_exec(
                command,
                &TyrusStringArray {
                    data: arguments.as_ptr(),
                    length: arguments.len(),
                    capacity: arguments.len(),
                },
            )
        };
        assert!(!child.is_null());
        let kill = unsafe { tyrus_child_process_kill(child) };
        unsafe {
            assert_settled_kill_ok(kill);
            crate::tyrus_promise_free(kill);
        }
        let wait = unsafe { tyrus_child_process_wait(child) };
        unsafe {
            let code = settled_wait_code(wait);
            assert_ne!(code, 0, "terminated process must not report success");
            crate::tyrus_promise_free(wait);
            tyrus_child_process_free(child);
        }
    }

    #[test]
    fn exec_reports_a_recoverable_error_for_an_unknown_command() {
        let command = unsafe { TyrusString::lend("tyrus-command-that-does-not-exist") }
            .expect("static command is valid");
        let child = unsafe {
            tyrus_process_exec(
                command,
                &TyrusStringArray {
                    data: std::ptr::null(),
                    length: 0,
                    capacity: 0,
                },
            )
        };
        assert!(child.is_null());

        let mut error = ProcessErrorTaken {
            code: TyrusString::null(),
            message: TyrusString::null(),
        };
        unsafe { tyrus_process_take_error(&mut error) };
        assert_eq!(unsafe { error.code.as_str() }, "ERR_PROCESS_SPAWN_FAILED");
        assert_eq!(unsafe { error.message.as_str() }, "failed to start process");
    }

    #[tokio::test]
    async fn child_process_wait_and_kill_fulfill_with_process_errors_for_null_handles() {
        let wait = unsafe { tyrus_child_process_wait(std::ptr::null()) };
        unsafe { assert_process_result_error(wait, "ERR_PROCESS_CLOSED") };

        let kill = unsafe { tyrus_child_process_kill(std::ptr::null()) };
        unsafe { assert_process_result_error(kill, "ERR_PROCESS_CLOSED") };
    }

    #[test]
    fn exec_result_returns_a_typed_error_for_an_unknown_command() {
        let command = unsafe { TyrusString::lend("tyrus-command-that-does-not-exist") }
            .expect("static command is valid");
        let mut result = std::mem::MaybeUninit::<ProcessExecResultFfi>::uninit();
        unsafe {
            tyrus_process_exec_result(
                command,
                &TyrusStringArray {
                    data: std::ptr::null(),
                    length: 0,
                    capacity: 0,
                },
                result.as_mut_ptr(),
            );
        }
        let result = unsafe { result.assume_init() };
        assert_eq!(result.tag, 1);
        let error = unsafe { ManuallyDrop::into_inner(result.payload.error) };
        assert_eq!(unsafe { error.code.as_str() }, "ERR_PROCESS_SPAWN_FAILED");
        assert_eq!(unsafe { error.message.as_str() }, "failed to start process");
    }

    #[cfg(unix)]
    #[test]
    fn exec_result_returns_a_child_in_the_ok_variant() {
        let command = unsafe { TyrusString::lend("sh") }.expect("static command is valid");
        let arguments = [
            unsafe { TyrusString::lend("-c") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("exit 0") }.expect("static argument is valid"),
        ];
        let args = TyrusStringArray {
            data: arguments.as_ptr(),
            length: arguments.len(),
            capacity: arguments.len(),
        };
        let mut result = std::mem::MaybeUninit::<ProcessExecResultFfi>::uninit();
        unsafe { tyrus_process_exec_result(command, &args, result.as_mut_ptr()) };
        let result = unsafe { result.assume_init() };
        assert_eq!(result.tag, 0);
        let child =
            unsafe { ManuallyDrop::into_inner(result.payload.child) }.cast::<TyrusChildProcess>();
        assert!(!child.is_null());
        unsafe { tyrus_child_process_free(child) };
    }

    #[cfg(unix)]
    #[test]
    fn exec_reports_permission_denied_for_a_file_without_the_execute_bit() {
        use std::os::unix::fs::PermissionsExt;

        let path = std::env::temp_dir().join(format!(
            "tyrus-process-not-executable-{}",
            std::process::id()
        ));
        std::fs::write(&path, "#!/bin/sh\nexit 0\n").expect("the fixture script is writable");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
            .expect("the fixture permissions can be set");
        let path_text = path.to_str().expect("the temp path is UTF-8").to_owned();
        let command = unsafe { TyrusString::lend(&path_text) }.expect("the fixture path is valid");
        let child = unsafe {
            tyrus_process_exec(
                command,
                &TyrusStringArray {
                    data: std::ptr::null(),
                    length: 0,
                    capacity: 0,
                },
            )
        };
        let _ = std::fs::remove_file(&path);
        assert!(child.is_null());

        let mut error = ProcessErrorTaken {
            code: TyrusString::null(),
            message: TyrusString::null(),
        };
        unsafe { tyrus_process_take_error(&mut error) };
        assert_eq!(
            unsafe { error.code.as_str() },
            "ERR_PROCESS_PERMISSION_DENIED"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn closing_stdin_delivers_eof_to_the_child() {
        let command = unsafe { TyrusString::lend("sh") }.expect("static command is valid");
        let arguments = [
            unsafe { TyrusString::lend("-c") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("IFS= read -r line; printf 'received:%s' \"$line\"") }
                .expect("static argument is valid"),
        ];
        let child = unsafe {
            tyrus_process_exec(
                command,
                &TyrusStringArray {
                    data: arguments.as_ptr(),
                    length: arguments.len(),
                    capacity: arguments.len(),
                },
            )
        };
        assert!(!child.is_null());
        let stdin = unsafe { tyrus_child_process_stdin(child) };
        let stdout = unsafe { tyrus_child_process_stdout(child) };
        unsafe {
            crate::stream::write_byte_stream(stdin, b"payload\n".to_vec())
                .await
                .unwrap();
            crate::stream::close_byte_writable_stream(stdin)
                .await
                .unwrap();
        }
        let mut output = Vec::new();
        while let Some(chunk) = unsafe { crate::stream::read_byte_stream(stdout).await.unwrap() } {
            output.extend_from_slice(&chunk);
        }
        let wait = unsafe { tyrus_child_process_wait(child) };
        unsafe {
            assert_eq!(settled_wait_code(wait), 0);
            crate::tyrus_promise_free(wait);
            crate::stream::tyrus_writable_stream_free(stdin);
            crate::stream::tyrus_readable_stream_free(stdout);
            tyrus_child_process_free(child);
        }
        assert_eq!(output, b"received:payload");
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn closing_stdin_delivers_eof_to_the_child() {
        let command =
            unsafe { TyrusString::lend("powershell.exe") }.expect("static command is valid");
        let arguments = [
            unsafe { TyrusString::lend("-NoLogo") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("-NoProfile") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("-NonInteractive") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("-Command") }.expect("static argument is valid"),
            unsafe {
                TyrusString::lend(
                    "$line = [Console]::In.ReadLine(); [Console]::Out.Write('received:' + $line)",
                )
            }
            .expect("static argument is valid"),
        ];
        let child = unsafe {
            tyrus_process_exec(
                command,
                &TyrusStringArray {
                    data: arguments.as_ptr(),
                    length: arguments.len(),
                    capacity: arguments.len(),
                },
            )
        };
        assert!(!child.is_null());
        let stdin = unsafe { tyrus_child_process_stdin(child) };
        let stdout = unsafe { tyrus_child_process_stdout(child) };
        unsafe {
            crate::stream::write_byte_stream(stdin, b"payload\n".to_vec())
                .await
                .unwrap();
            crate::stream::close_byte_writable_stream(stdin)
                .await
                .unwrap();
        }
        let mut output = Vec::new();
        while let Some(chunk) = unsafe { crate::stream::read_byte_stream(stdout).await.unwrap() } {
            output.extend_from_slice(&chunk);
        }
        let wait = unsafe { tyrus_child_process_wait(child) };
        unsafe {
            assert_eq!(settled_wait_code(wait), 0);
            crate::tyrus_promise_free(wait);
            crate::stream::tyrus_writable_stream_free(stdin);
            crate::stream::tyrus_readable_stream_free(stdout);
            tyrus_child_process_free(child);
        }
        assert_eq!(output, b"received:payload");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn stdout_and_stderr_remain_independent_streams() {
        let command = unsafe { TyrusString::lend("sh") }.expect("static command is valid");
        let arguments = [
            unsafe { TyrusString::lend("-c") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("printf output; printf diagnostic >&2") }
                .expect("static argument is valid"),
        ];
        let child = unsafe {
            tyrus_process_exec(
                command,
                &TyrusStringArray {
                    data: arguments.as_ptr(),
                    length: arguments.len(),
                    capacity: arguments.len(),
                },
            )
        };
        assert!(!child.is_null());
        let stdout = unsafe { tyrus_child_process_stdout(child) };
        let stderr = unsafe { tyrus_child_process_stderr(child) };
        let mut output = Vec::new();
        while let Some(chunk) = unsafe { crate::stream::read_byte_stream(stdout).await.unwrap() } {
            output.extend_from_slice(&chunk);
        }
        let mut diagnostics = Vec::new();
        while let Some(chunk) = unsafe { crate::stream::read_byte_stream(stderr).await.unwrap() } {
            diagnostics.extend_from_slice(&chunk);
        }
        unsafe {
            crate::stream::tyrus_readable_stream_free(stdout);
            crate::stream::tyrus_readable_stream_free(stderr);
            tyrus_child_process_free(child);
        }
        assert_eq!(output, b"output");
        assert_eq!(diagnostics, b"diagnostic");
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn stdout_and_stderr_remain_independent_streams() {
        let command = unsafe { TyrusString::lend("cmd.exe") }.expect("static command is valid");
        let arguments = [
            unsafe { TyrusString::lend("/D") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("/S") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("/C") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("echo output & echo diagnostic 1>&2") }
                .expect("static argument is valid"),
        ];
        let child = unsafe {
            tyrus_process_exec(
                command,
                &TyrusStringArray {
                    data: arguments.as_ptr(),
                    length: arguments.len(),
                    capacity: arguments.len(),
                },
            )
        };
        assert!(!child.is_null());
        let stdout = unsafe { tyrus_child_process_stdout(child) };
        let stderr = unsafe { tyrus_child_process_stderr(child) };
        let mut output = Vec::new();
        while let Some(chunk) = unsafe { crate::stream::read_byte_stream(stdout).await.unwrap() } {
            output.extend_from_slice(&chunk);
        }
        let mut diagnostics = Vec::new();
        while let Some(chunk) = unsafe { crate::stream::read_byte_stream(stderr).await.unwrap() } {
            diagnostics.extend_from_slice(&chunk);
        }
        unsafe {
            crate::stream::tyrus_readable_stream_free(stdout);
            crate::stream::tyrus_readable_stream_free(stderr);
            tyrus_child_process_free(child);
        }
        assert_eq!(output, b"output\r\n");
        assert_eq!(diagnostics, b"diagnostic\r\n");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn stdout_backpressure_preserves_output_larger_than_the_stream_capacity() {
        let command = unsafe { TyrusString::lend("dd") }.expect("static command is valid");
        let arguments = [
            unsafe { TyrusString::lend("if=/dev/zero") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("bs=1024") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("count=512") }.expect("static argument is valid"),
        ];
        let child = unsafe {
            tyrus_process_exec(
                command,
                &TyrusStringArray {
                    data: arguments.as_ptr(),
                    length: arguments.len(),
                    capacity: arguments.len(),
                },
            )
        };
        assert!(!child.is_null());
        let stdout = unsafe { tyrus_child_process_stdout(child) };
        let mut output = Vec::new();
        while let Some(chunk) = unsafe { crate::stream::read_byte_stream(stdout).await.unwrap() } {
            output.extend_from_slice(&chunk);
        }
        let wait = unsafe { tyrus_child_process_wait(child) };
        unsafe {
            assert_eq!(settled_wait_code(wait), 0);
            crate::tyrus_promise_free(wait);
            crate::stream::tyrus_readable_stream_free(stdout);
            tyrus_child_process_free(child);
        }
        assert_eq!(output.len(), PROCESS_STREAM_CAPACITY_BYTES * 2);
        assert!(output.iter().all(|byte| *byte == 0));
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn stdout_backpressure_preserves_output_larger_than_the_stream_capacity() {
        let command =
            unsafe { TyrusString::lend("powershell.exe") }.expect("static command is valid");
        let byte_count = PROCESS_STREAM_CAPACITY_BYTES * 2;
        let script = format!(
            "$bytes = New-Object byte[] {byte_count}; [Console]::OpenStandardOutput().Write($bytes, 0, $bytes.Length)"
        );
        let arguments = [
            unsafe { TyrusString::lend("-NoLogo") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("-NoProfile") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("-NonInteractive") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("-Command") }.expect("static argument is valid"),
            unsafe { TyrusString::lend(&script) }.expect("the PowerShell script is valid"),
        ];
        let child = unsafe {
            tyrus_process_exec(
                command,
                &TyrusStringArray {
                    data: arguments.as_ptr(),
                    length: arguments.len(),
                    capacity: arguments.len(),
                },
            )
        };
        assert!(!child.is_null());
        let stdout = unsafe { tyrus_child_process_stdout(child) };
        let mut output = Vec::new();
        while let Some(chunk) = unsafe { crate::stream::read_byte_stream(stdout).await.unwrap() } {
            output.extend_from_slice(&chunk);
        }
        let wait = unsafe { tyrus_child_process_wait(child) };
        unsafe {
            assert_eq!(settled_wait_code(wait), 0);
            crate::tyrus_promise_free(wait);
            crate::stream::tyrus_readable_stream_free(stdout);
            tyrus_child_process_free(child);
        }
        assert_eq!(output.len(), byte_count);
        assert!(output.iter().all(|byte| *byte == 0));
    }

    #[cfg(unix)]
    #[test]
    fn cancelling_a_blocked_stdout_read_keeps_the_child_lifecycle_usable() {
        let command = unsafe { TyrusString::lend("sleep") }.expect("static command is valid");
        let arguments = [unsafe { TyrusString::lend("30") }.expect("static argument is valid")];
        let child = unsafe {
            tyrus_process_exec(
                command,
                &TyrusStringArray {
                    data: arguments.as_ptr(),
                    length: arguments.len(),
                    capacity: arguments.len(),
                },
            )
        };
        assert!(!child.is_null());
        let stdout = unsafe { tyrus_child_process_stdout(child) };
        let read = unsafe { crate::stream::tyrus_readable_stream_read(stdout) };
        assert_eq!(
            unsafe {
                crate::tyrus_promise_request_cancel(
                    read,
                    TyrusString::literal("cancel process stdout read"),
                )
            },
            1
        );
        unsafe {
            crate::promise::wait_until_settled(read);
            assert_eq!(
                crate::tyrus_promise_state(read),
                crate::TYRUS_PROMISE_CANCELLED
            );
            crate::tyrus_promise_free(read);
        }

        let kill = unsafe { tyrus_child_process_kill(child) };
        unsafe {
            assert_settled_kill_ok(kill);
            crate::tyrus_promise_free(kill);
        }
        let wait = unsafe { tyrus_child_process_wait(child) };
        unsafe {
            assert_eq!(settled_wait_code(wait), -1);
            crate::tyrus_promise_free(wait);
            crate::stream::tyrus_readable_stream_free(stdout);
            tyrus_child_process_free(child);
        }
    }

    #[cfg(windows)]
    #[test]
    fn cancelling_a_blocked_stdout_read_keeps_the_child_lifecycle_usable() {
        let command =
            unsafe { TyrusString::lend("powershell.exe") }.expect("static command is valid");
        let arguments = [
            unsafe { TyrusString::lend("-NoLogo") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("-NoProfile") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("-NonInteractive") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("-Command") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("Start-Sleep -Seconds 30") }
                .expect("static argument is valid"),
        ];
        let child = unsafe {
            tyrus_process_exec(
                command,
                &TyrusStringArray {
                    data: arguments.as_ptr(),
                    length: arguments.len(),
                    capacity: arguments.len(),
                },
            )
        };
        assert!(!child.is_null());
        let stdout = unsafe { tyrus_child_process_stdout(child) };
        let read = unsafe { crate::stream::tyrus_readable_stream_read(stdout) };
        assert_eq!(
            unsafe {
                crate::tyrus_promise_request_cancel(
                    read,
                    TyrusString::literal("cancel process stdout read"),
                )
            },
            1
        );
        unsafe {
            crate::promise::wait_until_settled(read);
            assert_eq!(
                crate::tyrus_promise_state(read),
                crate::TYRUS_PROMISE_CANCELLED
            );
            crate::tyrus_promise_free(read);
        }

        let kill = unsafe { tyrus_child_process_kill(child) };
        unsafe {
            assert_settled_kill_ok(kill);
            crate::tyrus_promise_free(kill);
        }
        let wait = unsafe { tyrus_child_process_wait(child) };
        unsafe {
            assert_ne!(settled_wait_code(wait), 0);
            crate::tyrus_promise_free(wait);
            crate::stream::tyrus_readable_stream_free(stdout);
            tyrus_child_process_free(child);
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn cancelling_a_blocked_stdin_write_releases_its_unaccepted_chunk() {
        CANCELLED_STDIN_WRITE_DROPS.store(0, Ordering::Relaxed);
        let command = unsafe { TyrusString::lend("sleep") }.expect("static command is valid");
        let arguments = [unsafe { TyrusString::lend("30") }.expect("static argument is valid")];
        let child = unsafe {
            tyrus_process_exec(
                command,
                &TyrusStringArray {
                    data: arguments.as_ptr(),
                    length: arguments.len(),
                    capacity: arguments.len(),
                },
            )
        };
        assert!(!child.is_null());
        let stdin = unsafe { tyrus_child_process_stdin(child) };
        let full_chunk = vec![0; PROCESS_STREAM_CAPACITY_BYTES];
        tokio::time::timeout(std::time::Duration::from_secs(1), unsafe {
            crate::stream::write_byte_stream(stdin, full_chunk.clone())
        })
        .await
        .expect("the first stdin chunk must enter the stream")
        .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(1), unsafe {
            crate::stream::write_byte_stream(stdin, full_chunk)
        })
        .await
        .expect("the adapter must dequeue the first stdin chunk")
        .unwrap();

        let blocked = unsafe {
            crate::stream::tyrus_writable_stream_write(
                stdin,
                owned_byte_payload(PROCESS_STREAM_CAPACITY_BYTES),
                PROCESS_STREAM_CAPACITY_BYTES,
                Some(drop_cancelled_stdin_payload),
            )
        };
        assert_eq!(
            unsafe {
                crate::tyrus_promise_request_cancel(
                    blocked,
                    TyrusString::literal("cancel process stdin write"),
                )
            },
            1
        );
        unsafe {
            crate::promise::wait_until_settled(blocked);
            assert_eq!(
                crate::tyrus_promise_state(blocked),
                crate::TYRUS_PROMISE_CANCELLED
            );
            crate::tyrus_promise_free(blocked);
        }
        assert_eq!(CANCELLED_STDIN_WRITE_DROPS.load(Ordering::Relaxed), 1);

        let kill = unsafe { tyrus_child_process_kill(child) };
        unsafe {
            assert_settled_kill_ok(kill);
            crate::tyrus_promise_free(kill);
            crate::stream::tyrus_writable_stream_free(stdin);
            tyrus_child_process_free(child);
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn cancelling_a_blocked_stdin_write_releases_its_unaccepted_chunk() {
        CANCELLED_STDIN_WRITE_DROPS.store(0, Ordering::Relaxed);
        let command =
            unsafe { TyrusString::lend("powershell.exe") }.expect("static command is valid");
        let arguments = [
            unsafe { TyrusString::lend("-NoLogo") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("-NoProfile") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("-NonInteractive") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("-Command") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("Start-Sleep -Seconds 30") }
                .expect("static argument is valid"),
        ];
        let child = unsafe {
            tyrus_process_exec(
                command,
                &TyrusStringArray {
                    data: arguments.as_ptr(),
                    length: arguments.len(),
                    capacity: arguments.len(),
                },
            )
        };
        assert!(!child.is_null());
        let stdin = unsafe { tyrus_child_process_stdin(child) };
        let full_chunk = vec![0; PROCESS_STREAM_CAPACITY_BYTES];
        tokio::time::timeout(std::time::Duration::from_secs(1), unsafe {
            crate::stream::write_byte_stream(stdin, full_chunk.clone())
        })
        .await
        .expect("the first stdin chunk must enter the stream")
        .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(1), unsafe {
            crate::stream::write_byte_stream(stdin, full_chunk)
        })
        .await
        .expect("the adapter must dequeue the first stdin chunk")
        .unwrap();

        let blocked = unsafe {
            crate::stream::tyrus_writable_stream_write(
                stdin,
                owned_byte_payload(PROCESS_STREAM_CAPACITY_BYTES),
                PROCESS_STREAM_CAPACITY_BYTES,
                Some(drop_cancelled_stdin_payload),
            )
        };
        assert_eq!(
            unsafe {
                crate::tyrus_promise_request_cancel(
                    blocked,
                    TyrusString::literal("cancel process stdin write"),
                )
            },
            1
        );
        unsafe {
            crate::promise::wait_until_settled(blocked);
            assert_eq!(
                crate::tyrus_promise_state(blocked),
                crate::TYRUS_PROMISE_CANCELLED
            );
            crate::tyrus_promise_free(blocked);
        }
        assert_eq!(CANCELLED_STDIN_WRITE_DROPS.load(Ordering::Relaxed), 1);

        let kill = unsafe { tyrus_child_process_kill(child) };
        unsafe {
            assert_settled_kill_ok(kill);
            crate::tyrus_promise_free(kill);
            crate::stream::tyrus_writable_stream_free(stdin);
            tyrus_child_process_free(child);
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn closing_stdout_releases_the_owned_process_descriptor() {
        let command = unsafe { TyrusString::lend("yes") }.expect("static command is valid");
        let child = unsafe {
            tyrus_process_exec(
                command,
                &TyrusStringArray {
                    data: std::ptr::null(),
                    length: 0,
                    capacity: 0,
                },
            )
        };
        assert!(!child.is_null());
        let stdout = unsafe { tyrus_child_process_stdout(child) };
        let close = unsafe { crate::stream::tyrus_readable_stream_close(stdout) };
        unsafe {
            assert_eq!(crate::promise::wait_until_settled(close), 1);
            crate::tyrus_promise_free(close);
        }

        let wait = unsafe { tyrus_child_process_wait(child) };
        unsafe {
            assert_ne!(settled_wait_code(wait), 0);
            crate::tyrus_promise_free(wait);
            crate::stream::tyrus_readable_stream_free(stdout);
            tyrus_child_process_free(child);
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn closing_stdout_releases_the_owned_process_descriptor() {
        let command =
            unsafe { TyrusString::lend("powershell.exe") }.expect("static command is valid");
        let arguments = [
            unsafe { TyrusString::lend("-NoLogo") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("-NoProfile") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("-NonInteractive") }.expect("static argument is valid"),
            unsafe { TyrusString::lend("-Command") }.expect("static argument is valid"),
            unsafe {
                TyrusString::lend(
                    "$stream = [Console]::OpenStandardOutput(); $buffer = [byte[]]::new(4096); while ($true) { $stream.Write($buffer, 0, $buffer.Length) }",
                )
            }
            .expect("the PowerShell script is valid"),
        ];
        let child = unsafe {
            tyrus_process_exec(
                command,
                &TyrusStringArray {
                    data: arguments.as_ptr(),
                    length: arguments.len(),
                    capacity: arguments.len(),
                },
            )
        };
        assert!(!child.is_null());
        let stdout = unsafe { tyrus_child_process_stdout(child) };
        let close = unsafe { crate::stream::tyrus_readable_stream_close(stdout) };
        unsafe {
            assert_eq!(crate::promise::wait_until_settled(close), 1);
            crate::tyrus_promise_free(close);
        }

        let wait = unsafe { tyrus_child_process_wait(child) };
        unsafe {
            assert_ne!(settled_wait_code(wait), 0);
            crate::tyrus_promise_free(wait);
            crate::stream::tyrus_readable_stream_free(stdout);
            tyrus_child_process_free(child);
        }
    }

    #[cfg(unix)]
    #[test]
    fn dropping_the_last_handle_terminates_and_reaps_the_child() {
        let command = unsafe { TyrusString::lend("sleep") }.expect("static command is valid");
        let arguments = [unsafe { TyrusString::lend("30") }.expect("static argument is valid")];
        let child = unsafe {
            tyrus_process_exec(
                command,
                &TyrusStringArray {
                    data: arguments.as_ptr(),
                    length: arguments.len(),
                    capacity: arguments.len(),
                },
            )
        };
        assert!(!child.is_null());
        let pid = unsafe { tyrus_child_process_pid(child) };
        unsafe { tyrus_child_process_free(child) };

        let status = Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stderr(Stdio::null())
            .status()
            .expect("kill utility must be available on Unix");
        assert!(
            !status.success(),
            "dropped child must not remain running or zombie"
        );
    }
}
