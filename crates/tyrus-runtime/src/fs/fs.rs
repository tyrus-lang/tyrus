use super::abi::FsErrorTaken;
use super::os::BoxedPayload;
use crate::promise::TyrusPromise;
use crate::strings::alloc_string;
use std::ffi::{CStr, CString};
use std::fs;
use std::io::{Read, Write};
use std::mem::ManuallyDrop;
use std::path::Path;
use std::ptr;
use std::sync::OnceLock;
use tyrus_string::TyrusString;

#[repr(C)]
union FsSyncResultPayload {
    error: ManuallyDrop<FsErrorTaken>,
    string: ManuallyDrop<TyrusString>,
    disk_usage: ManuallyDrop<FsDiskUsage>,
    array: ManuallyDrop<crate::abi::TyrusArray>,
    stream: *const std::os::raw::c_void,
    words: [u64; 4],
}

#[repr(C)]
pub struct FsSyncResult {
    tag: u8,
    payload: FsSyncResultPayload,
}

const _: () = {
    assert!(std::mem::size_of::<FsErrorTaken>() == 32);
    assert!(std::mem::size_of::<FsDiskUsage>() == 24);
    assert!(std::mem::size_of::<FsSyncResult>() == 40);
    assert!(std::mem::offset_of!(FsSyncResult, payload) == 8);
};

const FILE_STREAM_CHUNK_BYTES: usize = 64 * 1024;
const FILE_STREAM_CAPACITY_BYTES: usize = 256 * 1024;

thread_local! {
    static LAST_FS_ERROR: std::cell::Cell<Option<(&'static CStr, &'static CStr)>> =
        const { std::cell::Cell::new(None) };
}

use notify::Watcher;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

fn set_fs_error(code: &'static CStr, message: &'static CStr) {
    LAST_FS_ERROR.with(|slot| slot.set(Some((code, message))));
}

fn replace_file_atomically(source: &Path, destination: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        };

        let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
        let destination: Vec<u16> = destination
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let succeeded = unsafe {
            MoveFileExW(
                source.as_ptr(),
                destination.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        if succeeded == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
    #[cfg(not(windows))]
    {
        fs::rename(source, destination)
    }
}

macro_rules! fs_error {
    ($code:expr_2021, $message:expr_2021, ()) => {{
        set_fs_error($code, $message);
        return;
    }};
    ($code:expr_2021, $message:expr_2021, $fallback:expr_2021) => {{
        set_fs_error($code, $message);
        return $fallback;
    }};
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_take_error(out: *mut FsErrorTaken) {
    if out.is_null() {
        return;
    }
    LAST_FS_ERROR.with(|slot| {
        if let Some((code, message)) = slot.take() {
            unsafe {
                (*out).code =
                    TyrusString::literal(code.to_str().expect("a static error literal is UTF-8"));
                (*out).message = TyrusString::literal(
                    message.to_str().expect("a static error literal is UTF-8"),
                );
            }
        }
    });
}

unsafe fn write_fs_void_result(out: *mut FsSyncResult) {
    let mut error = FsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_fs_take_error(&mut error) };
    let result = if error.code.is_null() {
        FsSyncResult {
            tag: 0,
            payload: FsSyncResultPayload { words: [0; 4] },
        }
    } else {
        FsSyncResult {
            tag: 1,
            payload: FsSyncResultPayload {
                error: ManuallyDrop::new(error),
            },
        }
    };
    if !out.is_null() {
        unsafe { ptr::write(out, result) };
    }
}

fn path_text(value: &TyrusString) -> Result<String, ()> {
    if value.is_null() {
        return Err(());
    }
    Ok(unsafe { value.as_str() }.to_owned())
}

fn resolve_path(raw: &str) -> String {
    // SAFETY: `raw` outlives the call, which is all the lending needs. It used
    // to be copied into a `CString` first, purely to gain a terminator.
    let Some(input) = (unsafe { TyrusString::lend(raw) }) else {
        return raw.to_string();
    };
    let resolved = unsafe { crate::path::tyrus_fs_resolve_path(input) };
    if resolved.is_null() {
        return raw.to_string();
    }
    let text = unsafe { resolved.as_str() }.to_owned();
    unsafe { resolved.release() };
    text
}

fn collect_dir_entries(dir: &Path, recursive: bool) -> std::io::Result<Vec<(String, i32)>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let type_ = if file_type.is_file() {
            0
        } else if file_type.is_dir() {
            1
        } else if file_type.is_symlink() {
            2
        } else {
            3
        };
        entries.push((entry.file_name().to_string_lossy().into_owned(), type_));
        if recursive && file_type.is_dir() {
            entries.extend(collect_dir_entries(&entry.path(), true)?);
        }
    }
    Ok(entries)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_read_file_sync(path: TyrusString) -> TyrusString {
    let Ok(text) = path_text(&path) else {
        set_fs_error(c"ERR_FS_INVALID_PATH", c"path must be a valid string");
        return TyrusString::null();
    };
    let resolved = resolve_path(&text);
    match fs::read_to_string(&resolved) {
        Ok(content) => alloc_string(&content),
        Err(_) => {
            fs_error!(
                c"ERR_FS_READ_FAILED",
                c"failed to read file",
                TyrusString::null()
            );
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_read_file_sync_result(path: TyrusString, out: *mut FsSyncResult) {
    LAST_FS_ERROR.with(|slot| slot.set(None));
    let value = unsafe { tyrus_fs_read_file_sync(path) };
    let mut error = FsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_fs_take_error(&mut error) };
    let result = if error.code.is_null() {
        FsSyncResult {
            tag: 0,
            payload: FsSyncResultPayload {
                string: ManuallyDrop::new(value),
            },
        }
    } else {
        FsSyncResult {
            tag: 1,
            payload: FsSyncResultPayload {
                error: ManuallyDrop::new(error),
            },
        }
    };
    if !out.is_null() {
        unsafe { ptr::write(out, result) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_write_file_sync(path: TyrusString, data: TyrusString) {
    let Ok(path_str) = path_text(&path) else {
        fs_error!(c"ERR_FS_INVALID_PATH", c"path must be a valid string", ());
    };
    let Ok(data_str) = path_text(&data) else {
        fs_error!(c"ERR_FS_INVALID_DATA", c"data must be a valid string", ());
    };
    let resolved = resolve_path(&path_str);
    if fs::write(&resolved, data_str).is_err() {
        fs_error!(c"ERR_FS_WRITE_FAILED", c"failed to write file", ());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_write_file_sync_result(
    path: TyrusString,
    data: TyrusString,
    out: *mut FsSyncResult,
) {
    LAST_FS_ERROR.with(|slot| slot.set(None));
    unsafe { tyrus_fs_write_file_sync(path, data) };
    let mut error = FsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_fs_take_error(&mut error) };
    let result = if error.code.is_null() {
        FsSyncResult {
            tag: 0,
            payload: FsSyncResultPayload { words: [0; 4] },
        }
    } else {
        FsSyncResult {
            tag: 1,
            payload: FsSyncResultPayload {
                error: ManuallyDrop::new(error),
            },
        }
    };
    if !out.is_null() {
        unsafe { ptr::write(out, result) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_exists(path: TyrusString) -> bool {
    let Ok(text) = path_text(&path) else {
        return false;
    };
    let resolved = resolve_path(&text);
    Path::new(&resolved).exists()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_read_file(path: TyrusString) -> *mut TyrusPromise {
    let Ok(text) = path_text(&path) else {
        let promise = crate::tyrus_promise_pending();
        unsafe { crate::tyrus_promise_retain(promise) };
        let addr = promise as usize;
        executor().spawn(async move {
            let p = addr as *mut TyrusPromise;
            unsafe {
                fulfill_string_error(p, "ERR_FS_INVALID_PATH");
                crate::tyrus_promise_free(p);
            }
        });
        return promise;
    };
    let resolved = resolve_path(&text);
    let promise = crate::tyrus_promise_pending();
    unsafe { crate::tyrus_promise_retain(promise) };
    let addr = promise as usize;
    executor().spawn(async move {
        let result = tokio::task::spawn_blocking(move || fs::read_to_string(&resolved)).await;
        let p = addr as *mut TyrusPromise;
        match result {
            Ok(Ok(content)) => unsafe {
                fulfill_string(p, content);
                crate::tyrus_promise_free(p);
            },
            _ => unsafe {
                fulfill_string_error(p, "ERR_FS_READ_FAILED");
                crate::tyrus_promise_free(p);
            },
        }
    });
    promise
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_write_file(
    path: TyrusString,
    data: TyrusString,
) -> *mut TyrusPromise {
    let Ok(path_str) = path_text(&path) else {
        let promise = crate::tyrus_promise_pending();
        unsafe { crate::tyrus_promise_retain(promise) };
        let addr = promise as usize;
        executor().spawn(async move {
            let p = addr as *mut TyrusPromise;
            unsafe {
                fulfill_void_error(p, "ERR_FS_INVALID_PATH");
                crate::tyrus_promise_free(p);
            }
        });
        return promise;
    };
    let Ok(data_str) = path_text(&data) else {
        let promise = crate::tyrus_promise_pending();
        unsafe { crate::tyrus_promise_retain(promise) };
        let addr = promise as usize;
        executor().spawn(async move {
            let p = addr as *mut TyrusPromise;
            unsafe {
                fulfill_void_error(p, "ERR_FS_INVALID_DATA");
                crate::tyrus_promise_free(p);
            }
        });
        return promise;
    };
    let resolved = resolve_path(&path_str);
    let content = data_str.clone();
    let promise = crate::tyrus_promise_pending();
    unsafe { crate::tyrus_promise_retain(promise) };
    let addr = promise as usize;
    executor().spawn(async move {
        let result = tokio::task::spawn_blocking(move || fs::write(&resolved, content)).await;
        let p = addr as *mut TyrusPromise;
        match result {
            Ok(Ok(())) => unsafe {
                fulfill_void(p);
                crate::tyrus_promise_free(p);
            },
            _ => unsafe {
                fulfill_void_error(p, "ERR_FS_WRITE_FAILED");
                crate::tyrus_promise_free(p);
            },
        }
    });
    promise
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_read_file_stream(
    path: TyrusString,
) -> *const crate::stream::TyrusReadableStream {
    let Ok(path_str) = path_text(&path) else {
        fs_error!(
            c"ERR_FS_INVALID_PATH",
            c"path must be a valid string",
            ptr::null()
        );
    };
    let resolved = resolve_path(&path_str);
    let Ok(file) = fs::File::open(resolved) else {
        fs_error!(
            c"ERR_FS_READ_FAILED",
            c"failed to open file for streaming",
            ptr::null()
        );
    };
    let (readable, producer) = crate::stream::byte_readable_stream(FILE_STREAM_CAPACITY_BYTES);
    executor().spawn(async move {
        let mut file = file;
        loop {
            let read = tokio::select! {
                () = producer.writable_reader_gone() => return,
                read = tokio::task::spawn_blocking(move || {
                    let mut chunk = vec![0_u8; FILE_STREAM_CHUNK_BYTES];
                    let result = file.read(&mut chunk);
                    (file, chunk, result)
                }) => read,
            };
            let Ok((returned_file, mut chunk, result)) = read else {
                producer.fail();
                return;
            };
            file = returned_file;
            match result {
                Ok(0) => {
                    producer.close();
                    return;
                }
                Ok(length) => {
                    chunk.truncate(length);
                    let write = tokio::select! {
                        () = producer.writable_reader_gone() => return,
                        result = producer.write(chunk) => result,
                    };
                    if write.is_err() {
                        return;
                    }
                }
                Err(_) => {
                    producer.fail();
                    return;
                }
            }
        }
    });
    readable
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_read_file_stream_result(
    path: TyrusString,
    out: *mut FsSyncResult,
) {
    if out.is_null() {
        return;
    }
    LAST_FS_ERROR.with(|slot| slot.set(None));
    let stream = unsafe { tyrus_fs_read_file_stream(path) };
    let mut error = FsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_fs_take_error(&mut error) };
    let result = if error.code.is_null() {
        FsSyncResult {
            tag: 0,
            payload: FsSyncResultPayload {
                stream: stream.cast::<std::os::raw::c_void>(),
            },
        }
    } else {
        FsSyncResult {
            tag: 1,
            payload: FsSyncResultPayload {
                error: ManuallyDrop::new(error),
            },
        }
    };
    if !out.is_null() {
        unsafe { ptr::write(out, result) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_write_file_stream(
    path: TyrusString,
) -> *const crate::stream::TyrusWritableStream {
    let Ok(path_str) = path_text(&path) else {
        fs_error!(
            c"ERR_FS_INVALID_PATH",
            c"path must be a valid string",
            ptr::null()
        );
    };
    let resolved = resolve_path(&path_str);
    let Ok(file) = fs::File::create(resolved) else {
        fs_error!(
            c"ERR_FS_WRITE_FAILED",
            c"failed to open file for streaming",
            ptr::null()
        );
    };
    let (writable, consumer) = crate::stream::byte_writable_stream(FILE_STREAM_CAPACITY_BYTES);
    executor().spawn(async move {
        let mut writer = std::io::BufWriter::new(file);
        loop {
            match consumer.read().await {
                Ok(Some(chunk)) => {
                    let write = tokio::task::spawn_blocking(move || {
                        let result = writer.write_all(&chunk);
                        (writer, result)
                    })
                    .await;
                    let Ok((returned_writer, result)) = write else {
                        consumer.fail();
                        return;
                    };
                    writer = returned_writer;
                    if result.is_err() {
                        consumer.fail();
                        return;
                    }
                }
                Ok(None) => {
                    let flushed = tokio::task::spawn_blocking(move || writer.flush()).await;
                    if !matches!(flushed, Ok(Ok(()))) {
                        consumer.fail();
                    }
                    consumer.close();
                    return;
                }
                Err(_) => return,
            }
        }
    });
    writable
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_write_file_stream_result(
    path: TyrusString,
    out: *mut FsSyncResult,
) {
    if out.is_null() {
        return;
    }
    LAST_FS_ERROR.with(|slot| slot.set(None));
    let stream = unsafe { tyrus_fs_write_file_stream(path) };
    let mut error = FsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_fs_take_error(&mut error) };
    let result = if error.code.is_null() {
        FsSyncResult {
            tag: 0,
            payload: FsSyncResultPayload {
                stream: stream.cast::<std::os::raw::c_void>(),
            },
        }
    } else {
        FsSyncResult {
            tag: 1,
            payload: FsSyncResultPayload {
                error: ManuallyDrop::new(error),
            },
        }
    };
    if !out.is_null() {
        unsafe { ptr::write(out, result) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_append_file(
    path: TyrusString,
    data: TyrusString,
) -> *mut TyrusPromise {
    let Ok(path_str) = path_text(&path) else {
        let promise = crate::tyrus_promise_pending();
        unsafe { crate::tyrus_promise_retain(promise) };
        let addr = promise as usize;
        executor().spawn(async move {
            let p = addr as *mut TyrusPromise;
            unsafe {
                fulfill_void_error(p, "ERR_FS_INVALID_PATH");
                crate::tyrus_promise_free(p);
            }
        });
        return promise;
    };
    let Ok(data_str) = path_text(&data) else {
        let promise = crate::tyrus_promise_pending();
        unsafe { crate::tyrus_promise_retain(promise) };
        let addr = promise as usize;
        executor().spawn(async move {
            let p = addr as *mut TyrusPromise;
            unsafe {
                fulfill_void_error(p, "ERR_FS_INVALID_DATA");
                crate::tyrus_promise_free(p);
            }
        });
        return promise;
    };
    let resolved = resolve_path(&path_str);
    let content = data_str.clone();
    let promise = crate::tyrus_promise_pending();
    unsafe { crate::tyrus_promise_retain(promise) };
    let addr = promise as usize;
    executor().spawn(async move {
        let result = tokio::task::spawn_blocking(move || {
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&resolved)?;
            file.write_all(content.as_bytes())?;
            Ok::<(), std::io::Error>(())
        })
        .await;
        let p = addr as *mut TyrusPromise;
        match result {
            Ok(Ok(())) => unsafe {
                fulfill_void(p);
                crate::tyrus_promise_free(p);
            },
            _ => unsafe {
                fulfill_void_error(p, "ERR_FS_APPEND_FAILED");
                crate::tyrus_promise_free(p);
            },
        }
    });
    promise
}

#[repr(C)]
pub struct FsDirEntry {
    pub name: TyrusString,
    pub type_: i32,
}

#[repr(C)]
pub struct ReaddirOptionsFfi {
    pub recursive: *mut bool,
}

#[repr(C)]
pub struct RmOptionsFfi {
    pub recursive: *mut bool,
    pub force: *mut bool,
}

#[repr(C)]
pub struct CpOptionsFfi {
    pub recursive: *mut bool,
    pub force: *mut bool,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_readdir(
    path: TyrusString,
    options: *const ReaddirOptionsFfi,
    out: *mut crate::abi::TyrusArray,
) {
    if out.is_null() {
        return;
    }
    let Ok(text) = path_text(&path) else {
        set_fs_error(c"ERR_FS_INVALID_PATH", c"path must be a valid string");
        unsafe {
            (*out).data = std::ptr::null_mut();
            (*out).length = 0;
            (*out).capacity = 0;
        }
        return;
    };
    let resolved = resolve_path(&text);
    let recursive =
        unsafe { !options.is_null() && !(*options).recursive.is_null() && *(*options).recursive };
    let entries = match collect_dir_entries(Path::new(&resolved), recursive) {
        Ok(entries) => entries,
        Err(_) => {
            set_fs_error(c"ERR_FS_READ_FAILED", c"failed to read directory");
            unsafe {
                (*out).data = std::ptr::null_mut();
                (*out).length = 0;
                (*out).capacity = 0;
            }
            return;
        }
    };
    let vec = entries
        .into_iter()
        .map(|(name, type_)| FsDirEntry {
            name: alloc_string(&name),
            type_,
        })
        .collect::<Vec<_>>();
    let len = vec.len();
    if len == 0 {
        unsafe {
            (*out).data = std::ptr::null_mut();
            (*out).length = 0;
            (*out).capacity = 0;
        }
        return;
    }
    let data = vec.as_ptr() as *mut u8;
    unsafe {
        (*out).data = data;
        (*out).length = len;
        (*out).capacity = len;
        std::mem::forget(vec);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_readdir_result(
    path: TyrusString,
    options: *const ReaddirOptionsFfi,
    out: *mut FsSyncResult,
) {
    if out.is_null() {
        return;
    }
    LAST_FS_ERROR.with(|slot| slot.set(None));
    let mut array = crate::abi::TyrusArray {
        data: ptr::null_mut(),
        length: 0,
        capacity: 0,
    };
    unsafe { tyrus_fs_readdir(path, options, &mut array) };
    let mut error = FsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_fs_take_error(&mut error) };
    let result = if error.code.is_null() {
        FsSyncResult {
            tag: 0,
            payload: FsSyncResultPayload {
                array: ManuallyDrop::new(array),
            },
        }
    } else {
        FsSyncResult {
            tag: 1,
            payload: FsSyncResultPayload {
                error: ManuallyDrop::new(error),
            },
        }
    };
    unsafe { ptr::write(out, result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_mkdir(path: TyrusString) {
    let Ok(text) = path_text(&path) else {
        fs_error!(c"ERR_FS_INVALID_PATH", c"path must be a valid string", ());
    };
    let resolved = resolve_path(&text);
    if fs::create_dir(&resolved).is_err() {
        fs_error!(c"ERR_FS_MKDIR_FAILED", c"failed to create directory", ());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_mkdir_result(path: TyrusString, out: *mut FsSyncResult) {
    LAST_FS_ERROR.with(|slot| slot.set(None));
    unsafe { tyrus_fs_mkdir(path) };
    let mut error = FsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_fs_take_error(&mut error) };
    let result = if error.code.is_null() {
        FsSyncResult {
            tag: 0,
            payload: FsSyncResultPayload { words: [0; 4] },
        }
    } else {
        FsSyncResult {
            tag: 1,
            payload: FsSyncResultPayload {
                error: ManuallyDrop::new(error),
            },
        }
    };
    if !out.is_null() {
        unsafe { ptr::write(out, result) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_rm(path: TyrusString, options: *const RmOptionsFfi) {
    let recursive = if options.is_null() {
        false
    } else {
        let rec_ptr = unsafe { (*options).recursive };
        if rec_ptr.is_null() {
            false
        } else {
            unsafe { *rec_ptr }
        }
    };
    let force = if options.is_null() {
        false
    } else {
        let force_ptr = unsafe { (*options).force };
        if force_ptr.is_null() {
            false
        } else {
            unsafe { *force_ptr }
        }
    };
    let Ok(text) = path_text(&path) else {
        if !force {
            fs_error!(c"ERR_FS_INVALID_PATH", c"path must be a valid string", ());
        }
        return;
    };
    let resolved = resolve_path(&text);
    let result = if recursive {
        fs::remove_dir_all(&resolved)
    } else {
        fs::remove_file(&resolved)
    };
    if result.is_err() {
        if force {
            return;
        }
        fs_error!(c"ERR_FS_RM_FAILED", c"failed to remove path", ());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_rm_result(
    path: TyrusString,
    options: *const RmOptionsFfi,
    out: *mut FsSyncResult,
) {
    LAST_FS_ERROR.with(|slot| slot.set(None));
    unsafe { tyrus_fs_rm(path, options) };
    unsafe { write_fs_void_result(out) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_cp(
    src: TyrusString,
    dest: TyrusString,
    options: *const CpOptionsFfi,
) {
    unsafe {
        let recursive = if options.is_null() {
            false
        } else {
            let rec_ptr = (*options).recursive;
            if rec_ptr.is_null() { false } else { *rec_ptr }
        };
        let force = if options.is_null() {
            false
        } else {
            let force_ptr = (*options).force;
            if force_ptr.is_null() {
                false
            } else {
                *force_ptr
            }
        };
        let Ok(src_text) = path_text(&src) else {
            fs_error!(
                c"ERR_FS_INVALID_PATH",
                c"source path must be a valid string",
                ()
            );
        };
        let Ok(dest_text) = path_text(&dest) else {
            fs_error!(
                c"ERR_FS_INVALID_PATH",
                c"dest path must be a valid string",
                ()
            );
        };
        let resolved_src = resolve_path(&src_text);
        let resolved_dest = resolve_path(&dest_text);
        let src_path = Path::new(&resolved_src);
        let dest_path = Path::new(&resolved_dest);
        if src_path.is_dir() {
            if recursive {
                if fs::create_dir_all(dest_path).is_err() {
                    fs_error!(
                        c"ERR_FS_CP_FAILED",
                        c"failed to create destination directory",
                        ()
                    );
                }
                let entries = match fs::read_dir(src_path) {
                    Ok(entries) => entries,
                    Err(_) => {
                        fs_error!(c"ERR_FS_CP_FAILED", c"failed to read source directory", ())
                    }
                };
                for entry in entries {
                    let entry = match entry {
                        Ok(entry) => entry,
                        Err(_) => fs_error!(
                            c"ERR_FS_CP_FAILED",
                            c"failed to read source directory entry",
                            ()
                        ),
                    };
                    let entry_path = entry.path();
                    let entry_name = entry.file_name();
                    let child_dest = dest_path.join(&entry_name);
                    let src_text = entry_path.to_string_lossy();
                    let dest_text = child_dest.to_string_lossy();
                    // SAFETY: both outlive the call they are lent to.
                    if let (Some(src), Some(dest)) =
                        { (TyrusString::lend(&src_text), TyrusString::lend(&dest_text)) }
                    {
                        tyrus_fs_cp(src, dest, options);
                        if LAST_FS_ERROR.with(|slot| slot.get().is_some()) {
                            return;
                        }
                    } else {
                        fs_error!(c"ERR_FS_INVALID_PATH", c"copy path is invalid", ());
                    }
                }
            } else {
                fs_error!(
                    c"ERR_FS_CP_FAILED",
                    c"cannot copy directory without recursive option",
                    ()
                );
            }
        } else {
            if force && dest_path.exists() {
                let _ = fs::remove_file(dest_path);
            }
            if fs::copy(src_path, dest_path).is_err() {
                fs_error!(c"ERR_FS_CP_FAILED", c"failed to copy file", ());
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_cp_result(
    src: TyrusString,
    dest: TyrusString,
    options: *const CpOptionsFfi,
    out: *mut FsSyncResult,
) {
    LAST_FS_ERROR.with(|slot| slot.set(None));
    unsafe { tyrus_fs_cp(src, dest, options) };
    unsafe { write_fs_void_result(out) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_mv(src: TyrusString, dest: TyrusString) {
    let Ok(src_text) = path_text(&src) else {
        fs_error!(
            c"ERR_FS_INVALID_PATH",
            c"source path must be a valid string",
            ()
        );
    };
    let Ok(dest_text) = path_text(&dest) else {
        fs_error!(
            c"ERR_FS_INVALID_PATH",
            c"dest path must be a valid string",
            ()
        );
    };
    let resolved_src = resolve_path(&src_text);
    let resolved_dest = resolve_path(&dest_text);
    if fs::rename(&resolved_src, &resolved_dest).is_err() {
        fs_error!(c"ERR_FS_MV_FAILED", c"failed to move file", ());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_mv_result(
    src: TyrusString,
    dest: TyrusString,
    out: *mut FsSyncResult,
) {
    LAST_FS_ERROR.with(|slot| slot.set(None));
    unsafe { tyrus_fs_mv(src, dest) };
    unsafe { write_fs_void_result(out) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_remove_all(path: TyrusString) {
    let Ok(text) = path_text(&path) else {
        fs_error!(c"ERR_FS_INVALID_PATH", c"path must be a valid string", ());
    };
    let resolved = resolve_path(&text);
    if fs::remove_dir_all(&resolved).is_err() {
        fs_error!(c"ERR_FS_REMOVE_FAILED", c"failed to remove path", ());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_remove_all_result(path: TyrusString, out: *mut FsSyncResult) {
    LAST_FS_ERROR.with(|slot| slot.set(None));
    unsafe { tyrus_fs_remove_all(path) };
    unsafe { write_fs_void_result(out) };
}

#[repr(C)]
pub struct FsFileStats {
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub modified: u64,
    pub created: u64,
    pub permissions: u32,
}

#[repr(C)]
union FsFileStatsResultPayload {
    error: ManuallyDrop<FsErrorTaken>,
    stats: ManuallyDrop<FsFileStats>,
    words: [u64; 5],
}

#[repr(C)]
pub struct FsFileStatsResult {
    tag: u8,
    payload: FsFileStatsResultPayload,
}

#[repr(C)]
pub struct FsMetadata {
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub modified: u64,
    pub created: u64,
    pub accessed: u64,
    pub permissions: u32,
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
}

#[repr(C)]
union FsMetadataResultPayload {
    error: ManuallyDrop<FsErrorTaken>,
    metadata: ManuallyDrop<FsMetadata>,
    words: [u64; 7],
}

#[repr(C)]
pub struct FsMetadataResult {
    tag: u8,
    payload: FsMetadataResultPayload,
}

const _: () = {
    assert!(std::mem::size_of::<FsFileStats>() == 40);
    assert!(std::mem::size_of::<FsFileStatsResult>() == 48);
    assert!(std::mem::offset_of!(FsFileStatsResult, payload) == 8);
    assert!(std::mem::size_of::<FsMetadata>() == 56);
    assert!(std::mem::size_of::<FsMetadataResult>() == 64);
    assert!(std::mem::offset_of!(FsMetadataResult, payload) == 8);
};

#[repr(C)]
pub struct FsDiskUsage {
    pub total: u64,
    pub used: u64,
    pub free: u64,
}

fn metadata_following_symlinks(path: &Path) -> std::io::Result<(fs::Metadata, bool)> {
    let entry_metadata = fs::symlink_metadata(path)?;
    let is_symlink = entry_metadata.file_type().is_symlink();
    let metadata = if is_symlink {
        fs::metadata(path)?
    } else {
        entry_metadata
    };
    Ok((metadata, is_symlink))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_stat(path: TyrusString, out: *mut FsFileStats) {
    if out.is_null() {
        return;
    }
    let Ok(text) = path_text(&path) else {
        set_fs_error(c"ERR_FS_INVALID_PATH", c"path must be a valid string");
        unsafe {
            std::ptr::write_bytes(out, 0, 1);
        }
        return;
    };
    let resolved = resolve_path(&text);
    match metadata_following_symlinks(Path::new(&resolved)) {
        Ok((meta, is_symlink)) => {
            #[cfg(unix)]
            let permissions = {
                use std::os::unix::fs::PermissionsExt;
                meta.permissions().mode()
            };
            #[cfg(not(unix))]
            let permissions: u32 = 0;
            let modified = meta.modified().map_or(0, |t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs())
            });
            let created = meta.created().map_or(0, |t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs())
            });
            unsafe {
                (*out).size = meta.len();
                (*out).is_file = meta.is_file();
                (*out).is_dir = meta.is_dir();
                (*out).is_symlink = is_symlink;
                (*out).modified = modified;
                (*out).created = created;
                (*out).permissions = permissions;
            }
        }
        Err(_) => {
            set_fs_error(c"ERR_FS_STAT_FAILED", c"failed to read file statistics");
            unsafe { std::ptr::write_bytes(out, 0, 1) };
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_stat_result(path: TyrusString, out: *mut FsFileStatsResult) {
    if out.is_null() {
        return;
    }
    LAST_FS_ERROR.with(|slot| slot.set(None));
    let mut stats = std::mem::MaybeUninit::<FsFileStats>::zeroed();
    unsafe { tyrus_fs_stat(path, stats.as_mut_ptr()) };
    let mut error = FsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_fs_take_error(&mut error) };
    let result = if error.code.is_null() {
        FsFileStatsResult {
            tag: 0,
            payload: FsFileStatsResultPayload {
                stats: ManuallyDrop::new(unsafe { stats.assume_init() }),
            },
        }
    } else {
        FsFileStatsResult {
            tag: 1,
            payload: FsFileStatsResultPayload {
                error: ManuallyDrop::new(error),
            },
        }
    };
    unsafe { ptr::write(out, result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_metadata(path: TyrusString, out: *mut FsMetadata) {
    if out.is_null() {
        return;
    }
    let Ok(text) = path_text(&path) else {
        set_fs_error(c"ERR_FS_INVALID_PATH", c"path must be a valid string");
        return;
    };
    let resolved = resolve_path(&text);
    match metadata_following_symlinks(Path::new(&resolved)) {
        Ok((meta, is_symlink)) => {
            #[cfg(unix)]
            let (permissions, uid, gid) = {
                use std::os::unix::fs::{MetadataExt, PermissionsExt};
                (meta.permissions().mode(), meta.uid(), meta.gid())
            };
            #[cfg(not(unix))]
            let (permissions, uid, gid): (u32, u32, u32) = (0, 0, 0);
            let modified = meta.modified().map_or(0, |t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs())
            });
            let created = meta.created().map_or(0, |t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs())
            });
            let accessed = meta.accessed().map_or(0, |t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs())
            });
            unsafe {
                (*out).size = meta.len();
                (*out).is_file = meta.is_file();
                (*out).is_dir = meta.is_dir();
                (*out).is_symlink = is_symlink;
                (*out).modified = modified;
                (*out).created = created;
                (*out).accessed = accessed;
                (*out).permissions = permissions;
                (*out).uid = uid;
                (*out).gid = gid;
                (*out).mode = permissions;
            }
        }
        Err(_) => {
            set_fs_error(c"ERR_FS_METADATA_FAILED", c"failed to read metadata");
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_metadata_result(path: TyrusString, out: *mut FsMetadataResult) {
    if out.is_null() {
        return;
    }
    LAST_FS_ERROR.with(|slot| slot.set(None));
    let mut metadata = std::mem::MaybeUninit::<FsMetadata>::zeroed();
    unsafe { tyrus_fs_metadata(path, metadata.as_mut_ptr()) };
    let mut error = FsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_fs_take_error(&mut error) };
    let result = if error.code.is_null() {
        FsMetadataResult {
            tag: 0,
            payload: FsMetadataResultPayload {
                metadata: ManuallyDrop::new(unsafe { metadata.assume_init() }),
            },
        }
    } else {
        FsMetadataResult {
            tag: 1,
            payload: FsMetadataResultPayload {
                error: ManuallyDrop::new(error),
            },
        }
    };
    unsafe { ptr::write(out, result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_disk_usage(path: TyrusString, out: *mut FsDiskUsage) {
    unsafe {
        if out.is_null() {
            return;
        }
        let Ok(text) = path_text(&path) else {
            set_fs_error(c"ERR_FS_INVALID_PATH", c"path must be a valid string");
            return;
        };
        let resolved = resolve_path(&text);
        let target = Path::new(&resolved);
        let canonical = if target.is_dir() {
            target
        } else {
            target.parent().unwrap_or(target)
        };

        #[cfg(unix)]
        {
            let Ok(c_path) = CString::new(canonical.to_string_lossy().as_ref()) else {
                set_fs_error(c"ERR_FS_INVALID_PATH", c"path must not contain NUL bytes");
                return;
            };
            let mut stat_buf: libc::statvfs = std::mem::zeroed();
            if libc::statvfs(c_path.as_ptr(), &mut stat_buf) == 0 {
                // `free` is `f_bavail`, what this process can still write; the
                // blocks only root may use are neither free nor used, so
                // `used + free` can be less than `total`, as in `df`.
                #[allow(clippy::useless_conversion)]
                let (fragment, blocks, unused, available) = (
                    u64::from(stat_buf.f_frsize),
                    u64::from(stat_buf.f_blocks),
                    u64::from(stat_buf.f_bfree),
                    u64::from(stat_buf.f_bavail),
                );
                (*out).total = blocks.saturating_mul(fragment);
                (*out).free = available.saturating_mul(fragment);
                (*out).used = blocks.saturating_sub(unused).saturating_mul(fragment);
            } else {
                set_fs_error(c"ERR_FS_DISK_USAGE_FAILED", c"failed to read disk usage");
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;

            let path: Vec<u16> = canonical
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let mut available_to_caller = 0_u64;
            let mut total = 0_u64;
            let mut total_free = 0_u64;
            if windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
                path.as_ptr(),
                &mut available_to_caller,
                &mut total,
                &mut total_free,
            ) != 0
            {
                (*out).total = total;
                (*out).used = total.saturating_sub(total_free);
                (*out).free = available_to_caller;
            } else {
                set_fs_error(c"ERR_FS_DISK_USAGE_FAILED", c"failed to read disk usage");
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            set_fs_error(
                c"ERR_FS_DISK_USAGE_FAILED",
                c"disk usage not supported on this platform",
            );
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_disk_usage_result(path: TyrusString, out: *mut FsSyncResult) {
    if out.is_null() {
        return;
    }
    LAST_FS_ERROR.with(|slot| slot.set(None));
    let mut usage = FsDiskUsage {
        total: 0,
        used: 0,
        free: 0,
    };
    unsafe { tyrus_fs_disk_usage(path, &mut usage) };
    let mut error = FsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_fs_take_error(&mut error) };
    let result = if error.code.is_null() {
        FsSyncResult {
            tag: 0,
            payload: FsSyncResultPayload {
                disk_usage: ManuallyDrop::new(usage),
            },
        }
    } else {
        FsSyncResult {
            tag: 1,
            payload: FsSyncResultPayload {
                error: ManuallyDrop::new(error),
            },
        }
    };
    unsafe { ptr::write(out, result) };
}

fn box_optional_string(value: TyrusString) -> BoxedPayload {
    let storage = unsafe { libc::malloc(std::mem::size_of::<TyrusString>()) } as *mut TyrusString;
    if storage.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { std::ptr::write(storage, value) };
    storage.cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_mime(path: TyrusString) -> BoxedPayload {
    let Ok(text) = path_text(&path) else {
        return ptr::null_mut();
    };
    let resolved = resolve_path(&text);
    let Some(ext) = Path::new(&resolved).extension() else {
        return ptr::null_mut();
    };
    let Some(ext_str) = ext.to_str() else {
        return ptr::null_mut();
    };
    let mime = match ext_str.to_lowercase().as_str() {
        "txt" | "md" | "rs" | "go" | "c" | "cpp" | "cc" | "py" => "text/plain",
        "json" => "application/json",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" | "ts" => "application/javascript",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",
        "bin" | "exe" => "application/octet-stream",
        _ => return std::ptr::null_mut(),
    };
    box_optional_string(alloc_string(mime))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_chmod(path: TyrusString, mode: i32) {
    let Ok(text) = path_text(&path) else {
        fs_error!(c"ERR_FS_INVALID_PATH", c"path must be a valid string", ());
    };
    let resolved = resolve_path(&text);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(mode as u32);
        if fs::set_permissions(&resolved, perms).is_err() {
            fs_error!(c"ERR_FS_CHMOD_FAILED", c"failed to change permissions", ());
        }
    }
    #[cfg(not(unix))]
    {
        fs_error!(
            c"ERR_FS_CHMOD_FAILED",
            c"chmod not supported on this platform",
            ()
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_chmod_result(
    path: TyrusString,
    mode: i32,
    out: *mut FsSyncResult,
) {
    LAST_FS_ERROR.with(|slot| slot.set(None));
    unsafe { tyrus_fs_chmod(path, mode) };
    unsafe { write_fs_void_result(out) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_chown(path: TyrusString, uid: u32, gid: u32) {
    unsafe {
        let Ok(text) = path_text(&path) else {
            fs_error!(c"ERR_FS_INVALID_PATH", c"path must be a valid string", ());
        };
        let resolved = resolve_path(&text);
        #[cfg(unix)]
        {
            let Ok(c_path) = CString::new(resolved.as_str()) else {
                fs_error!(c"ERR_FS_INVALID_PATH", c"path must not hold a NUL", ());
            };
            if libc::chown(c_path.as_ptr(), uid, gid) != 0 {
                fs_error!(c"ERR_FS_CHOWN_FAILED", c"failed to change ownership", ());
            }
        }
        #[cfg(not(unix))]
        {
            fs_error!(
                c"ERR_FS_CHOWN_FAILED",
                c"chown not supported on this platform",
                ()
            );
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_chown_result(
    path: TyrusString,
    uid: u32,
    gid: u32,
    out: *mut FsSyncResult,
) {
    LAST_FS_ERROR.with(|slot| slot.set(None));
    unsafe { tyrus_fs_chown(path, uid, gid) };
    unsafe { write_fs_void_result(out) };
}

#[repr(C)]
pub struct FsTempDir {
    pub path: TyrusString,
}

pub struct FsFSWatcher {
    #[allow(dead_code)]
    watcher: notify::RecommendedWatcher,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_watch(
    path: TyrusString,
    out: *mut *mut std::os::raw::c_void,
) -> *mut std::os::raw::c_void {
    if out.is_null() {
        return std::ptr::null_mut();
    }
    let Ok(text) = path_text(&path) else {
        set_fs_error(c"ERR_FS_INVALID_PATH", c"path must be a valid string");
        unsafe {
            *out = std::ptr::null_mut();
        }
        return std::ptr::null_mut();
    };
    let resolved = resolve_path(&text);
    let resolved_path = Path::new(&resolved);
    if !resolved_path.exists() {
        set_fs_error(c"ERR_FS_NOT_FOUND", c"watched path does not exist");
        unsafe {
            *out = std::ptr::null_mut();
        }
        return std::ptr::null_mut();
    }

    let mut watcher = match notify::recommended_watcher(|_event: notify::Result<notify::Event>| {})
    {
        Ok(watcher) => watcher,
        Err(_) => {
            set_fs_error(c"ERR_FS_WATCH_FAILED", c"failed to start watching path");
            unsafe {
                *out = std::ptr::null_mut();
            }
            return std::ptr::null_mut();
        }
    };
    if watcher
        .watch(resolved_path, notify::RecursiveMode::Recursive)
        .is_err()
    {
        set_fs_error(c"ERR_FS_WATCH_FAILED", c"failed to start watching path");
        unsafe {
            *out = std::ptr::null_mut();
        }
        return std::ptr::null_mut();
    }

    let handle = Box::into_raw(Box::new(FsFSWatcher { watcher }));

    unsafe {
        *out = handle as *mut _;
    }
    handle as *mut std::os::raw::c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_watch_result(path: TyrusString, out: *mut FsSyncResult) {
    if out.is_null() {
        return;
    }
    LAST_FS_ERROR.with(|slot| slot.set(None));
    let mut watcher = ptr::null_mut();
    unsafe { tyrus_fs_watch(path, &mut watcher) };
    let mut error = FsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_fs_take_error(&mut error) };
    let result = if error.code.is_null() {
        FsSyncResult {
            tag: 0,
            payload: FsSyncResultPayload { stream: watcher },
        }
    } else {
        FsSyncResult {
            tag: 1,
            payload: FsSyncResultPayload {
                error: ManuallyDrop::new(error),
            },
        }
    };
    unsafe { ptr::write(out, result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_watcher_close(watcher: *mut std::os::raw::c_void) {
    if watcher.is_null() {
        return;
    }
    let handle = watcher as *mut FsFSWatcher;
    drop(unsafe { Box::from_raw(handle) });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_readlink(path: TyrusString) -> TyrusString {
    let Ok(text) = path_text(&path) else {
        set_fs_error(c"ERR_FS_INVALID_PATH", c"path must be a valid string");
        return TyrusString::null();
    };
    let resolved = resolve_path(&text);
    match fs::read_link(&resolved) {
        Ok(target) => alloc_string(target.to_string_lossy().as_ref()),
        Err(_) => {
            set_fs_error(
                c"ERR_FS_READLINK_FAILED",
                c"failed to read symbolic link target",
            );
            TyrusString::null()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_readlink_result(path: TyrusString, out: *mut FsSyncResult) {
    if out.is_null() {
        return;
    }
    LAST_FS_ERROR.with(|slot| slot.set(None));
    let value = unsafe { tyrus_fs_readlink(path) };
    let mut error = FsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_fs_take_error(&mut error) };
    let result = if error.code.is_null() {
        FsSyncResult {
            tag: 0,
            payload: FsSyncResultPayload {
                string: ManuallyDrop::new(value),
            },
        }
    } else {
        FsSyncResult {
            tag: 1,
            payload: FsSyncResultPayload {
                error: ManuallyDrop::new(error),
            },
        }
    };
    unsafe { ptr::write(out, result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_symlink(original: TyrusString, link: TyrusString) {
    let Ok(orig_text) = path_text(&original) else {
        fs_error!(
            c"ERR_FS_INVALID_PATH",
            c"original path must be a valid string",
            ()
        );
    };
    let Ok(link_text) = path_text(&link) else {
        fs_error!(
            c"ERR_FS_INVALID_PATH",
            c"link path must be a valid string",
            ()
        );
    };
    let resolved_orig = resolve_path(&orig_text);
    let resolved_link = resolve_path(&link_text);
    #[cfg(unix)]
    {
        if std::os::unix::fs::symlink(&resolved_orig, &resolved_link).is_err() {
            fs_error!(c"ERR_FS_SYMLINK_FAILED", c"failed to create symlink", ());
        }
    }
    #[cfg(windows)]
    {
        if std::os::windows::fs::symlink_file(&resolved_orig, &resolved_link).is_err() {
            fs_error!(
                c"ERR_FS_SYMLINK_FAILED",
                c"failed to create file symlink",
                ()
            );
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        set_fs_error(
            c"ERR_FS_SYMLINK_FAILED",
            c"symlink not supported on this platform",
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_symlink_result(
    original: TyrusString,
    link: TyrusString,
    out: *mut FsSyncResult,
) {
    LAST_FS_ERROR.with(|slot| slot.set(None));
    unsafe { tyrus_fs_symlink(original, link) };
    unsafe { write_fs_void_result(out) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_symlink_dir(original: TyrusString, link: TyrusString) {
    let Ok(orig_text) = path_text(&original) else {
        fs_error!(
            c"ERR_FS_INVALID_PATH",
            c"original path must be a valid string",
            ()
        );
    };
    let Ok(link_text) = path_text(&link) else {
        fs_error!(
            c"ERR_FS_INVALID_PATH",
            c"link path must be a valid string",
            ()
        );
    };
    let resolved_orig = resolve_path(&orig_text);
    let resolved_link = resolve_path(&link_text);
    #[cfg(unix)]
    {
        if std::os::unix::fs::symlink(&resolved_orig, &resolved_link).is_err() {
            fs_error!(
                c"ERR_FS_SYMLINK_FAILED",
                c"failed to create directory symlink",
                ()
            );
        }
    }
    #[cfg(windows)]
    {
        if std::os::windows::fs::symlink_dir(&resolved_orig, &resolved_link).is_err() {
            fs_error!(
                c"ERR_FS_SYMLINK_FAILED",
                c"failed to create directory symlink",
                ()
            );
        }
    }
    #[cfg(not(any(unix, windows)))]
    set_fs_error(
        c"ERR_FS_SYMLINK_FAILED",
        c"directory symlink not supported on this platform",
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_symlink_dir_result(
    original: TyrusString,
    link: TyrusString,
    out: *mut FsSyncResult,
) {
    LAST_FS_ERROR.with(|slot| slot.set(None));
    unsafe { tyrus_fs_symlink_dir(original, link) };
    unsafe { write_fs_void_result(out) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_hard_link(original: TyrusString, link: TyrusString) {
    let Ok(orig_text) = path_text(&original) else {
        fs_error!(
            c"ERR_FS_INVALID_PATH",
            c"original path must be a valid string",
            ()
        );
    };
    let Ok(link_text) = path_text(&link) else {
        fs_error!(
            c"ERR_FS_INVALID_PATH",
            c"link path must be a valid string",
            ()
        );
    };
    let resolved_orig = resolve_path(&orig_text);
    let resolved_link = resolve_path(&link_text);
    if fs::hard_link(&resolved_orig, &resolved_link).is_err() {
        fs_error!(c"ERR_FS_HARDLINK_FAILED", c"failed to create hard link", ());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_hard_link_result(
    original: TyrusString,
    link: TyrusString,
    out: *mut FsSyncResult,
) {
    LAST_FS_ERROR.with(|slot| slot.set(None));
    unsafe { tyrus_fs_hard_link(original, link) };
    unsafe { write_fs_void_result(out) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_canonicalize(path: TyrusString) -> TyrusString {
    let Ok(text) = path_text(&path) else {
        set_fs_error(c"ERR_FS_INVALID_PATH", c"path must be a valid string");
        return TyrusString::null();
    };
    let resolved = resolve_path(&text);
    match fs::canonicalize(&resolved) {
        Ok(canonical) => alloc_string(canonical.to_string_lossy().as_ref()),
        Err(_) => {
            set_fs_error(
                c"ERR_FS_CANONICALIZE_FAILED",
                c"failed to canonicalize path",
            );
            TyrusString::null()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_canonicalize_result(path: TyrusString, out: *mut FsSyncResult) {
    if out.is_null() {
        return;
    }
    LAST_FS_ERROR.with(|slot| slot.set(None));
    let value = unsafe { tyrus_fs_canonicalize(path) };
    let mut error = FsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_fs_take_error(&mut error) };
    let result = if error.code.is_null() {
        FsSyncResult {
            tag: 0,
            payload: FsSyncResultPayload {
                string: ManuallyDrop::new(value),
            },
        }
    } else {
        FsSyncResult {
            tag: 1,
            payload: FsSyncResultPayload {
                error: ManuallyDrop::new(error),
            },
        }
    };
    unsafe { ptr::write(out, result) };
}

fn glob_pattern_match(pattern: &str, name: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    let re = glob_to_regex(pattern);
    if let Ok(regex) = re
        && regex.is_match(name)
    {
        return true;
    }
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            let prefix = parts[0];
            let suffix = parts[1];
            return name.starts_with(prefix)
                && name.ends_with(suffix)
                && name.len() >= prefix.len() + suffix.len();
        }
    }
    pattern == name
}

/// Matches a `/`-separated relative path against a glob with the same segment
/// rules `glob` walks the file system with: `**` spans any number of
/// directories and every other segment is matched by `glob_pattern_match`.
pub(crate) fn glob_path_match(pattern: &str, path: &str) -> bool {
    let (start, pattern_segments) = glob_pattern_segments(pattern);
    let prefix = start.to_string_lossy().replace('\\', "/");
    let path = if prefix == "." || prefix.is_empty() {
        path
    } else if let Some(path) = path.strip_prefix(&prefix) {
        path.trim_start_matches('/')
    } else {
        return false;
    };
    let path_segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .collect();
    glob_segments_match(&pattern_segments, &path_segments)
}

fn glob_segments_match(pattern: &[&str], path: &[&str]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        Some((&"**", rest)) => {
            (0..=path.len()).any(|skipped| glob_segments_match(rest, &path[skipped..]))
        }
        Some((segment, rest)) => path.split_first().is_some_and(|(name, remaining)| {
            glob_pattern_match(segment, name) && glob_segments_match(rest, remaining)
        }),
    }
}

fn glob_to_regex(pattern: &str) -> Result<regex::Regex, regex::Error> {
    let mut regex_str = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '*' => regex_str.push_str(".*"),
            '?' => regex_str.push('.'),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                regex_str.push('\\');
                regex_str.push(ch);
            }
            _ => regex_str.push(ch),
        }
    }
    regex_str.push('$');
    regex::Regex::new(&regex_str)
}

fn split_glob_pattern(pattern: &str) -> (String, String) {
    let normalized = pattern.replace('\\', "/");
    let Some(slash) = normalized.rfind('/') else {
        return (".".to_string(), pattern.to_string());
    };
    let base_dir = if slash == 0 {
        "/".to_string()
    } else {
        normalized[..slash].to_string()
    };
    (base_dir, normalized[slash + 1..].to_string())
}

fn glob_pattern_segments(pattern: &str) -> (std::path::PathBuf, Vec<&str>) {
    let components: Vec<_> = Path::new(pattern).components().collect();
    let segment_start = components
        .iter()
        .position(|component| {
            component.as_os_str().to_str().is_some_and(|value| {
                matches!(component, std::path::Component::Normal(_))
                    && (value.contains('*') || value.contains('?'))
            })
        })
        .unwrap_or_else(|| components.len().saturating_sub(1));
    let mut start = std::path::PathBuf::new();
    if segment_start == 0 && !Path::new(pattern).is_absolute() {
        start.push(".");
    }
    let mut segments = Vec::new();
    for (index, component) in components.iter().enumerate() {
        if index < segment_start {
            start.push(component.as_os_str());
        } else if let std::path::Component::Normal(value) = component {
            if let Some(value) = value.to_str() {
                segments.push(value);
            }
        }
    }
    (start, segments)
}

fn walk_glob_segments(
    dir: &Path,
    prefix: &str,
    segments: &[&str],
    results: &mut Vec<String>,
) -> Result<(), ()> {
    let Some((segment, rest)) = segments.split_first() else {
        return Ok(());
    };
    let entries = fs::read_dir(dir).map_err(|_| ())?;
    let join = |prefix: &str, name: &str| {
        if prefix.is_empty() {
            name.to_string()
        } else if prefix.ends_with('/') {
            format!("{prefix}{name}")
        } else {
            format!("{prefix}/{name}")
        }
    };
    if *segment == "**" {
        walk_glob_segments(dir, prefix, rest, results)?;
        for entry in entries {
            let entry = entry.map_err(|_| ())?;
            if entry.file_type().map_err(|_| ())?.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                let next_prefix = join(prefix, &name);
                walk_glob_segments(&entry.path(), &next_prefix, segments, results)?;
            }
        }
        return Ok(());
    }
    for entry in entries {
        let entry = entry.map_err(|_| ())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !glob_pattern_match(segment, &name) {
            continue;
        }
        let next_prefix = join(prefix, &name);
        if rest.is_empty() {
            results.push(next_prefix);
        } else if entry.file_type().map_err(|_| ())?.is_dir() {
            walk_glob_segments(&entry.path(), &next_prefix, rest, results)?;
        }
    }
    Ok(())
}

fn collect_glob(pattern: &str) -> Result<Vec<String>, ()> {
    let absolute_pattern = Path::new(pattern).is_absolute();
    let resolved = resolve_path(pattern);
    let (start, segments) = glob_pattern_segments(&resolved);
    let output_prefix = if absolute_pattern {
        start.to_string_lossy().replace('\\', "/")
    } else {
        let current_dir = std::env::current_dir().unwrap_or_default();
        start
            .strip_prefix(current_dir)
            .map(Path::to_path_buf)
            .unwrap_or_default()
            .to_string_lossy()
            .replace('\\', "/")
    };
    let output_prefix = if output_prefix == "."
        || output_prefix.is_empty()
        || output_prefix.ends_with('/')
            && output_prefix != "/"
            && !is_windows_drive_root(&output_prefix)
    {
        output_prefix.trim_end_matches('/').to_string()
    } else {
        output_prefix
    };
    let mut matches: Vec<String> = Vec::new();
    walk_glob_segments(&start, &output_prefix, &segments, &mut matches)?;
    matches.sort_unstable();
    Ok(matches)
}

fn is_windows_drive_root(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() == 3 && bytes[1] == b':' && bytes[2] == b'/'
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_glob(
    pattern: TyrusString,
    out: *mut crate::abi::TyrusStringArray,
) {
    if out.is_null() {
        return;
    }
    let Ok(pattern_text) = path_text(&pattern) else {
        set_fs_error(c"ERR_FS_INVALID_PATH", c"pattern must be a valid string");
        unsafe {
            (*out).data = std::ptr::null();
            (*out).length = 0;
            (*out).capacity = 0;
        }
        return;
    };
    let array = match collect_glob(&pattern_text) {
        Ok(results) => collect_string_array(results),
        Err(()) => {
            set_fs_error(c"ERR_FS_GLOB_FAILED", c"failed to search glob paths");
            crate::abi::TyrusStringArray {
                data: ptr::null(),
                length: 0,
                capacity: 0,
            }
        }
    };
    unsafe {
        (*out).data = array.data;
        (*out).length = array.length;
        (*out).capacity = array.capacity;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_glob_result(pattern: TyrusString, out: *mut FsSyncResult) {
    if out.is_null() {
        return;
    }
    LAST_FS_ERROR.with(|slot| slot.set(None));
    let mut array = crate::abi::TyrusStringArray {
        data: ptr::null(),
        length: 0,
        capacity: 0,
    };
    if pattern.is_null() {
        set_fs_error(c"ERR_FS_INVALID_PATH", c"pattern must be a valid string");
    } else {
        unsafe { tyrus_fs_glob(pattern, &mut array) };
    }
    let mut error = FsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_fs_take_error(&mut error) };
    let result = if error.code.is_null() {
        FsSyncResult {
            tag: 0,
            payload: FsSyncResultPayload {
                array: ManuallyDrop::new(crate::abi::TyrusArray {
                    data: array.data.cast_mut().cast::<u8>(),
                    length: array.length,
                    capacity: array.capacity,
                }),
            },
        }
    } else {
        FsSyncResult {
            tag: 1,
            payload: FsSyncResultPayload {
                error: ManuallyDrop::new(error),
            },
        }
    };
    unsafe { ptr::write(out, result) };
}

#[repr(C)]
pub struct SearchOptionsFfi {
    pub recursive: *mut bool,
    pub max_results: *mut u32,
}

fn collect_search(
    pattern: &str,
    recursive: bool,
    max_results: Option<usize>,
) -> Result<Vec<String>, ()> {
    let mut results: Vec<String> = Vec::new();
    let (base_dir, glob_pattern) = split_glob_pattern(pattern);
    let base_path = Path::new(&base_dir);

    fn walk_dir(
        dir: &Path,
        recursive: bool,
        glob_pattern: &str,
        max_results: Option<usize>,
        results: &mut Vec<String>,
    ) -> Result<(), ()> {
        if max_results.is_some_and(|max| results.len() >= max) {
            return Ok(());
        }
        let entries = fs::read_dir(dir).map_err(|_| ())?;
        for entry in entries {
            if max_results.is_some_and(|max| results.len() >= max) {
                break;
            }
            let entry = entry.map_err(|_| ())?;
            let file_type = entry.file_type().map_err(|_| ())?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            if glob_pattern_match(glob_pattern, &file_name) {
                results.push(file_name);
                if max_results.is_some_and(|max| results.len() >= max) {
                    break;
                }
            }
            if recursive && file_type.is_dir() {
                walk_dir(&entry.path(), recursive, glob_pattern, max_results, results)?;
            }
        }
        Ok(())
    }

    walk_dir(
        base_path,
        recursive,
        &glob_pattern,
        max_results,
        &mut results,
    )?;
    Ok(results)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_search(
    pattern: TyrusString,
    options: *const SearchOptionsFfi,
    out: *mut crate::abi::TyrusStringArray,
) {
    if out.is_null() {
        return;
    }
    let Ok(pattern_text) = path_text(&pattern) else {
        set_fs_error(c"ERR_FS_INVALID_PATH", c"pattern must be a valid string");
        unsafe {
            (*out).data = std::ptr::null();
            (*out).length = 0;
            (*out).capacity = 0;
        }
        return;
    };
    let resolved = resolve_path(&pattern_text);
    let recursive = if options.is_null() {
        false
    } else {
        let rec_ptr = unsafe { (*options).recursive };
        !rec_ptr.is_null() && unsafe { *rec_ptr }
    };
    let max_results = if options.is_null() {
        None
    } else {
        let max_ptr = unsafe { (*options).max_results };
        (!max_ptr.is_null()).then(|| unsafe { *max_ptr } as usize)
    };
    let array = match collect_search(&resolved, recursive, max_results) {
        Ok(results) => collect_string_array(results),
        Err(()) => {
            set_fs_error(c"ERR_FS_SEARCH_FAILED", c"failed to search directory");
            crate::abi::TyrusStringArray {
                data: ptr::null(),
                length: 0,
                capacity: 0,
            }
        }
    };
    unsafe {
        (*out).data = array.data;
        (*out).length = array.length;
        (*out).capacity = array.capacity;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_search_result(
    pattern: TyrusString,
    options: *const SearchOptionsFfi,
    out: *mut FsSyncResult,
) {
    if out.is_null() {
        return;
    }
    LAST_FS_ERROR.with(|slot| slot.set(None));
    let mut array = crate::abi::TyrusStringArray {
        data: ptr::null(),
        length: 0,
        capacity: 0,
    };
    if pattern.is_null() {
        set_fs_error(c"ERR_FS_INVALID_PATH", c"pattern must be a valid string");
    } else {
        unsafe { tyrus_fs_search(pattern, options, &mut array) };
    }
    let payload_array = crate::abi::TyrusArray {
        data: array.data.cast_mut().cast::<u8>(),
        length: array.length,
        capacity: array.capacity,
    };
    let mut error = FsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_fs_take_error(&mut error) };
    let result = if error.code.is_null() {
        FsSyncResult {
            tag: 0,
            payload: FsSyncResultPayload {
                array: ManuallyDrop::new(payload_array),
            },
        }
    } else {
        FsSyncResult {
            tag: 1,
            payload: FsSyncResultPayload {
                error: ManuallyDrop::new(error),
            },
        }
    };
    unsafe { ptr::write(out, result) };
}

#[repr(C)]
pub struct FsDiffResult {
    pub added: crate::abi::TyrusStringArray,
    pub removed: crate::abi::TyrusStringArray,
    pub modified: crate::abi::TyrusStringArray,
    pub unchanged: crate::abi::TyrusStringArray,
}

#[repr(C)]
union FsDiffResultPayload {
    error: ManuallyDrop<FsErrorTaken>,
    diff: ManuallyDrop<FsDiffResult>,
    words: [u64; 12],
}

#[repr(C)]
pub struct FsDiffResultUnion {
    tag: u8,
    payload: FsDiffResultPayload,
}

const _: () = {
    assert!(std::mem::size_of::<FsDiffResult>() == 96);
    assert!(std::mem::size_of::<FsDiffResultUnion>() == 104);
    assert!(std::mem::offset_of!(FsDiffResultUnion, payload) == 8);
};

fn collect_string_array(vec: Vec<String>) -> crate::abi::TyrusStringArray {
    let len = vec.len();
    if len == 0 {
        return crate::abi::TyrusStringArray {
            data: std::ptr::null(),
            length: 0,
            capacity: 0,
        };
    }
    let ptrs: Vec<TyrusString> = vec.iter().map(|s| alloc_string(s)).collect();
    let data = ptrs.as_ptr();
    let arr = crate::abi::TyrusStringArray {
        data,
        length: len,
        capacity: len,
    };
    std::mem::forget(ptrs);
    arr
}

fn read_diff_entries(path: &Path, metadata: &fs::Metadata) -> std::io::Result<Vec<String>> {
    if !metadata.is_dir() {
        return path
            .file_name()
            .map(|name| vec![name.to_string_lossy().into_owned()])
            .ok_or_else(|| std::io::Error::other("path has no file name"));
    }
    let mut entries = Vec::new();
    for entry in fs::read_dir(path)? {
        entries.push(entry?.file_name().to_string_lossy().into_owned());
    }
    entries.sort_unstable();
    Ok(entries)
}

fn diff_entry_matches(path1: &Path, path2: &Path) -> std::io::Result<bool> {
    let metadata1 = fs::metadata(path1)?;
    let metadata2 = fs::metadata(path2)?;
    match (metadata1.is_dir(), metadata2.is_dir()) {
        (true, true) => Ok(true),
        (false, false) => Ok(fs::read(path1)? == fs::read(path2)?),
        _ => Ok(false),
    }
}

fn collect_fs_diff(
    path1: &Path,
    path2: &Path,
) -> std::io::Result<(Vec<String>, Vec<String>, Vec<String>, Vec<String>)> {
    let metadata1 = fs::metadata(path1)?;
    let metadata2 = fs::metadata(path2)?;
    let entries1: std::collections::BTreeSet<String> =
        read_diff_entries(path1, &metadata1)?.into_iter().collect();
    let entries2: std::collections::BTreeSet<String> =
        read_diff_entries(path2, &metadata2)?.into_iter().collect();
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut modified = Vec::new();
    let mut unchanged = Vec::new();

    for name in entries1.difference(&entries2) {
        added.push(name.clone());
    }
    for name in entries2.difference(&entries1) {
        removed.push(name.clone());
    }
    for name in entries1.intersection(&entries2) {
        let entry1 = if metadata1.is_dir() {
            path1.join(name)
        } else {
            path1.to_path_buf()
        };
        let entry2 = if metadata2.is_dir() {
            path2.join(name)
        } else {
            path2.to_path_buf()
        };
        if diff_entry_matches(&entry1, &entry2)? {
            unchanged.push(name.clone());
        } else {
            modified.push(name.clone());
        }
    }
    Ok((added, removed, modified, unchanged))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_diff(
    path1: TyrusString,
    path2: TyrusString,
    out: *mut FsDiffResult,
) {
    if out.is_null() {
        return;
    }
    let Ok(text1) = path_text(&path1) else {
        set_fs_error(c"ERR_FS_INVALID_PATH", c"path1 must be a valid string");
        return;
    };
    let Ok(text2) = path_text(&path2) else {
        set_fs_error(c"ERR_FS_INVALID_PATH", c"path2 must be a valid string");
        return;
    };
    let resolved1 = resolve_path(&text1);
    let resolved2 = resolve_path(&text2);
    let (added, removed, modified, unchanged) =
        match collect_fs_diff(Path::new(&resolved1), Path::new(&resolved2)) {
            Ok(diff) => diff,
            Err(_) => {
                set_fs_error(c"ERR_FS_DIFF_FAILED", c"failed to compare paths");
                return;
            }
        };

    unsafe {
        (*out).added = collect_string_array(added);
        (*out).removed = collect_string_array(removed);
        (*out).modified = collect_string_array(modified);
        (*out).unchanged = collect_string_array(unchanged);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_diff_result(
    path1: TyrusString,
    path2: TyrusString,
    out: *mut FsDiffResultUnion,
) {
    if out.is_null() {
        return;
    }
    LAST_FS_ERROR.with(|slot| slot.set(None));
    let mut diff = std::mem::MaybeUninit::<FsDiffResult>::zeroed();
    unsafe { tyrus_fs_diff(path1, path2, diff.as_mut_ptr()) };
    let mut error = FsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_fs_take_error(&mut error) };
    let result = if error.code.is_null() {
        FsDiffResultUnion {
            tag: 0,
            payload: FsDiffResultPayload {
                diff: ManuallyDrop::new(unsafe { diff.assume_init() }),
            },
        }
    } else {
        FsDiffResultUnion {
            tag: 1,
            payload: FsDiffResultPayload {
                error: ManuallyDrop::new(error),
            },
        }
    };
    unsafe { ptr::write(out, result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_atomic_write(path: TyrusString, data: TyrusString) {
    let Ok(path_str) = path_text(&path) else {
        fs_error!(c"ERR_FS_INVALID_PATH", c"path must be a valid string", ());
    };
    let Ok(data_str) = path_text(&data) else {
        fs_error!(c"ERR_FS_INVALID_DATA", c"data must be a valid string", ());
    };
    let resolved = resolve_path(&path_str);
    let target = Path::new(&resolved);
    let parent = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = match tempfile::Builder::new()
        .prefix(".tyrus-atomic-")
        .tempfile_in(parent)
    {
        Ok(file) => file,
        Err(_) => fs_error!(c"ERR_FS_WRITE_FAILED", c"failed to create temp file", ()),
    };
    if temporary.write_all(data_str.as_bytes()).is_err() {
        fs_error!(c"ERR_FS_WRITE_FAILED", c"failed to write temp file", ());
    }
    let (file, temp_path) = temporary.into_parts();
    drop(file);
    if replace_file_atomically(&temp_path, target).is_err() {
        fs_error!(
            c"ERR_FS_WRITE_FAILED",
            c"failed to finalize atomic write",
            ()
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_atomic_write_result(
    path: TyrusString,
    data: TyrusString,
    out: *mut FsSyncResult,
) {
    LAST_FS_ERROR.with(|slot| slot.set(None));
    unsafe { tyrus_fs_atomic_write(path, data) };
    unsafe { write_fs_void_result(out) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_temp_dir(out: *mut FsTempDir) {
    if out.is_null() {
        return;
    }
    let temp = std::env::temp_dir();
    let temp_str = temp.to_string_lossy().to_string();
    unsafe {
        (*out).path = alloc_string(&temp_str);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_temp_file() -> TyrusString {
    let temp_dir = std::env::temp_dir();
    let temp_name = format!("tyrus_{}.tmp", uuid::Uuid::new_v4());
    let temp_path = temp_dir.join(&temp_name);
    if fs::write(&temp_path, "").is_err() {
        set_fs_error(c"ERR_FS_TEMP_FILE_FAILED", c"failed to create temp file");
        return TyrusString::null();
    }
    alloc_string(temp_path.to_string_lossy().as_ref())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_temp_file_result(out: *mut FsSyncResult) {
    if out.is_null() {
        return;
    }
    LAST_FS_ERROR.with(|slot| slot.set(None));
    let value = unsafe { tyrus_fs_temp_file() };
    let mut error = FsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_fs_take_error(&mut error) };
    let result = if error.code.is_null() {
        FsSyncResult {
            tag: 0,
            payload: FsSyncResultPayload {
                string: ManuallyDrop::new(value),
            },
        }
    } else {
        FsSyncResult {
            tag: 1,
            payload: FsSyncResultPayload {
                error: ManuallyDrop::new(error),
            },
        }
    };
    unsafe { ptr::write(out, result) };
}

fn executor() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Runtime::new().expect("failed to create tokio runtime for tyrus:fs")
    })
}

unsafe fn fulfill_string(promise: *mut TyrusPromise, value: String) {
    unsafe {
        let string = alloc_string(&value);
        if string.is_null() {
            fulfill_string_error(promise, "ERR_FS_ALLOCATION_FAILED");
            return;
        }
        let slot = libc::malloc(std::mem::size_of::<FsSyncResult>()).cast::<FsSyncResult>();
        if slot.is_null() {
            string.release();
            crate::tyrus_promise_reject_pending_structured(
                promise,
                TyrusString::literal("ERR_FS_ALLOCATION_FAILED"),
                TyrusString::literal("failed to allocate the result"),
            );
            return;
        }
        slot.write(FsSyncResult {
            tag: 0,
            payload: FsSyncResultPayload {
                string: ManuallyDrop::new(string),
            },
        });
        crate::tyrus_promise_fulfill_owned(promise, slot.cast(), Some(drop_fs_string_result));
    }
}

unsafe fn fulfill_string_error(promise: *mut TyrusPromise, code: &'static str) {
    unsafe { fulfill_fs_error_result(promise, code, true) };
}

unsafe fn fulfill_void(promise: *mut TyrusPromise) {
    unsafe {
        let slot = libc::malloc(std::mem::size_of::<FsSyncResult>()).cast::<FsSyncResult>();
        if slot.is_null() {
            crate::tyrus_promise_reject_pending_structured(
                promise,
                TyrusString::literal("ERR_FS_ALLOCATION_FAILED"),
                TyrusString::literal("failed to allocate the result"),
            );
            return;
        }
        slot.write(FsSyncResult {
            tag: 0,
            payload: FsSyncResultPayload { words: [0; 4] },
        });
        crate::tyrus_promise_fulfill_owned(promise, slot.cast(), Some(drop_fs_void_result));
    }
}

unsafe fn fulfill_void_error(promise: *mut TyrusPromise, code: &'static str) {
    unsafe { fulfill_fs_error_result(promise, code, false) };
}

unsafe fn fulfill_fs_error_result(
    promise: *mut TyrusPromise,
    code: &'static str,
    string_result: bool,
) {
    unsafe {
        let message = fs_error_message(code);
        let slot = libc::malloc(std::mem::size_of::<FsSyncResult>()).cast::<FsSyncResult>();
        if slot.is_null() {
            crate::tyrus_promise_reject_pending_structured(
                promise,
                TyrusString::literal("ERR_FS_ALLOCATION_FAILED"),
                TyrusString::literal("failed to allocate the result"),
            );
            return;
        }
        slot.write(FsSyncResult {
            tag: 1,
            payload: FsSyncResultPayload {
                error: ManuallyDrop::new(FsErrorTaken {
                    code: TyrusString::literal(code),
                    message: TyrusString::literal(message),
                }),
            },
        });
        crate::tyrus_promise_fulfill_owned(
            promise,
            slot.cast(),
            Some(if string_result {
                drop_fs_string_result
            } else {
                drop_fs_void_result
            }),
        );
    }
}

fn fs_error_message(code: &str) -> &'static str {
    match code {
        "ERR_FS_INVALID_PATH" => "path must be a valid string",
        "ERR_FS_INVALID_DATA" => "data must be a valid string",
        "ERR_FS_READ_FAILED" => "failed to read file",
        "ERR_FS_GLOB_FAILED" => "failed to search glob paths",
        "ERR_FS_DIFF_FAILED" => "failed to compare paths",
        "ERR_FS_SEARCH_FAILED" => "failed to search directory",
        "ERR_FS_WRITE_FAILED" => "failed to write file",
        "ERR_FS_APPEND_FAILED" => "failed to append to file",
        "ERR_FS_ALLOCATION_FAILED" => "failed to allocate the result",
        _ => "file operation failed",
    }
}

unsafe extern "C" fn drop_fs_string_result(value: *mut std::ffi::c_void) {
    unsafe {
        if value.is_null() {
            return;
        }
        let result = value.cast::<FsSyncResult>();
        if (*result).tag == 0 {
            ManuallyDrop::take(&mut (*result).payload.string).release();
        }
        libc::free(value);
    }
}

unsafe extern "C" fn drop_fs_void_result(value: *mut std::ffi::c_void) {
    if !value.is_null() {
        unsafe { libc::free(value) };
    }
}
