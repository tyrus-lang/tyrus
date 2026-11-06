//! Native Promise handle ABI used by async lowering.

use super::*;
pub use crate::abi::{TyrusPromiseCancelHook, TyrusPromiseDrop};
use std::ffi::CString;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use tyrus_string::TyrusString;

pub const TYRUS_PROMISE_PENDING: i32 = 0;
pub const TYRUS_PROMISE_FULFILLED: i32 = 1;
pub const TYRUS_PROMISE_REJECTED: i32 = 2;
pub const TYRUS_PROMISE_CANCELLED: i32 = 3;

pub type TyrusPromiseWake = Option<unsafe extern "C" fn(*mut c_void)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromiseState {
    Pending,
    Fulfilled,
    Rejected,
    Cancelled,
}

impl PromiseState {
    fn code(self) -> i32 {
        match self {
            PromiseState::Pending => TYRUS_PROMISE_PENDING,
            PromiseState::Fulfilled => TYRUS_PROMISE_FULFILLED,
            PromiseState::Rejected => TYRUS_PROMISE_REJECTED,
            PromiseState::Cancelled => TYRUS_PROMISE_CANCELLED,
        }
    }
}

#[repr(C)]
pub struct TyrusPromise {
    data: Mutex<PromiseData>,
    settled: Condvar,
}

struct PromiseData {
    state: PromiseState,
    value: *mut c_void,
    drop_value: TyrusPromiseDrop,
    error: Option<Box<PromiseError>>,
    waiters: Vec<Waiter>,
    pending_thrown: Option<Arc<ThrownCell>>,
    cancel_requested: Option<CString>,
    cancel_hook: Option<(TyrusPromiseCancelHook, *mut c_void, TyrusPromiseDrop)>,
}

/// One reference to a pending Promise, released whenever the holder goes away.
///
/// For a future that does not always reach its own end. A cancelled request is
/// aborted and a timed-out one is dropped, both at whatever `await` they were
/// sitting on, so a release written after that point never runs. Both cases
/// were leaking a whole Promise, its error and its waiter list: for the client,
/// one set per cancelled download; for the server, one per request that missed
/// its clock, which is a leak a busy service accumulates for as long as it runs.
///
/// The address is kept as a `usize` on purpose, matching the reason those
/// futures materialize their pointer only after the await: a raw pointer held
/// across a suspension makes the whole future non-`Send`, and the executor
/// cannot spawn one.
pub(crate) struct PromiseReference(pub(crate) usize);

impl Drop for PromiseReference {
    fn drop(&mut self) {
        unsafe { crate::tyrus_promise_free(self.0 as *mut TyrusPromise) };
    }
}

#[derive(Clone)]
struct PromiseError {
    message: CString,
    code: Option<CString>,
    errors: Vec<CString>,
    thrown: Option<Arc<ThrownCell>>,
}

struct ThrownCell {
    tag: u64,
    data: *mut c_void,
    drop_data: TyrusPromiseDrop,
}

impl Drop for ThrownCell {
    fn drop(&mut self) {
        if !self.data.is_null()
            && let Some(drop_data) = self.drop_data
        {
            unsafe { drop_data(self.data) };
        }
    }
}

unsafe impl Send for ThrownCell {}
unsafe impl Sync for ThrownCell {}

impl PromiseError {
    fn single(message: impl Into<String>) -> Box<Self> {
        Box::new(Self {
            message: cstring_lossy(message),
            code: None,
            errors: Vec::new(),
            thrown: None,
        })
    }

    fn from_cstring(message: CString) -> Box<Self> {
        Box::new(Self {
            message,
            code: None,
            errors: Vec::new(),
            thrown: None,
        })
    }

    fn structured(code: impl Into<String>, message: impl Into<String>) -> Box<Self> {
        Box::new(Self {
            message: cstring_lossy(message),
            code: Some(cstring_lossy(code)),
            errors: Vec::new(),
            thrown: None,
        })
    }

    fn aggregate(messages: Vec<String>) -> Box<Self> {
        Box::new(Self {
            message: cstring_lossy("All promises were rejected"),
            code: None,
            errors: messages.into_iter().map(cstring_lossy).collect(),
            thrown: None,
        })
    }
}

// Payloads and waiter contexts cross threads only through the runtime ABI. The
// compiler/runtime contract owns their lifetimes, while the mutex serializes all
// state transitions and waiter-list access.
impl PromiseData {
    fn take_requested_cancellation(&mut self) -> Option<Box<PromiseError>> {
        self.cancel_requested.take().map(PromiseError::from_cstring)
    }
}

unsafe impl Send for PromiseData {}

struct Waiter {
    wake: unsafe extern "C" fn(*mut c_void),
    context: *mut c_void,
    /// Frees `context` when the Promise is released without ever settling, so
    /// a waiter that is never woken does not leak what it holds. `None` for a
    /// waiter registered by generated code, whose context is a frame the
    /// Promise does not own.
    release: Option<unsafe extern "C" fn(*mut c_void)>,
}

impl TyrusPromise {
    fn pending() -> Self {
        Self {
            data: Mutex::new(PromiseData {
                state: PromiseState::Pending,
                value: ptr::null_mut(),
                drop_value: None,
                error: None,
                waiters: Vec::new(),
                pending_thrown: None,
                cancel_requested: None,
                cancel_hook: None,
            }),
            settled: Condvar::new(),
        }
    }

    fn fulfilled(value: *mut c_void, drop_value: TyrusPromiseDrop) -> Self {
        Self {
            data: Mutex::new(PromiseData {
                state: PromiseState::Fulfilled,
                value,
                drop_value,
                error: None,
                waiters: Vec::new(),
                pending_thrown: None,
                cancel_requested: None,
                cancel_hook: None,
            }),
            settled: Condvar::new(),
        }
    }

    fn rejected(message: String) -> Self {
        Self {
            data: Mutex::new(PromiseData {
                state: PromiseState::Rejected,
                value: ptr::null_mut(),
                drop_value: None,
                error: Some(PromiseError::single(message)),
                waiters: Vec::new(),
                pending_thrown: None,
                cancel_requested: None,
                cancel_hook: None,
            }),
            settled: Condvar::new(),
        }
    }

    fn cancelled(message: String) -> Self {
        Self {
            data: Mutex::new(PromiseData {
                state: PromiseState::Cancelled,
                value: ptr::null_mut(),
                drop_value: None,
                error: Some(PromiseError::single(message)),
                waiters: Vec::new(),
                pending_thrown: None,
                cancel_requested: None,
                cancel_hook: None,
            }),
            settled: Condvar::new(),
        }
    }
}

impl Drop for PromiseData {
    fn drop(&mut self) {
        unsafe {
            if let Some((_, context, Some(drop_context))) = self.cancel_hook.take() {
                drop_context(context);
            }
            if !self.value.is_null()
                && let Some(drop_value) = self.drop_value
            {
                drop_value(self.value);
            }
            for waiter in std::mem::take(&mut self.waiters) {
                if let Some(release) = waiter.release {
                    release(waiter.context);
                }
            }
        }
    }
}

fn cstring_lossy(message: impl Into<String>) -> CString {
    let message = message.into().replace('\0', "\\0");
    CString::new(message).unwrap_or_else(|_| CString::new("tyrus promise error").expect("no NUL"))
}

unsafe fn message_from_ptr(message: TyrusString, fallback: &str) -> String {
    if message.is_null() {
        fallback.to_string()
    } else {
        unsafe { message.as_str() }.to_owned()
    }
}

unsafe fn promise_ref<'a>(promise: *const TyrusPromise) -> &'a TyrusPromise {
    unsafe {
        if promise.is_null() {
            crate::panic::abort_with(
                TyrusString::null(),
                "tyrus runtime error: null Promise handle",
            );
        }
        &*promise
    }
}

fn promise_data(promise: &TyrusPromise) -> MutexGuard<'_, PromiseData> {
    promise
        .data
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn wake_all(waiters: Vec<Waiter>) {
    for waiter in waiters {
        unsafe { (waiter.wake)(waiter.context) };
    }
}

pub(crate) unsafe fn wait_until_settled(promise: *mut TyrusPromise) -> i32 {
    unsafe {
        let promise = promise_ref(promise);
        let mut data = promise_data(promise);
        while data.state == PromiseState::Pending {
            data = promise
                .settled
                .wait(data)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        data.state.code()
    }
}

struct SettleSignal {
    settled: AtomicBool,
    notify: tokio::sync::Notify,
}

struct SettleWaiter<K> {
    signal: Arc<SettleSignal>,
    _keep_alive: K,
}

unsafe extern "C" fn wake_settle_waiter<K>(context: *mut c_void) {
    unsafe {
        let waiter = Box::from_raw(context.cast::<SettleWaiter<K>>());
        waiter.signal.settled.store(true, Ordering::Release);
        waiter.signal.notify.notify_one();
    }
}

unsafe extern "C" fn release_settle_waiter<K>(context: *mut c_void) {
    unsafe {
        drop(Box::from_raw(context.cast::<SettleWaiter<K>>()));
    }
}

/// Waits for `promise` to settle without occupying a thread, and yields its
/// state.
///
/// Waiting with [`wait_until_settled`] from `spawn_blocking` held one thread
/// of the blocking pool for as long as the Promise stayed pending, so a server
/// could keep only as many handlers suspended as that pool had threads: a
/// WebSocket connection, which lives as long as its handler, was capped by
/// threads and not by memory.
///
/// `keep_alive` is held until the Promise settles or is released, which can be
/// after the returned future is dropped. That is what a caller that stops
/// waiting needs: a request that missed its clock stops awaiting, but the
/// handler keeps writing to what it was given until its Promise settles.
///
/// # Safety
///
/// `promise` must be a valid handle, and the caller must keep it alive for as
/// long as the returned future is polled.
pub(crate) unsafe fn settled<K: Send + 'static>(
    promise: *mut TyrusPromise,
    keep_alive: K,
) -> impl Future<Output = i32> + Send + 'static {
    unsafe {
        let signal = Arc::new(SettleSignal {
            settled: AtomicBool::new(false),
            notify: tokio::sync::Notify::new(),
        });
        let context = Box::into_raw(Box::new(SettleWaiter {
            signal: Arc::clone(&signal),
            _keep_alive: keep_alive,
        }))
        .cast::<c_void>();
        let registered = {
            let mut data = promise_data(promise_ref(promise));
            if data.state == PromiseState::Pending {
                data.waiters.push(Waiter {
                    wake: wake_settle_waiter::<K>,
                    context,
                    release: Some(release_settle_waiter::<K>),
                });
                true
            } else {
                false
            }
        };
        if !registered {
            wake_settle_waiter::<K>(context);
        }
        let promise = promise as usize;
        async move {
            while !signal.settled.load(Ordering::Acquire) {
                signal.notify.notified().await;
            }
            promise_data(promise_ref(promise as *const TyrusPromise))
                .state
                .code()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_attach_thrown(
    promise: *mut TyrusPromise,
    tag: u64,
    data: *mut c_void,
    drop_data: TyrusPromiseDrop,
) {
    unsafe {
        let cell = Arc::new(ThrownCell {
            tag,
            data,
            drop_data,
        });
        let mut state = promise_data(promise_ref(promise));
        if state.state == PromiseState::Rejected
            && let Some(error) = state.error.as_mut()
        {
            error.thrown = Some(cell);
            return;
        }
        if state.state == PromiseState::Pending {
            state.pending_thrown = Some(cell);
            return;
        }
        drop(state);
        drop(cell);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_error_thrown_tag(promise: *const TyrusPromise) -> u64 {
    unsafe {
        promise_data(promise_ref(promise))
            .error
            .as_ref()
            .and_then(|error| error.thrown.as_ref())
            .map_or(0, |cell| cell.tag)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_error_thrown_data(
    promise: *const TyrusPromise,
) -> *const c_void {
    unsafe {
        promise_data(promise_ref(promise))
            .error
            .as_ref()
            .and_then(|error| error.thrown.as_ref())
            .map_or(std::ptr::null(), |cell| cell.data.cast_const())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_promise_pending() -> *mut TyrusPromise {
    Arc::into_raw(Arc::new(TyrusPromise::pending())).cast_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_promise_resolve(
    value: *mut c_void,
    drop_value: TyrusPromiseDrop,
) -> *mut TyrusPromise {
    Arc::into_raw(Arc::new(TyrusPromise::fulfilled(value, drop_value))).cast_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_promise_resolve_void() -> *mut TyrusPromise {
    tyrus_promise_resolve(ptr::null_mut(), None)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_reject(message: TyrusString) -> *mut TyrusPromise {
    unsafe {
        let message = message_from_ptr(message, "tyrus promise rejected");
        Arc::into_raw(Arc::new(TyrusPromise::rejected(message))).cast_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_cancelled(message: TyrusString) -> *mut TyrusPromise {
    unsafe {
        let message = message_from_ptr(message, "tyrus promise cancelled");
        Arc::into_raw(Arc::new(TyrusPromise::cancelled(message))).cast_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_retain(promise: *mut TyrusPromise) -> *mut TyrusPromise {
    unsafe {
        let _ = promise_ref(promise);
        Arc::increment_strong_count(promise);
        promise
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_fulfill(
    promise: *mut TyrusPromise,
    value: *mut c_void,
    drop_value: TyrusPromiseDrop,
) -> i32 {
    unsafe {
        let promise = promise_ref(promise);
        let waiters = {
            let mut data = promise_data(promise);
            if data.state != PromiseState::Pending {
                return 0;
            }
            match data.take_requested_cancellation() {
                Some(cancellation) => {
                    data.state = PromiseState::Cancelled;
                    data.error = Some(cancellation);
                    if !value.is_null()
                        && let Some(drop_value) = drop_value
                    {
                        drop_value(value);
                    }
                    std::mem::take(&mut data.waiters)
                }
                _ => {
                    data.state = PromiseState::Fulfilled;
                    data.value = value;
                    data.drop_value = drop_value;
                    data.error = None;
                    std::mem::take(&mut data.waiters)
                }
            }
        };
        promise.settled.notify_all();
        wake_all(waiters);
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_fulfill_owned(
    promise: *mut TyrusPromise,
    value: *mut c_void,
    drop_value: TyrusPromiseDrop,
) -> i32 {
    unsafe {
        let settled = tyrus_promise_fulfill(promise, value, drop_value);
        if settled == 0
            && !value.is_null()
            && let Some(drop_value) = drop_value
        {
            drop_value(value);
        }
        settled
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_reject_pending(
    promise: *mut TyrusPromise,
    message: TyrusString,
) -> i32 {
    unsafe {
        let promise = promise_ref(promise);
        let waiters = {
            let mut data = promise_data(promise);
            if data.state != PromiseState::Pending {
                return 0;
            }
            match data.take_requested_cancellation() {
                Some(cancellation) => {
                    data.state = PromiseState::Cancelled;
                    data.error = Some(cancellation);
                }
                None => {
                    data.state = PromiseState::Rejected;
                    let mut error =
                        PromiseError::single(message_from_ptr(message, "tyrus promise rejected"));
                    error.thrown = data.pending_thrown.take();
                    data.error = Some(error);
                }
            }
            std::mem::take(&mut data.waiters)
        };
        promise.settled.notify_all();
        wake_all(waiters);
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_reject_pending_structured(
    promise: *mut TyrusPromise,
    code: TyrusString,
    message: TyrusString,
) -> i32 {
    unsafe {
        let promise_ref = promise_ref(promise);
        let waiters = {
            let mut data = promise_data(promise_ref);
            if data.state != PromiseState::Pending {
                return 0;
            }
            match data.take_requested_cancellation() {
                Some(cancellation) => {
                    data.state = PromiseState::Cancelled;
                    data.error = Some(cancellation);
                }
                None => {
                    data.state = PromiseState::Rejected;
                    let mut error = PromiseError::structured(
                        message_from_ptr(code, "ERR_RUNTIME"),
                        message_from_ptr(message, "tyrus promise rejected"),
                    );
                    error.thrown = data.pending_thrown.take();
                    data.error = Some(error);
                }
            }
            std::mem::take(&mut data.waiters)
        };
        promise_ref.settled.notify_all();
        wake_all(waiters);
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_cancel_pending(
    promise: *mut TyrusPromise,
    message: TyrusString,
) -> i32 {
    unsafe {
        let promise = promise_ref(promise);
        let waiters = {
            let mut data = promise_data(promise);
            if data.state != PromiseState::Pending {
                return 0;
            }
            data.state = PromiseState::Cancelled;
            data.error = Some(PromiseError::single(message_from_ptr(
                message,
                "tyrus promise cancelled",
            )));
            std::mem::take(&mut data.waiters)
        };
        promise.settled.notify_all();
        wake_all(waiters);
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_on_cancel_request(
    promise: *mut TyrusPromise,
    hook: TyrusPromiseCancelHook,
    context: *mut c_void,
    drop_context: TyrusPromiseDrop,
) -> i32 {
    unsafe {
        let promise = promise_ref(promise);
        let mut data = promise_data(promise);
        if data.state != PromiseState::Pending {
            return 0;
        }
        data.cancel_hook = Some((hook, context, drop_context));
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_request_cancel(
    promise: *mut TyrusPromise,
    reason: TyrusString,
) -> i32 {
    unsafe {
        let promise = promise_ref(promise);
        let reason = message_from_ptr(reason, "tyrus promise cancelled");
        let (hook, waiters) = {
            let mut data = promise_data(promise);
            if data.state != PromiseState::Pending || data.cancel_requested.is_some() {
                return 0;
            }
            match data.cancel_hook.take() {
                Some(hook) => {
                    data.cancel_requested = Some(cstring_lossy(reason));
                    (Some(hook), Vec::new())
                }
                None => {
                    data.state = PromiseState::Cancelled;
                    data.error = Some(PromiseError::single(reason));
                    (None, std::mem::take(&mut data.waiters))
                }
            }
        };
        match hook {
            Some((hook, context, drop_context)) => {
                hook(context);
                if let Some(drop_context) = drop_context {
                    drop_context(context);
                }
            }
            None => {
                promise.settled.notify_all();
                wake_all(waiters);
            }
        }
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_cancel_requested(promise: *const TyrusPromise) -> i32 {
    unsafe {
        i32::from(
            promise_data(promise_ref(promise))
                .cancel_requested
                .is_some(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_on_settle(
    promise: *mut TyrusPromise,
    wake: TyrusPromiseWake,
    context: *mut c_void,
) -> i32 {
    unsafe {
        let promise = promise_ref(promise);
        let Some(wake) = wake else {
            return 0;
        };
        {
            let mut data = promise_data(promise);
            if data.state == PromiseState::Pending {
                data.waiters.push(Waiter {
                    wake,
                    context,
                    release: None,
                });
                return 1;
            }
        }
        wake(context);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_state(promise: *const TyrusPromise) -> i32 {
    unsafe { promise_data(promise_ref(promise)).state.code() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_value_ptr(promise: *const TyrusPromise) -> *mut c_void {
    unsafe {
        let data = promise_data(promise_ref(promise));
        if data.state == PromiseState::Fulfilled {
            data.value
        } else {
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_take_value(promise: *mut TyrusPromise) -> *mut c_void {
    unsafe { tyrus_promise_take_payload(promise).0 }
}

pub(crate) unsafe fn tyrus_promise_take_payload(
    promise: *mut TyrusPromise,
) -> (*mut c_void, TyrusPromiseDrop) {
    unsafe {
        let mut data = promise_data(promise_ref(promise));
        if data.state != PromiseState::Fulfilled || data.value.is_null() {
            return (ptr::null_mut(), None);
        }
        let drop_value = data.drop_value.take();
        (
            std::mem::replace(&mut data.value, ptr::null_mut()),
            drop_value,
        )
    }
}

pub(crate) unsafe fn promise_error_string(promise: *const TyrusPromise) -> String {
    unsafe {
        let data = promise_data(promise_ref(promise));
        data.error.as_ref().map_or_else(
            || "Promise rejected without an error message".to_string(),
            |error| error.message.to_string_lossy().into_owned(),
        )
    }
}

pub(crate) unsafe fn promise_error_parts(promise: *const TyrusPromise) -> (String, Vec<String>) {
    unsafe {
        let data = promise_data(promise_ref(promise));
        data.error.as_ref().map_or_else(
            || {
                (
                    "Promise rejected without an error message".to_string(),
                    Vec::new(),
                )
            },
            |error| {
                (
                    error.message.to_string_lossy().into_owned(),
                    error
                        .errors
                        .iter()
                        .map(|message| message.to_string_lossy().into_owned())
                        .collect(),
                )
            },
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_error_message(promise: *const TyrusPromise) -> TyrusString {
    unsafe {
        promise_data(promise_ref(promise))
            .error
            .as_ref()
            .and_then(|error| TyrusString::lend(error.message.to_str().ok()?))
            .unwrap_or_else(TyrusString::null)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_error_code(promise: *const TyrusPromise) -> TyrusString {
    unsafe {
        promise_data(promise_ref(promise))
            .error
            .as_ref()
            .and_then(|error| TyrusString::lend(error.code.as_ref()?.to_str().ok()?))
            .unwrap_or_else(TyrusString::null)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_aggregate_error_count(
    promise: *const TyrusPromise,
) -> usize {
    unsafe {
        promise_data(promise_ref(promise))
            .error
            .as_ref()
            .map_or(0, |error| error.errors.len())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_aggregate_error_at(
    promise: *const TyrusPromise,
    index: usize,
) -> TyrusString {
    unsafe {
        promise_data(promise_ref(promise))
            .error
            .as_ref()
            .and_then(|error| error.errors.get(index))
            .and_then(|message| TyrusString::lend(message.to_str().ok()?))
            .unwrap_or_else(TyrusString::null)
    }
}

pub(crate) fn rejected_aggregate(messages: Vec<String>) -> *mut TyrusPromise {
    let mut promise = TyrusPromise::pending();
    let data = promise.data.get_mut().unwrap();
    data.state = PromiseState::Rejected;
    data.error = Some(PromiseError::aggregate(messages));
    Arc::into_raw(Arc::new(promise)).cast_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_propagate_failure(
    source: *const TyrusPromise,
    target: *mut TyrusPromise,
) -> i32 {
    unsafe {
        let (state, error) = {
            let source = promise_data(promise_ref(source));
            (source.state, source.error.clone())
        };
        if !matches!(state, PromiseState::Rejected | PromiseState::Cancelled) {
            return 0;
        }
        let target_ref = promise_ref(target);
        let waiters = {
            let mut target = promise_data(target_ref);
            if target.state != PromiseState::Pending {
                return 0;
            }
            match target.take_requested_cancellation() {
                Some(cancellation) => {
                    target.state = PromiseState::Cancelled;
                    target.error = Some(cancellation);
                }
                None => {
                    target.state = state;
                    target.error = error;
                }
            }
            std::mem::take(&mut target.waiters)
        };
        target_ref.settled.notify_all();
        wake_all(waiters);
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_await(promise: *mut TyrusPromise) -> *mut c_void {
    unsafe {
        let data = promise_data(promise_ref(promise));
        match data.state {
            PromiseState::Fulfilled => data.value,
            PromiseState::Pending => crate::panic::abort_with(
                TyrusString::null(),
                "tyrus runtime error: attempted to await a pending Promise before executor support is implemented",
            ),
            PromiseState::Rejected => {
                let message = data
                    .error
                    .as_ref()
                    .and_then(|error| TyrusString::lend(error.message.to_str().ok()?))
                    .unwrap_or_else(TyrusString::null);
                crate::panic::abort_with(message, "tyrus promise rejected")
            }
            PromiseState::Cancelled => {
                let message = data
                    .error
                    .as_ref()
                    .and_then(|error| TyrusString::lend(error.message.to_str().ok()?))
                    .unwrap_or_else(TyrusString::null);
                crate::panic::abort_with(message, "tyrus promise cancelled")
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_free(promise: *mut TyrusPromise) {
    unsafe {
        if !promise.is_null() {
            Arc::decrement_strong_count(promise);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    #[test]
    fn a_rejection_keeps_the_thrown_value_until_it_is_released() {
        let drops = Arc::new(AtomicUsize::new(0));
        let promise = tyrus_promise_pending();
        unsafe {
            tyrus_promise_reject_pending(promise, TyrusString::literal("boom"));
            tyrus_promise_attach_thrown(promise, 7, drop_tracked_i32(42, &drops), Some(count_drop));
            assert_eq!(tyrus_promise_error_thrown_tag(promise), 7);
            let data = tyrus_promise_error_thrown_data(promise).cast::<DropTrackedI32>();
            assert_eq!((*data).value, 42);
            assert_eq!(drops.load(Ordering::SeqCst), 0);
            tyrus_promise_free(promise);
        }
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_thrown_value_handed_over_before_the_rejection_is_published_with_it() {
        let drops = Arc::new(AtomicUsize::new(0));
        let promise = tyrus_promise_pending();
        unsafe {
            tyrus_promise_attach_thrown(promise, 7, drop_tracked_i32(42, &drops), Some(count_drop));
            assert_eq!(tyrus_promise_error_thrown_tag(promise), 0);
            assert_eq!(drops.load(Ordering::SeqCst), 0);
            tyrus_promise_reject_pending(promise, TyrusString::literal("boom"));
            assert_eq!(tyrus_promise_error_thrown_tag(promise), 7);
            let data = tyrus_promise_error_thrown_data(promise).cast::<DropTrackedI32>();
            assert_eq!((*data).value, 42);
            assert_eq!(drops.load(Ordering::SeqCst), 0);
            tyrus_promise_free(promise);
        }
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_staged_thrown_value_no_rejection_claims_is_released_with_the_promise() {
        // Staging must not turn into a leak when the promise settles some other
        // way, or never settles at all. `ThrownCell::drop` frees the value, so
        // the cell riding on the promise is released exactly once when the
        // handle goes.
        let drops = Arc::new(AtomicUsize::new(0));
        let promise = tyrus_promise_pending();
        unsafe {
            tyrus_promise_attach_thrown(promise, 7, drop_tracked_i32(42, &drops), Some(count_drop));
            assert_eq!(drops.load(Ordering::SeqCst), 0);
            tyrus_promise_free(promise);
        }
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_thrown_value_attached_to_a_settled_promise_is_released_at_once() {
        let drops = Arc::new(AtomicUsize::new(0));
        let value_drops = Arc::new(AtomicUsize::new(0));
        let promise = tyrus_promise_resolve(drop_tracked_i32(1, &value_drops), Some(count_drop));
        unsafe {
            tyrus_promise_attach_thrown(promise, 7, drop_tracked_i32(42, &drops), Some(count_drop));
            assert_eq!(drops.load(Ordering::SeqCst), 1);
            assert_eq!(tyrus_promise_error_thrown_tag(promise), 0);
            tyrus_promise_free(promise);
        }
    }

    struct CountsDrop(Arc<AtomicUsize>);

    impl Drop for CountsDrop {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("a test runtime")
            .block_on(future)
    }

    #[test]
    fn settled_waits_for_a_settlement_made_on_another_thread() {
        let promise = tyrus_promise_pending();
        let waiting = unsafe { settled(promise, ()) };
        let address = promise as usize;
        let settler = thread::spawn(move || {
            thread::sleep(std::time::Duration::from_millis(20));
            unsafe {
                tyrus_promise_fulfill(address as *mut TyrusPromise, std::ptr::null_mut(), None)
            };
        });
        assert_eq!(block_on(waiting), TYRUS_PROMISE_FULFILLED);
        settler.join().unwrap();
        unsafe { tyrus_promise_free(promise) };
    }

    #[test]
    fn settled_on_a_settled_promise_resolves_at_once() {
        let promise = tyrus_promise_pending();
        unsafe { tyrus_promise_cancel_pending(promise, TyrusString::null()) };
        let drops = Arc::new(AtomicUsize::new(0));
        let waiting = unsafe { settled(promise, CountsDrop(Arc::clone(&drops))) };
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(block_on(waiting), TYRUS_PROMISE_CANCELLED);
        unsafe { tyrus_promise_free(promise) };
    }

    #[test]
    fn a_dropped_wait_keeps_what_it_holds_until_the_promise_settles() {
        let promise = tyrus_promise_pending();
        let drops = Arc::new(AtomicUsize::new(0));
        drop(unsafe { settled(promise, CountsDrop(Arc::clone(&drops))) });
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        unsafe {
            tyrus_promise_fulfill(promise, std::ptr::null_mut(), None);
        }
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        unsafe { tyrus_promise_free(promise) };
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_promise_released_while_pending_releases_what_its_waiters_hold() {
        let promise = tyrus_promise_pending();
        let drops = Arc::new(AtomicUsize::new(0));
        drop(unsafe { settled(promise, CountsDrop(Arc::clone(&drops))) });
        unsafe { tyrus_promise_free(promise) };
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    struct DropTrackedI32 {
        value: i32,
        drops: Arc<AtomicUsize>,
    }

    fn drop_tracked_i32(value: i32, drops: &Arc<AtomicUsize>) -> *mut c_void {
        Box::into_raw(Box::new(DropTrackedI32 {
            value,
            drops: Arc::clone(drops),
        }))
        .cast()
    }

    unsafe extern "C" fn count_drop(value: *mut c_void) {
        unsafe {
            if !value.is_null() {
                let value = Box::from_raw(value.cast::<DropTrackedI32>());
                value.drops.fetch_add(1, Ordering::SeqCst);
            }
        }
    }

    unsafe extern "C" fn count_wake(context: *mut c_void) {
        unsafe {
            let counter = &*(context as *const AtomicUsize);
            counter.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn a_promise_pays_only_a_pointer_for_the_error_it_may_never_hold() {
        assert_eq!(
            std::mem::size_of::<Option<Box<PromiseError>>>(),
            std::mem::size_of::<*const c_void>()
        );
        assert!(std::mem::size_of::<PromiseError>() > std::mem::size_of::<*const c_void>());
    }

    #[test]
    fn resolved_promise_exposes_value_and_frees_payload() {
        let drops = Arc::new(AtomicUsize::new(0));
        let value = drop_tracked_i32(42, &drops);
        let promise = tyrus_promise_resolve(value, Some(count_drop));
        unsafe {
            assert_eq!(tyrus_promise_state(promise), TYRUS_PROMISE_FULFILLED);
            assert_eq!(tyrus_promise_value_ptr(promise), value);
            assert_eq!(tyrus_promise_await(promise), value);
            tyrus_promise_free(promise);
        }
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn fulfilled_payload_can_be_taken_exactly_once() {
        let drops = Arc::new(AtomicUsize::new(0));
        let value = drop_tracked_i32(42, &drops);
        let promise = tyrus_promise_resolve(value, Some(count_drop));
        unsafe {
            let taken = tyrus_promise_take_value(promise);
            assert_eq!((*taken.cast::<DropTrackedI32>()).value, 42);
            assert!(tyrus_promise_take_value(promise).is_null());
            tyrus_promise_free(promise);
            assert_eq!(drops.load(Ordering::SeqCst), 0);
            drop(Box::from_raw(taken.cast::<DropTrackedI32>()));
        }
        assert_eq!(drops.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn pending_promise_can_be_fulfilled_once() {
        let drops = Arc::new(AtomicUsize::new(0));
        let promise = tyrus_promise_pending();
        let first = drop_tracked_i32(1, &drops);
        let second = drop_tracked_i32(2, &drops);
        unsafe {
            assert_eq!(tyrus_promise_state(promise), TYRUS_PROMISE_PENDING);
            assert_eq!(tyrus_promise_fulfill(promise, first, Some(count_drop)), 1);
            assert_eq!(tyrus_promise_fulfill(promise, second, Some(count_drop)), 0);
            assert_eq!(tyrus_promise_state(promise), TYRUS_PROMISE_FULFILLED);
            assert_eq!(tyrus_promise_value_ptr(promise), first);
            tyrus_promise_free(promise);
            count_drop(second);
        }
        assert_eq!(drops.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn rejected_promise_exposes_error_message() {
        let message = TyrusString::literal("failed");
        let promise = unsafe { tyrus_promise_reject(message.borrow()) };
        unsafe {
            assert_eq!(tyrus_promise_state(promise), TYRUS_PROMISE_REJECTED);
            let reason = tyrus_promise_error_message(promise);
            let actual = reason.as_str();
            assert_eq!(actual, "failed");
            tyrus_promise_free(promise);
        }
    }

    #[test]
    fn cancelled_pending_promise_keeps_reason() {
        let promise = tyrus_promise_pending();
        let message = TyrusString::literal("shutdown");
        unsafe {
            assert_eq!(tyrus_promise_cancel_pending(promise, message.borrow()), 1);
            assert_eq!(tyrus_promise_state(promise), TYRUS_PROMISE_CANCELLED);
            let reason = tyrus_promise_error_message(promise);
            let actual = reason.as_str();
            assert_eq!(actual, "shutdown");
            tyrus_promise_free(promise);
        }
    }

    #[test]
    fn pending_promise_can_settle_from_another_thread_and_wake_once() {
        let wakes = Box::into_raw(Box::new(AtomicUsize::new(0)));
        let promise = tyrus_promise_pending();
        unsafe {
            assert_eq!(
                tyrus_promise_on_settle(promise, Some(count_wake), wakes.cast()),
                1
            );
            tyrus_promise_retain(promise);
        }

        let promise_address = promise as usize;
        let producer = thread::spawn(move || unsafe {
            let promise = promise_address as *mut TyrusPromise;
            assert_eq!(tyrus_promise_fulfill(promise, ptr::null_mut(), None), 1);
            tyrus_promise_free(promise);
        });
        producer.join().unwrap();

        unsafe {
            assert_eq!(tyrus_promise_state(promise), TYRUS_PROMISE_FULFILLED);
            assert_eq!((*wakes).load(Ordering::SeqCst), 1);
            tyrus_promise_free(promise);
            drop(Box::from_raw(wakes));
        }
    }

    #[test]
    fn settled_promise_wakes_new_waiter_immediately() {
        let wakes = Box::into_raw(Box::new(AtomicUsize::new(0)));
        let promise = tyrus_promise_resolve_void();
        unsafe {
            assert_eq!(
                tyrus_promise_on_settle(promise, Some(count_wake), wakes.cast()),
                0
            );
            assert_eq!((*wakes).load(Ordering::SeqCst), 1);
            tyrus_promise_free(promise);
            drop(Box::from_raw(wakes));
        }
    }

    #[test]
    fn settling_once_wakes_every_registered_waiter_once() {
        let first = Box::into_raw(Box::new(AtomicUsize::new(0)));
        let second = Box::into_raw(Box::new(AtomicUsize::new(0)));
        let promise = tyrus_promise_pending();
        unsafe {
            assert_eq!(
                tyrus_promise_on_settle(promise, Some(count_wake), first.cast()),
                1
            );
            assert_eq!(
                tyrus_promise_on_settle(promise, Some(count_wake), second.cast()),
                1
            );
            assert_eq!(tyrus_promise_fulfill(promise, ptr::null_mut(), None), 1);
            assert_eq!((*first).load(Ordering::SeqCst), 1);
            assert_eq!((*second).load(Ordering::SeqCst), 1);
            assert_eq!(tyrus_promise_fulfill(promise, ptr::null_mut(), None), 0);
            assert_eq!((*first).load(Ordering::SeqCst), 1);
            assert_eq!((*second).load(Ordering::SeqCst), 1);
            tyrus_promise_free(promise);
            drop(Box::from_raw(first));
            drop(Box::from_raw(second));
        }
    }

    #[test]
    fn rejected_and_cancelled_states_propagate_without_aborting() {
        unsafe {
            let rejected = tyrus_promise_reject(TyrusString::literal("boom"));
            let rejected_target = tyrus_promise_pending();
            let cancelled = tyrus_promise_cancelled(TyrusString::literal("stop"));
            let cancelled_target = tyrus_promise_pending();
            assert_eq!(
                tyrus_promise_propagate_failure(rejected, rejected_target),
                1
            );
            assert_eq!(tyrus_promise_state(rejected_target), TYRUS_PROMISE_REJECTED);
            assert_eq!(
                tyrus_promise_propagate_failure(cancelled, cancelled_target),
                1
            );
            assert_eq!(
                tyrus_promise_state(cancelled_target),
                TYRUS_PROMISE_CANCELLED
            );
            tyrus_promise_free(rejected);
            tyrus_promise_free(rejected_target);
            tyrus_promise_free(cancelled);
            tyrus_promise_free(cancelled_target);
        }
    }

    #[test]
    fn aggregate_error_preserves_input_order_and_stable_message() {
        let promise = rejected_aggregate(vec!["first".to_string(), "second".to_string()]);
        unsafe {
            assert_eq!(tyrus_promise_state(promise), TYRUS_PROMISE_REJECTED);
            assert_eq!(
                tyrus_promise_error_message(promise).as_str(),
                "All promises were rejected"
            );
            assert_eq!(tyrus_promise_aggregate_error_count(promise), 2);
            assert_eq!(
                tyrus_promise_aggregate_error_at(promise, 0).as_str(),
                "first"
            );
            assert_eq!(
                tyrus_promise_aggregate_error_at(promise, 1).as_str(),
                "second"
            );
            assert!(tyrus_promise_aggregate_error_at(promise, 2).is_null());
            tyrus_promise_free(promise);
        }
    }
}
