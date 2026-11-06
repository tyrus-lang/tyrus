use std::ffi::c_void;
use tyrus_string::TyrusString;

#[repr(C)]
struct TyrusArray {
    data: *mut c_void,
    len: usize,
    capacity: usize,
}

#[repr(C)]
pub struct TyrusAsyncError {
    message: TyrusString,
    code: TyrusString,
}

#[repr(C)]
pub struct TyrusAsyncErrorArray {
    data: *mut TyrusAsyncError,
    len: usize,
    capacity: usize,
}

#[repr(C)]
pub struct TyrusAsyncAggregateError {
    message: TyrusString,
    errors: TyrusAsyncErrorArray,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_clone_aggregate_error(
    promise: *const crate::promise::TyrusPromise,
) -> *mut TyrusAsyncAggregateError {
    unsafe {
        let (message, errors) = crate::promise::promise_error_parts(promise);
        let message = crate::strings::alloc_string(&message);
        let message = if message.is_null() {
            crate::strings::alloc_string("Promise rejected")
        } else {
            message
        };
        let mut errors = errors
            .into_iter()
            .map(|message| TyrusAsyncError {
                message: {
                    let allocated = crate::strings::alloc_string(&message);
                    if allocated.is_null() {
                        crate::strings::alloc_string("Promise rejected")
                    } else {
                        allocated
                    }
                },
                code: TyrusString::literal("ERR_RUNTIME"),
            })
            .collect::<Vec<_>>();
        let array = TyrusAsyncErrorArray {
            data: errors.as_mut_ptr(),
            len: errors.len(),
            capacity: errors.capacity(),
        };
        std::mem::forget(errors);
        Box::into_raw(Box::new(TyrusAsyncAggregateError {
            message,
            errors: array,
        }))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_drop_async_aggregate_error(
    aggregate: *mut TyrusAsyncAggregateError,
) {
    unsafe {
        if aggregate.is_null() {
            return;
        }
        let aggregate = Box::from_raw(aggregate);
        aggregate.message.release();
        if !aggregate.errors.data.is_null() {
            let errors = Vec::from_raw_parts(
                aggregate.errors.data,
                aggregate.errors.len,
                aggregate.errors.capacity,
            );
            for error in errors {
                error.message.release();
                error.code.release();
            }
        }
    }
}

unsafe extern "C" {
    fn free(value: *mut c_void);
}

#[repr(C)]
struct TyrusPromiseClosureContext {
    function: *mut c_void,
    environment: *mut c_void,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_drop_promise_closure_context(context: *mut c_void) {
    unsafe {
        if context.is_null() {
            return;
        }
        let closure = &*context.cast::<TyrusPromiseClosureContext>();
        if !closure.environment.is_null() {
            free(closure.environment);
        }
        free(context);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_drop_async_box(value: *mut c_void) {
    unsafe {
        free(value);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_drop_async_string(value: *mut c_void) {
    unsafe {
        if value.is_null() {
            return;
        }
        // Releasing the payload's string is the ownership bit's decision; the box
        // around it is still raw memory. The value is *taken* out of the slot
        // rather than borrowed from it: a borrow has its ownership bit cleared by
        // definition, so releasing one is a no-op and the text would leak.
        std::mem::replace(&mut *value.cast::<TyrusString>(), TyrusString::null()).release();
        free(value);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_drop_async_copy_array(value: *mut c_void) {
    unsafe {
        if value.is_null() {
            return;
        }
        let array = &*value.cast::<TyrusArray>();
        if !array.data.is_null() {
            free(array.data);
        }
        free(value);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_drop_async_string_array(value: *mut c_void) {
    unsafe {
        if value.is_null() {
            return;
        }
        let array = &*value.cast::<TyrusArray>();
        if !array.data.is_null() {
            let strings =
                std::slice::from_raw_parts_mut(array.data.cast::<TyrusString>(), array.len);
            for string in strings {
                std::mem::replace(string, TyrusString::null()).release();
            }
            free(array.data);
        }
        free(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_error_is_deep_cloned_for_an_async_catch_binding() {
        unsafe {
            let promise =
                crate::promise::rejected_aggregate(vec!["first".to_string(), "second".to_string()]);
            let aggregate = tyrus_promise_clone_aggregate_error(promise);
            crate::tyrus_promise_free(promise);
            assert_eq!((*aggregate).message.as_str(), "All promises were rejected");
            assert_eq!((*aggregate).errors.len, 2);
            assert_eq!((*(*aggregate).errors.data).message.as_str(), "first");
            assert_eq!(
                (*(*aggregate).errors.data.add(1)).message.as_str(),
                "second"
            );
            tyrus_drop_async_aggregate_error(aggregate);
        }
    }
}
