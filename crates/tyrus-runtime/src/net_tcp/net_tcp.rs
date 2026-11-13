use crate::abi::TyrusArray;
use crate::executor::executor;
use crate::promise::TyrusPromise;
use crate::strings::alloc_string;
use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering, fence},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, watch};
use tyrus_string::TyrusString;

pub struct TcpStreamHandle {
    stream: Arc<Mutex<Option<TcpStream>>>,
    local_address: Option<String>,
    peer_address: Option<String>,
    closed: AtomicBool,
    close_signal: watch::Sender<()>,
    references: AtomicUsize,
}

const TCP_STREAM_CHUNK_BYTES: usize = 64 * 1024;
const TCP_STREAM_CAPACITY_BYTES: usize = 256 * 1024;

fn release_handle_reference(references: &AtomicUsize) -> bool {
    if references.fetch_sub(1, Ordering::Release) == 1 {
        fence(Ordering::Acquire);
        true
    } else {
        false
    }
}

fn streamed_error() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::NotConnected,
        "the TCP stream was handed to stream()",
    )
}

pub struct TcpListenerHandle {
    listener: Arc<Mutex<TcpListener>>,
    closed: AtomicBool,
    close_signal: watch::Sender<()>,
    references: AtomicUsize,
}

#[repr(C)]
struct TcpErrorFfi {
    code: TyrusString,
    message: TyrusString,
}

#[repr(C)]
struct TcpResultFfi<T> {
    tag: u8,
    payload: TcpResultPayload<T>,
}

#[repr(C)]
union TcpResultPayload<T> {
    ok: ManuallyDrop<T>,
    err: ManuallyDrop<TcpErrorFfi>,
    words: [u64; 4],
}

const RESULT_TAG_OK: u8 = 0;
const RESULT_TAG_ERR: u8 = 1;

const _: () = {
    assert!(std::mem::size_of::<TcpErrorFfi>() == 32);
    assert!(std::mem::size_of::<TcpResultFfi<*mut c_void>>() == 40);
    assert!(std::mem::size_of::<TcpResultFfi<TyrusArray>>() == 40);
    assert!(std::mem::size_of::<TcpResultFfi<u64>>() == 40);
    assert!(std::mem::size_of::<TcpResultFfi<()>>() == 40);
    assert!(std::mem::offset_of!(TcpResultFfi<*mut c_void>, payload) == 8);
};

fn rejection_text(code: &'static str) -> (&'static str, &'static str) {
    match code {
        "ERR_NET_CLOSED" => ("ERR_NET_CLOSED", "the TCP resource is closed"),
        "ERR_NET_PERMISSION" => (
            "ERR_NET_PERMISSION",
            "the operating system denied the TCP operation",
        ),
        "ERR_NET_TIMEOUT" => ("ERR_NET_TIMEOUT", "the TCP operation timed out"),
        "ERR_NET_INVALID_ADDRESS" => ("ERR_NET_INVALID_ADDRESS", "the network address is invalid"),
        "ERR_NET_ALLOCATION_FAILED" => (
            "ERR_NET_ALLOCATION_FAILED",
            "network memory allocation failed",
        ),
        "ERR_NET_CONNECT" => ("ERR_NET_CONNECT", "the TCP connection failed"),
        "ERR_NET_CONNECTION_REFUSED" => (
            "ERR_NET_CONNECTION_REFUSED",
            "the TCP connection was refused",
        ),
        "ERR_NET_ADDRESS_IN_USE" => (
            "ERR_NET_ADDRESS_IN_USE",
            "the TCP address is already in use",
        ),
        "ERR_NET_INVALID_DATA" => ("ERR_NET_INVALID_DATA", "the TCP data is invalid"),
        "ERR_NET_BIND" => ("ERR_NET_BIND", "the TCP listener could not bind"),
        "ERR_NET_ACCEPT" => (
            "ERR_NET_ACCEPT",
            "the TCP listener could not accept a connection",
        ),
        "ERR_NET_READ" => ("ERR_NET_READ", "the TCP stream could not be read"),
        "ERR_NET_WRITE" => ("ERR_NET_WRITE", "the TCP stream could not be written"),
        "ERR_NET_SHUTDOWN" => ("ERR_NET_SHUTDOWN", "the TCP stream could not shut down"),
        "ERR_NET_STREAMED" => (
            "ERR_NET_STREAMED",
            "the TCP stream was handed to stream() and is read and written through it",
        ),
        "ERR_NET_BUSY" => (
            "ERR_NET_BUSY",
            "the TCP stream has a read, write or shutdown in progress",
        ),
        _ => ("ERR_NET", "the network operation failed"),
    }
}

fn reject(promise: *mut TyrusPromise, code: &'static str) {
    let (code, message) = rejection_text(code);
    unsafe {
        crate::tyrus_promise_reject_pending_structured(
            promise,
            TyrusString::literal(code),
            TyrusString::literal(message),
        )
    };
}

fn error_value(code: &'static str) -> TcpErrorFfi {
    let (code, message) = rejection_text(code);
    TcpErrorFfi {
        code: TyrusString::literal(code),
        message: TyrusString::literal(message),
    }
}

#[derive(Clone, Copy)]
enum TcpResultKind {
    Stream,
    Listener,
    Bytes,
    Count,
    Unit,
    Duplex,
}

impl TcpResultKind {
    unsafe fn release(self, value: *mut c_void) {
        unsafe {
            let result = value.cast::<TcpResultFfi<*mut c_void>>();
            if (*result).tag == RESULT_TAG_ERR {
                let error = &mut *(*result).payload.err;
                std::mem::replace(&mut error.code, TyrusString::null()).release();
                std::mem::replace(&mut error.message, TyrusString::null()).release();
            } else {
                match self {
                    Self::Stream => {
                        let handle = *(*result).payload.ok;
                        if !handle.is_null() {
                            tyrus_net_tcp_stream_free(handle);
                        }
                    }
                    Self::Listener => {
                        let handle = *(*result).payload.ok;
                        if !handle.is_null() {
                            tyrus_net_tcp_listener_free(handle);
                        }
                    }
                    Self::Bytes => {
                        let result = value.cast::<TcpResultFfi<TyrusArray>>();
                        let array = &mut *(*result).payload.ok;
                        if !array.data.is_null() {
                            libc::free(array.data.cast());
                            array.data = std::ptr::null_mut();
                            array.length = 0;
                            array.capacity = 0;
                        }
                    }
                    Self::Duplex => {
                        let result = value.cast::<TcpResultFfi<*mut c_void>>();
                        let stream = *(*result).payload.ok;
                        if !stream.is_null() {
                            crate::stream::tyrus_duplex_stream_free(stream.cast());
                        }
                    }
                    Self::Count | Self::Unit => {}
                }
            }
        }
    }

    fn drop_callback(self) -> unsafe extern "C" fn(*mut c_void) {
        match self {
            Self::Stream => drop_tcp_stream_result,
            Self::Listener => drop_tcp_listener_result,
            Self::Bytes => drop_tcp_bytes_result,
            Self::Count => drop_tcp_count_result,
            Self::Unit => drop_tcp_unit_result,
            Self::Duplex => drop_tcp_duplex_result,
        }
    }
}

unsafe extern "C" fn drop_tcp_stream_result(value: *mut c_void) {
    unsafe { drop_tcp_result(value, TcpResultKind::Stream) }
}

unsafe extern "C" fn drop_tcp_listener_result(value: *mut c_void) {
    unsafe { drop_tcp_result(value, TcpResultKind::Listener) }
}

unsafe extern "C" fn drop_tcp_bytes_result(value: *mut c_void) {
    unsafe {
        if value.is_null() {
            return;
        }
        TcpResultKind::Bytes.release(value);
        libc::free(value);
    }
}

unsafe extern "C" fn drop_tcp_count_result(value: *mut c_void) {
    unsafe {
        if !value.is_null() {
            TcpResultKind::Count.release(value);
            libc::free(value);
        }
    }
}

unsafe extern "C" fn drop_tcp_unit_result(value: *mut c_void) {
    unsafe { drop_tcp_result(value, TcpResultKind::Unit) }
}

unsafe extern "C" fn drop_tcp_duplex_result(value: *mut c_void) {
    unsafe { drop_tcp_result(value, TcpResultKind::Duplex) }
}

unsafe fn drop_tcp_result(value: *mut c_void, kind: TcpResultKind) {
    unsafe {
        if !value.is_null() {
            kind.release(value);
            libc::free(value);
        }
    }
}

unsafe fn fulfill_tcp_result<T>(
    promise: *mut TyrusPromise,
    outcome: Result<T, &'static str>,
    kind: TcpResultKind,
) {
    unsafe {
        let value = match outcome {
            Ok(value) => TcpResultFfi {
                tag: RESULT_TAG_OK,
                payload: TcpResultPayload {
                    ok: ManuallyDrop::new(value),
                },
            },
            Err(code) => TcpResultFfi {
                tag: RESULT_TAG_ERR,
                payload: TcpResultPayload {
                    err: ManuallyDrop::new(error_value(code)),
                },
            },
        };
        let result = libc::malloc(std::mem::size_of::<TcpResultFfi<T>>()).cast::<TcpResultFfi<T>>();
        if result.is_null() {
            let mut value = value;
            kind.release(std::ptr::addr_of_mut!(value).cast());
            reject(promise, "ERR_NET_ALLOCATION_FAILED");
            return;
        }
        std::ptr::write(result, value);
        crate::tyrus_promise_fulfill_owned(promise, result.cast(), Some(kind.drop_callback()));
    }
}

unsafe fn timeout_duration(timeout_ms: *const u64) -> Option<std::time::Duration> {
    unsafe {
        timeout_ms
            .as_ref()
            .map(|value| std::time::Duration::from_millis(*value))
    }
}

fn tcp_byte_array(bytes: Vec<u8>) -> Result<TyrusArray, &'static str> {
    let data = if bytes.is_empty() {
        std::ptr::null_mut()
    } else {
        let allocation = unsafe { libc::malloc(bytes.len()).cast::<u8>() };
        if allocation.is_null() {
            return Err("ERR_NET_ALLOCATION_FAILED");
        }
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), allocation, bytes.len()) };
        allocation
    };
    Ok(TyrusArray {
        data,
        length: bytes.len(),
        capacity: bytes.len(),
    })
}

fn tcp_stream_handle(stream: TcpStream) -> *mut TcpStreamHandle {
    let (close_signal, _) = watch::channel(());
    let local_address = stream.local_addr().ok().map(|address| address.to_string());
    let peer_address = stream.peer_addr().ok().map(|address| address.to_string());
    Box::into_raw(Box::new(TcpStreamHandle {
        stream: Arc::new(Mutex::new(Some(stream))),
        local_address,
        peer_address,
        closed: AtomicBool::new(false),
        close_signal,
        references: AtomicUsize::new(1),
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_tcp_listen(
    address: TyrusString,
    timeout_ms: *const u64,
) -> *mut TyrusPromise {
    unsafe {
        let promise = crate::tyrus_promise_pending();
        let Ok(address) = (if address.is_null() {
            Err(())
        } else {
            Ok(address.as_str().to_owned())
        }) else {
            fulfill_tcp_result::<*mut c_void>(
                promise,
                Err("ERR_NET_INVALID_ADDRESS"),
                TcpResultKind::Listener,
            );
            return promise;
        };
        let timeout = timeout_duration(timeout_ms);
        crate::tyrus_promise_retain(promise);
        let promise_address = promise as usize;
        executor().spawn(async move {
            let result = if let Some(timeout) = timeout {
                match tokio::time::timeout(timeout, TcpListener::bind(&address)).await {
                    Ok(result) => result,
                    Err(_) => Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "TCP bind timeout",
                    )),
                }
            } else {
                TcpListener::bind(&address).await
            };
            match result {
                Ok(listener) => {
                    let (close_signal, _) = watch::channel(());
                    let handle = Box::into_raw(Box::new(TcpListenerHandle {
                        listener: Arc::new(Mutex::new(listener)),
                        closed: AtomicBool::new(false),
                        close_signal,
                        references: AtomicUsize::new(1),
                    }));
                    fulfill_tcp_result::<*mut c_void>(
                        promise_address as *mut TyrusPromise,
                        Ok(handle.cast()),
                        TcpResultKind::Listener,
                    );
                }
                Err(error) => {
                    let code = match error.kind() {
                        std::io::ErrorKind::AddrInUse => "ERR_NET_ADDRESS_IN_USE",
                        std::io::ErrorKind::PermissionDenied => "ERR_NET_PERMISSION",
                        std::io::ErrorKind::TimedOut => "ERR_NET_TIMEOUT",
                        _ => "ERR_NET_BIND",
                    };
                    fulfill_tcp_result::<*mut c_void>(
                        promise_address as *mut TyrusPromise,
                        Err(code),
                        TcpResultKind::Listener,
                    );
                }
            }
            crate::tyrus_promise_free(promise_address as *mut TyrusPromise);
        });
        promise
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_tcp_connect(
    address: TyrusString,
    timeout_ms: *const u64,
) -> *mut TyrusPromise {
    unsafe {
        let promise = crate::tyrus_promise_pending();
        let Ok(address) = (if address.is_null() {
            Err(())
        } else {
            Ok(address.as_str().to_owned())
        }) else {
            fulfill_tcp_result::<*mut c_void>(
                promise,
                Err("ERR_NET_INVALID_ADDRESS"),
                TcpResultKind::Stream,
            );
            return promise;
        };
        let timeout = timeout_duration(timeout_ms);
        crate::tyrus_promise_retain(promise);
        let promise_address = promise as usize;
        executor().spawn(async move {
            let result = if let Some(timeout) = timeout {
                match tokio::time::timeout(timeout, TcpStream::connect(&address)).await {
                    Ok(result) => result,
                    Err(_) => Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "TCP connect timeout",
                    )),
                }
            } else {
                TcpStream::connect(&address).await
            };
            let promise = promise_address as *mut TyrusPromise;
            match result {
                Ok(stream) => {
                    let handle = tcp_stream_handle(stream);
                    fulfill_tcp_result::<*mut c_void>(
                        promise,
                        Ok(handle.cast()),
                        TcpResultKind::Stream,
                    );
                }
                Err(error) => {
                    let code = match error.kind() {
                        std::io::ErrorKind::ConnectionRefused => "ERR_NET_CONNECTION_REFUSED",
                        std::io::ErrorKind::PermissionDenied => "ERR_NET_PERMISSION",
                        std::io::ErrorKind::TimedOut => "ERR_NET_TIMEOUT",
                        _ => "ERR_NET_CONNECT",
                    };
                    fulfill_tcp_result::<*mut c_void>(promise, Err(code), TcpResultKind::Stream);
                }
            }
            crate::tyrus_promise_free(promise);
        });
        promise
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_tcp_stream_read(
    handle: *mut c_void,
    max_bytes: u32,
    timeout_ms: *const u64,
) -> *mut TyrusPromise {
    unsafe {
        let promise = crate::tyrus_promise_pending();
        let Some(handle) = handle.cast::<TcpStreamHandle>().as_ref() else {
            fulfill_tcp_result::<TyrusArray>(promise, Err("ERR_NET_CLOSED"), TcpResultKind::Bytes);
            return promise;
        };
        if handle.closed.load(Ordering::Acquire) {
            fulfill_tcp_result::<TyrusArray>(promise, Err("ERR_NET_CLOSED"), TcpResultKind::Bytes);
            return promise;
        }
        let stream = Arc::clone(&handle.stream);
        let mut close_signal = handle.close_signal.subscribe();
        let timeout = timeout_duration(timeout_ms);
        crate::tyrus_promise_retain(promise);
        let promise_address = promise as usize;
        executor().spawn(async move {
            let mut buffer = vec![0_u8; max_bytes as usize];
            let operation = async {
                tokio::select! {
                result = async {
                    let mut stream = stream.lock().await;
                    match stream.as_mut() {
                        Some(stream) => stream.read(&mut buffer).await,
                        None => Err(streamed_error()),
                    }
                } => result,
                _ = close_signal.changed() => Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "TCP stream closed",
                )),
                }
            };
            let result = if let Some(timeout) = timeout {
                match tokio::time::timeout(timeout, operation).await {
                    Ok(result) => result,
                    Err(_) => Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "TCP read timeout",
                    )),
                }
            } else {
                operation.await
            };
            let promise = promise_address as *mut TyrusPromise;
            match result {
                Ok(length) => {
                    buffer.truncate(length);
                    fulfill_tcp_result::<TyrusArray>(
                        promise,
                        tcp_byte_array(buffer),
                        TcpResultKind::Bytes,
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {
                    fulfill_tcp_result::<TyrusArray>(
                        promise,
                        Err("ERR_NET_TIMEOUT"),
                        TcpResultKind::Bytes,
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotConnected => {
                    fulfill_tcp_result::<TyrusArray>(
                        promise,
                        Err("ERR_NET_STREAMED"),
                        TcpResultKind::Bytes,
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {
                    fulfill_tcp_result::<TyrusArray>(
                        promise,
                        Err("ERR_NET_CLOSED"),
                        TcpResultKind::Bytes,
                    );
                }
                Err(_) => fulfill_tcp_result::<TyrusArray>(
                    promise,
                    Err("ERR_NET_READ"),
                    TcpResultKind::Bytes,
                ),
            }
            crate::tyrus_promise_free(promise);
        });
        promise
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_tcp_stream_write(
    handle: *mut c_void,
    data: *const TyrusArray,
    timeout_ms: *const u64,
) -> *mut TyrusPromise {
    unsafe {
        let promise = crate::tyrus_promise_pending();
        let Some(handle) = handle.cast::<TcpStreamHandle>().as_ref() else {
            fulfill_tcp_result::<u64>(promise, Err("ERR_NET_CLOSED"), TcpResultKind::Count);
            return promise;
        };
        let Some(data) = data.as_ref() else {
            fulfill_tcp_result::<u64>(promise, Err("ERR_NET_INVALID_DATA"), TcpResultKind::Count);
            return promise;
        };
        if handle.closed.load(Ordering::Acquire) {
            fulfill_tcp_result::<u64>(promise, Err("ERR_NET_CLOSED"), TcpResultKind::Count);
            return promise;
        }
        if data.length > 0 && data.data.is_null() {
            fulfill_tcp_result::<u64>(promise, Err("ERR_NET_INVALID_DATA"), TcpResultKind::Count);
            return promise;
        }
        let bytes = if data.length == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(data.data, data.length).to_vec()
        };
        let stream = Arc::clone(&handle.stream);
        let mut close_signal = handle.close_signal.subscribe();
        let timeout = timeout_duration(timeout_ms);
        crate::tyrus_promise_retain(promise);
        let promise_address = promise as usize;
        executor().spawn(async move {
            let operation = async {
                tokio::select! {
                result = async {
                    let mut stream = stream.lock().await;
                    match stream.as_mut() {
                        Some(stream) => stream.write(&bytes).await,
                        None => Err(streamed_error()),
                    }
                } => result,
                _ = close_signal.changed() => Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "TCP stream closed",
                )),
                }
            };
            let result = if let Some(timeout) = timeout {
                match tokio::time::timeout(timeout, operation).await {
                    Ok(result) => result,
                    Err(_) => Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "TCP write timeout",
                    )),
                }
            } else {
                operation.await
            };
            let promise = promise_address as *mut TyrusPromise;
            match result {
                Ok(written) => {
                    fulfill_tcp_result::<u64>(promise, Ok(written as u64), TcpResultKind::Count);
                }
                Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {
                    fulfill_tcp_result::<u64>(
                        promise,
                        Err("ERR_NET_TIMEOUT"),
                        TcpResultKind::Count,
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotConnected => {
                    fulfill_tcp_result::<u64>(
                        promise,
                        Err("ERR_NET_STREAMED"),
                        TcpResultKind::Count,
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {
                    fulfill_tcp_result::<u64>(promise, Err("ERR_NET_CLOSED"), TcpResultKind::Count);
                }
                Err(_) => {
                    fulfill_tcp_result::<u64>(promise, Err("ERR_NET_WRITE"), TcpResultKind::Count)
                }
            }
            crate::tyrus_promise_free(promise);
        });
        promise
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_tcp_stream_shutdown(handle: *mut c_void) -> *mut TyrusPromise {
    unsafe {
        let promise = crate::tyrus_promise_pending();
        let Some(handle) = handle.cast::<TcpStreamHandle>().as_ref() else {
            fulfill_tcp_result::<()>(promise, Err("ERR_NET_CLOSED"), TcpResultKind::Unit);
            return promise;
        };
        if handle.closed.load(Ordering::Acquire) {
            fulfill_tcp_result::<()>(promise, Err("ERR_NET_CLOSED"), TcpResultKind::Unit);
            return promise;
        }
        let stream = Arc::clone(&handle.stream);
        crate::tyrus_promise_retain(promise);
        let promise_address = promise as usize;
        executor().spawn(async move {
            let result = match stream.lock().await.as_mut() {
                Some(stream) => stream.shutdown().await,
                None => Err(streamed_error()),
            };
            let promise = promise_address as *mut TyrusPromise;
            match result {
                Ok(()) => fulfill_tcp_result::<()>(promise, Ok(()), TcpResultKind::Unit),
                Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {
                    fulfill_tcp_result::<()>(promise, Err("ERR_NET_TIMEOUT"), TcpResultKind::Unit);
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotConnected => {
                    fulfill_tcp_result::<()>(promise, Err("ERR_NET_STREAMED"), TcpResultKind::Unit);
                }
                Err(_) => {
                    fulfill_tcp_result::<()>(promise, Err("ERR_NET_SHUTDOWN"), TcpResultKind::Unit)
                }
            }
            crate::tyrus_promise_free(promise);
        });
        promise
    }
}

unsafe fn tcp_stream_address(handle: *mut c_void, peer: bool) -> TyrusString {
    unsafe {
        let Some(handle) = handle.cast::<TcpStreamHandle>().as_ref() else {
            return TyrusString::null();
        };
        let address = if peer {
            handle.peer_address.as_deref()
        } else {
            handle.local_address.as_deref()
        };
        address.map_or_else(TyrusString::null, alloc_string)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_tcp_stream_duplex(handle: *mut c_void) -> *mut TyrusPromise {
    unsafe {
        let promise = crate::tyrus_promise_pending();
        let Some(handle) = handle.cast::<TcpStreamHandle>().as_ref() else {
            fulfill_tcp_result::<*mut c_void>(
                promise,
                Err("ERR_NET_CLOSED"),
                TcpResultKind::Duplex,
            );
            return promise;
        };
        if handle.closed.load(Ordering::Acquire) {
            fulfill_tcp_result::<*mut c_void>(
                promise,
                Err("ERR_NET_CLOSED"),
                TcpResultKind::Duplex,
            );
            return promise;
        }
        let taken = match handle.stream.try_lock() {
            Ok(mut stream) => stream.take(),
            Err(_) => {
                fulfill_tcp_result::<*mut c_void>(
                    promise,
                    Err("ERR_NET_BUSY"),
                    TcpResultKind::Duplex,
                );
                return promise;
            }
        };
        let Some(socket) = taken else {
            fulfill_tcp_result::<*mut c_void>(
                promise,
                Err("ERR_NET_STREAMED"),
                TcpResultKind::Duplex,
            );
            return promise;
        };
        let (mut read_half, mut write_half) = socket.into_split();
        let (readable, producer) = crate::stream::byte_readable_stream(TCP_STREAM_CAPACITY_BYTES);
        let (writable, consumer) = crate::stream::byte_writable_stream(TCP_STREAM_CAPACITY_BYTES);
        executor().spawn(async move {
            loop {
                let mut chunk = vec![0_u8; TCP_STREAM_CHUNK_BYTES];
                match read_half.read(&mut chunk).await {
                    Ok(0) => {
                        producer.close();
                        return;
                    }
                    Ok(length) => {
                        chunk.truncate(length);
                        if producer.write(chunk).await.is_err() {
                            return;
                        }
                    }
                    Err(_) => {
                        producer.fail();
                        return;
                    }
                }
            }
        });
        executor().spawn(async move {
            loop {
                match consumer.read().await {
                    Ok(Some(chunk)) => {
                        if write_half.write_all(&chunk).await.is_err() {
                            consumer.fail();
                            return;
                        }
                    }
                    Ok(None) => {
                        match write_half.shutdown().await {
                            Ok(()) => consumer.close(),
                            Err(_) => consumer.fail(),
                        }
                        return;
                    }
                    Err(_) => return,
                }
            }
        });
        let duplex = crate::stream::tyrus_duplex_stream_new(readable, writable);
        crate::stream::tyrus_readable_stream_free(readable);
        crate::stream::tyrus_writable_stream_free(writable);
        fulfill_tcp_result::<*mut c_void>(
            promise,
            Ok(duplex.cast_mut().cast()),
            TcpResultKind::Duplex,
        );
        promise
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_tcp_stream_local_address(handle: *mut c_void) -> TyrusString {
    unsafe { tcp_stream_address(handle, false) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_tcp_stream_peer_address(handle: *mut c_void) -> TyrusString {
    unsafe { tcp_stream_address(handle, true) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_tcp_stream_close(handle: *mut c_void) {
    unsafe {
        if let Some(handle) = handle.cast::<TcpStreamHandle>().as_ref() {
            handle.closed.store(true, Ordering::Release);
            let _ = handle.close_signal.send(());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_tcp_stream_free(handle: *mut c_void) {
    unsafe {
        if !handle.is_null() {
            let stream = handle.cast::<TcpStreamHandle>();
            if release_handle_reference(&(*stream).references) {
                drop(Box::from_raw(stream));
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_tcp_stream_retain(handle: *mut c_void) -> *mut c_void {
    unsafe {
        if !handle.is_null() {
            (*handle.cast::<TcpStreamHandle>())
                .references
                .fetch_add(1, Ordering::Relaxed);
        }
    }
    handle
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_tcp_listener_accept(
    handle: *mut c_void,
    timeout_ms: *const u64,
) -> *mut TyrusPromise {
    unsafe {
        let promise = crate::tyrus_promise_pending();
        let Some(handle) = handle.cast::<TcpListenerHandle>().as_ref() else {
            fulfill_tcp_result::<*mut c_void>(
                promise,
                Err("ERR_NET_CLOSED"),
                TcpResultKind::Stream,
            );
            return promise;
        };
        if handle.closed.load(Ordering::Acquire) {
            fulfill_tcp_result::<*mut c_void>(
                promise,
                Err("ERR_NET_CLOSED"),
                TcpResultKind::Stream,
            );
            return promise;
        }
        let listener = Arc::clone(&handle.listener);
        let mut close_signal = handle.close_signal.subscribe();
        let timeout = timeout_duration(timeout_ms);
        crate::tyrus_promise_retain(promise);
        let promise_address = promise as usize;
        executor().spawn(async move {
            let operation = async {
                tokio::select! {
                result = async {
                    let listener = listener.lock().await;
                    listener.accept().await
                } => result,
                _ = close_signal.changed() => Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "TCP listener closed",
                )),
                }
            };
            let result = if let Some(timeout) = timeout {
                match tokio::time::timeout(timeout, operation).await {
                    Ok(result) => result,
                    Err(_) => Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "TCP accept timeout",
                    )),
                }
            } else {
                operation.await
            };
            let promise = promise_address as *mut TyrusPromise;
            match result {
                Ok((stream, _)) => {
                    let handle = tcp_stream_handle(stream);
                    fulfill_tcp_result::<*mut c_void>(
                        promise,
                        Ok(handle.cast()),
                        TcpResultKind::Stream,
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {
                    fulfill_tcp_result::<*mut c_void>(
                        promise,
                        Err("ERR_NET_TIMEOUT"),
                        TcpResultKind::Stream,
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {
                    fulfill_tcp_result::<*mut c_void>(
                        promise,
                        Err("ERR_NET_CLOSED"),
                        TcpResultKind::Stream,
                    );
                }
                Err(_) => fulfill_tcp_result::<*mut c_void>(
                    promise,
                    Err("ERR_NET_ACCEPT"),
                    TcpResultKind::Stream,
                ),
            }
            crate::tyrus_promise_free(promise);
        });
        promise
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_tcp_listener_local_address(handle: *mut c_void) -> TyrusString {
    unsafe {
        let Some(handle) = handle.cast::<TcpListenerHandle>().as_ref() else {
            return TyrusString::null();
        };
        let Ok(listener) = handle.listener.try_lock() else {
            return TyrusString::null();
        };
        listener
            .local_addr()
            .ok()
            .map_or_else(TyrusString::null, |address| {
                alloc_string(&address.to_string())
            })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_tcp_listener_close(handle: *mut c_void) {
    unsafe {
        if let Some(handle) = handle.cast::<TcpListenerHandle>().as_ref() {
            handle.closed.store(true, Ordering::Release);
            let _ = handle.close_signal.send(());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_tcp_listener_free(handle: *mut c_void) {
    unsafe {
        if !handle.is_null() {
            let listener = handle.cast::<TcpListenerHandle>();
            if release_handle_reference(&(*listener).references) {
                drop(Box::from_raw(listener));
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_tcp_listener_retain(handle: *mut c_void) -> *mut c_void {
    unsafe {
        if !handle.is_null() {
            (*handle.cast::<TcpListenerHandle>())
                .references
                .fetch_add(1, Ordering::Relaxed);
        }
    }
    handle
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream as StdTcpStream};
    use std::thread;
    use tyrus_string::TyrusString;

    #[test]
    fn listener_reference_is_released_only_by_the_last_owner() {
        let references = AtomicUsize::new(2);

        assert!(!release_handle_reference(&references));
        assert_eq!(references.load(Ordering::Relaxed), 1);
        assert!(release_handle_reference(&references));
        assert_eq!(references.load(Ordering::Relaxed), 0);
    }

    unsafe fn fulfilled_handle(promise: *mut TyrusPromise) -> *mut c_void {
        unsafe {
            let result = crate::tyrus_promise_await(promise).cast::<TcpResultFfi<*mut c_void>>();
            assert_eq!((*result).tag, RESULT_TAG_OK);
            *(*result).payload.ok
        }
    }

    unsafe fn fulfilled_bytes(promise: *mut TyrusPromise) -> *const TyrusArray {
        unsafe {
            let result = crate::tyrus_promise_await(promise).cast::<TcpResultFfi<TyrusArray>>();
            assert_eq!((*result).tag, RESULT_TAG_OK);
            std::ptr::addr_of!((*result).payload.ok).cast::<TyrusArray>()
        }
    }

    unsafe fn assert_error_result(promise: *mut TyrusPromise, expected_code: &str) {
        unsafe {
            assert_eq!(crate::promise::wait_until_settled(promise), 1);
            assert_eq!(
                crate::tyrus_promise_state(promise),
                crate::TYRUS_PROMISE_FULFILLED
            );
            let result = crate::tyrus_promise_await(promise).cast::<TcpResultFfi<*mut c_void>>();
            assert_eq!((*result).tag, RESULT_TAG_ERR);
            assert_eq!((*(*result).payload.err).code.as_str(), expected_code);
        }
    }

    fn loopback_bind_is_denied() -> bool {
        match TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => {
                drop(listener);
                false
            }
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                eprintln!(
                    "SKIP: runtime TCP tests require permission to bind a local TCP listener ({error})."
                );
                true
            }
            Err(error) => panic!("could not bind the runtime TCP test listener: {error}"),
        }
    }

    #[test]
    fn every_code_this_file_rejects_with_has_its_own_message() {
        let source = include_str!("net_tcp.rs");
        let mut unmapped = Vec::new();
        for (index, _) in source.match_indices("\"ERR_NET_") {
            let rest = &source[index + 1..];
            let Some(end) = rest.find('"') else {
                continue;
            };
            let code = &rest[..end];
            if code.ends_with('_')
                || !code
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte == b'_')
            {
                continue;
            }
            // `include_str!` hands back a `&'static str`, and a slice of one is
            // still `'static`, so the `&'static str` this asks for is already in
            // hand. Boxing and leaking a copy to get it left one allocation per
            // code behind, which LeakSanitizer reports as a leak because it is one.
            if super::rejection_text(code).0 != code {
                unmapped.push(code);
            }
        }
        assert!(
            unmapped.is_empty(),
            "codes without their own message: {unmapped:?}"
        );
    }

    #[test]
    fn connects_reads_writes_and_closes_a_loopback_stream() {
        if loopback_bind_is_denied() {
            return;
        }
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut input = [0_u8; 4];
            stream.read_exact(&mut input).unwrap();
            assert_eq!(&input, b"ping");
            stream.write_all(b"pong").unwrap();
        });
        let address = crate::strings::alloc_string(&address.to_string());
        let connect = unsafe { tyrus_net_tcp_connect(address.borrow(), std::ptr::null()) };
        unsafe { address.release() };
        unsafe { crate::wait_until_settled(connect) };
        let handle = unsafe { fulfilled_handle(connect).cast::<TcpStreamHandle>() };
        assert!(!handle.is_null());

        let mut input = *b"ping";
        let data = TyrusArray {
            data: input.as_mut_ptr(),
            length: input.len(),
            capacity: input.len(),
        };
        let write = unsafe { tyrus_net_tcp_stream_write(handle.cast(), &data, std::ptr::null()) };
        unsafe { crate::wait_until_settled(write) };
        let written_result =
            unsafe { crate::tyrus_promise_await(write).cast::<TcpResultFfi<u64>>() };
        assert_eq!(unsafe { (*written_result).tag }, RESULT_TAG_OK);
        let written = unsafe { *(*written_result).payload.ok };
        assert_eq!(written, 4);
        unsafe { crate::tyrus_promise_free(write) };

        let read = unsafe { tyrus_net_tcp_stream_read(handle.cast(), 4, std::ptr::null()) };
        unsafe { crate::wait_until_settled(read) };
        let output = unsafe { &*fulfilled_bytes(read) };
        let bytes = unsafe { std::slice::from_raw_parts(output.data, output.length) };
        assert_eq!(bytes, b"pong");
        unsafe { crate::tyrus_promise_free(read) };
        server.join().unwrap();
        let eof = unsafe { tyrus_net_tcp_stream_read(handle.cast(), 4, std::ptr::null()) };
        unsafe { crate::wait_until_settled(eof) };
        let eof_array = unsafe { &*fulfilled_bytes(eof) };
        assert_eq!(eof_array.length, 0);
        unsafe { crate::tyrus_promise_free(eof) };
        unsafe { tyrus_net_tcp_stream_close(handle.cast()) };
        unsafe { crate::tyrus_promise_free(connect) };
    }

    #[tokio::test]
    async fn duplex_stream_half_close_keeps_the_read_side_alive() {
        if loopback_bind_is_denied() {
            return;
        }
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut input = [0_u8; 4];
            stream.read_exact(&mut input).unwrap();
            assert_eq!(&input, b"ping");
            stream.write_all(b"pong").unwrap();
            let mut eof_probe = [0_u8; 1];
            assert_eq!(stream.read(&mut eof_probe).unwrap(), 0);
        });

        let address = crate::strings::alloc_string(&address.to_string());
        let connect = unsafe { tyrus_net_tcp_connect(address.borrow(), std::ptr::null()) };
        unsafe { address.release() };
        unsafe { crate::wait_until_settled(connect) };
        let handle = unsafe { fulfilled_handle(connect).cast::<TcpStreamHandle>() };
        assert!(!handle.is_null());
        let duplex_promise = unsafe { tyrus_net_tcp_stream_duplex(handle.cast()) };
        unsafe { crate::wait_until_settled(duplex_promise) };
        let duplex_result = unsafe {
            crate::tyrus_promise_await(duplex_promise).cast::<TcpResultFfi<*mut c_void>>()
        };
        assert_eq!(unsafe { (*duplex_result).tag }, RESULT_TAG_OK);
        let duplex = unsafe { *(*duplex_result).payload.ok }.cast();

        let readable = unsafe { crate::stream::tyrus_duplex_stream_readable(duplex) };
        let writable = unsafe { crate::stream::tyrus_duplex_stream_writable(duplex) };
        unsafe {
            crate::stream::write_byte_stream(writable, b"ping".to_vec())
                .await
                .unwrap();
            crate::stream::close_byte_writable_stream(writable)
                .await
                .unwrap();
        }
        let response = unsafe { crate::stream::read_byte_stream(readable).await.unwrap() };
        assert_eq!(response, Some(b"pong".to_vec()));
        let eof = unsafe { crate::stream::read_byte_stream(readable).await.unwrap() };
        assert_eq!(eof, None);

        unsafe {
            crate::stream::tyrus_readable_stream_free(readable);
            crate::stream::tyrus_writable_stream_free(writable);
            crate::tyrus_promise_free(duplex_promise);
            tyrus_net_tcp_stream_close(handle.cast());
            crate::tyrus_promise_free(connect);
        }
        server.join().unwrap();
    }

    #[test]
    fn listens_accepts_and_reports_its_local_address() {
        if loopback_bind_is_denied() {
            return;
        }
        let address = TyrusString::literal("127.0.0.1:0");
        let listen = unsafe { tyrus_net_tcp_listen(address.borrow(), std::ptr::null()) };
        unsafe { crate::wait_until_settled(listen) };
        let handle = unsafe { fulfilled_handle(listen).cast::<TcpListenerHandle>() };
        assert!(!handle.is_null());
        let local = unsafe { tyrus_net_tcp_listener_local_address(handle.cast()) };
        assert!(!local.is_null());
        let local = unsafe {
            let text = local.as_str().to_owned();
            local.release();
            text
        };

        let client = thread::spawn(move || {
            let mut stream = StdTcpStream::connect(local).unwrap();
            stream.write_all(b"ping").unwrap();
        });
        let accepted = unsafe { tyrus_net_tcp_listener_accept(handle.cast(), std::ptr::null()) };
        unsafe { crate::wait_until_settled(accepted) };
        let stream = unsafe { fulfilled_handle(accepted).cast::<TcpStreamHandle>() };
        let read = unsafe { tyrus_net_tcp_stream_read(stream.cast(), 4, std::ptr::null()) };
        unsafe { crate::wait_until_settled(read) };
        let output = unsafe { &*fulfilled_bytes(read) };
        let bytes = unsafe { std::slice::from_raw_parts(output.data, output.length) };
        assert_eq!(bytes, b"ping");
        unsafe { crate::tyrus_promise_free(read) };
        for address in [
            unsafe { tyrus_net_tcp_stream_local_address(stream.cast()) },
            unsafe { tyrus_net_tcp_stream_peer_address(stream.cast()) },
        ] {
            assert!(!address.is_null());
            unsafe { address.release() };
        }
        let shutdown = unsafe { tyrus_net_tcp_stream_shutdown(stream.cast()) };
        unsafe { crate::wait_until_settled(shutdown) };
        let shutdown_result =
            unsafe { crate::tyrus_promise_await(shutdown).cast::<TcpResultFfi<()>>() };
        assert_eq!(unsafe { (*shutdown_result).tag }, RESULT_TAG_OK);
        unsafe { crate::tyrus_promise_free(shutdown) };
        unsafe { crate::tyrus_promise_free(accepted) };
        unsafe { tyrus_net_tcp_listener_close(handle.cast()) };
        unsafe { crate::tyrus_promise_free(listen) };
        client.join().unwrap();
    }

    #[test]
    fn closing_a_listener_wakes_a_pending_accept() {
        if loopback_bind_is_denied() {
            return;
        }
        let address = TyrusString::literal("127.0.0.1:0");
        let listen = unsafe { tyrus_net_tcp_listen(address.borrow(), std::ptr::null()) };
        unsafe { crate::wait_until_settled(listen) };
        if unsafe { crate::tyrus_promise_state(listen) } != crate::TYRUS_PROMISE_FULFILLED {
            unsafe { crate::tyrus_promise_free(listen) };
            return;
        }
        let handle = unsafe { fulfilled_handle(listen).cast::<TcpListenerHandle>() };
        let accept = unsafe { tyrus_net_tcp_listener_accept(handle.cast(), std::ptr::null()) };
        unsafe { tyrus_net_tcp_listener_close(handle.cast()) };
        unsafe { crate::wait_until_settled(accept) };
        unsafe { assert_error_result(accept, "ERR_NET_CLOSED") };
        unsafe { crate::tyrus_promise_free(accept) };
        unsafe { crate::tyrus_promise_free(listen) };
    }

    #[test]
    fn closing_a_stream_wakes_a_pending_read() {
        if loopback_bind_is_denied() {
            return;
        }
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            std::thread::sleep(std::time::Duration::from_secs(1));
        });
        let address = crate::strings::alloc_string(&address.to_string());
        let connect = unsafe { tyrus_net_tcp_connect(address.borrow(), std::ptr::null()) };
        unsafe { address.release() };
        unsafe { crate::wait_until_settled(connect) };
        let handle = unsafe { fulfilled_handle(connect).cast::<TcpStreamHandle>() };
        let read = unsafe { tyrus_net_tcp_stream_read(handle.cast(), 16, std::ptr::null()) };
        unsafe { tyrus_net_tcp_stream_close(handle.cast()) };
        unsafe { crate::wait_until_settled(read) };
        unsafe { assert_error_result(read, "ERR_NET_CLOSED") };
        unsafe { crate::tyrus_promise_free(read) };
        unsafe { crate::tyrus_promise_free(connect) };
        server.join().unwrap();
    }

    #[test]
    fn connection_and_listener_failures_are_fulfilled_as_net_error_results() {
        let invalid_address = TyrusString::literal("not a socket address");
        let connect = unsafe { tyrus_net_tcp_connect(invalid_address.borrow(), std::ptr::null()) };
        let listen = unsafe { tyrus_net_tcp_listen(invalid_address.borrow(), std::ptr::null()) };
        unsafe { assert_error_result(connect, "ERR_NET_CONNECT") };
        unsafe { assert_error_result(listen, "ERR_NET_BIND") };
        unsafe {
            crate::tyrus_promise_free(connect);
            crate::tyrus_promise_free(listen);
        }
    }

    #[test]
    fn read_timeout_returns_error_result_without_blocking_the_executor() {
        if loopback_bind_is_denied() {
            return;
        }
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(100));
        });
        let address = crate::strings::alloc_string(&address.to_string());
        let connect = unsafe { tyrus_net_tcp_connect(address.borrow(), std::ptr::null()) };
        unsafe { address.release() };
        unsafe { crate::wait_until_settled(connect) };
        let handle = unsafe { fulfilled_handle(connect).cast::<TcpStreamHandle>() };
        let timeout_ms = 5_u64;
        let read = unsafe { tyrus_net_tcp_stream_read(handle.cast(), 16, &timeout_ms) };
        unsafe { crate::wait_until_settled(read) };
        unsafe { assert_error_result(read, "ERR_NET_TIMEOUT") };
        unsafe { crate::tyrus_promise_free(read) };
        unsafe { tyrus_net_tcp_stream_close(handle.cast()) };
        unsafe { crate::tyrus_promise_free(connect) };
        server.join().unwrap();
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::{AtomicUsize, Ordering, release_handle_reference};

    #[kani::proof]
    fn listener_reference_releases_only_the_last_owner() {
        let references = AtomicUsize::new(2);

        assert!(!release_handle_reference(&references));
        assert_eq!(references.load(Ordering::Relaxed), 1);
        assert!(release_handle_reference(&references));
        assert_eq!(references.load(Ordering::Relaxed), 0);
    }
}
