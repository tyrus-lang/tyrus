use super::{
    ArchiveFailure, ArchiveResult, ERR_INVALID_OPTION, EntryInfo, PackOptions, Summary,
    UnpackOptions, list, pack, unpack,
};
use crate::abi::TyrusArray;
use crate::executor::executor;
use crate::promise::TyrusPromise;
use crate::strings::alloc_string;
use std::ffi::c_void;
use std::mem::ManuallyDrop;
use tyrus_string::TyrusString;

/// Layout of `PackOptions` as the compiler lowers it: every field is optional,
/// so each is a pointer to its value, null when the field was omitted.
#[repr(C)]
pub struct PackOptionsFfi {
    pub level: *const i32,
    pub overwrite: *const bool,
    pub exclude: *const TyrusArray,
    pub prefix: *const TyrusString,
    pub reproducible: *const bool,
    pub timeout_ms: *const u64,
}

#[repr(C)]
pub struct UnpackOptionsFfi {
    pub overwrite: *const bool,
    pub strip_components: *const u32,
    pub max_entries: *const u64,
    pub max_bytes: *const u64,
    pub max_ratio: *const u32,
    pub timeout_ms: *const u64,
}

#[repr(C)]
pub struct ArchiveSummaryFfi {
    pub entries: u64,
    pub bytes: u64,
}

#[repr(C)]
pub struct ArchiveEntryFfi {
    pub name: TyrusString,
    pub kind: i32,
    pub size: u64,
    pub compressed_size: *mut u64,
}

/// Layout of `ArchiveError`. `entry` is a `string?`, boxed like every
/// optional: a `malloc`ed `TyrusString`, or null when absent.
#[repr(C)]
pub struct ArchiveErrorFfi {
    pub code: TyrusString,
    pub message: TyrusString,
    pub entry: *mut TyrusString,
}

/// Words of the payload the compiler reserves for a `Result` whose variants
/// are an archive value and `ArchiveError`: the largest variant, the 40-byte
/// `ArchiveError`, rounded up to whole `u64` words.
const RESULT_PAYLOAD_WORDS: usize = 5;

/// Layout of `Result<T, ArchiveError>` as the compiler lowers any `Result`: a
/// one-byte tag (`0` for `Ok`, `1` for `Err`) followed, at the next 8-byte
/// boundary, by the payload of the chosen variant. Every archive function
/// fulfils its promise with one of these; none of them rejects.
#[repr(C)]
pub struct ArchiveResultFfi<T> {
    pub tag: u8,
    pub payload: ArchiveResultPayload<T>,
}

#[repr(C)]
pub union ArchiveResultPayload<T> {
    pub ok: ManuallyDrop<T>,
    pub err: ManuallyDrop<ArchiveErrorFfi>,
    words: [u64; RESULT_PAYLOAD_WORDS],
}

const RESULT_TAG_OK: u8 = 0;
const RESULT_TAG_ERR: u8 = 1;

const _: () = {
    assert!(std::mem::size_of::<ArchiveErrorFfi>() == 40);
    assert!(std::mem::size_of::<ArchiveResultFfi<ArchiveSummaryFfi>>() == 48);
    assert!(std::mem::size_of::<ArchiveResultFfi<TyrusArray>>() == 48);
    assert!(std::mem::offset_of!(ArchiveResultFfi<TyrusArray>, payload) == 8);
};

unsafe fn owned_text(value: &TyrusString) -> Option<String> {
    unsafe { (!value.is_null()).then(|| crate::path::resolve(value.as_str())) }
}

unsafe fn read_string_array(array: *const TyrusArray) -> Vec<String> {
    unsafe {
        let Some(array) = array.as_ref() else {
            return Vec::new();
        };
        if array.data.is_null() {
            return Vec::new();
        }
        std::slice::from_raw_parts(array.data.cast::<TyrusString>(), array.length)
            .iter()
            .filter(|value| !value.is_null())
            .map(|value| value.as_str().to_owned())
            .collect()
    }
}

unsafe fn read_pack_options(options: *const PackOptionsFfi) -> PackOptions {
    unsafe {
        let Some(options) = options.as_ref() else {
            return PackOptions::default();
        };
        PackOptions {
            level: options.level.as_ref().copied(),
            overwrite: options.overwrite.as_ref().copied().unwrap_or(false),
            exclude: read_string_array(options.exclude),
            prefix: options
                .prefix
                .as_ref()
                .filter(|prefix| !prefix.is_null())
                .map(|prefix| prefix.as_str().to_owned()),
            reproducible: options.reproducible.as_ref().copied().unwrap_or(false),
            timeout_ms: options.timeout_ms.as_ref().copied(),
        }
    }
}

unsafe fn read_unpack_options(options: *const UnpackOptionsFfi) -> UnpackOptions {
    unsafe {
        let Some(options) = options.as_ref() else {
            return UnpackOptions::default();
        };
        UnpackOptions {
            overwrite: options.overwrite.as_ref().copied().unwrap_or(false),
            strip_components: options.strip_components.as_ref().copied().unwrap_or(0),
            max_entries: options.max_entries.as_ref().copied(),
            max_bytes: options.max_bytes.as_ref().copied(),
            max_ratio: options.max_ratio.as_ref().copied(),
            timeout_ms: options.timeout_ms.as_ref().copied(),
        }
    }
}

unsafe fn release_error(error: &mut ArchiveErrorFfi) {
    unsafe {
        std::ptr::replace(&mut error.code, TyrusString::null()).release();
        std::ptr::replace(&mut error.message, TyrusString::null()).release();
        let entry = std::mem::replace(&mut error.entry, std::ptr::null_mut());
        if !entry.is_null() {
            std::ptr::replace(entry, TyrusString::null()).release();
            libc::free(entry.cast());
        }
    }
}

unsafe fn box_optional_string(value: &str) -> *mut TyrusString {
    unsafe {
        let string = alloc_string(value);
        if string.is_null() {
            return std::ptr::null_mut();
        }
        let storage = libc::malloc(std::mem::size_of::<TyrusString>()).cast::<TyrusString>();
        if storage.is_null() {
            string.release();
            return std::ptr::null_mut();
        }
        std::ptr::write(storage, string);
        storage
    }
}

/// `code` and `message` are static literals; only `entry` allocates, and a
/// failed allocation just leaves it absent.
unsafe fn error_value(failure: &ArchiveFailure) -> ArchiveErrorFfi {
    unsafe {
        ArchiveErrorFfi {
            code: TyrusString::literal(failure.code),
            message: TyrusString::literal(failure.message()),
            entry: failure
                .entry
                .as_deref()
                .map_or(std::ptr::null_mut(), |entry| box_optional_string(entry)),
        }
    }
}

/// Fulfils `promise` with a boxed `Result`. When the box itself cannot be
/// allocated there is no value left to deliver: what the result holds is
/// released and the promise is rejected with the structured allocation error
/// every `catch` can read.
unsafe fn fulfill_result<T>(
    promise: *mut TyrusPromise,
    value: ArchiveResultFfi<T>,
    kind: ResultKind,
) {
    unsafe {
        let boxed =
            libc::malloc(std::mem::size_of::<ArchiveResultFfi<T>>()).cast::<ArchiveResultFfi<T>>();
        if boxed.is_null() {
            let mut value = value;
            kind.release(std::ptr::addr_of_mut!(value).cast());
            crate::tyrus_promise_reject_pending_structured(
                promise,
                TyrusString::literal(super::ERR_ALLOCATION_FAILED),
                TyrusString::literal(super::message_for(super::ERR_ALLOCATION_FAILED)),
            );
            return;
        }
        std::ptr::write(boxed, value);
        crate::tyrus_promise_fulfill_owned(promise, boxed.cast(), Some(kind.drop()));
    }
}

#[derive(Clone, Copy)]
enum ResultKind {
    Summary,
    Entries,
}

impl ResultKind {
    /// Releases what a result holds, never the result's own storage.
    unsafe fn release(self, value: *mut c_void) {
        unsafe {
            match self {
                Self::Summary => {
                    let result = value.cast::<ArchiveResultFfi<ArchiveSummaryFfi>>();
                    if (*result).tag == RESULT_TAG_ERR {
                        release_error(&mut (*result).payload.err);
                    }
                }
                Self::Entries => {
                    let result = value.cast::<ArchiveResultFfi<TyrusArray>>();
                    if (*result).tag == RESULT_TAG_ERR {
                        release_error(&mut (*result).payload.err);
                    } else {
                        let array = &*(*result).payload.ok;
                        if !array.data.is_null() {
                            release_entries(array.data.cast(), array.length);
                        }
                    }
                }
            }
        }
    }

    fn drop(self) -> unsafe extern "C" fn(*mut c_void) {
        match self {
            Self::Summary => drop_summary_result,
            Self::Entries => drop_entries_result,
        }
    }
}

unsafe extern "C" fn drop_summary_result(value: *mut c_void) {
    unsafe {
        if value.is_null() {
            return;
        }
        ResultKind::Summary.release(value);
        libc::free(value);
    }
}

unsafe extern "C" fn drop_entries_result(value: *mut c_void) {
    unsafe {
        if value.is_null() {
            return;
        }
        ResultKind::Entries.release(value);
        libc::free(value);
    }
}

unsafe fn fulfill_failure<T>(
    promise: *mut TyrusPromise,
    failure: &ArchiveFailure,
    kind: ResultKind,
) {
    unsafe {
        let value = ArchiveResultFfi::<T> {
            tag: RESULT_TAG_ERR,
            payload: ArchiveResultPayload {
                err: ManuallyDrop::new(error_value(failure)),
            },
        };
        fulfill_result(promise, value, kind);
    }
}

unsafe fn fulfill_summary(promise: *mut TyrusPromise, outcome: ArchiveResult<Summary>) {
    unsafe {
        match outcome {
            Ok(summary) => {
                let value = ArchiveResultFfi {
                    tag: RESULT_TAG_OK,
                    payload: ArchiveResultPayload {
                        ok: ManuallyDrop::new(ArchiveSummaryFfi {
                            entries: summary.entries,
                            bytes: summary.bytes,
                        }),
                    },
                };
                fulfill_result(promise, value, ResultKind::Summary);
            }
            Err(failure) => {
                fulfill_failure::<ArchiveSummaryFfi>(promise, &failure, ResultKind::Summary);
            }
        }
    }
}

unsafe fn release_entries(data: *mut ArchiveEntryFfi, length: usize) {
    unsafe {
        for index in 0..length {
            let entry = data.add(index);
            std::ptr::replace(&mut (*entry).name, TyrusString::null()).release();
            if !(*entry).compressed_size.is_null() {
                libc::free((*entry).compressed_size.cast());
            }
        }
        libc::free(data.cast());
    }
}

unsafe fn build_entry(info: &EntryInfo) -> Option<ArchiveEntryFfi> {
    unsafe {
        let name = alloc_string(&info.name);
        if name.is_null() {
            return None;
        }
        let compressed_size = match info.compressed_size {
            Some(size) => {
                let slot = libc::malloc(std::mem::size_of::<u64>()).cast::<u64>();
                if slot.is_null() {
                    name.release();
                    return None;
                }
                std::ptr::write(slot, size);
                slot
            }
            None => std::ptr::null_mut(),
        };
        Some(ArchiveEntryFfi {
            name,
            kind: info.kind as i32,
            size: info.size,
            compressed_size,
        })
    }
}

unsafe fn entry_array(entries: &[EntryInfo]) -> Option<TyrusArray> {
    unsafe {
        let bytes = std::mem::size_of::<ArchiveEntryFfi>() * entries.len().max(1);
        let data = libc::malloc(bytes).cast::<ArchiveEntryFfi>();
        if data.is_null() {
            return None;
        }
        for (index, info) in entries.iter().enumerate() {
            let Some(entry) = build_entry(info) else {
                release_entries(data, index);
                return None;
            };
            std::ptr::write(data.add(index), entry);
        }
        Some(TyrusArray {
            data: data.cast(),
            length: entries.len(),
            capacity: entries.len().max(1),
        })
    }
}

unsafe fn fulfill_entries(promise: *mut TyrusPromise, outcome: ArchiveResult<Vec<EntryInfo>>) {
    unsafe {
        let array = match outcome {
            Ok(entries) => entry_array(&entries)
                .ok_or_else(|| ArchiveFailure::new(super::ERR_ALLOCATION_FAILED)),
            Err(failure) => Err(failure),
        };
        match array {
            Ok(array) => {
                let value = ArchiveResultFfi {
                    tag: RESULT_TAG_OK,
                    payload: ArchiveResultPayload {
                        ok: ManuallyDrop::new(array),
                    },
                };
                fulfill_result(promise, value, ResultKind::Entries);
            }
            Err(failure) => fulfill_failure::<TyrusArray>(promise, &failure, ResultKind::Entries),
        }
    }
}

/// Runs `work` on the blocking pool and fulfils `promise` with its `Result`.
/// Archive work is disk- and CPU-bound, so it never runs on the executor
/// threads that poll the program's own async functions.
unsafe fn settle_in_background<T, W, F>(work: W, fulfill: F) -> *mut TyrusPromise
where
    T: Send + 'static,
    W: FnOnce() -> ArchiveResult<T> + Send + 'static,
    F: FnOnce(*mut TyrusPromise, ArchiveResult<T>) + Send + 'static,
{
    unsafe {
        let promise = crate::tyrus_promise_pending();
        crate::tyrus_promise_retain(promise);
        let address = promise as usize;
        executor().spawn(async move {
            let outcome = tokio::task::spawn_blocking(work)
                .await
                .unwrap_or_else(|_| Err(ArchiveFailure::new(super::ERR_IO)));
            let promise = address as *mut TyrusPromise;
            fulfill(promise, outcome);
            crate::tyrus_promise_free(promise);
        });
        promise
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_archive_pack(
    source: TyrusString,
    destination: TyrusString,
    options: *const PackOptionsFfi,
) -> *mut TyrusPromise {
    unsafe {
        let paths = (owned_text(&source), owned_text(&destination));
        let options = read_pack_options(options);
        settle_in_background(
            move || match paths {
                (Some(source), Some(destination)) => pack(&source, &destination, &options),
                _ => Err(ArchiveFailure::new(ERR_INVALID_OPTION)),
            },
            |promise, outcome| fulfill_summary(promise, outcome),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_archive_list(
    archive: TyrusString,
    options: *const UnpackOptionsFfi,
) -> *mut TyrusPromise {
    unsafe {
        let archive = owned_text(&archive);
        let options = read_unpack_options(options);
        settle_in_background(
            move || match archive {
                Some(archive) => list(&archive, &options),
                None => Err(ArchiveFailure::new(ERR_INVALID_OPTION)),
            },
            |promise, outcome| fulfill_entries(promise, outcome),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_archive_unpack(
    archive: TyrusString,
    destination: TyrusString,
    options: *const UnpackOptionsFfi,
) -> *mut TyrusPromise {
    unsafe {
        let paths = (owned_text(&archive), owned_text(&destination));
        let options = read_unpack_options(options);
        settle_in_background(
            move || match paths {
                (Some(archive), Some(destination)) => unpack(&archive, &destination, &options),
                _ => Err(ArchiveFailure::new(ERR_INVALID_OPTION)),
            },
            |promise, outcome| fulfill_summary(promise, outcome),
        )
    }
}

/// `ArchiveFormat` ordinals, part of the ABI.
fn archive_kind(ordinal: i32) -> Option<super::ArchiveKind> {
    match ordinal {
        0 => Some(super::ArchiveKind::Zip),
        1 => Some(super::ArchiveKind::Tar),
        2 => Some(super::ArchiveKind::TarGz),
        _ => None,
    }
}

/// An owned `ReadableStream<u8[]>` the archive is written into while the
/// program reads it. A failure, from a missing source to a full timeout,
/// fails the stream with the archive error instead of ending it.
///
/// # Safety
///
/// `options` must be null or point to a `PackOptions` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_archive_pack_stream(
    source: TyrusString,
    format: i32,
    options: *const PackOptionsFfi,
) -> *const crate::stream::TyrusReadableStream {
    unsafe {
        let (readable, producer) =
            crate::stream::byte_readable_stream(super::streaming::CHUNK_BYTES);
        let source = owned_text(&source);
        let options = read_pack_options(options);
        let (Some(source), Some(kind)) = (source, archive_kind(format)) else {
            producer.fail_with(crate::stream::StreamError {
                code: ERR_INVALID_OPTION,
                message: super::message_for(ERR_INVALID_OPTION),
            });
            return readable;
        };
        let handle = executor().handle().clone();
        executor().spawn_blocking(move || {
            super::streaming::pack_to_stream(kind, &source, &options, producer, handle);
        });
        readable
    }
}

/// Extracts the TAR or TAR.GZ `source` carries. The stream is borrowed: the
/// call takes its own reference and the program keeps its handle.
///
/// # Safety
///
/// `source` must be a live `ReadableStream` handle, and `options` null or a
/// pointer to an `UnpackOptions` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_archive_unpack_stream(
    source: *const crate::stream::TyrusReadableStream,
    destination: TyrusString,
    options: *const UnpackOptionsFfi,
) -> *mut TyrusPromise {
    unsafe {
        let stream = (!source.is_null()).then(|| {
            crate::stream::take_owned_readable_stream(crate::stream::tyrus_readable_stream_retain(
                source,
            ))
        });
        let destination = owned_text(&destination);
        let options = read_unpack_options(options);
        let handle = executor().handle().clone();
        settle_in_background(
            move || match (stream, destination) {
                (Some(stream), Some(destination)) => {
                    super::streaming::unpack_from_stream(stream, &destination, &options, handle)
                }
                _ => Err(ArchiveFailure::new(ERR_INVALID_OPTION)),
            },
            |promise, outcome| fulfill_summary(promise, outcome),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> std::path::PathBuf {
        let scratch =
            std::env::temp_dir().join(format!("tyrus-archive-ffi-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&scratch).unwrap();
        scratch
    }

    unsafe fn settled_result<T>(promise: *mut TyrusPromise) -> *const ArchiveResultFfi<T> {
        unsafe {
            crate::wait_until_settled(promise);
            assert_eq!(
                crate::tyrus_promise_state(promise),
                crate::TYRUS_PROMISE_FULFILLED,
                "archive promises fulfil with a Result, they never reject"
            );
            crate::promise::tyrus_promise_await(promise).cast()
        }
    }

    #[test]
    fn a_failure_fulfils_with_an_err_carrying_code_message_and_entry() {
        let scratch = scratch();
        let archive = scratch.join("missing.zip").to_string_lossy().into_owned();
        let destination = scratch.join("out").to_string_lossy().into_owned();
        unsafe {
            let promise = tyrus_archive_unpack(
                TyrusString::lend(&archive).unwrap(),
                TyrusString::lend(&destination).unwrap(),
                std::ptr::null(),
            );
            let result = &*settled_result::<ArchiveSummaryFfi>(promise);
            assert_eq!(result.tag, RESULT_TAG_ERR);
            let error = &result.payload.err;
            assert_eq!(error.code.as_str(), "ERR_ARCHIVE_NOT_FOUND");
            assert_eq!(
                error.message.as_str(),
                "the archive or source path does not exist"
            );
            assert!(error.entry.is_null());
            crate::tyrus_promise_free(promise);
        }
        let _ = std::fs::remove_dir_all(scratch);
    }

    #[test]
    fn pack_and_list_fulfil_with_ok_payloads() {
        let scratch = scratch();
        std::fs::create_dir_all(scratch.join("src")).unwrap();
        std::fs::write(scratch.join("src/a.txt"), b"hello").unwrap();
        let source = scratch.join("src").to_string_lossy().into_owned();
        let archive = scratch.join("a.tar.gz").to_string_lossy().into_owned();
        unsafe {
            let promise = tyrus_archive_pack(
                TyrusString::lend(&source).unwrap(),
                TyrusString::lend(&archive).unwrap(),
                std::ptr::null(),
            );
            let result = &*settled_result::<ArchiveSummaryFfi>(promise);
            assert_eq!(result.tag, RESULT_TAG_OK);
            assert_eq!((result.payload.ok.entries, result.payload.ok.bytes), (1, 5));
            crate::tyrus_promise_free(promise);

            let promise =
                tyrus_archive_list(TyrusString::lend(&archive).unwrap(), std::ptr::null());
            let result = &*settled_result::<TyrusArray>(promise);
            assert_eq!(result.tag, RESULT_TAG_OK);
            let array = &result.payload.ok;
            assert_eq!(array.length, 1);
            let entry = &*array.data.cast::<ArchiveEntryFfi>();
            assert_eq!(entry.name.as_str(), "a.txt");
            assert_eq!(entry.kind, 0);
            assert!(entry.compressed_size.is_null());
            crate::tyrus_promise_free(promise);
        }
        let _ = std::fs::remove_dir_all(scratch);
    }

    #[test]
    fn an_offending_entry_travels_in_the_err_payload() {
        let scratch = scratch();
        let mut header = tar::Header::new_gnu();
        header.as_old_mut().name[..10].copy_from_slice(b"../escaped");
        header.set_size(1);
        header.set_mode(0o644);
        header.set_cksum();
        let mut bytes = header.as_bytes().to_vec();
        bytes.extend_from_slice(&[b'x'; 512]);
        bytes.extend_from_slice(&[0; 1024]);
        std::fs::write(scratch.join("evil.tar"), bytes).unwrap();
        let archive = scratch.join("evil.tar").to_string_lossy().into_owned();
        let destination = scratch.join("out").to_string_lossy().into_owned();
        unsafe {
            let promise = tyrus_archive_unpack(
                TyrusString::lend(&archive).unwrap(),
                TyrusString::lend(&destination).unwrap(),
                std::ptr::null(),
            );
            let result = &*settled_result::<ArchiveSummaryFfi>(promise);
            assert_eq!(result.tag, RESULT_TAG_ERR);
            let error = &result.payload.err;
            assert_eq!(error.code.as_str(), "ERR_ARCHIVE_PATH_ESCAPE");
            assert_eq!((*error.entry).as_str(), "../escaped");
            crate::tyrus_promise_free(promise);
        }
        let _ = std::fs::remove_dir_all(scratch);
    }
}
