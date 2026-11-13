use std::os::raw::c_void;

type DropFn = unsafe extern "C" fn(*mut c_void);

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_closure_retain(environment: *mut c_void) {
    unsafe {
        if environment.is_null() {
            return;
        }
        let count = environment as *mut i64;
        *count += 1;
    }
}

/// One fewer owner shares this environment; the last one out tears it down.
///
/// The destructor at offset 8 frees the captures and then the block itself. A
/// null destructor means the environment has no owned captures, so the block is
/// freed directly. Releasing below zero cannot happen unless a retain was
/// missed at a copy site, that would be a use-after-free, which is why every
/// copy the compiler emits must be paired with exactly one release.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_closure_release(environment: *mut c_void) {
    unsafe {
        if environment.is_null() {
            return;
        }
        let count = environment as *mut i64;
        *count -= 1;
        if *count > 0 {
            return;
        }
        let drop_slot = (environment as *mut u8).add(8) as *mut Option<DropFn>;
        match *drop_slot {
            Some(drop_fn) => drop_fn(environment),
            None => free(environment),
        }
    }
}
