#[path = "entry_name/entry_name.rs"]
mod entry_name;
#[path = "ffi/ffi.rs"]
mod ffi;
#[path = "format/format.rs"]
mod format;
#[path = "pack/pack.rs"]
mod pack;
#[path = "streaming/streaming.rs"]
mod streaming;
#[path = "tar_reader/tar_reader.rs"]
mod tar_reader;
#[path = "unpack/unpack.rs"]
mod unpack;

#[cfg(test)]
#[path = "interop_tests/interop_tests.rs"]
mod interop_tests;
#[cfg(test)]
#[path = "tests/tests.rs"]
mod tests;

pub use ffi::*;
pub(crate) use pack::pack;
use std::cell::Cell;
use std::io::{Read, Seek, SeekFrom, Write};
use std::rc::Rc;
use std::time::{Duration, Instant};
pub(crate) use unpack::{list, unpack};

pub(crate) const DEFAULT_MAX_ENTRIES: u64 = 100_000;
pub(crate) const DEFAULT_MAX_BYTES: u64 = 4 * 1024 * 1024 * 1024;
pub(crate) const DEFAULT_MAX_RATIO: u32 = 200;
/// Ten minutes: room to extract the default 4 GiB `maxBytes` even on a slow
/// disk. A program that raises `maxBytes` can raise `timeoutMs` with it.
pub(crate) const DEFAULT_TIMEOUT_MS: u64 = 10 * 60 * 1000;

/// Longest entry name an `ArchiveError` carries, in bytes. The name comes from
/// the archive, so an attacker controls its length; the error must not become a
/// way to force a large allocation.
const MAX_ERROR_ENTRY_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArchiveKind {
    Zip,
    Tar,
    TarGz,
}

/// Ordinals are part of the ABI shared with the `ArchiveEntryKind` enum the
/// compiler registers; keep them in step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub(crate) enum EntryKind {
    File = 0,
    Directory = 1,
    Symlink = 2,
    HardLink = 3,
    Other = 4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EntryInfo {
    pub(crate) name: String,
    pub(crate) kind: EntryKind,
    pub(crate) size: u64,
    pub(crate) compressed_size: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Summary {
    pub(crate) entries: u64,
    pub(crate) bytes: u64,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct PackOptions {
    pub(crate) level: Option<i32>,
    pub(crate) overwrite: bool,
    pub(crate) exclude: Vec<String>,
    pub(crate) prefix: Option<String>,
    pub(crate) reproducible: bool,
    pub(crate) timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct UnpackOptions {
    pub(crate) overwrite: bool,
    pub(crate) strip_components: u32,
    pub(crate) max_entries: Option<u64>,
    pub(crate) max_bytes: Option<u64>,
    pub(crate) max_ratio: Option<u32>,
    pub(crate) timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArchiveFailure {
    pub(crate) code: &'static str,
    pub(crate) entry: Option<String>,
}

impl ArchiveFailure {
    pub(crate) fn new(code: &'static str) -> Self {
        Self { code, entry: None }
    }

    pub(crate) fn at(code: &'static str, entry: &str) -> Self {
        Self {
            code,
            entry: Some(truncate_entry(entry)),
        }
    }

    pub(crate) fn message(&self) -> &'static str {
        message_for(self.code)
    }
}

pub(crate) type ArchiveResult<T> = Result<T, ArchiveFailure>;

fn truncate_entry(entry: &str) -> String {
    if entry.len() <= MAX_ERROR_ENTRY_BYTES {
        return entry.to_owned();
    }
    let mut end = MAX_ERROR_ENTRY_BYTES;
    while !entry.is_char_boundary(end) {
        end -= 1;
    }
    entry[..end].to_owned()
}

pub(crate) const ERR_NOT_FOUND: &str = "ERR_ARCHIVE_NOT_FOUND";
pub(crate) const ERR_IO: &str = "ERR_ARCHIVE_IO";
pub(crate) const ERR_PERMISSION_DENIED: &str = "ERR_ARCHIVE_PERMISSION_DENIED";
pub(crate) const ERR_UNSUPPORTED_FORMAT: &str = "ERR_ARCHIVE_UNSUPPORTED_FORMAT";
pub(crate) const ERR_UNSUPPORTED_EXTENSION: &str = "ERR_ARCHIVE_UNSUPPORTED_EXTENSION";
pub(crate) const ERR_INVALID_ARCHIVE: &str = "ERR_ARCHIVE_INVALID_ARCHIVE";
pub(crate) const ERR_CHECKSUM_MISMATCH: &str = "ERR_ARCHIVE_CHECKSUM_MISMATCH";
pub(crate) const ERR_ENCRYPTED: &str = "ERR_ARCHIVE_ENCRYPTED";
pub(crate) const ERR_INVALID_OPTION: &str = "ERR_ARCHIVE_INVALID_OPTION";
pub(crate) const ERR_INVALID_COMPRESSION_LEVEL: &str = "ERR_ARCHIVE_INVALID_COMPRESSION_LEVEL";
pub(crate) const ERR_DESTINATION_EXISTS: &str = "ERR_ARCHIVE_DESTINATION_EXISTS";
pub(crate) const ERR_PATH_ESCAPE: &str = "ERR_ARCHIVE_PATH_ESCAPE";
pub(crate) const ERR_INVALID_ENTRY_NAME: &str = "ERR_ARCHIVE_INVALID_ENTRY_NAME";
pub(crate) const ERR_DUPLICATE_ENTRY: &str = "ERR_ARCHIVE_DUPLICATE_ENTRY";
pub(crate) const ERR_UNSAFE_LINK: &str = "ERR_ARCHIVE_UNSAFE_LINK";
pub(crate) const ERR_UNSUPPORTED_ENTRY: &str = "ERR_ARCHIVE_UNSUPPORTED_ENTRY";
pub(crate) const ERR_ENTRY_LIMIT: &str = "ERR_ARCHIVE_ENTRY_LIMIT";
pub(crate) const ERR_SIZE_LIMIT: &str = "ERR_ARCHIVE_SIZE_LIMIT";
pub(crate) const ERR_RATIO_LIMIT: &str = "ERR_ARCHIVE_RATIO_LIMIT";
pub(crate) const ERR_ALLOCATION_FAILED: &str = "ERR_ARCHIVE_ALLOCATION_FAILED";
pub(crate) const ERR_NO_SPACE: &str = "ERR_ARCHIVE_NO_SPACE";
pub(crate) const ERR_TIMEOUT: &str = "ERR_ARCHIVE_TIMEOUT";
pub(crate) const ERR_ZIP_NOT_STREAMABLE: &str = "ERR_ARCHIVE_ZIP_NOT_STREAMABLE";

pub(crate) fn message_for(code: &str) -> &'static str {
    match code {
        ERR_NOT_FOUND => "the archive or source path does not exist",
        ERR_IO => "an I/O error occurred while processing the archive",
        ERR_PERMISSION_DENIED => "permission was denied while processing the archive",
        ERR_UNSUPPORTED_FORMAT => "the archive format is not supported",
        ERR_UNSUPPORTED_EXTENSION => {
            "the destination must end with .zip, .tar, .tar.gz or .tgz in lowercase"
        }
        ERR_INVALID_ARCHIVE => "the archive is malformed or truncated",
        ERR_CHECKSUM_MISMATCH => "an archive entry failed its checksum",
        ERR_ENCRYPTED => "encrypted archive entries are not supported",
        ERR_INVALID_OPTION => "an archive option has an invalid value",
        ERR_INVALID_COMPRESSION_LEVEL => {
            "the compression level is outside the range the format accepts"
        }
        ERR_DESTINATION_EXISTS => "the destination already exists",
        ERR_PATH_ESCAPE => "an archive entry resolves outside the destination directory",
        ERR_INVALID_ENTRY_NAME => "an archive entry has an invalid or non-portable name",
        ERR_DUPLICATE_ENTRY => "an archive entry collides with another entry",
        ERR_UNSAFE_LINK => "symbolic and hard links are not extracted",
        ERR_UNSUPPORTED_ENTRY => "the archive contains an entry type that is not supported",
        ERR_ENTRY_LIMIT => "the archive exceeds the maximum number of entries",
        ERR_SIZE_LIMIT => "the archive exceeds the maximum extracted size",
        ERR_RATIO_LIMIT => "an archive entry exceeds the maximum compression ratio",
        ERR_ALLOCATION_FAILED => "archive memory allocation failed",
        ERR_NO_SPACE => "there is not enough disk space to finish the archive operation",
        ERR_TIMEOUT => "the archive operation exceeded its time limit",
        ERR_ZIP_NOT_STREAMABLE => {
            "a ZIP archive cannot be extracted from a stream; write it to a file and use `unpack`"
        }
        _ => "the archive operation failed",
    }
}

pub(crate) fn io_failure(error: &std::io::Error) -> ArchiveFailure {
    ArchiveFailure::new(io_code(error))
}

pub(crate) fn io_failure_at(error: &std::io::Error, entry: &str) -> ArchiveFailure {
    ArchiveFailure::at(io_code(error), entry)
}

fn io_code(error: &std::io::Error) -> &'static str {
    match error.kind() {
        std::io::ErrorKind::NotFound => ERR_NOT_FOUND,
        std::io::ErrorKind::PermissionDenied => ERR_PERMISSION_DENIED,
        std::io::ErrorKind::OutOfMemory => ERR_ALLOCATION_FAILED,
        std::io::ErrorKind::StorageFull | std::io::ErrorKind::QuotaExceeded => ERR_NO_SPACE,
        std::io::ErrorKind::TimedOut => ERR_TIMEOUT,
        _ => ERR_IO,
    }
}

/// When an operation must give up. Every limit of the module can be raised
/// but not switched off, so a timeout of `0` is an invalid option rather than
/// "no timeout"; only a timeout too large to represent never expires.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Deadline(Option<Instant>);

impl Deadline {
    pub(crate) fn from_timeout(timeout_ms: Option<u64>) -> ArchiveResult<Self> {
        let timeout = timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
        if timeout == 0 {
            return Err(ArchiveFailure::new(ERR_INVALID_OPTION));
        }
        Ok(Self(
            Instant::now().checked_add(Duration::from_millis(timeout)),
        ))
    }

    #[cfg(test)]
    pub(crate) fn expired() -> Self {
        Self(Some(Instant::now()))
    }

    fn passed(self) -> bool {
        self.0.is_some_and(|instant| Instant::now() >= instant)
    }

    /// Time left before the deadline, `None` when it never expires.
    pub(crate) fn remaining(self) -> Option<Duration> {
        self.0
            .map(|instant| instant.saturating_duration_since(Instant::now()))
    }

    pub(crate) fn check(self) -> ArchiveResult<()> {
        if self.passed() {
            return Err(ArchiveFailure::new(ERR_TIMEOUT));
        }
        Ok(())
    }

    pub(crate) fn check_at(self, entry: &str) -> ArchiveResult<()> {
        if self.passed() {
            return Err(ArchiveFailure::at(ERR_TIMEOUT, entry));
        }
        Ok(())
    }
}

/// Wraps every file the module reads. It fails a read once the deadline has
/// passed, as `TimedOut`, which the gzip, tar and zip readers pass through
/// untouched and `io_code` turns into `ERR_ARCHIVE_TIMEOUT`; that covers work
/// no loop of ours sees, such as skipping the data of a listed tar member. It
/// also counts the bytes read, which is the compressed side of the aggregate
/// ratio of a gzip stream.
pub(crate) struct GuardedReader<R> {
    inner: R,
    deadline: Deadline,
    consumed: Rc<Cell<u64>>,
}

impl<R> GuardedReader<R> {
    pub(crate) fn new(inner: R, deadline: Deadline) -> Self {
        Self {
            inner,
            deadline,
            consumed: Rc::new(Cell::new(0)),
        }
    }

    pub(crate) fn consumed(&self) -> Rc<Cell<u64>> {
        Rc::clone(&self.consumed)
    }
}

impl<R: Read> Read for GuardedReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.deadline.passed() {
            return Err(std::io::ErrorKind::TimedOut.into());
        }
        let count = self.inner.read(buffer)?;
        self.consumed
            .set(self.consumed.get().saturating_add(count as u64));
        Ok(count)
    }
}

impl<R: Seek> Seek for GuardedReader<R> {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(position)
    }
}

/// Every file the module writes goes through this. It only forwards, except
/// in tests, where `with_write_budget` makes it fail like a full disk once a
/// byte budget is spent, so the `ENOSPC` paths of `pack` and `unpack` run
/// without filling a real disk.
pub(crate) struct DiskWriter<W> {
    inner: W,
}

impl<W> DiskWriter<W> {
    pub(crate) fn new(inner: W) -> Self {
        Self { inner }
    }

    pub(crate) fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: Write> Write for DiskWriter<W> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        #[cfg(test)]
        let buffer = match WRITE_BUDGET.with(Cell::get) {
            Some(0) => return Err(std::io::ErrorKind::StorageFull.into()),
            Some(budget) => {
                let allowed = buffer
                    .len()
                    .min(usize::try_from(budget).unwrap_or(usize::MAX));
                WRITE_BUDGET.with(|slot| slot.set(Some(budget - allowed as u64)));
                &buffer[..allowed]
            }
            None => buffer,
        };
        self.inner.write(buffer)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl<W: Seek> Seek for DiskWriter<W> {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(position)
    }
}

#[cfg(test)]
thread_local! {
    static WRITE_BUDGET: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Runs `work` with every `DiskWriter` on this thread limited to `bytes` in
/// total, as if the disk filled up after them.
#[cfg(test)]
pub(crate) fn with_write_budget<T>(bytes: u64, work: impl FnOnce() -> T) -> T {
    WRITE_BUDGET.with(|slot| slot.set(Some(bytes)));
    let result = work();
    WRITE_BUDGET.with(|slot| slot.set(None));
    result
}
