//! `packStream` and `unpackStream`: TAR, TAR.GZ and ZIP written into a
//! `ReadableStream<u8[]>`, and TAR or TAR.GZ extracted from one. The archive
//! code speaks `std::io` on the blocking pool; these adapters block on the
//! stream's own backpressure, bounded by the operation's deadline.

use super::format::{
    GZIP_MAGIC, TAR_BLOCK, ZIP_EMPTY_ARCHIVE, ZIP_LOCAL_HEADER, is_tar_header, read_up_to,
};
use super::pack::pack_into;
use super::unpack::unpack_tar_from;
use super::{
    ArchiveFailure, ArchiveKind, ArchiveResult, Deadline, ERR_UNSUPPORTED_FORMAT, GuardedReader,
    PackOptions, Summary, UnpackOptions, message_for,
};
use crate::stream::{ByteStreamProducer, StreamError, TyrusReadableStream};
use std::io::{Read, Write};
use std::sync::Arc;

#[cfg(test)]
#[path = "tests/tests.rs"]
mod tests;

/// Bytes gathered before a chunk is handed to the stream, and the most one
/// chunk may hold: the stream's capacity.
pub(super) const CHUNK_BYTES: usize = 64 * 1024;

fn stream_error(failure: &ArchiveFailure) -> StreamError {
    StreamError {
        code: failure.code,
        message: message_for(failure.code),
    }
}

/// Waits for `future` on the calling blocking thread, giving up at the
/// deadline so a stream nobody reads or writes cannot hold the call forever.
fn wait<T>(
    handle: &tokio::runtime::Handle,
    deadline: Deadline,
    future: impl std::future::Future<Output = T>,
) -> std::io::Result<T> {
    match deadline.remaining() {
        None => Ok(handle.block_on(future)),
        Some(left) => handle
            .block_on(async { tokio::time::timeout(left, future).await })
            .map_err(|_| std::io::ErrorKind::TimedOut.into()),
    }
}

/// A `Write` that hands its bytes to a readable stream in chunks.
struct StreamSink {
    producer: ByteStreamProducer,
    handle: tokio::runtime::Handle,
    deadline: Deadline,
    buffer: Vec<u8>,
}

impl StreamSink {
    /// Hands the buffered bytes over in chunks no larger than the stream's
    /// capacity, which refuses a chunk it could never hold.
    fn send(&mut self) -> std::io::Result<()> {
        let buffered = std::mem::replace(&mut self.buffer, Vec::with_capacity(CHUNK_BYTES));
        for chunk in buffered.chunks(CHUNK_BYTES) {
            wait(
                &self.handle,
                self.deadline,
                self.producer.write(chunk.to_vec()),
            )?
            .map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "the archive stream was closed by its reader",
                )
            })?;
        }
        Ok(())
    }
}

impl Write for StreamSink {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.buffer.extend_from_slice(bytes);
        if self.buffer.len() >= CHUNK_BYTES {
            self.send()?;
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.send()
    }
}

/// Writes the archive of `source` into `producer`, closing it at the end or
/// failing it with the archive error.
pub(crate) fn pack_to_stream(
    kind: ArchiveKind,
    source: &str,
    options: &PackOptions,
    producer: ByteStreamProducer,
    handle: tokio::runtime::Handle,
) {
    let deadline = match Deadline::from_timeout(options.timeout_ms) {
        Ok(deadline) => deadline,
        Err(failure) => return producer.fail_with(stream_error(&failure)),
    };
    let mut sink = StreamSink {
        producer,
        handle,
        deadline,
        buffer: Vec::with_capacity(CHUNK_BYTES),
    };
    // Dropping the producer would end the stream as if the archive were
    // complete, so a failure must reach the reader as a failure.
    let written = pack_into(kind, source, options, deadline, &mut sink)
        .and_then(|(_, sink)| sink.send().map_err(|error| super::io_failure(&error)));
    match written {
        Ok(()) => sink.producer.close(),
        Err(failure) => sink.producer.fail_with(stream_error(&failure)),
    }
}

/// The bytes of a readable stream, as a `Read`.
struct StreamSource {
    stream: Arc<TyrusReadableStream>,
    handle: tokio::runtime::Handle,
    deadline: Deadline,
    piece: Vec<u8>,
    offset: usize,
    ended: bool,
}

impl Read for StreamSource {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        loop {
            if self.offset < self.piece.len() {
                let count = buffer.len().min(self.piece.len() - self.offset);
                buffer[..count].copy_from_slice(&self.piece[self.offset..self.offset + count]);
                self.offset += count;
                return Ok(count);
            }
            if self.ended || buffer.is_empty() {
                return Ok(0);
            }
            let next = wait(
                &self.handle,
                self.deadline,
                crate::stream::read_owned_byte_stream(&self.stream),
            )?;
            match next {
                Ok(Some(piece)) => {
                    self.piece = piece;
                    self.offset = 0;
                }
                Ok(None) => self.ended = true,
                // Not `Other`, which the TAR reader takes for a malformed
                // archive: the source failed, the archive did not.
                Err(_) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::ConnectionAborted,
                        "the archive stream failed",
                    ));
                }
            }
        }
    }
}

/// Extracts the TAR or TAR.GZ `stream` carries into `destination`, with the
/// safety of `unpack`. The format is identified from the first block.
pub(crate) fn unpack_from_stream(
    stream: Arc<TyrusReadableStream>,
    destination: &str,
    options: &UnpackOptions,
    handle: tokio::runtime::Handle,
) -> ArchiveResult<Summary> {
    let deadline = Deadline::from_timeout(options.timeout_ms)?;
    let source = StreamSource {
        stream,
        handle,
        deadline,
        piece: Vec::new(),
        offset: 0,
        ended: false,
    };
    let mut guarded = GuardedReader::new(source, deadline);
    let consumed = guarded.consumed();
    let mut head = vec![0_u8; TAR_BLOCK];
    let read = read_up_to(&mut guarded, &mut head)?;
    head.truncate(read);
    let kind = if head.starts_with(ZIP_LOCAL_HEADER) || head.starts_with(ZIP_EMPTY_ARCHIVE) {
        ArchiveKind::Zip
    } else if head.starts_with(GZIP_MAGIC) {
        ArchiveKind::TarGz
    } else if is_tar_header(&head) {
        ArchiveKind::Tar
    } else {
        return Err(ArchiveFailure::new(ERR_UNSUPPORTED_FORMAT));
    };
    let reader = std::io::Cursor::new(head).chain(guarded);
    unpack_tar_from(reader, kind, consumed, destination, options, deadline)
}
