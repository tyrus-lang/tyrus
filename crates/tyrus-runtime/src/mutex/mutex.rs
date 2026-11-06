use std::ffi::c_void;
use std::sync::Arc;

use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

use crate::TyrusPromise;
use crate::abi::TyrusPromiseDrop;
use crate::executor::executor;
use crate::promise::{tyrus_promise_fulfill, tyrus_promise_pending, tyrus_promise_retain};

pub struct TyrusMutex {
    lock: Arc<AsyncMutex<()>>,
    storage: *mut c_void,
    drop_value: TyrusPromiseDrop,
}

// SAFETY: `storage` is only ever read or written while the associated
// `lock` is held by whichever `TyrusMutexGuard` currently exists, the same
// mutual exclusion a real `std`/`tokio` `Mutex<T>` would provide, just
// enforced across the FFI boundary instead of by the type system.
unsafe impl Send for TyrusMutex {}
unsafe impl Sync for TyrusMutex {}

impl Drop for TyrusMutex {
    fn drop(&mut self) {
        if !self.storage.is_null()
            && let Some(drop_value) = self.drop_value
        {
            unsafe { drop_value(self.storage) };
        }
    }
}

pub struct TyrusMutexGuard {
    mutex: *const TyrusMutex,
    _permit: OwnedMutexGuard<()>,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_mutex_new(
    value: *mut c_void,
    drop_value: TyrusPromiseDrop,
) -> *const TyrusMutex {
    let mutex = Arc::new(TyrusMutex {
        lock: Arc::new(AsyncMutex::new(())),
        storage: value,
        drop_value,
    });
    Arc::into_raw(mutex)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_mutex_retain(mutex: *const TyrusMutex) -> *const TyrusMutex {
    unsafe {
        Arc::increment_strong_count(mutex);
        mutex
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_mutex_free(mutex: *const TyrusMutex) {
    unsafe {
        if !mutex.is_null() {
            drop(Arc::from_raw(mutex));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_mutex_lock(mutex: *const TyrusMutex) -> *mut TyrusPromise {
    unsafe {
        let mutex_arc = Arc::from_raw(mutex);
        let mutex_arc = std::mem::ManuallyDrop::new(mutex_arc);
        let mutex_clone: Arc<TyrusMutex> = Arc::clone(&mutex_arc);
        let promise = tyrus_promise_pending();
        tyrus_promise_retain(promise);
        let promise_address = promise as usize;
        let lock = Arc::clone(&mutex_clone.lock);
        executor().spawn(async move {
            let permit = lock.lock_owned().await;
            let guard = Box::new(TyrusMutexGuard {
                mutex: Arc::into_raw(mutex_clone),
                _permit: permit,
            });
            let guard_ptr: *mut TyrusMutexGuard = Box::into_raw(guard);
            let boxed = libc::malloc(std::mem::size_of::<*mut TyrusMutexGuard>())
                as *mut *mut TyrusMutexGuard;
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
pub unsafe extern "C" fn tyrus_mutex_try_lock(mutex: *const TyrusMutex) -> *mut c_void {
    unsafe {
        let mutex_ref = &*mutex;
        let Ok(permit) = Arc::clone(&mutex_ref.lock).try_lock_owned() else {
            return std::ptr::null_mut();
        };
        tyrus_mutex_retain(mutex);
        let guard = Box::new(TyrusMutexGuard {
            mutex,
            _permit: permit,
        });
        Box::into_raw(guard).cast()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_mutex_guard_storage(guard: *const TyrusMutexGuard) -> *mut c_void {
    unsafe { (*(*guard).mutex).storage }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_mutex_guard_unlock(guard: *mut TyrusMutexGuard) {
    unsafe {
        if guard.is_null() {
            return;
        }
        let guard = Box::from_raw(guard);
        tyrus_mutex_free(guard.mutex);
    }
}
