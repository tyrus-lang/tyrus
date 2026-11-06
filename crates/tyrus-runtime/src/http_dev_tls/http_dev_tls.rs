use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rcgen::{
    BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
    KeyUsagePurpose,
};

static DEVELOPMENT_HTTPS: AtomicBool = AtomicBool::new(false);

const CA_CERT: &str = "ca.cert.pem";
const CA_KEY: &str = "ca.key.pem";
const LEAF_CERT: &str = "localhost.cert.pem";
const LEAF_KEY: &str = "localhost.key.pem";

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_runtime_configure_development_https(enabled: i32) {
    DEVELOPMENT_HTTPS.store(enabled != 0, Ordering::Release);
}

fn certificate_directory() -> Option<PathBuf> {
    if let Some(directory) = std::env::var_os("TYRUS_DEV_HTTPS_DIR") {
        return Some(PathBuf::from(directory));
    }
    directories::BaseDirs::new().map(|dirs| dirs.config_dir().join("tyrus").join("dev-https"))
}

pub(crate) fn development_acceptor() -> Result<Option<Arc<tokio_rustls::TlsAcceptor>>, &'static str>
{
    if !DEVELOPMENT_HTTPS.load(Ordering::Acquire) {
        return Ok(None);
    }
    let environment = crate::env::lookup("TYRUS_ENV");
    if environment.as_deref() != Some("development") {
        return Ok(None);
    }
    let directory = certificate_directory()
        .ok_or("the development certificate directory could not be located")?;
    acceptor_for(&directory).map(Some)
}

fn authority_params() -> CertificateParams {
    let mut params = CertificateParams::default();
    params
        .distinguished_name
        .push(DnType::CommonName, "Tyrus Development CA");
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    params
}

fn write_private(path: &Path, contents: &str) -> Result<(), &'static str> {
    std::fs::write(path, contents)
        .map_err(|_| "a development certificate key could not be written")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|_| "a development certificate key could not be protected")?;
    }
    Ok(())
}

pub(crate) fn acceptor_for(
    directory: &Path,
) -> Result<Arc<tokio_rustls::TlsAcceptor>, &'static str> {
    std::fs::create_dir_all(directory)
        .map_err(|_| "the development certificate directory could not be created")?;
    let ca_cert = directory.join(CA_CERT);
    let ca_key = directory.join(CA_KEY);
    let created = !(ca_cert.is_file() && ca_key.is_file());
    let authority_key = if created {
        let key = KeyPair::generate()
            .map_err(|_| "the development authority key could not be generated")?;
        let certificate = authority_params()
            .self_signed(&key)
            .map_err(|_| "the development authority certificate could not be created")?;
        write_private(&ca_key, &key.serialize_pem())?;
        std::fs::write(&ca_cert, certificate.pem())
            .map_err(|_| "the development authority certificate could not be written")?;
        key
    } else {
        let pem = std::fs::read_to_string(&ca_key)
            .map_err(|_| "the development authority key could not be read")?;
        KeyPair::from_pem(&pem)
            .map_err(|_| "the development authority key is not a readable key")?
    };
    let issuer = Issuer::new(authority_params(), authority_key);

    let leaf_key = KeyPair::generate()
        .map_err(|_| "the development certificate key could not be generated")?;
    let mut leaf = CertificateParams::new(vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "::1".to_string(),
    ])
    .map_err(|_| "the development certificate names are invalid")?;
    leaf.distinguished_name
        .push(DnType::CommonName, "localhost");
    leaf.is_ca = IsCa::ExplicitNoCa;
    leaf.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    let leaf_certificate = leaf
        .signed_by(&leaf_key, &issuer)
        .map_err(|_| "the development certificate could not be signed")?;
    let leaf_cert = directory.join(LEAF_CERT);
    let leaf_key_path = directory.join(LEAF_KEY);
    std::fs::write(&leaf_cert, leaf_certificate.pem())
        .map_err(|_| "the development certificate could not be written")?;
    write_private(&leaf_key_path, &leaf_key.serialize_pem())?;

    if created {
        eprintln!(
            "note: created a local development certificate authority at {}\n      trust it once in your browser or system to open https://localhost without a warning",
            ca_cert.display()
        );
    }
    crate::http_tls::acceptor(
        &leaf_cert.to_string_lossy(),
        &leaf_key_path.to_string_lossy(),
    )
    .map_err(|error| error.message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_directory(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "tyrus-dev-https-{name}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
        ))
    }

    #[test]
    fn the_authority_is_created_once_and_reused() {
        let directory = temp_directory("reuse");
        acceptor_for(&directory).expect("the first start creates the authority");
        let authority = std::fs::read_to_string(directory.join(CA_CERT)).unwrap();
        let first_leaf = std::fs::read_to_string(directory.join(LEAF_CERT)).unwrap();

        acceptor_for(&directory).expect("a later start reuses the authority");
        assert_eq!(
            std::fs::read_to_string(directory.join(CA_CERT)).unwrap(),
            authority,
            "the authority a developer trusted must not change"
        );
        assert_ne!(
            std::fs::read_to_string(directory.join(LEAF_CERT)).unwrap(),
            first_leaf,
            "the localhost certificate is issued again on every start"
        );
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[cfg(unix)]
    #[test]
    fn the_private_keys_are_readable_only_by_their_owner() {
        use std::os::unix::fs::PermissionsExt;
        let directory = temp_directory("permissions");
        acceptor_for(&directory).unwrap();
        for key in [CA_KEY, LEAF_KEY] {
            let mode = std::fs::metadata(directory.join(key))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600, "{key}");
        }
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn development_https_is_off_unless_the_project_enables_it() {
        DEVELOPMENT_HTTPS.store(false, Ordering::Release);
        assert!(development_acceptor().unwrap().is_none());
    }
}
