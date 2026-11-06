use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

const STATE_FILE: &str = "clients.json";

#[derive(Clone, Debug)]
struct ClientSpec {
    name: &'static str,
    path: PathBuf,
    root_key: &'static str,
    is_zed: bool,
}

#[derive(Clone, Debug)]
struct ManagedEntry {
    name: String,
    path: PathBuf,
    root_key: String,
    entry: Value,
}

#[derive(Clone, Debug)]
pub struct ClientOutcome {
    pub name: String,
    pub status: String,
    pub detail: String,
}

pub fn install(root: &Path, executable: &Path) -> Vec<ClientOutcome> {
    let specs = detected_clients();
    let mut managed = load_state(root);
    let mut outcomes = Vec::new();
    for spec in specs {
        let entry = server_entry(&spec, executable);
        match install_one(root, &spec, &entry, &mut managed) {
            Ok(outcome) => outcomes.push(outcome),
            Err(error) => outcomes.push(ClientOutcome {
                name: spec.name.to_string(),
                status: "needs manual setup".to_string(),
                detail: error,
            }),
        }
    }
    if let Err(error) = save_state(root, &managed) {
        outcomes.push(ClientOutcome {
            name: "Tyrus MCP state".to_string(),
            status: "warning".to_string(),
            detail: error,
        });
    }
    if outcomes.is_empty() {
        outcomes.push(ClientOutcome {
            name: "MCP clients".to_string(),
            status: "not detected".to_string(),
            detail:
                "Add the server manually using the setup guide at https://tyruslang.org/tooling/mcp"
                    .to_string(),
        });
    }
    outcomes
}

pub fn status(root: &Path) -> Vec<ClientOutcome> {
    let managed = load_state(root);
    let detected = detected_clients();
    let mut outcomes = Vec::new();
    for spec in detected {
        let expected = server_entry(&spec, &current_executable());
        match read_config(&spec.path) {
            Ok(Some(value)) => {
                let actual = get_entry(&value, spec.root_key, "tyrus");
                let configured = actual == Some(&expected);
                let owned = managed.iter().any(|item| {
                    item.path == spec.path && item.name == spec.name && actual == Some(&item.entry)
                });
                outcomes.push(ClientOutcome {
                    name: spec.name.to_string(),
                    status: if configured {
                        "configured"
                    } else if actual.is_some() {
                        "conflict"
                    } else {
                        "not configured"
                    }
                    .to_string(),
                    detail: if configured && owned {
                        spec.path.display().to_string()
                    } else if configured {
                        format!("{} (pre-existing entry)", spec.path.display())
                    } else if actual.is_some() {
                        format!("`tyrus` entry differs in {}", spec.path.display())
                    } else {
                        spec.path.display().to_string()
                    },
                });
            }
            Ok(None) => outcomes.push(ClientOutcome {
                name: spec.name.to_string(),
                status: "not configured".to_string(),
                detail: spec.path.display().to_string(),
            }),
            Err(error) => outcomes.push(ClientOutcome {
                name: spec.name.to_string(),
                status: "needs manual review".to_string(),
                detail: error,
            }),
        }
    }
    outcomes
}

pub fn uninstall(root: &Path) -> Vec<ClientOutcome> {
    let managed = load_state(root);
    let mut outcomes = Vec::new();
    let mut retained = Vec::new();
    for record in &managed {
        match uninstall_one(record) {
            Ok(true) => outcomes.push(ClientOutcome {
                name: record.name.clone(),
                status: "removed".to_string(),
                detail: record.path.display().to_string(),
            }),
            Ok(false) => outcomes.push(ClientOutcome {
                name: record.name.clone(),
                status: "preserved changed entry".to_string(),
                detail: format!(
                    "Remove the Tyrus entry manually from {}",
                    record.path.display()
                ),
            }),
            Err(error) => {
                retained.push(record.clone());
                outcomes.push(ClientOutcome {
                    name: record.name.clone(),
                    status: "needs manual cleanup".to_string(),
                    detail: error,
                });
            }
        }
        if outcomes.last().is_some_and(|outcome| {
            outcome.name == record.name && outcome.status == "preserved changed entry"
        }) {
            retained.push(record.clone());
        }
    }
    if let Err(error) = save_state(root, &retained) {
        outcomes.push(ClientOutcome {
            name: "Tyrus MCP state".to_string(),
            status: "warning".to_string(),
            detail: error,
        });
    }
    outcomes
}

fn install_one(
    _root: &Path,
    spec: &ClientSpec,
    entry: &Value,
    managed: &mut Vec<ManagedEntry>,
) -> Result<ClientOutcome, String> {
    let existing_text = match fs::read_to_string(&spec.path) {
        Ok(source) => Some(source),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(format!("could not read {}: {error}", spec.path.display())),
    };
    let mut original = existing_text.clone().unwrap_or_else(|| "{}\n".to_string());
    let mut value = parse_jsonc(&original)?;
    let current = get_entry(&value, spec.root_key, "tyrus").cloned();
    let previous = managed
        .iter()
        .find(|record| record.path == spec.path && record.name == spec.name)
        .cloned();

    if current.as_ref() == Some(entry) {
        return Ok(ClientOutcome {
            name: spec.name.to_string(),
            status: "already configured".to_string(),
            detail: spec.path.display().to_string(),
        });
    }

    let mut was_managed = false;
    if let (Some(current_entry), Some(previous_record)) = (&current, &previous) {
        if current_entry == &previous_record.entry {
            let source = strip_jsonc(&original);
            let root_object = find_object_property(&source, 0, &previous_record.root_key)
                .ok_or_else(|| {
                    "could not locate the managed client configuration object".to_string()
                })?;
            let server_property = find_object_property(&source, root_object.0, "tyrus")
                .ok_or_else(|| "could not locate the managed Tyrus entry".to_string())?;
            if contains_comment(&original[server_property.1.0..server_property.1.1]) {
                return Err("the managed `tyrus` entry now contains comments; preserve it and update manually".to_string());
            }
            original.replace_range(
                server_property.1.0..server_property.1.1,
                &compact_json(entry)?,
            );
            value = parse_jsonc(&original)?;
            was_managed = true;
        } else {
            return Err(format!(
                "an existing `tyrus` entry conflicts in {}; it was left unchanged",
                spec.path.display()
            ));
        }
    } else if current.is_some() {
        return Err(format!(
            "an existing `tyrus` entry was not created by this installer in {}; it was left unchanged",
            spec.path.display()
        ));
    } else {
        let updated = add_entry(&original, spec.root_key, "tyrus", entry)?;
        original = updated;
        value = parse_jsonc(&original)?;
    }

    if existing_text.is_none() {
        if let Some(parent) = spec.path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!("could not create client configuration directory: {error}")
            })?;
        }
    } else if !was_managed {
        create_backup(&spec.path)?;
    }
    atomic_write(&spec.path, original.as_bytes())?;
    let actual = get_entry(&value, spec.root_key, "tyrus").ok_or_else(|| {
        "the Tyrus entry was not present after writing the configuration".to_string()
    })?;
    let record = ManagedEntry {
        name: spec.name.to_string(),
        path: spec.path.clone(),
        root_key: spec.root_key.to_string(),
        entry: actual.clone(),
    };
    managed.retain(|item| !(item.path == record.path && item.name == record.name));
    managed.push(record);
    Ok(ClientOutcome {
        name: spec.name.to_string(),
        status: if was_managed { "updated" } else { "configured" }.to_string(),
        detail: spec.path.display().to_string(),
    })
}

fn uninstall_one(record: &ManagedEntry) -> Result<bool, String> {
    let mut source = match fs::read_to_string(&record.path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(error) => return Err(format!("could not read {}: {error}", record.path.display())),
    };
    let value = parse_jsonc(&source)?;
    if get_entry(&value, &record.root_key, "tyrus") != Some(&record.entry) {
        return Ok(false);
    }
    let sanitized = strip_jsonc(&source);
    let root = find_object_property(&sanitized, 0, &record.root_key)
        .ok_or_else(|| "could not locate the MCP settings object".to_string())?;
    let properties = object_properties(&sanitized, root.0)?;
    let Some(index) = properties
        .iter()
        .position(|property| property.key == "tyrus")
    else {
        return Ok(true);
    };
    let property = &properties[index];
    if contains_comment(&source[property.key_start..property.value_end]) {
        return Ok(false);
    }
    let (start, end) = if let Some(comma) = property.comma_after {
        (property.key_start, comma + 1)
    } else if index > 0 {
        (
            properties[index - 1]
                .comma_after
                .unwrap_or(property.key_start),
            property.value_end,
        )
    } else {
        (property.key_start, property.value_end)
    };
    source.replace_range(start..end, "");
    atomic_write(&record.path, source.as_bytes())?;
    Ok(true)
}

fn detected_clients() -> Vec<ClientSpec> {
    let Some(base) = directories::BaseDirs::new() else {
        return Vec::new();
    };
    let home = base.home_dir();
    let config = base.config_dir();
    let app_data = env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| config.to_path_buf());
    let mut candidates = Vec::new();

    let claude = if cfg!(target_os = "macos") {
        home.join("Library/Application Support/Claude/claude_desktop_config.json")
    } else if cfg!(target_os = "windows") {
        app_data.join("Claude/claude_desktop_config.json")
    } else {
        let xdg = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        xdg.join("Claude/claude_desktop_config.json")
    };
    if claude.exists() || app_dir_exists(&claude, "Claude") {
        candidates.push(ClientSpec {
            name: "Claude Desktop",
            path: claude,
            root_key: "mcpServers",
            is_zed: false,
        });
    }

    let cursor = home.join(".cursor/mcp.json");
    if cursor.exists() || home.join(".cursor").exists() || executable_on_path("cursor") {
        candidates.push(ClientSpec {
            name: "Cursor",
            path: cursor,
            root_key: "mcpServers",
            is_zed: false,
        });
    }

    let zed = if cfg!(target_os = "macos") {
        home.join("Library/Application Support/Zed/settings.json")
    } else if cfg!(target_os = "windows") {
        app_data.join("Zed/settings.json")
    } else {
        config.join("zed/settings.json")
    };
    if zed.exists() || app_dir_exists(&zed, "zed") || executable_on_path("zed") {
        candidates.push(ClientSpec {
            name: "Zed",
            path: zed,
            root_key: "context_servers",
            is_zed: true,
        });
    }

    let copilot_home = env::var_os("COPILOT_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".copilot"));
    let vscode = copilot_home.join("mcp-config.json");
    if vscode.exists() || copilot_home.exists() || executable_on_path("code") {
        candidates.push(ClientSpec {
            name: "VS Code Copilot",
            path: vscode,
            root_key: "mcpServers",
            is_zed: false,
        });
    }

    candidates
}

fn app_dir_exists(config_path: &Path, app_name: &str) -> bool {
    config_path.parent().is_some_and(Path::exists)
        || env::var_os("HOME").is_some_and(|home| {
            PathBuf::from(home)
                .join("Applications")
                .join(format!("{app_name}.app"))
                .exists()
        })
        || PathBuf::from("/Applications")
            .join(format!("{app_name}.app"))
            .exists()
}

fn executable_on_path(name: &str) -> bool {
    env::var_os("PATH").is_some_and(|paths| {
        env::split_paths(&paths).any(|directory| {
            let executable = if cfg!(target_os = "windows") {
                directory.join(format!("{name}.exe"))
            } else {
                directory.join(name)
            };
            executable.is_file()
        })
    })
}

fn server_entry(spec: &ClientSpec, executable: &Path) -> Value {
    if spec.is_zed {
        json!({ "command": executable.to_string_lossy(), "args": ["mcp", "serve"], "env": {} })
    } else {
        json!({ "command": executable.to_string_lossy(), "args": ["mcp", "serve"] })
    }
}

fn read_config(path: &Path) -> Result<Option<Value>, String> {
    match fs::read_to_string(path) {
        Ok(source) => parse_jsonc(&source).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("could not read {}: {error}", path.display())),
    }
}

fn get_entry<'a>(value: &'a Value, root_key: &str, entry_key: &str) -> Option<&'a Value> {
    value.get(root_key)?.as_object()?.get(entry_key)
}

fn add_entry(
    source: &str,
    root_key: &str,
    entry_key: &str,
    entry: &Value,
) -> Result<String, String> {
    let value = parse_jsonc(source)?;
    if !value.is_object() {
        return Err("client configuration root must be a JSON object".to_string());
    }
    let sanitized = strip_jsonc(source);
    if let Some(root) = find_object_property(&sanitized, 0, root_key) {
        let Some(root_value) = value.get(root_key) else {
            return Err(format!("client setting `{root_key}` is not an object"));
        };
        if !root_value.is_object() {
            return Err(format!("client setting `{root_key}` is not an object"));
        }
        insert_object_property(source, root.0, entry_key, entry)
    } else {
        let mut object = Map::new();
        object.insert(entry_key.to_string(), entry.clone());
        insert_object_property(source, 0, root_key, &Value::Object(object))
    }
}

fn insert_object_property(
    source: &str,
    object_start: usize,
    key: &str,
    value: &Value,
) -> Result<String, String> {
    let sanitized = strip_jsonc(source);
    let object_end = scan_value_end(&sanitized, object_start)?;
    let properties = object_properties(&sanitized, object_start)?;
    if properties.iter().any(|property| property.key == key) {
        return Err(format!("setting `{key}` already exists"));
    }
    let closing = object_end - 1;
    let (indent, newline) = formatting(
        source,
        properties.first().map(|property| property.key_start),
        closing,
    );
    let (position, insertion) = if let Some(last) = properties.last() {
        (
            last.value_end,
            format!(
                ",{newline}{indent}{:?}: {}",
                key,
                pretty_json(value, &indent, newline)?
            ),
        )
    } else {
        (
            closing,
            format!(
                "{newline}{indent}{:?}: {}{newline}{}",
                key,
                pretty_json(value, &indent, newline)?,
                closing_indent(source, closing)
            ),
        )
    };
    let mut updated = source.to_string();
    updated.insert_str(position, &insertion);
    Ok(updated)
}

#[derive(Debug)]
struct Property {
    key: String,
    key_start: usize,
    value_start: usize,
    value_end: usize,
    comma_after: Option<usize>,
}

fn find_object_property(
    source: &str,
    object_start: usize,
    key: &str,
) -> Option<(usize, (usize, usize))> {
    let properties = object_properties(source, object_start).ok()?;
    let property = properties.iter().find(|property| property.key == key)?;
    let object_end = scan_value_end(source, property.value_start).ok()?;
    (source.as_bytes().get(property.value_start) == Some(&b'{'))
        .then_some((property.value_start, (property.value_start, object_end)))
}

fn object_properties(source: &str, object_start: usize) -> Result<Vec<Property>, String> {
    if source.as_bytes().get(object_start) != Some(&b'{') {
        return Err("expected a JSON object".to_string());
    }
    let mut properties = Vec::new();
    let mut cursor = object_start + 1;
    loop {
        cursor = skip_whitespace(source, cursor);
        if source.as_bytes().get(cursor) == Some(&b'}') {
            return Ok(properties);
        }
        let key_start = cursor;
        let key_end = scan_string_end(source, cursor)?;
        let key: String = serde_json::from_str(&source[key_start..key_end])
            .map_err(|_| "invalid JSON object key".to_string())?;
        cursor = skip_whitespace(source, key_end);
        if source.as_bytes().get(cursor) != Some(&b':') {
            return Err("invalid JSON object separator".to_string());
        }
        let value_start = skip_whitespace(source, cursor + 1);
        let value_end = scan_value_end(source, value_start)?;
        cursor = skip_whitespace(source, value_end);
        let comma_after = if source.as_bytes().get(cursor) == Some(&b',') {
            let comma = cursor;
            cursor += 1;
            Some(comma)
        } else {
            None
        };
        properties.push(Property {
            key,
            key_start,
            value_start,
            value_end,
            comma_after,
        });
        if comma_after.is_none() {
            return Ok(properties);
        }
    }
}

fn scan_value_end(source: &str, start: usize) -> Result<usize, String> {
    let bytes = source.as_bytes();
    match bytes.get(start) {
        Some(b'"') => scan_string_end(source, start),
        Some(b'{') | Some(b'[') => {
            let mut stack = vec![if bytes[start] == b'{' { b'}' } else { b']' }];
            let mut cursor = start + 1;
            while cursor < bytes.len() {
                match bytes[cursor] {
                    b'"' => cursor = scan_string_end(source, cursor)?,
                    b'{' => {
                        stack.push(b'}');
                        cursor += 1;
                    }
                    b'[' => {
                        stack.push(b']');
                        cursor += 1;
                    }
                    b'}' | b']' => {
                        if stack.pop() != Some(bytes[cursor]) {
                            return Err("mismatched JSON delimiter".to_string());
                        }
                        cursor += 1;
                        if stack.is_empty() {
                            return Ok(cursor);
                        }
                    }
                    _ => cursor += 1,
                }
            }
            Err("unterminated JSON value".to_string())
        }
        Some(_) => {
            let mut cursor = start;
            while cursor < bytes.len() && !matches!(bytes[cursor], b',' | b'}' | b']') {
                cursor += 1;
            }
            Ok(cursor)
        }
        None => Err("missing JSON value".to_string()),
    }
}

fn scan_string_end(source: &str, start: usize) -> Result<usize, String> {
    let bytes = source.as_bytes();
    if bytes.get(start) != Some(&b'"') {
        return Err("expected a JSON string".to_string());
    }
    let mut cursor = start + 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\\' => cursor += 2,
            b'"' => return Ok(cursor + 1),
            _ => cursor += 1,
        }
    }
    Err("unterminated JSON string".to_string())
}

fn skip_whitespace(source: &str, mut cursor: usize) -> usize {
    let bytes = source.as_bytes();
    while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    cursor
}

fn parse_jsonc(source: &str) -> Result<Value, String> {
    let sanitized = strip_jsonc(source);
    serde_json::from_str(&sanitized)
        .map_err(|error| format!("client configuration is not valid JSON/JSONC: {error}"))
}

fn strip_jsonc(source: &str) -> String {
    let mut output = source.as_bytes().to_vec();
    let mut cursor = 0;
    let mut in_string = false;
    let mut escaped = false;
    while cursor < output.len() {
        if in_string {
            if escaped {
                escaped = false;
            } else if output[cursor] == b'\\' {
                escaped = true;
            } else if output[cursor] == b'"' {
                in_string = false;
            }
            cursor += 1;
            continue;
        }
        if output[cursor] == b'"' {
            in_string = true;
            cursor += 1;
        } else if output.get(cursor..cursor + 2) == Some(b"//") {
            output[cursor] = b' ';
            output[cursor + 1] = b' ';
            cursor += 2;
            while cursor < output.len() && !matches!(output[cursor], b'\n' | b'\r') {
                output[cursor] = b' ';
                cursor += 1;
            }
        } else if output.get(cursor..cursor + 2) == Some(b"/*") {
            output[cursor] = b' ';
            output[cursor + 1] = b' ';
            cursor += 2;
            while cursor + 1 < output.len() && output.get(cursor..cursor + 2) != Some(b"*/") {
                if !matches!(output[cursor], b'\n' | b'\r') {
                    output[cursor] = b' ';
                }
                cursor += 1;
            }
            if cursor + 1 < output.len() {
                output[cursor] = b' ';
                output[cursor + 1] = b' ';
                cursor += 2;
            }
        } else if output[cursor] == b',' {
            let mut next = cursor + 1;
            while next < output.len() && output[next].is_ascii_whitespace() {
                next += 1;
            }
            if matches!(output.get(next), Some(b'}' | b']')) {
                output[cursor] = b' ';
            }
            cursor += 1;
        } else {
            cursor += 1;
        }
    }
    String::from_utf8(output).unwrap_or_default()
}

fn contains_comment(source: &str) -> bool {
    let bytes = source.as_bytes();
    let mut cursor = 0;
    let mut in_string = false;
    let mut escaped = false;
    while cursor < bytes.len() {
        if in_string {
            if escaped {
                escaped = false;
            } else if bytes[cursor] == b'\\' {
                escaped = true;
            } else if bytes[cursor] == b'"' {
                in_string = false;
            }
            cursor += 1;
        } else if bytes[cursor] == b'"' {
            in_string = true;
            cursor += 1;
        } else if bytes
            .get(cursor..cursor + 2)
            .is_some_and(|pair| pair == b"//" || pair == b"/*")
        {
            return true;
        } else {
            cursor += 1;
        }
    }
    false
}

fn formatting(source: &str, first_key: Option<usize>, closing: usize) -> (String, &str) {
    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let close_indent = closing_indent(source, closing);
    let indent = first_key.map_or_else(
        || format!("{close_indent}  "),
        |position| {
            let line_start = source[..position].rfind('\n').map_or(0, |index| index + 1);
            source[line_start..position].to_string()
        },
    );
    (indent, newline)
}

fn closing_indent(source: &str, closing: usize) -> String {
    let line_start = source[..closing].rfind('\n').map_or(0, |index| index + 1);
    source[line_start..closing]
        .chars()
        .take_while(|character| matches!(character, ' ' | '\t'))
        .collect()
}

fn pretty_json(value: &Value, indent: &str, newline: &str) -> Result<String, String> {
    let serialized = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    Ok(serialized.replace('\n', &format!("{newline}{indent}")))
}

fn compact_json(value: &Value) -> Result<String, String> {
    serde_json::to_string(value).map_err(|error| error.to_string())
}

fn create_backup(path: &Path) -> Result<(), String> {
    let backup = path.with_extension(format!("mcp-backup-{}", std::process::id()));
    if backup.exists() {
        return Ok(());
    }
    fs::copy(path, &backup)
        .map_err(|error| format!("could not back up {}: {error}", path.display()))?;
    Ok(())
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "configuration path has no parent directory".to_string())?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("could not stage {}: {error}", path.display()))?;
    std::io::Write::write_all(&mut temporary, content)
        .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    temporary
        .persist(path)
        .map_err(|error| format!("could not activate {}: {}", path.display(), error.error))?;
    Ok(())
}

fn load_state(root: &Path) -> Vec<ManagedEntry> {
    let Ok(bytes) = fs::read(root.join(STATE_FILE)) else {
        return Vec::new();
    };
    let Ok(values) = serde_json::from_slice::<Vec<Value>>(&bytes) else {
        return Vec::new();
    };
    values
        .into_iter()
        .filter_map(|value| {
            Some(ManagedEntry {
                name: value.get("name")?.as_str()?.to_string(),
                path: PathBuf::from(value.get("path")?.as_str()?),
                root_key: value.get("rootKey")?.as_str()?.to_string(),
                entry: value.get("entry")?.clone(),
            })
        })
        .collect()
}

fn save_state(root: &Path, records: &[ManagedEntry]) -> Result<(), String> {
    let values = records
        .iter()
        .map(|record| {
            json!({
                "name": record.name,
                "path": record.path,
                "rootKey": record.root_key,
                "entry": record.entry,
            })
        })
        .collect::<Vec<_>>();
    let bytes = serde_json::to_vec_pretty(&values)
        .map_err(|error| format!("could not encode client state: {error}"))?;
    atomic_write(&root.join(STATE_FILE), &bytes)
}

fn current_executable() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|path| fs::canonicalize(path).ok())
        .unwrap_or_else(|| PathBuf::from("tyrus"))
}

#[cfg(test)]
mod tests {
    use super::{
        ClientSpec, ManagedEntry, add_entry, install_one, parse_jsonc, server_entry, uninstall_one,
    };
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn install_preserves_comments_and_uninstall_removes_only_tyrus_entry() {
        let source = "{\n  // keep this setting\n  \"other\": {\"enabled\": true}\n}\n";
        let entry = json!({ "command": "/usr/bin/tyrus", "args": ["mcp", "serve"] });
        let updated = add_entry(source, "mcpServers", "tyrus", &entry).unwrap();
        assert!(updated.contains("// keep this setting"));
        assert_eq!(parse_jsonc(&updated).unwrap()["mcpServers"]["tyrus"], entry);

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        fs::write(&path, &updated).unwrap();
        let record = ManagedEntry {
            name: "Example Client".to_string(),
            path: path.clone(),
            root_key: "mcpServers".to_string(),
            entry,
        };
        assert!(uninstall_one(&record).unwrap());
        let remaining = fs::read_to_string(path).unwrap();
        assert!(remaining.contains("// keep this setting"));
        assert!(parse_jsonc(&remaining).unwrap()["other"]["enabled"] == true);
        assert!(parse_jsonc(&remaining).unwrap()["mcpServers"]["tyrus"].is_null());
    }

    #[test]
    fn existing_tyrus_configuration_is_not_overwritten_by_addition() {
        let existing = json!({ "mcpServers": { "tyrus": { "command": "custom" } } });
        assert_eq!(existing["mcpServers"]["tyrus"]["command"], "custom");
    }

    #[test]
    fn zed_entry_uses_the_context_servers_shape() {
        let spec = ClientSpec {
            name: "Zed",
            path: PathBuf::from("zed.json"),
            root_key: "context_servers",
            is_zed: true,
        };
        let entry = server_entry(&spec, PathBuf::from("/usr/bin/tyrus").as_path());
        assert_eq!(entry["command"], "/usr/bin/tyrus");
        assert_eq!(entry["args"][0], "mcp");
        assert!(entry["env"].is_object());
    }

    #[test]
    fn installation_is_idempotent_and_refuses_a_later_conflict() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("state");
        fs::create_dir_all(&root).unwrap();
        let path = directory.path().join("client.json");
        fs::write(&path, "{\n  // preserved\n  \"other\": true\n}\n").unwrap();
        let spec = ClientSpec {
            name: "Test Client",
            path: path.clone(),
            root_key: "mcpServers",
            is_zed: false,
        };
        let entry = server_entry(&spec, PathBuf::from("/usr/bin/tyrus").as_path());
        let mut managed = Vec::new();
        assert_eq!(
            install_one(&root, &spec, &entry, &mut managed)
                .unwrap()
                .status,
            "configured"
        );
        assert_eq!(
            install_one(&root, &spec, &entry, &mut managed)
                .unwrap()
                .status,
            "already configured"
        );
        assert!(
            path.with_extension(format!("mcp-backup-{}", std::process::id()))
                .exists()
        );
        let config = fs::read_to_string(&path).unwrap();
        assert!(config.contains("// preserved"));
        assert_eq!(parse_jsonc(&config).unwrap()["mcpServers"]["tyrus"], entry);

        fs::write(
            &path,
            "{\"mcpServers\":{\"tyrus\":{\"command\":\"custom\"}}}\n",
        )
        .unwrap();
        let result = install_one(&root, &spec, &entry, &mut managed);
        assert!(result.unwrap_err().contains("conflicts"));
        assert!(fs::read_to_string(path).unwrap().contains("custom"));
    }

    #[test]
    fn update_replaces_a_managed_entry_and_refreshes_its_removal_record() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("state");
        fs::create_dir_all(&root).unwrap();
        let path = directory.path().join("client.json");
        fs::write(&path, "{\n  // preserved\n  \"other\": true\n}\n").unwrap();
        let spec = ClientSpec {
            name: "Test Client",
            path: path.clone(),
            root_key: "mcpServers",
            is_zed: false,
        };
        let first_entry = server_entry(&spec, PathBuf::from("/usr/bin/tyrus-v1").as_path());
        let updated_entry = server_entry(&spec, PathBuf::from("/usr/bin/tyrus-v2").as_path());
        let mut managed = Vec::new();

        assert_eq!(
            install_one(&root, &spec, &first_entry, &mut managed)
                .unwrap()
                .status,
            "configured"
        );
        assert_eq!(
            install_one(&root, &spec, &updated_entry, &mut managed)
                .unwrap()
                .status,
            "updated"
        );
        assert_eq!(managed.len(), 1);
        assert_eq!(managed[0].entry, updated_entry);
        assert_eq!(
            parse_jsonc(&fs::read_to_string(&path).unwrap()).unwrap()["mcpServers"]["tyrus"],
            updated_entry
        );

        assert!(uninstall_one(&managed[0]).unwrap());
        let remaining = fs::read_to_string(path).unwrap();
        assert!(remaining.contains("// preserved"));
        assert!(parse_jsonc(&remaining).unwrap()["mcpServers"]["tyrus"].is_null());
    }

    #[test]
    fn install_creates_a_valid_new_client_configuration() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("state");
        fs::create_dir_all(&root).unwrap();
        let path = directory.path().join("mcp-config.json");
        let spec = ClientSpec {
            name: "VS Code Copilot",
            path: path.clone(),
            root_key: "mcpServers",
            is_zed: false,
        };
        let entry = server_entry(&spec, PathBuf::from("/usr/bin/tyrus").as_path());
        let outcome = install_one(&root, &spec, &entry, &mut Vec::new()).unwrap();
        let config = parse_jsonc(&fs::read_to_string(path).unwrap()).unwrap();

        assert_eq!(outcome.status, "configured");
        assert_eq!(config["mcpServers"]["tyrus"], entry);
    }

    #[test]
    fn uninstall_preserves_a_managed_entry_changed_by_the_user() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("client.json");
        fs::write(
            &path,
            "{\"mcpServers\":{\"tyrus\":{\"command\":\"changed\"}}}\n",
        )
        .unwrap();
        let record = ManagedEntry {
            name: "Test Client".to_string(),
            path: path.clone(),
            root_key: "mcpServers".to_string(),
            entry: json!({"command":"original"}),
        };
        assert!(!uninstall_one(&record).unwrap());
        assert!(fs::read_to_string(path).unwrap().contains("changed"));
    }

    #[test]
    fn invalid_client_configuration_is_left_unchanged() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("state");
        fs::create_dir_all(&root).unwrap();
        let path = directory.path().join("client.json");
        let original = b"{ invalid json";
        fs::write(&path, original).unwrap();
        let spec = ClientSpec {
            name: "Test Client",
            path: path.clone(),
            root_key: "mcpServers",
            is_zed: false,
        };
        let entry = server_entry(&spec, PathBuf::from("/usr/bin/tyrus").as_path());
        assert!(install_one(&root, &spec, &entry, &mut Vec::new()).is_err());
        assert_eq!(fs::read(path).unwrap(), original);
    }
}
