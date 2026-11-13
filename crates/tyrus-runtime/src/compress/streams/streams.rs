//! The compression streams: the incremental codecs behind a byte
//! `TransformStream`, failing both of its sides with the module's own error
//! code when the data or a limit is refused.

use super::{code_for, message_for};
use crate::codec::{CodecError, Decoder, Encoder};
use crate::stream::{ByteTransform, StreamError};

pub(crate) fn stream_error(error: CodecError) -> StreamError {
    let code = code_for(error);
    StreamError {
        code,
        message: message_for(code),
    }
}

pub(crate) struct EncoderTransform(pub(crate) Encoder);

impl ByteTransform for EncoderTransform {
    fn push(&mut self, chunk: &[u8]) -> Result<(), StreamError> {
        self.0.push(chunk).map_err(stream_error)
    }

    fn pull(&mut self) -> Result<Option<Vec<u8>>, StreamError> {
        Ok(self.0.pull())
    }

    fn finish(&mut self) -> Result<(), StreamError> {
        self.0.finish().map_err(stream_error)
    }
}

pub(crate) struct DecoderTransform(pub(crate) Decoder);

impl ByteTransform for DecoderTransform {
    fn push(&mut self, chunk: &[u8]) -> Result<(), StreamError> {
        self.0.push(chunk);
        Ok(())
    }

    fn pull(&mut self) -> Result<Option<Vec<u8>>, StreamError> {
        self.0.pull().map_err(stream_error)
    }

    fn finish(&mut self) -> Result<(), StreamError> {
        self.0.finish();
        Ok(())
    }
}
