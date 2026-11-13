use super::{
    ANALYSIS_WORKER_LIMIT, BACKGROUND_ANALYSIS_WORKER_LIMIT, Backend, CachedAnalysis,
    CancellationOnDrop, CancellationSignal, DocumentState, IndexedExport, InitializationOptions,
    LineIndex, MAX_CACHED_ANALYSES, MAX_CACHED_OCCURRENCES_ENTRIES,
    MAX_CACHED_OCCURRENCES_ENTRY_WEIGHT, MAX_CACHED_OCCURRENCES_WEIGHT, MAX_CACHED_PROJECT_ROOTS,
    MAX_CACHED_WORKSPACE_ANALYSIS_WEIGHT, MAX_CACHED_WORKSPACE_DEFINITIONS_ENTRIES,
    MAX_CACHED_WORKSPACE_DEFINITIONS_WEIGHT, MAX_CACHED_WORKSPACE_ROOTS, WorkspaceFilesCacheEntry,
    acquire_cancellable_permit, analysis_is_cancelled, analysis_retry_allowed,
    automatic_data_call_signature, containing_workspace_root, env_schema_for_document,
    estimated_document_analysis_weight, estimated_occurrence_cache_entry_weight,
    estimated_occurrences_weight, estimated_workspace_definition_weight,
    evict_project_root_cache_entries, evict_workspace_definition_cache_entries,
    evict_workspace_occurrence_cache_entries, evict_workspace_root_cache_entries,
    insert_cached_analysis, insert_cached_workspace_analysis, register_implicit_db_error_fields,
    update_workspace_files_cache_for_root, update_workspace_roots, workspace_content_fingerprint,
    workspace_export_is_visible, workspace_occurrence_cache_depends_on_root,
};
use crate::WorkspaceGeneration;
use crate::{
    MAX_CACHED_CONFIG_ENTRY_WEIGHT, MAX_CACHED_CONFIG_WEIGHT, estimated_config_cache_entry_weight,
    evict_config_cache_entries,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tower_lsp::lsp_types::Url;
use tyrus_core::analysis::DocumentAnalysis;
use tyrus_core::analysis::SymbolKind;
use tyrus_core::ast::{DbModelMetadata, Type};
use tyrus_core::collections::HashMap;

#[test]
fn env_schema_loads_from_the_document_project_outside_the_workspace_root() {
    let root = std::env::temp_dir().join(format!("tyrus-lsp-env-project-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("tyrus.json"),
        r#"{"project":{"name":"env-project","version":"0.1.0"},"security":{"envFile":".env"}}"#,
    )
    .unwrap();
    std::fs::write(root.join(".env"), "PORT=3000\n").unwrap();
    let document = root.join("unknown.tyr");

    let schema = env_schema_for_document(&document).expect("project env schema should load");

    assert!(schema.vars.iter().any(|variable| variable.name == "PORT"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn cancelled_analysis_sentinel_is_distinguishable_from_completed_analysis() {
    assert!(analysis_is_cancelled(&DocumentAnalysis::default()));
    let completed = tyrus_core::analysis::analyze("const value: i32 = 1;");
    assert!(!analysis_is_cancelled(&completed));
}

#[test]
fn cancelled_analysis_retries_only_for_current_snapshots_within_the_retry_limit() {
    let cancelled = DocumentAnalysis::default();
    let completed = tyrus_core::analysis::analyze("const value: i32 = 1;");
    assert!(analysis_retry_allowed(&cancelled, true, 0));
    assert!(!analysis_retry_allowed(&cancelled, false, 0));
    assert!(!analysis_retry_allowed(&cancelled, true, 2));
    assert!(!analysis_retry_allowed(&completed, true, 0));
}

#[test]
fn export_prefix_filter_preserves_order_and_omits_nonmatches() {
    let exports = ["zebra", "prelude", "prefix", "alpha"]
        .into_iter()
        .map(|name| IndexedExport {
            name: name.to_string(),
            is_default: false,
            module_source: None,
            ty: "string".to_string(),
            parameters: Vec::new(),
            kind: SymbolKind::Variable,
            uri: Url::parse("file:///workspace/source.tyr").unwrap(),
            definition: None,
            is_protected: false,
            metadata: Arc::new(crate::IndexedExportMetadata::Value { ty: None }),
        })
        .collect::<Vec<_>>();

    let filtered = Backend::filter_indexed_exports(&exports, Some("pre"));
    assert_eq!(
        filtered
            .iter()
            .map(|export| export.name.as_str())
            .collect::<Vec<_>>(),
        ["prelude", "prefix"]
    );
}

#[test]
fn indexed_export_prefix_filter_matches_linear_results_in_original_order() {
    let exports = ["Alpine", "Beta", "Alpha", "AlphaBeta", "alphabet"]
        .into_iter()
        .map(|name| IndexedExport {
            name: name.to_string(),
            is_default: false,
            module_source: None,
            ty: "string".to_string(),
            parameters: Vec::new(),
            kind: SymbolKind::Variable,
            uri: Url::parse("file:///workspace/source.tyr").unwrap(),
            definition: None,
            is_protected: false,
            metadata: Arc::new(crate::IndexedExportMetadata::Value { ty: None }),
        })
        .collect::<Vec<_>>();
    let name_order = Backend::indexed_export_name_order(&exports);

    for prefix in ["Al", "Alpha", "alphabet", "missing"] {
        let indexed =
            Backend::filter_indexed_exports_with_name_order(&exports, &name_order, Some(prefix));
        let linear = Backend::filter_indexed_exports(&exports, Some(prefix));
        assert_eq!(
            indexed
                .iter()
                .map(|export| &export.name)
                .collect::<Vec<_>>(),
            linear.iter().map(|export| &export.name).collect::<Vec<_>>()
        );
    }
}

#[test]
fn workspace_scan_initialization_options_are_bounded_and_extend_default_exclusions() {
    let options = InitializationOptions::parse(&Some(serde_json::json!({
        "workspaceMaxFiles": 200_000,
        "workspaceMaxDepth": 0,
        "workspaceExcludeDirectories": ["vendor", "../outside", "", "vendor"],
    })));

    assert_eq!(options.workspace_max_files, 100_000);
    assert_eq!(options.workspace_max_depth, 1);
    assert!(options.workspace_exclude_directories.contains("target"));
    assert!(options.workspace_exclude_directories.contains("vendor"));
    assert!(!options.workspace_exclude_directories.contains("../outside"));
}

#[test]
fn abandoned_analysis_requests_cancel_the_shared_worker_token() {
    let cancellation = Arc::new(CancellationSignal::new());
    {
        let _guard = CancellationOnDrop::new(Arc::clone(&cancellation));
    }
    assert!(cancellation.is_cancelled());

    let completed = Arc::new(CancellationSignal::new());
    {
        let mut guard = CancellationOnDrop::new(Arc::clone(&completed));
        guard.disarm();
    }
    assert!(!completed.is_cancelled());
}

#[test]
fn background_analysis_reserves_capacity_for_interactive_requests() {
    let analysis = tokio::sync::Semaphore::new(ANALYSIS_WORKER_LIMIT);
    let background = tokio::sync::Semaphore::new(BACKGROUND_ANALYSIS_WORKER_LIMIT);
    assert_eq!(analysis.available_permits(), 4);
    assert_eq!(background.available_permits(), 3);
    assert_eq!(
        analysis.available_permits() - background.available_permits(),
        1
    );
}

#[test]
fn occurrence_cache_weight_estimate_counts_urls_and_ranges() {
    let occurrences = vec![(
        Url::parse("file:///workspace/main.tyr").unwrap(),
        vec![0..4, 10..14],
    )];
    let weight = estimated_occurrences_weight(&occurrences);
    assert_eq!(
        weight,
        std::mem::size_of::<(Url, Vec<std::ops::Range<usize>>)>()
            + "file:///workspace/main.tyr".len()
            + 2 * std::mem::size_of::<std::ops::Range<usize>>()
    );
    let key = (
        Url::parse("file:///workspace/module.tyr").unwrap(),
        "exportedName".to_string(),
        false,
    );
    assert!(estimated_occurrence_cache_entry_weight(&key, &occurrences) > weight);
    const {
        assert!(
            MAX_CACHED_OCCURRENCES_ENTRY_WEIGHT * MAX_CACHED_OCCURRENCES_ENTRIES
                <= MAX_CACHED_OCCURRENCES_WEIGHT
        );
    }
}

#[test]
fn occurrence_cache_eviction_preserves_unrelated_entries() {
    let mut cache = HashMap::default();
    for index in 0..MAX_CACHED_OCCURRENCES_ENTRIES {
        let key = (
            Url::parse(&format!("file:///workspace/{index:04}.tyr")).unwrap(),
            "exported".to_string(),
            false,
        );
        let value = vec![(
            key.0.clone(),
            std::iter::once(0..std::mem::size_of::<usize>()).collect(),
        )];
        cache.insert(key, value);
    }
    let replacement_key = (
        Url::parse("file:///workspace/zzzz.tyr").unwrap(),
        "exported".to_string(),
        false,
    );
    let replacement = vec![(replacement_key.0.clone(), std::iter::once(0..4).collect())];
    let replacement_weight =
        estimated_occurrence_cache_entry_weight(&replacement_key, &replacement);
    assert_eq!(
        evict_workspace_occurrence_cache_entries(&mut cache, &replacement_key, replacement_weight,),
        1
    );
    cache.insert(replacement_key.clone(), replacement);
    assert_eq!(cache.len(), MAX_CACHED_OCCURRENCES_ENTRIES);
    assert!(
        !cache
            .keys()
            .any(|(uri, _, _)| uri.as_str().ends_with("0000.tyr"))
    );
    assert!(
        cache
            .keys()
            .any(|(uri, _, _)| uri.as_str().ends_with("0001.tyr"))
    );
    let weight_replacement_key = (
        Url::parse("file:///workspace/weight-replacement.tyr").unwrap(),
        "largeExport".to_string(),
        true,
    );
    assert!(
        evict_workspace_occurrence_cache_entries(
            &mut cache,
            &weight_replacement_key,
            MAX_CACHED_OCCURRENCES_WEIGHT,
        ) > 0
    );
    assert!(cache.is_empty());
}

#[test]
fn document_analysis_cache_weight_estimate_counts_semantic_records() {
    let source_len = 256;
    let empty = DocumentAnalysis::default();
    let base_weight = estimated_document_analysis_weight(&empty, source_len);
    let mut populated = DocumentAnalysis::default();
    populated.references.push(tyrus_core::analysis::Reference {
        name: "referencedName".to_string(),
        range: 16..30,
        definition: Some(0..8),
    });
    populated
        .expression_types
        .push(tyrus_core::analysis::ExpressionTypeInfo {
            range: 16..30,
            ty: "Map<string, Result<i32, Error>>".to_string(),
        });
    let populated_weight = estimated_document_analysis_weight(&populated, source_len);
    assert!(populated_weight > base_weight);
}

#[test]
fn workspace_definition_cache_weight_estimate_counts_dependency_urls() {
    let key = (
        Url::parse("file:///workspace/module.tyr").unwrap(),
        "exportedName".to_string(),
    );
    let dependencies = std::collections::HashSet::from([
        Url::parse("file:///workspace/module.tyr").unwrap(),
        Url::parse("file:///workspace/reexport.tyr").unwrap(),
    ]);
    let weight = estimated_workspace_definition_weight(&key, None, &dependencies);
    assert!(weight > std::mem::size_of::<crate::WorkspaceDefinitionCacheEntry>());
    assert!(weight <= MAX_CACHED_WORKSPACE_DEFINITIONS_WEIGHT);
    const {
        assert!(MAX_CACHED_WORKSPACE_DEFINITIONS_ENTRIES > 0);
    }
}

#[test]
fn workspace_definition_cache_eviction_preserves_unrelated_entries() {
    let mut cache = std::collections::HashMap::new();
    for index in 0..MAX_CACHED_WORKSPACE_DEFINITIONS_ENTRIES {
        let key = (
            Url::parse(&format!("file:///workspace/{index:04}.tyr")).unwrap(),
            "exported".to_string(),
        );
        cache.insert(
            key,
            crate::WorkspaceDefinitionCacheEntry {
                generation: WorkspaceGeneration { global: 0, root: 0 },
                location: None,
                dependencies: std::collections::HashSet::new(),
            },
        );
    }
    let replacement_key = (
        Url::parse("file:///workspace/zzzz.tyr").unwrap(),
        "exported".to_string(),
    );
    let replacement_weight = estimated_workspace_definition_weight(
        &replacement_key,
        None,
        &std::collections::HashSet::new(),
    );
    assert_eq!(
        evict_workspace_definition_cache_entries(&mut cache, &replacement_key, replacement_weight,),
        1
    );
    cache.insert(
        replacement_key.clone(),
        crate::WorkspaceDefinitionCacheEntry {
            generation: WorkspaceGeneration { global: 0, root: 0 },
            location: None,
            dependencies: std::collections::HashSet::new(),
        },
    );
    assert_eq!(cache.len(), MAX_CACHED_WORKSPACE_DEFINITIONS_ENTRIES);
    assert!(
        !cache
            .keys()
            .any(|(uri, _)| uri.as_str().ends_with("0000.tyr"))
    );
    assert!(
        cache
            .keys()
            .any(|(uri, _)| uri.as_str().ends_with("0001.tyr"))
    );
    let overweight_key = (
        Url::parse("file:///workspace/weight-replacement.tyr").unwrap(),
        "largeExport".to_string(),
    );
    assert!(
        evict_workspace_definition_cache_entries(
            &mut cache,
            &overweight_key,
            MAX_CACHED_WORKSPACE_DEFINITIONS_WEIGHT,
        ) > 0
    );
    assert!(cache.is_empty());
}

#[test]
fn workspace_root_cache_eviction_preserves_unrelated_roots() {
    let mut cache = std::collections::HashMap::new();
    for index in 0..MAX_CACHED_WORKSPACE_ROOTS {
        cache.insert(PathBuf::from(format!("/workspace/{index}")), index);
    }
    let replacement_root = Path::new("/workspace/new");
    evict_workspace_root_cache_entries(&mut cache, replacement_root);
    cache.insert(replacement_root.to_path_buf(), MAX_CACHED_WORKSPACE_ROOTS);
    assert_eq!(cache.len(), MAX_CACHED_WORKSPACE_ROOTS);
    assert!(!cache.contains_key(Path::new("/workspace/0")));
    assert!(cache.contains_key(Path::new("/workspace/1")));
    assert!(cache.contains_key(replacement_root));
}

#[test]
fn workspace_content_fingerprint_is_stable_and_tracks_source_changes() {
    let original = "import { value } from './value';\n";
    assert_eq!(
        workspace_content_fingerprint(original),
        workspace_content_fingerprint(original)
    );
    assert_ne!(
        workspace_content_fingerprint(original),
        workspace_content_fingerprint("import { other } from './value';\n")
    );
}

#[test]
fn configuration_cache_limits_weight_and_preserves_replacements() {
    let path = Path::new("/workspace/tyrus.json");
    let weight = estimated_config_cache_entry_weight(path, 128);
    assert!(weight > 128);
    assert!(weight <= MAX_CACHED_CONFIG_ENTRY_WEIGHT);
    let mut cache = std::collections::HashMap::new();
    for index in 0..32 {
        cache.insert(
            PathBuf::from(format!("/workspace/{index:02}/tyrus.json")),
            1usize,
        );
    }
    let replacement = PathBuf::from("/workspace/00/tyrus.json");
    assert_eq!(
        evict_config_cache_entries(&mut cache, &replacement, 100, |_, weight| *weight),
        0
    );
    cache.insert(replacement.clone(), 100);
    let new_path = PathBuf::from("/workspace/99/tyrus.json");
    assert_eq!(
        evict_config_cache_entries(&mut cache, &new_path, 1, |_, weight| *weight),
        1
    );
    cache.insert(new_path, 1);
    assert!(!cache.contains_key(&replacement));
    assert!(cache.contains_key(&PathBuf::from("/workspace/01/tyrus.json")));
    assert_eq!(cache.len(), 32);

    let mut weight_cache = std::collections::HashMap::new();
    for index in 0..8 {
        weight_cache.insert(
            PathBuf::from(format!("/workspace/weight-{index}")),
            512 * 1024,
        );
    }
    let replacement = PathBuf::from("/workspace/weight-new");
    assert_eq!(
        evict_config_cache_entries(&mut weight_cache, &replacement, 512 * 1024, |_, weight| {
            *weight
        },),
        1
    );
    weight_cache.insert(replacement.clone(), 512 * 1024);
    assert_eq!(weight_cache.len(), 8);
    assert!(weight_cache.contains_key(&replacement));
    assert_eq!(
        weight_cache.values().sum::<usize>(),
        MAX_CACHED_CONFIG_WEIGHT
    );
}

#[test]
fn project_root_cache_evicts_only_one_deterministic_uri_at_capacity() {
    let mut cache = std::collections::HashMap::new();
    for index in 0..MAX_CACHED_PROJECT_ROOTS {
        let uri = Url::parse(&format!("file:///workspace/{index:04}.tyr")).unwrap();
        cache.insert(uri, (0, PathBuf::from("/workspace")));
    }
    let replacement = Url::parse("file:///workspace/zzzz.tyr").unwrap();
    evict_project_root_cache_entries(&mut cache, &replacement);
    cache.insert(replacement.clone(), (0, PathBuf::from("/workspace")));
    assert_eq!(cache.len(), MAX_CACHED_PROJECT_ROOTS);
    assert!(!cache.contains_key(&Url::parse("file:///workspace/0000.tyr").unwrap()));
    assert!(cache.contains_key(&Url::parse("file:///workspace/0001.tyr").unwrap()));
    assert!(cache.contains_key(&replacement));
}

#[test]
fn occurrence_cache_invalidation_tracks_importers_in_the_changed_root() {
    let workspace = std::env::temp_dir().join("tyrus-lsp-occurrence-cache");
    let app_root = workspace.join("app");
    let defining_uri = Url::from_file_path(workspace.join("shared/module.tyr")).unwrap();
    let occurrence_uri = Url::from_file_path(app_root.join("src/main.tyr")).unwrap();
    let occurrences = vec![(occurrence_uri, std::iter::once(4..10).collect())];
    assert!(workspace_occurrence_cache_depends_on_root(
        &defining_uri,
        &occurrences,
        &app_root
    ));
    assert!(!workspace_occurrence_cache_depends_on_root(
        &defining_uri,
        &occurrences,
        &workspace.join("other")
    ));
}

#[tokio::test]
async fn cancelled_worker_wait_does_not_hold_a_semaphore_queue_slot() {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(0));
    let cancellation = Arc::new(CancellationSignal::new());
    let waiting = tokio::spawn(acquire_cancellable_permit(
        Arc::clone(&semaphore),
        Arc::clone(&cancellation),
    ));
    tokio::task::yield_now().await;
    cancellation.cancel();
    assert!(waiting.await.unwrap().is_none());

    semaphore.add_permits(1);
    let permit = semaphore
        .acquire_owned()
        .await
        .expect("the cancelled waiter must leave the permit available");
    drop(permit);
}

#[test]
fn content_invalidation_preserves_the_workspace_file_inventory() {
    let root = Path::new("/tmp/tyrus-lsp-workspace");
    let file = root.join("src/main.tyr");
    let second_root = Path::new("/tmp/tyrus-lsp-workspace-two");
    let mut cache = std::collections::HashMap::from([
        (
            root.to_path_buf(),
            WorkspaceFilesCacheEntry {
                generation: WorkspaceGeneration { global: 0, root: 4 },
                files: vec![file.clone()],
            },
        ),
        (
            second_root.to_path_buf(),
            WorkspaceFilesCacheEntry {
                generation: WorkspaceGeneration { global: 0, root: 4 },
                files: vec![second_root.join("main.tyr")],
            },
        ),
    ]);

    update_workspace_files_cache_for_root(
        &mut cache,
        root,
        WorkspaceGeneration { global: 0, root: 5 },
        false,
    );

    let entry = cache
        .get(root)
        .expect("content invalidation must retain the file inventory");
    assert_eq!(entry.generation, WorkspaceGeneration { global: 0, root: 5 });
    assert_eq!(entry.files, vec![file]);
    assert_eq!(
        cache.get(second_root).unwrap().generation,
        WorkspaceGeneration { global: 0, root: 4 }
    );

    update_workspace_files_cache_for_root(
        &mut cache,
        root,
        WorkspaceGeneration { global: 0, root: 6 },
        true,
    );
    assert!(!cache.contains_key(root));
    assert!(cache.contains_key(second_root));
}

#[test]
fn workspace_folder_changes_remove_roots_and_add_only_new_roots() {
    let first = Path::new("/workspace/first").to_path_buf();
    let removed = Path::new("/workspace/removed").to_path_buf();
    let added = Path::new("/workspace/added").to_path_buf();
    let mut roots = vec![first.clone(), removed.clone()];
    update_workspace_roots(
        &mut roots,
        &std::collections::HashSet::from([removed]),
        [added.clone(), first.clone()],
    );
    assert_eq!(roots, [first, added]);
}

#[test]
fn nested_workspace_roots_choose_the_most_specific_containing_folder() {
    let roots = [
        Path::new("/workspace").to_path_buf(),
        Path::new("/workspace/packages/app").to_path_buf(),
    ];
    assert_eq!(
        containing_workspace_root(Path::new("/workspace/packages/app/src/main.tyr"), &roots),
        Some(Path::new("/workspace/packages/app").to_path_buf())
    );
    assert_eq!(
        containing_workspace_root(Path::new("/other/main.tyr"), &roots),
        None
    );
}

#[test]
fn analysis_snapshot_cache_respects_its_document_limit() {
    let mut cache = std::collections::HashMap::new();
    for index in 0..=MAX_CACHED_ANALYSES {
        let uri =
            Url::from_file_path(std::env::temp_dir().join(format!("tyrus-lsp-cache-{index}.tyr")))
                .unwrap();
        let source = format!("const value{index}: i32 = {index};");
        cache.insert(
            uri.clone(),
            DocumentState {
                text: source.clone(),
                version: 1,
                analysis: None,
                published_diagnostics: vec![tower_lsp::lsp_types::Diagnostic {
                    message: format!("diagnostic-{index}"),
                    ..Default::default()
                }],
            },
        );
        insert_cached_analysis(
            &mut cache,
            uri.clone(),
            &source,
            Some(1),
            CachedAnalysis {
                context_generation: 1,
                analysis: Arc::new(DocumentAnalysis::default()),
                line_index: Arc::new(LineIndex::new("")),
                import_behaviors: Arc::new(HashMap::default()),
                lint_diagnostics: None,
                semantic_tokens: None,
                call_parameters: std::collections::HashMap::new(),
                inlay_hints: std::collections::HashMap::new(),
            },
        );
        drop(uri);
    }
    assert_eq!(
        cache
            .values()
            .filter(|document| document.analysis.is_some())
            .count(),
        MAX_CACHED_ANALYSES
    );
    assert_eq!(cache.len(), MAX_CACHED_ANALYSES + 1);
    assert!(
        cache
            .values()
            .all(|document| { !document.text.is_empty() && document.version == 1 })
    );
    assert!(cache.values().all(|document| {
        document.published_diagnostics.len() == 1
            && document.published_diagnostics[0]
                .message
                .starts_with("diagnostic-")
    }));
}

#[test]
fn workspace_analysis_cache_respects_count_and_weight_limits() {
    let mut cache = std::collections::HashMap::new();
    let analysis = Arc::new(DocumentAnalysis::default());
    for index in 0..=MAX_CACHED_ANALYSES {
        let uri = Url::from_file_path(
            std::env::temp_dir().join(format!("tyrus-workspace-cache-{index}.tyr")),
        )
        .unwrap();
        insert_cached_workspace_analysis(&mut cache, uri, "x", Arc::clone(&analysis));
    }
    assert_eq!(cache.len(), MAX_CACHED_ANALYSES);

    let oversized_uri =
        Url::from_file_path(std::env::temp_dir().join("tyrus-workspace-cache-oversized.tyr"))
            .unwrap();
    let oversized_source = "x".repeat(MAX_CACHED_WORKSPACE_ANALYSIS_WEIGHT / 8 + 1);
    insert_cached_workspace_analysis(
        &mut cache,
        oversized_uri.clone(),
        &oversized_source,
        analysis,
    );
    assert_eq!(cache.len(), MAX_CACHED_ANALYSES);
    assert!(!cache.contains_key(&oversized_uri));
}

#[test]
fn protected_workspace_exports_stay_within_their_declaring_folder() {
    let workspace = std::env::temp_dir().join("tyrus-workspace-protected-exports");
    let private = workspace.join("src/private");
    let export = IndexedExport {
        name: "internalValue".to_string(),
        is_default: false,
        module_source: None,
        ty: "number".to_string(),
        parameters: Vec::new(),
        kind: tyrus_core::analysis::SymbolKind::Variable,
        uri: Url::from_file_path(private.join("module.tyr")).unwrap(),
        definition: None,
        is_protected: true,
        metadata: Arc::new(crate::IndexedExportMetadata::Value { ty: None }),
    };
    assert!(workspace_export_is_visible(
        &export,
        Some(&private.join("consumer.tyr"))
    ));
    assert!(!workspace_export_is_visible(
        &export,
        Some(&workspace.join("src/public/main.tyr"))
    ));
    assert!(workspace_export_is_visible(&export, None));
}

#[test]
fn imported_orm_models_make_inferred_db_error_fields_available() {
    let model = DbModelMetadata {
        table: "users".to_string(),
        span: tyrus_core::span::Span::new(0, 0),
        timestamps: true,
        snake_case: false,
        fields: Vec::new(),
        relations: Vec::new(),
    };
    let mut imported_models = HashMap::default();
    imported_models.insert("User".to_string(), model);
    let mut interfaces = HashMap::default();

    register_implicit_db_error_fields(&imported_models, &mut interfaces);

    let fields = interfaces
        .get("DbError")
        .expect("ORM imports should expose inferred DbError fields");
    assert!(fields.iter().any(|field| field.name == "code"));
    assert!(fields.iter().any(|field| field.name == "message"));
}

#[test]
fn files_without_orm_imports_do_not_receive_implicit_db_error_fields() {
    let mut interfaces = HashMap::default();

    register_implicit_db_error_fields(&HashMap::default(), &mut interfaces);

    assert!(!interfaces.contains_key("DbError"));
}

#[test]
fn automatic_data_signatures_use_the_concrete_receiver_type() {
    let receiver = Type::Named("User".to_string());
    assert_eq!(
        automatic_data_call_signature(&receiver, "copy").and_then(|signature| signature.detail),
        Some("() => User".to_string())
    );
    assert_eq!(
        automatic_data_call_signature(&receiver, "equals").and_then(|signature| signature.detail),
        Some("(other: User) => boolean".to_string())
    );
    assert_eq!(
        automatic_data_call_signature(&receiver, "toJson").and_then(|signature| signature.detail),
        Some("(options?: { pretty: boolean }) => string".to_string())
    );
}
