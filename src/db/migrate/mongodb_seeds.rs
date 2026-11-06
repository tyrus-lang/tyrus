use super::{Seed, run_seed_program, server_connection_string, verify_seed_history};
use mongodb::bson::{Bson, DateTime, Document, doc};
use mongodb::options::{ClientOptions, IndexOptions, Tls, TlsOptions};
use mongodb::{Client, Database, IndexModel};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::time::timeout;
use tyrus_core::db::DatabaseConfig;
use zeroize::Zeroizing;

const HISTORY_COLLECTION: &str = "_tyrus_seeds";
const MAX_TLS_FILE_BYTES: u64 = 1024 * 1024;

struct MongoSeedDatabase {
    database: Database,
    _client_identity_file: Option<NamedTempFile>,
}

pub(super) fn apply(
    project_root: &Path,
    config: &DatabaseConfig,
    seeds: &[Seed],
    seed: &Seed,
) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "could not initialize MongoDB seed history".to_string())?;
    runtime.block_on(async {
        let context = open(project_root, config).await?;
        let collection = context.database.collection::<Document>(HISTORY_COLLECTION);
        let index = IndexModel::builder()
            .keys(doc! { "name": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build();
        collection
            .create_index(index)
            .await
            .map_err(|_| "could not prepare MongoDB seed history".to_string())?;

        let applied = read_history(&context.database).await?;
        verify_seed_history(seeds, &applied)?;
        if let Some(checksum) = applied.get(&seed.name) {
            if checksum == &seed.checksum {
                return Err(format!("seed `{}` is already applied", seed.name));
            }
            if checksum.starts_with("started:") {
                return Err(format!(
                    "seed `{}` is in an incomplete state; inspect the database before retrying",
                    seed.name
                ));
            }
            return Err(format!(
                "applied seed `{}` has changed; restore its original file and add a new seed",
                seed.name
            ));
        }

        collection
            .insert_one(doc! {
                "name": &seed.name,
                "checksum": format!("started:{}", seed.checksum),
                "state": "started",
                "appliedAt": Bson::Null,
            })
            .await
            .map_err(|_| {
                "could not reserve MongoDB seed history; another seed operation may be running"
                    .to_string()
            })?;

        run_seed_program(project_root, &seed.name)?;

        let finalized = collection
            .update_one(
                doc! {
                    "name": &seed.name,
                    "checksum": format!("started:{}", seed.checksum),
                    "state": "started",
                },
                doc! {
                    "$set": {
                        "checksum": &seed.checksum,
                        "state": "applied",
                        "appliedAt": DateTime::now(),
                    }
                },
            )
            .await
            .map_err(|_| "could not finalize MongoDB seed history".to_string())?;
        if finalized.modified_count != 1 {
            return Err(
                "MongoDB seed history changed while the seed program was running".to_string(),
            );
        }
        println!("applied: {}", seed.name);
        Ok(())
    })
}

pub(super) fn status(
    project_root: &Path,
    config: &DatabaseConfig,
    seeds: &[Seed],
    selected_seed: Option<&str>,
) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "could not initialize MongoDB seeds".to_string())?;
    runtime.block_on(async {
        let context = open(project_root, config).await?;
        let applied = read_history(&context.database).await?;
        verify_seed_history(seeds, &applied)?;
        let displayed = seeds
            .iter()
            .filter(|seed| selected_seed.is_none_or(|name| seed.name == name))
            .collect::<Vec<_>>();
        for seed in &displayed {
            let state = if applied.contains_key(&seed.name) {
                "applied"
            } else {
                "pending"
            };
            println!("{state}: {}", seed.name);
        }
        let applied_count = displayed
            .iter()
            .filter(|seed| applied.contains_key(&seed.name))
            .count();
        println!(
            "{} applied, {} pending",
            applied_count,
            displayed.len() - applied_count
        );
        Ok(())
    })
}

async fn open(project_root: &Path, config: &DatabaseConfig) -> Result<MongoSeedDatabase, String> {
    let connection_string = Zeroizing::new(server_connection_string(project_root, config)?);
    let runtime_target: serde_json::Value =
        serde_json::from_str(&config.runtime_target(project_root)?)
            .map_err(|_| "could not resolve MongoDB settings".to_string())?;
    let tls = runtime_target.get("tls");
    let ca_file = tls
        .and_then(|value| value.get("caFile"))
        .and_then(serde_json::Value::as_str);
    let cert_file = tls
        .and_then(|value| value.get("certFile"))
        .and_then(serde_json::Value::as_str);
    let key_file = tls
        .and_then(|value| value.get("keyFile"))
        .and_then(serde_json::Value::as_str);
    validate_tls_file(ca_file)?;
    validate_tls_file(cert_file)?;
    validate_tls_file(key_file)?;

    let connect_timeout = config
        .pool
        .as_ref()
        .and_then(|pool| pool.connect_timeout_ms)
        .unwrap_or(10_000);
    let mut options = timeout(
        Duration::from_millis(connect_timeout),
        ClientOptions::parse(&*connection_string),
    )
    .await
    .map_err(|_| "MongoDB connection timed out".to_string())?
    .map_err(|_| "could not parse MongoDB connection settings".to_string())?;
    let database_name = options
        .default_database
        .clone()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "MongoDB connection URL must include a database name".to_string())?;
    let pool = config.pool.as_ref();
    options.max_pool_size = Some(pool.and_then(|value| value.max_connections).unwrap_or(10));
    options.min_pool_size = Some(0);
    options.connect_timeout = Some(Duration::from_millis(connect_timeout));
    options.server_selection_timeout = Some(Duration::from_millis(connect_timeout));
    options.max_idle_time = Some(Duration::from_millis(
        pool.and_then(|value| value.idle_timeout_ms)
            .unwrap_or(30_000),
    ));

    let client_identity_file = configure_tls(&mut options, ca_file, cert_file, key_file)?;
    let client = Client::with_options(options)
        .map_err(|_| "could not initialize MongoDB client".to_string())?;
    let database = client.database(&database_name);
    timeout(
        Duration::from_millis(connect_timeout),
        database.run_command(doc! { "ping": 1 }),
    )
    .await
    .map_err(|_| "MongoDB connection timed out".to_string())?
    .map_err(|_| "could not connect to MongoDB".to_string())?;
    Ok(MongoSeedDatabase {
        database,
        _client_identity_file: client_identity_file,
    })
}

async fn read_history(database: &Database) -> Result<BTreeMap<String, String>, String> {
    let names = database
        .list_collection_names()
        .await
        .map_err(|_| "could not inspect MongoDB seed history".to_string())?;
    if !names.iter().any(|name| name == HISTORY_COLLECTION) {
        return Ok(BTreeMap::new());
    }
    let collection = database.collection::<Document>(HISTORY_COLLECTION);
    let mut cursor = collection
        .find(doc! {})
        .await
        .map_err(|_| "could not read MongoDB seed history".to_string())?;
    let mut applied = BTreeMap::new();
    while cursor
        .advance()
        .await
        .map_err(|_| "could not read MongoDB seed history".to_string())?
    {
        let row = cursor
            .deserialize_current()
            .map_err(|_| "invalid MongoDB seed history document".to_string())?;
        let name = row
            .get_str("name")
            .map_err(|_| "invalid MongoDB seed history name".to_string())?
            .to_string();
        let checksum = row
            .get_str("checksum")
            .map_err(|_| "invalid MongoDB seed history checksum".to_string())?
            .to_string();
        let state = row
            .get_str("state")
            .map_err(|_| "invalid MongoDB seed history state".to_string())?;
        if state != "applied" && state != "started" {
            return Err(format!(
                "seed `{name}` has an invalid MongoDB history state"
            ));
        }
        let checksum = if state == "started" {
            format!("started:{checksum}")
        } else {
            checksum
        };
        if applied.insert(name, checksum).is_some() {
            return Err("MongoDB seed history contains duplicate names".to_string());
        }
    }
    Ok(applied)
}

fn validate_tls_file(path: Option<&str>) -> Result<(), String> {
    let Some(path) = path else {
        return Ok(());
    };
    let metadata = fs::metadata(path)
        .map_err(|_| "configured MongoDB TLS path is not a readable file".to_string())?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_TLS_FILE_BYTES {
        return Err("configured MongoDB TLS path is not a readable file".to_string());
    }
    Ok(())
}

fn configure_tls(
    options: &mut ClientOptions,
    ca_file: Option<&str>,
    cert_file: Option<&str>,
    key_file: Option<&str>,
) -> Result<Option<NamedTempFile>, String> {
    if ca_file.is_none() && cert_file.is_none() && key_file.is_none() {
        if let Some(Tls::Enabled(tls)) = &options.tls
            && (tls.cert_key_file_path.is_some() || tls.allow_invalid_certificates == Some(true))
        {
            return Err("MongoDB TLS settings conflict with database.tls".to_string());
        }
        return Ok(None);
    }
    let mut tls = match options.tls.take() {
        Some(Tls::Disabled) => {
            return Err(
                "MongoDB connection URL disables TLS while database.tls is configured".to_string(),
            );
        }
        Some(Tls::Enabled(tls)) => tls,
        None => TlsOptions::default(),
    };
    if tls.cert_key_file_path.is_some()
        || tls.allow_invalid_certificates == Some(true)
        || tls.ca_file_path.is_some()
    {
        return Err("MongoDB TLS settings conflict with database.tls".to_string());
    }
    if let Some(path) = ca_file {
        tls.ca_file_path = Some(path.into());
    }
    let identity = match (cert_file, key_file) {
        (Some(cert), Some(key)) => {
            let certificate = Zeroizing::new(
                fs::read(cert)
                    .map_err(|_| "could not read MongoDB client certificate".to_string())?,
            );
            let key = Zeroizing::new(
                fs::read(key).map_err(|_| "could not read MongoDB client key".to_string())?,
            );
            let mut combined = NamedTempFile::new()
                .map_err(|_| "could not prepare MongoDB client identity".to_string())?;
            combined
                .write_all(&certificate)
                .and_then(|()| combined.write_all(b"\n"))
                .and_then(|()| combined.write_all(&key))
                .and_then(|()| combined.flush())
                .map_err(|_| "could not prepare MongoDB client identity".to_string())?;
            tls.cert_key_file_path = Some(combined.path().to_path_buf());
            Some(combined)
        }
        (None, None) => None,
        _ => {
            return Err(
                "MongoDB client certificate and key must be configured together".to_string(),
            );
        }
    };
    options.tls = Some(Tls::Enabled(tls));
    Ok(identity)
}
