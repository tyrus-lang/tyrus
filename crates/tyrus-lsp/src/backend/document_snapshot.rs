pub(super) fn document_snapshot_matches(
    source: &str,
    version: Option<i32>,
    current_source: Option<&str>,
    current_version: Option<i32>,
) -> bool {
    match current_source {
        Some(current_source) => current_source == source && current_version == version,
        None => version.is_none(),
    }
}

pub(super) async fn document_matches(
    backend: &super::Backend,
    uri: &tower_lsp::lsp_types::Url,
    source: &str,
    version: Option<i32>,
) -> bool {
    let lock_started = std::time::Instant::now();
    let current = backend.documents.read().await.get(uri).cloned();
    crate::trace_lsp_stage("document_snapshot_lock_wait", lock_started);
    document_snapshot_matches(
        source,
        version,
        current.as_ref().map(|document| document.text.as_str()),
        current.as_ref().map(|document| document.version),
    )
}

pub(super) async fn analysis_snapshot_is_current(
    backend: &super::Backend,
    uri: &tower_lsp::lsp_types::Url,
    source: &str,
    version: Option<i32>,
    context_generation: u64,
) -> bool {
    let current_document = backend.document_matches(uri, source, version).await;
    let current_context = backend
        .context_generation
        .load(std::sync::atomic::Ordering::Acquire)
        == context_generation;
    if !current_document || !current_context {
        crate::trace_lsp_event("analysis_stale_checkpoint");
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::document_snapshot_matches;

    #[test]
    fn interactive_results_require_the_current_document_snapshot() {
        assert!(document_snapshot_matches(
            "old",
            Some(1),
            Some("old"),
            Some(1)
        ));
        assert!(!document_snapshot_matches(
            "old",
            Some(1),
            Some("new"),
            Some(2)
        ));
        assert!(!document_snapshot_matches(
            "same",
            Some(1),
            Some("same"),
            Some(2)
        ));
        assert!(document_snapshot_matches("closed", None, None, None));
        assert!(!document_snapshot_matches("closed", Some(1), None, None));
    }
}
