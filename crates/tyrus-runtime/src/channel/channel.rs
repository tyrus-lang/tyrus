use std::ffi::c_void;
use std::sync::Arc;

use tokio::sync::{Mutex as AsyncMutex, mpsc};

use crate::TyrusPromise;
use crate::abi::TyrusPromiseDrop;
use crate::executor::executor;
use crate::promise::{tyrus_promise_fulfill, tyrus_promise_pending, tyrus_promise_retain};
use tyrus_string::TyrusString;

struct ChannelMessage {
    value: *mut c_void,
    drop_value: TyrusPromiseDrop,
}

// SAFETY: `value` is a pointer to data the sender heap-allocated specifically
// to hand off through this channel; ownership transfers to the receiver, so
// nothing else touches it concurrently while it is in flight.
unsafe impl Send for ChannelMessage {}

impl Drop for ChannelMessage {
    fn drop(&mut self) {
        // A message dropped without being received (the channel was closed
        // and discarded with pending values) still owns its payload and must
        // release it when the receiver has disappeared.
        if !self.value.is_null()
            && let Some(drop_value) = self.drop_value
        {
            unsafe { drop_value(self.value) };
        }
    }
}

#[repr(C)]
struct BoxedRecvPayload {
    inner: *mut c_void,
    inner_drop: TyrusPromiseDrop,
}

fn box_recv_payload(inner: *mut c_void, inner_drop: TyrusPromiseDrop) -> *mut c_void {
    let storage =
        unsafe { libc::malloc(std::mem::size_of::<BoxedRecvPayload>()) } as *mut BoxedRecvPayload;
    if storage.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { std::ptr::write(storage, BoxedRecvPayload { inner, inner_drop }) };
    storage.cast()
}

unsafe extern "C" fn drop_recv_payload(payload: *mut c_void) {
    if payload.is_null() {
        return;
    }
    let boxed = payload as *mut BoxedRecvPayload;
    // SAFETY: only ever called on a pointer `box_recv_payload` produced.
    let value = unsafe { std::ptr::read(boxed) };
    if !value.inner.is_null()
        && let Some(inner_drop) = value.inner_drop
    {
        unsafe { inner_drop(value.inner) };
    }
    unsafe { libc::free(payload) };
}

enum SenderKind {
    Bounded(mpsc::Sender<ChannelMessage>),
    Unbounded(mpsc::UnboundedSender<ChannelMessage>),
}

enum ReceiverKind {
    Bounded(mpsc::Receiver<ChannelMessage>),
    Unbounded(mpsc::UnboundedReceiver<ChannelMessage>),
}

pub struct TyrusChannelSender {
    kind: SenderKind,
}

pub struct TyrusChannelReceiver {
    kind: AsyncMutex<ReceiverKind>,
}

#[repr(C)]
pub struct TyrusChannelPair {
    pub sender: *const TyrusChannelSender,
    pub receiver: *const TyrusChannelReceiver,
}

// SAFETY: `out` must point to a valid, writable `TyrusChannelPair`.
unsafe fn new_pair(capacity: Option<usize>, out: *mut TyrusChannelPair) {
    let (sender_kind, receiver_kind) = match capacity {
        Some(capacity) => {
            let (tx, rx) = mpsc::channel(capacity.max(1));
            (SenderKind::Bounded(tx), ReceiverKind::Bounded(rx))
        }
        None => {
            let (tx, rx) = mpsc::unbounded_channel();
            (SenderKind::Unbounded(tx), ReceiverKind::Unbounded(rx))
        }
    };
    let sender = Arc::new(TyrusChannelSender { kind: sender_kind });
    let receiver = Arc::new(TyrusChannelReceiver {
        kind: AsyncMutex::new(receiver_kind),
    });
    unsafe {
        *out = TyrusChannelPair {
            sender: Arc::into_raw(sender),
            receiver: Arc::into_raw(receiver),
        };
    }
}

/// Create a channel, `channel<T>(capacity?)`. `capacity` is `usize?`'s own
/// native ABI shape: null for the omitted/`None` case (unbounded), or a
/// pointer to a boxed `usize` otherwise (bounded), the same "an `Optional`
/// is always a pointer, whatever it boxes" representation every other
/// `T?`-typed native parameter already uses (see `tyrus_net_resolve`'s own
/// `timeoutMs?: u64`), which matters here because `capacity?` is not only
/// omittable at the call site: `const cap: usize? = ...; channel<T>(cap)`
/// passes a value whose presence is decided at runtime, not by how many
/// arguments the call has syntactically. Writes the result through `out` (a
/// single out-param, not a returned struct: see the `<Módulo>Error`
/// convention in `AGENTS.md`, this avoids depending on how a multi-field
/// aggregate crosses the `extern "C"` ABI on every target, native and WASM
/// alike).
///
/// # Safety
///
/// `out` must point to a valid, writable `TyrusChannelPair`. `capacity` must
/// be null or point to a valid `usize`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_channel_new(capacity: *const usize, out: *mut TyrusChannelPair) {
    let capacity = if capacity.is_null() {
        None
    } else {
        Some(unsafe { *capacity })
    };
    unsafe { new_pair(capacity, out) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_channel_sender_retain(
    sender: *const TyrusChannelSender,
) -> *const TyrusChannelSender {
    unsafe {
        Arc::increment_strong_count(sender);
        sender
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_channel_sender_free(sender: *const TyrusChannelSender) {
    unsafe {
        if !sender.is_null() {
            drop(Arc::from_raw(sender));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_channel_receiver_retain(
    receiver: *const TyrusChannelReceiver,
) -> *const TyrusChannelReceiver {
    unsafe {
        Arc::increment_strong_count(receiver);
        receiver
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_channel_receiver_free(receiver: *const TyrusChannelReceiver) {
    unsafe {
        if !receiver.is_null() {
            drop(Arc::from_raw(receiver));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_channel_send(
    sender: *const TyrusChannelSender,
    value: *mut c_void,
    drop_value: TyrusPromiseDrop,
) -> *mut TyrusPromise {
    let sender = unsafe { Arc::from_raw(sender) };
    let sender = std::mem::ManuallyDrop::new(sender);
    let sender: Arc<TyrusChannelSender> = Arc::clone(&sender);
    let promise = tyrus_promise_pending();
    unsafe { tyrus_promise_retain(promise) };
    let message = ChannelMessage { value, drop_value };
    let promise_address = promise as usize;
    executor().spawn(async move {
        let sent = match &sender.kind {
            SenderKind::Bounded(tx) => tx.send(message).await.is_ok(),
            SenderKind::Unbounded(tx) => tx.send(message).is_ok(),
        };
        let promise = promise_address as *mut TyrusPromise;
        unsafe {
            if sent {
                tyrus_promise_fulfill(promise, std::ptr::null_mut(), None);
            } else {
                crate::promise::tyrus_promise_reject_pending(
                    promise,
                    TyrusString::literal("channel is closed"),
                );
            }
            crate::promise::tyrus_promise_free(promise);
        }
    });
    promise
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_channel_try_send(
    sender: *const TyrusChannelSender,
    value: *mut c_void,
    drop_value: TyrusPromiseDrop,
) -> i32 {
    let sender = unsafe { &*sender };
    let message = ChannelMessage { value, drop_value };
    let result = match &sender.kind {
        SenderKind::Bounded(tx) => tx.try_send(message).is_ok(),
        SenderKind::Unbounded(tx) => tx.send(message).is_ok(),
    };
    i32::from(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_channel_recv(
    receiver: *const TyrusChannelReceiver,
) -> *mut TyrusPromise {
    let receiver = unsafe { Arc::from_raw(receiver) };
    let receiver = std::mem::ManuallyDrop::new(receiver);
    let receiver: Arc<TyrusChannelReceiver> = Arc::clone(&receiver);
    let promise = tyrus_promise_pending();
    unsafe { tyrus_promise_retain(promise) };
    let promise_address = promise as usize;
    executor().spawn(async move {
        let mut guard = receiver.kind.lock().await;
        let received = match &mut *guard {
            ReceiverKind::Bounded(rx) => rx.recv().await,
            ReceiverKind::Unbounded(rx) => rx.recv().await,
        };
        drop(guard);
        let promise = promise_address as *mut TyrusPromise;
        unsafe {
            let outer = match received {
                Some(message) => {
                    let message = std::mem::ManuallyDrop::new(message);
                    box_recv_payload(message.value, message.drop_value)
                }
                None => box_recv_payload(std::ptr::null_mut(), None),
            };
            tyrus_promise_fulfill(promise, outer, Some(drop_recv_payload));
            crate::promise::tyrus_promise_free(promise);
        }
    });
    promise
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_channel_try_recv(
    receiver: *const TyrusChannelReceiver,
) -> *mut c_void {
    let receiver = unsafe { &*receiver };
    let Ok(mut guard) = receiver.kind.try_lock() else {
        return std::ptr::null_mut();
    };
    let received = match &mut *guard {
        ReceiverKind::Bounded(rx) => rx.try_recv().ok(),
        ReceiverKind::Unbounded(rx) => rx.try_recv().ok(),
    };
    match received {
        Some(message) => {
            let message = std::mem::ManuallyDrop::new(message);
            message.value
        }
        None => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_channel_close(receiver: *const TyrusChannelReceiver) {
    let receiver = unsafe { &*receiver };
    let Ok(mut guard) = receiver.kind.try_lock() else {
        return;
    };
    match &mut *guard {
        ReceiverKind::Bounded(rx) => rx.close(),
        ReceiverKind::Unbounded(rx) => rx.close(),
    }
}
