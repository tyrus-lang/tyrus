use super::{
    ArchiveFailure, ArchiveKind, ArchiveResult, ERR_INVALID_ARCHIVE, ERR_UNSUPPORTED_EXTENSION,
    ERR_UNSUPPORTED_FORMAT,
};
use std::io::{Read, Seek, SeekFrom};

pub(super) const TAR_BLOCK: usize = 512;
const TAR_MAGIC_OFFSET: usize = 257;
pub(super) const ZIP_LOCAL_HEADER: &[u8] = b"PK\x03\x04";
pub(super) const ZIP_EMPTY_ARCHIVE: &[u8] = b"PK\x05\x06";
pub(super) const GZIP_MAGIC: &[u8] = &[0x1f, 0x8b];

/// The archive format `pack` writes, decided by the destination's extension.
/// The extension must be lowercase, so `backup.ZIP` is rejected like any
/// unknown extension rather than guessed at.
pub(crate) fn kind_from_destination(destination: &str) -> ArchiveResult<ArchiveKind> {
    let name = destination
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(destination);
    let kind = if has_stem(name, ".tar.gz") || has_stem(name, ".tgz") {
        ArchiveKind::TarGz
    } else if has_stem(name, ".tar") {
        ArchiveKind::Tar
    } else if has_stem(name, ".zip") {
        ArchiveKind::Zip
    } else {
        return Err(ArchiveFailure::new(ERR_UNSUPPORTED_EXTENSION));
    };
    Ok(kind)
}

fn has_stem(name: &str, extension: &str) -> bool {
    name.len() > extension.len() && name.ends_with(extension)
}

/// Identifies an archive by its content. The file name is never consulted:
/// an archive from outside the program is not trusted to be named honestly.
pub(crate) fn detect_kind<R: Read + Seek>(reader: &mut R) -> ArchiveResult<ArchiveKind> {
    let mut head = [0u8; TAR_BLOCK];
    let read = read_up_to(reader, &mut head)?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|error| super::io_failure(&error))?;
    let head = &head[..read];
    if head.starts_with(ZIP_LOCAL_HEADER) || head.starts_with(ZIP_EMPTY_ARCHIVE) {
        return Ok(ArchiveKind::Zip);
    }
    if head.starts_with(GZIP_MAGIC) {
        let mut decoder = flate2::read::GzDecoder::new(&mut *reader);
        let mut block = [0u8; TAR_BLOCK];
        let inflated = read_up_to(&mut decoder, &mut block)
            .map_err(|_| ArchiveFailure::new(ERR_INVALID_ARCHIVE))?;
        drop(decoder);
        reader
            .seek(SeekFrom::Start(0))
            .map_err(|error| super::io_failure(&error))?;
        return if is_tar_header(&block[..inflated]) {
            Ok(ArchiveKind::TarGz)
        } else {
            Err(ArchiveFailure::new(ERR_UNSUPPORTED_FORMAT))
        };
    }
    if is_tar_header(head) {
        return Ok(ArchiveKind::Tar);
    }
    Err(ArchiveFailure::new(ERR_UNSUPPORTED_FORMAT))
}

/// A ustar or GNU tar header. Pre-POSIX v7 archives carry no magic and are
/// rejected, which the plan accepts: nothing current produces them.
pub(super) fn is_tar_header(block: &[u8]) -> bool {
    block.len() == TAR_BLOCK && &block[TAR_MAGIC_OFFSET..TAR_MAGIC_OFFSET + 5] == b"ustar"
}

pub(super) fn read_up_to<R: Read>(reader: &mut R, buffer: &mut [u8]) -> ArchiveResult<usize> {
    let mut filled = 0;
    while filled < buffer.len() {
        match reader.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(count) => filled += count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(super::io_failure(&error)),
        }
    }
    Ok(filled)
}

const EOCD_SIGNATURE: &[u8] = b"PK\x05\x06";
const ZIP64_LOCATOR_SIGNATURE: &[u8] = b"PK\x06\x07";
const ZIP64_EOCD_SIGNATURE: &[u8] = b"PK\x06\x06";
const EOCD_MIN_LEN: u64 = 22;
const EOCD_MAX_COMMENT: u64 = 0xFFFF;

/// The entry count a ZIP declares in its end-of-central-directory record.
/// Reading it first lets `maxEntries` reject an archive before the central
/// directory, whose size the archive controls, is parsed into memory.
pub(crate) fn zip_declared_entries<R: Read + Seek>(reader: &mut R) -> ArchiveResult<u64> {
    let invalid = || ArchiveFailure::new(ERR_INVALID_ARCHIVE);
    let length = reader
        .seek(SeekFrom::End(0))
        .map_err(|error| super::io_failure(&error))?;
    if length < EOCD_MIN_LEN {
        return Err(invalid());
    }
    let window = length.min(EOCD_MIN_LEN + EOCD_MAX_COMMENT);
    let start = length - window;
    let mut tail = vec![0u8; usize::try_from(window).map_err(|_| invalid())?];
    reader
        .seek(SeekFrom::Start(start))
        .map_err(|error| super::io_failure(&error))?;
    reader
        .read_exact(&mut tail)
        .map_err(|error| super::io_failure(&error))?;
    let eocd = (0..=tail.len() - EOCD_MIN_LEN as usize)
        .rev()
        .find(|&index| &tail[index..index + 4] == EOCD_SIGNATURE)
        .ok_or_else(invalid)?;
    let declared = u16::from_le_bytes([tail[eocd + 10], tail[eocd + 11]]);
    let result = if declared != u16::MAX {
        Ok(u64::from(declared))
    } else {
        zip64_declared_entries(reader, start + eocd as u64)
    };
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|error| super::io_failure(&error))?;
    result
}

fn zip64_declared_entries<R: Read + Seek>(reader: &mut R, eocd_offset: u64) -> ArchiveResult<u64> {
    let invalid = || ArchiveFailure::new(ERR_INVALID_ARCHIVE);
    let locator_offset = eocd_offset.checked_sub(20).ok_or_else(invalid)?;
    let mut locator = [0u8; 20];
    reader
        .seek(SeekFrom::Start(locator_offset))
        .map_err(|error| super::io_failure(&error))?;
    reader.read_exact(&mut locator).map_err(|_| invalid())?;
    if &locator[..4] != ZIP64_LOCATOR_SIGNATURE {
        return Ok(u64::from(u16::MAX));
    }
    let record_offset = u64::from_le_bytes(locator[8..16].try_into().map_err(|_| invalid())?);
    let mut record = [0u8; 40];
    reader
        .seek(SeekFrom::Start(record_offset))
        .map_err(|_| invalid())?;
    reader.read_exact(&mut record).map_err(|_| invalid())?;
    if &record[..4] != ZIP64_EOCD_SIGNATURE {
        return Err(invalid());
    }
    Ok(u64::from_le_bytes(
        record[32..40].try_into().map_err(|_| invalid())?,
    ))
}

#[cfg(test)]
mod tests {
    use super::{ArchiveKind, kind_from_destination};

    #[test]
    fn only_lowercase_known_extensions_pick_a_format() {
        assert_eq!(kind_from_destination("dist/a.zip"), Ok(ArchiveKind::Zip));
        assert_eq!(kind_from_destination("a.tar"), Ok(ArchiveKind::Tar));
        assert_eq!(kind_from_destination("a.tar.gz"), Ok(ArchiveKind::TarGz));
        assert_eq!(
            kind_from_destination("Release-1.0.tgz"),
            Ok(ArchiveKind::TarGz)
        );
        for rejected in [
            "a.ZIP",
            "a.Tar.gz",
            "a.jar",
            "a",
            ".zip",
            "dist/.tar.gz",
            "a.gz",
            "zip",
        ] {
            assert!(kind_from_destination(rejected).is_err(), "{rejected}");
        }
    }
}
