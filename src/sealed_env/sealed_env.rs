use std::fs;
use std::io::Write;
use std::iter;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use age::secrecy::ExposeSecret;
use directories::BaseDirs;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

const SEALED_HEADER: &str = "TYRUS_ENV_SEALED_V1";
const FINGERPRINT_PREFIX: &str = "fingerprint:";

pub struct SealedEnvBuild {
    pub fingerprint: String,
    pub key_path: PathBuf,
    pub public_key_path: PathBuf,
    pub sealed_path: PathBuf,
    pub generated_key: bool,
}

pub fn key_paths(project_name: &str) -> Result<(PathBuf, PathBuf), String> {
    let base_dirs = BaseDirs::new()
        .ok_or_else(|| "could not resolve the user configuration directory".to_string())?;
    let key_dir = base_dirs.config_dir().join("tyrus").join("keys");
    let key_name = sanitize_project_name(project_name);
    Ok((
        key_dir.join(format!("{key_name}.tyr.key")),
        key_dir.join(format!("{key_name}.tyr.pub")),
    ))
}

pub fn prepare(
    project_name: &str,
    env_source: &str,
    target_dir: &Path,
    rotate_key: bool,
) -> Result<SealedEnvBuild, String> {
    let (key_path, public_key_path) = key_paths(project_name)?;
    let (identity, generated_key) = load_or_generate_identity(&key_path, rotate_key)?;
    let recipient = identity.to_public();
    let recipient_text = recipient.to_string();
    let fingerprint = fingerprint(&recipient_text);
    write_public_key(&public_key_path, &recipient_text, &fingerprint)?;
    let sealed = seal_env(env_source.as_bytes(), &recipient, &fingerprint)?;
    let sealed_path = target_dir.join(".env.sealed");
    fs::write(&sealed_path, sealed).map_err(|e| {
        format!(
            "could not write sealed env file `{}`: {e}",
            sealed_path.display()
        )
    })?;
    Ok(SealedEnvBuild {
        fingerprint,
        key_path,
        public_key_path,
        sealed_path,
        generated_key,
    })
}

pub fn fingerprint(recipient: &str) -> String {
    hex(&Sha256::digest(recipient.as_bytes()))
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

fn load_or_generate_identity(
    key_path: &Path,
    rotate_key: bool,
) -> Result<(age::x25519::Identity, bool), String> {
    if key_path.exists() && !rotate_key {
        let mut content = fs::read_to_string(key_path)
            .map_err(|e| format!("could not read seal key `{}`: {e}", key_path.display()))?;
        let identity = age::x25519::Identity::from_str(content.trim()).map_err(|e| {
            format!(
                "could not parse seal key `{}` as an age identity: {e}",
                key_path.display()
            )
        })?;
        content.zeroize();
        return Ok((identity, false));
    }

    let identity = age::x25519::Identity::generate();
    let secret = identity.to_string();
    let parent = key_path
        .parent()
        .ok_or_else(|| format!("invalid seal key path `{}`", key_path.display()))?;
    fs::create_dir_all(parent).map_err(|e| {
        format!(
            "could not create seal key directory `{}`: {e}",
            parent.display()
        )
    })?;
    fs::write(key_path, format!("{}\n", secret.expose_secret()))
        .map_err(|e| format!("could not write seal key `{}`: {e}", key_path.display()))?;
    set_owner_only_permissions(key_path)?;
    Ok((identity, true))
}

fn write_public_key(path: &Path, recipient: &str, fingerprint: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid public seal key path `{}`", path.display()))?;
    fs::create_dir_all(parent).map_err(|e| {
        format!(
            "could not create public seal key directory `{}`: {e}",
            parent.display()
        )
    })?;
    fs::write(
        path,
        format!("# tyrus sealed env public recipient\n# fingerprint: {fingerprint}\n{recipient}\n"),
    )
    .map_err(|e| format!("could not write public seal key `{}`: {e}", path.display()))
}

fn seal_env(
    plaintext: &[u8],
    recipient: &age::x25519::Recipient,
    fingerprint: &str,
) -> Result<Vec<u8>, String> {
    let encryptor = age::Encryptor::with_recipients(iter::once(recipient as &dyn age::Recipient))
        .map_err(|e| format!("could not create age encryptor: {e}"))?;
    let mut encrypted = Vec::new();
    {
        let mut writer = encryptor
            .wrap_output(&mut encrypted)
            .map_err(|e| format!("could not initialize age encryption: {e}"))?;
        writer
            .write_all(plaintext)
            .map_err(|e| format!("could not encrypt env source: {e}"))?;
        writer
            .finish()
            .map_err(|e| format!("could not finish age encryption: {e}"))?;
    }
    let mut sealed = format!("{SEALED_HEADER}\n{FINGERPRINT_PREFIX}{fingerprint}\n\n").into_bytes();
    sealed.extend_from_slice(&encrypted);
    Ok(sealed)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(unix)]
fn set_owner_only_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .map_err(|e| format!("could not inspect seal key `{}`: {e}", path.display()))?
        .permissions();
    permissions.set_mode(0o600);
    fs::set_permissions(path, permissions).map_err(|e| {
        format!(
            "could not restrict seal key `{}` permissions: {e}",
            path.display()
        )
    })
}

#[cfg(not(unix))]
fn set_owner_only_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprints_are_stable_hex_sha256() {
        let fingerprint = fingerprint("age1example");
        assert_eq!(fingerprint.len(), 64);
        assert!(
            fingerprint
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        );
        assert_eq!(fingerprint, super::fingerprint("age1example"));
    }
}
