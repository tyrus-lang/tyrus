use core::cell::UnsafeCell;

const MAX_OUTPUT_SIZE: usize = 64 * 1024;

struct OutputBuffer {
    buf: UnsafeCell<[u8; MAX_OUTPUT_SIZE]>,
    len: UnsafeCell<usize>,
}

// SAFETY: WASM is single-threaded, no concurrent access possible.
unsafe impl Sync for OutputBuffer {}

impl OutputBuffer {
    const fn new() -> Self {
        Self {
            buf: UnsafeCell::new([0u8; MAX_OUTPUT_SIZE]),
            len: UnsafeCell::new(0),
        }
    }

    unsafe fn write(&self, bytes: &[u8]) {
        unsafe {
            let len = &mut *self.len.get();
            let buf = &mut *self.buf.get();
            let space = MAX_OUTPUT_SIZE - *len;
            let to_write = if bytes.len() > space {
                &bytes[..space]
            } else {
                bytes
            };
            buf[*len..*len + to_write.len()].copy_from_slice(to_write);
            *len += to_write.len();
        }
    }

    fn ptr(&self) -> *const u8 {
        // SAFETY: reading the pointer is safe; the data behind it is
        // only mutated via `write` which callers must serialize.
        unsafe { (*self.buf.get()).as_ptr() }
    }

    fn len(&self) -> usize {
        // SAFETY: single-threaded.
        unsafe { *self.len.get() }
    }

    unsafe fn clear(&self) {
        unsafe {
            *self.len.get() = 0;
        }
    }
}

static STDOUT: OutputBuffer = OutputBuffer::new();
static STDERR: OutputBuffer = OutputBuffer::new();

/// Append bytes to the stdout buffer.
///
/// # Safety
/// Must only be called from the WASM single-threaded context.
pub unsafe fn write_stdout(bytes: &[u8]) {
    unsafe {
        STDOUT.write(bytes);
    }
}

/// Append bytes to the stderr buffer.
///
/// # Safety
/// Must only be called from the WASM single-threaded context.
pub unsafe fn write_stderr(bytes: &[u8]) {
    unsafe {
        STDERR.write(bytes);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_playground_stdout_ptr() -> *const u8 {
    STDOUT.ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_playground_stdout_len() -> usize {
    STDOUT.len()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_playground_stderr_ptr() -> *const u8 {
    STDERR.ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_playground_stderr_len() -> usize {
    STDERR.len()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_playground_clear() {
    unsafe {
        STDOUT.clear();
        STDERR.clear();
    }
}
