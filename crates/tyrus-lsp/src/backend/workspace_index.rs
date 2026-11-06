use super::*;
use crate::backend::*;
use tyrus_core::collections::HashMap;

impl Backend {
    pub(super) async fn imported_definition(
        &self,
        document_uri: &Url,
        symbol: &Symbol,
    ) -> Option<Location> {
        let source = symbol.import_source.as_deref()?;
        if source.starts_with("tyrus:") {
            return None;
        }
        let target = self.resolve_module_source(document_uri, source).await?;
        let target_uri = Url::from_file_path(&target).ok()?;
        let imported = symbol.imported_name.as_deref().unwrap_or(&symbol.name);
        let key = (target_uri.clone(), imported.to_string());
        let root = self
            .project_root_for(&target_uri)
            .await
            .or_else(|| target.parent().map(Path::to_path_buf))?;
        let generation = self.workspace_index_generation_for(&root).await;
        if let Some(cached) = self.workspace_definitions_cache.read().await.get(&key)
            && cached.generation == generation
        {
            trace_lsp_event("workspace_definition_cache_hit");
            return cached.location.clone();
        }
        let indexed_definition = {
            let cache = self.workspace_exports_cache.read().await;
            cache.get(&root).and_then(|entry| {
                if entry.generation != generation {
                    return None;
                }
                let start = entry
                    .export_name_order
                    .partition_point(|index| entry.exports[*index].name.as_str() < imported);
                let export = entry.export_name_order[start..]
                    .iter()
                    .take_while(|index| entry.exports[**index].name == imported)
                    .map(|index| &entry.exports[*index])
                    .find(|export| export.uri == target_uri && export.kind != SymbolKind::Import)?;
                let fingerprint = entry.module_fingerprints.get(&target_uri)?.content;
                Some((export.definition.clone()?, fingerprint))
            })
        };
        if let Some((location, fingerprint)) = indexed_definition {
            let open_source = self
                .documents
                .read()
                .await
                .get(&target_uri)
                .map(|document| document.text.clone());
            if open_source
                .as_deref()
                .is_none_or(|source| workspace_content_fingerprint(source) == fingerprint)
                && self.workspace_index_generation_for(&root).await == generation
            {
                trace_lsp_event("workspace_definition_index_hit");
                return Some(location);
            }
            trace_lsp_event("workspace_definition_index_fingerprint_miss");
        }
        let (location, dependencies) = self
            .export_definition_at(target_uri, imported.to_string())
            .await;
        if self.workspace_index_generation_for(&root).await != generation {
            return None;
        }
        let entry_weight =
            estimated_workspace_definition_weight(&key, location.as_ref(), &dependencies);
        if entry_weight <= MAX_CACHED_WORKSPACE_DEFINITIONS_WEIGHT {
            let mut cache = self.workspace_definitions_cache.write().await;
            evict_workspace_definition_cache_entries(&mut cache, &key, entry_weight);
            cache.insert(
                key,
                WorkspaceDefinitionCacheEntry {
                    generation,
                    location: location.clone(),
                    dependencies,
                },
            );
        }
        location
    }

    async fn export_definition_at(
        &self,
        uri: Url,
        exported_name: String,
    ) -> (Option<Location>, HashSet<Url>) {
        let mut pending = std::collections::VecDeque::from([(uri, exported_name)]);
        let mut visited = HashSet::new();
        let mut dependencies = HashSet::new();
        while let Some((uri, name)) = pending.pop_front() {
            if visited.len() >= 128 || !visited.insert((uri.clone(), name.clone())) {
                continue;
            }
            dependencies.insert(uri.clone());
            let Some(source) = self.source(&uri).await else {
                continue;
            };
            let analysis = self.workspace_analysis_for(&uri, &source).await;
            let Some(program) = analysis.program.as_ref() else {
                continue;
            };
            let wildcard_targets = analysis
                .wildcard_export_indexes()
                .iter()
                .filter_map(|index| match &program.exports[*index] {
                    tyrus_core::ast::ExportDecl::All { source, .. } => Some(source.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            for export_index in analysis.named_export_indexes(&name) {
                let export = &program.exports[*export_index];
                match export {
                    tyrus_core::ast::ExportDecl::Named { local, .. } => {
                        let Some(symbol) = analysis.symbol_named(local) else {
                            continue;
                        };
                        if symbol.kind == SymbolKind::Import {
                            let Some(import_source) = symbol.import_source.as_deref() else {
                                continue;
                            };
                            let Some(path) = self.resolve_module_source(&uri, import_source).await
                            else {
                                continue;
                            };
                            let Ok(target_uri) = Url::from_file_path(path) else {
                                continue;
                            };
                            pending.push_front((
                                target_uri,
                                symbol
                                    .imported_name
                                    .as_deref()
                                    .unwrap_or(&symbol.name)
                                    .to_string(),
                            ));
                        } else {
                            return (
                                Some(Location {
                                    uri: uri.clone(),
                                    range: byte_range_to_lsp(&source, symbol.declaration.clone()),
                                }),
                                dependencies,
                            );
                        }
                    }
                    tyrus_core::ast::ExportDecl::Re {
                        imported,
                        source: import_source,
                        ..
                    } => {
                        if let Some(target) = self
                            .resolve_module_source(&uri, import_source)
                            .await
                            .and_then(|path| Url::from_file_path(path).ok())
                        {
                            pending.push_front((target, imported.clone()));
                        }
                    }
                    tyrus_core::ast::ExportDecl::All { .. } => {
                        unreachable!("wildcard exports are stored in a separate document index")
                    }
                }
            }
            if let Some(symbol) = analysis.exported_symbol_named(&name) {
                return (
                    Some(Location {
                        uri: uri.clone(),
                        range: byte_range_to_lsp(&source, symbol.declaration.clone()),
                    }),
                    dependencies,
                );
            }
            for import_source in wildcard_targets.into_iter().rev() {
                if let Some(target) = self
                    .resolve_module_source(&uri, &import_source)
                    .await
                    .and_then(|target| Url::from_file_path(target).ok())
                {
                    pending.push_front((target, name.clone()));
                }
            }
        }
        (None, dependencies)
    }

    pub(super) async fn project_root_for(&self, uri: &Url) -> Option<PathBuf> {
        let roots_generation = self
            .workspace_roots_generation
            .load(std::sync::atomic::Ordering::Acquire);
        if let Some((cached_generation, root)) = self.project_roots.read().await.get(uri)
            && *cached_generation == roots_generation
        {
            return Some(root.clone());
        }
        if let Ok(path) = uri.to_file_path() {
            let (containing_root, has_workspace_roots) = {
                let roots = self.workspace_roots.read().await;
                (containing_workspace_root(&path, &roots), !roots.is_empty())
            };
            let workspace_root = containing_root.or(if has_workspace_roots {
                None
            } else {
                self.workspace_root.read().await.clone()
            });
            let root = nearest_project_root(&path)
                .or(workspace_root)
                .or_else(|| path.parent().map(Path::to_path_buf));
            if self
                .workspace_roots_generation
                .load(std::sync::atomic::Ordering::Acquire)
                != roots_generation
            {
                return None;
            }
            if let Some(root) = &root {
                let mut roots = self.project_roots.write().await;
                evict_project_root_cache_entries(&mut roots, uri);
                if self
                    .workspace_roots_generation
                    .load(std::sync::atomic::Ordering::Acquire)
                    == roots_generation
                {
                    roots.insert(uri.clone(), (roots_generation, root.clone()));
                }
            }
            return root;
        }
        self.workspace_root.read().await.clone()
    }

    pub(super) async fn resolve_symbol_target(
        &self,
        uri: &Url,
        symbol: &Symbol,
    ) -> Option<(Url, String)> {
        if symbol.kind == SymbolKind::Import {
            let source = symbol.import_source.as_deref()?;
            if source.starts_with("tyrus:") {
                return None;
            }
            let target = self.resolve_module_source(uri, source).await?;
            let target_uri = Url::from_file_path(target).ok()?;
            let exported = symbol
                .imported_name
                .clone()
                .unwrap_or_else(|| symbol.name.clone());
            return Some((target_uri, exported));
        }
        if symbol.exported {
            return Some((uri.clone(), symbol.name.clone()));
        }
        None
    }

    pub(super) async fn resolve_module_source(
        &self,
        importer_uri: &Url,
        source: &str,
    ) -> Option<PathBuf> {
        if let Some(root) = self.project_root_for(importer_uri).await {
            let generation = self.workspace_index_generation_for(&root).await;
            let cached = self
                .workspace_exports_cache
                .read()
                .await
                .get(&root)
                .filter(|entry| entry.generation == generation)
                .and_then(|entry| {
                    Some((
                        entry
                            .resolved_imports
                            .get(&(importer_uri.clone(), source.to_string()))?
                            .clone(),
                        entry.module_fingerprints.get(importer_uri)?.content,
                    ))
                });
            if let Some((target, fingerprint)) = cached {
                let open_source = self
                    .documents
                    .read()
                    .await
                    .get(importer_uri)
                    .map(|document| document.text.clone());
                if open_source
                    .as_deref()
                    .is_none_or(|source| workspace_content_fingerprint(source) == fingerprint)
                {
                    trace_lsp_event("workspace_import_graph_cache_hit");
                    return target.to_file_path().ok();
                }
                trace_lsp_event("workspace_import_graph_fingerprint_miss");
            }
        }
        if source.starts_with('#') {
            let root = self.project_root_for(importer_uri).await?;
            return self
                .import_map_for_root(&root)
                .await
                .ok()?
                .resolve(&root, source)
                .ok()
                .flatten();
        }
        let importer_path = importer_uri.to_file_path().ok()?;
        resolve_import_path(&importer_path, source)
    }

    pub(super) async fn cross_file_occurrences(
        &self,
        defining_uri: &Url,
        exported_name: &str,
        rename: bool,
    ) -> Vec<(Url, Vec<std::ops::Range<usize>>)> {
        let workspace_index_started = std::time::Instant::now();
        let Some(root) = self.project_root_for(defining_uri).await else {
            return Vec::new();
        };
        let generation = self.workspace_index_generation_for(&root).await;
        let cache_key = (defining_uri.clone(), exported_name.to_string(), rename);
        if let Some(occurrences) = self
            .workspace_occurrences_cache
            .read()
            .await
            .get(&cache_key)
            .cloned()
            && self.workspace_index_generation_for(&root).await == generation
        {
            trace_lsp_stage("workspace_occurrences_cache_hit", workspace_index_started);
            return occurrences;
        }
        let paths = self.workspace_files(&root, generation).await;
        let documents = {
            let documents = self.documents.read().await;
            paths
                .iter()
                .filter_map(|path| {
                    let uri = Url::from_file_path(path).ok()?;
                    documents
                        .get(&uri)
                        .map(|document| (uri, document.text.clone()))
                })
                .collect::<HashMap<_, _>>()
        };
        let Ok(permit) = Arc::clone(&self.workspace_index_semaphore)
            .acquire_owned()
            .await
        else {
            return Vec::new();
        };
        let files = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            paths
                .into_iter()
                .filter_map(|path| {
                    let uri = Url::from_file_path(&path).ok()?;
                    let source = read_workspace_source(&path, &uri, &documents)?;
                    Some((uri, source))
                })
                .collect::<Vec<_>>()
        })
        .await
        .unwrap_or_default();
        let mut analyses = HashMap::default();
        let mut pending = tokio::task::JoinSet::new();
        let mut remaining_files = files.iter();
        for _ in 0..WORKSPACE_INDEX_CONCURRENCY {
            let Some((file_uri, source)) = remaining_files.next() else {
                break;
            };
            let backend = self.clone();
            let file_uri = file_uri.clone();
            let source = source.clone();
            pending.spawn(async move {
                let analysis = backend.workspace_analysis_for(&file_uri, &source).await;
                (file_uri, analysis)
            });
        }
        while let Some(result) = pending.join_next().await {
            if self.workspace_index_generation_for(&root).await != generation {
                pending.abort_all();
                return Vec::new();
            }
            if let Ok((file_uri, analysis)) = result {
                analyses.insert(file_uri, analysis);
            }
            if let Some((file_uri, source)) = remaining_files.next() {
                let backend = self.clone();
                let file_uri = file_uri.clone();
                let source = source.clone();
                pending.spawn(async move {
                    let analysis = backend.workspace_analysis_for(&file_uri, &source).await;
                    (file_uri, analysis)
                });
            }
        }
        let mut resolved_aliases = HashMap::default();
        let has_path_aliases = analyses.values().any(|analysis| {
            analysis.symbols.iter().any(|symbol| {
                symbol.kind == SymbolKind::Import
                    && symbol
                        .import_source
                        .as_deref()
                        .is_some_and(|source| source.starts_with('#'))
            })
        });
        if has_path_aliases && let Ok(import_map) = self.import_map_for_root(&root).await {
            for analysis in analyses.values() {
                for source in analysis
                    .symbols
                    .iter()
                    .filter(|symbol| symbol.kind == SymbolKind::Import)
                    .filter_map(|symbol| symbol.import_source.as_deref())
                    .filter(|source| source.starts_with('#'))
                {
                    if resolved_aliases.contains_key(source) {
                        continue;
                    }
                    if let Ok(Some(path)) = import_map.resolve(&root, source)
                        && let Ok(uri) = Url::from_file_path(path)
                    {
                        resolved_aliases.insert(source.to_string(), uri);
                    }
                }
            }
        }
        let indexed_import_targets = self
            .workspace_exports_cache
            .read()
            .await
            .get(&root)
            .filter(|entry| entry.generation == generation)
            .map(|entry| {
                entry
                    .resolved_imports
                    .iter()
                    .map(|(key, target)| (key.clone(), target.clone()))
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();
        let occurrences = cross_file_occurrences_with_resolved_aliases(
            &files,
            &analyses,
            defining_uri,
            exported_name,
            rename,
            &resolved_aliases,
            &indexed_import_targets,
        );
        if self.workspace_index_generation_for(&root).await != generation {
            return Vec::new();
        }
        let mut cache = self.workspace_occurrences_cache.write().await;
        if self.workspace_index_generation_for(&root).await != generation {
            return Vec::new();
        }
        let occurrence_weight = estimated_occurrence_cache_entry_weight(&cache_key, &occurrences);
        if occurrence_weight <= MAX_CACHED_OCCURRENCES_ENTRY_WEIGHT {
            evict_workspace_occurrence_cache_entries(&mut cache, &cache_key, occurrence_weight);
            cache.insert(cache_key, occurrences.clone());
        }
        trace_lsp_stage("workspace_occurrences_index", workspace_index_started);
        occurrences
    }

    pub(super) async fn workspace_analysis_for(
        &self,
        uri: &Url,
        source: &str,
    ) -> Arc<DocumentAnalysis> {
        if let Some(analysis) = self
            .workspace_analysis_cache
            .read()
            .await
            .get(uri)
            .filter(|(cached_source, _)| cached_source == source)
            .map(|(_, analysis)| Arc::clone(analysis))
        {
            trace_lsp_event("workspace_analysis_cache_hit");
            return analysis;
        }
        trace_lsp_event("workspace_analysis_computation");
        let Ok(permit) = Arc::clone(&self.workspace_index_semaphore)
            .acquire_owned()
            .await
        else {
            return Arc::new(DocumentAnalysis::default());
        };
        let source_for_analysis = source.to_string();
        let analysis = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            analyze(&source_for_analysis)
        })
        .await
        .unwrap_or_default();
        let analysis = Arc::new(analysis);
        let mut cache = self.workspace_analysis_cache.write().await;
        insert_cached_workspace_analysis(&mut cache, uri.clone(), source, Arc::clone(&analysis));
        analysis
    }
}
