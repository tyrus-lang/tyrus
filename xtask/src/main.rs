use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn editors_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../editors")
}

fn docs_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs")
}

fn workspace_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn collect_rust_files(dir: &std::path::Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in
        fs::read_dir(dir).map_err(|error| format!("could not read {}: {error}", dir.display()))?
    {
        let entry = entry.map_err(|error| format!("could not read directory entry: {error}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn public_error_code_literals(source: &str) -> BTreeSet<String> {
    let bytes = source.as_bytes();
    let mut codes = BTreeSet::new();
    for index in 0..bytes.len().saturating_sub(4) {
        if bytes[index] == b'E' && bytes[index + 1..index + 5].iter().all(u8::is_ascii_digit) {
            codes.insert(source[index..index + 5].to_string());
        }
    }
    codes
}

fn kebab_diagnostic_code_literals(source: &str) -> BTreeSet<String> {
    const EMITTERS: &[&str] = &[
        "self.error(",
        "self.warning(",
        "self.error_at(",
        "self.warning_at(",
        "self.error_at_with_help(",
        "self.warning_at_with_help(",
        "self.error_on_declaration(",
        "self.error_at_with_subject(",
        "self.error_with_help(",
        "code: ",
        "diagnostic.code == ",
    ];
    let mut codes = BTreeSet::new();
    for emitter in EMITTERS {
        let mut rest = source;
        while let Some(index) = rest.find(emitter) {
            rest = &rest[index + emitter.len()..];
            let argument = rest.trim_start();
            let Some(literal) = argument.strip_prefix('"') else {
                continue;
            };
            let Some(end) = literal.find('"') else {
                continue;
            };
            let literal = &literal[..end];
            if is_kebab_code(literal) {
                codes.insert(literal.to_string());
            }
        }
    }
    let mut rest = source;
    while let Some(index) = rest.find("let code = ") {
        rest = &rest[index + "let code = ".len()..];
        let initializer = &rest[..rest.find(';').unwrap_or(rest.len())];
        for literal in initializer.split('"').skip(1).step_by(2) {
            if is_kebab_code(literal) {
                codes.insert(literal.to_string());
            }
        }
    }
    codes
}

fn is_kebab_code(literal: &str) -> bool {
    literal.contains('-')
        && literal
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn verify_no_kebab_diagnostic_codes() -> Result<(), String> {
    let root = workspace_dir();
    let source_roots = [
        root.join("src"),
        root.join("crates/tyrus-core/src"),
        root.join("crates/tyrus-lsp/src"),
    ];
    let mut files = Vec::new();
    for source_root in source_roots {
        collect_rust_files(&source_root, &mut files)?;
    }
    let mut offenders = BTreeSet::new();
    for path in files {
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        for code in kebab_diagnostic_code_literals(&source) {
            offenders.insert(format!("{} ({})", code, path.display()));
        }
    }
    if offenders.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "diagnostics must use a registered E#### code, not a kebab-case name: {}",
            offenders.into_iter().collect::<Vec<_>>().join(", ")
        ))
    }
}

fn verify_public_diagnostic_codes() -> Result<(), String> {
    let root = workspace_dir();
    let source_roots = [
        root.join("src"),
        root.join("crates/tyrus-core/src"),
        root.join("crates/tyrus-lint/src"),
        root.join("crates/tyrus-lsp/src"),
    ];
    let mut files = Vec::new();
    for source_root in source_roots {
        collect_rust_files(&source_root, &mut files)?;
    }

    let diagnostics_dir = root.join("crates/tyrus-core/src/diagnostics");
    let mut unregistered = BTreeSet::new();
    for path in files {
        if path.starts_with(&diagnostics_dir) {
            continue;
        }
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        for code in public_error_code_literals(&source) {
            if tyrus_core::diagnostics::find(&code).is_none() {
                unregistered.insert(code);
            }
        }
    }

    if unregistered.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "public E#### diagnostics missing from crates/tyrus-core/src/diagnostics/codes.rs: {}",
            unregistered.into_iter().collect::<Vec<_>>().join(", ")
        ))
    }
}

fn render_error_codes() -> String {
    let mut output = String::from(
        "<!-- GENERATED from crates/tyrus-core/src/diagnostics/codes.rs by `xtask` - do not edit. -->\n\n| Code | Title | Category |\n| --- | --- | --- |\n",
    );
    for diagnostic in tyrus_core::diagnostics::codes() {
        output.push_str(&format!(
            "| `{}` | {} | {} |\n",
            diagnostic.code,
            diagnostic.title,
            diagnostic.category.label(),
        ));
    }
    output
}

fn reserved_keyword_match() -> String {
    let alternation = tyrus_core::keywords::RESERVED_FUTURE.join("|");
    format!(r"(?<!\\.)\\b(?:{alternation})\\b")
}

fn render_grammar(content: &str) -> Result<String, String> {
    let anchor = r#""name": "keyword.other.reserved.tyr", "match": ""#;
    let anchor_pos = content
        .find(anchor)
        .ok_or("reserved keyword pattern not found in the grammar")?;
    let match_start = anchor_pos + anchor.len();
    let match_len = content[match_start..]
        .find('"')
        .ok_or("unterminated `match` string in the grammar")?;
    let match_end = match_start + match_len;
    let with_reserved_match = format!(
        "{}{}{}",
        &content[..match_start],
        reserved_keyword_match(),
        &content[match_end..]
    );

    use tyrus_core::keywords::{ACTIVE_KEYWORDS, LITERAL_KEYWORDS, PRIMITIVE_TYPES};
    let with_keyword_fallback = render_fallback_pattern(
        &with_reserved_match,
        FallbackPattern {
            generated_name: "keyword.other.tyr",
            insert_after_anchor: r#""name": "keyword.other.reserved.tyr", "match": ""#,
            words: ACTIVE_KEYWORDS
                .iter()
                .filter(|word| !PRIMITIVE_TYPES.contains(word) && !LITERAL_KEYWORDS.contains(word))
                .copied()
                .collect(),
            comment: "GENERATED from ACTIVE_KEYWORDS in crates/tyrus-core/src/keywords.rs by `cargo run -p xtask`, catches any active keyword not already given a specific scope above, so a newly active keyword is never left unstyled.",
        },
    )?;
    render_fallback_pattern(
        &with_keyword_fallback,
        FallbackPattern {
            generated_name: "storage.type.tyr",
            insert_after_anchor: r#""name": "support.type.tyr", "match": ""#,
            words: PRIMITIVE_TYPES.to_vec(),
            comment: "GENERATED from PRIMITIVE_TYPES in crates/tyrus-core/src/keywords.rs by `cargo run -p xtask`, catches any primitive type not already given a specific scope above, so a newly added narrow type (e.g. i16/u8) is never left unstyled.",
        },
    )
}

fn words_already_scoped(content: &str) -> std::collections::HashSet<String> {
    let mut words = std::collections::HashSet::new();
    let anchor = "\"match\": \"";
    let mut cursor = 0;
    while let Some(relative) = content[cursor..].find(anchor) {
        let start = cursor + relative + anchor.len();
        let bytes = content.as_bytes();
        let mut end = start;
        while end < bytes.len() && !(bytes[end] == b'"' && bytes[end - 1] != b'\\') {
            end += 1;
        }
        let mut word = String::new();
        for ch in content[start..end].chars() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                word.push(ch);
            } else if !word.is_empty() {
                words.insert(std::mem::take(&mut word));
            }
        }
        if !word.is_empty() {
            words.insert(word);
        }
        cursor = end;
    }
    words
}

struct FallbackPattern {
    generated_name: &'static str,
    insert_after_anchor: &'static str,
    words: Vec<&'static str>,
    comment: &'static str,
}

fn strip_generated_pattern(content: &str, comment: &str) -> String {
    let anchor = format!("\"comment\": \"{comment}\"");
    let Some(comment_pos) = content.find(&anchor) else {
        return content.to_string();
    };
    let obj_start = content[..comment_pos].rfind('{').unwrap_or(comment_pos);
    let comma_start = content[..obj_start].rfind(',').unwrap_or(obj_start);
    let obj_end = content[comment_pos..]
        .find('}')
        .map_or(content.len(), |offset| comment_pos + offset + 1);
    format!("{}{}", &content[..comma_start], &content[obj_end..])
}

fn render_fallback_pattern(content: &str, spec: FallbackPattern) -> Result<String, String> {
    let clean = strip_generated_pattern(content, spec.comment);
    let scoped = words_already_scoped(&clean);
    let fallback: Vec<&str> = spec
        .words
        .iter()
        .filter(|word| !scoped.contains(**word))
        .copied()
        .collect();
    if fallback.is_empty() {
        return Ok(clean);
    }

    let anchor_pos = clean.find(spec.insert_after_anchor).ok_or_else(|| {
        format!(
            "fallback anchor not found in the grammar: {}",
            spec.insert_after_anchor
        )
    })?;
    let after_anchor = anchor_pos + spec.insert_after_anchor.len();
    let insertion_point = clean[after_anchor..]
        .find('}')
        .map(|offset| after_anchor + offset + 1)
        .ok_or("unterminated anchor pattern object")?;

    let alternation = fallback.join("|");
    let comment = spec.comment;
    let generated_name = spec.generated_name;
    let pattern = format!(
        ",\n        {{ \"comment\": \"{comment}\", \"name\": \"{generated_name}\", \"match\": \"\\\\b(?:{alternation})\\\\b\" }}"
    );
    Ok(format!(
        "{}{}{}",
        &clean[..insertion_point],
        pattern,
        &clean[insertion_point..]
    ))
}

fn render_neovim_syntax() -> String {
    use tyrus_core::keywords::{
        ACTIVE_KEYWORDS, LITERAL_KEYWORDS, PRIMITIVE_TYPES, RESERVED_FUTURE,
    };
    let control = ACTIVE_KEYWORDS
        .iter()
        .filter(|word| !PRIMITIVE_TYPES.contains(word) && !LITERAL_KEYWORDS.contains(word))
        .copied()
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        r#"" GENERATED by `cargo run -p xtask` from tyrus_core::keywords -- do not edit.
" Base highlighting for tyrus. Parameters, reserved-word errors and richer
" semantics come from the tyrus language server via LSP semantic tokens.
if exists("b:current_syntax")
  finish
endif

syntax keyword tyrusKeyword {control}
syntax keyword tyrusType {types}
syntax keyword tyrusBoolean {literals}
syntax keyword tyrusReserved {reserved}

syntax region tyrusString start=+"+ skip=+\\.+ end=+"+
syntax region tyrusString start=+'+ skip=+\\.+ end=+'+
syntax region tyrusTemplate start=+`+ skip=+\\.+ end=+`+
syntax match tyrusNumber /\v<\d+(\.\d+)?>/
syntax match tyrusLineComment +//.*$+
syntax region tyrusBlockComment start=+/\*+ end=+\*/+

highlight default link tyrusKeyword Keyword
highlight default link tyrusType Type
highlight default link tyrusBoolean Boolean
highlight default link tyrusReserved Keyword
highlight default link tyrusString String
highlight default link tyrusTemplate String
highlight default link tyrusNumber Number
highlight default link tyrusLineComment Comment
highlight default link tyrusBlockComment Comment

let b:current_syntax = "tyrus"
"#,
        control = control,
        types = PRIMITIVE_TYPES.join(" "),
        literals = LITERAL_KEYWORDS.join(" "),
        reserved = RESERVED_FUTURE.join(" "),
    )
}

fn control_keywords() -> Vec<&'static str> {
    use tyrus_core::keywords::{ACTIVE_KEYWORDS, LITERAL_KEYWORDS, PRIMITIVE_TYPES};
    ACTIVE_KEYWORDS
        .iter()
        .filter(|word| !PRIMITIVE_TYPES.contains(word) && !LITERAL_KEYWORDS.contains(word))
        .copied()
        .collect()
}

fn render_treesitter_highlights() -> String {
    let keywords = control_keywords()
        .iter()
        .map(|word| format!("\"{word}\""))
        .collect::<Vec<_>>()
        .join("\n  ");
    let reserved = tyrus_core::keywords::RESERVED_FUTURE
        .iter()
        .map(|word| format!("\"{word}\""))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        r#"; GENERATED by `cargo run -p xtask` from tyrus_core::keywords -- do not edit.

(comment) @comment

(string) @string
(template_string) @string
(regex) @string.regexp
(number) @number
[(true) (false)] @constant.builtin.boolean
(null) @constant.builtin

(identifier) @variable

(function_declaration name: (identifier) @function)
(call_expression function: (identifier) @function)
(parameter name: (identifier) @variable.parameter)
(pair key: (identifier) @property)
(property_signature name: (identifier) @property)
(member_expression property: (identifier) @property)
(decorator name: (identifier) @attribute)

(primitive_type) @type.builtin
(type_identifier) @type

((identifier) @keyword
  (#any-of? @keyword {reserved}))

[
  {keywords}
] @keyword
"#,
    )
}

fn render_intellij_grammar() -> Result<String, String> {
    let vscode_grammar_path = editors_dir().join("vscode/syntaxes/tyrus.tmLanguage.json");
    let content = fs::read_to_string(&vscode_grammar_path)
        .map_err(|error| format!("could not read {}: {error}", vscode_grammar_path.display()))?;
    render_grammar(&content)
}

fn render_kate_syntax() -> String {
    use tyrus_core::keywords::{ACTIVE_KEYWORDS, PRIMITIVE_TYPES, RESERVED_FUTURE};
    let mut all_keywords: Vec<&str> = ACTIVE_KEYWORDS
        .iter()
        .chain(RESERVED_FUTURE.iter())
        .copied()
        .collect();
    all_keywords.sort_unstable();
    all_keywords.dedup();
    let keyword_items: String = all_keywords
        .iter()
        .map(|w| format!("    <item> {w} </item>"))
        .collect::<Vec<_>>()
        .join("\n");
    let primitive_suffixes = PRIMITIVE_TYPES.join(" ");
    let single_quote = "'";
    let double_quote = "&quot;";
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE language SYSTEM "https://raw.githubusercontent.com/KDE/syntax-highlighting/master/data/syntax.dtd">
<language name="Tyrus" version="1.0" kateversion="5.79" section="Sources" extensions="*.tyr" case-insensitive="false" highlightSpaces="\S" highlightTrailingSpaces="true" highlightAnchorDelimiter="">

  <keywords>
{keyword_items}
  </keywords>

  <context name="Detect" attribute="Normal" lineEndContext="#Stay">
    <DetectChar char="{single_quote}" context="CharLiteral"/>
    <DetectChar char="{double_quote}" context="StringLiteral"/>
    <DetectChar char="/" context="PotentialCommentOrRegex"/>
    <Detect2Chars char="#" char2="!" context="Shebang" firstNonSpace="true"/>
    <StringDetect String="//" context="SingleLineComment" toEndOfLine="true"/>
    <StringDetect String="/*" context="MultiLineComment"/>
    <keywordString String="@validate @min @max @pattern @length @range @custom" context="Decorator" attribute="Decorator"/>
    <DetectIdentifier context="Identifier"/>
    <DetectChar char="0" context="Number" column="0"/>
    <Int regex="[0-9]" context="Number" attribute="Float"/>
    <DetectChar char="%" context="Topic" attribute="Special"/>
  </context>

  <context name="Identifier" attribute="Normal" lineEndContext="#Pop">
    <DetectIdentifier/>
    <keyword attribute="Keyword" context="#Pop" weak="true"/>
    <DetectChar char="(" context="#Pop" attribute="Normal"/>
    <DetectChar char=":" context="#Pop" attribute="Normal"/>
    <DetectChar char="=" context="#Pop" attribute="Normal"/>
  </context>

  <context name="SingleLineComment" attribute="Comment" lineEndContext="#Pop" transparent="true">
    <DetectChar char="\n" context="#Pop"/>
  </context>

  <context name="MultiLineComment" attribute="Comment" lineEndContext="#Stay">
    <DetectChar char="*" context="MultiLineCommentEnd"/>
  </context>

  <context name="MultiLineCommentEnd" attribute="Comment" lineEndContext="#Stay">
    <DetectChar char="/" context="#Pop"/>
    <DetectChar char="*" context="MultiLineCommentEnd"/>
    <DetectNone char="" context="#Pop"/>
  </context>

  <context name="PotentialCommentOrRegex" attribute="Normal" lineEndContext="#Pop" fallthrough="true" fallthroughContext="Regex">
    <DetectChar char="/" context="SingleLineComment"/>
    <DetectChar char="*" context="MultiLineComment"/>
  </context>

  <context name="Regex" attribute="String" lineEndContext="#Pop">
    <HlCStringChar char="\""/>
    <DetectChar char="/" context="#Pop"/>
  </context>

  <context name="StringLiteral" attribute="String" lineEndContext="#Pop">
    <HlCStringChar char="\""/>
  </context>

  <context name="CharLiteral" attribute="Char" lineEndContext="#Pop">
    <HlCStringChar char="\""/>
    <DetectChar char="{single_quote}" context="#Pop"/>
  </context>

  <context name="Number" attribute="Float" lineEndContext="#Pop">
    <Int regex="[0-9]" attribute="Float"/>
    <DetectChar char="." context="NumberDot"/>
    <StringDetect String="x" context="HexNumber" attribute="BaseN"/>
    <StringDetect String="b" context="BinNumber" attribute="BaseN"/>
    <StringDetect String="o" context="OctNumber" attribute="BaseN"/>
    <DetectChar char="e" context="Exponent" attribute="Float"/>
    <DetectChar char="E" context="Exponent" attribute="Float"/>
    <DetectChar char="_" context="#Stay" attribute="Float"/>
    <keywordString String="{primitive_suffixes}" context="#Pop" attribute="Type" weak="true"/>
  </context>

  <context name="NumberDot" attribute="Float" lineEndContext="#Pop">
    <Int regex="[0-9]" attribute="Float"/>
    <DetectChar char="e" context="Exponent" attribute="Float"/>
    <DetectChar char="E" context="Exponent" attribute="Float"/>
    <keywordString String="f32 f64 f128" context="#Pop" attribute="Type" weak="true"/>
  </context>

  <context name="HexNumber" attribute="BaseN" lineEndContext="#Pop">
    <DetectChar regex="[0-9a-fA-F]" context="#Stay" attribute="BaseN"/>
    <DetectChar char="_" context="#Stay" attribute="BaseN"/>
    <keywordString String="u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64 f128" context="#Pop" attribute="Type" weak="true"/>
  </context>

  <context name="BinNumber" attribute="BaseN" lineEndContext="#Pop">
    <DetectChar regex="[01]" context="#Stay" attribute="BaseN"/>
    <DetectChar char="_" context="#Stay" attribute="BaseN"/>
    <keywordString String="u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize" context="#Pop" attribute="Type" weak="true"/>
  </context>

  <context name="OctNumber" attribute="BaseN" lineEndContext="#Pop">
    <DetectChar regex="[0-7]" context="#Stay" attribute="BaseN"/>
    <DetectChar char="_" context="#Stay" attribute="BaseN"/>
    <keywordString String="u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize" context="#Pop" attribute="Type" weak="true"/>
  </context>

  <context name="Exponent" attribute="Float" lineEndContext="#Pop">
    <DetectChar regex="[0-9]" context="#Stay" attribute="Float"/>
    <DetectChar char="+" context="#Stay" attribute="Float"/>
    <DetectChar char="-" context="#Stay" attribute="Float"/>
    <DetectChar char="_" context="#Stay" attribute="Float"/>
    <keywordString String="f32 f64 f128" context="#Pop" attribute="Type" weak="true"/>
  </context>

  <context name="Shebang" attribute="Comment" lineEndContext="#Pop">
    <DetectChar char="\n" context="#Pop"/>
  </context>

  <context name="Decorator" attribute="Decorator" lineEndContext="#Pop">
    <DetectIdentifier/>
    <DetectChar char="(" context="#Pop"/>
  </context>

  <context name="Topic" attribute="Special" lineEndContext="#Pop"/>

</language>
"##
    )
}

fn artifacts() -> Result<Vec<(PathBuf, String)>, String> {
    verify_rust_file_line_limits()?;
    verify_public_diagnostic_codes()?;
    verify_no_kebab_diagnostic_codes()?;
    let grammar_path = editors_dir().join("vscode/syntaxes/tyrus.tmLanguage.json");
    let grammar = fs::read_to_string(&grammar_path)
        .map_err(|error| format!("could not read {}: {error}", grammar_path.display()))?;
    Ok(vec![
        (grammar_path.clone(), render_grammar(&grammar)?),
        (
            editors_dir().join("neovim/syntax/tyrus.vim"),
            render_neovim_syntax(),
        ),
        (
            editors_dir().join("tree-sitter-tyrus/queries/highlights.scm"),
            render_treesitter_highlights(),
        ),
        (
            editors_dir().join("intellij/tyrus.tmLanguage"),
            render_intellij_grammar()?,
        ),
        (editors_dir().join("kate/tyrus.xml"), render_kate_syntax()),
        (
            docs_dir().join(".fragments/language/errors/error-codes.md"),
            render_error_codes(),
        ),
    ])
}

const RUST_FILE_LINE_LIMIT: usize = 3_000;

const RUST_FILE_LINE_EXCEPTIONS: &[(&str, &str)] = &[
    (
        "crates/tyrus-codegen/src/core/core.rs",
        "async codegen state machine",
    ),
    (
        "crates/tyrus-codegen/src/expressions/expressions.rs",
        "expression dispatcher",
    ),
    (
        "crates/tyrus-codegen/src/methods/methods.rs",
        "method dispatcher",
    ),
    (
        "crates/tyrus-codegen/tests/unit/codegen.rs",
        "flat codegen test suite",
    ),
    (
        "crates/tyrus-core/src/analysis/analysis.rs",
        "analysis pipeline",
    ),
    (
        "crates/tyrus-core/src/ctfe/ctfe.rs",
        "compile-time evaluation pipeline",
    ),
    (
        "crates/tyrus-core/src/type_checker/type_checker.rs",
        "type checker core and dispatch",
    ),
    (
        "crates/tyrus-core/src/type_checker/support/support.rs",
        "type checker shared semantic support",
    ),
    (
        "crates/tyrus-core/tests/semantic_success.rs",
        "flat semantic test suite",
    ),
    (
        "crates/tyrus-core/tests/unit/analysis.rs",
        "flat analysis test suite",
    ),
    (
        "crates/tyrus-core/tests/unit/parser.rs",
        "flat parser test suite",
    ),
    (
        "crates/tyrus-core/tests/unit/type_checker.rs",
        "flat type checker test suite",
    ),
    (
        "crates/tyrus-lsp/src/completion/completion.rs",
        "completion dispatcher",
    ),
    (
        "crates/tyrus-lsp/src/server/server.rs",
        "LSP request dispatcher",
    ),
    ("crates/tyrus-lsp/tests/unit/lsp.rs", "flat LSP test suite"),
    (
        "crates/tyrus-runtime/src/crypto/crypto.rs",
        "crypto FFI dispatch and contracts",
    ),
    (
        "crates/tyrus-runtime/src/http/http.rs",
        "HTTP runtime dispatch",
    ),
    (
        "crates/tyrus-runtime/src/temporal/temporal.rs",
        "temporal runtime dispatch",
    ),
    (
        "crates/tyrus-runtime/tests/unit/http_runtime.rs",
        "flat HTTP runtime test suite",
    ),
    (
        "crates/tyrus-temporal-data/src/lib.rs",
        "generated temporal data table",
    ),
    ("src/repl/repl.rs", "REPL command dispatcher"),
];

fn verify_rust_file_line_limits() -> Result<(), String> {
    let root = repository_root();
    let mut files = Vec::new();
    for directory in ["crates", "src", "xtask"] {
        collect_rust_files(&root.join(directory), &mut files)?;
    }
    let exceptions = RUST_FILE_LINE_EXCEPTIONS
        .iter()
        .copied()
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut offenders = Vec::new();
    for path in files {
        let relative = path
            .strip_prefix(&root)
            .map_err(|error| format!("could not relativize {}: {error}", path.display()))?
            .to_string_lossy()
            .replace('\\', "/");
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        let lines = source.lines().count();
        if lines > RUST_FILE_LINE_LIMIT && !exceptions.contains_key(relative.as_str()) {
            offenders.push(format!("{relative} ({lines} lines)"));
        }
    }
    if offenders.is_empty() {
        return Ok(());
    }
    offenders.sort();
    Err(format!(
        "Rust files exceed the {RUST_FILE_LINE_LIMIT}-line limit: {}",
        offenders.join(", ")
    ))
}

const EM_DASH_SKIPPED_DIRECTORIES: &[&str] = &[
    "target",
    "node_modules",
    "cache",
    "graphify-out",
    "dist",
    ".git",
    ".claude",
    ".codex",
];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

fn em_dash_occurrences(root: &Path) -> Vec<(PathBuf, usize)> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            if path.is_dir() {
                if !EM_DASH_SKIPPED_DIRECTORIES.contains(&name.to_string_lossy().as_ref()) {
                    pending.push(path);
                }
                continue;
            }
            let Ok(bytes) = fs::read(&path) else {
                continue;
            };
            if bytes.iter().take(8192).any(|byte| *byte == 0) {
                continue;
            }
            let Ok(text) = std::str::from_utf8(&bytes) else {
                continue;
            };
            for (index, line) in text.lines().enumerate() {
                if line.contains('\u{2014}') {
                    found.push((path.clone(), index + 1));
                }
            }
        }
    }
    found.sort();
    found
}

fn main() -> ExitCode {
    let check = std::env::args().any(|arg| arg == "--check");
    let artifacts = match artifacts() {
        Ok(artifacts) => artifacts,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut drifted = false;
    for (path, desired) in artifacts {
        let current = fs::read_to_string(&path).unwrap_or_default();
        if current == desired {
            continue;
        }
        drifted = true;
        if check {
            eprintln!("out of date: {}", path.display());
            continue;
        }
        if let Some(parent) = path.parent()
            && let Err(error) = fs::create_dir_all(parent)
        {
            eprintln!("error: could not create {}: {error}", parent.display());
            return ExitCode::FAILURE;
        }
        if let Err(error) = fs::write(&path, &desired) {
            eprintln!("error: could not write {}: {error}", path.display());
            return ExitCode::FAILURE;
        }
        println!("updated {}", path.display());
    }

    let em_dashes = em_dash_occurrences(&repository_root());
    for (path, line) in &em_dashes {
        eprintln!("U+2014 found: {}:{line}", path.display());
    }
    if !em_dashes.is_empty() {
        eprintln!(
            "replace U+2014 with ordinary punctuation, a colon, parentheses or a separate sentence"
        );
        return ExitCode::FAILURE;
    }

    if check && drifted {
        eprintln!("run `cargo run -p xtask` to regenerate");
        return ExitCode::FAILURE;
    }
    if !drifted {
        println!("all generated artifacts are up to date");
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_kebab_code_chosen_before_it_is_emitted_or_compared_by_name() {
        let source = r#"
            let code = if is_async { "async-return-type-mismatch" } else { "E0308" };
            self.error(code, message, None);
            if diagnostic.code == "old-style-code" {}
            let code = if is_async { "E3296" } else { "E0308" };
        "#;
        let codes = kebab_diagnostic_code_literals(source);
        assert_eq!(
            codes.into_iter().collect::<Vec<_>>(),
            vec!["async-return-type-mismatch", "old-style-code"]
        );
    }

    #[test]
    fn the_em_dash_scan_reports_the_line_and_skips_dependency_folders() {
        let root = std::env::temp_dir().join(format!("xtask-em-dash-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("node_modules")).unwrap();
        fs::create_dir_all(root.join("dist")).unwrap();
        fs::create_dir_all(root.join("docs/.vitepress/cache/deps")).unwrap();
        fs::write(root.join("clean.rs"), "// plain comment\n").unwrap();
        fs::write(root.join("dirty.md"), "first\nsecond \u{2014} third\n").unwrap();
        fs::write(root.join("node_modules/vendor.js"), "\u{2014}\n").unwrap();
        fs::write(root.join("dist/page.html"), "\u{2014}\n").unwrap();
        fs::write(
            root.join("docs/.vitepress/cache/deps/generated.js"),
            "\u{2014}\n",
        )
        .unwrap();
        fs::write(
            root.join("docs/.vitepress/cache/deps/generated.js.map"),
            "\u{2014}\n",
        )
        .unwrap();
        let found = em_dash_occurrences(&root);
        assert_eq!(found, vec![(root.join("dirty.md"), 2)]);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn all_artifacts_are_in_sync_with_the_lexicon() {
        for (path, desired) in artifacts().unwrap() {
            let current = fs::read_to_string(&path).unwrap_or_default();
            assert_eq!(
                current.replace("\r\n", "\n"),
                desired.replace("\r\n", "\n"),
                "{} drifted from tyrus_core::keywords; run `cargo run -p xtask`",
                path.display()
            );
        }
    }

    #[test]
    fn reserved_match_covers_every_reserved_word() {
        let regex = reserved_keyword_match();
        for word in tyrus_core::keywords::RESERVED_FUTURE {
            assert!(
                regex.contains(word),
                "`{word}` missing from generated regex"
            );
        }
    }

    fn keyword_fallback_spec() -> FallbackPattern {
        use tyrus_core::keywords::{ACTIVE_KEYWORDS, LITERAL_KEYWORDS, PRIMITIVE_TYPES};
        FallbackPattern {
            generated_name: "keyword.other.tyr",
            insert_after_anchor: r#""name": "keyword.other.reserved.tyr", "match": ""#,
            words: ACTIVE_KEYWORDS
                .iter()
                .filter(|word| !PRIMITIVE_TYPES.contains(word) && !LITERAL_KEYWORDS.contains(word))
                .copied()
                .collect(),
            comment: "x-keyword-fallback",
        }
    }

    fn type_fallback_spec() -> FallbackPattern {
        use tyrus_core::keywords::PRIMITIVE_TYPES;
        FallbackPattern {
            generated_name: "storage.type.tyr",
            insert_after_anchor: r#""name": "support.type.tyr", "match": ""#,
            words: PRIMITIVE_TYPES.to_vec(),
            comment: "x-type-fallback",
        }
    }

    #[test]
    fn fallback_pattern_catches_an_active_keyword_missing_from_every_hand_written_scope() {
        use tyrus_core::keywords::{ACTIVE_KEYWORDS, LITERAL_KEYWORDS, PRIMITIVE_TYPES};
        let bare_grammar =
            r#"{ "comment": "x", "name": "keyword.other.reserved.tyr", "match": "" } ]"#;
        let rendered = render_fallback_pattern(bare_grammar, keyword_fallback_spec()).unwrap();
        assert!(rendered.contains("\"name\": \"keyword.other.tyr\""));
        for word in ACTIVE_KEYWORDS {
            if PRIMITIVE_TYPES.contains(word) || LITERAL_KEYWORDS.contains(word) {
                continue;
            }
            assert!(
                rendered.contains(word),
                "`{word}` missing from the generated fallback pattern"
            );
        }
    }

    #[test]
    fn fallback_pattern_never_reclaims_a_word_a_hand_written_scope_already_covers() {
        let grammar = r#"{ "name": "keyword.operator.logical.tyr", "match": "\\b(?:and|or|not)\\b" }, { "comment": "x", "name": "keyword.other.reserved.tyr", "match": "" } ]"#;
        let rendered = render_fallback_pattern(grammar, keyword_fallback_spec()).unwrap();
        let anchor = r#""name": "keyword.other.tyr", "match": ""#;
        let start = rendered.find(anchor).unwrap() + anchor.len();
        let end = start + rendered[start..].find('"').unwrap();
        let alternation: Vec<&str> = rendered[start..end]
            .trim_start_matches(r"\\b(?:")
            .trim_end_matches(r"\\b")
            .trim_end_matches(')')
            .split('|')
            .collect();
        for already_scoped in ["and", "or", "not"] {
            assert!(
                !alternation.contains(&already_scoped),
                "`{already_scoped}` should stay claimed by keyword.operator.logical.tyr, not the fallback"
            );
        }
    }

    #[test]
    fn fallback_pattern_regenerates_idempotently() {
        let grammar = r#"{ "comment": "x", "name": "keyword.other.reserved.tyr", "match": "" } ]"#;
        let once = render_fallback_pattern(grammar, keyword_fallback_spec()).unwrap();
        let twice = render_fallback_pattern(&once, keyword_fallback_spec()).unwrap();
        assert_eq!(once, twice);
    }

    #[test]
    fn type_fallback_pattern_catches_a_primitive_type_missing_from_every_hand_written_scope() {
        let bare_grammar = r#"{ "comment": "x", "name": "support.type.tyr", "match": "" } ]"#;
        let rendered = render_fallback_pattern(bare_grammar, type_fallback_spec()).unwrap();
        assert!(rendered.contains("\"name\": \"storage.type.tyr\""));
        for word in ["i8", "i16", "u8", "u16", "char"] {
            assert!(
                rendered.contains(word),
                "`{word}` missing from the generated type fallback pattern"
            );
        }
    }
}
