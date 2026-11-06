use super::*;
use crate::backend::InitializationOptions;

#[path = "archive_fixes/archive_fixes.rs"]
mod archive_fixes;

impl Backend {
    async fn add_alias_import_sources(&self, uri: &Url, exports: &mut [IndexedExport]) {
        let Some(root) = self.project_root_for(uri).await else {
            return;
        };
        let Ok(import_map) = self.import_map_for_root(&root).await else {
            return;
        };
        for export in exports {
            if let Ok(path) = export.uri.to_file_path() {
                export.module_source = import_map.source_for_path(&root, &path);
            }
        }
    }
}

async fn quick_fixes_for_diagnostics_on_worker(
    semaphore: Arc<tokio::sync::Semaphore>,
    uri: &Url,
    source: Arc<String>,
    current_path: PathBuf,
    exports: Vec<IndexedExport>,
    lint_diagnostics: Vec<tyrus_lint::LintDiagnostic>,
    diagnostics: Vec<Diagnostic>,
    analysis: Option<Arc<tyrus_core::analysis::DocumentAnalysis>>,
) -> Vec<CodeActionOrCommand> {
    let queue_started = std::time::Instant::now();
    let Ok(permit) = semaphore.acquire_owned().await else {
        return Vec::new();
    };
    trace_lsp_stage("quick_fix_worker_queue", queue_started);
    let uri = uri.clone();
    let generation_started = std::time::Instant::now();
    let actions = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        quick_fixes_for_diagnostics_with_analysis(
            &uri,
            source.as_str(),
            &current_path,
            &exports,
            &lint_diagnostics,
            &diagnostics,
            analysis.as_deref(),
        )
    })
    .await
    .unwrap_or_default();
    trace_lsp_stage("quick_fix_generation", generation_started);
    actions
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let _initialize_span = trace_lsp_span("initialize_request");
        let workspace_roots_started = std::time::Instant::now();
        let watched_files_dynamic_registration = params
            .capabilities
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.did_change_watched_files.as_ref())
            .and_then(|capabilities| capabilities.dynamic_registration)
            .unwrap_or(false);
        self.watched_files_dynamic_registration.store(
            watched_files_dynamic_registration,
            std::sync::atomic::Ordering::Release,
        );
        let mut roots = params
            .workspace_folders
            .as_ref()
            .into_iter()
            .flatten()
            .filter_map(|folder| folder.uri.to_file_path().ok())
            .collect::<Vec<_>>();
        if roots.is_empty()
            && let Some(root) = params.root_uri.and_then(|uri| uri.to_file_path().ok())
        {
            roots.push(root);
        }
        let root = roots.first().cloned();
        *self.workspace_root.write().await = root;
        *self.workspace_roots.write().await = roots;
        self.invalidate_project_roots().await;
        trace_lsp_stage("initialize_workspace_roots", workspace_roots_started);

        let options_started = std::time::Instant::now();
        let options = InitializationOptions::parse(&params.initialization_options);
        let format_enable = options.format_enable;
        self.diagnostic_debounce_ms.store(
            options.diagnostic_debounce_ms,
            std::sync::atomic::Ordering::Release,
        );
        self.workspace_scan_max_files.store(
            options.workspace_max_files,
            std::sync::atomic::Ordering::Release,
        );
        self.workspace_scan_max_depth.store(
            options.workspace_max_depth,
            std::sync::atomic::Ordering::Release,
        );
        *self.workspace_scan_excluded_directories.write().await =
            options.workspace_exclude_directories;
        trace_lsp_stage("initialize_options", options_started);

        let capabilities_started = std::time::Instant::now();
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    retrigger_characters: None,
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![
                        ".".to_string(),
                        "@".to_string(),
                        "(".to_string(),
                        "\"".to_string(),
                    ]),
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(true),
                    },
                    ..CompletionOptions::default()
                }),
                inlay_hint_provider: Some(OneOf::Left(true)),
                document_formatting_provider: format_enable.then_some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        work_done_progress_options: WorkDoneProgressOptions {
                            work_done_progress: Some(true),
                        },
                        ..CodeActionOptions::default()
                    },
                )),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["tyrus.db.explainSql".to_string()],
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                code_lens_provider: options.code_lens_quick_fixes_enable.then_some(
                    CodeLensOptions {
                        resolve_provider: Some(false),
                    },
                ),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: vec![
                                    SemanticTokenType::PARAMETER,
                                    SemanticTokenType::KEYWORD,
                                ],
                                token_modifiers: vec![],
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: None,
                            work_done_progress_options: WorkDoneProgressOptions::default(),
                        },
                    ),
                ),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    ..WorkspaceServerCapabilities::default()
                }),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "tyrus-language-server".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
        .inspect(|_| trace_lsp_stage("initialize_capabilities", capabilities_started))
    }

    async fn initialized(&self, _: InitializedParams) {
        let _initialized_span = trace_lsp_span("initialized_notification");
        let watchers_started = std::time::Instant::now();
        if self
            .watched_files_dynamic_registration
            .load(std::sync::atomic::Ordering::Acquire)
        {
            let watchers = [
                "**/*.tyr",
                "**/tyrus.json",
                "**/tyrus-lint.json",
                "**/tyrus-fmt.json",
                "**/.env",
                "**/.env.*",
            ]
            .into_iter()
            .map(|pattern| FileSystemWatcher {
                glob_pattern: GlobPattern::String(pattern.to_string()),
                kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
            })
            .collect();
            let registration = Registration {
                id: "tyrus.workspace-file-watcher".to_string(),
                method: "workspace/didChangeWatchedFiles".to_string(),
                register_options: serde_json::to_value(DidChangeWatchedFilesRegistrationOptions {
                    watchers,
                })
                .ok(),
            };
            if let Err(error) = self.client.register_capability(vec![registration]).await {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        format!("Could not register workspace file watchers: {error}"),
                    )
                    .await;
            }
        }
        trace_lsp_stage("initialized_watch_registration", watchers_started);
        let environment_started = std::time::Instant::now();
        self.reload_env().await;
        trace_lsp_stage("initialized_environment_reload", environment_started);
        self.client
            .log_message(MessageType::INFO, "tyrus Language Server initialized")
            .await;
    }

    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        let removed = params
            .event
            .removed
            .iter()
            .filter_map(|folder| folder.uri.to_file_path().ok())
            .collect::<std::collections::HashSet<_>>();
        let added = params
            .event
            .added
            .into_iter()
            .filter_map(|folder| folder.uri.to_file_path().ok())
            .collect::<Vec<_>>();
        let mut roots = self.workspace_roots.write().await;
        backend::update_workspace_roots(&mut roots, &removed, added.clone());
        let next_root = roots.first().cloned();
        drop(roots);
        *self.workspace_root.write().await = next_root;
        self.invalidate_project_roots().await;
        self.import_dependencies.write().await.clear();
        if !removed.is_empty() {
            self.detach_workspace_roots(&removed).await;
            self.workspace_analysis_cache
                .write()
                .await
                .retain(|uri, _| {
                    uri.to_file_path()
                        .ok()
                        .is_none_or(|path| !removed.iter().any(|root| path.starts_with(root)))
                });
        }
        if let Ok(mut cache) = self.format_config_cache.write() {
            cache.clear();
        }
        if let Ok(mut cache) = self.lint_config_cache.write() {
            cache.clear();
        }
        if let Ok(mut cache) = self.import_map_cache.write() {
            cache.clear();
        }
        if let Ok(mut cache) = self.project_database_config_cache.write() {
            cache.clear();
        }
        let roots = self.workspace_roots.read().await.clone();
        let removed_schema_watchers = {
            let mut watchers = self.schema_file_watchers.lock().await;
            let removed = watchers
                .iter()
                .filter(|(root, _)| !roots.iter().any(|workspace| root.starts_with(workspace)))
                .map(|(_, (_, registration_id))| registration_id.clone())
                .collect::<Vec<_>>();
            watchers.retain(|root, _| roots.iter().any(|workspace| root.starts_with(workspace)));
            removed
        };
        for registration_id in removed_schema_watchers {
            if let Err(error) = self
                .client
                .unregister_capability(vec![Unregistration {
                    id: registration_id,
                    method: "workspace/didChangeWatchedFiles".to_string(),
                }])
                .await
            {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        format!("Could not unregister database schema watcher: {error}"),
                    )
                    .await;
            }
        }
        let changed_roots = removed.iter().chain(added.iter()).collect::<Vec<_>>();
        let affected_roots = roots
            .iter()
            .filter(|root| {
                changed_roots
                    .iter()
                    .any(|changed| root.starts_with(changed) || changed.starts_with(root))
            })
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        for root in affected_roots {
            self.invalidate_workspace_content_index(&root, true).await;
        }
        self.reload_env().await;
        self.reanalyze_open_documents().await;
    }

    async fn shutdown(&self) -> Result<()> {
        self.invalidate_analysis_cache().await;
        let pending_diagnostics = std::mem::take(&mut *self.pending_diagnostics.write().await);
        for task in pending_diagnostics.into_values() {
            task.abort();
        }
        self.documents.write().await.clear();
        self.analysis_inflight.write().await.clear();
        self.workspace_analysis_cache.write().await.clear();
        self.import_dependencies.write().await.clear();
        self.project_roots.write().await.clear();
        self.workspace_exports_cache.write().await.clear();
        self.workspace_definitions_cache.write().await.clear();
        self.workspace_occurrences_cache.write().await.clear();
        self.workspace_files_cache.write().await.clear();
        self.workspace_root_generations.write().await.clear();
        *self.workspace_root.write().await = None;
        self.workspace_roots.write().await.clear();
        self.env_state.write().await.take();
        self.env_config.write().await.take();
        self.schema_file_watchers.lock().await.clear();
        if let Ok(mut cache) = self.format_config_cache.write() {
            cache.clear();
        }
        if let Ok(mut cache) = self.lint_config_cache.write() {
            cache.clear();
        }
        if let Ok(mut cache) = self.import_map_cache.write() {
            cache.clear();
        }
        if let Ok(mut cache) = self.project_database_config_cache.write() {
            cache.clear();
        }
        Ok(())
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> Result<Option<serde_json::Value>> {
        if params.command != "tyrus.db.explainSql" {
            return Ok(None);
        }
        let Some(uri) = params
            .arguments
            .first()
            .and_then(serde_json::Value::as_str)
            .and_then(|value| Url::parse(value).ok())
        else {
            return Ok(None);
        };
        let Some(offset) = params
            .arguments
            .get(1)
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
        else {
            return Ok(None);
        };
        let Some(source) = self.source(&uri).await else {
            return Ok(None);
        };
        let Some(literal) = sql_literal_at(&source, offset) else {
            self.client
                .show_message(
                    MessageType::WARNING,
                    "Place the cursor inside a `db.query` SQL string to explain it.",
                )
                .await;
            return Ok(None);
        };
        if literal.text.len() > 64 * 1024 {
            self.client
                .show_message(
                    MessageType::WARNING,
                    "SQL text exceeds the 64 KiB limit for editor explanations.",
                )
                .await;
            return Ok(None);
        }
        let Some(project_root) = self.project_root_for(&uri).await else {
            self.client
                .show_message(
                    MessageType::WARNING,
                    "Could not locate the Tyrus project root.",
                )
                .await;
            return Ok(None);
        };
        let executable = match std::env::current_exe() {
            Ok(executable) => executable,
            Err(_) => {
                self.client
                    .show_message(
                        MessageType::WARNING,
                        "Could not locate the Tyrus executable.",
                    )
                    .await;
                return Ok(None);
            }
        };
        let sql = literal.text.to_string();
        let result = tokio::task::spawn_blocking(move || {
            let mut child = std::process::Command::new(executable)
                .args(["db", "explain-sql"])
                .current_dir(project_root)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|_| "could not start `tyrus db explain-sql`".to_string())?;
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                stdin
                    .write_all(sql.as_bytes())
                    .map_err(|_| "could not send SQL to the explain command".to_string())?;
            }
            let output = child
                .wait_with_output()
                .map_err(|_| "could not read the explain command result".to_string())?;
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .map_err(|_| "the explain command returned invalid text".to_string())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
            }
        })
        .await
        .unwrap_or_else(|_| Err("the SQL explain task failed".to_string()));
        match result {
            Ok(explanation) => {
                self.client
                    .show_message(MessageType::INFO, explanation)
                    .await;
            }
            Err(error) => {
                self.client.show_message(MessageType::WARNING, error).await;
            }
        }
        Ok(None)
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let formatting_started = std::time::Instant::now();
        let uri = params.text_document.uri;
        if !is_tyrus_uri(&uri) {
            return Ok(None);
        }
        let context_generation = self
            .context_generation
            .load(std::sync::atomic::Ordering::Acquire);
        let Some((source, analysis)) = self.documents.read().await.get(&uri).map(|document| {
            let analysis = document
                .cached_analysis(&document.text, Some(document.version), context_generation)
                .map(|cached| Arc::clone(&cached.analysis));
            (document.text.clone(), analysis)
        }) else {
            return Ok(None);
        };
        let config_dir = self.workspace_root.read().await.clone().or_else(|| {
            uri.to_file_path()
                .ok()
                .and_then(|path| path.parent().map(Path::to_path_buf))
        });
        let source_for_format = source.clone();
        let analysis_for_format = analysis.clone();
        let config_cache = Arc::clone(&self.format_config_cache);
        let formatting_queue_started = std::time::Instant::now();
        let Ok(permit) = Arc::clone(&self.formatting_semaphore).acquire_owned().await else {
            return Ok(None);
        };
        trace_lsp_stage("formatting_worker_queue", formatting_queue_started);
        let formatted = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let config_started = std::time::Instant::now();
            let config = match config_dir {
                Some(dir) => {
                    let config_path = dir.join("tyrus-fmt.json");
                    let metadata = std::fs::metadata(&config_path).ok();
                    let modified = metadata
                        .as_ref()
                        .and_then(|metadata| metadata.modified().ok());
                    let len = metadata.as_ref().map_or(0, std::fs::Metadata::len);
                    if let Some(config) = config_cache
                        .read()
                        .ok()
                        .as_ref()
                        .and_then(|cache| cache.get(&config_path))
                        .filter(|entry| entry.modified == modified && entry.len == len)
                        .map(|entry| entry.config.clone())
                    {
                        config
                    } else {
                        let loaded = tyrus_fmt::config::Config::try_load(&dir);
                        let (config, should_cache) = match loaded {
                            Ok(Some(config)) => (config, true),
                            Ok(None) => (tyrus_fmt::config::Config::default(), true),
                            Err(_) => (tyrus_fmt::config::Config::default(), false),
                        };
                        let entry_weight = estimated_config_cache_entry_weight(&config_path, len);
                        if should_cache
                            && entry_weight <= MAX_CACHED_CONFIG_ENTRY_WEIGHT
                            && let Ok(mut cache) = config_cache.write()
                        {
                            evict_config_cache_entries(
                                &mut cache,
                                &config_path,
                                entry_weight,
                                |path, entry| estimated_config_cache_entry_weight(path, entry.len),
                            );
                            cache.insert(
                                config_path,
                                FormatConfigCacheEntry {
                                    modified,
                                    len,
                                    config: config.clone(),
                                },
                            );
                        }
                        config
                    }
                }
                None => tyrus_fmt::config::Config::default(),
            };
            trace_lsp_stage("formatting_config", config_started);
            let formatter_started = std::time::Instant::now();
            let edits = if let Some(program) = analysis_for_format
                .as_deref()
                .and_then(|analysis| analysis.program.as_ref())
            {
                trace_lsp_event("formatting_program_reuse");
                format_document_edits_with_program(&source_for_format, &config, program)
            } else {
                trace_lsp_event("formatting_parse_fallback");
                format_document_edits(&source_for_format, &config)
            };
            trace_lsp_stage("formatting_engine", formatter_started);
            trace_lsp_count(
                "formatting_edit_bytes",
                edits
                    .as_ref()
                    .into_iter()
                    .flatten()
                    .map(|edit| edit.new_text.len())
                    .sum(),
            );
            edits
        })
        .await;
        let Ok(formatted) = formatted else {
            return Ok(None);
        };
        trace_lsp_stage("formatting", formatting_started);
        let source_is_current = self
            .documents
            .read()
            .await
            .get(&uri)
            .is_some_and(|current| current.text == source);
        Ok(source_is_current.then_some(formatted).flatten())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let document = params.text_document;
        self.cancel_document_analysis(&document.uri).await;
        if let Some(task) = self.pending_diagnostics.write().await.remove(&document.uri) {
            task.abort();
        }
        if let Some(root) = self.project_root_for(&document.uri).await {
            self.invalidate_workspace_content_index(&root, false).await;
        }
        self.workspace_analysis_cache
            .write()
            .await
            .remove(&document.uri);
        self.remove_import_dependencies(&document.uri).await;
        self.documents.write().await.insert(
            document.uri.clone(),
            DocumentState {
                text: document.text.clone(),
                version: document.version,
                analysis: None,
                published_diagnostics: Vec::new(),
            },
        );
        if self.is_env_document(&document.uri).await {
            self.reload_env_from_buffer(&document.uri, &document.text, document.version)
                .await;
            return;
        }
        self.client
            .publish_diagnostics(document.uri.clone(), Vec::new(), Some(document.version))
            .await;
        if !is_tyrus_uri(&document.uri) {
            return;
        }
        let backend = self.clone();
        let task_uri = document.uri.clone();
        let task = tokio::spawn(async move {
            let Some(queue_permit) = backend.acquire_diagnostic_task_permit().await else {
                return;
            };
            let _queue_permit = queue_permit;
            if backend
                .document_matches(&document.uri, &document.text, Some(document.version))
                .await
            {
                backend
                    .publish_analysis(document.uri, &document.text, Some(document.version))
                    .await;
            }
        });
        let mut pending = self.pending_diagnostics.write().await;
        pending.retain(|_, handle| !handle.is_finished());
        if let Some(previous) = pending.insert(task_uri, task.abort_handle()) {
            previous.abort();
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let Some(document) = self
            .documents
            .read()
            .await
            .get(&params.text_document.uri)
            .cloned()
        else {
            return;
        };
        if !is_newer_document_version(Some(document.version), params.text_document.version) {
            return;
        }
        let mut source = document.text;
        if params.content_changes.is_empty() {
            return;
        }
        for change in params.content_changes {
            let Some(range) = change.range else {
                source = change.text;
                continue;
            };
            let Some(start) = position_to_byte_offset_checked(&source, range.start) else {
                return;
            };
            let Some(end) = position_to_byte_offset_checked(&source, range.end) else {
                return;
            };
            if start > end {
                return;
            }
            if change
                .range_length
                .is_some_and(|length| source[start..end].encode_utf16().count() != length as usize)
            {
                return;
            }
            source.replace_range(start..end, &change.text);
        }
        self.cancel_document_analysis(&params.text_document.uri)
            .await;
        if let Some(previous) = self
            .pending_diagnostics
            .write()
            .await
            .remove(&params.text_document.uri)
        {
            previous.abort();
        }
        if let Some(root) = self.project_root_for(&params.text_document.uri).await {
            self.invalidate_workspace_content_index(&root, false).await;
        }
        self.workspace_analysis_cache
            .write()
            .await
            .remove(&params.text_document.uri);
        self.remove_import_dependencies(&params.text_document.uri)
            .await;
        self.documents.write().await.insert(
            params.text_document.uri.clone(),
            DocumentState {
                text: source.clone(),
                version: params.text_document.version,
                analysis: None,
                published_diagnostics: Vec::new(),
            },
        );
        if self.is_env_document(&params.text_document.uri).await {
            self.reload_env_from_buffer(
                &params.text_document.uri,
                &source,
                params.text_document.version,
            )
            .await;
            return;
        }
        self.client
            .publish_diagnostics(
                params.text_document.uri.clone(),
                Vec::new(),
                Some(params.text_document.version),
            )
            .await;
        if !is_tyrus_uri(&params.text_document.uri) {
            return;
        }
        let backend = self.clone();
        let uri = params.text_document.uri;
        let task_uri = uri.clone();
        let version = params.text_document.version;
        let task = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(
                backend
                    .diagnostic_debounce_ms
                    .load(std::sync::atomic::Ordering::Acquire),
            ))
            .await;
            if !backend.document_matches(&uri, &source, Some(version)).await {
                return;
            }
            let Some(queue_permit) = backend.acquire_diagnostic_task_permit().await else {
                return;
            };
            let _queue_permit = queue_permit;
            if backend.document_matches(&uri, &source, Some(version)).await {
                backend.publish_analysis(uri, &source, Some(version)).await;
            }
        });
        let mut pending = self.pending_diagnostics.write().await;
        pending.retain(|_, handle| !handle.is_finished());
        pending.insert(task_uri, task.abort_handle());
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.cancel_document_analysis(&params.text_document.uri)
            .await;
        if let Some(task) = self
            .pending_diagnostics
            .write()
            .await
            .remove(&params.text_document.uri)
        {
            task.abort();
        }
        if let Some(root) = self.project_root_for(&params.text_document.uri).await {
            self.invalidate_workspace_content_index(&root, false).await;
        }
        self.workspace_analysis_cache
            .write()
            .await
            .remove(&params.text_document.uri);
        self.remove_import_dependencies(&params.text_document.uri)
            .await;
        self.analysis_inflight
            .write()
            .await
            .retain(|(uri, _, _, _), _| uri != &params.text_document.uri);
        self.documents
            .write()
            .await
            .remove(&params.text_document.uri);
        if self.is_env_document(&params.text_document.uri).await {
            self.reload_env().await;
            self.reanalyze_open_documents().await;
            return;
        }
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        trace_lsp_event("watched_files_change_received");
        let changed_uris = params
            .changes
            .iter()
            .map(|change| change.uri.clone())
            .collect::<Vec<_>>();
        let changed_paths = changed_uris
            .iter()
            .filter_map(|uri| uri.to_file_path().ok())
            .collect::<std::collections::HashSet<_>>();
        let schema_roots = self
            .schema_file_watchers
            .lock()
            .await
            .iter()
            .filter(|(_, (schema_path, _))| changed_paths.contains(schema_path))
            .map(|(root, _)| root.clone())
            .collect::<std::collections::HashSet<_>>();
        let schema_snapshot_changed = !schema_roots.is_empty();
        let mut affected_documents = self.import_dependents(&changed_uris).await;
        trace_lsp_event("watched_files_import_dependents_resolved");
        trace_lsp_count("watched_files_initial_dependents", affected_documents.len());
        let formatter_config_paths = changed_paths
            .iter()
            .filter(|path| {
                path.file_name()
                    .is_some_and(|name| name == "tyrus-fmt.json")
            })
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        if !formatter_config_paths.is_empty()
            && let Ok(mut cache) = self.format_config_cache.write()
        {
            cache.retain(|path, _| !formatter_config_paths.contains(path));
        }
        let lint_config_paths = changed_paths
            .iter()
            .filter(|path| {
                path.file_name()
                    .is_some_and(|name| name == "tyrus-lint.json")
            })
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        let lint_config_changed = !lint_config_paths.is_empty();
        if lint_config_changed && let Ok(mut cache) = self.lint_config_cache.write() {
            cache.retain(|path, _| !lint_config_paths.contains(path));
        }
        let project_config_roots = changed_paths
            .iter()
            .filter(|path| path.file_name().is_some_and(|name| name == "tyrus.json"))
            .filter_map(|path| path.parent().map(Path::to_path_buf))
            .collect::<std::collections::HashSet<_>>();
        if !project_config_roots.is_empty() {
            if let Ok(mut cache) = self.import_map_cache.write() {
                cache.retain(|root, _| !project_config_roots.contains(root));
            }
            if let Ok(mut cache) = self.project_database_config_cache.write() {
                cache.retain(|root, _| !project_config_roots.contains(root));
            }
        }
        let workspace_config_root = self.workspace_root.read().await.clone();
        let workspace_config_changed = workspace_config_root
            .as_ref()
            .is_some_and(|root| project_config_roots.contains(root));
        let active_env_uri = if let Some(uri) = self
            .env_state
            .read()
            .await
            .as_ref()
            .map(|state| state.env_uri.clone())
        {
            Some(uri)
        } else {
            self.env_config
                .read()
                .await
                .as_ref()
                .map(|config| config.uri.clone())
        };
        let env_file_changed = changed_uris.iter().any(|uri| {
            if active_env_uri.as_ref() == Some(uri) {
                return true;
            }
            let Ok(path) = uri.to_file_path() else {
                return false;
            };
            workspace_config_root.as_ref().is_some_and(|root| {
                path.parent() == Some(root.as_path())
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name == ".env" || name.starts_with(".env."))
            })
        });
        let project_config_relevant = !project_config_roots.is_empty() || env_file_changed;
        let global_context_changed = workspace_config_changed || env_file_changed;
        let source_file_changed = changed_uris.iter().any(|uri| uri.path().ends_with(".tyr"));
        if !project_config_roots.is_empty() {
            self.invalidate_project_roots().await;
        }
        if global_context_changed {
            self.reload_env().await;
        }
        trace_lsp_event("watched_files_project_context_updated");
        let document_uris = self
            .documents
            .read()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for document_uri in &document_uris {
            let Some(root) = self.project_root_for(document_uri).await else {
                continue;
            };
            if lint_config_paths.contains(&root.join("tyrus-lint.json")) {
                affected_documents.insert(document_uri.clone());
            }
        }
        if schema_snapshot_changed {
            for document_uri in &document_uris {
                if self
                    .project_root_for(document_uri)
                    .await
                    .is_some_and(|root| schema_roots.contains(&root))
                    && !affected_documents.contains(document_uri)
                {
                    affected_documents.insert(document_uri.clone());
                }
            }
        }
        trace_lsp_event("watched_files_affected_documents_resolved");
        trace_lsp_count("watched_files_affected_documents", affected_documents.len());
        affected_documents.extend(
            changed_uris
                .iter()
                .filter(|changed| document_uris.contains(changed))
                .cloned(),
        );
        if project_config_relevant && global_context_changed {
            affected_documents.extend(document_uris.iter().cloned());
        }
        if project_config_relevant && !global_context_changed {
            affected_documents.extend(self.import_dependents_in_roots(&project_config_roots).await);
            for document_uri in &document_uris {
                if self
                    .project_root_for(document_uri)
                    .await
                    .is_some_and(|root| project_config_roots.contains(&root))
                {
                    affected_documents.insert(document_uri.clone());
                }
            }
        }
        if lint_config_changed {
            for (uri, document) in self.documents.write().await.iter_mut() {
                if affected_documents.contains(uri)
                    && let Some(analysis) = document.analysis.as_mut()
                {
                    analysis.lint_diagnostics = None;
                }
            }
        }
        if project_config_relevant && global_context_changed {
            self.invalidate_analysis_cache().await;
        } else if project_config_relevant {
            self.invalidate_analysis_cache_for(&affected_documents)
                .await;
            self.workspace_analysis_cache
                .write()
                .await
                .retain(|uri, _| !affected_documents.contains(uri));
        } else if source_file_changed || schema_snapshot_changed {
            let mut invalidated_uris = affected_documents.clone();
            invalidated_uris.extend(changed_uris.iter().cloned());
            if schema_snapshot_changed || source_file_changed {
                for uri in &affected_documents {
                    self.cancel_document_analysis(uri).await;
                }
            }
            for (uri, document) in self.documents.write().await.iter_mut() {
                if invalidated_uris.contains(uri) {
                    document.analysis = None;
                }
            }
            self.analysis_inflight
                .write()
                .await
                .retain(|(uri, _, _, _), _| !invalidated_uris.contains(uri));
            self.workspace_analysis_cache
                .write()
                .await
                .retain(|uri, _| !changed_uris.contains(uri));
        }
        trace_lsp_event("watched_files_analysis_invalidated");
        if project_config_relevant
            || source_file_changed
            || lint_config_changed
            || schema_snapshot_changed
        {
            for uri in &affected_documents {
                if global_context_changed && self.is_env_document(uri).await {
                    continue;
                }
                if let Some(document) = self.documents.write().await.get_mut(uri) {
                    document.published_diagnostics.clear();
                }
                self.client
                    .publish_diagnostics(uri.clone(), Vec::new(), None)
                    .await;
            }
        }
        if project_config_relevant && global_context_changed {
            self.invalidate_workspace_index().await;
        } else if project_config_relevant {
            for root in &project_config_roots {
                self.invalidate_workspace_content_index(root, false).await;
            }
        } else if source_file_changed {
            let mut changed_roots = std::collections::HashMap::new();
            for uri in &changed_uris {
                if uri.path().ends_with(".tyr")
                    && let Some(root) = self.project_root_for(uri).await
                {
                    let inventory_changed = params.changes.iter().any(|change| {
                        change.uri == *uri
                            && matches!(
                                change.typ,
                                FileChangeType::CREATED | FileChangeType::DELETED
                            )
                    });
                    changed_roots
                        .entry(root)
                        .and_modify(|changed| *changed |= inventory_changed)
                        .or_insert(inventory_changed);
                }
            }
            for (root, inventory_changed) in changed_roots {
                self.invalidate_workspace_content_index(&root, inventory_changed)
                    .await;
            }
        }
        trace_lsp_event("watched_files_workspace_index_invalidated");
        for uri in document_uris {
            if !affected_documents.contains(&uri) || !is_tyrus_uri(&uri) {
                continue;
            }
            if let Some(previous) = self.pending_diagnostics.write().await.remove(&uri) {
                previous.abort();
            }
            let backend = self.clone();
            let task_uri = uri.clone();
            let task = tokio::spawn(async move {
                let Some(queue_permit) = backend.acquire_diagnostic_task_permit().await else {
                    return;
                };
                let _queue_permit = queue_permit;
                let Some(source) = backend.source(&uri).await else {
                    return;
                };
                backend.publish_analysis(uri, &source, None).await;
            });
            let mut pending = self.pending_diagnostics.write().await;
            pending.retain(|_, handle| !handle.is_finished());
            if let Some(previous) = pending.insert(task_uri, task.abort_handle()) {
                previous.abort();
            }
        }
        trace_lsp_event("watched_files_reanalysis_scheduled");
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let _trace = trace_lsp_span("provider_hover");
        let text_document = params.text_document_position_params;
        let Some(source) = self.source(&text_document.text_document.uri).await else {
            return Ok(None);
        };
        let offset = position_to_byte_offset(&source, text_document.position);
        if self.is_env_document(&text_document.text_document.uri).await {
            return Ok(
                validator_hover_at(&source, offset).map(|(range, markdown)| Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: markdown,
                    }),
                    range: Some(byte_range_to_lsp(&source, range)),
                }),
            );
        }
        if !is_tyrus_uri(&text_document.text_document.uri) {
            return Ok(None);
        }
        let analysis = self
            .analysis_for(&text_document.text_document.uri, &source)
            .await;
        let context_generation = self
            .context_generation
            .load(std::sync::atomic::Ordering::Acquire);
        if !self
            .current_document_matches(&text_document.text_document.uri, &source)
            .await
        {
            return Ok(None);
        }
        if let Some((range, markdown)) = comptime_hover_at(&analysis, offset) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: markdown,
                }),
                range: Some(byte_range_to_lsp(&source, range)),
            }));
        }
        if let Some((range, markdown)) = http_route_option_hover_at(&source, offset) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: markdown,
                }),
                range: Some(byte_range_to_lsp(&source, range)),
            }));
        }
        if let Some(diagnostic) = analysis.diagnostic_at(offset) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: diagnostic_hover_markdown(&source, diagnostic),
                }),
                range: Some(byte_range_to_lsp(&source, diagnostic.range.clone())),
            }));
        }
        if sql_literal_at(&source, offset).is_some()
            && let Ok(snapshot) = self
                .db_schema_for_uri(&text_document.text_document.uri)
                .await
        {
            if context_generation
                != self
                    .context_generation
                    .load(std::sync::atomic::Ordering::Acquire)
                || !self
                    .current_document_matches(&text_document.text_document.uri, &source)
                    .await
            {
                return Ok(None);
            }
            if let Some((range, markdown)) = sql_hover_at(&source, offset, &snapshot) {
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: markdown,
                    }),
                    range: Some(byte_range_to_lsp(&source, range)),
                }));
            }
        }
        if let Some((range, markdown)) = validator_hover_at(&source, offset) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: markdown,
                }),
                range: Some(byte_range_to_lsp(&source, range)),
            }));
        }
        if let Some((range, markdown)) = native_promise_hover_at(&analysis, offset) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: markdown,
                }),
                range: Some(byte_range_to_lsp(&source, range)),
            }));
        }
        if let Some((range, markdown)) = test_framework_hover_at(&source, offset) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: markdown,
                }),
                range: Some(byte_range_to_lsp(&source, range)),
            }));
        }
        if let Some((range, markdown)) = automatic_data_hover_at(&source, &analysis, offset) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: markdown,
                }),
                range: Some(byte_range_to_lsp(&source, range)),
            }));
        }
        let symbol_lookup_started = std::time::Instant::now();
        let symbol = analysis.symbol_at(offset);
        trace_lsp_stage("hover_symbol_lookup", symbol_lookup_started);
        if symbol.is_none()
            && let Some((range, markdown)) = global_builtin_hover_at(&analysis, offset)
        {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: markdown,
                }),
                range: Some(byte_range_to_lsp(&source, range)),
            }));
        }
        if let Some((range, markdown)) = native_namespace_member_hover_at(&source, offset) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: markdown,
                }),
                range: Some(byte_range_to_lsp(&source, range)),
            }));
        }
        let Some(symbol) = symbol else {
            return Ok(analysis.type_at(offset).map(|expression| Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("```tyrus\n{}\n```", expression.ty),
                }),
                range: Some(byte_range_to_lsp(&source, expression.range.clone())),
            }));
        };
        let interface_hover_started = std::time::Instant::now();
        let interface_hover = self
            .interface_markdown(
                &text_document.text_document.uri,
                symbol,
                &source,
                analysis.program.as_ref(),
            )
            .await;
        trace_lsp_stage("hover_interface_documentation", interface_hover_started);
        if context_generation
            != self
                .context_generation
                .load(std::sync::atomic::Ordering::Acquire)
            || !self
                .current_document_matches(&text_document.text_document.uri, &source)
                .await
        {
            return Ok(None);
        }
        if let Some(value) = interface_hover {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: Some(byte_range_to_lsp(&source, symbol.declaration.clone())),
            }));
        }
        let mut detail = symbol.ty.clone();
        if symbol.is_validator {
            detail = format!("validator function {detail}");
        }
        let mut example = function_usage_example(&symbol.name, symbol);
        let mut documentation = symbol.documentation.clone();
        if symbol.kind == SymbolKind::Import {
            let imported = symbol.imported_name.as_deref().unwrap_or(&symbol.name);
            if let Some(ty) = symbol
                .import_source
                .as_deref()
                .and_then(|module| native_import_type(module, imported))
            {
                detail = symbol
                    .import_source
                    .as_deref()
                    .and_then(|module| native_import_detail(module, imported))
                    .map_or_else(|| ty.to_string(), str::to_string);
                example = symbol
                    .import_source
                    .as_deref()
                    .and_then(|module| native_import_example(module, imported, &symbol.name));
            } else {
                let imported_hover_started = std::time::Instant::now();
                let location = self
                    .imported_definition(&text_document.text_document.uri, symbol)
                    .await;
                if context_generation
                    != self
                        .context_generation
                        .load(std::sync::atomic::Ordering::Acquire)
                    || !self
                        .current_document_matches(&text_document.text_document.uri, &source)
                        .await
                {
                    return Ok(None);
                }
                if let Some(location) = location
                    && let Some(target_source) = self.source(&location.uri).await
                    && let Some(target) = self
                        .workspace_analysis_for(&location.uri, &target_source)
                        .await
                        .symbols
                        .iter()
                        .find(|candidate| candidate.name == imported)
                {
                    detail = target.ty.clone();
                    example = function_usage_example(&symbol.name, target);
                    documentation = target.documentation.clone();
                }
                trace_lsp_stage("hover_imported_target", imported_hover_started);
            }
        }
        let hover_render_started = std::time::Instant::now();
        let markdown = hover_markdown(
            &symbol.name,
            &detail,
            example.as_deref(),
            documentation.as_ref(),
        );
        trace_lsp_stage("hover_markdown_render", hover_render_started);
        if context_generation
            != self
                .context_generation
                .load(std::sync::atomic::Ordering::Acquire)
            || !self
                .current_document_matches(&text_document.text_document.uri, &source)
                .await
        {
            return Ok(None);
        }
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: markdown,
            }),
            range: Some(byte_range_to_lsp(&source, symbol.declaration.clone())),
        }))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let _trace = trace_lsp_span("provider_definition");
        let text_document = params.text_document_position_params;
        let uri = text_document.text_document.uri;
        if !is_tyrus_uri(&uri) {
            return Ok(None);
        }
        let Some(source) = self.source(&uri).await else {
            return Ok(None);
        };
        let offset = position_to_byte_offset(&source, text_document.position);
        let analysis = self.analysis_for(&uri, &source).await;
        if !self.current_document_matches(&uri, &source).await {
            return Ok(None);
        }
        let Some(symbol) = analysis.symbol_at(offset) else {
            return Ok(None);
        };
        let location = if symbol.kind == SymbolKind::Import {
            self.imported_definition(&uri, symbol).await
        } else {
            Some(Location {
                uri: uri.clone(),
                range: byte_range_to_lsp(&source, symbol.declaration.clone()),
            })
        };
        if !self.current_document_matches(&uri, &source).await {
            return Ok(None);
        }
        Ok(location.map(GotoDefinitionResponse::Scalar))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let _trace = trace_lsp_span("provider_references");
        let text_document = params.text_document_position;
        let uri = text_document.text_document.uri;
        if !is_tyrus_uri(&uri) {
            return Ok(None);
        }
        let Some(source) = self.source(&uri).await else {
            return Ok(None);
        };
        let offset = position_to_byte_offset(&source, text_document.position);
        let analysis = self.analysis_for(&uri, &source).await;
        if !self.current_document_matches(&uri, &source).await {
            return Ok(None);
        }
        let Some(symbol) = analysis.symbol_at(offset) else {
            return Ok(None);
        };
        let line_index = self.line_index_for(&uri, &source).await;
        if let Some((defining_uri, name)) = self.resolve_symbol_target(&uri, symbol).await {
            let mut locations = Vec::new();
            for (file_uri, ranges) in self
                .cross_file_occurrences(&defining_uri, &name, false)
                .await
            {
                let Some(file_source) = self.source(&file_uri).await else {
                    continue;
                };
                let file_line_index = self.line_index_for(&file_uri, &file_source).await;
                for range in ranges {
                    locations.push(Location {
                        uri: file_uri.clone(),
                        range: byte_range_to_lsp_with_index(&file_source, range, &file_line_index),
                    });
                }
            }
            if !self.current_document_matches(&uri, &source).await {
                return Ok(None);
            }
            return Ok(Some(locations));
        }
        let declaration = symbol.declaration.clone();
        let ranges = symbol_occurrence_ranges(&analysis, symbol)
            .into_iter()
            .filter(|range| params.context.include_declaration || *range != declaration);
        let locations = ranges
            .map(|range| Location {
                uri: uri.clone(),
                range: byte_range_to_lsp_with_index(&source, range, &line_index),
            })
            .collect();
        if !self.current_document_matches(&uri, &source).await {
            return Ok(None);
        }
        Ok(Some(locations))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let _trace = trace_lsp_span("provider_rename");
        let uri = params.text_document_position.text_document.uri;
        let request_uri = uri.clone();
        if !is_tyrus_uri(&uri) {
            return Ok(None);
        }
        let new_name = params.new_name;
        if tyrus_core::keywords::reserved_kind(&new_name).is_some() {
            return Err(tower_lsp::jsonrpc::Error::invalid_params(format!(
                "`{new_name}` is a reserved keyword and cannot be used as an identifier"
            )));
        }
        let Some(source) = self.source(&uri).await else {
            return Ok(None);
        };
        let offset = position_to_byte_offset(&source, params.text_document_position.position);
        let analysis = self.analysis_for(&uri, &source).await;
        if !self.current_document_matches(&uri, &source).await {
            return Ok(None);
        }
        let Some(symbol) = analysis.symbol_at(offset) else {
            return Ok(None);
        };
        let line_index = self.line_index_for(&uri, &source).await;
        let mut changes: std::collections::HashMap<Url, Vec<TextEdit>> =
            std::collections::HashMap::new();
        if let Some((defining_uri, name)) = self.resolve_symbol_target(&uri, symbol).await {
            for (file_uri, ranges) in self
                .cross_file_occurrences(&defining_uri, &name, true)
                .await
            {
                let Some(file_source) = self.source(&file_uri).await else {
                    continue;
                };
                let file_line_index = self.line_index_for(&file_uri, &file_source).await;
                let edits = ranges
                    .into_iter()
                    .map(|range| TextEdit {
                        range: byte_range_to_lsp_with_index(&file_source, range, &file_line_index),
                        new_text: new_name.clone(),
                    })
                    .collect();
                changes.insert(file_uri, edits);
            }
        } else {
            let edits = symbol_occurrence_ranges(&analysis, symbol)
                .into_iter()
                .map(|range| TextEdit {
                    range: byte_range_to_lsp_with_index(&source, range, &line_index),
                    new_text: new_name.clone(),
                })
                .collect();
            changes.insert(uri, edits);
        }
        if !self.current_document_matches(&request_uri, &source).await {
            return Ok(None);
        }
        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }))
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        let _trace = trace_lsp_span("provider_signature_help");
        let text_document = params.text_document_position_params;
        let uri = text_document.text_document.uri;
        if !is_tyrus_uri(&uri) {
            return Ok(None);
        }
        let Some(source) = self.source(&uri).await else {
            return Ok(None);
        };
        let offset = position_to_byte_offset(&source, text_document.position);
        let analysis = self.analysis_for(&uri, &source).await;
        if !self.current_document_matches(&uri, &source).await {
            return Ok(None);
        }
        let Some(call) = analysis
            .call_sites
            .iter()
            .filter(|call| call.range.start <= offset && offset <= call.range.end)
            .min_by_key(|call| call.range.end - call.range.start)
        else {
            return Ok(None);
        };
        let Some(signature) = self
            .call_signature_with_receiver(&uri, &analysis, &call.callee, call.receiver.as_deref())
            .await
        else {
            return Ok(None);
        };
        if !self.current_document_matches(&uri, &source).await {
            return Ok(None);
        }
        let names = signature.parameters;
        let mut label = format!("{}(", call.callee);
        let mut parameters = Vec::with_capacity(names.len());
        for (index, name) in names.iter().enumerate() {
            if index > 0 {
                label.push_str(", ");
            }
            let start = label.len() as u32;
            label.push_str(name);
            let end = label.len() as u32;
            parameters.push(ParameterInformation {
                label: ParameterLabel::LabelOffsets([start, end]),
                documentation: None,
            });
        }
        label.push(')');
        if let Some(ret) = signature
            .detail
            .as_deref()
            .and_then(signature_return_type_from_detail)
        {
            label.push_str(": ");
            label.push_str(&ret);
        }
        let active = signature_active_parameter(call, offset, names.len());
        Ok(Some(SignatureHelp {
            signatures: vec![SignatureInformation {
                label,
                documentation: None,
                parameters: Some(parameters),
                active_parameter: Some(active),
            }],
            active_signature: Some(0),
            active_parameter: Some(active),
        }))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let _trace = trace_lsp_span("provider_completion");
        let work_done_token = params.work_done_progress_params.work_done_token.clone();
        let uri = params.text_document_position.text_document.uri;
        let Some(source) = self.source(&uri).await else {
            return Ok(None);
        };
        if self.is_env_document(&uri).await {
            let offset = position_to_byte_offset(&source, params.text_document_position.position);
            return Ok(env_completion_items(&source, offset).map(CompletionResponse::Array));
        }
        if !is_tyrus_uri(&uri) {
            return Ok(None);
        }
        let analysis = self.analysis_for(&uri, &source).await;
        let import_behaviors_started = std::time::Instant::now();
        let import_behaviors = self
            .import_behaviors_for(&uri, &source)
            .await
            .unwrap_or_else(|| Arc::new(HashMap::default()));
        trace_lsp_stage("completion_import_behaviors", import_behaviors_started);
        if !self.current_document_matches(&uri, &source).await {
            return Ok(None);
        }
        let offset = position_to_byte_offset(&source, params.text_document_position.position);
        if let Some(items) = validator_completion_items(&source, offset, &analysis) {
            return Ok(Some(CompletionResponse::Array(items)));
        }
        if let Some(items) = style_text_completion_items(&source, offset) {
            return Ok(Some(CompletionResponse::Array(items)));
        }
        if let Some(items) = figlet_font_completion_items(&source, offset) {
            return Ok(Some(CompletionResponse::Array(items)));
        }
        if let Some(items) = response_header_completion_items(&source, offset) {
            return Ok(Some(CompletionResponse::Array(items)));
        }
        if sql_literal_at(&source, offset).is_some()
            && let Ok(snapshot) = self.db_schema_for_uri(&uri).await
            && let Some(items) = sql_completion_items(&source, offset, &snapshot)
        {
            return Ok(Some(CompletionResponse::Array(items)));
        }
        if let Some(receiver) = object_projection_key_completion_receiver(&source, offset)
            && let Some(fields) = object_fields_for_receiver(&analysis, &receiver)
        {
            return Ok(Some(CompletionResponse::Array(
                object_projection_key_completion_items(&fields, in_string_literal(&source, offset)),
            )));
        }
        if let Some(receiver) = object_projection_type_key_completion_receiver(&source, offset)
            && let Some(fields) = object_fields_for_receiver(&analysis, &receiver)
        {
            return Ok(Some(CompletionResponse::Array(
                object_projection_key_completion_items(&fields, true),
            )));
        }
        if in_string_literal(&source, offset) {
            return Ok(None);
        }
        if let Some(items) =
            member_completion_items_with_behaviors(&source, offset, &analysis, &import_behaviors)
        {
            return Ok(Some(CompletionResponse::Array(items)));
        }
        if let Some(items) = http_route_option_completion_items(&source, offset) {
            return Ok(Some(CompletionResponse::Array(items)));
        }
        let terminate_function_statement = completion_starts_statement(&source, offset);
        let auto_import_prefix = completion_identifier_prefix(&source, offset);
        if let Some((type_name, existing)) = object_literal_context(&source, offset)
            && let Some(symbol) = analysis.symbols.iter().find(|symbol| {
                symbol.name == type_name
                    && matches!(
                        symbol.kind,
                        SymbolKind::Interface | SymbolKind::TypeAlias | SymbolKind::Import
                    )
            })
            && let Some(fields) = self.object_type_fields(&uri, symbol).await
        {
            return Ok(Some(CompletionResponse::Array(
                object_field_completion_items(&fields, &existing),
            )));
        }
        let mut items = Vec::new();
        let local_items_started = std::time::Instant::now();
        let trace_snippets = trace_lsp_enabled();
        let mut snippet_duration = std::time::Duration::ZERO;
        let mut snippet_count = 0usize;
        macro_rules! measure_completion_snippet {
            ($expression:expr) => {{
                let started = trace_snippets.then(std::time::Instant::now);
                let result = $expression;
                if let Some(started) = started {
                    snippet_duration += started.elapsed();
                    snippet_count += 1;
                }
                result
            }};
        }
        for symbol in &analysis.symbols {
            let mut item = symbol_completion_item(symbol);
            if symbol.kind == SymbolKind::Import {
                if symbol.import_source.as_deref() == Some("tyrus:utils")
                    && symbol.name == "styleText"
                {
                    item.kind = Some(CompletionItemKind::FUNCTION);
                    item.insert_text = Some("styleText($0)".to_string());
                    item.insert_text_format = Some(InsertTextFormat::SNIPPET);
                } else if symbol.import_source.as_deref() == Some("tyrus:utils")
                    && symbol.name == "figlet"
                {
                    item.kind = Some(CompletionItemKind::FUNCTION);
                    item.insert_text = Some("figlet(${1:text}, ${2:\"standard\"})".to_string());
                    item.insert_text_format = Some(InsertTextFormat::SNIPPET);
                } else if symbol.import_source.as_deref() == Some("tyrus:cli")
                    && symbol.name == "isTTY"
                {
                    item.kind = Some(CompletionItemKind::FUNCTION);
                    item.insert_text = Some("isTTY()".to_string());
                    item.insert_text_format = Some(InsertTextFormat::SNIPPET);
                } else if symbol.import_source.as_deref() == Some("tyrus:cli") {
                    item.kind = Some(CompletionItemKind::FUNCTION);
                    item.insert_text = Some(format!("{}({{ $0 }})", symbol.name));
                    item.insert_text_format = Some(InsertTextFormat::SNIPPET);
                } else if let Some(parameters) = self
                    .call_parameter_names(&uri, &analysis, &symbol.name)
                    .await
                {
                    item.kind = Some(CompletionItemKind::FUNCTION);
                    measure_completion_snippet!(apply_function_snippet_with_terminator(
                        &mut item,
                        &symbol.name,
                        &parameters,
                        terminate_function_statement,
                    ));
                }
            } else if symbol.kind == SymbolKind::Function {
                let parameters = symbol
                    .parameters
                    .iter()
                    .map(|parameter| parameter_pattern_argument(&parameter.pattern))
                    .collect::<Vec<_>>();
                measure_completion_snippet!(apply_function_snippet_with_terminator(
                    &mut item,
                    &symbol.name,
                    &parameters,
                    terminate_function_statement,
                ));
            }
            if let Some(info) = self.interface_info(&uri, symbol, &source).await {
                let markdown = interface_hover_markdown(
                    &info.name,
                    &info.fields,
                    info.documentation.as_ref(),
                    Some(&info.link),
                );
                item.documentation = Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: markdown.clone(),
                }));
                let mut snippet = measure_completion_snippet!(interface_object_snippet_item(
                    &info.name,
                    &info.fields,
                ));
                snippet.documentation = Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: markdown,
                }));
                items.push(snippet);
            }
            items.push(item);
        }

        trace_lsp_count("completion_local_symbols", analysis.symbols.len());
        trace_lsp_stage("completion_local_symbols", local_items_started);
        let keyword_items_started = std::time::Instant::now();
        let keyword_items_start_count = items.len();
        let keyword_generation_started = std::time::Instant::now();
        let keywords = keyword_completion_items();
        trace_lsp_count("completion_keyword_items", keywords.len());
        trace_lsp_stage("completion_keyword_generation", keyword_generation_started);
        items.extend(keywords);
        let type_generation_started = std::time::Instant::now();
        let types = type_completion_items(&source, analysis.program.as_ref());
        trace_lsp_count("completion_type_items", types.len());
        trace_lsp_stage("completion_type_generation", type_generation_started);
        items.extend(types);
        if !analysis
            .symbols
            .iter()
            .any(|symbol| symbol.name == "parseDecimal")
        {
            let builtins_generation_started = std::time::Instant::now();
            let builtins = global_builtin_completion_items();
            trace_lsp_count("completion_global_builtin_items", builtins.len());
            trace_lsp_stage(
                "completion_global_builtin_generation",
                builtins_generation_started,
            );
            items.extend(builtins);
        }
        trace_lsp_count(
            "completion_keyword_builtin_items",
            items.len() - keyword_items_start_count,
        );
        trace_lsp_stage("completion_keyword_builtin_items", keyword_items_started);

        let current_path = uri.to_file_path().ok();
        let declared = analysis
            .symbols
            .iter()
            .map(|symbol| symbol.name.clone())
            .collect::<std::collections::HashSet<_>>();
        let imported = imported_names(&source);
        let auto_import_items_started = std::time::Instant::now();
        let auto_import_prefix_text = auto_import_prefix;
        let mut exports = self
            .exported_symbols(
                current_path.as_deref(),
                auto_import_prefix_text,
                work_done_token,
            )
            .await;
        self.add_alias_import_sources(&uri, &mut exports).await;
        trace_lsp_count("completion_auto_import_candidates", exports.len());
        for export in exports {
            if declared.contains(export.name.as_str()) || export.uri == uri {
                continue;
            }
            let already_imported = imported.contains(&export.name);
            let import_edit = if already_imported {
                None
            } else {
                let module = export
                    .module_source
                    .clone()
                    .or_else(|| import_source_for_export(current_path.as_deref(), &export.uri));
                let Some(module) = module else {
                    continue;
                };
                match crate::imports::auto_import_edit_with_default(
                    &source,
                    &module,
                    &export.name,
                    export.is_default,
                ) {
                    Some(edit) => Some((edit, module)),
                    None => continue,
                }
            };
            let detail = match &import_edit {
                Some((_, module)) => format!("{}, auto import from {module}", export.ty),
                None => export.ty.clone(),
            };
            let mut item = CompletionItem {
                label: export.name.clone(),
                kind: Some(completion_kind(export.kind)),
                detail: Some(detail),
                additional_text_edits: import_edit.map(|(edit, _)| vec![edit]),
                sort_text: Some(format!("1_{}", export.name)),
                ..CompletionItem::default()
            };
            measure_completion_snippet!(apply_function_snippet_with_terminator(
                &mut item,
                &export.name,
                &export.parameters,
                terminate_function_statement,
            ));
            items.push(item);
        }
        trace_lsp_stage("completion_auto_import_items", auto_import_items_started);

        if !self.current_document_matches(&uri, &source).await {
            return Ok(None);
        }

        trace_lsp_count("completion_returned_items", items.len());
        trace_lsp_duration("completion_snippet_generation", snippet_duration);
        trace_lsp_count("completion_snippets", snippet_count);
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let _trace = trace_lsp_span("provider_inlay_hint");
        let uri = params.text_document.uri;
        if !is_tyrus_uri(&uri) {
            return Ok(None);
        }
        let Some(source) = self.source(&uri).await else {
            return Ok(None);
        };
        let range_key = (
            params.range.start.line,
            params.range.start.character,
            params.range.end.line,
            params.range.end.character,
        );
        let version = self.document_version(&uri).await;
        let context_generation = self
            .context_generation
            .load(std::sync::atomic::Ordering::Acquire);
        if let Some(hints) = self
            .documents
            .read()
            .await
            .get(&uri)
            .and_then(|document| document.cached_analysis(&source, version, context_generation))
            .and_then(|cached| cached.inlay_hints.get(&range_key))
            .cloned()
        {
            return Ok(Some(hints.as_ref().clone()));
        }
        let analysis = self.analysis_for(&uri, &source).await;
        if !self.current_document_matches(&uri, &source).await {
            return Ok(None);
        }
        let line_index = self.line_index_for(&uri, &source).await;
        let range_conversion_started = std::time::Instant::now();
        let visible_start = line_index.position_to_byte_offset(&source, params.range.start);
        let visible_end = line_index.position_to_byte_offset(&source, params.range.end);
        trace_lsp_stage("inlay_range_position_conversion", range_conversion_started);
        let call_sites_started = std::time::Instant::now();
        let call_sites = call_sites_in_range(&analysis, visible_start, visible_end);
        trace_lsp_stage("inlay_call_site_filter", call_sites_started);
        trace_lsp_count("inlay_visible_call_sites", call_sites.len());
        let mut pending_hints = Vec::new();
        let mut parameter_cache: HashMap<String, Option<Vec<String>>> = HashMap::new();
        let parameter_generation_started = std::time::Instant::now();
        let mut parameter_resolution = std::time::Duration::ZERO;

        for call in call_sites {
            let parameters = if let Some(parameters) = parameter_cache.get(&call.callee) {
                parameters.clone()
            } else {
                let started = std::time::Instant::now();
                let parameters = self
                    .call_parameter_names(&uri, &analysis, &call.callee)
                    .await;
                parameter_resolution = parameter_resolution.saturating_add(started.elapsed());
                parameter_cache.insert(call.callee.clone(), parameters.clone());
                parameters
            };
            let Some(parameters) = parameters else {
                continue;
            };
            let mut push_hint = |offset: usize, label: String| {
                pending_hints.push((offset, label, InlayHintKind::PARAMETER));
            };
            if let (Some(piped), Some(name), Some(first_arg)) = (
                &call.piped_value,
                parameters.first(),
                call.arguments.first(),
            ) && is_valid_inlay_argument_range(&source, first_arg, visible_start, visible_end)
                && let Some(text) = source.get(piped.clone())
                && let Some(label) = parameter_inlay_label(name)
            {
                push_hint(first_arg.start, format!("{label}: {text},"));
            }
            for (argument, parameter) in call
                .arguments
                .iter()
                .zip(parameters.iter().skip(call.arg_offset))
            {
                if is_valid_inlay_argument_range(&source, argument, visible_start, visible_end)
                    && let Some(label) = parameter_inlay_label(parameter)
                {
                    push_hint(argument.start, format!("{label}:"));
                }
            }
        }
        trace_lsp_stage(
            "inlay_parameter_hint_generation",
            parameter_generation_started,
        );
        trace_lsp_duration("inlay_signature_parameter_resolution", parameter_resolution);
        trace_lsp_count("inlay_signature_cache_misses", parameter_cache.len());

        let inferred_types_started = std::time::Instant::now();
        for (offset, ty) in inferred_type_hints(&source, &analysis) {
            if offset < visible_start || offset > visible_end {
                continue;
            }
            pending_hints.push((offset, format!(": {ty}"), InlayHintKind::TYPE));
        }
        trace_lsp_stage(
            "inlay_inferred_type_hint_generation",
            inferred_types_started,
        );

        let return_types_started = std::time::Instant::now();
        for (offset, ty) in fn_return_type_hints(&source, &analysis) {
            if offset < visible_start || offset > visible_end {
                continue;
            }
            pending_hints.push((offset, format!(": {ty}"), InlayHintKind::TYPE));
        }
        trace_lsp_stage("inlay_return_type_hint_generation", return_types_started);
        let position_conversion_started = std::time::Instant::now();
        let hints = pending_hints
            .into_iter()
            .map(|(offset, label, kind)| InlayHint {
                position: line_index.byte_offset_to_position(&source, offset),
                label: InlayHintLabel::String(label),
                kind: Some(kind),
                text_edits: None,
                tooltip: None,
                padding_left: None,
                padding_right: Some(true),
                data: None,
            })
            .collect::<Vec<_>>();
        trace_lsp_stage(
            "inlay_hint_position_conversion",
            position_conversion_started,
        );
        trace_lsp_count("inlay_hints_returned", hints.len());
        if self.document_matches(&uri, &source, version).await
            && self
                .context_generation
                .load(std::sync::atomic::Ordering::Acquire)
                == context_generation
        {
            let mut cache = self.documents.write().await;
            if let Some(cached) = cache.get_mut(&uri).and_then(|document| {
                document.cached_analysis_mut(&source, version, context_generation)
            }) {
                cache_inlay_hints(&mut cached.inlay_hints, range_key, hints.clone());
            }
            crate::backend::enforce_analysis_cache_limits(&mut cache);
        }
        if !self.current_document_matches(&uri, &source).await {
            return Ok(None);
        }
        Ok(Some(hints))
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let _trace = trace_lsp_span("provider_code_lens");
        let work_done_token = params.work_done_progress_params.work_done_token.clone();
        let uri = params.text_document.uri;
        if !is_tyrus_uri(&uri) {
            return Ok(Some(Vec::new()));
        }
        let diagnostics_lock_wait_started = std::time::Instant::now();
        let documents = self.documents.read().await;
        trace_lsp_stage("diagnostics_cache_lock_wait", diagnostics_lock_wait_started);
        let diagnostics_lock_hold_started = std::time::Instant::now();
        let diagnostics = documents
            .get(&uri)
            .map(|document| document.published_diagnostics.clone())
            .unwrap_or_default();
        drop(documents);
        trace_lsp_stage("diagnostics_cache_lock_hold", diagnostics_lock_hold_started);
        if diagnostics.is_empty() {
            return Ok(Some(Vec::new()));
        }
        let Some(source) = self.source(&uri).await else {
            return Ok(Some(Vec::new()));
        };
        let source = Arc::new(source);
        let Some(current_path) = uri.to_file_path().ok() else {
            return Ok(Some(Vec::new()));
        };
        let mut exports = if quick_fixes_need_workspace_exports(&diagnostics) {
            let _trace = trace_lsp_span("quick_fix_workspace_exports");
            self.exported_symbols(Some(&current_path), None, work_done_token.clone())
                .await
        } else {
            Vec::new()
        };
        self.add_alias_import_sources(&uri, &mut exports).await;
        let lint_diagnostics = if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.source.as_deref() == Some("tyrus-lint"))
        {
            self.lint_diagnostics_for(&uri, source.as_str()).await
        } else {
            Vec::new()
        };
        let analysis = if diagnostics
            .iter()
            .any(|diagnostic| diagnostic_code_is(diagnostic, "E3308"))
        {
            Some(self.analysis_for(&uri, source.as_str()).await)
        } else {
            None
        };
        let actions = quick_fixes_for_diagnostics_on_worker(
            Arc::clone(&self.quick_fix_semaphore),
            &uri,
            Arc::clone(&source),
            current_path,
            exports,
            lint_diagnostics,
            diagnostics,
            analysis,
        )
        .await;
        if !self.current_document_matches(&uri, source.as_str()).await {
            return Ok(Some(Vec::new()));
        }
        Ok(Some(quick_fix_code_lenses(&actions)))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let _trace = trace_lsp_span("provider_code_action");
        let work_done_token = params.work_done_progress_params.work_done_token.clone();
        let uri = params.text_document.uri;
        if self.is_env_document(&uri).await {
            let Some(source) = self.source(&uri).await else {
                return Ok(None);
            };
            let diagnostics = params
                .context
                .diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic_range_intersects_request(diagnostic.range, params.range)
                })
                .cloned()
                .collect::<Vec<_>>();
            return Ok(Some(env_code_actions(&uri, &source, &diagnostics)));
        }
        if !is_tyrus_uri(&uri) {
            return Ok(None);
        }
        let Some(source) = self.source(&uri).await else {
            return Ok(None);
        };
        let source = Arc::new(source);
        let diagnostics_lock_wait_started = std::time::Instant::now();
        let documents = self.documents.read().await;
        trace_lsp_stage("diagnostics_cache_lock_wait", diagnostics_lock_wait_started);
        let diagnostics_lock_hold_started = std::time::Instant::now();
        let published_diagnostics = documents
            .get(&uri)
            .map(|document| document.published_diagnostics.clone())
            .unwrap_or_default();
        drop(documents);
        trace_lsp_stage("diagnostics_cache_lock_hold", diagnostics_lock_hold_started);
        let diagnostics = params
            .context
            .diagnostics
            .into_iter()
            .filter(|requested| {
                diagnostic_range_intersects_request(requested.range, params.range)
                    && published_diagnostics
                        .iter()
                        .any(|published| same_diagnostic_identity(requested, published))
            })
            .collect::<Vec<_>>();
        let Some(current_path) = uri.to_file_path().ok() else {
            return Ok(None);
        };
        let mut exports = if quick_fixes_need_workspace_exports(&diagnostics) {
            let _trace = trace_lsp_span("quick_fix_workspace_exports");
            self.exported_symbols(Some(&current_path), None, work_done_token)
                .await
        } else {
            Vec::new()
        };
        self.add_alias_import_sources(&uri, &mut exports).await;
        let lint_fixes = if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.source.as_deref() == Some("tyrus-lint"))
        {
            self.lint_diagnostics_for(&uri, source.as_str()).await
        } else {
            Vec::new()
        };
        let analysis = if diagnostics
            .iter()
            .any(|diagnostic| diagnostic_code_is(diagnostic, "E3308"))
        {
            Some(self.analysis_for(&uri, source.as_str()).await)
        } else {
            None
        };
        let mut actions = quick_fixes_for_diagnostics_on_worker(
            Arc::clone(&self.quick_fix_semaphore),
            &uri,
            Arc::clone(&source),
            current_path,
            exports,
            lint_fixes,
            diagnostics.clone(),
            analysis,
        )
        .await;
        if let Some(action) = sql_explain_code_action(&uri, source.as_str(), params.range) {
            actions.push(action);
        }
        if trace_lsp_enabled() {
            let mut quick_fix_coverage =
                std::collections::BTreeMap::<String, (usize, usize, usize)>::new();
            let mut eligible_diagnostics = HashMap::<String, String>::new();
            for diagnostic in &diagnostics {
                let code = diagnostic_code(diagnostic).unwrap_or_else(|| "unknown".to_string());
                let counts = quick_fix_coverage.entry(code.clone()).or_default();
                counts.0 += 1;
                if diagnostic_has_quick_fix_candidate(diagnostic) {
                    counts.1 += 1;
                    eligible_diagnostics.insert(quick_fix_coverage_key(diagnostic), code);
                }
            }
            let mut diagnostics_with_actions = std::collections::HashSet::new();
            for action in &actions {
                if let CodeActionOrCommand::CodeAction(action) = action
                    && let Some(diagnostics) = &action.diagnostics
                {
                    for diagnostic in diagnostics {
                        let key = quick_fix_coverage_key(diagnostic);
                        if eligible_diagnostics.contains_key(&key) {
                            diagnostics_with_actions.insert(key);
                        }
                    }
                }
            }
            for key in diagnostics_with_actions {
                if let Some(code) = eligible_diagnostics.get(&key) {
                    quick_fix_coverage.entry(code.clone()).or_default().2 += 1;
                }
            }
            for (code, (diagnostic_count, eligible_count, action_count)) in quick_fix_coverage {
                trace_lsp_event(&format!(
                    "quick_fix_coverage code={code} diagnostics={diagnostic_count} eligible={eligible_count} with_actions={action_count} empty_eligible={}",
                    eligible_count.saturating_sub(action_count)
                ));
            }
        }
        if !self.current_document_matches(&uri, source.as_str()).await {
            return Ok(None);
        }
        Ok(Some(actions))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let _trace = trace_lsp_span("provider_semantic_tokens");
        let uri = params.text_document.uri;
        if !is_tyrus_uri(&uri) {
            return Ok(None);
        }
        let Some(source) = self.source(&uri).await else {
            return Ok(None);
        };
        if let Some(tokens) = self.cached_semantic_tokens(&uri, &source).await {
            if !self.current_document_matches(&uri, &source).await {
                return Ok(None);
            }
            return Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
                result_id: None,
                data: tokens.as_ref().clone(),
            })));
        }
        let analysis = self.analysis_for(&uri, &source).await;
        if !self.current_document_matches(&uri, &source).await {
            return Ok(None);
        }
        let mut entries: Vec<(std::ops::Range<usize>, u32)> = Vec::new();
        for range in analysis.parameter_refs(&source) {
            entries.push((range, SEMANTIC_TOKEN_PARAMETER));
        }
        for symbol in analysis
            .symbols
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::Parameter)
        {
            entries.push((symbol.declaration.clone(), SEMANTIC_TOKEN_PARAMETER));
        }
        for range in reserved_keyword_ranges(&source) {
            entries.push((range, SEMANTIC_TOKEN_KEYWORD));
        }
        for range in comptime_keyword_ranges(&source) {
            entries.push((range, SEMANTIC_TOKEN_KEYWORD));
        }
        let data = encode_semantic_tokens(&source, entries);
        if !self.current_document_matches(&uri, &source).await {
            return Ok(None);
        }
        self.store_semantic_tokens(&uri, &source, Arc::new(data.clone()))
            .await;
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data,
        })))
    }
}

pub(super) fn quick_fixes_need_workspace_exports(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|diagnostic| {
        diagnostic_code_is(diagnostic, "E0425")
            && diagnostic
                .message
                .starts_with("use of undeclared identifier `")
    })
}

fn quick_fix_coverage_key(diagnostic: &Diagnostic) -> String {
    format!(
        "{}:{}:{}:{}:{}:{}:{}",
        diagnostic.source.as_deref().unwrap_or_default(),
        diagnostic_code(diagnostic).unwrap_or_default(),
        diagnostic.range.start.line,
        diagnostic.range.start.character,
        diagnostic.range.end.line,
        diagnostic.range.end.character,
        diagnostic.message,
    )
}

pub(super) fn diagnostic_has_quick_fix_candidate(diagnostic: &Diagnostic) -> bool {
    let code = diagnostic_code(diagnostic).unwrap_or_default();
    if diagnostic.source.as_deref() == Some("tyrus-lint") {
        return tyrus_lint::RULES
            .iter()
            .any(|rule| rule.name == code && rule.fixable)
            || code == "http-no-raw-status-code";
    }
    matches!(
        code.as_str(),
        "E0308"
            | "E0310"
            | "E0382"
            | "E0412"
            | "E0599"
            | "E0706"
            | "E0708"
            | "E0795"
            | "E2743"
            | "E3000"
            | "E3011"
            | "E3031"
            | "E3074"
            | "E3075"
            | "E3079"
            | "E3298"
            | "E3303"
            | "E3308"
            | "E3311"
            | "E3312"
    ) || is_validator_diagnostic(diagnostic)
        || is_promise_modifier_diagnostic(diagnostic)
        || code == tyrus_core::comptime::INVALID_TARGET
        || code == tyrus_core::comptime::NON_CONSTANT
        || (code == "E0425"
            && diagnostic
                .message
                .starts_with("use of undeclared identifier `"))
        || diagnostic_help_strings(diagnostic).iter().any(|help| {
            diagnostic_change_annotation_to_borrow_help(help).is_some()
                || diagnostic_change_callback_parameter_type_help(help).is_some()
                || diagnostic_change_async_return_type_help(help).is_some()
                || diagnostic_object_import_help(help).is_some()
                || diagnostic_object_export_help(help).is_some()
        })
}

pub(super) fn is_newer_document_version(current: Option<i32>, incoming: i32) -> bool {
    current.is_none_or(|current| incoming > current)
}

pub(super) fn completion_identifier_prefix(source: &str, offset: usize) -> Option<&str> {
    let before = source.get(..offset)?;
    let end = before.trim_end().len();
    let prefix = before.get(..end)?;
    let start = prefix
        .char_indices()
        .rev()
        .take_while(|(_, character)| character.is_alphanumeric() || *character == '_')
        .last()
        .map(|(index, _)| index)?;
    let identifier = prefix.get(start..)?;
    identifier
        .chars()
        .next()
        .is_some_and(|character| character.is_alphabetic() || character == '_')
        .then_some(identifier)
}

pub(crate) fn parameter_inlay_label(parameter: &str) -> Option<&str> {
    (!parameter.starts_with(['[', '{'])).then_some(parameter)
}

pub(crate) fn call_sites_in_range(
    analysis: &DocumentAnalysis,
    visible_start: usize,
    visible_end: usize,
) -> &[tyrus_core::analysis::CallSite] {
    if analysis.call_site_prefix_max_end.len() != analysis.call_sites.len() {
        return &analysis.call_sites;
    }
    let first = analysis
        .call_site_prefix_max_end
        .partition_point(|maximum_end| *maximum_end < visible_start);
    let end = analysis
        .call_sites
        .partition_point(|call| call.range.start <= visible_end);
    &analysis.call_sites[first.min(end)..end]
}

fn automatic_data_hover_at(
    source: &str,
    analysis: &DocumentAnalysis,
    offset: usize,
) -> Option<(std::ops::Range<usize>, String)> {
    let call = analysis.call_site_at(offset)?;
    let receiver = call.receiver.as_deref()?;
    let automatic = tyrus_core::automatic_data_methods::AutomaticDataMethod::instance(&call.callee);
    let (automatic, type_name) = if let Some(automatic) = automatic {
        let Type::Named(type_name) = analysis
            .symbols
            .iter()
            .find(|symbol| symbol.name == receiver)
            .and_then(|symbol| symbol.semantic_type.as_ref())?
        else {
            return None;
        };
        (automatic, type_name.clone())
    } else {
        let automatic =
            tyrus_core::automatic_data_methods::AutomaticDataMethod::static_method(&call.callee)?;
        let is_data_interface = analysis.symbols.iter().any(|symbol| {
            symbol.name == receiver
                && symbol.kind == SymbolKind::Interface
                && matches!(symbol.semantic_type, Some(Type::Object(_)))
        });
        if !is_data_interface {
            return None;
        }
        (automatic, receiver.to_string())
    };
    let marker = format!(".{}", automatic.name());
    let method_start = source[call.range.clone()].find(&marker)? + call.range.start + 1;
    let range = method_start..method_start + automatic.name().len();
    if !range.contains(&offset) {
        return None;
    }
    let (signature, description) = match automatic {
        tyrus_core::automatic_data_methods::AutomaticDataMethod::Copy => (
            format!("{type_name} copy(): {type_name}"),
            "Creates an independent structural copy.",
        ),
        tyrus_core::automatic_data_methods::AutomaticDataMethod::Debug => (
            format!("{type_name} toString(): string"),
            "Returns a readable structural representation.",
        ),
        tyrus_core::automatic_data_methods::AutomaticDataMethod::Eq => (
            format!("{type_name} equals(other: {type_name}): boolean"),
            "Compares eligible fields structurally.",
        ),
        tyrus_core::automatic_data_methods::AutomaticDataMethod::JsonEncode => (
            format!("{type_name} toJson(options?: {{ pretty: boolean }}): string"),
            "Encodes this value as canonical JSON. Use `pretty: true` for four-space indentation.",
        ),
        tyrus_core::automatic_data_methods::AutomaticDataMethod::JsonDecode => (
            format!("{type_name}.fromJson(text: string): Result<{type_name}, JsonError>"),
            "Validates JSON and decodes a new value of this interface.",
        ),
    };
    Some((
        range,
        format!("```tyrus\n{signature}\n```\n\nAutomatic data method. {description}"),
    ))
}
#[cfg(test)]
pub(super) fn quick_fixes_for_diagnostics(
    uri: &Url,
    source: &str,
    current_path: &Path,
    exports: &[IndexedExport],
    lint_diagnostics: &[tyrus_lint::LintDiagnostic],
    diagnostics: &[Diagnostic],
) -> Vec<CodeActionOrCommand> {
    quick_fixes_for_diagnostics_with_analysis(
        uri,
        source,
        current_path,
        exports,
        lint_diagnostics,
        diagnostics,
        None,
    )
}

fn quick_fixes_for_diagnostics_with_analysis(
    uri: &Url,
    source: &str,
    current_path: &Path,
    exports: &[IndexedExport],
    lint_diagnostics: &[tyrus_lint::LintDiagnostic],
    diagnostics: &[Diagnostic],
    analysis: Option<&tyrus_core::analysis::DocumentAnalysis>,
) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();
    let lint_fix_index = index_lint_fixes(lint_diagnostics);
    let fallback_analysis = (analysis.is_none()
        && diagnostics
            .iter()
            .any(|diagnostic| diagnostic_code_is(diagnostic, "E3308")))
    .then(|| analyze(source));
    let analysis = analysis.or(fallback_analysis.as_ref());
    let needs_shared_tokens = diagnostics.iter().any(|diagnostic| {
        diagnostic_code_is(diagnostic, "E3000")
            || diagnostic_code_is(diagnostic, "E3011")
            || diagnostic_code_is(diagnostic, "E3298")
    });
    let shared_tokens = needs_shared_tokens.then(|| {
        let started = std::time::Instant::now();
        let tokens = tyrus_lint::lex(source);
        trace_lsp_stage("quick_fix_token_lex", started);
        trace_lsp_count("quick_fix_tokens", tokens.len());
        tokens
    });
    let shared_tokens = shared_tokens.as_deref().unwrap_or_default();
    for diagnostic in diagnostics {
        if diagnostic_code_is(diagnostic, "E3074") {
            let helps = diagnostic_help_strings(diagnostic);
            let needs_ok = helps.iter().any(|help| help.contains("`Ok(...)`"));
            let needs_err = helps.iter().any(|help| help.contains("`Err(...)`"));
            if needs_ok && needs_err {
                push_result_variant_code_actions(&mut actions, uri, source, diagnostic);
            } else if needs_ok {
                push_result_constructor_code_action(&mut actions, uri, source, diagnostic, "Ok");
            } else if needs_err {
                push_result_constructor_code_action(&mut actions, uri, source, diagnostic, "Err");
            }
            continue;
        }
        if diagnostic_code_is(diagnostic, "E3075") {
            push_result_void_ok_code_action(&mut actions, uri, diagnostic);
            continue;
        }
        if diagnostic_code_is(diagnostic, "E3079") {
            push_optional_presence_code_action(&mut actions, uri, source, diagnostic);
            continue;
        }
        if is_validator_diagnostic(diagnostic) {
            push_validator_code_actions(&mut actions, uri, source, diagnostic);
        }
        if diagnostic_code_is(diagnostic, "E3303")
            && diagnostic
                .message
                .contains("mutation statements do not take a result interface type")
        {
            if let Some(fix) = db_mutation_result_type_argument_fix(source, diagnostic) {
                push_lint_fix_code_action(&mut actions, uri, source, diagnostic, &fix);
            }
            continue;
        }
        let lint_fix = matching_indexed_lint_fix(&lint_fix_index, source, diagnostic);
        if let Some(fix) = lint_fix {
            if diagnostic_code_is(diagnostic, "http-no-raw-status-code") {
                push_http_status_code_fix(&mut actions, uri, source, diagnostic, fix);
            } else {
                push_lint_fix_code_action(&mut actions, uri, source, diagnostic, fix);
            }
        }
        if diagnostic_code_is(diagnostic, "E3011") {
            push_unhandled_throw_code_actions_with_tokens(
                &mut actions,
                uri,
                source,
                shared_tokens,
                diagnostic,
            );
            continue;
        }
        if diagnostic_code_is(diagnostic, "E3000") {
            push_add_async_code_action_with_tokens(
                &mut actions,
                uri,
                source,
                shared_tokens,
                diagnostic,
            );
            continue;
        }
        if diagnostic_code_is(diagnostic, "E3308") {
            if let Some(analysis) = analysis {
                push_protected_import_code_action(
                    &mut actions,
                    source,
                    current_path,
                    analysis,
                    diagnostic,
                );
            }
            continue;
        }
        if is_promise_modifier_diagnostic(diagnostic) {
            push_promise_modifier_code_actions(&mut actions, uri, source, diagnostic);
            continue;
        }
        if diagnostic_code_is(diagnostic, "E3298") {
            push_function_type_throws_code_action_with_tokens(
                &mut actions,
                uri,
                source,
                shared_tokens,
                diagnostic,
            );
            continue;
        }
        if diagnostic_code_is(diagnostic, "E3311") {
            archive_fixes::push_archive_extension_code_action(
                &mut actions,
                uri,
                source,
                diagnostic,
            );
            continue;
        }
        if diagnostic_code_is(diagnostic, "E3312") {
            archive_fixes::push_remove_tar_level_code_action(&mut actions, uri, source, diagnostic);
            continue;
        }
        if diagnostic_code_is(diagnostic, "E3031") {
            push_call_task_directly_code_action(&mut actions, uri, source, diagnostic);
            continue;
        }
        if diagnostic_code_is(diagnostic, tyrus_core::comptime::INVALID_TARGET)
            || diagnostic_code_is(diagnostic, tyrus_core::comptime::NON_CONSTANT)
        {
            push_remove_comptime_wrapper_code_action(&mut actions, uri, source, diagnostic);
            continue;
        }
        if diagnostic_code_is(diagnostic, "E0795") {
            if let Some(category) = diagnostic_help_strings(diagnostic)
                .iter()
                .find_map(|help| typeof_category_replacement(help))
            {
                push_replace_diagnostic_range_code_action(
                    &mut actions,
                    uri,
                    diagnostic,
                    format!("Replace with `{category}`"),
                    category,
                );
            }
            continue;
        }
        if diagnostic_code_is(diagnostic, "E0708") {
            if let Some(range) = annotation_range_before(source, diagnostic.range.start) {
                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Change type annotation to `string`".to_string(),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![diagnostic.clone()]),
                    edit: Some(WorkspaceEdit {
                        changes: Some(HashMap::from([(
                            uri.clone(),
                            vec![TextEdit {
                                range,
                                new_text: "string".to_string(),
                            }],
                        )])),
                        ..WorkspaceEdit::default()
                    }),
                    is_preferred: Some(true),
                    ..CodeAction::default()
                }));
            }
            continue;
        }
        if diagnostic_code_is(diagnostic, "E2743") {
            push_trim_cors_origin_code_action(&mut actions, uri, source, diagnostic);
            continue;
        }
        if diagnostic_code_is(diagnostic, "E0412") {
            push_import_native_type_code_action(&mut actions, uri, source, diagnostic);
            continue;
        }
        if diagnostic_code_is(diagnostic, "E0706") {
            push_widen_integer_annotation_code_action(&mut actions, uri, source, diagnostic);
            continue;
        }
        if diagnostic_code_is(diagnostic, "E0308")
            && let Some((expected, _)) =
                tyrus_core::analysis::numeric_mismatch_types(&diagnostic.message)
        {
            push_numeric_cast_code_action(&mut actions, uri, source, diagnostic, &expected);
        }
        if diagnostic_code_is(diagnostic, "E0310") {
            push_annotate_object_literal_code_actions(&mut actions, uri, source, diagnostic);
            continue;
        }
        if diagnostic_code_is(diagnostic, "E0382")
            && diagnostic.message.contains("use of moved value")
        {
            push_borrow_instead_of_move_code_actions(&mut actions, uri, source, diagnostic);
            continue;
        }
        if diagnostic_code_is(diagnostic, "E0412")
            && let Some(name) = diagnostic
                .message
                .strip_prefix("cannot find type `")
                .and_then(|value| value.strip_suffix("` in this scope"))
            && let Some(module) = tyrus_core::module_loader::native_interface_module(name)
        {
            push_import_code_action(&mut actions, uri, source, diagnostic, name, module, false);
        }
        if let Some(name) = diagnostic
            .message
            .strip_prefix("use of undeclared identifier `")
            .and_then(|value| value.strip_suffix('`'))
        {
            for export in exports.iter().filter(|export| export.name == name) {
                let Some(module) = export
                    .module_source
                    .clone()
                    .or_else(|| import_source_for_export(Some(current_path), &export.uri))
                else {
                    continue;
                };
                push_import_code_action(
                    &mut actions,
                    uri,
                    source,
                    diagnostic,
                    name,
                    &module,
                    export.is_default,
                );
            }
            continue;
        }
        for help in diagnostic_help_strings(diagnostic) {
            if let Some(ty) = diagnostic_change_annotation_to_borrow_help(&help) {
                push_replace_diagnostic_range_code_action(
                    &mut actions,
                    uri,
                    diagnostic,
                    format!("Change the annotation to `{ty}`"),
                    ty,
                );
            } else if let Some(ty) = diagnostic_change_callback_parameter_type_help(&help) {
                push_replace_diagnostic_range_code_action(
                    &mut actions,
                    uri,
                    diagnostic,
                    format!("Change callback parameter type to `{ty}`"),
                    ty,
                );
            } else if let Some(ty) = diagnostic_change_async_return_type_help(&help) {
                push_replace_diagnostic_range_code_action(
                    &mut actions,
                    uri,
                    diagnostic,
                    format!("Change return type to `{ty}`"),
                    ty,
                );
            } else if let Some((name, module)) = diagnostic_object_import_help(&help) {
                push_object_interface_code_action(
                    &mut actions,
                    uri,
                    source,
                    diagnostic,
                    &name,
                    &module,
                );
            } else if let Some((name, path)) = diagnostic_object_export_help(&help) {
                push_export_import_and_annotate_code_action(
                    &mut actions,
                    current_path,
                    uri,
                    source,
                    diagnostic,
                    &name,
                    &path,
                );
            }
        }
    }
    deduplicate_equivalent_quick_fixes(&mut actions);
    actions
}

pub(super) fn deduplicate_equivalent_quick_fixes(actions: &mut Vec<CodeActionOrCommand>) {
    let mut seen = std::collections::HashSet::new();
    actions.retain(|entry| {
        let CodeActionOrCommand::CodeAction(action) = entry else {
            return true;
        };
        let Some(identity) = quick_fix_identity(action) else {
            return true;
        };
        seen.insert(identity)
    });
}

fn quick_fix_identity(action: &CodeAction) -> Option<String> {
    if action.command.is_some() {
        return None;
    }
    let edit = action.edit.as_ref()?;
    if edit.document_changes.is_some() || edit.change_annotations.is_some() {
        return None;
    }
    let mut changes = edit
        .changes
        .as_ref()?
        .iter()
        .map(|(uri, edits)| {
            let mut edits = edits
                .iter()
                .map(|edit| {
                    (
                        edit.range.start.line,
                        edit.range.start.character,
                        edit.range.end.line,
                        edit.range.end.character,
                        edit.new_text.clone(),
                    )
                })
                .collect::<Vec<_>>();
            edits.sort();
            (uri.as_str().to_string(), edits)
        })
        .collect::<Vec<_>>();
    changes.sort_by(|left, right| left.0.cmp(&right.0));
    let mut diagnostics = action
        .diagnostics
        .as_ref()?
        .iter()
        .map(|diagnostic| {
            (
                diagnostic_code(diagnostic),
                diagnostic.source.clone(),
                diagnostic.message.clone(),
                diagnostic.range.start.line,
                diagnostic.range.start.character,
                diagnostic.range.end.line,
                diagnostic.range.end.character,
            )
        })
        .collect::<Vec<_>>();
    diagnostics.sort();
    Some(format!(
        "{:?}",
        (
            &action.title,
            action.kind.clone(),
            action.is_preferred,
            diagnostics,
            changes
        )
    ))
}

fn sql_explain_code_action(
    uri: &Url,
    source: &str,
    selection: Range,
) -> Option<CodeActionOrCommand> {
    let start = position_to_byte_offset(source, selection.start);
    let end = position_to_byte_offset(source, selection.end);
    let offsets = if start == end {
        vec![start]
    } else {
        vec![
            start,
            start.saturating_add(1),
            start + (end - start) / 2,
            end.saturating_sub(1),
        ]
    };
    let offset = offsets
        .into_iter()
        .find(|offset| sql_literal_at(source, *offset).is_some())?;
    Some(CodeActionOrCommand::Command(Command {
        title: "Explain SQL query".to_string(),
        command: "tyrus.db.explainSql".to_string(),
        arguments: Some(vec![
            serde_json::Value::String(uri.to_string()),
            serde_json::Value::from(offset),
        ]),
    }))
}

pub(super) fn quick_fix_code_lenses(actions: &[CodeActionOrCommand]) -> Vec<CodeLens> {
    let mut order: Vec<usize> = (0..actions.len())
        .filter(|&index| match &actions[index] {
            CodeActionOrCommand::CodeAction(action) => {
                action.kind == Some(CodeActionKind::QUICKFIX)
                    && action.edit.is_some()
                    && action
                        .diagnostics
                        .as_ref()
                        .is_some_and(|diagnostics| !diagnostics.is_empty())
            }
            CodeActionOrCommand::Command(_) => false,
        })
        .collect();
    order.sort_by_key(|&index| match &actions[index] {
        CodeActionOrCommand::CodeAction(action) => !action.is_preferred.unwrap_or(false),
        CodeActionOrCommand::Command(_) => true,
    });

    let command_name = "tyrus.applyFix".to_string();
    let mut seen = std::collections::HashSet::new();
    let mut lenses = Vec::new();
    for index in order {
        let action = match &actions[index] {
            CodeActionOrCommand::CodeAction(action) => action,
            CodeActionOrCommand::Command(_) => continue,
        };
        let diagnostic_range = action
            .diagnostics
            .as_ref()
            .and_then(|d| d.first())
            .map(|d| d.range);
        let Some(range) = diagnostic_range else {
            continue;
        };
        let key = (
            range.start.line,
            range.start.character,
            range.end.line,
            range.end.character,
            action.title.clone(),
        );
        if !seen.insert(key) {
            continue;
        }
        let Some(edit) = action.edit.clone() else {
            continue;
        };
        let Ok(arguments) = serde_json::to_value(edit) else {
            continue;
        };
        lenses.push(CodeLens {
            range,
            command: Some(Command {
                title: action.title.clone(),
                command: command_name.clone(),
                arguments: Some(vec![arguments]),
            }),
            data: None,
        });
    }
    lenses
}

pub(super) fn push_borrow_instead_of_move_code_actions(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
) {
    let Some(moved_name) = diagnostic.message.split('`').nth(1) else {
        return;
    };
    let Some(move_site) = diagnostic.related_information.as_ref().and_then(|related| {
        related
            .iter()
            .find(|entry| entry.message.starts_with("value moved here"))
            .map(|entry| entry.location.range)
    }) else {
        return;
    };
    let offset = position_to_byte_offset(source, move_site.start);
    let line_start = source[..offset].rfind('\n').map_or(0, |index| index + 1);
    let line_end = source[offset..]
        .find('\n')
        .map_or(source.len(), |index| offset + index);
    let indent_end = source[line_start..line_end]
        .find(|character: char| !character.is_whitespace())
        .map_or(line_start, |shift| line_start + shift);
    let statement = &source[indent_end..line_end];

    let annotated_without_borrow = declaration_annotation(statement)
        .is_some_and(|annotation| !annotation.trim_start().starts_with('&'));

    if !annotated_without_borrow {
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: format!("Borrow `{moved_name}` instead of moving it"),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(WorkspaceEdit {
                changes: Some(HashMap::from([(
                    uri.clone(),
                    vec![TextEdit {
                        range: move_site,
                        new_text: format!("&{moved_name}"),
                    }],
                )])),
                ..WorkspaceEdit::default()
            }),
            is_preferred: Some(true),
            ..CodeAction::default()
        }));
    }

    let example = diagnostic_help_strings(diagnostic)
        .into_iter()
        .find_map(|help| {
            help.strip_prefix("for example: `")
                .and_then(|rest| rest.strip_suffix('`'))
                .map(str::to_string)
        });
    let Some(example) = example else {
        return;
    };
    if !example.starts_with("const ") && !example.starts_with("let ") {
        return;
    }
    if !statement.contains('=') {
        return;
    }
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Replace the declaration with `{example}`"),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![TextEdit {
                    range: byte_range_to_lsp(source, indent_end..line_end),
                    new_text: example,
                }],
            )])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: annotated_without_borrow.then_some(true),
        ..CodeAction::default()
    }));
}

fn declaration_annotation(statement: &str) -> Option<&str> {
    let rest = statement
        .strip_prefix("const ")
        .or_else(|| statement.strip_prefix("let "))?;
    let (binding, _) = rest.split_once('=')?;
    let (_, annotation) = binding.split_once(':')?;
    let annotation = annotation.trim();
    (!annotation.is_empty()).then_some(annotation)
}

pub(super) fn push_result_variant_code_actions(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
) {
    let _ = source;
    for (variant, title) in [
        ("Ok", "Return this value as `Ok(...)`"),
        ("Err", "Return this value as `Err(...)`"),
    ] {
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: title.to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(WorkspaceEdit {
                changes: Some(HashMap::from([(
                    uri.clone(),
                    vec![
                        TextEdit {
                            range: Range {
                                start: diagnostic.range.start,
                                end: diagnostic.range.start,
                            },
                            new_text: format!("{variant}("),
                        },
                        TextEdit {
                            range: Range {
                                start: diagnostic.range.end,
                                end: diagnostic.range.end,
                            },
                            new_text: ")".to_string(),
                        },
                    ],
                )])),
                ..WorkspaceEdit::default()
            }),
            is_preferred: Some(variant == "Ok"),
            ..CodeAction::default()
        }));
    }
}

pub(super) fn push_result_constructor_code_action(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
    constructor: &str,
) {
    let _ = source;
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Return this value as `{constructor}(...)`"),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![
                    TextEdit {
                        range: Range {
                            start: diagnostic.range.start,
                            end: diagnostic.range.start,
                        },
                        new_text: format!("{constructor}("),
                    },
                    TextEdit {
                        range: Range {
                            start: diagnostic.range.end,
                            end: diagnostic.range.end,
                        },
                        new_text: ")".to_string(),
                    },
                ],
            )])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }));
}

pub(super) fn push_result_void_ok_code_action(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    diagnostic: &Diagnostic,
) {
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: "Return success as `Ok()`".to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![TextEdit {
                    range: diagnostic.range,
                    new_text: "return Ok();".to_string(),
                }],
            )])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }));
}

pub(super) fn push_remove_comptime_wrapper_code_action(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
) {
    let cause = position_to_byte_offset(source, diagnostic.range.start);
    let Ok(tokens) = tyrus_core::lexer::lex_spanned(source) else {
        return;
    };
    let mut candidates = Vec::new();
    for index in 0..tokens.len().saturating_sub(1) {
        if !matches!(tokens[index].0, tyrus_core::lexer::Token::Comptime)
            || !matches!(tokens[index + 1].0, tyrus_core::lexer::Token::LParen)
        {
            continue;
        }
        let mut depth = 0usize;
        for (token, range) in tokens.iter().skip(index + 1) {
            match token {
                tyrus_core::lexer::Token::LParen => depth += 1,
                tyrus_core::lexer::Token::RParen => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        if tokens[index].1.start <= cause && cause <= range.end {
                            candidates.push((
                                tokens[index].1.start..tokens[index + 1].1.end,
                                range.clone(),
                            ));
                        }
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    let Some((prefix, suffix)) = candidates
        .into_iter()
        .max_by_key(|(prefix, _)| prefix.start)
    else {
        return;
    };
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: "Evaluate this expression at runtime".to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![
                    TextEdit {
                        range: byte_range_to_lsp(source, prefix),
                        new_text: String::new(),
                    },
                    TextEdit {
                        range: byte_range_to_lsp(source, suffix),
                        new_text: String::new(),
                    },
                ],
            )])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }));
}

fn env_code_actions(
    uri: &Url,
    source: &str,
    diagnostics: &[Diagnostic],
) -> Vec<CodeActionOrCommand> {
    let export_diagnostics: Vec<Diagnostic> = diagnostics
        .iter()
        .filter(|diagnostic| {
            matches!(
                &diagnostic.code,
                Some(NumberOrString::String(code)) if code == "E3152"
            )
        })
        .cloned()
        .collect();
    if export_diagnostics.is_empty() {
        return Vec::new();
    }
    match comment_all_raw_decorators_action(uri, source, export_diagnostics) {
        Some(action) => vec![CodeActionOrCommand::CodeAction(action)],
        None => Vec::new(),
    }
}

pub(super) fn comment_all_raw_decorators_action(
    uri: &Url,
    source: &str,
    diagnostics: Vec<Diagnostic>,
) -> Option<CodeAction> {
    let mut edits = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('@') {
            let column = (line.len() - trimmed.len()) as u32;
            edits.push(TextEdit {
                range: Range::new(
                    Position::new(index as u32, column),
                    Position::new(index as u32, column),
                ),
                new_text: "# ".to_string(),
            });
        }
    }
    if edits.is_empty() {
        return None;
    }
    Some(CodeAction {
        title: "Comment all validator decorations for `export`".to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(diagnostics),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(uri.clone(), edits)])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    })
}

pub(super) fn push_widen_integer_annotation_code_action(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
) {
    let Some(ty) = diagnostic_help_strings(diagnostic)
        .iter()
        .find_map(|help| suggested_integer_type_help(help))
    else {
        return;
    };
    let Some(range) = annotation_range_before(source, diagnostic.range.start) else {
        return;
    };
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Change type annotation to `{ty}`"),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![TextEdit {
                    range,
                    new_text: ty,
                }],
            )])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }));
}

fn suggested_integer_type_help(help: &str) -> Option<String> {
    help.strip_prefix("the narrowest type that holds this value is `")?
        .split('`')
        .next()
        .map(ToString::to_string)
}

pub(super) fn push_numeric_cast_code_action(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
    expected: &str,
) {
    let Some(value_site) = diagnostic.related_information.as_ref().and_then(|related| {
        related
            .iter()
            .find(|entry| entry.message.starts_with("value of type `"))
            .map(|entry| entry.location.range)
    }) else {
        return;
    };
    let value = &source[position_to_byte_offset(source, value_site.start)
        ..position_to_byte_offset(source, value_site.end)];
    let replacement = if cast_requires_parentheses(value) {
        format!("({}) as {expected}", value.trim())
    } else {
        format!("{} as {expected}", value.trim())
    };
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Cast the expression to `{expected}`"),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![TextEdit {
                    range: value_site,
                    new_text: replacement,
                }],
            )])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }));
}

fn cast_requires_parentheses(value: &str) -> bool {
    let mut depth: i32 = 0;
    let mut quote: Option<char> = None;
    for ch in value.chars() {
        if let Some(delimiter) = quote {
            if ch == '\\' {
                continue;
            }
            if ch == delimiter {
                quote = None;
            }
            continue;
        }
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            '"' | '\'' => quote = Some(ch),
            '+' | '-' | '*' | '/' | '%' | '<' | '>' | '=' | '!' | '&' | '|' | '^' | '?'
                if depth == 0 =>
            {
                return true;
            }
            _ => {}
        }
    }
    false
}

pub(super) fn push_annotate_object_literal_code_actions(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
) {
    let candidates = diagnostic_help_strings(diagnostic)
        .iter()
        .filter_map(|help| suggested_annotation_help(help))
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return;
    }
    let Some(position) = binding_name_end(source, diagnostic.range.start) else {
        return;
    };
    let only_one = candidates.len() == 1;
    for ty in candidates {
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: format!("Annotate as `{ty}`"),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(WorkspaceEdit {
                changes: Some(HashMap::from([(
                    uri.clone(),
                    vec![TextEdit {
                        range: Range::new(position, position),
                        new_text: format!(": {ty}"),
                    }],
                )])),
                ..WorkspaceEdit::default()
            }),
            is_preferred: Some(only_one),
            ..CodeAction::default()
        }));
    }
}

fn suggested_annotation_help(help: &str) -> Option<String> {
    help.strip_prefix("annotate it as `: ")?
        .strip_suffix('`')
        .map(ToString::to_string)
}

fn binding_name_end(source: &str, name_start: Position) -> Option<Position> {
    let line = source.lines().nth(name_start.line as usize)?;
    let chars: Vec<char> = line.chars().collect();
    let start = name_start.character as usize;
    if start >= chars.len() {
        return None;
    }
    let end = (start..chars.len())
        .find(|index| !(chars[*index].is_alphanumeric() || chars[*index] == '_'))
        .unwrap_or(chars.len());
    Some(Position::new(name_start.line, end as u32))
}

fn annotation_range_before(source: &str, literal: Position) -> Option<Range> {
    let line = source.lines().nth(literal.line as usize)?;
    let chars: Vec<char> = line.chars().collect();
    let before_literal = chars.get(..literal.character as usize)?;
    let equals = before_literal.iter().rposition(|c| *c == '=')?;
    let colon = before_literal[..equals].iter().rposition(|c| *c == ':')?;
    let start = (colon + 1..equals).find(|index| !chars[*index].is_whitespace())?;
    let end = (start..equals)
        .rev()
        .find(|index| !chars[*index].is_whitespace())?
        + 1;
    Some(Range::new(
        Position::new(literal.line, start as u32),
        Position::new(literal.line, end as u32),
    ))
}

fn push_import_code_action(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
    name: &str,
    module: &str,
    is_default: bool,
) {
    let Some(edit) =
        crate::imports::auto_import_edit_with_default(source, module, name, is_default)
    else {
        return;
    };
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Import `{name}` from `{module}`"),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(uri.clone(), vec![edit])])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }));
}

fn push_replace_diagnostic_range_code_action(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    diagnostic: &Diagnostic,
    title: String,
    replacement: String,
) {
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title,
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![TextEdit {
                    range: diagnostic.range,
                    new_text: replacement,
                }],
            )])),
            ..WorkspaceEdit::default()
        }),
        ..CodeAction::default()
    }));
}

pub(super) fn matching_indexed_lint_fix<'a>(
    index: &std::collections::HashMap<(String, usize), &'a tyrus_lint::LintFix>,
    source: &str,
    diagnostic: &Diagnostic,
) -> Option<&'a tyrus_lint::LintFix> {
    let code = diagnostic_code(diagnostic)?;
    let offset = position_to_byte_offset(source, diagnostic.range.start);
    index.get(&(code, offset)).copied()
}

pub(super) fn index_lint_fixes(
    diagnostics: &[tyrus_lint::LintDiagnostic],
) -> std::collections::HashMap<(String, usize), &tyrus_lint::LintFix> {
    let mut index = std::collections::HashMap::new();
    for diagnostic in diagnostics {
        if let Some(fix) = diagnostic.fix.as_ref() {
            index
                .entry((diagnostic.code.clone(), diagnostic.range.start))
                .or_insert(fix);
        }
    }
    index
}

pub(super) fn push_lint_fix_code_action(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
    fix: &tyrus_lint::LintFix,
) {
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: fix.message.clone(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![TextEdit {
                    range: byte_range_to_lsp(source, fix.range.clone()),
                    new_text: fix.replacement.clone(),
                }],
            )])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }));
}

fn push_http_status_code_fix(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
    fix: &tyrus_lint::LintFix,
) {
    let Some(member) = fix.replacement.strip_prefix("HttpStatus.") else {
        return;
    };
    let imports = crate::imports::scan_imports(source);
    let status_import = imports.iter().find(|import| import.source == "tyrus:http");
    let existing_local = status_import.and_then(|import| {
        import
            .specifiers
            .iter()
            .find(|(imported, _)| imported == "HttpStatus")
            .map(|(_, local)| local.clone())
    });
    let (local_name, additional_edit) = if let Some(local_name) = existing_local {
        (local_name, None)
    } else if let Some(import) = status_import {
        let Some(import_text) = source.get(import.range.clone()) else {
            return;
        };
        let Some(relative_brace) = import_text.rfind('}') else {
            return;
        };
        let brace_offset = import.range.start + relative_brace;
        let before_brace = source
            .get(import.range.start..brace_offset)
            .unwrap_or_default();
        let insertion = if before_brace.trim_end().ends_with('{') {
            "HttpStatus"
        } else if before_brace.trim_end().ends_with(',') {
            " HttpStatus"
        } else {
            ", HttpStatus"
        };
        (
            "HttpStatus".to_string(),
            Some(TextEdit {
                range: Range::new(
                    byte_offset_to_position(source, brace_offset),
                    byte_offset_to_position(source, brace_offset),
                ),
                new_text: insertion.to_string(),
            }),
        )
    } else {
        let insertion_offset = imports
            .first()
            .map(|import| import.range.start)
            .or_else(|| {
                tyrus_core::lexer::lex_spanned(source)
                    .ok()
                    .and_then(|tokens| tokens.first().map(|(_, range)| range.start))
            })
            .unwrap_or(0);
        (
            "HttpStatus".to_string(),
            Some(TextEdit {
                range: Range::new(
                    byte_offset_to_position(source, insertion_offset),
                    byte_offset_to_position(source, insertion_offset),
                ),
                new_text: "import { HttpStatus } from \"tyrus:http\";\n".to_string(),
            }),
        )
    };
    let mut edits = vec![TextEdit {
        range: byte_range_to_lsp(source, fix.range.clone()),
        new_text: format!("{local_name}.{member}"),
    }];
    if let Some(additional_edit) = additional_edit {
        edits.push(additional_edit);
    }
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Use `{local_name}.{member}`"),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(uri.clone(), edits)])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }));
}

fn db_mutation_result_type_argument_fix(
    source: &str,
    diagnostic: &Diagnostic,
) -> Option<tyrus_lint::LintFix> {
    let diagnostic_start = position_to_byte_offset(source, diagnostic.range.start);
    let diagnostic_end = position_to_byte_offset(source, diagnostic.range.end);
    let tokens = tyrus_core::lexer::lex_spanned(source).ok()?;
    for (index, (token, range)) in tokens.iter().enumerate() {
        if !matches!(token, tyrus_core::lexer::Token::Ident(name) if name == "query")
            || range.start < diagnostic_start
            || range.end > diagnostic_end
            || index < 2
            || !matches!(tokens[index - 1].0, tyrus_core::lexer::Token::Dot)
            || !matches!(&tokens[index - 2].0, tyrus_core::lexer::Token::Ident(name) if name == "db")
            || !matches!(
                tokens.get(index + 1).map(|token| &token.0),
                Some(tyrus_core::lexer::Token::Lt)
            )
        {
            continue;
        }
        let mut depth = 0_usize;
        for (generic_index, (generic_token, generic_range)) in
            tokens.iter().enumerate().skip(index + 1)
        {
            match generic_token {
                tyrus_core::lexer::Token::Lt => depth += 1,
                tyrus_core::lexer::Token::Gt => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        if !matches!(
                            tokens.get(generic_index + 1).map(|token| &token.0),
                            Some(tyrus_core::lexer::Token::LParen)
                        ) {
                            break;
                        }
                        return Some(tyrus_lint::LintFix {
                            message: "Remove the result type argument from this mutation"
                                .to_string(),
                            range: range.start..generic_range.end,
                            replacement: String::new(),
                        });
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn push_trim_cors_origin_code_action(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
) {
    let start = position_to_byte_offset(source, diagnostic.range.start);
    let end = position_to_byte_offset(source, diagnostic.range.end);
    let Some(literal) = source.get(start..end) else {
        return;
    };
    let quote = literal.chars().next().filter(|c| *c == '"' || *c == '\'');
    let Some(quote) = quote else {
        return;
    };
    let Some(origin) = literal
        .strip_prefix(quote)
        .and_then(|rest| rest.strip_suffix(quote))
    else {
        return;
    };
    let Some((scheme, rest)) = origin.split_once("://") else {
        return;
    };
    let Some(index) = rest.find(['/', '?', '#']) else {
        return;
    };
    let trimmed = format!("{scheme}://{}", &rest[..index]);
    if trimmed == origin {
        return;
    }
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Trim the origin to `{trimmed}`"),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![TextEdit {
                    range: diagnostic.range,
                    new_text: format!("{quote}{trimmed}{quote}"),
                }],
            )])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }));
}

fn diagnostic_code_is(diagnostic: &Diagnostic, expected: &str) -> bool {
    matches!(
        diagnostic.code.as_ref(),
        Some(NumberOrString::String(code)) if code == expected
    )
}

pub(super) fn optional_presence_comparison_edit(
    source: &str,
    diagnostic_range: Range,
) -> Option<TextEdit> {
    let none_start = position_to_byte_offset(source, diagnostic_range.start);
    let none_end = position_to_byte_offset(source, diagnostic_range.end);
    let before = source.get(..none_start)?;
    let after = source.get(none_end..)?;

    let operator_end = before.trim_end().len();
    if !["===", "!=="].iter().any(|candidate| {
        before
            .get(..operator_end)
            .is_some_and(|text| text.ends_with(candidate))
    }) {
        let right = after.trim_start();
        let right_offset = after.len() - right.len();
        let operator = ["===", "!=="]
            .iter()
            .find(|candidate| right.starts_with(**candidate))
            .copied()?;
        let value = right[operator.len()..].trim_start();
        let name_len = value
            .char_indices()
            .take_while(|(_, character)| character.is_ascii_alphanumeric() || *character == '_')
            .last()
            .map_or(0, |(offset, character)| offset + character.len_utf8());
        let name = value.get(..name_len)?;
        if name.is_empty()
            || name
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_digit())
        {
            return None;
        }
        let replacement = if operator == "===" {
            format!("!{name}")
        } else {
            name.to_string()
        };
        let value_offset = right_offset + operator.len() + (value.len() - value.trim_start().len());
        return Some(TextEdit {
            range: byte_range_to_lsp(source, none_start..none_end + value_offset + name_len),
            new_text: replacement,
        });
    }
    let (operator_start, operator) = ["===", "!=="].iter().find_map(|candidate| {
        before
            .get(..operator_end)?
            .strip_suffix(candidate)
            .map(|prefix| (prefix.trim_end().len(), *candidate))
    })?;
    let identifier_start = before[..operator_start]
        .trim_end()
        .char_indices()
        .rev()
        .take_while(|(_, character)| character.is_ascii_alphanumeric() || *character == '_')
        .last()
        .map_or(operator_start, |(offset, _)| offset);
    let left_name = before.get(identifier_start..operator_start)?.trim();

    let (range_start, range_end, name) = if !left_name.is_empty() {
        if !left_name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
            || left_name
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_digit())
        {
            return None;
        }
        (identifier_start, none_end, left_name)
    } else {
        let right = after.trim_start();
        let name_len = right
            .char_indices()
            .take_while(|(_, character)| character.is_ascii_alphanumeric() || *character == '_')
            .last()
            .map_or(0, |(offset, character)| offset + character.len_utf8());
        let right_name = right.get(..name_len)?;
        if right_name.is_empty()
            || right_name
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_digit())
        {
            return None;
        }
        let right_offset = after.len() - right.len();
        (none_start, none_end + right_offset + name_len, right_name)
    };

    let replacement = if operator == "===" {
        format!("!{name}")
    } else {
        name.to_string()
    };
    Some(TextEdit {
        range: byte_range_to_lsp(source, range_start..range_end),
        new_text: replacement,
    })
}

fn push_optional_presence_code_action(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
) {
    let Some(edit) = optional_presence_comparison_edit(source, diagnostic.range) else {
        return;
    };
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: if edit.new_text.starts_with('!') {
            "Replace optional comparison with an absence check".to_string()
        } else {
            "Replace optional comparison with a presence check".to_string()
        },
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(uri.clone(), vec![edit])])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }));
}

#[cfg(test)]
pub(super) fn push_unhandled_throw_code_actions(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
) {
    let tokens = tyrus_lint::lex(source);
    push_unhandled_throw_code_actions_with_tokens(actions, uri, source, &tokens, diagnostic);
}

fn push_unhandled_throw_code_actions_with_tokens(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    tokens: &[tyrus_lint::SpannedToken],
    diagnostic: &Diagnostic,
) {
    let effect = diagnostic
        .message
        .split('`')
        .nth(3)
        .unwrap_or("ValidationError");
    let call_offset = position_to_byte_offset(source, diagnostic.range.start);
    let line_start = source[..call_offset]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let line_end = source[call_offset..]
        .find('\n')
        .map_or(source.len(), |index| call_offset + index + 1);
    let line = &source[line_start..line_end];
    let indent = line
        .chars()
        .take_while(|character| character.is_whitespace() && *character != '\n')
        .collect::<String>();
    let wrapped = format!(
        "{indent}try {{\n{indent}    {}{indent}}} catch (error: {effect}) {{\n{indent}    // Handle {effect}.\n{indent}}}\n",
        line.trim_start()
    );
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Wrap in try/catch for `{effect}`"),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![TextEdit {
                    range: byte_range_to_lsp(source, line_start..line_end),
                    new_text: wrapped,
                }],
            )])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }));
    let Some(arrow) = tokens.iter().rev().find(|token| {
        token.range.end <= call_offset && matches!(token.token, tyrus_core::lexer::Token::Arrow)
    }) else {
        return;
    };
    let function_prefix = &source[..arrow.range.start];
    if function_prefix.rfind("const ").is_some_and(|start| {
        function_prefix[start..]
            .trim_start()
            .starts_with("const main")
    }) {
        return;
    }
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Add `throws {effect}` to enclosing function"),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![TextEdit {
                    range: byte_range_to_lsp(source, arrow.range.start..arrow.range.start),
                    new_text: format!("throws {effect} "),
                }],
            )])),
            ..WorkspaceEdit::default()
        }),
        ..CodeAction::default()
    }));
}

#[cfg(test)]
pub(super) fn push_function_type_throws_code_action(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
) {
    let tokens = tyrus_lint::lex(source);
    push_function_type_throws_code_action_with_tokens(actions, uri, source, &tokens, diagnostic);
}

fn push_function_type_throws_code_action_with_tokens(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    tokens: &[tyrus_lint::SpannedToken],
    diagnostic: &Diagnostic,
) {
    use tyrus_core::lexer::Token;
    let Some(effects) = diagnostic.message.split('`').nth(1) else {
        return;
    };
    let offset = position_to_byte_offset(source, diagnostic.range.start);
    let Some((start, end)) = function_type_annotation_tokens(tokens, offset) else {
        return;
    };
    let mut depth = 0i32;
    let mut has_throws = false;
    for token in &tokens[start..end] {
        match token.token {
            Token::LParen | Token::LBracket | Token::LBrace | Token::Lt => depth += 1,
            Token::RParen | Token::RBracket | Token::RBrace | Token::Gt => depth -= 1,
            Token::Throws if depth == 0 => has_throws = true,
            _ => {}
        }
    }
    let insert_at = tokens[end - 1].range.end;
    let new_text = if has_throws {
        format!(" | {effects}")
    } else {
        format!(" throws {effects}")
    };
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Declare `throws {effects}` in the function type"),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![TextEdit {
                    range: byte_range_to_lsp(source, insert_at..insert_at),
                    new_text,
                }],
            )])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }));
}

fn function_type_annotation_tokens(
    tokens: &[tyrus_lint::SpannedToken],
    offset: usize,
) -> Option<(usize, usize)> {
    use tyrus_core::lexer::Token;
    let index = tokens
        .iter()
        .position(|token| token.range.start >= offset)?;
    if index == 0 {
        return None;
    }
    if matches!(tokens[index - 1].token, Token::Eq) {
        let mut depth = 0i32;
        for colon in (0..index - 1).rev() {
            match tokens[colon].token {
                Token::RParen | Token::RBracket | Token::RBrace | Token::Gt => depth += 1,
                Token::LParen | Token::LBracket | Token::LBrace | Token::Lt => depth -= 1,
                Token::Colon if depth == 0 => {
                    return (colon + 1 < index - 1).then_some((colon + 1, index - 1));
                }
                Token::Semicolon | Token::Const | Token::Let if depth == 0 => return None,
                _ => {}
            }
        }
        return None;
    }
    let mut depth = 0i32;
    let mut argument = 0usize;
    let mut open = None;
    for position in (0..index).rev() {
        match tokens[position].token {
            Token::RParen | Token::RBracket | Token::RBrace => depth += 1,
            Token::LParen if depth == 0 => {
                open = Some(position);
                break;
            }
            Token::LParen | Token::LBracket | Token::LBrace => depth -= 1,
            Token::Comma if depth == 0 => argument += 1,
            Token::Semicolon if depth == 0 => return None,
            _ => {}
        }
    }
    let open = open?;
    let Token::Ident(callee) = &tokens.get(open.checked_sub(1)?)?.token else {
        return None;
    };
    let parameters = function_parameters_open(tokens, callee)?;
    let mut depth = 0i32;
    let mut current = 0usize;
    let mut colon = None;
    for (position, spanned) in tokens.iter().enumerate().skip(parameters + 1) {
        match spanned.token {
            Token::LParen | Token::LBracket | Token::LBrace | Token::Lt => depth += 1,
            Token::RParen | Token::RBracket | Token::RBrace | Token::Gt if depth > 0 => depth -= 1,
            Token::RParen | Token::Comma | Token::Eq if depth == 0 => {
                if current == argument {
                    let colon: usize = colon?;
                    return (colon + 1 < position).then_some((colon + 1, position));
                }
                if matches!(tokens[position].token, Token::RParen) {
                    return None;
                }
                if matches!(tokens[position].token, Token::Comma) {
                    current += 1;
                    colon = None;
                }
            }
            Token::Colon if depth == 0 && current == argument && colon.is_none() => {
                colon = Some(position);
            }
            _ => {}
        }
    }
    None
}

fn function_parameters_open(tokens: &[tyrus_lint::SpannedToken], name: &str) -> Option<usize> {
    use tyrus_core::lexer::Token;
    for (position, token) in tokens.iter().enumerate() {
        if !matches!(&token.token, Token::Ident(candidate) if candidate == name) || position == 0 {
            continue;
        }
        let mut next = position + 1;
        match tokens[position - 1].token {
            Token::Function => {}
            Token::Const | Token::Let if matches!(tokens.get(next)?.token, Token::Eq) => {
                next += 1;
                if matches!(tokens.get(next)?.token, Token::Async) {
                    next += 1;
                }
            }
            _ => continue,
        }
        if matches!(tokens.get(next)?.token, Token::Lt) {
            let mut depth = 0i32;
            while let Some(token) = tokens.get(next) {
                match token.token {
                    Token::Lt => depth += 1,
                    Token::Gt => {
                        depth -= 1;
                        if depth == 0 {
                            next += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                next += 1;
            }
        }
        if matches!(tokens.get(next)?.token, Token::LParen) {
            return Some(next);
        }
    }
    None
}

pub(super) fn native_promise_hover_at(
    analysis: &DocumentAnalysis,
    offset: usize,
) -> Option<(std::ops::Range<usize>, String)> {
    let token = analysis.token_at_inclusive(offset)?;
    if !matches!(&token.token, tyrus_core::lexer::Token::Ident(name) if matches!(name.as_str(), "Promise" | "Result" | "Error" | "AggregateError"))
    {
        return None;
    }
    let name = match &token.token {
        tyrus_core::lexer::Token::Ident(name) => name.as_str(),
        _ => unreachable!(),
    };
    if name == "Error" {
        return Some((
            token.range.clone(),
            "Recoverable error type bound by `catch (error)` when no type is written.\n\n```tyrus\ninterface Error {\n    readonly message: string;\n    readonly code: string;\n}\n```"
                .to_string(),
        ));
    }
    if name == "Result" {
        return Some((
            token.range.clone(),
            "Built-in success-or-error result type.\n\n```tyrus\nResult<T, E>\n```\n\nThe first payload is the success value and the second is the error value. Use `match` to inspect the variants or `?` to propagate an error from a Result-returning function.".to_string(),
        ));
    }
    if name == "AggregateError" {
        return Some((
            token.range.clone(),
            "Ordered asynchronous aggregate error used when every input Promise is rejected.\n\n```tyrus\ninterface AggregateError {\n    readonly message: string;\n    readonly errors: Error[];\n}\n```"
                .to_string(),
        ));
    }
    Some((
        token.range.clone(),
        [
            "Native async result type.",
            "",
            "```tyrus",
            "Promise<T>",
            "```",
            "",
            "`Promise<T>` is the native asynchronous result type. Async lowering, suspension, failure propagation, task scheduling, and ownership-aware runtime primitives are available; the public Promise method surface is enabled only as each method completes its runtime and codegen gates.",
        ]
        .join("\n"),
    ))
}

pub(super) fn global_builtin_hover_at(
    analysis: &DocumentAnalysis,
    offset: usize,
) -> Option<(std::ops::Range<usize>, String)> {
    let token = analysis.token_at_inclusive(offset)?;
    if !matches!(&token.token, tyrus_core::lexer::Token::Ident(name) if matches!(name.as_str(), "parseDecimal" | "isNaN"))
    {
        return None;
    }
    let tyrus_core::lexer::Token::Ident(name) = &token.token else {
        return None;
    };
    let (detail, example) = match name.as_str() {
        "parseDecimal" => (
            "(text: string) => decimal?",
            "const value: decimal? = parseDecimal(\"19.3\");",
        ),
        "isNaN" => (
            "(value: f32 | f64 | f128) => boolean",
            "const invalid = isNaN(NaN);",
        ),
        _ => return None,
    };
    Some((
        token.range.clone(),
        hover_markdown(name, detail, Some(example), None),
    ))
}

pub(super) fn is_promise_modifier_diagnostic(diagnostic: &Diagnostic) -> bool {
    diagnostic_code_is(diagnostic, "E0599")
        && (diagnostic.message == "`.toThrow()` does not follow `.resolves`"
            || diagnostic.message
                == "`.resolves` and `.rejects` ask opposite questions of the same promise")
}

fn preceding_chain_modifier(
    source: &str,
    member_start: usize,
) -> Option<(std::ops::Range<usize>, std::ops::Range<usize>, &'static str)> {
    let head = source.get(..member_start)?.trim_end();
    let head = head.strip_suffix('.')?;
    let dot_end = head.len();
    let head = head.trim_end();
    let name_end = head.len();
    let name_start = head
        .rfind(|c: char| !c.is_alphanumeric() && c != '_')
        .map_or(0, |index| index + 1);
    let name = head.get(name_start..name_end)?;
    let modifier = [
        tyrus_core::test::NOT_MODIFIER,
        tyrus_core::test::REJECTS_MODIFIER,
        tyrus_core::test::RESOLVES_MODIFIER,
    ]
    .into_iter()
    .find(|candidate| *candidate == name)?;
    Some((name_start..name_end, name_start..dot_end + 1, modifier))
}

pub(super) fn push_promise_modifier_code_actions(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
) {
    let member_start = position_to_byte_offset(source, diagnostic.range.start);
    let Some((name, with_dot, modifier)) = preceding_chain_modifier(source, member_start) else {
        return;
    };
    let mut fix = |title: String, range: std::ops::Range<usize>, new_text: &str| {
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title,
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(WorkspaceEdit {
                changes: Some(HashMap::from([(
                    uri.clone(),
                    vec![TextEdit {
                        range: byte_range_to_lsp(source, range),
                        new_text: new_text.to_string(),
                    }],
                )])),
                ..WorkspaceEdit::default()
            }),
            ..CodeAction::default()
        }));
    };
    if diagnostic.message == "`.toThrow()` does not follow `.resolves`" {
        if modifier == tyrus_core::test::RESOLVES_MODIFIER {
            fix(
                "Change `.resolves` to `.rejects`".to_string(),
                name,
                tyrus_core::test::REJECTS_MODIFIER,
            );
        }
        return;
    }
    let Some((_, outer_with_dot, outer)) = preceding_chain_modifier(source, name.start) else {
        return;
    };
    fix(
        format!("Keep `.{outer}` and drop `.{modifier}`"),
        with_dot,
        "",
    );
    fix(
        format!("Keep `.{modifier}` and drop `.{outer}`"),
        outer_with_dot,
        "",
    );
}

pub(super) fn push_call_task_directly_code_action(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
) {
    if !diagnostic
        .message
        .contains("awaiting `spawn(...)` directly")
    {
        return;
    }
    let start = position_to_byte_offset(source, diagnostic.range.start);
    let end = position_to_byte_offset(source, diagnostic.range.end);
    let Some(call) = source.get(start..end) else {
        return;
    };
    let call = call.trim_end();
    let (Some(open), true) = (call.find('('), call.ends_with(')')) else {
        return;
    };
    let task = call[open + 1..call.len() - 1].trim();
    if task.is_empty() {
        return;
    }
    let replacement = if task.starts_with('(') || task.starts_with("async") {
        format!("({task})()")
    } else {
        format!("{task}()")
    };
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: "Call the task directly instead of spawning it".to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![TextEdit {
                    range: diagnostic.range,
                    new_text: replacement,
                }],
            )])),
            ..WorkspaceEdit::default()
        }),
        ..CodeAction::default()
    }));
}

pub(super) fn push_import_native_type_code_action(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
) {
    let Some(name) = diagnostic
        .message
        .strip_prefix("cannot find type `")
        .and_then(|rest| rest.split('`').next())
    else {
        return;
    };
    let Some(module) = tyrus_core::module_loader::native_module_exporting(name) else {
        return;
    };
    let Some(edit) = crate::imports::auto_import_edit(source, module, name) else {
        return;
    };
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Import `{name}` from `{module}`"),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(uri.clone(), vec![edit])])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }));
}

/// The only deterministic fix is to unprotect the declaration, which lives in
/// the exporting file. Moving the importing file is the other way out, but that
/// is not a text edit, so it stays in the `help:` text only.
pub(super) fn push_protected_import_code_action(
    actions: &mut Vec<CodeActionOrCommand>,
    source: &str,
    current_path: &Path,
    analysis: &tyrus_core::analysis::DocumentAnalysis,
    diagnostic: &Diagnostic,
) {
    let Some(symbol) = analysis.symbols.iter().find(|symbol| {
        symbol.kind == SymbolKind::Import
            && byte_range_to_lsp(source, symbol.declaration.clone()) == diagnostic.range
    }) else {
        return;
    };
    let Some(module) = symbol.import_source.as_deref() else {
        return;
    };
    let imported = symbol.imported_name.as_deref().unwrap_or(&symbol.name);
    let Ok(Some(target)) = tyrus_core::project::resolve_import_path(current_path, module) else {
        return;
    };
    let Some(violation) =
        tyrus_core::module_loader::protected_import_violation(current_path, &target, imported)
    else {
        return;
    };
    let Some(modifier) = violation.modifier.clone() else {
        return;
    };
    let Ok(declaring_source) = std::fs::read_to_string(&violation.declaring_module) else {
        return;
    };
    let Ok(declaring_uri) = Url::from_file_path(&violation.declaring_module) else {
        return;
    };
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!(
            "Remove `protect` from `{imported}` in `{}`",
            violation.declaring_file()
        ),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                declaring_uri,
                vec![TextEdit {
                    range: byte_range_to_lsp(&declaring_source, modifier),
                    new_text: String::new(),
                }],
            )])),
            ..WorkspaceEdit::default()
        }),
        ..CodeAction::default()
    }));
}

#[cfg(test)]
pub(super) fn push_add_async_code_action(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
) {
    let tokens = tyrus_lint::lex(source);
    push_add_async_code_action_with_tokens(actions, uri, source, &tokens, diagnostic);
}

fn push_add_async_code_action_with_tokens(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    tokens: &[tyrus_lint::SpannedToken],
    diagnostic: &Diagnostic,
) {
    let Some((await_index, _)) = tokens.iter().enumerate().find(|(_, token)| {
        matches!(token.token, tyrus_core::lexer::Token::Await)
            && byte_range_to_lsp(source, token.range.clone()) == diagnostic.range
    }) else {
        return;
    };
    let Some(function_token) = enclosing_non_async_function_token(tokens, await_index) else {
        return;
    };
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: "Add `async` to enclosing function".to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![TextEdit {
                    range: byte_range_to_lsp(
                        source,
                        function_token.range.start..function_token.range.start,
                    ),
                    new_text: "async ".to_string(),
                }],
            )])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }));
}

fn enclosing_non_async_function_token(
    tokens: &[tyrus_lint::SpannedToken],
    await_index: usize,
) -> Option<&tyrus_lint::SpannedToken> {
    for function_index in (0..await_index).rev() {
        let function_token = &tokens[function_index];
        if !matches!(function_token.token, tyrus_core::lexer::Token::Function) {
            continue;
        }
        if function_index > 0
            && matches!(
                tokens[function_index - 1].token,
                tyrus_core::lexer::Token::Async
            )
        {
            continue;
        }
        if !await_is_inside_function_body(tokens, function_index, await_index) {
            continue;
        }
        return Some(function_token);
    }
    None
}

fn await_is_inside_function_body(
    tokens: &[tyrus_lint::SpannedToken],
    function_index: usize,
    await_index: usize,
) -> bool {
    let Some(body_start) = tokens[function_index + 1..await_index]
        .iter()
        .position(|token| matches!(token.token, tyrus_core::lexer::Token::LBrace))
        .map(|offset| function_index + 1 + offset)
    else {
        return false;
    };
    let mut depth = 0usize;
    for token in &tokens[body_start..=await_index] {
        match token.token {
            tyrus_core::lexer::Token::LBrace => depth += 1,
            tyrus_core::lexer::Token::RBrace => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return false;
                }
            }
            tyrus_core::lexer::Token::Await => return depth > 0,
            _ => {}
        }
    }
    false
}

fn push_object_interface_code_action(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
    interface_name: &str,
    module: &str,
) {
    let binding_start = position_to_byte_offset(source, diagnostic.range.start);
    let binding_end = position_to_byte_offset(source, diagnostic.range.end);
    let binding = source.get(binding_start..binding_end);
    let Some(binding) = binding else {
        return;
    };
    let Some(annotation_edit) = object_binding_annotation_edit(source, binding_end, interface_name)
    else {
        return;
    };
    let mut current_edits = Vec::new();
    if let Some(import_edit) = auto_import_edit(source, module, interface_name) {
        current_edits.push(import_edit);
    }
    current_edits.push(annotation_edit);
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Import `{interface_name}` and annotate `{binding}`"),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(uri.clone(), current_edits)])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }));
}

fn push_export_import_and_annotate_code_action(
    actions: &mut Vec<CodeActionOrCommand>,
    current_path: &Path,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
    interface_name: &str,
    interface_path: &str,
) {
    let target_path = PathBuf::from(interface_path);
    let Ok(target_uri) = Url::from_file_path(&target_path) else {
        return;
    };
    let Some(module) = import_source_for_export(Some(current_path), &target_uri) else {
        return;
    };
    let Ok(target_source) = fs::read_to_string(&target_path) else {
        return;
    };
    let Some(export_edit) = export_interface_edit(&target_source, interface_name) else {
        return;
    };
    let binding_start = position_to_byte_offset(source, diagnostic.range.start);
    let binding_end = position_to_byte_offset(source, diagnostic.range.end);
    let binding = source.get(binding_start..binding_end);
    let Some(binding) = binding else {
        return;
    };
    let Some(annotation_edit) = object_binding_annotation_edit(source, binding_end, interface_name)
    else {
        return;
    };
    let mut current_edits = Vec::new();
    if let Some(import_edit) = auto_import_edit(source, &module, interface_name) {
        current_edits.push(import_edit);
    }
    current_edits.push(annotation_edit);
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Export, import, and annotate `{binding}` as `{interface_name}`"),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([
                (uri.clone(), current_edits),
                (target_uri, vec![export_edit]),
            ])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }));
}

fn push_validator_code_actions(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
) {
    let code = diagnostic_code(diagnostic);
    if matches!(code.as_deref(), Some("E3164" | "E3161"))
        && let Some(edit) = remove_validator_decorator_edit(source, diagnostic.range)
    {
        let title = if code.as_deref() == Some("E3164") {
            "Remove redundant validator decoration".to_string()
        } else {
            "Remove incompatible validator decoration".to_string()
        };
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title,
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(WorkspaceEdit {
                changes: Some(HashMap::from([(uri.clone(), vec![edit])])),
                ..WorkspaceEdit::default()
            }),
            is_preferred: Some(code.as_deref() == Some("E3164")),
            ..CodeAction::default()
        }));
    }

    if code.as_deref() == Some("E3161")
        && let Some((from, to)) = invalid_target_validator_replacement(&diagnostic.message)
        && let Some(edit) = replace_validator_name_edit(source, diagnostic.range, &from, &to)
    {
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: format!("Change validator to `@{to}`"),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(WorkspaceEdit {
                changes: Some(HashMap::from([(uri.clone(), vec![edit])])),
                ..WorkspaceEdit::default()
            }),
            is_preferred: Some(true),
            ..CodeAction::default()
        }));
    }

    if matches!(code.as_deref(), Some("E3161" | "E3162" | "E3163" | "E3164"))
        && let Some(edit) = add_validator_message_edit(source, diagnostic.range)
    {
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: "Add custom validator message".to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(WorkspaceEdit {
                changes: Some(HashMap::from([(uri.clone(), vec![edit])])),
                ..WorkspaceEdit::default()
            }),
            ..CodeAction::default()
        }));
    }
}

fn is_validator_diagnostic(diagnostic: &Diagnostic) -> bool {
    diagnostic_code(diagnostic).is_some_and(|code| code.starts_with("validator-decorations-"))
}

fn diagnostic_code(diagnostic: &Diagnostic) -> Option<String> {
    match diagnostic.code.as_ref()? {
        NumberOrString::String(code) => Some(code.clone()),
        NumberOrString::Number(code) => Some(code.to_string()),
    }
}

fn same_diagnostic_identity(left: &Diagnostic, right: &Diagnostic) -> bool {
    left.range == right.range
        && left.code == right.code
        && left.source == right.source
        && left.message == right.message
}

fn diagnostic_range_intersects_request(diagnostic: Range, requested: Range) -> bool {
    let start = |position: Position| (position.line, position.character);
    let diagnostic_start = start(diagnostic.start);
    let diagnostic_end = start(diagnostic.end);
    let requested_start = start(requested.start);
    let requested_end = start(requested.end);

    if requested_start == requested_end {
        diagnostic_start <= requested_start && requested_start <= diagnostic_end
    } else if diagnostic_start == diagnostic_end {
        requested_start <= diagnostic_start && diagnostic_start <= requested_end
    } else {
        diagnostic_start < requested_end && requested_start < diagnostic_end
    }
}

pub(super) fn remove_validator_decorator_edit(source: &str, range: Range) -> Option<TextEdit> {
    let start = position_to_byte_offset(source, range.start);
    let mut end = validator_decorator_end(source, start)?;
    while source
        .as_bytes()
        .get(end)
        .is_some_and(u8::is_ascii_whitespace)
        && !matches!(source.as_bytes().get(end), Some(b'\n' | b'\r'))
    {
        end += 1;
    }
    Some(TextEdit {
        range: Range::new(
            byte_offset_to_position(source, start),
            byte_offset_to_position(source, end),
        ),
        new_text: String::new(),
    })
}

pub(super) fn replace_validator_name_edit(
    source: &str,
    range: Range,
    from: &str,
    to: &str,
) -> Option<TextEdit> {
    let start = position_to_byte_offset(source, range.start);
    let end = position_to_byte_offset(source, range.end);
    let current = source.get(start..end)?;
    if current != format!("@{from}") {
        return None;
    }
    Some(TextEdit {
        range,
        new_text: format!("@{to}"),
    })
}

pub(super) fn add_validator_message_edit(source: &str, range: Range) -> Option<TextEdit> {
    let start = position_to_byte_offset(source, range.start);
    let name_end = position_to_byte_offset(source, range.end);
    let end = validator_decorator_end(source, start)?;
    if validator_has_custom_message(source.get(name_end..end)?) {
        return None;
    }
    if source.as_bytes().get(name_end) == Some(&b'(') {
        let insert_at = end.checked_sub(1)?;
        let prefix = if source
            .get(name_end + 1..insert_at)
            .is_some_and(|args| args.trim().is_empty())
        {
            ""
        } else {
            ", "
        };
        Some(TextEdit {
            range: Range::new(
                byte_offset_to_position(source, insert_at),
                byte_offset_to_position(source, insert_at),
            ),
            new_text: format!("{prefix}\"Validation failed\""),
        })
    } else {
        Some(TextEdit {
            range: Range::new(
                byte_offset_to_position(source, name_end),
                byte_offset_to_position(source, name_end),
            ),
            new_text: "(\"Validation failed\")".to_string(),
        })
    }
}

fn validator_decorator_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if bytes.get(start) != Some(&b'@') {
        return None;
    }
    let mut end = start + 1;
    while bytes
        .get(end)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        end += 1;
    }
    if bytes.get(end) != Some(&b'(') {
        return Some(end);
    }
    let mut depth = 0usize;
    let mut index = end;
    let mut string_quote: Option<u8> = None;
    let mut escaped = false;
    while let Some(byte) = bytes.get(index).copied() {
        if let Some(quote) = string_quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == quote {
                string_quote = None;
            }
            index += 1;
            continue;
        }
        match byte {
            b'"' | b'\'' => {
                string_quote = Some(byte);
                index += 1;
            }
            b'(' => {
                depth += 1;
                index += 1;
            }
            b')' => {
                depth = depth.saturating_sub(1);
                index += 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => index += 1,
        }
    }
    None
}

fn validator_has_custom_message(suffix: &str) -> bool {
    let Some(args) = suffix
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
    else {
        return false;
    };
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut has_string = false;
    for ch in args.chars() {
        if let Some(current) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == current {
                quote = None;
                has_string = true;
            }
        } else if matches!(ch, '"' | '\'') {
            quote = Some(ch);
        }
    }
    has_string
}

pub(super) fn invalid_target_validator_replacement(message: &str) -> Option<(String, String)> {
    let validator = message
        .strip_prefix("`@")
        .and_then(|value| value.split_once('`'))
        .map(|(name, _)| name)?;
    let target = message
        .split("cannot be used with `")
        .nth(1)
        .and_then(|value| value.split_once('`'))
        .map(|(target, _)| target)?;
    let replacement = match (validator, target) {
        ("min", "string") => "minLength",
        ("max", "string") => "maxLength",
        ("min", target) if target.ends_with("[]") => "minItems",
        ("max", target) if target.ends_with("[]") => "maxItems",
        ("minLength", target) if is_numeric_type_name(target) => "min",
        ("maxLength", target) if is_numeric_type_name(target) => "max",
        ("minLength", target) if target.ends_with("[]") => "minItems",
        ("maxLength", target) if target.ends_with("[]") => "maxItems",
        ("length", target) if target.ends_with("[]") => "lengthItems",
        ("lengthItems", "string") => "length",
        ("notEmpty", target) if target.ends_with("[]") => "notEmptyItems",
        ("notEmptyItems", "string") => "notEmpty",
        _ => return None,
    };
    Some((validator.to_string(), replacement.to_string()))
}

fn is_numeric_type_name(value: &str) -> bool {
    matches!(
        value,
        "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "f32"
            | "f64"
            | "f128"
            | "decimal"
    )
}

pub(super) fn object_binding_annotation_edit(
    source: &str,
    binding_end: usize,
    interface_name: &str,
) -> Option<TextEdit> {
    let suffix = source.get(binding_end..)?;
    let equals_offset = suffix.find('=')?;
    let between = &suffix[..equals_offset];
    if between.contains(':') || !between.trim().is_empty() {
        return None;
    }
    Some(TextEdit {
        range: Range::new(
            byte_offset_to_position(source, binding_end),
            byte_offset_to_position(source, binding_end),
        ),
        new_text: format!(": {interface_name}"),
    })
}

pub(super) fn diagnostic_object_import_help(message: &str) -> Option<(String, String)> {
    let start = message.find("import `{ ")? + "import `{ ".len();
    let rest = &message[start..];
    let name_end = rest.find(" }` from `")?;
    let name = &rest[..name_end];
    let module_start = name_end + " }` from `".len();
    let module_rest = &rest[module_start..];
    let module_end = module_rest.find('`')?;
    Some((name.to_string(), module_rest[..module_end].to_string()))
}

pub(super) fn diagnostic_object_export_help(message: &str) -> Option<(String, String)> {
    let start = message.find("interface `")? + "interface `".len();
    let rest = &message[start..];
    let name_end = rest.find("` in `")?;
    let name = &rest[..name_end];
    let path_start = name_end + "` in `".len();
    let path_rest = &rest[path_start..];
    let path_end = path_rest.find('`')?;
    Some((name.to_string(), path_rest[..path_end].to_string()))
}

pub(super) fn diagnostic_change_callback_parameter_type_help(message: &str) -> Option<String> {
    message
        .strip_prefix("change the callback parameter type to `")
        .and_then(|value| value.strip_suffix('`'))
        .map(str::to_string)
}

pub(super) fn diagnostic_change_async_return_type_help(message: &str) -> Option<String> {
    message
        .strip_prefix("change the return type to `")
        .and_then(|value| value.strip_suffix('`'))
        .map(str::to_string)
}

pub(super) fn diagnostic_change_annotation_to_borrow_help(message: &str) -> Option<String> {
    message
        .strip_prefix("change the annotation to `")
        .and_then(|value| value.strip_suffix("` if this binding should store a borrow"))
        .map(str::to_string)
}

pub(super) fn signature_return_type_from_detail(detail: &str) -> Option<String> {
    detail
        .rsplit_once("=>")
        .map(|(_, ret)| ret.trim().to_string())
        .filter(|ret| !ret.is_empty())
}

fn typeof_category_replacement(help: &str) -> Option<String> {
    let (_, replacement) = help.strip_prefix("replace `")?.split_once("` with `")?;
    let replacement = replacement.strip_suffix('`')?;
    let category = replacement.strip_prefix('"')?.strip_suffix('"')?;
    tyrus_core::typeof_category::is_typeof_category(category).then(|| replacement.to_string())
}

fn diagnostic_help_strings(diagnostic: &Diagnostic) -> Vec<String> {
    diagnostic
        .data
        .as_ref()
        .and_then(|data| data.get("help"))
        .and_then(|help| help.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn export_interface_edit(source: &str, interface_name: &str) -> Option<TextEdit> {
    let analysis = analyze(source);
    let symbol = analysis
        .symbols
        .iter()
        .find(|symbol| symbol.kind == SymbolKind::Interface && symbol.name == interface_name)?;
    if symbol.exported {
        return None;
    }
    let insert_at = source[..symbol.declaration.start].rfind("interface")?;
    Some(TextEdit {
        range: Range::new(
            byte_offset_to_position(source, insert_at),
            byte_offset_to_position(source, insert_at),
        ),
        new_text: "export ".to_string(),
    })
}

#[cfg(test)]
pub(crate) fn inferred_type_inlay_hint(source: &str, offset: usize, ty: &str) -> InlayHint {
    inferred_type_inlay_hint_with_index(source, offset, ty, &LineIndex::new(source))
}

#[cfg(test)]
fn inferred_type_inlay_hint_with_index(
    source: &str,
    offset: usize,
    ty: &str,
    line_index: &LineIndex,
) -> InlayHint {
    InlayHint {
        position: line_index.byte_offset_to_position(source, offset),
        label: InlayHintLabel::String(format!(": {ty}")),
        kind: Some(InlayHintKind::TYPE),
        text_edits: None,
        tooltip: None,
        padding_left: None,
        padding_right: Some(true),
        data: None,
    }
}

pub(crate) fn is_valid_inlay_argument_range(
    source: &str,
    argument: &std::ops::Range<usize>,
    visible_start: usize,
    visible_end: usize,
) -> bool {
    argument.start < argument.end
        && argument.end <= source.len()
        && argument.start >= visible_start
        && argument.start <= visible_end
        && source
            .get(argument.clone())
            .and_then(|text| text.trim_start().chars().next())
            .is_some_and(|ch| !matches!(ch, ')' | ']' | '}' | ',' | ';'))
}

#[cfg(test)]
mod automatic_data_hover_tests {
    use super::automatic_data_hover_at;
    use tyrus_core::analysis::analyze;

    #[test]
    fn exposes_concrete_data_method_signatures() {
        let source = r#"
            interface User { name: string; }
            const user: User = { name: "Ada" };
            user.toJson();
        "#;
        let offset = source.find("toJson").unwrap();
        let (_, markdown) = automatic_data_hover_at(source, &analyze(source), offset).unwrap();
        assert!(markdown.contains("User toJson(options?: { pretty: boolean }): string"));
        assert!(!markdown.contains("Self"));
    }

    #[test]
    fn exposes_static_json_decoding_signature() {
        let source = r#"
            interface User { name: string; }
            User.fromJson("{\"name\":\"Ada\"}");
        "#;
        let offset = source.find("fromJson").unwrap();
        let (_, markdown) = automatic_data_hover_at(source, &analyze(source), offset).unwrap();
        assert!(markdown.contains("User.fromJson(text: string): Result<User, JsonError>"));
    }
}

#[cfg(test)]
mod code_action_range_tests {
    use super::diagnostic_range_intersects_request;
    use tower_lsp::lsp_types::{Position, Range};

    fn range(start: (u32, u32), end: (u32, u32)) -> Range {
        Range::new(Position::new(start.0, start.1), Position::new(end.0, end.1))
    }

    #[test]
    fn filters_diagnostics_outside_requested_range() {
        let diagnostic = range((3, 2), (3, 5));
        assert!(diagnostic_range_intersects_request(
            diagnostic,
            range((3, 0), (3, 3))
        ));
        assert!(!diagnostic_range_intersects_request(
            diagnostic,
            range((0, 0), (2, 9))
        ));
        assert!(diagnostic_range_intersects_request(
            diagnostic,
            range((3, 3), (3, 3))
        ));
    }
}
