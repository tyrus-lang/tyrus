//! `Read` and `Write` over the incremental codecs, for code that already
//! speaks `std::io`, such as the TAR reader and writer of `tyrus:archive`.

use super::{CHUNK, CodecError, Decoder, Encoder, Format, Limits};
use std::io::{Read, Write};

fn io_error(error: CodecError) -> std::io::Error {
    let kind = match error {
        CodecError::AllocationFailed => std::io::ErrorKind::OutOfMemory,
        _ => std::io::ErrorKind::InvalidData,
    };
    std::io::Error::new(kind, format!("{error:?}"))
}

/// The decoded bytes of `inner`. The limits are the caller's: `tyrus:archive`
/// passes none and checks its own aggregate ratio over what it extracts.
pub(crate) struct DecoderReader<R> {
    inner: R,
    decoder: Decoder,
    input: Vec<u8>,
    piece: Vec<u8>,
    offset: usize,
    input_ended: bool,
    output_ended: bool,
}

impl<R: Read> DecoderReader<R> {
    pub(crate) fn new(inner: R, format: Option<Format>, limits: Limits) -> Self {
        Self {
            inner,
            decoder: Decoder::new(format, limits),
            input: vec![0; CHUNK],
            piece: Vec::new(),
            offset: 0,
            input_ended: false,
            output_ended: false,
        }
    }
}

impl<R: Read> Read for DecoderReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        loop {
            if self.offset < self.piece.len() {
                let count = buffer.len().min(self.piece.len() - self.offset);
                buffer[..count].copy_from_slice(&self.piece[self.offset..self.offset + count]);
                self.offset += count;
                return Ok(count);
            }
            if self.output_ended || buffer.is_empty() {
                return Ok(0);
            }
            match self.decoder.pull().map_err(io_error)? {
                Some(piece) => {
                    self.piece = piece;
                    self.offset = 0;
                }
                None if self.input_ended => {
                    self.output_ended = true;
                }
                None => {
                    let read = loop {
                        match self.inner.read(&mut self.input) {
                            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                            other => break other?,
                        }
                    };
                    if read == 0 {
                        self.decoder.finish();
                        self.input_ended = true;
                    } else {
                        self.decoder.push(&self.input[..read]);
                    }
                }
            }
        }
    }
}

/// Compresses what is written into `inner`; `finish` writes the trailer and
/// hands `inner` back.
pub(crate) struct EncoderWriter<W> {
    inner: W,
    encoder: Encoder,
}

impl<W: Write> EncoderWriter<W> {
    pub(crate) fn new(inner: W, format: Format, level: Option<i32>) -> Result<Self, CodecError> {
        Ok(Self {
            inner,
            encoder: Encoder::new(format, level)?,
        })
    }

    fn drain(&mut self) -> std::io::Result<()> {
        while let Some(piece) = self.encoder.pull() {
            self.inner.write_all(&piece)?;
        }
        Ok(())
    }

    pub(crate) fn finish(mut self) -> std::io::Result<W> {
        self.encoder.finish().map_err(io_error)?;
        self.drain()?;
        Ok(self.inner)
    }
}

impl<W: Write> Write for EncoderWriter<W> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.encoder.push(buffer).map_err(io_error)?;
        self.drain()?;
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
