//! Transparent response decompression for `fetch`: the request offers the
//! codings the runtime can decode, and a response body in one of them is
//! decoded with the same limits as `tyrus:compress`, applied while decoding.

use crate::codec::{CodecError, Decoder, Format, Limits};
use crate::stream::{ByteStreamProducer, StreamError};

#[cfg(test)]
#[path = "tests/tests.rs"]
mod tests;

/// What `fetch` asks for when the program has not chosen `Accept-Encoding`.
pub(super) const OFFERED_CODINGS: &str = "gzip, deflate, br";

/// A decoded body may expand at most this many times its compressed size once
/// past the first 1 MiB, as in `tyrus:compress`.
pub(super) const MAX_RATIO: u32 = 200;

pub(super) const DECODE_FAILED: (&str, &str) = (
    "ERR_HTTP_FETCH_DECODE",
    "the response body does not match its Content-Encoding",
);
const RATIO_EXCEEDED: (&str, &str) = (
    "ERR_HTTP_FETCH_RATIO_LIMIT",
    "the response body expands more than 200 times its compressed size",
);
const TOO_LARGE: (&str, &str) = (
    "ERR_HTTP_FETCH_BODY_TOO_LARGE",
    "the response body is larger than the caller allows",
);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Coding {
    Gzip,
    /// zlib as the protocol defines it, or the raw Deflate some servers send
    /// instead; the first two bytes tell them apart.
    Deflate,
    Brotli,
}

/// Adds `Accept-Encoding` unless the program already set it.
pub(super) fn offer_codings(headers: &mut Vec<(String, String)>) {
    if !headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("accept-encoding"))
    {
        headers.push(("accept-encoding".to_string(), OFFERED_CODINGS.to_string()));
    }
}

/// The coding of a response the runtime can decode. A body in several stacked
/// codings or in one the runtime does not know is left as it arrived.
pub(super) fn response_coding(headers: &[(String, String)]) -> Option<Coding> {
    let (_, value) = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-encoding"))?;
    match value.trim().to_ascii_lowercase().as_str() {
        "gzip" | "x-gzip" => Some(Coding::Gzip),
        "deflate" => Some(Coding::Deflate),
        "br" => Some(Coding::Brotli),
        _ => None,
    }
}

/// Once decoded, the body no longer has the length or the coding the headers
/// describe, so they are dropped rather than left to contradict it.
pub(super) fn forget_encoded_length(headers: &mut Vec<(String, String)>) {
    headers.retain(|(name, _)| {
        !name.eq_ignore_ascii_case("content-encoding")
            && !name.eq_ignore_ascii_case("content-length")
    });
}

fn format_for(coding: Coding, first_bytes: &[u8]) -> Format {
    match coding {
        Coding::Gzip => Format::Gzip,
        Coding::Deflate if crate::codec::detect(first_bytes) == Some(Format::Deflate) => {
            Format::Deflate
        }
        Coding::Deflate => Format::DeflateRaw,
        Coding::Brotli => Format::Brotli,
    }
}

fn limits(max_body_bytes: u64) -> Limits {
    Limits {
        max_output: max_body_bytes,
        max_ratio: MAX_RATIO,
    }
}

fn failure(error: CodecError) -> (&'static str, &'static str) {
    match error {
        CodecError::OutputLimit | CodecError::AllocationFailed => TOO_LARGE,
        CodecError::RatioLimit => RATIO_EXCEEDED,
        _ => DECODE_FAILED,
    }
}

fn stream_failure(error: CodecError) -> StreamError {
    let (code, message) = failure(error);
    StreamError { code, message }
}

/// Decodes a whole body. An empty body stays empty: a `204` or a `HEAD`
/// response may carry the header with nothing to decode.
pub(super) fn decode_body(
    body: &[u8],
    coding: Coding,
    max_body_bytes: u64,
) -> Result<Vec<u8>, (&'static str, &'static str)> {
    if body.is_empty() {
        return Ok(Vec::new());
    }
    crate::codec::decompress(body, Some(format_for(coding, body)), limits(max_body_bytes))
        .map_err(failure)
}

/// Decodes a streamed body chunk by chunk, writing each piece of output under
/// the stream's backpressure.
pub(super) struct StreamDecoder {
    coding: Coding,
    max_body_bytes: u64,
    decoder: Option<Decoder>,
    /// Bytes held until the raw-or-zlib choice of `deflate` can be made.
    held: Vec<u8>,
}

impl StreamDecoder {
    pub(super) fn new(coding: Coding, max_body_bytes: u64) -> Self {
        Self {
            coding,
            max_body_bytes,
            decoder: None,
            held: Vec::new(),
        }
    }

    fn start(&mut self) -> &mut Decoder {
        let format = format_for(self.coding, &self.held);
        let mut decoder = Decoder::new(Some(format), limits(self.max_body_bytes));
        decoder.push(&std::mem::take(&mut self.held));
        self.decoder.insert(decoder)
    }

    pub(super) async fn push(
        &mut self,
        chunk: &[u8],
        producer: &ByteStreamProducer,
    ) -> Result<(), StreamError> {
        match self.decoder.as_mut() {
            Some(decoder) => decoder.push(chunk),
            None => {
                self.held.extend_from_slice(chunk);
                if self.coding == Coding::Deflate && self.held.len() < 2 {
                    return Ok(());
                }
                self.start();
            }
        }
        self.drain(producer).await
    }

    pub(super) async fn finish(
        &mut self,
        producer: &ByteStreamProducer,
    ) -> Result<(), StreamError> {
        if self.decoder.is_none() {
            if self.held.is_empty() {
                return Ok(());
            }
            self.start();
        }
        if let Some(decoder) = self.decoder.as_mut() {
            decoder.finish();
        }
        self.drain(producer).await
    }

    async fn drain(&mut self, producer: &ByteStreamProducer) -> Result<(), StreamError> {
        let Some(decoder) = self.decoder.as_mut() else {
            return Ok(());
        };
        while let Some(piece) = decoder.pull().map_err(stream_failure)? {
            producer.write(piece).await?;
        }
        Ok(())
    }
}
