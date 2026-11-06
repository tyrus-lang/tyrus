use tyrus_string::TyrusString;

/// An optional payload as it crosses the ABI: a pointer to a slot holding the
/// value, or null for absence. Mirrors `os::BoxedPayload` on the native side.
pub(crate) type BoxedPayload = *mut std::ffi::c_void;

fn box_value_string(value: &str) -> BoxedPayload {
    unsafe extern "C" {
        fn malloc(size: usize) -> *mut c_void;
    }
    let storage = unsafe { malloc(std::mem::size_of::<TyrusString>()) } as *mut TyrusString;
    if storage.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { std::ptr::write(storage, crate::strings::alloc_string(value)) };
    storage.cast()
}
use std::ffi::{c_char, c_void};
use std::ptr;

#[cfg(target_arch = "wasm32")]
pub(crate) trait ByteSink {
    unsafe fn write_slice(&mut self, bytes: &[u8]);
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
pub(crate) enum OutputTarget {
    Stdout,
    Stderr,
}

#[cfg(target_arch = "wasm32")]
impl ByteSink for OutputTarget {
    #[inline]
    unsafe fn write_slice(&mut self, bytes: &[u8]) {
        unsafe {
            match self {
                OutputTarget::Stdout => crate::output::write_stdout(bytes),
                OutputTarget::Stderr => crate::output::write_stderr(bytes),
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
struct BufWriter {
    ptr: *mut u8,
    capacity: usize,
    pos: usize,
}

#[cfg(target_arch = "wasm32")]
impl ByteSink for BufWriter {
    #[inline]
    unsafe fn write_slice(&mut self, bytes: &[u8]) {
        unsafe {
            let remaining = self.capacity.saturating_sub(self.pos);
            let copy_len = bytes.len().min(remaining);
            if copy_len > 0 {
                core::ptr::copy_nonoverlapping(bytes.as_ptr(), self.ptr.add(self.pos), copy_len);
            }
            self.pos += bytes.len();
        }
    }
}

#[cfg(target_arch = "wasm32")]
unsafe fn emit_i64(sink: &mut impl ByteSink, mut val: i64) -> i32 {
    unsafe {
        let mut buf = [0u8; 24];
        let mut pos = 24;
        let negative = val < 0;
        if negative {
            val = -val;
        }
        let mut n = val as u64;
        if n == 0 {
            pos -= 1;
            buf[pos] = b'0';
        } else {
            while n > 0 {
                pos -= 1;
                buf[pos] = b'0' + (n % 10) as u8;
                n /= 10;
            }
        }
        if negative {
            pos -= 1;
            buf[pos] = b'-';
        }
        let len = 24 - pos;
        sink.write_slice(&buf[pos..24]);
        len as i32
    }
}

#[cfg(target_arch = "wasm32")]
unsafe fn emit_u64(sink: &mut impl ByteSink, mut val: u64) -> i32 {
    unsafe {
        let mut buf = [0u8; 24];
        let mut pos = 24;
        if val == 0 {
            pos -= 1;
            buf[pos] = b'0';
        } else {
            while val > 0 {
                pos -= 1;
                buf[pos] = b'0' + (val % 10) as u8;
                val /= 10;
            }
        }
        let len = 24 - pos;
        sink.write_slice(&buf[pos..24]);
        len as i32
    }
}

#[cfg(target_arch = "wasm32")]
unsafe fn format_into(
    sink: &mut impl ByteSink,
    format: *const c_char,
    varargs: *const c_void,
) -> i32 {
    unsafe {
        if format.is_null() {
            return 0;
        }
        let mut written = 0i32;
        let mut arg_ptr = varargs as *const u8;
        let mut i = 0;
        loop {
            let ch = *format.add(i) as u8;
            if ch == 0 {
                break;
            }
            if ch == b'%' {
                i += 1;
                let spec = *format.add(i) as u8;
                match spec {
                    b's' => {
                        let s_ptr = *(arg_ptr as *const *const c_char);
                        arg_ptr = arg_ptr.add(4);
                        if s_ptr.is_null() {
                            sink.write_slice(b"(null)");
                            written += 7;
                        } else {
                            let mut slen = 0;
                            while *s_ptr.add(slen) != 0 {
                                slen += 1;
                            }
                            if slen > 0 {
                                sink.write_slice(core::slice::from_raw_parts(
                                    s_ptr as *const u8,
                                    slen,
                                ));
                            }
                            written += slen as i32;
                        }
                    }
                    b'd' | b'i' => {
                        let val = *(arg_ptr as *const i32);
                        arg_ptr = arg_ptr.add(4);
                        written += emit_i64(sink, val as i64);
                    }
                    b'u' => {
                        let val = *(arg_ptr as *const u32);
                        arg_ptr = arg_ptr.add(4);
                        written += emit_u64(sink, val as u64);
                    }
                    b'l' => {
                        i += 1;
                        let spec2 = *format.add(i) as u8;
                        if spec2 == b'l' {
                            i += 1;
                            let spec3 = *format.add(i) as u8;
                            match spec3 {
                                b'd' | b'i' => {
                                    let val = (arg_ptr as *const i64).read_unaligned();
                                    arg_ptr = arg_ptr.add(8);
                                    written += emit_i64(sink, val);
                                }
                                b'u' => {
                                    let val = (arg_ptr as *const u64).read_unaligned();
                                    arg_ptr = arg_ptr.add(8);
                                    written += emit_u64(sink, val);
                                }
                                _ => {
                                    sink.write_slice(b"%ll");
                                    written += 3;
                                }
                            }
                        } else {
                            sink.write_slice(b"%");
                            written += 1;
                            sink.write_slice(core::slice::from_ref(&spec2));
                            written += 1;
                        }
                    }
                    b'%' => {
                        sink.write_slice(b"%");
                        written += 1;
                    }
                    _ => {
                        sink.write_slice(b"%");
                        written += 1;
                        sink.write_slice(core::slice::from_ref(&spec));
                        written += 1;
                    }
                }
            } else {
                let start = i;
                loop {
                    let c = *format.add(i) as u8;
                    if c == 0 || c == b'%' {
                        break;
                    }
                    i += 1;
                }
                let slice = core::slice::from_raw_parts(format.add(start) as *const u8, i - start);
                sink.write_slice(slice);
                written += (i - start) as i32;
                continue;
            }
            i += 1;
        }
        written
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printf(format: *const c_char, varargs: *const c_void) -> i32 {
    unsafe {
        let mut sink = OutputTarget::Stdout;
        format_into(&mut sink, format, varargs)
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fprintf(
    _stream: *mut c_void,
    format: *const c_char,
    varargs: *const c_void,
) -> i32 {
    unsafe {
        let mut sink = OutputTarget::Stdout;
        format_into(&mut sink, format, varargs)
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dprintf(fd: i32, format: *const c_char, varargs: *const c_void) -> i32 {
    unsafe {
        let mut sink = if fd == 2 {
            OutputTarget::Stderr
        } else {
            OutputTarget::Stdout
        };
        format_into(&mut sink, format, varargs)
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sprintf(
    buf: *mut c_char,
    format: *const c_char,
    varargs: *const c_void,
) -> i32 {
    unsafe {
        if buf.is_null() {
            return 0;
        }
        let mut sink = BufWriter {
            ptr: buf as *mut u8,
            capacity: usize::MAX,
            pos: 0,
        };
        let written = format_into(&mut sink, format, varargs);
        *buf.add(sink.pos) = 0;
        written
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn snprintf(
    buf: *mut c_char,
    size: i64,
    format: *const c_char,
    varargs: *const c_void,
) -> i32 {
    unsafe {
        let cap = size.max(0) as usize;
        if format.is_null() {
            return 0;
        }
        if buf.is_null() || cap == 0 {
            let mut sink = BufWriter {
                ptr: buf as *mut u8,
                capacity: 0,
                pos: 0,
            };
            return format_into(&mut sink, format, varargs);
        }
        let mut sink = BufWriter {
            ptr: buf as *mut u8,
            capacity: cap.saturating_sub(1),
            pos: 0,
        };
        let written = format_into(&mut sink, format, varargs);
        *buf.add(sink.pos.min(cap - 1)) = 0;
        written
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn puts(s: *const c_char) -> i32 {
    unsafe {
        if !s.is_null() {
            let mut len = 0;
            while *s.add(len) != 0 {
                len += 1;
            }
            if len > 0 {
                crate::output::write_stdout(core::slice::from_raw_parts(s as *const u8, len));
            }
            crate::output::write_stdout(b"\n");
        }
        0
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn putchar(c: i32) -> i32 {
    unsafe {
        let byte = (c & 0xFF) as u8;
        crate::output::write_stdout(core::slice::from_ref(&byte));
        c
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    unsafe {
        use std::alloc::{Layout, alloc};
        let layout = Layout::from_size_align_unchecked(size.max(1), 1);
        alloc(layout).cast()
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free(_ptr: *mut c_void) {}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    unsafe {
        use std::alloc::{Layout, alloc};
        if ptr.is_null() {
            let layout = Layout::from_size_align_unchecked(size.max(1), 1);
            alloc(layout).cast()
        } else {
            let new_layout = Layout::from_size_align_unchecked(size.max(1), 1);
            let new_ptr = alloc(new_layout);
            if !new_ptr.is_null() {
                core::ptr::copy_nonoverlapping(ptr as *const u8, new_ptr, size);
            }
            new_ptr.cast()
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn calloc(nmemb: usize, size: usize) -> *mut c_void {
    unsafe {
        use std::alloc::{Layout, alloc_zeroed};
        let total = (nmemb as u64).saturating_mul(size as u64) as usize;
        let layout = Layout::from_size_align_unchecked(total.max(1), 1);
        alloc_zeroed(layout).cast()
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strdup(s: *const c_char) -> *mut c_char {
    unsafe {
        if s.is_null() {
            return ptr::null_mut();
        }
        let mut len = 0;
        while *s.add(len) != 0 {
            len += 1;
        }
        let buf = malloc(len + 1);
        if buf.is_null() {
            return ptr::null_mut();
        }
        ptr::copy_nonoverlapping(s, buf.cast(), len + 1);
        buf.cast()
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strcmp(a: *const c_char, b: *const c_char) -> i32 {
    unsafe {
        if a.is_null() || b.is_null() {
            return 0;
        }
        let mut i = 0;
        loop {
            let ca = *a.add(i);
            let cb = *b.add(i);
            if ca != cb {
                return ca as i32 - cb as i32;
            }
            if ca == 0 {
                return 0;
            }
            i += 1;
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> i32 {
    unsafe {
        if a.is_null() || b.is_null() {
            return 0;
        }
        for i in 0..n {
            let ca = *a.add(i);
            let cb = *b.add(i);
            if ca != cb {
                return ca as i32 - cb as i32;
            }
            if ca == 0 {
                return 0;
            }
        }
        0
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strlen(s: *const c_char) -> usize {
    unsafe {
        if s.is_null() {
            return 0;
        }
        let mut i = 0;
        while *s.add(i) != 0 {
            i += 1;
        }
        i
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char {
    unsafe {
        if dst.is_null() || src.is_null() {
            return dst;
        }
        let mut i = 0;
        loop {
            let c = *src.add(i);
            *dst.add(i) = c;
            if c == 0 {
                break;
            }
            i += 1;
        }
        dst
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strcat(dst: *mut c_char, src: *const c_char) -> *mut c_char {
    unsafe {
        if dst.is_null() || src.is_null() {
            return dst;
        }
        let mut end = 0;
        while *dst.add(end) != 0 {
            end += 1;
        }
        let mut i = 0;
        loop {
            let c = *src.add(i);
            *dst.add(end + i) = c;
            if c == 0 {
                break;
            }
            i += 1;
        }
        dst
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strncat(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char {
    unsafe {
        if dst.is_null() || src.is_null() {
            return dst;
        }
        let mut end = 0;
        while *dst.add(end) != 0 {
            end += 1;
        }
        let mut i = 0;
        while i < n {
            let c = *src.add(i);
            *dst.add(end + i) = c;
            if c == 0 {
                break;
            }
            i += 1;
        }
        dst
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char {
    unsafe {
        if dst.is_null() || src.is_null() {
            return dst;
        }
        let mut i = 0;
        while i < n {
            let c = *src.add(i);
            *dst.add(i) = c;
            if c == 0 {
                break;
            }
            i += 1;
        }
        while i < n {
            *dst.add(i) = 0;
            i += 1;
        }
        dst
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    unsafe {
        core::ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, n);
        dst
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    unsafe {
        core::ptr::copy(src as *const u8, dst as *mut u8, n);
        dst
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset(dst: *mut c_void, c: i32, n: usize) -> *mut c_void {
    unsafe {
        core::ptr::write_bytes(dst as *mut u8, c as u8, n);
        dst
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> i32 {
    unsafe {
        let a = a as *const u8;
        let b = b as *const u8;
        for i in 0..n {
            let ca = *a.add(i);
            let cb = *b.add(i);
            if ca != cb {
                return ca as i32 - cb as i32;
            }
        }
        0
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn abs(x: i32) -> i32 {
    x.abs()
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn labs(x: i64) -> i64 {
    x.abs()
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fflush(_stream: *mut c_void) -> i32 {
    0
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn perror(msg: *const c_char) {
    unsafe {
        if !msg.is_null() && *msg != 0 {
            let mut sink = OutputTarget::Stderr;
            sink.write_slice(b"error: ");
            let mut len = 0;
            while *msg.add(len) != 0 {
                len += 1;
            }
            sink.write_slice(std::slice::from_raw_parts(msg as *const u8, len));
            sink.write_slice(b"\n");
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn exit(_code: i32) -> ! {
    unsafe {
        crate::panic::tyrus_panic(c"exit() called in WASM sandbox".as_ptr());
    }
}

macro_rules! fs_stub {
    ($name:ident, $ret:ty, $default:expr_2021) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(_: *const c_char, _: *const c_char) -> $ret {
            $default
        }
    };
}

fs_stub!(tyrus_fs_read_file, *mut c_char, ptr::null_mut());
fs_stub!(tyrus_fs_read_file_sync, *mut c_char, ptr::null_mut());
fs_stub!(tyrus_fs_write_file, i32, -1);
fs_stub!(tyrus_fs_write_file_sync, i32, -1);
fs_stub!(tyrus_fs_append_file, i32, -1);
fs_stub!(tyrus_fs_exists, i64, 0);
fs_stub!(tyrus_fs_stat, *mut c_char, ptr::null_mut());
fs_stub!(tyrus_fs_metadata, *mut c_char, ptr::null_mut());
fs_stub!(tyrus_fs_mkdir, i32, -1);
fs_stub!(tyrus_fs_rm, i32, -1);
fs_stub!(tyrus_fs_remove_all, i32, -1);
fs_stub!(tyrus_fs_mv, i32, -1);
fs_stub!(tyrus_fs_cp, i32, -1);
fs_stub!(tyrus_fs_readdir, *mut c_char, ptr::null_mut());
fs_stub!(tyrus_fs_glob, *mut c_char, ptr::null_mut());
fs_stub!(tyrus_fs_search, *mut c_char, ptr::null_mut());
fs_stub!(tyrus_fs_temp_dir, *mut c_char, ptr::null_mut());
fs_stub!(tyrus_fs_temp_file, *mut c_char, ptr::null_mut());
fs_stub!(tyrus_fs_resolve_path, *mut c_char, ptr::null_mut());
fs_stub!(tyrus_fs_canonicalize, *mut c_char, ptr::null_mut());
fs_stub!(tyrus_fs_chmod, i32, -1);
fs_stub!(tyrus_fs_chown, i32, -1);
fs_stub!(tyrus_fs_hard_link, i32, -1);
fs_stub!(tyrus_fs_symlink, i32, -1);
fs_stub!(tyrus_fs_readlink, *mut c_char, ptr::null_mut());
fs_stub!(tyrus_fs_diff, *mut c_char, ptr::null_mut());
fs_stub!(tyrus_fs_disk_usage, i64, -1);
fs_stub!(tyrus_fs_mime, *mut c_char, ptr::null_mut());
fs_stub!(tyrus_fs_atomic_write, i32, -1);
fs_stub!(tyrus_fs_watch, i32, -1);
fs_stub!(tyrus_fs_watcher_close, i32, -1);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_system_name() -> TyrusString {
    crate::types::alloc_value_string("WebAssembly")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_release() -> TyrusString {
    crate::types::alloc_value_string("unknown")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_hostname() -> TyrusString {
    crate::types::alloc_value_string("playground")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_user_name() -> TyrusString {
    crate::types::alloc_value_string("user")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_whoami() -> BoxedPayload {
    box_value_string("user")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_home_dir() -> TyrusString {
    crate::types::alloc_value_string("/home/user")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_temp_dir() -> TyrusString {
    crate::types::alloc_value_string("/tmp")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_language() -> TyrusString {
    crate::types::alloc_value_string("en")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_locale() -> TyrusString {
    crate::types::alloc_value_string("en-US")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_uptime() -> u64 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_total_mem() -> u64 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_free_mem() -> u64 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_battery() -> BoxedPayload {
    box_value_string("{}")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_clipboard_read() -> BoxedPayload {
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_terminal_size() -> BoxedPayload {
    box_value_string("{\"width\":80,\"height\":24}")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_clear_error() {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_process_cwd() -> TyrusString {
    crate::types::alloc_value_string("/")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_env_init(_path: TyrusString) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_env_init_sealed(_project: TyrusString, _fingerprint: TyrusString) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_env_get_str(_: TyrusString) -> TyrusString {
    TyrusString::literal("")
}

macro_rules! crypto_stub_ret_null {
    ($name:ident) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name() -> *mut c_char {
            ptr::null_mut()
        }
    };
}

crypto_stub_ret_null!(tyrus_crypto_take_error);
crypto_stub_ret_null!(tyrus_crypto_clear_error);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_rand_random_float() -> f64 {
    0.0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_rand_uuidv4() -> TyrusString {
    crate::types::alloc_value_string("00000000-0000-0000-0000-000000000000")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_rand_uuidv7() -> TyrusString {
    crate::types::alloc_value_string("00000000-0000-0000-0000-000000000000")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_rand_ulid() -> TyrusString {
    crate::types::alloc_value_string("00000000000000000000000000")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_print_str(value: TyrusString, fd: i32) {
    let bytes = value.byte_len();
    if value.is_null() || bytes == 0 {
        return;
    }
    let text = unsafe { std::slice::from_raw_parts(value.as_raw_ptr(), bytes) };
    unsafe {
        if fd == 2 {
            crate::output::write_stderr(text);
        } else {
            crate::output::write_stdout(text);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_stdout_lock_begin() {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_stdout_lock_end() {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_stderr_lock_begin() {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_stderr_lock_end() {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_promise_new() -> *mut c_void {
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_async_task_wake(_: *mut c_void) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_drop_async_box(_: *mut c_void) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_drop_async_string(_: *mut c_void) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_drop_async_copy_array(_: *mut c_void) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_drop_async_string_array(_: *mut c_void) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_drop_promise_closure_context(_: *mut c_void) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_parse_ip_address(_: TyrusString) -> *mut c_char {
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_net_parse_socket_address(_: TyrusString) -> *mut c_char {
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _setjmp(_buf: *mut c_void) -> i32 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn longjmp(_buf: *mut c_void, _val: i32) {
    #[cfg(target_arch = "wasm32")]
    core::arch::wasm32::unreachable();
    #[cfg(not(target_arch = "wasm32"))]
    std::process::abort();
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_runtime_configure_worker_threads(_threads: u64) {}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_runtime_pending_tasks() -> u64 {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_runtime_shutdown() {}
