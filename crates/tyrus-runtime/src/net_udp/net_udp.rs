use crate::abi::TyrusArray;
use crate::executor::executor;
use crate::promise::TyrusPromise;
use crate::strings::alloc_string;
use std::ffi::c_void;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::net::UdpSocket;
use tokio::sync::{Mutex, watch};
use tyrus_string::TyrusString;

const MAX_UDP_DATAGRAM_BYTES: usize = 65_507;

pub struct UdpSocketHandle {
    socket: Arc<Mutex<UdpSocket>>,
    closed: AtomicBool,
    close_signal: watch::Sender<()>,
}

#[repr(C)]
pub struct TyrusUdpPacket {
    pub data: TyrusArray,
    pub address: TyrusString,
}

#[repr(C)]
struct UdpErrorFfi {
    code: TyrusString,
    message: TyrusString,
}

#[repr(C)]
struct UdpResultFfi<T> {
    tag: u8,
    payload: UdpResultPayload<T>,
}

#[repr(C)]
union UdpResultPayload<T> {
    ok: std::mem::ManuallyDrop<T>,
    err: std::mem::ManuallyDrop<UdpErrorFfi>,
    words: [u64; 5],
}

const UDP_RESULT_OK: u8 = 0;
const UDP_RESULT_ERR: u8 = 1;

const _: () = {
    assert!(std::mem::size_of::<UdpErrorFfi>() == 32);
    assert!(std::mem::size_of::<TyrusUdpPacket>() == 40);
    assert!(std::mem::size_of::<UdpResultFfi<*mut c_void>>() == 48);
    assert!(std::mem::size_of::<UdpResultFfi<TyrusUdpPacket>>() == 48);
    assert!(std::mem::size_of::<UdpResultFfi<u64>>() == 48);
    assert!(std::mem::size_of::<UdpResultFfi<()>>() == 48);
    assert!(std::mem::offset_of!(UdpResultFfi<*mut c_void>, payload) == 8);
};

fn rejection_text(code: &'static str) -> (&'static str, &'static str) {
    match code {
        "ERR_NET_CLOSED" => ("ERR_NET_CLOSED", "the UDP socket is closed"),
        "ERR_NET_PERMISSION" => (
            "ERR_NET_PERMISSION",
            "the operating system denied the UDP operation",
        ),
        "ERR_NET_INVALID_TTL" => ("ERR_NET_INVALID_TTL", "UDP TTL must be between 1 and 255"),
        "ERR_NET_OPTION" => ("ERR_NET_OPTION", "the UDP socket option is not supported"),
        "ERR_NET_TIMEOUT" => ("ERR_NET_TIMEOUT", "the UDP operation timed out"),
        "ERR_NET_INVALID_ADDRESS" => ("ERR_NET_INVALID_ADDRESS", "the network address is invalid"),
        "ERR_NET_INVALID_DATA" => ("ERR_NET_INVALID_DATA", "the UDP datagram is invalid"),
        "ERR_NET_ALLOCATION_FAILED" => (
            "ERR_NET_ALLOCATION_FAILED",
            "network memory allocation failed",
        ),
        "ERR_NET_BIND" => ("ERR_NET_BIND", "the UDP socket could not bind"),
        "ERR_NET_RESOLVE" => ("ERR_NET_RESOLVE", "the host could not be resolved"),
        "ERR_NET_SEND" => ("ERR_NET_SEND", "the UDP datagram could not be sent"),
        "ERR_NET_RECEIVE" => ("ERR_NET_RECEIVE", "the UDP datagram could not be received"),
        "ERR_NET_UNSUPPORTED" => ("ERR_NET_UNSUPPORTED", "the network probe is unavailable"),
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

#[derive(Clone, Copy)]
enum UdpResultKind {
    Socket,
    Packet,
    Count,
    Unit,
}

impl UdpResultKind {
    unsafe fn release(self, value: *mut c_void) {
        unsafe {
            let result = value.cast::<UdpResultFfi<*mut c_void>>();
            if (*result).tag == UDP_RESULT_ERR {
                let error = &mut *(*result).payload.err;
                std::mem::replace(&mut error.code, TyrusString::null()).release();
                std::mem::replace(&mut error.message, TyrusString::null()).release();
            } else {
                match self {
                    Self::Socket => {
                        let handle = *(*result).payload.ok;
                        if !handle.is_null() {
                            drop(Box::from_raw(handle.cast::<UdpSocketHandle>()));
                        }
                    }
                    Self::Packet => {
                        let result = value.cast::<UdpResultFfi<TyrusUdpPacket>>();
                        let packet = &mut *(*result).payload.ok;
                        if !packet.data.data.is_null() {
                            libc::free(packet.data.data.cast());
                            packet.data.data = std::ptr::null_mut();
                            packet.data.length = 0;
                            packet.data.capacity = 0;
                        }
                        std::mem::replace(&mut packet.address, TyrusString::null()).release();
                    }
                    Self::Count | Self::Unit => {}
                }
            }
        }
    }

    fn drop_callback(self) -> unsafe extern "C" fn(*mut c_void) {
        match self {
            Self::Socket => drop_udp_socket_result,
            Self::Packet => drop_udp_packet_result,
            Self::Count => drop_udp_count_result,
            Self::Unit => drop_udp_unit_result,
        }
    }
}

unsafe extern "C" fn drop_udp_socket_result(value: *mut c_void) {
    unsafe { drop_udp_result(value, UdpResultKind::Socket) }
}

unsafe extern "C" fn drop_udp_packet_result(value: *mut c_void) {
    unsafe { drop_udp_result(value, UdpResultKind::Packet) }
}

unsafe extern "C" fn drop_udp_count_result(value: *mut c_void) {
    unsafe { drop_udp_result(value, UdpResultKind::Count) }
}

unsafe extern "C" fn drop_udp_unit_result(value: *mut c_void) {
    unsafe { drop_udp_result(value, UdpResultKind::Unit) }
}

unsafe fn drop_udp_result(value: *mut c_void, kind: UdpResultKind) {
    unsafe {
        if !value.is_null() {
            kind.release(value);
            libc::free(value);
        }
    }
}

unsafe fn fulfill_udp_result<T>(
    promise: *mut TyrusPromise,
    outcome: Result<T, &'static str>,
    kind: UdpResultKind,
) {
    unsafe {
        let value = match outcome {
            Ok(value) => UdpResultFfi {
                tag: UDP_RESULT_OK,
                payload: UdpResultPayload {
                    ok: std::mem::ManuallyDrop::new(value),
                },
            },
            Err(code) => {
                let (code, message) = rejection_text(code);
                UdpResultFfi {
                    tag: UDP_RESULT_ERR,
                    payload: UdpResultPayload {
                        err: std::mem::ManuallyDrop::new(UdpErrorFfi {
                            code: TyrusString::literal(code),
                            message: TyrusString::literal(message),
                        }),
                    },
                }
            }
        };
        let result = libc::malloc(std::mem::size_of::<UdpResultFfi<T>>()).cast::<UdpResultFfi<T>>();
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_udp_bind(address: TyrusString) -> *mut TyrusPromise {
    unsafe {
        let promise = crate::tyrus_promise_pending();
        let Ok(address) = (if address.is_null() {
            Err(())
        } else {
            Ok(address.as_str().to_owned())
        }) else {
            fulfill_udp_result::<*mut c_void>(
                promise,
                Err("ERR_NET_INVALID_ADDRESS"),
                UdpResultKind::Socket,
            );
            return promise;
        };
        crate::tyrus_promise_retain(promise);
        let promise_address = promise as usize;
        executor().spawn(async move {
            match UdpSocket::bind(&address).await {
                Ok(socket) => {
                    let (close_signal, _) = watch::channel(());
                    let handle = Box::into_raw(Box::new(UdpSocketHandle {
                        socket: Arc::new(Mutex::new(socket)),
                        closed: AtomicBool::new(false),
                        close_signal,
                    }));
                    fulfill_udp_result::<*mut c_void>(
                        promise_address as *mut TyrusPromise,
                        Ok(handle.cast()),
                        UdpResultKind::Socket,
                    );
                }
                Err(error) => {
                    let code = match error.kind() {
                        std::io::ErrorKind::PermissionDenied => "ERR_NET_PERMISSION",
                        std::io::ErrorKind::InvalidInput => "ERR_NET_INVALID_ADDRESS",
                        _ => "ERR_NET_BIND",
                    };
                    fulfill_udp_result::<*mut c_void>(
                        promise_address as *mut TyrusPromise,
                        Err(code),
                        UdpResultKind::Socket,
                    );
                }
            }
            crate::tyrus_promise_free(promise_address as *mut TyrusPromise);
        });
        promise
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_udp_send_to(
    handle: *mut c_void,
    data: *const TyrusArray,
    address: TyrusString,
    timeout_ms: *const u64,
) -> *mut TyrusPromise {
    unsafe {
        let promise = crate::tyrus_promise_pending();
        let Some(handle) = handle.cast::<UdpSocketHandle>().as_ref() else {
            fulfill_udp_result::<u64>(promise, Err("ERR_NET_CLOSED"), UdpResultKind::Count);
            return promise;
        };
        let Some(data) = data.as_ref() else {
            fulfill_udp_result::<u64>(promise, Err("ERR_NET_INVALID_DATA"), UdpResultKind::Count);
            return promise;
        };
        let Ok(address) = (if address.is_null() {
            Err(())
        } else {
            Ok(address.as_str().to_owned())
        }) else {
            fulfill_udp_result::<u64>(
                promise,
                Err("ERR_NET_INVALID_ADDRESS"),
                UdpResultKind::Count,
            );
            return promise;
        };
        if handle.closed.load(Ordering::Acquire) {
            fulfill_udp_result::<u64>(promise, Err("ERR_NET_CLOSED"), UdpResultKind::Count);
            return promise;
        }
        if data.length > 0 && data.data.is_null() {
            fulfill_udp_result::<u64>(promise, Err("ERR_NET_INVALID_DATA"), UdpResultKind::Count);
            return promise;
        }
        if data.length > MAX_UDP_DATAGRAM_BYTES {
            fulfill_udp_result::<u64>(promise, Err("ERR_NET_INVALID_DATA"), UdpResultKind::Count);
            return promise;
        }
        let bytes = if data.length == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(data.data.cast::<u8>(), data.length).to_vec()
        };
        let socket = Arc::clone(&handle.socket);
        let mut close_signal = handle.close_signal.subscribe();
        let timeout = timeout_duration(timeout_ms);
        crate::tyrus_promise_retain(promise);
        let promise_address = promise as usize;
        executor().spawn(async move { let operation = async { tokio::select! { result = async { socket.lock().await.send_to(&bytes, &address).await } => result, changed = close_signal.changed() => { let _ = changed; Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "UDP socket closed")) } } }; let result = if let Some(timeout) = timeout { tokio::time::timeout(timeout, operation).await.unwrap_or_else(|_| Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "UDP send timeout"))) } else { operation.await }; let promise = promise_address as *mut TyrusPromise; match result { Ok(sent) => fulfill_udp_result::<u64>(promise, Ok(sent as u64), UdpResultKind::Count), Err(error) if error.kind() == std::io::ErrorKind::TimedOut => fulfill_udp_result::<u64>(promise, Err("ERR_NET_TIMEOUT"), UdpResultKind::Count), Err(error) if error.kind() == std::io::ErrorKind::Interrupted => fulfill_udp_result::<u64>(promise, Err("ERR_NET_CLOSED"), UdpResultKind::Count), Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => fulfill_udp_result::<u64>(promise, Err("ERR_NET_INVALID_ADDRESS"), UdpResultKind::Count), Err(_) => fulfill_udp_result::<u64>(promise, Err("ERR_NET_SEND"), UdpResultKind::Count) } crate::tyrus_promise_free(promise); });
        promise
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_udp_receive_from(
    handle: *mut c_void,
    max_bytes: u32,
    timeout_ms: *const u64,
) -> *mut TyrusPromise {
    unsafe {
        let promise = crate::tyrus_promise_pending();
        let Some(handle) = handle.cast::<UdpSocketHandle>().as_ref() else {
            fulfill_udp_result::<TyrusUdpPacket>(
                promise,
                Err("ERR_NET_CLOSED"),
                UdpResultKind::Packet,
            );
            return promise;
        };
        if handle.closed.load(Ordering::Acquire) {
            fulfill_udp_result::<TyrusUdpPacket>(
                promise,
                Err("ERR_NET_CLOSED"),
                UdpResultKind::Packet,
            );
            return promise;
        }
        if max_bytes as usize > MAX_UDP_DATAGRAM_BYTES {
            fulfill_udp_result::<TyrusUdpPacket>(
                promise,
                Err("ERR_NET_INVALID_DATA"),
                UdpResultKind::Packet,
            );
            return promise;
        }
        let socket = Arc::clone(&handle.socket);
        let mut close_signal = handle.close_signal.subscribe();
        let timeout = timeout_duration(timeout_ms);
        crate::tyrus_promise_retain(promise);
        let promise_address = promise as usize;
        executor().spawn(async move { let mut buffer = vec![0_u8; max_bytes as usize]; let operation = async { tokio::select! { result = async { socket.lock().await.recv_from(&mut buffer).await } => result, changed = close_signal.changed() => { let _ = changed; Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "UDP socket closed")) } } }; let result = if let Some(timeout) = timeout { tokio::time::timeout(timeout, operation).await.unwrap_or_else(|_| Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "UDP receive timeout"))) } else { operation.await }; let promise = promise_address as *mut TyrusPromise; match result { Ok((length, address)) => { buffer.truncate(length); let data: *mut u8 = if buffer.is_empty() { std::ptr::null_mut() } else { let allocation = libc::malloc(buffer.len()).cast::<u8>(); if allocation.is_null() { fulfill_udp_result::<TyrusUdpPacket>(promise, Err("ERR_NET_ALLOCATION_FAILED"), UdpResultKind::Packet); crate::tyrus_promise_free(promise); return; } std::ptr::copy_nonoverlapping(buffer.as_ptr(), allocation, buffer.len()); allocation }; let address = alloc_string(&address.to_string()); if address.is_null() { if !data.is_null() { libc::free(data.cast()); } fulfill_udp_result::<TyrusUdpPacket>(promise, Err("ERR_NET_ALLOCATION_FAILED"), UdpResultKind::Packet); crate::tyrus_promise_free(promise); return; } let packet = TyrusUdpPacket { data: TyrusArray { data, length: buffer.len(), capacity: buffer.len() }, address }; fulfill_udp_result::<TyrusUdpPacket>(promise, Ok(packet), UdpResultKind::Packet); }, Err(error) if error.kind() == std::io::ErrorKind::TimedOut => fulfill_udp_result::<TyrusUdpPacket>(promise, Err("ERR_NET_TIMEOUT"), UdpResultKind::Packet), Err(error) if error.kind() == std::io::ErrorKind::Interrupted => fulfill_udp_result::<TyrusUdpPacket>(promise, Err("ERR_NET_CLOSED"), UdpResultKind::Packet), Err(_) => fulfill_udp_result::<TyrusUdpPacket>(promise, Err("ERR_NET_RECEIVE"), UdpResultKind::Packet) } crate::tyrus_promise_free(promise); });
        promise
    }
}

unsafe fn udp_option_result(
    handle: *mut c_void,
    operation: impl FnOnce(&UdpSocket) -> std::io::Result<()> + Send + 'static,
) -> *mut TyrusPromise {
    unsafe {
        let promise = crate::tyrus_promise_pending();
        let Some(handle) = handle.cast::<UdpSocketHandle>().as_ref() else {
            fulfill_udp_result::<()>(promise, Err("ERR_NET_CLOSED"), UdpResultKind::Unit);
            return promise;
        };
        if handle.closed.load(Ordering::Acquire) {
            fulfill_udp_result::<()>(promise, Err("ERR_NET_CLOSED"), UdpResultKind::Unit);
            return promise;
        }
        let socket = Arc::clone(&handle.socket);
        let mut close_signal = handle.close_signal.subscribe();
        crate::tyrus_promise_retain(promise);
        let promise_address = promise as usize;
        executor().spawn(async move {
        let result = tokio::select! {
            result = async {
                let socket = socket.lock().await;
                operation(&socket)
            } => result,
            _ = close_signal.changed() => Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "UDP socket closed")),
        };
        let promise = promise_address as *mut TyrusPromise;
        match result {
            Ok(()) => {
                fulfill_udp_result::<()>(promise, Ok(()), UdpResultKind::Unit);
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {
                fulfill_udp_result::<()>(promise, Err("ERR_NET_CLOSED"), UdpResultKind::Unit);
            }
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                fulfill_udp_result::<()>(promise, Err("ERR_NET_PERMISSION"), UdpResultKind::Unit);
            }
            Err(_) => fulfill_udp_result::<()>(promise, Err("ERR_NET_OPTION"), UdpResultKind::Unit),
        }
        crate::tyrus_promise_free(promise);
    });
        promise
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_udp_set_broadcast(
    handle: *mut c_void,
    enabled: bool,
) -> *mut TyrusPromise {
    unsafe { udp_option_result(handle, move |socket| socket.set_broadcast(enabled)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_udp_set_ttl(handle: *mut c_void, ttl: u32) -> *mut TyrusPromise {
    unsafe {
        if !(1..=255).contains(&ttl) {
            let promise = crate::tyrus_promise_pending();
            fulfill_udp_result::<()>(promise, Err("ERR_NET_INVALID_TTL"), UdpResultKind::Unit);
            return promise;
        }
        udp_option_result(handle, move |socket| socket.set_ttl(ttl))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_udp_local_address(handle: *mut c_void) -> TyrusString {
    unsafe {
        let Some(handle) = handle.cast::<UdpSocketHandle>().as_ref() else {
            return TyrusString::null();
        };
        let Ok(socket) = handle.socket.try_lock() else {
            return TyrusString::null();
        };
        socket
            .local_addr()
            .ok()
            .map_or_else(TyrusString::null, |address| {
                alloc_string(&address.to_string())
            })
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_udp_close(handle: *mut c_void) {
    unsafe {
        if let Some(handle) = handle.cast::<UdpSocketHandle>().as_ref() {
            handle.closed.store(true, Ordering::Release);
            let _ = handle.close_signal.send(());
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_udp_socket_free(handle: *mut c_void) {
    unsafe {
        if !handle.is_null() {
            drop(Box::from_raw(handle.cast::<UdpSocketHandle>()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tyrus_string::TyrusString;

    unsafe fn assert_error_result<T>(promise: *mut TyrusPromise, expected_code: &str) {
        unsafe {
            crate::wait_until_settled(promise);
            assert_eq!(
                crate::tyrus_promise_state(promise),
                crate::TYRUS_PROMISE_FULFILLED
            );
            let result = crate::tyrus_promise_await(promise).cast::<UdpResultFfi<T>>();
            assert_eq!((*result).tag, UDP_RESULT_ERR);
            assert_eq!((*(*result).payload.err).code.as_str(), expected_code);
        }
    }

    #[test]
    fn every_code_this_file_rejects_with_has_its_own_message() {
        let source = include_str!("net_udp.rs");
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

    unsafe fn promise_handle(promise: *mut TyrusPromise) -> Option<*mut UdpSocketHandle> {
        unsafe {
            crate::wait_until_settled(promise);
            if crate::tyrus_promise_state(promise) != crate::TYRUS_PROMISE_FULFILLED {
                return None;
            }
            let result = crate::tyrus_promise_await(promise).cast::<UdpResultFfi<*mut c_void>>();
            if (*result).tag != UDP_RESULT_OK {
                return None;
            }
            Some((*(*result).payload.ok).cast::<UdpSocketHandle>())
        }
    }

    #[test]
    fn sends_and_receives_a_loopback_datagram() {
        let bind_address = TyrusString::literal("127.0.0.1:0");
        let first = unsafe { tyrus_net_udp_bind(bind_address.borrow()) };
        let Some(first_handle) = (unsafe { promise_handle(first) }) else {
            unsafe { assert_error_result::<*mut c_void>(first, "ERR_NET_PERMISSION") };
            unsafe { crate::tyrus_promise_free(first) };
            return;
        };
        let first_address = unsafe { tyrus_net_udp_local_address(first_handle.cast()) };
        let first_address = unsafe {
            let value = first_address.as_str().to_owned();
            first_address.release();
            value
        };

        let second = unsafe { tyrus_net_udp_bind(bind_address.borrow()) };
        let Some(second_handle) = (unsafe { promise_handle(second) }) else {
            unsafe { assert_error_result::<*mut c_void>(second, "ERR_NET_PERMISSION") };
            unsafe {
                crate::tyrus_promise_free(second);
                tyrus_net_udp_close(first_handle.cast());
                crate::tyrus_promise_free(first);
            }
            return;
        };
        let second_address = unsafe { tyrus_net_udp_local_address(second_handle.cast()) };
        let second_address = unsafe {
            let value = second_address.as_str().to_owned();
            second_address.release();
            value
        };

        let mut bytes = *b"ping";
        let data = TyrusArray {
            data: bytes.as_mut_ptr(),
            length: bytes.len(),
            capacity: bytes.len(),
        };
        let destination = crate::strings::alloc_string(&second_address);
        let send = unsafe {
            tyrus_net_udp_send_to(
                first_handle.cast(),
                &data,
                destination.borrow(),
                std::ptr::null(),
            )
        };
        unsafe { destination.release() };
        unsafe { crate::wait_until_settled(send) };
        let send_result = unsafe { crate::tyrus_promise_await(send).cast::<UdpResultFfi<u64>>() };
        assert_eq!(unsafe { (*send_result).tag }, UDP_RESULT_OK);
        assert_eq!(unsafe { *(*send_result).payload.ok }, 4);

        let receive =
            unsafe { tyrus_net_udp_receive_from(second_handle.cast(), 16, std::ptr::null()) };
        unsafe { crate::wait_until_settled(receive) };
        let packet_result =
            unsafe { crate::tyrus_promise_await(receive).cast::<UdpResultFfi<TyrusUdpPacket>>() };
        assert_eq!(unsafe { (*packet_result).tag }, UDP_RESULT_OK);
        let packet =
            unsafe { &*std::ptr::addr_of!((*packet_result).payload.ok).cast::<TyrusUdpPacket>() };
        let received = unsafe { std::slice::from_raw_parts(packet.data.data, packet.data.length) };
        assert_eq!(received, b"ping");
        assert_eq!(unsafe { packet.address.as_str() }, first_address);

        unsafe {
            crate::tyrus_promise_free(receive);
            crate::tyrus_promise_free(send);
            tyrus_net_udp_close(first_handle.cast());
            tyrus_net_udp_close(second_handle.cast());
            crate::tyrus_promise_free(first);
            crate::tyrus_promise_free(second);
        }
    }

    #[test]
    fn receive_timeout_and_close_use_stable_errors() {
        let bind_address = TyrusString::literal("127.0.0.1:0");
        let bind = unsafe { tyrus_net_udp_bind(bind_address.borrow()) };
        let Some(handle) = (unsafe { promise_handle(bind) }) else {
            unsafe { assert_error_result::<*mut c_void>(bind, "ERR_NET_PERMISSION") };
            unsafe { crate::tyrus_promise_free(bind) };
            return;
        };
        let timeout = 5_u64;
        let receive = unsafe { tyrus_net_udp_receive_from(handle.cast(), 16, &timeout) };
        unsafe { crate::wait_until_settled(receive) };
        unsafe { assert_error_result::<TyrusUdpPacket>(receive, "ERR_NET_TIMEOUT") };
        unsafe { crate::tyrus_promise_free(receive) };

        let pending = unsafe { tyrus_net_udp_receive_from(handle.cast(), 16, std::ptr::null()) };
        unsafe { tyrus_net_udp_close(handle.cast()) };
        unsafe { crate::wait_until_settled(pending) };
        unsafe { assert_error_result::<TyrusUdpPacket>(pending, "ERR_NET_CLOSED") };
        unsafe {
            crate::tyrus_promise_free(pending);
            crate::tyrus_promise_free(bind);
        }
    }

    #[test]
    fn configures_broadcast_and_ttl_on_a_loopback_socket() {
        let bind_address = TyrusString::literal("127.0.0.1:0");
        let bind = unsafe { tyrus_net_udp_bind(bind_address.borrow()) };
        let Some(handle) = (unsafe { promise_handle(bind) }) else {
            unsafe { crate::tyrus_promise_free(bind) };
            return;
        };
        let broadcast = unsafe { tyrus_net_udp_set_broadcast(handle.cast(), true) };
        unsafe { crate::wait_until_settled(broadcast) };
        assert_eq!(
            unsafe { crate::tyrus_promise_state(broadcast) },
            crate::TYRUS_PROMISE_FULFILLED
        );
        let ttl = unsafe { tyrus_net_udp_set_ttl(handle.cast(), 1) };
        unsafe { crate::wait_until_settled(ttl) };
        assert_eq!(
            unsafe { crate::tyrus_promise_state(ttl) },
            crate::TYRUS_PROMISE_FULFILLED
        );
        let invalid_ttl = unsafe { tyrus_net_udp_set_ttl(handle.cast(), 0) };
        unsafe { crate::wait_until_settled(invalid_ttl) };
        unsafe { assert_error_result::<()>(invalid_ttl, "ERR_NET_INVALID_TTL") };
        unsafe {
            crate::tyrus_promise_free(broadcast);
            crate::tyrus_promise_free(ttl);
            crate::tyrus_promise_free(invalid_ttl);
            tyrus_net_udp_close(handle.cast());
            crate::tyrus_promise_free(bind);
        }
    }
}
