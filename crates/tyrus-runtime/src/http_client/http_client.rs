use crate::abi::TyrusArray;
use crate::promise::{PromiseReference, TyrusPromise};
use crate::stream::{TyrusReadableStream, byte_readable_stream, take_owned_readable_stream};
use axum::body::Bytes;
use std::collections::HashMap;
use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::sync::Mutex;
use std::time::Duration;
use tyrus_string::TyrusString;

#[path = "content_decoding/content_decoding.rs"]
mod content_decoding;

#[repr(C)]
pub struct HttpHeader {
    pub name: TyrusString,
    pub value: TyrusString,
}

/// What `fetch` was asked to do, beyond the URL.
///
/// Every field is the language ABI for an omitted option: a nullable pointer
/// for the boxed ones, `-1` for the numbers. The durations and byte sizes
/// arrive already parsed, because their text format is read during compilation
/// and the runtime must not carry a second parser that could disagree.
#[repr(C)]
pub struct FetchOptions {
    pub method: *const TyrusString,
    pub headers: *const TyrusArray,
    pub body: *const TyrusString,
    pub timeout_millis: i64,
    pub max_body_bytes: i64,
    pub redirects: i64,
    pub body_stream: *const TyrusReadableStream,
    /// `-1` when omitted, which means on; `0` off; `1` on.
    pub decompress: i64,
}

enum RequestBody {
    Text(String),
    Stream(std::sync::Arc<TyrusReadableStream>),
}

struct HttpRequestStreamSource(std::sync::Arc<TyrusReadableStream>);

impl Drop for HttpRequestStreamSource {
    fn drop(&mut self) {
        crate::stream::close_owned_byte_stream(&self.0);
    }
}

pub const DEFAULT_MAX_BODY_BYTES: u64 = 8 * 1024 * 1024;

pub const DEFAULT_REDIRECTS: i64 = 5;

const HTTP_STREAM_BUFFER_BYTES: usize = 64 * 1024;

pub struct HttpClientResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[repr(C)]
pub(crate) struct HttpErrorFfi {
    pub(crate) code: TyrusString,
    pub(crate) message: TyrusString,
}

#[repr(C)]
pub(crate) struct HttpResultFfi<T> {
    pub(crate) tag: u8,
    pub(crate) payload: HttpResultPayload<T>,
}

#[repr(C)]
pub(crate) union HttpResultPayload<T> {
    pub(crate) ok: ManuallyDrop<T>,
    pub(crate) err: ManuallyDrop<HttpErrorFfi>,
    words: [u64; 4],
}

pub(crate) const HTTP_RESULT_TAG_OK: u8 = 0;
pub(crate) const HTTP_RESULT_TAG_ERR: u8 = 1;

const _: () = {
    assert!(std::mem::size_of::<HttpErrorFfi>() == 32);
    assert!(std::mem::size_of::<HttpResultFfi<*mut c_void>>() == 40);
    assert!(std::mem::size_of::<HttpResultFfi<*const c_void>>() == 40);
    assert!(std::mem::offset_of!(HttpResultFfi<*mut c_void>, payload) == 8);
};

pub(crate) fn http_error(code: &'static str, message: &'static str) -> HttpErrorFfi {
    HttpErrorFfi {
        code: TyrusString::literal(code),
        message: TyrusString::literal(message),
    }
}

pub(crate) unsafe fn fulfill_http_result<T: Copy>(
    promise: *mut TyrusPromise,
    outcome: Result<T, HttpErrorFfi>,
    drop_ok: unsafe fn(T),
    drop_result: unsafe extern "C" fn(*mut c_void),
    allocation_error: (&'static str, &'static str),
) {
    let value = match outcome {
        Ok(value) => HttpResultFfi {
            tag: HTTP_RESULT_TAG_OK,
            payload: HttpResultPayload {
                ok: ManuallyDrop::new(value),
            },
        },
        Err(error) => HttpResultFfi {
            tag: HTTP_RESULT_TAG_ERR,
            payload: HttpResultPayload {
                err: ManuallyDrop::new(error),
            },
        },
    };
    let result =
        unsafe { libc::malloc(std::mem::size_of::<HttpResultFfi<T>>()).cast::<HttpResultFfi<T>>() };
    if result.is_null() {
        if value.tag == HTTP_RESULT_TAG_OK {
            unsafe { drop_ok(*value.payload.ok) };
        }
        unsafe {
            crate::tyrus_promise_reject_pending_structured(
                promise,
                TyrusString::literal(allocation_error.0),
                TyrusString::literal(allocation_error.1),
            )
        };
        return;
    }
    unsafe {
        std::ptr::write(result, value);
        crate::tyrus_promise_fulfill_owned(promise, result.cast(), Some(drop_result));
    }
}

pub struct HttpClientStreamResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    stream: *const TyrusReadableStream,
    request: tokio::task::AbortHandle,
}

impl Drop for HttpClientStreamResponse {
    fn drop(&mut self) {
        if unsafe { !crate::stream::readable_stream_has_other_owner(self.stream) } {
            self.request.abort();
        }
        unsafe { crate::stream::tyrus_readable_stream_free(self.stream) };
    }
}

fn client_for(redirects: i64) -> reqwest::Client {
    static CLIENTS: std::sync::OnceLock<Mutex<HashMap<i64, reqwest::Client>>> =
        std::sync::OnceLock::new();
    let clients = CLIENTS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut clients = match clients.lock() {
        Ok(clients) => clients,
        Err(poisoned) => poisoned.into_inner(),
    };
    clients
        .entry(redirects)
        .or_insert_with(|| {
            let policy = if redirects <= 0 {
                reqwest::redirect::Policy::none()
            } else {
                let limit = redirects as usize;
                reqwest::redirect::Policy::custom(move |attempt| {
                    if !crate::fetch_policy::url_is_permitted(attempt.url()) {
                        return attempt.error(crate::fetch_policy::BlockedDestination);
                    }
                    if attempt.previous().len() > limit {
                        return attempt.error(TooManyRedirects);
                    }
                    attempt.follow()
                })
            };
            reqwest::Client::builder()
                .redirect(policy)
                .dns_resolver(std::sync::Arc::new(crate::fetch_policy::PolicyResolver))
                .build()
                .unwrap_or_default()
        })
        .clone()
}

#[derive(Debug)]
struct TooManyRedirects;

impl std::fmt::Display for TooManyRedirects {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("too many redirects")
    }
}

impl std::error::Error for TooManyRedirects {}

fn error_chain(error: &reqwest::Error) -> String {
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(error);
    let mut chain = String::new();
    while let Some(level) = source {
        chain.push_str(&level.to_string());
        chain.push('\n');
        source = level.source();
    }
    chain
}

fn transport_failure(error: &reqwest::Error) -> (&'static str, &'static str) {
    let chain = error_chain(error);
    if chain.contains(crate::fetch_policy::BLOCKED_MARKER) {
        return crate::fetch_policy::BLOCKED;
    }
    if error.is_timeout() {
        return ("ERR_HTTP_FETCH_TIMEOUT", "the request took too long");
    }
    if error.is_connect() {
        if chain.contains("dns error") {
            return ("ERR_HTTP_FETCH_DNS", "the host name could not be resolved");
        }
        let is_https = error
            .url()
            .is_some_and(|url| url.scheme().eq_ignore_ascii_case("https"));
        if is_https && !chain.contains("tcp connect error") {
            return (
                "ERR_HTTP_FETCH_TLS",
                "the secure connection could not be established",
            );
        }
        return (
            "ERR_HTTP_FETCH_CONNECT",
            "the connection to the server failed",
        );
    }
    if error.is_redirect() {
        return (
            "ERR_HTTP_FETCH_REDIRECT",
            "the request was redirected more times than allowed",
        );
    }
    if error.is_body() || error.is_decode() {
        return ("ERR_HTTP_FETCH_BODY", "the response body could not be read");
    }
    ("ERR_HTTP_FETCH", "the request failed")
}

unsafe fn take_optional_string(value: *const TyrusString) -> Option<String> {
    unsafe {
        let boxed = value.as_ref()?;
        let text = boxed.as_str().to_owned();
        std::mem::replace(&mut *(value as *mut TyrusString), TyrusString::null()).release();
        libc::free(value.cast_mut().cast());
        Some(text)
    }
}

unsafe fn take_headers(headers: *const TyrusArray) -> Vec<(String, String)> {
    unsafe {
        let Some(array) = headers.as_ref() else {
            return Vec::new();
        };
        let mut read = Vec::new();
        if !array.data.is_null() && array.length > 0 && array.length <= array.capacity {
            let entries = array.data.cast::<HttpHeader>();
            for index in 0..array.length {
                let entry = &mut *entries.add(index);
                read.push((
                    entry.name.as_str().to_owned(),
                    entry.value.as_str().to_owned(),
                ));
                std::mem::replace(&mut entry.name, TyrusString::null()).release();
                std::mem::replace(&mut entry.value, TyrusString::null()).release();
            }
            libc::free(array.data.cast());
        }
        libc::free(headers.cast_mut().cast());
        read
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_fetch(
    url: TyrusString,
    options: *const FetchOptions,
) -> *mut TyrusPromise {
    unsafe {
        let promise = crate::tyrus_promise_pending();
        if url.is_null() {
            fulfill_response(
                promise,
                Err(("ERR_HTTP_FETCH_URL", "the request URL is empty")),
            );
            return promise;
        }
        let url = url.as_str().to_owned();
        let request = read_options(options);
        crate::tyrus_promise_retain(promise);
        let held = PromiseReference(promise as usize);
        let running = crate::executor::executor().spawn(async move {
            let outcome = perform(url, request).await;
            let promise = held.0 as *mut TyrusPromise;
            fulfill_response(promise, outcome);
        });
        register_request_cancellation(promise, running);
        promise
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_fetch_stream(
    url: TyrusString,
    options: *const FetchOptions,
) -> *mut TyrusPromise {
    unsafe {
        let promise = crate::tyrus_promise_pending();
        if url.is_null() {
            fulfill_stream_response(
                promise,
                Err(("ERR_HTTP_FETCH_URL", "the request URL is empty")),
            );
            return promise;
        }
        let url = url.as_str().to_owned();
        let request = read_options(options);
        crate::tyrus_promise_retain(promise);
        let held = PromiseReference(promise as usize);
        let running = crate::executor::executor().spawn(async move {
            let outcome = perform_stream(url, request).await;
            let promise = held.0 as *mut TyrusPromise;
            fulfill_stream_response(promise, outcome);
        });
        register_request_cancellation(promise, running);
        promise
    }
}

struct RequestCancellation {
    promise: usize,
    running: Option<tokio::task::JoinHandle<()>>,
}

unsafe fn register_request_cancellation(
    promise: *mut TyrusPromise,
    running: tokio::task::JoinHandle<()>,
) {
    let context = Box::into_raw(Box::new(RequestCancellation {
        promise: promise as usize,
        running: Some(running),
    }));
    if unsafe {
        crate::tyrus_promise_on_cancel_request(
            promise,
            abort_request,
            context.cast(),
            Some(drop_abort_handle),
        )
    } == 0
    {
        unsafe { drop_abort_handle(context.cast()) };
    }
}

unsafe extern "C" fn abort_request(context: *mut c_void) {
    let context = unsafe { &mut *context.cast::<RequestCancellation>() };
    let Some(running) = context.running.take() else {
        return;
    };
    let promise = context.promise as *mut TyrusPromise;
    unsafe { crate::tyrus_promise_retain(promise) };
    running.abort();
    let held = PromiseReference(context.promise);
    crate::executor::executor().spawn(async move {
        let _ = running.await;
        let promise = held.0 as *mut TyrusPromise;
        unsafe { crate::tyrus_promise_fulfill_owned(promise, std::ptr::null_mut(), None) };
    });
}

unsafe extern "C" fn drop_abort_handle(context: *mut c_void) {
    unsafe {
        drop(Box::from_raw(context.cast::<RequestCancellation>()));
    }
}

fn request_body(body: RequestBody) -> reqwest::Body {
    match body {
        RequestBody::Text(text) => reqwest::Body::from(text),
        RequestBody::Stream(stream) => {
            reqwest::Body::wrap_stream(futures_util::stream::try_unfold(
                HttpRequestStreamSource(stream),
                |source| async move {
                    match crate::stream::read_owned_byte_stream(&source.0).await {
                        Ok(Some(chunk)) => Ok(Some((Bytes::from(chunk), source))),
                        Ok(None) => Ok(None),
                        Err(_) => Err(std::io::Error::new(
                            std::io::ErrorKind::BrokenPipe,
                            "the request body stream failed",
                        )),
                    }
                },
            ))
        }
    }
}

/// Everything a request carries besides its URL, read once from the options.
struct Request {
    method: Option<String>,
    headers: Vec<(String, String)>,
    body: Option<RequestBody>,
    timeout: Option<Duration>,
    max_body_bytes: u64,
    redirects: i64,
    decompress: bool,
}

unsafe fn read_options(options: *const FetchOptions) -> Request {
    unsafe {
        let Some(options) = options.as_ref() else {
            return Request {
                method: None,
                headers: Vec::new(),
                body: None,
                timeout: None,
                max_body_bytes: DEFAULT_MAX_BODY_BYTES,
                redirects: DEFAULT_REDIRECTS,
                decompress: true,
            };
        };
        let text = take_optional_string(options.body);
        let stream = (!options.body_stream.is_null())
            .then(|| take_owned_readable_stream(options.body_stream));
        Request {
            method: take_optional_string(options.method),
            headers: take_headers(options.headers),
            body: stream
                .map(RequestBody::Stream)
                .or_else(|| text.map(RequestBody::Text)),
            timeout: (options.timeout_millis >= 0)
                .then(|| Duration::from_millis(options.timeout_millis as u64)),
            max_body_bytes: if options.max_body_bytes >= 0 {
                options.max_body_bytes as u64
            } else {
                DEFAULT_MAX_BODY_BYTES
            },
            redirects: if options.redirects >= 0 {
                options.redirects
            } else {
                DEFAULT_REDIRECTS
            },
            decompress: options.decompress != 0,
        }
    }
}

/// Sends the request and returns the response head, with the coding the body
/// must be decoded from when decompression is on.
async fn send(
    url: String,
    request: Request,
) -> Result<
    (
        reqwest::Response,
        Vec<(String, String)>,
        Option<content_decoding::Coding>,
        u64,
    ),
    (&'static str, &'static str),
> {
    let Request {
        method,
        mut headers,
        body,
        timeout,
        max_body_bytes,
        redirects,
        decompress,
    } = request;
    let redirects = if matches!(body.as_ref(), Some(RequestBody::Stream(_))) {
        0
    } else {
        redirects
    };
    let method = match method.as_deref() {
        None => reqwest::Method::GET,
        Some(name) => reqwest::Method::from_bytes(name.as_bytes())
            .map_err(|_| ("ERR_HTTP_FETCH_METHOD", "the request method is not valid"))?,
    };
    if decompress {
        content_decoding::offer_codings(&mut headers);
    }
    let client = client_for(redirects);
    let mut request = client.request(method, &url);
    for (name, value) in headers {
        request = request.header(name, value);
    }
    if let Some(body) = body {
        request = request.body(request_body(body));
    }
    if let Some(timeout) = timeout {
        request = request.timeout(timeout);
    }
    let request = request
        .build()
        .map_err(|_| ("ERR_HTTP_FETCH_URL", "the request URL is not valid"))?;
    if !crate::fetch_policy::url_is_permitted(request.url()) {
        return Err(crate::fetch_policy::BLOCKED);
    }
    let response = client
        .execute(request)
        .await
        .map_err(|error| transport_failure(&error))?;
    if response
        .content_length()
        .is_some_and(|length| length > max_body_bytes)
    {
        return Err((
            "ERR_HTTP_FETCH_BODY_TOO_LARGE",
            "the response body is larger than the caller allows",
        ));
    }
    let mut head = response
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_owned(), value.to_owned()))
        })
        .collect::<Vec<_>>();
    let coding = if decompress {
        content_decoding::response_coding(&head)
    } else {
        None
    };
    if coding.is_some() {
        content_decoding::forget_encoded_length(&mut head);
    }
    Ok((response, head, coding, max_body_bytes))
}

async fn perform(
    url: String,
    request: Request,
) -> Result<HttpClientResponse, (&'static str, &'static str)> {
    let (response, headers, coding, max_body_bytes) = send(url, request).await?;
    let status = response.status().as_u16();
    let body = read_bounded_body(response, max_body_bytes).await?;
    let body = match coding {
        Some(coding) => tokio::task::spawn_blocking(move || {
            content_decoding::decode_body(&body, coding, max_body_bytes)
        })
        .await
        .map_err(|_| content_decoding::DECODE_FAILED)??,
        None => body,
    };
    Ok(HttpClientResponse {
        status,
        headers,
        body,
    })
}

async fn perform_stream(
    url: String,
    request: Request,
) -> Result<HttpClientStreamResponse, (&'static str, &'static str)> {
    let (response, headers, coding, max_body_bytes) = send(url, request).await?;
    let status = response.status().as_u16();
    let (stream, producer) = byte_readable_stream(HTTP_STREAM_BUFFER_BYTES);
    let request = crate::executor::executor().spawn(async move {
        let mut response = response;
        let mut received = 0_u64;
        let mut decoder =
            coding.map(|coding| content_decoding::StreamDecoder::new(coding, max_body_bytes));
        loop {
            let chunk = match response.chunk().await {
                Ok(Some(chunk)) => chunk,
                Ok(None) => {
                    if let Some(decoder) = decoder.as_mut()
                        && let Err(error) = decoder.finish(&producer).await
                    {
                        producer.fail_with(error);
                        return;
                    }
                    producer.close();
                    return;
                }
                Err(_) => {
                    producer.fail();
                    return;
                }
            };
            received = received.saturating_add(chunk.len() as u64);
            if received > max_body_bytes {
                producer.fail();
                return;
            }
            if let Some(decoder) = decoder.as_mut() {
                match decoder.push(&chunk, &producer).await {
                    Ok(()) => continue,
                    Err(error) => {
                        producer.fail_with(error);
                        return;
                    }
                }
            }
            for piece in chunk.chunks(HTTP_STREAM_BUFFER_BYTES) {
                if producer.write(piece.to_vec()).await.is_err() {
                    producer.fail();
                    return;
                }
            }
        }
    });
    Ok(HttpClientStreamResponse {
        status,
        headers,
        stream,
        request: request.abort_handle(),
    })
}

async fn read_bounded_body(
    mut response: reqwest::Response,
    max_body_bytes: u64,
) -> Result<Vec<u8>, (&'static str, &'static str)> {
    let mut body = Vec::new();
    loop {
        let chunk = response
            .chunk()
            .await
            .map_err(|error| transport_failure(&error))?;
        let Some(chunk) = chunk else {
            return Ok(body);
        };
        if body.len() as u64 + chunk.len() as u64 > max_body_bytes {
            return Err((
                "ERR_HTTP_FETCH_BODY_TOO_LARGE",
                "the response body is larger than the caller allows",
            ));
        }
        body.extend_from_slice(&chunk);
    }
}

fn fulfill_response(
    promise: *mut TyrusPromise,
    outcome: Result<HttpClientResponse, (&'static str, &'static str)>,
) {
    match outcome {
        Ok(response) => unsafe {
            let handle = Box::into_raw(Box::new(response));
            crate::tyrus_promise_fulfill_owned(
                promise,
                Box::into_raw(Box::new(handle)).cast(),
                Some(drop_response_handle),
            );
        },
        Err((code, message)) => unsafe {
            crate::tyrus_promise_reject_pending_structured(
                promise,
                TyrusString::literal(code),
                TyrusString::literal(message),
            );
        },
    }
}

fn fulfill_stream_response(
    promise: *mut TyrusPromise,
    outcome: Result<HttpClientStreamResponse, (&'static str, &'static str)>,
) {
    match outcome {
        Ok(response) => unsafe {
            let handle = Box::into_raw(Box::new(response));
            crate::tyrus_promise_fulfill_owned(
                promise,
                Box::into_raw(Box::new(handle)).cast(),
                Some(drop_stream_response_handle),
            );
        },
        Err((code, message)) => unsafe {
            crate::tyrus_promise_reject_pending_structured(
                promise,
                TyrusString::literal(code),
                TyrusString::literal(message),
            );
        },
    }
}

unsafe extern "C" fn drop_response_handle(value: *mut c_void) {
    unsafe {
        if !value.is_null() {
            let handle = *Box::from_raw(value.cast::<*mut HttpClientResponse>());
            if !handle.is_null() {
                drop(Box::from_raw(handle));
            }
        }
    }
}

unsafe extern "C" fn drop_stream_response_handle(value: *mut c_void) {
    unsafe {
        if !value.is_null() {
            let handle = *Box::from_raw(value.cast::<*mut HttpClientStreamResponse>());
            if !handle.is_null() {
                drop(Box::from_raw(handle));
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_client_response_status(handle: *mut c_void) -> i32 {
    unsafe {
        handle
            .cast::<HttpClientResponse>()
            .as_ref()
            .map_or(0, |response| i32::from(response.status))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_client_response_ok(handle: *mut c_void) -> bool {
    unsafe {
        handle
            .cast::<HttpClientResponse>()
            .as_ref()
            .is_some_and(|response| (200..300).contains(&response.status))
    }
}

/// One response header by name, as a `string?`.
///
/// The box is built here rather than by generated code, which is the
/// convention every other optional string crossing this ABI already follows: a
/// null pointer is the absent one, and a present one owns its text. Boxing on
/// the other side needed a branch on a null string and a decision about who
/// frees it, and got both wrong before this.
///
/// Matched without regard to case, which is what the protocol says a header
/// name is; a caller that wrote `Content-Type` and a server that sent
/// `content-type` mean the same header.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_client_response_header(
    handle: *mut c_void,
    name: TyrusString,
) -> *const TyrusString {
    unsafe {
        let (Some(response), false) =
            (handle.cast::<HttpClientResponse>().as_ref(), name.is_null())
        else {
            return std::ptr::null();
        };
        let wanted = name.as_str().to_ascii_lowercase();
        let Some((_, value)) = response
            .headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(&wanted))
        else {
            return std::ptr::null();
        };
        let boxed = libc::malloc(std::mem::size_of::<TyrusString>()).cast::<TyrusString>();
        if boxed.is_null() {
            return std::ptr::null();
        }
        boxed.write(crate::strings::alloc_string(value));
        boxed
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_client_response_text(handle: *mut c_void) -> TyrusString {
    unsafe {
        handle
            .cast::<HttpClientResponse>()
            .as_ref()
            .map_or_else(TyrusString::null, |response| {
                crate::strings::alloc_string(&String::from_utf8_lossy(&response.body))
            })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_client_response_bytes(
    handle: *mut c_void,
    output: *mut TyrusArray,
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let empty = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let Some(response) = handle.cast::<HttpClientResponse>().as_ref() else {
            *output = empty;
            return;
        };
        if response.body.is_empty() {
            *output = empty;
            return;
        }
        let length = response.body.len();
        let data = libc::malloc(length).cast::<u8>();
        if data.is_null() {
            crate::panic::tyrus_panic(c"a response body did not fit in memory".as_ptr());
        }
        std::ptr::copy_nonoverlapping(response.body.as_ptr(), data, length);
        *output = TyrusArray {
            data,
            length,
            capacity: length,
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_client_response_free(handle: *mut c_void) {
    unsafe {
        if handle.is_null() {
            return;
        }
        drop(Box::from_raw(handle.cast::<HttpClientResponse>()));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_client_stream_response_status(handle: *mut c_void) -> i32 {
    unsafe {
        handle
            .cast::<HttpClientStreamResponse>()
            .as_ref()
            .map_or(0, |response| i32::from(response.status))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_client_stream_response_ok(handle: *mut c_void) -> bool {
    unsafe {
        handle
            .cast::<HttpClientStreamResponse>()
            .as_ref()
            .is_some_and(|response| (200..300).contains(&response.status))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_client_stream_response_header(
    handle: *mut c_void,
    name: TyrusString,
) -> *const TyrusString {
    unsafe {
        let (Some(response), false) = (
            handle.cast::<HttpClientStreamResponse>().as_ref(),
            name.is_null(),
        ) else {
            return std::ptr::null();
        };
        let wanted = name.as_str().to_ascii_lowercase();
        let Some((_, value)) = response
            .headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(&wanted))
        else {
            return std::ptr::null();
        };
        let boxed = libc::malloc(std::mem::size_of::<TyrusString>()).cast::<TyrusString>();
        if boxed.is_null() {
            return std::ptr::null();
        }
        boxed.write(crate::strings::alloc_string(value));
        boxed
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_client_stream_response_stream(
    handle: *mut c_void,
) -> *mut TyrusPromise {
    unsafe {
        let promise = crate::tyrus_promise_pending();
        let Some(response) = handle.cast::<HttpClientStreamResponse>().as_ref() else {
            crate::tyrus_promise_fulfill_owned(promise, std::ptr::null_mut(), None);
            return promise;
        };
        let stream = crate::stream::tyrus_readable_stream_retain(response.stream);
        let payload = Box::into_raw(Box::new(stream));
        crate::tyrus_promise_fulfill_owned(
            promise,
            payload.cast(),
            Some(drop_readable_stream_payload),
        );
        promise
    }
}

unsafe extern "C" fn drop_readable_stream_payload(value: *mut c_void) {
    unsafe {
        if value.is_null() {
            return;
        }
        let stream = *Box::from_raw(value.cast::<*const TyrusReadableStream>());
        crate::stream::tyrus_readable_stream_free(stream);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_client_stream_response_free(handle: *mut c_void) {
    unsafe {
        if !handle.is_null() {
            drop(Box::from_raw(handle.cast::<HttpClientStreamResponse>()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_fetch_url_rejects_with_http_error() {
        let promise = unsafe { tyrus_http_fetch(TyrusString::null(), std::ptr::null()) };
        assert_eq!(
            unsafe { crate::tyrus_promise_state(promise) },
            crate::TYRUS_PROMISE_REJECTED
        );
        let code = unsafe { crate::tyrus_promise_error_code(promise) };
        assert_eq!(unsafe { code.as_str() }, "ERR_HTTP_FETCH_URL");
        unsafe { code.release() };
        unsafe { crate::tyrus_promise_free(promise) };

        let promise = unsafe { tyrus_http_fetch_stream(TyrusString::null(), std::ptr::null()) };
        assert_eq!(
            unsafe { crate::tyrus_promise_state(promise) },
            crate::TYRUS_PROMISE_REJECTED
        );
        let code = unsafe { crate::tyrus_promise_error_code(promise) };
        assert_eq!(unsafe { code.as_str() }, "ERR_HTTP_FETCH_URL");
        unsafe { code.release() };
        unsafe { crate::tyrus_promise_free(promise) };
    }

    #[tokio::test]
    async fn closing_a_stream_response_keeps_a_retained_body_reader_alive() {
        let (stream, producer) = crate::stream::byte_readable_stream(64);
        let transport = crate::executor::executor().spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            producer.write(vec![7, 0, 9]).await.unwrap();
            producer.close();
        });
        let response = HttpClientStreamResponse {
            status: 200,
            headers: Vec::new(),
            stream,
            request: transport.abort_handle(),
        };
        let reader = unsafe { crate::stream::tyrus_readable_stream_retain(stream) };
        drop(response);

        let bytes = tokio::time::timeout(Duration::from_secs(1), unsafe {
            crate::stream::read_byte_stream(reader)
        })
        .await
        .expect("closing the response must not abort its retained reader")
        .unwrap();
        assert_eq!(bytes, Some(vec![7, 0, 9]));
        unsafe { crate::stream::tyrus_readable_stream_free(reader) };
    }

    #[tokio::test]
    async fn closing_a_stream_response_without_a_reader_aborts_its_transport() {
        let (stream, producer) = crate::stream::byte_readable_stream(64);
        let (started, started_at) = tokio::sync::oneshot::channel();
        let transport = crate::executor::executor().spawn(async move {
            let _ = started.send(());
            tokio::time::sleep(Duration::from_secs(5)).await;
            producer.write(vec![7]).await.unwrap();
            producer.close();
        });
        started_at.await.unwrap();
        let response = HttpClientStreamResponse {
            status: 200,
            headers: Vec::new(),
            stream,
            request: transport.abort_handle(),
        };
        drop(response);
        tokio::time::timeout(Duration::from_secs(1), transport)
            .await
            .expect("closing an unclaimed response must abort its transport")
            .unwrap_err();
    }

    #[tokio::test]
    async fn dropping_an_outbound_request_source_cancels_its_retained_producer() {
        let (stream, producer) = crate::stream::byte_readable_stream(64);
        let source =
            HttpRequestStreamSource(unsafe { crate::stream::take_owned_readable_stream(stream) });
        let retained = unsafe { crate::stream::tyrus_readable_stream_retain(stream) };
        drop(source);
        assert_eq!(
            producer.write(vec![7]).await.unwrap_err().code,
            "ERR_STREAM_CLOSED"
        );
        unsafe { crate::stream::tyrus_readable_stream_free(retained) };
    }
}
