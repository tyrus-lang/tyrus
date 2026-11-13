use super::*;
use std::cell::Cell;
use std::ffi::CStr;
use std::mem::ManuallyDrop;
use tyrus_string::{TyrusString, ZeroizeError};

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_runtime_heap_malloc(size: usize) -> *mut c_void {
    unsafe { malloc(size) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_runtime_heap_calloc(count: usize, size: usize) -> *mut c_void {
    unsafe { calloc(count, size) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_runtime_heap_free(pointer: *mut c_void) {
    unsafe { free(pointer) };
}

#[repr(C)]
pub struct MemoryErrorTaken {
    pub code: TyrusString,
    pub message: TyrusString,
}

#[repr(C)]
union MemoryVoidResultPayload {
    error: ManuallyDrop<MemoryErrorTaken>,
    words: [u64; 4],
}

#[repr(C)]
pub struct MemoryVoidResult {
    tag: u8,
    payload: MemoryVoidResultPayload,
}

const _: () = {
    assert!(std::mem::size_of::<MemoryErrorTaken>() == 2 * std::mem::size_of::<TyrusString>());
    assert!(std::mem::size_of::<MemoryVoidResult>() == 40);
    assert!(std::mem::offset_of!(MemoryVoidResult, payload) == 8);
};

thread_local! {
    static LAST_MEMORY_ERROR: Cell<Option<(&'static CStr, &'static CStr)>> = const { Cell::new(None) };
}

fn set_memory_error(code: &'static CStr, message: &'static CStr) {
    LAST_MEMORY_ERROR.with(|slot| slot.set(Some((code, message))));
}

unsafe fn volatile_zero_bytes(pointer: *mut u8, length: usize) {
    for index in 0..length {
        unsafe { std::ptr::write_volatile(pointer.add(index), 0) };
    }
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_memory_clear_error() {
    LAST_MEMORY_ERROR.with(|slot| slot.set(None));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_memory_take_error(out: *mut MemoryErrorTaken) {
    LAST_MEMORY_ERROR.with(|slot| {
        if let Some((code, message)) = slot.take()
            && let Some(out) = unsafe { out.as_mut() }
        {
            out.code = TyrusString::literal(code.to_str().expect("a static error code is UTF-8"));
            out.message =
                TyrusString::literal(message.to_str().expect("a static error message is UTF-8"));
        }
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_memory_zeroize_string(value: TyrusString) -> i32 {
    match unsafe { value.zeroize_and_release() } {
        Ok(()) => 0,
        Err((ZeroizeError::NotOwned, preserved)) => {
            unsafe { preserved.release() };
            set_memory_error(
                c"ERR_MEMORY_NOT_OWNED",
                c"zeroize requires an exclusively owned dynamic string",
            );
            -1
        }
        Err((ZeroizeError::Shared, preserved)) => {
            unsafe { preserved.release() };
            set_memory_error(
                c"ERR_MEMORY_SHARED_VALUE",
                c"zeroize cannot erase a string with shared references",
            );
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_memory_zeroize_u8_array(array: *mut TyrusArray) -> i32 {
    let Some(array) = (unsafe { array.as_mut() }) else {
        set_memory_error(
            c"ERR_MEMORY_INVALID_VALUE",
            c"zeroize received an invalid array",
        );
        return -1;
    };
    if array.length > array.capacity || (array.length > 0 && array.data.is_null()) {
        set_memory_error(
            c"ERR_MEMORY_INVALID_VALUE",
            c"zeroize received an invalid array",
        );
        return -1;
    }
    unsafe { volatile_zero_bytes(array.data, array.length) };
    if !array.data.is_null() {
        unsafe { free(array.data.cast()) };
    }
    array.data = std::ptr::null_mut();
    array.length = 0;
    array.capacity = 0;
    0
}

unsafe fn write_memory_void_result(out: *mut MemoryVoidResult, status: i32) {
    if out.is_null() {
        return;
    }
    if status == 0 {
        unsafe {
            std::ptr::write(
                out,
                MemoryVoidResult {
                    tag: 0,
                    payload: MemoryVoidResultPayload { words: [0; 4] },
                },
            )
        };
        return;
    }
    let mut error = MemoryErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_memory_take_error(&mut error) };
    unsafe {
        std::ptr::write(
            out,
            MemoryVoidResult {
                tag: 1,
                payload: MemoryVoidResultPayload {
                    error: ManuallyDrop::new(error),
                },
            },
        )
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_memory_zeroize_string_result(
    value: TyrusString,
    out: *mut MemoryVoidResult,
) {
    tyrus_memory_clear_error();
    let status = unsafe { tyrus_memory_zeroize_string(value) };
    unsafe { write_memory_void_result(out, status) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_memory_zeroize_u8_array_result(
    array: *mut TyrusArray,
    out: *mut MemoryVoidResult,
) {
    tyrus_memory_clear_error();
    let status = unsafe { tyrus_memory_zeroize_u8_array(array) };
    unsafe { write_memory_void_result(out, status) };
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" {
        fn malloc(size: usize) -> *mut c_void;
    }

    #[test]
    fn zeroize_u8_array_clears_the_handle_after_releasing_it() {
        let allocation = unsafe { malloc(3) }.cast::<u8>();
        assert!(!allocation.is_null());
        unsafe { allocation.copy_from_nonoverlapping([1, 2, 3].as_ptr(), 3) };
        let mut array = TyrusArray {
            data: allocation,
            length: 3,
            capacity: 3,
        };
        assert_eq!(unsafe { tyrus_memory_zeroize_u8_array(&mut array) }, 0);
        assert!(array.data.is_null());
        assert_eq!(array.length, 0);
        assert_eq!(array.capacity, 0);
    }

    #[test]
    fn volatile_zero_bytes_erases_only_the_requested_range() {
        let allocation = unsafe { malloc(4) }.cast::<u8>();
        assert!(!allocation.is_null());
        unsafe { allocation.copy_from_nonoverlapping([1, 2, 3, 0xaa].as_ptr(), 4) };

        unsafe { volatile_zero_bytes(allocation, 3) };

        let contents = unsafe { std::slice::from_raw_parts(allocation, 4) };
        assert_eq!(contents, &[0, 0, 0, 0xaa]);
        unsafe { free(allocation.cast()) };
    }

    #[test]
    fn zeroize_u8_array_rejects_an_invalid_handle_without_freeing_it() {
        tyrus_memory_clear_error();
        let mut array = TyrusArray {
            data: std::ptr::null_mut(),
            length: 1,
            capacity: 1,
        };
        assert_eq!(unsafe { tyrus_memory_zeroize_u8_array(&mut array) }, -1);
        assert!(array.data.is_null());
        assert_eq!(array.length, 1);
        assert_eq!(array.capacity, 1);
    }
}
