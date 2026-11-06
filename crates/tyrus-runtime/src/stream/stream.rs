#![allow(
    dead_code,
    reason = "the shared core is wired into native adapters in the next streaming phase"
)]

use std::collections::VecDeque;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use tokio::sync::Notify;

use crate::TyrusPromise;
use crate::abi::TyrusArray;
use crate::abi::TyrusPromiseDrop;
use crate::executor::executor;
use crate::promise::{tyrus_promise_fulfill, tyrus_promise_pending, tyrus_promise_retain};
use tyrus_string::TyrusString;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StreamError {
    pub code: &'static str,
    pub message: &'static str,
}

impl StreamError {
    const CLOSED: Self = Self {
        code: "ERR_STREAM_CLOSED",
        message: "stream is closed",
    };
    const CHUNK_TOO_LARGE: Self = Self {
        code: "ERR_STREAM_CHUNK_TOO_LARGE",
        message: "stream chunk exceeds the buffer capacity",
    };
    const CONCURRENT_OPERATION: Self = Self {
        code: "ERR_STREAM_CONCURRENT_OPERATION",
        message: "another operation is already pending on this stream side",
    };
    const CANCELLED: Self = Self {
        code: "ERR_STREAM_CANCELLED",
        message: "stream operation was cancelled",
    };
    pub(crate) const SOURCE_FAILED: Self = Self {
        code: "ERR_STREAM_SOURCE_FAILED",
        message: "stream source failed",
    };
    pub(crate) const DESTINATION_FAILED: Self = Self {
        code: "ERR_STREAM_DESTINATION_FAILED",
        message: "stream destination failed",
    };
    const ALLOCATION_FAILED: Self = Self {
        code: "ERR_STREAM_ALLOCATION_FAILED",
        message: "failed to allocate stream chunk",
    };
}

struct StreamPayload {
    value: *mut c_void,
    drop_value: TyrusPromiseDrop,
    bytes: usize,
}

unsafe impl Send for StreamPayload {}

impl Drop for StreamPayload {
    fn drop(&mut self) {
        if !self.value.is_null()
            && let Some(drop_value) = self.drop_value
        {
            unsafe { drop_value(self.value) };
        }
    }
}

struct Chunk<T> {
    value: T,
    bytes: usize,
}

struct StreamState<T> {
    queue: VecDeque<Chunk<T>>,
    buffered_bytes: usize,
    capacity_bytes: usize,
    writer_closed: bool,
    reader_closed: bool,
    failure: Option<StreamError>,
    adapter_pending: bool,
}

struct SharedStream<T> {
    state: Mutex<StreamState<T>>,
    readable: Notify,
    writable: Notify,
}

impl<T> SharedStream<T> {
    fn lock(&self) -> MutexGuard<'_, StreamState<T>> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn close_writer(&self) {
        let mut state = self.lock();
        if !state.writer_closed {
            state.writer_closed = true;
            drop(state);
            self.readable.notify_waiters();
        }
    }

    fn close_reader(&self) {
        let mut state = self.lock();
        if !state.reader_closed {
            state.reader_closed = true;
            state.queue.clear();
            state.buffered_bytes = 0;
            drop(state);
            self.readable.notify_waiters();
            self.writable.notify_waiters();
        }
    }

    fn fail(&self, error: StreamError) {
        let mut state = self.lock();
        if state.failure.is_none() {
            state.failure = Some(error);
            state.queue.clear();
            state.buffered_bytes = 0;
            drop(state);
            self.readable.notify_waiters();
            self.writable.notify_waiters();
        }
    }

    async fn wait_for_reader_gone(&self) {
        loop {
            let notified = self.writable.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            {
                let state = self.lock();
                if state.reader_closed || state.failure.is_some() {
                    return;
                }
            }
            notified.await;
        }
    }

    async fn wait_for_adapter(&self) -> Result<(), StreamError> {
        loop {
            let notified = self.writable.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if let Some(result) = self.adapter_result() {
                return result;
            }
            notified.await;
        }
    }

    fn adapter_result(&self) -> Option<Result<(), StreamError>> {
        let state = self.lock();
        if let Some(error) = state.failure {
            Some(Err(error))
        } else if !state.adapter_pending {
            Some(Ok(()))
        } else {
            None
        }
    }

    fn failure(&self) -> Option<StreamError> {
        self.lock().failure
    }
}

pub(crate) struct ReadableStreamHandle<T> {
    shared: Arc<SharedStream<T>>,
}

pub(crate) struct WritableStreamHandle<T> {
    shared: Arc<SharedStream<T>>,
}

pub struct TyrusReadableStream {
    inner: ReadableStreamHandle<StreamPayload>,
    operation_pending: Arc<AtomicBool>,
}

pub struct TyrusWritableStream {
    inner: WritableStreamHandle<StreamPayload>,
    operation_pending: Arc<AtomicBool>,
}

pub struct TyrusDuplexStream {
    readable: Arc<TyrusReadableStream>,
    writable: Arc<TyrusWritableStream>,
}

pub struct TyrusTransformStream {
    readable: Arc<TyrusReadableStream>,
    writable: Arc<TyrusWritableStream>,
}

#[repr(C)]
pub struct TyrusStreamPair {
    pub readable: *const TyrusReadableStream,
    pub writable: *const TyrusWritableStream,
}

struct OperationGuard {
    pending: Arc<AtomicBool>,
}

impl Drop for OperationGuard {
    fn drop(&mut self) {
        self.pending.store(false, Ordering::Release);
    }
}

struct CancelSignal {
    requested: AtomicBool,
    notify: Notify,
}

impl CancelSignal {
    fn new() -> Self {
        Self {
            requested: AtomicBool::new(false),
            notify: Notify::new(),
        }
    }

    async fn cancelled(&self) {
        loop {
            let notified = self.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.requested.load(Ordering::Acquire) {
                return;
            }
            notified.await;
        }
    }
}

unsafe extern "C" fn request_stream_cancel(context: *mut c_void) {
    let signal = unsafe { &*context.cast::<CancelSignal>() };
    signal.requested.store(true, Ordering::Release);
    signal.notify.notify_waiters();
}

unsafe extern "C" fn drop_stream_cancel(context: *mut c_void) {
    drop(unsafe { Arc::from_raw(context.cast::<CancelSignal>()) });
}

unsafe extern "C" fn drop_optional_stream_payload(payload: *mut c_void) {
    if payload.is_null() {
        return;
    }
    let optional = unsafe { Box::from_raw(payload.cast::<BoxedOptionalPayload>()) };
    if !optional.inner.is_null()
        && let Some(drop_value) = optional.inner_drop
    {
        unsafe { drop_value(optional.inner) };
    }
}

#[repr(C)]
struct BoxedOptionalPayload {
    inner: *mut c_void,
    inner_drop: TyrusPromiseDrop,
}

fn optional_payload(payload: Option<StreamPayload>) -> *mut c_void {
    let (inner, inner_drop) = match payload {
        Some(payload) => {
            let payload = std::mem::ManuallyDrop::new(payload);
            (payload.value, payload.drop_value)
        }
        None => (std::ptr::null_mut(), None),
    };
    Box::into_raw(Box::new(BoxedOptionalPayload { inner, inner_drop })).cast()
}

fn begin_operation(pending: &Arc<AtomicBool>) -> Result<OperationGuard, StreamError> {
    if pending.swap(true, Ordering::AcqRel) {
        Err(StreamError::CONCURRENT_OPERATION)
    } else {
        Ok(OperationGuard {
            pending: Arc::clone(pending),
        })
    }
}

unsafe fn retained_readable(stream: *const TyrusReadableStream) -> Arc<TyrusReadableStream> {
    let stream = unsafe { Arc::from_raw(stream) };
    let stream = std::mem::ManuallyDrop::new(stream);
    Arc::clone(&stream)
}

unsafe fn retained_writable(stream: *const TyrusWritableStream) -> Arc<TyrusWritableStream> {
    let stream = unsafe { Arc::from_raw(stream) };
    let stream = std::mem::ManuallyDrop::new(stream);
    Arc::clone(&stream)
}

unsafe fn retained_duplex(stream: *const TyrusDuplexStream) -> Arc<TyrusDuplexStream> {
    let stream = unsafe { Arc::from_raw(stream) };
    let stream = std::mem::ManuallyDrop::new(stream);
    Arc::clone(&stream)
}

unsafe fn retained_transform(stream: *const TyrusTransformStream) -> Arc<TyrusTransformStream> {
    let stream = unsafe { Arc::from_raw(stream) };
    let stream = std::mem::ManuallyDrop::new(stream);
    Arc::clone(&stream)
}

fn reject(promise: *mut TyrusPromise, error: StreamError) {
    unsafe {
        crate::tyrus_promise_reject_pending_structured(
            promise,
            TyrusString::literal(error.code),
            TyrusString::literal(error.message),
        );
    }
}

fn rejected_promise(error: StreamError) -> *mut TyrusPromise {
    let promise = tyrus_promise_pending();
    reject(promise, error);
    promise
}

fn cancellable_promise() -> (*mut TyrusPromise, Arc<CancelSignal>) {
    let promise = tyrus_promise_pending();
    unsafe { tyrus_promise_retain(promise) };
    let signal = Arc::new(CancelSignal::new());
    unsafe {
        crate::tyrus_promise_on_cancel_request(
            promise,
            request_stream_cancel,
            Arc::into_raw(Arc::clone(&signal)).cast_mut().cast(),
            Some(drop_stream_cancel),
        );
    }
    (promise, signal)
}

pub(crate) struct StreamController<T> {
    shared: Arc<SharedStream<T>>,
}

pub(crate) fn bounded_stream<T>(
    capacity_bytes: usize,
) -> (
    ReadableStreamHandle<T>,
    WritableStreamHandle<T>,
    StreamController<T>,
) {
    assert!(
        capacity_bytes > 0,
        "stream capacity must be greater than zero"
    );
    let shared = Arc::new(SharedStream {
        state: Mutex::new(StreamState {
            queue: VecDeque::new(),
            buffered_bytes: 0,
            capacity_bytes,
            writer_closed: false,
            reader_closed: false,
            failure: None,
            adapter_pending: false,
        }),
        readable: Notify::new(),
        writable: Notify::new(),
    });
    (
        ReadableStreamHandle {
            shared: Arc::clone(&shared),
        },
        WritableStreamHandle {
            shared: Arc::clone(&shared),
        },
        StreamController { shared },
    )
}

impl<T> ReadableStreamHandle<T> {
    pub(crate) async fn read(&self) -> Result<Option<T>, StreamError> {
        loop {
            let notified = self.shared.readable.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if let Some(result) = self.try_read() {
                return result;
            }
            notified.await;
        }
    }

    fn try_read(&self) -> Option<Result<Option<T>, StreamError>> {
        let mut state = self.shared.lock();
        if let Some(error) = state.failure {
            return Some(Err(error));
        }
        if state.reader_closed {
            return Some(Err(StreamError::CLOSED));
        }
        if let Some(chunk) = state.queue.pop_front() {
            state.buffered_bytes -= chunk.bytes;
            drop(state);
            self.shared.writable.notify_waiters();
            return Some(Ok(Some(chunk.value)));
        }
        state.writer_closed.then_some(Ok(None))
    }

    pub(crate) fn close(&self) {
        self.shared.close_reader();
    }

    pub(crate) fn close_result(&self) -> Result<(), StreamError> {
        self.shared.close_reader();
        self.shared.failure().map_or(Ok(()), Err)
    }
}

impl<T> Drop for ReadableStreamHandle<T> {
    fn drop(&mut self) {
        self.shared.close_reader();
    }
}

impl<T> WritableStreamHandle<T> {
    pub(crate) async fn write(&self, value: T, bytes: usize) -> Result<(), StreamError> {
        let mut value = Some(value);
        loop {
            let notified = self.shared.writable.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            match self.try_write(
                value
                    .take()
                    .expect("stream write retains its pending value"),
                bytes,
            ) {
                Ok(()) => return Ok(()),
                Err((_returned, Some(error))) => return Err(error),
                Err((returned, None)) => value = Some(returned),
            }
            notified.await;
        }
    }

    fn try_write(&self, value: T, bytes: usize) -> Result<(), (T, Option<StreamError>)> {
        let mut state = self.shared.lock();
        if let Some(error) = state.failure {
            return Err((value, Some(error)));
        }
        if state.writer_closed || state.reader_closed {
            return Err((value, Some(StreamError::CLOSED)));
        }
        if bytes > state.capacity_bytes {
            return Err((value, Some(StreamError::CHUNK_TOO_LARGE)));
        }
        if !can_reserve(state.buffered_bytes, bytes, state.capacity_bytes) {
            return Err((value, None));
        }
        state.buffered_bytes += bytes;
        state.queue.push_back(Chunk { value, bytes });
        drop(state);
        self.shared.readable.notify_one();
        Ok(())
    }

    pub(crate) fn close(&self) {
        self.shared.close_writer();
    }
}

impl<T> Drop for WritableStreamHandle<T> {
    fn drop(&mut self) {
        self.shared.close_writer();
    }
}

impl<T> StreamController<T> {
    pub(crate) fn fail(&self, error: StreamError) {
        self.shared.fail(error);
    }

    fn require_completion(&self) {
        self.shared.lock().adapter_pending = true;
    }

    fn complete(&self) {
        self.shared.lock().adapter_pending = false;
        self.shared.writable.notify_waiters();
    }
}

pub(crate) struct ByteStreamProducer {
    writable: WritableStreamHandle<StreamPayload>,
    controller: StreamController<StreamPayload>,
}

pub(crate) struct ByteStreamConsumer {
    readable: ReadableStreamHandle<StreamPayload>,
    controller: StreamController<StreamPayload>,
    completed: Arc<AtomicBool>,
}

unsafe extern "C" fn drop_byte_array_payload(value: *mut c_void) {
    unsafe { crate::async_payload::tyrus_drop_async_copy_array(value) };
}

fn byte_payload(bytes: Vec<u8>) -> Result<StreamPayload, StreamError> {
    let length = bytes.len();
    let data = if bytes.is_empty() {
        std::ptr::null_mut()
    } else {
        let allocation = unsafe { libc::malloc(length) }.cast::<u8>();
        if allocation.is_null() {
            return Err(StreamError::ALLOCATION_FAILED);
        }
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), allocation, length) };
        allocation
    };
    Ok(StreamPayload {
        value: Box::into_raw(Box::new(TyrusArray {
            data,
            length,
            capacity: length,
        }))
        .cast(),
        drop_value: Some(drop_byte_array_payload),
        bytes: length,
    })
}

pub(crate) fn byte_readable_stream(
    capacity_bytes: usize,
) -> (*const TyrusReadableStream, ByteStreamProducer) {
    let (readable, writable, controller) = bounded_stream(capacity_bytes);
    let readable = Arc::into_raw(Arc::new(TyrusReadableStream {
        inner: readable,
        operation_pending: Arc::new(AtomicBool::new(false)),
    }));
    (
        readable,
        ByteStreamProducer {
            writable,
            controller,
        },
    )
}

pub(crate) fn byte_writable_stream(
    capacity_bytes: usize,
) -> (*const TyrusWritableStream, ByteStreamConsumer) {
    let (readable, writable, controller) = bounded_stream(capacity_bytes);
    controller.require_completion();
    let writable = Arc::into_raw(Arc::new(TyrusWritableStream {
        inner: writable,
        operation_pending: Arc::new(AtomicBool::new(false)),
    }));
    (
        writable,
        ByteStreamConsumer {
            readable,
            controller,
            completed: Arc::new(AtomicBool::new(false)),
        },
    )
}

impl ByteStreamProducer {
    pub(crate) async fn write(&self, bytes: Vec<u8>) -> Result<(), StreamError> {
        let length = bytes.len();
        self.writable.write(byte_payload(bytes)?, length).await
    }

    pub(crate) fn close(&self) {
        self.writable.close();
    }

    pub(crate) fn fail(&self) {
        self.controller.fail(StreamError::SOURCE_FAILED);
    }

    pub(crate) fn fail_with(&self, error: StreamError) {
        self.controller.fail(error);
    }

    pub(crate) async fn writable_reader_gone(&self) {
        self.writable.shared.wait_for_reader_gone().await;
    }
}

impl ByteStreamConsumer {
    pub(crate) async fn read(&self) -> Result<Option<Vec<u8>>, StreamError> {
        let Some(payload) = self.readable.read().await? else {
            return Ok(None);
        };
        let array = unsafe { &*payload.value.cast::<TyrusArray>() };
        let bytes = if array.length == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(array.data, array.length) }.to_vec()
        };
        Ok(Some(bytes))
    }

    pub(crate) fn close(&self) {
        self.completed.store(true, Ordering::Release);
        self.controller.complete();
    }

    pub(crate) fn fail(&self) {
        self.fail_with(StreamError::DESTINATION_FAILED);
    }

    pub(crate) fn fail_with(&self, error: StreamError) {
        self.completed.store(true, Ordering::Release);
        self.controller.fail(error);
    }
}

impl Drop for ByteStreamConsumer {
    fn drop(&mut self) {
        if !self.completed.swap(true, Ordering::AcqRel) {
            self.controller.fail(StreamError::DESTINATION_FAILED);
        }
    }
}

pub(crate) async unsafe fn read_byte_stream(
    stream: *const TyrusReadableStream,
) -> Result<Option<Vec<u8>>, StreamError> {
    let stream = unsafe { retained_readable(stream) };
    let Some(payload) = stream.inner.read().await? else {
        return Ok(None);
    };
    let array = unsafe { &*payload.value.cast::<TyrusArray>() };
    Ok(Some(if array.length == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(array.data, array.length) }.to_vec()
    }))
}

/// Takes one owned ABI reference and turns it into the Rust handle the HTTP
/// response adapter keeps while Axum polls its body.
pub(crate) unsafe fn take_owned_readable_stream(
    stream: *const TyrusReadableStream,
) -> Arc<TyrusReadableStream> {
    unsafe { Arc::from_raw(stream) }
}

/// Reads a byte chunk from an already-owned stream handle without crossing the
/// C ABI again. Keeping the `Arc` in the response body makes its lifetime and
/// `Send` contract explicit to Axum.
pub(crate) async fn read_owned_byte_stream(
    stream: &Arc<TyrusReadableStream>,
) -> Result<Option<Vec<u8>>, StreamError> {
    let Some(payload) = stream.inner.read().await? else {
        return Ok(None);
    };
    let array = unsafe { &*payload.value.cast::<TyrusArray>() };
    Ok(Some(if array.length == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(array.data, array.length) }.to_vec()
    }))
}

/// Cancels an owned reader without releasing its ABI reference. HTTP response
/// adapters use this when a peer disconnects: the source may still have an
/// application-owned handle, but its producer must learn that this transport
/// will not consume another byte.
pub(crate) fn close_owned_byte_stream(stream: &Arc<TyrusReadableStream>) {
    stream.inner.close();
}

pub(crate) async fn write_owned_byte_stream(
    stream: &Arc<TyrusWritableStream>,
    bytes: Vec<u8>,
    piece_bytes: usize,
) -> Result<(), StreamError> {
    if bytes.len() <= piece_bytes {
        let length = bytes.len();
        return stream.inner.write(byte_payload(bytes)?, length).await;
    }
    for piece in bytes.chunks(piece_bytes) {
        stream
            .inner
            .write(byte_payload(piece.to_vec())?, piece.len())
            .await?;
    }
    Ok(())
}

pub(crate) fn close_owned_writable_stream(stream: &Arc<TyrusWritableStream>) {
    stream.inner.close();
}

pub(crate) fn fail_owned_writable_stream(stream: &Arc<TyrusWritableStream>) {
    stream.inner.shared.fail(StreamError::SOURCE_FAILED);
}

pub(crate) async fn owned_writable_reader_gone(stream: &Arc<TyrusWritableStream>) {
    stream.inner.shared.wait_for_reader_gone().await;
}

impl TyrusTransformStream {
    /// Independent references to both sides, as the ABI accessors hand out.
    pub(crate) fn sides(&self) -> (Arc<TyrusReadableStream>, Arc<TyrusWritableStream>) {
        (Arc::clone(&self.readable), Arc::clone(&self.writable))
    }
}

/// A native transform between a byte input and a byte output. Output leaves
/// in pieces through `pull`, so a transform that expands its input (a
/// decompressor) is drained one piece at a time under the output's
/// backpressure instead of producing everything at once.
pub(crate) trait ByteTransform: Send + 'static {
    /// Accepts the next input chunk.
    fn push(&mut self, chunk: &[u8]) -> Result<(), StreamError>;
    /// The next piece of output, `None` when more input is needed or, after
    /// `finish`, when the output is complete.
    fn pull(&mut self) -> Result<Option<Vec<u8>>, StreamError>;
    /// Marks the end of the input.
    fn finish(&mut self) -> Result<(), StreamError>;
}

pub(crate) fn byte_transform_stream(
    capacity_bytes: usize,
    transform: Box<dyn ByteTransform>,
) -> Arc<TyrusTransformStream> {
    let (writable, consumer) = byte_writable_stream(capacity_bytes);
    let (readable, producer) = byte_readable_stream(capacity_bytes);
    executor().spawn(pump_byte_transform(
        consumer,
        producer,
        transform,
        capacity_bytes,
    ));
    // Both adapters return one owned ABI reference, which the composite adopts.
    Arc::new(TyrusTransformStream {
        readable: unsafe { Arc::from_raw(readable) },
        writable: unsafe { Arc::from_raw(writable) },
    })
}

/// A transform whose input and output fail with `error` as soon as they are
/// used, for a stream whose configuration was already invalid.
pub(crate) fn failed_byte_transform_stream(
    capacity_bytes: usize,
    error: StreamError,
) -> Arc<TyrusTransformStream> {
    let (writable, consumer) = byte_writable_stream(capacity_bytes);
    let (readable, producer) = byte_readable_stream(capacity_bytes);
    producer.fail_with(error);
    consumer.fail_with(error);
    Arc::new(TyrusTransformStream {
        readable: unsafe { Arc::from_raw(readable) },
        writable: unsafe { Arc::from_raw(writable) },
    })
}

async fn write_byte_pieces(
    producer: &ByteStreamProducer,
    bytes: Vec<u8>,
    piece_bytes: usize,
) -> Result<(), StreamError> {
    if bytes.len() <= piece_bytes {
        return producer.write(bytes).await;
    }
    for piece in bytes.chunks(piece_bytes) {
        producer.write(piece.to_vec()).await?;
    }
    Ok(())
}

/// Runs one transform step on the blocking pool: compression is CPU-bound and
/// must not hold the executor threads that poll the program's async code.
async fn blocking_step<R: Send + 'static>(
    transform: Box<dyn ByteTransform>,
    step: impl FnOnce(&mut dyn ByteTransform) -> R + Send + 'static,
) -> Option<(Box<dyn ByteTransform>, R)> {
    tokio::task::spawn_blocking(move || {
        let mut transform = transform;
        let result = step(transform.as_mut());
        (transform, result)
    })
    .await
    .ok()
}

/// Writes every piece the transform has ready. `Err` carries the error both
/// sides should fail with; `Ok(None)` means the output reader is gone.
async fn drain_byte_transform(
    mut transform: Box<dyn ByteTransform>,
    producer: &ByteStreamProducer,
    piece_bytes: usize,
) -> Result<Option<Box<dyn ByteTransform>>, StreamError> {
    loop {
        let Some((returned, piece)) = blocking_step(transform, |transform| transform.pull()).await
        else {
            return Err(StreamError::SOURCE_FAILED);
        };
        transform = returned;
        let Some(piece) = piece? else {
            return Ok(Some(transform));
        };
        if !piece.is_empty()
            && write_byte_pieces(producer, piece, piece_bytes)
                .await
                .is_err()
        {
            return Ok(None);
        }
    }
}

async fn pump_byte_transform(
    consumer: ByteStreamConsumer,
    producer: ByteStreamProducer,
    mut transform: Box<dyn ByteTransform>,
    piece_bytes: usize,
) {
    let fail = |error: StreamError| {
        producer.fail_with(error);
        consumer.fail_with(error);
    };
    loop {
        let next = tokio::select! {
            next = consumer.read() => next,
            () = producer.writable.shared.wait_for_reader_gone() => {
                consumer.fail();
                return;
            }
        };
        let finishing = match next {
            Ok(Some(chunk)) => {
                let Some((returned, pushed)) =
                    blocking_step(transform, move |transform| transform.push(&chunk)).await
                else {
                    return fail(StreamError::SOURCE_FAILED);
                };
                transform = returned;
                if let Err(error) = pushed {
                    return fail(error);
                }
                false
            }
            Ok(None) => {
                let Some((returned, finished)) =
                    blocking_step(transform, |transform| transform.finish()).await
                else {
                    return fail(StreamError::SOURCE_FAILED);
                };
                transform = returned;
                if let Err(error) = finished {
                    return fail(error);
                }
                true
            }
            Err(_) => {
                producer.fail();
                return;
            }
        };
        match drain_byte_transform(transform, &producer, piece_bytes).await {
            Ok(Some(returned)) => transform = returned,
            Ok(None) => {
                consumer.fail();
                return;
            }
            Err(error) => return fail(error),
        }
        if finishing {
            producer.close();
            consumer.close();
            return;
        }
    }
}

pub(crate) async unsafe fn write_byte_stream(
    stream: *const TyrusWritableStream,
    bytes: Vec<u8>,
) -> Result<(), StreamError> {
    let stream = unsafe { retained_writable(stream) };
    let length = bytes.len();
    stream.inner.write(byte_payload(bytes)?, length).await
}

pub(crate) async unsafe fn close_byte_writable_stream(
    stream: *const TyrusWritableStream,
) -> Result<(), StreamError> {
    let stream = unsafe { retained_writable(stream) };
    stream.inner.close();
    stream.inner.shared.wait_for_adapter().await
}

fn can_reserve(buffered: usize, requested: usize, capacity: usize) -> bool {
    requested <= capacity.saturating_sub(buffered)
}

fn close_side_once(closed: &mut bool) -> bool {
    if *closed {
        false
    } else {
        *closed = true;
        true
    }
}

fn record_first_failure(slot: &mut u8, error: u8) -> bool {
    if *slot != 0 {
        false
    } else {
        *slot = error;
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_stream_new(capacity_bytes: usize, out: *mut TyrusStreamPair) {
    let (readable, writable, _controller) = bounded_stream(capacity_bytes.max(1));
    unsafe {
        *out = TyrusStreamPair {
            readable: Arc::into_raw(Arc::new(TyrusReadableStream {
                inner: readable,
                operation_pending: Arc::new(AtomicBool::new(false)),
            })),
            writable: Arc::into_raw(Arc::new(TyrusWritableStream {
                inner: writable,
                operation_pending: Arc::new(AtomicBool::new(false)),
            })),
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_readable_stream_retain(
    stream: *const TyrusReadableStream,
) -> *const TyrusReadableStream {
    unsafe {
        Arc::increment_strong_count(stream);
        stream
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_readable_stream_free(stream: *const TyrusReadableStream) {
    if !stream.is_null() {
        drop(unsafe { Arc::from_raw(stream) });
    }
}

/// Reports whether an ABI-readable stream has a retained owner other than the
/// caller's reference. Adapters use this before releasing their own reference
/// to decide whether closing the adapter must also stop its underlying I/O.
pub(crate) unsafe fn readable_stream_has_other_owner(stream: *const TyrusReadableStream) -> bool {
    if stream.is_null() {
        return false;
    }
    let stream = unsafe { Arc::from_raw(stream) };
    let retained = Arc::strong_count(&stream) > 1;
    std::mem::forget(stream);
    retained
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_writable_stream_retain(
    stream: *const TyrusWritableStream,
) -> *const TyrusWritableStream {
    unsafe {
        Arc::increment_strong_count(stream);
        stream
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_writable_stream_free(stream: *const TyrusWritableStream) {
    if !stream.is_null() {
        drop(unsafe { Arc::from_raw(stream) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_duplex_stream_new(
    readable: *const TyrusReadableStream,
    writable: *const TyrusWritableStream,
) -> *const TyrusDuplexStream {
    Arc::into_raw(Arc::new(TyrusDuplexStream {
        readable: unsafe { retained_readable(readable) },
        writable: unsafe { retained_writable(writable) },
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_duplex_stream_retain(
    stream: *const TyrusDuplexStream,
) -> *const TyrusDuplexStream {
    unsafe {
        Arc::increment_strong_count(stream);
        stream
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_duplex_stream_free(stream: *const TyrusDuplexStream) {
    if !stream.is_null() {
        drop(unsafe { Arc::from_raw(stream) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_duplex_stream_readable(
    stream: *const TyrusDuplexStream,
) -> *const TyrusReadableStream {
    Arc::into_raw(Arc::clone(&unsafe { &*stream }.readable))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_duplex_stream_writable(
    stream: *const TyrusDuplexStream,
) -> *const TyrusWritableStream {
    Arc::into_raw(Arc::clone(&unsafe { &*stream }.writable))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_duplex_stream_close(
    stream: *const TyrusDuplexStream,
) -> *mut TyrusPromise {
    let stream = unsafe { &*stream };
    close_composite_stream(Arc::clone(&stream.readable), Arc::clone(&stream.writable))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_transform_stream_new(
    readable: *const TyrusReadableStream,
    writable: *const TyrusWritableStream,
) -> *const TyrusTransformStream {
    Arc::into_raw(Arc::new(TyrusTransformStream {
        readable: unsafe { retained_readable(readable) },
        writable: unsafe { retained_writable(writable) },
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_transform_stream_retain(
    stream: *const TyrusTransformStream,
) -> *const TyrusTransformStream {
    unsafe {
        Arc::increment_strong_count(stream);
        stream
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_transform_stream_free(stream: *const TyrusTransformStream) {
    if !stream.is_null() {
        drop(unsafe { Arc::from_raw(stream) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_transform_stream_readable(
    stream: *const TyrusTransformStream,
) -> *const TyrusReadableStream {
    Arc::into_raw(Arc::clone(&unsafe { &*stream }.readable))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_transform_stream_writable(
    stream: *const TyrusTransformStream,
) -> *const TyrusWritableStream {
    Arc::into_raw(Arc::clone(&unsafe { &*stream }.writable))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_transform_stream_close(
    stream: *const TyrusTransformStream,
) -> *mut TyrusPromise {
    let stream = unsafe { &*stream };
    close_composite_stream(Arc::clone(&stream.readable), Arc::clone(&stream.writable))
}

fn close_composite_stream(
    readable: Arc<TyrusReadableStream>,
    writable: Arc<TyrusWritableStream>,
) -> *mut TyrusPromise {
    readable.inner.close();
    writable.inner.close();
    if let Some(error) = readable
        .inner
        .shared
        .failure()
        .or_else(|| writable.inner.shared.failure())
    {
        let promise = crate::tyrus_promise_pending();
        reject(promise, error);
        return promise;
    }
    let (promise, cancellation) = cancellable_promise();
    let promise_address = promise as usize;
    executor().spawn(async move {
        let outcome = tokio::select! {
            result = writable.inner.shared.wait_for_adapter() => Some(result),
            () = cancellation.cancelled() => None,
        };
        let promise = promise_address as *mut TyrusPromise;
        unsafe {
            match outcome {
                Some(Ok(())) | None => {
                    tyrus_promise_fulfill(promise, std::ptr::null_mut(), None);
                }
                Some(Err(error)) => reject(promise, error),
            }
            crate::tyrus_promise_free(promise);
        }
    });
    promise
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_readable_stream_read(
    stream: *const TyrusReadableStream,
) -> *mut TyrusPromise {
    let stream = unsafe { retained_readable(stream) };
    let operation = match begin_operation(&stream.operation_pending) {
        Ok(operation) => operation,
        Err(error) => return rejected_promise(error),
    };
    let (promise, cancellation) = cancellable_promise();
    let promise_address = promise as usize;
    executor().spawn(async move {
        let outcome = tokio::select! {
            value = stream.inner.read() => Some(value),
            () = cancellation.cancelled() => None,
        };
        drop(operation);
        let promise = promise_address as *mut TyrusPromise;
        unsafe {
            match outcome {
                Some(Ok(value)) => {
                    tyrus_promise_fulfill(
                        promise,
                        optional_payload(value),
                        Some(drop_optional_stream_payload),
                    );
                }
                Some(Err(error)) => reject(promise, error),
                None => {
                    tyrus_promise_fulfill(promise, std::ptr::null_mut(), None);
                }
            }
            crate::tyrus_promise_free(promise);
        }
    });
    promise
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_writable_stream_write(
    stream: *const TyrusWritableStream,
    value: *mut c_void,
    bytes: usize,
    drop_value: TyrusPromiseDrop,
) -> *mut TyrusPromise {
    let stream = unsafe { retained_writable(stream) };
    let operation = match begin_operation(&stream.operation_pending) {
        Ok(operation) => operation,
        Err(error) => return rejected_promise(error),
    };
    let (promise, cancellation) = cancellable_promise();
    let promise_address = promise as usize;
    let payload = StreamPayload {
        value,
        drop_value,
        bytes,
    };
    executor().spawn(async move {
        let outcome = tokio::select! {
            result = stream.inner.write(payload, bytes) => Some(result),
            () = cancellation.cancelled() => None,
        };
        drop(operation);
        let promise = promise_address as *mut TyrusPromise;
        unsafe {
            match outcome {
                Some(Ok(())) | None => {
                    tyrus_promise_fulfill(promise, std::ptr::null_mut(), None);
                }
                Some(Err(error)) => reject(promise, error),
            }
            crate::tyrus_promise_free(promise);
        }
    });
    promise
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_readable_stream_close(
    stream: *const TyrusReadableStream,
) -> *mut TyrusPromise {
    let promise = crate::tyrus_promise_pending();
    let result = unsafe { &*stream }.inner.close_result();
    match result {
        Ok(()) => unsafe {
            crate::tyrus_promise_fulfill(promise, std::ptr::null_mut(), None);
        },
        Err(error) => reject(promise, error),
    }
    promise
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_writable_stream_close(
    stream: *const TyrusWritableStream,
) -> *mut TyrusPromise {
    let stream = unsafe { retained_writable(stream) };
    stream.inner.close();
    let (promise, cancellation) = cancellable_promise();
    let promise_address = promise as usize;
    executor().spawn(async move {
        let outcome = tokio::select! {
            result = stream.inner.shared.wait_for_adapter() => Some(result),
            () = cancellation.cancelled() => None,
        };
        let promise = promise_address as *mut TyrusPromise;
        unsafe {
            match outcome {
                Some(Ok(())) | None => {
                    tyrus_promise_fulfill(promise, std::ptr::null_mut(), None);
                }
                Some(Err(error)) => reject(promise, error),
            }
            crate::tyrus_promise_free(promise);
        }
    });
    promise
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_readable_stream_pipe_to(
    source: *const TyrusReadableStream,
    destination: *const TyrusWritableStream,
) -> *mut TyrusPromise {
    let source = unsafe { retained_readable(source) };
    let destination = unsafe { retained_writable(destination) };
    let operation = match begin_operation(&source.operation_pending) {
        Ok(source_operation) => match begin_operation(&destination.operation_pending) {
            Ok(destination_operation) => Ok((source_operation, destination_operation)),
            Err(error) => Err(error),
        },
        Err(error) => Err(error),
    };
    let operation = match operation {
        Ok(operation) => operation,
        Err(error) => return rejected_promise(error),
    };
    let (promise, cancellation) = cancellable_promise();
    let promise_address = promise as usize;
    executor().spawn(async move {
        let outcome = tokio::select! {
            result = async {
                loop {
                    match source.inner.read().await? {
                        Some(payload) => {
                            let bytes = payload_bytes(&payload);
                            destination.inner.write(payload, bytes).await?;
                        }
                        None => {
                            destination.inner.close();
                            return destination.inner.shared.wait_for_adapter().await;
                        }
                    }
                }
            } => Some(result),
            () = cancellation.cancelled() => None,
        };
        drop(operation);
        let promise = promise_address as *mut TyrusPromise;
        unsafe {
            match outcome {
                Some(Ok(())) => {
                    tyrus_promise_fulfill(promise, std::ptr::null_mut(), None);
                }
                None => {
                    source.inner.shared.fail(StreamError::CANCELLED);
                    destination.inner.shared.fail(StreamError::CANCELLED);
                    reject(promise, StreamError::CANCELLED);
                }
                Some(Err(error)) => {
                    if error != StreamError::CONCURRENT_OPERATION {
                        source.inner.shared.fail(error);
                        destination.inner.shared.fail(error);
                    }
                    reject(promise, error);
                }
            }
            crate::tyrus_promise_free(promise);
        }
    });
    promise
}

fn payload_bytes(payload: &StreamPayload) -> usize {
    payload.bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn promise_error_code(promise: *mut TyrusPromise) -> String {
        let code = unsafe { crate::tyrus_promise_error_code(promise) };
        let value = unsafe { code.as_str().to_owned() };
        unsafe { code.release() };
        value
    }
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    static ABI_DROPS: AtomicUsize = AtomicUsize::new(0);

    struct DropProbe(Arc<AtomicUsize>);

    unsafe extern "C" fn drop_i32(value: *mut c_void) {
        drop(unsafe { Box::from_raw(value.cast::<i32>()) });
    }

    unsafe extern "C" fn count_and_drop_i32(value: *mut c_void) {
        ABI_DROPS.fetch_add(1, Ordering::Relaxed);
        unsafe { drop_i32(value) };
    }

    unsafe fn new_abi_pair(capacity: usize) -> TyrusStreamPair {
        let mut pair = TyrusStreamPair {
            readable: std::ptr::null(),
            writable: std::ptr::null(),
        };
        unsafe { tyrus_stream_new(capacity, &mut pair) };
        pair
    }

    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[tokio::test]
    async fn write_waits_for_capacity_and_read_releases_it() {
        let (reader, writer, _controller) = bounded_stream(4);
        writer.write("first", 4).await.unwrap();

        let blocked = writer.write("second", 1);
        tokio::pin!(blocked);
        assert!(
            tokio::time::timeout(Duration::from_millis(10), &mut blocked)
                .await
                .is_err()
        );

        assert_eq!(reader.read().await.unwrap(), Some("first"));
        assert_eq!(blocked.await, Ok(()));
        assert_eq!(reader.read().await.unwrap(), Some("second"));
    }

    #[tokio::test]
    async fn closing_the_writer_drains_then_reports_end_of_stream() {
        let (reader, writer, _controller) = bounded_stream(8);
        writer.write(7, 1).await.unwrap();
        writer.close();
        writer.close();

        assert_eq!(reader.read().await.unwrap(), Some(7));
        assert_eq!(reader.read().await.unwrap(), None);
    }

    #[tokio::test]
    async fn dropping_the_reader_wakes_a_blocked_writer_with_a_stable_error() {
        let (reader, writer, _controller) = bounded_stream(1);
        writer.write(1, 1).await.unwrap();
        let blocked = writer.write(2, 1);
        tokio::pin!(blocked);
        assert!(
            tokio::time::timeout(Duration::from_millis(10), &mut blocked)
                .await
                .is_err()
        );

        drop(reader);
        assert_eq!(blocked.await.unwrap_err(), StreamError::CLOSED);
    }

    #[tokio::test]
    async fn failure_reaches_both_capabilities_and_discards_buffered_data() {
        let (reader, writer, controller) = bounded_stream(8);
        writer.write(1, 1).await.unwrap();
        let error = StreamError {
            code: "ERR_STREAM_SOURCE",
            message: "stream source failed",
        };
        controller.fail(error);

        assert_eq!(reader.read().await.unwrap_err(), error);
        assert_eq!(writer.write(2, 1).await.unwrap_err(), error);
    }

    #[tokio::test]
    async fn closing_a_failed_readable_preserves_its_structured_error() {
        let (reader, _writer, controller) = bounded_stream::<u8>(8);
        let error = StreamError {
            code: "ERR_STREAM_TEST_FAILURE",
            message: "test failure",
        };
        controller.fail(error);

        assert_eq!(reader.close_result(), Err(error));
        assert_eq!(reader.read().await, Err(error));
    }

    #[test]
    fn side_closure_is_idempotent_and_failure_slots_are_write_once() {
        let mut closed = false;
        assert!(close_side_once(&mut closed));
        assert!(!close_side_once(&mut closed));
        assert!(closed);

        let mut slot = 0;
        assert!(record_first_failure(&mut slot, 1));
        assert!(!record_first_failure(&mut slot, 2));
        assert_eq!(slot, 1);
    }

    #[tokio::test]
    async fn oversized_chunks_are_rejected_before_entering_the_queue() {
        let (reader, writer, _controller) = bounded_stream(4);
        assert_eq!(
            writer.write(vec![0; 5], 5).await.unwrap_err(),
            StreamError::CHUNK_TOO_LARGE
        );
        writer.close();
        assert_eq!(reader.read().await.unwrap(), None);
    }

    #[tokio::test]
    async fn cancelling_a_blocked_write_releases_its_unaccepted_value() {
        let (reader, writer, _controller) = bounded_stream(1);
        let dropped = Arc::new(AtomicUsize::new(0));
        writer
            .write(DropProbe(Arc::clone(&dropped)), 1)
            .await
            .unwrap();
        let mut blocked = Box::pin(writer.write(DropProbe(Arc::clone(&dropped)), 1));
        assert!(
            tokio::time::timeout(Duration::from_millis(10), &mut blocked)
                .await
                .is_err()
        );

        drop(blocked);
        assert_eq!(dropped.load(Ordering::Relaxed), 1);
        drop(reader);
        assert_eq!(dropped.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn abi_write_read_close_and_eof_preserve_payload_ownership() {
        let pair = unsafe { new_abi_pair(4) };
        let value = Box::into_raw(Box::new(42_i32)).cast();
        let write = unsafe { tyrus_writable_stream_write(pair.writable, value, 4, Some(drop_i32)) };
        unsafe { crate::wait_until_settled(write) };
        assert_eq!(
            unsafe { crate::tyrus_promise_state(write) },
            crate::TYRUS_PROMISE_FULFILLED
        );
        unsafe { crate::tyrus_promise_free(write) };

        let read = unsafe { tyrus_readable_stream_read(pair.readable) };
        unsafe { crate::wait_until_settled(read) };
        let optional = unsafe { crate::tyrus_promise_await(read).cast::<*mut c_void>() };
        assert!(!optional.is_null());
        assert_eq!(unsafe { *(*optional).cast::<i32>() }, 42);
        unsafe { crate::tyrus_promise_free(read) };

        let close = unsafe { tyrus_writable_stream_close(pair.writable) };
        unsafe { crate::tyrus_promise_free(close) };
        let eof = unsafe { tyrus_readable_stream_read(pair.readable) };
        unsafe { crate::wait_until_settled(eof) };
        let optional = unsafe { crate::tyrus_promise_await(eof).cast::<*mut c_void>() };
        assert!(!optional.is_null());
        assert!(unsafe { *optional }.is_null());
        unsafe {
            crate::tyrus_promise_free(eof);
            tyrus_readable_stream_free(pair.readable);
            tyrus_writable_stream_free(pair.writable);
        }
    }

    #[test]
    fn abi_rejects_oversized_and_concurrent_operations_with_stable_codes() {
        let pair = unsafe { new_abi_pair(1) };
        let oversized = unsafe {
            tyrus_writable_stream_write(
                pair.writable,
                Box::into_raw(Box::new(1_i32)).cast(),
                2,
                Some(drop_i32),
            )
        };
        unsafe { crate::wait_until_settled(oversized) };
        assert_eq!(promise_error_code(oversized), "ERR_STREAM_CHUNK_TOO_LARGE");
        unsafe { crate::tyrus_promise_free(oversized) };

        let first = unsafe { tyrus_readable_stream_read(pair.readable) };
        let second = unsafe { tyrus_readable_stream_read(pair.readable) };
        unsafe { crate::wait_until_settled(second) };
        assert_eq!(
            promise_error_code(second),
            "ERR_STREAM_CONCURRENT_OPERATION"
        );
        let close = unsafe { tyrus_writable_stream_close(pair.writable) };
        unsafe { crate::tyrus_promise_free(close) };
        unsafe { crate::wait_until_settled(first) };
        unsafe {
            crate::tyrus_promise_free(first);
            crate::tyrus_promise_free(second);
            tyrus_readable_stream_free(pair.readable);
            tyrus_writable_stream_free(pair.writable);
        }
    }

    #[test]
    fn cancelling_an_abi_write_releases_the_unaccepted_payload() {
        ABI_DROPS.store(0, Ordering::Relaxed);
        let pair = unsafe { new_abi_pair(1) };
        let first = unsafe {
            tyrus_writable_stream_write(
                pair.writable,
                Box::into_raw(Box::new(1_i32)).cast(),
                1,
                Some(count_and_drop_i32),
            )
        };
        unsafe { crate::wait_until_settled(first) };
        unsafe { crate::tyrus_promise_free(first) };

        let blocked = unsafe {
            tyrus_writable_stream_write(
                pair.writable,
                Box::into_raw(Box::new(2_i32)).cast(),
                1,
                Some(count_and_drop_i32),
            )
        };
        assert_eq!(
            unsafe {
                crate::tyrus_promise_request_cancel(
                    blocked,
                    TyrusString::literal("cancel blocked write"),
                )
            },
            1
        );
        unsafe { crate::wait_until_settled(blocked) };
        assert_eq!(
            unsafe { crate::tyrus_promise_state(blocked) },
            crate::TYRUS_PROMISE_CANCELLED
        );
        unsafe { crate::tyrus_promise_free(blocked) };
        assert_eq!(ABI_DROPS.load(Ordering::Relaxed), 1);

        unsafe {
            tyrus_readable_stream_free(pair.readable);
            tyrus_writable_stream_free(pair.writable);
        }
        assert_eq!(ABI_DROPS.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn cancelling_pipe_fails_both_endpoints_and_discards_queued_chunks() {
        let source = unsafe { new_abi_pair(4) };
        let destination = unsafe { new_abi_pair(4) };
        let pipe = unsafe { tyrus_readable_stream_pipe_to(source.readable, destination.writable) };
        assert_eq!(
            unsafe {
                crate::tyrus_promise_request_cancel(
                    pipe,
                    TyrusString::literal("cancel stream pipe"),
                )
            },
            1
        );
        unsafe { crate::wait_until_settled(pipe) };
        assert_eq!(
            unsafe { crate::tyrus_promise_state(pipe) },
            crate::TYRUS_PROMISE_CANCELLED
        );

        let write = unsafe {
            tyrus_writable_stream_write(
                source.writable,
                Box::into_raw(Box::new(1_i32)).cast(),
                4,
                Some(drop_i32),
            )
        };
        unsafe { crate::wait_until_settled(write) };
        assert_eq!(promise_error_code(write), "ERR_STREAM_CANCELLED");

        let read = unsafe { tyrus_readable_stream_read(destination.readable) };
        unsafe { crate::wait_until_settled(read) };
        assert_eq!(promise_error_code(read), "ERR_STREAM_CANCELLED");
        unsafe {
            crate::tyrus_promise_free(pipe);
            crate::tyrus_promise_free(write);
            crate::tyrus_promise_free(read);
            tyrus_readable_stream_free(source.readable);
            tyrus_writable_stream_free(source.writable);
            tyrus_readable_stream_free(destination.readable);
            tyrus_writable_stream_free(destination.writable);
        }
    }

    #[test]
    fn duplex_and_transform_compose_independently_owned_capabilities() {
        let pair = unsafe { new_abi_pair(4) };
        let duplex = unsafe { tyrus_duplex_stream_new(pair.readable, pair.writable) };
        unsafe {
            tyrus_readable_stream_free(pair.readable);
            tyrus_writable_stream_free(pair.writable);
        }
        let readable = unsafe { tyrus_duplex_stream_readable(duplex) };
        let writable = unsafe { tyrus_duplex_stream_writable(duplex) };
        let write = unsafe {
            tyrus_writable_stream_write(
                writable,
                Box::into_raw(Box::new(7_i32)).cast(),
                4,
                Some(drop_i32),
            )
        };
        unsafe { crate::wait_until_settled(write) };
        unsafe { crate::tyrus_promise_free(write) };
        let read = unsafe { tyrus_readable_stream_read(readable) };
        unsafe { crate::wait_until_settled(read) };
        let optional = unsafe { crate::tyrus_promise_await(read).cast::<*mut c_void>() };
        assert_eq!(unsafe { *(*optional).cast::<i32>() }, 7);
        unsafe { crate::tyrus_promise_free(read) };
        let close = unsafe { tyrus_duplex_stream_close(duplex) };
        unsafe {
            crate::tyrus_promise_free(close);
            tyrus_readable_stream_free(readable);
            tyrus_writable_stream_free(writable);
            tyrus_duplex_stream_free(duplex);
        }

        let pair = unsafe { new_abi_pair(4) };
        let transform = unsafe { tyrus_transform_stream_new(pair.readable, pair.writable) };
        let transformed_readable = unsafe { tyrus_transform_stream_readable(transform) };
        let transformed_writable = unsafe { tyrus_transform_stream_writable(transform) };
        assert!(!transformed_readable.is_null());
        assert!(!transformed_writable.is_null());
        let close = unsafe { tyrus_transform_stream_close(transform) };
        unsafe {
            crate::tyrus_promise_free(close);
            tyrus_readable_stream_free(transformed_readable);
            tyrus_writable_stream_free(transformed_writable);
            tyrus_transform_stream_free(transform);
            tyrus_readable_stream_free(pair.readable);
            tyrus_writable_stream_free(pair.writable);
        }
    }

    struct PairingTransform {
        held: Vec<u8>,
        ready: Vec<Vec<u8>>,
    }

    impl PairingTransform {
        fn new() -> Box<Self> {
            Box::new(Self {
                held: Vec::new(),
                ready: Vec::new(),
            })
        }
    }

    impl ByteTransform for PairingTransform {
        fn push(&mut self, chunk: &[u8]) -> Result<(), StreamError> {
            if self.held.is_empty() {
                self.held = chunk.to_vec();
            } else {
                let mut output = std::mem::take(&mut self.held);
                output.extend_from_slice(chunk);
                self.ready.push(output);
            }
            Ok(())
        }

        fn pull(&mut self) -> Result<Option<Vec<u8>>, StreamError> {
            Ok((!self.ready.is_empty()).then(|| self.ready.remove(0)))
        }

        fn finish(&mut self) -> Result<(), StreamError> {
            if !self.held.is_empty() {
                self.ready.push(std::mem::take(&mut self.held));
            }
            Ok(())
        }
    }

    const TRANSFORM_FAILED: StreamError = StreamError {
        code: "ERR_TEST_TRANSFORM_FAILED",
        message: "transform failed",
    };

    struct FailingTransform;

    impl ByteTransform for FailingTransform {
        fn push(&mut self, _chunk: &[u8]) -> Result<(), StreamError> {
            Err(TRANSFORM_FAILED)
        }

        fn pull(&mut self) -> Result<Option<Vec<u8>>, StreamError> {
            Ok(None)
        }

        fn finish(&mut self) -> Result<(), StreamError> {
            Ok(())
        }
    }

    async fn read_all_chunks(readable: &Arc<TyrusReadableStream>) -> Vec<Vec<u8>> {
        let mut chunks = Vec::new();
        while let Some(chunk) = read_owned_byte_stream(readable).await.unwrap() {
            chunks.push(chunk);
        }
        chunks
    }

    #[tokio::test]
    async fn a_byte_transform_changes_the_chunk_count_and_flushes_its_tail() {
        let transform = byte_transform_stream(64, PairingTransform::new());
        let (readable, writable) = transform.sides();
        drop(transform);
        let reader = tokio::spawn(async move { read_all_chunks(&readable).await });
        for chunk in [b"ab".to_vec(), b"cd".to_vec(), b"e".to_vec()] {
            write_owned_byte_stream(&writable, chunk, 64).await.unwrap();
        }
        close_owned_writable_stream(&writable);
        assert_eq!(writable.inner.shared.wait_for_adapter().await, Ok(()));
        assert_eq!(reader.await.unwrap(), vec![b"abcd".to_vec(), b"e".to_vec()]);
    }

    #[tokio::test]
    async fn a_byte_transform_splits_output_larger_than_its_capacity() {
        let transform = byte_transform_stream(8, PairingTransform::new());
        let (readable, writable) = transform.sides();
        drop(transform);
        let reader = tokio::spawn(async move { read_all_chunks(&readable).await });
        write_owned_byte_stream(&writable, vec![1; 6], 8)
            .await
            .unwrap();
        write_owned_byte_stream(&writable, vec![2; 6], 8)
            .await
            .unwrap();
        close_owned_writable_stream(&writable);
        let chunks = reader.await.unwrap();
        assert!(chunks.iter().all(|chunk| chunk.len() <= 8));
        assert_eq!(chunks.concat(), [vec![1; 6], vec![2; 6]].concat());
    }

    #[tokio::test]
    async fn closing_a_transform_output_fails_its_idle_input() {
        let transform = byte_transform_stream(64, PairingTransform::new());
        let (readable, writable) = transform.sides();
        drop(transform);
        close_owned_byte_stream(&readable);
        tokio::time::timeout(
            Duration::from_secs(5),
            owned_writable_reader_gone(&writable),
        )
        .await
        .expect("the input learns that the output reader is gone");
        assert!(
            write_owned_byte_stream(&writable, vec![1], 64)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn a_failing_byte_transform_fails_its_output_instead_of_ending_it() {
        let transform = byte_transform_stream(64, Box::new(FailingTransform));
        let (readable, writable) = transform.sides();
        drop(transform);
        write_owned_byte_stream(&writable, vec![1], 64)
            .await
            .unwrap();
        assert_eq!(
            read_owned_byte_stream(&readable).await,
            Err(TRANSFORM_FAILED),
            "the output fails with the transform's own error"
        );
        close_owned_writable_stream(&writable);
        assert_eq!(
            writable.inner.shared.wait_for_adapter().await,
            Err(TRANSFORM_FAILED),
            "closing the input reports the same error"
        );
    }

    #[tokio::test]
    async fn a_failed_transform_input_fails_its_output_instead_of_ending_it() {
        let transform = byte_transform_stream(64, PairingTransform::new());
        let (readable, writable) = transform.sides();
        drop(transform);
        write_owned_byte_stream(&writable, b"held".to_vec(), 64)
            .await
            .unwrap();
        fail_owned_writable_stream(&writable);
        assert_eq!(
            read_owned_byte_stream(&readable).await,
            Err(StreamError::SOURCE_FAILED)
        );
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::{can_reserve, close_side_once, record_first_failure};

    #[kani::proof]
    fn an_accepted_reservation_never_exceeds_capacity() {
        let buffered: usize = kani::any();
        let requested: usize = kani::any();
        let capacity: usize = kani::any();
        kani::assume(buffered <= capacity);

        assert!(!can_reserve(buffered, requested, capacity) || buffered + requested <= capacity);
    }

    #[kani::proof]
    fn closing_a_side_is_idempotent() {
        let initially_closed: bool = kani::any();
        let mut closed = initially_closed;
        let first = close_side_once(&mut closed);
        let second = close_side_once(&mut closed);

        assert!(closed);
        assert!(!second);
        assert_eq!(first, !initially_closed);
    }

    #[kani::proof]
    fn the_first_stream_failure_wins() {
        let initial: u8 = kani::any();
        let first: u8 = kani::any();
        let second: u8 = kani::any();
        kani::assume(initial <= 1);
        kani::assume(first > 0);
        kani::assume(second > 0);

        let mut slot = initial;
        let accepted = record_first_failure(&mut slot, first);
        let replaced = record_first_failure(&mut slot, second);

        assert!(!replaced);
        assert_eq!(slot, if initial == 0 { first } else { initial });
        assert_eq!(accepted, initial == 0);
    }
}
