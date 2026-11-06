use crate::strings::alloc_string;
use std::ffi::c_char;
use std::net::{IpAddr, SocketAddr};
use tyrus_string::TyrusString;

#[repr(C)]
pub struct IpAddress {
    pub address: TyrusString,
    pub family: TyrusString,
}

#[repr(C)]
pub struct SocketAddress {
    pub ip: IpAddress,
    pub port: u16,
}

fn box_bytes(bytes: &[u8]) -> *mut c_char {
    let allocation = unsafe { libc::malloc(bytes.len()) };
    if allocation.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), allocation.cast::<u8>(), bytes.len());
    }
    allocation.cast()
}

fn ip_address_value(address: IpAddr) -> Option<IpAddress> {
    let family = if address.is_ipv4() { "IPv4" } else { "IPv6" };
    let value = IpAddress {
        address: alloc_string(&address.to_string()),
        family: alloc_string(family),
    };
    if value.address.is_null() || value.family.is_null() {
        unsafe {
            value.address.release();
            value.family.release();
        }
        None
    } else {
        Some(value)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_parse_ip_address(input: TyrusString) -> *mut c_char {
    unsafe {
        if input.is_null() {
            return std::ptr::null_mut();
        }
        let input = input.as_str();
        let Ok(address) = input.parse::<IpAddr>() else {
            return std::ptr::null_mut();
        };
        let Some(value) = ip_address_value(address) else {
            return std::ptr::null_mut();
        };
        let bytes = std::slice::from_raw_parts(
            (&value as *const IpAddress).cast::<u8>(),
            std::mem::size_of::<IpAddress>(),
        );
        let result = box_bytes(bytes);
        if result.is_null() {
            value.address.release();
            value.family.release();
        }
        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_parse_socket_address(input: TyrusString) -> *mut c_char {
    unsafe {
        if input.is_null() {
            return std::ptr::null_mut();
        }
        let input = input.as_str();
        let Ok(address) = input.parse::<SocketAddr>() else {
            return std::ptr::null_mut();
        };
        let Some(ip) = ip_address_value(address.ip()) else {
            return std::ptr::null_mut();
        };
        let value = SocketAddress {
            ip,
            port: address.port(),
        };
        let bytes = std::slice::from_raw_parts(
            (&value as *const SocketAddress).cast::<u8>(),
            std::mem::size_of::<SocketAddress>(),
        );
        let result = box_bytes(bytes);
        if result.is_null() {
            let ip = value.ip;
            ip.address.release();
            ip.family.release();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::tyrus_net_parse_ip_address;
    use tyrus_string::TyrusString;

    #[test]
    fn parses_ipv4_and_ipv6_and_rejects_invalid_text() {
        for (input, expected_family, expected_address) in [
            ("127.0.0.1", "IPv4", "127.0.0.1"),
            ("2001:db8::1", "IPv6", "2001:db8::1"),
        ] {
            let result = unsafe { tyrus_net_parse_ip_address(TyrusString::literal(input)) };
            assert!(!result.is_null());
            let value = unsafe { &*result.cast::<super::IpAddress>() };
            let family = unsafe { value.family.as_str() };
            let address = unsafe { value.address.as_str() };
            assert_eq!(family, expected_family);
            assert_eq!(address, expected_address);
            unsafe {
                let value = std::ptr::read(result.cast::<super::IpAddress>());
                value.address.release();
                value.family.release();
                libc::free(result.cast());
            }
        }
        assert!(unsafe { tyrus_net_parse_ip_address(TyrusString::literal("not-an-ip")) }.is_null());
    }

    #[test]
    fn parses_socket_addresses_and_rejects_unbracketed_ipv6_ports() {
        for (input, expected_family, expected_port) in [
            ("127.0.0.1:8080", "IPv4", 8080),
            ("[2001:db8::1]:443", "IPv6", 443),
        ] {
            let result =
                unsafe { super::tyrus_net_parse_socket_address(TyrusString::literal(input)) };
            assert!(!result.is_null());
            let value = unsafe { &*result.cast::<super::SocketAddress>() };
            let family = unsafe { value.ip.family.as_str() };
            assert_eq!(family, expected_family);
            assert_eq!(value.port, expected_port);
            unsafe {
                let ip = std::ptr::read(std::ptr::addr_of!(value.ip));
                ip.address.release();
                ip.family.release();
                libc::free(result.cast());
            }
        }
        assert!(
            unsafe {
                super::tyrus_net_parse_socket_address(TyrusString::literal("2001:db8::1:443"))
            }
            .is_null()
        );
    }
}
