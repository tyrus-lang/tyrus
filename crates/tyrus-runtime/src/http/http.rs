use super::*;
use crate::abi::TyrusArray;
use crate::temporal::{Duration as TemporalDuration, Instant};
use axum::body::HttpBody as AxumHttpBody;
use futures_util::StreamExt;
use http_body::{Frame, SizeHint};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::task::{Context, Poll};
use std::time::{Instant as StdInstant, UNIX_EPOCH};
use tyrus_string::TyrusString;

const MAX_RATE_LIMIT_BUCKETS: usize = 10_000;

const RATE_LIMIT_EVICTION_DIVISOR: usize = 16;
const MIN_COMPRESSIBLE_RESPONSE_BYTES: usize = 1024;
const HTTP_STREAM_BUFFER_BYTES: usize = 64 * 1024;

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);
static SIGNAL_SHUTDOWN_STARTED: AtomicBool = AtomicBool::new(false);
#[derive(Clone)]
pub(crate) struct RequestId(pub(crate) String);

pub(crate) fn next_request_id() -> RequestId {
    const BASE64_URL: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    request_id_from_counter(NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed), BASE64_URL)
}

pub(crate) fn request_id_from_counter(value: u64, alphabet: &[u8; 64]) -> RequestId {
    let bytes = value.to_be_bytes();
    let first = bytes.iter().position(|byte| *byte != 0).unwrap_or(7);
    let bytes = &bytes[first..];
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = *chunk.get(1).unwrap_or(&0);
        let third = *chunk.get(2).unwrap_or(&0);
        encoded.push(alphabet[usize::from(first >> 2)] as char);
        encoded.push(alphabet[usize::from((first & 0b0000_0011) << 4 | second >> 4)] as char);
        if chunk.len() > 1 {
            encoded.push(alphabet[usize::from((second & 0b0000_1111) << 2 | third >> 6)] as char);
        }
        if chunk.len() > 2 {
            encoded.push(alphabet[usize::from(third & 0b0011_1111)] as char);
        }
    }
    RequestId(format!("req-{encoded}"))
}

pub(crate) fn should_observe_response(logger: bool, hooks: &[HttpResponseHook]) -> bool {
    logger || !hooks.is_empty()
}

fn temporal_duration(elapsed: Duration) -> TemporalDuration {
    let seconds = i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX);
    let nanoseconds = elapsed.subsec_nanos();
    TemporalDuration {
        years: 0,
        months: 0,
        weeks: 0,
        days: 0,
        hours: 0,
        minutes: 0,
        seconds,
        milliseconds: i64::from(nanoseconds / 1_000_000),
        microseconds: i64::from((nanoseconds / 1_000) % 1_000),
        nanoseconds: i64::from(nanoseconds % 1_000),
    }
}

struct ResponseObservation {
    request_id: RequestId,
    method: Method,
    path: String,
    status: StatusCode,
    started: StdInstant,
    logger: bool,
    hooks: Arc<[HttpResponseHook]>,
}

impl ResponseObservation {
    fn complete(self) {
        let elapsed = self.started.elapsed();
        let duration_millis = elapsed.as_millis();
        let status = self.status.as_u16();
        if self.logger {
            eprintln!(
                "{}",
                serde_json::json!({
                    "event": "http.access",
                    "requestId": self.request_id.0,
                    "method": self.method.as_str(),
                    "path": self.path,
                    "status": status,
                    "durationMs": duration_millis,
                })
            );
        }
        if self.hooks.is_empty() {
            return;
        }
        let duration = temporal_duration(elapsed);
        let lend =
            |text: &str| unsafe { TyrusString::lend(text) }.unwrap_or_else(TyrusString::null);
        let access = HttpAccessLog {
            request_id: lend(&self.request_id.0),
            method: lend(self.method.as_str()),
            path: lend(&self.path),
            status: i32::from(status),
            duration,
        };
        for hook in self.hooks.iter().copied() {
            unsafe { hook(&access) };
        }
    }
}

struct ObservedBody {
    body: Body,
    observation: Option<ResponseObservation>,
}

struct InFlightBody {
    body: Body,
    _guard: crate::types::InFlightGuard,
}

impl AxumHttpBody for InFlightBody {
    type Data = Bytes;
    type Error = axum::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Pin::new(&mut self.get_mut().body).poll_frame(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}

impl ObservedBody {
    fn complete(&mut self) {
        if let Some(observation) = self.observation.take() {
            observation.complete();
        }
    }
}

impl Drop for ObservedBody {
    fn drop(&mut self) {
        self.complete();
    }
}

impl AxumHttpBody for ObservedBody {
    type Data = Bytes;
    type Error = axum::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        let frame = Pin::new(&mut this.body).poll_frame(cx);
        if matches!(frame, Poll::Ready(None | Some(Err(_)))) {
            this.complete();
        }
        frame
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}

pub(crate) fn observe_response(
    mut response: Response<Body>,
    request_id: &RequestId,
    method: &Method,
    path: &str,
    started: StdInstant,
    logger: bool,
    hooks: Arc<[HttpResponseHook]>,
) -> Response<Body> {
    if let Ok(value) = HeaderValue::from_str(&request_id.0) {
        response.headers_mut().insert("x-request-id", value);
    }
    let status = response.status();
    let (parts, body) = response.into_parts();
    Response::from_parts(
        parts,
        Body::new(ObservedBody {
            body,
            observation: Some(ResponseObservation {
                request_id: request_id.clone(),
                method: method.clone(),
                path: path.to_string(),
                status,
                started,
                logger,
                hooks,
            }),
        }),
    )
}

fn attach_request_id(mut response: Response<Body>, request_id: &RequestId) -> Response<Body> {
    if let Ok(value) = HeaderValue::from_str(&request_id.0) {
        response.headers_mut().insert("x-request-id", value);
    }
    response
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResponseEncoding {
    Brotli,
    Gzip,
}

impl ResponseEncoding {
    fn header_value(self) -> &'static str {
        match self {
            Self::Brotli => "br",
            Self::Gzip => "gzip",
        }
    }
}

fn query_method() -> Method {
    Method::from_bytes(b"QUERY").expect("QUERY is a valid HTTP method token")
}

fn is_query_method(method: &Method) -> bool {
    *method == query_method()
}

struct ResponseSlot {
    response: std::cell::UnsafeCell<HttpResponse>,
    cookie_names: AtomicBool,
}

// SAFETY: the slot has one writer at a time. Generated code writes it until the
// handler's Promise settles, and the response is read only after settlement,
// which synchronizes those writes.
unsafe impl Send for ResponseSlot {}
unsafe impl Sync for ResponseSlot {}

impl ResponseSlot {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            response: std::cell::UnsafeCell::new(HttpResponse::empty()),
            cookie_names: AtomicBool::new(false),
        })
    }

    fn register_cookie_names(&self, names: Option<&Arc<[String]>>) {
        if let Some(names) = names {
            request_cookie_names_lock()
                .insert(reply_header_key(self.response.get()), Arc::clone(names));
            self.cookie_names.store(true, Ordering::Relaxed);
        }
    }

    fn as_ptr(&self) -> *mut HttpResponse {
        self.response.get()
    }

    unsafe fn take(&self) -> HttpResponse {
        unsafe { std::ptr::replace(self.response.get(), HttpResponse::empty()) }
    }
}

impl Drop for ResponseSlot {
    fn drop(&mut self) {
        unregister_reply_writable(self.response.get());
        clear_reply_headers(self.response.get());
        if self.cookie_names.load(Ordering::Relaxed) {
            let _ = request_cookie_names_lock().remove(&reply_header_key(self.response.get()));
        }
        // SAFETY: the slot is being destroyed, so no writer remains.
        unsafe { (*self.response.get()).release() };
    }
}

/// Response headers are metadata owned by the native reply API, rather than a
/// field in the public `HttpResponse` ABI. Keeping them out of the ABI avoids
/// exposing a newline-delimited wire representation to generated programs.
type ReplyHeaders = HashMap<usize, Vec<(HeaderName, HeaderValue)>>;

static REPLY_HEADERS: LazyLock<Mutex<ReplyHeaders>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// The names of the cookies a request carried, filed under each response a
/// handler or preHandler writes, so `reply.clearCookies()` can expire them. A
/// reply only knows the request through this map: the `HttpResponse` ABI has no
/// room for it, and a function that takes only the reply has no request.
/// Requests without a `Cookie` header file nothing.
static REQUEST_COOKIE_NAMES: LazyLock<Mutex<HashMap<usize, Arc<[String]>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn request_cookie_names_lock() -> std::sync::MutexGuard<'static, HashMap<usize, Arc<[String]>>> {
    match REQUEST_COOKIE_NAMES.lock() {
        Ok(names) => names,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn request_cookie_names(headers: &axum::http::HeaderMap) -> Option<Arc<[String]>> {
    let mut seen = std::collections::HashSet::new();
    let mut names = Vec::new();
    for header in headers.get_all(axum::http::header::COOKIE) {
        let Ok(header) = header.to_str() else {
            continue;
        };
        for pair in header.split(';') {
            let name = pair.trim().split('=').next().unwrap_or_default();
            if valid_cookie_name(name) && seen.insert(name) {
                names.push(name.to_string());
            }
        }
    }
    (!names.is_empty()).then(|| names.into())
}

struct PreHandlerCookieNames(Option<usize>);

impl PreHandlerCookieNames {
    fn register(response: *const HttpResponse, names: Option<&Arc<[String]>>) -> Self {
        let Some(names) = names else {
            return Self(None);
        };
        let key = reply_header_key(response);
        request_cookie_names_lock().insert(key, Arc::clone(names));
        Self(Some(key))
    }
}

impl Drop for PreHandlerCookieNames {
    fn drop(&mut self) {
        if let Some(key) = self.0 {
            let _ = request_cookie_names_lock().remove(&key);
        }
    }
}

#[cfg(test)]
pub(crate) fn request_cookie_names_holding(name: &str) -> usize {
    request_cookie_names_lock()
        .values()
        .filter(|names| names.iter().any(|held| held == name))
        .count()
}

static REPLY_WRITABLE_OPEN: LazyLock<Mutex<HashMap<usize, ReplyWritableSignal>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone)]
struct ReplyWritableSignal {
    ready: Arc<tokio::sync::Notify>,
    opened: Arc<AtomicBool>,
}

fn register_reply_writable(response: *const HttpResponse) -> ReplyWritableSignal {
    let ready = Arc::new(tokio::sync::Notify::new());
    let opened = Arc::new(AtomicBool::new(false));
    let signal = ReplyWritableSignal {
        ready: Arc::clone(&ready),
        opened: Arc::clone(&opened),
    };
    let mut entries = REPLY_WRITABLE_OPEN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    entries.insert(reply_header_key(response), signal.clone());
    signal
}

fn unregister_reply_writable(response: *const HttpResponse) {
    let mut entries = REPLY_WRITABLE_OPEN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    entries.remove(&reply_header_key(response));
}

fn notify_reply_writable(response: *const HttpResponse) {
    let entries = REPLY_WRITABLE_OPEN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(signal) = entries.get(&reply_header_key(response)) {
        signal.opened.store(true, Ordering::Release);
        signal.ready.notify_one();
    }
}

#[derive(Clone, Copy)]
struct RateLimitBucket {
    updated_at: StdInstant,
    credit: u128,
    exceedances: u64,
    banned_until: Option<StdInstant>,
}

impl RateLimitBucket {
    fn is_banned_at(&self, now: StdInstant) -> bool {
        self.banned_until.is_some_and(|until| now < until)
    }
}

struct RateLimiterState {
    buckets: HashMap<IpAddr, RateLimitBucket>,
    global: Option<RateLimitBucket>,
}

pub(crate) struct RateLimiter {
    max: u64,
    global_max: Option<u64>,
    window: Duration,
    capacity: u128,
    global_capacity: Option<u128>,
    window_nanos: u128,
    ban: Option<u64>,
    ban_window: Duration,
    state: Mutex<RateLimiterState>,
}

impl RateLimiter {
    #[cfg(test)]
    pub(crate) fn new_with_global(max: i64, window_millis: i64, global_max: i64) -> Arc<Self> {
        Self::new_with_ban(max, window_millis, global_max, -1, -1)
    }

    pub(crate) fn new_with_ban(
        max: i64,
        window_millis: i64,
        global_max: i64,
        ban: i64,
        ban_window_millis: i64,
    ) -> Arc<Self> {
        let max = u64::try_from(max).unwrap_or(0);
        let window_millis = u64::try_from(window_millis).unwrap_or(0).max(1);
        let window = Duration::from_millis(window_millis);
        let window_nanos = window.as_nanos().max(1);
        let capacity = u128::from(max).saturating_mul(window_nanos);
        let global_max = (global_max >= 0).then(|| u64::try_from(global_max).unwrap_or(0));
        let global_capacity = global_max.map(|max| u128::from(max).saturating_mul(window_nanos));
        let ban = (ban >= 0).then(|| u64::try_from(ban).unwrap_or(0));
        let ban_window = Duration::from_millis(
            u64::try_from(ban_window_millis)
                .ok()
                .filter(|_| ban.is_some())
                .unwrap_or(window_millis),
        );
        Arc::new(Self {
            max,
            global_max,
            window,
            capacity,
            global_capacity,
            window_nanos,
            ban,
            ban_window,
            state: Mutex::new(RateLimiterState {
                buckets: HashMap::new(),
                global: None,
            }),
        })
    }

    pub(crate) fn allow(&self, peer: IpAddr) -> Result<(), RateLimitRejection> {
        self.allow_at(peer, StdInstant::now())
    }

    fn allow_at(&self, peer: IpAddr, now: StdInstant) -> Result<(), RateLimitRejection> {
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        };
        let RateLimiterState { buckets, global } = &mut *state;
        if !buckets.contains_key(&peer) && buckets.len() >= MAX_RATE_LIMIT_BUCKETS {
            buckets.retain(|_, bucket| {
                bucket.is_banned_at(now) || now.duration_since(bucket.updated_at) < self.window
            });
            if buckets.len() >= MAX_RATE_LIMIT_BUCKETS {
                Self::evict_oldest_batch(buckets, now);
            }
        }
        let bucket = buckets.entry(peer).or_insert(RateLimitBucket {
            updated_at: now,
            credit: self.capacity,
            exceedances: 0,
            banned_until: None,
        });
        if let Some(until) = bucket.banned_until {
            if now < until {
                bucket.updated_at = now;
                return Err(RateLimitRejection::Banned(until.duration_since(now)));
            }
            bucket.banned_until = None;
            bucket.exceedances = 0;
        }
        Self::refill(bucket, now, self.max, self.capacity);
        if let (Some(global_max), Some(global_capacity)) = (self.global_max, self.global_capacity) {
            let global = global.get_or_insert(RateLimitBucket {
                updated_at: now,
                credit: global_capacity,
                exceedances: 0,
                banned_until: None,
            });
            Self::refill(global, now, global_max, global_capacity);
            let identity_retry = Self::retry_after(bucket, self.window_nanos, self.max);
            let global_retry = Self::retry_after(global, self.window_nanos, global_max);
            if identity_retry.is_some() || global_retry.is_some() {
                if let Some(retry) = global_retry {
                    return Err(RateLimitRejection::Limited(retry));
                }
                return Err(self.reject_identity(
                    bucket,
                    identity_retry.expect("identity limit is rejected"),
                    now,
                ));
            }
            bucket.credit -= self.window_nanos;
            global.credit -= self.window_nanos;
            bucket.exceedances = 0;
            return Ok(());
        }
        if let Some(retry) = Self::retry_after(bucket, self.window_nanos, self.max) {
            return Err(self.reject_identity(bucket, retry, now));
        }
        bucket.credit -= self.window_nanos;
        bucket.exceedances = 0;
        Ok(())
    }

    fn reject_identity(
        &self,
        bucket: &mut RateLimitBucket,
        retry_after: Duration,
        now: StdInstant,
    ) -> RateLimitRejection {
        let Some(ban) = self.ban else {
            return RateLimitRejection::Limited(retry_after);
        };
        bucket.exceedances = bucket.exceedances.saturating_add(1);
        if bucket.exceedances > ban {
            bucket.banned_until = Some(now + self.ban_window);
            return RateLimitRejection::Banned(self.ban_window);
        }
        RateLimitRejection::Limited(retry_after)
    }

    fn evict_oldest_batch(buckets: &mut HashMap<IpAddr, RateLimitBucket>, now: StdInstant) {
        let target = (MAX_RATE_LIMIT_BUCKETS / RATE_LIMIT_EVICTION_DIVISOR).max(1);
        let mut candidates = buckets
            .iter()
            .filter(|(_, bucket)| !bucket.is_banned_at(now))
            .map(|(peer, bucket)| (bucket.updated_at, *peer))
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            candidates = buckets
                .iter()
                .map(|(peer, bucket)| (bucket.updated_at, *peer))
                .collect();
        }
        let target = target.min(candidates.len());
        if target == 0 {
            return;
        }
        candidates.select_nth_unstable(target - 1);
        let doomed = candidates[..target]
            .iter()
            .map(|(_, peer)| *peer)
            .collect::<Vec<_>>();
        for peer in doomed {
            buckets.remove(&peer);
        }
    }

    fn refill(bucket: &mut RateLimitBucket, now: StdInstant, max: u64, capacity: u128) {
        let elapsed = now.saturating_duration_since(bucket.updated_at).as_nanos();
        bucket.credit = refill_credit(bucket.credit, elapsed, max, capacity);
        bucket.updated_at = now;
    }

    fn retry_after(bucket: &RateLimitBucket, cost: u128, max: u64) -> Option<Duration> {
        retry_after_nanos(bucket.credit, cost, max).map(Duration::from_nanos)
    }
}

fn refill_credit(credit: u128, elapsed_nanos: u128, max: u64, capacity: u128) -> u128 {
    credit
        .saturating_add(elapsed_nanos.saturating_mul(u128::from(max)))
        .min(capacity)
}

fn retry_after_nanos(credit: u128, cost: u128, max: u64) -> Option<u64> {
    (credit < cost).then(|| {
        let missing = cost - credit;
        let divisor = u128::from(max).max(1);
        let nanos = missing.saturating_add(divisor - 1) / divisor;
        u64::try_from(nanos).unwrap_or(u64::MAX)
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RateLimitRejection {
    Limited(Duration),
    Banned(Duration),
}

fn empty_json_object() -> String {
    "{}".to_string()
}

fn http_text_value(value: &str) -> serde_json::Value {
    serde_json::Value::String(value.to_string())
}

fn object_json(object: serde_json::Map<String, serde_json::Value>) -> String {
    serde_json::Value::Object(object).to_string()
}

#[derive(Debug, Default, Clone)]
pub(crate) struct RouteParams {
    names: Vec<(usize, String)>,
}

impl RouteParams {
    pub(crate) fn parse(route_path: &str) -> Self {
        let names = route_path
            .split('/')
            .enumerate()
            .filter_map(|(index, segment)| {
                segment
                    .strip_prefix(':')
                    .filter(|name| !name.is_empty())
                    .map(|name| (index, name.to_string()))
            })
            .collect();
        Self { names }
    }

    fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

pub(crate) fn path_params_json(params: &RouteParams, request_path: &str) -> String {
    if params.is_empty() {
        return empty_json_object();
    }
    let segments: Vec<&str> = request_path.split('/').collect();
    let mut object = serde_json::Map::new();
    for (index, name) in &params.names {
        let value = segments.get(*index).copied().unwrap_or_default();
        object.insert(name.clone(), http_text_value(&decode_url_component(value)));
    }
    object_json(object)
}

fn decode_url_component(value: &str) -> String {
    let value = value.replace('+', " ");
    super::strings_core::url_decode(&value)
}

pub(crate) fn query_json(query: &str) -> String {
    let mut object = serde_json::Map::new();
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let mut parts = pair.splitn(2, '=');
        let Some(key) = parts.next().filter(|key| !key.is_empty()) else {
            continue;
        };
        let key = decode_url_component(key);
        let value = http_text_value(&decode_url_component(parts.next().unwrap_or_default()));
        match object.entry(key) {
            serde_json::map::Entry::Vacant(entry) => {
                entry.insert(value);
            }
            serde_json::map::Entry::Occupied(mut entry) => match entry.get_mut() {
                serde_json::Value::Array(values) => values.push(value),
                previous => {
                    let previous = std::mem::replace(previous, serde_json::Value::Null);
                    *entry.get_mut() = serde_json::Value::Array(vec![previous, value]);
                }
            },
        }
    }
    object_json(object)
}

pub(crate) fn headers_json(headers: &axum::http::HeaderMap) -> String {
    let mut object = serde_json::Map::new();
    for (name, value) in headers {
        if let Ok(value) = value.to_str() {
            object.insert(
                name.as_str().to_ascii_lowercase(),
                serde_json::Value::String(value.to_string()),
            );
        }
    }
    object_json(object)
}

pub(crate) fn cookies_json(headers: &axum::http::HeaderMap) -> String {
    let mut object = serde_json::Map::new();
    let Some(cookie) = headers.get("cookie").and_then(|value| value.to_str().ok()) else {
        return object_json(object);
    };
    for cookie in cookie.split(';') {
        let mut parts = cookie.trim().splitn(2, '=');
        let Some(name) = parts.next().filter(|name| !name.is_empty()) else {
            continue;
        };
        object.insert(
            name.to_string(),
            serde_json::Value::String(parts.next().unwrap_or_default().to_string()),
        );
    }
    object_json(object)
}

fn cookie_value<'a>(headers: &'a axum::http::HeaderMap, expected: &str) -> Option<&'a str> {
    let cookie = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    cookie.split(';').find_map(|part| {
        let mut pair = part.trim().splitn(2, '=');
        let name = pair.next()?.trim();
        (name == expected).then(|| pair.next().unwrap_or_default())
    })
}

fn csrf_tokens_match(cookie: &str, header: &str) -> bool {
    let mut different = cookie.len() ^ header.len();
    let longest = cookie.len().max(header.len());
    for index in 0..longest {
        let left = cookie.as_bytes().get(index).copied().unwrap_or(0);
        let right = header.as_bytes().get(index).copied().unwrap_or(0);
        different |= usize::from(left ^ right);
    }
    different == 0
}

fn csrf_protected_method(method: &Method) -> bool {
    !matches!(
        *method,
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    )
}

fn csrf_request_is_valid(headers: &axum::http::HeaderMap, csrf: &CsrfConfig) -> bool {
    let Some(cookie) = cookie_value(headers, &csrf.cookie) else {
        return false;
    };
    let Some(header) = headers
        .get(&csrf.header)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    csrf_tokens_match(cookie, header)
}

pub(crate) fn axum_route_path(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            segment
                .strip_prefix(':')
                .map_or_else(|| segment.to_string(), |name| format!("{{{name}}}"))
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn http_error_json(status: i32, code: &str, message: &str) -> String {
    serde_json::json!({
        "statusCode": status,
        "code": code,
        "message": message,
    })
    .to_string()
}

fn internal_server_error_response(reason: &str) -> Response<Body> {
    eprintln!("error: internal HTTP handler error: {reason}");
    let body = http_error_json(500, "internal_server_error", "Internal server error");
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header("content-type", "application/json; charset=utf-8")
        .header("content-length", body.len())
        .body(Body::from(body))
        .unwrap()
}

pub(crate) fn rate_limited_response(retry_after: Duration) -> Response<Body> {
    let retry_after_seconds = retry_after
        .as_secs()
        .saturating_add(u64::from(retry_after.subsec_nanos() != 0))
        .max(1);
    let body = http_error_json(429, "rate_limit_exceeded", "Too many requests");
    Response::builder()
        .status(StatusCode::TOO_MANY_REQUESTS)
        .header("content-type", "application/json; charset=utf-8")
        .header("retry-after", retry_after_seconds.to_string())
        .header("content-length", body.len())
        .body(Body::from(body))
        .unwrap()
}

pub(crate) fn rate_limit_banned_response(retry_after: Duration) -> Response<Body> {
    let retry_after_seconds = retry_after
        .as_secs()
        .saturating_add(u64::from(retry_after.subsec_nanos() != 0))
        .max(1);
    let body = http_error_json(
        403,
        "rate_limit_banned",
        "Client identity is temporarily banned",
    );
    Response::builder()
        .status(StatusCode::FORBIDDEN)
        .header("content-type", "application/json; charset=utf-8")
        .header("retry-after", retry_after_seconds.to_string())
        .header("content-length", body.len())
        .body(Body::from(body))
        .unwrap()
}

pub(crate) fn rate_limit_response(rejection: RateLimitRejection) -> Response<Body> {
    match rejection {
        RateLimitRejection::Limited(retry_after) => rate_limited_response(retry_after),
        RateLimitRejection::Banned(retry_after) => rate_limit_banned_response(retry_after),
    }
}

fn csrf_forbidden_response() -> Response<Body> {
    let body = http_error_json(403, "csrf_invalid", "CSRF token is missing or invalid");
    Response::builder()
        .status(StatusCode::FORBIDDEN)
        .header("content-type", "application/json; charset=utf-8")
        .header("content-length", body.len())
        .body(Body::from(body))
        .unwrap()
}

thread_local! {
    static LAST_HTTP_ERROR: std::cell::Cell<Option<(&'static str, &'static str)>> =
        const { std::cell::Cell::new(None) };
}

pub(crate) fn set_http_error(code: &'static str, message: &'static str) {
    LAST_HTTP_ERROR.with(|slot| slot.set(Some((code, message))));
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_http_clear_error() {
    LAST_HTTP_ERROR.with(|slot| slot.set(None));
}

/// Consumes this thread's pending `tyrus:http` error, if any, filling `out`.
///
/// Leaves `out` untouched when nothing is pending, so the caller decides
/// success by reading `out->code` from its own pre-zeroed slot rather than by
/// a `bool` return whose ABI width is not worth depending on. The strings
/// point into process-lifetime storage: the caller may retain them and must
/// not free them. Mirrors `tyrus_os_take_error`.
///
/// # Safety
///
/// `out` must point at a writable, zeroed `HttpErrorTaken`, or be null (a null
/// `out` still consumes the pending error, discarding it).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_take_error(out: *mut crate::abi::HttpErrorTaken) {
    LAST_HTTP_ERROR.with(|slot| {
        let (code, message) = slot
            .take()
            .unwrap_or(("ERR_HTTP_CONFIGURATION", "HTTP configuration was rejected"));
        if !out.is_null() {
            unsafe {
                (*out).code = TyrusString::literal(code);
                (*out).message = TyrusString::literal(message);
            }
        }
    });
}

unsafe fn server_host(options: *const HttpServerOptions) -> Result<IpAddr, String> {
    unsafe {
        let Some(options) = options.as_ref() else {
            return Ok(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        };
        let Ok(host) = text(&options.host) else {
            return Ok(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        };
        if host.trim().is_empty() {
            return Ok(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        }
        host.trim().parse().map_err(|_| host)
    }
}

fn bind_failure_message(kind: std::io::ErrorKind) -> &'static str {
    match kind {
        std::io::ErrorKind::AddrInUse => "the address is already in use",
        std::io::ErrorKind::AddrNotAvailable => "the address is not available on this host",
        std::io::ErrorKind::PermissionDenied => {
            "permission denied for this address; ports below 1024 usually require elevated privileges"
        }
        _ => "could not bind the server address",
    }
}

unsafe fn server_tls(
    options: &HttpServerOptions,
) -> Result<Option<std::sync::Arc<tokio_rustls::TlsAcceptor>>, &'static str> {
    let cert = text(&options.tls_cert).unwrap_or_default();
    let key = text(&options.tls_key).unwrap_or_default();
    match (cert.trim().is_empty(), key.trim().is_empty()) {
        (true, true) => Ok(None),
        (false, false) => crate::http_tls::acceptor(cert.trim(), key.trim())
            .map(Some)
            .map_err(|error| error.message),
        (true, false) => Err("`tls.cert` is missing; a key without a certificate names nobody"),
        (false, true) => Err("`tls.key` is missing; a certificate without its key proves nothing"),
    }
}

/// Moves the request clock, the limits and the TLS material into the shared
/// server config, where the accept loop and the routes read them.
///
/// Every failure here happens before the socket is bound, so a server that
/// cannot honour the configuration it was given never claims the port.
pub(crate) unsafe fn apply_server_options(
    router: &HttpRouter,
    options: *const HttpServerOptions,
) -> i32 {
    unsafe {
        let Some(options) = options.as_ref() else {
            return match crate::http_dev_tls::development_acceptor() {
                Ok(Some(tls)) => {
                    let mut config = match router.config.write() {
                        Ok(config) => config,
                        Err(poisoned) => poisoned.into_inner(),
                    };
                    config.tls = Some(tls);
                    0
                }
                Ok(None) => 0,
                Err(message) => {
                    set_http_error("ERR_HTTP_TLS", message);
                    -1
                }
            };
        };
        let request_timeout = match options.request_timeout_millis {
            millis if millis > 0 => Some(Duration::from_millis(millis as u64)),
            0 => {
                set_http_error(
                    "ERR_HTTP_LISTEN",
                    "`requestTimeout` must be greater than zero",
                );
                return -1;
            }
            _ => None,
        };
        let concurrency = match options.max_concurrent_requests {
            limit if limit > 0 => {
                let limit = usize::try_from(limit).unwrap_or(usize::MAX);
                Some(std::sync::Arc::new(tokio::sync::Semaphore::new(limit)))
            }
            0 => {
                set_http_error(
                    "ERR_HTTP_LISTEN",
                    "`maxConcurrentRequests` must be greater than zero",
                );
                return -1;
            }
            _ => None,
        };
        let max_connections = match options.max_connections {
            limit if limit > 0 => {
                let limit = usize::try_from(limit).unwrap_or(usize::MAX);
                Some(std::sync::Arc::new(tokio::sync::Semaphore::new(limit)))
            }
            0 => {
                set_http_error(
                    "ERR_HTTP_LISTEN",
                    "`maxConnections` must be greater than zero",
                );
                return -1;
            }
            _ => None,
        };
        let keep_alive = options.keep_alive != 0;
        let tls = match server_tls(options) {
            Ok(Some(tls)) => Some(tls),
            Ok(None) => match crate::http_dev_tls::development_acceptor() {
                Ok(tls) => tls,
                Err(message) => {
                    set_http_error("ERR_HTTP_TLS", message);
                    return -1;
                }
            },
            Err(message) => {
                set_http_error("ERR_HTTP_TLS", message);
                return -1;
            }
        };
        let mut config = match router.config.write() {
            Ok(config) => config,
            Err(poisoned) => poisoned.into_inner(),
        };
        config.request_timeout = request_timeout;
        config.concurrency = concurrency;
        config.max_connections = max_connections;
        config.keep_alive = keep_alive;
        config.logger = options.logger == 1;
        config.tls = tls;
        if let Ok(millis) = u64::try_from(options.header_timeout_millis)
            && millis > 0
        {
            config.header_read_timeout = Duration::from_millis(millis);
        }
        if let Ok(bytes) = usize::try_from(options.max_header_bytes)
            && bytes > 0
        {
            config.max_header_bytes = bytes.max(MIN_MAX_HEADER_BYTES);
        }
        0
    }
}

async fn acquire_permit(
    semaphore: &Arc<tokio::sync::Semaphore>,
    request_timeout: Option<Duration>,
) -> Option<tokio::sync::OwnedSemaphorePermit> {
    let waiting = Arc::clone(semaphore).acquire_owned();
    match request_timeout {
        None => waiting.await.ok(),
        Some(limit) => tokio::time::timeout(limit, waiting).await.ok()?.ok(),
    }
}

fn declared_request_content_type(request: &Request) -> Option<String> {
    request
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn is_json_content_type(content_type: &str) -> bool {
    let media_type = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    media_type == "application/json" || media_type.ends_with("+json")
}

fn unsupported_media_type_response() -> Response<Body> {
    let body = http_error_json(
        415,
        "unsupported_media_type",
        "Request body must be application/json",
    );
    Response::builder()
        .status(StatusCode::UNSUPPORTED_MEDIA_TYPE)
        .header("content-type", "application/json; charset=utf-8")
        .header("accept", "application/json")
        .header("content-length", body.len())
        .body(Body::from(body))
        .unwrap()
}

fn overloaded_response() -> Response<Body> {
    let body = http_error_json(503, "server_overloaded", "Server is at capacity");
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header("content-type", "application/json; charset=utf-8")
        .header("content-length", body.len())
        .body(Body::from(body))
        .unwrap()
}

fn exceeded_body_limit(error: &axum::Error) -> bool {
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(current) = source {
        if current
            .downcast_ref::<http_body_util::LengthLimitError>()
            .is_some()
        {
            return true;
        }
        source = current.source();
    }
    false
}

fn payload_too_large_response() -> Response<Body> {
    let body = http_error_json(413, "payload_too_large", "Request body is too large");
    Response::builder()
        .status(StatusCode::PAYLOAD_TOO_LARGE)
        .header("content-type", "application/json; charset=utf-8")
        .header("content-length", body.len())
        .body(Body::from(body))
        .unwrap()
}

fn multipart_refusal_response(refusal: crate::http_multipart::MultipartRefusal) -> Response<Body> {
    use crate::http_multipart::MultipartRefusal;
    let (code, message) = match refusal {
        MultipartRefusal::NotMultipart => (
            "unsupported_media_type",
            "Request body must be multipart/form-data",
        ),
        MultipartRefusal::Malformed => (
            "malformed_body",
            "Request body could not be read as multipart/form-data",
        ),
        MultipartRefusal::TooLarge => ("payload_too_large", "Request body is too large"),
    };
    let status = refusal.status();
    let body = http_error_json(i32::from(status), code, message);
    Response::builder()
        .status(status)
        .header("content-type", "application/json; charset=utf-8")
        .header("content-length", body.len())
        .body(Body::from(body))
        .unwrap()
}

fn malformed_body_response() -> Response<Body> {
    let body = http_error_json(400, "malformed_body", "Request body could not be read");
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("content-type", "application/json; charset=utf-8")
        .header("content-length", body.len())
        .body(Body::from(body))
        .unwrap()
}

fn ambiguous_framing_response() -> Response<Body> {
    let body = http_error_json(
        400,
        "ambiguous_framing",
        "Request carries both content-length and transfer-encoding",
    );
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("content-type", "application/json; charset=utf-8")
        .header("content-length", body.len())
        .header("connection", "close")
        .body(Body::from(body))
        .unwrap()
}

fn timed_out_response() -> Response<Body> {
    let body = http_error_json(503, "request_timeout", "Request took too long");
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header("content-type", "application/json; charset=utf-8")
        .header("content-length", body.len())
        .body(Body::from(body))
        .unwrap()
}

pub(crate) fn client_address(
    peer: IpAddr,
    headers: &axum::http::HeaderMap,
    config: &std::sync::RwLock<HttpServerConfig>,
) -> IpAddr {
    let (trusted, identity_header) = match config.read() {
        Ok(config) => (
            config.trusted_proxies.clone(),
            config.trusted_proxy_identity_header.clone(),
        ),
        Err(poisoned) => {
            let config = poisoned.into_inner();
            (
                config.trusted_proxies.clone(),
                config.trusted_proxy_identity_header.clone(),
            )
        }
    };
    if trusted.is_empty() || !trusted.iter().any(|proxy| proxy.contains(peer)) {
        return peer;
    }
    let chains = match identity_header.as_deref() {
        Some(header) => vec![forwarded_addresses_from_header(headers, header)],
        None => forwarded_chains(headers),
    };
    let mut agreed: Option<IpAddr> = None;
    for chain in chains {
        let Some(candidate) = chain
            .iter()
            .rev()
            .find(|address| !trusted.iter().any(|proxy| proxy.contains(**address)))
            .copied()
        else {
            continue;
        };
        match agreed {
            Some(previous) if previous != candidate => return peer,
            _ => agreed = Some(candidate),
        }
    }
    agreed.unwrap_or(peer)
}

fn forwarded_chains(headers: &axum::http::HeaderMap) -> Vec<Vec<IpAddr>> {
    let standard = headers
        .get_all(axum::http::header::FORWARDED)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .filter_map(forwarded_element_address)
        .collect::<Vec<_>>();
    let legacy = headers
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .filter_map(|value| parse_forwarded_address(value.trim()))
        .collect::<Vec<_>>();
    [standard, legacy]
        .into_iter()
        .filter(|chain| !chain.is_empty())
        .collect()
}

fn forwarded_addresses_from_header(headers: &axum::http::HeaderMap, header: &str) -> Vec<IpAddr> {
    headers
        .get_all(header)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .filter_map(|value| parse_forwarded_address(value.trim()))
        .collect()
}

fn forwarded_element_address(element: &str) -> Option<IpAddr> {
    element.split(';').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        name.trim().eq_ignore_ascii_case("for").then_some(())?;
        parse_forwarded_address(value.trim().trim_matches('"'))
    })
}

fn parse_forwarded_address(value: &str) -> Option<IpAddr> {
    let value = value.trim();
    if let Some(rest) = value.strip_prefix('[') {
        let (address, _) = rest.split_once(']')?;
        return address.parse().ok();
    }
    if let Ok(address) = value.parse::<IpAddr>() {
        return Some(address);
    }
    value.rsplit_once(':')?.0.parse().ok()
}

fn effective_body_limit(body_limit: i64) -> usize {
    match body_limit {
        0 => usize::MAX,
        n if n > 0 => usize::try_from(n).unwrap_or(usize::MAX),
        _ => MAX_REQUEST_BODY,
    }
}

pub(crate) fn stream_request_body(
    body: Body,
    body_limit: usize,
) -> *const crate::stream::TyrusReadableStream {
    let capacity = body_limit.clamp(1, HTTP_STREAM_BUFFER_BYTES);
    let (readable, producer) = crate::stream::byte_readable_stream(capacity);
    executor().spawn(async move {
        let mut received = 0usize;
        let mut source = body.into_data_stream();
        loop {
            let chunk = tokio::select! {
                chunk = source.next() => chunk,
                () = producer.writable_reader_gone() => return,
            };
            let Some(chunk) = chunk else {
                producer.close();
                return;
            };
            let Ok(chunk) = chunk else {
                producer.fail();
                return;
            };
            let next = match received.checked_add(chunk.len()) {
                Some(next) if next <= body_limit => next,
                _ => {
                    producer.fail();
                    return;
                }
            };
            received = next;
            for piece in chunk.chunks(capacity) {
                if producer.write(piece.to_vec()).await.is_err() {
                    return;
                }
            }
        }
    });
    readable
}

#[allow(clippy::too_many_arguments)]
async fn invoke_with_timeout(
    handler: HttpHandler,
    route_params: std::sync::Arc<RouteParams>,
    body_limit: i64,
    route_flags: i64,
    rate_limiter: Option<Arc<RateLimiter>>,
    pre_handlers: Vec<HttpPreHandler>,
    config: Arc<std::sync::RwLock<HttpServerConfig>>,
    request: Request,
) -> Response<Body> {
    let is_head = request.method() == Method::HEAD;
    let limit = match config.read() {
        Ok(config) => config.request_timeout,
        Err(poisoned) => {
            let config = poisoned.into_inner();
            config.request_timeout
        }
    };
    let running = invoke(
        handler,
        route_params,
        body_limit,
        route_flags,
        rate_limiter,
        pre_handlers,
        config,
        request,
    );
    match limit {
        None => running.await,
        Some(limit) => match tokio::time::timeout(limit, running).await {
            Ok(response) => response,
            Err(_) => omit_body_for_head(timed_out_response(), is_head),
        },
    }
}

#[allow(clippy::too_many_arguments)]
async fn invoke(
    handler: HttpHandler,
    route_params: std::sync::Arc<RouteParams>,
    body_limit: i64,
    route_flags: i64,
    rate_limiter: Option<Arc<RateLimiter>>,
    pre_handlers: Vec<HttpPreHandler>,
    config: Arc<std::sync::RwLock<HttpServerConfig>>,
    request: Request,
) -> Response<Body> {
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .map_or_else(|| "req-unknown".to_string(), |id| id.0.clone());
    let is_head = request.method() == Method::HEAD;
    let conditional_get = request.method() == Method::GET || request.method() == Method::HEAD;
    let if_none_match = request
        .headers()
        .get(axum::http::header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let if_modified_since = request
        .headers()
        .get(axum::http::header::IF_MODIFIED_SINCE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let accept_encoding = request
        .headers()
        .get(axum::http::header::ACCEPT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let peer = request
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED), |peer| peer.0.ip());
    let client = client_address(peer, request.headers(), &config);
    let (request_timeout, concurrency, server_rate_limit) = match config.read() {
        Ok(config) => (
            config.request_timeout,
            config.concurrency.clone(),
            config.rate_limit.clone(),
        ),
        Err(poisoned) => {
            let config = poisoned.into_inner();
            (
                config.request_timeout,
                config.concurrency.clone(),
                config.rate_limit.clone(),
            )
        }
    };
    let csrf = match config.read() {
        Ok(config) => config.csrf.clone(),
        Err(poisoned) => poisoned.into_inner().csrf.clone(),
    };
    if csrf_protected_method(request.method())
        && csrf.is_some_and(|csrf| !csrf_request_is_valid(request.headers(), &csrf))
    {
        return omit_body_for_head(csrf_forbidden_response(), is_head);
    }
    if let Some(rate_limiter) = rate_limiter.or(server_rate_limit)
        && let Err(rejection) = rate_limiter.allow(client)
    {
        return omit_body_for_head(rate_limit_response(rejection), is_head);
    }
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let query = request.uri().query().unwrap_or_default().to_string();
    let params = path_params_json(&route_params, &path);
    let query = query_json(&query);
    let headers = headers_json(request.headers());
    let cookies = cookies_json(request.headers());
    let cookie_names = request_cookie_names(request.headers());
    if request
        .headers()
        .contains_key(axum::http::header::CONTENT_LENGTH)
        && request
            .headers()
            .contains_key(axum::http::header::TRANSFER_ENCODING)
    {
        return omit_body_for_head(ambiguous_framing_response(), is_head);
    }
    if route_flags & ROUTE_EXPECTS_JSON_BODY != 0
        && let Some(content_type) = declared_request_content_type(&request)
        && !is_json_content_type(&content_type)
    {
        return omit_body_for_head(unsupported_media_type_response(), is_head);
    }
    let multipart_content_type = if route_flags & ROUTE_EXPECTS_MULTIPART_BODY != 0 {
        match declared_request_content_type(&request) {
            Some(content_type) => Some(content_type),
            None => return omit_body_for_head(unsupported_media_type_response(), is_head),
        }
    } else {
        None
    };
    let _permit = match concurrency {
        None => None,
        Some(semaphore) => match acquire_permit(&semaphore, request_timeout).await {
            Some(permit) => Some(permit),
            None => return omit_body_for_head(overloaded_response(), is_head),
        },
    };
    let stream_body = route_flags & ROUTE_EXPECTS_STREAM_BODY != 0;
    let stream_limit = effective_body_limit(body_limit);
    let request_body = request.into_body();
    let (body, body_stream_address) = if stream_body {
        (
            Bytes::new(),
            Some(stream_request_body(request_body, stream_limit) as usize),
        )
    } else {
        let body = match to_bytes(request_body, stream_limit).await {
            Ok(body) => body,
            Err(error) if exceeded_body_limit(&error) => {
                return omit_body_for_head(payload_too_large_response(), is_head);
            }
            Err(_) => {
                return omit_body_for_head(malformed_body_response(), is_head);
            }
        };
        (body, None)
    };
    let multipart_parts = match multipart_content_type {
        None => Vec::new(),
        Some(content_type) => {
            match crate::http_multipart::parse_multipart(
                body.clone(),
                &content_type,
                crate::http_multipart::DEFAULT_MAX_PART_BYTES,
            )
            .await
            {
                Ok(parts) => unsafe { crate::http_multipart::into_abi_parts(parts) },
                Err(refusal) => {
                    return omit_body_for_head(multipart_refusal_response(refusal), is_head);
                }
            }
        }
    };
    let slot = ResponseSlot::new();
    slot.register_cookie_names(cookie_names.as_ref());
    let writable_signal = register_reply_writable(slot.as_ptr());
    let promise_address = {
        let body_bytes = body.as_ref();
        let body = String::from_utf8_lossy(body_bytes);
        // SAFETY: every source outlives this scope, which ends before the await.
        let lend =
            |text: &str| unsafe { TyrusString::lend(text) }.unwrap_or_else(TyrusString::null);
        let request = HttpRequest {
            id: lend(&request_id),
            method: lend(&method),
            path: lend(&path),
            body: lend(&body),
            params: lend(&params),
            query: lend(&query),
            headers: lend(&headers),
            cookies: lend(&cookies),
            raw_body: body_bytes.as_ptr(),
            raw_body_length: body_bytes.len(),
            parts: multipart_parts.as_ptr(),
            parts_length: multipart_parts.len(),
            body_stream: body_stream_address.map_or(std::ptr::null(), |stream| {
                stream as *const crate::stream::TyrusReadableStream
            }),
        };
        for pre_handler in &pre_handlers {
            let mut pre_response = HttpResponse::empty();
            let pre_key: *const HttpResponse = &pre_response;
            let _cookie_names = PreHandlerCookieNames::register(pre_key, cookie_names.as_ref());
            let produced = unsafe { pre_handler(&request, &mut pre_response) };
            if produced != 0 {
                return finalize_response(
                    pre_response,
                    pre_key,
                    is_head,
                    conditional_get,
                    if_none_match.as_deref(),
                    if_modified_since.as_deref(),
                    accept_encoding.as_deref(),
                );
            }
        }
        (unsafe { handler(&request, slot.as_ptr()) }) as usize
    };
    if let Some(stream) = body_stream_address {
        unsafe {
            crate::stream::tyrus_readable_stream_free(
                stream as *const crate::stream::TyrusReadableStream,
            );
        };
    }
    unsafe { crate::http_multipart::release_parts(multipart_parts) };
    if promise_address != 0 {
        let held = crate::promise::PromiseReference(promise_address);
        let writing = Arc::clone(&slot);
        let mut settled = std::pin::pin!(unsafe {
            crate::promise::settled(promise_address as *mut TyrusPromise, writing)
        });
        let settled = loop {
            if writable_signal.opened.load(Ordering::Acquire) {
                break None;
            }
            tokio::select! {
                state = &mut settled => break Some(state),
                () = writable_signal.ready.notified() => {}
            }
        };
        if settled.is_none() {
            let response = unsafe { slot.take() };
            return finalize_response(
                response,
                slot.as_ptr(),
                is_head,
                conditional_get,
                if_none_match.as_deref(),
                if_modified_since.as_deref(),
                accept_encoding.as_deref(),
            );
        }
        let promise = held.0 as *mut TyrusPromise;
        let state = settled.expect("the handler settlement is present");
        if state != TYRUS_PROMISE_FULFILLED {
            let reason = unsafe {
                let message = tyrus_promise_error_message(promise);
                if message.is_null() {
                    format!("async handler failed with state {state}")
                } else {
                    message.as_str().to_owned()
                }
            };
            return omit_body_for_head(internal_server_error_response(&reason), is_head);
        }
    }
    // SAFETY: a synchronous handler has returned, and an async one settled its
    // Promise above, so every write to the slot has happened.
    let response = unsafe { slot.take() };
    finalize_response(
        response,
        slot.as_ptr(),
        is_head,
        conditional_get,
        if_none_match.as_deref(),
        if_modified_since.as_deref(),
        accept_encoding.as_deref(),
    )
}

pub(crate) fn finalize_response(
    mut response: HttpResponse,
    header_key: *const HttpResponse,
    is_head: bool,
    conditional_get: bool,
    if_none_match: Option<&str>,
    if_modified_since: Option<&str>,
    accept_encoding: Option<&str>,
) -> Response<Body> {
    let streaming_body = response.body.tag == 2;
    let writable_streaming_body = response.body.tag == 3;
    let binary_body = response.body.tag == 1 || streaming_body || writable_streaming_body;
    let body = unsafe {
        match response.body.tag {
            0 => response
                .body
                .text()
                .ok_or(())
                .and_then(|body_text| text(body_text).map(std::string::String::into_bytes)),
            1 => response.body.bytes().ok_or(()).map(|(data, length)| {
                if data.is_null() || length == 0 {
                    Vec::new()
                } else {
                    std::slice::from_raw_parts(data, length).to_vec()
                }
            }),
            2 => Ok(Vec::new()),
            3 => Ok(Vec::new()),
            _ => Err(()),
        }
    };
    let content_type = unsafe {
        response
            .content_type
            .as_ref()
            .and_then(|value| text(value).ok())
    };
    let cookies: Vec<HeaderValue> = text(&response.cookies)
        .unwrap_or_default()
        .lines()
        .filter(|cookie| !cookie.trim().is_empty())
        .filter_map(|cookie| HeaderValue::from_str(cookie).ok())
        .collect();
    let mut headers = take_reply_headers(header_key);

    let stream = if streaming_body {
        unsafe {
            response
                .body
                .take_stream()
                .map(|stream| crate::stream::take_owned_readable_stream(stream))
        }
    } else {
        None
    };
    let writable = if writable_streaming_body {
        unsafe { response.body.take_writable_stream() }
    } else {
        None
    };

    // Which field was safe to reclaim used to be an open question: `.body` can
    // hold text the handler allocated or text that lives in `.rodata`, and
    // nothing in a bare pointer told the two apart, so freeing it
    // unconditionally risked a crash and not freeing it leaked every allocated
    // body. The ownership bit answers it: releasing a value that owns nothing is
    // a no-op. This is the same release a cancelled request gets from the slot's
    // `Drop`, which is why both paths call one function.
    // SAFETY: every writer has finished, and the values were read above.
    unsafe { response.release() };

    let Ok(body) = body else {
        if let Some(stream) = stream {
            drop(stream);
        }
        if let Some(writable) = writable {
            drop(writable);
        }
        return omit_body_for_head(
            internal_server_error_response("handler returned an invalid body"),
            is_head,
        );
    };
    let mut status = match u16::try_from(response.status)
        .ok()
        .and_then(|status| StatusCode::from_u16(status).ok())
    {
        Some(status) => status,
        None => {
            if let Some(stream) = stream {
                drop(stream);
            }
            if let Some(writable) = writable {
                drop(writable);
            }
            return omit_body_for_head(
                internal_server_error_response("handler returned an invalid status"),
                is_head,
            );
        }
    };
    if !streaming_body
        && !writable_streaming_body
        && conditional_get
        && status.is_success()
        && conditional_request_matches(if_none_match, if_modified_since, &headers)
    {
        status = StatusCode::NOT_MODIFIED;
    }
    let bodyless = http_status_forbids_body(status);
    let content_type = response_content_type(&headers, content_type, binary_body);
    let stream_encoding = (streaming_body || writable_streaming_body)
        .then(|| streaming_response_encoding(accept_encoding, content_type.as_deref(), &headers))
        .flatten();
    if let Some(encoding) = stream_encoding {
        add_or_merge_vary_accept_encoding(&mut headers);
        headers.push((
            axum::http::header::CONTENT_ENCODING,
            HeaderValue::from_static(encoding.header_value()),
        ));
    }
    let body = if bodyless || streaming_body || writable_streaming_body {
        body
    } else {
        maybe_compress_response(body, accept_encoding, content_type.as_deref(), &mut headers)
    };
    let mut builder = Response::builder().status(status);
    if !bodyless
        && !headers
            .iter()
            .any(|(name, _)| *name == axum::http::header::CONTENT_TYPE)
    {
        builder = builder.header("content-type", content_type.unwrap_or_default());
    }
    if is_head && !bodyless && !streaming_body && !writable_streaming_body {
        builder = builder.header(axum::http::header::CONTENT_LENGTH, body.len());
    }
    let mut response = builder
        .body(match stream {
            Some(stream) if !bodyless && !is_head => Body::from_stream(response_stream_body(
                HttpResponseStreamSource::Readable(stream),
                stream_encoding,
            )),
            Some(stream) => {
                drop(stream);
                Body::empty()
            }
            None if writable_streaming_body && !bodyless && !is_head => {
                Body::from_stream(response_stream_body(
                    HttpResponseStreamSource::Writable(writable),
                    stream_encoding,
                ))
            }
            None if writable_streaming_body => {
                drop(writable);
                Body::empty()
            }
            None if bodyless || is_head => Body::empty(),
            None => Body::from(body),
        })
        .unwrap();
    for cookie in cookies {
        response.headers_mut().append("set-cookie", cookie);
    }
    for (name, value) in headers {
        response.headers_mut().append(name, value);
    }
    response
}

fn response_content_type(
    headers: &[(HeaderName, HeaderValue)],
    explicit_content_type: Option<String>,
    binary_body: bool,
) -> Option<String> {
    headers
        .iter()
        .find(|(name, _)| *name == axum::http::header::CONTENT_TYPE)
        .and_then(|(_, value)| value.to_str().ok().map(str::to_owned))
        .or(explicit_content_type)
        .or_else(|| {
            Some(if binary_body {
                "application/octet-stream".to_string()
            } else {
                "text/plain; charset=utf-8".to_string()
            })
        })
}

fn maybe_compress_response(
    body: Vec<u8>,
    accept_encoding: Option<&str>,
    content_type: Option<&str>,
    headers: &mut Vec<(HeaderName, HeaderValue)>,
) -> Vec<u8> {
    let Some(encoding) = preferred_response_encoding(accept_encoding) else {
        return body;
    };
    if body.len() < MIN_COMPRESSIBLE_RESPONSE_BYTES
        || !content_type.is_some_and(is_compressible_content_type)
        || headers
            .iter()
            .any(|(name, _)| *name == axum::http::header::CONTENT_ENCODING)
        || headers.iter().any(|(name, value)| {
            *name == axum::http::header::ETAG
                && value
                    .to_str()
                    .is_ok_and(|etag| valid_entity_tag(etag) && !etag.starts_with("W/"))
        })
    {
        return body;
    }
    let original_len = body.len();
    let compressed = match compress_body(&body, encoding) {
        Ok(compressed) => compressed,
        Err(_) => return body,
    };
    if compressed.len() >= original_len {
        return body;
    }
    add_or_merge_vary_accept_encoding(headers);
    headers.push((
        axum::http::header::CONTENT_ENCODING,
        HeaderValue::from_static(encoding.header_value()),
    ));
    compressed
}

enum HttpResponseStreamSource {
    Readable(Arc<crate::stream::TyrusReadableStream>),
    Writable(Option<Box<crate::stream::ByteStreamConsumer>>),
}

impl Drop for HttpResponseStreamSource {
    fn drop(&mut self) {
        if let Self::Readable(stream) = self {
            crate::stream::close_owned_byte_stream(stream);
        }
    }
}

impl HttpResponseStreamSource {
    async fn next(&mut self) -> Result<Option<Vec<u8>>, std::io::Error> {
        match self {
            Self::Readable(stream) => crate::stream::read_owned_byte_stream(stream)
                .await
                .map_err(|error| std::io::Error::other(error.message)),
            Self::Writable(writable) => {
                let Some(writable) = writable.as_ref() else {
                    return Ok(None);
                };
                match writable.read().await {
                    Ok(Some(bytes)) => Ok(Some(bytes)),
                    Ok(None) => {
                        writable.close();
                        *self = Self::Writable(None);
                        Ok(None)
                    }
                    Err(error) => {
                        writable.fail();
                        *self = Self::Writable(None);
                        Err(std::io::Error::other(error.message))
                    }
                }
            }
        }
    }
}

/// Streamed responses flush every chunk, so each one reaches the client as
/// soon as it is produced; the levels match the whole-body compression.
fn response_stream_encoder(
    encoding: Option<ResponseEncoding>,
) -> Option<crate::compress::streams::EncoderTransform> {
    let (format, level) = match encoding? {
        ResponseEncoding::Brotli => (crate::codec::Format::Brotli, 5),
        ResponseEncoding::Gzip => (crate::codec::Format::Gzip, 6),
    };
    let encoder = crate::codec::Encoder::new(format, Some(level)).ok()?;
    Some(crate::compress::streams::EncoderTransform(
        encoder.flushing(),
    ))
}

const RESPONSE_TRANSFORM_CAPACITY_BYTES: usize = 256 * 1024;

fn compressed_response_source(
    mut source: HttpResponseStreamSource,
    encoder: crate::compress::streams::EncoderTransform,
) -> HttpResponseStreamSource {
    let transform =
        crate::stream::byte_transform_stream(RESPONSE_TRANSFORM_CAPACITY_BYTES, Box::new(encoder));
    let (readable, writable) = transform.sides();
    drop(transform);
    crate::executor::executor().spawn(async move {
        loop {
            let next = tokio::select! {
                next = source.next() => next,
                () = crate::stream::owned_writable_reader_gone(&writable) => return,
            };
            match next {
                Ok(Some(bytes)) => {
                    if crate::stream::write_owned_byte_stream(
                        &writable,
                        bytes,
                        RESPONSE_TRANSFORM_CAPACITY_BYTES,
                    )
                    .await
                    .is_err()
                    {
                        return;
                    }
                }
                Ok(None) => {
                    crate::stream::close_owned_writable_stream(&writable);
                    return;
                }
                Err(_) => {
                    crate::stream::fail_owned_writable_stream(&writable);
                    return;
                }
            }
        }
    });
    HttpResponseStreamSource::Readable(readable)
}

fn response_stream_body(
    source: HttpResponseStreamSource,
    encoding: Option<ResponseEncoding>,
) -> impl futures_util::Stream<Item = Result<Bytes, std::io::Error>> {
    let source = match response_stream_encoder(encoding) {
        Some(encoder) => compressed_response_source(source, encoder),
        None => source,
    };
    futures_util::stream::unfold(Some(source), |source| async move {
        let mut source = source?;
        loop {
            match source.next().await {
                Ok(Some(bytes)) if bytes.is_empty() => continue,
                Ok(Some(bytes)) => return Some((Ok(Bytes::from(bytes)), Some(source))),
                Ok(None) => return None,
                Err(error) => return Some((Err(error), None)),
            }
        }
    })
}

fn streaming_response_encoding(
    accept_encoding: Option<&str>,
    content_type: Option<&str>,
    headers: &[(HeaderName, HeaderValue)],
) -> Option<ResponseEncoding> {
    let encoding = preferred_response_encoding(accept_encoding)?;
    if !content_type.is_some_and(is_compressible_content_type)
        || headers
            .iter()
            .any(|(name, _)| *name == axum::http::header::CONTENT_ENCODING)
        || headers.iter().any(|(name, value)| {
            *name == axum::http::header::ETAG
                && value
                    .to_str()
                    .is_ok_and(|etag| valid_entity_tag(etag) && !etag.starts_with("W/"))
        })
    {
        return None;
    }
    Some(encoding)
}

/// Brotli at quality 5 keeps a whole-body response fast to produce; Gzip uses
/// the usual default of 6.
fn compress_body(body: &[u8], encoding: ResponseEncoding) -> std::io::Result<Vec<u8>> {
    let (format, level) = match encoding {
        ResponseEncoding::Brotli => (crate::codec::Format::Brotli, 5),
        ResponseEncoding::Gzip => (crate::codec::Format::Gzip, 6),
    };
    crate::codec::compress(body, format, Some(level))
        .map_err(|error| std::io::Error::other(format!("{error:?}")))
}

fn preferred_response_encoding(accept_encoding: Option<&str>) -> Option<ResponseEncoding> {
    let accept_encoding = accept_encoding?;
    let mut wildcard_quality = None;
    let mut gzip_quality = None;
    let mut brotli_quality = None;
    for entry in accept_encoding.split(',') {
        let mut parts = entry.split(';');
        let encoding = parts.next().unwrap_or_default().trim().to_ascii_lowercase();
        let quality = accept_encoding_quality(parts);
        match encoding.as_str() {
            "gzip" => gzip_quality = Some(quality),
            "br" => brotli_quality = Some(quality),
            "*" => wildcard_quality = Some(quality),
            _ => {}
        }
    }
    let gzip = gzip_quality.or(wildcard_quality).unwrap_or(0.0);
    let brotli = brotli_quality.or(wildcard_quality).unwrap_or(0.0);
    if brotli > 0.0 && brotli >= gzip {
        Some(ResponseEncoding::Brotli)
    } else if gzip > 0.0 {
        Some(ResponseEncoding::Gzip)
    } else {
        None
    }
}

fn accept_encoding_quality<'a>(parts: impl Iterator<Item = &'a str>) -> f32 {
    for part in parts {
        let Some((name, value)) = part.trim().split_once('=') else {
            continue;
        };
        if !name.trim().eq_ignore_ascii_case("q") {
            continue;
        }
        let Ok(quality) = value.trim().parse::<f32>() else {
            return 0.0;
        };
        return if (0.0..=1.0).contains(&quality) {
            quality
        } else {
            0.0
        };
    }
    1.0
}

fn is_compressible_content_type(content_type: &str) -> bool {
    let media_type = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    media_type.starts_with("text/")
        || matches!(
            media_type.as_str(),
            "application/json"
                | "application/problem+json"
                | "application/javascript"
                | "application/xml"
                | "application/xhtml+xml"
                | "application/wasm"
        )
        || media_type.ends_with("+json")
        || media_type.ends_with("+xml")
}

fn add_or_merge_vary_accept_encoding(headers: &mut Vec<(HeaderName, HeaderValue)>) {
    let Some((_, value)) = headers
        .iter_mut()
        .find(|(name, _)| *name == axum::http::header::VARY)
    else {
        headers.push((
            axum::http::header::VARY,
            HeaderValue::from_static("accept-encoding"),
        ));
        return;
    };
    let Ok(existing) = value.to_str() else {
        return;
    };
    if existing
        .split(',')
        .any(|entry| entry.trim().eq_ignore_ascii_case("accept-encoding"))
    {
        return;
    }
    let Ok(merged) = HeaderValue::from_str(&format!("{existing}, accept-encoding")) else {
        return;
    };
    *value = merged;
}

fn conditional_request_matches(
    if_none_match: Option<&str>,
    if_modified_since: Option<&str>,
    response_headers: &[(HeaderName, HeaderValue)],
) -> bool {
    if if_none_match.is_some() {
        return if_none_match_matches(if_none_match, response_headers);
    }
    if_modified_since_matches(if_modified_since, response_headers)
}

fn if_none_match_matches(
    if_none_match: Option<&str>,
    response_headers: &[(HeaderName, HeaderValue)],
) -> bool {
    let Some(if_none_match) = if_none_match else {
        return false;
    };
    let Some(etag) = response_headers
        .iter()
        .find(|(name, _)| *name == axum::http::header::ETAG)
        .and_then(|(_, value)| value.to_str().ok())
    else {
        return false;
    };
    if !valid_entity_tag(etag) {
        return false;
    }
    if if_none_match.trim() == "*" {
        return true;
    }
    if_none_match
        .split(',')
        .map(str::trim)
        .filter(|candidate| valid_entity_tag(candidate))
        .any(|candidate| weak_entity_tag(candidate) == weak_entity_tag(etag))
}

fn valid_entity_tag(value: &str) -> bool {
    let value = value.strip_prefix("W/").unwrap_or(value);
    value.len() >= 2
        && value.starts_with('"')
        && value.ends_with('"')
        && value[1..value.len() - 1]
            .bytes()
            .all(|byte| byte >= 0x21 && byte != b'"' && byte != 0x7f)
}

fn weak_entity_tag(value: &str) -> &str {
    value.strip_prefix("W/").unwrap_or(value)
}

fn if_modified_since_matches(
    if_modified_since: Option<&str>,
    response_headers: &[(HeaderName, HeaderValue)],
) -> bool {
    let Some(if_modified_since) = if_modified_since else {
        return false;
    };
    let Some(last_modified) = response_headers
        .iter()
        .find(|(name, _)| *name == axum::http::header::LAST_MODIFIED)
        .and_then(|(_, value)| value.to_str().ok())
    else {
        return false;
    };
    let (Ok(last_modified), Ok(if_modified_since)) = (
        httpdate::parse_http_date(last_modified),
        httpdate::parse_http_date(if_modified_since),
    ) else {
        return false;
    };
    last_modified <= if_modified_since
}

fn reject_reply(reason: std::fmt::Arguments<'_>) -> i32 {
    eprintln!("error: HTTP reply rejected: {reason}");
    -1
}

fn reply_header_key(response: *const HttpResponse) -> usize {
    response as usize
}

fn reply_headers_lock()
-> std::sync::MutexGuard<'static, HashMap<usize, Vec<(HeaderName, HeaderValue)>>> {
    match REPLY_HEADERS.lock() {
        Ok(headers) => headers,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn take_reply_headers(response: *const HttpResponse) -> Vec<(HeaderName, HeaderValue)> {
    reply_headers_lock()
        .remove(&reply_header_key(response))
        .unwrap_or_default()
}

#[cfg(test)]
pub(crate) fn reply_headers_contains(response: *const HttpResponse, name: &str) -> bool {
    reply_headers_lock()
        .get(&reply_header_key(response))
        .is_some_and(|headers| headers.iter().any(|(header, _)| header.as_str() == name))
}

fn clear_reply_headers(response: *const HttpResponse) {
    let _ = reply_headers_lock().remove(&reply_header_key(response));
}

fn omit_body_for_head(response: Response<Body>, is_head: bool) -> Response<Body> {
    if !is_head {
        return response;
    }
    let (parts, _) = response.into_parts();
    Response::from_parts(parts, Body::empty())
}

fn http_status_forbids_body(status: StatusCode) -> bool {
    status.is_informational()
        || matches!(
            status,
            StatusCode::NO_CONTENT | StatusCode::RESET_CONTENT | StatusCode::NOT_MODIFIED
        )
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_http_router_new() -> *mut HttpRouter {
    Box::into_raw(Box::new(HttpRouter {
        router: Router::new(),
        cors: None,
        response_hooks: Arc::new(Mutex::new(Vec::new())),
        config: Arc::new(std::sync::RwLock::new(HttpServerConfig::default())),
        openapi_fragments: Arc::new(Mutex::new(Vec::new())),
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_router_free(router: *mut HttpRouter) {
    unsafe {
        if !router.is_null() {
            drop(Box::from_raw(router));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_router_on_response(
    router: *mut HttpRouter,
    hook: Option<HttpResponseHook>,
) -> i32 {
    unsafe {
        let (Some(router), Some(hook)) = (router.as_mut(), hook) else {
            set_http_error(
                "ERR_HTTP_RESPONSE_HOOK",
                "router or response hook is invalid",
            );
            return -1;
        };
        let mut hooks = match router.response_hooks.lock() {
            Ok(hooks) => hooks,
            Err(poisoned) => poisoned.into_inner(),
        };
        hooks.push(hook);
        0
    }
}

unsafe fn collect_pre_handlers(
    pre_handlers: *const HttpPreHandler,
    pre_handlers_len: usize,
) -> Vec<HttpPreHandler> {
    unsafe {
        if pre_handlers.is_null() || pre_handlers_len == 0 {
            return Vec::new();
        }
        std::slice::from_raw_parts(pre_handlers, pre_handlers_len).to_vec()
    }
}

const ROUTE_EXPECTS_JSON_BODY: i64 = 1;

const ROUTE_EXPECTS_MULTIPART_BODY: i64 = 2;

const ROUTE_EXPECTS_STREAM_BODY: i64 = 4;

#[allow(clippy::too_many_arguments)]
unsafe fn add_route(
    router: *mut HttpRouter,
    path: TyrusString,
    handler: Option<HttpHandler>,
    body_limit: i64,
    rate_limit_max: i64,
    rate_limit_window_millis: i64,
    rate_limit_global_max: i64,
    rate_limit_ban: i64,
    rate_limit_ban_window_millis: i64,
    route_flags: i64,
    pre_handlers: *const HttpPreHandler,
    pre_handlers_len: usize,
    method: Method,
) -> i32 {
    unsafe {
        let (Some(router), Some(handler), Ok(path)) = (router.as_mut(), handler, text(&path))
        else {
            set_http_error(
                "ERR_HTTP_ROUTE_CONFIG",
                "router, handler, or route path is invalid",
            );
            return -1;
        };
        if !path.starts_with('/') {
            set_http_error("ERR_HTTP_ROUTE_CONFIG", "route path must start with `/`");
            return -1;
        }
        const KNOWN_ROUTE_FLAGS: i64 =
            ROUTE_EXPECTS_JSON_BODY | ROUTE_EXPECTS_MULTIPART_BODY | ROUTE_EXPECTS_STREAM_BODY;
        if route_flags & !KNOWN_ROUTE_FLAGS != 0 {
            set_http_error(
                "ERR_HTTP_ROUTE_CONFIG",
                "route body configuration is invalid",
            );
            return -1;
        }
        let route_params = std::sync::Arc::new(RouteParams::parse(&path));
        let rate_limiter = (rate_limit_max >= 0 && rate_limit_window_millis >= 0).then(|| {
            RateLimiter::new_with_ban(
                rate_limit_max,
                rate_limit_window_millis,
                rate_limit_global_max,
                rate_limit_ban,
                rate_limit_ban_window_millis,
            )
        });
        let pre_handlers = collect_pre_handlers(pre_handlers, pre_handlers_len);
        let config = Arc::clone(&router.config);
        let query_route_params = route_params.clone();
        let query_rate_limiter = rate_limiter.clone();
        let query_pre_handlers = pre_handlers.clone();
        let query_config = Arc::clone(&config);
        let service = move |request| {
            invoke_with_timeout(
                handler,
                route_params.clone(),
                body_limit,
                route_flags,
                rate_limiter.clone(),
                pre_handlers.clone(),
                Arc::clone(&config),
                request,
            )
        };
        let current = std::mem::take(&mut router.router);
        let axum_path = axum_route_path(&path);
        router.router = if is_query_method(&method) {
            current.route(
                &axum_path,
                MethodRouter::new().fallback(move |request: Request| {
                    let route_params = query_route_params.clone();
                    let rate_limiter = query_rate_limiter.clone();
                    let pre_handlers = query_pre_handlers.clone();
                    let config = Arc::clone(&query_config);
                    async move {
                        if is_query_method(request.method()) {
                            invoke_with_timeout(
                                handler,
                                route_params,
                                body_limit,
                                route_flags,
                                rate_limiter,
                                pre_handlers,
                                config,
                                request,
                            )
                            .await
                        } else {
                            Response::builder()
                                .status(StatusCode::METHOD_NOT_ALLOWED)
                                .body(Body::empty())
                                .expect("valid empty method-not-allowed response")
                        }
                    }
                }),
            )
        } else {
            match method {
                Method::GET => current.route(&axum_path, get(service)),
                Method::POST => current.route(&axum_path, post(service)),
                Method::PUT => current.route(&axum_path, put(service)),
                Method::PATCH => current.route(&axum_path, patch(service)),
                Method::DELETE => current.route(&axum_path, delete(service)),
                Method::HEAD => current.route(&axum_path, head(service)),
                Method::OPTIONS => current.route(&axum_path, options(service)),
                Method::CONNECT => current.route(&axum_path, connect(service)),
                Method::TRACE => current.route(&axum_path, trace(service)),
                _ => {
                    set_http_error("ERR_HTTP_ROUTE_CONFIG", "route method is unsupported");
                    return -1;
                }
            }
        };
        0
    }
}

macro_rules! route_function {
    ($name:ident, $method:expr_2021) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            router: *mut HttpRouter,
            path: TyrusString,
            handler: Option<HttpHandler>,
            body_limit: i64,
            rate_limit_max: i64,
            rate_limit_window_millis: i64,
            rate_limit_global_max: i64,
            rate_limit_ban: i64,
            rate_limit_ban_window_millis: i64,
            route_flags: i64,
            pre_handlers: *const HttpPreHandler,
            pre_handlers_len: usize,
        ) -> i32 {
            let status = unsafe {
                add_route(
                    router,
                    path,
                    handler,
                    body_limit,
                    rate_limit_max,
                    rate_limit_window_millis,
                    rate_limit_global_max,
                    rate_limit_ban,
                    rate_limit_ban_window_millis,
                    route_flags,
                    pre_handlers,
                    pre_handlers_len,
                    $method,
                )
            };
            if status < 0 {
                set_http_error("ERR_HTTP_ROUTE_CONFIG", "route registration was rejected");
            }
            status
        }
    };
}

route_function!(tyrus_http_router_get, Method::GET);
route_function!(tyrus_http_router_post, Method::POST);
route_function!(tyrus_http_router_put, Method::PUT);
route_function!(tyrus_http_router_patch, Method::PATCH);
route_function!(tyrus_http_router_delete, Method::DELETE);
route_function!(tyrus_http_router_head, Method::HEAD);
route_function!(tyrus_http_router_options, Method::OPTIONS);
route_function!(tyrus_http_router_connect, Method::CONNECT);
route_function!(tyrus_http_router_query, query_method());
route_function!(tyrus_http_router_trace, Method::TRACE);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_router_rate_limit(
    router: *mut HttpRouter,
    max: i64,
    window_millis: i64,
    global_max: i64,
    ban: i64,
    ban_window_millis: i64,
) -> i32 {
    unsafe {
        let Some(router) = router.as_mut() else {
            set_http_error("ERR_HTTP_RATE_LIMIT_CONFIG", "router is unavailable");
            return -1;
        };
        if max <= 0
            || window_millis <= 0
            || global_max == 0
            || global_max < -1
            || ban < -1
            || (ban >= 0 && ban_window_millis != -1 && ban_window_millis <= 0)
        {
            set_http_error(
                "ERR_HTTP_RATE_LIMIT_CONFIG",
                "rate limit configuration is invalid",
            );
            return -1;
        }
        let mut config = match router.config.write() {
            Ok(config) => config,
            Err(poisoned) => poisoned.into_inner(),
        };
        config.rate_limit = Some(RateLimiter::new_with_ban(
            max,
            window_millis,
            global_max,
            ban,
            ban_window_millis,
        ));
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_router_trusted_proxies(
    router: *mut HttpRouter,
    proxies: *const TyrusStringArray,
    identity_header: *const TyrusString,
) -> i32 {
    unsafe {
        let (Some(router), Some(proxies)) = (router.as_mut(), proxies.as_ref()) else {
            set_http_error(
                "ERR_HTTP_TRUSTED_PROXIES",
                "router or proxy list is invalid",
            );
            return -1;
        };
        let length = proxies.length;
        if length > proxies.capacity || (length > 0 && proxies.data.is_null()) {
            set_http_error(
                "ERR_HTTP_TRUSTED_PROXIES",
                "proxy list has an invalid layout",
            );
            return -1;
        }
        let values = if length == 0 {
            &[][..]
        } else {
            std::slice::from_raw_parts(proxies.data, length)
        };
        let mut parsed = Vec::with_capacity(values.len());
        for value in values {
            let Ok(value) = text(value) else {
                set_http_error(
                    "ERR_HTTP_TRUSTED_PROXIES",
                    "proxy list contains invalid text",
                );
                return -1;
            };
            let Some(proxy) = TrustedProxy::parse(&value) else {
                set_http_error("ERR_HTTP_TRUSTED_PROXIES", "proxy entry is invalid");
                return -1;
            };
            parsed.push(proxy);
        }
        let identity_header = match identity_header.as_ref() {
            None => None,
            Some(value) if value.is_null() => None,
            Some(value) => {
                let Ok(value) = text(value) else {
                    set_http_error(
                        "ERR_HTTP_TRUSTED_PROXIES",
                        "identity header is invalid text",
                    );
                    return -1;
                };
                if axum::http::header::HeaderName::from_bytes(value.as_bytes()).is_err() {
                    set_http_error(
                        "ERR_HTTP_TRUSTED_PROXIES",
                        "identity header name is invalid",
                    );
                    return -1;
                }
                Some(value.to_ascii_lowercase())
            }
        };
        match router.config.write() {
            Ok(mut config) => {
                config.trusted_proxies = parsed;
                config.trusted_proxy_identity_header = identity_header;
            }
            Err(poisoned) => {
                let mut config = poisoned.into_inner();
                config.trusted_proxies = parsed;
                config.trusted_proxy_identity_header = identity_header;
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_router_csrf(
    router: *mut HttpRouter,
    options: *const CsrfOptions,
) -> i32 {
    unsafe {
        let (Some(router), Some(options)) = (router.as_mut(), options.as_ref()) else {
            set_http_error("ERR_HTTP_CSRF_CONFIG", "router or CSRF options are invalid");
            return -1;
        };
        let (Ok(cookie), Ok(header)) = (text(&options.cookie), text(&options.header)) else {
            set_http_error(
                "ERR_HTTP_CSRF_CONFIG",
                "CSRF cookie or header is invalid text",
            );
            return -1;
        };
        let Ok(header) = axum::http::HeaderName::from_bytes(header.as_bytes()) else {
            set_http_error("ERR_HTTP_CSRF_CONFIG", "CSRF header name is invalid");
            return -1;
        };
        if !valid_cookie_name(&cookie) {
            set_http_error("ERR_HTTP_CSRF_CONFIG", "CSRF cookie name is invalid");
            return -1;
        }
        match router.config.write() {
            Ok(mut config) => config.csrf = Some(CsrfConfig { cookie, header }),
            Err(poisoned) => poisoned.into_inner().csrf = Some(CsrfConfig { cookie, header }),
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_router_fallback(
    router: *mut HttpRouter,
    handler: Option<HttpHandler>,
    pre_handlers: *const HttpPreHandler,
    pre_handlers_len: usize,
) -> i32 {
    unsafe {
        let (Some(router), Some(handler)) = (router.as_mut(), handler) else {
            set_http_error(
                "ERR_HTTP_ROUTE_CONFIG",
                "router or fallback handler is invalid",
            );
            return -1;
        };
        let current = std::mem::take(&mut router.router);
        let pre_handlers = collect_pre_handlers(pre_handlers, pre_handlers_len);
        let config = Arc::clone(&router.config);
        router.router = current.fallback(move |request| {
            invoke_with_timeout(
                handler,
                std::sync::Arc::new(RouteParams::default()),
                -1,
                0,
                None,
                pre_handlers.clone(),
                Arc::clone(&config),
                request,
            )
        });
        0
    }
}

pub struct TyrusHttpServer {
    shutdown: tokio::sync::watch::Sender<bool>,
    stopped: tokio::sync::watch::Sender<bool>,
    _task: tokio::task::JoinHandle<()>,
    hard_stop: tokio::sync::watch::Sender<bool>,
    shutdown_timeout: Option<Duration>,
    in_flight: Arc<crate::types::InFlight>,
    websocket_going_away: Arc<tokio::sync::watch::Sender<bool>>,
    port: u16,
}

unsafe fn bind_http_server(
    router: &HttpRouter,
    port: i32,
    options: *const HttpServerOptions,
) -> Option<(std::net::TcpListener, SocketAddr)> {
    let Ok(port) = u16::try_from(port) else {
        set_http_error("ERR_HTTP_LISTEN", "port must be between 0 and 65535");
        return None;
    };
    let host = match unsafe { server_host(options) } {
        Ok(host) => host,
        Err(_) => {
            set_http_error(
                "ERR_HTTP_LISTEN",
                "host is not a valid address; use an address like `127.0.0.1` or `::1`",
            );
            return None;
        }
    };
    if unsafe { apply_server_options(router, options) } != 0 {
        return None;
    }
    let address = SocketAddr::new(host, port);
    let listener = match std::net::TcpListener::bind(address) {
        Ok(listener) => listener,
        Err(error) => {
            set_http_error("ERR_HTTP_LISTEN", bind_failure_message(error.kind()));
            return None;
        }
    };
    if listener.set_nonblocking(true).is_err() {
        set_http_error("ERR_HTTP_LISTEN", "could not prepare the bound socket");
        return None;
    }
    let address = listener.local_addr().unwrap_or(address);
    Some((listener, address))
}

pub(crate) const DEFAULT_HEADER_READ_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) const MIN_MAX_HEADER_BYTES: usize = 8 * 1024;

pub(crate) const DEFAULT_MAX_HEADER_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub(crate) struct ConnectionSettings {
    header_read_timeout: Duration,
    max_header_bytes: usize,
    keep_alive: bool,
    logger: bool,
    tls: Option<std::sync::Arc<tokio_rustls::TlsAcceptor>>,
}

struct HttpConnectionTask {
    response_hooks: Arc<[HttpResponseHook]>,
    settings: ConnectionSettings,
    in_flight: Arc<crate::types::InFlight>,
    permit: Option<tokio::sync::OwnedSemaphorePermit>,
    hard_stop: tokio::sync::watch::Receiver<bool>,
}

async fn serve_http_connection(
    app: Router,
    stream: tokio::net::TcpStream,
    peer: SocketAddr,
    task: HttpConnectionTask,
) {
    let HttpConnectionTask {
        response_hooks,
        settings,
        in_flight,
        permit,
        mut hard_stop,
    } = task;
    let _permit = permit;
    let connection =
        serve_one_http_connection(app, response_hooks, stream, peer, settings, in_flight);
    tokio::select! {
        () = connection => {}
        _ = hard_stop.changed() => {}
    }
}

async fn serve_one_http_connection(
    app: Router,
    response_hooks: Arc<[HttpResponseHook]>,
    stream: tokio::net::TcpStream,
    peer: SocketAddr,
    settings: ConnectionSettings,
    in_flight: Arc<crate::types::InFlight>,
) {
    match settings.tls.clone() {
        Some(acceptor) => {
            let handshake =
                tokio::time::timeout(settings.header_read_timeout, acceptor.accept(stream));
            let Ok(Ok(stream)) = handshake.await else {
                return;
            };
            serve_http_io(app, response_hooks, stream, peer, settings, in_flight).await;
        }
        None => serve_http_io(app, response_hooks, stream, peer, settings, in_flight).await,
    }
}

async fn serve_http_io<I>(
    app: Router,
    response_hooks: Arc<[HttpResponseHook]>,
    io: I,
    peer: SocketAddr,
    settings: ConnectionSettings,
    in_flight: Arc<crate::types::InFlight>,
) where
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let logger = settings.logger;
    let service = hyper::service::service_fn(move |request: Request<hyper::body::Incoming>| {
        let counted = Arc::clone(&in_flight).enter();
        let request_id = next_request_id();
        let observe = should_observe_response(logger, &response_hooks);
        let observation = observe.then(|| {
            (
                StdInstant::now(),
                request.method().clone(),
                request.uri().path().to_string(),
                Arc::clone(&response_hooks),
            )
        });
        let mut request = request.map(Body::new);
        request
            .extensions_mut()
            .insert(axum::extract::ConnectInfo(peer));
        request.extensions_mut().insert(request_id.clone());
        let app = app.clone();
        async move {
            let response = tower::Service::call(&mut app.clone(), request).await?;
            let (parts, body) = response.into_parts();
            let response = Response::from_parts(
                parts,
                Body::new(InFlightBody {
                    body,
                    _guard: counted,
                }),
            );
            let response = match observation {
                Some((started, method, path, hooks)) => observe_response(
                    response,
                    &request_id,
                    &method,
                    &path,
                    started,
                    logger,
                    hooks,
                ),
                None => attach_request_id(response, &request_id),
            };
            Ok::<_, std::convert::Infallible>(response)
        }
    });
    let mut builder =
        hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
    builder
        .http1()
        .timer(hyper_util::rt::TokioTimer::new())
        .header_read_timeout(settings.header_read_timeout)
        .max_buf_size(settings.max_header_bytes)
        .keep_alive(settings.keep_alive);
    builder.http2().timer(hyper_util::rt::TokioTimer::new());
    let _ = builder
        .serve_connection_with_upgrades(hyper_util::rt::TokioIo::new(io), service)
        .await;
}

unsafe fn spawn_http_server(
    router: &HttpRouter,
    listener: std::net::TcpListener,
    address: SocketAddr,
    shutdown_timeout: Option<Duration>,
    on_stop: Option<std::sync::mpsc::Sender<()>>,
) -> Arc<TyrusHttpServer> {
    let app = match &router.cors {
        Some(cors) => router.router.clone().layer(cors.clone()),
        None => router.router.clone(),
    };
    let (in_flight, settings, max_connections) = {
        let config = match router.config.read() {
            Ok(config) => config,
            Err(poisoned) => poisoned.into_inner(),
        };
        (
            Arc::clone(&config.in_flight),
            ConnectionSettings {
                header_read_timeout: config.header_read_timeout,
                max_header_bytes: config.max_header_bytes,
                keep_alive: config.keep_alive,
                logger: config.logger,
                tls: config.tls.clone(),
            },
            config.max_connections.clone(),
        )
    };
    let websocket_going_away = {
        let config = match router.config.read() {
            Ok(config) => config,
            Err(poisoned) => poisoned.into_inner(),
        };
        Arc::clone(&config.websocket_going_away)
    };
    websocket_going_away.send_replace(false);
    let response_hooks: Arc<[HttpResponseHook]> = {
        let hooks = match router.response_hooks.lock() {
            Ok(hooks) => hooks,
            Err(poisoned) => poisoned.into_inner(),
        };
        Arc::from(hooks.clone())
    };
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let (stopped_tx, _) = tokio::sync::watch::channel(false);
    let (hard_stop_tx, hard_stop_rx) = tokio::sync::watch::channel(false);
    let stopped_for_task = stopped_tx.clone();
    let server_in_flight = Arc::clone(&in_flight);
    let task = executor().spawn(async move {
        let listener = match tokio::net::TcpListener::from_std(listener) {
            Ok(listener) => listener,
            Err(_) => {
                stopped_for_task.send_replace(true);
                if let Some(sender) = &on_stop {
                    let _ = sender.send(());
                }
                return;
            }
        };
        let mut shutdown_rx = shutdown_rx;
        loop {
            let permit = match &max_connections {
                Some(limit) => {
                    let waiting = std::sync::Arc::clone(limit).acquire_owned();
                    let permit = tokio::select! {
                        biased;
                        _ = shutdown_rx.changed() => break,
                        permit = waiting => permit,
                    };
                    let Ok(permit) = permit else {
                        break;
                    };
                    Some(permit)
                }
                None => None,
            };
            let accepted = tokio::select! {
                biased;
                _ = shutdown_rx.changed() => break,
                accepted = listener.accept() => accepted,
            };
            let (stream, peer) = match accepted {
                Ok(accepted) => accepted,
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::ConnectionAborted
                            | std::io::ErrorKind::ConnectionReset
                            | std::io::ErrorKind::Interrupted
                    ) =>
                {
                    continue;
                }
                Err(_) => {
                    set_http_error(
                        "ERR_HTTP_CONNECTION",
                        "the server stopped accepting connections",
                    );
                    break;
                }
            };
            executor().spawn(serve_http_connection(
                app.clone(),
                stream,
                peer,
                HttpConnectionTask {
                    response_hooks: Arc::clone(&response_hooks),
                    settings: settings.clone(),
                    in_flight: Arc::clone(&in_flight),
                    permit,
                    hard_stop: hard_stop_rx.clone(),
                },
            ));
        }
        drop(listener);
        stopped_for_task.send_replace(true);
        if let Some(sender) = &on_stop {
            let _ = sender.send(());
        }
    });
    let server = Arc::new(TyrusHttpServer {
        shutdown: shutdown_tx,
        stopped: stopped_tx,
        _task: task,
        hard_stop: hard_stop_tx,
        shutdown_timeout,
        in_flight: server_in_flight,
        websocket_going_away,
        port: address.port(),
    });
    register_for_shutdown_signals(&server);
    server
}

unsafe fn server_shutdown_timeout(options: *const HttpServerOptions) -> Option<Duration> {
    let options = unsafe { options.as_ref() }?;
    match options.shutdown_timeout_millis {
        millis if millis > 0 => Some(Duration::from_millis(millis as u64)),
        _ => None,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_server_port(server: *const TyrusHttpServer) -> i32 {
    let Some(server) = (unsafe { server.as_ref() }) else {
        return -1;
    };
    i32::from(server.port)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_server_retain(
    server: *const TyrusHttpServer,
) -> *const TyrusHttpServer {
    if !server.is_null() {
        unsafe { Arc::increment_strong_count(server) };
    }
    server
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_server_free(server: *const TyrusHttpServer) {
    if !server.is_null() {
        drop(unsafe { Arc::from_raw(server) });
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CloseOutcome {
    Drained,
    Cut,
}

fn begin_close(
    handle: &TyrusHttpServer,
) -> impl std::future::Future<Output = CloseOutcome> + Send + 'static + use<> {
    handle.shutdown.send_replace(true);
    handle.websocket_going_away.send_replace(true);
    let mut stopped = handle.stopped.subscribe();
    let deadline = handle.shutdown_timeout;
    let hard_stop = handle.hard_stop.clone();
    let in_flight = Arc::clone(&handle.in_flight);
    async move {
        let wait = async {
            while !*stopped.borrow_and_update() {
                if stopped.changed().await.is_err() {
                    break;
                }
            }
            in_flight.wait_idle().await;
        };
        match deadline {
            None => {
                wait.await;
                CloseOutcome::Drained
            }
            Some(deadline) => match tokio::time::timeout(deadline, wait).await {
                Ok(()) => CloseOutcome::Drained,
                Err(_) => {
                    let _ = hard_stop.send(true);
                    CloseOutcome::Cut
                }
            },
        }
    }
}

static RUNNING_SERVERS: LazyLock<Mutex<Vec<Arc<TyrusHttpServer>>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

fn register_for_shutdown_signals(server: &Arc<TyrusHttpServer>) {
    {
        let mut servers = match RUNNING_SERVERS.lock() {
            Ok(servers) => servers,
            Err(poisoned) => poisoned.into_inner(),
        };
        servers.retain(|server| !*server.stopped.borrow());
        servers.push(Arc::clone(server));
    }
    static WATCHER: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    WATCHER.get_or_init(|| {
        executor().spawn(watch_shutdown_signals());
    });
}

fn exit_now(code: i32) -> ! {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    std::process::exit(code)
}

pub(crate) fn wait_for_servers() {
    loop {
        if SIGNAL_SHUTDOWN_STARTED.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(20));
            continue;
        }
        let running = {
            let mut servers = match RUNNING_SERVERS.lock() {
                Ok(servers) => servers,
                Err(poisoned) => poisoned.into_inner(),
            };
            servers.retain(|server| !*server.stopped.borrow());
            !servers.is_empty()
        };
        if !running {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

async fn shut_down_on_signal<F>(name: &str, second: F)
where
    F: std::future::Future<Output = i32>,
{
    let servers: Vec<Arc<TyrusHttpServer>> = {
        let servers = match RUNNING_SERVERS.lock() {
            Ok(servers) => servers,
            Err(poisoned) => poisoned.into_inner(),
        };
        servers.clone()
    };
    if servers.is_empty() {
        exit_now(if name == "SIGTERM" { 143 } else { 130 });
    }
    eprintln!("Received {name}, shutting down");
    let closing: Vec<_> = servers.iter().map(|server| begin_close(server)).collect();
    drop(servers);
    tokio::select! {
        outcomes = futures_util::future::join_all(closing) => {
            let cut = outcomes.contains(&CloseOutcome::Cut);
            exit_now(if cut { 1 } else { 0 });
        }
        code = second => exit_now(code),
    }
}

#[cfg(unix)]
async fn watch_shutdown_signals() {
    use tokio::signal::unix::{SignalKind, signal};
    let (Ok(mut interrupt), Ok(mut terminate)) = (
        signal(SignalKind::interrupt()),
        signal(SignalKind::terminate()),
    ) else {
        return;
    };
    let name = tokio::select! {
        _ = interrupt.recv() => "SIGINT",
        _ = terminate.recv() => "SIGTERM",
    };
    SIGNAL_SHUTDOWN_STARTED.store(true, Ordering::Release);
    let second = async move {
        tokio::select! {
            _ = interrupt.recv() => 130,
            _ = terminate.recv() => 143,
        }
    };
    shut_down_on_signal(name, second).await;
}

#[cfg(windows)]
async fn watch_shutdown_signals() {
    let Ok(mut break_signal) = tokio::signal::windows::ctrl_break() else {
        return;
    };
    let name = tokio::select! {
        _ = tokio::signal::ctrl_c() => "Ctrl+C",
        _ = break_signal.recv() => "Ctrl+Break",
    };
    SIGNAL_SHUTDOWN_STARTED.store(true, Ordering::Release);
    let second = async move {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => 130,
            _ = break_signal.recv() => 130,
        }
    };
    shut_down_on_signal(name, second).await;
}

#[cfg(not(any(unix, windows)))]
async fn watch_shutdown_signals() {
    if tokio::signal::ctrl_c().await.is_err() {
        return;
    }
    SIGNAL_SHUTDOWN_STARTED.store(true, Ordering::Release);
    let second = async {
        let _ = tokio::signal::ctrl_c().await;
        130
    };
    shut_down_on_signal("Ctrl+C", second).await;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_server_close(
    server: *const TyrusHttpServer,
) -> *mut TyrusPromise {
    let promise = crate::tyrus_promise_pending();
    unsafe { crate::tyrus_promise_retain(promise) };
    let owner = crate::executor::PromiseTaskOwner {
        promise: promise as usize,
    };
    let Some(handle) = (unsafe { server.as_ref() }) else {
        unsafe { owner.fulfill_void() };
        return promise;
    };
    let closing = begin_close(handle);
    executor().spawn(async move {
        match closing.await {
            CloseOutcome::Drained => unsafe { owner.fulfill_void() },
            CloseOutcome::Cut => unsafe {
                owner.reject_structured(
                    "ERR_HTTP_SHUTDOWN_TIMEOUT",
                    "the shutdown deadline expired and the requests still running were ended",
                );
            },
        }
    });
    promise
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_router_listen(
    router: *mut HttpRouter,
    port: i32,
    options: *const HttpServerOptions,
) -> *const TyrusHttpServer {
    LAST_HTTP_ERROR.with(|slot| slot.set(None));
    let Some(router) = (unsafe { router.as_ref() }) else {
        set_http_error("ERR_HTTP_LISTEN", "the router was not created");
        return std::ptr::null();
    };
    let Some((listener, address)) = (unsafe { bind_http_server(router, port, options) }) else {
        return std::ptr::null();
    };
    let shutdown_timeout = unsafe { server_shutdown_timeout(options) };
    let server = unsafe { spawn_http_server(router, listener, address, shutdown_timeout, None) };
    Arc::into_raw(server)
}

/// Allocates the optional `content_type` box the response ABI expects.
///
/// The field is freed with `free`, because generated code builds it with
/// `malloc`. Allocating it here with `Box` instead would pair Rust's allocator
/// with libc's: it happens to work while the global allocator is the system
/// one, and is undefined behaviour the moment it is not.
unsafe fn alloc_content_type(value: &'static str) -> *const TyrusString {
    unsafe {
        let storage = libc::malloc(std::mem::size_of::<TyrusString>()).cast::<TyrusString>();
        if storage.is_null() {
            return std::ptr::null();
        }
        storage.write(TyrusString::literal(value));
        storage
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_set_response_error(
    response: *mut HttpResponse,
    status: i32,
    message: TyrusString,
) {
    unsafe {
        let Some(response) = response.as_mut() else {
            return;
        };
        let message = text(&message).unwrap_or_else(|()| "response validation failed".to_string());
        let issues = crate::validation::take_validation_issues();
        if issues.is_empty() {
            eprintln!("error: HTTP response does not match its declared schema: {message}");
        } else {
            eprintln!("error: HTTP response does not match its declared schema: {message}");
            for (field, validator, detail) in &issues {
                eprintln!("  field `{field}` failed `{validator}`: {detail}");
            }
        }
        let body = http_error_json(status, "internal_server_error", "Internal server error");
        response.status = status;
        response.body = HttpBody::empty_text();
        {
            *response.body.payload.as_mut_ptr().cast::<TyrusString>() =
                crate::strings::alloc_string(&body);
        }
        response.content_type = alloc_content_type("application/json; charset=utf-8");
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_set_validation_error(
    response: *mut HttpResponse,
    status: i32,
    message: TyrusString,
) {
    unsafe {
        let Some(response) = response.as_mut() else {
            return;
        };
        let message = text(&message).unwrap_or_else(|()| "validation failed".to_string());
        let issues = crate::validation::take_validation_issues()
            .into_iter()
            .map(|(field, validator, message)| {
                serde_json::json!({
                    "field": field,
                    "validator": validator,
                    "message": message,
                })
            })
            .collect::<Vec<_>>();
        let body = serde_json::json!({
            "code": "validation",
            "issues": issues,
            "message": message,
            "statusCode": status,
        })
        .to_string();
        response.status = status;
        response.body = HttpBody::empty_text();
        {
            *response.body.payload.as_mut_ptr().cast::<TyrusString>() =
                crate::strings::alloc_string(&body);
        }
        response.content_type = alloc_content_type("application/json; charset=utf-8");
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_reply_set_cookie(
    response: *mut HttpResponse,
    name: TyrusString,
    value: TyrusString,
) -> i32 {
    unsafe {
        let (Some(response), Ok(name), Ok(value)) = (response.as_mut(), text(&name), text(&value))
        else {
            return reject_reply(format_args!(
                "`setCookie` received an unreadable name or value"
            ));
        };
        if !valid_cookie_name(&name) {
            return reject_reply(format_args!("`{name}` is not a valid cookie name"));
        }
        if invalid_cookie_value(&value) {
            return reject_reply(format_args!(
                "the value of cookie `{name}` contains a character a cookie cannot carry"
            ));
        }
        append_cookie(response, format!("{name}={value}"));
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_reply_set_cookie_with_options(
    response: *mut HttpResponse,
    name: TyrusString,
    value: TyrusString,
    options: *const CookieOptions,
) -> i32 {
    unsafe {
        let (Some(response), Ok(name), Ok(value)) = (response.as_mut(), text(&name), text(&value))
        else {
            return reject_reply(format_args!(
                "`setCookie` received an unreadable name or value"
            ));
        };
        if !valid_cookie_name(&name) {
            return reject_reply(format_args!("`{name}` is not a valid cookie name"));
        }
        if invalid_cookie_value(&value) {
            return reject_reply(format_args!(
                "the value of cookie `{name}` contains a character a cookie cannot carry"
            ));
        }
        let Ok(cookie) = format_cookie_with_options(&name, &value, options) else {
            return reject_reply(format_args!(
                "the options of cookie `{name}` are not a valid combination; \
             `sameSite: \"none\"` requires `secure: true`, `maxAge` cannot be negative, \
             and `domain` and `path` must be well formed"
            ));
        };
        append_cookie(response, cookie);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_reply_clear_cookie(
    response: *mut HttpResponse,
    name: TyrusString,
) -> i32 {
    unsafe {
        let (Some(response), Ok(name)) = (response.as_mut(), text(&name)) else {
            return reject_reply(format_args!("`clearCookie` received an unreadable name"));
        };
        if !valid_cookie_name(&name) {
            return reject_reply(format_args!("`{name}` is not a valid cookie name"));
        }
        append_cookie(response, expired_cookie(&name));
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_reply_set_cookies(
    response: *mut HttpResponse,
    cookies: *const TyrusArray,
) -> i32 {
    unsafe {
        let (Some(response), Some(cookies)) = (response.as_mut(), cookies.as_ref()) else {
            return reject_reply(format_args!(
                "`setCookies` received an unreadable list of cookies"
            ));
        };
        if cookies.length == 0 {
            return 0;
        }
        if cookies.data.is_null() {
            return reject_reply(format_args!(
                "`setCookies` received an unreadable list of cookies"
            ));
        }
        let entries = std::slice::from_raw_parts(
            cookies.data.cast::<crate::types::HttpCookie>(),
            cookies.length,
        );
        let mut lines = String::new();
        for entry in entries {
            let (Ok(name), Ok(value)) = (text(&entry.name), text(&entry.value)) else {
                return reject_reply(format_args!(
                    "`setCookies` received an unreadable name or value"
                ));
            };
            if !valid_cookie_name(&name) {
                return reject_reply(format_args!("`{name}` is not a valid cookie name"));
            }
            if invalid_cookie_value(&value) {
                return reject_reply(format_args!(
                    "the value of cookie `{name}` contains a character a cookie cannot carry"
                ));
            }
            let Ok(cookie) = format_cookie_with_options(&name, &value, entry.options) else {
                return reject_reply(format_args!(
                    "the options of cookie `{name}` are not a valid combination; \
                 `sameSite: \"none\"` requires `secure: true`, `maxAge` cannot be negative, \
                 and `domain` and `path` must be well formed"
                ));
            };
            if !lines.is_empty() {
                lines.push('\n');
            }
            lines.push_str(&cookie);
        }
        append_cookie(response, lines);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_reply_clear_cookies(response: *mut HttpResponse) -> i32 {
    unsafe {
        let key = reply_header_key(response);
        let Some(response) = response.as_mut() else {
            return reject_reply(format_args!("`clearCookies` received an unreadable reply"));
        };
        let Some(names) = request_cookie_names_lock().get(&key).cloned() else {
            return 0;
        };
        let mut lines = String::new();
        for name in names.iter() {
            if !lines.is_empty() {
                lines.push('\n');
            }
            lines.push_str(&expired_cookie(name));
        }
        append_cookie(response, lines);
        0
    }
}

fn expired_cookie(name: &str) -> String {
    format!("{name}=; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_reply_header(
    response: *mut HttpResponse,
    name: TyrusString,
    value: TyrusString,
) -> i32 {
    unsafe { set_reply_header(response, name, value, false) }
}

unsafe fn set_reply_etag(response: *mut HttpResponse, value: &TyrusString, weak: bool) -> i32 {
    unsafe {
        let (Some(response), Ok(value)) = (response.as_mut(), text(value)) else {
            return reject_reply(format_args!("the cache options carry an unreadable ETag"));
        };
        if value
            .bytes()
            .any(|byte| byte < 0x21 || byte == b'"' || byte == 0x7f)
        {
            return reject_reply(format_args!(
                "`{value}` is not a valid ETag; the runtime supplies the quotes, so the value \
             itself cannot contain a quote, a space or a control character"
            ));
        }
        let value = if weak {
            format!("W/\"{value}\"")
        } else {
            format!("\"{value}\"")
        };
        let Ok(value) = HeaderValue::from_str(&value) else {
            return reject_reply(format_args!("`{value}` is not a valid ETag header value"));
        };
        let mut headers = reply_headers_lock();
        let entries = headers.entry(reply_header_key(response)).or_default();
        entries.retain(|(name, _)| *name != axum::http::header::ETAG);
        entries.push((axum::http::header::ETAG, value));
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_reply_apply_options(
    response: *mut HttpResponse,
    options: *const HttpReplyOptions,
) -> i32 {
    unsafe {
        let Some(options) = options.as_ref() else {
            return 0;
        };
        let Some(cache) = options.cache.as_ref() else {
            return 0;
        };
        let weak = cache.weak.as_ref().is_some_and(|weak| *weak);
        if set_reply_etag(response, &cache.etag, weak) != 0 {
            return -1;
        }
        if let Some(last_modified) = cache.last_modified.as_ref()
            && set_reply_last_modified(response, *last_modified) != 0
        {
            return -1;
        }
        if let Some(control) = cache.control.as_ref()
            && set_reply_cache_control(response, control) != 0
        {
            return -1;
        }
        0
    }
}

unsafe fn set_reply_last_modified(response: *mut HttpResponse, value: Instant) -> i32 {
    unsafe {
        let timestamp = if value.seconds >= 0 {
            UNIX_EPOCH.checked_add(Duration::from_secs(value.seconds as u64))
        } else {
            UNIX_EPOCH.checked_sub(Duration::from_secs(value.seconds.unsigned_abs()))
        };
        let Some(timestamp) = timestamp else {
            return reject_reply(format_args!(
                "`lastModified` is outside the range an HTTP date can express"
            ));
        };
        let Ok(header_value) = HeaderValue::from_str(&httpdate::fmt_http_date(timestamp)) else {
            return reject_reply(format_args!("`lastModified` is not a valid HTTP date"));
        };
        let Some(response) = response.as_mut() else {
            return reject_reply(format_args!("`lastModified` has no response to apply to"));
        };
        let mut headers = reply_headers_lock();
        let entries = headers.entry(reply_header_key(response)).or_default();
        entries.retain(|(name, _)| *name != axum::http::header::LAST_MODIFIED);
        entries.push((axum::http::header::LAST_MODIFIED, header_value));
        0
    }
}

unsafe fn set_reply_cache_control(response: *mut HttpResponse, value: &TyrusString) -> i32 {
    unsafe {
        let (Some(response), Ok(value)) = (response.as_mut(), text(value)) else {
            return reject_reply(format_args!(
                "the cache options carry an unreadable `control` value"
            ));
        };
        let Ok(value) = HeaderValue::from_str(&value) else {
            return reject_reply(format_args!(
                "`{value}` is not a valid `Cache-Control` header value"
            ));
        };
        let mut headers = reply_headers_lock();
        let entries = headers.entry(reply_header_key(response)).or_default();
        entries.retain(|(name, _)| *name != axum::http::header::CACHE_CONTROL);
        entries.push((axum::http::header::CACHE_CONTROL, value));
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_reply_append_header(
    response: *mut HttpResponse,
    name: TyrusString,
    value: TyrusString,
) -> i32 {
    unsafe { set_reply_header(response, name, value, true) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_reply_remove_header(
    response: *mut HttpResponse,
    name: TyrusString,
) -> i32 {
    unsafe {
        let (Some(response), Ok(name)) = (response.as_mut(), text(&name)) else {
            return reject_reply(format_args!("`removeHeader` received an unreadable name"));
        };
        let Ok(name) = HeaderName::from_bytes(name.as_bytes()) else {
            return reject_reply(format_args!("`{name}` is not a valid header name"));
        };
        if protected_reply_header(&name) {
            return reject_reply(format_args!(
                "`{name}` is set by the runtime and cannot be removed by a handler"
            ));
        }
        let mut headers = reply_headers_lock();
        if let Some(entries) = headers.get_mut(&reply_header_key(response)) {
            entries.retain(|(entry_name, _)| *entry_name != name);
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_reply_type(
    response: *mut HttpResponse,
    value: TyrusString,
) -> i32 {
    unsafe { set_reply_header(response, TyrusString::literal("content-type"), value, false) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_request_body_bytes(
    data: *const u8,
    length: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        if data.is_null() || length == 0 {
            *output = TyrusArray {
                data: std::ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            return;
        }
        let allocation = libc::malloc(length).cast::<u8>();
        if allocation.is_null() {
            crate::panic::tyrus_panic(c"a request body did not fit in memory".as_ptr());
        }
        std::ptr::copy_nonoverlapping(data, allocation, length);
        *output = TyrusArray {
            data: allocation,
            length,
            capacity: length,
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_request_multipart_parts(
    parts: *const crate::types::MultipartPart,
    length: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let element_size = std::mem::size_of::<crate::types::MultipartPart>();
        if parts.is_null() || length == 0 {
            *output = TyrusArray {
                data: std::ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            return;
        }
        let storage = libc::malloc(length * element_size).cast::<crate::types::MultipartPart>();
        if storage.is_null() {
            crate::panic::tyrus_panic(c"a multipart body did not fit in memory".as_ptr());
        }
        for index in 0..length {
            let source = &*parts.add(index);
            storage.add(index).write(crate::types::MultipartPart {
                name: source.name.share(),
                file_name: clone_optional_string(source.file_name),
                content_type: clone_optional_string(source.content_type),
                bytes: clone_owned_bytes(&source.bytes),
            });
        }
        *output = TyrusArray {
            data: storage.cast::<u8>(),
            length,
            capacity: length,
        };
    }
}

unsafe fn clone_optional_string(value: *const TyrusString) -> *const TyrusString {
    unsafe {
        if value.is_null() {
            return std::ptr::null();
        }
        let boxed = libc::malloc(std::mem::size_of::<TyrusString>()).cast::<TyrusString>();
        if boxed.is_null() {
            return std::ptr::null();
        }
        boxed.write((*value).share());
        boxed
    }
}

unsafe fn clone_owned_bytes(source: &TyrusArray) -> TyrusArray {
    unsafe {
        if source.data.is_null() || source.length == 0 {
            return TyrusArray {
                data: std::ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
        }
        let data = libc::malloc(source.length).cast::<u8>();
        if data.is_null() {
            crate::panic::tyrus_panic(c"a multipart part did not fit in memory".as_ptr());
        }
        std::ptr::copy_nonoverlapping(source.data, data, source.length);
        TyrusArray {
            data,
            length: source.length,
            capacity: source.length,
        }
    }
}

/// Stores a binary response body, copying it into memory the response owns.
///
/// The copy is what makes ownership unambiguous. A `string` carries a bit that
/// says whether it owns its buffer, so releasing one that owns nothing is a
/// no-op; a `u8[]` carries no such bit, and the array a handler sends may be a
/// local it still uses, a slice of another array, or a literal that lives in
/// read-only memory. Freeing any of those would be wrong, and freeing none of
/// them would leak every body a handler allocates. Copying costs one pass over
/// bytes the runtime is about to write to a socket anyway.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_reply_bytes(
    response: *mut HttpResponse,
    data: *const u8,
    length: usize,
) -> i32 {
    unsafe {
        let Some(response) = response.as_mut() else {
            return reject_reply(format_args!("a reply body has no response to store it in"));
        };
        response.body.release();
        if length == 0 || data.is_null() {
            response.body = HttpBody::from_owned_bytes(std::ptr::null_mut(), 0);
            return 0;
        }
        let copy = libc::malloc(length).cast::<u8>();
        if copy.is_null() {
            return reject_reply(format_args!(
                "a reply body of {length} byte(s) did not fit in memory"
            ));
        }
        std::ptr::copy_nonoverlapping(data, copy, length);
        response.body = HttpBody::from_owned_bytes(copy, length);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_reply_stream(
    response: *mut HttpResponse,
    stream: *const crate::stream::TyrusReadableStream,
) -> i32 {
    unsafe {
        let Some(response) = response.as_mut() else {
            return reject_reply(format_args!("a reply body has no response to store it in"));
        };
        if stream.is_null() {
            return reject_reply(format_args!("a reply stream has no readable source"));
        }
        response.body.release();
        response.body = HttpBody::from_stream(crate::stream::tyrus_readable_stream_retain(stream));
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_reply_writable_stream(
    response: *mut HttpResponse,
    out: *mut *const crate::stream::TyrusWritableStream,
) -> i32 {
    unsafe {
        let (Some(response), Some(out)) = (response.as_mut(), out.as_mut()) else {
            return reject_reply(format_args!(
                "a writable reply stream needs a response and output"
            ));
        };
        let (writer, consumer) = crate::stream::byte_writable_stream(HTTP_STREAM_BUFFER_BYTES);
        response.body.release();
        response.body = HttpBody::from_writable_stream(consumer);
        *out = writer;
        notify_reply_writable(response);
        0
    }
}

unsafe fn set_reply_header(
    response: *mut HttpResponse,
    name: TyrusString,
    value: TyrusString,
    append: bool,
) -> i32 {
    unsafe {
        let (Some(response), Ok(name), Ok(value)) = (response.as_mut(), text(&name), text(&value))
        else {
            return reject_reply(format_args!(
                "a reply header has an unreadable name or value"
            ));
        };
        let Ok(header_name) = HeaderName::from_bytes(name.as_bytes()) else {
            return reject_reply(format_args!("`{name}` is not a valid header name"));
        };
        let Ok(value) = HeaderValue::from_str(&value) else {
            return reject_reply(format_args!(
                "the value of header `{name}` contains a character a header cannot carry"
            ));
        };
        let name = header_name;
        if protected_reply_header(&name) {
            return reject_reply(format_args!(
                "`{name}` is set by the runtime and cannot be set by a handler"
            ));
        }
        let mut headers = reply_headers_lock();
        let entries = headers.entry(reply_header_key(response)).or_default();
        if !append {
            entries.retain(|(entry_name, _)| *entry_name != name);
        }
        entries.push((name, value));
        0
    }
}

fn protected_reply_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "connection" | "content-length" | "set-cookie" | "transfer-encoding"
    )
}

unsafe fn append_cookie(response: &mut HttpResponse, cookie: String) {
    unsafe {
        let mut current = text(&response.cookies).unwrap_or_default();
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(&cookie);
        let previous = std::mem::replace(
            &mut response.cookies,
            crate::strings::alloc_string(&current),
        );
        previous.release();
    }
}

unsafe fn format_cookie_with_options(
    name: &str,
    value: &str,
    options: *const CookieOptions,
) -> Result<String, ()> {
    unsafe {
        let mut cookie = format!("{name}={value}");
        if options.is_null() {
            return Ok(cookie);
        }
        let options = &*options;
        let domain = optional_cookie_string(options.domain)?;
        let path = optional_cookie_string(options.path)?;
        let same_site = optional_cookie_string(options.same_site)?;
        let max_age = options.max_age.as_ref().copied();
        let secure = options.secure.as_ref().copied().unwrap_or(false);
        let http_only = options.http_only.as_ref().copied().unwrap_or(false);

        if let Some(domain) = domain {
            if !valid_cookie_domain(&domain) {
                return Err(());
            }
            cookie.push_str("; Domain=");
            cookie.push_str(&domain);
        }
        if let Some(path) = path {
            if !valid_cookie_path(&path) {
                return Err(());
            }
            cookie.push_str("; Path=");
            cookie.push_str(&path);
        }
        if let Some(max_age) = max_age {
            if max_age < 0 {
                return Err(());
            }
            cookie.push_str("; Max-Age=");
            cookie.push_str(&max_age.to_string());
        }
        if let Some(same_site) = same_site.as_deref() {
            if !matches!(same_site, "lax" | "strict" | "none") || (same_site == "none" && !secure) {
                return Err(());
            }
            cookie.push_str("; SameSite=");
            cookie.push_str(match same_site {
                "lax" => "Lax",
                "strict" => "Strict",
                "none" => "None",
                _ => unreachable!("same-site values are validated above"),
            });
        }
        if secure {
            cookie.push_str("; Secure");
        }
        if http_only {
            cookie.push_str("; HttpOnly");
        }
        HeaderValue::from_str(&cookie).map_err(|_| ())?;
        Ok(cookie)
    }
}

unsafe fn optional_cookie_string(value: *const TyrusString) -> Result<Option<String>, ()> {
    unsafe {
        if value.is_null() {
            return Ok(None);
        }
        text(&*value).map(Some)
    }
}

fn valid_cookie_domain(domain: &str) -> bool {
    !domain.is_empty()
        && domain
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
}

fn valid_cookie_path(path: &str) -> bool {
    path.starts_with('/')
        && !path
            .bytes()
            .any(|byte| byte.is_ascii_control() || matches!(byte, b';' | b','))
}

fn valid_cookie_name(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|byte| {
            matches!(
                byte,
                b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.' | b'0'..=b'9' | b'A'..=b'Z' | b'^' | b'_' | b'`' | b'a'..=b'z' | b'|' | b'~'
            )
        })
}

fn invalid_cookie_value(value: &str) -> bool {
    value.bytes().any(|byte| {
        byte < 0x21 || byte == b'"' || byte == b',' || byte == b';' || byte == b'\\' || byte == 0x7f
    })
}

#[cfg(test)]
mod route_params_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn a_request_body_piece_wider_than_the_capacity_is_split_rather_than_refused() {
        let oversized = vec![7_u8; HTTP_STREAM_BUFFER_BYTES * 3 + 11];
        let readable = stream_request_body(Body::from(oversized.clone()), oversized.len() + 1);
        let stream = unsafe { crate::stream::take_owned_readable_stream(readable) };

        let mut received = Vec::new();
        while let Some(chunk) = crate::stream::read_owned_byte_stream(&stream)
            .await
            .unwrap()
        {
            assert!(
                chunk.len() <= HTTP_STREAM_BUFFER_BYTES,
                "a chunk wider than the capacity reached the reader"
            );
            received.extend_from_slice(&chunk);
        }
        assert_eq!(received, oversized);
    }

    #[test]
    fn parameters_are_resolved_from_the_route_once() {
        let params = RouteParams::parse("/users/:id/posts/:slug");
        assert_eq!(
            params.names,
            vec![(2, "id".to_string()), (4, "slug".to_string())]
        );

        let json = path_params_json(&params, "/users/42/posts/hello");
        assert!(json.contains("\"id\""), "{json}");
        assert!(json.contains("42"), "{json}");
        assert!(json.contains("hello"), "{json}");
    }

    #[test]
    fn url_components_are_decoded_before_schema_validation() {
        let params = RouteParams::parse("/users/:name");
        let params: serde_json::Value =
            serde_json::from_str(&path_params_json(&params, "/users/Ada%20Lovelace")).unwrap();
        assert_eq!(params["name"], "Ada Lovelace");

        let query: serde_json::Value =
            serde_json::from_str(&query_json("term=type%20safe&plus=a%2Bb&space=hello+world"))
                .unwrap();
        assert_eq!(query["term"], "type safe");
        assert_eq!(query["plus"], "a+b");
        assert_eq!(query["space"], "hello world");
    }

    #[test]
    fn repeated_query_keys_are_preserved_as_arrays_of_text() {
        let query: serde_json::Value = serde_json::from_str(&query_json(
            "tag=rust&tag=llvm&tag=wasm&limit=25&enabled=true",
        ))
        .unwrap();
        assert_eq!(query["tag"], serde_json::json!(["rust", "llvm", "wasm"]));
        assert_eq!(query["limit"], "25");
        assert_eq!(query["enabled"], "true");
    }

    async fn streamed_response_bytes(encoding: ResponseEncoding, chunks: &[&[u8]]) -> Vec<Vec<u8>> {
        use futures_util::StreamExt;
        let (readable, producer) = crate::stream::byte_readable_stream(64 * 1024);
        let readable = unsafe { crate::stream::take_owned_readable_stream(readable) };
        let body =
            response_stream_body(HttpResponseStreamSource::Readable(readable), Some(encoding));
        tokio::pin!(body);
        let mut pieces = Vec::new();
        for chunk in chunks {
            producer.write(chunk.to_vec()).await.unwrap();
            // Each flushed chunk reaches the client before the next is written.
            let piece = body.next().await.expect("a flushed piece").unwrap();
            pieces.push(piece.to_vec());
        }
        producer.close();
        while let Some(piece) = body.next().await {
            pieces.push(piece.unwrap().to_vec());
        }
        pieces
    }

    #[tokio::test]
    async fn a_streamed_response_is_compressed_chunk_by_chunk() {
        let chunks: [&[u8]; 3] = [b"first event\n", b"second event\n", b"third event\n"];
        for (encoding, format) in [
            (ResponseEncoding::Gzip, crate::codec::Format::Gzip),
            (ResponseEncoding::Brotli, crate::codec::Format::Brotli),
        ] {
            let pieces = streamed_response_bytes(encoding, &chunks).await;
            assert!(
                pieces.len() > chunks.len(),
                "{encoding:?} flushed each chunk"
            );
            let limits = crate::codec::Limits {
                max_output: u64::MAX,
                max_ratio: u32::MAX,
            };
            assert_eq!(
                crate::codec::decompress(&pieces.concat(), Some(format), limits).unwrap(),
                chunks.concat()
            );
        }
    }

    #[test]
    fn malformed_accept_encoding_quality_never_enables_a_coding() {
        assert_eq!(
            preferred_response_encoding(Some("br;q=invalid, gzip;q=0.5")),
            Some(ResponseEncoding::Gzip)
        );
        assert_eq!(preferred_response_encoding(Some("br;q=2")), None);
    }

    #[test]
    fn a_static_route_has_no_parameters() {
        let params = RouteParams::parse("/health");
        assert!(params.is_empty());
        assert_eq!(path_params_json(&params, "/health"), empty_json_object());
    }

    #[test]
    fn malformed_and_short_paths_do_not_panic() {
        let bare = RouteParams::parse("/users/:");
        assert!(bare.is_empty());

        let params = RouteParams::parse("/users/:id/posts/:slug");
        let json = path_params_json(&params, "/users/42");
        assert!(json.contains("\"id\""), "{json}");
        assert!(json.contains("\"slug\""), "{json}");
    }

    #[test]
    fn rate_limiter_never_exceeds_maximum_under_concurrent_requests() {
        let limiter = RateLimiter::new_with_global(32, 60_000, -1);
        let accepted = Arc::new(AtomicUsize::new(0));
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let limiter = Arc::clone(&limiter);
                let accepted = Arc::clone(&accepted);
                std::thread::spawn(move || {
                    for _ in 0..32 {
                        if limiter.allow(IpAddr::V4(Ipv4Addr::LOCALHOST)).is_ok() {
                            accepted.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
        assert_eq!(accepted.load(Ordering::Relaxed), 32);
    }

    #[test]
    fn a_full_rate_limiter_still_serves_a_new_client() {
        let limiter = RateLimiter::new_with_global(1, 60_000, -1);
        let start = StdInstant::now();
        for index in 0..MAX_RATE_LIMIT_BUCKETS {
            let peer = IpAddr::V4(Ipv4Addr::from((index as u32).to_be_bytes()));
            assert!(limiter.allow_at(peer, start).is_ok());
        }
        let newcomer = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 9));
        assert!(
            limiter.allow_at(newcomer, start).is_ok(),
            "a full table refused a client it had never seen"
        );
    }

    #[test]
    fn a_ban_survives_a_table_full_of_other_clients() {
        let limiter = RateLimiter::new_with_ban(1, 60_000, -1, 0, 3_600_000);
        let start = StdInstant::now();
        let offender = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 9));

        assert!(limiter.allow_at(offender, start).is_ok());
        assert!(matches!(
            limiter.allow_at(offender, start),
            Err(RateLimitRejection::Banned(_))
        ));

        let later = start + Duration::from_millis(120_000);
        for index in 0..MAX_RATE_LIMIT_BUCKETS {
            let peer = IpAddr::V4(Ipv4Addr::from((index as u32).to_be_bytes()));
            let _ = limiter.allow_at(peer, later);
        }

        assert!(
            matches!(
                limiter.allow_at(offender, later),
                Err(RateLimitRejection::Banned(_))
            ),
            "a banned client was pardoned by the table filling up"
        );
    }

    #[test]
    fn a_trusted_proxy_matches_only_its_own_network() {
        let network = TrustedProxy::parse("10.0.0.0/8").unwrap();
        assert!(network.contains("10.1.2.3".parse().unwrap()));
        assert!(!network.contains("11.0.0.1".parse().unwrap()));

        let single = TrustedProxy::parse("192.168.0.7").unwrap();
        assert!(single.contains("192.168.0.7".parse().unwrap()));
        assert!(!single.contains("192.168.0.8".parse().unwrap()));

        let partial = TrustedProxy::parse("192.168.128.0/17").unwrap();
        assert!(partial.contains("192.168.200.1".parse().unwrap()));
        assert!(!partial.contains("192.168.127.1".parse().unwrap()));

        let v6 = TrustedProxy::parse("::1/128").unwrap();
        assert!(v6.contains("::1".parse().unwrap()));
        assert!(!v6.contains("127.0.0.1".parse().unwrap()));

        assert!(TrustedProxy::parse("10.0.0.0/33").is_none());
        assert!(TrustedProxy::parse("not-an-address").is_none());
    }

    #[test]
    fn a_forwarded_client_is_read_only_from_a_trusted_peer() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.9".parse().unwrap());

        let untrusted = std::sync::RwLock::new(HttpServerConfig::default());
        let peer: IpAddr = "127.0.0.1".parse().unwrap();
        assert_eq!(client_address(peer, &headers, &untrusted), peer);

        let trusted = std::sync::RwLock::new(HttpServerConfig {
            trusted_proxies: vec![TrustedProxy::parse("127.0.0.0/8").unwrap()],
            ..HttpServerConfig::default()
        });
        assert_eq!(
            client_address(peer, &headers, &trusted),
            "203.0.113.9".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn a_custom_identity_header_is_trusted_only_from_a_trusted_peer() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-client-ip", "203.0.113.9".parse().unwrap());
        headers.insert("x-forwarded-for", "198.51.100.7".parse().unwrap());
        let peer: IpAddr = "127.0.0.1".parse().unwrap();

        let untrusted = std::sync::RwLock::new(HttpServerConfig {
            trusted_proxy_identity_header: Some("x-client-ip".to_string()),
            ..HttpServerConfig::default()
        });
        assert_eq!(client_address(peer, &headers, &untrusted), peer);

        let trusted = std::sync::RwLock::new(HttpServerConfig {
            trusted_proxies: vec![TrustedProxy::parse("127.0.0.0/8").unwrap()],
            trusted_proxy_identity_header: Some("x-client-ip".to_string()),
            ..HttpServerConfig::default()
        });
        assert_eq!(
            client_address(peer, &headers, &trusted),
            "203.0.113.9".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn forwarded_chains_stop_at_the_first_untrusted_hop() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "1.2.3.4, 203.0.113.9, 10.0.0.5".parse().unwrap(),
        );
        let config = std::sync::RwLock::new(HttpServerConfig {
            trusted_proxies: vec![
                TrustedProxy::parse("127.0.0.0/8").unwrap(),
                TrustedProxy::parse("10.0.0.0/8").unwrap(),
            ],
            ..HttpServerConfig::default()
        });
        assert_eq!(
            client_address("127.0.0.1".parse().unwrap(), &headers, &config),
            "203.0.113.9".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn forwarded_addresses_are_read_in_every_shape_they_arrive_in() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-forwarded-for", "198.51.100.1".parse().unwrap());
        headers.insert(
            axum::http::header::FORWARDED,
            r#"for="[2001:db8::1]:8080";proto=https"#.parse().unwrap(),
        );
        assert_eq!(
            forwarded_chains(&headers),
            vec![
                vec!["2001:db8::1".parse::<IpAddr>().unwrap()],
                vec!["198.51.100.1".parse::<IpAddr>().unwrap()],
            ]
        );

        let mut ports = axum::http::HeaderMap::new();
        ports.insert("x-forwarded-for", "198.51.100.1:4321".parse().unwrap());
        assert_eq!(
            forwarded_chains(&ports),
            vec![vec!["198.51.100.1".parse::<IpAddr>().unwrap()]]
        );
    }

    #[test]
    fn forwarding_headers_that_disagree_fall_back_to_the_peer() {
        let peer: IpAddr = "127.0.0.1".parse().unwrap();
        let config = std::sync::RwLock::new(HttpServerConfig {
            trusted_proxies: vec![TrustedProxy::parse("127.0.0.0/8").unwrap()],
            ..HttpServerConfig::default()
        });

        let mut forged = axum::http::HeaderMap::new();
        forged.insert("x-forwarded-for", "203.0.113.9".parse().unwrap());
        forged.insert(
            axum::http::header::FORWARDED,
            "for=198.51.100.7".parse().unwrap(),
        );
        assert_eq!(client_address(peer, &forged, &config), peer);

        let mut agreeing = axum::http::HeaderMap::new();
        agreeing.insert("x-forwarded-for", "203.0.113.9".parse().unwrap());
        agreeing.insert(
            axum::http::header::FORWARDED,
            "for=203.0.113.9".parse().unwrap(),
        );
        assert_eq!(
            client_address(peer, &agreeing, &config),
            "203.0.113.9".parse::<IpAddr>().unwrap()
        );

        let declared = std::sync::RwLock::new(HttpServerConfig {
            trusted_proxies: vec![TrustedProxy::parse("127.0.0.0/8").unwrap()],
            trusted_proxy_identity_header: Some("x-client-ip".to_string()),
            ..HttpServerConfig::default()
        });
        let mut mixed = axum::http::HeaderMap::new();
        mixed.insert("x-client-ip", "203.0.113.9".parse().unwrap());
        mixed.insert(
            axum::http::header::FORWARDED,
            "for=198.51.100.7".parse().unwrap(),
        );
        assert_eq!(
            client_address(peer, &mixed, &declared),
            "203.0.113.9".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn rate_limiter_refills_one_token_over_its_window() {
        let limiter = RateLimiter::new_with_global(1, 1_000, -1);
        let peer = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let start = StdInstant::now();
        assert!(limiter.allow_at(peer, start).is_ok());
        assert!(
            limiter
                .allow_at(peer, start + Duration::from_millis(999))
                .is_err()
        );
        assert!(
            limiter
                .allow_at(peer, start + Duration::from_millis(1_000))
                .is_ok()
        );
    }

    #[test]
    fn rate_limiter_refills_continuously_without_a_window_edge_burst() {
        let limiter = RateLimiter::new_with_global(2, 1_000, -1);
        let peer = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let start = StdInstant::now();
        assert!(limiter.allow_at(peer, start).is_ok());
        assert!(limiter.allow_at(peer, start).is_ok());
        assert!(
            limiter
                .allow_at(peer, start + Duration::from_millis(500))
                .is_ok()
        );
        assert!(
            limiter
                .allow_at(peer, start + Duration::from_millis(999))
                .is_err()
        );
        assert!(
            limiter
                .allow_at(peer, start + Duration::from_millis(1_000))
                .is_ok()
        );
    }

    #[test]
    fn rate_limiter_enforces_global_maximum_across_client_identities() {
        let limiter = RateLimiter::new_with_global(2, 1_000, 3);
        let first = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
        let second = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 2));
        let start = StdInstant::now();
        assert!(limiter.allow_at(first, start).is_ok());
        assert!(limiter.allow_at(first, start).is_ok());
        assert!(limiter.allow_at(second, start).is_ok());
        assert!(limiter.allow_at(second, start).is_err());
    }

    #[test]
    fn rate_limiter_bans_after_configured_identity_exceedances() {
        let limiter = RateLimiter::new_with_ban(1, 1_000, -1, 2, 5_000);
        let peer = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let start = StdInstant::now();
        assert!(limiter.allow_at(peer, start).is_ok());
        assert_eq!(
            limiter.allow_at(peer, start),
            Err(RateLimitRejection::Limited(Duration::from_secs(1)))
        );
        assert!(matches!(
            limiter.allow_at(peer, start),
            Err(RateLimitRejection::Limited(_))
        ));
        assert_eq!(
            limiter.allow_at(peer, start),
            Err(RateLimitRejection::Banned(Duration::from_secs(5)))
        );
        assert!(matches!(
            limiter.allow_at(peer, start + Duration::from_secs(1)),
            Err(RateLimitRejection::Banned(_))
        ));
    }

    #[test]
    fn global_limit_does_not_accumulate_identity_ban_strikes() {
        let limiter = RateLimiter::new_with_ban(2, 1_000, 1, 0, 5_000);
        let first = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
        let second = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 2));
        let start = StdInstant::now();
        assert!(limiter.allow_at(first, start).is_ok());
        assert!(matches!(
            limiter.allow_at(second, start),
            Err(RateLimitRejection::Limited(_))
        ));
        assert!(matches!(
            limiter.allow_at(second, start),
            Err(RateLimitRejection::Limited(_))
        ));
        assert_eq!(
            limiter.state.lock().unwrap().buckets[&second].exceedances,
            0
        );
    }

    #[test]
    fn rejected_global_limit_does_not_spend_the_identity_token() {
        let limiter = RateLimiter::new_with_global(2, 1_000, 1);
        let first = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
        let second = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 2));
        let start = StdInstant::now();
        assert!(limiter.allow_at(first, start).is_ok());
        assert!(limiter.allow_at(second, start).is_err());
        let state = limiter.state.lock().unwrap();
        assert_eq!(state.buckets.get(&second).unwrap().credit, limiter.capacity);
        drop(state);
        assert!(
            limiter
                .allow_at(second, start + Duration::from_millis(1_000))
                .is_ok()
        );
    }

    #[test]
    fn rate_limit_numeric_helpers_cap_credit_and_round_retry_up() {
        assert_eq!(refill_credit(3, 10, 2, 20), 20);
        assert_eq!(refill_credit(3, 2, 2, 20), 7);
        assert_eq!(retry_after_nanos(3, 8, 2), Some(3));
        assert_eq!(retry_after_nanos(8, 8, 2), None);
    }

    #[test]
    fn csrf_requires_matching_cookie_and_header_only_for_unsafe_methods() {
        let csrf = CsrfConfig {
            cookie: "__Host-csrf".to_string(),
            header: axum::http::HeaderName::from_static("x-csrf-token"),
        };
        let mut matching = axum::http::HeaderMap::new();
        matching.insert(
            "cookie",
            "session=abc; __Host-csrf=token-123".parse().unwrap(),
        );
        matching.insert("x-csrf-token", "token-123".parse().unwrap());
        assert!(csrf_request_is_valid(&matching, &csrf));
        assert!(csrf_protected_method(&Method::POST));
        assert!(csrf_protected_method(&Method::DELETE));
        assert!(!csrf_protected_method(&Method::GET));
        assert!(!csrf_protected_method(&Method::HEAD));
        assert!(!csrf_protected_method(&Method::OPTIONS));

        let mut mismatching = matching.clone();
        mismatching.insert("x-csrf-token", "token-124".parse().unwrap());
        assert!(!csrf_request_is_valid(&mismatching, &csrf));
        assert!(!csrf_request_is_valid(&axum::http::HeaderMap::new(), &csrf));
    }

    #[test]
    fn csrf_comparison_handles_different_lengths_without_accepting_prefixes() {
        assert!(csrf_tokens_match("same", "same"));
        assert!(!csrf_tokens_match("same", "same-but-longer"));
        assert!(!csrf_tokens_match("same-but-longer", "same"));
        assert!(!csrf_tokens_match("same", "sane"));
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::{refill_credit, retry_after_nanos};

    #[kani::proof]
    fn rate_limit_refill_never_exceeds_capacity() {
        let credit: u128 = kani::any();
        let elapsed_nanos: u128 = kani::any();
        let max: u64 = kani::any();
        let capacity: u128 = kani::any();
        kani::assume(credit <= capacity);

        let refilled = refill_credit(credit, elapsed_nanos, max, capacity);

        assert!(refilled <= capacity);
        assert!(refilled >= credit);
    }

    #[kani::proof]
    fn rate_limit_retry_after_is_precise_or_absent() {
        let credit: u128 = kani::any();
        let cost: u128 = kani::any();
        let max: u64 = kani::any();

        match retry_after_nanos(credit, cost, max) {
            None => assert!(credit >= cost),
            Some(delay) => {
                assert!(credit < cost);
                assert!(delay > 0);
            }
        }
    }
}
