use super::ServerMigrationBackend;
use sqlx::any::AnyConnectOptions;

pub(super) fn configure_server_url(
    backend: ServerMigrationBackend,
    options: &mut AnyConnectOptions,
    ca_file: Option<&str>,
    cert_file: Option<&str>,
    key_file: Option<&str>,
) -> Result<(), String> {
    let url = &mut options.database_url;
    if ca_file.is_none() && cert_file.is_none() && key_file.is_none() {
        return Ok(());
    }

    let (mode_key, verified_mode, ca_key, cert_key, key_key) = match backend {
        ServerMigrationBackend::Postgresql => {
            ("sslmode", "verify-full", "sslrootcert", "sslcert", "sslkey")
        }
        ServerMigrationBackend::Mysql => (
            "ssl-mode",
            "VERIFY_IDENTITY",
            "ssl-ca",
            "ssl-cert",
            "ssl-key",
        ),
    };
    let modes = url
        .query_pairs()
        .filter(|(key, _)| key.eq_ignore_ascii_case(mode_key))
        .map(|(_, value)| value.into_owned())
        .collect::<Vec<_>>();
    match modes.as_slice() {
        [] => {
            url.query_pairs_mut().append_pair(mode_key, verified_mode);
        }
        [mode] if mode.eq_ignore_ascii_case(verified_mode) => {}
        _ => {
            return Err(
                "configured server database TLS requires certificate and hostname verification"
                    .to_string(),
            );
        }
    }
    if let Some(ca_file) = ca_file {
        if url
            .query_pairs()
            .any(|(key, _)| key.eq_ignore_ascii_case(ca_key))
        {
            return Err(
                "database URL and database.tls.caFile cannot both set the CA file".to_string(),
            );
        }
        url.query_pairs_mut().append_pair(ca_key, ca_file);
    }
    if url
        .query_pairs()
        .any(|(key, _)| key.eq_ignore_ascii_case(cert_key) || key.eq_ignore_ascii_case(key_key))
    {
        return Err(
            "database URL and database.tls cannot both set the client certificate".to_string(),
        );
    }
    if let (Some(cert_file), Some(key_file)) = (cert_file, key_file) {
        url.query_pairs_mut()
            .append_pair(cert_key, cert_file)
            .append_pair(key_key, key_file);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_migration_tls_is_optional_and_preserves_url_modes() {
        for (backend, input) in [
            (
                ServerMigrationBackend::Postgresql,
                "postgresql://localhost/app?sslmode=disable",
            ),
            (
                ServerMigrationBackend::Mysql,
                "mysql://localhost/app?ssl-mode=DISABLED",
            ),
        ] {
            let mut options = input.parse::<AnyConnectOptions>().unwrap();
            let before = options.database_url.clone();
            configure_server_url(backend, &mut options, None, None, None).unwrap();
            assert_eq!(options.database_url, before);
        }
    }

    #[test]
    fn configured_server_tls_requires_verified_modes_and_adds_ca() {
        let mut options = "postgresql://localhost/app"
            .parse::<AnyConnectOptions>()
            .unwrap();
        configure_server_url(
            ServerMigrationBackend::Postgresql,
            &mut options,
            Some("certs/ca.pem"),
            None,
            None,
        )
        .unwrap();
        assert!(
            options
                .database_url
                .as_str()
                .contains("sslmode=verify-full")
        );
        assert!(
            options
                .database_url
                .as_str()
                .contains("sslrootcert=certs%2Fca.pem")
        );

        let mut weak_options = "mysql://localhost/app?ssl-mode=DISABLED"
            .parse::<AnyConnectOptions>()
            .unwrap();
        assert!(
            configure_server_url(
                ServerMigrationBackend::Mysql,
                &mut weak_options,
                Some("certs/ca.pem"),
                None,
                None,
            )
            .is_err()
        );
    }
}
