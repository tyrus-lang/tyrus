use crate::promise::{
    TYRUS_PROMISE_FULFILLED, TYRUS_PROMISE_PENDING, TyrusPromise, TyrusPromiseDrop,
    promise_error_string, rejected_aggregate, tyrus_promise_take_payload,
};
use crate::{
    tyrus_promise_free, tyrus_promise_fulfill, tyrus_promise_on_settle,
    tyrus_promise_propagate_failure, tyrus_promise_retain, tyrus_promise_state,
};
use std::ffi::c_void;
use std::ptr;
use std::sync::{Arc, Mutex};
use tyrus_string::TyrusString;

unsafe extern "C" {
    fn free(value: *mut c_void);
    fn malloc(size: usize) -> *mut c_void;
}

const MAX_COMBINATOR_INPUTS: usize = 1_000_000;

#[repr(C)]
pub struct TyrusOwnedPromisePayload {
    pub value: *mut c_void,
    pub drop_value: TyrusPromiseDrop,
}

unsafe impl Send for TyrusOwnedPromisePayload {}

#[repr(C)]
pub struct TyrusPromisePayloadList {
    pub items: *mut TyrusOwnedPromisePayload,
    pub len: usize,
}

#[repr(C)]
pub struct TyrusSettledPromisePayload {
    pub state: i32,
    pub value: *mut c_void,
    pub drop_value: TyrusPromiseDrop,
    pub error: TyrusString,
}

#[repr(C)]
pub struct TyrusSettledPromiseList {
    pub items: *mut TyrusSettledPromisePayload,
    pub len: usize,
}

#[repr(C)]
struct TyrusArray {
    data: *mut c_void,
    len: usize,
    capacity: usize,
}

struct TypedAllContext {
    element_size: usize,
    drop_array: TyrusPromiseDrop,
}

struct TypedAllSettledContext {
    element_size: usize,
    result_size: usize,
    payload_offset: usize,
    fulfilled_value_offset: usize,
    rejected_reason_offset: usize,
    rejected_code_offset: usize,
    drop_array: TyrusPromiseDrop,
}

unsafe extern "C" fn drop_typed_all_context(context: *mut c_void) {
    unsafe {
        drop(Box::from_raw(context.cast::<TypedAllContext>()));
    }
}

unsafe extern "C" fn drop_typed_all_settled_context(context: *mut c_void) {
    unsafe {
        drop(Box::from_raw(context.cast::<TypedAllSettledContext>()));
    }
}

unsafe extern "C" fn materialize_typed_all(
    value: *mut c_void,
    context: *mut c_void,
) -> *mut TyrusPromise {
    unsafe {
        let list = &mut *value.cast::<TyrusPromisePayloadList>();
        let context = &*context.cast::<TypedAllContext>();
        let Some(byte_len) = list.len.checked_mul(context.element_size) else {
            return crate::tyrus_promise_reject(TyrusString::literal(
                "Promise.all result is too large",
            ));
        };
        let data = if byte_len == 0 {
            ptr::null_mut()
        } else {
            malloc(byte_len)
        };
        if byte_len != 0 && data.is_null() {
            return crate::tyrus_promise_reject(TyrusString::literal(
                "Promise.all could not allocate its result",
            ));
        }
        for index in 0..list.len {
            let item = &mut *list.items.add(index);
            if context.element_size == 0 {
                if !item.value.is_null() {
                    match item.drop_value {
                        Some(drop_value) => drop_value(item.value),
                        None => free(item.value),
                    }
                }
                item.value = ptr::null_mut();
                item.drop_value = None;
                continue;
            }
            if item.value.is_null() {
                free(data);
                return crate::tyrus_promise_reject(TyrusString::literal(
                    "Promise.all received an empty payload",
                ));
            }
            ptr::copy_nonoverlapping(
                item.value.cast::<u8>(),
                data.cast::<u8>().add(index * context.element_size),
                context.element_size,
            );
            free(item.value);
            item.value = ptr::null_mut();
            item.drop_value = None;
        }
        let array = Box::new(TyrusArray {
            data,
            len: list.len,
            capacity: list.len,
        });
        crate::tyrus_promise_resolve(Box::into_raw(array).cast(), context.drop_array)
    }
}

unsafe extern "C" fn materialize_typed_all_settled(
    value: *mut c_void,
    context: *mut c_void,
) -> *mut TyrusPromise {
    unsafe {
        let list = &mut *value.cast::<TyrusSettledPromiseList>();
        let context = &*context.cast::<TypedAllSettledContext>();
        let Some(byte_len) = list.len.checked_mul(context.result_size) else {
            return crate::tyrus_promise_reject(TyrusString::literal(
                "Promise.allSettled result is too large",
            ));
        };
        let data = if byte_len == 0 {
            ptr::null_mut()
        } else {
            malloc(byte_len)
        };
        if byte_len != 0 && data.is_null() {
            return crate::tyrus_promise_reject(TyrusString::literal(
                "Promise.allSettled could not allocate its result",
            ));
        }
        if byte_len != 0 {
            ptr::write_bytes(data, 0, byte_len);
        }
        for index in 0..list.len {
            let item = &mut *list.items.add(index);
            let target = data.cast::<u8>().add(index * context.result_size);
            if item.state == TYRUS_PROMISE_FULFILLED {
                *target = 0;
                *target.add(context.payload_offset).cast::<TyrusString>() =
                    TyrusString::literal("fulfilled");
                if !item.value.is_null() {
                    ptr::copy_nonoverlapping(
                        item.value.cast::<u8>(),
                        target.add(context.fulfilled_value_offset),
                        context.element_size,
                    );
                    free(item.value);
                    item.value = ptr::null_mut();
                    item.drop_value = None;
                }
            } else {
                *target = 1;
                *target.add(context.payload_offset).cast::<TyrusString>() =
                    TyrusString::literal("rejected");
                // The reason moves into the struct, so the payload stops owning it,
                // one owner at a time is what keeps this from double freeing.
                *target
                    .add(context.rejected_reason_offset)
                    .cast::<TyrusString>() =
                    std::mem::replace(&mut item.error, TyrusString::null());
                *target
                    .add(context.rejected_code_offset)
                    .cast::<TyrusString>() = TyrusString::literal("ERR_RUNTIME");
            }
        }
        let array = Box::new(TyrusArray {
            data,
            len: list.len,
            capacity: list.len,
        });
        crate::tyrus_promise_resolve(Box::into_raw(array).cast(), context.drop_array)
    }
}

unsafe extern "C" fn drop_settled_list(value: *mut c_void) {
    unsafe {
        if value.is_null() {
            return;
        }
        let list = Box::from_raw(value.cast::<TyrusSettledPromiseList>());
        if list.len == 0 {
            return;
        }
        let items = Vec::from_raw_parts(list.items, list.len, list.len);
        for item in items {
            if !item.value.is_null()
                && let Some(drop_value) = item.drop_value
            {
                drop_value(item.value);
            }
            item.error.release();
        }
    }
}

unsafe extern "C" fn drop_payload_list(value: *mut c_void) {
    unsafe {
        if value.is_null() {
            return;
        }
        let list = Box::from_raw(value.cast::<TyrusPromisePayloadList>());
        if list.len == 0 {
            return;
        }
        let items = Vec::from_raw_parts(list.items, list.len, list.len);
        for payload in items {
            if !payload.value.is_null()
                && let Some(drop_value) = payload.drop_value
            {
                drop_value(payload.value);
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Mode {
    All,
    Race,
    Any,
    AllSettled,
}

struct CombinatorData {
    remaining: usize,
    finished: bool,
    errors: Vec<Option<String>>,
}

struct CombinatorState {
    mode: Mode,
    inputs: Vec<*mut TyrusPromise>,
    output: *mut TyrusPromise,
    data: Mutex<CombinatorData>,
}

unsafe impl Send for CombinatorState {}
unsafe impl Sync for CombinatorState {}

impl Drop for CombinatorState {
    fn drop(&mut self) {
        unsafe {
            for input in &self.inputs {
                tyrus_promise_free(*input);
            }
            tyrus_promise_free(self.output);
        }
    }
}

struct WaitContext {
    state: Arc<CombinatorState>,
    index: usize,
}

unsafe extern "C" fn combinator_wake(context: *mut c_void) {
    unsafe {
        let context = Box::from_raw(context.cast::<WaitContext>());
        settle_input(&context.state, context.index);
    }
}

unsafe fn fulfill_with_payload(output: *mut TyrusPromise, input: *mut TyrusPromise) {
    unsafe {
        let (value, drop_value) = tyrus_promise_take_payload(input);
        tyrus_promise_fulfill(output, value, drop_value);
    }
}

unsafe fn settle_input(state: &Arc<CombinatorState>, index: usize) {
    unsafe {
        let input = state.inputs[index];
        let input_state = tyrus_promise_state(input);
        if input_state == TYRUS_PROMISE_PENDING {
            return;
        }
        let mut data = state
            .data
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if data.remaining == 0 {
            return;
        }
        data.remaining -= 1;
        match state.mode {
            Mode::Race if !data.finished => {
                data.finished = true;
                if input_state == TYRUS_PROMISE_FULFILLED {
                    fulfill_with_payload(state.output, input);
                } else {
                    tyrus_promise_propagate_failure(input, state.output);
                }
            }
            Mode::All if input_state != TYRUS_PROMISE_FULFILLED && !data.finished => {
                data.finished = true;
                tyrus_promise_propagate_failure(input, state.output);
            }
            Mode::Any if input_state == TYRUS_PROMISE_FULFILLED && !data.finished => {
                data.finished = true;
                fulfill_with_payload(state.output, input);
            }
            Mode::Any if input_state != TYRUS_PROMISE_FULFILLED => {
                data.errors[index] = Some(promise_error_string(input));
            }
            _ => {}
        }
        if data.remaining != 0 || data.finished {
            return;
        }
        data.finished = true;
        match state.mode {
            Mode::All => {
                let mut payloads = Vec::new();
                if payloads.try_reserve_exact(state.inputs.len()).is_err() {
                    crate::tyrus_promise_reject_pending(
                        state.output,
                        TyrusString::literal("Promise.all could not allocate its result"),
                    );
                    return;
                }
                for input in &state.inputs {
                    let (value, drop_value) = tyrus_promise_take_payload(*input);
                    payloads.push(TyrusOwnedPromisePayload { value, drop_value });
                }
                let mut payloads = payloads.into_boxed_slice();
                let list = Box::new(TyrusPromisePayloadList {
                    items: payloads.as_mut_ptr(),
                    len: payloads.len(),
                });
                std::mem::forget(payloads);
                tyrus_promise_fulfill(
                    state.output,
                    Box::into_raw(list).cast(),
                    Some(drop_payload_list),
                );
            }
            Mode::Any => {
                let errors = data
                    .errors
                    .iter_mut()
                    .map(|error| {
                        error
                            .take()
                            .unwrap_or_else(|| "Promise cancelled".to_string())
                    })
                    .collect();
                let aggregate = rejected_aggregate(errors);
                tyrus_promise_propagate_failure(aggregate, state.output);
                tyrus_promise_free(aggregate);
            }
            Mode::AllSettled => {
                let mut entries = Vec::new();
                if entries.try_reserve_exact(state.inputs.len()).is_err() {
                    crate::tyrus_promise_reject_pending(
                        state.output,
                        TyrusString::literal("Promise.allSettled could not allocate its result"),
                    );
                    return;
                }
                for input in &state.inputs {
                    let input_state = tyrus_promise_state(*input);
                    let (value, drop_value, error) = if input_state == TYRUS_PROMISE_FULFILLED {
                        let (value, drop_value) = tyrus_promise_take_payload(*input);
                        (value, drop_value, TyrusString::null())
                    } else {
                        let error = crate::strings::alloc_string(&promise_error_string(*input));
                        let error = if error.is_null() {
                            crate::strings::alloc_string("Promise rejected")
                        } else {
                            error
                        };
                        (ptr::null_mut(), None, error)
                    };
                    entries.push(TyrusSettledPromisePayload {
                        state: input_state,
                        value,
                        drop_value,
                        error,
                    });
                }
                let mut entries = entries.into_boxed_slice();
                let list = Box::new(TyrusSettledPromiseList {
                    items: entries.as_mut_ptr(),
                    len: entries.len(),
                });
                std::mem::forget(entries);
                tyrus_promise_fulfill(
                    state.output,
                    Box::into_raw(list).cast(),
                    Some(drop_settled_list),
                );
            }
            Mode::Race => unreachable!(),
        }
    }
}

unsafe fn combinator(
    inputs: *const *mut TyrusPromise,
    len: usize,
    mode: Mode,
) -> *mut TyrusPromise {
    unsafe {
        if len > MAX_COMBINATOR_INPUTS
            || len
                .checked_mul(std::mem::size_of::<*mut TyrusPromise>())
                .is_none()
        {
            return crate::tyrus_promise_reject(TyrusString::literal(
                "Promise combinator input is too large",
            ));
        }
        if len == 0 {
            return match mode {
                Mode::All => {
                    let list = Box::new(TyrusPromisePayloadList {
                        items: ptr::null_mut(),
                        len: 0,
                    });
                    crate::tyrus_promise_resolve(
                        Box::into_raw(list).cast(),
                        Some(drop_payload_list),
                    )
                }
                Mode::AllSettled => {
                    let list = Box::new(TyrusSettledPromiseList {
                        items: ptr::null_mut(),
                        len: 0,
                    });
                    crate::tyrus_promise_resolve(
                        Box::into_raw(list).cast(),
                        Some(drop_settled_list),
                    )
                }
                Mode::Any => rejected_aggregate(Vec::new()),
                Mode::Race => crate::tyrus_promise_pending(),
            };
        }
        if inputs.is_null() {
            return crate::tyrus_promise_reject(TyrusString::literal(
                "Promise combinator received null inputs",
            ));
        }
        let inputs = std::slice::from_raw_parts(inputs, len).to_vec();
        let output = crate::tyrus_promise_pending();
        tyrus_promise_retain(output);
        for input in &inputs {
            tyrus_promise_retain(*input);
        }
        let state = Arc::new(CombinatorState {
            mode,
            inputs,
            output,
            data: Mutex::new(CombinatorData {
                remaining: len,
                finished: false,
                errors: vec![None; len],
            }),
        });
        for index in 0..len {
            let context = Box::new(WaitContext {
                state: state.clone(),
                index,
            });
            tyrus_promise_on_settle(
                state.inputs[index],
                Some(combinator_wake),
                Box::into_raw(context).cast(),
            );
        }
        drop(state);
        output
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_all(
    inputs: *const *mut TyrusPromise,
    len: usize,
) -> *mut TyrusPromise {
    unsafe { combinator(inputs, len, Mode::All) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_race(
    inputs: *const *mut TyrusPromise,
    len: usize,
) -> *mut TyrusPromise {
    unsafe { combinator(inputs, len, Mode::Race) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_any(
    inputs: *const *mut TyrusPromise,
    len: usize,
) -> *mut TyrusPromise {
    unsafe { combinator(inputs, len, Mode::Any) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_all_settled(
    inputs: *const *mut TyrusPromise,
    len: usize,
) -> *mut TyrusPromise {
    unsafe { combinator(inputs, len, Mode::AllSettled) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_all_typed(
    inputs: *const *mut TyrusPromise,
    len: usize,
    element_size: usize,
    drop_array: TyrusPromiseDrop,
) -> *mut TyrusPromise {
    unsafe {
        let raw = tyrus_promise_all(inputs, len);
        let context = Box::new(TypedAllContext {
            element_size,
            drop_array,
        });
        let output = crate::promise_chains::tyrus_promise_then(
            raw,
            Some(materialize_typed_all),
            Box::into_raw(context).cast(),
            Some(drop_typed_all_context),
        );
        tyrus_promise_free(raw);
        output
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_all_settled_typed(
    inputs: *const *mut TyrusPromise,
    len: usize,
    element_size: usize,
    result_size: usize,
    payload_offset: usize,
    fulfilled_value_offset: usize,
    rejected_reason_offset: usize,
    rejected_code_offset: usize,
    drop_array: TyrusPromiseDrop,
) -> *mut TyrusPromise {
    unsafe {
        if element_size == 0 || result_size == 0 {
            return crate::tyrus_promise_reject(TyrusString::literal(
                "Promise.allSettled requires sized payloads",
            ));
        }
        let raw = tyrus_promise_all_settled(inputs, len);
        let context = Box::new(TypedAllSettledContext {
            element_size,
            result_size,
            payload_offset,
            fulfilled_value_offset,
            rejected_reason_offset,
            rejected_code_offset,
            drop_array,
        });
        let output = crate::promise_chains::tyrus_promise_then(
            raw,
            Some(materialize_typed_all_settled),
            Box::into_raw(context).cast(),
            Some(drop_typed_all_settled_context),
        );
        tyrus_promise_free(raw);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        TYRUS_PROMISE_REJECTED, tyrus_promise_error_message, tyrus_promise_pending,
        tyrus_promise_reject_pending, tyrus_promise_resolve, tyrus_promise_state,
    };

    unsafe extern "C" fn drop_i32(value: *mut c_void) {
        unsafe {
            drop(Box::from_raw(value.cast::<i32>()));
        }
    }

    fn resolved(value: i32) -> *mut TyrusPromise {
        tyrus_promise_resolve(Box::into_raw(Box::new(value)).cast(), Some(drop_i32))
    }

    unsafe extern "C" fn drop_test_array(value: *mut c_void) {
        unsafe {
            let array = Box::from_raw(value.cast::<TyrusArray>());
            free(array.data);
        }
    }

    #[test]
    fn typed_combinators_materialize_language_array_layouts() {
        unsafe {
            let inputs = [resolved(20), resolved(22)];
            let output = tyrus_promise_all_typed(
                inputs.as_ptr(),
                inputs.len(),
                std::mem::size_of::<i32>(),
                Some(drop_test_array),
            );
            assert_eq!(crate::tyrus_async_block_on(output), 0);
            let (payload, drop_payload) = tyrus_promise_take_payload(output);
            let array = &*payload.cast::<TyrusArray>();
            assert_eq!(array.len, 2);
            assert_eq!(
                std::slice::from_raw_parts(array.data.cast::<i32>(), 2),
                &[20, 22]
            );
            drop_payload.unwrap()(payload);
            tyrus_promise_free(output);
            for input in inputs {
                tyrus_promise_free(input);
            }

            let settled_inputs = [
                resolved(42),
                crate::tyrus_promise_reject(TyrusString::literal("expected")),
            ];
            const RESULT_SIZE: usize = 56;
            const STATUS_OFFSET: usize = 8;
            const PAYLOAD_OFFSET: usize = 24;
            const REASON_CODE_OFFSET: usize = 40;
            let settled = tyrus_promise_all_settled_typed(
                settled_inputs.as_ptr(),
                settled_inputs.len(),
                std::mem::size_of::<i32>(),
                RESULT_SIZE,
                STATUS_OFFSET,
                PAYLOAD_OFFSET,
                PAYLOAD_OFFSET,
                REASON_CODE_OFFSET,
                Some(drop_test_array),
            );
            assert_eq!(crate::tyrus_async_block_on(settled), 0);
            let (payload, _) = tyrus_promise_take_payload(settled);
            let array = Box::from_raw(payload.cast::<TyrusArray>());
            let first = array.data.cast::<u8>();
            let second = first.add(RESULT_SIZE);
            assert_eq!(*first, 0);
            assert_eq!(*first.add(PAYLOAD_OFFSET).cast::<i32>(), 42);
            assert_eq!(*second, 1);
            let reason = &*second.add(PAYLOAD_OFFSET).cast::<TyrusString>();
            assert_eq!(reason.as_str(), "expected");
            std::ptr::read(reason as *const TyrusString).release();
            let code = &*second.add(REASON_CODE_OFFSET).cast::<TyrusString>();
            assert_eq!(code.as_str(), "ERR_RUNTIME");
            free(array.data);
            tyrus_promise_free(settled);
            for input in settled_inputs {
                tyrus_promise_free(input);
            }
        }
    }

    #[test]
    fn typed_all_over_void_payloads_counts_inputs_without_a_buffer() {
        unsafe {
            let inputs = [
                crate::tyrus_promise_resolve_void(),
                crate::tyrus_promise_resolve_void(),
            ];
            let output =
                tyrus_promise_all_typed(inputs.as_ptr(), inputs.len(), 0, Some(drop_test_array));
            assert_eq!(crate::tyrus_async_block_on(output), 0);
            assert_ne!(tyrus_promise_state(output), TYRUS_PROMISE_REJECTED);
            let (payload, drop_payload) = tyrus_promise_take_payload(output);
            let array = &*payload.cast::<TyrusArray>();
            assert_eq!(array.len, 2);
            assert!(array.data.is_null());
            drop_payload.unwrap()(payload);
            tyrus_promise_free(output);
            for input in inputs {
                tyrus_promise_free(input);
            }
        }
    }

    #[test]
    fn all_preserves_input_order_and_transfers_owned_payloads() {
        unsafe {
            let inputs = [resolved(2), resolved(1), resolved(3)];
            let output = tyrus_promise_all(inputs.as_ptr(), inputs.len());
            assert_eq!(tyrus_promise_state(output), TYRUS_PROMISE_FULFILLED);
            let (payload, drop_payload) = tyrus_promise_take_payload(output);
            let list = &*payload.cast::<TyrusPromisePayloadList>();
            let items = std::slice::from_raw_parts(list.items, list.len);
            let values = items
                .iter()
                .map(|item| *item.value.cast::<i32>())
                .collect::<Vec<_>>();
            assert_eq!(values, vec![2, 1, 3]);
            drop_payload.unwrap()(payload);
            for input in inputs {
                tyrus_promise_free(input);
            }
            tyrus_promise_free(output);
        }
    }

    #[test]
    fn race_uses_the_first_observed_settlement_and_retains_losers() {
        unsafe {
            let first = tyrus_promise_pending();
            let second = tyrus_promise_pending();
            let inputs = [first, second];
            let output = tyrus_promise_race(inputs.as_ptr(), inputs.len());
            tyrus_promise_fulfill(
                second,
                Box::into_raw(Box::new(9_i32)).cast(),
                Some(drop_i32),
            );
            assert_eq!(tyrus_promise_state(output), TYRUS_PROMISE_FULFILLED);
            let value = crate::tyrus_promise_take_value(output);
            assert_eq!(*value.cast::<i32>(), 9);
            drop(Box::from_raw(value.cast::<i32>()));
            tyrus_promise_free(output);
            tyrus_promise_fulfill(first, Box::into_raw(Box::new(1_i32)).cast(), Some(drop_i32));
            tyrus_promise_free(first);
            tyrus_promise_free(second);
        }
    }

    #[test]
    fn any_aggregate_error_preserves_rejection_order() {
        unsafe {
            let first = tyrus_promise_pending();
            let second = tyrus_promise_pending();
            let inputs = [first, second];
            let output = tyrus_promise_any(inputs.as_ptr(), inputs.len());
            tyrus_promise_reject_pending(second, TyrusString::literal("second"));
            tyrus_promise_reject_pending(first, TyrusString::literal("first"));
            assert_eq!(tyrus_promise_state(output), TYRUS_PROMISE_REJECTED);
            assert_eq!(
                tyrus_promise_error_message(output).as_str(),
                "All promises were rejected"
            );
            assert_eq!(crate::tyrus_promise_aggregate_error_count(output), 2);
            assert_eq!(
                crate::tyrus_promise_aggregate_error_at(output, 0).as_str(),
                "first"
            );
            assert_eq!(
                crate::tyrus_promise_aggregate_error_at(output, 1).as_str(),
                "second"
            );
            tyrus_promise_free(output);
            tyrus_promise_free(first);
            tyrus_promise_free(second);
        }
    }

    #[test]
    fn all_settled_preserves_input_order_and_each_outcome() {
        unsafe {
            let fulfilled = resolved(7);
            let rejected = tyrus_promise_pending();
            tyrus_promise_reject_pending(rejected, TyrusString::literal("nope"));
            let inputs = [fulfilled, rejected];
            let output = tyrus_promise_all_settled(inputs.as_ptr(), inputs.len());
            assert_eq!(tyrus_promise_state(output), TYRUS_PROMISE_FULFILLED);
            let (payload, drop_payload) = tyrus_promise_take_payload(output);
            let list = &*payload.cast::<TyrusSettledPromiseList>();
            let entries = std::slice::from_raw_parts(list.items, list.len);
            assert_eq!(entries[0].state, TYRUS_PROMISE_FULFILLED);
            assert_eq!(*entries[0].value.cast::<i32>(), 7);
            assert_eq!(entries[1].state, TYRUS_PROMISE_REJECTED);
            assert_eq!(entries[1].error.as_str(), "nope");
            drop_payload.unwrap()(payload);
            tyrus_promise_free(output);
            tyrus_promise_free(fulfilled);
            tyrus_promise_free(rejected);
        }
    }

    #[test]
    fn empty_inputs_have_stable_semantics() {
        unsafe {
            let all = tyrus_promise_all(ptr::null(), 0);
            let any = tyrus_promise_any(ptr::null(), 0);
            let race = tyrus_promise_race(ptr::null(), 0);
            let settled = tyrus_promise_all_settled(ptr::null(), 0);
            assert_eq!(tyrus_promise_state(all), TYRUS_PROMISE_FULFILLED);
            assert_eq!(tyrus_promise_state(any), TYRUS_PROMISE_REJECTED);
            assert_eq!(tyrus_promise_state(race), TYRUS_PROMISE_PENDING);
            assert_eq!(tyrus_promise_state(settled), TYRUS_PROMISE_FULFILLED);
            tyrus_promise_free(all);
            tyrus_promise_free(any);
            tyrus_promise_free(race);
            tyrus_promise_free(settled);
        }
    }

    #[test]
    fn oversized_input_is_rejected_before_pointer_access_or_allocation() {
        unsafe {
            let output = tyrus_promise_all(ptr::null(), MAX_COMBINATOR_INPUTS + 1);
            assert_eq!(tyrus_promise_state(output), TYRUS_PROMISE_REJECTED);
            assert_eq!(
                tyrus_promise_error_message(output).as_str(),
                "Promise combinator input is too large"
            );
            tyrus_promise_free(output);
        }
    }
}
