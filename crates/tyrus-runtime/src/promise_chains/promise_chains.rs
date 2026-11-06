use crate::executor::schedule_runtime_callback;
use crate::promise::{
    TyrusPromise, promise_error_string, tyrus_promise_take_payload, tyrus_promise_value_ptr,
};
use crate::{
    TYRUS_PROMISE_FULFILLED, tyrus_promise_free, tyrus_promise_on_settle, tyrus_promise_pending,
    tyrus_promise_propagate_failure, tyrus_promise_reject_pending, tyrus_promise_retain,
    tyrus_promise_state,
};
use std::ffi::c_void;
use std::sync::Arc;
use tyrus_string::TyrusString;

pub type TyrusThenCallback =
    Option<unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut TyrusPromise>;
pub type TyrusCatchCallback =
    Option<unsafe extern "C" fn(TyrusString, *mut c_void) -> *mut TyrusPromise>;
pub type TyrusFinallyCallback = Option<unsafe extern "C" fn(*mut c_void) -> *mut TyrusPromise>;
pub type TyrusDropContext = Option<unsafe extern "C" fn(*mut c_void)>;

enum ChainCallback {
    Then(TyrusThenCallback),
    Catch(TyrusCatchCallback),
    Finally(TyrusFinallyCallback),
}

struct ChainState {
    source: *mut TyrusPromise,
    output: *mut TyrusPromise,
    callback: ChainCallback,
    context: *mut c_void,
    drop_context: TyrusDropContext,
}

unsafe impl Send for ChainState {}
unsafe impl Sync for ChainState {}

impl Drop for ChainState {
    fn drop(&mut self) {
        unsafe {
            if let Some(drop_context) = self.drop_context {
                drop_context(self.context);
            }
            tyrus_promise_free(self.source);
            tyrus_promise_free(self.output);
        }
    }
}

struct AdoptContext {
    chain: Arc<ChainState>,
    returned: *mut TyrusPromise,
    preserve_source: bool,
}

struct DirectAdoptContext {
    source: *mut TyrusPromise,
    target: *mut TyrusPromise,
}

unsafe extern "C" fn direct_adopt_wake(context: *mut c_void) {
    unsafe {
        let context = Box::from_raw(context.cast::<DirectAdoptContext>());
        transfer_or_propagate(context.source, context.target);
        tyrus_promise_free(context.source);
        tyrus_promise_free(context.target);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_adopt(
    source: *mut TyrusPromise,
    target: *mut TyrusPromise,
) -> i32 {
    unsafe {
        if source.is_null() || target.is_null() || source == target {
            return 0;
        }
        tyrus_promise_retain(source);
        tyrus_promise_retain(target);
        let context = Box::new(DirectAdoptContext { source, target });
        tyrus_promise_on_settle(
            source,
            Some(direct_adopt_wake),
            Box::into_raw(context).cast(),
        );
        1
    }
}

unsafe extern "C" fn adopt_wake(context: *mut c_void) {
    unsafe {
        let context = Box::from_raw(context.cast::<AdoptContext>());
        let state = tyrus_promise_state(context.returned);
        if state == TYRUS_PROMISE_FULFILLED {
            if context.preserve_source {
                transfer_or_propagate(context.chain.source, context.chain.output);
            } else {
                transfer_or_propagate(context.returned, context.chain.output);
            }
        } else {
            tyrus_promise_propagate_failure(context.returned, context.chain.output);
        }
        tyrus_promise_free(context.returned);
    }
}

unsafe fn adopt(chain: Arc<ChainState>, returned: *mut TyrusPromise, preserve_source: bool) {
    unsafe {
        if returned.is_null() {
            tyrus_promise_reject_pending(
                chain.output,
                TyrusString::literal("Promise callback returned an invalid Promise"),
            );
            return;
        }
        let context = Box::new(AdoptContext {
            chain,
            returned,
            preserve_source,
        });
        tyrus_promise_on_settle(returned, Some(adopt_wake), Box::into_raw(context).cast());
    }
}

unsafe fn transfer_or_propagate(source: *mut TyrusPromise, output: *mut TyrusPromise) {
    unsafe {
        if tyrus_promise_state(source) == TYRUS_PROMISE_FULFILLED {
            let (value, drop_value) = tyrus_promise_take_payload(source);
            crate::tyrus_promise_fulfill(output, value, drop_value);
        } else {
            tyrus_promise_propagate_failure(source, output);
        }
    }
}

unsafe extern "C" fn run_chain(context: *mut c_void) {
    unsafe {
        let chain = Arc::from_raw(context.cast::<ChainState>());
        let source_state = tyrus_promise_state(chain.source);
        match &chain.callback {
            ChainCallback::Then(callback) if source_state == TYRUS_PROMISE_FULFILLED => {
                let Some(callback) = callback else {
                    transfer_or_propagate(chain.source, chain.output);
                    return;
                };
                let value = tyrus_promise_value_ptr(chain.source);
                adopt(chain.clone(), callback(value, chain.context), false);
            }
            ChainCallback::Catch(callback) if source_state != TYRUS_PROMISE_FULFILLED => {
                let Some(callback) = callback else {
                    transfer_or_propagate(chain.source, chain.output);
                    return;
                };
                let message = promise_error_string(chain.source);
                // SAFETY: `message` outlives the callback it is lent to. The
                // callback only reads it, per the ownership rule.
                let lent = { TyrusString::lend(&message) }.unwrap_or_else(TyrusString::null);
                adopt(chain.clone(), callback(lent, chain.context), false);
            }
            ChainCallback::Finally(callback) => {
                let Some(callback) = callback else {
                    transfer_or_propagate(chain.source, chain.output);
                    return;
                };
                adopt(chain.clone(), callback(chain.context), true);
            }
            _ => transfer_or_propagate(chain.source, chain.output),
        }
    }
}

unsafe extern "C" fn source_wake(context: *mut c_void) {
    schedule_runtime_callback(run_chain, context);
}

unsafe fn chain(
    source: *mut TyrusPromise,
    callback: ChainCallback,
    context: *mut c_void,
    drop_context: TyrusDropContext,
) -> *mut TyrusPromise {
    unsafe {
        let output = tyrus_promise_pending();
        tyrus_promise_retain(source);
        tyrus_promise_retain(output);
        let state = Arc::new(ChainState {
            source,
            output,
            callback,
            context,
            drop_context,
        });
        tyrus_promise_on_settle(
            source,
            Some(source_wake),
            Arc::into_raw(state).cast_mut().cast(),
        );
        output
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_then(
    source: *mut TyrusPromise,
    callback: TyrusThenCallback,
    context: *mut c_void,
    drop_context: TyrusDropContext,
) -> *mut TyrusPromise {
    unsafe { chain(source, ChainCallback::Then(callback), context, drop_context) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_catch(
    source: *mut TyrusPromise,
    callback: TyrusCatchCallback,
    context: *mut c_void,
    drop_context: TyrusDropContext,
) -> *mut TyrusPromise {
    unsafe {
        chain(
            source,
            ChainCallback::Catch(callback),
            context,
            drop_context,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_finally(
    source: *mut TyrusPromise,
    callback: TyrusFinallyCallback,
    context: *mut c_void,
    drop_context: TyrusDropContext,
) -> *mut TyrusPromise {
    unsafe {
        chain(
            source,
            ChainCallback::Finally(callback),
            context,
            drop_context,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        TYRUS_PROMISE_CANCELLED, TYRUS_PROMISE_FULFILLED, TYRUS_PROMISE_PENDING,
        TYRUS_PROMISE_REJECTED, tyrus_async_block_on, tyrus_promise_cancel_pending,
        tyrus_promise_free, tyrus_promise_fulfill_owned, tyrus_promise_reject,
        tyrus_promise_resolve, tyrus_promise_resolve_void, tyrus_promise_state,
        tyrus_promise_take_value,
    };

    use std::sync::Barrier;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct GatedCallback {
        calls: AtomicUsize,
        gate: Barrier,
    }

    unsafe extern "C" fn drop_i32(value: *mut c_void) {
        unsafe {
            drop(Box::from_raw(value.cast::<i32>()));
        }
    }

    unsafe extern "C" fn gated_double_value(
        value: *mut c_void,
        context: *mut c_void,
    ) -> *mut TyrusPromise {
        unsafe {
            let context = &*context.cast::<GatedCallback>();
            context.gate.wait();
            context.calls.fetch_add(1, Ordering::SeqCst);
            let value = *value.cast::<i32>();
            tyrus_promise_resolve(Box::into_raw(Box::new(value * 2)).cast(), Some(drop_i32))
        }
    }

    unsafe extern "C" fn recover(message: TyrusString, context: *mut c_void) -> *mut TyrusPromise {
        unsafe {
            assert_eq!(message.as_str(), "boom");
            let calls = &*context.cast::<AtomicUsize>();
            calls.fetch_add(1, Ordering::SeqCst);
            tyrus_promise_resolve(Box::into_raw(Box::new(7_i32)).cast(), Some(drop_i32))
        }
    }

    unsafe extern "C" fn finalize(context: *mut c_void) -> *mut TyrusPromise {
        unsafe {
            let calls = &*context.cast::<AtomicUsize>();
            calls.fetch_add(1, Ordering::SeqCst);
            tyrus_promise_resolve_void()
        }
    }

    #[test]
    fn then_runs_on_the_executor_and_flattens_the_returned_promise() {
        let context = GatedCallback {
            calls: AtomicUsize::new(0),
            gate: Barrier::new(2),
        };
        unsafe {
            let source =
                tyrus_promise_resolve(Box::into_raw(Box::new(21_i32)).cast(), Some(drop_i32));
            let output = tyrus_promise_then(
                source,
                Some(gated_double_value),
                (&context as *const GatedCallback).cast_mut().cast(),
                None,
            );
            context.gate.wait();
            assert_eq!(tyrus_async_block_on(output), 0);
            assert_eq!(context.calls.load(Ordering::SeqCst), 1);
            let value = tyrus_promise_take_value(output);
            assert_eq!(*value.cast::<i32>(), 42);
            drop(Box::from_raw(value.cast::<i32>()));
            tyrus_promise_free(output);
            tyrus_promise_free(source);
        }
    }

    #[test]
    fn direct_adoption_transfers_immediate_and_pending_payloads() {
        unsafe {
            let immediate =
                tyrus_promise_resolve(Box::into_raw(Box::new(21_i32)).cast(), Some(drop_i32));
            let immediate_target = tyrus_promise_pending();
            assert_eq!(tyrus_promise_adopt(immediate, immediate_target), 1);
            assert_eq!(
                tyrus_promise_state(immediate_target),
                TYRUS_PROMISE_FULFILLED
            );
            let value = tyrus_promise_take_value(immediate_target);
            assert_eq!(*value.cast::<i32>(), 21);
            drop(Box::from_raw(value.cast::<i32>()));
            tyrus_promise_free(immediate_target);
            tyrus_promise_free(immediate);

            let pending = tyrus_promise_pending();
            let pending_target = tyrus_promise_pending();
            assert_eq!(tyrus_promise_adopt(pending, pending_target), 1);
            assert_eq!(tyrus_promise_state(pending_target), TYRUS_PROMISE_PENDING);
            assert_eq!(
                tyrus_promise_fulfill_owned(
                    pending,
                    Box::into_raw(Box::new(42_i32)).cast(),
                    Some(drop_i32),
                ),
                1
            );
            assert_eq!(tyrus_promise_state(pending_target), TYRUS_PROMISE_FULFILLED);
            let value = tyrus_promise_take_value(pending_target);
            assert_eq!(*value.cast::<i32>(), 42);
            drop(Box::from_raw(value.cast::<i32>()));
            tyrus_promise_free(pending_target);
            tyrus_promise_free(pending);
        }
    }

    #[test]
    fn direct_adoption_preserves_rejection_and_cancellation() {
        unsafe {
            let rejected = tyrus_promise_reject(TyrusString::literal("boom"));
            let rejected_target = tyrus_promise_pending();
            assert_eq!(tyrus_promise_adopt(rejected, rejected_target), 1);
            assert_eq!(
                tyrus_async_block_on(rejected_target),
                TYRUS_PROMISE_REJECTED
            );
            assert_eq!(
                crate::tyrus_promise_error_message(rejected_target).as_str(),
                "boom"
            );
            tyrus_promise_free(rejected_target);
            tyrus_promise_free(rejected);

            let cancelled = tyrus_promise_pending();
            let cancelled_target = tyrus_promise_pending();
            assert_eq!(tyrus_promise_adopt(cancelled, cancelled_target), 1);
            assert_eq!(
                tyrus_promise_cancel_pending(cancelled, TyrusString::literal("stop")),
                1
            );
            assert_eq!(
                tyrus_promise_state(cancelled_target),
                TYRUS_PROMISE_CANCELLED
            );
            tyrus_promise_free(cancelled_target);
            tyrus_promise_free(cancelled);
        }
    }

    #[test]
    fn catch_recovers_and_finally_preserves_the_original_result() {
        let recover_calls = AtomicUsize::new(0);
        let finally_calls = AtomicUsize::new(0);
        unsafe {
            let rejected = tyrus_promise_reject(TyrusString::literal("boom"));
            let recovered = tyrus_promise_catch(
                rejected,
                Some(recover),
                (&recover_calls as *const AtomicUsize).cast_mut().cast(),
                None,
            );
            let finalized = tyrus_promise_finally(
                recovered,
                Some(finalize),
                (&finally_calls as *const AtomicUsize).cast_mut().cast(),
                None,
            );
            assert_eq!(tyrus_async_block_on(finalized), 0);
            assert_eq!(recover_calls.load(Ordering::SeqCst), 1);
            assert_eq!(finally_calls.load(Ordering::SeqCst), 1);
            let value = tyrus_promise_take_value(finalized);
            assert_eq!(*value.cast::<i32>(), 7);
            drop(Box::from_raw(value.cast::<i32>()));
            tyrus_promise_free(finalized);
            tyrus_promise_free(recovered);
            tyrus_promise_free(rejected);
        }
    }
}
