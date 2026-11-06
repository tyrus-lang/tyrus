use super::*;
use crate::abi::TyrusArray;
use crate::temporal::Instant;
use std::ffi::c_void;
use tyrus_string::TyrusString;

#[repr(C)]
pub struct HttpRequest {
    pub id: TyrusString,
    pub method: TyrusString,
    pub path: TyrusString,
    pub body: TyrusString,
    pub params: TyrusString,
    pub query: TyrusString,
    pub headers: TyrusString,
    pub cookies: TyrusString,
    pub raw_body: *const u8,
    pub raw_body_length: usize,
    pub parts: *const MultipartPart,
    pub parts_length: usize,
    pub body_stream: *const crate::stream::TyrusReadableStream,
}

/// One part of a `multipart/form-data` body, as generated code reads it.
///
/// The optional fields are nullable pointers because that is what a `string?`
/// is in the language ABI, and absence here is a real state rather than an
/// empty string: a text field genuinely has no file name, and a sender is
/// allowed not to say what a part contains.
#[repr(C)]
pub struct MultipartPart {
    pub name: TyrusString,
    pub file_name: *const TyrusString,
    pub content_type: *const TyrusString,
    pub bytes: TyrusArray,
}

unsafe impl Send for MultipartPart {}

#[repr(C)]
pub struct HttpBody {
    pub tag: u8,
    pub padding: [u8; 7],
    pub payload: [u64; 3],
}

impl HttpBody {
    pub const fn empty_text() -> Self {
        Self {
            tag: 0,
            padding: [0; 7],
            payload: [0; 3],
        }
    }

    pub fn from_text(text: TyrusString) -> Self {
        let mut body = Self::empty_text();
        unsafe {
            body.payload.as_mut_ptr().cast::<TyrusString>().write(text);
        }
        body
    }

    pub unsafe fn text(&self) -> Option<&TyrusString> {
        unsafe { (self.tag == 0).then(|| &*self.payload.as_ptr().cast::<TyrusString>()) }
    }

    pub fn from_owned_bytes(data: *mut u8, length: usize) -> Self {
        let mut body = Self::empty_text();
        body.tag = 1;
        body.payload = [data as u64, length as u64, length as u64];
        body
    }

    pub unsafe fn bytes(&self) -> Option<(*mut u8, usize)> {
        if self.tag != 1 {
            return None;
        }
        let data = self.payload[0] as *mut u8;
        let length = self.payload[1] as usize;
        Some((data, length))
    }

    pub fn from_stream(stream: *const crate::stream::TyrusReadableStream) -> Self {
        let mut body = Self::empty_text();
        body.tag = 2;
        body.payload[0] = stream as u64;
        body
    }

    /// Transfers the stream handle out of the response without releasing it.
    /// The HTTP body adapter becomes responsible for releasing that handle
    /// once Axum has finished polling it.
    pub unsafe fn take_stream(&mut self) -> Option<*const crate::stream::TyrusReadableStream> {
        if self.tag != 2 {
            return None;
        }
        let stream = self.payload[0] as *const crate::stream::TyrusReadableStream;
        *self = Self::empty_text();
        Some(stream)
    }

    pub(crate) fn from_writable_stream(consumer: crate::stream::ByteStreamConsumer) -> Self {
        let mut body = Self::empty_text();
        body.tag = 3;
        body.payload[0] = Box::into_raw(Box::new(consumer)) as u64;
        body
    }

    pub(crate) unsafe fn take_writable_stream(
        &mut self,
    ) -> Option<Box<crate::stream::ByteStreamConsumer>> {
        unsafe {
            if self.tag != 3 {
                return None;
            }
            let consumer = self.payload[0] as *mut crate::stream::ByteStreamConsumer;
            *self = Self::empty_text();
            (!consumer.is_null()).then(|| Box::from_raw(consumer))
        }
    }

    pub unsafe fn release(&mut self) {
        unsafe {
            match self.tag {
                0 => {
                    let text = &mut *self.payload.as_mut_ptr().cast::<TyrusString>();
                    std::mem::replace(text, TyrusString::null()).release();
                }
                1 => {
                    let data = self.payload[0] as *mut c_void;
                    if !data.is_null() {
                        libc::free(data);
                    }
                }
                2 => crate::stream::tyrus_readable_stream_free(
                    self.payload[0] as *const crate::stream::TyrusReadableStream,
                ),
                3 => {
                    let consumer = self.payload[0] as *mut crate::stream::ByteStreamConsumer;
                    if !consumer.is_null() {
                        drop(Box::from_raw(consumer));
                    }
                }
                _ => {}
            }
            *self = Self::empty_text();
        }
    }
}

#[repr(C)]
pub struct HttpResponse {
    pub status: i32,
    pub body: HttpBody,
    pub content_type: *const TyrusString,
    pub cookies: TyrusString,
}

impl HttpResponse {
    pub const fn empty() -> Self {
        Self {
            status: 500,
            body: HttpBody::empty_text(),
            content_type: std::ptr::null(),
            cookies: TyrusString::null(),
        }
    }

    /// Releases everything this response owns, leaving it empty.
    ///
    /// # Safety
    ///
    /// No other writer may hold a pointer into this response.
    pub unsafe fn release(&mut self) {
        unsafe {
            self.body.release();
            if !self.content_type.is_null() {
                let content_type = &mut *(self.content_type as *mut TyrusString);
                std::mem::replace(content_type, TyrusString::null()).release();
                libc::free(self.content_type.cast_mut().cast());
                self.content_type = std::ptr::null();
            }
            std::mem::replace(&mut self.cookies, TyrusString::null()).release();
        }
    }
}

unsafe impl Send for HttpResponse {}

#[repr(C)]
pub struct CorsOptions {
    pub origin: TyrusStringArray,
    pub methods: TyrusStringArray,
    pub allowed_headers: TyrusStringArray,
    pub exposed_headers: TyrusStringArray,
    pub credentials: bool,
    pub max_age: i32,
}

#[repr(C)]
pub struct CsrfOptions {
    pub cookie: TyrusString,
    pub header: TyrusString,
}

/// Optional cookie attributes. Each nullable pointer is the language ABI for
/// an omitted option field, and points to a live value for the duration of the
/// native reply call.
#[repr(C)]
pub struct CookieOptions {
    pub domain: *const TyrusString,
    pub path: *const TyrusString,
    pub max_age: *const i64,
    pub secure: *const bool,
    pub http_only: *const bool,
    pub same_site: *const TyrusString,
}

#[repr(C)]
pub struct HttpCookie {
    pub name: TyrusString,
    pub value: TyrusString,
    pub options: *const CookieOptions,
}

#[repr(C)]
pub struct HttpCacheOptions {
    pub etag: TyrusString,
    pub weak: *const bool,
    pub last_modified: *const Instant,
    pub control: *const TyrusString,
}

#[repr(C)]
pub struct HttpReplyOptions {
    pub cache: *const HttpCacheOptions,
}

pub type HttpHandler =
    unsafe extern "C" fn(*const HttpRequest, *mut HttpResponse) -> *mut TyrusPromise;

pub type HttpPreHandler = unsafe extern "C" fn(*const HttpRequest, *mut HttpResponse) -> i32;

pub type HttpResponseHook = unsafe extern "C" fn(*const HttpAccessLog);

#[repr(C)]
pub struct HttpAccessLog {
    pub request_id: TyrusString,
    pub method: TyrusString,
    pub path: TyrusString,
    pub status: i32,
    pub duration: crate::temporal::Duration,
}

pub struct HttpRouter {
    pub(super) router: Router,
    pub(super) cors: Option<CorsLayer>,
    pub(super) response_hooks: std::sync::Arc<std::sync::Mutex<Vec<HttpResponseHook>>>,
    pub(super) config: std::sync::Arc<std::sync::RwLock<HttpServerConfig>>,
    pub(super) openapi_fragments: crate::http_docs::RouteFragments,
}

#[repr(C)]
pub struct HttpServerOptions {
    pub host: TyrusString,
    pub request_timeout_millis: i64,
    pub max_concurrent_requests: i64,
    pub shutdown_timeout_millis: i64,
    pub header_timeout_millis: i64,
    pub max_header_bytes: i64,
    pub max_connections: i64,
    pub keep_alive: i64,
    pub logger: i64,
    pub tls_cert: TyrusString,
    pub tls_key: TyrusString,
}

pub struct HttpServerConfig {
    pub(super) trusted_proxies: Vec<TrustedProxy>,
    pub(super) trusted_proxy_identity_header: Option<String>,
    pub(super) csrf: Option<CsrfConfig>,
    pub(super) request_timeout: Option<Duration>,
    pub(super) concurrency: Option<std::sync::Arc<tokio::sync::Semaphore>>,
    pub(super) in_flight: std::sync::Arc<InFlight>,
    pub(super) websocket_going_away: std::sync::Arc<tokio::sync::watch::Sender<bool>>,
    pub(super) rate_limit: Option<std::sync::Arc<crate::http::RateLimiter>>,
    pub(super) header_read_timeout: Duration,
    pub(super) max_header_bytes: usize,
    pub(super) max_connections: Option<std::sync::Arc<tokio::sync::Semaphore>>,
    pub(super) keep_alive: bool,
    pub(super) logger: bool,
    pub(super) tls: Option<std::sync::Arc<tokio_rustls::TlsAcceptor>>,
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            trusted_proxies: Vec::new(),
            trusted_proxy_identity_header: None,
            csrf: None,
            request_timeout: None,
            concurrency: None,
            in_flight: std::sync::Arc::new(InFlight::default()),
            websocket_going_away: std::sync::Arc::new(tokio::sync::watch::channel(false).0),
            rate_limit: None,
            header_read_timeout: crate::http::DEFAULT_HEADER_READ_TIMEOUT,
            max_header_bytes: crate::http::DEFAULT_MAX_HEADER_BYTES,
            max_connections: None,
            keep_alive: true,
            logger: false,
            tls: None,
        }
    }
}

#[derive(Clone)]
pub struct CsrfConfig {
    pub(super) cookie: String,
    pub(super) header: axum::http::HeaderName,
}

#[derive(Default)]
pub struct InFlight {
    count: std::sync::atomic::AtomicUsize,
    idle: tokio::sync::Notify,
}

pub struct InFlightGuard(std::sync::Arc<InFlight>);

impl InFlight {
    pub(super) fn enter(self: &std::sync::Arc<Self>) -> InFlightGuard {
        self.count.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        InFlightGuard(std::sync::Arc::clone(self))
    }

    pub(super) async fn wait_idle(&self) {
        loop {
            let idle = self.idle.notified();
            if self.count.load(std::sync::atomic::Ordering::Acquire) == 0 {
                return;
            }
            idle.await;
        }
    }
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        if self
            .0
            .count
            .fetch_sub(1, std::sync::atomic::Ordering::AcqRel)
            == 1
        {
            self.0.idle.notify_waiters();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrustedProxy {
    network: std::net::IpAddr,
    prefix: u8,
}

impl TrustedProxy {
    pub fn parse(value: &str) -> Option<Self> {
        let (address, prefix) = match value.split_once('/') {
            Some((address, prefix)) => (address, Some(prefix)),
            None => (value, None),
        };
        let network: std::net::IpAddr = address.trim().parse().ok()?;
        let full = if network.is_ipv4() { 32 } else { 128 };
        let prefix = match prefix {
            None => full,
            Some(prefix) => {
                let prefix: u8 = prefix.trim().parse().ok()?;
                if prefix > full {
                    return None;
                }
                prefix
            }
        };
        Some(Self { network, prefix })
    }

    pub fn contains(&self, address: std::net::IpAddr) -> bool {
        match (self.network, address) {
            (std::net::IpAddr::V4(network), std::net::IpAddr::V4(address)) => {
                Self::same_prefix(&network.octets(), &address.octets(), self.prefix)
            }
            (std::net::IpAddr::V6(network), std::net::IpAddr::V6(address)) => {
                Self::same_prefix(&network.octets(), &address.octets(), self.prefix)
            }
            _ => false,
        }
    }

    fn same_prefix(network: &[u8], address: &[u8], prefix: u8) -> bool {
        let whole = usize::from(prefix / 8);
        if network[..whole] != address[..whole] {
            return false;
        }
        let bits = prefix % 8;
        if bits == 0 {
            return true;
        }
        let mask = 0xffu8 << (8 - bits);
        network[whole] & mask == address[whole] & mask
    }
}

pub(super) fn text(value: &TyrusString) -> Result<String, ()> {
    if value.is_null() {
        return Err(());
    }
    Ok(unsafe { value.as_str() }.to_owned())
}
