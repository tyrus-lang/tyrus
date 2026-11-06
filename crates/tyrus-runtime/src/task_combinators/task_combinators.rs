use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::executor::schedule_runtime_callback;
use crate::promise::{TyrusPromise, tyrus_promise_take_payload};
use crate::promise_combinators::tyrus_promise_race;
use crate::{
    TYRUS_PROMISE_FULFILLED, tyrus_promise_cancel_pending, tyrus_promise_fulfill,
    tyrus_promise_on_settle, tyrus_promise_pending, tyrus_promise_propagate_failure,
    tyrus_promise_reject_pending, tyrus_promise_retain, tyrus_promise_state,
    tyrus_task_sleep_async,
};
use tyrus_string::TyrusString;

struct RaceCancelContext {
    losers: Vec<*mut TyrusPromise>,
}

unsafe impl Send for RaceCancelContext {}

unsafe extern "C" fn cancel_race_losers(context: *mut c_void) {
    schedule_runtime_callback(cancel_race_losers_deferred, context);
}

unsafe extern "C" fn cancel_race_losers_deferred(context: *mut c_void) {
    unsafe {
        let context = Box::from_raw(context.cast::<RaceCancelContext>());
        for loser in context.losers {
            tyrus_promise_cancel_pending(loser, TyrusString::literal("lost the race"));
            crate::tyrus_promise_free(loser);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_task_race(
    handles: *const *mut TyrusPromise,
    len: usize,
) -> *mut TyrusPromise {
    unsafe {
        let output = tyrus_promise_race(handles, len);
        if len == 0 || handles.is_null() {
            return output;
        }
        let losers: Vec<*mut TyrusPromise> = std::slice::from_raw_parts(handles, len).to_vec();
        for loser in &losers {
            tyrus_promise_retain(*loser);
        }
        let context = Box::new(RaceCancelContext { losers });
        tyrus_promise_on_settle(
            output,
            Some(cancel_race_losers),
            Box::into_raw(context).cast(),
        );
        output
    }
}

struct TimeoutState {
    output: *mut TyrusPromise,
    task: *mut TyrusPromise,
    timer: *mut TyrusPromise,
    decided: AtomicBool,
}

unsafe impl Send for TimeoutState {}
unsafe impl Sync for TimeoutState {}

unsafe fn transfer_or_propagate(source: *mut TyrusPromise, output: *mut TyrusPromise) {
    unsafe {
        if tyrus_promise_state(source) == TYRUS_PROMISE_FULFILLED {
            let (value, drop_value) = tyrus_promise_take_payload(source);
            tyrus_promise_fulfill(output, value, drop_value);
        } else {
            tyrus_promise_propagate_failure(source, output);
        }
    }
}

unsafe extern "C" fn task_settled_first(context: *mut c_void) {
    unsafe {
        let state = Arc::from_raw(context.cast::<TimeoutState>());
        if state.decided.swap(true, Ordering::SeqCst) {
            return;
        }
        transfer_or_propagate(state.task, state.output);
        tyrus_promise_cancel_pending(state.timer, TyrusString::literal("task settled first"));
        crate::tyrus_promise_free(state.output);
        crate::tyrus_promise_free(state.task);
        crate::tyrus_promise_free(state.timer);
    }
}

unsafe extern "C" fn timer_settled_first(context: *mut c_void) {
    unsafe {
        let state = Arc::from_raw(context.cast::<TimeoutState>());
        if state.decided.swap(true, Ordering::SeqCst) {
            return;
        }
        tyrus_promise_reject_pending(
            state.output,
            TyrusString::literal("Timed out before the task completed"),
        );
        tyrus_promise_cancel_pending(state.task, TyrusString::literal("timed out"));
        crate::tyrus_promise_free(state.output);
        crate::tyrus_promise_free(state.task);
        crate::tyrus_promise_free(state.timer);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_task_timeout(ms: u64, task: *mut TyrusPromise) -> *mut TyrusPromise {
    unsafe {
        let output = tyrus_promise_pending();
        tyrus_promise_retain(output);
        tyrus_promise_retain(task);
        let timer = tyrus_task_sleep_async(ms);

        let state = Arc::new(TimeoutState {
            output,
            task,
            timer,
            decided: AtomicBool::new(false),
        });
        tyrus_promise_on_settle(
            task,
            Some(task_settled_first),
            Arc::into_raw(state.clone()) as *mut TimeoutState as *mut c_void,
        );
        tyrus_promise_on_settle(
            timer,
            Some(timer_settled_first),
            Arc::into_raw(state) as *mut TimeoutState as *mut c_void,
        );
        output
    }
}
