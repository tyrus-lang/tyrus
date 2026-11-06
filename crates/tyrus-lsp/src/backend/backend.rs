use super::*;
use tyrus_core::ast::ExprKind;
use tyrus_core::ast::StmtKind;
use tyrus_core::collections::HashMap;
use tyrus_core::doc_comments::DocComment;
use tyrus_core::span::Span;

#[path = "analyze_document.rs"]
mod analyze_document;
#[path = "imported_symbols.rs"]
mod imported_symbols;
use imported_symbols::imported_symbols_from_program;
#[path = "document_snapshot.rs"]
mod document_snapshot;

#[path = "backend_helpers.rs"]
mod backend_helpers;
pub(super) use backend_helpers::*;
#[cfg(test)]
#[path = "backend_tests.rs"]
mod tests;

const MAX_CACHED_ANALYSES: usize = 256;
const MAX_CACHED_ANALYSIS_WEIGHT: usize = 128 * 1024 * 1024;
const MAX_CACHED_WORKSPACE_ANALYSIS_WEIGHT: usize = 64 * 1024 * 1024;
pub(super) const MAX_CACHED_OCCURRENCES_WEIGHT: usize = 32 * 1024 * 1024;
pub(super) const MAX_CACHED_OCCURRENCES_ENTRY_WEIGHT: usize = 64 * 1024;
pub(super) const MAX_CACHED_OCCURRENCES_ENTRIES: usize = 512;
pub(super) const MAX_CACHED_WORKSPACE_ROOTS: usize = 8;
pub(super) const MAX_CACHED_WORKSPACE_DEFINITIONS_WEIGHT: usize = 8 * 1024 * 1024;
pub(super) const MAX_CACHED_WORKSPACE_DEFINITIONS_ENTRIES: usize = 512;
pub(super) const MAX_CACHED_PROJECT_ROOTS: usize = 512;
const WORKSPACE_PROGRESS_MIN_FILES: usize = 256;
pub(super) const MAX_CACHED_WORKSPACE_FILE_PATHS: usize = 100_000;
pub(super) const MAX_CACHED_WORKSPACE_EXPORT_WEIGHT: usize = 64 * 1024 * 1024;
pub(super) const MAX_CACHED_WORKSPACE_EXPORTS_WEIGHT: usize = 128 * 1024 * 1024;
pub(super) const WORKSPACE_INDEX_CONCURRENCY: usize = 2;

pub(super) fn evict_workspace_root_cache_entries<T>(
    cache: &mut std::collections::HashMap<PathBuf, T>,
    replacement_root: &Path,
) {
    while cache.len() >= MAX_CACHED_WORKSPACE_ROOTS && !cache.contains_key(replacement_root) {
        let Some(victim) = cache
            .keys()
            .filter(|root| root.as_path() != replacement_root)
            .min()
            .cloned()
        else {
            break;
        };
        cache.remove(&victim);
    }
}

pub(super) fn estimated_occurrences_weight(
    occurrences: &[(Url, Vec<std::ops::Range<usize>>)],
) -> usize {
    occurrences.iter().fold(0usize, |total, (uri, ranges)| {
        total
            .saturating_add(std::mem::size_of::<(Url, Vec<std::ops::Range<usize>>)>())
            .saturating_add(uri.as_str().len())
            .saturating_add(
                ranges
                    .len()
                    .saturating_mul(std::mem::size_of::<std::ops::Range<usize>>()),
            )
    })
}

pub(super) fn estimated_occurrence_cache_entry_weight(
    key: &(Url, String, bool),
    occurrences: &[(Url, Vec<std::ops::Range<usize>>)],
) -> usize {
    std::mem::size_of::<((Url, String, bool), Vec<(Url, Vec<std::ops::Range<usize>>)>)>()
        .saturating_add(key.0.as_str().len())
        .saturating_add(key.1.len())
        .saturating_add(estimated_occurrences_weight(occurrences))
}

pub(super) fn evict_workspace_occurrence_cache_entries<S: std::hash::BuildHasher>(
    cache: &mut WorkspaceOccurrenceCache<S>,
    replacement_key: &(Url, String, bool),
    replacement_weight: usize,
) -> usize {
    let existing_weight = cache.get(replacement_key).map_or(0, |occurrences| {
        estimated_occurrence_cache_entry_weight(replacement_key, occurrences)
    });
    let mut cached_weight = cache.iter().fold(0usize, |total, (key, occurrences)| {
        total.saturating_add(estimated_occurrence_cache_entry_weight(key, occurrences))
    });
    cached_weight = cached_weight.saturating_sub(existing_weight);
    let mut projected_count = cache.len() + usize::from(!cache.contains_key(replacement_key));
    let mut evicted = 0;
    while projected_count > MAX_CACHED_OCCURRENCES_ENTRIES
        || cached_weight.saturating_add(replacement_weight) > MAX_CACHED_OCCURRENCES_WEIGHT
    {
        let Some(victim_key) = cache
            .keys()
            .filter(|key| *key != replacement_key)
            .min_by(|left, right| {
                left.0
                    .as_str()
                    .cmp(right.0.as_str())
                    .then_with(|| left.1.cmp(&right.1))
                    .then_with(|| left.2.cmp(&right.2))
            })
            .cloned()
        else {
            break;
        };
        if let Some(occurrences) = cache.remove(&victim_key) {
            cached_weight = cached_weight.saturating_sub(estimated_occurrence_cache_entry_weight(
                &victim_key,
                &occurrences,
            ));
            projected_count = projected_count.saturating_sub(1);
            evicted += 1;
        }
    }
    evicted
}

pub(super) fn estimated_workspace_definition_weight(
    key: &(Url, String),
    location: Option<&Location>,
    dependencies: &std::collections::HashSet<Url>,
) -> usize {
    std::mem::size_of::<WorkspaceDefinitionCacheEntry>()
        .saturating_add(key.0.as_str().len())
        .saturating_add(key.1.len())
        .saturating_add(location.map_or(0, |location| location.uri.as_str().len()))
        .saturating_add(
            dependencies
                .iter()
                .map(|uri| uri.as_str().len())
                .sum::<usize>(),
        )
        .saturating_add(
            dependencies
                .len()
                .saturating_mul(std::mem::size_of::<Url>()),
        )
}

pub(super) fn evict_workspace_definition_cache_entries<S: std::hash::BuildHasher>(
    cache: &mut std::collections::HashMap<(Url, String), WorkspaceDefinitionCacheEntry, S>,
    replacement_key: &(Url, String),
    replacement_weight: usize,
) -> usize {
    let existing_weight = cache.get(replacement_key).map_or(0, |entry| {
        estimated_workspace_definition_weight(
            replacement_key,
            entry.location.as_ref(),
            &entry.dependencies,
        )
    });
    let mut cached_weight = cache.iter().fold(0usize, |total, (key, entry)| {
        total.saturating_add(estimated_workspace_definition_weight(
            key,
            entry.location.as_ref(),
            &entry.dependencies,
        ))
    });
    cached_weight = cached_weight.saturating_sub(existing_weight);
    let mut projected_count = cache.len() + usize::from(!cache.contains_key(replacement_key));
    let mut evicted = 0;
    while projected_count > MAX_CACHED_WORKSPACE_DEFINITIONS_ENTRIES
        || cached_weight.saturating_add(replacement_weight)
            > MAX_CACHED_WORKSPACE_DEFINITIONS_WEIGHT
    {
        let Some(victim_key) = cache
            .keys()
            .filter(|key| *key != replacement_key)
            .min_by(|left, right| {
                left.0
                    .as_str()
                    .cmp(right.0.as_str())
                    .then_with(|| left.1.cmp(&right.1))
            })
            .cloned()
        else {
            break;
        };
        if let Some(entry) = cache.remove(&victim_key) {
            cached_weight = cached_weight.saturating_sub(estimated_workspace_definition_weight(
                &victim_key,
                entry.location.as_ref(),
                &entry.dependencies,
            ));
            projected_count = projected_count.saturating_sub(1);
            evicted += 1;
        }
    }
    evicted
}

pub(super) fn evict_project_root_cache_entries<S: std::hash::BuildHasher>(
    cache: &mut std::collections::HashMap<Url, (u64, PathBuf), S>,
    replacement_uri: &Url,
) {
    while cache.len() >= MAX_CACHED_PROJECT_ROOTS && !cache.contains_key(replacement_uri) {
        let Some(victim) = cache
            .keys()
            .filter(|uri| *uri != replacement_uri)
            .min_by(|left, right| left.as_str().cmp(right.as_str()))
            .cloned()
        else {
            break;
        };
        cache.remove(&victim);
    }
}

fn workspace_occurrence_cache_depends_on_root(
    defining_uri: &Url,
    occurrences: &[(Url, Vec<std::ops::Range<usize>>)],
    root: &Path,
) -> bool {
    defining_uri
        .to_file_path()
        .is_ok_and(|path| path.starts_with(root))
        || occurrences
            .iter()
            .any(|(uri, _)| uri.to_file_path().is_ok_and(|path| path.starts_with(root)))
}

pub(super) fn read_workspace_source(
    path: &Path,
    uri: &Url,
    open_documents: &HashMap<Url, String>,
) -> Option<String> {
    if let Some(source) = open_documents.get(uri) {
        return Some(source.clone());
    }
    let started = std::time::Instant::now();
    let source = fs::read_to_string(path).ok();
    trace_lsp_stage("workspace_source_filesystem_read", started);
    source
}

pub(super) fn estimated_workspace_export_weight(
    exports: &[IndexedExport],
    interfaces: &[ObjectInterfaceCandidate],
    resolved_imports: &std::collections::HashMap<(Url, String), Url>,
    module_fingerprints: &std::collections::HashMap<Url, WorkspaceModuleFingerprint>,
) -> usize {
    let export_weight = exports.iter().fold(0usize, |total, export| {
        total
            .saturating_add(std::mem::size_of::<IndexedExport>())
            .saturating_add(export.name.len())
            .saturating_add(export.ty.len())
            .saturating_add(export.uri.as_str().len())
            .saturating_add(
                export
                    .definition
                    .as_ref()
                    .map_or(0, |location| location.uri.as_str().len()),
            )
            .saturating_add(export.parameters.iter().map(String::len).sum::<usize>())
            .saturating_add(estimated_indexed_export_metadata_weight(
                export.metadata.as_ref(),
            ))
    });
    interfaces
        .iter()
        .fold(export_weight, |total, interface| {
            total
                .saturating_add(std::mem::size_of::<ObjectInterfaceCandidate>())
                .saturating_add(interface.name.len())
                .saturating_add(interface.uri.as_str().len())
                .saturating_add(
                    interface
                        .fields
                        .iter()
                        .map(|field| std::mem::size_of::<Field>() + format!("{field:?}").len())
                        .sum::<usize>(),
                )
        })
        .saturating_add(exports.len().saturating_mul(std::mem::size_of::<usize>()))
        .saturating_add(resolved_imports.iter().fold(
            0usize,
            |total, ((importer, source), target)| {
                total
                    .saturating_add(std::mem::size_of::<((Url, String), Url)>())
                    .saturating_add(importer.as_str().len())
                    .saturating_add(source.len())
                    .saturating_add(target.as_str().len())
            },
        ))
        .saturating_add(
            module_fingerprints
                .iter()
                .fold(0usize, |total, (uri, fingerprint)| {
                    total
                        .saturating_add(std::mem::size_of::<(Url, WorkspaceModuleFingerprint)>())
                        .saturating_add(uri.as_str().len())
                        .saturating_add(fingerprint.canonical_path.to_string_lossy().len())
                }),
        )
}

fn debug_value_weight<T: std::fmt::Debug>(value: &T) -> usize {
    format!("{value:?}").len()
}

fn estimated_indexed_parameter_weight(parameter: &IndexedParameter) -> usize {
    std::mem::size_of::<IndexedParameter>()
        .saturating_add(debug_value_weight(&parameter.pattern))
        .saturating_add(debug_value_weight(&parameter.ty))
        .saturating_add(parameter.optional as usize)
        .saturating_add(parameter.rest as usize)
        .saturating_add(
            parameter
                .validators
                .iter()
                .map(debug_value_weight)
                .sum::<usize>(),
        )
}

fn estimated_behavior_method_weight(method: &IndexedBehaviorMethod) -> usize {
    std::mem::size_of::<IndexedBehaviorMethod>()
        .saturating_add(method.name.len())
        .saturating_add(
            method
                .type_params
                .iter()
                .map(debug_value_weight)
                .sum::<usize>(),
        )
        .saturating_add(
            method
                .parameters
                .iter()
                .map(estimated_indexed_parameter_weight)
                .sum::<usize>(),
        )
        .saturating_add(debug_value_weight(&method.return_type))
        .saturating_add(method.throws.iter().map(debug_value_weight).sum::<usize>())
}

fn estimated_indexed_export_metadata_weight(metadata: &IndexedExportMetadata) -> usize {
    match metadata {
        IndexedExportMetadata::Function {
            is_validator,
            type_params,
            parameters,
            return_type,
            throws,
        } => std::mem::size_of::<IndexedExportMetadata>()
            .saturating_add(*is_validator as usize)
            .saturating_add(type_params.iter().map(debug_value_weight).sum::<usize>())
            .saturating_add(
                parameters
                    .iter()
                    .map(estimated_indexed_parameter_weight)
                    .sum::<usize>(),
            )
            .saturating_add(debug_value_weight(return_type))
            .saturating_add(throws.iter().map(debug_value_weight).sum::<usize>()),
        IndexedExportMetadata::Interface {
            type_params,
            extends,
            fields,
            db_model,
        } => std::mem::size_of::<IndexedExportMetadata>()
            .saturating_add(type_params.iter().map(debug_value_weight).sum::<usize>())
            .saturating_add(extends.iter().map(debug_value_weight).sum::<usize>())
            .saturating_add(fields.iter().map(debug_value_weight).sum::<usize>())
            .saturating_add(db_model.as_ref().map_or(0, debug_value_weight)),
        IndexedExportMetadata::Behavior {
            type_params,
            targets,
            extends,
            methods,
            bindings,
        } => std::mem::size_of::<IndexedExportMetadata>()
            .saturating_add(type_params.iter().map(debug_value_weight).sum::<usize>())
            .saturating_add(targets.iter().map(debug_value_weight).sum::<usize>())
            .saturating_add(extends.iter().map(debug_value_weight).sum::<usize>())
            .saturating_add(
                methods
                    .iter()
                    .map(estimated_behavior_method_weight)
                    .sum::<usize>(),
            )
            .saturating_add(bindings.iter().map(String::len).sum::<usize>()),
        IndexedExportMetadata::TypeAlias {
            type_params,
            ty,
            merged_fields,
        } => std::mem::size_of::<IndexedExportMetadata>()
            .saturating_add(type_params.iter().map(debug_value_weight).sum::<usize>())
            .saturating_add(debug_value_weight(ty))
            .saturating_add(merged_fields.iter().map(debug_value_weight).sum::<usize>()),
        IndexedExportMetadata::Enum { repr, members } => {
            std::mem::size_of::<IndexedExportMetadata>()
                .saturating_add(debug_value_weight(repr))
                .saturating_add(members.iter().map(debug_value_weight).sum::<usize>())
        }
        IndexedExportMetadata::Value { ty } => std::mem::size_of::<IndexedExportMetadata>()
            .saturating_add(ty.as_ref().map_or(0, debug_value_weight)),
    }
}

pub(super) fn workspace_content_fingerprint(source: &str) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

pub(super) fn update_workspace_roots(
    roots: &mut Vec<PathBuf>,
    removed: &std::collections::HashSet<PathBuf>,
    added: impl IntoIterator<Item = PathBuf>,
) {
    roots.retain(|root| !removed.contains(root));
    for root in added {
        if !roots.contains(&root) {
            roots.push(root);
        }
    }
}

pub(super) fn containing_workspace_root(path: &Path, roots: &[PathBuf]) -> Option<PathBuf> {
    roots
        .iter()
        .filter(|root| path.starts_with(root))
        .max_by_key(|root| root.components().count())
        .cloned()
}

pub(super) async fn acquire_cancellable_permit(
    semaphore: Arc<tokio::sync::Semaphore>,
    cancellation: Arc<CancellationSignal>,
) -> Option<tokio::sync::OwnedSemaphorePermit> {
    if cancellation.is_cancelled() {
        return None;
    }
    let permit = tokio::select! {
        biased;
        _ = cancellation.notify.notified() => return None,
        permit = semaphore.acquire_owned() => permit.ok()?,
    };
    (!cancellation.is_cancelled()).then_some(permit)
}

fn normalized_file_uri(path: &Path) -> Option<Url> {
    let normalized = path.canonicalize().unwrap_or_else(|_| {
        path.parent()
            .and_then(|parent| parent.canonicalize().ok())
            .zip(path.file_name())
            .map(|(parent, name)| parent.join(name))
            .unwrap_or_else(|| path.to_path_buf())
    });
    Url::from_file_path(normalized).ok()
}

fn estimated_document_analysis_weight(analysis: &DocumentAnalysis, source_len: usize) -> usize {
    let symbol_weight = analysis.symbols.iter().fold(
        analysis
            .symbols
            .len()
            .saturating_mul(std::mem::size_of::<tyrus_core::analysis::Symbol>()),
        |total, symbol| {
            total
                .saturating_add(symbol.name.len())
                .saturating_add(symbol.ty.len())
                .saturating_add(symbol.import_source.as_ref().map_or(0, String::len))
                .saturating_add(symbol.imported_name.as_ref().map_or(0, String::len))
                .saturating_add(
                    symbol
                        .parameters
                        .len()
                        .saturating_mul(std::mem::size_of::<tyrus_core::ast::Param>()),
                )
        },
    );
    let reference_weight = analysis.references.iter().fold(
        analysis
            .references
            .len()
            .saturating_mul(std::mem::size_of::<tyrus_core::analysis::Reference>()),
        |total, reference| total.saturating_add(reference.name.len()),
    );
    let diagnostic_weight = analysis.diagnostics.iter().fold(
        analysis
            .diagnostics
            .len()
            .saturating_mul(std::mem::size_of::<tyrus_core::analysis::AnalysisDiagnostic>()),
        |total, diagnostic| {
            total
                .saturating_add(diagnostic.code.len())
                .saturating_add(diagnostic.message.len())
                .saturating_add(
                    diagnostic
                        .help
                        .iter()
                        .fold(0usize, |weight, help| weight.saturating_add(help.len())),
                )
                .saturating_add(diagnostic.related.iter().fold(0usize, |weight, related| {
                    weight
                        .saturating_add(std::mem::size_of::<
                            tyrus_core::analysis::AnalysisRelatedInformation,
                        >())
                        .saturating_add(related.message.len())
                }))
        },
    );
    let expression_weight = analysis.expression_types.iter().fold(
        analysis
            .expression_types
            .len()
            .saturating_mul(std::mem::size_of::<tyrus_core::analysis::ExpressionTypeInfo>()),
        |total, expression| total.saturating_add(expression.ty.len()),
    );
    let call_site_weight = analysis.call_sites.iter().fold(
        analysis
            .call_sites
            .len()
            .saturating_mul(std::mem::size_of::<tyrus_core::analysis::CallSite>()),
        |total, call| {
            total
                .saturating_add(call.callee.len())
                .saturating_add(call.receiver.as_ref().map_or(0, String::len))
                .saturating_add(
                    call.arguments
                        .len()
                        .saturating_mul(std::mem::size_of::<std::ops::Range<usize>>()),
                )
        },
    );
    let reference_index_weight = analysis
        .references_by_definition
        .len()
        .saturating_mul(std::mem::size_of::<(
            (usize, usize),
            Vec<std::ops::Range<usize>>,
        )>())
        .saturating_add(analysis.references_by_definition.values().fold(
            0usize,
            |total, ranges| {
                total.saturating_add(
                    ranges
                        .len()
                        .saturating_mul(std::mem::size_of::<std::ops::Range<usize>>()),
                )
            },
        ));
    let indexed_storage_weight = analysis
        .symbol_declaration_index
        .len()
        .saturating_mul(std::mem::size_of::<usize>())
        .saturating_add(
            analysis
                .symbol_name_index
                .len()
                .saturating_mul(std::mem::size_of::<usize>()),
        )
        .saturating_add(
            analysis
                .import_symbol_index
                .len()
                .saturating_mul(std::mem::size_of::<usize>()),
        )
        .saturating_add(
            analysis
                .named_export_index
                .len()
                .saturating_mul(std::mem::size_of::<usize>()),
        )
        .saturating_add(
            analysis
                .wildcard_export_index
                .len()
                .saturating_mul(std::mem::size_of::<usize>()),
        )
        .saturating_add(
            analysis
                .reference_range_index
                .len()
                .saturating_mul(std::mem::size_of::<usize>()),
        )
        .saturating_add(
            analysis
                .diagnostic_range_index
                .len()
                .saturating_mul(std::mem::size_of::<(usize, usize, usize)>()),
        )
        .saturating_add(
            analysis
                .diagnostic_range_prefix_max_end
                .len()
                .saturating_mul(std::mem::size_of::<usize>()),
        )
        .saturating_add(
            analysis
                .expression_type_index
                .len()
                .saturating_mul(std::mem::size_of::<usize>()),
        )
        .saturating_add(
            analysis
                .expression_type_prefix_max_end
                .len()
                .saturating_mul(std::mem::size_of::<usize>()),
        )
        .saturating_add(
            analysis
                .call_site_prefix_max_end
                .len()
                .saturating_mul(std::mem::size_of::<usize>()),
        );
    source_len
        .saturating_mul(8)
        .saturating_add(std::mem::size_of::<DocumentAnalysis>())
        .saturating_add(symbol_weight)
        .saturating_add(reference_weight)
        .saturating_add(diagnostic_weight)
        .saturating_add(expression_weight)
        .saturating_add(call_site_weight)
        .saturating_add(reference_index_weight)
        .saturating_add(indexed_storage_weight)
        .saturating_add(
            analysis
                .node_spans
                .len()
                .saturating_mul(std::mem::size_of::<tyrus_core::analysis::NodeSpan>()),
        )
        .saturating_add(
            analysis
                .tokens
                .len()
                .saturating_mul(std::mem::size_of::<tyrus_core::analysis::SpannedToken>()),
        )
        .saturating_add(
            analysis
                .program
                .as_ref()
                .map_or(0, |_| std::mem::size_of::<tyrus_core::ast::Program>()),
        )
        .max(1024)
}

fn estimated_analysis_weight(analysis: &CachedAnalysis, source_len: usize) -> usize {
    let inlay_weight = analysis.inlay_hints.values().fold(0usize, |total, hints| {
        hints.iter().fold(total, |total, hint| {
            let label_weight = match &hint.label {
                InlayHintLabel::String(label) => label.len(),
                InlayHintLabel::LabelParts(parts) => {
                    parts.iter().map(|part| part.value.len()).sum()
                }
            };
            total
                .saturating_add(std::mem::size_of::<InlayHint>())
                .saturating_add(label_weight)
        })
    });
    let semantic_weight = analysis.semantic_tokens.as_ref().map_or(0, |tokens| {
        tokens
            .len()
            .saturating_mul(std::mem::size_of::<SemanticToken>())
    });
    let call_parameter_weight =
        analysis
            .call_parameters
            .iter()
            .fold(0usize, |total, (callee, parameters)| {
                let parameter_weight = parameters
                    .as_ref()
                    .map_or(0, |parameters| parameters.iter().map(String::len).sum());
                total
                    .saturating_add(std::mem::size_of::<(String, Option<Vec<String>>)>())
                    .saturating_add(callee.len())
                    .saturating_add(parameter_weight)
            });
    let import_behavior_weight = analysis.import_behaviors.iter().fold(
        analysis
            .import_behaviors
            .len()
            .saturating_mul(std::mem::size_of::<(
                String,
                tyrus_core::ast::BehaviorInterfaceInfo,
            )>()),
        |total, (name, behavior)| {
            let method_weight = |methods: &[tyrus_core::ast::BehaviorMethod]| {
                methods.iter().fold(0usize, |weight, method| {
                    weight
                        .saturating_add(std::mem::size_of::<tyrus_core::ast::BehaviorMethod>())
                        .saturating_add(method.name.len())
                        .saturating_add(
                            method
                                .params
                                .len()
                                .saturating_mul(std::mem::size_of::<tyrus_core::ast::Param>()),
                        )
                        .saturating_add(
                            method
                                .type_params
                                .len()
                                .saturating_mul(std::mem::size_of::<tyrus_core::ast::TypeParam>()),
                        )
                        .saturating_add(
                            method
                                .throws
                                .len()
                                .saturating_mul(std::mem::size_of::<tyrus_core::ast::Type>()),
                        )
                        .saturating_add(
                            method
                                .body
                                .len()
                                .saturating_mul(std::mem::size_of::<tyrus_core::ast::Stmt>()),
                        )
                })
            };
            total
                .saturating_add(name.len())
                .saturating_add(
                    behavior
                        .targets
                        .len()
                        .saturating_mul(std::mem::size_of::<tyrus_core::ast::Type>()),
                )
                .saturating_add(
                    behavior
                        .extends
                        .len()
                        .saturating_mul(std::mem::size_of::<tyrus_core::ast::Type>()),
                )
                .saturating_add(behavior.bindings.iter().map(String::len).sum::<usize>())
                .saturating_add(method_weight(&behavior.methods))
                .saturating_add(method_weight(&behavior.binding_methods))
        },
    );
    let lint_diagnostic_weight = analysis.lint_diagnostics.as_ref().map_or(0, |diagnostics| {
        diagnostics.iter().fold(
            diagnostics
                .len()
                .saturating_mul(std::mem::size_of::<tyrus_lint::LintDiagnostic>()),
            |total, diagnostic| {
                total
                    .saturating_add(diagnostic.code.len())
                    .saturating_add(diagnostic.message.len())
                    .saturating_add(diagnostic.help.as_ref().map_or(0, String::len))
                    .saturating_add(diagnostic.note.as_ref().map_or(0, String::len))
                    .saturating_add(diagnostic.fix.as_ref().map_or(0, |fix| {
                        std::mem::size_of::<tyrus_lint::LintFix>()
                            .saturating_add(fix.message.len())
                            .saturating_add(fix.replacement.len())
                    }))
            },
        )
    });
    estimated_document_analysis_weight(&analysis.analysis, source_len)
        .saturating_add(std::mem::size_of::<CachedAnalysis>())
        .saturating_add(import_behavior_weight)
        .saturating_add(lint_diagnostic_weight)
        .saturating_add(inlay_weight)
        .saturating_add(semantic_weight)
        .saturating_add(call_parameter_weight)
        .max(1024)
}

pub(crate) fn enforce_analysis_cache_limits(
    documents: &mut std::collections::HashMap<Url, DocumentState>,
) {
    loop {
        let cached_count = documents
            .values()
            .filter(|document| document.analysis.is_some())
            .count();
        let weight = documents.values().fold(0usize, |total, document| {
            total.saturating_add(document.analysis.as_ref().map_or(0, |analysis| {
                estimated_analysis_weight(analysis, document.text.len())
            }))
        });
        if cached_count <= MAX_CACHED_ANALYSES && weight <= MAX_CACHED_ANALYSIS_WEIGHT {
            return;
        }
        let Some(document) = documents
            .values_mut()
            .find(|document| document.analysis.is_some())
        else {
            return;
        };
        document.analysis = None;
    }
}

fn insert_cached_analysis(
    documents: &mut std::collections::HashMap<Url, DocumentState>,
    uri: Url,
    source: &str,
    version: Option<i32>,
    analysis: CachedAnalysis,
) {
    let Some(document) = documents.get_mut(&uri) else {
        return;
    };
    if document.text != source || Some(document.version) != version {
        return;
    }
    if estimated_analysis_weight(&analysis, source.len()) > MAX_CACHED_ANALYSIS_WEIGHT {
        return;
    }
    document.analysis = Some(analysis);
    enforce_analysis_cache_limits(documents);
}

pub(super) fn insert_cached_workspace_analysis(
    cache: &mut WorkspaceAnalysisEntries,
    uri: Url,
    source: &str,
    analysis: Arc<DocumentAnalysis>,
) {
    let entry_weight = estimated_document_analysis_weight(&analysis, source.len());
    if entry_weight > MAX_CACHED_WORKSPACE_ANALYSIS_WEIGHT {
        return;
    }
    cache.remove(&uri);
    let mut weight = cache.values().fold(0usize, |total, (source, analysis)| {
        total.saturating_add(estimated_document_analysis_weight(analysis, source.len()))
    });
    while cache.len() >= MAX_CACHED_ANALYSES
        || weight.saturating_add(entry_weight) > MAX_CACHED_WORKSPACE_ANALYSIS_WEIGHT
    {
        let Some(victim) = cache.keys().next().cloned() else {
            break;
        };
        if let Some((source, analysis)) = cache.remove(&victim) {
            weight =
                weight.saturating_sub(estimated_document_analysis_weight(&analysis, source.len()));
        }
    }
    cache.insert(uri, (source.to_string(), analysis));
}

pub(super) fn imported_db_model_metadata(
    program: &tyrus_core::ast::Program,
    name: &str,
    backend: Option<&tyrus_core::db::DatabaseBackend>,
) -> Option<tyrus_core::ast::DbModelMetadata> {
    let mut program = program.clone();
    tyrus_core::db::apply_backend_model_defaults(&mut program, backend);
    program.items.into_iter().find_map(|item| match item {
        tyrus_core::ast::Item::InterfaceDecl {
            name: item_name,
            db_model: Some(model),
            ..
        } if item_name == name => Some(model),
        _ => None,
    })
}

pub(super) fn register_implicit_db_error_fields(
    imported_db_models: &HashMap<String, tyrus_core::ast::DbModelMetadata>,
    import_interfaces: &mut HashMap<String, Vec<Field>>,
) {
    if imported_db_models.is_empty() {
        return;
    }
    import_interfaces
        .entry("DbError".to_string())
        .or_insert_with(tyrus_core::db::db_error_fields);
}

pub(super) fn import_source_range(
    source_text: &str,
    declaration: &std::ops::Range<usize>,
    module: &str,
) -> Option<std::ops::Range<usize>> {
    let tokens = tyrus_core::lexer::lex_spanned(source_text).ok()?;
    let declaration_token = tokens
        .iter()
        .position(|(_, range)| range.start <= declaration.start && declaration.start < range.end)?;
    tokens[declaration_token..]
        .iter()
        .take_while(|(token, _)| !matches!(token, tyrus_core::lexer::Token::Semicolon))
        .find_map(|(token, range)| match token {
            tyrus_core::lexer::Token::StringLit(value) if value == module => Some(range.clone()),
            _ => None,
        })
}

pub(super) fn attach_import_alias_related_information(
    diagnostic: &mut Diagnostic,
    uri: &Url,
    source_text: &str,
    analysis_diagnostic: &AnalysisDiagnostic,
) {
    if analysis_diagnostic.code != "E0432" {
        return;
    }
    let Some(specifier) = source_text
        .get(analysis_diagnostic.range.clone())
        .and_then(|value| value.strip_prefix('"'))
        .and_then(|value| value.strip_suffix('"'))
        .filter(|value| value.starts_with('#'))
    else {
        return;
    };
    let Ok(document) = uri.to_file_path() else {
        return;
    };
    let Some(root) = tyrus_core::project::find_project_root(&document) else {
        return;
    };
    let config_path = root.join("tyrus.json");
    let Ok(config_source) = fs::read_to_string(&config_path) else {
        return;
    };
    let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&config_source) else {
        return;
    };
    let Some(imports) = manifest
        .get("imports")
        .and_then(serde_json::Value::as_object)
    else {
        return;
    };
    let Some(alias) = imports
        .keys()
        .filter(|key| {
            specifier == key.as_str()
                || specifier
                    .strip_prefix(key.as_str())
                    .is_some_and(|subpath| subpath.starts_with('/'))
        })
        .max_by_key(|key| key.len())
    else {
        return;
    };
    let Ok(key) = serde_json::to_string(alias) else {
        return;
    };
    let Some(start) = config_source.find(&key).map(|offset| offset + 1) else {
        return;
    };
    let Ok(config_uri) = Url::from_file_path(config_path) else {
        return;
    };
    diagnostic
        .related_information
        .get_or_insert_with(Vec::new)
        .push(DiagnosticRelatedInformation {
            location: Location {
                uri: config_uri,
                range: byte_range_to_lsp(&config_source, start..start + alias.len()),
            },
            message: format!("import alias `{alias}` is declared here"),
        });
}

pub(super) fn function_throws_from_program(
    program: &tyrus_core::ast::Program,
    name: &str,
) -> Option<Vec<Type>> {
    let tyrus_core::ast::Item::FnDecl { params, throws, .. } = program.items.iter().find(|item| matches!(item, tyrus_core::ast::Item::FnDecl { name: item_name, .. } if item_name == name))? else {
        return None;
    };
    let mut effects = throws.clone();
    if params.iter().any(|param| !param.decorators.is_empty())
        && !effects
            .iter()
            .any(|ty| ty == &Type::Named("ValidationError".to_string()))
    {
        effects.push(Type::Named("ValidationError".to_string()));
    }
    Some(effects)
}

pub(super) fn is_entry_point_uri(uri: &Url) -> bool {
    uri.to_file_path()
        .is_ok_and(|path| tyrus_core::project::is_entry_module(&path))
}

pub(super) struct InitializationOptions {
    pub(super) format_enable: bool,
    pub(super) code_lens_quick_fixes_enable: bool,
    pub(super) diagnostic_debounce_ms: u64,
    pub(super) workspace_max_files: usize,
    pub(super) workspace_max_depth: usize,
    pub(super) workspace_exclude_directories: HashSet<String>,
}

impl InitializationOptions {
    pub(super) fn parse(options: &Option<serde_json::Value>) -> Self {
        let flag = |key: &str| {
            options
                .as_ref()
                .and_then(|options| options.get(key))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true)
        };
        let bounded_limit = |key: &str, default: usize, maximum: usize| {
            options
                .as_ref()
                .and_then(|options| options.get(key))
                .and_then(serde_json::Value::as_u64)
                .and_then(|value| usize::try_from(value).ok())
                .unwrap_or(default)
                .clamp(1, maximum)
        };
        let mut workspace_exclude_directories = IGNORED_WORKSPACE_DIRECTORIES
            .iter()
            .map(|directory| (*directory).to_string())
            .collect::<HashSet<_>>();
        if let Some(directories) = options
            .as_ref()
            .and_then(|options| options.get("workspaceExcludeDirectories"))
            .and_then(serde_json::Value::as_array)
        {
            workspace_exclude_directories.extend(directories.iter().filter_map(|directory| {
                let directory = directory.as_str()?.trim();
                (!directory.is_empty()
                    && directory.len() <= 128
                    && directory != "."
                    && directory != ".."
                    && !directory.contains('/')
                    && !directory.contains('\\'))
                .then(|| directory.to_string())
            }));
        }
        Self {
            format_enable: flag("formatEnable"),
            code_lens_quick_fixes_enable: flag("codeLensQuickFixesEnable"),
            diagnostic_debounce_ms: options
                .as_ref()
                .and_then(|options| options.get("diagnosticDebounceMs"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(120)
                .min(1_000),
            workspace_max_files: bounded_limit("workspaceMaxFiles", MAX_WORKSPACE_FILES, 100_000),
            workspace_max_depth: bounded_limit("workspaceMaxDepth", MAX_WORKSPACE_SCAN_DEPTH, 256),
            workspace_exclude_directories,
        }
    }
}

pub(super) struct InterfaceInfo {
    pub name: String,
    pub fields: Vec<Field>,
    pub link: String,
    pub documentation: Option<DocComment>,
}

#[derive(Clone)]
pub(super) struct ObjectInterfaceCandidate {
    pub(super) name: String,
    pub(super) fields: Vec<Field>,
    pub(super) exported: bool,
    pub(super) uri: Url,
}

pub(super) struct CallSignature {
    pub(super) parameters: Vec<String>,
    pub(super) detail: Option<String>,
}

impl Backend {
    pub(super) fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(std::collections::HashMap::new())),
            workspace_analysis_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            import_dependencies: Arc::new(RwLock::new(std::collections::HashMap::new())),
            workspace_definitions_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            workspace_occurrences_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            workspace_exports_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            workspace_files_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            workspace_index_generation: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            workspace_root_generations: Arc::new(RwLock::new(std::collections::HashMap::new())),
            workspace_generation_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            workspace_index_semaphore: Arc::new(tokio::sync::Semaphore::new(
                WORKSPACE_INDEX_CONCURRENCY,
            )),
            workspace_scan_max_files: Arc::new(std::sync::atomic::AtomicUsize::new(
                MAX_WORKSPACE_FILES,
            )),
            workspace_scan_max_depth: Arc::new(std::sync::atomic::AtomicUsize::new(
                MAX_WORKSPACE_SCAN_DEPTH,
            )),
            workspace_scan_excluded_directories: Arc::new(RwLock::new(
                IGNORED_WORKSPACE_DIRECTORIES
                    .iter()
                    .map(|directory| (*directory).to_string())
                    .collect(),
            )),
            analysis_inflight: Arc::new(RwLock::new(std::collections::HashMap::new())),
            analysis_cancellation: Arc::new(RwLock::new(std::collections::HashMap::new())),
            diagnostic_cancellation: Arc::new(RwLock::new(std::collections::HashMap::new())),
            pending_diagnostics: Arc::new(RwLock::new(std::collections::HashMap::new())),
            diagnostic_task_semaphore: Arc::new(tokio::sync::Semaphore::new(
                MAX_PENDING_DIAGNOSTIC_TASKS,
            )),
            context_generation: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            analysis_semaphore: Arc::new(tokio::sync::Semaphore::new(ANALYSIS_WORKER_LIMIT)),
            background_analysis_semaphore: Arc::new(tokio::sync::Semaphore::new(
                BACKGROUND_ANALYSIS_WORKER_LIMIT,
            )),
            diagnostic_debounce_ms: Arc::new(std::sync::atomic::AtomicU64::new(120)),
            formatting_semaphore: Arc::new(tokio::sync::Semaphore::new(2)),
            quick_fix_semaphore: Arc::new(tokio::sync::Semaphore::new(2)),
            format_config_cache: Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
            lint_config_cache: Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
            import_map_cache: Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
            project_database_config_cache: Arc::new(std::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
            schema_file_watchers: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            db_schema_cache: Arc::new(std::sync::Mutex::new(
                tyrus_core::db::DbSchemaCache::default(),
            )),
            db_schema_semaphore: Arc::new(tokio::sync::Semaphore::new(2)),
            project_io_semaphore: Arc::new(tokio::sync::Semaphore::new(2)),
            workspace_root: Arc::new(RwLock::new(None)),
            workspace_roots: Arc::new(RwLock::new(Vec::new())),
            workspace_roots_generation: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            project_roots: Arc::new(RwLock::new(std::collections::HashMap::new())),
            watched_files_dynamic_registration: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            env_state: Arc::new(RwLock::new(None)),
            env_config: Arc::new(RwLock::new(None)),
        }
    }

    pub(super) async fn env_schema(&self) -> Option<tyrus_core::env::EnvSchema> {
        let wait_started = std::time::Instant::now();
        let state = self.env_state.read().await;
        trace_lsp_stage("env_state_lock_wait", wait_started);
        let hold_started = std::time::Instant::now();
        let schema = state.as_ref().map(|state| state.schema.clone());
        drop(state);
        trace_lsp_stage("env_state_lock_hold", hold_started);
        schema
    }

    pub(super) async fn is_env_document(&self, uri: &Url) -> bool {
        let wait_started = std::time::Instant::now();
        let config = self.env_config.read().await;
        trace_lsp_stage("env_config_lock_wait", wait_started);
        let hold_started = std::time::Instant::now();
        let is_env = config.as_ref().is_some_and(|config| &config.uri == uri);
        drop(config);
        trace_lsp_stage("env_config_lock_hold", hold_started);
        is_env
    }

    pub(super) async fn reload_env_from_buffer(&self, uri: &Url, source: &str, version: i32) {
        let config_wait_started = std::time::Instant::now();
        let config_guard = self.env_config.read().await;
        trace_lsp_stage("env_config_lock_wait", config_wait_started);
        let config_hold_started = std::time::Instant::now();
        let config = config_guard.clone();
        drop(config_guard);
        trace_lsp_stage("env_config_lock_hold", config_hold_started);
        let Some(config) = config else {
            return;
        };
        let (vars, mut diagnostics) = tyrus_core::env::parse_env_source(source);
        diagnostics.extend(tyrus_core::env::check_env_schema(&vars));
        let Ok(file_path) = uri.to_file_path() else {
            return;
        };
        let schema = tyrus_core::env::EnvSchema {
            env_file: config.env_file.clone(),
            file_path,
            source: source.to_string(),
            vars,
        };
        if let Err(error) = tyrus_core::env::sync_env_example(&schema) {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!("could not write the .env.example mirror: {error}"),
                )
                .await;
        }
        let lsp_diagnostics = diagnostics
            .iter()
            .map(|diagnostic| diagnostic_to_lsp(uri, source, diagnostic))
            .collect::<Vec<_>>();
        let state_wait_started = std::time::Instant::now();
        let mut state = self.env_state.write().await;
        trace_lsp_stage("env_state_lock_wait", state_wait_started);
        let state_hold_started = std::time::Instant::now();
        *state = Some(EnvState {
            schema,
            env_uri: uri.clone(),
        });
        drop(state);
        trace_lsp_stage("env_state_lock_hold", state_hold_started);
        self.client
            .publish_diagnostics(uri.clone(), lsp_diagnostics, Some(version))
            .await;
        self.reanalyze_open_documents().await;
    }

    pub(super) async fn reanalyze_open_documents(&self) {
        self.invalidate_analysis_cache().await;
        let mut document_uris = self
            .documents
            .read()
            .await
            .iter()
            .filter(|(uri, _)| is_tyrus_uri(uri))
            .map(|(uri, _)| uri.clone())
            .collect::<Vec<_>>()
            .into_iter();
        let mut tasks = tokio::task::JoinSet::new();
        for _ in 0..BACKGROUND_ANALYSIS_WORKER_LIMIT {
            let Some((uri, source, version)) =
                self.next_open_document_snapshot(&mut document_uris).await
            else {
                break;
            };
            let backend = self.clone();
            tasks.spawn(async move {
                backend.publish_analysis(uri, &source, Some(version)).await;
            });
        }
        while tasks.join_next().await.is_some() {
            let Some((uri, source, version)) =
                self.next_open_document_snapshot(&mut document_uris).await
            else {
                continue;
            };
            let backend = self.clone();
            tasks.spawn(async move {
                backend.publish_analysis(uri, &source, Some(version)).await;
            });
        }
    }

    async fn next_open_document_snapshot(
        &self,
        document_uris: &mut impl Iterator<Item = Url>,
    ) -> Option<(Url, String, i32)> {
        for uri in document_uris {
            let snapshot = {
                let documents = self.documents.read().await;
                documents
                    .get(&uri)
                    .map(|document| (document.text.clone(), document.version))
            };
            if let Some((source, version)) = snapshot {
                return Some((uri, source, version));
            }
        }
        None
    }

    pub(super) async fn reload_env(&self) {
        let state_wait_started = std::time::Instant::now();
        let state = self.env_state.read().await;
        trace_lsp_stage("env_state_lock_wait", state_wait_started);
        let state_hold_started = std::time::Instant::now();
        let previous_uri = state.as_ref().map(|state| state.env_uri.clone());
        drop(state);
        trace_lsp_stage("env_state_lock_hold", state_hold_started);
        let root = self.workspace_root.read().await.clone();

        let mut next: Option<EnvState> = None;
        let mut env_config: Option<EnvConfig> = None;
        let mut env_diagnostics: Vec<Diagnostic> = Vec::new();
        let mut env_diagnostic_version = None;
        let mut config_diagnostics: Vec<Diagnostic> = Vec::new();
        let config_uri = root
            .as_ref()
            .and_then(|root| Url::from_file_path(root.join("tyrus.json")).ok());
        if let Some(root) = root
            && let Ok(config) = tyrus_core::project::load_project(&root)
        {
            if let Some(env_file) = config
                .security
                .as_ref()
                .and_then(|security| security.env_file.as_deref())
                && let Ok(uri) = Url::from_file_path(root.join(env_file))
            {
                env_config = Some(EnvConfig {
                    env_file: env_file.to_string(),
                    uri,
                });
            }
            match tyrus_core::env::load_env_schema(&root, &config) {
                Ok(Some((mut schema, mut diagnostics))) => {
                    if let Err(error) = tyrus_core::env::sync_env_example(&schema) {
                        self.client
                            .log_message(
                                MessageType::WARNING,
                                format!("could not write the .env.example mirror: {error}"),
                            )
                            .await;
                    }
                    if let Ok(env_uri) = Url::from_file_path(&schema.file_path) {
                        let open_document = self
                            .documents
                            .read()
                            .await
                            .get(&env_uri)
                            .map(|document| (document.text.clone(), document.version));
                        if let Some((source, version)) = open_document {
                            let (vars, mut buffer_diagnostics) =
                                tyrus_core::env::parse_env_source(&source);
                            buffer_diagnostics.extend(tyrus_core::env::check_env_schema(&vars));
                            schema.source = source;
                            schema.vars = vars;
                            diagnostics = buffer_diagnostics;
                            env_diagnostic_version = Some(version);
                        }
                        env_diagnostics = diagnostics
                            .iter()
                            .map(|diagnostic| {
                                diagnostic_to_lsp(&env_uri, &schema.source, diagnostic)
                            })
                            .collect();
                        next = Some(EnvState { schema, env_uri });
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    if let Some(diagnostic) = env_file_missing_diagnostic(&root, &error) {
                        config_diagnostics.push(diagnostic);
                    }
                }
            }
        }

        let next_uri = next.as_ref().map(|state| state.env_uri.clone());
        let state_wait_started = std::time::Instant::now();
        let mut state = self.env_state.write().await;
        trace_lsp_stage("env_state_lock_wait", state_wait_started);
        let state_hold_started = std::time::Instant::now();
        *state = next;
        drop(state);
        trace_lsp_stage("env_state_lock_hold", state_hold_started);
        let config_wait_started = std::time::Instant::now();
        let mut config = self.env_config.write().await;
        trace_lsp_stage("env_config_lock_wait", config_wait_started);
        let config_hold_started = std::time::Instant::now();
        *config = env_config;
        drop(config);
        trace_lsp_stage("env_config_lock_hold", config_hold_started);
        if let Some(previous) = previous_uri.filter(|previous| Some(previous) != next_uri.as_ref())
        {
            self.client
                .publish_diagnostics(previous, Vec::new(), None)
                .await;
        }
        if let Some(env_uri) = next_uri {
            self.client
                .publish_diagnostics(env_uri, env_diagnostics, env_diagnostic_version)
                .await;
        }
        if let Some(config_uri) = config_uri {
            self.client
                .publish_diagnostics(config_uri, config_diagnostics, None)
                .await;
        }
    }

    pub(super) async fn source(&self, uri: &Url) -> Option<String> {
        if let Some(document) = self.documents.read().await.get(uri) {
            return Some(document.text.clone());
        }
        let path = uri.to_file_path().ok()?;
        let started = std::time::Instant::now();
        let source = fs::read_to_string(path).ok();
        trace_lsp_stage("document_source_filesystem_read", started);
        source
    }

    pub(super) async fn invalidate_analysis_cache(&self) {
        self.context_generation
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        let mut cancellation = self.analysis_cancellation.write().await;
        for token in cancellation.values() {
            token.cancel();
        }
        cancellation.clear();
        drop(cancellation);
        let mut cancellation = self.diagnostic_cancellation.write().await;
        for token in cancellation.values() {
            token.cancel();
        }
        cancellation.clear();
        drop(cancellation);
        for document in self.documents.write().await.values_mut() {
            document.analysis = None;
        }
    }

    pub(super) async fn invalidate_analysis_cache_for(&self, uris: &HashSet<Url>) {
        let generation = self
            .context_generation
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel)
            + 1;
        let mut cancellation = self.analysis_cancellation.write().await;
        cancellation.retain(|uri, token| {
            if uris.contains(uri) {
                token.cancel();
                false
            } else {
                true
            }
        });
        drop(cancellation);
        let mut cancellation = self.diagnostic_cancellation.write().await;
        cancellation.retain(|uri, token| {
            if uris.contains(uri) {
                token.cancel();
                false
            } else {
                true
            }
        });
        drop(cancellation);
        for (uri, document) in self.documents.write().await.iter_mut() {
            if uris.contains(uri) {
                document.analysis = None;
            } else if let Some(analysis) = document.analysis.as_mut() {
                analysis.context_generation = generation;
            }
        }
        self.analysis_inflight
            .write()
            .await
            .retain(|(uri, _, _, _), _| !uris.contains(uri));
    }

    pub(super) async fn cancel_document_analysis(&self, uri: &Url) {
        if let Some(token) = self.analysis_cancellation.write().await.remove(uri) {
            token.cancel();
        }
        if let Some(token) = self.diagnostic_cancellation.write().await.remove(uri) {
            token.cancel();
        }
    }

    pub(super) async fn invalidate_workspace_index(&self) {
        self.workspace_index_generation
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        self.workspace_root_generations.write().await.clear();
        self.workspace_exports_cache.write().await.clear();
        self.workspace_definitions_cache.write().await.clear();
        self.workspace_files_cache.write().await.clear();
        self.workspace_occurrences_cache.write().await.clear();
    }

    pub(super) async fn invalidate_project_roots(&self) {
        self.workspace_roots_generation
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        self.project_roots.write().await.clear();
    }

    pub(super) async fn detach_workspace_roots(&self, removed_roots: &HashSet<PathBuf>) {
        let contains_path = |path: &Path| removed_roots.iter().any(|root| path.starts_with(root));
        let mut evicted = 0usize;
        let mut generations = self.workspace_root_generations.write().await;
        let previous = generations.len();
        generations.retain(|root, _| !contains_path(root));
        evicted += previous - generations.len();
        drop(generations);
        let mut exports = self.workspace_exports_cache.write().await;
        let previous = exports.len();
        exports.retain(|root, _| !contains_path(root));
        evicted += previous - exports.len();
        drop(exports);
        let mut files = self.workspace_files_cache.write().await;
        let previous = files.len();
        files.retain(|root, _| !contains_path(root));
        evicted += previous - files.len();
        drop(files);
        let mut definitions = self.workspace_definitions_cache.write().await;
        let previous = definitions.len();
        definitions.retain(|(uri, _), entry| {
            !uri.to_file_path().is_ok_and(|path| contains_path(&path))
                && !entry.dependencies.iter().any(|dependency| {
                    dependency
                        .to_file_path()
                        .is_ok_and(|path| contains_path(&path))
                })
        });
        evicted += previous - definitions.len();
        drop(definitions);
        let mut occurrences = self.workspace_occurrences_cache.write().await;
        let previous = occurrences.len();
        occurrences.retain(|(uri, _, _), occurrences| {
            !removed_roots
                .iter()
                .any(|root| workspace_occurrence_cache_depends_on_root(uri, occurrences, root))
        });
        evicted += previous - occurrences.len();
        trace_lsp_count("workspace_detached_cache_entries", evicted);
    }

    pub(super) async fn invalidate_workspace_content_index(
        &self,
        changed_root: &Path,
        inventory_changed: bool,
    ) {
        let evicted_root = {
            let mut root_generations = self.workspace_root_generations.write().await;
            let evicted_root = if root_generations.len() >= MAX_CACHED_WORKSPACE_ROOTS
                && !root_generations.contains_key(changed_root)
            {
                let victim = root_generations.keys().min().cloned();
                if let Some(victim) = &victim {
                    root_generations.remove(victim);
                }
                victim
            } else {
                None
            };
            let root_generation = self
                .workspace_generation_counter
                .fetch_add(1, std::sync::atomic::Ordering::AcqRel)
                + 1;
            root_generations.insert(changed_root.to_path_buf(), root_generation);
            evicted_root
        };
        if let Some(evicted_root) = evicted_root {
            self.workspace_exports_cache
                .write()
                .await
                .retain(|root, _| root != &evicted_root);
            self.workspace_files_cache
                .write()
                .await
                .retain(|root, _| root != &evicted_root);
            self.workspace_definitions_cache
                .write()
                .await
                .retain(|(uri, _), entry| {
                    !uri.to_file_path()
                        .is_ok_and(|path| path.starts_with(&evicted_root))
                        && !entry.dependencies.iter().any(|dependency| {
                            dependency
                                .to_file_path()
                                .is_ok_and(|path| path.starts_with(&evicted_root))
                        })
                });
            self.workspace_occurrences_cache
                .write()
                .await
                .retain(|(uri, _, _), occurrences| {
                    !workspace_occurrence_cache_depends_on_root(uri, occurrences, &evicted_root)
                });
        }
        self.workspace_exports_cache
            .write()
            .await
            .retain(|root, _| root != changed_root);
        self.workspace_definitions_cache
            .write()
            .await
            .retain(|(uri, _), entry| {
                !uri.to_file_path()
                    .is_ok_and(|path| path.starts_with(changed_root))
                    && !entry.dependencies.iter().any(|dependency| {
                        dependency
                            .to_file_path()
                            .is_ok_and(|path| path.starts_with(changed_root))
                    })
            });
        self.workspace_occurrences_cache
            .write()
            .await
            .retain(|(uri, _, _), occurrences| {
                !workspace_occurrence_cache_depends_on_root(uri, occurrences, changed_root)
            });
        let generation = self.workspace_index_generation_for(changed_root).await;
        let mut files_cache = self.workspace_files_cache.write().await;
        update_workspace_files_cache_for_root(
            &mut files_cache,
            changed_root,
            generation,
            inventory_changed,
        );
    }

    pub(super) async fn workspace_index_generation_for(&self, root: &Path) -> WorkspaceGeneration {
        let global = self
            .workspace_index_generation
            .load(std::sync::atomic::Ordering::Acquire);
        let root = self
            .workspace_root_generations
            .read()
            .await
            .get(root)
            .copied()
            .unwrap_or_default();
        WorkspaceGeneration { global, root }
    }

    pub(super) async fn begin_workspace_index_progress(
        &self,
        token: Option<ProgressToken>,
        file_count: usize,
    ) -> Option<ProgressToken> {
        let token = token.filter(|_| file_count >= WORKSPACE_PROGRESS_MIN_FILES)?;
        self.client
            .send_notification::<notification::Progress>(ProgressParams {
                token: token.clone(),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(
                    WorkDoneProgressBegin {
                        title: "Indexing Tyrus workspace".to_string(),
                        cancellable: Some(false),
                        message: Some(format!("Analyzing {file_count} files")),
                        percentage: None,
                    },
                )),
            })
            .await;
        Some(token)
    }

    pub(super) async fn end_workspace_index_progress(&self, token: Option<ProgressToken>) {
        let Some(token) = token else { return };
        self.client
            .send_notification::<notification::Progress>(ProgressParams {
                token,
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
                    message: Some("Workspace index ready".to_string()),
                })),
            })
            .await;
    }

    pub(super) async fn workspace_files(
        &self,
        root: &Path,
        generation: WorkspaceGeneration,
    ) -> Vec<PathBuf> {
        let files_started = std::time::Instant::now();
        let cache_wait_started = std::time::Instant::now();
        let cache = self.workspace_files_cache.read().await;
        trace_lsp_stage("workspace_files_cache_lock_wait", cache_wait_started);
        let cache_hold_started = std::time::Instant::now();
        let cached = cache
            .get(root)
            .filter(|entry| entry.generation == generation)
            .map(|entry| entry.files.clone());
        drop(cache);
        trace_lsp_stage("workspace_files_cache_lock_hold", cache_hold_started);
        let (mut files, mut truncated) = if let Some(files) = cached {
            trace_lsp_event("workspace_files_cache_hit");
            (files, false)
        } else {
            let Ok(permit) = Arc::clone(&self.workspace_index_semaphore)
                .acquire_owned()
                .await
            else {
                return Vec::new();
            };
            let root_path = root.to_path_buf();
            let scan_root = root_path.clone();
            let max_files = self
                .workspace_scan_max_files
                .load(std::sync::atomic::Ordering::Acquire);
            let max_depth = self
                .workspace_scan_max_depth
                .load(std::sync::atomic::Ordering::Acquire);
            let excluded_directories = self
                .workspace_scan_excluded_directories
                .read()
                .await
                .clone();
            let (files, truncated) = tokio::task::spawn_blocking(move || {
                let _permit = permit;
                let mut files = Vec::new();
                let truncated = collect_tyrus_files_with_options(
                    &scan_root,
                    &mut files,
                    max_files,
                    max_depth,
                    &excluded_directories,
                    0,
                );
                (files, truncated)
            })
            .await
            .unwrap_or_default();
            let current_generation = self.workspace_index_generation_for(&root_path).await;
            let mut cache = self.workspace_files_cache.write().await;
            if current_generation == generation {
                evict_workspace_root_cache_entries(&mut cache, &root_path);
                let cached_paths = cache
                    .iter()
                    .filter(|(cached_root, _)| *cached_root != &root_path)
                    .fold(0usize, |total, (_, entry)| {
                        total.saturating_add(entry.files.len())
                    });
                if cached_paths.saturating_add(files.len()) <= MAX_CACHED_WORKSPACE_FILE_PATHS {
                    cache.insert(
                        root_path.clone(),
                        WorkspaceFilesCacheEntry {
                            generation,
                            files: files.clone(),
                        },
                    );
                }
            }
            trace_lsp_stage("workspace_files_build", files_started);
            (files, truncated)
        };
        let open_paths = self
            .documents
            .read()
            .await
            .keys()
            .filter_map(|uri| uri.to_file_path().ok())
            .collect::<Vec<_>>();
        let mut open_paths = open_paths;
        open_paths.sort();
        let mut seen_paths = files.iter().cloned().collect::<HashSet<_>>();
        let max_files = self
            .workspace_scan_max_files
            .load(std::sync::atomic::Ordering::Acquire);
        for path in open_paths {
            if !seen_paths.insert(path.clone()) {
                continue;
            }
            if files.len() >= max_files {
                truncated = true;
                break;
            }
            files.push(path);
        }
        if truncated {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!(
                        "Workspace indexing reached its limit of {} Tyr files or depth {}; some workspace-wide results may omit files.",
                        self.workspace_scan_max_files.load(std::sync::atomic::Ordering::Acquire),
                        self.workspace_scan_max_depth.load(std::sync::atomic::Ordering::Acquire)
                    ),
                )
                .await;
        }
        files
    }

    pub(super) async fn document_version(&self, uri: &Url) -> Option<i32> {
        let lock_started = std::time::Instant::now();
        let documents = self.documents.read().await;
        trace_lsp_stage("document_version_lock_wait", lock_started);
        let lock_hold_started = std::time::Instant::now();
        let version = documents.get(uri).map(|document| document.version);
        drop(documents);
        trace_lsp_stage("document_version_lock_hold", lock_hold_started);
        version
    }

    pub(super) async fn remove_import_dependencies(&self, importer: &Url) {
        let mut dependencies = self.import_dependencies.write().await;
        dependencies.retain(|_, dependents| {
            dependents.remove(importer);
            !dependents.is_empty()
        });
    }

    pub(super) async fn replace_import_dependency_graph(
        &self,
        updates: HashMap<Url, HashSet<Url>>,
    ) {
        let mut dependencies = self.import_dependencies.write().await;
        let importers = updates.keys().cloned().collect::<HashSet<_>>();
        dependencies.retain(|_, dependents| {
            dependents.retain(|importer| !importers.contains(importer));
            !dependents.is_empty()
        });
        for (importer, targets) in updates {
            for target in targets {
                dependencies
                    .entry(target)
                    .or_default()
                    .insert(importer.clone());
            }
        }
    }

    pub(super) async fn import_dependents(&self, changed: &[Url]) -> HashSet<Url> {
        let dependencies = self.import_dependencies.read().await;
        let mut pending = changed
            .iter()
            .filter_map(|uri| {
                uri.to_file_path()
                    .ok()
                    .and_then(|path| normalized_file_uri(&path))
            })
            .collect::<std::collections::VecDeque<_>>();
        let mut visited = HashSet::new();
        let mut result = HashSet::new();
        while let Some(target) = pending.pop_front() {
            if !visited.insert(target.clone()) {
                continue;
            }
            if let Some(dependents) = dependencies.get(&target) {
                for dependent in dependents {
                    if result.insert(dependent.clone())
                        && let Ok(path) = dependent.to_file_path()
                        && let Some(uri) = normalized_file_uri(&path)
                    {
                        pending.push_back(uri);
                    }
                }
            }
        }
        result
    }

    pub(super) async fn import_dependents_in_roots(
        &self,
        roots: &HashSet<PathBuf>,
    ) -> HashSet<Url> {
        let dependencies = self.import_dependencies.read().await;
        dependencies
            .iter()
            .filter(|(target, _)| {
                target
                    .to_file_path()
                    .ok()
                    .is_some_and(|path| roots.iter().any(|root| path.starts_with(root)))
            })
            .flat_map(|(_, dependents)| dependents.iter().cloned())
            .collect()
    }

    pub(super) async fn analysis_for(&self, uri: &Url, source: &str) -> Arc<DocumentAnalysis> {
        for attempt in 0..3 {
            let analysis = self.analysis_for_snapshot(uri, source).await;
            if !analysis_is_cancelled(&analysis) {
                return analysis;
            }
            let version = self.document_version(uri).await;
            let document_is_current = self.document_matches(uri, source, version).await;
            if !analysis_retry_allowed(&analysis, document_is_current, attempt) {
                return analysis;
            }
            trace_lsp_event("analysis_retry_after_invalidation");
        }
        Arc::new(DocumentAnalysis::default())
    }

    async fn analysis_for_snapshot(&self, uri: &Url, source: &str) -> Arc<DocumentAnalysis> {
        let analysis_started = std::time::Instant::now();
        let version = self.document_version(uri).await;
        let context_generation = self
            .context_generation
            .load(std::sync::atomic::Ordering::Acquire);
        let analysis_cache_lock_started = std::time::Instant::now();
        let documents = self.documents.read().await;
        trace_lsp_stage("analysis_cache_lock_wait", analysis_cache_lock_started);
        let analysis_cache_lock_hold_started = std::time::Instant::now();
        let cached = documents
            .get(uri)
            .and_then(|document| document.cached_analysis(source, version, context_generation))
            .cloned();
        drop(documents);
        trace_lsp_stage("analysis_cache_lock_hold", analysis_cache_lock_hold_started);
        if let Some(cached) = cached
            && self.document_matches(uri, source, version).await
            && self
                .context_generation
                .load(std::sync::atomic::Ordering::Acquire)
                == context_generation
        {
            trace_lsp_stage("analysis_cache_hit", analysis_started);
            return Arc::clone(&cached.analysis);
        }
        let key = (uri.clone(), version, context_generation, source.to_string());
        let inflight_lock_started = std::time::Instant::now();
        let (inflight, registered) = {
            let mut tasks = self.analysis_inflight.write().await;
            tasks.retain(|_, cell| cell.get().is_none());
            if let Some(cell) = tasks.get(&key) {
                (Arc::clone(cell), true)
            } else {
                let cell = Arc::new(tokio::sync::OnceCell::new());
                let registered = tasks.len() < 256;
                if registered {
                    tasks.insert(key.clone(), Arc::clone(&cell));
                }
                (cell, registered)
            }
        };
        trace_lsp_stage("analysis_inflight_lock_wait", inflight_lock_started);
        let analysis = inflight
            .get_or_init(|| async {
                trace_lsp_event("analysis_computation");
                let cancellation = Arc::new(CancellationSignal::new());
                let mut cancellation_on_drop = CancellationOnDrop::new(Arc::clone(&cancellation));
                if let Some(previous) = self
                    .analysis_cancellation
                    .write()
                    .await
                    .insert(uri.clone(), Arc::clone(&cancellation))
                {
                    previous.cancel();
                }
                let (analysis, import_behaviors) = self
                    .analyze_document(
                        uri,
                        source,
                        version,
                        context_generation,
                        Arc::clone(&cancellation),
                    )
                    .await;
                let mut running = self.analysis_cancellation.write().await;
                if running
                    .get(uri)
                    .is_some_and(|current| Arc::ptr_eq(current, &cancellation))
                {
                    running.remove(uri);
                }
                drop(running);
                if cancellation.is_cancelled() {
                    cancellation_on_drop.disarm();
                    trace_lsp_event("analysis_cancelled_before_cache");
                    return Arc::new(DocumentAnalysis::default());
                }
                trace_lsp_stage("semantic_analysis", analysis_started);
                let analysis = Arc::new(analysis);
                if self.document_matches(uri, source, version).await
                    && self
                        .context_generation
                        .load(std::sync::atomic::Ordering::Acquire)
                        == context_generation
                {
                    let mut cache = self.documents.write().await;
                    if self
                        .context_generation
                        .load(std::sync::atomic::Ordering::Acquire)
                        == context_generation
                    {
                        insert_cached_analysis(
                            &mut cache,
                            uri.clone(),
                            source,
                            version,
                            CachedAnalysis {
                                context_generation,
                                analysis: Arc::clone(&analysis),
                                line_index: Arc::new(LineIndex::new(source)),
                                import_behaviors: Arc::new(import_behaviors),
                                lint_diagnostics: None,
                                semantic_tokens: None,
                                call_parameters: std::collections::HashMap::new(),
                                inlay_hints: std::collections::HashMap::new(),
                            },
                        );
                    }
                }
                cancellation_on_drop.disarm();
                analysis
            })
            .await
            .clone();
        if registered {
            let mut tasks = self.analysis_inflight.write().await;
            if tasks
                .get(&key)
                .is_some_and(|current| Arc::ptr_eq(current, &inflight))
            {
                tasks.remove(&key);
            }
        }
        analysis
    }

    pub(super) async fn line_index_for(&self, uri: &Url, source: &str) -> Arc<LineIndex> {
        let version = self.document_version(uri).await;
        let context_generation = self
            .context_generation
            .load(std::sync::atomic::Ordering::Acquire);
        let cached = self
            .documents
            .read()
            .await
            .get(uri)
            .and_then(|document| document.cached_analysis(source, version, context_generation))
            .map(|cached| Arc::clone(&cached.line_index));
        if let Some(cached) = cached
            && self.document_matches(uri, source, version).await
            && self
                .context_generation
                .load(std::sync::atomic::Ordering::Acquire)
                == context_generation
        {
            return cached;
        }
        Arc::new(LineIndex::new(source))
    }

    pub(super) async fn import_behaviors_for(
        &self,
        uri: &Url,
        source: &str,
    ) -> Option<Arc<HashMap<String, tyrus_core::ast::BehaviorInterfaceInfo>>> {
        let version = self.document_version(uri).await;
        let context_generation = self
            .context_generation
            .load(std::sync::atomic::Ordering::Acquire);
        let behaviors = self
            .documents
            .read()
            .await
            .get(uri)
            .and_then(|document| document.cached_analysis(source, version, context_generation))
            .map(|cached| Arc::clone(&cached.import_behaviors));
        if self.document_matches(uri, source, version).await
            && self
                .context_generation
                .load(std::sync::atomic::Ordering::Acquire)
                == context_generation
        {
            behaviors
        } else {
            None
        }
    }

    pub(super) async fn cached_lint_diagnostics(
        &self,
        uri: &Url,
        source: &str,
    ) -> Option<Arc<Vec<tyrus_lint::LintDiagnostic>>> {
        let version = self.document_version(uri).await;
        let context_generation = self
            .context_generation
            .load(std::sync::atomic::Ordering::Acquire);
        let cached = self
            .documents
            .read()
            .await
            .get(uri)
            .and_then(|document| document.cached_analysis(source, version, context_generation))
            .and_then(|cached| cached.lint_diagnostics.as_ref().map(Arc::clone));
        if self.document_matches(uri, source, version).await
            && self
                .context_generation
                .load(std::sync::atomic::Ordering::Acquire)
                == context_generation
        {
            cached
        } else {
            None
        }
    }

    pub(super) async fn cached_semantic_tokens(
        &self,
        uri: &Url,
        source: &str,
    ) -> Option<Arc<Vec<SemanticToken>>> {
        let version = self.document_version(uri).await;
        let context_generation = self
            .context_generation
            .load(std::sync::atomic::Ordering::Acquire);
        let tokens = self
            .documents
            .read()
            .await
            .get(uri)
            .and_then(|document| document.cached_analysis(source, version, context_generation))
            .and_then(|cached| cached.semantic_tokens.as_ref().map(Arc::clone));
        if self.document_matches(uri, source, version).await
            && self
                .context_generation
                .load(std::sync::atomic::Ordering::Acquire)
                == context_generation
        {
            tokens
        } else {
            None
        }
    }

    pub(super) async fn store_semantic_tokens(
        &self,
        uri: &Url,
        source: &str,
        tokens: Arc<Vec<SemanticToken>>,
    ) {
        let version = self.document_version(uri).await;
        let context_generation = self
            .context_generation
            .load(std::sync::atomic::Ordering::Acquire);
        if !self.document_matches(uri, source, version).await {
            return;
        }
        let mut cache = self.documents.write().await;
        if self
            .context_generation
            .load(std::sync::atomic::Ordering::Acquire)
            == context_generation
            && let Some(cached) = cache.get_mut(uri).and_then(|document| {
                document.cached_analysis_mut(source, version, context_generation)
            })
        {
            cached.semantic_tokens = Some(tokens);
        }
    }

    pub(super) async fn lint_diagnostics_for(
        &self,
        uri: &Url,
        source: &str,
    ) -> Vec<tyrus_lint::LintDiagnostic> {
        if let Some(cached) = self.cached_lint_diagnostics(uri, source).await {
            trace_lsp_event("lint_cache_hit");
            return cached.as_ref().clone();
        }
        trace_lsp_event("lint_computation");
        let context_generation = self
            .context_generation
            .load(std::sync::atomic::Ordering::Acquire);
        let analysis = self.analysis_for(uri, source).await;
        let config = self.lint_config_for(uri).await;
        let diagnostics = Arc::new(tyrus_lint::lint_rules_source_with_analysis(
            source, &analysis, &config,
        ));
        let version = self.document_version(uri).await;
        if self.document_matches(uri, source, version).await {
            let mut cache = self.documents.write().await;
            if self
                .context_generation
                .load(std::sync::atomic::Ordering::Acquire)
                == context_generation
                && let Some(cached) = cache.get_mut(uri).and_then(|document| {
                    document.cached_analysis_mut(source, version, context_generation)
                })
            {
                cached.lint_diagnostics = Some(Arc::clone(&diagnostics));
            }
        }
        diagnostics.as_ref().clone()
    }

    pub(super) async fn document_matches(
        &self,
        uri: &Url,
        source: &str,
        version: Option<i32>,
    ) -> bool {
        document_snapshot::document_matches(self, uri, source, version).await
    }

    pub(super) async fn current_document_matches(&self, uri: &Url, source: &str) -> bool {
        let version = self.document_version(uri).await;
        self.document_matches(uri, source, version).await
    }

    pub(super) async fn db_schema_for_uri(
        &self,
        uri: &Url,
    ) -> std::result::Result<Arc<tyrus_core::db::DbSchemaSnapshot>, String> {
        let schema_started = std::time::Instant::now();
        let root = self
            .project_root_for(uri)
            .await
            .ok_or_else(|| "project root is unavailable".to_string())?;
        let config = self
            .project_database_config_for_root(&root)
            .await?
            .ok_or_else(|| "configure `database` in tyrus.json".to_string())?;
        let Ok(permit) = Arc::clone(&self.db_schema_semaphore).acquire_owned().await else {
            return Err("database schema worker is unavailable".to_string());
        };
        let cache = Arc::clone(&self.db_schema_cache);
        let result = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            cache
                .lock()
                .map_err(|_| "database schema cache is unavailable".to_string())?
                .load_configured(&root, &config)
        })
        .await
        .map_err(|_| "database schema worker failed".to_string())?;
        trace_lsp_stage("database_schema_cache", schema_started);
        result
    }

    pub(super) async fn db_backend_for_uri(
        &self,
        uri: &Url,
    ) -> std::result::Result<tyrus_core::db::DatabaseBackend, String> {
        let root = self
            .project_root_for(uri)
            .await
            .ok_or_else(|| "project root is unavailable".to_string())?;
        self.project_database_config_for_root(&root)
            .await?
            .map(|config| config.backend)
            .ok_or_else(|| "configure `database` in tyrus.json".to_string())
    }

    async fn project_database_config_for_root(
        &self,
        root: &Path,
    ) -> std::result::Result<Option<tyrus_core::db::DatabaseConfig>, String> {
        let config_started = std::time::Instant::now();
        let config_path = root.join("tyrus.json");
        let metadata = tokio::fs::metadata(&config_path).await.ok();
        let modified = metadata
            .as_ref()
            .and_then(|metadata| metadata.modified().ok());
        let len = metadata.as_ref().map_or(0, std::fs::Metadata::len);
        let cached_config = self
            .project_database_config_cache
            .read()
            .ok()
            .as_ref()
            .and_then(|cache| cache.get(root))
            .filter(|entry| entry.modified == modified && entry.len == len)
            .map(|entry| entry.config.clone());
        if let Some(config) = cached_config {
            trace_lsp_stage("project_database_config_cache", config_started);
            self.configure_schema_file_watcher(root, config.as_ref().ok().and_then(Option::as_ref))
                .await;
            return config;
        }
        let config_load_started = std::time::Instant::now();
        let Ok(permit) = Arc::clone(&self.project_io_semaphore).acquire_owned().await else {
            return Err("project configuration worker is unavailable".to_string());
        };
        let root = root.to_path_buf();
        let root_for_load = root.clone();
        let root_for_cache = root.clone();
        let config_cache = Arc::clone(&self.project_database_config_cache);
        let config = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let config =
                tyrus_core::project::load_project(&root_for_load).map(|project| project.database);
            let entry_weight = estimated_config_cache_entry_weight(&root_for_cache, len);
            if config.is_ok()
                && entry_weight <= MAX_CACHED_CONFIG_ENTRY_WEIGHT
                && let Ok(mut cache) = config_cache.write()
            {
                evict_config_cache_entries(
                    &mut cache,
                    &root_for_cache,
                    entry_weight,
                    |path, entry| estimated_config_cache_entry_weight(path, entry.len),
                );
                cache.insert(
                    root_for_cache,
                    ProjectDatabaseConfigCacheEntry {
                        modified,
                        len,
                        config: config.clone(),
                    },
                );
            }
            config
        })
        .await
        .map_err(|_| "project configuration worker failed".to_string())?;
        trace_lsp_event(if config.is_ok() {
            "project_database_config_valid"
        } else {
            "project_database_config_invalid"
        });
        trace_lsp_stage("project_database_config_load", config_load_started);
        trace_lsp_stage("project_database_config_cache", config_started);
        self.configure_schema_file_watcher(&root, config.as_ref().ok().and_then(Option::as_ref))
            .await;
        config
    }

    async fn configure_schema_file_watcher(
        &self,
        root: &Path,
        config: Option<&tyrus_core::db::DatabaseConfig>,
    ) {
        if !self
            .watched_files_dynamic_registration
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return;
        }
        let root = root.to_path_buf();
        let Some(root_uri) = Url::from_directory_path(&root).ok() else {
            trace_lsp_event("database_schema_watcher_invalid_root");
            return;
        };
        let next_schema = config.map(|config| config.schema.clone());
        trace_lsp_event(if next_schema.is_some() {
            "database_schema_watcher_configured"
        } else {
            "database_schema_watcher_unconfigured"
        });
        let next_path = next_schema.as_ref().map(|schema| root.join(schema));
        let (previous, next) = {
            let mut watchers = self.schema_file_watchers.lock().await;
            let current = watchers.get(&root).cloned();
            if current.as_ref().map(|(path, _)| path) == next_path.as_ref() {
                trace_lsp_event("database_schema_watcher_unchanged");
                return;
            }
            let mut previous = watchers.remove(&root).into_iter().collect::<Vec<_>>();
            let next = next_schema.zip(next_path).map(|(schema, path)| {
                if watchers.len() >= MAX_SCHEMA_FILE_WATCHERS
                    && let Some(evicted_root) = watchers.keys().next().cloned()
                    && let Some(evicted) = watchers.remove(&evicted_root)
                {
                    previous.push(evicted);
                }
                let registration_id = format!(
                    "tyrus.database-schema.{}",
                    NEXT_SCHEMA_WATCHER_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                );
                watchers.insert(root.clone(), (path, registration_id.clone()));
                (schema, registration_id)
            });
            (previous, next)
        };
        let client = self.client.clone();
        let watchers = Arc::clone(&self.schema_file_watchers);
        tokio::spawn(async move {
            for (_, registration_id) in previous {
                let _ = client
                    .unregister_capability(vec![Unregistration {
                        id: registration_id,
                        method: "workspace/didChangeWatchedFiles".to_string(),
                    }])
                    .await;
            }
            let Some((schema, registration_id)) = next else {
                return;
            };
            let registration = Registration {
                id: registration_id.clone(),
                method: "workspace/didChangeWatchedFiles".to_string(),
                register_options: serde_json::to_value(DidChangeWatchedFilesRegistrationOptions {
                    watchers: vec![FileSystemWatcher {
                        glob_pattern: GlobPattern::Relative(RelativePattern {
                            base_uri: OneOf::Right(root_uri),
                            pattern: schema.replace('\\', "/"),
                        }),
                        kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
                    }],
                })
                .ok(),
            };
            if let Err(error) = client.register_capability(vec![registration]).await {
                trace_lsp_event("database_schema_watcher_registration_failed");
                let mut watchers = watchers.lock().await;
                if watchers
                    .get(&root)
                    .is_some_and(|(_, current_id)| current_id == &registration_id)
                {
                    watchers.remove(&root);
                }
                drop(watchers);
                client
                    .log_message(
                        MessageType::WARNING,
                        format!("Could not register database schema watcher: {error}"),
                    )
                    .await;
            } else {
                trace_lsp_event("database_schema_watcher_registered");
                let is_current = watchers
                    .lock()
                    .await
                    .get(&root)
                    .is_some_and(|(_, current_id)| current_id == &registration_id);
                if !is_current {
                    let _ = client
                        .unregister_capability(vec![Unregistration {
                            id: registration_id,
                            method: "workspace/didChangeWatchedFiles".to_string(),
                        }])
                        .await;
                }
            }
        });
    }

    pub(super) async fn import_map_for_root(
        &self,
        root: &Path,
    ) -> std::result::Result<tyrus_core::project::ImportMap, String> {
        let import_map_started = std::time::Instant::now();
        let config_path = root.join("tyrus.json");
        let metadata = tokio::fs::metadata(&config_path).await.ok();
        let modified = metadata
            .as_ref()
            .and_then(|metadata| metadata.modified().ok());
        let len = metadata.as_ref().map_or(0, std::fs::Metadata::len);
        if let Some(import_map) = self
            .import_map_cache
            .read()
            .ok()
            .as_ref()
            .and_then(|cache| cache.get(root))
            .filter(|entry| entry.modified == modified && entry.len == len)
            .map(|entry| entry.import_map.clone())
        {
            trace_lsp_stage("import_map_cache", import_map_started);
            return import_map;
        }
        let import_map_load_started = std::time::Instant::now();
        let Ok(permit) = Arc::clone(&self.project_io_semaphore).acquire_owned().await else {
            return Err("project import map worker is unavailable".to_string());
        };
        let root = root.to_path_buf();
        let import_map_cache = Arc::clone(&self.import_map_cache);
        let import_map = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let import_map = tyrus_core::project::ImportMap::load(&root);
            let entry_weight = estimated_config_cache_entry_weight(&root, len);
            if import_map.is_ok()
                && entry_weight <= MAX_CACHED_CONFIG_ENTRY_WEIGHT
                && let Ok(mut cache) = import_map_cache.write()
            {
                evict_config_cache_entries(&mut cache, &root, entry_weight, |path, entry| {
                    estimated_config_cache_entry_weight(path, entry.len)
                });
                cache.insert(
                    root,
                    ImportMapCacheEntry {
                        modified,
                        len,
                        import_map: import_map.clone(),
                    },
                );
            }
            import_map
        })
        .await
        .map_err(|_| "project import map worker failed".to_string())?;
        trace_lsp_stage("import_map_load", import_map_load_started);
        trace_lsp_stage("import_map_cache", import_map_started);
        import_map
    }

    pub(super) async fn publish_analysis(&self, uri: Url, source: &str, version: Option<i32>) {
        let publication_started = std::time::Instant::now();
        let current_version = self.document_version(&uri).await;
        if version.is_some_and(|version| Some(version) != current_version)
            || !self.document_matches(&uri, source, current_version).await
        {
            return;
        }
        let cancellation = Arc::new(CancellationSignal::new());
        if let Some(previous) = self
            .diagnostic_cancellation
            .write()
            .await
            .insert(uri.clone(), Arc::clone(&cancellation))
        {
            previous.cancel();
        }
        let background_queue_started = std::time::Instant::now();
        let Some(_background_permit) = acquire_cancellable_permit(
            Arc::clone(&self.background_analysis_semaphore),
            Arc::clone(&cancellation),
        )
        .await
        else {
            return;
        };
        trace_lsp_stage("background_analysis_queue", background_queue_started);
        let context_generation = self
            .context_generation
            .load(std::sync::atomic::Ordering::Acquire);
        let analysis = self.analysis_for(&uri, source).await;
        let current_version = self.document_version(&uri).await;
        if version.is_some_and(|version| Some(version) != current_version)
            || !self.document_matches(&uri, source, current_version).await
            || self
                .context_generation
                .load(std::sync::atomic::Ordering::Acquire)
                != context_generation
            || cancellation.is_cancelled()
        {
            return;
        }
        let import_behaviors = self
            .import_behaviors_for(&uri, source)
            .await
            .map(|behaviors| behaviors.as_ref().clone())
            .unwrap_or_default();
        let diagnostic_conversion_started = std::time::Instant::now();
        let mut diagnostics = analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| !is_lint_owned_analysis_diagnostic(diagnostic))
            .map(|diagnostic| {
                let mut lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
                attach_import_alias_related_information(
                    &mut lsp_diagnostic,
                    &uri,
                    source,
                    diagnostic,
                );
                lsp_diagnostic
            })
            .collect::<Vec<_>>();
        trace_lsp_stage(
            "analysis_diagnostics_conversion",
            diagnostic_conversion_started,
        );
        let lint_config = self.lint_config_for(&uri).await;
        let lint_source = source.to_string();
        let lint_analysis = Arc::clone(&analysis);
        let lint_started = std::time::Instant::now();
        let lint_worker_queue_started = std::time::Instant::now();
        if cancellation.is_cancelled()
            || self
                .context_generation
                .load(std::sync::atomic::Ordering::Acquire)
                != context_generation
        {
            return;
        }
        let Some(permit) = acquire_cancellable_permit(
            Arc::clone(&self.analysis_semaphore),
            Arc::clone(&cancellation),
        )
        .await
        else {
            return;
        };
        trace_lsp_stage("lint_worker_queue", lint_worker_queue_started);
        let lint_cancellation = Arc::clone(&cancellation.cancelled);
        let lint_diagnostics = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            tyrus_lint::lint_rules_source_with_analysis_cancellable(
                &lint_source,
                &lint_analysis,
                &lint_config,
                lint_cancellation,
            )
        })
        .await
        .unwrap_or(None);
        let Some(lint_diagnostics) = lint_diagnostics else {
            return;
        };
        let lint_diagnostics = Arc::new(lint_diagnostics);
        trace_lsp_stage("lint", lint_started);
        let lint_conversion_started = std::time::Instant::now();
        diagnostics.extend(
            lint_diagnostics
                .iter()
                .map(|diagnostic| lint_diagnostic_to_lsp(source, diagnostic)),
        );
        trace_lsp_stage("lint_diagnostics_conversion", lint_conversion_started);
        let current_version = self.document_version(&uri).await;
        if version.is_some_and(|version| Some(version) != current_version)
            || !self.document_matches(&uri, source, current_version).await
            || self
                .context_generation
                .load(std::sync::atomic::Ordering::Acquire)
                != context_generation
            || cancellation.is_cancelled()
        {
            return;
        }
        let line_index = self.line_index_for(&uri, source).await;
        let published_diagnostics = diagnostics.clone();
        let diagnostics_lock_wait_started = std::time::Instant::now();
        let mut cache = self.documents.write().await;
        trace_lsp_stage("diagnostics_cache_lock_wait", diagnostics_lock_wait_started);
        let diagnostics_lock_hold_started = std::time::Instant::now();
        if self
            .context_generation
            .load(std::sync::atomic::Ordering::Acquire)
            != context_generation
            || cancellation.is_cancelled()
        {
            drop(cache);
            trace_lsp_stage("diagnostics_cache_lock_hold", diagnostics_lock_hold_started);
            return;
        }
        if !cache.get(&uri).is_some_and(|document| {
            document.text == source && Some(document.version) == current_version
        }) {
            drop(cache);
            trace_lsp_stage("diagnostics_cache_lock_hold", diagnostics_lock_hold_started);
            return;
        }
        insert_cached_analysis(
            &mut cache,
            uri.clone(),
            source,
            current_version,
            CachedAnalysis {
                context_generation,
                analysis,
                line_index,
                import_behaviors: Arc::new(import_behaviors),
                lint_diagnostics: Some(lint_diagnostics),
                semantic_tokens: None,
                call_parameters: std::collections::HashMap::new(),
                inlay_hints: std::collections::HashMap::new(),
            },
        );
        if let Some(document) = cache.get_mut(&uri)
            && document.text == source
            && Some(document.version) == current_version
        {
            document.published_diagnostics = published_diagnostics;
        }
        drop(cache);
        trace_lsp_stage("diagnostics_cache_lock_hold", diagnostics_lock_hold_started);
        if cancellation.is_cancelled()
            || self
                .context_generation
                .load(std::sync::atomic::Ordering::Acquire)
                != context_generation
        {
            return;
        }
        self.client
            .publish_diagnostics(uri, diagnostics, version)
            .await;
        trace_lsp_stage("diagnostics_publication", publication_started);
    }

    pub(super) async fn lint_config_for(&self, uri: &Url) -> tyrus_lint::LintConfig {
        let workspace_root = self.workspace_root.read().await.clone();
        let directory = self
            .project_root_for(uri)
            .await
            .or(workspace_root)
            .or_else(|| {
                uri.to_file_path()
                    .ok()
                    .and_then(|path| path.parent().map(Path::to_path_buf))
            });
        let Some(directory) = directory else {
            return tyrus_lint::LintConfig::default();
        };
        let config_path = directory.join("tyrus-lint.json");
        let metadata = tokio::fs::metadata(&config_path).await.ok();
        let modified = metadata
            .as_ref()
            .and_then(|metadata| metadata.modified().ok());
        let len = metadata.as_ref().map_or(0, std::fs::Metadata::len);
        if let Some(config) = self
            .lint_config_cache
            .read()
            .ok()
            .as_ref()
            .and_then(|cache| cache.get(&config_path))
            .filter(|entry| entry.modified == modified && entry.len == len)
            .map(|entry| entry.config.clone())
        {
            return config;
        }
        let Ok(permit) = Arc::clone(&self.project_io_semaphore).acquire_owned().await else {
            return tyrus_lint::LintConfig::default();
        };
        let directory = directory.to_path_buf();
        let cache = Arc::clone(&self.lint_config_cache);
        let cache_path = config_path.clone();
        let loaded = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let loaded = tyrus_lint::LintConfig::load(&directory);
            if let Ok(config) = &loaded
                && let Ok(mut cache) = cache.write()
                && estimated_config_cache_entry_weight(&cache_path, len)
                    <= MAX_CACHED_CONFIG_ENTRY_WEIGHT
            {
                let entry_weight = estimated_config_cache_entry_weight(&cache_path, len);
                evict_config_cache_entries(&mut cache, &cache_path, entry_weight, |path, entry| {
                    estimated_config_cache_entry_weight(path, entry.len)
                });
                cache.insert(
                    cache_path,
                    LintConfigCacheEntry {
                        modified,
                        len,
                        config: (*config).clone(),
                    },
                );
            }
            loaded
        })
        .await;
        match loaded {
            Ok(Ok(config)) => config,
            Ok(Err(error)) => {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        format!("{error}; using default lint configuration"),
                    )
                    .await;
                tyrus_lint::LintConfig::default()
            }
            Err(error) => {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        format!("Could not load lint configuration: {error}; using default lint configuration"),
                    )
                    .await;
                tyrus_lint::LintConfig::default()
            }
        }
    }

    pub(super) async fn call_parameter_names(
        &self,
        document_uri: &Url,
        analysis: &DocumentAnalysis,
        callee: &str,
    ) -> Option<Vec<String>> {
        if let Some(parameters) = self
            .documents
            .read()
            .await
            .get(document_uri)
            .and_then(|document| document.analysis.as_ref())
            .filter(|cached| std::ptr::eq(cached.analysis.as_ref(), analysis))
            .and_then(|cached| cached.call_parameters.get(callee))
        {
            return parameters.clone();
        }
        let parameters = self
            .call_signature(document_uri, analysis, callee)
            .await
            .map(|signature| signature.parameters);
        let mut documents = self.documents.write().await;
        if let Some(cached) = documents
            .get_mut(document_uri)
            .and_then(|document| document.analysis.as_mut())
            .filter(|cached| std::ptr::eq(cached.analysis.as_ref(), analysis))
        {
            cache_call_parameters(
                &mut cached.call_parameters,
                callee.to_string(),
                parameters.clone(),
            );
        }
        parameters
    }

    pub(super) async fn call_signature(
        &self,
        document_uri: &Url,
        analysis: &DocumentAnalysis,
        callee: &str,
    ) -> Option<CallSignature> {
        self.call_signature_with_receiver(document_uri, analysis, callee, None)
            .await
    }

    pub(super) async fn call_signature_with_receiver(
        &self,
        document_uri: &Url,
        analysis: &DocumentAnalysis,
        callee: &str,
        receiver: Option<&str>,
    ) -> Option<CallSignature> {
        if let Some(signature) = receiver
            .and_then(|receiver| receiver_semantic_type(analysis, receiver))
            .and_then(|ty| tasks_instance_call_signature(&ty, callee))
        {
            return Some(signature);
        }
        if let Some(signature) = receiver
            .and_then(|receiver| receiver_semantic_type(analysis, receiver))
            .and_then(|ty| automatic_data_call_signature(&ty, callee))
        {
            return Some(signature);
        }
        if let Some(signature) = receiver
            .and_then(|receiver| automatic_data_static_call_signature(analysis, receiver, callee))
        {
            return Some(signature);
        }
        if analysis.symbol_named(callee).is_none()
            && let Some(signature) = global_builtin_call_signature(callee)
        {
            return Some(signature);
        }
        if let Some(signature) = native_promise_call_signature(callee) {
            return Some(signature);
        }
        if let Some(signature) = native_crypto_call_signature(analysis, callee) {
            return Some(signature);
        }
        let symbol = analysis.symbol_named(callee)?;
        if symbol.kind != SymbolKind::Import {
            return (symbol.kind == SymbolKind::Function).then(|| CallSignature {
                parameters: parameter_names(symbol),
                detail: Some(symbol.ty.clone()),
            });
        }

        let imported = symbol.imported_name.as_deref().unwrap_or(&symbol.name);
        if let Some(parameters) = symbol
            .import_source
            .as_deref()
            .and_then(|module| native_parameter_names(module, imported))
        {
            return Some(CallSignature {
                parameters,
                detail: symbol
                    .import_source
                    .as_deref()
                    .and_then(|module| native_import_detail(module, imported))
                    .map(str::to_string),
            });
        }
        let location = self.imported_definition(document_uri, symbol).await?;
        let target_source = self.source(&location.uri).await?;
        self.workspace_analysis_for(&location.uri, &target_source)
            .await
            .symbol_named(imported)
            .filter(|target| target.kind == SymbolKind::Function)
            .map(|target| CallSignature {
                parameters: parameter_names(target),
                detail: Some(target.ty.clone()),
            })
    }

    pub(super) async fn acquire_diagnostic_task_permit(
        &self,
    ) -> Option<tokio::sync::OwnedSemaphorePermit> {
        let started = std::time::Instant::now();
        let permit = self
            .diagnostic_task_semaphore
            .clone()
            .acquire_owned()
            .await
            .ok();
        trace_lsp_stage("diagnostic_queue_wait", started);
        permit
    }
}
