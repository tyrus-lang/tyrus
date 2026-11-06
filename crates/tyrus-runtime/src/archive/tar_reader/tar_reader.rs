use super::{
    ArchiveFailure, ArchiveResult, ERR_ENTRY_LIMIT, ERR_INVALID_ARCHIVE, ERR_UNSUPPORTED_ENTRY,
    EntryKind,
};
use std::io::Read;

/// Largest GNU long-name or PAX header accepted, in bytes. The `tar` crate
/// reads those headers whole into memory with the size the archive declares,
/// so a few megabytes of compressed zeros could claim gigabytes; reading them
/// here, capped, keeps the allocation bounded by this constant.
pub(crate) const MAX_EXTENSION_BYTES: u64 = 64 * 1024;

/// A member can be preceded by a PAX header, a GNU long name and a GNU long
/// link, so it spans at most this many raw headers. The raw-header budget is
/// the entry limit times this, so extension headers cannot be used to loop
/// through an unbounded stream without ever reaching `maxEntries`.
const HEADERS_PER_MEMBER: u64 = 4;

/// One archive member, with the name its extension headers gave it.
pub(crate) struct TarMember<'entry, 'archive, R: Read> {
    pub(crate) name: Vec<u8>,
    pub(crate) kind: EntryKind,
    pub(crate) size: u64,
    pub(crate) executable: bool,
    pub(crate) data: &'entry mut tar::Entry<'archive, R>,
}

pub(crate) fn tar_entry_kind(entry_type: tar::EntryType) -> EntryKind {
    match entry_type {
        tar::EntryType::Regular | tar::EntryType::Continuous => EntryKind::File,
        tar::EntryType::Directory => EntryKind::Directory,
        tar::EntryType::Symlink => EntryKind::Symlink,
        tar::EntryType::Link => EntryKind::HardLink,
        _ => EntryKind::Other,
    }
}

/// The `tar` crate reports every malformed header or truncated block as
/// `ErrorKind::Other`, so inside tar parsing that kind means a bad archive,
/// not a failing disk.
pub(crate) fn tar_failure(error: &std::io::Error, entry: Option<&str>) -> ArchiveFailure {
    let code = match error.kind() {
        std::io::ErrorKind::Other
        | std::io::ErrorKind::InvalidData
        | std::io::ErrorKind::InvalidInput
        | std::io::ErrorKind::UnexpectedEof => ERR_INVALID_ARCHIVE,
        _ => {
            return entry.map_or_else(
                || super::io_failure(error),
                |entry| super::io_failure_at(error, entry),
            );
        }
    };
    match entry {
        Some(entry) => ArchiveFailure::at(code, entry),
        None => ArchiveFailure::new(code),
    }
}

fn invalid() -> ArchiveFailure {
    ArchiveFailure::new(ERR_INVALID_ARCHIVE)
}

fn read_extension<R: Read>(entry: &mut tar::Entry<'_, R>) -> ArchiveResult<Vec<u8>> {
    let size = entry
        .header()
        .size()
        .map_err(|error| tar_failure(&error, None))?;
    if size > MAX_EXTENSION_BYTES {
        return Err(invalid());
    }
    let mut data = Vec::with_capacity(usize::try_from(size).map_err(|_| invalid())?);
    entry
        .take(MAX_EXTENSION_BYTES)
        .read_to_end(&mut data)
        .map_err(|error| tar_failure(&error, None))?;
    if data.len() as u64 != size {
        return Err(invalid());
    }
    Ok(data)
}

#[derive(Default)]
struct PaxRecords {
    path: Option<Vec<u8>>,
    size: Option<u64>,
}

/// Parses `<length> <key>=<value>\n` records. Only `path` and `size` change
/// how a member is read; every other key is metadata the module never applies.
fn parse_pax(data: &[u8]) -> ArchiveResult<PaxRecords> {
    let mut records = PaxRecords::default();
    let mut rest = data;
    while !rest.is_empty() {
        let space = rest
            .iter()
            .position(|byte| *byte == b' ')
            .ok_or_else(invalid)?;
        let length: usize = std::str::from_utf8(&rest[..space])
            .ok()
            .and_then(|digits| digits.parse().ok())
            .ok_or_else(invalid)?;
        if length <= space + 1 || length > rest.len() || rest[length - 1] != b'\n' {
            return Err(invalid());
        }
        let record = &rest[space + 1..length - 1];
        let equals = record
            .iter()
            .position(|byte| *byte == b'=')
            .ok_or_else(invalid)?;
        let (key, value) = (&record[..equals], &record[equals + 1..]);
        match key {
            b"path" => records.path = Some(value.to_vec()),
            b"size" => {
                records.size = Some(
                    std::str::from_utf8(value)
                        .ok()
                        .and_then(|digits| digits.parse().ok())
                        .ok_or_else(invalid)?,
                );
            }
            _ => {}
        }
        rest = &rest[length..];
    }
    Ok(records)
}

fn strip_trailing_nul(mut name: Vec<u8>) -> Vec<u8> {
    while name.last() == Some(&0) {
        name.pop();
    }
    name
}

/// Walks the members of a tar stream, resolving GNU long names and PAX paths
/// under `MAX_EXTENSION_BYTES`, and hands each member to `visit` with its data
/// still unread.
pub(crate) fn for_each_member<R, F>(reader: R, max_entries: u64, mut visit: F) -> ArchiveResult<()>
where
    R: Read,
    F: FnMut(TarMember<'_, '_, R>) -> ArchiveResult<()>,
{
    let mut archive = tar::Archive::new(reader);
    let entries = archive
        .entries()
        .map_err(|error| tar_failure(&error, None))?
        .raw(true);
    let header_budget = max_entries.saturating_mul(HEADERS_PER_MEMBER);
    let mut headers: u64 = 0;
    let mut long_name: Option<Vec<u8>> = None;
    let mut pax: Option<PaxRecords> = None;
    for entry in entries {
        let mut entry = entry.map_err(|error| tar_failure(&error, None))?;
        headers += 1;
        if headers > header_budget {
            return Err(ArchiveFailure::new(ERR_ENTRY_LIMIT));
        }
        let entry_type = entry.header().entry_type();
        if entry_type.is_gnu_longname() {
            if long_name.is_some() {
                return Err(invalid());
            }
            long_name = Some(strip_trailing_nul(read_extension(&mut entry)?));
            continue;
        }
        if entry_type.is_gnu_longlink() || entry_type.is_pax_global_extensions() {
            read_extension(&mut entry)?;
            continue;
        }
        if entry_type.is_pax_local_extensions() {
            if pax.is_some() {
                return Err(invalid());
            }
            pax = Some(parse_pax(&read_extension(&mut entry)?)?);
            continue;
        }
        let records = pax.take().unwrap_or_default();
        let name = long_name
            .take()
            .or(records.path)
            .unwrap_or_else(|| entry.path_bytes().into_owned());
        let size = entry
            .header()
            .size()
            .map_err(|error| tar_failure(&error, None))?;
        if records.size.is_some_and(|declared| declared != size) {
            let lossy = String::from_utf8_lossy(&name).into_owned();
            return Err(ArchiveFailure::at(ERR_UNSUPPORTED_ENTRY, &lossy));
        }
        let kind = if entry_type.is_gnu_sparse() {
            EntryKind::Other
        } else {
            tar_entry_kind(entry_type)
        };
        let executable = entry.header().mode().is_ok_and(|mode| mode & 0o100 != 0);
        visit(TarMember {
            name,
            kind,
            size,
            executable,
            data: &mut entry,
        })?;
    }
    if long_name.is_some() || pax.is_some() {
        return Err(invalid());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{MAX_EXTENSION_BYTES, for_each_member, parse_pax};
    use crate::archive::{ERR_ENTRY_LIMIT, ERR_INVALID_ARCHIVE, ERR_UNSUPPORTED_ENTRY, EntryKind};
    use std::io::Write;

    fn raw_header(name: &[u8], type_flag: u8, size: u64) -> [u8; 512] {
        let mut header = tar::Header::new_ustar();
        header.as_old_mut().name[..name.len()].copy_from_slice(name);
        header.set_size(size);
        header.set_mode(0o644);
        header.set_entry_type(tar::EntryType::new(type_flag));
        header.set_cksum();
        *header.as_bytes()
    }

    fn padded(mut bytes: Vec<u8>) -> Vec<u8> {
        bytes.resize(bytes.len().div_ceil(512) * 512, 0);
        bytes
    }

    fn names(archive: &[u8]) -> Result<Vec<(String, EntryKind, u64)>, &'static str> {
        let mut seen = Vec::new();
        for_each_member(archive, 100, |member| {
            seen.push((
                String::from_utf8_lossy(&member.name).into_owned(),
                member.kind,
                member.size,
            ));
            Ok(())
        })
        .map_err(|failure| failure.code)?;
        Ok(seen)
    }

    #[test]
    fn resolves_gnu_long_names_and_pax_paths() {
        let long = "d/".repeat(60) + "file.txt";
        let mut builder = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_size(2);
        header.set_mode(0o644);
        builder.append_data(&mut header, &long, &b"hi"[..]).unwrap();
        builder
            .append_pax_extensions([("path", b"pax/named.txt".as_slice())])
            .unwrap();
        let mut header = tar::Header::new_ustar();
        header.set_size(3);
        header.set_mode(0o644);
        builder
            .append_data(&mut header, "short", &b"abc"[..])
            .unwrap();
        let archive = builder.into_inner().unwrap();
        assert_eq!(
            names(&archive).unwrap(),
            vec![
                (long, EntryKind::File, 2),
                ("pax/named.txt".to_string(), EntryKind::File, 3),
            ]
        );
    }

    #[test]
    fn an_oversized_long_name_header_is_rejected_before_it_is_read() {
        let mut archive = raw_header(b"././@LongLink", b'L', 1 << 30).to_vec();
        archive.extend_from_slice(&[b'a'; 512]);
        assert_eq!(names(&archive), Err(ERR_INVALID_ARCHIVE));

        let mut archive = raw_header(b"pax", b'x', MAX_EXTENSION_BYTES + 1).to_vec();
        archive.extend_from_slice(&[b'a'; 512]);
        assert_eq!(names(&archive), Err(ERR_INVALID_ARCHIVE));
    }

    #[test]
    fn a_compressed_bomb_in_a_long_name_never_reaches_memory() {
        // 256 MiB of zeros declared as one GNU long name, a few hundred KiB
        // once gzipped: the header is refused from its declared size, so the
        // decompressor never produces more than the first block of it.
        let size: u64 = 256 * 1024 * 1024;
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder
            .write_all(&raw_header(b"././@LongLink", b'L', size))
            .unwrap();
        let zeros = vec![0u8; 1024 * 1024];
        for _ in 0..(size / zeros.len() as u64) {
            encoder.write_all(&zeros).unwrap();
        }
        let compressed = encoder.finish().unwrap();
        let consumed = std::cell::Cell::new(0u64);
        let decoder = Counting {
            inner: flate2::read::GzDecoder::new(compressed.as_slice()),
            consumed: &consumed,
        };
        let result = for_each_member(decoder, 100, |_| Ok(()));
        assert_eq!(
            result.map_err(|failure| failure.code),
            Err(ERR_INVALID_ARCHIVE)
        );
        assert!(
            consumed.get() < 64 * 1024,
            "read {} decompressed bytes of a header that should be refused unread",
            consumed.get()
        );
    }

    struct Counting<'a, R> {
        inner: R,
        consumed: &'a std::cell::Cell<u64>,
    }

    impl<R: std::io::Read> std::io::Read for Counting<'_, R> {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let count = self.inner.read(buffer)?;
            self.consumed.set(self.consumed.get() + count as u64);
            Ok(count)
        }
    }

    #[test]
    fn rejects_dangling_and_repeated_extension_headers() {
        let mut archive = raw_header(b"././@LongLink", b'L', 4).to_vec();
        archive.extend(padded(b"name".to_vec()));
        archive.extend_from_slice(&[0u8; 1024]);
        assert_eq!(names(&archive), Err(ERR_INVALID_ARCHIVE));

        let mut archive = raw_header(b"././@LongLink", b'L', 1).to_vec();
        archive.extend(padded(b"a".to_vec()));
        archive.extend_from_slice(&raw_header(b"././@LongLink", b'L', 1));
        archive.extend(padded(b"b".to_vec()));
        assert_eq!(names(&archive), Err(ERR_INVALID_ARCHIVE));
    }

    #[test]
    fn a_pax_size_that_disagrees_with_the_header_is_refused() {
        let mut builder = tar::Builder::new(Vec::new());
        builder
            .append_pax_extensions([("size", b"99999999999".as_slice())])
            .unwrap();
        let mut header = tar::Header::new_ustar();
        header.set_size(1);
        header.set_mode(0o644);
        builder.append_data(&mut header, "big", &b"x"[..]).unwrap();
        assert_eq!(
            names(&builder.into_inner().unwrap()),
            Err(ERR_UNSUPPORTED_ENTRY)
        );
    }

    #[test]
    fn extension_headers_count_toward_the_header_budget() {
        let mut archive = Vec::new();
        for _ in 0..10 {
            archive.extend_from_slice(&raw_header(b"global", b'g', 0));
        }
        let result = for_each_member(archive.as_slice(), 2, |_| Ok(()));
        assert_eq!(result.map_err(|failure| failure.code), Err(ERR_ENTRY_LIMIT));
    }

    #[test]
    fn parses_pax_records_and_rejects_malformed_ones() {
        let records = parse_pax(b"18 path=a/b/c.txt\n11 size=42\n").unwrap();
        assert_eq!(records.path.as_deref(), Some(b"a/b/c.txt".as_slice()));
        assert_eq!(records.size, Some(42));
        for malformed in [
            &b"99 path=x\n"[..],
            b"x path=y\n",
            b"9 pathx\n",
            b"10 size=a\n",
        ] {
            assert!(parse_pax(malformed).is_err(), "{malformed:?}");
        }
    }
}
