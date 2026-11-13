use super::*;
use tyrus_core::lexer::Token;

pub(super) struct ExistingImport {
    pub source: String,
    pub specifiers: Vec<(String, String)>,
    pub range: std::ops::Range<usize>,
}

pub(super) fn scan_imports(source: &str) -> Vec<ExistingImport> {
    let tokens = tyrus_core::lexer::lex_spanned(source).unwrap_or_default();
    let mut imports = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        if !matches!(tokens[index].0, Token::Import) {
            index += 1;
            continue;
        }
        let start = tokens[index].1.start;
        let mut cursor = index + 1;
        if !matches!(tokens.get(cursor).map(|t| &t.0), Some(Token::LBrace)) {
            index = cursor;
            continue;
        }
        cursor += 1;
        let mut specifiers = Vec::new();
        loop {
            match tokens.get(cursor).map(|t| &t.0) {
                Some(Token::Ident(imported)) => {
                    let imported = imported.clone();
                    cursor += 1;
                    let local = if matches!(tokens.get(cursor).map(|t| &t.0), Some(Token::As)) {
                        cursor += 1;
                        match tokens.get(cursor).map(|t| &t.0) {
                            Some(Token::Ident(local)) => {
                                let local = local.clone();
                                cursor += 1;
                                local
                            }
                            _ => imported.clone(),
                        }
                    } else {
                        imported.clone()
                    };
                    specifiers.push((imported, local));
                }
                Some(Token::Comma) => cursor += 1,
                Some(Token::RBrace) => {
                    cursor += 1;
                    break;
                }
                _ => break,
            }
        }
        if !matches!(tokens.get(cursor).map(|t| &t.0), Some(Token::From)) {
            index = cursor;
            continue;
        }
        cursor += 1;
        let Some(Token::StringLit(module)) = tokens.get(cursor).map(|t| &t.0) else {
            index = cursor;
            continue;
        };
        let module = module.clone();
        let mut end = tokens[cursor].1.end;
        cursor += 1;
        if matches!(tokens.get(cursor).map(|t| &t.0), Some(Token::Semicolon)) {
            end = tokens[cursor].1.end;
            cursor += 1;
        }
        imports.push(ExistingImport {
            source: module,
            specifiers,
            range: start..end,
        });
        index = cursor;
    }
    imports
}

pub(super) fn imported_names(source: &str) -> std::collections::HashSet<String> {
    scan_imports(source)
        .into_iter()
        .flat_map(|import| import.specifiers.into_iter().map(|(_, local)| local))
        .collect()
}

pub(super) fn import_source_for_export(
    current_path: Option<&Path>,
    export_uri: &Url,
) -> Option<String> {
    if export_uri.scheme() == "tyrus" {
        return Some(export_uri.as_str().to_string());
    }
    let current_path = current_path?;
    let export_path = export_uri.to_file_path().ok()?;
    relative_module_source(current_path, &export_path)
}

pub(super) fn auto_import_edit(source: &str, module: &str, name: &str) -> Option<TextEdit> {
    auto_import_edit_with_default(source, module, name, false)
}

pub(super) fn auto_import_edit_with_default(
    source: &str,
    module: &str,
    name: &str,
    is_default: bool,
) -> Option<TextEdit> {
    if is_default {
        let declaration = format!("import {name} from \"{module}\"");
        if source.contains(&declaration) {
            return None;
        }
        let imports = scan_imports(source);
        let first_internal = imports
            .iter()
            .filter(|import| !import.source.starts_with("tyrus:"))
            .map(|import| import.range.start)
            .min();
        if let Some(start) = first_internal {
            return Some(TextEdit {
                range: Range::new(
                    byte_offset_to_position(source, start),
                    byte_offset_to_position(source, start),
                ),
                new_text: format!("{declaration};\n"),
            });
        }
        if let Some(mut end) = imports.iter().map(|import| import.range.end).max() {
            while end < source.len() && source.as_bytes()[end].is_ascii_whitespace() {
                end += 1;
            }
            return Some(TextEdit {
                range: Range::new(
                    byte_offset_to_position(source, end),
                    byte_offset_to_position(source, end),
                ),
                new_text: format!("\n\n{declaration};\n\n"),
            });
        }
        return Some(TextEdit {
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
            new_text: format!("{declaration};\n\n"),
        });
    }
    let imports = scan_imports(source);
    if imports.iter().any(|import| {
        import.source == module && import.specifiers.iter().any(|(_, local)| local == name)
    }) {
        return None;
    }
    let mut grouped = HashMap::<String, Vec<String>>::new();
    for import in &imports {
        for (imported, local) in &import.specifiers {
            let specifier = if imported == local {
                imported.clone()
            } else {
                format!("{imported} as {local}")
            };
            grouped
                .entry(import.source.clone())
                .or_default()
                .push(specifier);
        }
    }
    grouped
        .entry(module.to_string())
        .or_default()
        .push(name.to_string());

    for specifiers in grouped.values_mut() {
        specifiers.sort_by_key(|specifier| specifier.to_ascii_lowercase());
        specifiers.dedup();
    }
    let mut native = grouped
        .iter()
        .filter(|(source, _)| source.starts_with("tyrus:"))
        .collect::<Vec<_>>();
    native.sort_by_key(|(source, _)| source.trim_start_matches("tyrus:").to_ascii_lowercase());
    let mut internal = grouped
        .iter()
        .filter(|(source, _)| !source.starts_with("tyrus:"))
        .collect::<Vec<_>>();
    internal.sort_by_key(|(source, _)| internal_import_sort_key(source).to_ascii_lowercase());

    let render_group = |entries: &[(&String, &Vec<String>)]| {
        entries
            .iter()
            .map(|(source, specifiers)| {
                format!("import {{ {} }} from \"{source}\";", specifiers.join(", "))
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let mut groups = Vec::new();
    if !native.is_empty() {
        groups.push(render_group(&native));
    }
    if !internal.is_empty() {
        groups.push(render_group(&internal));
    }
    let new_text = format!("{}\n\n", groups.join("\n\n"));

    let (start, mut end) = match (
        imports.iter().map(|import| import.range.start).min(),
        imports.iter().map(|import| import.range.end).max(),
    ) {
        (Some(start), Some(end)) => (start, end),
        _ => (0, 0),
    };
    while end < source.len() && source.as_bytes()[end].is_ascii_whitespace() {
        end += 1;
    }
    Some(TextEdit {
        range: Range::new(
            byte_offset_to_position(source, start),
            byte_offset_to_position(source, end),
        ),
        new_text,
    })
}

pub(super) fn internal_import_sort_key(source: &str) -> &str {
    let mut normalized = source;
    while let Some(rest) = normalized
        .strip_prefix("../")
        .or_else(|| normalized.strip_prefix("./"))
    {
        normalized = rest;
    }
    normalized
}
