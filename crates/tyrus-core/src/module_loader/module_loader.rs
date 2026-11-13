use crate::analysis::{AnalysisDiagnostic, AnalyzeContext, DiagnosticSeverity, render_diagnostic};
use crate::ast::*;
use crate::collections::{HashMap, HashSet};
use crate::env::EnvSchema;
use crate::lexer::Token;
use crate::parser::ParseError;
use crate::parser::Parser;
use crate::project::{ImportMap, resolve_import_path_in_project};
use crate::span::Span;
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

#[path = "http_handler_context/http_handler_context.rs"]
mod http_handler_context;
#[path = "native_modules/native_modules.rs"]
mod native_modules;
use http_handler_context::HandlerContext;
pub(crate) use native_modules::*;
pub use native_modules::{
    native_export_analysis_info, native_export_behavior, native_export_enum, native_export_info,
    native_export_names, native_export_target, native_interface_module, native_module_exporting,
    native_target_return_type,
};

pub fn render_parse_error(path: &Path, source: &str, error: &ParseError) -> String {
    let Some(range) = error.span.clone() else {
        return format!("error[parse] in `{}`: {}", path.display(), error.message);
    };
    render_diagnostic(
        path,
        source,
        &AnalysisDiagnostic {
            code: error.code.unwrap_or("parse").to_string(),
            range,
            severity: DiagnosticSeverity::Error,
            message: error.message.clone(),
            related: Vec::new(),
            help: error.help.clone(),
        },
    )
}

pub fn dir_entry_type_enum() -> ResolvedEnum {
    ResolvedEnum {
        repr: Type::I32,
        nominal: false,
        members: vec![
            ("File".to_string(), Expr::synthetic(ExprKind::IntLit(0))),
            (
                "Directory".to_string(),
                Expr::synthetic(ExprKind::IntLit(1)),
            ),
            ("Symlink".to_string(), Expr::synthetic(ExprKind::IntLit(2))),
            ("Other".to_string(), Expr::synthetic(ExprKind::IntLit(3))),
        ],
        member_kinds: vec![EnumPayload::Unit; 4],
        is_algebraic: false,
    }
}

pub fn file_mode_enum() -> ResolvedEnum {
    ResolvedEnum {
        repr: Type::I32,
        nominal: false,
        members: vec![
            ("Read".to_string(), Expr::synthetic(ExprKind::IntLit(0o400))),
            (
                "Write".to_string(),
                Expr::synthetic(ExprKind::IntLit(0o200)),
            ),
            (
                "Execute".to_string(),
                Expr::synthetic(ExprKind::IntLit(0o100)),
            ),
            (
                "OwnerRead".to_string(),
                Expr::synthetic(ExprKind::IntLit(0o400)),
            ),
            (
                "OwnerWrite".to_string(),
                Expr::synthetic(ExprKind::IntLit(0o200)),
            ),
            (
                "OwnerExecute".to_string(),
                Expr::synthetic(ExprKind::IntLit(0o100)),
            ),
            (
                "GroupRead".to_string(),
                Expr::synthetic(ExprKind::IntLit(0o040)),
            ),
            (
                "GroupWrite".to_string(),
                Expr::synthetic(ExprKind::IntLit(0o020)),
            ),
            (
                "GroupExecute".to_string(),
                Expr::synthetic(ExprKind::IntLit(0o010)),
            ),
            (
                "OtherRead".to_string(),
                Expr::synthetic(ExprKind::IntLit(0o004)),
            ),
            (
                "OtherWrite".to_string(),
                Expr::synthetic(ExprKind::IntLit(0o002)),
            ),
            (
                "OtherExecute".to_string(),
                Expr::synthetic(ExprKind::IntLit(0o001)),
            ),
            (
                "AllRead".to_string(),
                Expr::synthetic(ExprKind::IntLit(0o444)),
            ),
            (
                "AllWrite".to_string(),
                Expr::synthetic(ExprKind::IntLit(0o222)),
            ),
            (
                "AllExecute".to_string(),
                Expr::synthetic(ExprKind::IntLit(0o111)),
            ),
            (
                "FullOwner".to_string(),
                Expr::synthetic(ExprKind::IntLit(0o700)),
            ),
            (
                "FullGroup".to_string(),
                Expr::synthetic(ExprKind::IntLit(0o070)),
            ),
            (
                "FullOther".to_string(),
                Expr::synthetic(ExprKind::IntLit(0o007)),
            ),
            ("Full".to_string(), Expr::synthetic(ExprKind::IntLit(0o777))),
        ],
        member_kinds: vec![EnumPayload::Unit; 20],
        is_algebraic: false,
    }
}

pub fn http_status_enum() -> ResolvedEnum {
    ResolvedEnum {
        repr: Type::I32,
        nominal: false,
        members: vec![
            (
                "Continue".to_string(),
                Expr::synthetic(ExprKind::IntLit(100)),
            ),
            (
                "SwitchingProtocols".to_string(),
                Expr::synthetic(ExprKind::IntLit(101)),
            ),
            (
                "Processing".to_string(),
                Expr::synthetic(ExprKind::IntLit(102)),
            ),
            (
                "EarlyHints".to_string(),
                Expr::synthetic(ExprKind::IntLit(103)),
            ),
            ("Ok".to_string(), Expr::synthetic(ExprKind::IntLit(200))),
            (
                "Created".to_string(),
                Expr::synthetic(ExprKind::IntLit(201)),
            ),
            (
                "Accepted".to_string(),
                Expr::synthetic(ExprKind::IntLit(202)),
            ),
            (
                "NonAuthoritativeInformation".to_string(),
                Expr::synthetic(ExprKind::IntLit(203)),
            ),
            (
                "NoContent".to_string(),
                Expr::synthetic(ExprKind::IntLit(204)),
            ),
            (
                "ResetContent".to_string(),
                Expr::synthetic(ExprKind::IntLit(205)),
            ),
            (
                "PartialContent".to_string(),
                Expr::synthetic(ExprKind::IntLit(206)),
            ),
            (
                "MultiStatus".to_string(),
                Expr::synthetic(ExprKind::IntLit(207)),
            ),
            (
                "AlreadyReported".to_string(),
                Expr::synthetic(ExprKind::IntLit(208)),
            ),
            ("ImUsed".to_string(), Expr::synthetic(ExprKind::IntLit(226))),
            (
                "MultipleChoices".to_string(),
                Expr::synthetic(ExprKind::IntLit(300)),
            ),
            (
                "MovedPermanently".to_string(),
                Expr::synthetic(ExprKind::IntLit(301)),
            ),
            ("Found".to_string(), Expr::synthetic(ExprKind::IntLit(302))),
            (
                "SeeOther".to_string(),
                Expr::synthetic(ExprKind::IntLit(303)),
            ),
            (
                "NotModified".to_string(),
                Expr::synthetic(ExprKind::IntLit(304)),
            ),
            (
                "TemporaryRedirect".to_string(),
                Expr::synthetic(ExprKind::IntLit(307)),
            ),
            (
                "PermanentRedirect".to_string(),
                Expr::synthetic(ExprKind::IntLit(308)),
            ),
            (
                "BadRequest".to_string(),
                Expr::synthetic(ExprKind::IntLit(400)),
            ),
            (
                "Unauthorized".to_string(),
                Expr::synthetic(ExprKind::IntLit(401)),
            ),
            (
                "PaymentRequired".to_string(),
                Expr::synthetic(ExprKind::IntLit(402)),
            ),
            (
                "Forbidden".to_string(),
                Expr::synthetic(ExprKind::IntLit(403)),
            ),
            (
                "NotFound".to_string(),
                Expr::synthetic(ExprKind::IntLit(404)),
            ),
            (
                "MethodNotAllowed".to_string(),
                Expr::synthetic(ExprKind::IntLit(405)),
            ),
            (
                "NotAcceptable".to_string(),
                Expr::synthetic(ExprKind::IntLit(406)),
            ),
            (
                "ProxyAuthenticationRequired".to_string(),
                Expr::synthetic(ExprKind::IntLit(407)),
            ),
            (
                "RequestTimeout".to_string(),
                Expr::synthetic(ExprKind::IntLit(408)),
            ),
            (
                "Conflict".to_string(),
                Expr::synthetic(ExprKind::IntLit(409)),
            ),
            ("Gone".to_string(), Expr::synthetic(ExprKind::IntLit(410))),
            (
                "LengthRequired".to_string(),
                Expr::synthetic(ExprKind::IntLit(411)),
            ),
            (
                "PreconditionFailed".to_string(),
                Expr::synthetic(ExprKind::IntLit(412)),
            ),
            (
                "PayloadTooLarge".to_string(),
                Expr::synthetic(ExprKind::IntLit(413)),
            ),
            (
                "UriTooLong".to_string(),
                Expr::synthetic(ExprKind::IntLit(414)),
            ),
            (
                "UnsupportedMediaType".to_string(),
                Expr::synthetic(ExprKind::IntLit(415)),
            ),
            (
                "RangeNotSatisfiable".to_string(),
                Expr::synthetic(ExprKind::IntLit(416)),
            ),
            (
                "ExpectationFailed".to_string(),
                Expr::synthetic(ExprKind::IntLit(417)),
            ),
            (
                "ImATeapot".to_string(),
                Expr::synthetic(ExprKind::IntLit(418)),
            ),
            (
                "MisdirectedRequest".to_string(),
                Expr::synthetic(ExprKind::IntLit(421)),
            ),
            (
                "UnprocessableContent".to_string(),
                Expr::synthetic(ExprKind::IntLit(422)),
            ),
            ("Locked".to_string(), Expr::synthetic(ExprKind::IntLit(423))),
            (
                "FailedDependency".to_string(),
                Expr::synthetic(ExprKind::IntLit(424)),
            ),
            (
                "TooEarly".to_string(),
                Expr::synthetic(ExprKind::IntLit(425)),
            ),
            (
                "UpgradeRequired".to_string(),
                Expr::synthetic(ExprKind::IntLit(426)),
            ),
            (
                "PreconditionRequired".to_string(),
                Expr::synthetic(ExprKind::IntLit(428)),
            ),
            (
                "TooManyRequests".to_string(),
                Expr::synthetic(ExprKind::IntLit(429)),
            ),
            (
                "RequestHeaderFieldsTooLarge".to_string(),
                Expr::synthetic(ExprKind::IntLit(431)),
            ),
            (
                "UnavailableForLegalReasons".to_string(),
                Expr::synthetic(ExprKind::IntLit(451)),
            ),
            (
                "InternalServerError".to_string(),
                Expr::synthetic(ExprKind::IntLit(500)),
            ),
            (
                "NotImplemented".to_string(),
                Expr::synthetic(ExprKind::IntLit(501)),
            ),
            (
                "BadGateway".to_string(),
                Expr::synthetic(ExprKind::IntLit(502)),
            ),
            (
                "ServiceUnavailable".to_string(),
                Expr::synthetic(ExprKind::IntLit(503)),
            ),
            (
                "GatewayTimeout".to_string(),
                Expr::synthetic(ExprKind::IntLit(504)),
            ),
            (
                "HttpVersionNotSupported".to_string(),
                Expr::synthetic(ExprKind::IntLit(505)),
            ),
            (
                "VariantAlsoNegotiates".to_string(),
                Expr::synthetic(ExprKind::IntLit(506)),
            ),
            (
                "InsufficientStorage".to_string(),
                Expr::synthetic(ExprKind::IntLit(507)),
            ),
            (
                "LoopDetected".to_string(),
                Expr::synthetic(ExprKind::IntLit(508)),
            ),
            (
                "NotExtended".to_string(),
                Expr::synthetic(ExprKind::IntLit(510)),
            ),
            (
                "NetworkAuthenticationRequired".to_string(),
                Expr::synthetic(ExprKind::IntLit(511)),
            ),
        ],
        member_kinds: vec![EnumPayload::Unit; 61],
        is_algebraic: false,
    }
}

pub fn math_constant_names() -> &'static [&'static str] {
    &[
        "PI", "E", "LN2", "LN10", "LOG2E", "LOG10E", "SQRT2", "EPSILON", "TAU", "PHI",
    ]
}

pub fn math_constant_value(name: &str) -> Option<f64> {
    use std::f64::consts;
    match name {
        "PI" => Some(consts::PI),
        "E" => Some(consts::E),
        "LN2" => Some(consts::LN_2),
        "LN10" => Some(consts::LN_10),
        "LOG2E" => Some(consts::LOG2_E),
        "LOG10E" => Some(consts::LOG10_E),
        "SQRT2" => Some(consts::SQRT_2),
        "EPSILON" => Some(f64::EPSILON),
        "TAU" => Some(consts::TAU),
        #[allow(clippy::excessive_precision)]
        "PHI" => Some(1.618_033_988_749_894_848_204_586_834_365_638_118),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModuleRestriction {
    #[default]
    None,
    Sandbox,
    WasmTarget,
}

impl ModuleRestriction {
    fn forbids(self, source: &str) -> bool {
        self != Self::None
            && (SANDBOX_FORBIDDEN_MODULES.contains(&source)
                || NOT_YET_PORTABLE_MODULES.contains(&source))
    }

    fn explain(self, source: &str) -> String {
        if NOT_YET_PORTABLE_MODULES.contains(&source) {
            let place = match self {
                Self::None => return String::new(),
                Self::Sandbox => "in a sandboxed build",
                Self::WasmTarget => "when compiling to WebAssembly",
            };
            return format!(
                "error: module `{source}` is not available {place} yet; it runs its work on a \
                 background thread pool that this target does not provide\n\
                 help: build this program for the native target, or remove the import"
            );
        }
        match self {
            Self::None => String::new(),
            Self::Sandbox => format!(
                "error: module `{source}` is not available in a sandboxed build; it needs \
                 operating system access that the playground does not have"
            ),
            Self::WasmTarget => format!(
                "error: module `{source}` is not available when compiling to WebAssembly; it \
                 needs operating system access that the WebAssembly target does not have\n\
                 help: build this program for the native target, or remove the import"
            ),
        }
    }
}

pub const SANDBOX_FORBIDDEN_MODULES: &[&str] = &[
    "tyrus:archive",
    "tyrus:cli",
    "tyrus:crypto",
    "tyrus:db",
    "tyrus:encoding",
    "tyrus:fs",
    "tyrus:http",
    "tyrus:net",
    "tyrus:os",
    "tyrus:process",
    "tyrus:simd",
    "tyrus:stream",
    "tyrus:task",
];

/// Modules that need no operating system access but depend on runtime
/// support the sandbox and the WebAssembly target do not have yet.
pub const NOT_YET_PORTABLE_MODULES: &[&str] = &["tyrus:compress"];

pub const NATIVE_MODULES: &[&str] = &[
    "tyrus:algorithm",
    "tyrus:archive",
    "tyrus:cli",
    "tyrus:compress",
    "tyrus:crypto",
    "tyrus:db",
    "tyrus:encoding",
    "tyrus:fs",
    "tyrus:http",
    "tyrus:math",
    "tyrus:memory",
    "tyrus:net",
    "tyrus:os",
    "tyrus:process",
    "tyrus:simd",
    "tyrus:stream",
    "tyrus:task",
    "tyrus:temporal",
    "tyrus:utils",
];

fn task_handle_type_export(name: &str) -> (String, ExportInfo) {
    (
        name.to_string(),
        ExportInfo {
            target: name.to_string(),
            ty: Type::Named(name.to_string()),
            interface_fields: Some(Vec::new()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: Vec::new(),
            db_model: None,
        },
    )
}

pub fn load_program(entry: &Path, env_schema: Option<&EnvSchema>) -> Result<Program, String> {
    load_program_sandboxed(entry, env_schema, ModuleRestriction::None)
}

pub fn load_program_sandboxed(
    entry: &Path,
    env_schema: Option<&EnvSchema>,
    sandbox: ModuleRestriction,
) -> Result<Program, String> {
    load_program_with_db_model_validation(entry, env_schema, sandbox, true)
}

pub fn load_program_for_db_migrations(entry: &Path) -> Result<Program, String> {
    load_program_with_db_model_validation(entry, None, ModuleRestriction::None, false)
}

pub fn contextualize_http_controller(
    path: &Path,
    program: &mut Program,
) -> Result<Option<HashMap<String, Vec<Field>>>, String> {
    let path = path.canonicalize().map_err(|error| error.to_string())?;
    let root = project_root(&path);
    let mut loader = ModuleLoader {
        imports: ImportMap::load(&root)?,
        root: root.clone(),
        validate_db_models: false,
        db_backend: crate::project::load_project(&root)
            .ok()
            .and_then(|project| project.database.map(|database| database.backend)),
        ..ModuleLoader::default()
    };
    let mut routes = Vec::new();
    collect_route_modules(&root.join("src/routes"), &mut routes)?;
    for route in routes {
        let source = fs::read_to_string(&route).map_err(|error| error.to_string())?;
        let (_, route_program) = crate::analysis::parse_for_analysis(&source).map_err(|_| {
            format!(
                "error: could not parse HTTP route module `{}`",
                route.display()
            )
        })?;
        loader.prepare_http_handler_contexts(&route, &route_program)?;
    }
    let Some(contexts) = loader.handler_contexts.get(&path) else {
        return Ok(None);
    };
    http_handler_context::apply_handler_contexts(program, contexts);
    let mut interfaces = HashMap::default();
    for context in contexts.values() {
        interfaces.extend(context.interfaces.clone());
    }
    if let Some(reply) = native_builtins("tyrus:http")
        .and_then(|exports| exports.get("HttpReply"))
        .and_then(|export| export.interface_fields.clone())
    {
        interfaces.insert("HttpReply".to_string(), reply);
    }
    Ok(Some(interfaces))
}

fn load_program_with_db_model_validation(
    entry: &Path,
    env_schema: Option<&EnvSchema>,
    sandbox: ModuleRestriction,
    validate_db_models: bool,
) -> Result<Program, String> {
    let entry = entry
        .canonicalize()
        .map_err(|error| format!("error: could not resolve `{}`: {error}", entry.display()))?;
    ensure_tyrus_source(&entry)?;
    let root = project_root(&entry);
    let imports = ImportMap::load(&root)?;
    let (db_schema, db_schema_error, db_backend) = if root.join("tyrus.json").is_file() {
        match crate::project::load_project(&root) {
            Ok(project) => match project.database {
                Some(config) => {
                    let backend = config.backend.clone();
                    match crate::db::DbSchemaCache::default().load_configured(&root, &config) {
                        Ok(snapshot) => (Some(snapshot), None, Some(backend)),
                        Err(error) => (None, Some(error), Some(backend)),
                    }
                }
                None => (
                    None,
                    Some(
                        "configure `database` in tyrus.json before calling `db.query`".to_string(),
                    ),
                    None,
                ),
            },
            Err(error) => (None, Some(error), None),
        }
    } else {
        (
            None,
            Some(
                "create tyrus.json with a `database` configuration before calling `db.query`"
                    .to_string(),
            ),
            None,
        )
    };
    let mut loader = ModuleLoader {
        env_schema,
        db_schema,
        db_schema_error,
        db_backend,
        validate_db_models,
        root,
        imports,
        sandbox,
        ..ModuleLoader::default()
    };
    loader.resolve_module(&entry, true)?;
    if crate::project::is_entry_module(&entry) {
        let mut route_modules = Vec::new();
        collect_route_modules(&loader.root.join("src").join("routes"), &mut route_modules)?;
        route_modules.sort();
        for route_module in route_modules {
            let exports = loader.resolve_module(&route_module, false)?;
            if let Some(route_export) = exports.get(DEFAULT_EXPORT) {
                if !is_http_route_plugin(&route_export.ty) {
                    return Err(format!(
                        "error: default export in route module `{}` must have type `(router: HttpRouter) => void`",
                        route_module.display()
                    ));
                }
                loader.auto_route_exports.push(route_export.target.clone());
            } else {
                let mut route_exports = exports
                    .iter()
                    .filter(|(name, route_export)| {
                        name.as_str() != DEFAULT_EXPORT && is_http_route_plugin(&route_export.ty)
                    })
                    .collect::<Vec<_>>();
                route_exports.sort_by_key(|(left, _)| *left);
                loader.auto_route_exports.extend(
                    route_exports
                        .into_iter()
                        .map(|(_, route_export)| route_export.target.clone()),
                );
            }
        }
    }
    Ok(Program {
        imports: Vec::new(),
        exports: Vec::new(),
        items: loader.items,
        auto_route_exports: loader.auto_route_exports,
    })
}

fn project_root(entry: &Path) -> PathBuf {
    let mut dir = entry.parent();
    while let Some(current) = dir {
        if current.join("tyrus.json").is_file() {
            return current.to_path_buf();
        }
        dir = current.parent();
    }
    entry
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

#[derive(Default)]
struct ModuleLoader<'a> {
    cache: HashMap<PathBuf, ModuleExports>,
    export_reachable_interfaces: HashMap<String, HashMap<String, Vec<Field>>>,
    root: PathBuf,
    imports: ImportMap,
    visiting: Vec<PathBuf>,
    items: Vec<Item>,
    auto_route_exports: Vec<String>,
    handler_contexts: HashMap<PathBuf, HashMap<String, HandlerContext>>,
    next_module_id: usize,
    validator_targets: HashSet<String>,

    env_schema: Option<&'a EnvSchema>,
    db_schema: Option<std::sync::Arc<crate::db::DbSchemaSnapshot>>,
    db_schema_error: Option<String>,
    db_backend: Option<crate::db::DatabaseBackend>,
    validate_db_models: bool,
    sandbox: ModuleRestriction,
}

/// Every module path reaching the loader is canonical: the entry through
/// `canonicalize` and every dependency through `resolve_candidate_path`. The
/// folder-scope check below compares path prefixes and depends on that.
#[derive(Debug, Clone, Default)]
pub(crate) struct ModuleExports {
    pub(crate) symbols: HashMap<String, ExportInfo>,
    pub(crate) scopes: HashMap<String, PathBuf>,
}

impl ModuleExports {
    pub(crate) fn get(&self, name: &str) -> Option<&ExportInfo> {
        self.symbols.get(name)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&String, &ExportInfo)> {
        self.symbols.iter()
    }

    fn from_native(symbols: HashMap<String, ExportInfo>) -> Self {
        ModuleExports {
            symbols,
            scopes: HashMap::default(),
        }
    }
}

/// A `protect` export is visible in the directory that declares it and in every
/// descendant of that directory, never in an ancestor or a sibling.
pub fn import_allowed_by_visibility(importer: &Path, scope: Option<&Path>) -> bool {
    let Some(scope) = scope else {
        return true;
    };
    importer
        .parent()
        .is_some_and(|directory| directory.starts_with(scope))
}

#[derive(Debug, Clone)]
pub(crate) struct ExportInfo {
    pub(crate) target: String,
    pub(crate) ty: Type,
    pub(crate) interface_fields: Option<Vec<Field>>,
    pub(crate) behavior_info: Option<BehaviorInterfaceInfo>,
    pub(crate) type_alias_ty: Option<Type>,
    pub(crate) enum_info: Option<ResolvedEnum>,
    pub(crate) throws: Vec<Type>,
    pub(crate) db_model: Option<crate::ast::DbModelMetadata>,
}

pub fn namespace_member_binding(namespace: &str, member: &str) -> String {
    format!("__tyrus_ns${namespace}${member}")
}

pub fn namespace_member_display(binding: &str) -> Option<String> {
    let rest = binding.strip_prefix("__tyrus_ns$")?;
    let (namespace, member) = rest.split_once('$')?;
    Some(format!("{namespace}.{member}"))
}

pub fn is_namespace_specifier(source: &str, specifier: &ImportSpecifier) -> bool {
    specifier.kind == ImportKind::Namespace
        || (source == "tyrus:db"
            && specifier.kind == ImportKind::Named
            && specifier.imported == "db")
}

pub fn specifier_bindings(
    source: &str,
    specifier: &ImportSpecifier,
    import_types: &HashMap<String, Type>,
) -> Vec<String> {
    if !is_namespace_specifier(source, specifier) {
        return vec![specifier.local.clone()];
    }
    let prefix = namespace_member_binding(&specifier.local, "");
    let mut bindings = import_types
        .keys()
        .filter(|name| name.starts_with(&prefix))
        .cloned()
        .collect::<Vec<_>>();
    bindings.sort();
    bindings
}

pub(crate) fn namespace_locals(program: &Program) -> HashSet<String> {
    program
        .imports
        .iter()
        .flat_map(|import| {
            import
                .specifiers
                .iter()
                .map(move |specifier| (&import.source, specifier))
        })
        .filter(|(source, specifier)| is_namespace_specifier(source, specifier))
        .map(|(_, specifier)| specifier.local.clone())
        .collect()
}

pub(crate) fn desugar_namespace_imports(program: &mut Program) {
    let namespaces = namespace_locals(program);
    if namespaces.is_empty() {
        return;
    }
    let mut renamer = Renamer::new(HashMap::default());
    renamer.namespaces = namespaces;
    let items = std::mem::take(&mut program.items);
    program.items = items
        .into_iter()
        .map(|item| renamer.rename_item(item))
        .collect();
}

impl ModuleLoader<'_> {
    fn reject_if_sandboxed(&self, source: &str) -> Result<(), String> {
        if self.sandbox.forbids(source) {
            return Err(self.sandbox.explain(source));
        }
        Ok(())
    }

    fn reject_test_framework_outside_test_file(
        importer: &Path,
        source: &str,
    ) -> Result<(), String> {
        if source != crate::test::MODULE || crate::test::is_test_file(importer) {
            return Ok(());
        }
        let name = importer
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("this file");
        let stem = importer
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("example");
        Err(format!(
            "error[E0434]: `{source}` can only be imported by a test file, and `{name}` is not one\n\
             help: rename it to `{stem}.test.tyr`, or move the import into a `*.test.tyr` file such as `__tests__/{stem}.test.tyr`"
        ))
    }

    fn resolve_export_source(
        &mut self,
        importer: &Path,
        source: &str,
    ) -> Result<ModuleExports, String> {
        if source.starts_with("tyrus:") {
            self.reject_if_sandboxed(source)?;
            Self::reject_test_framework_outside_test_file(importer, source)?;
            return native_builtins(source)
                .cloned()
                .map(ModuleExports::from_native)
                .ok_or_else(|| format!("error: unsupported native module `{source}`"));
        }
        let dependency =
            resolve_import_path_in_project(&self.root, &self.imports, importer, source)?;
        self.resolve_module(&dependency, false)
    }

    #[allow(clippy::too_many_lines)]
    fn resolve_module(&mut self, path: &Path, is_entry: bool) -> Result<ModuleExports, String> {
        ensure_tyrus_source(path)?;
        if !is_entry && !self.root.as_os_str().is_empty() && !path.starts_with(&self.root) {
            return Err(format!(
                "error: import `{}` resolves outside the project root `{}`",
                path.display(),
                self.root.display()
            ));
        }
        if let Some(exports) = self.cache.get(path) {
            return Ok(exports.clone());
        }
        if let Some(start) = self.visiting.iter().position(|candidate| candidate == path) {
            let mut cycle = self.visiting[start..]
                .iter()
                .map(|item| item.display().to_string())
                .collect::<Vec<_>>();
            cycle.push(path.display().to_string());
            return Err(format!(
                "error: circular module import: {}",
                cycle.join(" -> ")
            ));
        }

        self.visiting.push(path.to_path_buf());
        let source = fs::read_to_string(path)
            .map_err(|error| format!("error: could not read `{}`: {error}", path.display()))?;
        // Lexed and parsed once here, then handed to the analysis below. Doing
        // it twice, once for the imports and once inside `analyze_with_context`,
        // was the duplicate work the allocation profile showed.
        let (tokens, mut program) =
            crate::analysis::parse_for_analysis(&source).map_err(|error| match error {
                crate::analysis::ParseForAnalysis::Lex(_) => {
                    format!("error[lex] in `{}`: invalid token", path.display())
                }
                crate::analysis::ParseForAnalysis::Parse(error, _) => {
                    render_parse_error(path, &source, &error)
                }
                crate::analysis::ParseForAnalysis::Cancelled => {
                    format!("error: analysis cancelled for `{}`", path.display())
                }
            })?;
        crate::db::apply_backend_model_defaults(&mut program, self.db_backend.as_ref());
        self.prepare_http_handler_contexts(path, &program)?;
        if let Some(contexts) = self.handler_contexts.get(path) {
            http_handler_context::apply_handler_contexts(&mut program, contexts);
        }
        let analysis_program = program.clone();
        desugar_namespace_imports(&mut program);
        crate::await_hoist::hoist_nested_awaits(&mut program);

        let module_id = self.next_module_id;
        self.next_module_id += 1;
        let mut symbols = HashMap::default();
        let mut import_types = HashMap::default();
        let mut import_interfaces = HashMap::default();
        let mut import_behaviors = HashMap::default();
        let mut import_type_aliases = HashMap::default();
        let mut import_enums = HashMap::default();
        let mut import_throws = HashMap::default();
        let mut import_db_models = HashMap::default();
        let mut import_validators = HashMap::default();
        let mut validator_bindings = HashSet::default();
        let mut reachable_interfaces = HashMap::default();
        let mut occupied = HashSet::default();

        if let Some(contexts) = self.handler_contexts.get(path) {
            for context in contexts.values() {
                for (name, fields) in &context.interfaces {
                    import_types
                        .entry(name.clone())
                        .or_insert_with(|| Type::Named(name.clone()));
                    import_interfaces
                        .entry(name.clone())
                        .or_insert_with(|| fields.clone());
                }
                for (name, target) in &context.targets {
                    symbols
                        .entry(name.clone())
                        .or_insert_with(|| target.clone());
                }
            }
            if let Some(reply) =
                native_builtins("tyrus:http").and_then(|exports| exports.get("HttpReply"))
            {
                import_types
                    .entry("HttpReply".to_string())
                    .or_insert_with(|| reply.ty.clone());
                if let Some(fields) = &reply.interface_fields {
                    import_interfaces
                        .entry("HttpReply".to_string())
                        .or_insert_with(|| fields.clone());
                }
            }
        }

        for import in &program.imports {
            let is_native = import.source.starts_with("tyrus:");
            let imported_symbols = if is_native {
                self.reject_if_sandboxed(&import.source)?;
                Self::reject_test_framework_outside_test_file(path, &import.source)?;
                native_builtins(&import.source)
                    .cloned()
                    .map(ModuleExports::from_native)
                    .ok_or_else(|| {
                        format!("error: unsupported native module `{}`", import.source)
                    })?
            } else {
                let dependency = resolve_import_path_in_project(
                    &self.root,
                    &self.imports,
                    path,
                    &import.source,
                )?;
                self.resolve_module(&dependency, false)?
            };

            for specifier in &import.specifiers {
                if !occupied.insert(specifier.local.clone()) {
                    return Err(format!(
                        "error: duplicate imported name `{}` in `{}`",
                        specifier.local,
                        path.display()
                    ));
                }
                if is_namespace_specifier(&import.source, specifier) {
                    for (name, target) in &imported_symbols.symbols {
                        let scope = imported_symbols.scopes.get(name);
                        if !import_allowed_by_visibility(path, scope.map(PathBuf::as_path)) {
                            continue;
                        }
                        if import.source == "tyrus:db" && name == "db" {
                            continue;
                        }
                        let local = namespace_member_binding(&specifier.local, name);
                        symbols.insert(local.clone(), target.target.clone());
                        if self.validator_targets.contains(&target.target) {
                            import_validators.insert(local.clone(), target.ty.clone());
                            validator_bindings.insert(local.clone());
                        }
                        let ty = if target.enum_info.is_some() {
                            Type::Named(local.clone())
                        } else {
                            target.ty.clone()
                        };
                        import_types.insert(local.clone(), ty);
                        if let Some(fields) = &target.interface_fields {
                            import_interfaces.insert(local.clone(), fields.clone());
                            if is_native {
                                import_interfaces
                                    .entry(name.clone())
                                    .or_insert_with(|| fields.clone());
                                if local != name.as_str() {
                                    import_type_aliases
                                        .insert(local.clone(), Type::Named(name.clone()));
                                }
                            }
                        }
                        if let Some(behavior) = &target.behavior_info {
                            import_behaviors.insert(local.clone(), behavior.clone());
                        }
                        if let Some(alias_ty) = &target.type_alias_ty {
                            import_type_aliases.insert(local.clone(), alias_ty.clone());
                        }
                        if let Some(enum_info) = &target.enum_info {
                            import_enums.insert(local.clone(), enum_info.clone());
                            if is_native && local != name.as_str() {
                                import_type_aliases
                                    .insert(local.clone(), Type::Named(name.clone()));
                            }
                        }
                        if !target.throws.is_empty() {
                            import_throws.insert(local.clone(), target.throws.clone());
                        }
                        if let Some(model) = &target.db_model {
                            import_db_models.insert(local.clone(), model.clone());
                        }
                    }
                    continue;
                }
                let target = imported_symbols
                    .symbols
                    .get(&specifier.imported)
                    .ok_or_else(|| {
                        format!(
                            "error: module `{}` does not export `{}`",
                            import.source, specifier.imported
                        )
                    })?;
                let scope = imported_symbols.scopes.get(&specifier.imported);
                if !import_allowed_by_visibility(path, scope.map(PathBuf::as_path)) {
                    return Err(protected_export_error(
                        &specifier.imported,
                        &import.source,
                        path,
                        scope.expect("a refused import always carries a scope"),
                    ));
                }
                if let Some(reachable) = self.export_reachable_interfaces.get(&target.target) {
                    reachable_interfaces.extend(reachable.clone());
                }
                if let Some(model) = &target.db_model {
                    import_db_models.insert(specifier.local.clone(), model.clone());
                }
                if specifier.kind != ImportKind::Type {
                    symbols.insert(specifier.local.clone(), target.target.clone());
                    if self.validator_targets.contains(&target.target) {
                        import_validators.insert(specifier.local.clone(), target.ty.clone());
                        validator_bindings.insert(specifier.local.clone());
                    }
                }
                let ty = if target.enum_info.is_some() {
                    Type::Named(specifier.local.clone())
                } else {
                    target.ty.clone()
                };
                import_types.insert(specifier.local.clone(), ty);
                if let Some(fields) = &target.interface_fields {
                    import_interfaces.insert(specifier.local.clone(), fields.clone());
                    if is_native {
                        import_interfaces
                            .entry(specifier.imported.clone())
                            .or_insert_with(|| fields.clone());
                        if specifier.local != specifier.imported {
                            import_type_aliases.insert(
                                specifier.local.clone(),
                                Type::Named(specifier.imported.clone()),
                            );
                        }
                    }
                }
                if let Some(behavior) = &target.behavior_info {
                    import_behaviors.insert(specifier.local.clone(), behavior.clone());
                }
                if let Some(alias_ty) = &target.type_alias_ty {
                    import_type_aliases.insert(specifier.local.clone(), alias_ty.clone());
                }
                if let Some(enum_info) = &target.enum_info {
                    import_enums.insert(specifier.local.clone(), enum_info.clone());
                    if is_native && specifier.local != specifier.imported {
                        import_type_aliases.insert(
                            specifier.local.clone(),
                            Type::Named(specifier.imported.clone()),
                        );
                    }
                }
                if !target.throws.is_empty() {
                    import_throws.insert(specifier.local.clone(), target.throws.clone());
                }
            }
            if !is_native {
                let roots = import
                    .specifiers
                    .iter()
                    .flat_map(|specifier| {
                        if is_namespace_specifier(&import.source, specifier) {
                            imported_symbols.symbols.values().collect::<Vec<_>>()
                        } else {
                            imported_symbols
                                .symbols
                                .get(&specifier.imported)
                                .into_iter()
                                .collect()
                        }
                    })
                    .filter_map(|target| target.interface_fields.clone())
                    .collect::<Vec<_>>();
                crate::analysis::collect_reachable_interfaces(
                    &roots,
                    &|name| {
                        imported_symbols
                            .symbols
                            .get(name)
                            .and_then(|target| target.interface_fields.clone())
                    },
                    &mut reachable_interfaces,
                );
            }
        }

        register_implicit_timestamp_types(
            &program,
            &import_db_models,
            &mut import_types,
            &mut import_interfaces,
            &mut reachable_interfaces,
        );
        register_implicit_db_error_fields(&program, &import_db_models, &mut import_interfaces);

        let analyze = if self.validate_db_models {
            crate::analysis::analyze_parsed
        } else {
            crate::analysis::analyze_parsed_for_db_migrations
        };
        let analysis = analyze(
            &source,
            tokens,
            analysis_program,
            AnalyzeContext {
                import_types: Some(&import_types),
                import_targets: Some(&symbols),
                entry_point: is_entry && crate::project::is_entry_module(path),
                import_interfaces: Some(&import_interfaces),
                import_behaviors: Some(&import_behaviors),
                import_type_aliases: Some(&import_type_aliases),
                import_enums: Some(&import_enums),
                import_throws: Some(&import_throws),
                import_db_models: Some(&import_db_models),
                import_validators: Some(&import_validators),
                reachable_interfaces: Some(&reachable_interfaces),
                env_schema: self.env_schema,
                db_schema: self.db_schema.as_deref(),
                db_schema_error: self.db_schema_error.as_deref(),
                db_backend: self.db_backend.as_ref(),
            },
        );
        let errors = analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.severity == DiagnosticSeverity::Error
                    || diagnostic.code == crate::definite_assignment::USE_BEFORE_ASSIGN
            })
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            let rendered = errors
                .into_iter()
                .map(|diagnostic| render_diagnostic(path, &source, diagnostic))
                .collect::<Vec<_>>()
                .join("\n");
            return Err(rendered);
        }

        crate::ctfe::apply_evaluations(&mut program, &analysis.ctfe.evaluations);

        let mut local_declarations = Vec::new();
        for item in &program.items {
            let Some(name) = item_name(item) else {
                continue;
            };
            if !occupied.insert(name.to_string()) {
                return Err(format!(
                    "error: duplicate top-level name `{name}` in `{}`",
                    path.display()
                ));
            }
            let target = if is_entry {
                debug_assert_eq!(module_id, ENTRY_MODULE_ID);
                entry_symbol(name)
            } else {
                module_symbol(module_id, name)
            };
            if matches!(
                item,
                Item::FnDecl {
                    is_validator: true,
                    ..
                }
            ) {
                validator_bindings.insert(name.to_string());
                self.validator_targets.insert(target.clone());
            }
            local_declarations.push((name.to_string(), target.clone(), item));
            symbols.insert(name.to_string(), target);
        }

        let mut inferred_types: HashMap<&str, &Type> = HashMap::default();
        let mut inferred_scopes: HashMap<&str, usize> = HashMap::default();
        for symbol in &analysis.symbols {
            let Some(ty) = &symbol.semantic_type else {
                continue;
            };
            if matches!(ty, Type::Infer) {
                continue;
            }
            let width = symbol.scope.end.saturating_sub(symbol.scope.start);
            if inferred_scopes
                .get(symbol.name.as_str())
                .is_none_or(|current| width > *current)
            {
                inferred_scopes.insert(symbol.name.as_str(), width);
                inferred_types.insert(symbol.name.as_str(), ty);
            }
        }

        let mut declarations = HashMap::default();
        let export_defs = crate::type_resolve::GenericDefs::collect(&program);
        for (name, target, item) in local_declarations {
            let Some(mut ty) = item_type(item) else {
                continue;
            };
            if (matches!(ty, Type::Infer)
                || matches!(&ty, Type::Function { ret, .. } if matches!(ret.as_ref(), Type::Infer)))
                && let Some(inferred) = inferred_types.get(name.as_str())
            {
                ty = (*inferred).clone();
            }
            let info = ExportInfo {
                target,
                ty,
                interface_fields: item_interface_fields(item, &export_defs),
                behavior_info: behavior_interface_info(item, &program.items),
                type_alias_ty: item_type_alias_ty(item),
                enum_info: item_enum_info(item),
                throws: item_throws(item),
                db_model: item_db_model(item),
            };
            declarations.insert(name, info);
        }

        let mut exports: HashMap<String, ExportInfo> = HashMap::default();
        let mut export_scopes: HashMap<String, PathBuf> = HashMap::default();
        let mut publish =
            |name: &str, info: ExportInfo, scope: Option<PathBuf>| -> Result<(), String> {
                if exports.insert(name.to_string(), info).is_some() {
                    return Err(format!(
                        "error: duplicate export `{name}` in `{}`",
                        path.display()
                    ));
                }
                if let Some(scope) = scope {
                    export_scopes.insert(name.to_string(), scope);
                }
                Ok(())
            };
        for export in &program.exports {
            match export {
                ExportDecl::Named {
                    exported,
                    local,
                    visibility,
                    ..
                } => {
                    let target = declarations.get(local).ok_or_else(|| {
                        format!(
                            "error: cannot export undeclared name `{local}` from `{}`",
                            path.display()
                        )
                    })?;
                    let scope = match visibility {
                        Visibility::Protect => Some(
                            path.parent()
                                .unwrap_or_else(|| Path::new("."))
                                .to_path_buf(),
                        ),
                        Visibility::Public => None,
                    };
                    publish(exported, target.clone(), scope)?;
                }
                ExportDecl::Re {
                    exported,
                    imported,
                    source,
                    ..
                } => {
                    let forwarded = self.resolve_export_source(path, source)?;
                    let target = forwarded.symbols.get(imported).ok_or_else(|| {
                        format!("error: module `{source}` does not export `{imported}`")
                    })?;
                    publish(
                        exported,
                        target.clone(),
                        forwarded.scopes.get(imported).cloned(),
                    )?;
                }
                ExportDecl::All { source, .. } => {
                    let forwarded = self.resolve_export_source(path, source)?;
                    let mut names = forwarded.symbols.keys().cloned().collect::<Vec<_>>();
                    names.sort();
                    for name in names {
                        if name == DEFAULT_EXPORT {
                            continue;
                        }
                        let info = forwarded.symbols[&name].clone();
                        let scope = forwarded.scopes.get(&name).cloned();
                        publish(&name, info, scope)?;
                    }
                }
            }
        }

        let mut exported_reachable_interfaces = reachable_interfaces.clone();
        exported_reachable_interfaces.extend(import_interfaces.clone());
        for info in exports.values() {
            if info.interface_fields.is_some() {
                self.export_reachable_interfaces
                    .insert(info.target.clone(), exported_reachable_interfaces.clone());
            }
        }

        for (local, enum_info) in &import_enums {
            let target = symbols.get(local).cloned().unwrap_or_else(|| local.clone());
            if self
                .items
                .iter()
                .any(|item| matches!(item, Item::EnumDecl { name, .. } if name == &target))
                || program
                    .items
                    .iter()
                    .any(|item| matches!(item, Item::EnumDecl { name, .. } if name == &target))
            {
                continue;
            }
            self.items.push(Item::EnumDecl {
                span: Span::DUMMY,
                name: target,
                repr: enum_info.repr.clone(),
                members: enum_info
                    .members
                    .iter()
                    .map(|(name, value)| EnumMember {
                        name: name.clone(),
                        value: Some(value.clone()),
                        payload: EnumPayload::Unit,
                    })
                    .collect(),
            });
        }
        for (local, behavior) in &import_behaviors {
            let target = symbols.get(local).cloned().unwrap_or_else(|| local.clone());
            if self.items.iter().any(
                |item| matches!(item, Item::BehaviorInterfaceDecl { name, .. } if name == &target),
            ) {
                continue;
            }
            self.items.push(Item::BehaviorInterfaceDecl {
                name: target,
                type_params: Vec::new(),
                targets: behavior.targets.clone(),
                extends: behavior.extends.clone(),
                methods: behavior.methods.clone(),
                bindings: behavior.bindings.clone(),
                span: Span::DUMMY,
            });
        }
        let mut renamer = Renamer::new(symbols);
        renamer.validator_bindings = validator_bindings;
        renamer.namespaces = namespace_locals(&program);
        self.items.extend(
            program
                .items
                .into_iter()
                .map(|item| renamer.rename_item(item)),
        );
        self.visiting.pop();
        let exports = ModuleExports {
            symbols: exports,
            scopes: export_scopes,
        };
        self.cache.insert(path.to_path_buf(), exports.clone());
        Ok(exports)
    }
}

/// Whether `importer` may import a `protect` export declared in
/// `declaring_module`. Canonicalizes both, because the editor resolvers hand
/// over paths that may still contain `..`, which breaks prefix comparison.
#[must_use]
pub fn protect_scope_allows(importer: &Path, declaring_module: &Path) -> bool {
    let importer = importer
        .canonicalize()
        .unwrap_or_else(|_| importer.to_path_buf());
    let declaring = declaring_module
        .canonicalize()
        .unwrap_or_else(|_| declaring_module.to_path_buf());
    import_allowed_by_visibility(&importer, declaring.parent())
}

/// The names a module publishes with `export protect`. The editor index uses it
/// to keep an out-of-scope name out of auto-import completion, so the suggestion
/// never inserts an import the compiler would refuse.
#[must_use]
pub fn protected_export_names(source: &str) -> Vec<String> {
    // Most workspace files do not declare protected exports. Avoid parsing
    // those files just to discover that fact: editor completion calls this
    // helper while indexing every file in the project.
    if !source.contains("protect") {
        return Vec::new();
    }
    let Some(program) = Parser::from_source(source)
        .ok()
        .and_then(|mut parser| parser.parse_program().ok())
    else {
        return Vec::new();
    };
    program
        .exports
        .iter()
        .filter_map(|export| match export {
            ExportDecl::Named {
                exported,
                visibility: Visibility::Protect,
                ..
            } => Some(exported.clone()),
            _ => None,
        })
        .collect()
}

/// The `protect` rule for the editor paths. The compiler enforces it inside
/// `resolve_module`, where the export tables are already in hand; the linter and
/// the Language Server resolve one import at a time and call this instead, so
/// the rule itself exists once.
#[derive(Debug, Clone)]
pub struct ProtectedImport {
    pub scope: PathBuf,
    pub declaring_module: PathBuf,
    /// Byte range of ` protect` in the declaring file, from the end of `export`
    /// to the end of the modifier, so deleting it leaves `export` intact. The
    /// Quick Fix edits the declaring file, not the importing one.
    pub modifier: Option<Range<usize>>,
}

impl ProtectedImport {
    #[must_use]
    pub fn folder(&self) -> String {
        display_relative(&self.scope, &self.scope)
    }

    #[must_use]
    pub fn declaring_file(&self) -> String {
        display_relative(&self.declaring_module, &self.scope)
    }

    /// The linter keeps only the first `help:` entry, so the scope belongs in the
    /// message and the actionable fix comes first in the help.
    #[must_use]
    pub fn message(&self, imported: &str, module: &str) -> String {
        format!(
            "`{imported}` is a protected export of `{module}` and cannot be imported here; `export protect` in `{}` limits it to `{}` and its subfolders",
            self.declaring_file(),
            self.folder()
        )
    }

    #[must_use]
    pub fn help(&self, imported: &str) -> Vec<String> {
        vec![format!(
            "drop `protect` from the declaration of `{imported}`, or move this file into `{}`",
            self.folder()
        )]
    }
}

#[must_use]
pub fn protected_import_violation(
    importer: &Path,
    target: &Path,
    imported: &str,
) -> Option<ProtectedImport> {
    let importer = importer.canonicalize().ok()?;
    let target = target.canonicalize().ok()?;
    let declaration = protected_declaration(&target, imported, &mut HashSet::default())?;
    if import_allowed_by_visibility(&importer, Some(&declaration.scope)) {
        return None;
    }
    Some(declaration)
}

/// The `protect` modifier does not appear in the AST span of the declaration it
/// prefixes, so the Quick Fix locates it in the token stream: every occurrence is
/// an `export` immediately followed by `protect`, and the statement that follows
/// names what it exports.
#[must_use]
pub fn protect_modifier_range(source: &str, name: &str) -> Option<Range<usize>> {
    let spanned = crate::lexer::lex_spanned(source).ok()?;
    for index in 0..spanned.len().saturating_sub(1) {
        let (Token::Export, export_range) = (&spanned[index].0, &spanned[index].1) else {
            continue;
        };
        let (Token::Protect, protect_range) = (&spanned[index + 1].0, &spanned[index + 1].1) else {
            continue;
        };
        let rest = &spanned[index + 2..];
        let names_it = if matches!(rest.first(), Some((Token::LBrace, _))) {
            rest.iter()
                .skip(1)
                .take_while(|(token, _)| !matches!(token, Token::RBrace))
                .any(|(token, _)| matches!(token, Token::Ident(found) if found == name))
        } else {
            rest.iter()
                .take_while(|(token, _)| !matches!(token, Token::Semicolon | Token::LBrace))
                .any(|(token, _)| matches!(token, Token::Ident(found) if found == name))
        };
        if names_it {
            return Some(export_range.end..protect_range.end);
        }
    }
    None
}

fn protected_declaration(
    module: &Path,
    name: &str,
    visited: &mut HashSet<PathBuf>,
) -> Option<ProtectedImport> {
    if !visited.insert(module.to_path_buf()) {
        return None;
    }
    let source = fs::read_to_string(module).ok()?;
    let program = Parser::from_source(&source)
        .ok()
        .and_then(|mut parser| parser.parse_program().ok())?;
    let mut wildcards = Vec::new();
    for export in &program.exports {
        match export {
            ExportDecl::Named {
                exported,
                visibility,
                ..
            } if exported == name => {
                if *visibility != Visibility::Protect {
                    return None;
                }
                return Some(ProtectedImport {
                    scope: module
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .to_path_buf(),
                    declaring_module: module.to_path_buf(),
                    modifier: protect_modifier_range(&source, name),
                });
            }
            ExportDecl::Re {
                exported,
                imported,
                source,
                ..
            } if exported == name => {
                if source.starts_with("tyrus:") {
                    return None;
                }
                let next = crate::project::resolve_import_path(module, source.as_str())
                    .ok()??
                    .canonicalize()
                    .ok()?;
                return protected_declaration(&next, imported, visited);
            }
            ExportDecl::All { source, .. } if !source.starts_with("tyrus:") => {
                wildcards.push(source.clone());
            }
            _ => {}
        }
    }
    if name == DEFAULT_EXPORT {
        return None;
    }
    for source in wildcards {
        let Ok(Some(next)) = crate::project::resolve_import_path(module, source.as_str()) else {
            continue;
        };
        let Ok(next) = next.canonicalize() else {
            continue;
        };
        if let Some(found) = protected_declaration(&next, name, visited) {
            return Some(found);
        }
    }
    None
}

fn display_relative(path: &Path, scope: &Path) -> String {
    scope
        .parent()
        .and_then(|base| path.strip_prefix(base).ok())
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

fn protected_export_error(name: &str, source: &str, importer: &Path, scope: &Path) -> String {
    let folder = display_relative(scope, scope);
    let importing = display_relative(importer, scope);
    format!(
        "error[E3308]: `{name}` is a protected export of `{source}` and cannot be imported from `{importing}`\n\
         note: `export protect` limits `{name}` to `{folder}` and its subfolders\n\
         help: drop `protect` from the declaration of `{name}`, or move `{importing}` into `{folder}`"
    )
}

fn is_http_route_plugin(ty: &Type) -> bool {
    let Type::Function { params, ret } = ty else {
        return false;
    };
    matches!(params.as_slice(), [Type::Named(name)] if name == "HttpRouter")
        && matches!(ret.as_ref(), Type::Void)
}

fn collect_route_modules(directory: &Path, modules: &mut Vec<PathBuf>) -> Result<(), String> {
    if !directory.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(directory).map_err(|error| {
        format!(
            "error: could not read route directory `{}`: {error}",
            directory.display()
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "error: could not read an entry in route directory `{}`: {error}",
                directory.display()
            )
        })?;
        let file_type = entry.file_type().map_err(|error| {
            format!(
                "error: could not inspect `{}`: {error}",
                entry.path().display()
            )
        })?;
        let path = entry.path();
        if file_type.is_dir() {
            collect_route_modules(&path, modules)?;
        } else if file_type.is_file()
            && path.extension().is_some_and(|extension| extension == "tyr")
            && !path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .is_some_and(|stem| stem.ends_with(".test"))
        {
            modules.push(path);
        }
    }
    Ok(())
}

fn ensure_tyrus_source(path: &Path) -> Result<(), String> {
    if path.extension().is_some_and(|extension| extension == "tyr") {
        return Ok(());
    }
    Err(format!(
        "error: unsupported source file `{}`; Tyrus source files must use the `.tyr` extension",
        path.display()
    ))
}

fn item_name(item: &Item) -> Option<&str> {
    match item {
        Item::FnDecl { name, .. }
        | Item::Stmt(Stmt {
            kind: StmtKind::VarDecl { name, .. },
            ..
        })
        | Item::TypeAlias { name, .. }
        | Item::EnumDecl { name, .. }
        | Item::InterfaceDecl { name, .. }
        | Item::BehaviorInterfaceDecl { name, .. } => Some(name),
        _ => None,
    }
}

fn item_type(item: &Item) -> Option<Type> {
    match item {
        Item::FnDecl {
            params, ret_type, ..
        } => Some(Type::Function {
            params: params.iter().map(|param| param.ty.clone()).collect(),
            ret: Box::new(ret_type.clone()),
        }),
        Item::Stmt(Stmt {
            kind: StmtKind::VarDecl { ty, .. },
            ..
        }) => Some(ty.clone()),
        Item::TypeAlias { ty, .. } => Some(ty.clone()),
        Item::EnumDecl { name, .. }
        | Item::InterfaceDecl { name, .. }
        | Item::BehaviorInterfaceDecl { name, .. } => Some(Type::Named(name.clone())),
        _ => None,
    }
}

/// The consumer of an export never sees the declaring module's items, so an
/// interface has to cross the boundary already flattened: with the raw fields,
/// everything it inherits through `extends` is lost on the other side, and an
/// intersection alias, whose operands all live in `extends`, arrives empty.
fn item_interface_fields(
    item: &Item,
    defs: &crate::type_resolve::GenericDefs,
) -> Option<Vec<Field>> {
    match item {
        Item::InterfaceDecl { name, fields, .. } => {
            let mut ignored = Vec::new();
            Some(
                defs.interface_fields(name, &mut ignored)
                    .unwrap_or_else(|| fields.clone()),
            )
        }
        Item::BehaviorInterfaceDecl { .. } => Some(Vec::new()),
        _ => None,
    }
}

fn register_implicit_timestamp_types(
    program: &Program,
    imported_models: &HashMap<String, DbModelMetadata>,
    import_types: &mut HashMap<String, Type>,
    import_interfaces: &mut HashMap<String, Vec<Field>>,
    reachable_interfaces: &mut HashMap<String, Vec<Field>>,
) {
    let imported_timestamps = imported_models.values().any(|model| model.timestamps);
    let local_timestamps = program.items.iter().any(|item| {
        matches!(
            item,
            Item::InterfaceDecl {
                db_model: Some(model),
                ..
            } if model.timestamps
        )
    });
    if !imported_timestamps && !local_timestamps {
        return;
    }
    let Some(instant) =
        native_builtins("tyrus:temporal").and_then(|exports| exports.get("Instant"))
    else {
        return;
    };
    import_types
        .entry("Instant".to_string())
        .or_insert_with(|| instant.ty.clone());
    if let Some(fields) = &instant.interface_fields {
        import_interfaces
            .entry("Instant".to_string())
            .or_insert_with(|| fields.clone());
        reachable_interfaces
            .entry("Instant".to_string())
            .or_insert_with(|| fields.clone());
    }
}

fn register_implicit_db_error_fields(
    program: &Program,
    imported_models: &HashMap<String, DbModelMetadata>,
    import_interfaces: &mut HashMap<String, Vec<Field>>,
) {
    let local_models = program.items.iter().any(|item| {
        matches!(
            item,
            Item::InterfaceDecl {
                db_model: Some(_),
                ..
            }
        )
    });
    let imports_db_api = program.imports.iter().any(|import| {
        import.source == "tyrus:db"
            && import.specifiers.iter().any(|specifier| {
                specifier.imported == "db" || is_namespace_specifier(&import.source, specifier)
            })
    });
    if imported_models.is_empty() && !local_models && !imports_db_api {
        return;
    }
    import_interfaces
        .entry("DbError".to_string())
        .or_insert_with(crate::db::db_error_fields);
}

fn item_db_model(item: &Item) -> Option<crate::ast::DbModelMetadata> {
    match item {
        Item::InterfaceDecl { db_model, .. } => db_model.clone(),
        _ => None,
    }
}

pub const ENTRY_MODULE_ID: usize = 0;

pub(crate) fn module_symbol(module_id: usize, name: &str) -> String {
    format!("__tyrus_module_{module_id}_{name}")
}

pub fn entry_symbol(name: &str) -> String {
    if name == "main" {
        name.to_string()
    } else {
        module_symbol(ENTRY_MODULE_ID, name)
    }
}

pub fn original_global_name(symbol: &str) -> &str {
    symbol
        .strip_prefix("__tyrus_module_")
        .and_then(|rest| rest.split_once('_'))
        .filter(|(module_id, _)| {
            !module_id.is_empty() && module_id.chars().all(|c| c.is_ascii_digit())
        })
        .map(|(_, name)| name)
        .unwrap_or(symbol)
}

pub fn behavior_interface_info(item: &Item, items: &[Item]) -> Option<BehaviorInterfaceInfo> {
    let Item::BehaviorInterfaceDecl {
        targets,
        extends,
        methods,
        bindings,
        ..
    } = item
    else {
        return None;
    };
    let binding_methods = bindings
        .iter()
        .filter_map(|binding| {
            let item = items.iter().find(|item| match item {
                Item::FnDecl { name, .. } => name == binding,
                Item::Stmt(Stmt {
                    kind: StmtKind::VarDecl { name, .. },
                    ..
                }) => name == binding,
                _ => false,
            })?;
            match item {
                Item::FnDecl {
                    name,
                    type_params,
                    params,
                    ret_type,
                    throws,
                    span,
                    ..
                } => Some(BehaviorMethod {
                    name: name.clone(),
                    type_params: type_params.clone(),
                    params: params.clone(),
                    ret_type: ret_type.clone(),
                    throws: throws.clone(),
                    body: Vec::new(),
                    span: *span,
                }),
                Item::Stmt(Stmt {
                    kind:
                        StmtKind::VarDecl {
                            value:
                                Some(Expr {
                                    kind:
                                        ExprKind::ArrowFn {
                                            params, ret_type, ..
                                        },
                                    ..
                                }),
                            ..
                        },
                    span,
                }) => Some(BehaviorMethod {
                    name: binding.clone(),
                    type_params: Vec::new(),
                    params: params.clone(),
                    ret_type: ret_type.clone(),
                    throws: Vec::new(),
                    body: Vec::new(),
                    span: *span,
                }),
                _ => None,
            }
        })
        .collect();
    Some(BehaviorInterfaceInfo {
        targets: targets.clone(),
        extends: extends.clone(),
        methods: methods.clone(),
        bindings: bindings.clone(),
        binding_methods,
    })
}

fn item_throws(item: &Item) -> Vec<Type> {
    let Item::FnDecl { params, throws, .. } = item else {
        return Vec::new();
    };
    let mut effects = throws.clone();
    if params.iter().any(|param| !param.decorators.is_empty())
        && !effects
            .iter()
            .any(|ty| ty == &Type::Named("ValidationError".to_string()))
    {
        effects.push(Type::Named("ValidationError".to_string()));
    }
    effects
}

fn item_type_alias_ty(item: &Item) -> Option<Type> {
    match item {
        Item::TypeAlias { ty, .. } => Some(ty.clone()),
        _ => None,
    }
}

fn item_enum_info(item: &Item) -> Option<ResolvedEnum> {
    match item {
        Item::EnumDecl { repr, members, .. } => Some(resolve_enum(repr, members).0),
        _ => None,
    }
}

struct Renamer {
    symbols: HashMap<String, String>,
    scopes: Vec<HashSet<String>>,
    namespaces: HashSet<String>,
    validator_bindings: HashSet<String>,
}

impl Renamer {
    fn new(symbols: HashMap<String, String>) -> Self {
        Self {
            symbols,
            scopes: Vec::new(),
            namespaces: HashSet::default(),
            validator_bindings: HashSet::default(),
        }
    }

    fn namespace_of(&self, expr: &Expr) -> Option<String> {
        match &expr.kind {
            ExprKind::Ident(name)
                if self.namespaces.contains(name)
                    && !self.scopes.iter().any(|scope| scope.contains(name)) =>
            {
                Some(name.clone())
            }
            _ => None,
        }
    }

    fn collapse_namespace_access(&mut self, expr: &mut Expr) {
        match &mut expr.kind {
            ExprKind::FieldAccess { object, field } => {
                if let Some(namespace) = self.namespace_of(object) {
                    expr.kind = ExprKind::Ident(namespace_member_binding(&namespace, field));
                }
            }
            ExprKind::MethodCall {
                object,
                method,
                type_args,
                args,
                ..
            } => {
                if let Some(namespace) = self.namespace_of(object) {
                    let callee = namespace_member_binding(&namespace, method);
                    expr.kind = ExprKind::FnCall {
                        callee,
                        type_args: std::mem::take(type_args),
                        args: std::mem::take(args),
                    };
                }
            }
            _ => {}
        }
    }

    fn rename_item(&mut self, item: Item) -> Item {
        match item {
            Item::InterfaceDecl {
                name,
                mut type_params,
                mut extends,
                mut fields,
                db_model,
                from_type_alias,
                span,
            } => {
                self.rename_type_param_constraints(&mut type_params);
                for ty in &mut extends {
                    self.rename_type(ty);
                }
                for field in &mut fields {
                    self.rename_type(&mut field.ty);
                    self.rename_decorators(&mut field.decorators);
                }
                Item::InterfaceDecl {
                    name: self.rename_global(&name),
                    type_params,
                    extends,
                    fields,
                    db_model,
                    from_type_alias,
                    span,
                }
            }
            Item::BehaviorInterfaceDecl {
                name,
                mut type_params,
                mut targets,
                mut extends,
                mut methods,
                mut bindings,
                span,
            } => {
                self.rename_type_param_constraints(&mut type_params);
                for target in &mut targets {
                    self.rename_type(target);
                }
                for supertype in &mut extends {
                    self.rename_type(supertype);
                }
                for method in &mut methods {
                    self.rename_type_param_constraints(&mut method.type_params);
                    for param in &mut method.params {
                        self.rename_type(&mut param.ty);
                        self.rename_decorators(&mut param.decorators);
                    }
                    self.rename_type(&mut method.ret_type);
                    for ty in &mut method.throws {
                        self.rename_type(ty);
                    }
                    self.push_scope();
                    for param in &method.params {
                        for binding in param.pattern.binding_names() {
                            self.declare_local(binding);
                        }
                    }
                    self.rename_stmts(&mut method.body);
                    self.pop_scope();
                }
                for binding in &mut bindings {
                    *binding = self.rename_global(binding);
                }
                Item::BehaviorInterfaceDecl {
                    name: self.rename_global(&name),
                    type_params,
                    targets,
                    extends,
                    methods,
                    bindings,
                    span,
                }
            }
            Item::TypeAlias {
                name,
                mut type_params,
                mut ty,
                mut merged_fields,
                span,
            } => {
                self.rename_type_param_constraints(&mut type_params);
                self.rename_type(&mut ty);
                for field in &mut merged_fields {
                    self.rename_type(&mut field.ty);
                    self.rename_decorators(&mut field.decorators);
                }
                Item::TypeAlias {
                    name: self.rename_global(&name),
                    type_params,
                    ty,
                    merged_fields,
                    span,
                }
            }
            Item::EnumDecl {
                name,
                mut repr,
                members,
                span,
            } => {
                self.rename_type(&mut repr);
                Item::EnumDecl {
                    name: self.rename_global(&name),
                    repr,
                    members,
                    span,
                }
            }
            Item::FnDecl {
                name,
                is_async,
                is_comptime,
                is_validator,
                mut type_params,
                mut params,
                mut ret_type,
                mut throws,
                mut body,
                span,
            } => {
                let renamed = self.rename_global(&name);
                self.rename_type_param_constraints(&mut type_params);
                for param in &mut params {
                    self.rename_type(&mut param.ty);
                    self.rename_decorators(&mut param.decorators);
                }
                self.rename_type(&mut ret_type);
                for ty in &mut throws {
                    self.rename_type(ty);
                }
                self.push_scope();
                for param in &params {
                    for name in param.pattern.binding_names() {
                        self.declare_local(name);
                    }
                }
                self.rename_stmts(&mut body);
                self.pop_scope();
                Item::FnDecl {
                    name: renamed,
                    is_async,
                    is_comptime,
                    is_validator,
                    type_params,
                    params,
                    ret_type,
                    throws,
                    body,
                    span,
                }
            }
            Item::Stmt(Stmt {
                kind:
                    StmtKind::VarDecl {
                        mutable,
                        name,
                        mut ty,
                        mut value,
                        onfail,
                    },
                span,
            }) => {
                self.rename_type(&mut ty);
                if let Some(value) = &mut value {
                    self.rename_expr(value);
                }
                Item::Stmt(Stmt {
                    kind: StmtKind::VarDecl {
                        mutable,
                        name: self.rename_global(&name),
                        ty,
                        value,
                        onfail,
                    },
                    span,
                })
            }
            Item::Stmt(mut stmt) => {
                self.rename_stmt(&mut stmt);
                Item::Stmt(stmt)
            }
        }
    }

    fn rename_stmts(&mut self, statements: &mut [Stmt]) {
        for statement in statements {
            self.rename_stmt(statement);
        }
    }

    fn rename_stmt(&mut self, statement: &mut Stmt) {
        match &mut statement.kind {
            StmtKind::VarDecl {
                name, ty, value, ..
            } => {
                self.rename_type(ty);
                if let Some(value) = value {
                    self.rename_expr(value);
                }
                self.declare_local(name);
            }
            StmtKind::Assign { name, value, .. } => {
                *name = self.rename_reference(name);
                self.rename_expr(value);
            }
            StmtKind::IndexAssign { name, index, value } => {
                *name = self.rename_reference(name);
                self.rename_expr(index);
                self.rename_expr(value);
            }
            StmtKind::FieldAssign { object, value, .. } => {
                self.rename_expr(object);
                self.rename_expr(value);
            }
            StmtKind::Return(Some(expr)) | StmtKind::Expr(expr) => self.rename_expr(expr),
            StmtKind::Return(None) => {}
            StmtKind::Break(value) => {
                if let Some(value) = value {
                    self.rename_expr(value);
                }
            }
            StmtKind::Continue => {}
            StmtKind::ArrayDestructure {
                bindings, value, ..
            }
            | StmtKind::ObjectDestructure {
                bindings, value, ..
            } => {
                self.rename_expr(value);
                for binding in bindings {
                    self.declare_local(&binding.name);
                }
            }
            StmtKind::While { cond, body } => {
                self.rename_expr(cond);
                self.push_scope();
                self.rename_stmts(body);
                self.pop_scope();
            }
            StmtKind::DoWhile { body, cond } => {
                self.push_scope();
                self.rename_stmts(body);
                self.pop_scope();
                self.rename_expr(cond);
            }
            StmtKind::For {
                init,
                cond,
                update,
                body,
            } => {
                self.push_scope();
                self.rename_stmt(init);
                self.rename_expr(cond);
                self.rename_stmt(update);
                self.push_scope();
                self.rename_stmts(body);
                self.pop_scope();
                self.pop_scope();
            }
            StmtKind::ForOf {
                name,
                iterable,
                body,
                ..
            } => {
                self.rename_expr(iterable);
                self.push_scope();
                self.declare_local(name);
                self.rename_stmts(body);
                self.pop_scope();
            }
            StmtKind::ForIn { name, object, body } => {
                self.rename_expr(object);
                self.push_scope();
                self.declare_local(name);
                self.rename_stmts(body);
                self.pop_scope();
            }
            StmtKind::Switch {
                expr,
                cases,
                default,
                ..
            } => {
                self.rename_expr(expr);
                for case in cases {
                    self.rename_expr(&mut case.value);
                    for stmt in &mut case.body {
                        self.rename_stmt(stmt);
                    }
                }
                if let Some(default_body) = default {
                    self.rename_stmts(default_body);
                }
            }
            StmtKind::Match {
                expr,
                arms,
                default,
            } => {
                self.rename_expr(expr);
                for arm in arms {
                    if let Some(guard) = &mut arm.guard {
                        self.rename_expr(guard);
                    }
                    self.rename_stmts(&mut arm.body);
                }
                if let Some(default_body) = default {
                    self.rename_stmts(default_body);
                }
            }
            StmtKind::Throw(expr) => {
                self.rename_expr(expr);
            }
            StmtKind::TryCatch {
                try_body,
                catch_type,
                catch_body,
                finally_body,
                ..
            } => {
                self.rename_type(catch_type);
                self.rename_stmts(try_body);
                self.rename_stmts(catch_body);
                if let Some(stmts) = finally_body {
                    self.rename_stmts(stmts);
                }
            }
            StmtKind::Defer { body, .. } => {
                self.push_scope();
                self.rename_stmts(body);
                self.pop_scope();
            }
        }
    }

    fn rename_expr(&mut self, expr: &mut Expr) {
        if !self.namespaces.is_empty() {
            self.collapse_namespace_access(expr);
        }
        match &mut expr.kind {
            ExprKind::ObjectLit(fields) => {
                for (_, value) in fields {
                    self.rename_expr(value);
                }
            }
            ExprKind::ObjectUpdate { base, fields } => {
                self.rename_expr(base);
                for (_, value) in fields {
                    self.rename_expr(value);
                }
            }
            ExprKind::ArrayLit(values) => {
                for value in values {
                    self.rename_expr(value);
                }
            }
            ExprKind::ArrayRepeat { value, count } => {
                self.rename_expr(value);
                self.rename_expr(count);
            }
            ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
                self.rename_expr(object);
            }
            ExprKind::IndexAccess { object, index } => {
                self.rename_expr(object);
                self.rename_expr(index);
            }
            ExprKind::Ident(name) => *name = self.rename_reference(name),
            ExprKind::Borrow { name, .. } => *name = self.rename_reference(name),
            ExprKind::CallValue {
                callee,
                type_args,
                args,
            } => {
                self.rename_expr(callee);
                for ty in type_args {
                    self.rename_type(ty);
                }
                for arg in args {
                    self.rename_expr(arg);
                }
            }
            ExprKind::FnCall {
                callee,
                type_args,
                args,
            } => {
                *callee = self.rename_reference(callee);
                for ty in type_args {
                    self.rename_type(ty);
                }
                for arg in args {
                    self.rename_expr(arg);
                }
            }
            ExprKind::BinOp { left, right, .. } => {
                self.rename_expr(left);
                self.rename_expr(right);
            }
            ExprKind::ChainedComparison { terms, .. } => {
                for term in terms {
                    self.rename_expr(term);
                }
            }
            ExprKind::Pipeline { left, right } => {
                self.rename_expr(left);
                self.rename_expr(right);
            }
            ExprKind::Not(inner)
            | ExprKind::Neg(inner)
            | ExprKind::BitNot(inner)
            | ExprKind::Typeof(inner)
            | ExprKind::Unwrap(inner)
            | ExprKind::Try(inner)
            | ExprKind::Spread(inner) => self.rename_expr(inner),
            ExprKind::TypeAssert { value, ty } => {
                self.rename_expr(value);
                self.rename_type(ty);
            }
            ExprKind::Is { expr, ty } => {
                self.rename_expr(expr);
                self.rename_type(ty);
            }
            ExprKind::ConstAssert(inner) | ExprKind::Comptime(inner) => self.rename_expr(inner),
            ExprKind::PrefixIncr(inner)
            | ExprKind::PrefixDecr(inner)
            | ExprKind::PostfixIncr(inner)
            | ExprKind::PostfixDecr(inner) => self.rename_expr(inner),
            ExprKind::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                self.rename_expr(cond);
                self.rename_expr(then_expr);
                self.rename_expr(else_expr);
            }
            ExprKind::TemplateLit(parts) => {
                for part in parts {
                    if let TemplatePart::Expr(expr, _) = part {
                        self.rename_expr(expr);
                    }
                }
            }
            ExprKind::MethodCall {
                object,
                type_args,
                args,
                ..
            }
            | ExprKind::OptionalMethodCall {
                object,
                type_args,
                args,
                ..
            } => {
                self.rename_expr(object);
                for ty in type_args {
                    self.rename_type(ty);
                }
                for arg in args {
                    self.rename_expr(arg);
                }
            }
            ExprKind::NewSet { ty, initial } => {
                if let Some(ty) = ty {
                    self.rename_type(ty);
                }
                if let Some(initial) = initial {
                    self.rename_expr(initial);
                }
            }
            ExprKind::NewMap {
                key,
                value,
                initial,
            } => {
                if let Some(key) = key {
                    self.rename_type(key);
                }
                if let Some(value) = value {
                    self.rename_type(value);
                }
                if let Some(initial) = initial {
                    self.rename_expr(initial);
                }
            }
            ExprKind::ArrowFn {
                type_params,
                params,
                ret_type,
                throws,
                body,
                ..
            } => {
                self.rename_type_param_constraints(type_params);
                self.push_scope();
                for param in params {
                    self.rename_type(&mut param.ty);
                    for name in param.pattern.binding_names() {
                        self.declare_local(name);
                    }
                }
                self.rename_type(ret_type);
                for ty in throws {
                    self.rename_type(ty);
                }
                self.rename_stmts(body);
                self.pop_scope();
            }
            ExprKind::If {
                cond,
                then_block,
                else_block,
            } => {
                self.rename_expr(cond);
                self.push_scope();
                self.rename_stmts(then_block);
                self.pop_scope();
                if let Some(block) = else_block {
                    self.push_scope();
                    self.rename_stmts(block);
                    self.pop_scope();
                }
            }
            ExprKind::Loop { body } => {
                self.push_scope();
                self.rename_stmts(body);
                self.pop_scope();
            }
            ExprKind::In { key, object } => {
                self.rename_expr(key);
                self.rename_expr(object);
            }
            ExprKind::Range { start, end, .. } => {
                self.rename_expr(start);
                self.rename_expr(end);
            }
            ExprKind::Await(inner) => {
                self.rename_expr(inner);
            }
            ExprKind::StrLit(_)
            | ExprKind::RegexLit { .. }
            | ExprKind::IntLit(_)
            | ExprKind::UIntLit(_)
            | ExprKind::FloatLit(_)
            | ExprKind::DecimalLit(_)
            | ExprKind::BoolLit(_)
            | ExprKind::Null
            | ExprKind::None
            | ExprKind::Topic => {}
        }
    }

    fn rename_decorators(&self, decorators: &mut [crate::ast::ValidatorDecorator]) {
        for decorator in decorators {
            if self.validator_bindings.contains(&decorator.name)
                && let Some(target) = self.symbols.get(&decorator.name)
            {
                decorator.name = target.clone();
            }
        }
    }

    fn rename_type(&mut self, ty: &mut Type) {
        match ty {
            Type::Named(name) => {
                if name == "Semaphore" {
                    return;
                }
                let renamed = self.rename_global(name);
                if !crate::algorithm::is_algorithm_type_name(name)
                    || renamed.starts_with("__tyrus_module_")
                {
                    *name = renamed;
                }
            }
            Type::Array(inner)
            | Type::Readonly(inner)
            | Type::Ref(inner)
            | Type::MutRef(inner)
            | Type::Optional(inner)
            | Type::KeyOf(inner)
            | Type::Set(inner)
            | Type::BinaryHeap(inner)
            | Type::BinomialHeap(inner)
            | Type::UnionFind(inner)
            | Type::Counter(inner)
            | Type::Deque(inner)
            | Type::LinkedList(inner)
            | Type::DoubleLinkedList(inner)
            | Type::AvlTree(inner)
            | Type::RedBlackTree(inner)
            | Type::SplayTree(inner)
            | Type::Treap(inner)
            | Type::SortedSet(inner)
            | Type::SkipList(inner)
            | Type::Range(inner) => self.rename_type(inner),
            Type::Throws { function, effects } => {
                self.rename_type(function);
                for effect in effects {
                    self.rename_type(effect);
                }
            }
            Type::Result { success, error } => {
                self.rename_type(success);
                self.rename_type(error);
            }
            Type::SortedMap(key, value) => {
                self.rename_type(key);
                self.rename_type(value);
            }
            Type::Graph(node, weight) => {
                self.rename_type(node);
                self.rename_type(weight);
            }
            Type::Tuple(elements) | Type::Union(elements) => {
                for element in elements {
                    self.rename_type(element);
                }
            }
            Type::Function { params, ret } | Type::GenericFunction { params, ret, .. } => {
                for param in params {
                    self.rename_type(param);
                }
                self.rename_type(ret);
            }
            Type::Object(fields) => {
                for field in fields {
                    self.rename_type(&mut field.ty);
                    self.rename_decorators(&mut field.decorators);
                }
            }
            Type::Apply { name, args } => {
                if !matches!(
                    name.as_str(),
                    "Mutex" | "MutexGuard" | "RwLock" | "RwReadGuard" | "RwWriteGuard"
                ) {
                    let renamed = self.rename_global(name);
                    if !crate::algorithm::is_algorithm_type_name(name)
                        || renamed.starts_with("__tyrus_module_")
                    {
                        *name = renamed;
                    }
                }
                for arg in args {
                    self.rename_type(arg);
                }
            }
            Type::Conditional {
                check,
                bound,
                then,
                otherwise,
            } => {
                self.rename_type(check);
                self.rename_type(bound);
                self.rename_type(then);
                self.rename_type(otherwise);
            }
            Type::Map(key, value) => {
                self.rename_type(key);
                self.rename_type(value);
            }
            Type::TypeQuery(expr) | Type::Lit(expr) => self.rename_expr(expr),
            Type::Infer
            | Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::I128
            | Type::Isize
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::Usize
            | Type::F32
            | Type::F64
            | Type::F128
            | Type::Decimal
            | Type::Char
            | Type::Bool
            | Type::Regex
            | Type::Str
            | Type::Void
            | Type::Generic(_)
            | Type::Unknown
            | Type::Never
            | Type::InferVar(_)
            | Type::FenwickTree
            | Type::BubbleSort
            | Type::SegmentTree
            | Type::SegmentTreeLazy
            | Type::SparseTable
            | Type::BitSet
            | Type::Trie
            | Type::BloomFilter
            | Type::SuffixTree
            | Type::Lca
            | Type::Hld
            | Type::Centroid => {}
        }
    }

    fn rename_type_param_constraints(&mut self, type_params: &mut [TypeParam]) {
        for type_param in type_params {
            if let Some(constraint) = &mut type_param.constraint {
                self.rename_type(constraint);
            }
        }
    }

    fn rename_global(&self, name: &str) -> String {
        self.symbols
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }

    fn rename_reference(&self, name: &str) -> String {
        if self.scopes.iter().rev().any(|scope| scope.contains(name)) {
            name.to_string()
        } else {
            self.rename_global(name)
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashSet::default());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare_local(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string());
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/module_loader.rs"]
mod tests;

#[cfg(test)]
mod implicit_id_tests {
    use super::*;

    #[test]
    fn imported_models_make_inferred_db_error_fields_available() {
        let model_program = crate::parser::Parser::from_source(
            "@model(\"users\") export interface User { name: string; }",
        )
        .expect("model source should lex")
        .parse_program()
        .expect("model source should parse");
        let Item::InterfaceDecl {
            db_model: Some(model),
            ..
        } = &model_program.items[0]
        else {
            panic!("expected a database Model declaration");
        };
        let mut imported_models = HashMap::default();
        imported_models.insert("User".to_string(), model.clone());
        let consumer_program =
            crate::parser::Parser::from_source("import { User } from \"#/models/users\";")
                .expect("consumer source should lex")
                .parse_program()
                .expect("consumer source should parse");
        let mut interfaces = HashMap::default();

        register_implicit_db_error_fields(&consumer_program, &imported_models, &mut interfaces);

        let fields = interfaces
            .get("DbError")
            .expect("DbError fields should be available");
        assert!(fields.iter().any(|field| field.name == "code"));
        assert!(fields.iter().any(|field| field.name == "message"));
    }

    #[test]
    fn importing_db_registers_inferred_db_error_fields_without_model_imports() {
        let program = crate::parser::Parser::from_source("import { db } from \"tyrus:db\";")
            .expect("consumer source should lex")
            .parse_program()
            .expect("consumer source should parse");
        let mut interfaces = HashMap::default();

        register_implicit_db_error_fields(&program, &HashMap::default(), &mut interfaces);

        let fields = interfaces
            .get("DbError")
            .expect("DbError fields should be available");
        assert_eq!(fields.len(), 2);
    }

    #[test]
    fn imported_models_make_implicit_instant_available_to_type_checking() {
        let model_program = crate::parser::Parser::from_source(
            "@model(\"users\") export interface User { name: string; }",
        )
        .expect("model source should lex")
        .parse_program()
        .expect("model source should parse");
        let Item::InterfaceDecl {
            db_model: Some(model),
            ..
        } = &model_program.items[0]
        else {
            panic!("expected a database Model declaration");
        };
        let mut imported_models = HashMap::default();
        imported_models.insert("User".to_string(), model.clone());
        let consumer_program = crate::parser::Parser::from_source("const ready = true;")
            .expect("consumer source should lex")
            .parse_program()
            .expect("consumer source should parse");
        let mut types = HashMap::default();
        let mut interfaces = HashMap::default();
        let mut reachable_interfaces = HashMap::default();

        register_implicit_timestamp_types(
            &consumer_program,
            &imported_models,
            &mut types,
            &mut interfaces,
            &mut reachable_interfaces,
        );

        assert_eq!(
            types.get("Instant"),
            Some(&Type::Named("Instant".to_string()))
        );
        assert!(interfaces.contains_key("Instant"));
        assert!(reachable_interfaces.contains_key("Instant"));
    }

    #[test]
    fn sqlite_models_receive_an_implicit_readonly_i64_id() {
        let mut program = crate::parser::Parser::from_source(
            "@model(\"users\") export interface User { name: string; }",
        )
        .expect("model source should lex")
        .parse_program()
        .expect("model source should parse");
        crate::db::apply_backend_model_defaults(
            &mut program,
            Some(&crate::db::DatabaseBackend::Sqlite),
        );

        let Item::InterfaceDecl {
            fields,
            db_model: Some(model),
            ..
        } = &program.items[0]
        else {
            panic!("expected a database Model declaration");
        };
        let id = fields
            .iter()
            .find(|field| field.name == "id")
            .expect("SQL backend should synthesize the id property");
        assert_eq!(id.ty, Type::I64);
        assert!(id.readonly);
        let id_metadata = model
            .fields
            .iter()
            .find(|field| field.field == "id")
            .expect("SQL backend should synthesize ID metadata");
        assert!(id_metadata.id);
        assert_eq!(id_metadata.column, "id");
    }

    #[test]
    fn server_sql_models_receive_the_default_implicit_i64_id() {
        for backend in [
            crate::db::DatabaseBackend::Mysql,
            crate::db::DatabaseBackend::Postgresql,
        ] {
            let mut program = crate::parser::Parser::from_source(
                "@model(\"users\") export interface User { name: string; }",
            )
            .expect("model source should lex")
            .parse_program()
            .expect("model source should parse");
            crate::db::apply_backend_model_defaults(&mut program, Some(&backend));

            let Item::InterfaceDecl {
                fields,
                db_model: Some(model),
                ..
            } = &program.items[0]
            else {
                panic!("expected a database Model declaration");
            };
            let id = fields
                .iter()
                .find(|field| field.name == "id")
                .expect("server SQL backend should synthesize id");
            assert_eq!(id.ty, Type::I64);
            assert!(id.readonly);
            let id_metadata = model
                .fields
                .iter()
                .find(|field| field.field == "id")
                .expect("server SQL backend should synthesize ID metadata");
            assert!(id_metadata.id);
            assert!(id_metadata.id_strategy.is_none());
        }
    }

    #[test]
    fn mongodb_model_results_include_an_implicit_readonly_object_id() {
        let result = crate::db::mongo_model_result_type(&[Field {
            name: "email".to_string(),
            ty: Type::Str,
            readonly: false,
            optional: false,
            decorators: Vec::new(),
            span: Span::DUMMY,
        }]);
        let Type::Object(fields) = result else {
            panic!("MongoDB Model result should be an object type");
        };
        let id = fields
            .iter()
            .find(|field| field.name == "_id")
            .expect("MongoDB Model result should include _id");
        assert_eq!(id.ty, Type::Named("ObjectId".to_string()));
        assert!(id.readonly);
    }
}
