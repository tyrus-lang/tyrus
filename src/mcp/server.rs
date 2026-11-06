use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, GetPromptRequestParams, GetPromptResponse, GetPromptResult, ListPromptsResult,
    ListResourcesResult, Prompt, PromptArgument, PromptMessage, ReadResourceRequestParams,
    ReadResourceResponse, ReadResourceResult, Resource, ResourceContents, Role, ServerCapabilities,
    ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::transport::stdio;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, ServiceExt, tool, tool_handler, tool_router,
};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

use super::docs::DocsCache;

const MAX_SOURCE_BYTES: usize = 128 * 1024;
const MAX_QUERY_BYTES: usize = 512;
const ANALYSIS_TIMEOUT: Duration = Duration::from_secs(5);
const RESOURCE_INDEX: &str = "tyrus://docs/index";
const RESOURCE_LLM_INDEX: &str = "tyrus://llms.txt";
const RESOURCE_LLM_FULL: &str = "tyrus://llms-full.txt";

#[derive(Clone)]
pub struct TyrusMcpServer {
    cache: Arc<DocsCache>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SearchDocsArgs {
    #[schemars(description = "Words or phrases to find in the official Tyrus documentation.")]
    query: String,
    #[schemars(description = "Maximum number of results, from 1 to 20.")]
    max_results: Option<u32>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
struct GetPageArgs {
    #[schemars(description = "Relative Markdown path listed in the search index.")]
    path: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
struct CheckSourceArgs {
    #[schemars(
        description = "Standalone Tyrus source code. Project-relative imports are not supported."
    )]
    code: String,
}

impl TyrusMcpServer {
    pub fn new(cache: DocsCache) -> Self {
        Self {
            cache: Arc::new(cache),
        }
    }

    pub async fn serve(self) -> Result<(), String> {
        let server = ServiceExt::serve(self, stdio())
            .await
            .map_err(|error| format!("could not start the MCP stdio server: {error}"))?;
        server
            .waiting()
            .await
            .map_err(|error| format!("MCP server stopped unexpectedly: {error}"))?;
        Ok(())
    }
}

#[tool_router]
impl TyrusMcpServer {
    #[tool(
        description = "Search the official Tyrus documentation and return a small ranked set of relevant sections. This tool is read-only.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    fn search_docs(
        &self,
        Parameters(args): Parameters<SearchDocsArgs>,
    ) -> Result<CallToolResult, McpError> {
        if args.query.trim().is_empty() || args.query.len() > MAX_QUERY_BYTES {
            return Ok(CallToolResult::structured_error(json!({
                "error": "invalid_query",
                "message": format!("query must contain between 1 and {MAX_QUERY_BYTES} bytes"),
            })));
        }
        let max_results = args.max_results.unwrap_or(5).clamp(1, 20) as usize;
        let results = self.cache.search(&args.query, max_results);
        let count = results.len();
        Ok(CallToolResult::structured(json!({
            "query": args.query,
            "results": results,
            "count": count,
            "docsVersion": self.cache.docs_version,
        })))
    }

    #[tool(
        description = "Read one complete page from the cached official Tyrus documentation. The path must be returned by search_docs. This tool is read-only.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    fn get_page(
        &self,
        Parameters(args): Parameters<GetPageArgs>,
    ) -> Result<CallToolResult, McpError> {
        if args.path.len() > 512 {
            return Ok(CallToolResult::structured_error(json!({
                "error": "invalid_path",
                "message": "page path exceeds the maximum length",
            })));
        }
        let Some(markdown) = self.cache.page(&args.path) else {
            return Ok(CallToolResult::structured_error(json!({
                "error": "page_not_found",
                "message": "page path is not present in the documentation index",
            })));
        };
        if markdown.len() > super::docs::MAX_PAGE_BYTES {
            return Ok(CallToolResult::structured_error(json!({
                "error": "page_too_large",
                "message": "page exceeds the maximum response size",
            })));
        }
        Ok(CallToolResult::structured(json!({
            "path": args.path,
            "markdown": markdown,
            "docsVersion": self.cache.docs_version,
        })))
    }

    #[tool(
        description = "Parse, type-check, and borrow-check standalone Tyrus source in memory. It does not create files, load a project, or execute code. Project-relative imports are unsupported.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn check_source(
        &self,
        Parameters(args): Parameters<CheckSourceArgs>,
    ) -> Result<CallToolResult, McpError> {
        if args.code.len() > MAX_SOURCE_BYTES {
            return Ok(CallToolResult::structured_error(json!({
                "error": "source_too_large",
                "message": format!("source exceeds the {MAX_SOURCE_BYTES}-byte limit"),
                "maxBytes": MAX_SOURCE_BYTES,
            })));
        }
        let code = args.code;
        let worker_code = code.clone();
        let cancellation = Arc::new(AtomicBool::new(false));
        let worker_cancellation = Arc::clone(&cancellation);
        let (unsupported, analysis) = match tokio::time::timeout(
            ANALYSIS_TIMEOUT,
            tokio::task::spawn_blocking(move || analyze_source(worker_code, worker_cancellation)),
        )
        .await
        {
            Ok(Ok(Ok(result))) => result,
            Ok(Ok(Err(()))) => {
                return Ok(CallToolResult::structured_error(json!({
                    "error": "analysis_cancelled",
                    "message": "source analysis was cancelled",
                })));
            }
            Ok(Err(error)) => {
                return Ok(CallToolResult::structured_error(json!({
                    "error": "analysis_failed",
                    "message": format!("analysis worker failed: {error}"),
                })));
            }
            Err(_) => {
                cancellation.store(true, Ordering::Release);
                return Ok(CallToolResult::structured_error(json!({
                    "error": "analysis_timeout",
                    "message": "source analysis exceeded the 5-second limit",
                })));
            }
        };
        if let Some(reason) = unsupported {
            return Ok(CallToolResult::structured(json!({
                "ok": false,
                "status": "unsupported_context",
                "message": reason,
                "diagnostics": [],
            })));
        }
        let mut has_errors = false;
        let mut has_warnings = false;
        let diagnostics = analysis
            .diagnostics
            .iter()
            .map(|diagnostic| {
                let severity = match diagnostic.severity {
                    tyrus_core::analysis::DiagnosticSeverity::Error => {
                        has_errors = true;
                        "error"
                    }
                    tyrus_core::analysis::DiagnosticSeverity::Warning => {
                        has_warnings = true;
                        "warning"
                    }
                };
                let (start_line, start_column) = line_column(&code, diagnostic.range.start);
                let (end_line, end_column) = line_column(&code, diagnostic.range.end);
                let related = diagnostic
                    .related
                    .iter()
                    .map(|item| {
                        let (line, column) = line_column(&code, item.range.start);
                        json!({ "message": item.message, "line": line, "column": column })
                    })
                    .collect::<Vec<_>>();
                json!({
                    "code": diagnostic.code,
                    "severity": severity,
                    "message": diagnostic.message,
                    "line": start_line,
                    "column": start_column,
                    "endLine": end_line,
                    "endColumn": end_column,
                    "startByte": diagnostic.range.start,
                    "endByte": diagnostic.range.end,
                    "related": related,
                    "help": diagnostic.help,
                })
            })
            .collect::<Vec<_>>();
        let status = if has_errors {
            "errors"
        } else if has_warnings {
            "warnings"
        } else {
            "ok"
        };
        Ok(CallToolResult::structured(json!({
            "ok": !has_errors,
            "status": status,
            "diagnostics": diagnostics,
            "compilerVersion": env!("CARGO_PKG_VERSION"),
        })))
    }
}

#[tool_handler]
impl ServerHandler for TyrusMcpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
        )
        .with_server_info(rmcp::model::Implementation::new(
            "tyrus-mcp",
            env!("CARGO_PKG_VERSION"),
        ))
    }

    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let mut resources = vec![
            Resource::new(RESOURCE_INDEX, "Tyrus documentation index")
                .with_description("Compact list of official Tyrus documentation pages.")
                .with_mime_type("text/markdown"),
            Resource::new(RESOURCE_LLM_INDEX, "Tyrus llms.txt")
                .with_description("Compact machine-readable Tyrus documentation index.")
                .with_mime_type("text/plain"),
            Resource::new(RESOURCE_LLM_FULL, "Tyrus full documentation")
                .with_description(
                    "Complete documentation bundle. Read only when the full bundle is needed.",
                )
                .with_mime_type("text/markdown"),
        ];
        resources.extend(self.cache.pages.keys().map(|path| {
            Resource::new(format!("tyrus://docs/{path}"), path)
                .with_description("One page from the official Tyrus documentation.")
                .with_mime_type("text/markdown")
        }));
        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
            meta: None,
            ..Default::default()
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, McpError> {
        let text = if request.uri == RESOURCE_INDEX || request.uri == RESOURCE_LLM_INDEX {
            Some(self.cache.index_text.as_str())
        } else if request.uri == RESOURCE_LLM_FULL {
            Some(self.cache.full_text.as_str())
        } else if let Some(path) = request.uri.strip_prefix("tyrus://docs/") {
            self.cache.page(path)
        } else {
            None
        };
        let Some(text) = text else {
            return Err(McpError::resource_not_found(
                "documentation resource not found",
                Some(json!({ "uri": request.uri })),
            ));
        };
        Ok(ReadResourceResult::new(vec![ResourceContents::text(text, &request.uri)]).into())
    }

    async fn list_prompts(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult {
            prompts: vec![
                Prompt::new(
                    "generate_tyrus_code",
                    Some("Generate Tyrus code by consulting the official documentation."),
                    Some(vec![
                        PromptArgument::new("task")
                            .with_description("Describe the code to generate.")
                            .with_required(true),
                    ]),
                ),
                Prompt::new(
                    "fix_tyrus_code",
                    Some("Fix Tyrus code using diagnostics from check_source."),
                    Some(vec![
                        PromptArgument::new("code")
                            .with_description("Standalone Tyrus source to fix.")
                            .with_required(true),
                    ]),
                ),
            ],
            next_cursor: None,
            meta: None,
            ..Default::default()
        })
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResponse, McpError> {
        let (argument, instructions) = match request.name.as_str() {
            "generate_tyrus_code" => (
                request
                    .arguments
                    .as_ref()
                    .and_then(|arguments| arguments.get("task"))
                    .and_then(Value::as_str),
                "Use search_docs to find the relevant official Tyrus documentation. Generate code using Tyrus syntax, then call check_source on the standalone source. Explain any unsupported project-relative imports.",
            ),
            "fix_tyrus_code" => (
                request
                    .arguments
                    .as_ref()
                    .and_then(|arguments| arguments.get("code"))
                    .and_then(Value::as_str),
                "Call check_source on the supplied standalone Tyrus source. Correct the reported diagnostics and validate the corrected source again. Do not claim validation succeeded unless check_source returned status ok or warnings.",
            ),
            _ => {
                return Err(McpError::invalid_params(
                    "unknown Tyrus prompt",
                    Some(json!({ "name": request.name })),
                ));
            }
        };
        let Some(argument) = argument else {
            return Err(McpError::invalid_params(
                "required prompt argument is missing",
                Some(json!({ "name": request.name })),
            ));
        };
        let prompt_text = if request.name == "generate_tyrus_code" {
            format!("Task: {argument}\n\n{instructions}")
        } else {
            format!("Source:\n```tyrus\n{argument}\n```\n\n{instructions}")
        };
        Ok(GetPromptResponse::Complete(
            GetPromptResult::new(vec![PromptMessage::new_text(Role::User, prompt_text)])
                .with_description("Tyrus code generation and validation guidance."),
        ))
    }
}

fn analyze_source(
    source: String,
    cancellation: Arc<AtomicBool>,
) -> Result<(Option<String>, tyrus_core::analysis::DocumentAnalysis), ()> {
    match tyrus_core::analysis::parse_for_analysis_cancellable(&source, Arc::clone(&cancellation)) {
        Err(tyrus_core::analysis::ParseForAnalysis::Cancelled) => return Err(()),
        Err(_) => {}
        Ok((_, program)) => {
            for import in &program.imports {
                if !import.source.starts_with("tyrus:") {
                    return Ok((
                        Some(format!(
                            "Project-relative import `{}` needs a project context, which check_source does not load.",
                            import.source
                        )),
                        tyrus_core::analysis::DocumentAnalysis::default(),
                    ));
                }
                if import
                    .specifiers
                    .iter()
                    .any(|specifier| specifier.kind == tyrus_core::ast::ImportKind::Namespace)
                {
                    return Ok((
                        Some("Namespace imports need project module resolution and are not supported by check_source.".to_string()),
                        tyrus_core::analysis::DocumentAnalysis::default(),
                    ));
                }
                for specifier in &import.specifiers {
                    if tyrus_core::module_loader::native_export_analysis_info(
                        &import.source,
                        &specifier.imported,
                    )
                    .is_none()
                    {
                        return Ok((
                            Some(format!(
                                "Native export `{}::{}` is unavailable to standalone analysis.",
                                import.source, specifier.imported
                            )),
                            tyrus_core::analysis::DocumentAnalysis::default(),
                        ));
                    }
                }
            }
        }
    }
    let analysis =
        tyrus_core::analysis::analyze_standalone_source_cancellable(&source, cancellation)
            .map_err(|_| ())?;
    Ok((None, analysis))
}

fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(source.len());
    let prefix = &source[..source.floor_char_boundary(offset)];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix
        .rsplit('\n')
        .next()
        .unwrap_or_default()
        .chars()
        .count()
        + 1;
    (line, column)
}

pub async fn probe(executable: &Path, cache_root: &Path) -> Result<(), String> {
    let mut child = Command::new(executable)
        .arg("mcp")
        .arg("serve")
        .env("TYRUS_MCP_HOME", cache_root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| format!("could not launch the MCP server probe: {error}"))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "MCP probe has no stdin".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "MCP probe has no stdout".to_string())?;
    let mut input = stdin;
    let mut output = BufReader::new(stdout);
    let initialize = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": { "name": "tyrus-mcp-install-check", "version": env!("CARGO_PKG_VERSION") },
        },
    });
    let initialize_line = serde_json::to_vec(&initialize).map_err(|error| error.to_string())?;
    input
        .write_all(&initialize_line)
        .await
        .map_err(|error| format!("could not send MCP initialize: {error}"))?;
    input
        .write_all(b"\n")
        .await
        .map_err(|error| error.to_string())?;
    input.flush().await.map_err(|error| error.to_string())?;
    let mut response = String::new();
    let read_result = timeout(Duration::from_secs(5), output.read_line(&mut response)).await;
    match read_result {
        Ok(Ok(count)) if count > 0 => {}
        Ok(Ok(_)) => {
            let _ = child.kill().await;
            return Err("MCP server closed stdout before the initialize response".to_string());
        }
        Ok(Err(error)) => {
            let _ = child.kill().await;
            return Err(format!("could not read MCP initialize response: {error}"));
        }
        Err(_) => {
            let _ = child.kill().await;
            return Err("MCP server initialize handshake timed out".to_string());
        }
    }
    let parsed: Value = serde_json::from_str(response.trim())
        .map_err(|error| format!("MCP server returned invalid JSON during initialize: {error}"))?;
    if parsed.get("id") != Some(&json!(1)) || parsed.get("result").is_none() {
        let _ = child.kill().await;
        return Err("MCP server did not return a valid initialize result".to_string());
    }
    input
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\",\"params\":{}}\n")
        .await
        .map_err(|error| format!("could not send MCP ping: {error}"))?;
    input.flush().await.map_err(|error| error.to_string())?;
    response.clear();
    match timeout(Duration::from_secs(5), output.read_line(&mut response)).await {
        Ok(Ok(count)) if count > 0 => {}
        Ok(Ok(_)) => {
            let _ = child.kill().await;
            return Err("MCP server closed stdout before the ping response".to_string());
        }
        Ok(Err(error)) => {
            let _ = child.kill().await;
            return Err(format!("could not read MCP ping response: {error}"));
        }
        Err(_) => {
            let _ = child.kill().await;
            return Err("MCP server ping timed out".to_string());
        }
    }
    let parsed: Value = serde_json::from_str(response.trim())
        .map_err(|error| format!("MCP server returned invalid JSON during ping: {error}"))?;
    if parsed.get("id") != Some(&json!(2)) || parsed.get("result").is_none() {
        let _ = child.kill().await;
        return Err("MCP server did not return a valid ping result".to_string());
    }
    drop(input);
    match timeout(Duration::from_secs(3), child.wait()).await {
        Ok(Ok(status)) if status.success() => Ok(()),
        Ok(Ok(status)) => Err(format!("MCP server exited with status {status}")),
        Ok(Err(error)) => Err(format!("could not stop MCP server probe: {error}")),
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err("MCP server did not stop after stdio closed".to_string())
        }
    }
}

pub fn run_server(cache_root: PathBuf) -> Result<(), String> {
    let cache = DocsCache::load(&cache_root)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("could not create MCP runtime: {error}"))?;
    runtime.block_on(TyrusMcpServer::new(cache).serve())
}

#[cfg(test)]
mod tests {
    use super::line_column;

    #[test]
    fn diagnostics_use_one_based_character_positions() {
        assert_eq!(line_column("let café = 1;\nprint(café);", 12), (1, 12));
        assert_eq!(line_column("first\nsecond", 8), (2, 3));
    }
}
