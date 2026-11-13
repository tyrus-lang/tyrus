use crate::abi::TyrusArray;
use crate::types::MultipartPart;
use axum::body::Bytes;
use tyrus_string::TyrusString;

pub const MAX_MULTIPART_PARTS: usize = 128;

pub const DEFAULT_MAX_PART_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultipartRefusal {
    NotMultipart,
    Malformed,
    TooLarge,
}

impl MultipartRefusal {
    pub const fn status(self) -> u16 {
        match self {
            Self::NotMultipart => 415,
            Self::Malformed => 400,
            Self::TooLarge => 413,
        }
    }
}

pub struct OwnedPart {
    pub name: String,
    pub file_name: Option<String>,
    pub content_type: Option<String>,
    pub bytes: Vec<u8>,
}

fn base_file_name(raw: &str) -> Option<String> {
    let name = raw
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    if name.is_empty() || name == "." || name == ".." {
        return None;
    }
    Some(name)
}

pub async fn parse_multipart(
    body: Bytes,
    content_type: &str,
    max_part_bytes: u64,
) -> Result<Vec<OwnedPart>, MultipartRefusal> {
    let Ok(boundary) = multer::parse_boundary(content_type) else {
        return Err(MultipartRefusal::NotMultipart);
    };
    let constraints =
        multer::Constraints::new().size_limit(multer::SizeLimit::new().per_field(max_part_bytes));
    let stream = futures_util::stream::once(async move { Ok::<Bytes, std::io::Error>(body) });
    let mut multipart = multer::Multipart::with_constraints(stream, boundary, constraints);
    let mut parts = Vec::new();
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(error) => return Err(refusal_for(&error)),
        };
        if parts.len() == MAX_MULTIPART_PARTS {
            return Err(MultipartRefusal::TooLarge);
        }
        let name = field.name().unwrap_or_default().to_string();
        let file_name = field.file_name().and_then(base_file_name);
        let content_type = field.content_type().map(std::string::ToString::to_string);
        let bytes = match field.bytes().await {
            Ok(bytes) => bytes.to_vec(),
            Err(error) => return Err(refusal_for(&error)),
        };
        parts.push(OwnedPart {
            name,
            file_name,
            content_type,
            bytes,
        });
    }
    Ok(parts)
}

fn refusal_for(error: &multer::Error) -> MultipartRefusal {
    match error {
        multer::Error::FieldSizeExceeded { .. } | multer::Error::StreamSizeExceeded { .. } => {
            MultipartRefusal::TooLarge
        }
        multer::Error::NoMultipart | multer::Error::NoBoundary => MultipartRefusal::NotMultipart,
        _ => MultipartRefusal::Malformed,
    }
}

/// Moves parsed parts into the C layout generated code reads.
///
/// The values are leaked deliberately: this vector is handed to the handler as
/// a borrowed pointer and length, exactly like the raw body beside it, and it
/// is reclaimed by [`release_parts`] once the call that borrowed it returns.
///
/// # Safety
///
/// The returned parts borrow nothing; the caller owns them and must pass them
/// to [`release_parts`].
pub unsafe fn into_abi_parts(parts: Vec<OwnedPart>) -> Vec<MultipartPart> {
    unsafe {
        parts
            .into_iter()
            .map(|part| MultipartPart {
                name: crate::strings::alloc_string(&part.name),
                file_name: optional_string(part.file_name),
                content_type: optional_string(part.content_type),
                bytes: owned_bytes(part.bytes),
            })
            .collect()
    }
}

/// Frees everything [`into_abi_parts`] allocated.
///
/// # Safety
///
/// The parts must be the ones that function produced, and nothing may still
/// hold a pointer into them.
pub unsafe fn release_parts(parts: Vec<MultipartPart>) {
    unsafe {
        for mut part in parts {
            std::mem::replace(&mut part.name, TyrusString::null()).release();
            release_optional_string(part.file_name);
            release_optional_string(part.content_type);
            if !part.bytes.data.is_null() {
                libc::free(part.bytes.data.cast());
            }
        }
    }
}

unsafe fn optional_string(value: Option<String>) -> *const TyrusString {
    unsafe {
        let Some(value) = value else {
            return std::ptr::null();
        };
        let boxed = libc::malloc(std::mem::size_of::<TyrusString>()).cast::<TyrusString>();
        if boxed.is_null() {
            return std::ptr::null();
        }
        boxed.write(crate::strings::alloc_string(&value));
        boxed
    }
}

unsafe fn release_optional_string(value: *const TyrusString) {
    unsafe {
        if value.is_null() {
            return;
        }
        let owned = &mut *(value as *mut TyrusString);
        std::mem::replace(owned, TyrusString::null()).release();
        libc::free(value.cast_mut().cast());
    }
}

unsafe fn owned_bytes(bytes: Vec<u8>) -> TyrusArray {
    unsafe {
        if bytes.is_empty() {
            return TyrusArray {
                data: std::ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
        }
        let length = bytes.len();
        let data = libc::malloc(length).cast::<u8>();
        if data.is_null() {
            return TyrusArray {
                data: std::ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, length);
        TyrusArray {
            data,
            length,
            capacity: length,
        }
    }
}
