use crate::promise::{TyrusPromise, TyrusPromiseDrop};
use crate::{
    tyrus_promise_free, tyrus_promise_fulfill, tyrus_promise_pending, tyrus_promise_reject_pending,
    tyrus_promise_retain,
};
use std::ffi::c_void;
use tyrus_string::TyrusString;

#[repr(C)]
pub struct TyrusPromiseCapability {
    promise: *mut TyrusPromise,
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_promise_with_resolvers() -> *mut TyrusPromiseCapability {
    Box::into_raw(Box::new(TyrusPromiseCapability {
        promise: tyrus_promise_pending(),
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_capability_promise(
    capability: *mut TyrusPromiseCapability,
) -> *mut TyrusPromise {
    unsafe {
        if capability.is_null() {
            return std::ptr::null_mut();
        }
        tyrus_promise_retain((*capability).promise)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_capability_resolve(
    capability: *mut TyrusPromiseCapability,
    value: *mut c_void,
    drop_value: TyrusPromiseDrop,
) -> i32 {
    unsafe {
        if capability.is_null() {
            if !value.is_null()
                && let Some(drop_value) = drop_value
            {
                drop_value(value);
            }
            return 0;
        }
        let settled = tyrus_promise_fulfill((*capability).promise, value, drop_value);
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
pub unsafe extern "C" fn tyrus_promise_capability_reject(
    capability: *mut TyrusPromiseCapability,
    message: TyrusString,
) -> i32 {
    unsafe {
        if capability.is_null() {
            return 0;
        }
        tyrus_promise_reject_pending((*capability).promise, message)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_capability_free(capability: *mut TyrusPromiseCapability) {
    unsafe {
        if capability.is_null() {
            return;
        }
        let capability = Box::from_raw(capability);
        tyrus_promise_free(capability.promise);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TYRUS_PROMISE_FULFILLED, tyrus_promise_free, tyrus_promise_state};

    #[test]
    fn capability_settles_once_and_keeps_the_promise_alive() {
        unsafe {
            let capability = tyrus_promise_with_resolvers();
            let promise = tyrus_promise_capability_promise(capability);
            assert_eq!(
                tyrus_promise_capability_resolve(capability, std::ptr::null_mut(), None),
                1
            );
            assert_eq!(
                tyrus_promise_capability_reject(capability, TyrusString::literal("late")),
                0
            );
            tyrus_promise_capability_free(capability);
            assert_eq!(tyrus_promise_state(promise), TYRUS_PROMISE_FULFILLED);
            tyrus_promise_free(promise);
        }
    }
}
