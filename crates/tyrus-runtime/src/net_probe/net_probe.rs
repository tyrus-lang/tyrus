use crate::abi::TyrusArray;
use crate::executor::executor;
use crate::promise::TyrusPromise;
use crate::strings::alloc_string;
use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};
use tyrus_string::TyrusString;

#[repr(C)]
pub(crate) struct ProbeErrorFfi {
    pub(crate) code: TyrusString,
    pub(crate) message: TyrusString,
}

#[repr(C)]
pub(crate) struct ProbeResultFfi<T> {
    pub(crate) tag: u8,
    pub(crate) payload: ProbeResultPayload<T>,
}

#[repr(C)]
pub(crate) union ProbeResultPayload<T> {
    pub(crate) ok: ManuallyDrop<T>,
    pub(crate) err: ManuallyDrop<ProbeErrorFfi>,
    words: [u64; 4],
}

const RESULT_TAG_OK: u8 = 0;
const RESULT_TAG_ERR: u8 = 1;

const _: () = {
    assert!(std::mem::size_of::<ProbeErrorFfi>() == 32);
    assert!(std::mem::size_of::<ProbeResultFfi<u64>>() == 40);
    assert!(std::mem::size_of::<ProbeResultFfi<TyrusArray>>() == 40);
    assert!(std::mem::offset_of!(ProbeResultFfi<u64>, payload) == 8);
};

fn rejection_text(code: &'static str) -> (&'static str, &'static str) {
    match code {
        "ERR_NET_PERMISSION" => (
            "ERR_NET_PERMISSION",
            "the operating system denied the ICMP operation",
        ),
        "ERR_NET_TIMEOUT" => ("ERR_NET_TIMEOUT", "the network probe timed out"),
        "ERR_NET_INVALID_ADDRESS" => ("ERR_NET_INVALID_ADDRESS", "the network address is invalid"),
        "ERR_NET_RESOLVE" => (
            "ERR_NET_RESOLVE",
            "the probe destination could not be resolved",
        ),
        "ERR_NET_UNSUPPORTED" => (
            "ERR_NET_UNSUPPORTED",
            "ICMP probes are unavailable on this platform",
        ),
        "ERR_NET_PROBE" => ("ERR_NET_PROBE", "the network probe failed"),
        "ERR_NET_ALLOCATION_FAILED" => (
            "ERR_NET_ALLOCATION_FAILED",
            "network memory allocation failed",
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

fn probe_timeout(timeout_ms: *const u64) -> Duration {
    unsafe { timeout_ms.as_ref() }.map_or_else(
        || Duration::from_secs(3),
        |value| Duration::from_millis(*value),
    )
}

fn resolve_ipv4(address: &str) -> Result<Ipv4Addr, &'static str> {
    (address, 0)
        .to_socket_addrs()
        .map_err(|_| "ERR_NET_RESOLVE")?
        .find_map(|address| match address {
            SocketAddr::V4(address) => Some(*address.ip()),
            SocketAddr::V6(_) => None,
        })
        .ok_or("ERR_NET_UNSUPPORTED")
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
struct Socket(libc::c_int);

#[cfg(any(target_os = "linux", target_os = "macos"))]
impl Drop for Socket {
    fn drop(&mut self) {
        unsafe { libc::close(self.0) };
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn errno_code() -> &'static str {
    match std::io::Error::last_os_error().raw_os_error() {
        Some(libc::EPERM | libc::EACCES) => "ERR_NET_PERMISSION",
        Some(libc::ETIMEDOUT | libc::EAGAIN) => "ERR_NET_TIMEOUT",
        _ => "ERR_NET_PROBE",
    }
}

#[cfg(target_os = "linux")]
fn sockaddr(address: Ipv4Addr, port: u16) -> libc::sockaddr_in {
    libc::sockaddr_in {
        sin_family: libc::AF_INET as _,
        sin_port: port.to_be(),
        sin_addr: libc::in_addr {
            s_addr: u32::from_ne_bytes(address.octets()),
        },
        sin_zero: [0; 8],
    }
}

#[cfg(target_os = "macos")]
fn sockaddr(address: Ipv4Addr, port: u16) -> libc::sockaddr_in {
    libc::sockaddr_in {
        sin_len: std::mem::size_of::<libc::sockaddr_in>() as u8,
        sin_family: libc::AF_INET as _,
        sin_port: port.to_be(),
        sin_addr: libc::in_addr {
            s_addr: u32::from_ne_bytes(address.octets()),
        },
        sin_zero: [0; 8],
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn set_timeout(fd: libc::c_int, timeout: Duration) -> Result<(), &'static str> {
    let timeout = libc::timeval {
        tv_sec: timeout.as_secs() as _,
        tv_usec: timeout.subsec_micros() as _,
    };
    let result = unsafe {
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_RCVTIMEO,
            (&timeout as *const libc::timeval).cast(),
            std::mem::size_of::<libc::timeval>() as _,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(errno_code())
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn checksum(bytes: &[u8]) -> u16 {
    let mut sum = 0_u32;
    for chunk in bytes.chunks(2) {
        sum += u16::from_be_bytes([chunk[0], *chunk.get(1).unwrap_or(&0)]) as u32;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

#[cfg(target_os = "linux")]
fn ping_ipv4(address: Ipv4Addr, timeout: Duration) -> Result<u64, &'static str> {
    let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, libc::IPPROTO_ICMP) };
    if fd < 0 {
        return Err(errno_code());
    }
    let socket = Socket(fd);
    set_timeout(socket.0, timeout)?;
    let target = sockaddr(address, 0);
    if unsafe {
        libc::connect(
            socket.0,
            (&target as *const libc::sockaddr_in).cast(),
            std::mem::size_of::<libc::sockaddr_in>() as _,
        )
    } != 0
    {
        return Err(errno_code());
    }
    let mut packet = [0_u8; 8];
    packet[0] = 8;
    packet[5] = 1;
    let checksum = checksum(&packet).to_be_bytes();
    packet[2..4].copy_from_slice(&checksum);
    let started = Instant::now();
    if unsafe { libc::send(socket.0, packet.as_ptr().cast(), packet.len(), 0) }
        != packet.len() as isize
    {
        return Err(errno_code());
    }
    let mut response = [0_u8; 1024];
    if unsafe { libc::recv(socket.0, response.as_mut_ptr().cast(), response.len(), 0) } < 0 {
        return Err(errno_code());
    }
    Ok(started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64)
}

#[cfg(target_os = "macos")]
fn ping_ipv4(address: Ipv4Addr, timeout: Duration) -> Result<u64, &'static str> {
    let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_RAW, libc::IPPROTO_ICMP) };
    if fd < 0 {
        return Err(errno_code());
    }
    let socket = Socket(fd);
    set_timeout(socket.0, timeout)?;
    let target = sockaddr(address, 0);
    if unsafe {
        libc::connect(
            socket.0,
            (&target as *const libc::sockaddr_in).cast(),
            std::mem::size_of::<libc::sockaddr_in>() as _,
        )
    } != 0
    {
        return Err(errno_code());
    }

    let mut packet = [0_u8; 16];
    packet[0] = 8;
    packet[4..6].copy_from_slice(&(unsafe { libc::getpid() } as u16).to_be_bytes());
    packet[6..8].copy_from_slice(&1_u16.to_be_bytes());
    let checksum = checksum(&packet).to_be_bytes();
    packet[2..4].copy_from_slice(&checksum);
    let started = Instant::now();
    if unsafe { libc::send(socket.0, packet.as_ptr().cast(), packet.len(), 0) }
        != packet.len() as isize
    {
        return Err(errno_code());
    }

    let mut response = [0_u8; 1500];
    loop {
        let received =
            unsafe { libc::recv(socket.0, response.as_mut_ptr().cast(), response.len(), 0) };
        if received < 0 {
            return Err(errno_code());
        }
        let received = received as usize;
        if received < 28 || (response[0] >> 4) != 4 {
            continue;
        }
        let icmp_offset = usize::from(response[0] & 0x0f) * 4;
        if received < icmp_offset.saturating_add(8) {
            continue;
        }
        let icmp = &response[icmp_offset..];
        let identifier = (unsafe { libc::getpid() } as u16).to_be_bytes();
        if icmp[0] == 0
            && icmp[1] == 0
            && icmp[4..6] == identifier
            && icmp[6..8] == 1_u16.to_be_bytes()
        {
            return Ok(started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64);
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn traceroute_ipv4(address: Ipv4Addr, timeout: Duration) -> Result<Vec<String>, &'static str> {
    let icmp_fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_RAW, libc::IPPROTO_ICMP) };
    if icmp_fd < 0 {
        return Err(errno_code());
    }
    let icmp = Socket(icmp_fd);
    set_timeout(icmp.0, timeout)?;
    let udp_fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, libc::IPPROTO_UDP) };
    if udp_fd < 0 {
        return Err(errno_code());
    }
    let udp = Socket(udp_fd);
    let mut hops = Vec::new();
    for ttl in 1_i32..=30 {
        if unsafe {
            libc::setsockopt(
                udp.0,
                libc::IPPROTO_IP,
                libc::IP_TTL,
                (&ttl as *const i32).cast(),
                std::mem::size_of::<i32>() as _,
            )
        } != 0
        {
            return Err(errno_code());
        }
        let target = sockaddr(address, 33_434_u16.saturating_add(ttl as u16));
        let payload = [0_u8; 1];
        if unsafe {
            libc::sendto(
                udp.0,
                payload.as_ptr().cast(),
                payload.len(),
                0,
                (&target as *const libc::sockaddr_in).cast(),
                std::mem::size_of::<libc::sockaddr_in>() as _,
            )
        } < 0
        {
            return Err(errno_code());
        }
        let mut source: libc::sockaddr_in = unsafe { std::mem::zeroed() };
        let mut source_len = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
        let mut packet = [0_u8; 1500];
        let received = unsafe {
            libc::recvfrom(
                icmp.0,
                packet.as_mut_ptr().cast(),
                packet.len(),
                0,
                (&mut source as *mut libc::sockaddr_in).cast(),
                &mut source_len,
            )
        };
        if received < 0 {
            if errno_code() == "ERR_NET_TIMEOUT" {
                hops.push("*".to_string());
                continue;
            }
            return Err(errno_code());
        }
        let hop = Ipv4Addr::from(source.sin_addr.s_addr.to_ne_bytes()).to_string();
        hops.push(hop);
        let header_len = ((packet[0] & 0x0f) as usize).saturating_mul(4);
        if received as usize > header_len && packet[header_len] == 3 {
            break;
        }
    }
    Ok(hops)
}

#[cfg(windows)]
struct IcmpHandle(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl IcmpHandle {
    fn open() -> Result<Self, &'static str> {
        let handle = unsafe { windows_sys::Win32::NetworkManagement::IpHelper::IcmpCreateFile() };
        if handle.is_null() || handle as isize == -1 {
            Err(windows_error_code(unsafe {
                windows_sys::Win32::Foundation::GetLastError()
            }))
        } else {
            Ok(Self(handle))
        }
    }
}

#[cfg(windows)]
impl Drop for IcmpHandle {
    fn drop(&mut self) {
        unsafe { windows_sys::Win32::NetworkManagement::IpHelper::IcmpCloseHandle(self.0) };
    }
}

#[cfg(windows)]
fn windows_error_code(error: u32) -> &'static str {
    match error {
        5 => "ERR_NET_PERMISSION",
        1460 | 11010 => "ERR_NET_TIMEOUT",
        50 => "ERR_NET_UNSUPPORTED",
        _ => "ERR_NET_PROBE",
    }
}

#[cfg(windows)]
fn send_icmp_echo(
    handle: &IcmpHandle,
    address: Ipv4Addr,
    timeout: Duration,
    ttl: u8,
) -> Result<windows_sys::Win32::NetworkManagement::IpHelper::ICMP_ECHO_REPLY, &'static str> {
    use windows_sys::Win32::NetworkManagement::IpHelper::{IP_OPTION_INFORMATION, IcmpSendEcho2};

    let payload = [0_u8; 8];
    let reply_bytes =
        std::mem::size_of::<windows_sys::Win32::NetworkManagement::IpHelper::ICMP_ECHO_REPLY>()
            .saturating_add(payload.len())
            .saturating_add(8);
    let words = reply_bytes.div_ceil(std::mem::size_of::<u64>());
    let mut reply_buffer = vec![0_u64; words];
    let mut options = IP_OPTION_INFORMATION::default();
    options.Ttl = ttl;
    let timeout_ms = timeout.as_millis().min(u128::from(u32::MAX)) as u32;
    let count = unsafe {
        IcmpSendEcho2(
            handle.0,
            std::ptr::null_mut(),
            None,
            std::ptr::null(),
            u32::from_ne_bytes(address.octets()),
            payload.as_ptr().cast(),
            payload.len() as u16,
            &options,
            reply_buffer.as_mut_ptr().cast(),
            (reply_buffer.len() * std::mem::size_of::<u64>()) as u32,
            timeout_ms,
        )
    };
    if count == 0 {
        return Err(windows_error_code(unsafe {
            windows_sys::Win32::Foundation::GetLastError()
        }));
    }
    let reply = unsafe {
        std::ptr::read(
            reply_buffer
                .as_ptr()
                .cast::<windows_sys::Win32::NetworkManagement::IpHelper::ICMP_ECHO_REPLY>(),
        )
    };
    Ok(reply)
}

#[cfg(windows)]
fn ping_ipv4(address: Ipv4Addr, timeout: Duration) -> Result<u64, &'static str> {
    use windows_sys::Win32::NetworkManagement::IpHelper::IP_SUCCESS;

    let handle = IcmpHandle::open()?;
    let reply = send_icmp_echo(&handle, address, timeout, 128)?;
    if reply.Status != IP_SUCCESS {
        return Err(windows_error_code(reply.Status));
    }
    Ok(u64::from(reply.RoundTripTime))
}

#[cfg(windows)]
fn traceroute_ipv4(address: Ipv4Addr, timeout: Duration) -> Result<Vec<String>, &'static str> {
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        IP_DEST_HOST_UNREACHABLE, IP_DEST_NET_UNREACHABLE, IP_DEST_PORT_UNREACHABLE,
        IP_DEST_PROT_UNREACHABLE, IP_REQ_TIMED_OUT, IP_SUCCESS, IP_TTL_EXPIRED_TRANSIT,
    };

    let handle = IcmpHandle::open()?;
    let mut hops = Vec::with_capacity(30);
    for ttl in 1_u8..=30 {
        let reply = match send_icmp_echo(&handle, address, timeout, ttl) {
            Ok(reply) => reply,
            Err("ERR_NET_TIMEOUT") => {
                hops.push("*".to_string());
                continue;
            }
            Err(error) => return Err(error),
        };
        match reply.Status {
            IP_REQ_TIMED_OUT => hops.push("*".to_string()),
            IP_TTL_EXPIRED_TRANSIT => {
                hops.push(Ipv4Addr::from(reply.Address.to_ne_bytes()).to_string());
            }
            IP_SUCCESS => {
                hops.push(Ipv4Addr::from(reply.Address.to_ne_bytes()).to_string());
                break;
            }
            IP_DEST_NET_UNREACHABLE
            | IP_DEST_HOST_UNREACHABLE
            | IP_DEST_PROT_UNREACHABLE
            | IP_DEST_PORT_UNREACHABLE => {
                if reply.Address != 0 {
                    hops.push(Ipv4Addr::from(reply.Address.to_ne_bytes()).to_string());
                }
                break;
            }
            status => return Err(windows_error_code(status)),
        }
    }
    Ok(hops)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn ping_ipv4(_address: Ipv4Addr, _timeout: Duration) -> Result<u64, &'static str> {
    Err("ERR_NET_UNSUPPORTED")
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn traceroute_ipv4(_address: Ipv4Addr, _timeout: Duration) -> Result<Vec<String>, &'static str> {
    Err("ERR_NET_UNSUPPORTED")
}

fn error_value(code: &'static str) -> ProbeErrorFfi {
    let (code, message) = rejection_text(code);
    ProbeErrorFfi {
        code: TyrusString::literal(code),
        message: TyrusString::literal(message),
    }
}

unsafe fn release_string_array(array: &mut TyrusArray) {
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

unsafe fn string_array(values: Vec<String>) -> Option<TyrusArray> {
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
                release_string_array(&mut array);
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

unsafe fn fulfill_u64_result(promise: *mut TyrusPromise, outcome: Result<u64, &'static str>) {
    unsafe {
        let value = match outcome {
            Ok(value) => ProbeResultFfi {
                tag: RESULT_TAG_OK,
                payload: ProbeResultPayload {
                    ok: ManuallyDrop::new(value),
                },
            },
            Err(code) => ProbeResultFfi {
                tag: RESULT_TAG_ERR,
                payload: ProbeResultPayload {
                    err: ManuallyDrop::new(error_value(code)),
                },
            },
        };
        let result =
            libc::malloc(std::mem::size_of::<ProbeResultFfi<u64>>()).cast::<ProbeResultFfi<u64>>();
        if result.is_null() {
            reject(promise, "ERR_NET_ALLOCATION_FAILED");
            return;
        }
        std::ptr::write(result, value);
        crate::tyrus_promise_fulfill_owned(promise, result.cast(), Some(drop_u64_result));
    }
}

unsafe fn fulfill_string_array_result(
    promise: *mut TyrusPromise,
    outcome: Result<Vec<String>, &'static str>,
) {
    unsafe {
        let mut value = match outcome
            .and_then(|values| string_array(values).ok_or("ERR_NET_ALLOCATION_FAILED"))
        {
            Ok(array) => ProbeResultFfi {
                tag: RESULT_TAG_OK,
                payload: ProbeResultPayload {
                    ok: ManuallyDrop::new(array),
                },
            },
            Err(code) => ProbeResultFfi {
                tag: RESULT_TAG_ERR,
                payload: ProbeResultPayload {
                    err: ManuallyDrop::new(error_value(code)),
                },
            },
        };
        let result = libc::malloc(std::mem::size_of::<ProbeResultFfi<TyrusArray>>())
            .cast::<ProbeResultFfi<TyrusArray>>();
        if result.is_null() {
            if value.tag == RESULT_TAG_OK {
                release_string_array(&mut value.payload.ok);
            }
            reject(promise, "ERR_NET_ALLOCATION_FAILED");
            return;
        }
        std::ptr::write(result, value);
        crate::tyrus_promise_fulfill_owned(promise, result.cast(), Some(drop_string_array_result));
    }
}

unsafe extern "C" fn drop_u64_result(value: *mut c_void) {
    unsafe {
        if !value.is_null() {
            libc::free(value);
        }
    }
}

unsafe extern "C" fn drop_string_array_result(value: *mut c_void) {
    unsafe {
        if value.is_null() {
            return;
        }
        let result = value.cast::<ProbeResultFfi<TyrusArray>>();
        if (*result).tag == RESULT_TAG_OK {
            release_string_array(&mut (*result).payload.ok);
        }
        libc::free(value);
    }
}

unsafe fn input(address: TyrusString) -> Result<String, &'static str> {
    unsafe {
        if address.is_null() {
            return Err("ERR_NET_INVALID_ADDRESS");
        }
        Ok(address.as_str().to_owned())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_ping(
    address: TyrusString,
    timeout_ms: *const u64,
) -> *mut TyrusPromise {
    unsafe {
        let promise = crate::tyrus_promise_pending();
        let address = match input(address) {
            Ok(address) => address,
            Err(code) => {
                fulfill_u64_result(promise, Err(code));
                return promise;
            }
        };
        let timeout = probe_timeout(timeout_ms);
        if timeout.is_zero() {
            fulfill_u64_result(promise, Err("ERR_NET_TIMEOUT"));
            return promise;
        }
        crate::tyrus_promise_retain(promise);
        let promise_address = promise as usize;
        executor().spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                resolve_ipv4(&address).and_then(|address| ping_ipv4(address, timeout))
            })
            .await;
            let promise = promise_address as *mut TyrusPromise;
            let outcome = match result {
                Ok(outcome) => outcome,
                Err(_) => Err("ERR_NET_PROBE"),
            };
            fulfill_u64_result(promise, outcome);
            crate::tyrus_promise_free(promise);
        });
        promise
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_traceroute(
    address: TyrusString,
    timeout_ms: *const u64,
) -> *mut TyrusPromise {
    unsafe {
        let promise = crate::tyrus_promise_pending();
        let address = match input(address) {
            Ok(address) => address,
            Err(code) => {
                fulfill_string_array_result(promise, Err(code));
                return promise;
            }
        };
        let timeout = probe_timeout(timeout_ms);
        if timeout.is_zero() {
            fulfill_string_array_result(promise, Err("ERR_NET_TIMEOUT"));
            return promise;
        }
        crate::tyrus_promise_retain(promise);
        let promise_address = promise as usize;
        executor().spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                resolve_ipv4(&address).and_then(|address| traceroute_ipv4(address, timeout))
            })
            .await;
            let promise = promise_address as *mut TyrusPromise;
            let outcome = match result {
                Ok(outcome) => outcome,
                Err(_) => Err("ERR_NET_PROBE"),
            };
            fulfill_string_array_result(promise, outcome);
            crate::tyrus_promise_free(promise);
        });
        promise
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_net_supports_icmp() -> bool {
    #[cfg(target_os = "linux")]
    {
        let ping = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, libc::IPPROTO_ICMP) };
        let trace = unsafe { libc::socket(libc::AF_INET, libc::SOCK_RAW, libc::IPPROTO_ICMP) };
        if ping >= 0 {
            unsafe { libc::close(ping) };
        }
        if trace >= 0 {
            unsafe { libc::close(trace) };
        }
        ping >= 0 && trace >= 0
    }
    #[cfg(target_os = "macos")]
    {
        let icmp = unsafe { libc::socket(libc::AF_INET, libc::SOCK_RAW, libc::IPPROTO_ICMP) };
        if icmp >= 0 {
            unsafe { libc::close(icmp) };
        }
        icmp >= 0
    }
    #[cfg(windows)]
    {
        IcmpHandle::open().is_ok()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe fn assert_error<T>(promise: *mut TyrusPromise, expected: &str) {
        unsafe {
            assert_eq!(
                crate::tyrus_promise_state(promise),
                crate::TYRUS_PROMISE_FULFILLED
            );
            let result = crate::tyrus_promise_await(promise).cast::<ProbeResultFfi<T>>();
            assert_eq!((*result).tag, RESULT_TAG_ERR);
            assert_eq!((*(*result).payload.err).code.as_str(), expected);
        }
    }

    #[test]
    fn every_probe_error_code_has_its_own_message() {
        let source = include_str!("net_probe.rs");
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
    fn probe_failures_are_fulfilled_as_net_error_results() {
        let address = TyrusString::literal("invalid host\n");
        let ping = unsafe { tyrus_net_ping(address.borrow(), std::ptr::null()) };
        let trace = unsafe { tyrus_net_traceroute(address.borrow(), std::ptr::null()) };
        unsafe { crate::wait_until_settled(ping) };
        unsafe { assert_error::<u64>(ping, "ERR_NET_RESOLVE") };
        unsafe { crate::tyrus_promise_free(ping) };
        unsafe { crate::wait_until_settled(trace) };
        unsafe { assert_error::<TyrusArray>(trace, "ERR_NET_RESOLVE") };
        unsafe { crate::tyrus_promise_free(trace) };
    }

    #[test]
    fn zero_timeout_returns_net_timeout_without_opening_sockets() {
        let address = TyrusString::literal("127.0.0.1");
        let timeout = 0_u64;
        let ping = unsafe { tyrus_net_ping(address.borrow(), &timeout) };
        let trace = unsafe { tyrus_net_traceroute(address.borrow(), &timeout) };
        unsafe { assert_error::<u64>(ping, "ERR_NET_TIMEOUT") };
        unsafe { crate::tyrus_promise_free(ping) };
        unsafe { assert_error::<TyrusArray>(trace, "ERR_NET_TIMEOUT") };
        unsafe { crate::tyrus_promise_free(trace) };
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_icmp_capability_matches_ping_socket_access() {
        let ping = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, libc::IPPROTO_ICMP) };
        let trace = unsafe { libc::socket(libc::AF_INET, libc::SOCK_RAW, libc::IPPROTO_ICMP) };
        let expected = ping >= 0 && trace >= 0;
        if ping >= 0 {
            unsafe { libc::close(ping) };
        }
        if trace >= 0 {
            unsafe { libc::close(trace) };
        }
        assert_eq!(tyrus_net_supports_icmp(), expected);
    }

    #[cfg(windows)]
    #[test]
    fn windows_icmp_capability_matches_ip_helper_handle_access() {
        assert_eq!(tyrus_net_supports_icmp(), IcmpHandle::open().is_ok());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_icmp_capability_matches_raw_socket_access() {
        let icmp = unsafe { libc::socket(libc::AF_INET, libc::SOCK_RAW, libc::IPPROTO_ICMP) };
        let expected = icmp >= 0;
        if expected {
            unsafe { libc::close(icmp) };
        }
        assert_eq!(tyrus_net_supports_icmp(), expected);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_ping_and_traceroute_loopback_when_raw_icmp_is_permitted() {
        if !tyrus_net_supports_icmp() {
            return;
        }
        let address = Ipv4Addr::LOCALHOST;
        assert!(ping_ipv4(address, Duration::from_secs(1)).is_ok());
        let hops = traceroute_ipv4(address, Duration::from_secs(1)).unwrap();
        assert_eq!(hops.last().map(String::as_str), Some("127.0.0.1"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_ping_and_traceroute_reach_loopback_when_ip_helper_is_available() {
        if !tyrus_net_supports_icmp() {
            return;
        }
        let address = Ipv4Addr::LOCALHOST;
        assert!(ping_ipv4(address, Duration::from_secs(1)).is_ok());
        let hops = traceroute_ipv4(address, Duration::from_secs(1)).unwrap();
        assert_eq!(hops.last().map(String::as_str), Some("127.0.0.1"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn ping_loopback_when_the_ping_socket_is_permitted() {
        let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, libc::IPPROTO_ICMP) };
        if fd < 0 {
            return;
        }
        unsafe { libc::close(fd) };
        let address = TyrusString::literal("127.0.0.1");
        let timeout = 1_000_u64;
        let promise = unsafe { tyrus_net_ping(address.borrow(), &timeout) };
        unsafe { crate::wait_until_settled(promise) };
        assert_eq!(
            unsafe { crate::tyrus_promise_state(promise) },
            crate::TYRUS_PROMISE_FULFILLED
        );
        let result = unsafe { crate::tyrus_promise_await(promise).cast::<ProbeResultFfi<u64>>() };
        assert_eq!(unsafe { (*result).tag }, RESULT_TAG_OK);
        let _rtt = unsafe { *(*result).payload.ok };
        unsafe { crate::tyrus_promise_free(promise) };
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn traceroute_loopback_when_full_probe_capability_is_permitted() {
        if !tyrus_net_supports_icmp() {
            return;
        }
        let address = TyrusString::literal("127.0.0.1");
        let timeout = 1_000_u64;
        let promise = unsafe { tyrus_net_traceroute(address.borrow(), &timeout) };
        unsafe { crate::wait_until_settled(promise) };
        assert_eq!(
            unsafe { crate::tyrus_promise_state(promise) },
            crate::TYRUS_PROMISE_FULFILLED
        );
        let result =
            unsafe { crate::tyrus_promise_await(promise).cast::<ProbeResultFfi<TyrusArray>>() };
        assert_eq!(unsafe { (*result).tag }, RESULT_TAG_OK);
        let array = unsafe { &*(*result).payload.ok };
        assert!(!array.data.is_null());
        assert_ne!(array.length, 0);
        unsafe { crate::tyrus_promise_free(promise) };
    }
}
