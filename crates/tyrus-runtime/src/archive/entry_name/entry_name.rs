/// Longest single path component accepted in an entry name, in bytes. Every
/// mainstream file system caps a component at 255, so a longer one would fail
/// on extraction anyway, and failing here names the entry.
const MAX_COMPONENT_BYTES: usize = 255;

/// Longest whole entry name accepted, in bytes.
const MAX_NAME_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntryNameError {
    /// The name would resolve outside the extraction root.
    Escape,
    /// The name is malformed or would not be portable across platforms.
    Invalid,
}

/// Splits a raw entry name into the relative components it would create,
/// after dropping `strip` leading components.
///
/// Returns `Ok(None)` when stripping consumes the whole name, which callers
/// skip. Every component of an `Ok(Some(..))` is non-empty, is neither `.` nor
/// `..`, and contains no `/`, `\`, `:` or NUL, so joining the components under
/// a root can never leave that root.
pub(crate) fn normalize_entry_name(
    raw: &str,
    strip: u32,
) -> Result<Option<Vec<&str>>, EntryNameError> {
    if raw.is_empty() || raw.len() > MAX_NAME_BYTES || raw.contains('\0') {
        return Err(EntryNameError::Invalid);
    }
    if raw.starts_with('/') || raw.starts_with('\\') {
        return Err(EntryNameError::Escape);
    }
    if raw.contains('\\') {
        return Err(EntryNameError::Invalid);
    }
    if has_drive_prefix(raw) {
        return Err(EntryNameError::Escape);
    }
    let mut components = Vec::new();
    for component in raw.split('/') {
        match component {
            "" | "." => continue,
            ".." => return Err(EntryNameError::Escape),
            _ => {
                if !is_portable_component(component) {
                    return Err(EntryNameError::Invalid);
                }
                components.push(component);
            }
        }
    }
    let strip = usize::try_from(strip).unwrap_or(usize::MAX);
    if strip >= components.len() {
        return Ok(None);
    }
    components.drain(..strip);
    Ok(Some(components))
}

fn has_drive_prefix(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

/// A component that extracts to the same name on every supported platform.
/// Windows silently strips a trailing dot or space and reserves device names,
/// so an entry that relies on either would extract to a different file there.
fn is_portable_component(component: &str) -> bool {
    if component.len() > MAX_COMPONENT_BYTES {
        return false;
    }
    if component.bytes().any(|byte| {
        byte == b':' || byte < 0x20 || matches!(byte, b'<' | b'>' | b'"' | b'|' | b'?' | b'*')
    }) {
        return false;
    }
    if component.ends_with('.') || component.ends_with(' ') {
        return false;
    }
    !is_reserved_device_name(component)
}

fn is_reserved_device_name(component: &str) -> bool {
    let stem = component.split('.').next().unwrap_or(component);
    let stem = stem.trim_end_matches(' ');
    let upper = stem.to_ascii_uppercase();
    match upper.as_str() {
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$" => true,
        _ => {
            let bytes = upper.as_bytes();
            bytes.len() == 4
                && (upper.starts_with("COM") || upper.starts_with("LPT"))
                && bytes[3].is_ascii_digit()
                && bytes[3] != b'0'
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EntryNameError, normalize_entry_name};

    fn parts(raw: &str, strip: u32) -> Result<Option<Vec<&str>>, EntryNameError> {
        normalize_entry_name(raw, strip)
    }

    #[test]
    fn accepts_plain_relative_names() {
        assert_eq!(parts("a/b.txt", 0), Ok(Some(vec!["a", "b.txt"])));
        assert_eq!(parts("dir/", 0), Ok(Some(vec!["dir"])));
        assert_eq!(parts("./a//b", 0), Ok(Some(vec!["a", "b"])));
        assert_eq!(parts("relatório.pdf", 0), Ok(Some(vec!["relatório.pdf"])));
    }

    #[test]
    fn rejects_every_form_of_escape() {
        for raw in [
            "../a",
            "a/../../b",
            "a/..",
            "/etc/passwd",
            "\\windows",
            "C:/Windows",
            "c:evil",
        ] {
            assert_eq!(parts(raw, 0), Err(EntryNameError::Escape), "{raw}");
        }
    }

    #[test]
    fn rejects_names_that_are_not_portable() {
        for raw in [
            "",
            "a\\b",
            "a\0b",
            "file:stream",
            "CON",
            "con.txt",
            "dir/LPT1",
            "com9.log",
            "trailing.",
            "trailing ",
            "question?",
            "tab\tname",
        ] {
            assert_eq!(parts(raw, 0), Err(EntryNameError::Invalid), "{raw:?}");
        }
        assert!(parts(&"a".repeat(256), 0).is_err());
        assert!(parts(&"a/".repeat(2049), 0).is_err());
    }

    #[test]
    fn keeps_names_that_only_resemble_devices() {
        assert_eq!(parts("CONSOLE", 0), Ok(Some(vec!["CONSOLE"])));
        assert_eq!(parts("COM0", 0), Ok(Some(vec!["COM0"])));
        assert_eq!(parts("com10", 0), Ok(Some(vec!["com10"])));
    }

    #[test]
    fn strips_leading_components_before_returning() {
        assert_eq!(parts("tool-v2/bin/tool", 1), Ok(Some(vec!["bin", "tool"])));
        assert_eq!(parts("tool-v2/", 1), Ok(None));
        assert_eq!(parts("a/b", 5), Ok(None));
    }

    #[test]
    fn strip_never_hides_an_escape() {
        assert_eq!(parts("root/../../x", 1), Err(EntryNameError::Escape));
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::normalize_entry_name;

    const ALPHABET: [u8; 6] = [b'a', b'.', b'/', b'\\', b':', b'C'];

    #[kani::proof]
    #[kani::unwind(6)]
    fn an_accepted_name_never_leaves_the_root() {
        let length: usize = kani::any();
        kani::assume(length <= 4);
        let mut bytes = [0u8; 4];
        for byte in bytes.iter_mut().take(length) {
            let index: usize = kani::any();
            kani::assume(index < ALPHABET.len());
            *byte = ALPHABET[index];
        }
        let raw = core::str::from_utf8(&bytes[..length]).unwrap();
        let strip: u32 = kani::any();
        kani::assume(strip <= 2);
        if let Ok(Some(components)) = normalize_entry_name(raw, strip) {
            assert!(!components.is_empty());
            for component in components {
                assert!(!component.is_empty());
                assert!(component != "." && component != "..");
                assert!(!component.contains('/'));
                assert!(!component.contains('\\'));
                assert!(!component.contains(':'));
            }
        }
    }
}
