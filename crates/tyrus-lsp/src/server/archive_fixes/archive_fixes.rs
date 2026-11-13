use super::*;

const ARCHIVE_EXTENSIONS: [&str; 4] = [".tar.gz", ".tgz", ".tar", ".zip"];

/// The destination with its extension lowercased, when lowercasing is all it
/// takes to reach an extension `pack` accepts. Anything else, `.jar` or no
/// extension at all, needs a format choice the server cannot make.
fn lowercased_destination(literal: &str) -> Option<(String, &'static str)> {
    let lower = literal.to_ascii_lowercase();
    let content = lower.trim_end_matches(['"', '\'']);
    let extension = ARCHIVE_EXTENSIONS
        .into_iter()
        .find(|extension| content.ends_with(extension))?;
    let start = literal.len() - (lower.len() - content.len()) - extension.len();
    let mut fixed = literal.to_string();
    fixed.replace_range(start..start + extension.len(), extension);
    (fixed != literal).then_some((fixed, extension))
}

pub(super) fn push_archive_extension_code_action(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
) {
    let start = position_to_byte_offset(source, diagnostic.range.start);
    let end = position_to_byte_offset(source, diagnostic.range.end);
    let Some(literal) = source.get(start..end) else {
        return;
    };
    let Some((fixed, extension)) = lowercased_destination(literal) else {
        return;
    };
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Write the extension as `{extension}`"),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![TextEdit {
                    range: diagnostic.range,
                    new_text: fixed,
                }],
            )])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }));
}

/// The byte range of the whole `level: <value>` property around the value at
/// `value_start..value_end`, with the comma that separates it from a
/// neighbour, so removing it leaves a well-formed object literal.
fn level_property_range(
    source: &str,
    value_start: usize,
    value_end: usize,
) -> Option<(usize, usize)> {
    let before = source.get(..value_start)?;
    let colon = before.trim_end().strip_suffix(':')?;
    let key = colon.trim_end().strip_suffix("level")?;
    let key_start = key.len();
    let after = source.get(value_end..)?;
    let trimmed = after.trim_start();
    if let Some(rest) = trimmed.strip_prefix(',') {
        let next = rest.len() - rest.trim_start().len();
        return Some((key_start, value_end + (after.len() - rest.len()) + next));
    }
    match key.trim_end().strip_suffix(',') {
        Some(comma) => Some((comma.len(), value_end)),
        None => Some((key_start, value_end)),
    }
}

pub(super) fn push_remove_tar_level_code_action(
    actions: &mut Vec<CodeActionOrCommand>,
    uri: &Url,
    source: &str,
    diagnostic: &Diagnostic,
) {
    if !diagnostic.message.contains("`.tar`") {
        return;
    }
    let value_start = position_to_byte_offset(source, diagnostic.range.start);
    let value_end = position_to_byte_offset(source, diagnostic.range.end);
    let Some((start, end)) = level_property_range(source, value_start, value_end) else {
        return;
    };
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: "Remove `level`".to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![TextEdit {
                    range: byte_range_to_lsp(source, start..end),
                    new_text: String::new(),
                }],
            )])),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(true),
        ..CodeAction::default()
    }));
}

#[cfg(test)]
mod tests {
    use super::{level_property_range, lowercased_destination};

    #[test]
    fn only_a_case_change_is_offered_for_the_extension() {
        assert_eq!(
            lowercased_destination("\"./dist/App.ZIP\""),
            Some(("\"./dist/App.zip\"".to_string(), ".zip"))
        );
        assert_eq!(
            lowercased_destination("\"Release.Tar.GZ\""),
            Some(("\"Release.tar.gz\"".to_string(), ".tar.gz"))
        );
        assert_eq!(lowercased_destination("\"app.jar\""), None);
        assert_eq!(lowercased_destination("\"app.zip\""), None);
    }

    #[test]
    fn removing_level_keeps_the_object_well_formed() {
        let remove = |source: &str| {
            let start = source.find("1 }").or_else(|| source.find("1,")).unwrap();
            let (from, to) = level_property_range(source, start, start + 1).unwrap();
            format!("{}{}", &source[..from], &source[to..])
        };
        assert_eq!(
            remove("f({ level: 1, overwrite: true })"),
            "f({ overwrite: true })"
        );
        assert_eq!(
            remove("f({ overwrite: true, level: 1 })"),
            "f({ overwrite: true })"
        );
        assert_eq!(remove("f({ level: 1 })"), "f({  })");
    }
}
