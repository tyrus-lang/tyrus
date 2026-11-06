use super::entry_name::{EntryNameError, normalize_entry_name};
use super::format::kind_from_destination;
use super::{
    ArchiveFailure, ArchiveKind, ArchiveResult, Deadline, DiskWriter, ERR_DESTINATION_EXISTS,
    ERR_INVALID_COMPRESSION_LEVEL, ERR_INVALID_ENTRY_NAME, ERR_INVALID_OPTION, ERR_PATH_ESCAPE,
    ERR_UNSUPPORTED_ENTRY, GuardedReader, PackOptions, Summary, io_failure, io_failure_at,
};
use crate::codec::{EncoderWriter, Format};
use std::fs::{self, File, Metadata};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

const WRITE_BUFFER: usize = 64 * 1024;
const DEFAULT_LEVEL: i32 = 6;
const REPRODUCIBLE_FILE_MODE: u32 = 0o644;
const REPRODUCIBLE_EXEC_MODE: u32 = 0o755;
const PERMISSION_BITS: u32 = 0o777;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// One file or directory `pack` writes, already named as it will appear in
/// the archive.
struct PackEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    size: u64,
    mode: u32,
    modified: Option<SystemTime>,
}

pub(crate) fn pack(
    source: &str,
    destination: &str,
    options: &PackOptions,
) -> ArchiveResult<Summary> {
    let deadline = Deadline::from_timeout(options.timeout_ms)?;
    pack_until(source, destination, options, deadline)
}

pub(super) fn pack_until(
    source: &str,
    destination: &str,
    options: &PackOptions,
    deadline: Deadline,
) -> ArchiveResult<Summary> {
    let kind = kind_from_destination(destination)?;
    let level = validate_level(kind, options.level)?;
    let prefix = validate_prefix(options.prefix.as_deref())?;
    let destination_path = Path::new(destination);
    let destination_parent = parent_of(destination_path);
    let destination_name = destination_path
        .file_name()
        .ok_or_else(|| ArchiveFailure::new(ERR_INVALID_OPTION))?
        .to_owned();
    let canonical_parent =
        fs::canonicalize(destination_parent).map_err(|error| io_failure(&error))?;
    match fs::symlink_metadata(destination_path) {
        Ok(metadata) if metadata.is_file() && options.overwrite => {}
        Ok(_) => return Err(ArchiveFailure::new(ERR_DESTINATION_EXISTS)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_failure(&error)),
    }
    let temp_name = format!(
        ".{}.tyrus-{}-{}.tmp",
        destination_name.to_string_lossy(),
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let temp_path = canonical_parent.join(&temp_name);
    let skipped = [canonical_parent.join(&destination_name), temp_path.clone()];
    let walk = Walk {
        prefix: prefix.as_deref(),
        options,
        skipped: &skipped,
        deadline,
    };
    let entries = collect_entries(source, &walk)?;
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .map_err(|error| io_failure(&error))?;
    let settings = WriteSettings {
        level,
        reproducible: options.reproducible,
        deadline,
    };
    let written = write_archive(kind, &settings, file, &entries);
    let result = written.and_then(|summary| {
        fs::rename(&temp_path, destination_path).map_err(|error| io_failure(&error))?;
        Ok(summary)
    });
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn parent_of(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

fn validate_level(kind: ArchiveKind, level: Option<i32>) -> ArchiveResult<u32> {
    match (kind, level) {
        (ArchiveKind::Tar, None) => Ok(0),
        (ArchiveKind::Tar, Some(_)) => Err(ArchiveFailure::new(ERR_INVALID_COMPRESSION_LEVEL)),
        (_, None) => Ok(DEFAULT_LEVEL as u32),
        (_, Some(level)) => u32::try_from(level)
            .ok()
            .filter(|level| *level <= 9)
            .ok_or_else(|| ArchiveFailure::new(ERR_INVALID_COMPRESSION_LEVEL)),
    }
}

fn validate_prefix(prefix: Option<&str>) -> ArchiveResult<Option<String>> {
    let Some(prefix) = prefix else {
        return Ok(None);
    };
    match normalize_entry_name(prefix, 0) {
        Ok(Some(components)) => Ok(Some(components.join("/"))),
        Ok(None) | Err(_) => Err(ArchiveFailure::new(ERR_INVALID_OPTION)),
    }
}

/// What the source walk needs at every level of the tree.
struct Walk<'a> {
    prefix: Option<&'a str>,
    options: &'a PackOptions,
    skipped: &'a [PathBuf],
    deadline: Deadline,
}

struct WriteSettings {
    level: u32,
    reproducible: bool,
    deadline: Deadline,
}

fn collect_entries(source: &str, walk: &Walk<'_>) -> ArchiveResult<Vec<PackEntry>> {
    let (prefix, options) = (walk.prefix, walk.options);
    let source_path = Path::new(source);
    let metadata = fs::symlink_metadata(source_path).map_err(|error| io_failure(&error))?;
    if metadata.file_type().is_symlink() {
        return Err(ArchiveFailure::at(ERR_UNSUPPORTED_ENTRY, source));
    }
    let root = fs::canonicalize(source_path).map_err(|error| io_failure(&error))?;
    let mut entries = Vec::new();
    if metadata.is_file() {
        let name = root
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| ArchiveFailure::new(ERR_INVALID_ENTRY_NAME))?
            .to_owned();
        if !is_excluded(&options.exclude, &name) {
            entries.push(pack_entry(prefixed(prefix, &name), root, &metadata)?);
        }
    } else if metadata.is_dir() {
        if let Some(prefix) = prefix {
            entries.push(pack_entry(prefix.to_owned(), root.clone(), &metadata)?);
        }
        walk_directory(&root, "", walk, &mut entries)?;
    } else {
        return Err(ArchiveFailure::at(ERR_UNSUPPORTED_ENTRY, source));
    }
    Ok(entries)
}

fn walk_directory(
    directory: &Path,
    relative: &str,
    walk: &Walk<'_>,
    entries: &mut Vec<PackEntry>,
) -> ArchiveResult<()> {
    let (prefix, options) = (walk.prefix, walk.options);
    let listing = fs::read_dir(directory).map_err(|error| io_failure_at(&error, relative))?;
    let mut children = Vec::new();
    for child in listing {
        let child = child.map_err(|error| io_failure_at(&error, relative))?;
        children.push(child.path());
    }
    children.sort();
    for path in children {
        walk.deadline.check_at(relative)?;
        if walk.skipped.contains(&path) {
            continue;
        }
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
        let child_relative = if relative.is_empty() {
            file_name.to_string()
        } else {
            format!("{relative}/{file_name}")
        };
        if path.file_name().and_then(|name| name.to_str()).is_none() {
            return Err(ArchiveFailure::at(ERR_INVALID_ENTRY_NAME, &child_relative));
        }
        if is_excluded(&options.exclude, &child_relative) {
            continue;
        }
        let metadata =
            fs::symlink_metadata(&path).map_err(|error| io_failure_at(&error, &child_relative))?;
        let entry_name = prefixed(prefix, &child_relative);
        if metadata.is_dir() {
            entries.push(pack_entry(entry_name, path.clone(), &metadata)?);
            walk_directory(&path, &child_relative, walk, entries)?;
        } else if metadata.is_file() {
            entries.push(pack_entry(entry_name, path, &metadata)?);
        } else {
            return Err(ArchiveFailure::at(ERR_UNSUPPORTED_ENTRY, &child_relative));
        }
    }
    Ok(())
}

fn is_excluded(patterns: &[String], relative: &str) -> bool {
    patterns
        .iter()
        .any(|pattern| crate::fs::glob_path_match(pattern, relative))
}

fn prefixed(prefix: Option<&str>, name: &str) -> String {
    match prefix {
        Some(prefix) => format!("{prefix}/{name}"),
        None => name.to_owned(),
    }
}

/// Every name `pack` writes goes through the same validator `unpack` applies,
/// so an archive the program creates is always one it can extract, on every
/// platform.
fn pack_entry(name: String, path: PathBuf, metadata: &Metadata) -> ArchiveResult<PackEntry> {
    match normalize_entry_name(&name, 0) {
        Ok(Some(_)) => {}
        Err(EntryNameError::Escape) => return Err(ArchiveFailure::at(ERR_PATH_ESCAPE, &name)),
        Ok(None) | Err(EntryNameError::Invalid) => {
            return Err(ArchiveFailure::at(ERR_INVALID_ENTRY_NAME, &name));
        }
    }
    Ok(PackEntry {
        name,
        path,
        is_dir: metadata.is_dir(),
        size: if metadata.is_file() {
            metadata.len()
        } else {
            0
        },
        mode: permission_bits(metadata),
        modified: metadata.modified().ok(),
    })
}

#[cfg(unix)]
fn permission_bits(metadata: &Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & PERMISSION_BITS
}

#[cfg(not(unix))]
fn permission_bits(metadata: &Metadata) -> u32 {
    if metadata.is_dir() {
        REPRODUCIBLE_EXEC_MODE
    } else if metadata.permissions().readonly() {
        0o444
    } else {
        REPRODUCIBLE_FILE_MODE
    }
}

fn entry_mode(entry: &PackEntry, reproducible: bool) -> u32 {
    if !reproducible {
        return entry.mode;
    }
    if entry.is_dir || entry.mode & 0o100 != 0 {
        REPRODUCIBLE_EXEC_MODE
    } else {
        REPRODUCIBLE_FILE_MODE
    }
}

fn write_archive(
    kind: ArchiveKind,
    settings: &WriteSettings,
    file: File,
    entries: &[PackEntry],
) -> ArchiveResult<Summary> {
    let writer = BufWriter::with_capacity(WRITE_BUFFER, DiskWriter::new(file));
    let (summary, file) = match kind {
        ArchiveKind::Zip => write_zip(writer, settings, entries)?,
        ArchiveKind::Tar => {
            let (summary, writer) = write_tar(writer, settings, entries)?;
            (summary, into_file(writer)?)
        }
        ArchiveKind::TarGz => {
            let encoder = gzip_writer(writer, settings.level)?;
            let (summary, encoder) = write_tar(encoder, settings, entries)?;
            let writer = encoder.finish().map_err(|error| io_failure(&error))?;
            (summary, into_file(writer)?)
        }
    };
    file.sync_all().map_err(|error| io_failure(&error))?;
    Ok(summary)
}

type ArchiveWriter = BufWriter<DiskWriter<File>>;

fn gzip_writer<W: Write>(writer: W, level: u32) -> ArchiveResult<EncoderWriter<W>> {
    let level =
        i32::try_from(level).map_err(|_| ArchiveFailure::new(ERR_INVALID_COMPRESSION_LEVEL))?;
    EncoderWriter::new(writer, Format::Gzip, Some(level))
        .map_err(|_| ArchiveFailure::new(ERR_INVALID_COMPRESSION_LEVEL))
}

fn into_file(writer: ArchiveWriter) -> ArchiveResult<File> {
    writer
        .into_inner()
        .map(DiskWriter::into_inner)
        .map_err(|error| io_failure(error.error()))
}

fn open_source(entry: &PackEntry, deadline: Deadline) -> ArchiveResult<GuardedReader<File>> {
    File::open(&entry.path)
        .map(|file| GuardedReader::new(file, deadline))
        .map_err(|error| io_failure_at(&error, &entry.name))
}

/// A write error inside the zip writer still carries the `io::Error` that
/// caused it, so a full disk reports `ERR_ARCHIVE_NO_SPACE`, not a bare I/O
/// failure.
fn zip_write_failure(error: &zip::result::ZipError, entry: Option<&str>) -> ArchiveFailure {
    match (error, entry) {
        (zip::result::ZipError::Io(error), Some(entry)) => io_failure_at(error, entry),
        (zip::result::ZipError::Io(error), None) => io_failure(error),
        (_, Some(entry)) => ArchiveFailure::at(super::ERR_IO, entry),
        (_, None) => ArchiveFailure::new(super::ERR_IO),
    }
}

fn write_zip(
    writer: ArchiveWriter,
    settings: &WriteSettings,
    entries: &[PackEntry],
) -> ArchiveResult<(Summary, File)> {
    let mut zip = zip::ZipWriter::new(writer);
    let summary = write_zip_entries(&mut zip, settings, entries, false)?;
    let writer = zip
        .finish()
        .map_err(|error| zip_write_failure(&error, None))?;
    Ok((summary, into_file(writer)?))
}

fn write_zip_entries<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    settings: &WriteSettings,
    entries: &[PackEntry],
    streaming: bool,
) -> ArchiveResult<Summary> {
    let (level, reproducible) = (settings.level, settings.reproducible);
    let mut summary = Summary::default();
    for entry in entries {
        settings.deadline.check_at(&entry.name)?;
        let method = if level == 0 {
            zip::CompressionMethod::Stored
        } else {
            zip::CompressionMethod::Deflated
        };
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(method)
            .compression_level((level > 0).then_some(i64::from(level)))
            .unix_permissions(entry_mode(entry, reproducible))
            .last_modified_time(zip_timestamp(entry.modified, reproducible))
            .large_file(entry.size >= u64::from(u32::MAX));
        let failed = |error| zip_write_failure(&error, Some(&entry.name));
        if entry.is_dir && streaming {
            // In stream mode `add_directory` sets the data-descriptor flag and
            // writes no descriptor, which `unzip` rejects as overlapping
            // entries; an empty entry named with a `/` is the same directory.
            zip.start_file(format!("{}/", entry.name), options)
                .map_err(failed)?;
        } else if entry.is_dir {
            zip.add_directory(format!("{}/", entry.name), options)
                .map_err(failed)?;
        } else {
            zip.start_file(entry.name.as_str(), options)
                .map_err(failed)?;
            let mut source = open_source(entry, settings.deadline)?;
            let copied = std::io::copy(&mut source, zip)
                .map_err(|error| io_failure_at(&error, &entry.name))?;
            summary.bytes += copied;
        }
        summary.entries += 1;
    }
    Ok(summary)
}

/// Writes the archive of `source` into `writer` as it walks the tree, with no
/// destination file: the bytes go wherever `writer` takes them. ZIP is written
/// with data descriptors, since a stream cannot go back to patch sizes in.
pub(super) fn pack_into<W: Write>(
    kind: ArchiveKind,
    source: &str,
    options: &PackOptions,
    deadline: Deadline,
    writer: W,
) -> ArchiveResult<(Summary, W)> {
    let level = validate_level(kind, options.level)?;
    let prefix = validate_prefix(options.prefix.as_deref())?;
    let walk = Walk {
        prefix: prefix.as_deref(),
        options,
        skipped: &[],
        deadline,
    };
    let entries = collect_entries(source, &walk)?;
    let settings = WriteSettings {
        level,
        reproducible: options.reproducible,
        deadline,
    };
    match kind {
        ArchiveKind::Zip => {
            let mut zip = zip::ZipWriter::new_stream(writer);
            let summary = write_zip_entries(&mut zip, &settings, &entries, true)?;
            let writer = zip
                .finish()
                .map_err(|error| zip_write_failure(&error, None))?;
            Ok((summary, writer.into_inner()))
        }
        ArchiveKind::Tar => write_tar(writer, &settings, &entries),
        ArchiveKind::TarGz => {
            let encoder = gzip_writer(writer, settings.level)?;
            let (summary, encoder) = write_tar(encoder, &settings, &entries)?;
            let writer = encoder.finish().map_err(|error| io_failure(&error))?;
            Ok((summary, writer))
        }
    }
}

/// ZIP stores local civil time with no zone and nothing before 1980. UTC is
/// the only choice that makes the same file produce the same archive on every
/// machine, and the 1980 floor doubles as the reproducible timestamp.
fn zip_timestamp(modified: Option<SystemTime>, reproducible: bool) -> zip::DateTime {
    let fallback = zip::DateTime::default();
    if reproducible {
        return fallback;
    }
    let Some(modified) = modified else {
        return fallback;
    };
    let Ok(timestamp) = jiff::Timestamp::try_from(modified) else {
        return fallback;
    };
    let civil = timestamp.to_zoned(jiff::tz::TimeZone::UTC).datetime();
    let (Ok(year), Ok(month), Ok(day), Ok(hour), Ok(minute), Ok(second)) = (
        u16::try_from(civil.year()),
        u8::try_from(civil.month()),
        u8::try_from(civil.day()),
        u8::try_from(civil.hour()),
        u8::try_from(civil.minute()),
        u8::try_from(civil.second()),
    ) else {
        return fallback;
    };
    zip::DateTime::from_date_and_time(year, month, day, hour, minute, second).unwrap_or(fallback)
}

fn write_tar<W: Write>(
    writer: W,
    settings: &WriteSettings,
    entries: &[PackEntry],
) -> ArchiveResult<(Summary, W)> {
    let reproducible = settings.reproducible;
    let mut builder = tar::Builder::new(writer);
    let mut summary = Summary::default();
    for entry in entries {
        settings.deadline.check_at(&entry.name)?;
        let mut header = tar::Header::new_gnu();
        header.set_mode(entry_mode(entry, reproducible));
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(tar_mtime(entry.modified, reproducible));
        let failed = |error: std::io::Error| io_failure_at(&error, &entry.name);
        if entry.is_dir {
            header.set_entry_type(tar::EntryType::Directory);
            header.set_size(0);
            builder
                .append_data(&mut header, format!("{}/", entry.name), std::io::empty())
                .map_err(failed)?;
        } else {
            header.set_entry_type(tar::EntryType::Regular);
            header.set_size(entry.size);
            let source = open_source(entry, settings.deadline)?;
            let mut exact = ExactReader {
                inner: std::io::Read::take(source, entry.size),
                remaining: entry.size,
            };
            builder
                .append_data(&mut header, &entry.name, &mut exact)
                .map_err(failed)?;
            summary.bytes += entry.size;
        }
        summary.entries += 1;
    }
    let writer = builder.into_inner().map_err(|error| io_failure(&error))?;
    Ok((summary, writer))
}

/// A tar header states the entry size before the data, so a file that shrinks
/// between `stat` and the copy would leave a corrupt archive. This reader turns
/// that early end into an error instead.
struct ExactReader<R> {
    inner: R,
    remaining: u64,
}

impl<R: std::io::Read> std::io::Read for ExactReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let count = self.inner.read(buffer)?;
        if count == 0 && self.remaining > 0 && !buffer.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "file changed while it was archived",
            ));
        }
        self.remaining -= count as u64;
        Ok(count)
    }
}

fn tar_mtime(modified: Option<SystemTime>, reproducible: bool) -> u64 {
    if reproducible {
        return 0;
    }
    modified
        .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_secs())
}
