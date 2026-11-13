use super::panic::abort_with;
use directories::BaseDirs;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
use std::iter;
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};
use tyrus_string::TyrusString;
use zeroize::Zeroize;

const SEALED_HEADER: &str = "TYRUS_ENV_SEALED_V1";
const FINGERPRINT_PREFIX: &str = "fingerprint:";

pub const ENV_TYPE_STRING: u32 = 0;
pub const ENV_TYPE_BOOL: u32 = 1;
pub const ENV_TYPE_I32: u32 = 2;
pub const ENV_TYPE_I64: u32 = 3;
pub const ENV_TYPE_U32: u32 = 4;
pub const ENV_TYPE_U64: u32 = 5;
pub const ENV_TYPE_F32: u32 = 6;
pub const ENV_TYPE_F64: u32 = 7;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Parsed {
    Str,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
}

#[derive(Default)]
struct EnvState {
    raw: HashMap<String, String>,
    parsed: HashMap<String, Parsed>,
    sealed: bool,
}

static ENV: OnceLock<Mutex<EnvState>> = OnceLock::new();

fn state() -> &'static Mutex<EnvState> {
    ENV.get_or_init(Mutex::default)
}

fn parse_env_file(content: &str) -> HashMap<String, String> {
    let mut entries: Vec<(String, String, bool)> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('@') {
            continue;
        }
        let Some(eq_index) = line.find('=') else {
            continue;
        };
        let key = tyrus_validators::env_strip_export(&line[..eq_index]);
        if key.is_empty() || !seen.insert(key.to_string()) {
            continue;
        }
        let (value, interpolatable) = tyrus_validators::env_unquote(line[eq_index + 1..].trim());
        entries.push((key.to_string(), value, interpolatable));
    }
    entries
        .iter()
        .zip(tyrus_validators::env_resolve(&entries))
        .map(|((name, raw, _), resolved)| (name.clone(), resolved.unwrap_or_else(|_| raw.clone())))
        .collect()
}

pub(crate) fn lookup(name: &str) -> Option<String> {
    let state = state()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    resolve(name, &state)
}

fn resolve(name: &str, state: &EnvState) -> Option<String> {
    resolve_with(name, state, |name| std::env::var(name).ok())
}

fn resolve_with(
    name: &str,
    state: &EnvState,
    real_env: impl Fn(&str) -> Option<String>,
) -> Option<String> {
    if state.sealed {
        state.raw.get(name).cloned()
    } else {
        real_env(name).or_else(|| state.raw.get(name).cloned())
    }
}

fn parse_typed(value: &str, type_code: u32) -> Option<Parsed> {
    match type_code {
        ENV_TYPE_STRING => Some(Parsed::Str),
        ENV_TYPE_BOOL => match value {
            "true" => Some(Parsed::Bool(true)),
            "false" => Some(Parsed::Bool(false)),
            _ => None,
        },
        ENV_TYPE_I32 => value.parse::<i32>().ok().map(|v| Parsed::I64(v as i64)),
        ENV_TYPE_I64 => value.parse::<i64>().ok().map(Parsed::I64),
        ENV_TYPE_U32 => value.parse::<u32>().ok().map(|v| Parsed::U64(v as u64)),
        ENV_TYPE_U64 => value.parse::<u64>().ok().map(Parsed::U64),
        ENV_TYPE_F32 => value.parse::<f32>().ok().map(|v| Parsed::F64(v as f64)),
        ENV_TYPE_F64 => value.parse::<f64>().ok().map(Parsed::F64),
        _ => None,
    }
}

unsafe fn text_or_abort(value: TyrusString, what: &str) -> String {
    unsafe {
        if value.is_null() {
            abort_with(
                TyrusString::null(),
                &format!("tyrus runtime error: null {what}"),
            );
        }
        { value.as_str() }.to_owned()
    }
}

fn abort_runtime(message: impl AsRef<str>) -> ! {
    let message = message.as_ref();
    // SAFETY: `message` outlives the call, which never returns.
    let lent = unsafe { TyrusString::lend(message) }.unwrap_or_else(TyrusString::null);
    unsafe { abort_with(lent, "tyrus runtime error") }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_env_init(path: TyrusString) {
    unsafe {
        let path = text_or_abort(path, "env file path");
        let file_values = std::fs::read_to_string(&path)
            .map(|content| parse_env_file(&content))
            .unwrap_or_default();
        let mut state = state()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.raw = file_values;
        state.parsed.clear();
        state.sealed = false;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_env_init_sealed(
    project_name: TyrusString,
    fingerprint: TyrusString,
) {
    unsafe {
        let project_name = text_or_abort(project_name, "project name");
        let expected_fingerprint = text_or_abort(fingerprint, "sealed env fingerprint");
        let file_values = load_sealed_env(&project_name, &expected_fingerprint);
        let mut state = state()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.raw = file_values;
        state.parsed.clear();
        state.sealed = true;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_env_require(
    name: TyrusString,
    type_code: u32,
    message: TyrusString,
) {
    unsafe {
        let name = text_or_abort(name, "env variable name");
        let mut state = state()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let value = resolve(&name, &state);
        if let Some(resolved) = &value {
            state.raw.insert(name.clone(), resolved.clone());
        }
        let parsed = value
            .as_deref()
            .filter(|value| !value.is_empty() || type_code == ENV_TYPE_STRING)
            .and_then(|value| parse_typed(value, type_code));
        match parsed {
            Some(parsed) => {
                state.parsed.insert(name, parsed);
            }
            None => {
                drop(state);
                abort_with(message, "tyrus validation error")
            }
        }
    }
}

fn raw_value(name: TyrusString) -> String {
    let name = unsafe { text_or_abort(name, "env variable name") };
    let state = state()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    match state.raw.get(&name) {
        Some(value) => value.clone(),
        None => {
            drop(state);
            let message =
                format!("tyrus runtime error: environment variable `{name}` was not loaded");
            // SAFETY: `message` outlives the call, which never returns.
            let lent = unsafe { TyrusString::lend(&message) }.unwrap_or_else(TyrusString::null);
            unsafe { abort_with(lent, "tyrus runtime error") }
        }
    }
}

fn parsed_value(name: TyrusString) -> Parsed {
    let name_text = unsafe { text_or_abort(name, "env variable name") };
    let state = state()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    match state.parsed.get(&name_text) {
        Some(parsed) => *parsed,
        None => {
            drop(state);
            let message = format!(
                "tyrus runtime error: environment variable `{name_text}` was not validated at startup"
            );
            // SAFETY: `message` outlives the call, which never returns.
            let lent = unsafe { TyrusString::lend(&message) }.unwrap_or_else(TyrusString::null);
            unsafe { abort_with(lent, "tyrus runtime error") }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_env_get_str(name: TyrusString) -> TyrusString {
    let value = raw_value(name);
    crate::strings::alloc_string(&value)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_env_get_i64(name: TyrusString) -> i64 {
    match parsed_value(name) {
        Parsed::I64(value) => value,
        Parsed::U64(value) => value as i64,
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_env_get_u64(name: TyrusString) -> u64 {
    match parsed_value(name) {
        Parsed::U64(value) => value,
        Parsed::I64(value) => value as u64,
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_env_get_f64(name: TyrusString) -> f64 {
    match parsed_value(name) {
        Parsed::F64(value) => value,
        _ => 0.0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_env_get_bool(name: TyrusString) -> bool {
    matches!(parsed_value(name), Parsed::Bool(true))
}

fn load_sealed_env(project_name: &str, expected_fingerprint: &str) -> HashMap<String, String> {
    let sealed_path = sealed_env_path()
        .unwrap_or_else(|message| abort_runtime(format!("tyrus runtime error: {message}")));
    let sealed = std::fs::read(&sealed_path).unwrap_or_else(|e| {
        abort_runtime(format!(
            "tyrus runtime error: could not read sealed env file `{}`: {e}",
            sealed_path.display()
        ))
    });
    let (actual_fingerprint, encrypted) = parse_sealed_env(&sealed).unwrap_or_else(|message| {
        abort_runtime(format!(
            "tyrus runtime error: invalid sealed env file `{}`: {message}",
            sealed_path.display()
        ))
    });
    if actual_fingerprint != expected_fingerprint {
        abort_runtime("tyrus runtime error: sealed env fingerprint does not match this binary");
    }

    let key_path = key_path(project_name)
        .unwrap_or_else(|message| abort_runtime(format!("tyrus runtime error: {message}")));
    let mut key_text = std::fs::read_to_string(&key_path).unwrap_or_else(|e| {
        abort_runtime(format!(
            "tyrus runtime error: could not read seal key `{}`: {e}",
            key_path.display()
        ))
    });
    let identity = age::x25519::Identity::from_str(key_text.trim()).unwrap_or_else(|e| {
        abort_runtime(format!(
            "tyrus runtime error: could not parse seal key `{}`: {e}",
            key_path.display()
        ))
    });
    key_text.zeroize();
    let key_fingerprint = fingerprint(&identity.to_public().to_string());
    if key_fingerprint != expected_fingerprint {
        abort_runtime("tyrus runtime error: seal key fingerprint does not match this binary");
    }

    let plaintext = decrypt_sealed_env(encrypted, &identity).unwrap_or_else(|message| {
        abort_runtime(format!(
            "tyrus runtime error: could not decrypt sealed env: {message}"
        ))
    });
    let mut source = match String::from_utf8(plaintext) {
        Ok(source) => source,
        Err(e) => {
            let mut bytes = e.into_bytes();
            bytes.zeroize();
            abort_runtime("tyrus runtime error: sealed env is not valid UTF-8");
        }
    };
    let values = parse_env_file(&source);
    source.zeroize();
    values
}

fn sealed_env_path() -> Result<std::path::PathBuf, String> {
    let executable = std::env::current_exe()
        .map_err(|e| format!("could not resolve the current executable path: {e}"))?;
    let directory = executable.parent().ok_or_else(|| {
        format!(
            "could not resolve executable directory for `{}`",
            executable.display()
        )
    })?;
    Ok(directory.join(".env.sealed"))
}

fn key_path(project_name: &str) -> Result<std::path::PathBuf, String> {
    let base_dirs = BaseDirs::new()
        .ok_or_else(|| "could not resolve the user configuration directory".to_string())?;
    Ok(base_dirs
        .config_dir()
        .join("tyrus")
        .join("keys")
        .join(format!("{}.tyr.key", sanitize_project_name(project_name))))
}

fn sanitize_project_name(project_name: &str) -> String {
    let sanitized = project_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "project".to_string()
    } else {
        sanitized
    }
}

fn parse_sealed_env(sealed: &[u8]) -> Result<(String, &[u8]), String> {
    let separator = sealed
        .windows(2)
        .position(|window| window == b"\n\n")
        .ok_or_else(|| "missing sealed env header separator".to_string())?;
    let header = std::str::from_utf8(&sealed[..separator])
        .map_err(|_| "sealed env header is not valid UTF-8".to_string())?;
    let mut lines = header.lines();
    if lines.next() != Some(SEALED_HEADER) {
        return Err("unsupported sealed env header".to_string());
    }
    let fingerprint = lines
        .next()
        .and_then(|line| line.strip_prefix(FINGERPRINT_PREFIX))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "missing sealed env fingerprint".to_string())?;
    Ok((fingerprint.to_string(), &sealed[separator + 2..]))
}

fn decrypt_sealed_env(
    encrypted: &[u8],
    identity: &age::x25519::Identity,
) -> Result<Vec<u8>, String> {
    let decryptor =
        age::Decryptor::new(encrypted).map_err(|e| format!("invalid age payload: {e}"))?;
    let mut reader = decryptor
        .decrypt(iter::once(identity as &dyn age::Identity))
        .map_err(|e| format!("{e}"))?;
    let mut decrypted = Vec::new();
    reader
        .read_to_end(&mut decrypted)
        .map_err(|e| format!("could not read decrypted env: {e}"))?;
    Ok(decrypted)
}

fn fingerprint(recipient: &str) -> String {
    hex(&Sha256::digest(recipient.as_bytes()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_env_file_lines() {
        let values = parse_env_file(
            "# comment\n@type(u32)\nPORT=3000\nSECRET=key=with=equals\nEMPTY=\n\nbroken line\n",
        );
        assert_eq!(values.get("PORT").map(String::as_str), Some("3000"));
        assert_eq!(
            values.get("SECRET").map(String::as_str),
            Some("key=with=equals")
        );
        assert_eq!(values.get("EMPTY").map(String::as_str), Some(""));
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn first_declaration_wins_within_the_file() {
        let values = parse_env_file("PORT=3000\nPORT=4000\n");
        assert_eq!(values.get("PORT").map(String::as_str), Some("3000"));
    }

    #[test]
    fn strips_quotes_and_expands_interpolation() {
        let values = parse_env_file(
            "HOST=\"localhost\"\nPORT=3000\nURL=http://${HOST}:${PORT}/api\nLITERAL='${HOST}'\n",
        );
        assert_eq!(values.get("HOST").map(String::as_str), Some("localhost"));
        assert_eq!(
            values.get("URL").map(String::as_str),
            Some("http://localhost:3000/api")
        );
        assert_eq!(values.get("LITERAL").map(String::as_str), Some("${HOST}"));
    }

    #[test]
    fn real_environment_wins_over_file_values() {
        let mut file_values = HashMap::new();
        file_values.insert("TYRUS_TEST_ENV_PRECEDENCE".to_string(), "file".to_string());
        let mut state = EnvState {
            raw: file_values,
            parsed: HashMap::new(),
            sealed: false,
        };

        let real_present = |name: &str| -> Option<String> {
            (name == "TYRUS_TEST_ENV_PRECEDENCE").then(|| "real".to_string())
        };
        let real_absent = |_: &str| -> Option<String> { None };

        assert_eq!(
            resolve_with("TYRUS_TEST_ENV_PRECEDENCE", &state, real_present).as_deref(),
            Some("real")
        );
        assert_eq!(
            resolve_with("TYRUS_TEST_ENV_PRECEDENCE", &state, real_absent).as_deref(),
            Some("file")
        );
        assert_eq!(
            resolve_with("TYRUS_TEST_ENV_MISSING", &state, real_absent),
            None
        );
        state.sealed = true;
        assert_eq!(
            resolve_with("TYRUS_TEST_ENV_PRECEDENCE", &state, real_present).as_deref(),
            Some("file")
        );
    }

    #[test]
    fn sealed_env_header_carries_the_expected_fingerprint() {
        let sealed = b"TYRUS_ENV_SEALED_V1\nfingerprint:abc123\n\npayload";
        let (fingerprint, payload) = parse_sealed_env(sealed).unwrap();
        assert_eq!(fingerprint, "abc123");
        assert_eq!(payload, b"payload");
        assert!(parse_sealed_env(b"wrong\nfingerprint:abc\n\npayload").is_err());
    }

    #[test]
    fn fingerprint_is_stable_hex_sha256() {
        let fingerprint = super::fingerprint("age1example");
        assert_eq!(fingerprint.len(), 64);
        assert!(
            fingerprint
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        );
        assert_eq!(fingerprint, super::fingerprint("age1example"));
    }

    #[test]
    fn parses_typed_values_with_range_checks() {
        assert_eq!(parse_typed("3000", ENV_TYPE_U32), Some(Parsed::U64(3000)));
        assert_eq!(parse_typed("4294967296", ENV_TYPE_U32), None);
        assert_eq!(
            parse_typed("4294967296", ENV_TYPE_U64),
            Some(Parsed::U64(4294967296))
        );
        assert_eq!(parse_typed("-1", ENV_TYPE_U32), None);
        assert_eq!(parse_typed("-5", ENV_TYPE_I32), Some(Parsed::I64(-5)));
        assert_eq!(parse_typed("2147483648", ENV_TYPE_I32), None);
        assert_eq!(parse_typed("true", ENV_TYPE_BOOL), Some(Parsed::Bool(true)));
        assert_eq!(parse_typed("yes", ENV_TYPE_BOOL), None);
        assert_eq!(parse_typed("0.5", ENV_TYPE_F64), Some(Parsed::F64(0.5)));
        assert_eq!(parse_typed("abc", ENV_TYPE_F64), None);
        assert_eq!(parse_typed("anything", ENV_TYPE_STRING), Some(Parsed::Str));
    }

    #[test]
    fn strips_export_prefix() {
        let values = parse_env_file("export HOST=localhost\nexport PORT=3000\n");
        assert_eq!(values.get("HOST").map(String::as_str), Some("localhost"));
        assert_eq!(values.get("PORT").map(String::as_str), Some("3000"));
    }
}
