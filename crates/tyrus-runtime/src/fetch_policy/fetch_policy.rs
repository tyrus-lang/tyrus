use std::collections::HashSet;
use std::ffi::c_void;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::{Arc, RwLock};

pub(crate) const BLOCKED: (&str, &str) = (
    "ERR_HTTP_FETCH_BLOCKED",
    "the destination is refused by the fetch policy",
);

#[derive(Default)]
pub(crate) struct FetchPolicy {
    pub(crate) deny_private: bool,
    pub(crate) allow: HashSet<String>,
}

impl FetchPolicy {
    fn exempts(&self, host: &str) -> bool {
        !self.allow.is_empty() && self.allow.contains(&host.to_ascii_lowercase())
    }

    pub(crate) fn permits(&self, host: &str, address: IpAddr) -> bool {
        if !self.deny_private || self.exempts(host) {
            return true;
        }
        !address_is_internal(address)
    }
}

fn policy_slot() -> &'static RwLock<Option<Arc<FetchPolicy>>> {
    static POLICY: std::sync::OnceLock<RwLock<Option<Arc<FetchPolicy>>>> =
        std::sync::OnceLock::new();
    POLICY.get_or_init(|| RwLock::new(None))
}

pub(crate) fn set_policy(policy: FetchPolicy) {
    let mut slot = match policy_slot().write() {
        Ok(slot) => slot,
        Err(poisoned) => poisoned.into_inner(),
    };
    *slot = Some(Arc::new(policy));
}

pub(crate) fn current_policy() -> Option<Arc<FetchPolicy>> {
    match policy_slot().read() {
        Ok(slot) => slot.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

pub(crate) fn address_is_internal(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => ipv4_is_internal(address),
        IpAddr::V6(address) => ipv6_is_internal(address),
    }
}

fn ipv4_is_internal(address: Ipv4Addr) -> bool {
    let [a, b, ..] = address.octets();
    address.is_loopback()
        || address.is_private()
        || address.is_link_local()
        || address.is_broadcast()
        || address.is_multicast()
        || a == 0
        || (a == 100 && (64..128).contains(&b))
        || (a == 198 && (b == 18 || b == 19))
        || a >= 240
}

fn ipv6_is_internal(address: Ipv6Addr) -> bool {
    if let Some(mapped) = address.to_ipv4_mapped() {
        return ipv4_is_internal(mapped);
    }
    let segments = address.segments();
    address.is_loopback()
        || address.is_unspecified()
        || address.is_multicast()
        || (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
        || segments[..6].iter().all(|part| *part == 0)
}

pub(crate) fn url_is_permitted(url: &reqwest::Url) -> bool {
    let Some(policy) = current_policy() else {
        return true;
    };
    if !policy.deny_private {
        return true;
    }
    let Some(host) = url.host_str() else {
        return true;
    };
    let literal = host.trim_start_matches('[').trim_end_matches(']');
    match literal.parse::<IpAddr>() {
        Ok(address) => policy.permits(host, address),
        Err(_) => true,
    }
}

pub(crate) struct PolicyResolver;

impl reqwest::dns::Resolve for PolicyResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        Box::pin(async move {
            let host = name.as_str().to_owned();
            let resolved = tokio::net::lookup_host((host.as_str(), 0))
                .await?
                .collect::<Vec<_>>();
            let Some(policy) = current_policy() else {
                return Ok(Box::new(resolved.into_iter()) as reqwest::dns::Addrs);
            };
            let permitted = resolved
                .into_iter()
                .filter(|address| policy.permits(&host, address.ip()))
                .collect::<Vec<_>>();
            if permitted.is_empty() {
                return Err(
                    Box::new(BlockedDestination) as Box<dyn std::error::Error + Send + Sync>
                );
            }
            Ok(Box::new(permitted.into_iter()) as reqwest::dns::Addrs)
        })
    }
}

#[derive(Debug)]
pub(crate) struct BlockedDestination;

pub(crate) const BLOCKED_MARKER: &str = "tyrus fetch policy refused this destination";

impl std::fmt::Display for BlockedDestination {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(BLOCKED_MARKER)
    }
}

impl std::error::Error for BlockedDestination {}

#[repr(C)]
pub struct FetchPolicyOptions {
    pub allow: crate::abi::TyrusStringArray,
    pub deny_private: bool,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_router_fetch_policy(
    router: *mut c_void,
    options: *const FetchPolicyOptions,
) -> i32 {
    unsafe {
        if router.is_null() {
            crate::http::set_http_error("ERR_HTTP_FETCH_POLICY_CONFIG", "router is unavailable");
            return -1;
        }
        let Some(options) = options.as_ref() else {
            crate::http::set_http_error(
                "ERR_HTTP_FETCH_POLICY_CONFIG",
                "fetch policy options are invalid",
            );
            return -1;
        };
        let allow = &options.allow;
        if allow.length > allow.capacity || (allow.length > 0 && allow.data.is_null()) {
            crate::http::set_http_error(
                "ERR_HTTP_FETCH_POLICY_CONFIG",
                "allow list has an invalid layout",
            );
            return -1;
        }
        let entries = if allow.length == 0 {
            &[][..]
        } else {
            std::slice::from_raw_parts(allow.data, allow.length)
        };
        let mut hosts = HashSet::with_capacity(entries.len());
        for entry in entries {
            let Ok(host) = std::str::from_utf8(entry.as_bytes()) else {
                crate::http::set_http_error(
                    "ERR_HTTP_FETCH_POLICY_CONFIG",
                    "allow list contains invalid text",
                );
                return -1;
            };
            hosts.insert(host.to_ascii_lowercase());
        }
        set_policy(FetchPolicy {
            deny_private: options.deny_private,
            allow: hosts,
        });
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn internal(text: &str) -> bool {
        address_is_internal(text.parse().expect("a test address parses"))
    }

    #[test]
    fn the_metadata_service_is_internal_in_every_notation() {
        assert!(internal("169.254.169.254"));
        assert!(internal("::ffff:169.254.169.254"));
        assert!(internal("::ffff:a9fe:a9fe"));
    }

    #[test]
    fn the_private_and_loopback_ranges_are_internal() {
        for address in [
            "127.0.0.1",
            "10.0.0.5",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.1.1",
            "0.0.0.0",
            "0.1.2.3",
            "0.255.255.255",
            "100.64.0.1",
            "198.18.0.1",
            "240.0.0.1",
            "255.255.255.255",
            "::1",
            "::",
            "fc00::1",
            "fd00::1",
            "fe80::1",
        ] {
            assert!(internal(address), "{address} should be internal");
        }
    }

    #[test]
    fn public_addresses_are_not_internal() {
        for address in [
            "1.1.1.1",
            "8.8.8.8",
            "172.15.255.255",
            "172.32.0.1",
            "100.128.0.1",
            "198.20.0.1",
            "2606:4700:4700::1111",
            "2001:db8::1",
        ] {
            assert!(!internal(address), "{address} should not be internal");
        }
    }

    #[test]
    fn an_exemption_is_matched_without_regard_to_case() {
        let policy = FetchPolicy {
            deny_private: true,
            allow: HashSet::from(["internal.example.com".to_string()]),
        };
        let address: IpAddr = "10.0.0.5".parse().unwrap();
        assert!(policy.permits("Internal.Example.COM", address));
        assert!(!policy.permits("other.example.com", address));
    }

    #[test]
    fn an_exemption_does_not_widen_the_policy_to_other_hosts() {
        let policy = FetchPolicy {
            deny_private: true,
            allow: HashSet::from(["10.0.0.5".to_string()]),
        };
        assert!(policy.permits("10.0.0.5", "10.0.0.5".parse().unwrap()));
        assert!(!policy.permits("10.0.0.6", "10.0.0.6".parse().unwrap()));
    }

    #[test]
    fn a_policy_that_denies_nothing_permits_everything() {
        let policy = FetchPolicy::default();
        assert!(policy.permits("localhost", "127.0.0.1".parse().unwrap()));
    }
}
