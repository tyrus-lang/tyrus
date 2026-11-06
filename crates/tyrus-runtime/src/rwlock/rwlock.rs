use std::ffi::c_void;
use std::sync::Arc;

use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock as AsyncRwLock};

use crate::TyrusPromise;
use crate::abi::TyrusPromiseDrop;
use crate::executor::executor;
use crate::promise::{tyrus_promise_fulfill, tyrus_promise_pending, tyrus_promise_retain};

pub struct TyrusRwLock {
    lock: Arc<AsyncRwLock<()>>,
    storage: *mut c_void,
    drop_value: TyrusPromiseDrop,
}

// SAFETY: `storage` is only ever read while a `TyrusRwLockReadGuard` exists,
// or read/written while a `TyrusRwLockWriteGuard` exists, the same
// shared-or-exclusive access a real `std`/`tokio` `RwLock<T>` would provide,
// just enforced across the FFI boundary instead of by the type system.
unsafe impl Send for TyrusRwLock {}
unsafe impl Sync for TyrusRwLock {}

impl Drop for TyrusRwLock {
    fn drop(&mut self) {
        if !self.storage.is_null()
            && let Some(drop_value) = self.drop_value
        {
            unsafe { drop_value(self.storage) };
        }
    }
}

pub struct TyrusRwLockReadGuard {
    rwlock: *const TyrusRwLock,
    _permit: OwnedRwLockReadGuard<()>,
}

pub struct TyrusRwLockWriteGuard {
    rwlock: *const TyrusRwLock,
    _permit: OwnedRwLockWriteGuard<()>,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_rwlock_new(
    value: *mut c_void,
    drop_value: TyrusPromiseDrop,
) -> *const TyrusRwLock {
    let rwlock = Arc::new(TyrusRwLock {
        lock: Arc::new(AsyncRwLock::new(())),
        storage: value,
        drop_value,
    });
    Arc::into_raw(rwlock)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_rwlock_retain(rwlock: *const TyrusRwLock) -> *const TyrusRwLock {
    unsafe {
        Arc::increment_strong_count(rwlock);
        rwlock
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_rwlock_free(rwlock: *const TyrusRwLock) {
    unsafe {
        if !rwlock.is_null() {
            drop(Arc::from_raw(rwlock));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_rwlock_read(rwlock: *const TyrusRwLock) -> *mut TyrusPromise {
    unsafe {
        let rwlock_arc = Arc::from_raw(rwlock);
        let rwlock_arc = std::mem::ManuallyDrop::new(rwlock_arc);
        let rwlock_clone: Arc<TyrusRwLock> = Arc::clone(&rwlock_arc);
        let promise = tyrus_promise_pending();
        tyrus_promise_retain(promise);
        let promise_address = promise as usize;
        let lock = Arc::clone(&rwlock_clone.lock);
        executor().spawn(async move {
            let permit = lock.read_owned().await;
            let guard = Box::new(TyrusRwLockReadGuard {
                rwlock: Arc::into_raw(rwlock_clone),
                _permit: permit,
            });
            let guard_ptr: *mut TyrusRwLockReadGuard = Box::into_raw(guard);
            let boxed = libc::malloc(std::mem::size_of::<*mut TyrusRwLockReadGuard>())
                as *mut *mut TyrusRwLockReadGuard;
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
pub unsafe extern "C" fn tyrus_rwlock_write(rwlock: *const TyrusRwLock) -> *mut TyrusPromise {
    unsafe {
        let rwlock_arc = Arc::from_raw(rwlock);
        let rwlock_arc = std::mem::ManuallyDrop::new(rwlock_arc);
        let rwlock_clone: Arc<TyrusRwLock> = Arc::clone(&rwlock_arc);
        let promise = tyrus_promise_pending();
        tyrus_promise_retain(promise);
        let promise_address = promise as usize;
        let lock = Arc::clone(&rwlock_clone.lock);
        executor().spawn(async move {
            let permit = lock.write_owned().await;
            let guard = Box::new(TyrusRwLockWriteGuard {
                rwlock: Arc::into_raw(rwlock_clone),
                _permit: permit,
            });
            let guard_ptr: *mut TyrusRwLockWriteGuard = Box::into_raw(guard);
            let boxed = libc::malloc(std::mem::size_of::<*mut TyrusRwLockWriteGuard>())
                as *mut *mut TyrusRwLockWriteGuard;
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
pub unsafe extern "C" fn tyrus_rwlock_try_read(rwlock: *const TyrusRwLock) -> *mut c_void {
    unsafe {
        let rwlock_ref = &*rwlock;
        let Ok(permit) = Arc::clone(&rwlock_ref.lock).try_read_owned() else {
            return std::ptr::null_mut();
        };
        tyrus_rwlock_retain(rwlock);
        let guard = Box::new(TyrusRwLockReadGuard {
            rwlock,
            _permit: permit,
        });
        Box::into_raw(guard).cast()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_rwlock_try_write(rwlock: *const TyrusRwLock) -> *mut c_void {
    unsafe {
        let rwlock_ref = &*rwlock;
        let Ok(permit) = Arc::clone(&rwlock_ref.lock).try_write_owned() else {
            return std::ptr::null_mut();
        };
        tyrus_rwlock_retain(rwlock);
        let guard = Box::new(TyrusRwLockWriteGuard {
            rwlock,
            _permit: permit,
        });
        Box::into_raw(guard).cast()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_rwlock_read_guard_storage(
    guard: *const TyrusRwLockReadGuard,
) -> *mut c_void {
    unsafe { (*(*guard).rwlock).storage }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_rwlock_write_guard_storage(
    guard: *const TyrusRwLockWriteGuard,
) -> *mut c_void {
    unsafe { (*(*guard).rwlock).storage }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_rwlock_read_guard_unlock(guard: *mut TyrusRwLockReadGuard) {
    unsafe {
        if guard.is_null() {
            return;
        }
        let guard = Box::from_raw(guard);
        tyrus_rwlock_free(guard.rwlock);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_rwlock_write_guard_unlock(guard: *mut TyrusRwLockWriteGuard) {
    unsafe {
        if guard.is_null() {
            return;
        }
        let guard = Box::from_raw(guard);
        tyrus_rwlock_free(guard.rwlock);
    }
}
