use super::{CodecError, Sink};
use flate2::{Decompress, FlushDecompress, Status};

pub(super) const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];
pub(super) const GZIP_TRAILER: usize = 8;
pub(super) const ZLIB_TRAILER: usize = 4;

const FLAG_HEADER_CRC: u8 = 0x02;
const FLAG_EXTRA: u8 = 0x04;
const FLAG_NAME: u8 = 0x08;
const FLAG_COMMENT: u8 = 0x10;
const FLAG_RESERVED: u8 = 0xe0;
const ZLIB_PRESET_DICTIONARY: u8 = 0x20;

/// Inflates one raw Deflate stream from the start of `input` and returns how
/// many bytes of it the stream used. `consumed_before` is what earlier framing
/// already took, so the ratio limit sees the whole buffer.
fn inflate(input: &[u8], sink: &mut Sink, consumed_before: u64) -> Result<usize, CodecError> {
    let mut state = Decompress::new(false);
    loop {
        sink.reserve_chunk()?;
        let used = usize::try_from(state.total_in()).map_err(|_| CodecError::InvalidData)?;
        let produced = sink.output.len();
        let status = state
            .decompress_vec(&input[used..], &mut sink.output, FlushDecompress::None)
            .map_err(|_| CodecError::InvalidData)?;
        let now_used = usize::try_from(state.total_in()).map_err(|_| CodecError::InvalidData)?;
        sink.check(consumed_before + now_used as u64)?;
        match status {
            Status::StreamEnd => return Ok(now_used),
            Status::Ok | Status::BufError => {
                // Room for a whole chunk and nothing moved: the stream stops
                // before its final block.
                if now_used == used && sink.output.len() == produced {
                    return Err(CodecError::InvalidData);
                }
            }
        }
    }
}

pub(super) fn raw(input: &[u8], sink: &mut Sink) -> Result<(), CodecError> {
    let used = inflate(input, sink, 0)?;
    if used != input.len() {
        return Err(CodecError::InvalidData);
    }
    Ok(())
}

pub(super) fn zlib(input: &[u8], sink: &mut Sink) -> Result<(), CodecError> {
    zlib_header(input)?.ok_or(CodecError::InvalidData)?;
    let body = 2 + inflate(&input[2..], sink, 2)?;
    let trailer = input
        .get(body..body + ZLIB_TRAILER)
        .ok_or(CodecError::InvalidData)?;
    let expected = u32::from_be_bytes([trailer[0], trailer[1], trailer[2], trailer[3]]);
    if adler32(&sink.output) != expected {
        return Err(CodecError::ChecksumMismatch);
    }
    if body + ZLIB_TRAILER != input.len() {
        return Err(CodecError::InvalidData);
    }
    Ok(())
}

/// Every member of a multi-member file, as `gzip -d` reads them. Anything after
/// the last member that is not another member is refused.
pub(super) fn gzip(input: &[u8], sink: &mut Sink) -> Result<(), CodecError> {
    let mut position = 0;
    loop {
        position += gzip_header(&input[position..])?;
        let start = sink.output.len();
        position += inflate(&input[position..], sink, position as u64)?;
        let trailer = input
            .get(position..position + GZIP_TRAILER)
            .ok_or(CodecError::InvalidData)?;
        let crc = u32::from_le_bytes([trailer[0], trailer[1], trailer[2], trailer[3]]);
        let size = u32::from_le_bytes([trailer[4], trailer[5], trailer[6], trailer[7]]);
        let member = &sink.output[start..];
        let mut actual = flate2::Crc::new();
        actual.update(member);
        #[allow(clippy::cast_possible_truncation)]
        let wrapped_size = member.len() as u32;
        if actual.sum() != crc || wrapped_size != size {
            return Err(CodecError::ChecksumMismatch);
        }
        position += GZIP_TRAILER;
        if position == input.len() {
            return Ok(());
        }
        if !input[position..].starts_with(&GZIP_MAGIC) {
            return Err(CodecError::InvalidData);
        }
    }
}

/// Length of the RFC 1952 member header at the start of `input`.
fn gzip_header(input: &[u8]) -> Result<usize, CodecError> {
    gzip_header_length(input)?.ok_or(CodecError::InvalidData)
}

/// Length of the member header at the start of `input`, or `None` when
/// `input` ends before the header does and more bytes may complete it.
pub(super) fn gzip_header_length(input: &[u8]) -> Result<Option<usize>, CodecError> {
    let checked = |length: usize| input.len() >= length;
    if !input.is_empty() && input[0] != GZIP_MAGIC[0]
        || input.len() > 1 && input[1] != GZIP_MAGIC[1]
        || input.len() > 2 && input[2] != 8
        || input.len() > 3 && input[3] & FLAG_RESERVED != 0
    {
        return Err(CodecError::InvalidData);
    }
    if !checked(10) {
        return Ok(None);
    }
    let flags = input[3];
    let mut length = 10;
    if flags & FLAG_EXTRA != 0 {
        if !checked(length + 2) {
            return Ok(None);
        }
        length += 2 + usize::from(u16::from_le_bytes([input[length], input[length + 1]]));
    }
    for flag in [FLAG_NAME, FLAG_COMMENT] {
        if flags & flag != 0 {
            let Some(text) = input.get(length..) else {
                return Ok(None);
            };
            let Some(end) = text.iter().position(|byte| *byte == 0) else {
                return Ok(None);
            };
            length += end + 1;
        }
    }
    if flags & FLAG_HEADER_CRC != 0 {
        if !checked(length + 2) {
            return Ok(None);
        }
        let mut crc = flate2::Crc::new();
        crc.update(&input[..length]);
        #[allow(clippy::cast_possible_truncation)]
        let low = crc.sum() as u16;
        if low != u16::from_le_bytes([input[length], input[length + 1]]) {
            return Err(CodecError::ChecksumMismatch);
        }
        length += 2;
    }
    if !checked(length) {
        return Ok(None);
    }
    Ok(Some(length))
}

/// Checks the two-byte zlib header, `None` while fewer bytes are known.
pub(super) fn zlib_header(input: &[u8]) -> Result<Option<()>, CodecError> {
    let [cmf, flg, ..] = input else {
        return Ok(None);
    };
    if cmf & 0x0f != 8
        || cmf >> 4 > 7
        || (u16::from(*cmf) * 256 + u16::from(*flg)) % 31 != 0
        || flg & ZLIB_PRESET_DICTIONARY != 0
    {
        return Err(CodecError::InvalidData);
    }
    Ok(Some(()))
}

pub(super) fn adler32(data: &[u8]) -> u32 {
    let mut adler = Adler32::new();
    adler.update(data);
    adler.sum()
}

/// Adler-32 over data that arrives in pieces.
pub(super) struct Adler32 {
    low: u32,
    high: u32,
}

impl Adler32 {
    const MODULUS: u32 = 65_521;
    // The largest run of bytes whose sums cannot overflow a `u32` before the
    // reduction, as zlib uses.
    const RUN: usize = 5_552;

    pub(super) fn new() -> Self {
        Self { low: 1, high: 0 }
    }

    pub(super) fn update(&mut self, data: &[u8]) {
        for run in data.chunks(Self::RUN) {
            for byte in run {
                self.low += u32::from(*byte);
                self.high += self.low;
            }
            self.low %= Self::MODULUS;
            self.high %= Self::MODULUS;
        }
    }

    pub(super) fn sum(&self) -> u32 {
        (self.high << 16) | self.low
    }
}
