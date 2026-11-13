use crate::analysis::{AnalysisDiagnostic, AnalysisRelatedInformation, DiagnosticSeverity};
use crate::ast::{Expr, ExprKind, Field, FloatLiteral, Type, ValidatorDecorator};
use crate::parser::Parser;
use crate::project::ProjectConfig;
use crate::span::Span;
use crate::validators::{
    ValidatorDiagnostic, ValidatorDiagnosticSeverity, validate_decorator, validate_decorator_set,
    validate_literal_value,
};
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

pub const ENV_TYPE_NAMES: &[(&str, Type)] = &[
    ("string", Type::Str),
    ("boolean", Type::Bool),
    ("bool", Type::Bool),
    ("i32", Type::I32),
    ("i64", Type::I64),
    ("u32", Type::U32),
    ("u64", Type::U64),
    ("f32", Type::F32),
    ("f64", Type::F64),
];

#[derive(Debug, Clone)]
pub struct EnvVarDecl {
    pub name: String,
    pub name_range: Range<usize>,
    pub raw_value: String,
    pub value_range: Range<usize>,
    pub declared_type: Type,
    pub type_message: Option<String>,
    pub type_range: Option<Range<usize>>,
    pub decorators: Vec<ValidatorDecorator>,
    pub decorator_ranges: Vec<Range<usize>>,
    pub interpolatable: bool,
}

#[derive(Debug, Clone)]
pub struct EnvSchema {
    pub env_file: String,
    pub file_path: PathBuf,
    pub source: String,
    pub vars: Vec<EnvVarDecl>,
}

impl EnvSchema {
    pub fn var(&self, name: &str) -> Option<&EnvVarDecl> {
        self.vars.iter().find(|var| var.name == name)
    }

    pub fn object_type(&self) -> Vec<Field> {
        self.vars
            .iter()
            .map(|var| Field {
                name: var.name.clone(),
                ty: var.declared_type.clone(),
                readonly: true,
                optional: false,
                decorators: var.decorators.clone(),
                span: Span::DUMMY,
            })
            .collect()
    }
}

pub fn load_env_schema(
    project_dir: &Path,
    config: &ProjectConfig,
) -> Result<Option<(EnvSchema, Vec<AnalysisDiagnostic>)>, String> {
    let Some(env_file) = config
        .security
        .as_ref()
        .and_then(|security| security.env_file.as_deref())
    else {
        return Ok(None);
    };
    let file_path = project_dir.join(env_file);
    if !file_path.exists() {
        return Err(format!(
            "env file `{env_file}` configured in tyrus.json (security.envFile) was not found in '{}'",
            project_dir.display()
        ));
    }
    let source = fs::read_to_string(&file_path)
        .map_err(|e| format!("could not read env file `{env_file}`: {e}"))?;
    let (vars, mut diagnostics) = parse_env_source(&source);
    diagnostics.extend(check_env_schema(&vars));
    Ok(Some((
        EnvSchema {
            env_file: env_file.to_string(),
            file_path,
            source,
            vars,
        },
        diagnostics,
    )))
}

pub fn render_env_example(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    for line in source.split_inclusive('\n') {
        let (body, terminator) = split_line_terminator(line);
        match classify_line(body) {
            LineKind::Assignment { eq_index } => {
                output.push_str(&body[..=eq_index]);
            }
            _ => output.push_str(body),
        }
        output.push_str(terminator);
    }
    output
}

pub fn example_path(env_path: &Path) -> PathBuf {
    let mut name = env_path
        .file_name()
        .map(std::ffi::OsStr::to_os_string)
        .unwrap_or_default();
    name.push(".example");
    env_path.with_file_name(name)
}

pub fn sync_env_example(schema: &EnvSchema) -> std::io::Result<bool> {
    let example = render_env_example(&schema.source);
    let path = example_path(&schema.file_path);
    if matches!(fs::read_to_string(&path), Ok(current) if current == example) {
        return Ok(false);
    }
    fs::write(&path, example)?;
    Ok(true)
}

enum LineKind {
    Blank,
    Comment,
    Decorators {
        content_start: usize,
        commented: bool,
    },
    Assignment {
        eq_index: usize,
    },
    BareKey,
    Invalid,
}

fn classify_line(body: &str) -> LineKind {
    let trimmed = body.trim_start();
    let indent = body.len() - trimmed.len();
    if trimmed.is_empty() {
        return LineKind::Blank;
    }
    if let Some(rest) = trimmed.strip_prefix('#') {
        let rest_trimmed = rest.trim_start();
        if rest_trimmed.starts_with('@') {
            let content_start = indent + 1 + (rest.len() - rest_trimmed.len());
            return LineKind::Decorators {
                content_start,
                commented: true,
            };
        }
        return LineKind::Comment;
    }
    if trimmed.starts_with('@') {
        return LineKind::Decorators {
            content_start: indent,
            commented: false,
        };
    }
    match body.find('=') {
        Some(eq_index) => LineKind::Assignment { eq_index },
        None if is_valid_env_name(trimmed.trim_end()) => LineKind::BareKey,
        None => LineKind::Invalid,
    }
}

fn source_uses_export(source: &str) -> bool {
    source.lines().any(|line| {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('@') {
            return false;
        }
        trimmed
            .strip_prefix("export")
            .is_some_and(|rest| rest.starts_with([' ', '\t']))
    })
}

fn split_line_terminator(line: &str) -> (&str, &str) {
    if let Some(body) = line.strip_suffix("\r\n") {
        return (body, "\r\n");
    }
    if let Some(body) = line.strip_suffix('\n') {
        return (body, "\n");
    }
    (line, "")
}

pub fn parse_env_source(source: &str) -> (Vec<EnvVarDecl>, Vec<AnalysisDiagnostic>) {
    let mut vars: Vec<EnvVarDecl> = Vec::new();
    let mut diagnostics = Vec::new();
    let mut pending = PendingDecorators::default();
    let mut offset = 0;
    let uses_export = source_uses_export(source);

    for line in source.split_inclusive('\n') {
        let (body, _) = split_line_terminator(line);
        match classify_line(body) {
            LineKind::Blank | LineKind::Comment => {}
            LineKind::Decorators {
                content_start,
                commented,
            } => {
                if !commented && uses_export {
                    let content = &body[content_start..];
                    let end = content_start + content.trim_end().len();
                    diagnostics.push(env_error(
                        "E3152",
                        offset + content_start..offset + end,
                        "a raw validator decoration breaks a shell-sourced env file (it uses `export`)"
                            .to_string(),
                        Some(
                            "comment it with `# ` (e.g. `# @type(u32)`) so the file stays shell-sourceable"
                                .to_string(),
                        ),
                    ));
                }
                parse_decorator_line(
                    &body[content_start..],
                    offset + content_start,
                    &mut pending,
                    &mut diagnostics,
                );
            }
            LineKind::Assignment { eq_index } => {
                parse_assignment_line(
                    body,
                    Some(eq_index),
                    offset,
                    &mut pending,
                    &mut vars,
                    &mut diagnostics,
                );
            }
            LineKind::BareKey => {
                parse_assignment_line(
                    body,
                    None,
                    offset,
                    &mut pending,
                    &mut vars,
                    &mut diagnostics,
                );
            }
            LineKind::Invalid => {
                let trimmed = body.trim();
                let start = offset + (body.len() - body.trim_start().len());
                diagnostics.push(env_error(
                    "E3146",
                    start..start + trimmed.len(),
                    format!("invalid line in env file: `{trimmed}`"),
                    Some(
                        "expected `KEY=value`, a `@validator(...)` decorator, or a `#` comment"
                            .to_string(),
                    ),
                ));
            }
        }
        offset += line.len();
    }

    if let Some(range) = pending.first_range() {
        diagnostics.push(env_error(
            "E3146",
            range,
            "validator decorators must be followed by a `KEY=value` line".to_string(),
            Some("move the decorators directly above the variable they validate".to_string()),
        ));
    }

    resolve_interpolation(&mut vars, &mut diagnostics);
    (vars, diagnostics)
}

fn resolve_interpolation(vars: &mut [EnvVarDecl], diagnostics: &mut Vec<AnalysisDiagnostic>) {
    let entries: Vec<(String, String, bool)> = vars
        .iter()
        .map(|var| (var.name.clone(), var.raw_value.clone(), var.interpolatable))
        .collect();
    for (var, resolved) in vars.iter_mut().zip(tyrus_validators::env_resolve(&entries)) {
        match resolved {
            Ok(value) => var.raw_value = value,
            Err(tyrus_validators::EnvRefError::Undefined(name)) => {
                diagnostics.push(env_error(
                    "E3150",
                    var.value_range.clone(),
                    format!(
                        "`{}` references undefined environment variable `{name}`",
                        var.name
                    ),
                    Some(format!(
                        "declare `{name}` in this env file or remove `${{{name}}}`"
                    )),
                ));
            }
            Err(tyrus_validators::EnvRefError::Cyclic(_)) => {
                diagnostics.push(env_error(
                    "E3151",
                    var.value_range.clone(),
                    format!("`{}` is part of a `${{...}}` reference cycle", var.name),
                    Some("break the cycle so the value can be resolved".to_string()),
                ));
            }
        }
    }
}

#[derive(Default)]
struct PendingDecorators {
    decorators: Vec<ValidatorDecorator>,
    decorator_ranges: Vec<Range<usize>>,
    declared_type: Option<(Type, Range<usize>)>,
    type_message: Option<String>,
}

impl PendingDecorators {
    fn first_range(&self) -> Option<Range<usize>> {
        let type_range = self.declared_type.as_ref().map(|(_, range)| range.clone());
        match (self.decorator_ranges.first(), type_range) {
            (Some(range), Some(type_range)) => Some(if type_range.start < range.start {
                type_range
            } else {
                range.clone()
            }),
            (Some(range), None) => Some(range.clone()),
            (None, type_range) => type_range,
        }
    }
}

fn parse_assignment_line(
    body: &str,
    eq_index: Option<usize>,
    line_offset: usize,
    pending: &mut PendingDecorators,
    vars: &mut Vec<EnvVarDecl>,
    diagnostics: &mut Vec<AnalysisDiagnostic>,
) {
    let taken = std::mem::take(pending);
    let key_part = match eq_index {
        Some(eq_index) => &body[..eq_index],
        None => body,
    };
    let key = tyrus_validators::env_strip_export(key_part).trim();
    let key_start = line_offset + (key.as_ptr() as usize - body.as_ptr() as usize);
    let name_range = key_start..key_start + key.len();

    if key.is_empty() || !is_valid_env_name(key) {
        diagnostics.push(env_error(
            "E3146",
            name_range,
            format!("invalid environment variable name `{key}`"),
            Some(
                "names must start with a letter or `_` and contain only letters, digits, and `_`"
                    .to_string(),
            ),
        ));
        return;
    }

    let (raw_value, value_range) = match eq_index {
        Some(eq_index) => {
            let value_part = &body[eq_index + 1..];
            let raw_value = value_part.trim();
            let value_start =
                line_offset + eq_index + 1 + (value_part.len() - value_part.trim_start().len());
            (raw_value, value_start..value_start + raw_value.len())
        }
        None => ("", name_range.end..name_range.end),
    };

    if let Some(previous) = vars.iter().find(|var| var.name == key) {
        diagnostics.push(AnalysisDiagnostic {
            code: "E3142".to_string(),
            range: name_range,
            severity: DiagnosticSeverity::Error,
            message: format!("environment variable `{key}` is declared more than once"),
            related: vec![AnalysisRelatedInformation {
                range: previous.name_range.clone(),
                message: format!("`{key}` is first declared here"),
            }],
            help: vec!["remove the duplicated declaration".to_string()],
        });
        return;
    }

    let (declared_type, type_range) = match taken.declared_type {
        Some((ty, range)) => (ty, Some(range)),
        None => (Type::Str, None),
    };
    let (value, interpolatable) = tyrus_validators::env_unquote(raw_value);
    vars.push(EnvVarDecl {
        name: key.to_string(),
        name_range,
        raw_value: value,
        value_range,
        declared_type,
        type_message: taken.type_message,
        type_range,
        decorators: taken.decorators,
        decorator_ranges: taken.decorator_ranges,
        interpolatable,
    });
}

fn parse_decorator_line(
    content: &str,
    content_offset: usize,
    pending: &mut PendingDecorators,
    diagnostics: &mut Vec<AnalysisDiagnostic>,
) {
    for chunk in split_decorator_chunks(content) {
        let chunk_text = &content[chunk.clone()];
        let chunk_range = content_offset + chunk.start..content_offset + chunk.end;
        let after_at = &chunk_text[1..];
        let name_len = after_at
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .unwrap_or(after_at.len());
        if &after_at[..name_len] == "type" {
            parse_type_decorator(chunk_text, chunk_range, pending, diagnostics);
        } else {
            parse_validator_chunk(chunk_text, chunk_range, pending, diagnostics);
        }
    }
}

fn split_decorator_chunks(content: &str) -> Vec<Range<usize>> {
    let mut chunks = Vec::new();
    let mut start = None;
    let mut depth = 0usize;
    let mut in_string: Option<char> = None;
    let mut escaped = false;
    for (index, c) in content.char_indices() {
        if let Some(quote) = in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == quote {
                in_string = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => in_string = Some(c),
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            '@' if depth == 0 => {
                if let Some(chunk_start) = start.take() {
                    chunks.push(trim_range(content, chunk_start..index));
                }
                start = Some(index);
            }
            _ => {}
        }
    }
    if let Some(chunk_start) = start {
        chunks.push(trim_range(content, chunk_start..content.len()));
    }
    chunks
}

fn trim_range(content: &str, range: Range<usize>) -> Range<usize> {
    let text = &content[range.clone()];
    let end = range.start + text.trim_end().len();
    range.start..end
}

fn parse_validator_chunk(
    chunk_text: &str,
    chunk_range: Range<usize>,
    pending: &mut PendingDecorators,
    diagnostics: &mut Vec<AnalysisDiagnostic>,
) {
    let tokens = match crate::lexer::lex_spanned(chunk_text) {
        Ok(tokens) => tokens,
        Err(_) => {
            diagnostics.push(env_error(
                "E3146",
                chunk_range,
                format!("invalid validator decorator `{chunk_text}`"),
                Some("expected `@validator(arguments...)`".to_string()),
            ));
            return;
        }
    };
    let (tokens, ranges): (Vec<_>, Vec<_>) = tokens
        .into_iter()
        .map(|(token, range)| {
            (
                token,
                chunk_range.start + range.start..chunk_range.start + range.end,
            )
        })
        .unzip();
    let mut parser = Parser::new_with_ranges(tokens, ranges);
    match parser.parse_decorators() {
        Ok(decorators) if parser.is_at_end() && decorators.len() == 1 => {
            let decorator = decorators.into_iter().next().unwrap();
            pending.decorators.push(decorator);
            pending.decorator_ranges.push(chunk_range);
        }
        _ => {
            diagnostics.push(env_error(
                "E3146",
                chunk_range,
                format!("invalid validator decorator `{chunk_text}`"),
                Some("expected `@validator(arguments...)`".to_string()),
            ));
        }
    }
}

fn parse_type_decorator(
    chunk_text: &str,
    chunk_range: Range<usize>,
    pending: &mut PendingDecorators,
    diagnostics: &mut Vec<AnalysisDiagnostic>,
) {
    let invalid = |message: String, help: &str| {
        env_error(
            "E3146",
            chunk_range.clone(),
            message,
            Some(help.to_string()),
        )
    };
    let arguments = chunk_text["@type".len()..].trim();
    let Some(arguments) = arguments
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
    else {
        diagnostics.push(invalid(
            format!("invalid `@type` decorator `{chunk_text}`"),
            "expected `@type(T)` or `@type(T, \"message\")`",
        ));
        return;
    };
    let (type_name, message_part) = match arguments.find(',') {
        Some(comma) => (&arguments[..comma], Some(&arguments[comma + 1..])),
        None => (arguments, None),
    };
    let type_name = type_name.trim();
    let type_name_offset = chunk_text.find(type_name).unwrap_or(0);
    let type_name_range = chunk_range.start + type_name_offset
        ..chunk_range.start + type_name_offset + type_name.len();

    let Some((_, declared_type)) = ENV_TYPE_NAMES.iter().find(|(name, _)| *name == type_name)
    else {
        diagnostics.push(env_error(
            "E3149",
            type_name_range,
            format!("unknown env variable type `{type_name}`"),
            Some(
                "supported types are `string`, `boolean`, `i32`, `i64`, `u32`, `u64`, `f32`, and `f64`"
                    .to_string(),
            ),
        ));
        return;
    };

    let type_message = match message_part {
        Some(message_source) => match parse_string_literal(message_source) {
            Some(message) => Some(message),
            None => {
                diagnostics.push(invalid(
                    format!("invalid `@type` message `{}`", message_source.trim()),
                    "the second argument of `@type` must be a string literal",
                ));
                return;
            }
        },
        None => None,
    };

    if pending.declared_type.is_some() {
        diagnostics.push(invalid(
            "duplicate `@type` decorator for the same variable".to_string(),
            "declare a single `@type` per variable",
        ));
        return;
    }
    pending.declared_type = Some((declared_type.clone(), chunk_range));
    pending.type_message = type_message;
}

fn parse_string_literal(source: &str) -> Option<String> {
    let tokens = crate::lexer::lex(source).ok()?;
    match tokens.as_slice() {
        [crate::lexer::Token::StringLit(value)] => Some(value.clone()),
        _ => None,
    }
}

fn is_valid_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    let first = chars.next().unwrap_or(' ');
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn check_env_schema(vars: &[EnvVarDecl]) -> Vec<AnalysisDiagnostic> {
    let mut diagnostics = Vec::new();
    for var in vars {
        if var.type_range.is_none() {
            diagnostics.push(env_error(
                "E3147",
                var.name_range.clone(),
                format!(
                    "environment variable `{}` is missing a `@type` declaration",
                    var.name
                ),
                Some(format!(
                    "add a `@type` decorator above `{}`, e.g. `@type(string)`",
                    var.name
                )),
            ));
        }
        for (decorator, range) in var.decorators.iter().zip(&var.decorator_ranges) {
            for diagnostic in validate_decorator(decorator, &var.declared_type) {
                diagnostics.push(from_validator_diagnostic(diagnostic, range.clone()));
            }
        }
        for diagnostic in validate_decorator_set(&var.decorators) {
            let range = decorator_subject_range(var, diagnostic.subject.as_deref());
            diagnostics.push(from_validator_diagnostic(diagnostic, range));
        }

        if var.raw_value.is_empty() {
            diagnostics.push(env_error(
                "E3148",
                var.name_range.clone(),
                format!("environment variable `{}` has no value", var.name),
                Some(format!("assign a value, e.g. `{}=...`", var.name)),
            ));
            continue;
        }
        let Some(value) = parse_env_value(&var.raw_value, &var.declared_type) else {
            let message = var
                .type_message
                .clone()
                .unwrap_or_else(|| format!("value is not a valid `{}`", var.declared_type));
            diagnostics.push(AnalysisDiagnostic {
                code: "E3143".to_string(),
                range: var.value_range.clone(),
                severity: DiagnosticSeverity::Error,
                message: format!("`@type` failed for `{}`: {message}", var.name),
                related: var
                    .type_range
                    .clone()
                    .map(|range| AnalysisRelatedInformation {
                        range,
                        message: format!(
                            "`{}` is declared as `{}` here",
                            var.name, var.declared_type
                        ),
                    })
                    .into_iter()
                    .collect(),
                help: vec![format!(
                    "set `{}` to a `{}` value",
                    var.name, var.declared_type
                )],
            });
            continue;
        };
        for (decorator, range) in var.decorators.iter().zip(&var.decorator_ranges) {
            if let Some(message) = validate_literal_value(decorator, &value, &var.declared_type) {
                diagnostics.push(AnalysisDiagnostic {
                    code: "E3143".to_string(),
                    range: var.value_range.clone(),
                    severity: DiagnosticSeverity::Error,
                    message: format!(
                        "`{}` failed for `{}`: {message}",
                        crate::validators::validator_display(decorator),
                        var.name
                    ),
                    related: vec![AnalysisRelatedInformation {
                        range: range.clone(),
                        message: "the failing validator is declared here".to_string(),
                    }],
                    help: vec![format!(
                        "change the value of `{}` to satisfy the validator",
                        var.name
                    )],
                });
            }
        }
    }
    diagnostics
}

const REDACTED_VALUE: &str = "<redacted>";

type RedactedSpanMap = Vec<(Range<usize>, Range<usize>)>;

fn redact_env_source(source: &str, vars: &[EnvVarDecl]) -> (String, RedactedSpanMap) {
    let mut redacted = String::with_capacity(source.len());
    let mut spans: Vec<(Range<usize>, Range<usize>)> = Vec::new();
    let mut cursor = 0;
    for var in vars {
        let value = var.value_range.clone();
        if value.is_empty() || value.start < cursor || value.end > source.len() {
            continue;
        }
        redacted.push_str(&source[cursor..value.start]);
        let start = redacted.len();
        redacted.push_str(REDACTED_VALUE);
        spans.push((value.clone(), start..redacted.len()));
        cursor = value.end;
    }
    redacted.push_str(&source[cursor..]);
    (redacted, spans)
}

fn remap_redacted_range(
    range: &Range<usize>,
    spans: &[(Range<usize>, Range<usize>)],
) -> Range<usize> {
    if let Some((_, replacement)) = spans
        .iter()
        .find(|(original, _)| range.start >= original.start && range.start < original.end)
    {
        return replacement.clone();
    }
    let delta: isize = spans
        .iter()
        .filter(|(original, _)| original.end <= range.start)
        .map(|(original, replacement)| replacement.len() as isize - original.len() as isize)
        .sum();
    let shift = |offset: usize| (offset as isize + delta).max(0) as usize;
    shift(range.start)..shift(range.end)
}

pub fn render_env_diagnostic(
    path: &Path,
    source: &str,
    vars: &[EnvVarDecl],
    diagnostic: &AnalysisDiagnostic,
) -> String {
    let (redacted, spans) = redact_env_source(source, vars);
    let mut diagnostic = diagnostic.clone();
    diagnostic.range = remap_redacted_range(&diagnostic.range, &spans);
    for related in &mut diagnostic.related {
        related.range = remap_redacted_range(&related.range, &spans);
    }
    crate::analysis::render_diagnostic(path, &redacted, &diagnostic)
}

pub fn parse_env_value(raw_value: &str, declared_type: &Type) -> Option<Expr> {
    match declared_type {
        Type::Str => Some(Expr::synthetic(ExprKind::StrLit(raw_value.to_string()))),
        Type::Bool => match raw_value {
            "true" => Some(Expr::synthetic(ExprKind::BoolLit(true))),
            "false" => Some(Expr::synthetic(ExprKind::BoolLit(false))),
            _ => None,
        },
        Type::I32 => raw_value
            .parse::<i32>()
            .ok()
            .map(|value| Expr::synthetic(ExprKind::IntLit(value as i128))),
        Type::I64 => raw_value
            .parse::<i64>()
            .ok()
            .map(|value| Expr::synthetic(ExprKind::IntLit(value as i128))),
        Type::U32 => raw_value
            .parse::<u32>()
            .ok()
            .map(|value| Expr::synthetic(ExprKind::IntLit(value as i128))),
        Type::U64 => raw_value
            .parse::<u64>()
            .ok()
            .map(|value| Expr::synthetic(ExprKind::IntLit(value as i128))),
        Type::F32 => raw_value.parse::<f32>().ok().map(|value| {
            Expr::synthetic(ExprKind::FloatLit(FloatLiteral::from_value(value as f64)))
        }),
        Type::F64 => raw_value
            .parse::<f64>()
            .ok()
            .map(|value| Expr::synthetic(ExprKind::FloatLit(FloatLiteral::from_value(value)))),
        _ => None,
    }
}

fn decorator_subject_range(var: &EnvVarDecl, subject: Option<&str>) -> Range<usize> {
    let fallback = var.name_range.clone();
    let Some(subject) = subject else {
        return fallback;
    };
    let subject = subject.strip_prefix('@').unwrap_or(subject);
    let (name, ordinal) = match subject.split_once('#') {
        Some((name, ordinal)) => (name, ordinal.parse::<usize>().unwrap_or(1)),
        None => (subject, 1),
    };
    var.decorators
        .iter()
        .zip(&var.decorator_ranges)
        .filter(|(decorator, _)| decorator.name == name)
        .nth(ordinal.saturating_sub(1))
        .map(|(_, range)| range.clone())
        .unwrap_or(fallback)
}

fn from_validator_diagnostic(
    diagnostic: ValidatorDiagnostic,
    range: Range<usize>,
) -> AnalysisDiagnostic {
    AnalysisDiagnostic {
        code: diagnostic.code.to_string(),
        range,
        severity: match diagnostic.severity {
            ValidatorDiagnosticSeverity::Error => DiagnosticSeverity::Error,
            ValidatorDiagnosticSeverity::Warning => DiagnosticSeverity::Warning,
        },
        message: diagnostic.message,
        related: Vec::new(),
        help: diagnostic.help.into_iter().collect(),
    }
}

fn env_error(
    code: &str,
    range: Range<usize>,
    message: String,
    help: Option<String>,
) -> AnalysisDiagnostic {
    AnalysisDiagnostic {
        code: code.to_string(),
        range,
        severity: DiagnosticSeverity::Error,
        message,
        related: Vec::new(),
        help: help.into_iter().collect(),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/env.rs"]
mod tests;
