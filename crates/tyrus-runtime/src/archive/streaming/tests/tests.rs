use super::super::ffi::*;
use crate::executor::executor;
use crate::stream::{TyrusReadableStream, read_owned_byte_stream, take_owned_readable_stream};
use std::sync::Arc;
use tyrus_string::TyrusString;

struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("tyrus-archive-stream-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("src/nested")).unwrap();
        std::fs::write(root.join("src/readme.txt"), b"streamed archive\n").unwrap();
        std::fs::write(root.join("src/nested/data.bin"), vec![7_u8; 200_000]).unwrap();
        Self(root)
    }

    fn text(&self, relative: &str) -> String {
        self.0.join(relative).to_string_lossy().into_owned()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const ZIP: i32 = 0;
const TAR: i32 = 1;
const TAR_GZ: i32 = 2;

fn pack_stream(source: &str, format: i32) -> Arc<TyrusReadableStream> {
    unsafe {
        take_owned_readable_stream(tyrus_archive_pack_stream(
            TyrusString::lend(source).unwrap(),
            format,
            std::ptr::null(),
        ))
    }
}

fn read_all(stream: &Arc<TyrusReadableStream>) -> Result<Vec<u8>, &'static str> {
    executor().block_on(async {
        let mut bytes = Vec::new();
        loop {
            match read_owned_byte_stream(stream).await {
                Ok(Some(chunk)) => bytes.extend(chunk),
                Ok(None) => return Ok(bytes),
                Err(error) => return Err(error.code),
            }
        }
    })
}

fn bytes_stream(bytes: Vec<u8>) -> Arc<TyrusReadableStream> {
    let (readable, producer) = crate::stream::byte_readable_stream(64 * 1024);
    executor().spawn(async move {
        for piece in bytes.chunks(5_000) {
            if producer.write(piece.to_vec()).await.is_err() {
                return;
            }
        }
        producer.close();
    });
    unsafe { take_owned_readable_stream(readable) }
}

fn unpack_stream(
    stream: &Arc<TyrusReadableStream>,
    destination: &str,
) -> Result<(u64, u64), &'static str> {
    unsafe {
        let promise = tyrus_archive_unpack_stream(
            Arc::as_ptr(stream),
            TyrusString::lend(destination).unwrap(),
            std::ptr::null(),
        );
        crate::wait_until_settled(promise);
        let result = &*crate::promise::tyrus_promise_await(promise)
            .cast::<ArchiveResultFfi<ArchiveSummaryFfi>>();
        let outcome = if result.tag == 0 {
            Ok((result.payload.ok.entries, result.payload.ok.bytes))
        } else {
            Err(match result.payload.err.code.as_str() {
                "ERR_ARCHIVE_ZIP_NOT_STREAMABLE" => "ERR_ARCHIVE_ZIP_NOT_STREAMABLE",
                "ERR_ARCHIVE_PATH_ESCAPE" => "ERR_ARCHIVE_PATH_ESCAPE",
                "ERR_ARCHIVE_UNSUPPORTED_FORMAT" => "ERR_ARCHIVE_UNSUPPORTED_FORMAT",
                other => Box::leak(other.to_owned().into_boxed_str()),
            })
        };
        crate::tyrus_promise_free(promise);
        outcome
    }
}

#[test]
fn a_packed_stream_extracts_like_a_packed_file_in_every_format() {
    let scratch = Scratch::new();
    for (format, name) in [
        (TAR, "stream.tar"),
        (TAR_GZ, "stream.tgz"),
        (ZIP, "stream.zip"),
    ] {
        let bytes = read_all(&pack_stream(&scratch.text("src"), format)).unwrap();
        std::fs::write(scratch.text(name), &bytes).unwrap();
        let out = scratch.text(&format!("out-{format}"));
        let summary =
            crate::archive::unpack(&scratch.text(name), &out, &Default::default()).unwrap();
        assert_eq!(summary.entries, 3, "{name}");
        assert_eq!(
            std::fs::read(format!("{out}/nested/data.bin")).unwrap(),
            vec![7_u8; 200_000]
        );
        if format != ZIP {
            let streamed_out = scratch.text(&format!("streamed-{format}"));
            let summary = unpack_stream(&bytes_stream(bytes), &streamed_out).unwrap();
            assert_eq!(summary, (3, 200_017), "{name} through unpackStream");
            assert_eq!(
                std::fs::read(format!("{streamed_out}/readme.txt")).unwrap(),
                b"streamed archive\n"
            );
        }
    }
}

#[test]
fn a_zip_stream_is_refused_and_a_missing_source_fails_the_stream() {
    let scratch = Scratch::new();
    let zip = read_all(&pack_stream(&scratch.text("src"), ZIP)).unwrap();
    assert_eq!(
        unpack_stream(&bytes_stream(zip), &scratch.text("zip-out")),
        Err("ERR_ARCHIVE_ZIP_NOT_STREAMABLE")
    );
    assert!(!std::path::Path::new(&scratch.text("zip-out")).exists());
    assert_eq!(
        unpack_stream(
            &bytes_stream(b"plain text, not an archive".to_vec()),
            &scratch.text("text-out")
        ),
        Err("ERR_ARCHIVE_UNSUPPORTED_FORMAT")
    );
    assert_eq!(
        read_all(&pack_stream(&scratch.text("missing"), TAR)),
        Err("ERR_ARCHIVE_NOT_FOUND")
    );
    assert_eq!(
        read_all(&pack_stream(&scratch.text("src"), 9)),
        Err("ERR_ARCHIVE_INVALID_OPTION")
    );
}

#[test]
fn an_escaping_entry_in_a_stream_is_refused_and_rolled_back() {
    let scratch = Scratch::new();
    let mut header = tar::Header::new_gnu();
    header.set_size(4);
    header.set_mode(0o644);
    header.set_entry_type(tar::EntryType::Regular);
    {
        let name = b"../escaped.txt";
        let raw = header.as_old_mut();
        raw.name[..name.len()].copy_from_slice(name);
    }
    header.set_cksum();
    let mut archive = header.as_bytes().to_vec();
    archive.extend_from_slice(b"evil");
    archive.resize(archive.len() + (512 - 4) + 1024, 0);
    assert_eq!(
        unpack_stream(&bytes_stream(archive), &scratch.text("out")),
        Err("ERR_ARCHIVE_PATH_ESCAPE")
    );
    assert!(!std::path::Path::new(&scratch.text("escaped.txt")).exists());
    assert!(!std::path::Path::new(&scratch.text("out")).exists());
}

#[test]
fn a_reader_that_stops_early_ends_the_pack_instead_of_blocking_it() {
    let scratch = Scratch::new();
    let stream = pack_stream(&scratch.text("src"), TAR);
    let first = executor()
        .block_on(read_owned_byte_stream(&stream))
        .unwrap()
        .unwrap();
    assert!(!first.is_empty());
    crate::stream::close_owned_byte_stream(&stream);
    drop(stream);
    // Nothing to assert beyond not hanging: the writer learns the reader is
    // gone on its next write and the blocking task returns.
    std::thread::sleep(std::time::Duration::from_millis(200));
}

#[test]
fn streamed_archives_are_read_by_unzip_and_tar() {
    let scratch = Scratch::new();
    for (format, name, program, args) in [
        (ZIP, "tool.zip", "unzip", &["-tq"][..]),
        (TAR_GZ, "tool.tgz", "tar", &["-tzf"][..]),
        (TAR, "tool.tar", "tar", &["-tf"][..]),
    ] {
        let bytes = read_all(&pack_stream(&scratch.text("src"), format)).unwrap();
        std::fs::write(scratch.text(name), &bytes).unwrap();
        let Ok(output) = std::process::Command::new(program)
            .args(args)
            .arg(scratch.text(name))
            .output()
        else {
            continue;
        };
        assert!(
            output.status.success(),
            "{program} rejected {name}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if program == "tar" {
            let listing = String::from_utf8_lossy(&output.stdout);
            assert!(listing.contains("nested/data.bin"), "{listing}");
        }
    }
}
