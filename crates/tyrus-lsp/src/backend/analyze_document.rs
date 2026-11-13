use super::*;

impl super::Backend {
    pub(super) async fn analyze_document(
        &self,
        uri: &Url,
        source: &str,
        version: Option<i32>,
        context_generation: u64,
        cancellation: Arc<CancellationSignal>,
    ) -> (
        DocumentAnalysis,
        HashMap<String, tyrus_core::ast::BehaviorInterfaceInfo>,
    ) {
        if !document_snapshot::analysis_snapshot_is_current(
            self,
            uri,
            source,
            version,
            context_generation,
        )
        .await
            || cancellation.is_cancelled()
        {
            return (DocumentAnalysis::default(), HashMap::default());
        }
        let worker_queue_started = std::time::Instant::now();
        let Some(permit) = acquire_cancellable_permit(
            Arc::clone(&self.analysis_semaphore),
            Arc::clone(&cancellation),
        )
        .await
        else {
            return (DocumentAnalysis::default(), HashMap::default());
        };
        trace_lsp_stage("analysis_worker_queue", worker_queue_started);
        if !document_snapshot::analysis_snapshot_is_current(
            self,
            uri,
            source,
            version,
            context_generation,
        )
        .await
            || cancellation.is_cancelled()
        {
            drop(permit);
            return (DocumentAnalysis::default(), HashMap::default());
        }
        let backend = self.clone();
        let uri = uri.clone();
        let source = source.to_string();
        let runtime = tokio::runtime::Handle::current();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            runtime.block_on(backend.analyze_document_inner(
                &uri,
                &source,
                version,
                context_generation,
                Arc::clone(&cancellation.cancelled),
            ))
        })
        .await
        .unwrap_or_else(|_| (DocumentAnalysis::default(), HashMap::default()))
    }

    async fn analyze_document_inner(
        &self,
        uri: &Url,
        source: &str,
        version: Option<i32>,
        context_generation: u64,
        cancellation: Arc<std::sync::atomic::AtomicBool>,
    ) -> (
        DocumentAnalysis,
        HashMap<String, tyrus_core::ast::BehaviorInterfaceInfo>,
    ) {
        let cached_env_schema = self.env_schema().await;
        let env_schema = uri
            .to_file_path()
            .ok()
            .and_then(|file| {
                let project_root = tyrus_core::project::find_project_root(&file)?;
                cached_env_schema
                    .as_ref()
                    .filter(|schema| schema.file_path.starts_with(&project_root))
                    .cloned()
                    .or_else(|| env_schema_for_document(&file))
            })
            .or(cached_env_schema);
        let entry_point = is_entry_point_uri(uri);
        let db_backend = self.db_backend_for_uri(uri).await.ok();
        let (db_schema, db_schema_error) = match self.db_schema_for_uri(uri).await {
            Ok(snapshot) => (Some(snapshot), None),
            Err(error) => (None, Some(error)),
        };
        if !document_snapshot::analysis_snapshot_is_current(
            self,
            uri,
            source,
            version,
            context_generation,
        )
        .await
        {
            return (DocumentAnalysis::default(), HashMap::default());
        }
        let (parse_result, parse_metrics) =
            tyrus_core::analysis::parse_for_analysis_cancellable_with_metrics(
                source,
                Arc::clone(&cancellation),
            );
        trace_lsp_duration("lexing", parse_metrics.lexing);
        trace_lsp_duration("parsing", parse_metrics.parsing);
        trace_lsp_count("document_lexed_tokens", parse_metrics.token_count);
        let parsed = match parse_result {
            Ok(parsed) => Some(parsed),
            Err(tyrus_core::analysis::ParseForAnalysis::Cancelled) => {
                return (DocumentAnalysis::default(), HashMap::default());
            }
            Err(_) => None,
        };
        if !document_snapshot::analysis_snapshot_is_current(
            self,
            uri,
            source,
            version,
            context_generation,
        )
        .await
        {
            return (DocumentAnalysis::default(), HashMap::default());
        }
        let initial_context = AnalyzeContext {
            entry_point,
            env_schema: env_schema.as_ref(),
            db_schema: db_schema.as_deref(),
            db_schema_error: db_schema_error.as_deref(),
            db_backend: db_backend.as_ref(),
            ..AnalyzeContext::default()
        };
        let mut analysis = if let Some((tokens, program)) = parsed.as_ref() {
            DocumentAnalysis {
                symbols: imported_symbols_from_program(tokens, program),
                ..DocumentAnalysis::default()
            }
        } else {
            match tyrus_core::analysis::analyze_with_context_cancellable(
                source,
                initial_context,
                Arc::clone(&cancellation),
            ) {
                Ok(analysis) => analysis,
                Err(_) => return (DocumentAnalysis::default(), HashMap::default()),
            }
        };
        let import_resolution_started = std::time::Instant::now();
        let mut dependency_updates = HashMap::default();
        let mut module_stack = HashSet::from([uri.clone()]);
        let imported_context = {
            let mut import_state = crate::project_analysis::ImportResolutionState {
                db_backend: db_backend.as_ref(),
                dependency_updates: &mut dependency_updates,
                module_stack: &mut module_stack,
                cancellation: &cancellation,
            };
            self.add_import_diagnostics(uri, source, &mut analysis, 0, &mut import_state)
                .await
        };
        let crate::project_analysis::ImportedModuleContext {
            types: mut import_types,
            interfaces: mut import_interfaces,
            behaviors: import_behaviors,
            type_aliases: import_type_aliases,
            enums: import_enums,
            throws: import_throws,
            db_models: import_db_models,
            validators: import_validators,
            targets: import_targets,
            reachable_interfaces,
        } = imported_context;
        trace_lsp_stage("import_resolution", import_resolution_started);
        if cancellation.load(std::sync::atomic::Ordering::Acquire) {
            return (DocumentAnalysis::default(), HashMap::default());
        }
        if !document_snapshot::analysis_snapshot_is_current(
            self,
            uri,
            source,
            version,
            context_generation,
        )
        .await
        {
            return (DocumentAnalysis::default(), HashMap::default());
        }
        let import_diagnostics = analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| matches!(diagnostic.code.as_str(), "E0432" | "E0583" | "E3308"))
            .cloned()
            .collect::<Vec<_>>();
        register_implicit_db_error_fields(&import_db_models, &mut import_interfaces);
        let mut reachable_interfaces = reachable_interfaces;
        let has_automatic_model_timestamps = parsed.as_ref().is_some_and(|(_, program)| {
            program.items.iter().any(|item| {
                matches!(
                    item,
                    tyrus_core::ast::Item::InterfaceDecl {
                        db_model: Some(model),
                        ..
                    } if model.timestamps
                )
            })
        });
        if has_automatic_model_timestamps {
            import_types
                .entry("Instant".to_string())
                .or_insert_with(|| tyrus_core::ast::Type::Named("Instant".to_string()));
            import_interfaces
                .entry("Instant".to_string())
                .or_insert_with(tyrus_core::builtin_interfaces::instant_fields);
        }
        if import_db_models.values().any(|model| model.timestamps) {
            reachable_interfaces
                .entry("Instant".to_string())
                .or_insert_with(tyrus_core::builtin_interfaces::instant_fields);
        }
        let contextual_program = uri.to_file_path().ok().and_then(|path| {
            let (tokens, mut program) = parsed.clone()?;
            let interfaces =
                tyrus_core::module_loader::contextualize_http_controller(&path, &mut program)
                    .ok()??;
            for (name, fields) in interfaces {
                import_types
                    .entry(name.clone())
                    .or_insert_with(|| tyrus_core::ast::Type::Named(name.clone()));
                import_interfaces.entry(name).or_insert(fields);
            }
            if let Some((ty, fields, _)) =
                tyrus_core::module_loader::native_export_info("tyrus:http", "HttpReply")
            {
                import_types.entry("HttpReply".to_string()).or_insert(ty);
                if let Some(fields) = fields {
                    import_interfaces
                        .entry("HttpReply".to_string())
                        .or_insert(fields);
                }
            }
            Some((tokens, program))
        });
        let has_import_context = !import_types.is_empty()
            || !import_interfaces.is_empty()
            || !import_behaviors.is_empty()
            || !import_type_aliases.is_empty()
            || !import_enums.is_empty()
            || !import_throws.is_empty()
            || !import_db_models.is_empty()
            || !import_validators.is_empty()
            || !import_targets.is_empty()
            || contextual_program.is_some();
        if has_import_context {
            let context = AnalyzeContext {
                import_types: Some(&import_types),
                import_targets: Some(&import_targets),
                entry_point,
                import_interfaces: Some(&import_interfaces),
                import_behaviors: Some(&import_behaviors),
                import_type_aliases: Some(&import_type_aliases),
                import_enums: Some(&import_enums),
                import_throws: Some(&import_throws),
                import_db_models: Some(&import_db_models),
                import_validators: Some(&import_validators),
                reachable_interfaces: Some(&reachable_interfaces),
                env_schema: env_schema.as_ref(),
                db_schema: db_schema.as_deref(),
                db_schema_error: db_schema_error.as_deref(),
                db_backend: db_backend.as_ref(),
            };
            let contextual_analysis_started = std::time::Instant::now();
            analysis =
                if let Some((tokens, program)) = contextual_program.or_else(|| parsed.clone()) {
                    tyrus_core::analysis::analyze_parsed(source, tokens, program, context)
                } else {
                    match tyrus_core::analysis::analyze_with_context_cancellable(
                        source,
                        context,
                        Arc::clone(&cancellation),
                    ) {
                        Ok(analysis) => analysis,
                        Err(_) => return (DocumentAnalysis::default(), HashMap::default()),
                    }
                };
            trace_lsp_stage("contextual_semantic_analysis", contextual_analysis_started);
        } else if let Some((tokens, program)) = parsed {
            analysis =
                tyrus_core::analysis::analyze_parsed(source, tokens, program, initial_context);
        }
        if !document_snapshot::analysis_snapshot_is_current(
            self,
            uri,
            source,
            version,
            context_generation,
        )
        .await
        {
            return (DocumentAnalysis::default(), HashMap::default());
        }
        analysis.diagnostics.extend(import_diagnostics);
        self.add_object_interface_match_hints(uri, source, &mut analysis)
            .await;
        if !cancellation.load(std::sync::atomic::Ordering::Acquire)
            && document_snapshot::analysis_snapshot_is_current(
                self,
                uri,
                source,
                version,
                context_generation,
            )
            .await
        {
            self.replace_import_dependency_graph(dependency_updates)
                .await;
        }
        (analysis, import_behaviors)
    }
}
