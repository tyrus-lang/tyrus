use super::DbValue;
use futures_util::TryStreamExt;
use mongodb::Client;
use mongodb::bson::{Bson, Document, Regex, doc, oid::ObjectId};
use mongodb::error::ErrorKind;
use mongodb::options::{ClientOptions, Tls, TlsOptions};
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::future::Future;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const MAX_BSON_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;
const MAX_BSON_NESTING_DEPTH: usize = 100;
const MAX_TLS_FILE_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Copy)]
pub(crate) struct MongoPoolOptions {
    max_connections: u32,
    connect_timeout_ms: u64,
    acquire_timeout_ms: u64,
    query_timeout_ms: u64,
    idle_timeout_ms: u64,
}

impl Default for MongoPoolOptions {
    fn default() -> Self {
        Self {
            max_connections: 10,
            connect_timeout_ms: 10_000,
            acquire_timeout_ms: 5_000,
            query_timeout_ms: 30_000,
            idle_timeout_ms: 30_000,
        }
    }
}

impl MongoPoolOptions {
    pub(crate) fn from_json(value: &serde_json::Value) -> Option<Self> {
        let object = value.as_object()?;
        if object.keys().any(|key| {
            !matches!(
                key.as_str(),
                "maxConnections"
                    | "connectTimeoutMs"
                    | "acquireTimeoutMs"
                    | "queryTimeoutMs"
                    | "idleTimeoutMs"
                    | "maxLifetimeMs"
            )
        }) {
            return None;
        }
        let mut options = Self::default();
        if let Some(value) = json_u64(object.get("maxConnections"))? {
            options.max_connections = u32::try_from(value).ok()?;
        }
        options.connect_timeout_ms =
            json_u64(object.get("connectTimeoutMs"))?.unwrap_or(options.connect_timeout_ms);
        options.acquire_timeout_ms =
            json_u64(object.get("acquireTimeoutMs"))?.unwrap_or(options.acquire_timeout_ms);
        options.query_timeout_ms =
            json_u64(object.get("queryTimeoutMs"))?.unwrap_or(options.query_timeout_ms);
        options.idle_timeout_ms =
            json_u64(object.get("idleTimeoutMs"))?.unwrap_or(options.idle_timeout_ms);
        if object.contains_key("maxLifetimeMs") {
            return None;
        }
        Some(options)
    }

    pub(crate) fn validate(self) -> bool {
        (1..=100).contains(&self.max_connections)
            && (1..=120_000).contains(&self.connect_timeout_ms)
            && (1..=120_000).contains(&self.acquire_timeout_ms)
            && (1..=120_000).contains(&self.query_timeout_ms)
            && (1..=86_400_000).contains(&self.idle_timeout_ms)
    }
}

fn json_u64(value: Option<&serde_json::Value>) -> Option<Option<u64>> {
    value.map_or(Some(None), |value| value.as_u64().map(Some))
}

#[derive(Clone)]
pub(crate) struct MongoDatabase {
    client: Client,
    _client_identity_file: Option<Arc<NamedTempFile>>,
    operation_slots: Arc<Semaphore>,
    initialized_collections: Arc<tokio::sync::Mutex<HashSet<String>>>,
    acquire_timeout_ms: u64,
    query_timeout_ms: u64,
}

impl MongoDatabase {
    pub(crate) fn unsupported_operation(&self) -> MongoRuntimeError {
        MongoRuntimeError::unsupported_operation()
    }

    pub(crate) async fn open(
        connection_string: &str,
        options: MongoPoolOptions,
        ca_file: Option<&str>,
        cert_file: Option<&str>,
        key_file: Option<&str>,
    ) -> Result<Self, MongoRuntimeError> {
        validate_tls_files(ca_file, cert_file, key_file)?;
        let mut client_options = tokio::time::timeout(
            Duration::from_millis(options.connect_timeout_ms),
            ClientOptions::parse(connection_string),
        )
        .await
        .map_err(|_| MongoRuntimeError::connection_timeout())?
        .map_err(|_| MongoRuntimeError::invalid_connection())?;
        let database_name = client_options
            .default_database
            .clone()
            .filter(|name| !name.is_empty())
            .ok_or_else(MongoRuntimeError::database_name_required)?;
        client_options.max_pool_size = Some(options.max_connections);
        client_options.min_pool_size = Some(0);
        client_options.connect_timeout = Some(Duration::from_millis(options.connect_timeout_ms));
        client_options.server_selection_timeout =
            Some(Duration::from_millis(options.connect_timeout_ms));
        client_options.max_idle_time = Some(Duration::from_millis(options.idle_timeout_ms));
        let client_identity_file =
            configure_tls(&mut client_options, ca_file, cert_file, key_file)?;

        let client = Client::with_options(client_options)
            .map_err(|_| MongoRuntimeError::invalid_connection())?;
        let database = client.database(&database_name);
        tokio::time::timeout(
            Duration::from_millis(options.connect_timeout_ms),
            database.run_command(doc! { "ping": 1 }),
        )
        .await
        .map_err(|_| MongoRuntimeError::connection_timeout())?
        .map_err(|_| MongoRuntimeError::connection_failed())?;

        Ok(Self {
            client,
            _client_identity_file: client_identity_file.map(Arc::new),
            operation_slots: Arc::new(Semaphore::new(options.max_connections as usize)),
            initialized_collections: Arc::new(tokio::sync::Mutex::new(HashSet::new())),
            acquire_timeout_ms: options.acquire_timeout_ms,
            query_timeout_ms: options.query_timeout_ms,
        })
    }

    pub(crate) async fn read(
        &self,
        operation: MongoReadOperation,
        descriptor: &str,
        parameters: &[DbValue],
    ) -> Result<Value, MongoRuntimeError> {
        self.with_operation_slot(self.read_inner(operation, descriptor, parameters))
            .await
    }

    pub(crate) async fn mutate(
        &self,
        descriptor: &str,
        parameters: &[DbValue],
    ) -> Result<Value, MongoRuntimeError> {
        self.with_operation_slot(self.mutate_inner(descriptor, parameters))
            .await
    }

    async fn with_operation_slot<T>(
        &self,
        operation: impl Future<Output = Result<T, MongoRuntimeError>>,
    ) -> Result<T, MongoRuntimeError> {
        let _permit = self.acquire_operation_slot().await?;
        tokio::time::timeout(Duration::from_millis(self.query_timeout_ms), operation)
            .await
            .map_err(|_| MongoRuntimeError::query_timeout())?
    }

    async fn acquire_operation_slot(&self) -> Result<OwnedSemaphorePermit, MongoRuntimeError> {
        tokio::time::timeout(
            Duration::from_millis(self.acquire_timeout_ms),
            Arc::clone(&self.operation_slots).acquire_owned(),
        )
        .await
        .map_err(|_| MongoRuntimeError::pool_timeout())?
        .map_err(|_| MongoRuntimeError::pool_timeout())
    }

    async fn read_inner(
        &self,
        operation: MongoReadOperation,
        descriptor: &str,
        parameters: &[DbValue],
    ) -> Result<Value, MongoRuntimeError> {
        let descriptor: Value =
            serde_json::from_str(descriptor).map_err(|_| MongoRuntimeError::invalid_operation())?;
        let collection_name = descriptor
            .get("collection")
            .and_then(Value::as_str)
            .filter(|name| valid_collection_name(name))
            .ok_or_else(MongoRuntimeError::invalid_operation)?;
        let kind = descriptor
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(MongoRuntimeError::invalid_operation)?;
        let expected_kind = match &operation {
            MongoReadOperation::Count => "count",
            MongoReadOperation::One => "findOne",
            MongoReadOperation::Many
            | MongoReadOperation::Page { .. }
            | MongoReadOperation::CursorPage { .. } => "findMany",
        };
        if kind != expected_kind {
            return Err(MongoRuntimeError::invalid_operation());
        }
        let filter = descriptor
            .get("filter")
            .and_then(Value::as_object)
            .ok_or_else(MongoRuntimeError::invalid_operation)?;
        let filter = resolve_filter(filter, parameters)?;
        validate_document_size(&filter)?;
        let collection = self
            .client
            .default_database()
            .ok_or_else(MongoRuntimeError::database_name_required)?
            .collection::<Document>(collection_name);
        match operation {
            MongoReadOperation::Count => collection
                .count_documents(filter)
                .await
                .map(|count| Value::Number(count.into()))
                .map_err(|_| MongoRuntimeError::query_failed()),
            MongoReadOperation::One
            | MongoReadOperation::Many
            | MongoReadOperation::Page { .. }
            | MongoReadOperation::CursorPage { .. } => {
                let mappings = parse_field_mappings(descriptor.get("fields"))?;
                let mut projection = Document::new();
                for field in &mappings {
                    projection.insert(&field.document, 1);
                }
                if matches!(
                    operation,
                    MongoReadOperation::Page { .. } | MongoReadOperation::CursorPage { .. }
                ) {
                    let total = collection
                        .count_documents(filter.clone())
                        .await
                        .map_err(|_| MongoRuntimeError::query_failed())?;
                    return Self::read_mongo_page(
                        collection, filter, projection, mappings, operation, total,
                    )
                    .await;
                }
                let mut action = collection.find(filter).projection(projection);
                if matches!(operation, MongoReadOperation::One) {
                    action = action.limit(1);
                }
                let cursor = action
                    .await
                    .map_err(|_| MongoRuntimeError::query_failed())?;
                let documents = cursor
                    .try_collect::<Vec<_>>()
                    .await
                    .map_err(|_| MongoRuntimeError::query_failed())?;
                let values = documents
                    .iter()
                    .map(|document| document_to_json(document, &mappings))
                    .collect::<Result<Vec<_>, _>>()?;
                if matches!(operation, MongoReadOperation::One) {
                    Ok(values.into_iter().next().unwrap_or(Value::Null))
                } else {
                    Ok(Value::Array(values))
                }
            }
        }
    }

    async fn read_mongo_page(
        collection: mongodb::Collection<Document>,
        mut filter: Document,
        projection: Document,
        mappings: Vec<MongoFieldMapping>,
        operation: MongoReadOperation,
        total: u64,
    ) -> Result<Value, MongoRuntimeError> {
        let (page_size, numbered_page, cursor_id) = match operation {
            MongoReadOperation::Page {
                page_size,
                page_number,
            } if page_size > 0 && page_number > 0 => {
                let skip = page_size
                    .checked_mul(page_number - 1)
                    .ok_or_else(MongoRuntimeError::invalid_pagination)?;
                (page_size, Some((page_number, skip)), None)
            }
            MongoReadOperation::CursorPage { page_size, after } if page_size > 0 => {
                if after.len() > 4096 {
                    return Err(MongoRuntimeError::invalid_cursor());
                }
                let id = if after.is_empty() {
                    None
                } else {
                    let value: Value = serde_json::from_str(&after)
                        .map_err(|_| MongoRuntimeError::invalid_cursor())?;
                    let text = value
                        .as_str()
                        .ok_or_else(MongoRuntimeError::invalid_cursor)?;
                    Some(
                        mongodb::bson::oid::ObjectId::parse_str(text)
                            .map_err(|_| MongoRuntimeError::invalid_cursor())?,
                    )
                };
                (page_size, None, id)
            }
            MongoReadOperation::Page { .. } | MongoReadOperation::CursorPage { .. } => {
                return Err(MongoRuntimeError::invalid_pagination());
            }
            _ => return Err(MongoRuntimeError::invalid_operation()),
        };
        if let Some(cursor_id) = cursor_id {
            let cursor_filter = doc! { "_id": { "$gt": cursor_id } };
            filter = if filter.is_empty() {
                cursor_filter
            } else {
                doc! { "$and": [filter, cursor_filter] }
            };
        }
        let fetch_size = page_size
            .checked_add(u64::from(numbered_page.is_none()))
            .ok_or_else(MongoRuntimeError::invalid_pagination)?;
        let mut action = collection
            .find(filter)
            .projection(projection)
            .sort(doc! { "_id": 1 })
            .limit(i64::try_from(fetch_size).map_err(|_| MongoRuntimeError::invalid_pagination())?);
        if let Some((_, skip)) = numbered_page {
            action = action.skip(skip);
        }
        let cursor = action
            .await
            .map_err(|_| MongoRuntimeError::query_failed())?;
        let documents = cursor
            .try_collect::<Vec<_>>()
            .await
            .map_err(|_| MongoRuntimeError::query_failed())?;
        let mut result = serde_json::Map::new();
        if let Some((page, _)) = numbered_page {
            let items = documents
                .iter()
                .map(|document| document_to_json(document, &mappings))
                .collect::<Result<Vec<_>, _>>()?;
            result.insert("items".to_string(), Value::Array(items));
            result.insert("total".to_string(), Value::Number(total.into()));
            result.insert("pageSize".to_string(), Value::Number(page_size.into()));
            result.insert("page".to_string(), Value::Number(page.into()));
            result.insert(
                "totalPages".to_string(),
                Value::Number(total.div_ceil(page_size).into()),
            );
            return Ok(Value::Object(result));
        }
        let has_more = documents.len() as u64 > page_size;
        let mut documents = documents;
        if has_more {
            documents.pop();
        }
        let next_cursor = if has_more {
            let id = documents
                .last()
                .and_then(|document| document.get_object_id("_id").ok())
                .ok_or_else(MongoRuntimeError::invalid_operation)?;
            Some(
                serde_json::to_string(&Value::String(id.to_hex()))
                    .map_err(|_| MongoRuntimeError::invalid_operation())?,
            )
        } else {
            None
        };
        let items = documents
            .iter()
            .map(|document| document_to_json(document, &mappings))
            .collect::<Result<Vec<_>, _>>()?;
        result.insert("items".to_string(), Value::Array(items));
        result.insert("total".to_string(), Value::Number(total.into()));
        result.insert("pageSize".to_string(), Value::Number(page_size.into()));
        result.insert("hasMore".to_string(), Value::Bool(has_more));
        if let Some(cursor) = next_cursor {
            result.insert("nextCursor".to_string(), Value::String(cursor));
        }
        Ok(Value::Object(result))
    }

    async fn mutate_inner(
        &self,
        source: &str,
        parameters: &[DbValue],
    ) -> Result<Value, MongoRuntimeError> {
        let descriptor: Value =
            serde_json::from_str(source).map_err(|_| MongoRuntimeError::invalid_operation())?;
        let collection_name = descriptor
            .get("collection")
            .and_then(Value::as_str)
            .filter(|name| valid_collection_name(name))
            .ok_or_else(MongoRuntimeError::invalid_operation)?;
        let kind = descriptor
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(MongoRuntimeError::invalid_operation)?;
        let collection = self
            .client
            .default_database()
            .ok_or_else(MongoRuntimeError::database_name_required)?
            .collection::<Document>(collection_name);
        let return_fields = parse_field_mappings(descriptor.get("returnFields"))?;
        let result = match kind {
            "insertOne" => {
                let document = descriptor
                    .get("document")
                    .and_then(Value::as_object)
                    .ok_or_else(MongoRuntimeError::invalid_operation)?;
                let mut document =
                    resolve_document(document, parameters, descriptor.get("fieldTypes"))?;
                apply_model_timestamps(
                    &mut document,
                    descriptor.get("timestamps"),
                    descriptor.get("timestampFields"),
                    false,
                );
                validate_model_document(&document, descriptor.get("fieldTypes"), true)?;
                validate_document_size(&document)?;
                self.ensure_model_collection(collection_name, descriptor.get("fieldTypes"))
                    .await?;
                let result = collection
                    .insert_one(document.clone())
                    .await
                    .map_err(|_| MongoRuntimeError::mutation_failed())?;
                document.insert("_id", result.inserted_id);
                document_to_json(&document, &return_fields)?
            }
            "insertMany" => {
                let documents = descriptor
                    .get("documents")
                    .and_then(Value::as_array)
                    .filter(|documents| !documents.is_empty() && documents.len() <= 500)
                    .ok_or_else(MongoRuntimeError::invalid_operation)?;
                let documents = documents
                    .iter()
                    .map(|document| {
                        document
                            .as_object()
                            .map(|document| {
                                let mut document = resolve_document(
                                    document,
                                    parameters,
                                    descriptor.get("fieldTypes"),
                                )?;
                                apply_model_timestamps(
                                    &mut document,
                                    descriptor.get("timestamps"),
                                    descriptor.get("timestampFields"),
                                    false,
                                );
                                validate_model_document(
                                    &document,
                                    descriptor.get("fieldTypes"),
                                    true,
                                )?;
                                validate_document_size(&document)?;
                                Ok(document)
                            })
                            .unwrap_or_else(|| Err(MongoRuntimeError::invalid_operation()))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                self.ensure_model_collection(collection_name, descriptor.get("fieldTypes"))
                    .await?;
                let mut documents = documents;
                for document in &mut documents {
                    document
                        .entry("_id".to_string())
                        .or_insert_with(|| Bson::ObjectId(ObjectId::new()));
                }
                let result = collection
                    .insert_many(documents.clone())
                    .await
                    .map_err(|_| MongoRuntimeError::mutation_failed())?;
                let returned = result
                    .inserted_ids
                    .into_iter()
                    .map(|(index, id)| {
                        let document = documents
                            .get_mut(index)
                            .ok_or_else(MongoRuntimeError::mutation_failed)?;
                        document.insert("_id", id);
                        document_to_json(document, &return_fields)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Value::Array(returned)
            }
            "updateOne" | "updateMany" => {
                let filter = descriptor
                    .get("filter")
                    .and_then(Value::as_object)
                    .ok_or_else(MongoRuntimeError::invalid_operation)?;
                let filter = resolve_filter(filter, parameters)?;
                validate_document_size(&filter)?;
                let changes = descriptor
                    .get("changes")
                    .and_then(Value::as_object)
                    .filter(|changes| !changes.is_empty())
                    .ok_or_else(MongoRuntimeError::invalid_operation)?;
                let mut changes =
                    resolve_document(changes, parameters, descriptor.get("fieldTypes"))?;
                apply_model_timestamps(
                    &mut changes,
                    descriptor.get("timestamps"),
                    descriptor.get("timestampFields"),
                    true,
                );
                validate_model_update(&changes, descriptor.get("fieldTypes"))?;
                validate_document_size(&changes)?;
                let update = doc! { "$set": changes };
                if kind == "updateOne" {
                    let document = collection
                        .find_one_and_update(filter, update)
                        .return_document(mongodb::options::ReturnDocument::After)
                        .await
                        .map_err(|_| MongoRuntimeError::mutation_failed())?;
                    match document {
                        Some(document) => document_to_json(&document, &return_fields)?,
                        None => Value::Null,
                    }
                } else {
                    let modified = collection
                        .update_many(filter, update)
                        .await
                        .map_err(|_| MongoRuntimeError::mutation_failed())?
                        .modified_count;
                    Value::Number(modified.into())
                }
            }
            "deleteOne" | "deleteMany" => {
                let filter = descriptor
                    .get("filter")
                    .and_then(Value::as_object)
                    .ok_or_else(MongoRuntimeError::invalid_operation)?;
                let filter = resolve_filter(filter, parameters)?;
                validate_document_size(&filter)?;
                if kind == "deleteOne" {
                    match collection
                        .find_one_and_delete(filter)
                        .await
                        .map_err(|_| MongoRuntimeError::mutation_failed())?
                    {
                        Some(document) => document_to_json(&document, &return_fields)?,
                        None => Value::Null,
                    }
                } else {
                    let deleted = collection
                        .delete_many(filter)
                        .await
                        .map_err(|_| MongoRuntimeError::mutation_failed())?
                        .deleted_count;
                    Value::Number(deleted.into())
                }
            }
            _ => return Err(MongoRuntimeError::invalid_operation()),
        };
        Ok(result)
    }

    async fn ensure_model_collection(
        &self,
        collection_name: &str,
        field_types: Option<&Value>,
    ) -> Result<(), MongoRuntimeError> {
        let mut initialized = self.initialized_collections.lock().await;
        if initialized.contains(collection_name) {
            return Ok(());
        }
        let database = self
            .client
            .default_database()
            .ok_or_else(MongoRuntimeError::database_name_required)?;
        let mut existing = database
            .list_collections()
            .filter(doc! { "name": collection_name })
            .await
            .map_err(|_| MongoRuntimeError::mutation_failed())?;
        if existing
            .try_next()
            .await
            .map_err(|_| MongoRuntimeError::mutation_failed())?
            .is_none()
        {
            let validator = model_collection_validator(field_types)?;
            let result = database
                .run_command(doc! {
                    "create": collection_name,
                    "validator": validator,
                    "validationLevel": "strict",
                    "validationAction": "error",
                })
                .await;
            if let Err(error) = result {
                let collection_was_created_concurrently = matches!(
                    error.kind.as_ref(),
                    ErrorKind::Command(command) if command.code == 48
                );
                if !collection_was_created_concurrently {
                    return Err(MongoRuntimeError::mutation_failed());
                }
            }
        }
        initialized.insert(collection_name.to_string());
        Ok(())
    }

    pub(crate) async fn close(self) {
        self.client.shutdown().await;
    }
}

fn validate_tls_files(
    ca_file: Option<&str>,
    cert_file: Option<&str>,
    key_file: Option<&str>,
) -> Result<(), MongoRuntimeError> {
    if cert_file.is_some() != key_file.is_some() {
        return Err(MongoRuntimeError::tls_configuration());
    }
    for path in [ca_file, cert_file, key_file].into_iter().flatten() {
        let metadata =
            std::fs::metadata(path).map_err(|_| MongoRuntimeError::tls_configuration())?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_TLS_FILE_BYTES {
            return Err(MongoRuntimeError::tls_configuration());
        }
    }
    Ok(())
}

fn configure_tls(
    options: &mut ClientOptions,
    ca_file: Option<&str>,
    cert_file: Option<&str>,
    key_file: Option<&str>,
) -> Result<Option<NamedTempFile>, MongoRuntimeError> {
    if ca_file.is_none() && cert_file.is_none() && key_file.is_none() {
        if let Some(Tls::Enabled(tls)) = &options.tls
            && (tls.cert_key_file_path.is_some() || tls.allow_invalid_certificates == Some(true))
        {
            return Err(MongoRuntimeError::tls_configuration());
        }
        return Ok(None);
    }

    let mut tls = match options.tls.take() {
        Some(Tls::Disabled) => return Err(MongoRuntimeError::tls_configuration()),
        Some(Tls::Enabled(tls)) => tls,
        None => TlsOptions::default(),
    };
    if tls.cert_key_file_path.is_some() || tls.allow_invalid_certificates == Some(true) {
        return Err(MongoRuntimeError::tls_configuration());
    }
    if tls.ca_file_path.is_some() {
        return Err(MongoRuntimeError::tls_configuration());
    }
    if let Some(path) = ca_file {
        tls.ca_file_path = Some(PathBuf::from(path));
    }
    let client_identity_file = match (cert_file, key_file) {
        (Some(cert_file), Some(key_file)) => {
            use zeroize::Zeroizing;
            let certificate = Zeroizing::new(
                std::fs::read(cert_file).map_err(|_| MongoRuntimeError::tls_configuration())?,
            );
            let key = Zeroizing::new(
                std::fs::read(key_file).map_err(|_| MongoRuntimeError::tls_configuration())?,
            );
            let mut combined =
                NamedTempFile::new().map_err(|_| MongoRuntimeError::tls_configuration())?;
            combined
                .write_all(&certificate)
                .and_then(|()| combined.write_all(b"\n"))
                .and_then(|()| combined.write_all(&key))
                .and_then(|()| combined.flush())
                .map_err(|_| MongoRuntimeError::tls_configuration())?;
            tls.cert_key_file_path = Some(combined.path().to_path_buf());
            Some(combined)
        }
        (None, None) => None,
        _ => return Err(MongoRuntimeError::tls_configuration()),
    };
    options.tls = Some(Tls::Enabled(tls));
    Ok(client_identity_file)
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MongoRuntimeError {
    pub(crate) code: &'static str,
    pub(crate) message: &'static str,
}

impl MongoRuntimeError {
    pub(crate) const fn invalid_connection() -> Self {
        Self {
            code: "ERR_DB_INVALID_CONNECTION",
            message: "connection variable does not contain a valid MongoDB URL",
        }
    }

    pub(crate) const fn connection_timeout() -> Self {
        Self {
            code: "ERR_DB_CONNECTION_TIMEOUT",
            message: "MongoDB connection exceeded the configured timeout",
        }
    }

    pub(crate) const fn connection_failed() -> Self {
        Self {
            code: "ERR_DB_CONNECTION_FAILED",
            message: "MongoDB connection failed",
        }
    }

    pub(crate) const fn database_name_required() -> Self {
        Self {
            code: "ERR_DB_INVALID_CONNECTION",
            message: "MongoDB connection URL must name a database",
        }
    }

    pub(crate) const fn tls_configuration() -> Self {
        Self {
            code: "ERR_DB_TLS_CONFIGURATION",
            message: "MongoDB TLS must verify the server certificate and hostname",
        }
    }

    pub(crate) const fn unsupported_operation() -> Self {
        Self {
            code: "ERR_DB_FEATURE_UNSUPPORTED",
            message: "this MongoDB Model operation is not available yet",
        }
    }

    pub(crate) const fn invalid_operation() -> Self {
        Self {
            code: "ERR_DB_INVALID_OPERATION",
            message: "MongoDB model operation descriptor is invalid",
        }
    }

    pub(crate) const fn invalid_pagination() -> Self {
        Self {
            code: "ERR_DB_INVALID_PAGINATION",
            message: "page and pageSize must be positive and their offset must fit in u64",
        }
    }

    pub(crate) const fn invalid_cursor() -> Self {
        Self {
            code: "ERR_DB_INVALID_CURSOR",
            message: "cursor token is malformed or does not match the model id type",
        }
    }

    pub(crate) const fn query_failed() -> Self {
        Self {
            code: "ERR_DB_QUERY_FAILED",
            message: "MongoDB model read failed",
        }
    }

    pub(crate) const fn mutation_failed() -> Self {
        Self {
            code: "ERR_DB_MUTATION_FAILED",
            message: "MongoDB model mutation failed",
        }
    }

    pub(crate) const fn query_timeout() -> Self {
        Self {
            code: "ERR_DB_QUERY_TIMEOUT",
            message: "MongoDB model operation exceeded the configured query timeout",
        }
    }

    pub(crate) const fn pool_timeout() -> Self {
        Self {
            code: "ERR_DB_POOL_TIMEOUT",
            message: "MongoDB operation could not acquire a runtime connection slot before the configured timeout",
        }
    }

    pub(crate) const fn unsupported_value() -> Self {
        Self {
            code: "ERR_DB_UNSUPPORTED_VALUE",
            message: "MongoDB returned a value that cannot be represented by the declared Model",
        }
    }

    pub(crate) const fn invalid_object_id() -> Self {
        Self {
            code: "ERR_DB_INVALID_OBJECT_ID",
            message: "MongoDB ObjectId must be a 24-character hexadecimal string",
        }
    }

    pub(crate) const fn document_too_large() -> Self {
        Self {
            code: "ERR_DB_DOCUMENT_TOO_LARGE",
            message: "MongoDB document or filter exceeds the 16 MiB BSON limit",
        }
    }

    pub(crate) const fn nesting_too_deep() -> Self {
        Self {
            code: "ERR_DB_VALUE_TOO_DEEP",
            message: "MongoDB document values cannot exceed 100 nested levels",
        }
    }

    pub(crate) const fn invalid_temporal_value() -> Self {
        Self {
            code: "ERR_DB_INVALID_TEMPORAL_VALUE",
            message: "MongoDB Instant values must have valid seconds and nanoseconds fields",
        }
    }

    pub(crate) const fn invalid_model_value() -> Self {
        Self {
            code: "ERR_DB_INVALID_MODEL_VALUE",
            message: "MongoDB Model value does not match its declared field type or required fields",
        }
    }

    pub(crate) const fn temporal_precision_loss() -> Self {
        Self {
            code: "ERR_DB_VALUE_OUT_OF_RANGE",
            message: "MongoDB BSON Date stores millisecond precision; this Instant has sub-millisecond precision",
        }
    }
}

#[derive(Clone)]
pub(crate) enum MongoReadOperation {
    Count,
    One,
    Many,
    Page { page_size: u64, page_number: u64 },
    CursorPage { page_size: u64, after: String },
}

fn valid_collection_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 120
        && !name.starts_with("system.")
        && !name.contains('\0')
        && !name.contains('$')
}

fn valid_field_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('$') && !name.contains('.') && !name.contains('\0')
}

fn model_collection_validator(field_types: Option<&Value>) -> Result<Document, MongoRuntimeError> {
    let Some(Value::Array(fields)) = field_types else {
        return Err(MongoRuntimeError::invalid_operation());
    };
    let mut properties = Document::new();
    let mut required = vec![Bson::String("_id".to_string())];
    let mut names = HashSet::with_capacity(fields.len());
    properties.insert("_id", doc! { "bsonType": "objectId" });
    for field in fields {
        let name = field
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| valid_field_name(name))
            .ok_or_else(MongoRuntimeError::invalid_operation)?;
        if !names.insert(name) {
            return Err(MongoRuntimeError::invalid_operation());
        }
        if name == "_id" {
            if field.get("kind").and_then(Value::as_str) != Some("objectId") {
                return Err(MongoRuntimeError::invalid_operation());
            }
            continue;
        }
        if field.get("required").and_then(Value::as_bool) == Some(true) {
            required.push(Bson::String(name.to_string()));
        }
        let schema = field.get("schema");
        let kind = field
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(MongoRuntimeError::invalid_operation)?;
        let nullable = field
            .get("nullable")
            .and_then(Value::as_bool)
            .ok_or_else(MongoRuntimeError::invalid_operation)?;
        if let Some(schema) = validator_schema(
            schema,
            kind,
            nullable,
            schema
                .and_then(|schema| schema.get("definitions"))
                .and_then(Value::as_object),
            &mut HashSet::new(),
            0,
        )? {
            properties.insert(name, schema);
        }
    }
    Ok(doc! {
        "$jsonSchema": {
            "bsonType": "object",
            "required": required,
            "properties": properties,
            "additionalProperties": false,
        }
    })
}

fn validator_schema(
    schema: Option<&Value>,
    fallback_kind: &str,
    fallback_nullable: bool,
    definitions: Option<&Map<String, Value>>,
    building: &mut HashSet<String>,
    depth: usize,
) -> Result<Option<Document>, MongoRuntimeError> {
    if depth > MAX_BSON_NESTING_DEPTH {
        return Err(MongoRuntimeError::nesting_too_deep());
    }
    let kind = schema
        .and_then(|schema| schema.get("kind"))
        .and_then(Value::as_str)
        .unwrap_or(fallback_kind);
    let nullable = schema
        .and_then(|schema| schema.get("nullable"))
        .and_then(Value::as_bool)
        .unwrap_or(fallback_nullable);
    if kind == "reference" {
        let name = schema
            .and_then(|schema| schema.get("name"))
            .and_then(Value::as_str)
            .ok_or_else(MongoRuntimeError::invalid_operation)?;
        let Some(definition) = definitions.and_then(|definitions| definitions.get(name)) else {
            return Err(MongoRuntimeError::invalid_operation());
        };
        if !building.insert(name.to_string()) {
            return Ok(None);
        }
        let result = validator_schema(
            Some(definition),
            fallback_kind,
            nullable,
            definitions,
            building,
            depth + 1,
        );
        building.remove(name);
        return result;
    }
    let bson_type = match kind {
        "any" => return Ok(None),
        "array" | "objectIdArray" => "array",
        "boolean" => "bool",
        "bytes" => "binData",
        "document" => "object",
        "integer" => "intOrLong",
        "instant" => "date",
        "number" => "number",
        "objectId" => "objectId",
        "string" => "string",
        _ => return Err(MongoRuntimeError::invalid_operation()),
    };
    let mut schema_document = Document::new();
    let bson_types = match bson_type {
        "intOrLong" => vec![
            Bson::String("int".to_string()),
            Bson::String("long".to_string()),
        ],
        "number" => vec![
            Bson::String("double".to_string()),
            Bson::String("int".to_string()),
            Bson::String("long".to_string()),
            Bson::String("decimal".to_string()),
        ],
        bson_type => vec![Bson::String(bson_type.to_string())],
    };
    let mut bson_types = bson_types;
    if nullable {
        bson_types.push(Bson::String("null".to_string()));
    }
    schema_document.insert("bsonType", Bson::Array(bson_types));
    if matches!(kind, "array" | "objectIdArray")
        && let Some(items) = schema.and_then(|schema| schema.get("items"))
    {
        let item_schema = if items.is_array() {
            Document::new()
        } else {
            let item_kind = items
                .get("kind")
                .and_then(Value::as_str)
                .ok_or_else(MongoRuntimeError::invalid_operation)?;
            let item_nullable = items
                .get("nullable")
                .and_then(Value::as_bool)
                .ok_or_else(MongoRuntimeError::invalid_operation)?;
            validator_schema(
                Some(items),
                item_kind,
                item_nullable,
                definitions,
                building,
                depth + 1,
            )?
            .unwrap_or_default()
        };
        schema_document.insert("items", item_schema);
    }
    if kind == "document"
        && let Some(fields) = schema
            .and_then(|schema| schema.get("fields"))
            .and_then(Value::as_array)
    {
        let mut properties = Document::new();
        let mut required = Vec::new();
        let mut names = HashSet::with_capacity(fields.len());
        for field in fields {
            let name = field
                .get("name")
                .and_then(Value::as_str)
                .filter(|name| valid_field_name(name))
                .ok_or_else(MongoRuntimeError::invalid_operation)?;
            if !names.insert(name) {
                return Err(MongoRuntimeError::invalid_operation());
            }
            if field.get("required").and_then(Value::as_bool) == Some(true) {
                required.push(Bson::String(name.to_string()));
            }
            let child = field
                .get("schema")
                .ok_or_else(MongoRuntimeError::invalid_operation)?;
            let child_kind = child
                .get("kind")
                .and_then(Value::as_str)
                .ok_or_else(MongoRuntimeError::invalid_operation)?;
            let child_nullable = child
                .get("nullable")
                .and_then(Value::as_bool)
                .ok_or_else(MongoRuntimeError::invalid_operation)?;
            if let Some(child_schema) = validator_schema(
                Some(child),
                child_kind,
                child_nullable,
                definitions,
                building,
                depth + 1,
            )? {
                properties.insert(name, child_schema);
            }
        }
        schema_document.insert("required", required);
        schema_document.insert("properties", properties);
        schema_document.insert("additionalProperties", false);
    }
    Ok(Some(schema_document))
}

struct MongoFieldMapping {
    logical: String,
    document: String,
    object_id: bool,
    object_id_array: bool,
    nullable_items: bool,
    kind: Option<String>,
    schema: Option<Value>,
    nullable: bool,
}

fn parse_field_mappings(
    value: Option<&Value>,
) -> Result<Vec<MongoFieldMapping>, MongoRuntimeError> {
    let Some(Value::Array(fields)) = value else {
        return Err(MongoRuntimeError::invalid_operation());
    };
    let mut logical_names = HashSet::with_capacity(fields.len());
    fields
        .iter()
        .map(|field| {
            let logical = field
                .get("logical")
                .and_then(Value::as_str)
                .ok_or_else(MongoRuntimeError::invalid_operation)?;
            let document = field
                .get("document")
                .and_then(Value::as_str)
                .ok_or_else(MongoRuntimeError::invalid_operation)?;
            if !valid_field_name(document) {
                return Err(MongoRuntimeError::invalid_operation());
            }
            if !logical_names.insert(logical) {
                return Err(MongoRuntimeError::invalid_operation());
            }
            let object_id = field
                .get("objectId")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let object_id_array = field
                .get("objectIdArray")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let object_id_array_nullable_items = field
                .get("objectIdArrayNullableItems")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if object_id_array_nullable_items && !object_id_array {
                return Err(MongoRuntimeError::invalid_operation());
            }
            Ok(MongoFieldMapping {
                logical: logical.to_string(),
                document: document.to_string(),
                object_id,
                object_id_array,
                nullable_items: object_id_array_nullable_items,
                kind: field
                    .get("kind")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                schema: field.get("schema").cloned(),
                nullable: field
                    .get("nullable")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect()
}

fn resolve_filter(
    input: &Map<String, Value>,
    parameters: &[DbValue],
) -> Result<Document, MongoRuntimeError> {
    resolve_filter_at_depth(input, parameters, 0)
}

fn resolve_filter_at_depth(
    input: &Map<String, Value>,
    parameters: &[DbValue],
    depth: usize,
) -> Result<Document, MongoRuntimeError> {
    if depth > MAX_BSON_NESTING_DEPTH {
        return Err(MongoRuntimeError::nesting_too_deep());
    }
    let mut output = Document::new();
    for (key, value) in input {
        if key != "$or" && !valid_field_name(key) {
            return Err(MongoRuntimeError::invalid_operation());
        }
        let value = if key == "$or" {
            let Value::Array(groups) = value else {
                return Err(MongoRuntimeError::invalid_operation());
            };
            Bson::Array(
                groups
                    .iter()
                    .map(|group| {
                        group
                            .as_object()
                            .map(|group| {
                                resolve_filter_at_depth(group, parameters, depth + 1)
                                    .map(Bson::Document)
                            })
                            .unwrap_or_else(|| Err(MongoRuntimeError::invalid_operation()))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            )
        } else if let Some(operators) = value.as_object().filter(|operators| {
            !operators.is_empty()
                && operators.keys().all(|operator| {
                    matches!(
                        operator.as_str(),
                        "$eq" | "$ne" | "$gt" | "$gte" | "$lt" | "$lte" | "$in" | "$tyrusLike"
                    )
                })
        }) {
            let mut values = Document::new();
            for (operator, value) in operators {
                if !matches!(
                    operator.as_str(),
                    "$eq" | "$ne" | "$gt" | "$gte" | "$lt" | "$lte" | "$in" | "$tyrusLike"
                ) {
                    return Err(MongoRuntimeError::invalid_operation());
                }
                let value = if operator == "$in" {
                    let Value::Array(values) = value else {
                        return Err(MongoRuntimeError::invalid_operation());
                    };
                    Bson::Array(
                        values
                            .iter()
                            .map(|value| resolve_parameter(value, parameters))
                            .collect::<Result<Vec<_>, _>>()?,
                    )
                } else {
                    resolve_parameter(value, parameters)?
                };
                if operator == "$tyrusLike" {
                    let Bson::String(pattern) = value else {
                        return Err(MongoRuntimeError::unsupported_value());
                    };
                    values.insert(
                        "$regex",
                        Bson::RegularExpression(Regex {
                            pattern: sql_like_pattern(&pattern),
                            options: String::new(),
                        }),
                    );
                } else {
                    values.insert(operator, value);
                }
            }
            Bson::Document(values)
        } else {
            resolve_parameter(value, parameters)?
        };
        output.insert(key, value);
    }
    Ok(output)
}

fn resolve_document(
    input: &Map<String, Value>,
    parameters: &[DbValue],
    field_types: Option<&Value>,
) -> Result<Document, MongoRuntimeError> {
    let Some(Value::Array(field_types)) = field_types else {
        return Err(MongoRuntimeError::invalid_operation());
    };
    let mut output = Document::new();
    for (key, value) in input {
        if !valid_field_name(key) {
            return Err(MongoRuntimeError::invalid_operation());
        }
        let schema = field_types
            .iter()
            .find(|field| field.get("name").and_then(Value::as_str) == Some(key))
            .and_then(|field| field.get("schema"));
        output.insert(
            key,
            resolve_parameter_with_schema(value, parameters, schema)?,
        );
    }
    Ok(output)
}

fn resolve_parameter_with_schema(
    value: &Value,
    parameters: &[DbValue],
    schema: Option<&Value>,
) -> Result<Bson, MongoRuntimeError> {
    let Some(object) = value.as_object().filter(|object| object.len() == 1) else {
        return resolve_parameter(value, parameters);
    };
    let Some(index) = object
        .get("$tyrusParam")
        .and_then(Value::as_u64)
        .and_then(|index| usize::try_from(index).ok())
    else {
        return resolve_parameter(value, parameters);
    };
    let parameter = parameters
        .get(index)
        .ok_or_else(MongoRuntimeError::invalid_operation)?;
    let Some(schema) = schema else {
        return db_value_to_bson(parameter);
    };
    db_value_to_bson_with_model_schema(parameter, schema)
}

fn db_value_to_bson_with_model_schema(
    value: &DbValue,
    schema: &Value,
) -> Result<Bson, MongoRuntimeError> {
    let Some(kind) = schema.get("kind").and_then(Value::as_str) else {
        return db_value_to_bson(value);
    };
    if kind == "objectId"
        && let DbValue::Text(value) = value
    {
        return mongodb::bson::oid::ObjectId::parse_str(value)
            .map(Bson::ObjectId)
            .map_err(|_| MongoRuntimeError::invalid_object_id());
    }
    let DbValue::Json(value) = value else {
        return db_value_to_bson(value);
    };
    let definitions = schema.get("definitions").and_then(Value::as_object);
    json_value_to_bson_with_model_schema(value, schema, definitions, 0)
}

fn json_value_to_bson_with_model_schema(
    value: &Value,
    schema: &Value,
    definitions: Option<&Map<String, Value>>,
    depth: usize,
) -> Result<Bson, MongoRuntimeError> {
    if depth > MAX_BSON_NESTING_DEPTH {
        return Err(MongoRuntimeError::nesting_too_deep());
    }
    if value.is_null() {
        return Ok(Bson::Null);
    }
    let kind = schema
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(MongoRuntimeError::invalid_operation)?;
    if kind == "reference" {
        let name = schema
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(MongoRuntimeError::invalid_operation)?;
        let definition = definitions
            .and_then(|definitions| definitions.get(name))
            .ok_or_else(MongoRuntimeError::invalid_operation)?;
        return json_value_to_bson_with_model_schema(value, definition, definitions, depth + 1);
    }
    match kind {
        "instant" => {
            let fields = value
                .as_object()
                .filter(|fields| fields.len() == 2)
                .ok_or_else(MongoRuntimeError::invalid_temporal_value)?;
            let seconds = fields
                .get("seconds")
                .and_then(Value::as_i64)
                .ok_or_else(MongoRuntimeError::invalid_temporal_value)?;
            let nanoseconds = fields
                .get("nanoseconds")
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
                .filter(|value| *value < 1_000_000_000)
                .ok_or_else(MongoRuntimeError::invalid_temporal_value)?;
            if nanoseconds % 1_000_000 != 0 {
                return Err(MongoRuntimeError::temporal_precision_loss());
            }
            let milliseconds = seconds
                .checked_mul(1_000)
                .and_then(|value| value.checked_add(i64::from(nanoseconds / 1_000_000)))
                .ok_or_else(MongoRuntimeError::invalid_temporal_value)?;
            Ok(Bson::DateTime(mongodb::bson::DateTime::from_millis(
                milliseconds,
            )))
        }
        "objectId" => {
            let value = value
                .as_str()
                .ok_or_else(MongoRuntimeError::invalid_object_id)?;
            mongodb::bson::oid::ObjectId::parse_str(value)
                .map(Bson::ObjectId)
                .map_err(|_| MongoRuntimeError::invalid_object_id())
        }
        "bytes" => {
            let bytes = value
                .as_array()
                .ok_or_else(MongoRuntimeError::unsupported_value)?
                .iter()
                .map(|byte| {
                    byte.as_u64()
                        .and_then(|byte| u8::try_from(byte).ok())
                        .ok_or_else(MongoRuntimeError::unsupported_value)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Bson::Binary(mongodb::bson::Binary {
                subtype: mongodb::bson::spec::BinarySubtype::Generic,
                bytes,
            }))
        }
        "array" | "objectIdArray" => {
            let values = value
                .as_array()
                .ok_or_else(MongoRuntimeError::unsupported_value)?;
            let items = schema
                .get("items")
                .ok_or_else(MongoRuntimeError::invalid_operation)?;
            let tuple = schema.get("tuple").and_then(Value::as_bool) == Some(true);
            let output = if tuple {
                let schemas = items
                    .as_array()
                    .filter(|schemas| schemas.len() == values.len())
                    .ok_or_else(MongoRuntimeError::invalid_model_value)?;
                values
                    .iter()
                    .zip(schemas)
                    .map(|(value, schema)| {
                        json_value_to_bson_with_model_schema(value, schema, definitions, depth + 1)
                    })
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                values
                    .iter()
                    .map(|value| {
                        json_value_to_bson_with_model_schema(value, items, definitions, depth + 1)
                    })
                    .collect::<Result<Vec<_>, _>>()?
            };
            Ok(Bson::Array(output))
        }
        "document" => {
            let values = value
                .as_object()
                .ok_or_else(MongoRuntimeError::unsupported_value)?;
            let fields = schema
                .get("fields")
                .and_then(Value::as_array)
                .ok_or_else(MongoRuntimeError::invalid_operation)?;
            let mut output = Document::new();
            for (name, value) in values {
                let field_schema = fields
                    .iter()
                    .find(|field| field.get("name").and_then(Value::as_str) == Some(name))
                    .and_then(|field| field.get("schema"));
                let converted = match field_schema {
                    Some(field_schema) => json_value_to_bson_with_model_schema(
                        value,
                        field_schema,
                        definitions,
                        depth + 1,
                    )?,
                    None => json_value_to_bson(value, depth + 1)?,
                };
                output.insert(name, converted);
            }
            Ok(Bson::Document(output))
        }
        _ => json_value_to_bson(value, depth),
    }
}

fn apply_model_timestamps(
    document: &mut Document,
    enabled: Option<&Value>,
    names: Option<&Value>,
    updating: bool,
) {
    if enabled.and_then(Value::as_bool) != Some(true) {
        return;
    }
    let field_name = |logical: &str| {
        names
            .and_then(Value::as_object)
            .and_then(|fields| fields.get(logical))
            .and_then(Value::as_str)
            .unwrap_or(logical)
            .to_string()
    };
    let created_at = field_name("createdAt");
    let updated_at = field_name("updatedAt");
    let now = Bson::DateTime(mongodb::bson::DateTime::now());
    if updating {
        document.insert(updated_at, now);
    } else {
        document.entry(created_at).or_insert_with(|| now.clone());
        document.entry(updated_at).or_insert(now);
    }
}

fn validate_document_size(document: &Document) -> Result<(), MongoRuntimeError> {
    let encoded =
        mongodb::bson::to_vec(document).map_err(|_| MongoRuntimeError::invalid_operation())?;
    if encoded.len() > MAX_BSON_DOCUMENT_BYTES {
        return Err(MongoRuntimeError::document_too_large());
    }
    Ok(())
}

fn validate_model_document(
    document: &Document,
    field_types: Option<&Value>,
    require_fields: bool,
) -> Result<(), MongoRuntimeError> {
    let Some(Value::Array(field_types)) = field_types else {
        return Err(MongoRuntimeError::invalid_operation());
    };
    let mut names = HashSet::with_capacity(field_types.len());
    for field in field_types {
        let name = field
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| valid_field_name(name))
            .ok_or_else(MongoRuntimeError::invalid_operation)?;
        let kind = field
            .get("kind")
            .and_then(Value::as_str)
            .filter(|kind| {
                matches!(
                    *kind,
                    "any"
                        | "array"
                        | "boolean"
                        | "bytes"
                        | "document"
                        | "integer"
                        | "instant"
                        | "number"
                        | "objectId"
                        | "objectIdArray"
                        | "string"
                )
            })
            .ok_or_else(MongoRuntimeError::invalid_operation)?;
        let nullable = field
            .get("nullable")
            .and_then(Value::as_bool)
            .ok_or_else(MongoRuntimeError::invalid_operation)?;
        let nullable_items = field
            .get("nullableItems")
            .and_then(Value::as_bool)
            .ok_or_else(MongoRuntimeError::invalid_operation)?;
        let required = field
            .get("required")
            .and_then(Value::as_bool)
            .ok_or_else(MongoRuntimeError::invalid_operation)?;
        if !names.insert(name) {
            return Err(MongoRuntimeError::invalid_operation());
        }
        if require_fields && required && !document.contains_key(name) {
            return Err(MongoRuntimeError::invalid_model_value());
        }
        if let Some(value) = document.get(name) {
            let matches = field
                .get("schema")
                .map(|schema| {
                    let definitions = schema.get("definitions").and_then(Value::as_object);
                    bson_matches_model_schema(value, schema, definitions, 0)
                })
                .unwrap_or_else(|| {
                    (value == &Bson::Null && nullable)
                        || bson_matches_model_kind(value, kind, nullable_items)
                });
            if !matches {
                return Err(MongoRuntimeError::invalid_model_value());
            }
        }
    }
    if document.keys().any(|name| !names.contains(name.as_str())) {
        return Err(MongoRuntimeError::invalid_model_value());
    }
    Ok(())
}

fn validate_model_update(
    changes: &Document,
    field_types: Option<&Value>,
) -> Result<(), MongoRuntimeError> {
    let Some(Value::Array(fields)) = field_types else {
        return Err(MongoRuntimeError::invalid_operation());
    };
    let changes_identifier = changes.contains_key("_id")
        || fields.iter().any(|field| {
            field.get("id").and_then(Value::as_bool) == Some(true)
                && field
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|name| changes.contains_key(name))
        });
    if changes_identifier {
        return Err(MongoRuntimeError::invalid_model_value());
    }
    validate_model_document(changes, field_types, false)
}

fn bson_matches_model_schema(
    value: &Bson,
    schema: &Value,
    definitions: Option<&Map<String, Value>>,
    depth: usize,
) -> bool {
    if depth > 100 {
        return false;
    }
    let Some(kind) = schema.get("kind").and_then(Value::as_str) else {
        return false;
    };
    let Some(nullable) = schema.get("nullable").and_then(Value::as_bool) else {
        return false;
    };
    if matches!(value, Bson::Null) {
        return nullable;
    }
    match kind {
        "reference" => {
            let Some(name) = schema.get("name").and_then(Value::as_str) else {
                return false;
            };
            let Some(definition) = definitions.and_then(|definitions| definitions.get(name)) else {
                return false;
            };
            bson_matches_model_schema(value, definition, definitions, depth + 1)
        }
        "array" | "objectIdArray" => {
            let Bson::Array(values) = value else {
                return false;
            };
            let Some(items) = schema.get("items") else {
                return bson_matches_model_kind(value, kind, false);
            };
            if schema.get("tuple").and_then(Value::as_bool) == Some(true) {
                let Some(item_schemas) = items.as_array() else {
                    return false;
                };
                values.len() == item_schemas.len()
                    && values.iter().zip(item_schemas).all(|(item, item_schema)| {
                        bson_matches_model_schema(item, item_schema, definitions, depth + 1)
                    })
            } else {
                values
                    .iter()
                    .all(|item| bson_matches_model_schema(item, items, definitions, depth + 1))
            }
        }
        "document" => {
            let Bson::Document(document) = value else {
                return false;
            };
            let Some(fields) = schema.get("fields").and_then(Value::as_array) else {
                return true;
            };
            let mut names = HashSet::with_capacity(fields.len());
            for field in fields {
                let Some(name) = field.get("name").and_then(Value::as_str) else {
                    return false;
                };
                let Some(required) = field.get("required").and_then(Value::as_bool) else {
                    return false;
                };
                let Some(field_schema) = field.get("schema") else {
                    return false;
                };
                if !names.insert(name) {
                    return false;
                }
                if let Some(value) = document.get(name) {
                    if !bson_matches_model_schema(value, field_schema, definitions, depth + 1) {
                        return false;
                    }
                } else if required {
                    return false;
                }
            }
            document.keys().all(|name| names.contains(name.as_str()))
        }
        _ => bson_matches_model_kind(value, kind, false),
    }
}

fn bson_matches_model_kind(value: &Bson, kind: &str, nullable_items: bool) -> bool {
    match kind {
        "any" => true,
        "array" => matches!(value, Bson::Array(_)),
        "boolean" => matches!(value, Bson::Boolean(_)),
        "bytes" => matches!(value, Bson::Binary(_)),
        "document" => matches!(value, Bson::Document(_)),
        "integer" => matches!(value, Bson::Int32(_) | Bson::Int64(_)),
        "instant" => matches!(value, Bson::DateTime(_)),
        "number" => matches!(value, Bson::Double(_) | Bson::Int32(_) | Bson::Int64(_)),
        "objectId" => matches!(value, Bson::ObjectId(_)),
        "objectIdArray" => {
            matches!(value, Bson::Array(values) if values.iter().all(|item| matches!(item, Bson::ObjectId(_)) || (nullable_items && matches!(item, Bson::Null))))
        }
        "string" => matches!(value, Bson::String(_)),
        _ => false,
    }
}

fn sql_like_pattern(pattern: &str) -> String {
    let mut output = String::with_capacity(pattern.len() + 2);
    output.push('^');
    for character in pattern.chars() {
        match character {
            '%' => output.push_str(".*"),
            '_' => output.push('.'),
            '\\' | '^' | '$' | '.' | '|' | '?' | '*' | '+' | '(' | ')' | '[' | ']' | '{' | '}' => {
                output.push('\\');
                output.push(character);
            }
            character => output.push(character),
        }
    }
    output.push('$');
    output
}

fn resolve_parameter(value: &Value, parameters: &[DbValue]) -> Result<Bson, MongoRuntimeError> {
    let Some(object) = value.as_object().filter(|object| object.len() == 1) else {
        return Err(MongoRuntimeError::invalid_operation());
    };
    if let Some(index) = object
        .get("$tyrusObjectIdParam")
        .and_then(Value::as_u64)
        .and_then(|index| usize::try_from(index).ok())
    {
        let Some(DbValue::Text(value)) = parameters.get(index) else {
            return Err(MongoRuntimeError::invalid_object_id());
        };
        return mongodb::bson::oid::ObjectId::parse_str(value)
            .map(Bson::ObjectId)
            .map_err(|_| MongoRuntimeError::invalid_object_id());
    }
    if let Some(index) = object
        .get("$tyrusObjectIdArrayParam")
        .and_then(Value::as_u64)
        .and_then(|index| usize::try_from(index).ok())
    {
        let Some(value) = parameters.get(index) else {
            return Err(MongoRuntimeError::invalid_object_id());
        };
        let DbValue::Json(Value::Array(values)) = value else {
            return if matches!(value, DbValue::Null) {
                Ok(Bson::Null)
            } else {
                Err(MongoRuntimeError::invalid_object_id())
            };
        };
        let values = values
            .iter()
            .map(|value| match value {
                Value::String(value) => mongodb::bson::oid::ObjectId::parse_str(value)
                    .map(Bson::ObjectId)
                    .map_err(|_| MongoRuntimeError::invalid_object_id()),
                Value::Null => Ok(Bson::Null),
                _ => Err(MongoRuntimeError::invalid_object_id()),
            })
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(Bson::Array(values));
    }
    let Some(index) = object
        .get("$tyrusParam")
        .and_then(Value::as_u64)
        .and_then(|index| usize::try_from(index).ok())
    else {
        return Err(MongoRuntimeError::invalid_operation());
    };
    parameters
        .get(index)
        .map(db_value_to_bson)
        .transpose()?
        .ok_or_else(MongoRuntimeError::invalid_operation)
}

fn db_value_to_bson(value: &DbValue) -> Result<Bson, MongoRuntimeError> {
    match value {
        DbValue::Null => Ok(Bson::Null),
        DbValue::Bool(value) => Ok(Bson::Boolean(*value)),
        DbValue::Signed(value) => Ok(Bson::Int64(*value)),
        DbValue::Unsigned(value) => i64::try_from(*value)
            .map(Bson::Int64)
            .map_err(|_| MongoRuntimeError::unsupported_value()),
        DbValue::Real(value) if value.is_finite() => Ok(Bson::Double(*value)),
        DbValue::Text(value) => Ok(Bson::String(value.clone())),
        DbValue::Instant {
            seconds,
            nanoseconds,
        } => {
            if nanoseconds % 1_000_000 != 0 {
                return Err(MongoRuntimeError::unsupported_value());
            }
            let timestamp = jiff::Timestamp::new(
                *seconds,
                i32::try_from(*nanoseconds).map_err(|_| MongoRuntimeError::unsupported_value())?,
            )
            .map_err(|_| MongoRuntimeError::unsupported_value())?;
            Ok(Bson::DateTime(mongodb::bson::DateTime::from_millis(
                timestamp.as_millisecond(),
            )))
        }
        DbValue::Blob(value) => Ok(Bson::Binary(mongodb::bson::Binary {
            subtype: mongodb::bson::spec::BinarySubtype::Generic,
            bytes: value.clone(),
        })),
        DbValue::Json(value) => json_value_to_bson(value, 0),
        DbValue::Real(_) => Err(MongoRuntimeError::unsupported_value()),
    }
}

fn json_value_to_bson(value: &Value, depth: usize) -> Result<Bson, MongoRuntimeError> {
    if depth > MAX_BSON_NESTING_DEPTH {
        return Err(MongoRuntimeError::nesting_too_deep());
    }
    match value {
        Value::Null => Ok(Bson::Null),
        Value::Bool(value) => Ok(Bson::Boolean(*value)),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Ok(Bson::Int64(value))
            } else if let Some(value) = value.as_u64() {
                i64::try_from(value)
                    .map(Bson::Int64)
                    .map_err(|_| MongoRuntimeError::unsupported_value())
            } else {
                value
                    .as_f64()
                    .filter(|value| value.is_finite())
                    .map(Bson::Double)
                    .ok_or_else(MongoRuntimeError::unsupported_value)
            }
        }
        Value::String(value) => Ok(Bson::String(value.clone())),
        Value::Array(values) => values
            .iter()
            .map(|value| json_value_to_bson(value, depth + 1))
            .collect::<Result<Vec<_>, _>>()
            .map(Bson::Array),
        Value::Object(values) => {
            let mut document = Document::new();
            for (key, value) in values {
                if !valid_field_name(key) {
                    return Err(MongoRuntimeError::invalid_operation());
                }
                document.insert(key, json_value_to_bson(value, depth + 1)?);
            }
            Ok(Bson::Document(document))
        }
    }
}

fn document_to_json(
    document: &Document,
    mappings: &[MongoFieldMapping],
) -> Result<Value, MongoRuntimeError> {
    let mut object = Map::new();
    for field in mappings {
        if let Some(value) = document.get(&field.document) {
            if *value == Bson::Null && !field.nullable {
                return Err(MongoRuntimeError::invalid_model_value());
            }
            if field.object_id && !matches!(value, Bson::ObjectId(_) | Bson::Null) {
                return Err(MongoRuntimeError::unsupported_value());
            }
            if field.object_id_array
                && !matches!(value, Bson::Array(items) if items.iter().all(|item| matches!(item, Bson::ObjectId(_)) || (field.nullable_items && matches!(item, Bson::Null))))
            {
                return Err(MongoRuntimeError::unsupported_value());
            }
            if let Some(kind) = field.kind.as_deref()
                && !(value == &Bson::Null && field.nullable)
                && !bson_matches_model_kind(value, kind, field.nullable_items)
            {
                return Err(MongoRuntimeError::invalid_model_value());
            }
            if let Some(schema) = &field.schema {
                let definitions = schema.get("definitions").and_then(Value::as_object);
                if !bson_matches_model_schema(value, schema, definitions, 0) {
                    return Err(MongoRuntimeError::invalid_model_value());
                }
            }
            object.insert(field.logical.clone(), bson_to_json(value)?);
        } else if !field.nullable {
            return Err(MongoRuntimeError::invalid_model_value());
        }
    }
    Ok(Value::Object(object))
}

fn bson_to_json(value: &Bson) -> Result<Value, MongoRuntimeError> {
    bson_to_json_at_depth(value, 0)
}

fn bson_to_json_at_depth(value: &Bson, depth: usize) -> Result<Value, MongoRuntimeError> {
    if depth > MAX_BSON_NESTING_DEPTH {
        return Err(MongoRuntimeError::nesting_too_deep());
    }
    match value {
        Bson::Double(value) if value.is_finite() => serde_json::Number::from_f64(*value)
            .map(Value::Number)
            .ok_or_else(MongoRuntimeError::unsupported_value),
        Bson::Double(_) => Err(MongoRuntimeError::unsupported_value()),
        Bson::String(value) => Ok(Value::String(value.clone())),
        Bson::Array(values) => values
            .iter()
            .map(|value| bson_to_json_at_depth(value, depth + 1))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Bson::Document(values) => {
            let mut output = Map::new();
            for (key, value) in values {
                output.insert(key.clone(), bson_to_json_at_depth(value, depth + 1)?);
            }
            Ok(Value::Object(output))
        }
        Bson::Boolean(value) => Ok(Value::Bool(*value)),
        Bson::Null => Ok(Value::Null),
        Bson::Int32(value) => Ok(Value::Number((*value).into())),
        Bson::Int64(value) => Ok(Value::Number((*value).into())),
        Bson::Binary(value) => Ok(Value::Array(
            value
                .bytes
                .iter()
                .map(|byte| Value::Number(u64::from(*byte).into()))
                .collect(),
        )),
        Bson::ObjectId(value) => Ok(Value::String(value.to_hex())),
        Bson::DateTime(value) => {
            let milliseconds = value.timestamp_millis();
            let mut instant = Map::new();
            instant.insert(
                "seconds".to_string(),
                Value::Number(milliseconds.div_euclid(1_000).into()),
            );
            instant.insert(
                "nanoseconds".to_string(),
                Value::Number(
                    u64::try_from(milliseconds.rem_euclid(1_000) * 1_000_000)
                        .map_err(|_| MongoRuntimeError::invalid_temporal_value())?
                        .into(),
                ),
            );
            Ok(Value::Object(instant))
        }
        _ => Err(MongoRuntimeError::unsupported_value()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_database(acquire_timeout_ms: u64, query_timeout_ms: u64) -> MongoDatabase {
        let options = ClientOptions::parse("mongodb://localhost:27017/tyrus_test")
            .await
            .unwrap();
        MongoDatabase {
            client: Client::with_options(options).unwrap(),
            _client_identity_file: None,
            operation_slots: Arc::new(Semaphore::new(1)),
            initialized_collections: Arc::new(tokio::sync::Mutex::new(HashSet::new())),
            acquire_timeout_ms,
            query_timeout_ms,
        }
    }

    #[test]
    fn mongo_pool_options_validate_every_supported_limit() {
        assert!(MongoPoolOptions::default().validate());
        for options in [
            serde_json::json!({"maxConnections": 0}),
            serde_json::json!({"maxConnections": 101}),
            serde_json::json!({"connectTimeoutMs": 0}),
            serde_json::json!({"connectTimeoutMs": 120_001}),
            serde_json::json!({"acquireTimeoutMs": 0}),
            serde_json::json!({"queryTimeoutMs": 120_001}),
            serde_json::json!({"idleTimeoutMs": 86_400_001}),
        ] {
            assert!(
                !MongoPoolOptions::from_json(&options)
                    .expect("known numeric pool option should parse")
                    .validate(),
                "accepted invalid MongoDB pool options: {options}"
            );
        }
        for options in [
            serde_json::json!({"maxLifetimeMs": 1000}),
            serde_json::json!({"queryTimeoutMs": -1}),
            serde_json::json!({"queryTimeoutMs": 1.5}),
            serde_json::json!({"unknown": true}),
            serde_json::json!([]),
        ] {
            assert!(
                MongoPoolOptions::from_json(&options).is_none(),
                "accepted unsupported MongoDB pool options: {options}"
            );
        }
        assert!(
            MongoPoolOptions::from_json(&serde_json::json!({
                "maxConnections": 100,
                "connectTimeoutMs": 120_000,
                "acquireTimeoutMs": 120_000,
                "queryTimeoutMs": 120_000,
                "idleTimeoutMs": 86_400_000
            }))
            .unwrap()
            .validate()
        );
    }

    #[test]
    fn mongo_bson_document_size_and_nesting_limits_are_enforced_at_boundaries() {
        let accepted = Document::from_iter([(
            "x".to_string(),
            Bson::String("x".repeat(MAX_BSON_DOCUMENT_BYTES - 13)),
        )]);
        assert_eq!(
            mongodb::bson::to_vec(&accepted).unwrap().len(),
            MAX_BSON_DOCUMENT_BYTES
        );
        assert!(validate_document_size(&accepted).is_ok());

        let oversized = Document::from_iter([(
            "x".to_string(),
            Bson::String("x".repeat(MAX_BSON_DOCUMENT_BYTES - 12)),
        )]);
        assert_eq!(
            validate_document_size(&oversized).unwrap_err().code,
            "ERR_DB_DOCUMENT_TOO_LARGE"
        );

        let nested_value = |levels: usize| {
            (0..levels).fold(
                Value::String("leaf".to_string()),
                |value, _| serde_json::json!({"child": value}),
            )
        };
        assert!(json_value_to_bson(&nested_value(MAX_BSON_NESTING_DEPTH), 0).is_ok());
        assert_eq!(
            json_value_to_bson(&nested_value(MAX_BSON_NESTING_DEPTH + 1), 0)
                .unwrap_err()
                .code,
            "ERR_DB_VALUE_TOO_DEEP"
        );
    }

    #[tokio::test]
    async fn mongo_operation_timeouts_cancel_futures_and_release_concurrency_slots() {
        struct DropFlag(Arc<std::sync::atomic::AtomicBool>);

        impl Drop for DropFlag {
            fn drop(&mut self) {
                self.0.store(true, std::sync::atomic::Ordering::Release);
            }
        }

        let database = test_database(10, 10).await;
        let held = database.acquire_operation_slot().await.unwrap();
        let error = database.acquire_operation_slot().await.unwrap_err();
        assert_eq!(error.code, "ERR_DB_POOL_TIMEOUT");
        drop(held);

        let future_dropped = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let drop_flag = DropFlag(future_dropped.clone());
        let error = database
            .with_operation_slot(async move {
                let _drop_flag = drop_flag;
                std::future::pending::<Result<(), MongoRuntimeError>>().await
            })
            .await
            .unwrap_err();
        assert_eq!(error.code, "ERR_DB_QUERY_TIMEOUT");
        assert!(future_dropped.load(std::sync::atomic::Ordering::Acquire));

        let recovered = database.acquire_operation_slot().await;
        assert!(recovered.is_ok(), "timed-out operation leaked its slot");
    }

    #[test]
    fn mongo_tls_files_require_existing_nonempty_files_and_complete_identity_pairs() {
        let mut ca = NamedTempFile::new().unwrap();
        let mut cert = NamedTempFile::new().unwrap();
        let mut key = NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut ca, b"ca").unwrap();
        std::io::Write::write_all(&mut cert, b"certificate").unwrap();
        std::io::Write::write_all(&mut key, b"private key").unwrap();
        assert!(
            validate_tls_files(
                Some(ca.path().to_str().unwrap()),
                Some(cert.path().to_str().unwrap()),
                Some(key.path().to_str().unwrap())
            )
            .is_ok()
        );
        assert_eq!(
            validate_tls_files(None, Some(cert.path().to_str().unwrap()), None)
                .unwrap_err()
                .code,
            MongoRuntimeError::tls_configuration().code
        );
        assert_eq!(
            validate_tls_files(Some("/missing/tyrus-test-ca.pem"), None, None)
                .unwrap_err()
                .code,
            MongoRuntimeError::tls_configuration().code
        );

        let empty = NamedTempFile::new().unwrap();
        assert_eq!(
            validate_tls_files(Some(empty.path().to_str().unwrap()), None, None)
                .unwrap_err()
                .code,
            MongoRuntimeError::tls_configuration().code
        );

        let oversized = NamedTempFile::new().unwrap();
        oversized.as_file().set_len(MAX_TLS_FILE_BYTES + 1).unwrap();
        assert_eq!(
            validate_tls_files(Some(oversized.path().to_str().unwrap()), None, None)
                .unwrap_err()
                .code,
            MongoRuntimeError::tls_configuration().code
        );
    }

    #[tokio::test]
    async fn mongo_tls_is_optional_and_keeps_mtls_material_alive() {
        let mut plaintext = ClientOptions::parse("mongodb://localhost:27017/tyrus_test?tls=false")
            .await
            .unwrap();
        configure_tls(&mut plaintext, None, None, None).unwrap();
        assert!(matches!(plaintext.tls, Some(Tls::Disabled)));

        let mut insecure = ClientOptions::parse(
            "mongodb://localhost:27017/tyrus_test?tlsAllowInvalidCertificates=true",
        )
        .await
        .unwrap();
        assert_eq!(
            configure_tls(&mut insecure, None, None, None)
                .unwrap_err()
                .code,
            MongoRuntimeError::tls_configuration().code
        );

        let mut defaults = ClientOptions::parse("mongodb://localhost:27017/tyrus_test")
            .await
            .unwrap();
        configure_tls(&mut defaults, None, None, None).unwrap();
        assert!(defaults.tls.is_none());

        let directory = tempfile::tempdir().unwrap();
        let ca_path = directory.path().join("ca.pem");
        let cert_path = directory.path().join("client.pem");
        let key_path = directory.path().join("client.key");
        std::fs::write(&ca_path, b"test CA").unwrap();
        std::fs::write(&cert_path, b"test certificate").unwrap();
        std::fs::write(&key_path, b"test private key").unwrap();
        let ca = ca_path.to_str().unwrap();
        let cert = cert_path.to_str().unwrap();
        let key = key_path.to_str().unwrap();
        let mut options = ClientOptions::parse("mongodb://localhost:27017/tyrus_test")
            .await
            .unwrap();
        let identity = configure_tls(&mut options, Some(ca), Some(cert), Some(key))
            .unwrap()
            .expect("mTLS configuration should retain its combined identity file");
        let identity_path = options
            .tls
            .as_ref()
            .and_then(|tls| match tls {
                Tls::Enabled(options) => options.cert_key_file_path.as_ref(),
                Tls::Disabled => None,
            })
            .expect("mTLS identity path should be passed to the driver");
        assert_eq!(identity.path(), identity_path);
        let contents = std::fs::read_to_string(identity_path).unwrap();
        assert!(contents.contains("test certificate"));
        assert!(contents.contains("test private key"));
    }

    #[tokio::test]
    async fn mongo_connection_urls_reject_invalid_schemes_and_missing_database_names() {
        for (connection_string, expected_code) in [
            ("not-a-mongodb-url", "ERR_DB_INVALID_CONNECTION"),
            (
                "mongodb://localhost:27017/?directConnection=true",
                "ERR_DB_INVALID_CONNECTION",
            ),
        ] {
            let result = MongoDatabase::open(
                connection_string,
                MongoPoolOptions::default(),
                None,
                None,
                None,
            )
            .await;
            match result {
                Err(error) => assert_eq!(error.code, expected_code),
                Ok(_) => panic!("invalid MongoDB connection string was accepted"),
            }
        }
    }

    #[test]
    fn mongo_names_reject_operator_injection_and_system_collections() {
        assert!(valid_collection_name("users"));
        assert!(!valid_collection_name("system.users"));
        assert!(!valid_collection_name("users$archive"));
        assert!(valid_field_name("user_name"));
        assert!(!valid_field_name("$where"));
        assert!(!valid_field_name("profile.email"));

        let malicious = serde_json::json!({"$where": {"$tyrusParam": 0}});
        let error = resolve_filter(
            malicious.as_object().unwrap(),
            &[DbValue::Text("true".into())],
        )
        .unwrap_err();
        assert_eq!(error.code, MongoRuntimeError::invalid_operation().code);
    }

    #[test]
    fn mongo_filters_bind_values_and_translate_like_wildcards() {
        let filter = serde_json::json!({
            "$or": [
                {"age": {"$gte": {"$tyrusParam": 0}}},
                {"name": {"$tyrusLike": {"$tyrusParam": 1}}}
            ]
        });
        let resolved = resolve_filter(
            filter.as_object().unwrap(),
            &[DbValue::Signed(18), DbValue::Text("A_d%".into())],
        )
        .unwrap();
        let branches = resolved.get_array("$or").unwrap();
        assert_eq!(branches.len(), 2);
        let first = branches[0].as_document().unwrap();
        assert_eq!(first.get_document("age").unwrap().get_i64("$gte"), Ok(18));
        let second = branches[1].as_document().unwrap();
        let Bson::RegularExpression(regex) =
            second.get_document("name").unwrap().get("$regex").unwrap()
        else {
            panic!("LIKE filter did not produce a regular expression");
        };
        assert_eq!(regex.pattern, "^A.d.*$");
        assert_eq!(sql_like_pattern(r"a.%_\"), r"^a\..*.\\$");
    }

    #[test]
    fn mongo_model_validator_requires_generated_object_ids_and_required_fields() {
        let field_types = serde_json::json!([
            {"name": "_id", "kind": "objectId", "required": true, "nullable": false},
            {"name": "email", "kind": "string", "required": true, "nullable": false},
            {"name": "nickname", "kind": "string", "required": false, "nullable": true}
        ]);
        let validator = model_collection_validator(Some(&field_types)).unwrap();
        let schema = validator.get_document("$jsonSchema").unwrap();
        let required = schema.get_array("required").unwrap();
        assert!(required.contains(&Bson::String("_id".into())));
        assert!(required.contains(&Bson::String("email".into())));
        assert!(!required.contains(&Bson::String("nickname".into())));
        assert_eq!(
            schema
                .get_document("properties")
                .unwrap()
                .get_document("_id")
                .unwrap()
                .get_str("bsonType"),
            Ok("objectId")
        );
        assert_eq!(schema.get_bool("additionalProperties"), Ok(false));

        let duplicate_fields = serde_json::json!([
            {"name": "email", "kind": "string", "required": true, "nullable": false},
            {"name": "email", "kind": "string", "required": true, "nullable": false}
        ]);
        assert!(model_collection_validator(Some(&duplicate_fields)).is_err());
    }

    #[test]
    fn mongo_runtime_rejects_updates_to_generated_and_declared_identifiers() {
        let fields = serde_json::json!([
            {"name": "_id", "id": false, "kind": "objectId", "required": false, "nullable": false, "nullableItems": false},
            {"name": "user_id", "id": true, "kind": "integer", "required": false, "nullable": false, "nullableItems": false},
            {"name": "displayName", "id": false, "kind": "string", "required": false, "nullable": false, "nullableItems": false}
        ]);

        for changes in [
            doc! { "_id": "507f1f77bcf86cd799439011" },
            doc! { "user_id": 42 },
        ] {
            assert_eq!(
                validate_model_update(&changes, Some(&fields))
                    .unwrap_err()
                    .code,
                MongoRuntimeError::invalid_model_value().code
            );
        }

        assert!(validate_model_update(&doc! { "displayName": "Ada" }, Some(&fields)).is_ok());
    }

    #[test]
    fn mongo_schema_conversion_preserves_object_ids_and_instant_milliseconds() {
        let id = "507f1f77bcf86cd799439011";
        let object_id = db_value_to_bson_with_model_schema(
            &DbValue::Text(id.into()),
            &serde_json::json!({"kind": "objectId"}),
        )
        .unwrap();
        assert_eq!(object_id.as_object_id().unwrap().to_hex(), id);
        assert!(
            db_value_to_bson_with_model_schema(
                &DbValue::Text("not-an-object-id".into()),
                &serde_json::json!({"kind": "objectId"}),
            )
            .is_err()
        );

        let instant = serde_json::json!({"seconds": 1_735_689_600, "nanoseconds": 123_000_000});
        let bson_date = json_value_to_bson_with_model_schema(
            &instant,
            &serde_json::json!({"kind": "instant"}),
            None,
            0,
        )
        .unwrap();
        assert_eq!(
            bson_date.as_datetime().unwrap().timestamp_millis(),
            1_735_689_600_123
        );
        let submillisecond = serde_json::json!({"seconds": 1, "nanoseconds": 1});
        assert_eq!(
            json_value_to_bson_with_model_schema(
                &submillisecond,
                &serde_json::json!({"kind": "instant"}),
                None,
                0,
            )
            .unwrap_err()
            .code,
            MongoRuntimeError::temporal_precision_loss().code
        );
    }

    #[test]
    fn mongo_field_mapping_restores_logical_object_id_and_timestamp_names() {
        let mappings = parse_field_mappings(Some(&serde_json::json!([
            {"logical": "userId", "document": "user_id", "objectId": true, "kind": "objectId"}
        ])))
        .unwrap();
        let id = mongodb::bson::oid::ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap();
        let value = document_to_json(&doc! {"user_id": id}, &mappings).unwrap();
        assert_eq!(value["userId"], "507f1f77bcf86cd799439011");

        let mut inserted = Document::new();
        apply_model_timestamps(&mut inserted, Some(&Value::Bool(true)), None, false);
        let created_at = inserted.get_datetime("createdAt").unwrap().to_owned();
        assert_eq!(inserted.get_datetime("updatedAt"), Ok(&created_at));

        let mut changed = doc! {"created_at": "preserved"};
        apply_model_timestamps(
            &mut changed,
            Some(&Value::Bool(true)),
            Some(&serde_json::json!({"createdAt": "created_at", "updatedAt": "updated_at"})),
            true,
        );
        assert_eq!(changed.get_str("created_at"), Ok("preserved"));
        assert!(changed.get_datetime("updated_at").is_ok());
    }

    #[tokio::test]
    async fn mongo_env_backend_runs_model_crud_timestamps_and_cursor_pages() {
        let Ok(connection_string) = std::env::var("TYRUS_TEST_MONGODB_URL") else {
            eprintln!("Skipping MongoDB integration: TYRUS_TEST_MONGODB_URL is unset");
            return;
        };
        let database = MongoDatabase::open(
            &connection_string,
            MongoPoolOptions::default(),
            None,
            None,
            None,
        )
        .await
        .unwrap();
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let collection = format!("tyrus_db_check_{}_{}", std::process::id(), suffix);
        let field_types = serde_json::json!([
            {"name": "_id", "kind": "objectId", "required": true, "nullable": false},
            {"name": "displayName", "kind": "string", "required": true, "nullable": false, "schema": {"kind": "string", "nullable": false}},
            {"name": "category", "kind": "string", "required": false, "nullable": true, "schema": {"kind": "string", "nullable": true}},
            {"name": "createdAt", "kind": "instant", "required": true, "nullable": false, "schema": {"kind": "instant", "nullable": false}},
            {"name": "updatedAt", "kind": "instant", "required": true, "nullable": false, "schema": {"kind": "instant", "nullable": false}}
        ]);
        let fields = serde_json::json!([
            {"logical": "_id", "document": "_id", "objectId": true, "kind": "objectId", "nullable": false},
            {"logical": "displayName", "document": "displayName", "kind": "string", "nullable": false},
            {"logical": "category", "document": "category", "kind": "string", "nullable": true},
            {"logical": "createdAt", "document": "createdAt", "kind": "instant", "nullable": false},
            {"logical": "updatedAt", "document": "updatedAt", "kind": "instant", "nullable": false}
        ]);
        let result = async {
            let insert_one = serde_json::json!({
                "collection": collection,
                "kind": "insertOne",
                "document": {"displayName": {"$tyrusParam": 0}},
                "fieldTypes": field_types,
                "timestamps": true
            });
            let inserted = database
                .mutate(
                    &insert_one.to_string(),
                    &[DbValue::Text("first".to_string())],
                )
                .await?;
            assert_eq!(inserted["rowsAffected"], 1);

            let insert_many = serde_json::json!({
                "collection": collection,
                "kind": "insertMany",
                "documents": [
                    {"displayName": {"$tyrusParam": 0}}
                ],
                "fieldTypes": field_types,
                "timestamps": true
            });
            let inserted = database
                .mutate(
                    &insert_many.to_string(),
                    &[DbValue::Text("second".to_string())],
                )
                .await?;
            assert_eq!(inserted["rowsAffected"], 1);

            let count = serde_json::json!({
                "collection": collection,
                "kind": "count",
                "filter": {}
            });
            assert_eq!(
                database
                    .read(MongoReadOperation::Count, &count.to_string(), &[])
                    .await?,
                serde_json::json!(2)
            );

            let cursor_descriptor = serde_json::json!({
                "collection": collection,
                "kind": "findMany",
                "filter": {},
                "fields": fields
            });
            let first_page = database
                .read(
                    MongoReadOperation::CursorPage {
                        page_size: 1,
                        after: String::new(),
                    },
                    &cursor_descriptor.to_string(),
                    &[],
                )
                .await?;
            assert_eq!(first_page["total"], 2);
            assert_eq!(first_page["items"].as_array().unwrap().len(), 1);
            assert_eq!(first_page["hasMore"], true);
            let item = &first_page["items"][0];
            assert_eq!(item["_id"].as_str().unwrap().len(), 24);
            assert!(item["createdAt"]["seconds"].is_i64());
            let cursor = first_page["nextCursor"].as_str().unwrap().to_string();

            let second_page = database
                .read(
                    MongoReadOperation::CursorPage {
                        page_size: 1,
                        after: cursor,
                    },
                    &cursor_descriptor.to_string(),
                    &[],
                )
                .await?;
            assert_eq!(second_page["items"].as_array().unwrap().len(), 1);
            assert_eq!(second_page["hasMore"], false);

            let find_one = serde_json::json!({
                "collection": collection,
                "kind": "findOne",
                "filter": {"displayName": {"$eq": {"$tyrusParam": 0}}},
                "fields": fields
            });
            let found = database
                .read(
                    MongoReadOperation::One,
                    &find_one.to_string(),
                    &[DbValue::Text("first".to_string())],
                )
                .await?;
            assert_eq!(found["displayName"], "first");

            let find_many = serde_json::json!({
                "collection": collection,
                "kind": "findMany",
                "filter": {},
                "fields": fields
            });
            let found = database
                .read(MongoReadOperation::Many, &find_many.to_string(), &[])
                .await?;
            assert_eq!(found.as_array().map(Vec::len), Some(2));

            let update = serde_json::json!({
                "collection": collection,
                "kind": "updateOne",
                "filter": {"displayName": {"$eq": {"$tyrusParam": 0}}},
                "changes": {"displayName": {"$tyrusParam": 1}},
                "fieldTypes": field_types,
                "timestamps": true
            });
            let updated = database
                .mutate(
                    &update.to_string(),
                    &[
                        DbValue::Text("first".to_string()),
                        DbValue::Text("renamed".to_string()),
                    ],
                )
                .await?;
            assert_eq!(updated["rowsAffected"], 1);

            let update_many = serde_json::json!({
                "collection": collection,
                "kind": "updateMany",
                "filter": {},
                "changes": {"category": {"$tyrusParam": 0}},
                "fieldTypes": field_types,
                "timestamps": true
            });
            let updated = database
                .mutate(
                    &update_many.to_string(),
                    &[DbValue::Text("integration".to_string())],
                )
                .await?;
            assert_eq!(updated["rowsAffected"], 2);

            let delete_one = serde_json::json!({
                "collection": collection,
                "kind": "deleteOne",
                "filter": {"displayName": {"$eq": {"$tyrusParam": 0}}}
            });
            let deleted = database
                .mutate(
                    &delete_one.to_string(),
                    &[DbValue::Text("renamed".to_string())],
                )
                .await?;
            assert_eq!(deleted["rowsAffected"], 1);

            let delete = serde_json::json!({
                "collection": collection,
                "kind": "deleteMany",
                "filter": {"displayName": {"$ne": {"$tyrusParam": 0}}}
            });
            let deleted = database
                .mutate(&delete.to_string(), &[DbValue::Text("keep".to_string())])
                .await?;
            assert_eq!(deleted["rowsAffected"], 1);
            Ok::<(), MongoRuntimeError>(())
        }
        .await;
        let cleanup = database
            .client
            .default_database()
            .expect("MongoDB URL was validated at open")
            .collection::<Document>(&collection)
            .drop()
            .await;
        database.close().await;
        result.unwrap_or_else(|error| panic!("MongoDB integration failed: {error:?}"));
        cleanup.expect("MongoDB test collection cleanup failed");
    }
}
