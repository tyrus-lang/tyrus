use crate::abi::{TyrusArray, TyrusStringArray};
use crate::executor::executor;
use crate::http_client::{HTTP_RESULT_TAG_OK, HttpResultFfi, fulfill_http_result, http_error};
use crate::promise::TyrusPromise;
use crate::types::{HttpRouter, text};
use axum::extract::ws::{self as axum_ws, WebSocketUpgrade};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use futures_util::{Sink, SinkExt, Stream, StreamExt};
use std::ffi::c_void;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Weak};
use tokio::sync::{Mutex, mpsc, watch};
use tokio_tungstenite::tungstenite;
use tyrus_string::TyrusString;

const DEFAULT_MAX_MESSAGE_BYTES: usize = 1024 * 1024;
#[cfg(not(test))]
const HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);
#[cfg(test)]
const HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(200);
const WEBSOCKET_STREAM_CAPACITY_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WebSocketError {
    pub code: &'static str,
    pub message: &'static str,
}

impl WebSocketError {
    const CLOSED: Self = Self {
        code: "ERR_HTTP_WS_CLOSED",
        message: "the WebSocket is closed",
    };
    const BUSY: Self = Self {
        code: "ERR_HTTP_WS_BUSY",
        message: "another receive is already pending on this WebSocket",
    };
    const MESSAGE_TOO_LARGE: Self = Self {
        code: "ERR_HTTP_WS_MESSAGE_TOO_LARGE",
        message: "the WebSocket message exceeds the size limit",
    };
    const RECEIVE: Self = Self {
        code: "ERR_HTTP_WS_RECEIVE",
        message: "the WebSocket message could not be received",
    };
    const SEND: Self = Self {
        code: "ERR_HTTP_WS_SEND",
        message: "the WebSocket message could not be sent",
    };
    const CLOSE_CODE: Self = Self {
        code: "ERR_HTTP_WS_CLOSE_CODE",
        message: "a WebSocket close code must be 1000 or between 3000 and 4999",
    };
    const CLOSE_REASON: Self = Self {
        code: "ERR_HTTP_WS_CLOSE_REASON",
        message: "a WebSocket close reason must be at most 123 bytes",
    };
    const ALLOCATION: Self = Self {
        code: "ERR_HTTP_WS_ALLOCATION",
        message: "WebSocket memory allocation failed",
    };
    const STREAMED: Self = Self {
        code: "ERR_HTTP_WS_STREAMED",
        message: "the WebSocket was handed to stream() and is read and written through it",
    };
    const URL: Self = Self {
        code: "ERR_HTTP_WS_URL",
        message: "the WebSocket URL is not a valid ws:// or wss:// URL",
    };
    const DNS: Self = Self {
        code: "ERR_HTTP_WS_DNS",
        message: "the host name could not be resolved",
    };
    const CONNECT: Self = Self {
        code: "ERR_HTTP_WS_CONNECT",
        message: "the WebSocket connection failed",
    };
    const TLS: Self = Self {
        code: "ERR_HTTP_WS_TLS",
        message: "the secure connection could not be established",
    };
    const HANDSHAKE: Self = Self {
        code: "ERR_HTTP_WS_HANDSHAKE",
        message: "the server did not accept the WebSocket upgrade",
    };
    const BLOCKED: Self = Self {
        code: crate::fetch_policy::BLOCKED.0,
        message: crate::fetch_policy::BLOCKED.1,
    };
}

pub(crate) enum Outgoing {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close(u16, String),
}

pub(crate) enum Incoming {
    Text(String),
    Binary(Vec<u8>),
    Closed(Option<u16>),
    Ping(Vec<u8>),
    Pong,
    Control,
}

#[derive(Clone, Copy)]
pub(crate) enum Failure {
    TooLarge,
    Closed,
    Transport,
}

type FrameSink = Pin<Box<dyn Sink<Outgoing, Error = Failure> + Send>>;
type FrameSource = Pin<Box<dyn Stream<Item = Result<Incoming, Failure>> + Send>>;

pub struct WebSocketHandle {
    sink: Mutex<FrameSink>,
    source: Mutex<mpsc::Receiver<Result<Incoming, Failure>>>,
    protocol: String,
    closed: AtomicBool,
    streamed: AtomicBool,
    write_closed: AtomicBool,
    peer_close_code: AtomicI32,
    going_away: AtomicBool,
    close_signal: watch::Sender<bool>,
}

fn tungstenite_failure(error: &tungstenite::Error) -> Failure {
    match error {
        tungstenite::Error::Capacity(_) => Failure::TooLarge,
        tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed => Failure::Closed,
        _ => Failure::Transport,
    }
}

fn axum_failure(error: axum::Error) -> Failure {
    let inner = error.into_inner();
    match inner.downcast_ref::<tungstenite::Error>() {
        Some(error) => tungstenite_failure(error),
        None => Failure::Transport,
    }
}

impl WebSocketHandle {
    fn new(sink: FrameSink, source: FrameSource, protocol: String) -> Arc<Self> {
        let (close_signal, _) = watch::channel(false);
        let (sender, receiver) = mpsc::channel(1);
        let handle = Arc::new(Self {
            sink: Mutex::new(sink),
            source: Mutex::new(receiver),
            protocol,
            closed: AtomicBool::new(false),
            streamed: AtomicBool::new(false),
            write_closed: AtomicBool::new(false),
            peer_close_code: AtomicI32::new(-1),
            going_away: AtomicBool::new(false),
            close_signal,
        });
        Self::spawn_frame_driver(source, sender, Arc::downgrade(&handle));
        handle
    }

    fn spawn_frame_driver(
        mut source: FrameSource,
        sender: mpsc::Sender<Result<Incoming, Failure>>,
        socket: Weak<Self>,
    ) {
        executor().spawn(async move {
            let mut ticks = tokio::time::interval_at(
                tokio::time::Instant::now() + HEARTBEAT_INTERVAL,
                HEARTBEAT_INTERVAL,
            );
            let mut awaiting_pong = false;
            let mut closed = socket
                .upgrade()
                .map(|handle| handle.close_signal.subscribe());
            loop {
                tokio::select! {
                    () = async {
                        if let Some(closed) = &mut closed {
                            let _ = closed.changed().await;
                        }
                    } => return,
                    _ = ticks.tick() => {
                        if awaiting_pong {
                            let _ = sender.send(Ok(Incoming::Closed(None))).await;
                            return;
                        }
                        let Some(handle) = socket.upgrade() else {
                            return;
                        };
                        let mut sink = handle.sink.lock().await;
                        if sink.send(Outgoing::Ping(Vec::new())).await.is_err() {
                            let _ = sender.send(Err(Failure::Transport)).await;
                            return;
                        }
                        awaiting_pong = true;
                    }
                    frame = source.next() => match frame {
                        Some(Ok(Incoming::Pong)) => awaiting_pong = false,
                        Some(Ok(Incoming::Control)) => {},
                        Some(Ok(Incoming::Ping(payload))) => {
                            let Some(handle) = socket.upgrade() else {
                                return;
                            };
                            let mut sink = handle.sink.lock().await;
                            if sink.send(Outgoing::Pong(payload)).await.is_err() {
                                let _ = sender.send(Err(Failure::Transport)).await;
                                return;
                            }
                        }
                        Some(Ok(frame)) => {
                            if sender.send(Ok(frame)).await.is_err() {
                                return;
                            }
                        }
                        Some(Err(error)) => {
                            let _ = sender.send(Err(error)).await;
                            return;
                        }
                        None => {
                            let _ = sender.send(Ok(Incoming::Closed(None))).await;
                            return;
                        }
                    },
                }
            }
        });
    }

    fn from_axum(socket: axum_ws::WebSocket, protocol: String) -> Arc<Self> {
        let (sink, source) = socket.split();
        let sink = sink
            .with(|frame: Outgoing| async move {
                Ok::<_, axum::Error>(match frame {
                    Outgoing::Text(text) => axum_ws::Message::Text(text.into()),
                    Outgoing::Binary(bytes) => axum_ws::Message::Binary(bytes.into()),
                    Outgoing::Ping(bytes) => axum_ws::Message::Ping(bytes.into()),
                    Outgoing::Pong(bytes) => axum_ws::Message::Pong(bytes.into()),
                    Outgoing::Close(code, reason) => {
                        axum_ws::Message::Close(Some(axum_ws::CloseFrame {
                            code,
                            reason: reason.into(),
                        }))
                    }
                })
            })
            .sink_map_err(axum_failure);
        let source = source.map(|item| match item {
            Ok(axum_ws::Message::Text(text)) => Ok(Incoming::Text(text.as_str().to_owned())),
            Ok(axum_ws::Message::Binary(bytes)) => Ok(Incoming::Binary(bytes.to_vec())),
            Ok(axum_ws::Message::Close(frame)) => {
                Ok(Incoming::Closed(frame.map(|frame| frame.code)))
            }
            Ok(axum_ws::Message::Ping(bytes)) => Ok(Incoming::Ping(bytes.to_vec())),
            Ok(axum_ws::Message::Pong(_)) => Ok(Incoming::Pong),
            Err(error) => Err(axum_failure(error)),
        });
        Self::new(Box::pin(sink), Box::pin(source), protocol)
    }

    fn from_tungstenite<S>(
        socket: tokio_tungstenite::WebSocketStream<S>,
        protocol: String,
    ) -> Arc<Self>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        use tungstenite::protocol::frame::coding::CloseCode;

        let (sink, source) = socket.split();
        let sink = sink
            .with(|frame: Outgoing| async move {
                Ok::<_, tungstenite::Error>(match frame {
                    Outgoing::Text(text) => tungstenite::Message::Text(text.into()),
                    Outgoing::Binary(bytes) => tungstenite::Message::Binary(bytes.into()),
                    Outgoing::Ping(bytes) => tungstenite::Message::Ping(bytes.into()),
                    Outgoing::Pong(bytes) => tungstenite::Message::Pong(bytes.into()),
                    Outgoing::Close(code, reason) => {
                        tungstenite::Message::Close(Some(tungstenite::protocol::CloseFrame {
                            code: CloseCode::from(code),
                            reason: reason.into(),
                        }))
                    }
                })
            })
            .sink_map_err(|error| tungstenite_failure(&error));
        let source = source.map(|item| match item {
            Ok(tungstenite::Message::Text(text)) => Ok(Incoming::Text(text.as_str().to_owned())),
            Ok(tungstenite::Message::Binary(bytes)) => Ok(Incoming::Binary(bytes.to_vec())),
            Ok(tungstenite::Message::Close(frame)) => {
                Ok(Incoming::Closed(frame.map(|frame| u16::from(frame.code))))
            }
            Ok(tungstenite::Message::Ping(bytes)) => Ok(Incoming::Ping(bytes.to_vec())),
            Ok(tungstenite::Message::Pong(_)) => Ok(Incoming::Pong),
            Ok(tungstenite::Message::Frame(_)) => Ok(Incoming::Control),
            Err(error) => Err(tungstenite_failure(&error)),
        });
        Self::new(Box::pin(sink), Box::pin(source), protocol)
    }

    pub(crate) async fn receive(&self) -> Result<Option<Incoming>, WebSocketError> {
        if self.streamed.load(Ordering::Acquire) {
            return Err(WebSocketError::STREAMED);
        }
        self.receive_frame().await
    }

    async fn receive_frame(&self) -> Result<Option<Incoming>, WebSocketError> {
        let mut closed = self.close_signal.subscribe();
        if self.closed.load(Ordering::Acquire) {
            return self.closed_receive();
        }
        let Ok(mut source) = self.source.try_lock() else {
            return Err(WebSocketError::BUSY);
        };
        let read = async {
            match source.recv().await {
                Some(Ok(Incoming::Closed(code))) => {
                    if let Some(code) = code {
                        self.peer_close_code
                            .store(i32::from(code), Ordering::Release);
                    }
                    Ok(None)
                }
                None => {
                    if self.closed.load(Ordering::Acquire) {
                        self.closed_receive()
                    } else {
                        Ok(None)
                    }
                }
                Some(Err(Failure::Closed)) => Ok(None),
                Some(Ok(message)) => Ok(Some(message)),
                Some(Err(Failure::TooLarge)) => Err(WebSocketError::MESSAGE_TOO_LARGE),
                Some(Err(Failure::Transport)) => Err(WebSocketError::RECEIVE),
            }
        };
        tokio::select! {
            result = read => result,
            _ = closed.wait_for(|closed| *closed) => self.closed_receive(),
        }
    }

    fn closed_receive(&self) -> Result<Option<Incoming>, WebSocketError> {
        if self.going_away.load(Ordering::Acquire) {
            Ok(None)
        } else {
            Err(WebSocketError::CLOSED)
        }
    }

    fn peer_close_code(&self) -> Option<i32> {
        let code = self.peer_close_code.load(Ordering::Acquire);
        (code >= 0).then_some(code)
    }

    pub(crate) async fn send(&self, frame: Outgoing) -> Result<(), WebSocketError> {
        if self.streamed.load(Ordering::Acquire) {
            return Err(WebSocketError::STREAMED);
        }
        self.send_frame(frame).await
    }

    async fn send_frame(&self, frame: Outgoing) -> Result<(), WebSocketError> {
        let mut closed = self.close_signal.subscribe();
        if self.closed.load(Ordering::Acquire) || self.write_closed.load(Ordering::Acquire) {
            return Err(WebSocketError::CLOSED);
        }
        tokio::select! {
            result = async {
                let mut sink = self.sink.lock().await;
                sink.send(frame).await
            } => result.map_err(|failure| match failure {
                Failure::Closed => WebSocketError::CLOSED,
                _ => WebSocketError::SEND,
            }),
            _ = closed.wait_for(|closed| *closed) => Err(WebSocketError::CLOSED),
        }
    }

    pub(crate) async fn close(&self) {
        self.close_frame(1000, String::new()).await;
    }

    pub(crate) async fn close_with(&self, code: u16, reason: &str) -> Result<(), WebSocketError> {
        if code != 1000 && !(3000..=4999).contains(&code) {
            return Err(WebSocketError::CLOSE_CODE);
        }
        if reason.len() > 123 {
            return Err(WebSocketError::CLOSE_REASON);
        }
        self.close_frame(code, reason.to_owned()).await;
        Ok(())
    }

    pub(crate) async fn go_away(&self) {
        self.going_away.store(true, Ordering::Release);
        self.close_frame(1001, "server shutting down".to_owned())
            .await;
    }

    async fn close_frame(&self, code: u16, reason: String) {
        if self.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        self.close_signal.send_replace(true);
        let mut sink = self.sink.lock().await;
        let _ = sink.send(Outgoing::Close(code, reason)).await;
        let _ = sink.close().await;
    }

    async fn close_write(&self) {
        if self.write_closed.swap(true, Ordering::AcqRel) || self.closed.load(Ordering::Acquire) {
            return;
        }
        let mut sink = self.sink.lock().await;
        let _ = sink.send(Outgoing::Close(1000, String::new())).await;
    }

    fn take_stream(&self) -> Result<(), WebSocketError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(WebSocketError::CLOSED);
        }
        if self
            .streamed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(WebSocketError::STREAMED);
        }
        if self.source.try_lock().is_err() {
            self.streamed.store(false, Ordering::Release);
            return Err(WebSocketError::BUSY);
        }
        Ok(())
    }
}

unsafe fn retained(handle: *const WebSocketHandle) -> Option<Arc<WebSocketHandle>> {
    if handle.is_null() {
        return None;
    }
    let handle = std::mem::ManuallyDrop::new(unsafe { Arc::from_raw(handle) });
    Some(Arc::clone(&handle))
}

#[cfg(test)]
fn reject(promise: *mut TyrusPromise, error: WebSocketError) {
    unsafe {
        crate::tyrus_promise_reject_pending_structured(
            promise,
            TyrusString::literal(error.code),
            TyrusString::literal(error.message),
        );
    }
}

/// The ABI shape of the `WebSocketMessage` interface, in its field order.
#[repr(C)]
pub struct TyrusWebSocketMessage {
    pub binary: bool,
    pub text: TyrusString,
    pub bytes: TyrusArray,
}

unsafe extern "C" fn drop_websocket_message_payload(value: *mut c_void) {
    if value.is_null() {
        return;
    }
    let message = unsafe { Box::from_raw(value.cast::<TyrusWebSocketMessage>()) };
    unsafe { message.text.release() };
    if !message.bytes.data.is_null() {
        unsafe { libc::free(message.bytes.data.cast()) };
    }
}

fn empty_array() -> TyrusArray {
    TyrusArray {
        data: std::ptr::null_mut(),
        length: 0,
        capacity: 0,
    }
}

fn allocate_message_payload(
    message: Option<Incoming>,
) -> Result<*mut TyrusWebSocketMessage, WebSocketError> {
    let message = match message {
        Some(Incoming::Text(text)) => {
            let text = if text.is_empty() {
                TyrusString::literal("")
            } else {
                let allocated = crate::strings::alloc_string(&text);
                if allocated.is_null() {
                    return Err(WebSocketError::ALLOCATION);
                }
                allocated
            };
            Box::into_raw(Box::new(TyrusWebSocketMessage {
                binary: false,
                text,
                bytes: empty_array(),
            }))
        }
        Some(Incoming::Binary(bytes)) => {
            let data = if bytes.is_empty() {
                std::ptr::null_mut()
            } else {
                let allocation = unsafe { libc::malloc(bytes.len()) }.cast::<u8>();
                if allocation.is_null() {
                    return Err(WebSocketError::ALLOCATION);
                }
                unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), allocation, bytes.len()) };
                allocation
            };
            Box::into_raw(Box::new(TyrusWebSocketMessage {
                binary: true,
                text: TyrusString::literal(""),
                bytes: TyrusArray {
                    data,
                    length: bytes.len(),
                    capacity: bytes.len(),
                },
            }))
        }
        _ => std::ptr::null_mut(),
    };
    Ok(message)
}

unsafe extern "C" fn drop_receive_result(value: *mut c_void) {
    if value.is_null() {
        return;
    }
    let mut result =
        unsafe { Box::from_raw(value.cast::<HttpResultFfi<*mut TyrusWebSocketMessage>>()) };
    if result.tag == HTTP_RESULT_TAG_OK {
        let payload = unsafe { std::mem::ManuallyDrop::take(&mut result.payload.ok) };
        unsafe { drop_websocket_message_payload(payload.cast()) };
    } else {
        let error = unsafe { std::mem::ManuallyDrop::take(&mut result.payload.err) };
        unsafe {
            error.code.release();
            error.message.release();
        }
    }
}

unsafe fn spawn_receive_result_operation<F>(operation: F) -> *mut TyrusPromise
where
    F: std::future::Future<Output = Result<Option<Incoming>, WebSocketError>> + Send + 'static,
{
    let promise = crate::tyrus_promise_pending();
    unsafe { crate::tyrus_promise_retain(promise) };
    let promise_address = promise as usize;
    executor().spawn(async move {
        let outcome = operation
            .await
            .and_then(allocate_message_payload)
            .map_err(|error| http_error(error.code, error.message));
        let promise = promise_address as *mut TyrusPromise;
        unsafe {
            fulfill_http_result(
                promise,
                outcome,
                |payload| drop_websocket_message_payload(payload.cast()),
                drop_receive_result,
                (
                    "ERR_HTTP_WS_ALLOCATION",
                    "WebSocket result allocation failed",
                ),
            );
            crate::tyrus_promise_free(promise);
        }
    });
    promise
}

fn closed_receive_result_promise() -> *mut TyrusPromise {
    let promise = crate::tyrus_promise_pending();
    unsafe {
        fulfill_http_result(
            promise,
            Err(http_error(
                WebSocketError::CLOSED.code,
                WebSocketError::CLOSED.message,
            )),
            |payload: *mut TyrusWebSocketMessage| {
                drop_websocket_message_payload(payload.cast());
            },
            drop_receive_result,
            (
                "ERR_HTTP_WS_ALLOCATION",
                "WebSocket result allocation failed",
            ),
        )
    };
    promise
}

#[cfg(test)]
unsafe fn spawn_operation<F, T>(
    operation: F,
    settle: unsafe fn(*mut TyrusPromise, T),
) -> *mut TyrusPromise
where
    F: std::future::Future<Output = Result<T, WebSocketError>> + Send + 'static,
    T: Send + 'static,
{
    let promise = crate::tyrus_promise_pending();
    unsafe { crate::tyrus_promise_retain(promise) };
    let promise_address = promise as usize;
    executor().spawn(async move {
        let result = operation.await;
        let promise = promise_address as *mut TyrusPromise;
        match result {
            Ok(value) => unsafe { settle(promise, value) },
            Err(error) => reject(promise, error),
        }
        unsafe { crate::tyrus_promise_free(promise) };
    });
    promise
}

unsafe extern "C" fn drop_void_result(value: *mut c_void) {
    if value.is_null() {
        return;
    }
    let mut result = unsafe { Box::from_raw(value.cast::<HttpResultFfi<()>>()) };
    if result.tag == HTTP_RESULT_TAG_OK {
        unsafe { std::mem::ManuallyDrop::drop(&mut result.payload.ok) };
    } else {
        let error = unsafe { std::mem::ManuallyDrop::take(&mut result.payload.err) };
        unsafe {
            error.code.release();
            error.message.release();
        }
    }
}

unsafe fn spawn_void_result_operation<F>(operation: F) -> *mut TyrusPromise
where
    F: std::future::Future<Output = Result<(), WebSocketError>> + Send + 'static,
{
    let promise = crate::tyrus_promise_pending();
    unsafe { crate::tyrus_promise_retain(promise) };
    let promise_address = promise as usize;
    executor().spawn(async move {
        let outcome = operation
            .await
            .map_err(|error| http_error(error.code, error.message));
        let promise = promise_address as *mut TyrusPromise;
        unsafe {
            fulfill_http_result(
                promise,
                outcome,
                |(): ()| {},
                drop_void_result,
                (
                    "ERR_HTTP_WS_ALLOCATION",
                    "WebSocket result allocation failed",
                ),
            );
            crate::tyrus_promise_free(promise);
        }
    });
    promise
}

fn closed_void_result_promise() -> *mut TyrusPromise {
    let promise = crate::tyrus_promise_pending();
    unsafe {
        fulfill_http_result(
            promise,
            Err(http_error(
                WebSocketError::CLOSED.code,
                WebSocketError::CLOSED.message,
            )),
            |(): ()| {},
            drop_void_result,
            (
                "ERR_HTTP_WS_ALLOCATION",
                "WebSocket result allocation failed",
            ),
        )
    };
    promise
}

#[cfg(test)]
unsafe fn fulfill_void(promise: *mut TyrusPromise, (): ()) {
    unsafe { crate::tyrus_promise_fulfill(promise, std::ptr::null_mut(), None) };
}

unsafe extern "C" fn drop_duplex_stream_payload(value: *mut c_void) {
    if value.is_null() {
        return;
    }
    unsafe { crate::stream::tyrus_duplex_stream_free(value.cast()) };
}

unsafe extern "C" fn drop_websocket_stream_result(value: *mut c_void) {
    if value.is_null() {
        return;
    }
    let result = unsafe { value.cast::<HttpResultFfi<*mut c_void>>().as_mut() };
    let Some(result) = result else {
        return;
    };
    if result.tag == HTTP_RESULT_TAG_OK {
        unsafe { drop_duplex_stream_payload(*result.payload.ok) };
    } else {
        let error = unsafe { std::mem::ManuallyDrop::take(&mut result.payload.err) };
        unsafe {
            error.code.release();
            error.message.release();
        }
    }
    unsafe { libc::free(value) };
}

fn websocket_stream_error() -> crate::stream::StreamError {
    crate::stream::StreamError {
        code: "ERR_STREAM_WS_TEXT_MESSAGE",
        message: "the WebSocket byte stream received a text message",
    }
}

fn spawn_duplex_pumps(
    socket: Arc<WebSocketHandle>,
    producer: crate::stream::ByteStreamProducer,
    consumer: crate::stream::ByteStreamConsumer,
) {
    let reader_socket = Arc::clone(&socket);
    executor().spawn(async move {
        loop {
            let next = tokio::select! {
                message = reader_socket.receive_frame() => Some(message),
                () = producer.writable_reader_gone() => None,
            };
            match next {
                Some(Ok(Some(Incoming::Binary(bytes)))) => {
                    if producer.write(bytes).await.is_err() {
                        return;
                    }
                }
                Some(Ok(Some(Incoming::Text(_)))) => {
                    producer.fail_with(websocket_stream_error());
                    return;
                }
                Some(Ok(Some(_) | None)) => {
                    producer.close();
                    return;
                }
                Some(Err(_)) => {
                    producer.fail();
                    return;
                }
                None => return,
            }
        }
    });
    executor().spawn(async move {
        loop {
            match consumer.read().await {
                Ok(Some(bytes)) => {
                    if socket.send_frame(Outgoing::Binary(bytes)).await.is_err() {
                        consumer.fail();
                        return;
                    }
                }
                Ok(None) => {
                    socket.close_write().await;
                    consumer.close();
                    return;
                }
                Err(_) => return,
            }
        }
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_websocket_stream_duplex(
    handle: *const WebSocketHandle,
) -> *mut TyrusPromise {
    let promise = crate::tyrus_promise_pending();
    let outcome: Result<*mut c_void, crate::http_client::HttpErrorFfi> = (|| {
        let socket = unsafe { retained(handle) }.ok_or(WebSocketError::CLOSED)?;
        socket.take_stream()?;
        let (readable, producer) =
            crate::stream::byte_readable_stream(WEBSOCKET_STREAM_CAPACITY_BYTES);
        let (writable, consumer) =
            crate::stream::byte_writable_stream(WEBSOCKET_STREAM_CAPACITY_BYTES);
        spawn_duplex_pumps(socket, producer, consumer);
        let duplex = unsafe { crate::stream::tyrus_duplex_stream_new(readable, writable) };
        unsafe {
            crate::stream::tyrus_readable_stream_free(readable);
            crate::stream::tyrus_writable_stream_free(writable);
        }
        Ok(duplex.cast_mut().cast::<c_void>())
    })()
    .map_err(|error: WebSocketError| http_error(error.code, error.message));
    unsafe {
        fulfill_http_result(
            promise,
            outcome,
            |payload| drop_duplex_stream_payload(payload),
            drop_websocket_stream_result,
            (
                "ERR_HTTP_WS_ALLOCATION",
                "WebSocket result allocation failed",
            ),
        )
    };
    promise
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_websocket_receive(
    handle: *const WebSocketHandle,
) -> *mut TyrusPromise {
    let Some(socket) = (unsafe { retained(handle) }) else {
        return closed_receive_result_promise();
    };
    unsafe { spawn_receive_result_operation(async move { socket.receive().await }) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_websocket_send_text(
    handle: *const WebSocketHandle,
    message: TyrusString,
) -> *mut TyrusPromise {
    let Some(socket) = (unsafe { retained(handle) }) else {
        return closed_void_result_promise();
    };
    let message = if message.is_null() {
        String::new()
    } else {
        unsafe { message.as_str() }.to_owned()
    };
    unsafe {
        spawn_void_result_operation(async move { socket.send(Outgoing::Text(message)).await })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_websocket_send_bytes(
    handle: *const WebSocketHandle,
    data: *const TyrusArray,
) -> *mut TyrusPromise {
    let Some(socket) = (unsafe { retained(handle) }) else {
        return closed_void_result_promise();
    };
    let bytes = match unsafe { data.as_ref() } {
        Some(array) if array.length > 0 && !array.data.is_null() => {
            unsafe { std::slice::from_raw_parts(array.data, array.length) }.to_vec()
        }
        _ => Vec::new(),
    };
    unsafe {
        spawn_void_result_operation(async move { socket.send(Outgoing::Binary(bytes)).await })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_websocket_close(
    handle: *const WebSocketHandle,
    code: i32,
    reason: TyrusString,
) -> *mut TyrusPromise {
    let Some(socket) = (unsafe { retained(handle) }) else {
        return closed_void_result_promise();
    };
    let code = if code < 0 {
        Some(1000)
    } else {
        u16::try_from(code).ok()
    };
    let reason = if reason.is_null() {
        String::new()
    } else {
        unsafe { reason.as_str() }.to_owned()
    };
    unsafe {
        spawn_void_result_operation(async move {
            let code = code.ok_or(WebSocketError::CLOSE_CODE)?;
            socket.close_with(code, &reason).await
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_websocket_protocol(handle: *const WebSocketHandle) -> TyrusString {
    match unsafe { handle.as_ref() } {
        Some(socket) if !socket.protocol.is_empty() => {
            let protocol = crate::strings::alloc_string(&socket.protocol);
            if protocol.is_null() {
                TyrusString::literal("")
            } else {
                protocol
            }
        }
        _ => TyrusString::literal(""),
    }
}

/// Returns an owned optional `i32` payload. The nullable pointer is the ABI
/// representation of `i32?`; the code generator releases a present box.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_websocket_close_code(handle: *const WebSocketHandle) -> *mut i32 {
    unsafe { handle.as_ref() }
        .and_then(WebSocketHandle::peer_close_code)
        .map_or(std::ptr::null_mut(), |code| Box::into_raw(Box::new(code)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_websocket_retain(
    handle: *const WebSocketHandle,
) -> *const WebSocketHandle {
    if !handle.is_null() {
        unsafe { Arc::increment_strong_count(handle) };
    }
    handle
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_websocket_free(handle: *const WebSocketHandle) {
    if !handle.is_null() {
        drop(unsafe { Arc::from_raw(handle) });
    }
}

pub type WebSocketHandler = unsafe extern "C" fn(*const WebSocketHandle) -> *mut TyrusPromise;

struct WebSocketRoute {
    handler: WebSocketHandler,
    origins: Vec<String>,
    protocols: Vec<String>,
    max_message_bytes: usize,
    config: Arc<std::sync::RwLock<crate::types::HttpServerConfig>>,
    pre_handlers: Vec<crate::types::HttpPreHandler>,
    route_params: crate::http::RouteParams,
    rate_limiter: Option<Arc<crate::http::RateLimiter>>,
}

struct UpgradeRequest {
    headers: HeaderMap,
    uri: axum::http::Uri,
    peer: Option<SocketAddr>,
    request_id: Option<String>,
}

fn websocket_route_service(route: Arc<WebSocketRoute>) -> axum::routing::MethodRouter {
    axum::routing::get(
        move |upgrade: WebSocketUpgrade,
              headers: HeaderMap,
              uri: axum::http::Uri,
              peer: Option<axum::Extension<axum::extract::ConnectInfo<SocketAddr>>>,
              request_id: Option<axum::Extension<crate::http::RequestId>>| {
            let route = Arc::clone(&route);
            async move {
                let request = UpgradeRequest {
                    headers,
                    uri,
                    peer: peer.map(|axum::Extension(info)| info.0),
                    request_id: request_id.map(|axum::Extension(id)| id.0),
                };
                accept(route, upgrade, request)
            }
        },
    )
}

unsafe fn string_list(values: *const TyrusStringArray, field: &str) -> Option<Vec<String>> {
    let Some(values) = (unsafe { values.as_ref() }) else {
        return Some(Vec::new());
    };
    if values.length > values.capacity || (values.length > 0 && values.data.is_null()) {
        let _ = field;
        crate::http::set_http_error(
            "ERR_HTTP_WS_ROUTE_CONFIG",
            "WebSocket list has an invalid layout",
        );
        return None;
    }
    if values.length == 0 {
        return Some(Vec::new());
    }
    let items = unsafe { std::slice::from_raw_parts(values.data, values.length) };
    let mut list = Vec::with_capacity(items.len());
    for item in items {
        let Ok(item) = text(item) else {
            let _ = field;
            crate::http::set_http_error(
                "ERR_HTTP_WS_ROUTE_CONFIG",
                "WebSocket list contains invalid text",
            );
            return None;
        };
        list.push(item);
    }
    Some(list)
}

fn origin_allowed(allowed: &[String], headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get(header::ORIGIN) else {
        return true;
    };
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    if !allowed.is_empty() {
        return allowed
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(origin));
    }
    let Some(host) = headers
        .get(header::HOST)
        .and_then(|host| host.to_str().ok())
    else {
        return false;
    };
    origin
        .split_once("://")
        .is_some_and(|(_, authority)| authority.eq_ignore_ascii_case(host))
}

fn refused_by_pre_handlers(route: &WebSocketRoute, request: &UpgradeRequest) -> Option<Response> {
    crate::http_docs::refused_by_pre_handlers(
        &route.pre_handlers,
        &route.route_params,
        &request.headers,
        &request.uri,
        request.request_id.as_deref(),
    )
}

fn accept(
    route: Arc<WebSocketRoute>,
    upgrade: WebSocketUpgrade,
    request: UpgradeRequest,
) -> Response {
    let headers = &request.headers;
    if !origin_allowed(&route.origins, headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let rate_limit = route
        .rate_limiter
        .clone()
        .or_else(|| match route.config.read() {
            Ok(config) => config.rate_limit.clone(),
            Err(poisoned) => poisoned.into_inner().rate_limit.clone(),
        });
    if let Some(limiter) = rate_limit {
        let peer = request
            .peer
            .map_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED), |peer| peer.ip());
        let client = crate::http::client_address(peer, headers, &route.config);
        if let Err(rejection) = limiter.allow(client) {
            return crate::http::rate_limit_response(rejection);
        }
    }
    if let Some(refusal) = refused_by_pre_handlers(&route, &request) {
        return refusal;
    }
    let upgrade = upgrade
        .protocols(route.protocols.clone())
        .max_message_size(route.max_message_bytes)
        .max_frame_size(route.max_message_bytes);
    let protocol = upgrade
        .selected_protocol()
        .and_then(|protocol| protocol.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let handler = route.handler;
    let (in_flight, going_away) = {
        let config = match route.config.read() {
            Ok(config) => config,
            Err(poisoned) => poisoned.into_inner(),
        };
        (
            Arc::clone(&config.in_flight),
            config.websocket_going_away.subscribe(),
        )
    };
    upgrade.on_upgrade(move |socket| serve(handler, socket, protocol, in_flight, going_away))
}

/// The reference handed to the handler, given back whatever becomes of this
/// future.
///
/// `Arc::into_raw` paired with a `from_raw` at the end of an `async fn` is not
/// cancellation safe, and this future is cancelled routinely: the server shuts
/// down, or the connection task is dropped. The end of the function is then
/// never reached, the strong reference is never given back, and the handle
/// stays alive with everything it owns, its frame driver and that driver's
/// buffers among them. It was the one root the whole 1.6 MB of this file's
/// sanitizer report hung from. A guard gives it back on the way out, on every
/// way out.
struct HandlerReference(*const WebSocketHandle);

unsafe impl Send for HandlerReference {}

impl Drop for HandlerReference {
    fn drop(&mut self) {
        unsafe { drop(Arc::from_raw(self.0)) };
    }
}

async fn serve(
    handler: WebSocketHandler,
    socket: axum_ws::WebSocket,
    protocol: String,
    in_flight: Arc<crate::types::InFlight>,
    mut going_away: watch::Receiver<bool>,
) {
    let _in_flight = in_flight.enter();
    let socket = WebSocketHandle::from_axum(socket, protocol);
    let borrowed = HandlerReference(Arc::into_raw(Arc::clone(&socket)));
    let promise = unsafe { handler(borrowed.0) };
    if !promise.is_null() {
        let held = crate::promise::PromiseReference(promise as usize);
        let mut settled = std::pin::pin!(unsafe { crate::promise::settled(promise, ()) });
        let going_away = async move { going_away.wait_for(|going_away| *going_away).await.is_ok() };
        let state = tokio::select! {
            state = &mut settled => state,
            true = going_away => {
                socket.go_away().await;
                settled.await
            }
        };
        if state != crate::TYRUS_PROMISE_FULFILLED {
            let reason = unsafe {
                let message = crate::tyrus_promise_error_message(held.0 as *mut TyrusPromise);
                if message.is_null() {
                    "the handler did not complete".to_owned()
                } else {
                    message.as_str().to_owned()
                }
            };
            eprintln!("error: WebSocket handler failed: {reason}");
        }
        drop(held);
    }
    socket.close().await;
    drop(borrowed);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_router_ws(
    router: *mut HttpRouter,
    path: TyrusString,
    handler: Option<WebSocketHandler>,
    origins: *const TyrusStringArray,
    protocols: *const TyrusStringArray,
    max_message_bytes: i64,
    pre_handlers: *const crate::types::HttpPreHandler,
    pre_handlers_len: usize,
    rate_limit_max: i64,
    rate_limit_window_millis: i64,
    rate_limit_global_max: i64,
    rate_limit_ban: i64,
    rate_limit_ban_window_millis: i64,
) -> i32 {
    let (Some(router), Some(handler), Ok(path)) =
        (unsafe { router.as_mut() }, handler, text(&path))
    else {
        crate::http::set_http_error(
            "ERR_HTTP_WS_ROUTE_CONFIG",
            "router, handler, or path is invalid",
        );
        return -1;
    };
    if !path.starts_with('/') {
        crate::http::set_http_error(
            "ERR_HTTP_WS_ROUTE_CONFIG",
            "WebSocket path must start with `/`",
        );
        return -1;
    }
    let (Some(origins), Some(protocols)) = (unsafe { string_list(origins, "origins") }, unsafe {
        string_list(protocols, "subprotocols")
    }) else {
        return -1;
    };
    let route = Arc::new(WebSocketRoute {
        handler,
        origins,
        protocols,
        max_message_bytes: usize::try_from(max_message_bytes)
            .ok()
            .filter(|bytes| *bytes > 0)
            .unwrap_or(DEFAULT_MAX_MESSAGE_BYTES),
        config: Arc::clone(&router.config),
        pre_handlers: if pre_handlers.is_null() || pre_handlers_len == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(pre_handlers, pre_handlers_len) }.to_vec()
        },
        route_params: crate::http::RouteParams::parse(&path),
        rate_limiter: (rate_limit_max >= 0 && rate_limit_window_millis >= 0).then(|| {
            crate::http::RateLimiter::new_with_ban(
                rate_limit_max,
                rate_limit_window_millis,
                rate_limit_global_max,
                rate_limit_ban,
                rate_limit_ban_window_millis,
            )
        }),
    });
    let current = std::mem::take(&mut router.router);
    router.router = current.route(
        &crate::http::axum_route_path(&path),
        websocket_route_service(route),
    );
    0
}

fn client_tls_config() -> Result<Arc<rustls::ClientConfig>, WebSocketError> {
    static CONFIG: std::sync::OnceLock<Option<Arc<rustls::ClientConfig>>> =
        std::sync::OnceLock::new();
    CONFIG
        .get_or_init(|| {
            use rustls_platform_verifier::BuilderVerifierExt;

            let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
            rustls::ClientConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .ok()?
                .with_platform_verifier()
                .ok()
                .map(|builder| Arc::new(builder.with_no_client_auth()))
        })
        .clone()
        .ok_or(WebSocketError::TLS)
}

async fn connect(
    url: String,
    protocols: Vec<String>,
) -> Result<Arc<WebSocketHandle>, WebSocketError> {
    use tungstenite::client::IntoClientRequest;

    let mut request = url
        .as_str()
        .into_client_request()
        .map_err(|_| WebSocketError::URL)?;
    let secure = match request.uri().scheme_str() {
        Some(scheme) if scheme.eq_ignore_ascii_case("wss") => true,
        Some(scheme) if scheme.eq_ignore_ascii_case("ws") => false,
        _ => return Err(WebSocketError::URL),
    };
    let host = request
        .uri()
        .host()
        .ok_or(WebSocketError::URL)?
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_owned();
    let port = request
        .uri()
        .port_u16()
        .unwrap_or(if secure { 443 } else { 80 });
    if !protocols.is_empty() {
        let value =
            HeaderValue::from_str(&protocols.join(", ")).map_err(|_| WebSocketError::URL)?;
        request
            .headers_mut()
            .insert(header::SEC_WEBSOCKET_PROTOCOL, value);
    }
    let resolved = tokio::net::lookup_host((host.as_str(), port))
        .await
        .map_err(|_| WebSocketError::DNS)?
        .collect::<Vec<SocketAddr>>();
    if resolved.is_empty() {
        return Err(WebSocketError::DNS);
    }
    let permitted = match crate::fetch_policy::current_policy() {
        Some(policy) => resolved
            .into_iter()
            .filter(|address| policy.permits(&host, address.ip()))
            .collect::<Vec<_>>(),
        None => resolved,
    };
    if permitted.is_empty() {
        return Err(WebSocketError::BLOCKED);
    }
    let stream = tokio::net::TcpStream::connect(permitted.as_slice())
        .await
        .map_err(|_| WebSocketError::CONNECT)?;
    let connector = if secure {
        tokio_tungstenite::Connector::Rustls(client_tls_config()?)
    } else {
        tokio_tungstenite::Connector::Plain
    };
    let config = tungstenite::protocol::WebSocketConfig::default()
        .max_message_size(Some(DEFAULT_MAX_MESSAGE_BYTES))
        .max_frame_size(Some(DEFAULT_MAX_MESSAGE_BYTES));
    let (socket, response) = tokio_tungstenite::client_async_tls_with_config(
        request,
        stream,
        Some(config),
        Some(connector),
    )
    .await
    .map_err(|error| match error {
        tungstenite::Error::Tls(_) => WebSocketError::TLS,
        tungstenite::Error::Io(_) if secure => WebSocketError::TLS,
        tungstenite::Error::Io(_) => WebSocketError::CONNECT,
        _ => WebSocketError::HANDSHAKE,
    })?;
    let protocol = response
        .headers()
        .get(header::SEC_WEBSOCKET_PROTOCOL)
        .and_then(|protocol| protocol.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    Ok(WebSocketHandle::from_tungstenite(socket, protocol))
}

unsafe fn drop_websocket_handle(value: *mut c_void) {
    if value.is_null() {
        return;
    }
    unsafe { tyrus_websocket_free(value.cast::<WebSocketHandle>()) };
}

unsafe extern "C" fn drop_websocket_result(value: *mut c_void) {
    if value.is_null() {
        return;
    }
    let result = value.cast::<HttpResultFfi<*mut c_void>>();
    unsafe {
        if (*result).tag == HTTP_RESULT_TAG_OK {
            drop_websocket_handle(*(*result).payload.ok);
        }
        libc::free(value);
    }
}

fn fulfill_websocket_result(
    promise: *mut TyrusPromise,
    outcome: Result<Arc<WebSocketHandle>, WebSocketError>,
) {
    let outcome = outcome.map_or_else(
        |error| Err(http_error(error.code, error.message)),
        |socket| Ok(Arc::into_raw(socket).cast_mut().cast::<c_void>()),
    );
    unsafe {
        fulfill_http_result(
            promise,
            outcome,
            drop_websocket_handle,
            drop_websocket_result,
            (
                "ERR_HTTP_WS_ALLOCATION",
                "the WebSocket result could not be allocated",
            ),
        )
    };
}

unsafe fn spawn_connect_operation(
    operation: impl std::future::Future<Output = Result<Arc<WebSocketHandle>, WebSocketError>>
    + Send
    + 'static,
) -> *mut TyrusPromise {
    let promise = crate::tyrus_promise_pending();
    unsafe { crate::tyrus_promise_retain(promise) };
    let promise_address = promise as usize;
    executor().spawn(async move {
        let outcome = operation.await;
        fulfill_websocket_result(promise_address as *mut TyrusPromise, outcome);
        unsafe { crate::tyrus_promise_free(promise_address as *mut TyrusPromise) };
    });
    promise
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_websocket_connect(
    url: TyrusString,
    protocols: *const TyrusStringArray,
) -> *mut TyrusPromise {
    let url = match text(&url) {
        Ok(url) => url,
        Err(_) => {
            let promise = crate::tyrus_promise_pending();
            fulfill_websocket_result(promise, Err(WebSocketError::URL));
            return promise;
        }
    };
    let protocols = match unsafe { string_list(protocols, "protocols") } {
        Some(protocols) => protocols,
        None => {
            let promise = crate::tyrus_promise_pending();
            fulfill_websocket_result(promise, Err(WebSocketError::URL));
            return promise;
        }
    };
    unsafe { spawn_connect_operation(connect(url, protocols)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;
    use std::time::Duration;

    #[test]
    fn invalid_websocket_url_is_fulfilled_as_http_error_result() {
        let url = TyrusString::literal("https://example.com/socket");
        let promise = unsafe { tyrus_http_websocket_connect(url.borrow(), std::ptr::null()) };
        unsafe { crate::wait_until_settled(promise) };
        assert_eq!(
            unsafe { crate::tyrus_promise_state(promise) },
            crate::TYRUS_PROMISE_FULFILLED
        );
        let result =
            unsafe { crate::tyrus_promise_await(promise).cast::<HttpResultFfi<*mut c_void>>() };
        assert_eq!(
            unsafe { (*result).tag },
            crate::http_client::HTTP_RESULT_TAG_ERR
        );
        assert_eq!(
            unsafe { (*(*result).payload.err).code.as_str() },
            "ERR_HTTP_WS_URL"
        );
        unsafe { crate::tyrus_promise_free(promise) };
    }

    #[test]
    fn send_on_missing_websocket_is_fulfilled_as_http_error_result() {
        let promise =
            unsafe { tyrus_websocket_send_text(std::ptr::null(), TyrusString::literal("payload")) };
        unsafe { crate::wait_until_settled(promise) };
        assert_eq!(
            unsafe { crate::tyrus_promise_state(promise) },
            crate::TYRUS_PROMISE_FULFILLED
        );
        let result = unsafe { crate::tyrus_promise_await(promise).cast::<HttpResultFfi<()>>() };
        assert_eq!(
            unsafe { (*result).tag },
            crate::http_client::HTTP_RESULT_TAG_ERR
        );
        assert_eq!(
            unsafe { (*(*result).payload.err).code.as_str() },
            "ERR_HTTP_WS_CLOSED"
        );
        unsafe { crate::tyrus_promise_free(promise) };
    }

    #[test]
    fn receive_on_missing_websocket_is_fulfilled_as_http_error_result() {
        let promise = unsafe { tyrus_websocket_receive(std::ptr::null()) };
        unsafe { crate::wait_until_settled(promise) };
        assert_eq!(
            unsafe { crate::tyrus_promise_state(promise) },
            crate::TYRUS_PROMISE_FULFILLED
        );
        let result = unsafe {
            crate::tyrus_promise_await(promise).cast::<HttpResultFfi<*mut TyrusWebSocketMessage>>()
        };
        assert_eq!(
            unsafe { (*result).tag },
            crate::http_client::HTTP_RESULT_TAG_ERR
        );
        assert_eq!(
            unsafe { (*(*result).payload.err).code.as_str() },
            "ERR_HTTP_WS_CLOSED"
        );
        unsafe { crate::tyrus_promise_free(promise) };
    }

    #[test]
    fn stream_on_missing_websocket_is_fulfilled_as_http_error_result() {
        let promise = unsafe { tyrus_websocket_stream_duplex(std::ptr::null()) };
        unsafe { crate::wait_until_settled(promise) };
        assert_eq!(
            unsafe { crate::tyrus_promise_state(promise) },
            crate::TYRUS_PROMISE_FULFILLED
        );
        let result =
            unsafe { crate::tyrus_promise_await(promise).cast::<HttpResultFfi<*mut c_void>>() };
        assert_eq!(
            unsafe { (*result).tag },
            crate::http_client::HTTP_RESULT_TAG_ERR
        );
        assert_eq!(
            unsafe { (*(*result).payload.err).code.as_str() },
            "ERR_HTTP_WS_CLOSED"
        );
        unsafe { crate::tyrus_promise_free(promise) };
    }

    unsafe extern "C" fn echo_handler(handle: *const WebSocketHandle) -> *mut TyrusPromise {
        let socket = unsafe { retained(handle) }.expect("the route passes a live socket");
        unsafe {
            spawn_operation(
                async move {
                    while let Some(message) = socket.receive().await? {
                        let reply = match message {
                            Incoming::Text(text) => Outgoing::Text(format!("echo {text}")),
                            Incoming::Binary(mut bytes) => {
                                bytes.reverse();
                                Outgoing::Binary(bytes)
                            }
                            _ => continue,
                        };
                        socket.send(reply).await?;
                    }
                    Ok(())
                },
                fulfill_void,
            )
        }
    }

    /// A test server, and the guard that stops it.
    ///
    /// `axum::serve` runs until the process ends, and nothing here ever asked it
    /// to stop, so every test that started one left the task, its connections
    /// and their buffers alive at exit, where LeakSanitizer counted them: 1.6 MB
    /// across the handful of tests that serve. Aborting on drop ends the task at
    /// the end of the test that started it, which is the scope it belongs to.
    struct TestServer {
        task: tokio::task::JoinHandle<()>,
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    async fn serve_router(router: axum::Router) -> Option<(SocketAddr, TestServer)> {
        let listener = match tokio::net::TcpListener::bind("127.0.0.1:0").await {
            Ok(listener) => listener,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                eprintln!(
                    "SKIP: runtime WebSocket tests require permission to bind a local TCP listener ({error})."
                );
                return None;
            }
            Err(error) => panic!("could not bind the runtime WebSocket test listener: {error}"),
        };
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });
        Some((address, TestServer { task }))
    }

    fn test_config() -> Arc<std::sync::RwLock<crate::types::HttpServerConfig>> {
        Arc::new(std::sync::RwLock::new(
            crate::types::HttpServerConfig::default(),
        ))
    }

    fn echo_route(origins: Vec<String>, protocols: Vec<String>, max: usize) -> axum::Router {
        echo_route_with(origins, protocols, max, test_config(), Vec::new())
    }

    #[tokio::test]
    async fn binary_duplex_preserves_message_boundaries_and_excludes_direct_io() {
        let sink = futures_util::sink::drain().sink_map_err(|never| match never {});
        let source = stream::iter(vec![
            Ok(Incoming::Binary(vec![1, 2])),
            Ok(Incoming::Binary(vec![3])),
            Ok(Incoming::Closed(None)),
        ]);
        let socket = WebSocketHandle::new(Box::pin(sink), Box::pin(source), String::new());
        socket.take_stream().unwrap();
        let (readable, producer) = crate::stream::byte_readable_stream(16);
        let (writable, consumer) = crate::stream::byte_writable_stream(16);
        spawn_duplex_pumps(Arc::clone(&socket), producer, consumer);

        assert_eq!(
            unsafe { crate::stream::read_byte_stream(readable) }
                .await
                .unwrap(),
            Some(vec![1, 2])
        );
        assert_eq!(
            unsafe { crate::stream::read_byte_stream(readable) }
                .await
                .unwrap(),
            Some(vec![3])
        );
        assert_eq!(
            unsafe { crate::stream::read_byte_stream(readable) }
                .await
                .unwrap(),
            None
        );
        assert!(matches!(
            socket.receive().await,
            Err(WebSocketError::STREAMED)
        ));
        assert_eq!(
            socket.send(Outgoing::Binary(vec![9])).await,
            Err(WebSocketError::STREAMED)
        );
        unsafe {
            crate::stream::tyrus_readable_stream_free(readable);
            crate::stream::tyrus_writable_stream_free(writable);
        }
    }

    #[tokio::test]
    async fn binary_duplex_half_closes_writes_without_losing_remote_messages() {
        let sent = Arc::new(Mutex::new(Vec::<Outgoing>::new()));
        let sink = futures_util::sink::unfold(Arc::clone(&sent), |sent, frame| async move {
            sent.lock().await.push(frame);
            Ok::<_, Failure>(sent)
        });
        let source = stream::iter(vec![
            Ok(Incoming::Binary(vec![9, 8])),
            Ok(Incoming::Closed(Some(1000))),
        ]);
        let socket = WebSocketHandle::new(Box::pin(sink), Box::pin(source), String::new());
        socket.take_stream().unwrap();
        let (readable, producer) = crate::stream::byte_readable_stream(16);
        let (writable, consumer) = crate::stream::byte_writable_stream(16);
        spawn_duplex_pumps(Arc::clone(&socket), producer, consumer);

        unsafe {
            crate::stream::close_byte_writable_stream(writable)
                .await
                .unwrap();
        }
        assert_eq!(
            unsafe { crate::stream::read_byte_stream(readable).await.unwrap() },
            Some(vec![9, 8])
        );
        assert_eq!(
            unsafe { crate::stream::read_byte_stream(readable).await.unwrap() },
            None
        );
        let sent = sent.lock().await;
        assert!(
            sent.iter()
                .any(|frame| matches!(frame, Outgoing::Close(1000, reason) if reason.is_empty()))
        );
        unsafe {
            crate::stream::tyrus_readable_stream_free(readable);
            crate::stream::tyrus_writable_stream_free(writable);
        }
    }

    #[tokio::test]
    async fn binary_duplex_rejects_a_text_message_without_coercing_it() {
        let sink = futures_util::sink::drain().sink_map_err(|never| match never {});
        let source = stream::iter(vec![Ok(Incoming::Text("text".into()))]);
        let socket = WebSocketHandle::new(Box::pin(sink), Box::pin(source), String::new());
        socket.take_stream().unwrap();
        let (readable, producer) = crate::stream::byte_readable_stream(16);
        let (writable, consumer) = crate::stream::byte_writable_stream(16);
        spawn_duplex_pumps(socket, producer, consumer);

        assert_eq!(
            unsafe { crate::stream::read_byte_stream(readable) }
                .await
                .unwrap_err()
                .code,
            "ERR_STREAM_WS_TEXT_MESSAGE"
        );
        unsafe {
            crate::stream::tyrus_readable_stream_free(readable);
            crate::stream::tyrus_writable_stream_free(writable);
        }
    }

    fn echo_route_with(
        origins: Vec<String>,
        protocols: Vec<String>,
        max: usize,
        config: Arc<std::sync::RwLock<crate::types::HttpServerConfig>>,
        pre_handlers: Vec<crate::types::HttpPreHandler>,
    ) -> axum::Router {
        let route = Arc::new(WebSocketRoute {
            handler: echo_handler,
            origins,
            protocols,
            max_message_bytes: max,
            config,
            pre_handlers,
            route_params: crate::http::RouteParams::parse("/echo"),
            rate_limiter: None,
        });
        axum::Router::new().route("/echo", websocket_route_service(route))
    }

    #[tokio::test]
    async fn a_route_and_a_client_exchange_whole_text_and_binary_messages() {
        let Some((address, _server)) =
            serve_router(echo_route(Vec::new(), vec!["chat".into()], 1024)).await
        else {
            return;
        };
        let socket = connect(format!("ws://{address}/echo"), vec!["chat".into()])
            .await
            .unwrap();
        assert_eq!(socket.protocol, "chat");

        socket.send(Outgoing::Text("hello".into())).await.unwrap();
        match socket.receive().await.unwrap() {
            Some(Incoming::Text(text)) => assert_eq!(text, "echo hello"),
            _ => panic!("expected a text message"),
        }
        socket.send(Outgoing::Binary(vec![0, 1, 2])).await.unwrap();
        match socket.receive().await.unwrap() {
            Some(Incoming::Binary(bytes)) => assert_eq!(bytes, vec![2, 1, 0]),
            _ => panic!("expected a binary message"),
        }

        socket.close().await;
        assert_eq!(
            socket.send(Outgoing::Text("late".into())).await,
            Err(WebSocketError::CLOSED)
        );
        assert_eq!(socket.receive().await.err(), Some(WebSocketError::CLOSED));
    }

    #[tokio::test]
    async fn a_server_that_closes_resolves_the_client_receive_with_no_message() {
        unsafe extern "C" fn closing_handler(_handle: *const WebSocketHandle) -> *mut TyrusPromise {
            let promise = crate::tyrus_promise_pending();
            unsafe { crate::tyrus_promise_fulfill(promise, std::ptr::null_mut(), None) };
            promise
        }
        let route = Arc::new(WebSocketRoute {
            handler: closing_handler,
            origins: Vec::new(),
            protocols: Vec::new(),
            max_message_bytes: 1024,
            config: test_config(),
            pre_handlers: Vec::new(),
            route_params: crate::http::RouteParams::parse("/bye"),
            rate_limiter: None,
        });
        let router = axum::Router::new().route("/bye", websocket_route_service(route));
        let Some((address, _server)) = serve_router(router).await else {
            return;
        };
        let socket = connect(format!("ws://{address}/bye"), Vec::new())
            .await
            .unwrap();
        let received = tokio::time::timeout(Duration::from_secs(5), socket.receive())
            .await
            .expect("the close reaches the client");
        assert!(matches!(received, Ok(None)));
        assert_eq!(socket.peer_close_code(), Some(1000));
    }

    #[tokio::test]
    async fn a_message_above_the_route_limit_fails_the_server_receive() {
        let Some((address, _server)) = serve_router(echo_route(Vec::new(), Vec::new(), 8)).await
        else {
            return;
        };
        let socket = connect(format!("ws://{address}/echo"), Vec::new())
            .await
            .unwrap();
        socket.send(Outgoing::Binary(vec![7; 64])).await.unwrap();
        let received = tokio::time::timeout(Duration::from_secs(5), socket.receive())
            .await
            .expect("the connection ends");
        assert!(!matches!(received, Ok(Some(_))));
    }

    #[test]
    fn a_cross_site_origin_is_refused_unless_it_is_listed() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("app.example:8080"));
        assert!(origin_allowed(&[], &headers));

        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("http://app.example:8080"),
        );
        assert!(origin_allowed(&[], &headers));

        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://evil.example"),
        );
        assert!(!origin_allowed(&[], &headers));
        assert!(origin_allowed(
            &["https://evil.example".to_owned()],
            &headers
        ));
        assert!(!origin_allowed(
            &["https://other.example".to_owned()],
            &headers
        ));
    }

    #[tokio::test]
    async fn a_cross_site_upgrade_is_answered_with_forbidden() {
        let Some((address, _server)) = serve_router(echo_route(Vec::new(), Vec::new(), 1024)).await
        else {
            return;
        };
        let mut request = tungstenite::client::IntoClientRequest::into_client_request(
            format!("ws://{address}/echo").as_str(),
        )
        .unwrap();
        request.headers_mut().insert(
            header::ORIGIN,
            HeaderValue::from_static("https://evil.example"),
        );
        let stream = tokio::net::TcpStream::connect(address).await.unwrap();
        match tokio_tungstenite::client_async(request, stream).await {
            Err(tungstenite::Error::Http(response)) => {
                assert_eq!(response.status(), StatusCode::FORBIDDEN);
            }
            _ => panic!("the upgrade must be refused"),
        }
    }

    #[tokio::test]
    async fn a_client_url_with_another_scheme_is_refused() {
        assert_eq!(
            connect("http://127.0.0.1:1/".to_owned(), Vec::new())
                .await
                .err(),
            Some(WebSocketError::URL)
        );
    }

    #[tokio::test]
    async fn a_second_concurrent_receive_is_refused_as_busy() {
        let Some((address, _server)) = serve_router(echo_route(Vec::new(), Vec::new(), 1024)).await
        else {
            return;
        };
        let socket = connect(format!("ws://{address}/echo"), Vec::new())
            .await
            .unwrap();
        let waiting = Arc::clone(&socket);
        let first = tokio::spawn(async move { waiting.receive().await.map(|_| ()) });
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(socket.receive().await.err(), Some(WebSocketError::BUSY));
        socket.close().await;
        assert_eq!(first.await.unwrap(), Err(WebSocketError::CLOSED));
    }

    #[tokio::test]
    async fn a_peer_that_does_not_answer_a_heartbeat_is_observed_as_closed() {
        let sink = futures_util::sink::drain::<Outgoing>().sink_map_err(|never| match never {});
        let source = futures_util::stream::pending::<Result<Incoming, Failure>>();
        let socket = WebSocketHandle::new(Box::pin(sink), Box::pin(source), String::new());

        let received = tokio::time::timeout(HEARTBEAT_INTERVAL * 4, socket.receive())
            .await
            .expect("the missing pong closes the socket");
        assert!(matches!(received, Ok(None)));
    }

    #[tokio::test]
    async fn a_close_code_or_reason_a_program_may_not_send_is_refused() {
        let Some((address, _server)) = serve_router(echo_route(Vec::new(), Vec::new(), 1024)).await
        else {
            return;
        };
        let socket = connect(format!("ws://{address}/echo"), Vec::new())
            .await
            .unwrap();
        assert_eq!(
            socket.close_with(1001, "").await,
            Err(WebSocketError::CLOSE_CODE)
        );
        assert_eq!(
            socket.close_with(4000, &"x".repeat(124)).await,
            Err(WebSocketError::CLOSE_REASON)
        );
        assert_eq!(socket.close_with(4000, "done").await, Ok(()));
        assert_eq!(socket.receive().await.err(), Some(WebSocketError::CLOSED));
    }

    #[tokio::test]
    async fn a_server_shutdown_closes_open_sockets_and_waits_for_their_handlers() {
        let config = test_config();
        let Some((address, _server)) = serve_router(echo_route_with(
            Vec::new(),
            Vec::new(),
            1024,
            Arc::clone(&config),
            Vec::new(),
        ))
        .await
        else {
            return;
        };
        let socket = connect(format!("ws://{address}/echo"), Vec::new())
            .await
            .unwrap();
        socket.send(Outgoing::Text("ping".into())).await.unwrap();
        assert!(matches!(
            socket.receive().await,
            Ok(Some(Incoming::Text(_)))
        ));

        let (in_flight, going_away) = {
            let config = config.read().unwrap();
            (
                Arc::clone(&config.in_flight),
                Arc::clone(&config.websocket_going_away),
            )
        };
        going_away.send_replace(true);

        let received = tokio::time::timeout(Duration::from_secs(5), socket.receive())
            .await
            .expect("the shutdown close reaches the client");
        assert!(matches!(received, Ok(None)));
        tokio::time::timeout(Duration::from_secs(5), in_flight.wait_idle())
            .await
            .expect("the handler finishes once its socket goes away");
    }

    #[tokio::test]
    async fn many_sockets_on_one_route_keep_their_own_messages_and_end_with_their_clients() {
        let config = test_config();
        let Some((address, _server)) = serve_router(echo_route_with(
            Vec::new(),
            Vec::new(),
            1024,
            Arc::clone(&config),
            Vec::new(),
        ))
        .await
        else {
            return;
        };
        let mut sockets = Vec::new();
        for _ in 0..8 {
            sockets.push(
                connect(format!("ws://{address}/echo"), Vec::new())
                    .await
                    .unwrap(),
            );
        }
        for (index, socket) in sockets.iter().enumerate() {
            socket
                .send(Outgoing::Text(format!("client-{index}")))
                .await
                .unwrap();
        }
        for (index, socket) in sockets.iter().enumerate() {
            match tokio::time::timeout(Duration::from_secs(5), socket.receive())
                .await
                .expect("each client is answered")
            {
                Ok(Some(Incoming::Text(text))) => assert_eq!(text, format!("echo client-{index}")),
                _ => panic!("client {index} did not receive its own text message"),
            }
        }

        for socket in &sockets {
            socket.close().await;
        }
        let in_flight = Arc::clone(&config.read().unwrap().in_flight);
        tokio::time::timeout(Duration::from_secs(5), in_flight.wait_idle())
            .await
            .expect("every handler ends when its client leaves");
    }

    #[tokio::test]
    async fn a_send_waits_for_a_peer_that_is_not_taking_messages() {
        let (accepted, mut taken) = mpsc::channel::<Outgoing>(1);
        let sink = futures_util::sink::unfold(accepted, |accepted, message: Outgoing| async move {
            accepted.send(message).await.map_err(|_| Failure::Closed)?;
            Ok::<_, Failure>(accepted)
        });
        let socket = WebSocketHandle::new(
            Box::pin(sink),
            Box::pin(stream::pending::<Result<Incoming, Failure>>()),
            String::new(),
        );

        socket.send(Outgoing::Text("first".into())).await.unwrap();
        let mut waiting = tokio::spawn({
            let socket = Arc::clone(&socket);
            async move { socket.send(Outgoing::Text("second".into())).await }
        });
        assert!(
            tokio::time::timeout(Duration::from_millis(100), &mut waiting)
                .await
                .is_err(),
            "the send resolved before the peer took the first message"
        );

        while let Some(message) = taken.recv().await {
            if matches!(message, Outgoing::Text(_)) {
                break;
            }
        }
        tokio::time::timeout(Duration::from_secs(5), waiting)
            .await
            .expect("the send finishes once the peer takes the message")
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn a_large_binary_message_below_the_limit_survives_the_round_trip() {
        let Some((address, _server)) =
            serve_router(echo_route(Vec::new(), Vec::new(), 2 * 1024 * 1024)).await
        else {
            return;
        };
        let socket = connect(format!("ws://{address}/echo"), Vec::new())
            .await
            .unwrap();
        let payload: Vec<u8> = (0..512 * 1024).map(|index| (index % 251) as u8).collect();
        socket
            .send(Outgoing::Binary(payload.clone()))
            .await
            .unwrap();
        let mut expected = payload;
        expected.reverse();
        match tokio::time::timeout(Duration::from_secs(10), socket.receive())
            .await
            .expect("the echo arrives")
        {
            Ok(Some(Incoming::Binary(bytes))) => {
                assert_eq!(bytes.len(), expected.len());
                assert_eq!(bytes, expected);
            }
            _ => panic!("the binary echo did not arrive whole"),
        }
    }

    async fn upgrade_status(
        address: SocketAddr,
        header: Option<(&'static str, &'static str)>,
    ) -> Result<(), StatusCode> {
        let mut request = tungstenite::client::IntoClientRequest::into_client_request(
            format!("ws://{address}/echo").as_str(),
        )
        .unwrap();
        if let Some((name, value)) = header {
            request
                .headers_mut()
                .insert(name, HeaderValue::from_static(value));
        }
        let stream = tokio::net::TcpStream::connect(address).await.unwrap();
        match tokio_tungstenite::client_async(request, stream).await {
            Ok(_) => Ok(()),
            Err(tungstenite::Error::Http(response)) => Err(response.status()),
            Err(other) => panic!("unexpected handshake failure: {other}"),
        }
    }

    unsafe extern "C" fn require_authorization(
        request: *const crate::types::HttpRequest,
        response: *mut crate::types::HttpResponse,
    ) -> i32 {
        let headers = unsafe { (*request).headers.as_str() };
        if headers.contains("authorization") {
            return 0;
        }
        unsafe {
            (*response).status = 401;
            (*response).body =
                crate::types::HttpBody::from_text(TyrusString::literal("unauthorized"));
        }
        1
    }

    #[tokio::test]
    async fn a_pre_handler_refuses_an_upgrade_before_it_is_accepted() {
        let Some((address, _server)) = serve_router(echo_route_with(
            Vec::new(),
            Vec::new(),
            1024,
            test_config(),
            vec![require_authorization],
        ))
        .await
        else {
            return;
        };
        assert_eq!(
            upgrade_status(address, None).await,
            Err(StatusCode::UNAUTHORIZED)
        );
        assert_eq!(
            upgrade_status(address, Some(("authorization", "Bearer token"))).await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn the_server_rate_limit_counts_websocket_upgrades() {
        let config = test_config();
        config.write().unwrap().rate_limit =
            Some(crate::http::RateLimiter::new_with_global(1, 60_000, -1));
        let Some((address, _server)) = serve_router(echo_route_with(
            Vec::new(),
            Vec::new(),
            1024,
            config,
            Vec::new(),
        ))
        .await
        else {
            return;
        };
        assert_eq!(upgrade_status(address, None).await, Ok(()));
        assert_eq!(
            upgrade_status(address, None).await,
            Err(StatusCode::TOO_MANY_REQUESTS)
        );
    }

    #[tokio::test]
    async fn a_route_rate_limit_replaces_the_server_one_for_websocket_upgrades() {
        let route = Arc::new(WebSocketRoute {
            handler: echo_handler,
            origins: Vec::new(),
            protocols: Vec::new(),
            max_message_bytes: 1024,
            config: test_config(),
            pre_handlers: Vec::new(),
            route_params: crate::http::RouteParams::parse("/echo"),
            rate_limiter: Some(crate::http::RateLimiter::new_with_global(1, 60_000, -1)),
        });
        let Some((address, _server)) =
            serve_router(axum::Router::new().route("/echo", websocket_route_service(route))).await
        else {
            return;
        };
        assert_eq!(upgrade_status(address, None).await, Ok(()));
        assert_eq!(
            upgrade_status(address, None).await,
            Err(StatusCode::TOO_MANY_REQUESTS)
        );
    }
}
