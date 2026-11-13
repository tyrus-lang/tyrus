use crate::collections::HashMap;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct ImportMap {
    aliases: Vec<(String, PathBuf)>,
}

#[derive(Deserialize)]
struct ImportManifest {
    #[serde(default)]
    imports: Option<ImportEntries>,
}

struct ImportEntries(Vec<(String, String)>);

impl<'de> Deserialize<'de> for ImportEntries {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ImportEntriesVisitor;

        impl<'de> Visitor<'de> for ImportEntriesVisitor {
            type Value = ImportEntries;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an object mapping import aliases to string destinations")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut entries = Vec::new();
                let mut seen = std::collections::HashSet::new();
                while let Some((key, destination)) = map.next_entry::<String, String>()? {
                    if !seen.insert(key.clone()) {
                        return Err(serde::de::Error::custom(format!(
                            "duplicate import alias `{key}`"
                        )));
                    }
                    entries.push((key, destination));
                }
                Ok(ImportEntries(entries))
            }
        }

        deserializer.deserialize_map(ImportEntriesVisitor)
    }
}

impl ImportMap {
    pub fn load(root: &Path) -> Result<Self, String> {
        let config_path = root.join("tyrus.json");
        if !config_path.is_file() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&config_path)
            .map_err(|error| format!("could not read tyrus.json: {error}"))?;
        let manifest: ImportManifest = serde_json::from_str(&content)
            .map_err(|error| format!("invalid tyrus.json: {error}"))?;
        let Some(imports) = manifest.imports else {
            return Ok(Self::default());
        };
        let mut map = Self::default();
        for (key, destination) in imports.0 {
            validate_import_alias(&key, &destination)?;
            let destination = PathBuf::from(destination);
            map.aliases.push((key, destination));
        }
        map.aliases
            .sort_by_key(|(alias, _)| std::cmp::Reverse(alias.len()));
        Ok(map)
    }

    pub fn resolve(&self, root: &Path, source: &str) -> Result<Option<PathBuf>, String> {
        let destination = if let Some((alias, destination)) =
            self.aliases.iter().find_map(|(alias, destination)| {
                import_alias_subpath(alias, source).map(|subpath| (subpath, destination))
            }) {
            if !alias.is_empty() {
                validate_alias_capture(source, alias)?;
            }
            if alias.is_empty() {
                destination.clone()
            } else {
                destination.join(alias)
            }
        } else {
            return Ok(None);
        };
        resolve_project_path(root, &destination, source).map(Some)
    }

    pub fn source_for_path(&self, root: &Path, target: &Path) -> Option<String> {
        let target = target.strip_prefix(root).ok()?;
        self.aliases
            .iter()
            .filter_map(|(alias, destination)| {
                let destination = root.join(destination);
                let suffix = target
                    .strip_prefix(destination.strip_prefix(root).ok()?)
                    .ok()?;
                let suffix = suffix
                    .with_extension("")
                    .to_string_lossy()
                    .replace('\\', "/");
                let specificity = destination.components().count();
                let source = if suffix.is_empty() {
                    alias.clone()
                } else {
                    format!("{alias}/{suffix}")
                };
                Some((specificity, source))
            })
            .max_by_key(|(specificity, _)| *specificity)
            .map(|(_, source)| source)
    }
}

pub fn find_project_root(file: &Path) -> Option<PathBuf> {
    let mut directory = file.parent();
    while let Some(current) = directory {
        if current.join("tyrus.json").is_file() {
            return Some(current.to_path_buf());
        }
        directory = current.parent();
    }
    None
}

pub fn resolve_import_path(document: &Path, source: &str) -> Result<Option<PathBuf>, String> {
    if source.starts_with('.') {
        let parent = document.parent().unwrap_or_else(|| Path::new("."));
        return resolve_candidate_path(parent.join(source), source).map(Some);
    }
    if !source.starts_with('#') {
        return Ok(None);
    }
    let Some(root) = find_project_root(document) else {
        return Err(format!(
            "import alias `{source}` requires a tyrus.json project manifest"
        ));
    };
    let root = root.canonicalize().map_err(|error| {
        format!(
            "could not resolve project root `{}`: {error}",
            root.display()
        )
    })?;
    let imports = ImportMap::load(&root)?;
    imports
        .resolve(&root, source)?
        .map(Some)
        .ok_or_else(|| format!("import alias `{source}` is not defined in tyrus.json"))
}

pub fn resolve_import_path_in_project(
    root: &Path,
    imports: &ImportMap,
    importer: &Path,
    source: &str,
) -> Result<PathBuf, String> {
    if source.starts_with('.') {
        return resolve_candidate_path(
            importer
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(source),
            source,
        );
    }
    if source.starts_with('#') {
        return imports
            .resolve(root, source)?
            .ok_or_else(|| format!("import alias `{source}` is not defined in tyrus.json"));
    }
    Err(format!(
        "unsupported module `{source}`; use a relative path, an import alias, or a `tyrus:` module"
    ))
}

fn validate_import_alias(key: &str, destination: &str) -> Result<(), String> {
    if !key.starts_with('#') {
        return Err(format!(
            "invalid tyrus.json: import alias `{key}` must start with '#'"
        ));
    }
    if key.contains('\\') || destination.contains('\\') {
        return Err(format!(
            "invalid tyrus.json: import alias `{key}` must use '/' as its path separator"
        ));
    }
    if key.contains('*') {
        return Err(format!(
            "invalid tyrus.json: import alias `{key}` must not contain `*`"
        ));
    }
    if key
        .split('/')
        .any(|segment| matches!(segment, "." | "..") || segment.is_empty())
    {
        return Err(format!(
            "invalid tyrus.json: import alias `{key}` has an invalid path segment"
        ));
    }
    if !destination.starts_with("./") || destination.is_empty() {
        return Err(format!(
            "invalid tyrus.json: import alias `{key}` must use a `./` destination"
        ));
    }
    if destination.contains('*') {
        return Err(format!(
            "invalid tyrus.json: import alias `{key}` must not use a destination wildcard"
        ));
    }
    if Path::new(destination).components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(format!(
            "invalid tyrus.json: import alias `{key}` escapes the project root"
        ));
    }
    if Path::new(destination).extension().is_some() {
        return Err(format!(
            "invalid tyrus.json: import alias `{key}` must target a directory, not a file"
        ));
    }
    Ok(())
}

fn import_alias_subpath<'a>(alias: &str, source: &'a str) -> Option<&'a str> {
    if source == alias {
        return Some("");
    }
    source
        .strip_prefix(alias)?
        .strip_prefix('/')
        .filter(|subpath| !subpath.is_empty())
}

fn validate_alias_capture(source: &str, capture: &str) -> Result<(), String> {
    if capture.is_empty()
        || Path::new(capture).components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("import alias `{source}` has an invalid subpath"));
    }
    Ok(())
}

fn resolve_project_path(root: &Path, destination: &Path, source: &str) -> Result<PathBuf, String> {
    let canonical_root = root.canonicalize().map_err(|error| {
        format!(
            "could not resolve project root `{}`: {error}",
            root.display()
        )
    })?;
    let candidate = resolve_candidate_path(canonical_root.join(destination), source)?;
    if !candidate.starts_with(&canonical_root) {
        return Err(format!(
            "import alias `{source}` resolves outside the project root"
        ));
    }
    Ok(candidate)
}

fn resolve_candidate_path(mut candidate: PathBuf, source: &str) -> Result<PathBuf, String> {
    if candidate.is_dir() {
        candidate = candidate.join("index.tyr");
    } else if !candidate.is_file() {
        let mut with_extension = candidate.clone().into_os_string();
        with_extension.push(".tyr");
        let with_extension = PathBuf::from(with_extension);
        if with_extension.is_file() {
            candidate = with_extension;
        } else if candidate.extension().is_none() {
            candidate.set_extension("tyr");
        }
    }
    candidate
        .canonicalize()
        .map_err(|error| format!("could not resolve module `{source}`: {error}"))
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ProjectConfig {
    pub project: ProjectInfo,
    #[serde(default)]
    pub build: BuildConfig,
    #[serde(default)]
    pub scripts: ScriptsConfig,
    #[serde(default)]
    pub security: Option<SecurityConfig>,
    #[serde(default)]
    pub runtime: RuntimeConfig,
    #[serde(default)]
    pub development: DevelopmentConfig,
    #[serde(default)]
    pub database: Option<crate::db::DatabaseConfig>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ProjectInfo {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct BuildConfig {
    #[serde(default)]
    pub optimize: bool,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct SecurityConfig {
    #[serde(rename = "envFile", default)]
    pub env_file: Option<String>,
    #[serde(rename = "envSealed", default)]
    pub env_sealed: bool,
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    #[serde(rename = "workerThreads", default)]
    pub worker_threads: Option<u32>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct DevelopmentConfig {
    #[serde(default)]
    pub https: bool,
}

pub type ScriptsConfig = HashMap<String, String>;

pub fn load_project(dir: &Path) -> Result<ProjectConfig, String> {
    let config_path = dir.join("tyrus.json");

    if !config_path.exists() {
        return Err(format!(
            "error: tyrus.json not found in '{}'\nRun `tyrus new <name>` to create a project.",
            dir.display()
        ));
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("error: could not read tyrus.json: {e}"))?;

    let config: ProjectConfig =
        serde_json::from_str(&content).map_err(|e| format!("error: invalid tyrus.json: {e}"))?;
    if let Some(database) = &config.database {
        database.validate()?;
    }
    Ok(config)
}

pub fn default_project_config_json(project_name: &str) -> String {
    project_config_json(project_name, None)
}

pub fn project_config_json(project_name: &str, target: Option<&str>) -> String {
    let target = target
        .map(|target| format!(",\n    \"target\": {}", json_string(target)))
        .unwrap_or_default();
    format!(
        concat!(
            "{{\n",
            "  \"project\": {{\n",
            "    \"name\": {},\n",
            "    \"version\": \"0.1.0\",\n",
            "    \"authors\": [],\n",
            "    \"description\": \"\"{}\n",
            "  }},\n",
            "  \"scripts\": {{\n",
            "    \"fmt\": \"tyrus fmt\",\n",
            "    \"lint\": \"tyrus lint\",\n",
            "    \"test\": \"tyrus test\"\n",
            "  }},\n",
            "  \"security\": {{\n",
            "    \"envFile\": \".env\",\n",
            "    \"envSealed\": false\n",
            "  }},\n",
            "  \"imports\": {{\n",
            "    \"#\": \"./src\"\n",
            "  }}\n",
            "}}\n"
        ),
        json_string(project_name),
        target,
    )
}

pub fn project_config_json_with_database(
    project_name: &str,
    target: Option<&str>,
    database: Option<serde_json::Value>,
) -> String {
    let Some(database) = database else {
        return project_config_json(project_name, target);
    };
    let mut value: serde_json::Value =
        serde_json::from_str(&project_config_json(project_name, target))
            .expect("the built-in project configuration is valid JSON");
    value
        .as_object_mut()
        .expect("the built-in project configuration is an object")
        .insert("database".to_string(), database);
    render_project_config_json(&value)
}

pub fn merge_project_config_json(existing: &str, project_name: &str) -> Result<String, String> {
    let mut value: serde_json::Value =
        serde_json::from_str(existing).map_err(|error| format!("invalid tyrus.json: {error}"))?;
    if let Some(object) = value.as_object_mut() {
        object.remove("dependencies");
    }
    let defaults: serde_json::Value =
        serde_json::from_str(&default_project_config_json(project_name)).unwrap();
    merge_missing_json(&mut value, &defaults);
    Ok(render_project_config_json(&value))
}

fn merge_missing_json(target: &mut serde_json::Value, defaults: &serde_json::Value) {
    let (Some(target), Some(defaults)) = (target.as_object_mut(), defaults.as_object()) else {
        return;
    };
    for (key, default_value) in defaults {
        match target.get_mut(key) {
            Some(existing_value) if existing_value.is_object() && default_value.is_object() => {
                merge_missing_json(existing_value, default_value);
            }
            Some(_) => {}
            None => {
                target.insert(key.clone(), default_value.clone());
            }
        }
    }
}

fn render_project_config_json(value: &serde_json::Value) -> String {
    let project = value.get("project").and_then(|value| value.as_object());
    let scripts = value.get("scripts").and_then(|value| value.as_object());
    let security = value.get("security").and_then(|value| value.as_object());

    let project_name = project
        .and_then(|project| project.get("name"))
        .and_then(|value| value.as_str())
        .unwrap_or("tyrus-app");
    let version = project
        .and_then(|project| project.get("version"))
        .and_then(|value| value.as_str())
        .unwrap_or("0.1.0");
    let authors = project
        .and_then(|project| project.get("authors"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let description = project
        .and_then(|project| project.get("description"))
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let target = project
        .and_then(|project| project.get("target"))
        .and_then(|value| value.as_str());
    let fmt = scripts
        .and_then(|scripts| scripts.get("fmt"))
        .and_then(|value| value.as_str())
        .unwrap_or("tyrus fmt");
    let lint = scripts
        .and_then(|scripts| scripts.get("lint"))
        .and_then(|value| value.as_str())
        .unwrap_or("tyrus lint");
    let test = scripts
        .and_then(|scripts| scripts.get("test"))
        .and_then(|value| value.as_str())
        .unwrap_or("tyrus test");
    let env_file = security
        .and_then(|security| security.get("envFile"))
        .and_then(|value| value.as_str())
        .unwrap_or(".env");
    let env_sealed = security
        .and_then(|security| security.get("envSealed"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let imports = value
        .get("imports")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({ "#": "./src" }));

    let mut rendered = String::new();
    rendered.push_str("{\n");
    rendered.push_str("  \"project\": {\n");
    rendered.push_str(&format!("    \"name\": {},\n", json_string(project_name)));
    rendered.push_str(&format!("    \"version\": {},\n", json_string(version)));
    rendered.push_str(&format!("    \"authors\": {},\n", compact_json(&authors)));
    rendered.push_str(&format!(
        "    \"description\": {}",
        json_string(description)
    ));
    if let Some(target) = target {
        rendered.push_str(&format!(",\n    \"target\": {}", json_string(target)));
    }
    rendered.push_str("\n  },\n");
    rendered.push_str("  \"scripts\": {\n");
    let mut rendered_scripts = vec![
        ("fmt".to_string(), json_string(fmt)),
        ("lint".to_string(), json_string(lint)),
        ("test".to_string(), json_string(test)),
    ];
    for (name, script) in scripts.into_iter().flat_map(|scripts| scripts.iter()) {
        if !matches!(name.as_str(), "fmt" | "lint" | "test") {
            rendered_scripts.push((name.clone(), compact_json(script)));
        }
    }
    for (index, (name, script)) in rendered_scripts.iter().enumerate() {
        rendered.push_str(&format!(
            "    {}: {}{}\n",
            json_string(name),
            script,
            if index + 1 < rendered_scripts.len() {
                ","
            } else {
                ""
            }
        ));
    }
    rendered.push_str("  },\n");
    rendered.push_str("  \"security\": {\n");
    rendered.push_str(&format!("    \"envFile\": {},\n", json_string(env_file)));
    rendered.push_str(&format!("    \"envSealed\": {env_sealed}\n"));
    rendered.push_str("  },\n");
    rendered.push_str("  \"imports\": ");
    rendered.push_str(&indented_json(&imports, 2));
    rendered.push_str(",\n");
    let extras = value
        .as_object()
        .map(|object| {
            object
                .iter()
                .filter(|(key, _)| {
                    !matches!(
                        key.as_str(),
                        "project" | "scripts" | "security" | "imports" | "dependencies"
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if extras.is_empty() {
        rendered.pop();
        rendered.pop();
    } else {
        for (index, (key, value)) in extras.iter().enumerate() {
            rendered.push_str(&format!(
                "  {}: {}",
                json_string(key),
                indented_json(value, 2)
            ));
            if index + 1 < extras.len() {
                rendered.push(',');
            }
            rendered.push('\n');
        }
    }
    rendered.push_str("}\n");
    rendered
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).unwrap()
}

fn compact_json(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap()
}

fn indented_json(value: &serde_json::Value, spaces: usize) -> String {
    let json = serde_json::to_string_pretty(value).unwrap();
    let indent = " ".repeat(spaces);
    json.replace('\n', &format!("\n{indent}"))
}

pub fn is_entry_module(file: &Path) -> bool {
    let mut directory = file.parent();
    while let Some(current) = directory {
        if current.join("tyrus.json").is_file() {
            return file == current.join("src").join("main.tyr");
        }
        directory = current.parent();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_sealed_defaults_to_false_when_missing() {
        let config: ProjectConfig = serde_json::from_str(
            r##"{
                "project": { "name": "app", "version": "0.1.0" },
                "security": { "envFile": ".env" }
            }"##,
        )
        .unwrap();
        assert!(!config.security.unwrap().env_sealed);
    }

    #[test]
    fn rejects_the_removed_build_entry_property() {
        let error = serde_json::from_str::<ProjectConfig>(
            r##"{
                "project": { "name": "app", "version": "0.1.0" },
                "build": { "entry": "main.tyr" }
            }"##,
        )
        .expect_err("build.entry must no longer be accepted");
        assert!(error.to_string().contains("unknown field `entry`"));
    }

    #[test]
    fn ignores_the_removed_project_manifest_name() {
        let root = std::env::temp_dir().join(format!(
            "tyrus-removed-manifest-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let removed_name = format!("{}.json", ["rs", "ts"].concat());
        fs::write(
            root.join(removed_name),
            default_project_config_json("removed-manifest"),
        )
        .unwrap();

        let error = load_project(&root).expect_err("the removed manifest must be ignored");
        assert!(error.contains("tyrus.json not found"), "got: {error}");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resolves_directory_import_aliases_inside_the_project() {
        let root = std::env::temp_dir().join(format!(
            "tyrus-import-aliases-project-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src/utils")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            r##"{
                "imports": {
                    "#": "./src",
                    "#theme": "./src/theme",
                    "#utils": "./src/utils"
                }
            }"##,
        )
        .unwrap();
        fs::create_dir_all(root.join("src/theme")).unwrap();
        fs::write(
            root.join("src/theme/index.tyr"),
            "export const color = `blue`;",
        )
        .unwrap();
        fs::write(
            root.join("src/utils/strings.tyr"),
            "export const upper = `UP`;",
        )
        .unwrap();

        let root = root.canonicalize().unwrap();
        let imports = ImportMap::load(&root).unwrap();
        assert_eq!(
            imports.resolve(&root, "#theme").unwrap(),
            Some(root.join("src/theme/index.tyr"))
        );
        assert_eq!(
            imports.resolve(&root, "#utils/strings").unwrap(),
            Some(root.join("src/utils/strings.tyr"))
        );
        assert_eq!(
            imports.resolve(&root, "#/utils/strings").unwrap(),
            Some(root.join("src/utils/strings.tyr"))
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_import_aliases_that_can_escape_the_project() {
        let error = validate_import_alias("#src", "./src/../outside")
            .expect_err("parent traversal must be rejected");
        assert!(error.contains("escapes the project root"));

        let error = validate_alias_capture("#src/../../secret", "../../secret")
            .expect_err("a captured parent path must be rejected");
        assert!(error.contains("invalid subpath"));
    }

    #[test]
    fn validates_alias_syntax_and_destination_wildcards() {
        for (key, destination, expected) in [
            ("utils/*", "./src/*", "must start with '#'"),
            ("#utils/*", "./src/utils", "must not contain `*`"),
            ("#utils", "./src/*", "must not use a destination wildcard"),
            ("#utils/**", "./src/*", "must not contain `*`"),
            ("#utils", ".\\src\\utils", "must use '/'"),
            ("#config", "./src/config.tyr", "must target a directory"),
        ] {
            let error = validate_import_alias(key, destination)
                .expect_err("the invalid alias configuration must be rejected");
            assert!(error.contains(expected), "got: {error}");
        }
    }

    #[test]
    fn missing_alias_reports_its_original_specifier() {
        let root = std::env::temp_dir().join(format!(
            "tyrus-missing-import-alias-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("tyrus.json"), "{}\n").unwrap();
        fs::write(root.join("src/main.tyr"), "export const value: i32 = 1;").unwrap();

        let error = resolve_import_path(&root.join("src/main.tyr"), "#missing/value")
            .expect_err("an undefined alias must not fall through to another resolver");
        assert!(error.contains("`#missing/value`"), "got: {error}");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn relative_imports_remain_independent_from_import_aliases() {
        let root = std::env::temp_dir().join(format!(
            "tyrus-relative-import-with-aliases-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src/nested")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            r##"{ "imports": { "#": "./src" } }"##,
        )
        .unwrap();
        fs::write(root.join("src/shared.tyr"), "export const value: i32 = 1;").unwrap();

        let resolved = resolve_import_path(&root.join("src/nested/main.tyr"), "../shared")
            .expect("relative imports must not consult the alias map");
        let expected = root.join("src/shared.tyr").canonicalize().unwrap();
        assert_eq!(resolved, Some(expected));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_duplicate_aliases_and_accepts_a_manifest_without_imports() {
        let root = std::env::temp_dir().join(format!(
            "tyrus-import-alias-manifest-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        fs::write(root.join("tyrus.json"), "{}\n").unwrap();
        assert!(ImportMap::load(&root).unwrap().aliases.is_empty());

        fs::write(
            root.join("tyrus.json"),
            r##"{ "imports": { "#same": "./src/a", "#same": "./src/b" } }"##,
        )
        .unwrap();
        let error = ImportMap::load(&root).expect_err("duplicate aliases must be rejected");
        assert!(
            error.contains("duplicate import alias `#same`"),
            "got: {error}"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_init_adds_missing_defaults_without_overwriting_existing_values() {
        let merged = merge_project_config_json(
            r#"{
                "project": { "name": "app", "version": "1.2.3" },
                "scripts": { "example": "tyrus run example.tyr" },
                "security": { "envSealed": true },
                "dependencies": { "legacy": "0.1.0" }
            }"#,
            "fallback",
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&merged).unwrap();

        assert_eq!(value["project"]["name"], "app");
        assert_eq!(value["project"]["version"], "1.2.3");
        assert_eq!(value["project"]["authors"], serde_json::json!([]));
        assert_eq!(value["scripts"]["example"], "tyrus run example.tyr");
        assert_eq!(value["scripts"]["lint"], "tyrus lint");
        assert_eq!(value["scripts"]["test"], "tyrus test");
        assert_eq!(value["security"]["envFile"], ".env");
        assert_eq!(value["security"]["envSealed"], true);
        assert_eq!(value["imports"]["#"], "./src");
        assert!(value.get("dependencies").is_none());
    }

    #[test]
    fn project_config_json_uses_boilerplate_order_without_build() {
        let config = project_config_json("app", Some("cli"));
        let project = config.find("\"project\"").unwrap();
        let scripts = config.find("\"scripts\"").unwrap();
        let security = config.find("\"security\"").unwrap();
        let imports = config.find("\"imports\"").unwrap();

        assert!(project < scripts);
        assert!(scripts < security);
        assert!(security < imports);
        assert!(!config.contains("\"build\""));
        assert!(!config.contains("\"dependencies\""));
        assert!(config.contains("\"lint\": \"tyrus lint\""));
        assert!(config.contains("\"test\": \"tyrus test\""));
        assert!(config.contains("\"imports\": {\n    \"#\": \"./src\"\n  }"));
        assert!(config.contains("\"target\": \"cli\""));
    }

    #[test]
    fn database_project_config_uses_pretty_imports_without_dependencies() {
        let config = project_config_json_with_database(
            "api-sqlite",
            Some("api"),
            Some(serde_json::json!({ "backend": "sqlite", "path": "./data/app.db" })),
        );
        let value: serde_json::Value = serde_json::from_str(&config).unwrap();

        assert_eq!(value["database"]["backend"], "sqlite");
        assert_eq!(value["imports"]["#"], "./src");
        assert!(value.get("dependencies").is_none());
        assert!(config.contains("\"imports\": {\n    \"#\": \"./src\"\n  }"));
    }

    #[test]
    fn a_module_path_resolves_through_a_dotted_name() {
        let root = std::env::temp_dir().join(format!(
            "tyrus-resolve-dotted-{}-{}",
            std::process::id(),
            line!()
        ));
        let src = root.join("src");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("helpers.test.tyr"), "").unwrap();
        fs::write(src.join("plain.tyr"), "").unwrap();
        let document = src.join("main.tyr");
        let helper = src.join("helpers.test.tyr").canonicalize().unwrap();

        assert_eq!(
            resolve_import_path(&document, "./helpers.test").unwrap(),
            Some(helper.clone())
        );
        assert_eq!(
            resolve_import_path(&document, "./helpers.test.tyr").unwrap(),
            Some(helper)
        );
        assert_eq!(
            resolve_import_path(&document, "./plain").unwrap(),
            Some(src.join("plain.tyr").canonicalize().unwrap())
        );
        assert!(resolve_import_path(&document, "./missing.test").is_err());

        fs::remove_dir_all(&root).unwrap();
    }
}
