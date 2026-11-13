use super::*;

pub(super) fn diagnostic_to_lsp(
    uri: &Url,
    source: &str,
    diagnostic: &AnalysisDiagnostic,
) -> Diagnostic {
    let related_information = diagnostic
        .related
        .iter()
        .map(|related| DiagnosticRelatedInformation {
            location: Location {
                uri: uri.clone(),
                range: byte_range_to_lsp(source, related.range.clone()),
            },
            message: related.message.clone(),
        })
        .collect::<Vec<_>>();
    Diagnostic {
        range: byte_range_to_lsp(source, diagnostic.range.clone()),
        severity: Some(match diagnostic.severity {
            DiagnosticSeverity::Error => tower_lsp::lsp_types::DiagnosticSeverity::ERROR,
            DiagnosticSeverity::Warning => tower_lsp::lsp_types::DiagnosticSeverity::WARNING,
        }),
        source: Some("tyrus".to_string()),
        code: Some(NumberOrString::String(diagnostic.code.clone())),
        message: diagnostic.message.clone(),
        related_information: (!related_information.is_empty()).then_some(related_information),
        data: (!diagnostic.help.is_empty()).then(|| {
            serde_json::json!({
                "help": diagnostic.help,
            })
        }),
        ..Diagnostic::default()
    }
}

pub(super) fn lint_diagnostic_to_lsp(
    source: &str,
    diagnostic: &tyrus_lint::LintDiagnostic,
) -> Diagnostic {
    Diagnostic {
        range: byte_range_to_lsp(source, diagnostic.range.clone()),
        severity: Some(match diagnostic.severity {
            tyrus_lint::Severity::Off => tower_lsp::lsp_types::DiagnosticSeverity::HINT,
            tyrus_lint::Severity::Info => tower_lsp::lsp_types::DiagnosticSeverity::INFORMATION,
            tyrus_lint::Severity::Warn => tower_lsp::lsp_types::DiagnosticSeverity::WARNING,
            tyrus_lint::Severity::Error => tower_lsp::lsp_types::DiagnosticSeverity::ERROR,
        }),
        source: Some("tyrus-lint".to_string()),
        code: Some(NumberOrString::String(diagnostic.code.clone())),
        message: diagnostic.message.clone(),
        ..Diagnostic::default()
    }
}

pub(super) fn diagnostic_hover_markdown(source: &str, diagnostic: &AnalysisDiagnostic) -> String {
    let mut markdown = format!("**{}**\n`{}`", diagnostic.message, diagnostic.code);
    markdown.push_str("\n\n");
    markdown.push_str(&diagnostic_markdown_block(source, &diagnostic.range, None));
    for related in &diagnostic.related {
        markdown.push_str("\n\n");
        markdown.push_str(&diagnostic_markdown_block(
            source,
            &related.range,
            Some(&related.message),
        ));
    }
    if !diagnostic.help.is_empty() {
        markdown.push_str("\n\n---");
        for help in &diagnostic.help {
            markdown.push_str("\n\n");
            markdown.push_str(&format_hover_help(help));
        }
    }
    markdown
}

fn diagnostic_markdown_block(
    source: &str,
    range: &std::ops::Range<usize>,
    label: Option<&str>,
) -> String {
    let (line, column, line_text) = diagnostic_line(source, range);
    let underline_width = range.end.saturating_sub(range.start).max(1);
    let underline = format!(
        "{}{}{}",
        " ".repeat(column.saturating_sub(1)),
        "^".repeat(underline_width),
        label.map_or(String::new(), |message| format!(" {message}"))
    );
    format!("**line {line}:{column}**\n```\n{line_text}\n{underline}\n```")
}

fn diagnostic_line<'a>(source: &'a str, range: &std::ops::Range<usize>) -> (usize, usize, &'a str) {
    let offset = range.start.min(source.len());
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let column = source[line_start..offset].chars().count() + 1;
    let line_text = source[offset..]
        .find('\n')
        .map_or(&source[line_start..], |length| {
            &source[line_start..offset + length]
        });
    (line, column, line_text)
}

fn format_hover_help(help: &str) -> String {
    let Some(example) = help.strip_prefix("for example: `") else {
        return format!("help: {help}");
    };
    let Some(example) = example.strip_suffix('`') else {
        return format!("help: {help}");
    };
    format!("help: for example:\n\n```tyrus\n{example}\n```")
}

pub(super) fn byte_range_to_lsp(source: &str, range: std::ops::Range<usize>) -> Range {
    Range::new(
        byte_offset_to_position(source, range.start),
        byte_offset_to_position(source, range.end),
    )
}

pub(super) fn imported_specifier_range(
    analysis: &DocumentAnalysis,
    imported: &str,
    local: &str,
    declaration: &std::ops::Range<usize>,
) -> Option<std::ops::Range<usize>> {
    use tyrus_core::lexer::Token;
    if imported == local {
        return Some(declaration.clone());
    }
    let start = analysis
        .tokens
        .partition_point(|token| token.range.end < declaration.start);
    let local_index = analysis.tokens[start..]
        .iter()
        .take_while(|token| token.range.start < declaration.end)
        .position(|token| {
            token.range.start <= declaration.start
                && declaration.end <= token.range.end
                && matches!(&token.token, Token::Ident(name) if name == local)
        })?
        .saturating_add(start);
    let imported_index = local_index.checked_sub(2)?;
    let as_index = local_index.checked_sub(1)?;
    (matches!(&analysis.tokens.get(as_index)?.token, Token::As)
        && matches!(&analysis.tokens.get(imported_index)?.token, Token::Ident(name) if name == imported))
        .then(|| analysis.tokens[imported_index].range.clone())
}

pub(super) fn signature_active_parameter(
    call: &tyrus_core::analysis::CallSite,
    offset: usize,
    param_count: usize,
) -> u32 {
    let explicit = call
        .arguments
        .iter()
        .position(|argument| offset <= argument.end)
        .unwrap_or(call.arguments.len());
    ((call.arg_offset + explicit) as u32).min(param_count.saturating_sub(1) as u32)
}

pub(super) fn symbol_occurrence_ranges(
    analysis: &DocumentAnalysis,
    symbol: &Symbol,
) -> Vec<std::ops::Range<usize>> {
    let mut ranges = vec![symbol.declaration.clone()];
    if let Some(references) = analysis
        .references_by_definition
        .get(&(symbol.declaration.start, symbol.declaration.end))
    {
        ranges.extend(references.iter().cloned());
    } else {
        for reference in &analysis.references {
            if reference.definition.as_ref() == Some(&symbol.declaration) {
                ranges.push(reference.range.clone());
            }
        }
    }
    ranges.sort_by_key(|range| range.start);
    ranges.dedup();
    ranges
}

#[cfg(test)]
pub(super) fn cross_file_occurrences_in(
    files: &[(Url, String)],
    defining_uri: &Url,
    exported_name: &str,
    rename: bool,
) -> Vec<(Url, Vec<std::ops::Range<usize>>)> {
    let analyses = files
        .iter()
        .map(|(uri, source)| (uri.clone(), Arc::new(analyze(source))))
        .collect();
    cross_file_occurrences_with_analyses(files, &analyses, defining_uri, exported_name, rename)
}

#[cfg(test)]
pub(super) fn cross_file_occurrences_with_analyses(
    files: &[(Url, String)],
    analyses: &tyrus_core::collections::HashMap<Url, Arc<DocumentAnalysis>>,
    defining_uri: &Url,
    exported_name: &str,
    rename: bool,
) -> Vec<(Url, Vec<std::ops::Range<usize>>)> {
    cross_file_occurrences_with_resolved_aliases(
        files,
        analyses,
        defining_uri,
        exported_name,
        rename,
        &tyrus_core::collections::HashMap::default(),
        &tyrus_core::collections::HashMap::default(),
    )
}

pub(super) fn cross_file_occurrences_with_resolved_aliases(
    files: &[(Url, String)],
    analyses: &tyrus_core::collections::HashMap<Url, Arc<DocumentAnalysis>>,
    defining_uri: &Url,
    exported_name: &str,
    rename: bool,
    resolved_aliases: &tyrus_core::collections::HashMap<String, Url>,
    resolved_import_targets: &tyrus_core::collections::HashMap<(Url, String), Url>,
) -> Vec<(Url, Vec<std::ops::Range<usize>>)> {
    let mut result = Vec::new();
    for (file_uri, _) in files {
        let Some(analysis) = analyses.get(file_uri) else {
            continue;
        };
        let mut ranges = Vec::new();
        if file_uri == defining_uri {
            if let Some(symbol) = analysis.exported_symbol_named(exported_name) {
                ranges.extend(symbol_occurrence_ranges(analysis, symbol));
            }
        } else {
            let Ok(document) = file_uri.to_file_path() else {
                continue;
            };
            let mut process_import = |symbol: &Symbol| {
                let imported = symbol.imported_name.as_deref().unwrap_or(&symbol.name);
                if imported != exported_name {
                    return;
                }
                let Some(import_source) = symbol.import_source.as_deref() else {
                    return;
                };
                let cached_target =
                    resolved_import_targets.get(&(file_uri.clone(), import_source.to_string()));
                let resolves_here = cached_target.is_some_and(|target| target == defining_uri)
                    || (cached_target.is_none()
                        && if import_source.starts_with('#') {
                            resolved_aliases
                                .get(import_source)
                                .is_some_and(|target| target == defining_uri)
                        } else {
                            resolve_import_path(&document, import_source)
                                .and_then(|path| Url::from_file_path(path).ok())
                                .is_some_and(|target| target == *defining_uri)
                        });
                if !resolves_here {
                    return;
                }
                let aliased = symbol.name.as_str() != imported;
                if let Some(specifier) =
                    imported_specifier_range(analysis, imported, &symbol.name, &symbol.declaration)
                {
                    ranges.push(specifier);
                }
                if !rename || !aliased {
                    ranges.extend(symbol_occurrence_ranges(analysis, symbol));
                }
            };
            if let Some(import_indexes) = analysis.imported_symbol_indexes_named(exported_name) {
                for index in import_indexes {
                    process_import(&analysis.symbols[*index]);
                }
            } else {
                for symbol in analysis
                    .symbols
                    .iter()
                    .filter(|symbol| symbol.kind == SymbolKind::Import)
                {
                    process_import(symbol);
                }
            }
        }
        if !ranges.is_empty() {
            ranges.sort_by_key(|range| range.start);
            ranges.dedup();
            result.push((file_uri.clone(), ranges));
        }
    }
    result
}

pub(super) const SEMANTIC_TOKEN_PARAMETER: u32 = 0;
pub(super) const SEMANTIC_TOKEN_KEYWORD: u32 = 1;

pub(super) fn comptime_keyword_ranges(source: &str) -> Vec<std::ops::Range<usize>> {
    use tyrus_core::lexer::Token;
    let Ok(tokens) = tyrus_core::lexer::lex_spanned(source) else {
        return Vec::new();
    };
    let mut ranges = Vec::new();
    let mut after_comptime = false;
    for (token, range) in tokens {
        if matches!(token, Token::Comptime) {
            ranges.push(range);
            after_comptime = true;
            continue;
        }
        if after_comptime && matches!(token, Token::Const) {
            ranges.push(range);
        }
        after_comptime = false;
    }
    ranges
}

pub(super) fn comptime_hover_at(
    analysis: &DocumentAnalysis,
    offset: usize,
) -> Option<(std::ops::Range<usize>, String)> {
    let evaluation = analysis.ctfe.evaluations.iter().find(|evaluation| {
        let range = evaluation.expression_span.to_range();
        let keyword_end = range.start.saturating_add("comptime".len()).min(range.end);
        range.start <= offset && offset < keyword_end
    })?;
    let wrapper = evaluation.expression_span.to_range();
    let ty = analysis
        .type_at(wrapper.start)
        .map_or_else(|| "unknown".to_string(), |expression| expression.ty.clone());
    let keyword = wrapper.start..wrapper.start + "comptime".len();
    Some((
        keyword,
        format!(
            "Evaluated at compile time.\n\n```tyrus\n{ty}\n```\n\nValue: `{}`",
            summarize_const_value(&evaluation.value)
        ),
    ))
}

fn summarize_const_value(value: &tyrus_core::ctfe::ConstValue) -> String {
    use tyrus_core::ctfe::ConstValue;
    fn summarize(value: &ConstValue, depth: usize) -> String {
        if depth >= 4 {
            return "…".to_string();
        }
        match value {
            ConstValue::Bool(value) => value.to_string(),
            ConstValue::Int { value, .. } => value.to_string(),
            ConstValue::UInt { value, .. } => value.to_string(),
            ConstValue::Float { text, .. } => text.clone(),
            ConstValue::Decimal(value) if value.exponent == 0 => value.coefficient.to_string(),
            ConstValue::Decimal(value) => format!("{}e{}d", value.coefficient, value.exponent),
            ConstValue::Char(value) => format!("'{value}'"),
            ConstValue::String(value) => {
                let mut characters = value.escape_default();
                let escaped = characters.by_ref().take(80).collect::<String>();
                let suffix = if characters.next().is_some() {
                    "…"
                } else {
                    ""
                };
                format!("\"{escaped}{suffix}\"")
            }
            ConstValue::Void => "void".to_string(),
            ConstValue::Array(values) | ConstValue::Tuple(values) => {
                let mut parts = values
                    .iter()
                    .take(4)
                    .map(|value| summarize(value, depth + 1))
                    .collect::<Vec<_>>();
                if values.len() > 4 {
                    parts.push("…".to_string());
                }
                format!("[{}]", parts.join(", "))
            }
            ConstValue::Record(fields) => {
                let mut parts = fields
                    .iter()
                    .take(4)
                    .map(|(name, value)| format!("{name}: {}", summarize(value, depth + 1)))
                    .collect::<Vec<_>>();
                if fields.len() > 4 {
                    parts.push("…".to_string());
                }
                format!("{{ {} }}", parts.join(", "))
            }
            ConstValue::Enum {
                enum_name, member, ..
            } => format!("{enum_name}.{member}"),
        }
    }

    let summary = summarize(value, 0);
    let mut chars = summary.chars();
    let truncated = chars.by_ref().take(160).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}…")
    } else {
        truncated
    }
}

pub(super) fn reserved_keyword_ranges(source: &str) -> Vec<std::ops::Range<usize>> {
    use tyrus_core::lexer::Token;
    let Ok(tokens) = tyrus_core::lexer::lex_spanned(source) else {
        return Vec::new();
    };
    let mut ranges = Vec::new();
    let mut previous: Option<&Token> = None;
    for (token, range) in &tokens {
        if let Token::Ident(name) = token {
            let is_member = matches!(previous, Some(Token::Dot | Token::OptChain));
            if !is_member && tyrus_core::keywords::is_reserved_future(name) {
                ranges.push(range.clone());
            }
        }
        previous = Some(token);
    }
    ranges
}

pub(super) fn encode_semantic_tokens(
    source: &str,
    mut entries: Vec<(std::ops::Range<usize>, u32)>,
) -> Vec<SemanticToken> {
    entries.sort_by_key(|entry| entry.0.start);
    entries.dedup_by(|a, b| a.0 == b.0);
    let mut tokens = Vec::with_capacity(entries.len());
    let mut prev_line = 0u32;
    let mut prev_char = 0u32;
    for (range, token_type) in entries {
        let pos = byte_offset_to_position(source, range.start);
        let length = (range.end - range.start) as u32;
        let delta_line = pos.line - prev_line;
        let delta_start = if delta_line == 0 {
            pos.character - prev_char
        } else {
            pos.character
        };
        tokens.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset: 0,
        });
        prev_line = pos.line;
        prev_char = pos.character;
    }
    tokens
}

pub(super) fn format_document_edits(
    source: &str,
    config: &tyrus_fmt::config::Config,
) -> Option<Vec<TextEdit>> {
    let formatted = tyrus_fmt::format_source(source, config);
    format_document_edits_from_output(source, formatted)
}

pub(super) fn format_document_edits_with_program(
    source: &str,
    config: &tyrus_fmt::config::Config,
    program: &tyrus_core::ast::Program,
) -> Option<Vec<TextEdit>> {
    let formatted = tyrus_fmt::format_source_with_program(source, config, program);
    format_document_edits_from_output(source, formatted)
}

fn format_document_edits_from_output(source: &str, formatted: String) -> Option<Vec<TextEdit>> {
    if formatted == source {
        return None;
    }
    let mut prefix = source
        .bytes()
        .zip(formatted.bytes())
        .take_while(|(left, right)| left == right)
        .count();
    while prefix > 0 && (!source.is_char_boundary(prefix) || !formatted.is_char_boundary(prefix)) {
        prefix -= 1;
    }
    let max_suffix = source.len().min(formatted.len()).saturating_sub(prefix);
    let mut suffix = source
        .bytes()
        .rev()
        .zip(formatted.bytes().rev())
        .take(max_suffix)
        .take_while(|(left, right)| left == right)
        .count();
    while suffix > 0
        && (!source.is_char_boundary(source.len() - suffix)
            || !formatted.is_char_boundary(formatted.len() - suffix))
    {
        suffix -= 1;
    }
    Some(vec![TextEdit {
        range: byte_range_to_lsp(source, prefix..source.len() - suffix),
        new_text: formatted[prefix..formatted.len() - suffix].to_string(),
    }])
}

#[derive(Clone)]
pub(super) struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    pub(super) fn new(source: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(source.match_indices('\n').map(|(offset, _)| offset + 1));
        Self { starts }
    }

    pub(super) fn byte_offset_to_position(&self, source: &str, offset: usize) -> Position {
        let mut offset = offset.min(source.len());
        while !source.is_char_boundary(offset) {
            offset -= 1;
        }
        let line_index = self
            .starts
            .partition_point(|start| *start <= offset)
            .saturating_sub(1);
        let line_start = self.starts[line_index];
        let character = source[line_start..offset]
            .chars()
            .filter(|character| *character != '\n')
            .map(char::len_utf16)
            .sum::<usize>() as u32;
        Position::new(line_index as u32, character)
    }

    pub(super) fn position_to_byte_offset(&self, source: &str, position: Position) -> usize {
        let Some(line_start) = self.starts.get(position.line as usize).copied() else {
            return source.len();
        };
        let line_end = self
            .starts
            .get(position.line as usize + 1)
            .map_or(source.len(), |next_start| next_start.saturating_sub(1));
        let mut utf16_column = 0;
        for (relative_offset, character) in source[line_start..line_end].char_indices() {
            if utf16_column >= position.character {
                return line_start + relative_offset;
            }
            utf16_column += character.len_utf16() as u32;
        }
        line_end
    }
}

pub(super) fn byte_range_to_lsp_with_index(
    source: &str,
    range: std::ops::Range<usize>,
    index: &LineIndex,
) -> Range {
    Range::new(
        index.byte_offset_to_position(source, range.start),
        index.byte_offset_to_position(source, range.end),
    )
}

pub(super) fn byte_offset_to_position(source: &str, offset: usize) -> Position {
    LineIndex::new(source).byte_offset_to_position(source, offset)
}

pub(super) fn position_to_byte_offset(source: &str, position: Position) -> usize {
    LineIndex::new(source).position_to_byte_offset(source, position)
}

pub(super) fn position_to_byte_offset_checked(source: &str, position: Position) -> Option<usize> {
    let index = LineIndex::new(source);
    let offset = index.position_to_byte_offset(source, position);
    (index.byte_offset_to_position(source, offset) == position).then_some(offset)
}
