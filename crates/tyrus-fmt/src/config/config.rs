use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IndentStyle {
    Tab,
    #[default]
    Spaces,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum QuoteStyle {
    Off,
    Single,
    #[default]
    Double,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BraceStyle {
    Off,
    #[default]
    SameLine,
    NextLine,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LiteralExpand {
    Off,
    #[default]
    Simple,
    ExpandAll,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BlankLines {
    Off,
    #[default]
    Collapse,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PipelineChainLayout {
    Off,
    #[default]
    LineByLine,
    Inline,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub array_expand: LiteralExpand,
    #[serde(default)]
    pub blank_lines: BlankLines,
    #[serde(default = "default_true")]
    pub blank_lines_around_blocks: bool,
    #[serde(default = "default_true")]
    pub blank_lines_around_decorations: bool,
    #[serde(default = "default_true")]
    pub brace_spacing: bool,
    #[serde(default)]
    pub brace_style: BraceStyle,
    #[serde(default = "default_true")]
    pub final_newline: bool,
    #[serde(default)]
    pub indent_style: IndentStyle,
    #[serde(default = "default_indent_width")]
    pub indent_width: u8,
    #[serde(default = "default_true")]
    pub imports_sort: bool,
    #[serde(default = "default_line_width")]
    pub line_width: u16,
    #[serde(default)]
    pub object_expand: LiteralExpand,
    #[serde(default)]
    pub pipeline_chain_layout: PipelineChainLayout,
    #[serde(default)]
    pub prefer_arrow_function: bool,
    #[serde(default)]
    pub quote_style: QuoteStyle,
    #[serde(default = "default_true")]
    pub remove_unused_imports: bool,
    #[serde(default = "default_true")]
    pub trailing_comma: bool,
}

fn default_indent_width() -> u8 {
    4
}

fn default_line_width() -> u16 {
    90
}

fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            array_expand: LiteralExpand::Simple,
            blank_lines: BlankLines::Collapse,
            blank_lines_around_blocks: true,
            blank_lines_around_decorations: true,
            brace_spacing: true,
            brace_style: BraceStyle::SameLine,
            final_newline: true,
            indent_style: IndentStyle::Spaces,
            indent_width: 4,
            imports_sort: true,
            line_width: 90,
            object_expand: LiteralExpand::Simple,
            pipeline_chain_layout: PipelineChainLayout::LineByLine,
            prefer_arrow_function: false,
            quote_style: QuoteStyle::Double,
            remove_unused_imports: true,
            trailing_comma: true,
        }
    }
}

impl Config {
    pub fn load(dir: &Path) -> Self {
        Self::try_load(dir).ok().flatten().unwrap_or_default()
    }

    pub fn try_load(dir: &Path) -> Result<Option<Self>, String> {
        let config_path = dir.join("tyrus-fmt.json");
        let content = match fs::read_to_string(&config_path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("could not read tyrus-fmt.json: {error}")),
        };
        serde_json::from_str(&content)
            .map(Some)
            .map_err(|error| format!("invalid tyrus-fmt.json: {error}"))
    }

    pub fn indent_string(&self) -> String {
        match self.indent_style {
            IndentStyle::Tab => "\t".to_string(),
            IndentStyle::Spaces => " ".repeat(self.indent_width as usize),
        }
    }
}

fn to_pretty_json(config: &Config) -> String {
    let mut buffer = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut serializer = serde_json::Serializer::with_formatter(&mut buffer, formatter);
    config
        .serialize(&mut serializer)
        .expect("serializing Config to an in-memory buffer cannot fail");
    let mut out = String::from_utf8(buffer).expect("serde_json always emits valid UTF-8");
    out.push('\n');
    out
}

pub fn default_config_json() -> String {
    to_pretty_json(&Config::default())
}

pub fn merge_config_json(existing: &str) -> Result<String, String> {
    let merged: Config = serde_json::from_str(existing)
        .map_err(|error| format!("invalid tyrus-fmt.json: {error}"))?;
    Ok(to_pretty_json(&merged))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    fn temp_directory() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "tyrus-fmt-config-{}-{}",
            std::process::id(),
            NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    const DEFAULT_JSON: &str = r#"{
    "array_expand": "simple",
    "blank_lines": "collapse",
    "blank_lines_around_blocks": true,
    "blank_lines_around_decorations": true,
    "brace_spacing": true,
    "brace_style": "same-line",
    "final_newline": true,
    "indent_style": "spaces",
    "indent_width": 4,
    "imports_sort": true,
    "line_width": 90,
    "object_expand": "simple",
    "pipeline_chain_layout": "line-by-line",
    "prefer_arrow_function": false,
    "quote_style": "double",
    "remove_unused_imports": true,
    "trailing_comma": true
}
"#;

    #[test]
    fn default_config_json_matches_expected() {
        assert_eq!(default_config_json(), DEFAULT_JSON);
    }

    #[test]
    fn try_load_distinguishes_missing_and_invalid_configuration() {
        let directory = temp_directory();
        assert!(Config::try_load(&directory).unwrap().is_none());
        fs::write(directory.join("tyrus-fmt.json"), "{").unwrap();
        assert!(Config::try_load(&directory).is_err());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn merge_keeps_existing_and_adds_missing() {
        let existing = "{\n    \"line_width\": 100,\n    \"quote_style\": \"single\"\n}\n";
        let merged = merge_config_json(existing).unwrap();
        assert!(merged.contains("\"line_width\": 100"));
        assert!(merged.contains("\"quote_style\": \"single\""));
        assert!(merged.contains("\"brace_style\": \"same-line\""));
        assert!(merged.contains("\"blank_lines_around_decorations\": true"));
        assert_eq!(merged.lines().count(), DEFAULT_JSON.lines().count());
    }

    #[test]
    fn merge_rejects_invalid_config() {
        assert!(merge_config_json("{ not valid json").is_err());
    }

    #[test]
    fn quote_style_accepts_off() {
        let config: Config = serde_json::from_str(r#"{ "quote_style": "off" }"#).unwrap();
        assert!(matches!(config.quote_style, QuoteStyle::Off));
    }

    #[test]
    fn literal_expand_accepts_off() {
        let config: Config =
            serde_json::from_str(r#"{ "array_expand": "off", "object_expand": "off" }"#).unwrap();
        assert!(matches!(config.array_expand, LiteralExpand::Off));
        assert!(matches!(config.object_expand, LiteralExpand::Off));
    }

    #[test]
    fn pipeline_chain_layout_accepts_off() {
        let config: Config = serde_json::from_str(r#"{ "pipeline_chain_layout": "off" }"#).unwrap();
        assert!(matches!(
            config.pipeline_chain_layout,
            PipelineChainLayout::Off
        ));
    }

    #[test]
    fn brace_style_accepts_off() {
        let config: Config = serde_json::from_str(r#"{ "brace_style": "off" }"#).unwrap();
        assert!(matches!(config.brace_style, BraceStyle::Off));
    }

    #[test]
    fn trailing_comma_accepts_boolean_values() {
        let enabled: Config = serde_json::from_str(r#"{ "trailing_comma": true }"#).unwrap();
        let disabled: Config = serde_json::from_str(r#"{ "trailing_comma": false }"#).unwrap();
        assert!(enabled.trailing_comma);
        assert!(!disabled.trailing_comma);
    }
}
