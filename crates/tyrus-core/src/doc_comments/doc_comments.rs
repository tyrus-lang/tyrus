use crate::collections::HashMap;
use std::ops::Range;

type ParsedDocLines = (
    String,
    String,
    HashMap<String, String>,
    Option<String>,
    Option<String>,
    Vec<String>,
);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocComment {
    pub range: Range<usize>,
    pub summary: String,
    pub description: String,
    pub params: HashMap<String, String>,
    pub returns: Option<String>,
    pub deprecated: Option<String>,
    pub examples: Vec<String>,
}

impl DocComment {
    pub fn to_markdown(&self) -> String {
        let mut parts = Vec::new();
        if !self.summary.is_empty() {
            parts.push(self.summary.clone());
        }
        if !self.description.is_empty() && self.description != self.summary {
            parts.push(self.description.clone());
        }
        for (name, description) in &self.params {
            parts.push(format!("@param {name}, {description}"));
        }
        if let Some(returns) = &self.returns {
            parts.push(format!("@returns {returns}"));
        }
        if let Some(message) = &self.deprecated {
            parts.push(format!("@deprecated {message}"));
        }
        for example in &self.examples {
            parts.push(format!("```tyrus\n{example}\n```"));
        }
        parts.join("\n\n")
    }
}

pub fn extract_doc_comments(source: &str) -> Vec<DocComment> {
    let mut docs = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0;

    while index < source.len() {
        if let Some(next) = skip_string_literal(source, index) {
            index = next;
            continue;
        }

        if index + 2 < source.len()
            && bytes[index] == b'/'
            && bytes[index + 1] == b'*'
            && bytes[index + 2] == b'*'
        {
            let start = index;
            index += 3;
            while index + 1 < source.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                index += 1;
            }
            if index + 1 < source.len() {
                index += 2;
            }
            let raw = &source[start..index];
            docs.push(parse_doc_comment(start..index, raw));
            continue;
        }

        if index + 1 < source.len() && bytes[index] == b'/' && bytes[index + 1] == b'/' {
            index += 2;
            while index < source.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }

        if index + 1 < source.len() && bytes[index] == b'/' && bytes[index + 1] == b'*' {
            index += 2;
            while index + 1 < source.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                index += 1;
            }
            if index + 1 < source.len() {
                index += 2;
            }
            continue;
        }

        index += 1;
    }

    docs
}

pub fn doc_comment_before_decl<'a>(
    source: &str,
    decl_start: usize,
    docs: &'a [DocComment],
) -> Option<&'a DocComment> {
    let anchor = declaration_doc_anchor(source, decl_start);
    let gap = source.get(..anchor)?.trim_end();
    docs.iter()
        .filter(|doc| doc.range.end <= anchor)
        .max_by_key(|doc| doc.range.end)
        .filter(|doc| {
            source
                .get(doc.range.clone())
                .is_some_and(|text| gap.ends_with(text))
        })
}

pub fn declaration_doc_anchor(source: &str, ident_start: usize) -> usize {
    const KEYWORDS: &[&str] = &["export", "const", "let", "interface", "type", "function"];
    let mut anchor = ident_start.min(source.len());
    while anchor > 0 && !source.is_char_boundary(anchor) {
        anchor -= 1;
    }
    loop {
        while anchor > 0 && source.as_bytes()[anchor - 1].is_ascii_whitespace() {
            anchor -= 1;
        }
        let mut moved = false;
        for keyword in KEYWORDS {
            if anchor < keyword.len() {
                continue;
            }
            let Some(candidate) = source.get(anchor - keyword.len()..anchor) else {
                continue;
            };
            if candidate != *keyword {
                continue;
            }
            let before = anchor - keyword.len();
            if before == 0 || !is_ident_continue(source.as_bytes()[before - 1]) {
                anchor = before;
                moved = true;
                break;
            }
        }
        if !moved {
            break;
        }
    }
    anchor
}

fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn parse_doc_comment(range: Range<usize>, raw: &str) -> DocComment {
    let inner = raw
        .strip_prefix("/**")
        .and_then(|value| value.strip_suffix("*/"))
        .unwrap_or(raw);
    let lines = inner.lines().map(strip_doc_margin).collect::<Vec<_>>();
    let (summary, description, params, returns, deprecated, examples) = parse_doc_lines(&lines);
    DocComment {
        range,
        summary,
        description,
        params,
        returns,
        deprecated,
        examples,
    }
}

fn strip_doc_margin(line: &str) -> String {
    let trimmed = line.trim_start();
    let without_star = trimmed.strip_prefix('*').unwrap_or(trimmed).trim_start();
    without_star.to_string()
}

fn parse_doc_lines(lines: &[String]) -> ParsedDocLines {
    let mut prose = Vec::new();
    let mut params = HashMap::default();
    let mut returns = None;
    let mut deprecated = None;
    let mut examples = Vec::new();

    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim();
        if let Some(rest) = line.strip_prefix("@param ") {
            let (name, description) = parse_param_tag(rest, lines, &mut index);
            if !name.is_empty() {
                params.insert(name, description);
            }
            index += 1;
            continue;
        }
        if let Some(rest) = line
            .strip_prefix("@returns ")
            .or_else(|| line.strip_prefix("@return "))
        {
            returns = Some(rest.trim().to_string());
            index += 1;
            continue;
        }
        if line == "@returns" || line == "@return" {
            returns = Some(String::new());
            index += 1;
            continue;
        }
        if let Some(rest) = line.strip_prefix("@deprecated") {
            let message = rest.trim();
            deprecated = Some(if message.is_empty() {
                "deprecated".to_string()
            } else {
                message.to_string()
            });
            index += 1;
            continue;
        }
        if line == "@example" {
            let mut example = Vec::new();
            index += 1;
            while index < lines.len() && !lines[index].trim().starts_with('@') {
                let value = lines[index].trim();
                if !value.is_empty() {
                    example.push(value.to_string());
                }
                index += 1;
            }
            if !example.is_empty() {
                examples.push(example.join("\n"));
            }
            continue;
        }
        if line.starts_with('@') {
            index += 1;
            continue;
        }
        if !line.is_empty() {
            prose.push(line.to_string());
        }
        index += 1;
    }

    let description = prose.join("\n");
    let summary = prose.first().cloned().unwrap_or_default();
    (summary, description, params, returns, deprecated, examples)
}

fn parse_param_tag(first_line: &str, lines: &[String], index: &mut usize) -> (String, String) {
    let mut parts = first_line.split_whitespace();
    let name = parts.next().unwrap_or("").trim_start_matches('{');
    let name = name
        .rsplit_once('}')
        .map_or(name, |(_, value)| value)
        .trim()
        .to_string();
    let mut description = parts.collect::<Vec<_>>().join(" ");
    let mut next = *index + 1;
    while next < lines.len() {
        let line = lines[next].trim();
        if line.starts_with('@') {
            break;
        }
        if !line.is_empty() {
            if !description.is_empty() {
                description.push(' ');
            }
            description.push_str(line);
        }
        next += 1;
    }
    *index = next.saturating_sub(1);
    (name, description)
}

fn skip_string_literal(source: &str, start: usize) -> Option<usize> {
    let quote = source.as_bytes().get(start)?;
    if !matches!(quote, b'"' | b'\'' | b'`') {
        return None;
    }
    let mut index = start + 1;
    let bytes = source.as_bytes();
    while index < source.len() {
        let byte = bytes[index];
        if byte == b'\\' {
            index = (index + 2).min(source.len());
            continue;
        }
        if byte == *quote {
            return Some(index + 1);
        }
        index += 1;
    }
    Some(source.len())
}

#[cfg(test)]
#[path = "../../tests/unit/doc_comments.rs"]
mod tests;
