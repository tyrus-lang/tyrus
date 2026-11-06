use crate::abi::TyrusArray;
use crate::executor::executor;
use crate::promise::TyrusPromise;
use crate::strings::alloc_string;
use std::ffi::c_void;
use std::ffi::{CStr, c_char};
use std::mem::ManuallyDrop;
use std::net::{IpAddr, ToSocketAddrs};
use tyrus_string::TyrusString;

#[repr(C)]
struct NetErrorFfi {
    code: TyrusString,
    message: TyrusString,
}

#[repr(C)]
struct NetResultFfi<T> {
    tag: u8,
    payload: NetResultPayload<T>,
}

#[repr(C)]
union NetResultPayload<T> {
    ok: ManuallyDrop<T>,
    err: ManuallyDrop<NetErrorFfi>,
    words: [u64; 4],
}

const RESULT_TAG_OK: u8 = 0;
const RESULT_TAG_ERR: u8 = 1;

const _: () = {
    assert!(std::mem::size_of::<NetErrorFfi>() == 32);
    assert!(std::mem::size_of::<NetResultFfi<TyrusArray>>() == 40);
    assert!(std::mem::size_of::<NetResultFfi<TyrusString>>() == 40);
    assert!(std::mem::offset_of!(NetResultFfi<TyrusArray>, payload) == 8);
};

unsafe fn timeout_duration(timeout_ms: *const u64) -> Option<std::time::Duration> {
    unsafe {
        timeout_ms
            .as_ref()
            .map(|value| std::time::Duration::from_millis(*value))
    }
}

fn rejection_text(code: &'static str) -> (&'static str, &'static str) {
    match code {
        "ERR_NET_INVALID_HOST" => ("ERR_NET_INVALID_HOST", "the host name is invalid"),
        "ERR_NET_INVALID_ADDRESS" => ("ERR_NET_INVALID_ADDRESS", "the network address is invalid"),
        "ERR_NET_RESOLVE" => ("ERR_NET_RESOLVE", "the host could not be resolved"),
        "ERR_NET_REVERSE_LOOKUP" => ("ERR_NET_REVERSE_LOOKUP", "reverse name lookup failed"),
        "ERR_NET_TIMEOUT" => ("ERR_NET_TIMEOUT", "name resolution timed out"),
        "ERR_NET_UNSUPPORTED" => ("ERR_NET_UNSUPPORTED", "reverse lookup is unavailable"),
        "ERR_NET_ALLOCATION_FAILED" => (
            "ERR_NET_ALLOCATION_FAILED",
            "network memory allocation failed",
        ),
        _ => ("ERR_NET", "the network operation failed"),
    }
}

fn resolve_host(host: &str) -> Result<Vec<String>, &'static str> {
    let mut addresses = Vec::new();
    let resolved = (host, 0).to_socket_addrs().map_err(|_| "ERR_NET_RESOLVE")?;
    for address in resolved {
        let text = address.ip().to_string();
        if !addresses.contains(&text) {
            addresses.push(text);
        }
    }
    if addresses.is_empty() {
        return Err("ERR_NET_RESOLVE");
    }
    Ok(addresses)
}

#[cfg(unix)]
fn reverse_ip(address: &str) -> Result<String, &'static str> {
    use std::mem::size_of;

    let address = address
        .parse::<IpAddr>()
        .map_err(|_| "ERR_NET_INVALID_ADDRESS")?;
    let mut host = [0 as c_char; 1025];
    let (socket, length) = match address {
        IpAddr::V4(address) => {
            let socket = libc::sockaddr_in {
                sin_family: libc::AF_INET as _,
                sin_port: 0,
                sin_addr: libc::in_addr {
                    s_addr: u32::from_ne_bytes(address.octets()),
                },
                sin_zero: [0; 8],
            };
            let length = size_of::<libc::sockaddr_in>() as libc::socklen_t;
            (socket, length)
        }
        IpAddr::V6(address) => {
            let socket = libc::sockaddr_in6 {
                sin6_family: libc::AF_INET6 as _,
                sin6_port: 0,
                sin6_flowinfo: 0,
                sin6_addr: libc::in6_addr {
                    s6_addr: address.octets(),
                },
                sin6_scope_id: 0,
            };
            let mut output = String::new();
            let result = unsafe {
                libc::getnameinfo(
                    (&socket as *const libc::sockaddr_in6).cast(),
                    size_of::<libc::sockaddr_in6>() as libc::socklen_t,
                    host.as_mut_ptr(),
                    host.len() as libc::socklen_t,
                    std::ptr::null_mut(),
                    0,
                    libc::NI_NAMEREQD,
                )
            };
            if result != 0 {
                return Err("ERR_NET_REVERSE_LOOKUP");
            }
            let name = unsafe { CStr::from_ptr(host.as_ptr()) }
                .to_str()
                .map_err(|_| "ERR_NET_REVERSE_LOOKUP")?;
            output.push_str(name);
            return Ok(output);
        }
    };
    let result = unsafe {
        libc::getnameinfo(
            (&socket as *const libc::sockaddr_in).cast(),
            length,
            host.as_mut_ptr(),
            host.len() as libc::socklen_t,
            std::ptr::null_mut(),
            0,
            libc::NI_NAMEREQD,
        )
    };
    if result != 0 {
        return Err("ERR_NET_REVERSE_LOOKUP");
    }
    unsafe { CStr::from_ptr(host.as_ptr()) }
        .to_str()
        .map(str::to_owned)
        .map_err(|_| "ERR_NET_REVERSE_LOOKUP")
}

#[cfg(not(unix))]
#[cfg(not(windows))]
fn reverse_ip(_address: &str) -> Result<String, &'static str> {
    Err("ERR_NET_UNSUPPORTED")
}

#[cfg(windows)]
fn reverse_ip(address: &str) -> Result<String, &'static str> {
    use windows_sys::Win32::Networking::WinSock::{
        AF_INET, AF_INET6, GetNameInfoW, NI_NAMEREQD, SOCKADDR, SOCKADDR_IN, SOCKADDR_IN6,
        SOCKADDR_IN6_0, WSACleanup, WSADATA, WSAStartup,
    };

    let address = address
        .parse::<IpAddr>()
        .map_err(|_| "ERR_NET_INVALID_ADDRESS")?;
    let mut startup_data = WSADATA::default();
    if unsafe { WSAStartup(0x0202, &mut startup_data) } != 0 {
        return Err("ERR_NET_REVERSE_LOOKUP");
    }
    let _winsock = WinsockCleanup;
    let mut host = [0u16; 1025];
    let result = match address {
        IpAddr::V4(address) => {
            let socket = SOCKADDR_IN {
                sin_family: AF_INET,
                sin_port: 0,
                sin_addr: windows_sys::Win32::Networking::WinSock::IN_ADDR {
                    S_un: windows_sys::Win32::Networking::WinSock::IN_ADDR_0 {
                        S_addr: u32::from_ne_bytes(address.octets()),
                    },
                },
                sin_zero: [0; 8],
            };
            unsafe {
                GetNameInfoW(
                    (&socket as *const SOCKADDR_IN).cast::<SOCKADDR>(),
                    std::mem::size_of::<SOCKADDR_IN>() as i32,
                    host.as_mut_ptr(),
                    host.len() as u32,
                    std::ptr::null_mut(),
                    0,
                    NI_NAMEREQD as i32,
                )
            }
        }
        IpAddr::V6(address) => {
            let socket = SOCKADDR_IN6 {
                sin6_family: AF_INET6,
                sin6_port: 0,
                sin6_flowinfo: 0,
                sin6_addr: windows_sys::Win32::Networking::WinSock::IN6_ADDR {
                    u: windows_sys::Win32::Networking::WinSock::IN6_ADDR_0 {
                        Byte: address.octets(),
                    },
                },
                Anonymous: SOCKADDR_IN6_0 { sin6_scope_id: 0 },
            };
            unsafe {
                GetNameInfoW(
                    (&socket as *const SOCKADDR_IN6).cast::<SOCKADDR>(),
                    std::mem::size_of::<SOCKADDR_IN6>() as i32,
                    host.as_mut_ptr(),
                    host.len() as u32,
                    std::ptr::null_mut(),
                    0,
                    NI_NAMEREQD as i32,
                )
            }
        }
    };
    if result != 0 {
        return Err("ERR_NET_REVERSE_LOOKUP");
    }
    let length = host
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(host.len());
    String::from_utf16(&host[..length]).map_err(|_| "ERR_NET_REVERSE_LOOKUP")
}

#[cfg(windows)]
struct WinsockCleanup;

#[cfg(windows)]
impl Drop for WinsockCleanup {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Networking::WinSock::WSACleanup();
        }
    }
}

unsafe fn release_array(array: &mut TyrusArray) {
    unsafe {
        if array.data.is_null() {
            return;
        }
        let strings =
            std::slice::from_raw_parts_mut(array.data.cast::<TyrusString>(), array.length);
        for string in strings {
            std::mem::replace(string, TyrusString::null()).release();
        }
        libc::free(array.data.cast());
        array.data = std::ptr::null_mut();
        array.length = 0;
        array.capacity = 0;
    }
}

unsafe fn array_value(values: Vec<String>) -> Option<TyrusArray> {
    unsafe {
        let capacity = values.len().max(1);
        let bytes = capacity.checked_mul(std::mem::size_of::<TyrusString>())?;
        let data = libc::malloc(bytes).cast::<TyrusString>();
        if data.is_null() {
            return None;
        }
        for (index, value) in values.iter().enumerate() {
            let string = alloc_string(value);
            if string.is_null() {
                let mut array = TyrusArray {
                    data: data.cast(),
                    length: index,
                    capacity,
                };
                release_array(&mut array);
                return None;
            }
            std::ptr::write(data.add(index), string);
        }
        Some(TyrusArray {
            data: data.cast(),
            length: values.len(),
            capacity,
        })
    }
}

fn error_value(code: &'static str) -> NetErrorFfi {
    let (code, message) = rejection_text(code);
    NetErrorFfi {
        code: TyrusString::literal(code),
        message: TyrusString::literal(message),
    }
}

unsafe fn fulfill_array_result(
    promise: *mut TyrusPromise,
    outcome: Result<Vec<String>, &'static str>,
) {
    unsafe {
        let mut value = match outcome
            .and_then(|values| array_value(values).ok_or("ERR_NET_ALLOCATION_FAILED"))
        {
            Ok(array) => NetResultFfi {
                tag: RESULT_TAG_OK,
                payload: NetResultPayload {
                    ok: ManuallyDrop::new(array),
                },
            },
            Err(code) => NetResultFfi {
                tag: RESULT_TAG_ERR,
                payload: NetResultPayload {
                    err: ManuallyDrop::new(error_value(code)),
                },
            },
        };
        let result = libc::malloc(std::mem::size_of::<NetResultFfi<TyrusArray>>())
            .cast::<NetResultFfi<TyrusArray>>();
        if result.is_null() {
            if value.tag == RESULT_TAG_OK {
                release_array(&mut value.payload.ok);
            }
            crate::tyrus_promise_reject_pending_structured(
                promise,
                TyrusString::literal("ERR_NET_ALLOCATION_FAILED"),
                TyrusString::literal("network memory allocation failed"),
            );
            return;
        }
        std::ptr::write(result, value);
        crate::tyrus_promise_fulfill_owned(promise, result.cast(), Some(drop_array_result));
    }
}

unsafe fn fulfill_string_result(promise: *mut TyrusPromise, outcome: Result<String, &'static str>) {
    unsafe {
        let outcome = outcome.and_then(|value| {
            let string = alloc_string(&value);
            if string.is_null() {
                Err("ERR_NET_ALLOCATION_FAILED")
            } else {
                Ok(string)
            }
        });
        let mut value = match outcome {
            Ok(string) => NetResultFfi {
                tag: RESULT_TAG_OK,
                payload: NetResultPayload {
                    ok: ManuallyDrop::new(string),
                },
            },
            Err(code) => NetResultFfi {
                tag: RESULT_TAG_ERR,
                payload: NetResultPayload {
                    err: ManuallyDrop::new(error_value(code)),
                },
            },
        };
        let result = libc::malloc(std::mem::size_of::<NetResultFfi<TyrusString>>())
            .cast::<NetResultFfi<TyrusString>>();
        if result.is_null() {
            if value.tag == RESULT_TAG_OK {
                std::mem::replace(&mut *value.payload.ok, TyrusString::null()).release();
            }
            crate::tyrus_promise_reject_pending_structured(
                promise,
                TyrusString::literal("ERR_NET_ALLOCATION_FAILED"),
                TyrusString::literal("network memory allocation failed"),
            );
            return;
        }
        std::ptr::write(result, value);
        crate::tyrus_promise_fulfill_owned(promise, result.cast(), Some(drop_string_result));
    }
}

unsafe extern "C" fn drop_array_result(value: *mut c_void) {
    unsafe {
        if value.is_null() {
            return;
        }
        let result = value.cast::<NetResultFfi<TyrusArray>>();
        if (*result).tag == RESULT_TAG_OK {
            release_array(&mut (*result).payload.ok);
        }
        libc::free(value);
    }
}

unsafe extern "C" fn drop_string_result(value: *mut c_void) {
    unsafe {
        if value.is_null() {
            return;
        }
        let result = value.cast::<NetResultFfi<TyrusString>>();
        if (*result).tag == RESULT_TAG_OK {
            std::mem::replace(&mut *(*result).payload.ok, TyrusString::null()).release();
        }
        libc::free(value);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_resolve(
    host: TyrusString,
    timeout_ms: *const u64,
) -> *mut TyrusPromise {
    unsafe {
        let promise = crate::tyrus_promise_pending();
        let Ok(host) = (if host.is_null() {
            Err(())
        } else {
            Ok(host.as_str().to_owned())
        }) else {
            crate::tyrus_promise_retain(promise);
            let address = promise as usize;
            executor().spawn(async move {
                let promise = address as *mut TyrusPromise;
                fulfill_array_result(promise, Err("ERR_NET_INVALID_HOST"));
                crate::tyrus_promise_free(promise);
            });
            return promise;
        };
        let timeout = timeout_duration(timeout_ms);
        crate::tyrus_promise_retain(promise);
        let address = promise as usize;
        executor().spawn(async move {
            let lookup = tokio::task::spawn_blocking(move || resolve_host(&host));
            let outcome = if let Some(timeout) = timeout {
                match tokio::time::timeout(timeout, lookup).await {
                    Ok(Ok(Ok(values))) => Ok(values),
                    Ok(Ok(Err(code))) => Err(code),
                    Ok(Err(_)) => Err("ERR_NET_RESOLVE"),
                    Err(_) => Err("ERR_NET_TIMEOUT"),
                }
            } else {
                match lookup.await {
                    Ok(Ok(values)) => Ok(values),
                    Ok(Err(code)) => Err(code),
                    Err(_) => Err("ERR_NET_RESOLVE"),
                }
            };
            let promise = address as *mut TyrusPromise;
            fulfill_array_result(promise, outcome);
            crate::tyrus_promise_free(promise);
        });
        promise
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_reverse_lookup(
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
            crate::tyrus_promise_retain(promise);
            let address = promise as usize;
            executor().spawn(async move {
                let promise = address as *mut TyrusPromise;
                fulfill_string_result(promise, Err("ERR_NET_INVALID_ADDRESS"));
                crate::tyrus_promise_free(promise);
            });
            return promise;
        };
        let timeout = timeout_duration(timeout_ms);
        crate::tyrus_promise_retain(promise);
        let promise_address = promise as usize;
        executor().spawn(async move {
            let lookup = tokio::task::spawn_blocking(move || reverse_ip(&address));
            let outcome = if let Some(timeout) = timeout {
                match tokio::time::timeout(timeout, lookup).await {
                    Ok(Ok(Ok(name))) => Ok(name),
                    Ok(Ok(Err(code))) => Err(code),
                    Ok(Err(_)) => Err("ERR_NET_REVERSE_LOOKUP"),
                    Err(_) => Err("ERR_NET_TIMEOUT"),
                }
            } else {
                match lookup.await {
                    Ok(Ok(name)) => Ok(name),
                    Ok(Err(code)) => Err(code),
                    Err(_) => Err("ERR_NET_REVERSE_LOOKUP"),
                }
            };
            let promise = promise_address as *mut TyrusPromise;
            fulfill_string_result(promise, outcome);
            crate::tyrus_promise_free(promise);
        });
        promise
    }
}

#[cfg(test)]
mod tests {
    use super::{NetResultFfi, RESULT_TAG_ERR, RESULT_TAG_OK, TyrusArray, tyrus_net_resolve};
    use crate::promise::{tyrus_promise_await, tyrus_promise_free};
    use tyrus_string::TyrusString;

    #[test]
    fn every_network_error_code_has_its_own_message() {
        let source = include_str!("net_resolve.rs");
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
    fn resolves_localhost_into_a_result_without_blocking_the_calling_thread() {
        let host = TyrusString::literal("localhost");
        let promise = unsafe { tyrus_net_resolve(host.borrow(), std::ptr::null()) };
        unsafe { crate::wait_until_settled(promise) };
        let payload = unsafe { tyrus_promise_await(promise).cast::<NetResultFfi<TyrusArray>>() };
        assert!(!payload.is_null());
        assert_eq!(unsafe { (*payload).tag }, RESULT_TAG_OK);
        unsafe { tyrus_promise_free(promise) };
    }

    #[test]
    fn fulfills_invalid_reverse_lookup_addresses_with_net_error() {
        let address = TyrusString::literal("not-an-ip");
        let promise =
            unsafe { super::tyrus_net_reverse_lookup(address.borrow(), std::ptr::null()) };
        unsafe { crate::wait_until_settled(promise) };
        assert_eq!(
            unsafe { crate::tyrus_promise_state(promise) },
            crate::TYRUS_PROMISE_FULFILLED
        );
        let result = unsafe { tyrus_promise_await(promise).cast::<NetResultFfi<TyrusString>>() };
        assert_eq!(unsafe { (*result).tag }, RESULT_TAG_ERR);
        assert_eq!(
            unsafe { (*(*result).payload.err).code.as_str() },
            "ERR_NET_INVALID_ADDRESS"
        );
        unsafe { crate::tyrus_promise_free(promise) };
    }
}
