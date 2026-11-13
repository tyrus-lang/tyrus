use super::entry_name::{EntryNameError, normalize_entry_name};
use super::format::{detect_kind, zip_declared_entries};
use super::tar_reader::for_each_member;
use super::{
    ArchiveFailure, ArchiveKind, ArchiveResult, DEFAULT_MAX_BYTES, DEFAULT_MAX_ENTRIES,
    DEFAULT_MAX_RATIO, Deadline, DiskWriter, ERR_CHECKSUM_MISMATCH, ERR_DESTINATION_EXISTS,
    ERR_DUPLICATE_ENTRY, ERR_ENCRYPTED, ERR_ENTRY_LIMIT, ERR_INVALID_ARCHIVE,
    ERR_INVALID_ENTRY_NAME, ERR_INVALID_OPTION, ERR_NO_SPACE, ERR_PATH_ESCAPE, ERR_RATIO_LIMIT,
    ERR_SIZE_LIMIT, ERR_UNSAFE_LINK, ERR_UNSUPPORTED_ENTRY, EntryInfo, EntryKind, GuardedReader,
    Summary, UnpackOptions, io_failure, io_failure_at,
};
use crate::codec::{DecoderReader, Format, exceeds_ratio};
use cap_std::ambient_authority;
use cap_std::fs::Dir;
use std::cell::Cell;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use unicode_normalization::UnicodeNormalization;

const COPY_BUFFER: usize = 64 * 1024;
const READ_BUFFER: usize = 64 * 1024;
const ZIP_FILE_TYPE_MASK: u32 = 0o170_000;
const ZIP_REGULAR: u32 = 0o100_000;
const ZIP_DIRECTORY: u32 = 0o040_000;
const ZIP_SYMLINK: u32 = 0o120_000;

type ArchiveReader = GuardedReader<File>;
type ZipReader = zip::ZipArchive<BufReader<ArchiveReader>>;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct Limits {
    entries: u64,
    bytes: u64,
    ratio: u32,
}

fn limits(options: &UnpackOptions) -> ArchiveResult<Limits> {
    let invalid = || ArchiveFailure::new(ERR_INVALID_OPTION);
    let entries = options.max_entries.unwrap_or(DEFAULT_MAX_ENTRIES);
    let bytes = options.max_bytes.unwrap_or(DEFAULT_MAX_BYTES);
    let ratio = options.max_ratio.unwrap_or(DEFAULT_MAX_RATIO);
    if entries == 0 || bytes == 0 || ratio == 0 {
        return Err(invalid());
    }
    Ok(Limits {
        entries,
        bytes,
        ratio,
    })
}

fn open_archive(archive: &str, deadline: Deadline) -> ArchiveResult<(ArchiveReader, ArchiveKind)> {
    let mut file = File::open(archive).map_err(|error| io_failure(&error))?;
    if !file
        .metadata()
        .map_err(|error| io_failure(&error))?
        .is_file()
    {
        return Err(ArchiveFailure::new(super::ERR_UNSUPPORTED_FORMAT));
    }
    let kind = detect_kind(&mut file)?;
    Ok((GuardedReader::new(file, deadline), kind))
}

fn open_zip(mut file: ArchiveReader, limits: &Limits) -> ArchiveResult<ZipReader> {
    if zip_declared_entries(&mut file)? > limits.entries {
        return Err(ArchiveFailure::new(ERR_ENTRY_LIMIT));
    }
    let reader = BufReader::with_capacity(READ_BUFFER, file);
    zip::ZipArchive::new(reader).map_err(|error| zip_failure(&error, None))
}

fn zip_failure(error: &zip::result::ZipError, entry: Option<&str>) -> ArchiveFailure {
    let code = match error {
        zip::result::ZipError::UnsupportedArchive(_) => ERR_UNSUPPORTED_ENTRY,
        zip::result::ZipError::Io(error) => return io_or_invalid(error, entry),
        _ => ERR_INVALID_ARCHIVE,
    };
    match entry {
        Some(entry) => ArchiveFailure::at(code, entry),
        None => ArchiveFailure::new(code),
    }
}

fn io_or_invalid(error: &std::io::Error, entry: Option<&str>) -> ArchiveFailure {
    let code = match error.kind() {
        std::io::ErrorKind::InvalidData
        | std::io::ErrorKind::InvalidInput
        | std::io::ErrorKind::UnexpectedEof => ERR_INVALID_ARCHIVE,
        _ => return entry.map_or_else(|| io_failure(error), |entry| io_failure_at(error, entry)),
    };
    match entry {
        Some(entry) => ArchiveFailure::at(code, entry),
        None => ArchiveFailure::new(code),
    }
}

fn zip_entry_kind(file: &zip::read::ZipFile<'_, BufReader<ArchiveReader>>) -> EntryKind {
    match file.unix_mode().map(|mode| mode & ZIP_FILE_TYPE_MASK) {
        Some(ZIP_SYMLINK) => EntryKind::Symlink,
        Some(ZIP_DIRECTORY) => EntryKind::Directory,
        Some(0 | ZIP_REGULAR) | None if file.is_dir() => EntryKind::Directory,
        Some(0 | ZIP_REGULAR) | None => EntryKind::File,
        Some(_) => EntryKind::Other,
    }
}

pub(crate) fn list(archive: &str, options: &UnpackOptions) -> ArchiveResult<Vec<EntryInfo>> {
    list_until(
        archive,
        options,
        Deadline::from_timeout(options.timeout_ms)?,
    )
}

pub(super) fn list_until(
    archive: &str,
    options: &UnpackOptions,
    deadline: Deadline,
) -> ArchiveResult<Vec<EntryInfo>> {
    let limits = limits(options)?;
    let (file, kind) = open_archive(archive, deadline)?;
    match kind {
        ArchiveKind::Zip => {
            let mut zip = open_zip(file, &limits)?;
            let mut entries = Vec::with_capacity(zip.len());
            for index in 0..zip.len() {
                deadline.check()?;
                let file = zip
                    .by_index_raw(index)
                    .map_err(|error| zip_failure(&error, None))?;
                entries.push(EntryInfo {
                    name: file.name().to_owned(),
                    kind: zip_entry_kind(&file),
                    size: file.size(),
                    compressed_size: Some(file.compressed_size()),
                });
            }
            Ok(entries)
        }
        ArchiveKind::Tar => list_tar(file, &limits, deadline),
        ArchiveKind::TarGz => list_tar(gzip_reader(file), &limits, deadline),
    }
}

/// Every member of the gzip stream, as `gzip -d` reads them; bytes after the
/// last member are refused. No limit here: the ratio is checked over the whole
/// stream by the extractor.
fn gzip_reader(file: ArchiveReader) -> DecoderReader<ArchiveReader> {
    DecoderReader::new(file, Some(Format::Gzip), crate::codec::UNLIMITED)
}

fn list_tar<R: Read>(
    reader: R,
    limits: &Limits,
    deadline: Deadline,
) -> ArchiveResult<Vec<EntryInfo>> {
    let mut entries = Vec::new();
    for_each_member(reader, limits.entries, |member| {
        deadline.check()?;
        if entries.len() as u64 >= limits.entries {
            return Err(ArchiveFailure::new(ERR_ENTRY_LIMIT));
        }
        entries.push(EntryInfo {
            name: String::from_utf8_lossy(&member.name).into_owned(),
            kind: member.kind,
            size: member.size,
            compressed_size: None,
        });
        Ok(())
    })?;
    Ok(entries)
}

pub(crate) fn unpack(
    archive: &str,
    destination: &str,
    options: &UnpackOptions,
) -> ArchiveResult<Summary> {
    let deadline = Deadline::from_timeout(options.timeout_ms)?;
    unpack_until(archive, destination, options, deadline)
}

pub(super) fn unpack_until(
    archive: &str,
    destination: &str,
    options: &UnpackOptions,
    deadline: Deadline,
) -> ArchiveResult<Summary> {
    let limits = limits(options)?;
    let (file, kind) = open_archive(archive, deadline)?;
    let (root, created_root) = open_destination(destination)?;
    let mut extractor = Extractor {
        root,
        destination: PathBuf::from(destination),
        overwrite: options.overwrite,
        strip: options.strip_components,
        limits,
        deadline,
        compressed_stream: (kind == ArchiveKind::TarGz).then(|| file.consumed()),
        seen: HashMap::new(),
        created: Vec::new(),
        summary: Summary::default(),
        visited: 0,
    };
    let result = match kind {
        ArchiveKind::Zip => extract_zip(&mut extractor, file),
        ArchiveKind::Tar => extract_tar(&mut extractor, file),
        ArchiveKind::TarGz => extract_tar(&mut extractor, gzip_reader(file)),
    };
    finish_extraction(extractor, result, created_root, destination)
}

/// Keeps what a successful extraction wrote; after a failure, removes what
/// this call created, including the destination directory itself.
fn finish_extraction(
    mut extractor: Extractor,
    result: ArchiveResult<()>,
    created_root: bool,
    destination: &str,
) -> ArchiveResult<Summary> {
    match result {
        Ok(()) => Ok(extractor.summary),
        Err(failure) => {
            extractor.rollback();
            drop(extractor);
            if created_root {
                let _ = std::fs::remove_dir(destination);
            }
            Err(failure)
        }
    }
}

/// Extracts a TAR or TAR.GZ read from `reader`, which cannot seek: the
/// content of a stream. `consumed` counts the compressed bytes read so far,
/// for the ratio over the whole stream. ZIP needs its central directory, at
/// the end, before any entry can be trusted, so it is refused here.
pub(super) fn unpack_tar_from<R: Read>(
    reader: R,
    kind: ArchiveKind,
    consumed: Rc<Cell<u64>>,
    destination: &str,
    options: &UnpackOptions,
    deadline: Deadline,
) -> ArchiveResult<Summary> {
    let limits = limits(options)?;
    if kind == ArchiveKind::Zip {
        return Err(ArchiveFailure::new(super::ERR_ZIP_NOT_STREAMABLE));
    }
    let (root, created_root) = open_destination(destination)?;
    let mut extractor = Extractor {
        root,
        destination: PathBuf::from(destination),
        overwrite: options.overwrite,
        strip: options.strip_components,
        limits,
        deadline,
        compressed_stream: (kind == ArchiveKind::TarGz).then_some(consumed),
        seen: HashMap::new(),
        created: Vec::new(),
        summary: Summary::default(),
        visited: 0,
    };
    let result = match kind {
        ArchiveKind::TarGz => extract_tar(
            &mut extractor,
            DecoderReader::new(reader, Some(Format::Gzip), crate::codec::UNLIMITED),
        ),
        _ => extract_tar(&mut extractor, reader),
    };
    finish_extraction(extractor, result, created_root, destination)
}

fn open_destination(destination: &str) -> ArchiveResult<(Dir, bool)> {
    let path = Path::new(destination);
    let created = match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => false,
        Ok(_) => return Err(ArchiveFailure::new(ERR_DESTINATION_EXISTS)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir(path).map_err(|error| io_failure(&error))?;
            true
        }
        Err(error) => return Err(io_failure(&error)),
    };
    match Dir::open_ambient_dir(path, ambient_authority()) {
        Ok(root) => Ok((root, created)),
        Err(error) => {
            if created {
                let _ = std::fs::remove_dir(path);
            }
            Err(io_failure(&error))
        }
    }
}

fn extract_zip(extractor: &mut Extractor, file: ArchiveReader) -> ArchiveResult<()> {
    let mut zip = open_zip(file, &extractor.limits)?;
    reject_overlapping_entries(&mut zip)?;
    extractor.check_free_space(declared_total(&mut zip)?)?;
    for index in 0..zip.len() {
        let (name, kind, compressed, executable) = {
            let raw = zip
                .by_index_raw(index)
                .map_err(|error| zip_failure(&error, None))?;
            let name = raw.name().to_owned();
            if raw.encrypted() {
                return Err(ArchiveFailure::at(ERR_ENCRYPTED, &name));
            }
            let executable = raw.unix_mode().is_some_and(|mode| mode & 0o100 != 0);
            (
                name,
                zip_entry_kind(&raw),
                raw.compressed_size(),
                executable,
            )
        };
        let Some(target) = extractor.begin_entry(&name, kind)? else {
            continue;
        };
        if kind == EntryKind::Directory {
            extractor.make_directory(&target, &name)?;
            continue;
        }
        let (declared_size, crc) = {
            let raw = zip
                .by_index_raw(index)
                .map_err(|error| zip_failure(&error, Some(&name)))?;
            (raw.size(), raw.crc32())
        };
        extractor.check_declared_size(declared_size, &name)?;
        let mut reader = zip
            .by_index(index)
            .map_err(|error| zip_failure(&error, Some(&name)))?;
        let source = EntrySource {
            reader: &mut reader,
            declared_size,
            compressed: Some(compressed),
            crc: Some(crc),
            executable,
        };
        extractor.write_file(&target, &name, source)?;
    }
    Ok(())
}

/// Two central-directory entries whose data ranges overlap are the
/// non-recursive zip bomb: a few kilobytes of deflate data referenced by
/// thousands of entries. No honest writer produces overlap.
fn reject_overlapping_entries(zip: &mut ZipReader) -> ArchiveResult<()> {
    let mut ranges = Vec::with_capacity(zip.len());
    for index in 0..zip.len() {
        let file = zip
            .by_index_raw(index)
            .map_err(|error| zip_failure(&error, None))?;
        let start = file.header_start();
        let data_start = file
            .data_start()
            .ok_or_else(|| ArchiveFailure::at(ERR_INVALID_ARCHIVE, file.name()))?;
        let end = data_start
            .checked_add(file.compressed_size())
            .ok_or_else(|| ArchiveFailure::at(ERR_INVALID_ARCHIVE, file.name()))?;
        ranges.push((start, end, index));
    }
    ranges.sort_unstable();
    for pair in ranges.windows(2) {
        if pair[1].0 < pair[0].1 {
            let name = zip
                .by_index_raw(pair[1].2)
                .map(|file| file.name().to_owned())
                .unwrap_or_default();
            return Err(ArchiveFailure::at(ERR_INVALID_ARCHIVE, &name));
        }
    }
    Ok(())
}

/// The sum of what the central directory says the files will occupy once
/// extracted. It is only an early answer to "does this fit": the sizes can
/// lie, so the real guards stay `maxBytes`, counted on written bytes, and
/// `ENOSPC` during the write.
fn declared_total(zip: &mut ZipReader) -> ArchiveResult<u64> {
    let mut total: u64 = 0;
    for index in 0..zip.len() {
        let file = zip
            .by_index_raw(index)
            .map_err(|error| zip_failure(&error, None))?;
        if !file.is_dir() {
            total = total.saturating_add(file.size());
        }
    }
    Ok(total)
}

fn extract_tar<R: Read>(extractor: &mut Extractor, reader: R) -> ArchiveResult<()> {
    let max_entries = extractor.limits.entries;
    for_each_member(reader, max_entries, |member| {
        let name = match String::from_utf8(member.name) {
            Ok(name) => name,
            Err(error) => {
                let lossy = String::from_utf8_lossy(error.as_bytes()).into_owned();
                return Err(ArchiveFailure::at(ERR_INVALID_ENTRY_NAME, &lossy));
            }
        };
        let Some(target) = extractor.begin_entry(&name, member.kind)? else {
            return Ok(());
        };
        if member.kind == EntryKind::Directory {
            return extractor.make_directory(&target, &name);
        }
        extractor.check_declared_size(member.size, &name)?;
        let source = EntrySource {
            reader: member.data,
            declared_size: member.size,
            compressed: None,
            crc: None,
            executable: member.executable,
        };
        extractor.write_file(&target, &name, source)
    })
}

struct EntrySource<'a> {
    reader: &'a mut dyn Read,
    declared_size: u64,
    compressed: Option<u64>,
    crc: Option<u32>,
    executable: bool,
}

struct Extractor {
    root: Dir,
    destination: PathBuf,
    overwrite: bool,
    strip: u32,
    limits: Limits,
    deadline: Deadline,
    /// Compressed bytes read so far from a gzip stream, whose members carry
    /// no compressed size of their own; the ratio is checked over the whole
    /// stream instead of per entry.
    compressed_stream: Option<Rc<Cell<u64>>>,
    /// Collision keys (case-folded, NFC-normalized paths) of everything this
    /// call has extracted, mapped to whether the entry is a directory.
    seen: HashMap<String, bool>,
    /// Paths this call created, in creation order, for rollback.
    created: Vec<(PathBuf, bool)>,
    summary: Summary,
    visited: u64,
}

impl Extractor {
    fn begin_entry(&mut self, name: &str, kind: EntryKind) -> ArchiveResult<Option<Vec<String>>> {
        self.deadline.check_at(name)?;
        self.visited += 1;
        if self.visited > self.limits.entries {
            return Err(ArchiveFailure::new(ERR_ENTRY_LIMIT));
        }
        match kind {
            EntryKind::File | EntryKind::Directory => {}
            EntryKind::Symlink | EntryKind::HardLink => {
                return Err(ArchiveFailure::at(ERR_UNSAFE_LINK, name));
            }
            EntryKind::Other => return Err(ArchiveFailure::at(ERR_UNSUPPORTED_ENTRY, name)),
        }
        let components = match normalize_entry_name(name, self.strip) {
            Ok(Some(components)) => components,
            Ok(None) => return Ok(None),
            Err(EntryNameError::Escape) => return Err(ArchiveFailure::at(ERR_PATH_ESCAPE, name)),
            Err(EntryNameError::Invalid) => {
                return Err(ArchiveFailure::at(ERR_INVALID_ENTRY_NAME, name));
            }
        };
        let is_dir = kind == EntryKind::Directory;
        let mut key = String::new();
        for (index, component) in components.iter().enumerate() {
            if index > 0 {
                key.push('/');
            }
            key.extend(component.nfc().flat_map(char::to_lowercase));
            let last = index + 1 == components.len();
            match self.seen.get(&key) {
                Some(false) => return Err(ArchiveFailure::at(ERR_DUPLICATE_ENTRY, name)),
                Some(true) if last && !is_dir => {
                    return Err(ArchiveFailure::at(ERR_DUPLICATE_ENTRY, name));
                }
                Some(true) => {}
                None => {
                    self.seen.insert(key.clone(), !last || is_dir);
                }
            }
        }
        Ok(Some(components.into_iter().map(str::to_owned).collect()))
    }

    fn check_declared_size(&self, declared: u64, name: &str) -> ArchiveResult<()> {
        if self.summary.bytes.saturating_add(declared) > self.limits.bytes {
            return Err(ArchiveFailure::at(ERR_SIZE_LIMIT, name));
        }
        Ok(())
    }

    /// Refuses up front an archive whose declared contents, capped at
    /// `maxBytes`, exceed the space this process may still write in the
    /// destination, before a single file is created.
    fn check_free_space(&self, declared: u64) -> ArchiveResult<()> {
        let needed = declared.min(self.limits.bytes);
        match available_space(&self.root, &self.destination) {
            Some(available) if needed > available => Err(ArchiveFailure::new(ERR_NO_SPACE)),
            _ => Ok(()),
        }
    }

    /// Opens (creating as needed) the directory at `components` below the
    /// root, one component at a time, refusing to pass through a symbolic link
    /// even when it already existed in the destination.
    fn directory(&mut self, components: &[String], name: &str) -> ArchiveResult<Dir> {
        let mut current = self
            .root
            .try_clone()
            .map_err(|error| io_failure_at(&error, name))?;
        let mut relative = PathBuf::new();
        for component in components {
            relative.push(component);
            match current.symlink_metadata(component) {
                Ok(metadata) if metadata.is_symlink() => {
                    return Err(ArchiveFailure::at(ERR_PATH_ESCAPE, name));
                }
                Ok(metadata) if metadata.is_dir() => {}
                Ok(_) => return Err(ArchiveFailure::at(ERR_DESTINATION_EXISTS, name)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    current
                        .create_dir(component)
                        .map_err(|error| io_failure_at(&error, name))?;
                    self.created.push((relative.clone(), true));
                }
                Err(error) => return Err(io_failure_at(&error, name)),
            }
            current = current
                .open_dir(component)
                .map_err(|error| io_failure_at(&error, name))?;
        }
        Ok(current)
    }

    fn make_directory(&mut self, components: &[String], name: &str) -> ArchiveResult<()> {
        self.directory(components, name)?;
        self.summary.entries += 1;
        Ok(())
    }

    fn write_file(
        &mut self,
        components: &[String],
        name: &str,
        source: EntrySource<'_>,
    ) -> ArchiveResult<()> {
        let (file_name, parents) = components
            .split_last()
            .ok_or_else(|| ArchiveFailure::at(ERR_INVALID_ENTRY_NAME, name))?;
        let parent = self.directory(parents, name)?;
        let replaces = match parent.symlink_metadata(file_name) {
            Ok(metadata) if metadata.is_file() && self.overwrite => true,
            Ok(_) => return Err(ArchiveFailure::at(ERR_DESTINATION_EXISTS, name)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(io_failure_at(&error, name)),
        };
        let temp_name = format!(
            ".tyrus-{}-{}.tmp",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let file = create_file(&parent, &temp_name, source.executable)
            .map_err(|error| io_failure_at(&error, name))?;
        let copied = self.copy_entry(file, name, source);
        let result = copied.and_then(|written| {
            parent
                .rename(&temp_name, &parent, file_name)
                .map_err(|error| io_failure_at(&error, name))?;
            Ok(written)
        });
        match result {
            Ok(written) => {
                if !replaces {
                    let mut relative: PathBuf = parents.iter().collect();
                    relative.push(file_name);
                    self.created.push((relative, false));
                }
                self.summary.bytes += written;
                self.summary.entries += 1;
                Ok(())
            }
            Err(failure) => {
                let _ = parent.remove_file(&temp_name);
                Err(failure)
            }
        }
    }

    fn copy_entry(&self, file: File, name: &str, source: EntrySource<'_>) -> ArchiveResult<u64> {
        let mut writer = std::io::BufWriter::with_capacity(COPY_BUFFER, DiskWriter::new(file));
        let mut buffer = vec![0u8; COPY_BUFFER];
        let mut hasher = source.crc.map(|_| crc32fast::Hasher::new());
        let mut written: u64 = 0;
        loop {
            let count = match source.reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => count,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    let checksum_failed = matches!(
                        (&hasher, source.crc),
                        (Some(hasher), Some(expected)) if hasher.clone().finalize() != expected
                    ) && error.kind() == std::io::ErrorKind::InvalidData;
                    return Err(if checksum_failed {
                        ArchiveFailure::at(ERR_CHECKSUM_MISMATCH, name)
                    } else {
                        io_or_invalid(&error, Some(name))
                    });
                }
            };
            self.deadline.check_at(name)?;
            written += count as u64;
            let total = self.summary.bytes.saturating_add(written);
            if total > self.limits.bytes {
                return Err(ArchiveFailure::at(ERR_SIZE_LIMIT, name));
            }
            let over_ratio = match (source.compressed, &self.compressed_stream) {
                (Some(compressed), _) => exceeds_ratio(written, compressed, self.limits.ratio),
                (None, Some(consumed)) => exceeds_ratio(total, consumed.get(), self.limits.ratio),
                (None, None) => false,
            };
            if over_ratio {
                return Err(ArchiveFailure::at(ERR_RATIO_LIMIT, name));
            }
            if let Some(hasher) = hasher.as_mut() {
                hasher.update(&buffer[..count]);
            }
            writer
                .write_all(&buffer[..count])
                .map_err(|error| io_failure_at(&error, name))?;
        }
        if written != source.declared_size {
            return Err(ArchiveFailure::at(ERR_INVALID_ARCHIVE, name));
        }
        if let (Some(hasher), Some(expected)) = (hasher, source.crc)
            && hasher.finalize() != expected
        {
            return Err(ArchiveFailure::at(ERR_CHECKSUM_MISMATCH, name));
        }
        writer
            .flush()
            .map_err(|error| io_failure_at(&error, name))?;
        Ok(written)
    }

    /// Removes what this call created, newest first, so directories are
    /// empty by the time they are reached. Anything that already existed in
    /// the destination is left alone.
    fn rollback(&mut self) {
        for (path, is_dir) in self.created.drain(..).rev() {
            let _ = if is_dir {
                self.root.remove_dir(&path)
            } else {
                self.root.remove_file(&path)
            };
        }
    }
}

/// Bytes an unprivileged process can still write on the destination's file
/// system (`f_bavail`, which leaves out the blocks reserved for root). `None`
/// when the platform or the file system cannot tell, and then the up-front
/// check is skipped.
#[cfg(unix)]
pub(super) fn available_space(root: &Dir, _destination: &Path) -> Option<u64> {
    use std::os::fd::AsRawFd;
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
    // SAFETY: `root` owns an open directory descriptor for the whole call and
    // `stats` is a valid, writable `statvfs`.
    if unsafe { libc::fstatvfs(root.as_raw_fd(), &mut stats) } != 0 {
        return None;
    }
    #[allow(clippy::useless_conversion)]
    let (blocks, size) = (u64::from(stats.f_bavail), u64::from(stats.f_frsize));
    Some(blocks.saturating_mul(size))
}

#[cfg(windows)]
pub(super) fn available_space(_root: &Dir, destination: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;

    let path = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut available = 0_u64;
    let succeeded = unsafe {
        windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
            path.as_ptr(),
            &mut available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    } != 0;
    succeeded.then_some(available)
}

#[cfg(not(any(unix, windows)))]
pub(super) fn available_space(_root: &Dir, _destination: &Path) -> Option<u64> {
    None
}

#[cfg(unix)]
fn create_file(parent: &Dir, name: &str, executable: bool) -> std::io::Result<File> {
    use cap_std::fs::OpenOptionsExt;
    let mut options = cap_std::fs::OpenOptions::new();
    options
        .write(true)
        .create_new(true)
        .mode(if executable { 0o755 } else { 0o644 });
    parent
        .open_with(name, &options)
        .map(cap_std::fs::File::into_std)
}

#[cfg(not(unix))]
fn create_file(parent: &Dir, name: &str, _executable: bool) -> std::io::Result<File> {
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    parent
        .open_with(name, &options)
        .map(cap_std::fs::File::into_std)
}
