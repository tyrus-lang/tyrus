use std::io::ErrorKind;
use std::path::Path;
use std::sync::Arc;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

pub(crate) struct TlsSetupError {
    pub(crate) message: &'static str,
}

impl TlsSetupError {
    fn new(message: &'static str) -> Self {
        Self { message }
    }
}

pub(crate) fn acceptor(
    cert_path: &str,
    key_path: &str,
) -> Result<Arc<tokio_rustls::TlsAcceptor>, TlsSetupError> {
    let certificates = read_certificates(cert_path)?;
    let key = read_private_key(key_path)?;
    let mut config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certificates, key)
        .map_err(|_| {
            TlsSetupError::new(
                "the private key does not match the certificate, or the certificate chain is out of order",
            )
        })?;
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(Arc::new(tokio_rustls::TlsAcceptor::from(Arc::new(config))))
}

fn read_certificates(path: &str) -> Result<Vec<CertificateDer<'static>>, TlsSetupError> {
    if let Err(error) = std::fs::metadata(Path::new(path)) {
        return Err(TlsSetupError::new(file_failure_message(error.kind(), true)));
    }
    let certificates = CertificateDer::pem_file_iter(path)
        .map_err(|_| TlsSetupError::new("the certificate file is not readable PEM"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| TlsSetupError::new("the certificate file is not readable PEM"))?;
    if certificates.is_empty() {
        return Err(TlsSetupError::new(
            "the certificate file holds no certificate",
        ));
    }
    Ok(certificates)
}

fn read_private_key(path: &str) -> Result<PrivateKeyDer<'static>, TlsSetupError> {
    if let Err(error) = std::fs::metadata(Path::new(path)) {
        return Err(TlsSetupError::new(file_failure_message(
            error.kind(),
            false,
        )));
    }
    PrivateKeyDer::from_pem_file(path)
        .map_err(|_| TlsSetupError::new("the key file holds no usable private key"))
}

fn file_failure_message(kind: ErrorKind, certificate: bool) -> &'static str {
    match (kind, certificate) {
        (ErrorKind::NotFound, true) => "the certificate file does not exist",
        (ErrorKind::NotFound, false) => "the key file does not exist",
        (ErrorKind::PermissionDenied, true) => "the certificate file cannot be read",
        (ErrorKind::PermissionDenied, false) => "the key file cannot be read",
        (_, true) => "the certificate file could not be opened",
        (_, false) => "the key file could not be opened",
    }
}
