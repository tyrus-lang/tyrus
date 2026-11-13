use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::sync::Arc;

use tokio::sync::Semaphore as AsyncSemaphore;

use crate::TyrusPromise;
use crate::executor::executor;
use crate::promise::{tyrus_promise_fulfill, tyrus_promise_pending, tyrus_promise_retain};

pub type TyrusSemaphore = AsyncSemaphore;

pub struct TyrusSemaphorePermit {
    _permit: tokio::sync::OwnedSemaphorePermit,
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_semaphore_new(permits: usize) -> *const TyrusSemaphore {
    Arc::into_raw(Arc::new(AsyncSemaphore::new(permits)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_semaphore_retain(
    semaphore: *const TyrusSemaphore,
) -> *const TyrusSemaphore {
    unsafe {
        Arc::increment_strong_count(semaphore);
        semaphore
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_semaphore_free(semaphore: *const TyrusSemaphore) {
    unsafe {
        if !semaphore.is_null() {
            drop(Arc::from_raw(semaphore));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_semaphore_acquire(
    semaphore: *const TyrusSemaphore,
) -> *mut TyrusPromise {
    unsafe {
        let semaphore_arc = Arc::from_raw(semaphore);
        let semaphore_arc = ManuallyDrop::new(semaphore_arc);
        let semaphore_clone: Arc<TyrusSemaphore> = Arc::clone(&semaphore_arc);
        let promise = tyrus_promise_pending();
        tyrus_promise_retain(promise);
        let promise_address = promise as usize;
        executor().spawn(async move {
            let permit = semaphore_clone
                .acquire_owned()
                .await
                .expect("tyrus:task never closes a Semaphore");
            let guard = Box::new(TyrusSemaphorePermit { _permit: permit });
            let guard_ptr: *mut TyrusSemaphorePermit = Box::into_raw(guard);
            let boxed = libc::malloc(std::mem::size_of::<*mut TyrusSemaphorePermit>())
                as *mut *mut TyrusSemaphorePermit;
            std::ptr::write(boxed, guard_ptr);
            let promise = promise_address as *mut TyrusPromise;
            tyrus_promise_fulfill(
                promise,
                boxed.cast(),
                Some(crate::async_payload::tyrus_drop_async_box),
            );
            crate::promise::tyrus_promise_free(promise);
        });
        promise
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_semaphore_try_acquire(
    semaphore: *const TyrusSemaphore,
) -> *mut c_void {
    unsafe {
        let semaphore_arc = Arc::from_raw(semaphore);
        let semaphore_arc = ManuallyDrop::new(semaphore_arc);
        let semaphore_clone: Arc<TyrusSemaphore> = Arc::clone(&semaphore_arc);
        let Ok(permit) = semaphore_clone.try_acquire_owned() else {
            return std::ptr::null_mut();
        };
        let guard = Box::new(TyrusSemaphorePermit { _permit: permit });
        Box::into_raw(guard).cast()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_semaphore_permit_release(permit: *mut TyrusSemaphorePermit) {
    unsafe {
        if permit.is_null() {
            return;
        }
        drop(Box::from_raw(permit));
    }
}
