use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;
use tempfile::TempDir;

struct LspProcess {
    child: Child,
    input: ChildStdin,
    output: Receiver<Value>,
    trace: Option<Receiver<String>>,
    trace_buffer: Vec<String>,
    progress: Vec<Value>,
    deferred: Vec<Value>,
    next_id: u64,
}

impl LspProcess {
    fn start(workspace: &Path) -> Self {
        Self::start_with_trace_option(workspace, false)
    }

    fn start_with_trace(workspace: &Path) -> Self {
        Self::start_with_trace_option(workspace, true)
    }

    fn start_with_trace_option(workspace: &Path, trace_enabled: bool) -> Self {
        Self::start_with_options(workspace, trace_enabled, false)
    }

    fn start_with_options(
        workspace: &Path,
        trace_enabled: bool,
        watched_files_dynamic_registration: bool,
    ) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_tyrus"));
        command
            .arg("lsp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(if trace_enabled {
                Stdio::piped()
            } else {
                Stdio::null()
            });
        if trace_enabled {
            command.env("TYRUS_LSP_TRACE", "1");
        }
        let mut child = command.spawn().expect("start tyrus lsp");
        let input = child.stdin.take().expect("LSP stdin");
        let mut stdout = BufReader::new(child.stdout.take().expect("LSP stdout"));
        let (sender, output) = mpsc::sync_channel(128);
        std::thread::spawn(move || {
            while let Some(message) = read_lsp_message(&mut stdout) {
                if sender.send(message).is_err() {
                    break;
                }
            }
        });
        let trace = child.stderr.take().map(|mut stderr| {
            let (sender, receiver) = mpsc::channel();
            std::thread::spawn(move || {
                let mut reader = BufReader::new(&mut stderr);
                let mut line = String::new();
                while reader.read_line(&mut line).is_ok_and(|count| count > 0) {
                    if sender.send(line.trim_end().to_string()).is_err() {
                        break;
                    }
                    line.clear();
                }
            });
            receiver
        });
        let mut server = Self {
            child,
            input,
            output,
            trace,
            trace_buffer: Vec::new(),
            progress: Vec::new(),
            deferred: Vec::new(),
            next_id: 1,
        };
        let root_uri = file_uri(workspace);
        let initialized = server.request(
            "initialize",
            json!({
                "processId": null,
                "rootUri": root_uri,
                "workspaceFolders": [{ "uri": root_uri, "name": "lsp-protocol" }],
                "capabilities": if watched_files_dynamic_registration {
                    json!({"workspace": {"didChangeWatchedFiles": {"dynamicRegistration": true}}})
                } else {
                    json!({})
                },
            }),
        );
        assert!(initialized.get("capabilities").is_some());
        assert_eq!(
            initialized["capabilities"]["workspace"]["workspaceFolders"]["supported"],
            true
        );
        server.notify("initialized", json!({}));
        server
    }

    fn drain_trace(&mut self) -> Vec<String> {
        let mut events = std::mem::take(&mut self.trace_buffer);
        if let Some(trace) = self.trace.as_ref() {
            events.extend(trace.try_iter());
        }
        events
    }

    fn wait_for_trace_event(&mut self, expected: &str) -> Vec<String> {
        let Some(trace) = self.trace.as_ref() else {
            panic!("trace output is not enabled");
        };
        while !self
            .trace_buffer
            .iter()
            .any(|event| event.contains(expected))
        {
            match trace.recv_timeout(Duration::from_secs(5)) {
                Ok(event) => self.trace_buffer.push(event),
                Err(error) => {
                    panic!("timed out waiting for trace event `{expected}`: {error}")
                }
            }
        }
        self.drain_trace()
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.send(json!({"jsonrpc": "2.0", "method": method, "params": params}));
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        loop {
            let message = self.receive();
            if message.get("id") == Some(&json!(id)) {
                assert!(
                    message.get("error").is_none(),
                    "LSP request error: {message}"
                );
                return message.get("result").cloned().unwrap_or(Value::Null);
            }
            if message.get("method").and_then(Value::as_str) == Some("$/progress") {
                self.progress.push(message);
            }
        }
    }

    fn take_progress_until_end(&mut self, token: &str) -> Vec<Value> {
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !self.progress.iter().any(|message| {
            message["params"]["token"] == token && message["params"]["value"]["kind"] == "end"
        }) {
            let Some(remaining) = deadline.checked_duration_since(std::time::Instant::now()) else {
                break;
            };
            match self.output.recv_timeout(remaining) {
                Ok(message)
                    if message.get("method").and_then(Value::as_str) == Some("$/progress") =>
                {
                    self.progress.push(message);
                }
                Ok(message) => self.deferred.push(message),
                Err(_) => break,
            }
        }
        std::mem::take(&mut self.progress)
    }

    fn open(&mut self, path: &Path, version: i32, text: &str) {
        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": file_uri(path),
                    "languageId": "tyrus",
                    "version": version,
                    "text": text,
                }
            }),
        );
    }

    fn wait_for_diagnostic(&mut self, uri: &str, predicate: impl Fn(&Value) -> bool) -> Value {
        loop {
            let message = self.receive();
            if message.get("method").and_then(Value::as_str)
                != Some("textDocument/publishDiagnostics")
            {
                continue;
            }
            let params = &message["params"];
            if params["uri"] == uri
                && params["diagnostics"]
                    .as_array()
                    .is_some_and(|diagnostics| diagnostics.iter().any(&predicate))
            {
                return params["diagnostics"].clone();
            }
        }
    }

    fn wait_for_diagnostics_for_uris(
        &mut self,
        uris: &[String],
        predicate: impl Fn(&Value) -> bool,
    ) {
        let mut received = std::collections::HashSet::new();
        while received.len() < uris.len() {
            let message = self.receive();
            if message.get("method").and_then(Value::as_str)
                != Some("textDocument/publishDiagnostics")
            {
                continue;
            }
            let params = &message["params"];
            let Some(uri) = params["uri"].as_str() else {
                continue;
            };
            if uris.iter().any(|expected| expected == uri)
                && params["diagnostics"]
                    .as_array()
                    .is_some_and(|diagnostics| diagnostics.iter().any(&predicate))
            {
                received.insert(uri.to_string());
            }
        }
    }

    fn wait_for_publications(&mut self, uri: &str, count: usize) {
        let mut received = 0;
        while received < count {
            let message = self.receive();
            if message.get("method").and_then(Value::as_str)
                == Some("textDocument/publishDiagnostics")
                && message["params"]["uri"] == uri
            {
                received += 1;
            }
        }
    }

    fn wait_for_publication(&mut self, uri: &str) -> Value {
        loop {
            let message = self.receive();
            if message.get("method").and_then(Value::as_str)
                == Some("textDocument/publishDiagnostics")
                && message["params"]["uri"] == uri
            {
                return message["params"]["diagnostics"].clone();
            }
        }
    }

    fn wait_for_empty_diagnostics(&mut self, uri: &str) -> Value {
        loop {
            let message = self.receive();
            if message.get("method").and_then(Value::as_str)
                == Some("textDocument/publishDiagnostics")
                && message["params"]["uri"] == uri
                && message["params"]["diagnostics"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
            {
                return message["params"]["diagnostics"].clone();
            }
        }
    }

    fn wait_for_close_clear(&mut self, uri: &str) {
        loop {
            let message = self.receive();
            if message.get("method").and_then(Value::as_str)
                == Some("textDocument/publishDiagnostics")
                && message["params"]["uri"] == uri
                && message["params"].get("version").is_none()
            {
                return;
            }
        }
    }

    fn assert_no_publication_for(&self, uri: &str, duration: Duration) {
        let deadline = std::time::Instant::now() + duration;
        while let Some(remaining) = deadline.checked_duration_since(std::time::Instant::now()) {
            match self.output.recv_timeout(remaining) {
                Ok(message)
                    if message.get("method").and_then(Value::as_str)
                        == Some("textDocument/publishDiagnostics")
                        && message["params"]["uri"] == uri =>
                {
                    panic!("received unexpected diagnostics publication: {message}");
                }
                Ok(_) => {}
                Err(mpsc::RecvTimeoutError::Timeout) => return,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    fn assert_no_empty_diagnostics_for(&self, uri: &str, duration: Duration) {
        let deadline = std::time::Instant::now() + duration;
        while let Some(remaining) = deadline.checked_duration_since(std::time::Instant::now()) {
            match self.output.recv_timeout(remaining) {
                Ok(message)
                    if message.get("method").and_then(Value::as_str)
                        == Some("textDocument/publishDiagnostics")
                        && message["params"]["uri"] == uri
                        && message["params"]["diagnostics"]
                            .as_array()
                            .is_some_and(Vec::is_empty) =>
                {
                    panic!("received unexpected empty diagnostics publication: {message}");
                }
                Ok(_) => {}
                Err(mpsc::RecvTimeoutError::Timeout) => return,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    fn assert_no_diagnostic_code_for(&self, uri: &str, code: &str, duration: Duration) {
        let deadline = std::time::Instant::now() + duration;
        while let Some(remaining) = deadline.checked_duration_since(std::time::Instant::now()) {
            match self.output.recv_timeout(remaining) {
                Ok(message)
                    if message.get("method").and_then(Value::as_str)
                        == Some("textDocument/publishDiagnostics")
                        && message["params"]["uri"] == uri
                        && message["params"]["diagnostics"].as_array().is_some_and(
                            |diagnostics| {
                                diagnostics
                                    .iter()
                                    .any(|diagnostic| diagnostic["code"] == code)
                            },
                        ) =>
                {
                    panic!("received unexpected {code} diagnostic: {message}");
                }
                Ok(_) => {}
                Err(mpsc::RecvTimeoutError::Timeout) => return,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    fn send(&mut self, message: Value) {
        let body = serde_json::to_vec(&message).expect("serialize LSP message");
        write!(self.input, "Content-Length: {}\r\n\r\n", body.len()).expect("write LSP header");
        self.input.write_all(&body).expect("write LSP body");
        self.input.flush().expect("flush LSP message");
    }

    fn receive(&mut self) -> Value {
        if !self.deferred.is_empty() {
            return self.deferred.remove(0);
        }
        self.output
            .recv_timeout(Duration::from_secs(10))
            .unwrap_or_else(|error| {
                let trace = self
                    .trace
                    .as_ref()
                    .map(|trace| trace.try_iter().collect::<Vec<_>>().join("\n"))
                    .unwrap_or_default();
                panic!("LSP process did not send a response before the timeout: {error}\n{trace}");
            })
    }
}

#[test]
fn registers_workspace_watchers_when_client_supports_dynamic_registration() {
    let workspace = temp_workspace();
    let mut lsp = LspProcess::start_with_options(workspace.path(), false, true);
    let request = lsp.receive();
    assert_eq!(request["method"], "client/registerCapability");
    let registration = &request["params"]["registrations"][0];
    assert_eq!(registration["method"], "workspace/didChangeWatchedFiles");
    let watchers = registration["registerOptions"]["watchers"]
        .as_array()
        .unwrap();
    for expected in [
        "**/*.tyr",
        "**/tyrus.json",
        "**/tyrus-lint.json",
        "**/tyrus-fmt.json",
        "**/.env",
        "**/.env.*",
    ] {
        assert!(
            watchers
                .iter()
                .any(|watcher| watcher["globPattern"] == expected)
        );
    }
    lsp.send(json!({"jsonrpc": "2.0", "id": request["id"], "result": null}));
}

#[test]
fn workspace_folder_changes_retarget_cross_file_references() {
    let first_workspace = temp_workspace();
    let second_workspace = temp_workspace();
    let first_target_path = first_workspace.path().join("src/lib.tyr");
    let first_importer_path = first_workspace.path().join("src/main.tyr");
    let target_path = second_workspace.path().join("src/lib.tyr");
    let importer_path = second_workspace.path().join("src/main.tyr");
    fs::create_dir_all(first_target_path.parent().unwrap()).unwrap();
    fs::create_dir_all(target_path.parent().unwrap()).unwrap();
    fs::write(&first_target_path, "export const shared = 1;\n").unwrap();
    fs::write(
        &first_importer_path,
        "import { shared } from \"./lib.tyr\";\nprint(shared);\n",
    )
    .unwrap();
    fs::write(&target_path, "export const shared = 1;\n").unwrap();
    fs::write(
        &importer_path,
        "import { shared } from \"./lib.tyr\";\nprint(shared);\n",
    )
    .unwrap();

    let mut lsp = LspProcess::start_with_trace(first_workspace.path());
    lsp.open(&first_target_path, 1, "export const shared = 1;\n");
    lsp.open(
        &first_importer_path,
        1,
        "import { shared } from \"./lib.tyr\";\nprint(shared);\n",
    );
    let first_references = lsp.request(
        "textDocument/references",
        json!({
            "textDocument": {"uri": file_uri(&first_target_path)},
            "position": {"line": 0, "character": 13},
            "context": {"includeDeclaration": true},
        }),
    );
    assert!(first_references.as_array().is_some_and(|locations| {
        locations
            .iter()
            .any(|location| location["uri"] == file_uri(&first_importer_path))
    }));
    lsp.drain_trace();
    lsp.notify(
        "workspace/didChangeWorkspaceFolders",
        json!({
            "event": {
                "added": [{"uri": file_uri(second_workspace.path()), "name": "second"}],
                "removed": [{"uri": file_uri(first_workspace.path()), "name": "first"}],
            }
        }),
    );
    lsp.open(&target_path, 1, "export const shared = 1;\n");
    lsp.open(
        &importer_path,
        1,
        "import { shared } from \"./lib.tyr\";\nprint(shared);\n",
    );

    let references = lsp.request(
        "textDocument/references",
        json!({
            "textDocument": {"uri": file_uri(&target_path)},
            "position": {"line": 0, "character": 13},
            "context": {"includeDeclaration": true},
        }),
    );
    let locations = references.as_array().expect("references response");
    assert!(
        locations
            .iter()
            .any(|location| location["uri"] == file_uri(&importer_path)),
        "workspace references must include the importer in the newly added root: {locations:?}"
    );
    let trace = lsp.drain_trace().join("\n");
    assert!(
        trace.lines().any(
            |line| line.contains("count=workspace_detached_cache_entries")
                && !line.ends_with("value=0")
        ),
        "detaching a warmed workspace must evict its cached indexes: {trace}"
    );
}

#[test]
fn custom_database_schema_watcher_reanalyzes_open_documents_on_change() {
    let workspace = temp_workspace();
    let schema_directory = workspace.path().join("schemas");
    fs::create_dir(&schema_directory).expect("schema directory");
    let schema_path = schema_directory.join("current.json");
    let schema = tyrus_core::db::DbSchemaSnapshot::new("lsp test".to_string(), 1, Vec::new())
        .to_pretty_json()
        .expect("serialize database schema");
    fs::write(&schema_path, schema).expect("write schema snapshot");
    let project_config = tyrus_core::project::project_config_json_with_database(
        "schema-watcher",
        None,
        Some(json!({
            "backend": "sqlite",
            "path": "data.db",
            "schema": "schemas/current.json"
        })),
    );
    fs::write(workspace.path().join("tyrus.json"), project_config)
        .expect("write project configuration");
    let source_path = workspace.path().join("src/main.tyr");
    let uri = file_uri(&source_path);
    let source = "const main = (): void => {};\n";
    fs::write(&source_path, source).expect("write source");

    let mut lsp = LspProcess::start_with_options(workspace.path(), true, true);
    let workspace_registration = lsp.receive();
    assert_eq!(
        workspace_registration["method"],
        "client/registerCapability"
    );
    lsp.send(json!({
        "jsonrpc": "2.0",
        "id": workspace_registration["id"],
        "result": null,
    }));
    lsp.open(&source_path, 1, source);

    let mut saw_initial_publication = false;
    let schema_registration = loop {
        let message = lsp.receive();
        if message["method"] == "textDocument/publishDiagnostics" && message["params"]["uri"] == uri
        {
            saw_initial_publication = true;
        }
        if message["method"] == "client/registerCapability"
            && message["params"]["registrations"][0]["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("tyrus.database-schema."))
        {
            break message;
        }
    };
    let watcher =
        &schema_registration["params"]["registrations"][0]["registerOptions"]["watchers"][0];
    assert_eq!(watcher["globPattern"]["pattern"], "schemas/current.json");
    assert_eq!(
        watcher["globPattern"]["baseUri"],
        format!("{}/", file_uri(workspace.path()))
    );
    lsp.send(json!({
        "jsonrpc": "2.0",
        "id": schema_registration["id"],
        "result": null,
    }));
    if !saw_initial_publication {
        lsp.wait_for_publication(&uri);
    }
    lsp.drain_trace();

    let updated_schema =
        tyrus_core::db::DbSchemaSnapshot::new("lsp test".to_string(), 2, Vec::new())
            .to_pretty_json()
            .expect("serialize updated database schema");
    fs::write(&schema_path, updated_schema).expect("update schema snapshot");
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({"changes": [{"uri": file_uri(&schema_path), "type": 2}]}),
    );
    lsp.wait_for_publications(&uri, 2);
    let trace = lsp.drain_trace().join("\n");
    assert!(trace.contains("stage=database_schema_cache"), "{trace}");
    assert!(trace.contains("event=analysis_computation"), "{trace}");
    assert!(
        trace.contains("stage=request_dispatch_workspace/didChangeWatchedFiles"),
        "{trace}"
    );

    lsp.drain_trace();
    fs::remove_file(&schema_path).expect("remove schema snapshot");
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({"changes": [{"uri": file_uri(&schema_path), "type": 3}]}),
    );
    lsp.wait_for_publications(&uri, 2);
    let trace = lsp.drain_trace().join("\n");
    assert!(trace.contains("stage=database_schema_cache"), "{trace}");
    assert!(trace.contains("event=analysis_computation"), "{trace}");
}

fn read_lsp_message(output: &mut BufReader<ChildStdout>) -> Option<Value> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        if output.read_line(&mut line).ok()? == 0 {
            return None;
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            content_length = Some(value.trim().parse::<usize>().expect("content length"));
        }
    }
    let mut body = vec![0; content_length?];
    output.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

impl Drop for LspProcess {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn file_uri(path: &Path) -> String {
    format!("file://{}", path.display())
}

fn temp_workspace() -> TempDir {
    let directory = tempfile::tempdir().expect("temporary workspace");
    fs::create_dir(directory.path().join("src")).expect("workspace source directory");
    directory
}

#[test]
fn workspace_completion_reports_progress_for_large_indexes() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let source = "const main = (): void => { sym };\n";
    fs::write(&source_path, source).expect("write completion source");
    for index in 0..256 {
        fs::write(
            workspace.path().join(format!("src/module_{index}.tyr")),
            format!("export const symbol_{index} = {index};\n"),
        )
        .expect("write workspace module");
    }

    let mut lsp = LspProcess::start_with_trace(workspace.path());
    lsp.open(&source_path, 1, source);
    let _ = lsp.request(
        "textDocument/completion",
        json!({
            "textDocument": {"uri": file_uri(&source_path)},
            "position": {"line": 0, "character": 30},
            "workDoneToken": "workspace-completion-index",
        }),
    );

    let progress = lsp.take_progress_until_end("workspace-completion-index");
    assert!(progress.iter().any(|message| {
        message["params"]["token"] == "workspace-completion-index"
            && message["params"]["value"]["kind"] == "begin"
    }));
    assert!(progress.iter().any(|message| {
        message["params"]["token"] == "workspace-completion-index"
            && message["params"]["value"]["kind"] == "report"
            && message["params"]["value"]["percentage"]
                .as_u64()
                .unwrap_or(0)
                > 0
    }));
    assert!(progress.iter().any(|message| {
        message["params"]["token"] == "workspace-completion-index"
            && message["params"]["value"]["kind"] == "end"
    }));
}

#[test]
fn go_to_definition_uses_open_unsaved_import_targets_and_resolves_aliases() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let target_path = workspace.path().join("src/math.tyr");
    let source = "import { answer as compute } from \"./math\";\nconst local = (value: i32): i32 => value;\nconst main = (): void => {\n    print(local(compute(1)));\n};\n";
    let disk_target = "export const answer = (input: i32): i32 => input;\n";
    let open_target = "const marker = 1;\nexport const answer = (input: i32): i32 => input;\n";
    fs::write(&source_path, source).expect("write client source");
    fs::write(&target_path, disk_target).expect("write on-disk module");

    let mut lsp = LspProcess::start_with_trace(workspace.path());
    lsp.open(&target_path, 1, open_target);
    lsp.open(&source_path, 1, source);

    let imported_position = json!({"line": 3, "character": 17});
    let imported = lsp.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": file_uri(&source_path)},
            "position": imported_position,
        }),
    );
    assert_eq!(imported["uri"], file_uri(&target_path));
    assert_eq!(imported["range"]["start"]["line"], 1);

    let local_position = json!({"line": 3, "character": 11});
    let local = lsp.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": file_uri(&source_path)},
            "position": local_position,
        }),
    );
    assert_eq!(local["uri"], file_uri(&source_path));
    assert_eq!(local["range"]["start"]["line"], 1);
}

#[test]
fn repeated_go_to_definition_reuses_the_versioned_import_target() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let target_path = workspace.path().join("src/value.tyr");
    let source = "import { answer } from \"./value\";\nprint(answer);\n";
    let target = "export const answer: i32 = 42;\n";
    fs::write(&source_path, source).expect("write importing source");
    fs::write(&target_path, target).expect("write imported module");

    let mut lsp = LspProcess::start_with_trace(workspace.path());
    lsp.open(&target_path, 1, target);
    lsp.open(&source_path, 1, source);
    let _ = lsp.request(
        "textDocument/completion",
        json!({
            "textDocument": {"uri": file_uri(&source_path)},
            "position": {"line": 1, "character": 7},
            "context": {"triggerKind": 1},
        }),
    );
    lsp.drain_trace();
    let params = json!({
        "textDocument": {"uri": file_uri(&source_path)},
        "position": {"line": 1, "character": 7},
    });

    let first = lsp.request("textDocument/definition", params.clone());
    assert_eq!(first["uri"], file_uri(&target_path));
    let trace = lsp
        .wait_for_trace_event("event=workspace_definition_cache_hit")
        .join("\n");
    assert!(
        trace.contains("event=workspace_definition_cache_hit"),
        "a warm export index should serve a direct definition: {trace}"
    );

    let second = lsp.request("textDocument/definition", params);
    assert_eq!(second, first);
    let trace = lsp
        .wait_for_trace_event("event=workspace_definition_cache_hit")
        .join("\n");
    assert!(
        trace.contains("event=workspace_definition_cache_hit"),
        "{trace}"
    );

    let changed_target = "const marker: i32 = 0;\nexport const answer: i32 = 42;\n";
    lsp.notify(
        "textDocument/didChange",
        json!({
            "textDocument": {"uri": file_uri(&target_path), "version": 2},
            "contentChanges": [{"text": changed_target}],
        }),
    );
    loop {
        let message = lsp.receive();
        if message["method"] == "textDocument/publishDiagnostics"
            && message["params"]["uri"] == file_uri(&target_path)
            && message["params"]["version"] == 2
        {
            break;
        }
    }
    let changed = lsp.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": file_uri(&source_path)},
            "position": {"line": 1, "character": 7},
        }),
    );
    assert_eq!(changed["uri"], file_uri(&target_path));
    assert_eq!(changed["range"]["start"]["line"], 1);
    let trace = lsp
        .wait_for_trace_event("stage=provider_definition")
        .join("\n");
    assert!(
        !trace.contains("event=workspace_definition_cache_hit"),
        "{trace}"
    );
}

#[test]
fn go_to_definition_follows_named_and_wildcard_re_exports() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let barrel_path = workspace.path().join("src/barrel.tyr");
    let all_path = workspace.path().join("src/all.tyr");
    let math_path = workspace.path().join("src/math.tyr");
    let other_path = workspace.path().join("src/other.tyr");
    let source = "import { result as compute } from \"./barrel\";\nimport { other } from \"./all\";\nconst main = (): void => {\n    print(compute(1));\n    print(other);\n};\n";
    fs::write(&source_path, source).expect("write source");
    fs::write(
        &barrel_path,
        "export { answer as result } from \"./math\";\n",
    )
    .expect("write named re-export");
    fs::write(&all_path, "export * from \"./other\";\n").expect("write wildcard re-export");
    fs::write(
        &math_path,
        "export const answer = (value: i32): i32 => value;\n",
    )
    .expect("write named export");
    fs::write(&other_path, "export const other = 2;\n").expect("write wildcard export");

    let mut lsp = LspProcess::start(workspace.path());
    lsp.open(&source_path, 1, source);

    let named = lsp.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": file_uri(&source_path)},
            "position": {"line": 3, "character": 11},
        }),
    );
    assert_eq!(named["uri"], file_uri(&math_path));
    assert_eq!(named["range"]["start"]["line"], 0);

    let wildcard = lsp.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": file_uri(&source_path)},
            "position": {"line": 4, "character": 11},
        }),
    );
    assert_eq!(wildcard["uri"], file_uri(&other_path));
    assert_eq!(wildcard["range"]["start"]["line"], 0);
}

#[test]
fn go_to_definition_follows_chained_re_exports_and_local_aliases() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let barrel_path = workspace.path().join("src/barrel.tyr");
    let middle_path = workspace.path().join("src/middle.tyr");
    let target_path = workspace.path().join("src/nested/target.tyr");
    let source = "import { external as local } from \"./barrel\";\nconst main = (): void => {\n    print(local);\n};\n";
    fs::write(&source_path, source).expect("write source");
    fs::write(
        &barrel_path,
        "export { alias as external } from \"./middle\";\n",
    )
    .expect("write first re-export");
    fs::write(
        &middle_path,
        "export { concrete as alias } from \"./nested/target\";\n",
    )
    .expect("write chained re-export");
    fs::create_dir_all(target_path.parent().expect("target parent"))
        .expect("create target directory");
    fs::write(&target_path, "export const concrete: i32 = 1;\n").expect("write target");

    let mut lsp = LspProcess::start(workspace.path());
    lsp.open(&source_path, 1, source);
    let definition = lsp.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": file_uri(&source_path)},
            "position": {"line": 2, "character": 11},
        }),
    );
    assert_eq!(definition["uri"], file_uri(&target_path));
    assert_eq!(definition["range"]["start"]["line"], 0);
}

#[test]
fn go_to_definition_stops_at_re_export_cycles_and_missing_exports() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let first_path = workspace.path().join("src/first.tyr");
    let second_path = workspace.path().join("src/second.tyr");
    let source = "import { missing } from \"./first\";\nconst main = (): void => {\n    print(missing);\n};\n";
    fs::write(&source_path, source).expect("write source");
    fs::write(&first_path, "export * from \"./second\";\n").expect("write first module");
    fs::write(&second_path, "export * from \"./first\";\n").expect("write second module");

    let mut lsp = LspProcess::start(workspace.path());
    lsp.open(&source_path, 1, source);
    let definition = lsp.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": file_uri(&source_path)},
            "position": {"line": 2, "character": 11},
        }),
    );
    assert!(definition.is_null());
}

#[test]
fn go_to_definition_returns_no_fake_location_for_native_modules() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let source =
        "import { cpus } from \"tyrus:os\";\nconst main = (): void => { print(cpus()); };\n";
    fs::write(&source_path, source).expect("write source");

    let mut lsp = LspProcess::start(workspace.path());
    lsp.open(&source_path, 1, source);
    let definition = lsp.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": file_uri(&source_path)},
            "position": {"line": 1, "character": 34},
        }),
    );
    assert!(definition.is_null());
}

#[test]
fn concurrent_hover_requests_share_analysis_across_open_documents() {
    let workspace = temp_workspace();
    let source_directory = workspace.path().join("src");
    fs::create_dir_all(&source_directory).expect("create source directory");
    let mut documents = Vec::new();
    for index in 0..16 {
        let path = source_directory.join(format!("document_{index}.tyr"));
        let (ty, value) = match index % 3 {
            0 => ("i32", "1"),
            1 => ("string", "\"ready\""),
            _ => ("bool", "true"),
        };
        let source = format!("const value: {ty} = {value};\n");
        fs::write(&path, &source).expect("write source document");
        documents.push((path, source, ty));
    }

    let mut lsp = LspProcess::start(workspace.path());
    for (path, source, _) in &documents {
        lsp.open(path, 1, source);
    }
    let request_ids = (0..documents.len() * 2)
        .map(|index| 1000 + index as u64)
        .collect::<Vec<_>>();
    for (document_index, (path, _, _)) in documents.iter().enumerate() {
        for request_id in &request_ids[document_index * 2..document_index * 2 + 2] {
            lsp.send(json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": {"uri": file_uri(path)},
                    "position": {"line": 0, "character": 6},
                },
            }));
        }
    }

    let mut received = std::collections::HashSet::new();
    while received.len() < request_ids.len() {
        let message = lsp.receive();
        let Some(request_id) = message.get("id").and_then(Value::as_u64) else {
            continue;
        };
        if !request_ids.contains(&request_id) {
            continue;
        }
        let document_index = (request_id as usize - 1000) / 2;
        assert!(
            message["result"]
                .to_string()
                .contains(documents[document_index].2),
            "unexpected hover result for request {request_id}: {message}"
        );
        received.insert(request_id);
    }
}

#[test]
fn repeated_hover_reuses_cached_analysis_without_another_computation() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let source = "const value: i32 = 1;\n";
    fs::write(&source_path, source).expect("write source");

    let mut lsp = LspProcess::start_with_trace(workspace.path());
    let uri = file_uri(&source_path);
    lsp.open(&source_path, 1, source);
    let first_hover = lsp.request(
        "textDocument/hover",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": 7},
        }),
    );
    assert!(
        first_hover.to_string().contains("value: i32"),
        "{first_hover}"
    );
    lsp.drain_trace();

    let second_hover = lsp.request(
        "textDocument/hover",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": 7},
        }),
    );
    assert!(
        second_hover.to_string().contains("value: i32"),
        "{second_hover}"
    );
    let trace = lsp.drain_trace().join("\n");
    assert!(trace.contains("stage=analysis_cache_hit"), "{trace}");
    assert!(!trace.contains("event=analysis_computation"), "{trace}");
}

#[test]
fn repeated_hover_reuses_the_cached_analysis_of_an_imported_definition() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let module_path = workspace.path().join("src/module.tyr");
    fs::write(&module_path, "export const answer: i32 = 42;\n").expect("write imported module");
    let source = "import { answer as alias } from \"./module\";\nconst main = (): void => { print(alias); };\n";
    fs::write(&source_path, source).expect("write source");

    let mut lsp = LspProcess::start_with_trace(workspace.path());
    let uri = file_uri(&source_path);
    lsp.open(&source_path, 1, source);
    let first_hover = lsp.request(
        "textDocument/hover",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": 34},
        }),
    );
    assert!(
        first_hover.to_string().contains("alias: i32"),
        "{first_hover}"
    );
    lsp.drain_trace();

    let second_hover = lsp.request(
        "textDocument/hover",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": 34},
        }),
    );
    assert!(
        second_hover.to_string().contains("alias: i32"),
        "{second_hover}"
    );
    let trace = lsp.drain_trace().join("\n");
    assert!(
        trace.matches("event=workspace_analysis_cache_hit").count() >= 2,
        "{trace}"
    );
}

#[test]
fn lint_config_changes_reuse_semantic_analysis_and_refresh_diagnostics() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let source = "const main = (): void => {\n    let unused: i32 = 1;\n};\n";
    fs::write(&source_path, source).expect("write source");
    let mut lsp = LspProcess::start(workspace.path());
    let uri = file_uri(&source_path);
    lsp.open(&source_path, 1, source);
    lsp.wait_for_diagnostic(&uri, |diagnostic| diagnostic["code"] == "no-unused-vars");

    let config_path = workspace.path().join("tyrus-lint.json");
    fs::write(
        &config_path,
        "{\"no-unused-vars\": \"off\", \"prefer-const\": \"off\"}\n",
    )
    .expect("disable unused variable lint");
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({
            "changes": [{"uri": file_uri(&config_path), "type": 2}],
        }),
    );
    lsp.wait_for_publications(&uri, 1);
    let diagnostics = lsp.wait_for_empty_diagnostics(&uri);
    assert!(diagnostics.as_array().unwrap().is_empty());
}

#[test]
fn lsp_stays_alive_while_reanalyzing_documents_with_type_errors() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let mut lsp = LspProcess::start(workspace.path());
    let uri = file_uri(&source_path);

    for version in 1..=5 {
        let source = "interface Score { name: string; points: i32; }\n\
             const main = (): void => {\n\
                 let scores1: Score[] = [{ name: \"Ada\", points: 30 }];\n\
                 scores1.sortBy(\"points\");\n\
                 print(scores);\n\
             };\n"
            .to_string();
        if version == 1 {
            lsp.open(&source_path, version, &source);
        } else {
            lsp.notify(
                "textDocument/didChange",
                json!({
                    "textDocument": {"uri": uri, "version": version},
                    "contentChanges": [{"text": source}],
                }),
            );
        }

        loop {
            let message = lsp.receive();
            if message["method"] == "textDocument/publishDiagnostics"
                && message["params"]["uri"] == uri
                && message["params"]["version"] == version
                && message["params"]["diagnostics"]
                    .as_array()
                    .is_some_and(|diagnostics| {
                        diagnostics
                            .iter()
                            .any(|diagnostic| diagnostic["code"] == "E0425")
                    })
            {
                break;
            }
        }
    }

    let response = lsp.request(
        "textDocument/hover",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 3, "character": 20},
        }),
    );
    assert!(
        !response.is_null(),
        "LSP should keep serving requests: {response}"
    );
    let code_lenses = lsp.request(
        "textDocument/codeLens",
        json!({"textDocument": {"uri": uri}}),
    );
    assert!(code_lenses.is_array(), "code lens response: {code_lenses}");
    let inlay_hints = lsp.request(
        "textDocument/inlayHint",
        json!({
            "textDocument": {"uri": uri},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 6, "character": 0},
            },
        }),
    );
    assert!(inlay_hints.is_array(), "inlay hint response: {inlay_hints}");
    let semantic_tokens = lsp.request(
        "textDocument/semanticTokens/full",
        json!({"textDocument": {"uri": uri}}),
    );
    assert!(
        semantic_tokens.is_object(),
        "semantic token response: {semantic_tokens}"
    );
    std::thread::sleep(Duration::from_secs(5));
    assert!(
        lsp.child
            .try_wait()
            .expect("check LSP process status")
            .is_none(),
        "LSP must remain alive while diagnostics stay visible"
    );
}

#[test]
fn lsp_stays_alive_while_reanalyzing_incomplete_console_identifier() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let mut lsp = LspProcess::start(workspace.path());
    let uri = file_uri(&source_path);
    let source = "const main = (): void => {\n    con\n};\n";

    for version in 1..=5 {
        if version == 1 {
            lsp.open(&source_path, version, source);
        } else {
            lsp.notify(
                "textDocument/didChange",
                json!({
                    "textDocument": {"uri": uri, "version": version},
                    "contentChanges": [{"text": source}],
                }),
            );
        }
        loop {
            let message = lsp.receive();
            if message["method"] == "textDocument/publishDiagnostics"
                && message["params"]["uri"] == uri
                && message["params"]["version"] == version
            {
                break;
            }
        }
    }

    std::thread::sleep(Duration::from_secs(5));
    assert!(
        lsp.child
            .try_wait()
            .expect("check LSP process status")
            .is_none(),
        "LSP crashed while `con` remained incomplete"
    );
}

#[test]
fn completion_survives_long_unicode_block_comment_in_workspace() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    fs::create_dir_all(workspace.path().join("src/algorithm"))
        .expect("create sibling source directory");
    let rule = char::from_u32(0x2500).expect("box drawing character");
    fs::write(
        workspace.path().join("src/algorithm/examples.tyr"),
        format!(
            "/* {} */\nexport const answer = 42;\n",
            rule.to_string().repeat(1_521)
        ),
    )
    .expect("write commented sibling source");
    let source = "const main = (): void => {\n    const broken: i32 = \"wrong\";\n    con\n};\n";
    fs::write(&source_path, source).expect("write source");
    let mut lsp = LspProcess::start_with_trace(workspace.path());
    let uri = file_uri(&source_path);
    lsp.open(&source_path, 1, source);
    let diagnostics = lsp.wait_for_diagnostic(&uri, |diagnostic| diagnostic["code"] == "E0425");
    let diagnostic = diagnostics
        .as_array()
        .expect("diagnostics array")
        .iter()
        .find(|diagnostic| diagnostic["code"] == "E0425")
        .expect("unresolved identifier diagnostic");
    let actions = lsp.request(
        "textDocument/codeAction",
        json!({
            "textDocument": {"uri": uri},
            "range": diagnostic["range"],
            "context": {"diagnostics": [diagnostic]},
        }),
    );
    assert!(actions.is_array(), "code action response: {actions}");
    let response = lsp.request(
        "textDocument/completion",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 2, "character": 7},
            "context": {"triggerKind": 1},
        }),
    );
    assert!(response.is_array(), "completion response: {response}");
    let trace = lsp.drain_trace().join("\n");
    assert!(
        trace.contains("workspace_modules_indexed"),
        "workspace analysis should finish: {trace}"
    );
    assert!(
        lsp.child
            .try_wait()
            .expect("check LSP process status")
            .is_none(),
        "LSP should survive completion after `con`: {trace}"
    );
}

#[test]
fn lint_config_changes_invalidate_only_documents_in_their_project() {
    let workspace = temp_workspace();
    let mut documents = Vec::new();
    for project in ["first", "second"] {
        let project_root = workspace.path().join(project);
        let source_directory = project_root.join("src");
        fs::create_dir_all(&source_directory).expect("create project source directory");
        fs::write(
            project_root.join("tyrus.json"),
            tyrus_core::project::project_config_json(project, None),
        )
        .expect("write project configuration");
        let source_path = source_directory.join("main.tyr");
        let source = "const main = (): void => { let value: i32 = 1; print(value); };\n";
        fs::write(&source_path, source).expect("write source");
        documents.push((project, project_root, source_path, source));
    }

    let mut lsp = LspProcess::start(workspace.path());
    for (_, _, source_path, source) in &documents {
        lsp.open(source_path, 1, source);
    }
    let document_uris = documents
        .iter()
        .map(|(_, _, source_path, _)| file_uri(source_path))
        .collect::<Vec<_>>();
    lsp.wait_for_diagnostics_for_uris(&document_uris, |diagnostic| {
        diagnostic["code"] == "prefer-const"
    });

    let first_config = documents[0].1.join("tyrus-lint.json");
    fs::write(&first_config, "{\"prefer-const\": \"off\"}\n")
        .expect("disable prefer-const in the first project");
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({"changes": [{"uri": file_uri(&first_config), "type": 2}]}),
    );

    let first_uri = file_uri(&documents[0].2);
    let cleared_diagnostics = lsp.wait_for_publication(&first_uri);
    assert!(cleared_diagnostics.as_array().unwrap().is_empty());
    lsp.assert_no_diagnostic_code_for(&first_uri, "prefer-const", Duration::from_millis(250));
    let second_uri = file_uri(&documents[1].2);
    lsp.assert_no_publication_for(&second_uri, Duration::from_millis(250));
}

#[test]
fn project_config_changes_invalidate_only_documents_in_their_project() {
    let workspace = temp_workspace();
    let mut documents = Vec::new();
    for project in ["first", "second"] {
        let project_root = workspace.path().join(project);
        let source_directory = project_root.join("src");
        fs::create_dir_all(&source_directory).expect("create project source directory");
        fs::write(
            project_root.join("tyrus.json"),
            tyrus_core::project::project_config_json(project, None),
        )
        .expect("write project configuration");
        let source_path = source_directory.join("main.tyr");
        let source = "const main = (): void => { let value: i32 = 1; print(value); };\n";
        fs::write(&source_path, source).expect("write source");
        documents.push((project, project_root, source_path, source));
    }

    let mut lsp = LspProcess::start(workspace.path());
    for (_, _, source_path, source) in &documents {
        lsp.open(source_path, 1, source);
    }
    let document_uris = documents
        .iter()
        .map(|(_, _, source_path, _)| file_uri(source_path))
        .collect::<Vec<_>>();
    lsp.wait_for_diagnostics_for_uris(&document_uris, |diagnostic| {
        diagnostic["code"] == "prefer-const"
    });

    let first_config = documents[0].1.join("tyrus.json");
    fs::write(
        &first_config,
        tyrus_core::project::project_config_json("first-updated", None),
    )
    .expect("update the first project configuration");
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({"changes": [{"uri": file_uri(&first_config), "type": 2}]}),
    );

    let first_uri = file_uri(&documents[0].2);
    assert!(
        lsp.wait_for_publication(&first_uri)
            .as_array()
            .unwrap()
            .is_empty()
    );
    let diagnostics = lsp.wait_for_publication(&first_uri);
    assert!(
        diagnostics
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| diagnostic["code"] == "prefer-const"),
        "{diagnostics}"
    );
    let second_uri = file_uri(&documents[1].2);
    lsp.assert_no_publication_for(&second_uri, Duration::from_millis(250));
}

#[test]
fn project_import_alias_changes_invalidate_cached_module_configuration() {
    let workspace = temp_workspace();
    let source_directory = workspace.path().join("src");
    let first_module_directory = source_directory.join("app");
    let second_module_directory = source_directory.join("other_folder");
    fs::create_dir_all(&first_module_directory).expect("create initial module directory");
    fs::create_dir_all(&second_module_directory).expect("create replacement module directory");
    let source_path = source_directory.join("main.tyr");
    let target_name = "value.tyr";
    fs::write(
        first_module_directory.join(target_name),
        "export const answer: i32 = 1;\n",
    )
    .expect("write initial target module");
    fs::write(
        second_module_directory.join(target_name),
        "export const answer: string = \"updated\";\n",
    )
    .expect("write replacement target module");
    let config_path = workspace.path().join("tyrus.json");
    let initial_config =
        r##"{"project":{"name":"sample","version":"0.1.0"},"imports":{"#app":"./src/app"}}"##;
    fs::write(&config_path, initial_config).expect("write initial project config");
    let updated_config = r##"{"project":{"name":"sample","version":"0.1.0"},"imports":{"#app":"./src/other_folder"}}"##;
    let source = "import { answer } from \"#app/value\";\nconst main = (): void => {\n    print(answer);\n};\n";
    fs::write(&source_path, source).expect("write importing source");

    let mut lsp = LspProcess::start(workspace.path());
    let uri = file_uri(&source_path);
    lsp.open(&source_path, 1, source);
    lsp.wait_for_publications(&uri, 2);
    let definition = lsp.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 2, "character": 11},
        }),
    );
    assert_eq!(
        definition["uri"],
        file_uri(&first_module_directory.join(target_name))
    );
    let references = lsp.request(
        "textDocument/references",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 2, "character": 11},
            "context": {"includeDeclaration": true},
        }),
    );
    assert!(
        references.as_array().is_some_and(|locations| {
            locations.iter().any(|location| location["uri"] == uri)
                && locations.iter().any(|location| {
                    location["uri"] == file_uri(&first_module_directory.join(target_name))
                })
        }),
        "path alias references should include the importer and target: {references}"
    );
    let rename = lsp.request(
        "textDocument/rename",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 2, "character": 11},
            "newName": "renamedAnswer",
        }),
    );
    assert!(rename["changes"].get(&uri).is_some());
    assert!(
        rename["changes"]
            .get(file_uri(&first_module_directory.join(target_name)))
            .is_some()
    );
    let first_hover = lsp.request(
        "textDocument/hover",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 2, "character": 11},
        }),
    );
    assert!(first_hover.to_string().contains("i32"), "{first_hover}");

    fs::write(&config_path, updated_config).expect("update project import alias");
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({
            "changes": [{"uri": file_uri(&config_path), "type": 2}],
        }),
    );
    lsp.wait_for_publications(&uri, 1);
    let updated_definition = lsp.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 2, "character": 11},
        }),
    );
    assert_eq!(
        updated_definition["uri"],
        file_uri(&second_module_directory.join(target_name))
    );
    let updated_hover = lsp.request(
        "textDocument/hover",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 2, "character": 11},
        }),
    );
    assert!(
        updated_hover.to_string().contains("string"),
        "{updated_hover}"
    );

    fs::remove_file(&config_path).expect("remove project config");
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({
            "changes": [{"uri": file_uri(&config_path), "type": 3}],
        }),
    );
    let removed_config_hover = lsp.request(
        "textDocument/hover",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 2, "character": 11},
        }),
    );
    assert!(
        removed_config_hover.to_string().contains("answer: unknown"),
        "{removed_config_hover}"
    );
}

#[test]
fn nested_project_alias_changes_refresh_cross_file_occurrence_results() {
    let workspace = temp_workspace();
    let project_root = workspace.path().join("app");
    let source_directory = project_root.join("src");
    let first_directory = project_root.join("first");
    let second_directory = project_root.join("second");
    fs::create_dir_all(&source_directory).expect("create nested project source directory");
    fs::create_dir_all(&first_directory).expect("create first alias target directory");
    fs::create_dir_all(&second_directory).expect("create second alias target directory");
    let first_target = first_directory.join("value.tyr");
    let second_target = second_directory.join("value.tyr");
    let source_path = source_directory.join("main.tyr");
    let source = "import { answer } from \"#shared/value\";\nprint(answer);\n";
    let config_path = project_root.join("tyrus.json");
    fs::write(
        &config_path,
        r##"{"project":{"name":"app","version":"0.1.0"},"imports":{"#shared":"./first"}}"##,
    )
    .expect("write initial nested project config");
    fs::write(&first_target, "export const answer: i32 = 1;\n").expect("write first target");
    fs::write(
        &second_target,
        "export const answer: string = \"second\";\n",
    )
    .expect("write second target");
    fs::write(&source_path, source).expect("write importing source");

    let mut lsp = LspProcess::start(workspace.path());
    let uri = file_uri(&source_path);
    lsp.open(&source_path, 1, source);
    lsp.wait_for_publications(&uri, 2);
    let first_references = lsp.request(
        "textDocument/references",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": 8},
            "context": {"includeDeclaration": true},
        }),
    );
    assert!(first_references.as_array().is_some_and(|locations| {
        locations
            .iter()
            .any(|location| location["uri"] == file_uri(&first_target))
    }));

    fs::write(
        &config_path,
        r##"{"project":{"name":"app","version":"0.1.0"},"imports":{"#shared":"./second"}}"##,
    )
    .expect("update nested project import alias");
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({"changes": [{"uri": file_uri(&config_path), "type": 2}]}),
    );
    lsp.wait_for_publications(&uri, 2);
    let second_references = lsp.request(
        "textDocument/references",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": 8},
            "context": {"includeDeclaration": true},
        }),
    );
    assert!(
        second_references.as_array().is_some_and(|locations| {
            locations.iter().any(|location| location["uri"] == uri)
                && locations
                    .iter()
                    .any(|location| location["uri"] == file_uri(&second_target))
        }),
        "nested alias invalidation should refresh cross-file references: {second_references}"
    );
}

#[test]
fn out_of_order_changes_do_not_replace_the_latest_document_snapshot() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let initial = "const initialName: i32 = 1;\n";
    fs::write(&source_path, initial).expect("write source");

    let mut lsp = LspProcess::start(workspace.path());
    lsp.open(&source_path, 1, initial);
    lsp.notify(
        "textDocument/didChange",
        json!({
            "textDocument": {"uri": file_uri(&source_path), "version": 3},
            "contentChanges": [{"text": "const currentName: i32 = 2;\n"}],
        }),
    );
    lsp.notify(
        "textDocument/didChange",
        json!({
            "textDocument": {"uri": file_uri(&source_path), "version": 2},
            "contentChanges": [{"text": "const staleName: i32 = 3;\n"}],
        }),
    );

    let hover = lsp.request(
        "textDocument/hover",
        json!({
            "textDocument": {"uri": file_uri(&source_path)},
            "position": {"line": 0, "character": 9},
        }),
    );
    let rendered = hover.to_string();
    assert!(rendered.contains("currentName"), "{hover}");
    assert!(!rendered.contains("staleName"), "{hover}");
}

#[test]
fn rapid_changes_publish_only_the_latest_diagnostic_snapshot() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let initial = "const initialValue: i32 = 0;\nprint(initialValue);\n";
    fs::write(&source_path, initial).expect("write source");

    let mut lsp = LspProcess::start(workspace.path());
    let uri = file_uri(&source_path);
    lsp.open(&source_path, 1, initial);

    for version in 2..=24 {
        let text = if version == 24 {
            "const latestValue: i32 = 42;\nprint(latestValue);\n"
        } else if version % 2 == 0 {
            "const staleValue: i32 = \"wrong type\";\n"
        } else {
            "const transientValue: i32 = 1;\n"
        };
        lsp.notify(
            "textDocument/didChange",
            json!({
                "textDocument": {"uri": uri, "version": version},
                "contentChanges": [{"text": text}],
            }),
        );
    }

    let mut latest_publications = Vec::new();
    while latest_publications.len() < 2 {
        let message = lsp.receive();
        if message["method"] == "textDocument/publishDiagnostics" && message["params"]["uri"] == uri
        {
            let version = message["params"]["version"]
                .as_i64()
                .expect("diagnostic publication includes a document version");
            let diagnostics = message["params"]["diagnostics"].clone();
            if version == 24 {
                assert_eq!(diagnostics, json!([]));
                latest_publications.push(diagnostics);
            } else if !latest_publications.is_empty() {
                panic!("received stale diagnostic version {version} after version 24");
            }
        }
    }

    let hover = lsp.request(
        "textDocument/hover",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": 6},
        }),
    );
    let rendered = hover.to_string();
    assert!(rendered.contains("latestValue"), "{hover}");
    assert!(!rendered.contains("staleValue"), "{hover}");
    assert!(!rendered.contains("transientValue"), "{hover}");
    lsp.assert_no_publication_for(&uri, Duration::from_millis(100));
}

#[test]
fn incremental_changes_apply_sequential_utf16_ranges() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let source = "const value: string = \"A😀B\";\n";
    fs::write(&source_path, source).expect("write source");

    let mut lsp = LspProcess::start(workspace.path());
    lsp.open(&source_path, 1, source);
    lsp.notify(
        "textDocument/didChange",
        json!({
            "textDocument": {"uri": file_uri(&source_path), "version": 2},
            "contentChanges": [
                {
                    "range": {
                        "start": {"line": 0, "character": 24},
                        "end": {"line": 0, "character": 26},
                    },
                    "rangeLength": 2,
                    "text": "C",
                },
                {
                    "range": {
                        "start": {"line": 0, "character": 6},
                        "end": {"line": 0, "character": 11},
                    },
                    "rangeLength": 5,
                    "text": "answer",
                },
            ],
        }),
    );

    let hover = lsp.request(
        "textDocument/hover",
        json!({
            "textDocument": {"uri": file_uri(&source_path)},
            "position": {"line": 0, "character": 8},
        }),
    );
    assert!(hover.to_string().contains("answer"), "{hover}");
    assert!(hover.to_string().contains("string"), "{hover}");
}

#[test]
fn closing_document_cancels_pending_diagnostic_publication() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let uri = file_uri(&source_path);
    fs::write(&source_path, "const value: i32 = 1;\n").expect("write source");

    let mut lsp = LspProcess::start(workspace.path());
    lsp.open(&source_path, 1, "const value: i32 = 1;\n");
    lsp.notify(
        "textDocument/didChange",
        json!({
            "textDocument": {"uri": uri, "version": 2},
            "contentChanges": [{"text": "const value: i32 = 2;\n"}],
        }),
    );
    lsp.notify(
        "textDocument/didClose",
        json!({"textDocument": {"uri": uri}}),
    );
    lsp.wait_for_close_clear(&uri);
    lsp.assert_no_publication_for(&uri, Duration::from_millis(250));
}

#[test]
fn unrelated_nested_env_changes_do_not_invalidate_workspace_documents() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let source = "const main = (): void => { let value: i32 = 1; print(value); };\n";
    fs::write(&source_path, source).expect("write source");
    let nested_directory = workspace.path().join("nested");
    fs::create_dir_all(&nested_directory).expect("create unrelated directory");
    let nested_env = nested_directory.join(".env");
    fs::write(&nested_env, "UNRELATED=value\n").expect("write unrelated environment file");

    let mut lsp = LspProcess::start(workspace.path());
    lsp.open(&source_path, 1, source);
    let uri = file_uri(&source_path);
    lsp.wait_for_diagnostic(&uri, |diagnostic| diagnostic["code"] == "prefer-const");
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({"changes": [{"uri": file_uri(&nested_env), "type": 2}]}),
    );

    lsp.assert_no_publication_for(&uri, Duration::from_millis(300));
}

#[test]
fn watched_env_changes_keep_the_reloaded_env_diagnostics_published() {
    let workspace = temp_workspace();
    let env_path = workspace.path().join(".env");
    let env_source = "@type(string)\n@minLength(5)\nTEST=test\n";
    fs::write(
        workspace.path().join("tyrus.json"),
        tyrus_core::project::project_config_json("dotenv-watcher", None),
    )
    .expect("write project config");
    fs::write(&env_path, env_source).expect("write env file");

    let mut lsp = LspProcess::start_with_trace(workspace.path());
    let uri = file_uri(&env_path);
    lsp.open(&env_path, 1, env_source);
    let diagnostics = lsp.wait_for_publication(&uri);
    assert!(
        diagnostics
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["code"] == "E3143")),
        "expected E3143 in initial env diagnostics: {diagnostics}"
    );

    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({"changes": [{"uri": uri, "type": 2}]}),
    );
    let diagnostics = lsp.wait_for_publication(&uri);
    assert!(
        diagnostics
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["code"] == "E3143")),
        "expected E3143 after env reload: {diagnostics}"
    );
    lsp.assert_no_empty_diagnostics_for(&uri, Duration::from_millis(250));
}

#[test]
fn watched_module_changes_reanalyze_open_importers() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let target_path = workspace.path().join("src/value.tyr");
    let source =
        "import { answer } from \"./value\";\nconst main = (): void => {\n    print(answer);\n};\n";
    fs::write(&source_path, source).expect("write importing source");
    fs::write(&target_path, "export const answer: i32 = 1;\n").expect("write target module");

    let mut lsp = LspProcess::start_with_trace(workspace.path());
    let source_uri = file_uri(&source_path);
    let target_uri = file_uri(&target_path);
    lsp.open(&source_path, 1, source);
    lsp.wait_for_publications(&source_uri, 2);

    fs::write(&target_path, "export const answer: string = \"updated\";\n")
        .expect("update target module");
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({
            "changes": [{"uri": target_uri, "type": 2}],
        }),
    );
    lsp.wait_for_publications(&source_uri, 1);

    let hover = lsp.request(
        "textDocument/hover",
        json!({
            "textDocument": {"uri": source_uri},
            "position": {"line": 2, "character": 11},
        }),
    );
    assert!(hover.to_string().contains("string"), "{hover}");

    fs::remove_file(&target_path).expect("remove target module");
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({"changes": [{"uri": target_uri, "type": 3}]}),
    );
    let diagnostics =
        lsp.wait_for_diagnostic(&source_uri, |diagnostic| diagnostic["code"] == "E0583");
    assert!(
        diagnostics
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| diagnostic["code"] == "E0583")
    );
}

#[test]
fn watched_module_changes_reanalyze_transitive_importers() {
    let workspace = temp_workspace();
    let source_directory = workspace.path().join("src");
    let source_path = source_directory.join("main.tyr");
    let barrel_path = source_directory.join("barrel.tyr");
    let target_path = source_directory.join("value.tyr");
    let source = "import { answer } from \"./barrel\";\nconst main = (): void => {\n    print(answer);\n};\n";
    fs::write(&source_path, source).expect("write importing source");
    fs::write(&barrel_path, "export { answer } from \"./value\";\n")
        .expect("write re-export module");
    fs::write(&target_path, "export const answer: i32 = 1;\n").expect("write target module");

    let mut lsp = LspProcess::start_with_trace(workspace.path());
    let source_uri = file_uri(&source_path);
    let target_uri = file_uri(&target_path);
    lsp.open(&source_path, 1, source);
    lsp.wait_for_publications(&source_uri, 2);

    fs::write(&target_path, "export const answer: string = \"updated\";\n")
        .expect("update target module");
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({"changes": [{"uri": target_uri, "type": 2}]}),
    );
    lsp.wait_for_publications(&source_uri, 2);

    let hover = lsp.request(
        "textDocument/hover",
        json!({
            "textDocument": {"uri": source_uri},
            "position": {"line": 2, "character": 11},
        }),
    );
    assert!(hover.to_string().contains("string"), "{hover}");
}

#[test]
fn cyclic_import_analysis_publishes_for_both_open_modules() {
    let workspace = temp_workspace();
    let first_path = workspace.path().join("src/first.tyr");
    let second_path = workspace.path().join("src/second.tyr");
    let first_source =
        "import { secondValue } from \"./second\";\nexport const firstValue: i32 = secondValue;\n";
    let second_source =
        "import { firstValue } from \"./first\";\nexport const secondValue: i32 = firstValue;\n";
    fs::write(&first_path, first_source).expect("write first cyclic module");
    fs::write(&second_path, second_source).expect("write second cyclic module");

    let mut lsp = LspProcess::start(workspace.path());
    lsp.open(&first_path, 1, first_source);
    lsp.open(&second_path, 1, second_source);
    let first_diagnostics = lsp.wait_for_publication(&file_uri(&first_path));
    let second_diagnostics = lsp.wait_for_publication(&file_uri(&second_path));
    assert!(first_diagnostics.is_array());
    assert!(second_diagnostics.is_array());
}

#[test]
fn watched_module_creation_resolves_missing_imports_and_definitions() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let target_path = workspace.path().join("src/value.tyr");
    let source =
        "import { answer } from \"./value\";\nconst main = (): void => {\n    print(answer);\n};\n";
    fs::write(&source_path, source).expect("write importing source");

    let mut lsp = LspProcess::start(workspace.path());
    let source_uri = file_uri(&source_path);
    let target_uri = file_uri(&target_path);
    lsp.open(&source_path, 1, source);
    lsp.wait_for_diagnostic(&source_uri, |diagnostic| diagnostic["code"] == "E0583");

    fs::write(&target_path, "export const answer: i32 = 42;\n").expect("create target module");
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({
            "changes": [{"uri": target_uri, "type": 1}],
        }),
    );
    let diagnostics = lsp.wait_for_publication(&source_uri);
    assert!(
        diagnostics
            .as_array()
            .unwrap()
            .iter()
            .all(|diagnostic| diagnostic["code"] != "E0583"),
        "{diagnostics}"
    );

    let definition = lsp.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": source_uri},
            "position": {"line": 2, "character": 11},
        }),
    );
    assert_eq!(definition["uri"], file_uri(&target_path));
}

#[test]
fn code_action_protocol_returns_the_published_lint_edit() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let uri = file_uri(&source_path);
    let source =
        "const main = (): void => {\n    let retries: i32 = 3;\n    console.log(retries);\n};\n";
    fs::write(&source_path, source).expect("write source");

    let mut lsp = LspProcess::start_with_trace(workspace.path());
    lsp.open(&source_path, 1, source);
    let diagnostics =
        lsp.wait_for_diagnostic(&uri, |diagnostic| diagnostic["code"] == "prefer-const");
    let _ = lsp.wait_for_trace_event("stage=diagnostics_publication");
    let diagnostic = diagnostics
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == "prefer-const")
        .unwrap()
        .clone();
    let actions = lsp.request(
        "textDocument/codeAction",
        json!({
            "textDocument": {"uri": uri},
            "range": diagnostic["range"],
            "context": {"diagnostics": [diagnostic]},
        }),
    );
    let trace = lsp
        .wait_for_trace_event("quick_fix_coverage code=prefer-const")
        .join("\n");
    assert!(
        trace.contains(
            "quick_fix_coverage code=prefer-const diagnostics=1 eligible=1 with_actions=1"
        ),
        "{trace}"
    );
    assert!(trace.contains("event=lint_cache_hit"), "{trace}");
    assert!(!trace.contains("event=lint_computation"), "{trace}");
    let action = actions
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["title"] == "replace `let` with `const`")
        .expect("prefer-const quick fix");
    let edit = &action["edit"]["changes"][&uri][0];
    assert_eq!(edit["newText"], "const");
    assert_eq!(edit["range"]["start"]["line"], 1);

    let updated_source = source.replace("let retries", "const retries");
    lsp.notify(
        "textDocument/didChange",
        json!({
            "textDocument": {"uri": uri, "version": 2},
            "contentChanges": [{"text": updated_source}],
        }),
    );
    lsp.wait_for_empty_diagnostics(&uri);
    let stale_actions = lsp.request(
        "textDocument/codeAction",
        json!({
            "textDocument": {"uri": uri},
            "range": diagnostic["range"],
            "context": {"diagnostics": [diagnostic]},
        }),
    );
    assert!(stale_actions.as_array().unwrap().is_empty());
}

#[test]
fn code_action_protocol_skips_published_diagnostics_outside_the_requested_range() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let uri = file_uri(&source_path);
    let source =
        "const main = (): void => {\n    let retries: i32 = 3;\n    console.log(retries);\n};\n";
    fs::write(&source_path, source).expect("write source");

    let mut lsp = LspProcess::start(workspace.path());
    lsp.open(&source_path, 1, source);
    let diagnostics =
        lsp.wait_for_diagnostic(&uri, |diagnostic| diagnostic["code"] == "prefer-const");
    let diagnostic = diagnostics
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == "prefer-const")
        .unwrap()
        .clone();
    let actions = lsp.request(
        "textDocument/codeAction",
        json!({
            "textDocument": {"uri": uri},
            "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 4}},
            "context": {"diagnostics": [diagnostic]},
        }),
    );

    assert_eq!(actions, json!([]));
}

#[test]
fn protected_import_quick_fix_reuses_the_published_analysis_snapshot() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let declaring_path = workspace.path().join("src/auth/token.tyr");
    fs::create_dir_all(declaring_path.parent().unwrap()).expect("create auth module directory");
    let source = "import { verify } from \"./auth/token\";\nconst main = (): void => {\n    verify(1);\n};\n";
    let declaring_source = "export protect const verify = (id: i32): i32 => id + 1;\n";
    fs::write(&source_path, source).expect("write importing source");
    fs::write(&declaring_path, declaring_source).expect("write protected declaration");

    let mut lsp = LspProcess::start_with_trace(workspace.path());
    let source_uri = file_uri(&source_path);
    let declaring_uri = file_uri(&declaring_path);
    lsp.open(&source_path, 1, source);
    let diagnostics =
        lsp.wait_for_diagnostic(&source_uri, |diagnostic| diagnostic["code"] == "E3308");
    let diagnostic = diagnostics
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == "E3308")
        .unwrap()
        .clone();
    let actions = lsp.request(
        "textDocument/codeAction",
        json!({
            "textDocument": {"uri": source_uri},
            "range": diagnostic["range"],
            "context": {"diagnostics": [diagnostic]},
        }),
    );
    let action = actions
        .as_array()
        .unwrap()
        .iter()
        .find(|action| {
            action["title"]
                .as_str()
                .is_some_and(|title| title.contains("Remove `protect` from `verify`"))
        })
        .expect("protected import quick fix");
    let edit = &action["edit"]["changes"][&declaring_uri][0];
    assert_eq!(edit["newText"], "");
    assert_eq!(edit["range"]["start"]["line"], 0);
    assert_eq!(edit["range"]["start"]["character"], 6);
}

#[test]
fn return_await_quick_fix_removes_only_the_redundant_await() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let source = "async function load(value: Promise<i32>): Promise<i32> { return await value; }\n";
    fs::write(&source_path, source).expect("write source");
    let mut lsp = LspProcess::start(workspace.path());
    let uri = file_uri(&source_path);
    lsp.open(&source_path, 1, source);
    let diagnostics = lsp.wait_for_diagnostic(&uri, |diagnostic| {
        diagnostic["code"] == "no-unnecessary-await"
    });
    let diagnostic = diagnostics
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == "no-unnecessary-await")
        .unwrap();
    let actions = lsp.request(
        "textDocument/codeAction",
        json!({
            "textDocument": {"uri": uri},
            "range": diagnostic["range"],
            "context": {"diagnostics": [diagnostic]},
        }),
    );
    let action = actions
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["title"] == "Remove the unnecessary `await`")
        .expect("return await quick fix");
    let edit = &action["edit"]["changes"][&uri][0];
    assert_eq!(edit["newText"], "");
    assert_eq!(edit["range"]["start"]["character"], 64);
    assert_eq!(edit["range"]["end"]["character"], 69);
}

#[test]
fn db_explicit_projection_quick_fix_uses_model_column_names() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let source = r#"@model("users")
interface User {
    @id
    id: i64;
    @column("email_address")
    email: string;
}

const users = db.query<User>("SELECT * FROM users", []);
"#;
    fs::write(&source_path, source).expect("write source");
    let mut lsp = LspProcess::start(workspace.path());
    let uri = file_uri(&source_path);
    lsp.open(&source_path, 1, source);
    let diagnostics = lsp.wait_for_diagnostic(&uri, |diagnostic| {
        diagnostic["code"] == "db-explicit-select-columns"
    });
    let diagnostic = diagnostics
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == "db-explicit-select-columns")
        .unwrap();
    let actions = lsp.request(
        "textDocument/codeAction",
        json!({
            "textDocument": {"uri": uri},
            "range": diagnostic["range"],
            "context": {"diagnostics": [diagnostic]},
        }),
    );
    let action = actions
        .as_array()
        .unwrap()
        .iter()
        .find(|action| {
            action["title"] == "Replace `*` with the fields declared by the result interface"
        })
        .expect("explicit projection quick fix");
    let edit = &action["edit"]["changes"][&uri][0];
    assert_eq!(edit["newText"], "\"id\", \"email_address\"");
    assert_eq!(edit["range"]["start"]["line"], 8);
    assert_eq!(edit["range"]["start"]["character"], 37);
    assert_eq!(edit["range"]["end"]["character"], 38);
}

#[test]
fn no_unused_imports_quick_fix_removes_only_the_unused_specifier() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let module_path = workspace.path().join("src/module.tyr");
    fs::write(
        &module_path,
        "export const unused: i32 = 1;\nexport const used: i32 = 2;\n",
    )
    .expect("write imported module");
    let source =
        "import { unused, used } from \"./module\";\nconst main = (): void => { print(used); };\n";
    fs::write(&source_path, source).expect("write source");

    let mut lsp = LspProcess::start(workspace.path());
    let uri = file_uri(&source_path);
    lsp.open(&source_path, 1, source);
    let diagnostics =
        lsp.wait_for_diagnostic(&uri, |diagnostic| diagnostic["code"] == "no-unused-imports");
    let diagnostic = diagnostics
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == "no-unused-imports")
        .unwrap()
        .clone();
    let actions = lsp.request(
        "textDocument/codeAction",
        json!({
            "textDocument": {"uri": uri},
            "range": diagnostic["range"],
            "context": {"diagnostics": [diagnostic]},
        }),
    );
    let action = actions
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["title"] == "remove unused import `unused`")
        .expect("unused import quick fix");
    let edit = &action["edit"]["changes"][&uri][0];
    assert_eq!(edit["newText"], "");
    assert_eq!(edit["range"]["start"]["line"], 0);
    assert_eq!(edit["range"]["start"]["character"], 9);
    assert_eq!(edit["range"]["end"]["line"], 0);
    assert_eq!(edit["range"]["end"]["character"], 17);
    let fixed = source.replacen("unused, ", "", 1);
    assert!(fixed.starts_with("import { used }"), "{fixed}");
}

#[test]
fn unused_caught_error_quick_fix_prefixes_only_the_binding() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let source = "const main = (): void => {\n    try {\n        print(\"ok\");\n    } catch (error: Error) {\n        print(\"failed\");\n    }\n};\n";
    fs::write(&source_path, source).expect("write source");

    let mut lsp = LspProcess::start(workspace.path());
    let uri = file_uri(&source_path);
    lsp.open(&source_path, 1, source);
    let diagnostics =
        lsp.wait_for_diagnostic(&uri, |diagnostic| diagnostic["code"] == "use-caught-error");
    let diagnostic = diagnostics
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == "use-caught-error")
        .unwrap()
        .clone();
    let actions = lsp.request(
        "textDocument/codeAction",
        json!({
            "textDocument": {"uri": uri},
            "range": diagnostic["range"],
            "context": {"diagnostics": [diagnostic]},
        }),
    );
    let action = actions
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["title"] == "Rename `error` to `_error`")
        .expect("unused catch binding quick fix");
    let edit = &action["edit"]["changes"][&uri][0];
    assert_eq!(edit["newText"], "_error");
    let binding_column = source.lines().nth(3).unwrap().find("error").unwrap() as u64;
    assert_eq!(edit["range"]["start"]["line"], 3);
    assert_eq!(edit["range"]["start"]["character"], binding_column);
    assert_eq!(
        edit["range"]["end"]["character"],
        binding_column + "error".len() as u64
    );
    let fixed = source.replacen("catch (error:", "catch (_error:", 1);
    assert!(fixed.contains("catch (_error: Error)"), "{fixed}");
}

#[test]
fn unused_simple_variable_quick_fix_prefixes_the_binding_when_name_is_available() {
    let workspace = temp_workspace();
    let source_path = workspace.path().join("src/main.tyr");
    let source = "function main(): void {\n    const unused: string = \"x\";\n}\n";
    fs::write(&source_path, source).expect("write source");

    let mut lsp = LspProcess::start(workspace.path());
    let uri = file_uri(&source_path);
    lsp.open(&source_path, 1, source);
    let diagnostics =
        lsp.wait_for_diagnostic(&uri, |diagnostic| diagnostic["code"] == "no-unused-vars");
    let diagnostic = diagnostics
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| {
            diagnostic["code"] == "no-unused-vars"
                && diagnostic["message"]
                    .as_str()
                    .is_some_and(|message| message.contains("`unused`"))
        })
        .unwrap()
        .clone();
    let actions = lsp.request(
        "textDocument/codeAction",
        json!({
            "textDocument": {"uri": uri},
            "range": diagnostic["range"],
            "context": {"diagnostics": [diagnostic]},
        }),
    );
    let action = actions
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["title"] == "Rename `unused` to `_unused`")
        .expect("unused variable quick fix");
    let edit = &action["edit"]["changes"][&uri][0];
    assert_eq!(edit["newText"], "_unused");
    assert_eq!(edit["range"]["start"]["line"], 1);
    assert_eq!(edit["range"]["start"]["character"], 10);
    assert_eq!(edit["range"]["end"]["character"], 16);
}
