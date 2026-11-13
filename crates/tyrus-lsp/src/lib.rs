use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::AtomicU64;
use tokio::sync::{Notify, RwLock};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tyrus_core::analysis::{
    AnalysisDiagnostic, AnalyzeContext, DiagnosticSeverity, DocumentAnalysis, Symbol, SymbolKind,
    analyze, resolve_import_path,
};
use tyrus_core::ast::{Expr, Field, Item, Stmt, Type};
use tyrus_core::parser::Parser;

static LSP_TRACE_ENABLED: OnceLock<bool> = OnceLock::new();

pub(crate) fn trace_lsp_enabled() -> bool {
    *LSP_TRACE_ENABLED.get_or_init(|| std::env::var_os("TYRUS_LSP_TRACE").is_some())
}

pub(crate) fn trace_lsp_stage(stage: &str, started: std::time::Instant) {
    if trace_lsp_enabled() {
        eprintln!(
            "tyrus-lsp trace stage={stage} duration_ms={:.3}",
            started.elapsed().as_secs_f64() * 1000.0
        );
    }
}

pub(crate) fn trace_lsp_event(event: &str) {
    if trace_lsp_enabled() {
        eprintln!("tyrus-lsp trace event={event}");
    }
}

pub(crate) fn trace_lsp_count(stage: &str, count: usize) {
    if trace_lsp_enabled() {
        eprintln!("tyrus-lsp trace count={stage} value={count}");
    }
}

pub(crate) fn trace_lsp_duration(stage: &str, duration: std::time::Duration) {
    if trace_lsp_enabled() {
        eprintln!(
            "tyrus-lsp trace stage={stage} duration_ms={:.3}",
            duration.as_secs_f64() * 1000.0
        );
    }
}

pub(crate) struct LspTraceSpan {
    stage: &'static str,
    started: std::time::Instant,
}

impl Drop for LspTraceSpan {
    fn drop(&mut self) {
        trace_lsp_stage(self.stage, self.started);
    }
}

pub(crate) fn trace_lsp_span(stage: &'static str) -> Option<LspTraceSpan> {
    trace_lsp_enabled().then(|| LspTraceSpan {
        stage,
        started: std::time::Instant::now(),
    })
}

struct TracedLspService<S> {
    inner: S,
}

impl<S> tower::Service<tower_lsp::jsonrpc::Request> for TracedLspService<S>
where
    S: tower::Service<
            tower_lsp::jsonrpc::Request,
            Response = Option<tower_lsp::jsonrpc::Response>,
            Error = tower_lsp::ExitedError,
        > + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Response = Option<tower_lsp::jsonrpc::Response>;
    type Error = tower_lsp::ExitedError;
    type Future = std::pin::Pin<
        Box<
            dyn std::future::Future<Output = std::result::Result<Self::Response, Self::Error>>
                + Send
                + 'static,
        >,
    >;

    fn poll_ready(
        &mut self,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), Self::Error>> {
        self.inner.poll_ready(context)
    }

    fn call(&mut self, request: tower_lsp::jsonrpc::Request) -> Self::Future {
        let started = std::time::Instant::now();
        let method = request.method().to_owned();
        let future = self.inner.call(request);
        Box::pin(async move {
            let result = future.await;
            if trace_lsp_enabled() {
                trace_lsp_stage(&format!("request_dispatch_{method}"), started);
            }
            if result.as_ref().is_ok_and(|response| response.is_some()) {
                trace_lsp_count("protocol_responses", 1);
            }
            result
        })
    }
}

#[path = "backend/backend.rs"]
mod backend;
#[path = "completion/completion.rs"]
mod completion;
#[path = "imports/imports.rs"]
mod imports;
#[path = "native/native.rs"]
mod native;
#[path = "positions/positions.rs"]
mod positions;
#[path = "backend/project_analysis.rs"]
mod project_analysis;
#[path = "server/server.rs"]
mod server;
#[path = "sql/sql.rs"]
mod sql;
#[path = "workspace/workspace.rs"]
mod workspace;
#[path = "backend/workspace_index.rs"]
mod workspace_index;

use completion::*;
use imports::*;
use native::*;
use positions::*;
use sql::*;
use workspace::*;

type AnalysisKey = (Url, Option<i32>, u64, String);
type InflightAnalysis = Arc<tokio::sync::OnceCell<Arc<DocumentAnalysis>>>;
type WorkspaceAnalysisEntries = HashMap<Url, (String, Arc<DocumentAnalysis>)>;
type WorkspaceOccurrenceKey = (Url, String, bool);
type WorkspaceOccurrence = (Url, Vec<std::ops::Range<usize>>);
type WorkspaceOccurrenceEntries = HashMap<WorkspaceOccurrenceKey, Vec<WorkspaceOccurrence>>;
type WorkspaceOccurrenceCache<S> = HashMap<WorkspaceOccurrenceKey, Vec<WorkspaceOccurrence>, S>;
const ANALYSIS_WORKER_LIMIT: usize = 4;
const BACKGROUND_ANALYSIS_WORKER_LIMIT: usize = ANALYSIS_WORKER_LIMIT - 1;
const MAX_PENDING_DIAGNOSTIC_TASKS: usize = 256;
const MAX_CACHED_CALL_PARAMETERS: usize = 256;
pub(crate) const MAX_CACHED_CONFIG_ENTRY_WEIGHT: usize = 512 * 1024;
pub(crate) const MAX_CACHED_CONFIG_WEIGHT: usize = 4 * 1024 * 1024;
const MAX_CACHED_CONFIG_ENTRIES: usize = 32;
const MAX_SCHEMA_FILE_WATCHERS: usize = 128;
static NEXT_SCHEMA_WATCHER_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn estimated_config_cache_entry_weight(path: &std::path::Path, len: u64) -> usize {
    std::mem::size_of::<PathBuf>()
        .saturating_add(path.as_os_str().len())
        .saturating_add(usize::try_from(len).unwrap_or(usize::MAX))
}

pub(crate) fn evict_config_cache_entries<T>(
    cache: &mut HashMap<PathBuf, T>,
    replacement_path: &Path,
    entry_weight: usize,
    mut cached_entry_weight: impl FnMut(&Path, &T) -> usize,
) -> usize {
    let mut cached_weight = cache.iter().fold(0usize, |total, (path, entry)| {
        total.saturating_add(cached_entry_weight(path, entry))
    });
    cached_weight = cached_weight.saturating_sub(
        cache
            .get(replacement_path)
            .map_or(0, |entry| cached_entry_weight(replacement_path, entry)),
    );
    let mut projected_count = cache.len() + usize::from(!cache.contains_key(replacement_path));
    let mut evicted = 0;
    while projected_count > MAX_CACHED_CONFIG_ENTRIES
        || cached_weight.saturating_add(entry_weight) > MAX_CACHED_CONFIG_WEIGHT
    {
        let Some(victim_path) = cache
            .keys()
            .filter(|path| path.as_path() != replacement_path)
            .min()
            .cloned()
        else {
            break;
        };
        if let Some(victim) = cache.remove(&victim_path) {
            cached_weight =
                cached_weight.saturating_sub(cached_entry_weight(&victim_path, &victim));
            projected_count = projected_count.saturating_sub(1);
            evicted += 1;
        }
    }
    evicted
}

pub(crate) struct CancellationSignal {
    cancelled: Arc<std::sync::atomic::AtomicBool>,
    notify: Notify,
}

impl CancellationSignal {
    fn new() -> Self {
        Self {
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            notify: Notify::new(),
        }
    }

    fn cancel(&self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::Release);
        self.notify.notify_one();
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::Acquire)
    }
}

pub(crate) struct CancellationOnDrop {
    signal: Arc<CancellationSignal>,
    armed: bool,
}

impl CancellationOnDrop {
    pub(crate) fn new(signal: Arc<CancellationSignal>) -> Self {
        Self {
            signal,
            armed: true,
        }
    }

    pub(crate) fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for CancellationOnDrop {
    fn drop(&mut self) {
        if self.armed {
            self.signal.cancel();
            trace_lsp_event("analysis_abandoned");
        }
    }
}

#[derive(Debug, Clone)]
struct IndexedExport {
    name: String,
    is_default: bool,
    module_source: Option<String>,
    ty: String,
    parameters: Vec<String>,
    kind: SymbolKind,
    uri: Url,
    definition: Option<Location>,
    is_protected: bool,
    metadata: Arc<IndexedExportMetadata>,
}

#[derive(Debug, Clone)]
enum IndexedExportMetadata {
    Function {
        is_validator: bool,
        type_params: Vec<tyrus_core::ast::TypeParam>,
        parameters: Vec<IndexedParameter>,
        return_type: tyrus_core::ast::Type,
        throws: Vec<tyrus_core::ast::Type>,
    },
    Interface {
        type_params: Vec<tyrus_core::ast::TypeParam>,
        extends: Vec<tyrus_core::ast::Type>,
        fields: Vec<tyrus_core::ast::Field>,
        db_model: Option<tyrus_core::ast::DbModelMetadata>,
    },
    Behavior {
        type_params: Vec<tyrus_core::ast::TypeParam>,
        targets: Vec<tyrus_core::ast::Type>,
        extends: Vec<tyrus_core::ast::Type>,
        methods: Vec<IndexedBehaviorMethod>,
        bindings: Vec<String>,
    },
    TypeAlias {
        type_params: Vec<tyrus_core::ast::TypeParam>,
        ty: tyrus_core::ast::Type,
        merged_fields: Vec<tyrus_core::ast::Field>,
    },
    Enum {
        repr: tyrus_core::ast::Type,
        members: Vec<tyrus_core::ast::EnumMember>,
    },
    Value {
        ty: Option<tyrus_core::ast::Type>,
    },
}

#[derive(Debug, Clone)]
struct IndexedParameter {
    pattern: tyrus_core::ast::ParamPattern,
    ty: tyrus_core::ast::Type,
    optional: bool,
    rest: bool,
    validators: Vec<tyrus_core::ast::ValidatorDecorator>,
}

#[derive(Debug, Clone)]
struct IndexedBehaviorMethod {
    name: String,
    type_params: Vec<tyrus_core::ast::TypeParam>,
    parameters: Vec<IndexedParameter>,
    return_type: tyrus_core::ast::Type,
    throws: Vec<tyrus_core::ast::Type>,
}

struct WorkspaceExportCacheEntry {
    generation: WorkspaceGeneration,
    estimated_weight: usize,
    exports: Vec<IndexedExport>,
    export_name_order: Vec<usize>,
    interfaces: Vec<backend::ObjectInterfaceCandidate>,
    resolved_imports: HashMap<(Url, String), Url>,
    module_fingerprints: HashMap<Url, WorkspaceModuleFingerprint>,
}

struct WorkspaceModuleFingerprint {
    canonical_path: PathBuf,
    content: u64,
}

struct WorkspaceFilesCacheEntry {
    generation: WorkspaceGeneration,
    files: Vec<PathBuf>,
}

struct WorkspaceDefinitionCacheEntry {
    generation: WorkspaceGeneration,
    location: Option<Location>,
    dependencies: HashSet<Url>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct WorkspaceGeneration {
    global: u64,
    root: u64,
}

#[derive(Clone)]
struct DocumentState {
    text: String,
    version: i32,
    analysis: Option<CachedAnalysis>,
    published_diagnostics: Vec<Diagnostic>,
}

impl DocumentState {
    fn cached_analysis(
        &self,
        source: &str,
        version: Option<i32>,
        context_generation: u64,
    ) -> Option<&CachedAnalysis> {
        (self.text == source && Some(self.version) == version)
            .then_some(self.analysis.as_ref())
            .flatten()
            .filter(|analysis| analysis.context_generation == context_generation)
    }

    fn cached_analysis_mut(
        &mut self,
        source: &str,
        version: Option<i32>,
        context_generation: u64,
    ) -> Option<&mut CachedAnalysis> {
        if self.text != source
            || Some(self.version) != version
            || self
                .analysis
                .as_ref()
                .is_none_or(|analysis| analysis.context_generation != context_generation)
        {
            return None;
        }
        self.analysis.as_mut()
    }
}

#[derive(Clone)]
struct Backend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, DocumentState>>>,
    workspace_analysis_cache: Arc<RwLock<WorkspaceAnalysisEntries>>,
    import_dependencies: Arc<RwLock<HashMap<Url, HashSet<Url>>>>,
    workspace_definitions_cache: Arc<RwLock<HashMap<(Url, String), WorkspaceDefinitionCacheEntry>>>,
    workspace_occurrences_cache: Arc<RwLock<WorkspaceOccurrenceEntries>>,
    workspace_exports_cache: Arc<RwLock<HashMap<PathBuf, WorkspaceExportCacheEntry>>>,
    workspace_files_cache: Arc<RwLock<HashMap<PathBuf, WorkspaceFilesCacheEntry>>>,
    workspace_index_generation: Arc<AtomicU64>,
    workspace_root_generations: Arc<RwLock<HashMap<PathBuf, u64>>>,
    workspace_generation_counter: Arc<AtomicU64>,
    workspace_index_semaphore: Arc<tokio::sync::Semaphore>,
    workspace_scan_max_files: Arc<std::sync::atomic::AtomicUsize>,
    workspace_scan_max_depth: Arc<std::sync::atomic::AtomicUsize>,
    workspace_scan_excluded_directories: Arc<RwLock<HashSet<String>>>,
    analysis_inflight: Arc<RwLock<HashMap<AnalysisKey, InflightAnalysis>>>,
    analysis_cancellation: Arc<RwLock<HashMap<Url, Arc<CancellationSignal>>>>,
    diagnostic_cancellation: Arc<RwLock<HashMap<Url, Arc<CancellationSignal>>>>,
    pending_diagnostics: Arc<RwLock<HashMap<Url, tokio::task::AbortHandle>>>,
    diagnostic_task_semaphore: Arc<tokio::sync::Semaphore>,
    context_generation: Arc<AtomicU64>,
    analysis_semaphore: Arc<tokio::sync::Semaphore>,
    background_analysis_semaphore: Arc<tokio::sync::Semaphore>,
    diagnostic_debounce_ms: Arc<AtomicU64>,
    formatting_semaphore: Arc<tokio::sync::Semaphore>,
    quick_fix_semaphore: Arc<tokio::sync::Semaphore>,
    format_config_cache: Arc<std::sync::RwLock<HashMap<PathBuf, FormatConfigCacheEntry>>>,
    lint_config_cache: Arc<std::sync::RwLock<HashMap<PathBuf, LintConfigCacheEntry>>>,
    import_map_cache: Arc<std::sync::RwLock<HashMap<PathBuf, ImportMapCacheEntry>>>,
    project_database_config_cache:
        Arc<std::sync::RwLock<HashMap<PathBuf, ProjectDatabaseConfigCacheEntry>>>,
    schema_file_watchers: Arc<tokio::sync::Mutex<HashMap<PathBuf, (PathBuf, String)>>>,
    db_schema_cache: Arc<std::sync::Mutex<tyrus_core::db::DbSchemaCache>>,
    db_schema_semaphore: Arc<tokio::sync::Semaphore>,
    project_io_semaphore: Arc<tokio::sync::Semaphore>,
    workspace_root: Arc<RwLock<Option<PathBuf>>>,
    workspace_roots: Arc<RwLock<Vec<PathBuf>>>,
    workspace_roots_generation: Arc<AtomicU64>,
    project_roots: Arc<RwLock<HashMap<Url, (u64, PathBuf)>>>,
    watched_files_dynamic_registration: Arc<std::sync::atomic::AtomicBool>,
    env_state: Arc<RwLock<Option<EnvState>>>,
    env_config: Arc<RwLock<Option<EnvConfig>>>,
}

#[derive(Clone)]
struct CachedAnalysis {
    context_generation: u64,
    analysis: Arc<DocumentAnalysis>,
    line_index: Arc<LineIndex>,
    import_behaviors:
        Arc<tyrus_core::collections::HashMap<String, tyrus_core::ast::BehaviorInterfaceInfo>>,
    lint_diagnostics: Option<Arc<Vec<tyrus_lint::LintDiagnostic>>>,
    semantic_tokens: Option<Arc<Vec<SemanticToken>>>,
    call_parameters: HashMap<String, Option<Vec<String>>>,
    inlay_hints: std::collections::HashMap<(u32, u32, u32, u32), Arc<Vec<InlayHint>>>,
}

pub(crate) fn cache_call_parameters(
    cache: &mut HashMap<String, Option<Vec<String>>>,
    callee: String,
    parameters: Option<Vec<String>>,
) {
    if cache.len() >= MAX_CACHED_CALL_PARAMETERS
        && !cache.contains_key(&callee)
        && let Some(victim) = cache.keys().min().cloned()
    {
        cache.remove(&victim);
    }
    cache.insert(callee, parameters);
}

pub(crate) fn cache_inlay_hints(
    cache: &mut std::collections::HashMap<(u32, u32, u32, u32), Arc<Vec<InlayHint>>>,
    key: (u32, u32, u32, u32),
    hints: Vec<InlayHint>,
) {
    if cache.len() >= 4
        && !cache.contains_key(&key)
        && let Some(victim) = cache.keys().min().copied()
    {
        cache.remove(&victim);
    }
    cache.insert(key, Arc::new(hints));
}

#[derive(Clone)]
struct FormatConfigCacheEntry {
    modified: Option<std::time::SystemTime>,
    len: u64,
    config: tyrus_fmt::config::Config,
}

#[derive(Clone)]
struct LintConfigCacheEntry {
    modified: Option<std::time::SystemTime>,
    len: u64,
    config: tyrus_lint::LintConfig,
}

#[derive(Clone)]
struct ProjectDatabaseConfigCacheEntry {
    modified: Option<std::time::SystemTime>,
    len: u64,
    config: std::result::Result<Option<tyrus_core::db::DatabaseConfig>, String>,
}

#[derive(Clone)]
struct ImportMapCacheEntry {
    modified: Option<std::time::SystemTime>,
    len: u64,
    import_map: std::result::Result<tyrus_core::project::ImportMap, String>,
}

struct EnvState {
    schema: tyrus_core::env::EnvSchema,
    env_uri: Url,
}

#[derive(Clone)]
struct EnvConfig {
    env_file: String,
    uri: Url,
}

fn is_tyrus_uri(uri: &Url) -> bool {
    uri.path().ends_with(".tyr")
}

pub fn run() {
    let runtime = tokio::runtime::Runtime::new().expect("failed to create LSP runtime");
    runtime.block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let (service, socket) = LspService::new(Backend::new);
        let service = TracedLspService { inner: service };
        Server::new(stdin, stdout, socket).serve(service).await;
    });
}

#[cfg(test)]
#[path = "../tests/unit/lsp.rs"]
mod tests;
