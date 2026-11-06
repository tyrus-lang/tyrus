use super::*;

#[test]
pub(super) fn quick_fix_code_lenses_prefer_is_preferred_and_collapse_duplicate_titles() {
    let shared = Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: 0,
            character: 5,
        },
    };
    let actions = vec![
        synthetic_quickfix_action("second option", shared, false),
        synthetic_quickfix_action("preferred fix", shared, true),
    ];
    let lenses = quick_fix_code_lenses(&actions);
    assert_eq!(lenses.len(), 2);
    assert_eq!(
        lenses[0].command.as_ref().expect("command").title,
        "preferred fix"
    );
    assert_eq!(
        lenses[1].command.as_ref().expect("command").title,
        "second option"
    );

    let duplicated = vec![
        synthetic_quickfix_action("same fix", shared, false),
        synthetic_quickfix_action("same fix", shared, false),
    ];
    assert_eq!(quick_fix_code_lenses(&duplicated).len(), 1);
}

#[test]
pub(super) fn quick_fix_dispatch_deduplicates_equivalent_text_edits_stably() {
    let range = Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: 0,
            character: 5,
        },
    };
    let mut actions = vec![
        synthetic_quickfix_action("same fix", range, true),
        synthetic_quickfix_action("same fix", range, true),
        synthetic_quickfix_action("other fix", range, false),
    ];
    crate::server::deduplicate_equivalent_quick_fixes(&mut actions);
    assert_eq!(actions.len(), 2);
    let titles = actions
        .iter()
        .map(|action| match action {
            CodeActionOrCommand::CodeAction(action) => action.title.as_str(),
            CodeActionOrCommand::Command(_) => panic!("text fix should be a code action"),
        })
        .collect::<Vec<_>>();
    assert_eq!(titles, ["same fix", "other fix"]);
}

#[test]
pub(super) fn quick_fix_code_lenses_serve_the_real_dispatch_with_multiple_fixes() {
    let source = "const main = (): void => {\n    let x = 10;\n};\n";
    let lints = tyrus_lint::lint_rules_source(source, &tyrus_lint::LintConfig::default());
    let lint = lints
        .iter()
        .find(|lint| lint.code == "prefer-const")
        .expect("prefer-const should fire for the never-reassigned binding");
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let lsp_diagnostic = lint_diagnostic_to_lsp(source, lint);

    let current_path = std::path::PathBuf::from("/workspace/src/main.tyr");
    let actions = quick_fixes_for_diagnostics(
        &uri,
        source,
        &current_path,
        &[],
        &lints,
        std::slice::from_ref(&lsp_diagnostic),
    );
    let lenses = quick_fix_code_lenses(&actions);
    assert_eq!(lenses.len(), 1);
    assert_eq!(lenses[0].range, lsp_diagnostic.range);
    assert_eq!(
        lenses[0].command.as_ref().expect("command").title,
        lint.fix.as_ref().expect("prefer-const is fixable").message
    );

    let unknown = Diagnostic {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 1,
            },
        },
        message: "something no quick fix covers".to_string(),
        ..Diagnostic::default()
    };
    let actions = quick_fixes_for_diagnostics(&uri, source, &current_path, &[], &lints, &[unknown]);
    assert!(quick_fix_code_lenses(&actions).is_empty());
}

#[test]
pub(super) fn an_unimported_native_type_offers_its_import_as_a_quick_fix() {
    let source = "const main = (): void => {\n    try {\n        throw { code: \"ERR_OS_TEST\", message: \"test\" };\n    } catch (error: OsError) {\n        console.log(error.code);\n    }\n};\n";
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let current_path = std::path::PathBuf::from("/workspace/src/main.tyr");
    let diagnostic = Diagnostic {
        range: Range {
            start: Position {
                line: 3,
                character: 20,
            },
            end: Position {
                line: 3,
                character: 27,
            },
        },
        code: Some(tower_lsp::lsp_types::NumberOrString::String(
            "E0412".to_string(),
        )),
        message: "cannot find type `OsError` in this scope".to_string(),
        ..Diagnostic::default()
    };

    let actions = quick_fixes_for_diagnostics(
        &uri,
        source,
        &current_path,
        &[],
        &[],
        std::slice::from_ref(&diagnostic),
    );
    let action = actions
        .iter()
        .find_map(|action| match action {
            CodeActionOrCommand::CodeAction(action) => Some(action),
            _ => None,
        })
        .expect("the missing native type offers its import");
    assert_eq!(action.title, "Import `OsError` from `tyrus:os`");
    let edit = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .and_then(|changes| changes.get(&uri))
        .expect("the fix edits this file");
    assert_eq!(edit.len(), 1);
    assert!(
        edit[0]
            .new_text
            .contains("import { OsError } from \"tyrus:os\";"),
        "unexpected edit: {}",
        edit[0].new_text
    );

    let unknown = Diagnostic {
        message: "cannot find type `NotAnyModulesType` in this scope".to_string(),
        ..diagnostic
    };
    let actions = quick_fixes_for_diagnostics(&uri, source, &current_path, &[], &[], &[unknown]);
    assert!(actions.is_empty());
}

#[test]
pub(super) fn raw_http_status_quick_fix_uses_the_imported_enum_alias() {
    let source = "import { HttpReply, HttpStatus as Status } from \"tyrus:http\";\nconst main = (): void => {\n    const reply: HttpReply = getReply();\n    reply.status(404).send({ ok: false });\n};\n";
    let lints = tyrus_lint::lint_rules_source(source, &tyrus_lint::LintConfig::default());
    let lint = lints
        .iter()
        .find(|lint| lint.code == "http-no-raw-status-code")
        .expect("the raw HTTP status should be reported");
    assert_eq!(
        lint.fix.as_ref().map(|fix| fix.replacement.as_str()),
        Some("HttpStatus.NotFound")
    );
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let current_path = std::path::PathBuf::from("/workspace/src/main.tyr");
    let diagnostic = lint_diagnostic_to_lsp(source, lint);
    let actions = quick_fixes_for_diagnostics(
        &uri,
        source,
        &current_path,
        &[],
        &lints,
        std::slice::from_ref(&diagnostic),
    );
    let action = actions
        .iter()
        .find_map(|action| match action {
            CodeActionOrCommand::CodeAction(action) => Some(action),
            _ => None,
        })
        .expect("the imported enum makes this replacement deterministic");
    assert_eq!(action.title, "Use `Status.NotFound`");
    let edits = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .and_then(|changes| changes.get(&uri))
        .expect("the fix edits this document");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "Status.NotFound");

    let source = "import { HttpReply } from \"tyrus:http\";\nconst main = (): void => {\n    const reply: HttpReply = getReply();\n    reply.status(404).send({ ok: false });\n};\n";
    let lints = tyrus_lint::lint_rules_source(source, &tyrus_lint::LintConfig::default());
    let lint = lints
        .iter()
        .find(|lint| lint.code == "http-no-raw-status-code")
        .expect("the raw HTTP status should be reported");
    let diagnostic = lint_diagnostic_to_lsp(source, lint);
    let actions = quick_fixes_for_diagnostics(
        &uri,
        source,
        &current_path,
        &[],
        &lints,
        std::slice::from_ref(&diagnostic),
    );
    let action = actions
        .iter()
        .find_map(|action| match action {
            CodeActionOrCommand::CodeAction(action) => Some(action),
            _ => None,
        })
        .expect("the fix adds the missing enum import");
    let edits = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .and_then(|changes| changes.get(&uri))
        .expect("the fix edits this document");
    assert_eq!(edits.len(), 2);
    assert!(
        edits
            .iter()
            .any(|edit| edit.new_text == "HttpStatus.NotFound")
    );
    assert!(edits.iter().any(|edit| edit.new_text == ", HttpStatus"));
}

#[test]
pub(super) fn callback_parameter_type_help_supports_quick_fix_replacement() {
    let source = "const main = (): void => {\n\
                  const values: i32[] = [1, 2, 3];\n\
                  const filtered: i32[] = values.filter((value: i32, index: i64) => {\n\
                      return index >= 1;\n\
                  });\n\
                  };\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "E0308"
                && diagnostic
                    .message
                    .contains("callback parameter `index` of `filter`")
        })
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let lsp = diagnostic_to_lsp(&uri, source, diagnostic);
    let replacement = lsp
        .data
        .as_ref()
        .and_then(|data| data.get("help"))
        .and_then(|help| help.as_array())
        .into_iter()
        .flatten()
        .filter_map(|help| help.as_str())
        .find_map(crate::server::diagnostic_change_callback_parameter_type_help)
        .expect("callback parameter type help should be parsed");

    let start = position_to_byte_offset(source, lsp.range.start);
    let end = position_to_byte_offset(source, lsp.range.end);
    let mut fixed = source.to_string();
    fixed.replace_range(start..end, &replacement);
    assert!(fixed.contains("index: usize"));
}

#[test]
pub(super) fn async_return_type_help_supports_quick_fix_replacement() {
    let source = "async function load(): i32 { return 42; }\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3003")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let lsp = diagnostic_to_lsp(&uri, source, diagnostic);
    let replacement = lsp
        .data
        .as_ref()
        .and_then(|data| data.get("help"))
        .and_then(|help| help.as_array())
        .into_iter()
        .flatten()
        .filter_map(|help| help.as_str())
        .find_map(crate::server::diagnostic_change_async_return_type_help)
        .expect("async return type help should be parsed");

    let start = position_to_byte_offset(source, lsp.range.start);
    let end = position_to_byte_offset(source, lsp.range.end);
    let mut fixed = source.to_string();
    fixed.replace_range(start..end, &replacement);
    assert!(fixed.contains("async function load(): Promise<i32>"));
}

#[test]
pub(super) fn moved_value_quick_fix_offers_borrow_and_annotated_declaration() {
    let source = "const main = (): void => {\n    const a = \"a\";\n    const b = a;\n\n    console.log(a);\n    console.log(b);\n};\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0382")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let lsp = diagnostic_to_lsp(&uri, source, diagnostic);
    let actions = quick_fixes_for_diagnostics(
        &uri,
        source,
        std::path::Path::new("/workspace/src/main.tyr"),
        &[],
        &[],
        &[lsp],
    );
    assert_eq!(actions.len(), 2, "actions: {actions:#?}");

    let apply = |action: &CodeAction| -> String {
        let edits = action
            .edit
            .as_ref()
            .and_then(|edit| edit.changes.as_ref())
            .and_then(|changes| changes.get(&uri))
            .expect("action should carry workspace edits");
        let mut fixed = source.to_string();
        for edit in edits {
            let start = position_to_byte_offset(source, edit.range.start);
            let end = position_to_byte_offset(source, edit.range.end);
            fixed.replace_range(start..end, &edit.new_text);
        }
        fixed
    };

    let CodeActionOrCommand::CodeAction(borrow) = &actions[0] else {
        panic!("expected a code action, got {:#?}", actions[0]);
    };
    assert_eq!(borrow.title, "Borrow `a` instead of moving it");
    assert_eq!(borrow.is_preferred, Some(true));
    let borrow_edits = borrow
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .and_then(|changes| changes.get(&uri))
        .expect("borrow action should carry workspace edits");
    let move_site_start = source.find("= a;").unwrap() + 2;
    assert_eq!(
        borrow_edits[0].range,
        byte_range_to_lsp(source, move_site_start..move_site_start + 1)
    );
    assert_eq!(
        borrow_edits[0].new_text, "&a",
        "the borrow edit must anchor on `a`, not on another token"
    );
    assert!(
        apply(borrow).contains("const b = &a;"),
        "fixed: {}",
        apply(borrow)
    );

    let CodeActionOrCommand::CodeAction(annotated) = &actions[1] else {
        panic!("expected a code action, got {:#?}", actions[1]);
    };
    assert_eq!(
        annotated.title,
        "Replace the declaration with `const b: &string = &a;`"
    );
    assert!(annotated.is_preferred.is_none());
    let replaced = apply(annotated);
    assert!(
        replaced.contains("\n    const b: &string = &a;\n"),
        "replaced: {replaced}"
    );
}

#[test]
pub(super) fn numeric_mismatch_quick_fix_inserts_the_cast_at_the_value_expression() {
    let source = "const main = (): void => {\n    let sum = 0;\n    const values = [0, 1, 2, 3];\n    for (let i = 0; i < values.length; i += 1) {\n        sum += values[i];\n        sum += i;\n    }\n    console.log(sum);\n};\n";
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "E0308" && diagnostic.message.contains("type mismatch for `sum`")
        })
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let lsp = diagnostic_to_lsp(&uri, source, diagnostic);
    assert_eq!(
        lsp.message,
        "type mismatch for `sum`: expected `i32`, found `usize`"
    );
    let actions = quick_fixes_for_diagnostics(
        &uri,
        source,
        std::path::Path::new("/workspace/src/main.tyr"),
        &[],
        &[],
        std::slice::from_ref(&lsp),
    );
    assert_eq!(actions.len(), 1, "actions: {actions:#?}");
    let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
        panic!("expected a code action, got {:#?}", actions[0]);
    };
    assert_eq!(action.title, "Cast the expression to `i32`");
    assert_eq!(action.is_preferred, Some(true));
    let edits = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .and_then(|changes| changes.get(&uri))
        .expect("the action should carry an edit for this document");
    assert_eq!(edits.len(), 1);
    let value_start = source.rfind("sum += i;").unwrap() + "sum += ".len();
    assert_eq!(
        edits[0].range,
        byte_range_to_lsp(source, value_start..value_start + 1),
        "the cast must anchor on the value expression, not on the whole statement"
    );
    assert_eq!(edits[0].new_text, "i as i32");

    let lenses = quick_fix_code_lenses(&actions);
    assert_eq!(lenses.len(), 1);
    assert_eq!(lenses[0].range, lsp.range);
    assert_eq!(
        lenses[0]
            .command
            .as_ref()
            .expect("lens carries its command")
            .title,
        "Cast the expression to `i32`"
    );
}

#[test]
pub(super) fn numeric_mismatch_quick_fix_wraps_compound_value_expressions_in_parentheses() {
    let source = "const main = (): void => {\n    let total: u32 = 0;\n    let offset: i64 = 1;\n    total = offset + 1;\n    console.log(total);\n};\n";
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "E0308" && diagnostic.message.contains("type mismatch for `total`")
        })
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let lsp = diagnostic_to_lsp(&uri, source, diagnostic);
    let actions = quick_fixes_for_diagnostics(
        &uri,
        source,
        std::path::Path::new("/workspace/src/main.tyr"),
        &[],
        &[],
        std::slice::from_ref(&lsp),
    );
    assert_eq!(actions.len(), 1, "actions: {actions:#?}");
    let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
        panic!("expected a code action, got {:#?}", actions[0]);
    };
    assert_eq!(action.title, "Cast the expression to `u32`");
    let edits = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .and_then(|changes| changes.get(&uri))
        .expect("the action should carry an edit for this document");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "(offset + 1) as u32");
    let start = position_to_byte_offset(source, edits[0].range.start);
    let end = position_to_byte_offset(source, edits[0].range.end);
    assert_eq!(&source[start..end], "offset + 1");
}

#[test]
pub(super) fn non_numeric_mismatch_offers_no_cast_quick_fix() {
    let source = "const main = (): void => {\n    const result: i32 = \"not a number\";\n    console.log(result);\n};\n";
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0308")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let lsp = diagnostic_to_lsp(&uri, source, diagnostic);
    let actions = quick_fixes_for_diagnostics(
        &uri,
        source,
        std::path::Path::new("/workspace/src/main.tyr"),
        &[],
        &[],
        std::slice::from_ref(&lsp),
    );
    assert!(actions.is_empty(), "actions: {actions:#?}");
}

#[test]
pub(super) fn validator_quick_fix_removes_full_decorator() {
    let source = "interface User { @email() @email(\"invalid e-mail\") email: string; }\n";
    let start = source.rfind("@email").unwrap();
    let range = Range::new(
        byte_offset_to_position(source, start),
        byte_offset_to_position(source, start + "@email".len()),
    );
    let edit = crate::server::remove_validator_decorator_edit(source, range)
        .expect("redundant validator should produce removal edit");
    let mut fixed = source.to_string();
    let edit_start = position_to_byte_offset(source, edit.range.start);
    let edit_end = position_to_byte_offset(source, edit.range.end);
    fixed.replace_range(edit_start..edit_end, &edit.new_text);
    assert_eq!(fixed, "interface User { @email() email: string; }\n");
}

#[test]
pub(super) fn validator_quick_fix_replaces_obvious_invalid_target() {
    let source = "interface User { @min(3) name: string; }\n";
    let start = source.find("@min").unwrap();
    let range = Range::new(
        byte_offset_to_position(source, start),
        byte_offset_to_position(source, start + "@min".len()),
    );
    let replacement =
        crate::server::invalid_target_validator_replacement("`@min` cannot be used with `string`")
            .expect("string min has minLength replacement");
    assert_eq!(replacement, ("min".to_string(), "minLength".to_string()));
    let edit =
        crate::server::replace_validator_name_edit(source, range, &replacement.0, &replacement.1)
            .expect("validator name should be replaced");
    let mut fixed = source.to_string();
    let edit_start = position_to_byte_offset(source, edit.range.start);
    let edit_end = position_to_byte_offset(source, edit.range.end);
    fixed.replace_range(edit_start..edit_end, &edit.new_text);
    assert_eq!(fixed, "interface User { @minLength(3) name: string; }\n");
}

#[test]
pub(super) fn validator_quick_fix_adds_custom_message() {
    let source = "interface User { @minLength(3) name: string; @email email: string; }\n";
    let min_start = source.find("@minLength").unwrap();
    let min_range = Range::new(
        byte_offset_to_position(source, min_start),
        byte_offset_to_position(source, min_start + "@minLength".len()),
    );
    let email_start = source.find("@email").unwrap();
    let email_range = Range::new(
        byte_offset_to_position(source, email_start),
        byte_offset_to_position(source, email_start + "@email".len()),
    );

    let min_edit = crate::server::add_validator_message_edit(source, min_range)
        .expect("validator with args should receive appended message");
    let email_edit = crate::server::add_validator_message_edit(source, email_range)
        .expect("validator without args should receive message args");

    let mut fixed = source.to_string();
    let email_insert = position_to_byte_offset(source, email_edit.range.start);
    fixed.replace_range(email_insert..email_insert, &email_edit.new_text);
    let min_insert = position_to_byte_offset(source, min_edit.range.start);
    fixed.replace_range(min_insert..min_insert, &min_edit.new_text);

    assert_eq!(
        fixed,
        "interface User { @minLength(3, \"Validation failed\") name: string; @email(\"Validation failed\") email: string; }\n"
    );
}

#[test]
pub(super) fn await_outside_async_quick_fix_adds_async_to_enclosing_function() {
    let source = "function load(value: Promise<i32>): Promise<i32> { return await value; }\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3000")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    let mut actions = Vec::new();
    crate::server::push_add_async_code_action(&mut actions, &uri, source, &lsp_diagnostic);
    let [CodeActionOrCommand::CodeAction(action)] = actions.as_slice() else {
        panic!("expected one code action");
    };
    assert_eq!(action.title, "Add `async` to enclosing function");
    let edit = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .and_then(|changes| changes.get(&uri))
        .and_then(|edits| edits.first())
        .expect("expected edit");
    assert_eq!(edit.new_text, "async ");
    assert_eq!(edit.range, byte_range_to_lsp(source, 0..0));
}

#[test]
pub(super) fn ambiguous_result_return_offers_ok_and_err_quick_fixes() {
    let source = "function choose(): Result<string, string> { return \"value\"; }\n";
    let uri = Url::parse("file:///test.tyr").unwrap();
    let value_start = source.find("\"value\"").unwrap();
    let diagnostic = Diagnostic::new_simple(
        byte_range_to_lsp(source, value_start..value_start + "\"value\"".len()),
        "ambiguous Result return".to_string(),
    );
    let mut actions = Vec::new();
    crate::server::push_result_variant_code_actions(&mut actions, &uri, source, &diagnostic);
    assert_eq!(actions.len(), 2);
    let titles: Vec<&str> = actions
        .iter()
        .filter_map(|action| match action {
            CodeActionOrCommand::CodeAction(action) => Some(action.title.as_str()),
            CodeActionOrCommand::Command(_) => None,
        })
        .collect();
    assert!(titles.contains(&"Return this value as `Ok(...)`"));
    assert!(titles.contains(&"Return this value as `Err(...)`"));
}

#[test]
pub(super) fn unambiguous_result_return_offers_only_the_matching_constructor() {
    let source = "function choose(): Result<string, boolean> { return \"value\"; }\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3074")
        .expect("the unwrapped success value should report E3074");
    let uri = Url::parse("file:///workspace/main.tyr").unwrap();
    let current_path = std::path::PathBuf::from("/workspace/main.tyr");
    let published = diagnostic_to_lsp(&uri, source, diagnostic);
    let actions = quick_fixes_for_diagnostics(
        &uri,
        source,
        &current_path,
        &[],
        &[],
        std::slice::from_ref(&published),
    );
    let titles = actions
        .iter()
        .filter_map(|action| match action {
            CodeActionOrCommand::CodeAction(action) => Some(action.title.as_str()),
            CodeActionOrCommand::Command(_) => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(titles, ["Return this value as `Ok(...)`"]);
}

#[test]
pub(super) fn result_return_constructor_quick_fix_wraps_the_precise_payload_range() {
    let source = "function choose(): Result<string, boolean> { return \"value\"; }\n";
    let uri = Url::parse("file:///test.tyr").unwrap();
    let value_start = source.find("\"value\"").unwrap();
    let diagnostic = Diagnostic::new_simple(
        byte_range_to_lsp(source, value_start..value_start + "\"value\"".len()),
        "Result return requires Ok".to_string(),
    );
    let mut actions = Vec::new();
    crate::server::push_result_constructor_code_action(
        &mut actions,
        &uri,
        source,
        &diagnostic,
        "Ok",
    );
    let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
        panic!("expected a code action");
    };
    let edits = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .and_then(|changes| changes.get(&uri))
        .expect("expected edits");
    assert_eq!(edits[0].new_text, "Ok(");
    assert_eq!(
        edits[0].range,
        byte_range_to_lsp(source, value_start..value_start)
    );
    assert_eq!(edits[1].new_text, ")");
    assert_eq!(
        edits[1].range,
        byte_range_to_lsp(
            source,
            value_start + "\"value\"".len()..value_start + "\"value\"".len()
        )
    );
}

#[test]
pub(super) fn void_result_return_quick_fix_replaces_the_bare_return() {
    let source = "function done(): Result<void, string> { return; }\n";
    let uri = Url::parse("file:///test.tyr").unwrap();
    let return_start = source.find("return;").unwrap();
    let diagnostic = Diagnostic::new_simple(
        byte_range_to_lsp(source, return_start..return_start + "return;".len()),
        "Result return requires Ok".to_string(),
    );
    let mut actions = Vec::new();
    crate::server::push_result_void_ok_code_action(&mut actions, &uri, &diagnostic);
    let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
        panic!("expected a code action");
    };
    let edit = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .and_then(|changes| changes.get(&uri))
        .and_then(|edits| edits.first())
        .expect("expected edit");
    assert_eq!(
        edit.range,
        byte_range_to_lsp(source, return_start..return_start + "return;".len())
    );
    assert_eq!(edit.new_text, "return Ok();");
}

#[test]
pub(super) fn unhandled_validation_error_offers_catch_and_propagation_quick_fixes() {
    let source = "const accept = (@min(18) age: i32): void => {};\nconst run = (age: i32): void => {\n    accept(age);\n};\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3011")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    let mut actions = Vec::new();
    crate::server::push_unhandled_throw_code_actions(&mut actions, &uri, source, &lsp_diagnostic);
    assert_eq!(actions.len(), 2);
    let titles = actions
        .iter()
        .map(|action| match action {
            CodeActionOrCommand::CodeAction(action) => action.title.as_str(),
            _ => "",
        })
        .collect::<Vec<_>>();
    assert!(titles.contains(&"Wrap in try/catch for `ValidationError`"));
    assert!(titles.contains(&"Add `throws ValidationError` to enclosing function"));
}

#[test]
pub(super) fn unhandled_validation_error_in_main_only_offers_catch_quick_fix() {
    let source = "const accept = (@min(18) age: i32): void => {};\nconst main = (): void => {\n    accept(readAge());\n};\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3011")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    let mut actions = Vec::new();
    crate::server::push_unhandled_throw_code_actions(&mut actions, &uri, source, &lsp_diagnostic);
    assert_eq!(actions.len(), 1);
}

#[test]
pub(super) fn prefer_const_quick_fix_rewrites_let_to_const() {
    let source =
        "const main = (): void => {\n    let retries: i32 = 3;\n    console.log(retries);\n};\n";
    let diagnostics = tyrus_lint::lint_source(source, &tyrus_lint::LintConfig::default());
    let lint = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "prefer-const")
        .unwrap_or_else(|| panic!("diagnostics: {diagnostics:#?}"));
    let fix = lint
        .fix
        .as_ref()
        .expect("expected safe `let` to `const` fix");
    assert_eq!(&source[fix.range.clone()], "let");
    assert_eq!(fix.replacement, "const");

    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp = lint_diagnostic_to_lsp(source, lint);
    let fix_index = crate::server::index_lint_fixes(&diagnostics);
    let matched = crate::server::matching_indexed_lint_fix(&fix_index, source, &lsp)
        .expect("expected the published diagnostic to match the lint fix");
    let mut actions = Vec::new();
    crate::server::push_lint_fix_code_action(&mut actions, &uri, source, &lsp, matched);
    let [CodeActionOrCommand::CodeAction(action)] = actions.as_slice() else {
        panic!("expected one code action");
    };
    assert_eq!(action.title, "replace `let` with `const`");
    assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
    assert_eq!(action.is_preferred, Some(true));
    assert!(
        action
            .diagnostics
            .as_ref()
            .is_some_and(|diagnostics| diagnostics.iter().any(|d| d == &lsp))
    );
    let edit = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .and_then(|changes| changes.get(&uri))
        .and_then(|edits| edits.first())
        .expect("expected edit");
    assert_eq!(edit.new_text, "const");
    assert_eq!(edit.range, byte_range_to_lsp(source, fix.range.clone()));
}

#[test]
pub(super) fn reassigned_let_offers_no_prefer_const_quick_fix() {
    let source = "const main = (): void => {\n    let counter: i32 = 0;\n    counter += 1;\n    console.log(counter);\n};\n";
    let diagnostics = tyrus_lint::lint_source(source, &tyrus_lint::LintConfig::default());
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "prefer-const"),
        "diagnostics: {diagnostics:#?}"
    );
}

#[test]
pub(super) fn lint_rule_without_fix_offers_no_quick_fix() {
    let source = "const main = (): void => {\n    if (true) {}\n};\n";
    let diagnostics = tyrus_lint::lint_source(source, &tyrus_lint::LintConfig::default());
    let lint = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "no-empty-block")
        .unwrap_or_else(|| panic!("diagnostics: {diagnostics:#?}"));
    let lsp = lint_diagnostic_to_lsp(source, lint);
    let fix_index = crate::server::index_lint_fixes(&diagnostics);
    assert!(crate::server::matching_indexed_lint_fix(&fix_index, source, &lsp).is_none());
}

#[test]
pub(super) fn optional_presence_comparison_offers_a_quick_fix() {
    let source = "const check = (value: string?): boolean => value === None;\n";
    let none_start = source.find("None").unwrap();
    let range = byte_range_to_lsp(source, none_start..none_start + 4);
    let edit = crate::server::optional_presence_comparison_edit(source, range)
        .expect("expected optional comparison quick fix");
    assert_eq!(edit.new_text, "!value");
    assert_eq!(
        edit.range,
        byte_range_to_lsp(
            source,
            source.find("value === None").unwrap()..source.find("None").unwrap() + 4
        )
    );

    let source = "const check = (value: string?): boolean => None !== value;\n";
    let none_start = source.find("None").unwrap();
    let range = byte_range_to_lsp(source, none_start..none_start + 4);
    let edit = crate::server::optional_presence_comparison_edit(source, range)
        .expect("expected reversed optional comparison quick fix");
    assert_eq!(edit.new_text, "value");
}

#[test]
pub(super) fn env_export_quick_fix_comments_every_raw_decorator() {
    use crate::server::comment_all_raw_decorators_action;
    let source = "@type(string)\nHOST=localhost\n\n# @type(u32)\n@port()\nexport PORT=3000\n";
    let uri = Url::parse("file:///project/.env").unwrap();
    let action =
        comment_all_raw_decorators_action(&uri, source, Vec::new()).expect("expected a quick fix");
    assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
    let edits = action
        .edit
        .and_then(|edit| edit.changes)
        .and_then(|mut changes| changes.remove(&uri))
        .expect("expected edits for the .env");
    assert!(edits.iter().all(|edit| edit.new_text == "# "));
    let lines: Vec<u32> = edits.iter().map(|edit| edit.range.start.line).collect();
    assert_eq!(edits.len(), 2);
    assert!(lines.contains(&0) && lines.contains(&4));
    assert!(!lines.contains(&3));
}

#[test]
pub(super) fn out_of_range_integer_literal_quick_fix_widens_the_annotation() {
    let source =
        "const main = (): void => {\n    const byte: u8 = 300;\n    console.log(byte);\n};\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0706")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    let mut actions = Vec::new();
    crate::server::push_widen_integer_annotation_code_action(
        &mut actions,
        &uri,
        source,
        &lsp_diagnostic,
    );
    let [CodeActionOrCommand::CodeAction(action)] = actions.as_slice() else {
        panic!("expected one code action");
    };
    assert_eq!(action.title, "Change type annotation to `u16`");
    let edit = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .and_then(|changes| changes.get(&uri))
        .and_then(|edits| edits.first())
        .expect("expected edit");
    assert_eq!(edit.new_text, "u16");
    let annotation = source.find("u8").unwrap();
    assert_eq!(
        edit.range,
        byte_range_to_lsp(source, annotation..annotation + 2)
    );
}

#[test]
pub(super) fn out_of_range_integer_literal_offers_no_quick_fix_without_an_annotation() {
    let source = "const main = (): void => {\n    let byte: u8 = 1;\n    byte = 300;\n    console.log(byte);\n};\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0706")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    let mut actions = Vec::new();
    crate::server::push_widen_integer_annotation_code_action(
        &mut actions,
        &uri,
        source,
        &lsp_diagnostic,
    );
    assert!(actions.is_empty());
}

#[test]
pub(super) fn awaiting_a_spawn_join_chain_offers_no_quick_fix() {
    let source =
        "async function main(): Promise<void> { const value = await spawn(work).join(); }\n";
    assert!(spawn_quick_fix_edit(
        source,
        "spawn(work).join()",
        "awaiting `spawn(...).join()` in the same expression defeats the purpose of running it in the background",
    )
    .is_none());
}

#[test]
pub(super) fn promise_modifier_quick_fixes_repair_the_chain() {
    let uri = Url::parse("file:///suite.test.tyr").unwrap();
    let diagnostic_for = |source: &str, member: &str, message: &str| {
        let start = source.find(member).expect("member is present");
        Diagnostic {
            range: byte_range_to_lsp(source, start..start + member.len()),
            severity: Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String("E0599".to_string())),
            message: message.to_string(),
            ..Diagnostic::default()
        }
    };
    let edits_of = |action: &CodeActionOrCommand, uri: &Url| match action {
        CodeActionOrCommand::CodeAction(action) => (
            action.title.clone(),
            action
                .edit
                .as_ref()
                .and_then(|edit| edit.changes.as_ref())
                .and_then(|changes| changes.get(uri))
                .cloned()
                .expect("expected edits"),
        ),
        CodeActionOrCommand::Command(_) => panic!("expected a code action"),
    };

    let source = "expect(load()).resolves.toThrow(\"boom\");\n";
    let mut actions = Vec::new();
    crate::server::push_promise_modifier_code_actions(
        &mut actions,
        &uri,
        source,
        &diagnostic_for(
            source,
            "toThrow",
            "`.toThrow()` does not follow `.resolves`",
        ),
    );
    assert_eq!(actions.len(), 1, "{actions:#?}");
    let (title, edits) = edits_of(&actions[0], &uri);
    assert_eq!(title, "Change `.resolves` to `.rejects`");
    assert_eq!(
        apply_edits(source, &edits),
        "expect(load()).rejects.toThrow(\"boom\");\n"
    );

    let source = "expect(load()).resolves.rejects.toThrow();\n";
    let mut actions = Vec::new();
    crate::server::push_promise_modifier_code_actions(
        &mut actions,
        &uri,
        source,
        &diagnostic_for(
            source,
            "toThrow",
            "`.resolves` and `.rejects` ask opposite questions of the same promise",
        ),
    );
    assert_eq!(actions.len(), 2, "{actions:#?}");
    let (first, first_edits) = edits_of(&actions[0], &uri);
    assert_eq!(first, "Keep `.resolves` and drop `.rejects`");
    assert_eq!(
        apply_edits(source, &first_edits),
        "expect(load()).resolves.toThrow();\n"
    );
    let (second, second_edits) = edits_of(&actions[1], &uri);
    assert_eq!(second, "Keep `.rejects` and drop `.resolves`");
    assert_eq!(
        apply_edits(source, &second_edits),
        "expect(load()).rejects.toThrow();\n"
    );

    let source = "expect(load()).toThrow();\n";
    let mut actions = Vec::new();
    crate::server::push_promise_modifier_code_actions(
        &mut actions,
        &uri,
        source,
        &diagnostic_for(
            source,
            "toThrow",
            "`.toThrow()` does not follow `.resolves`",
        ),
    );
    assert!(actions.is_empty(), "{actions:#?}");
}

#[test]
pub(super) fn invalid_typeof_category_quick_fix_replaces_only_an_unambiguous_literal() {
    let source = "const main = (): void => {\n    const value: unknown = 1;\n    if (typeof value === \"String\") {\n        console.log(value);\n    }\n    if (typeof value === \"number\") {\n        console.log(value);\n    }\n};\n";
    let analysis = analyze(source);
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let diagnostics = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "E0795")
        .map(|diagnostic| diagnostic_to_lsp(&uri, source, diagnostic))
        .collect::<Vec<_>>();
    assert_eq!(
        diagnostics.len(),
        2,
        "diagnostics: {:#?}",
        analysis.diagnostics
    );

    let actions = quick_fixes_for_diagnostics(
        &uri,
        source,
        std::path::Path::new("/workspace/src/main.tyr"),
        &[],
        &[],
        &diagnostics,
    );
    assert_eq!(
        actions.len(),
        1,
        "`\"number\"` must not get a guessed fix: {actions:#?}"
    );
    let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
        panic!("expected a code action, got {:#?}", actions[0]);
    };
    assert_eq!(action.title, "Replace with `\"string\"`");
    assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
    let edits = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .and_then(|changes| changes.get(&uri))
        .expect("action should carry workspace edits");
    let mut fixed = source.to_string();
    for edit in edits {
        let start = position_to_byte_offset(source, edit.range.start);
        let end = position_to_byte_offset(source, edit.range.end);
        fixed.replace_range(start..end, &edit.new_text);
    }
    assert!(
        fixed.contains("if (typeof value === \"string\") {"),
        "{fixed}"
    );
    assert!(
        fixed.contains("if (typeof value === \"number\") {"),
        "{fixed}"
    );
}

#[test]
pub(super) fn unknown_native_interface_type_offers_an_import_quick_fix() {
    let source = "import { utf8 } from \"tyrus:encoding\";\n\nconst main = (): void => {\n    try {\n        console.log(utf8.decode([255]));\n    } catch (error: EncodingError) {\n        console.log(error.code);\n    }\n};\n";
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let current_path = std::path::PathBuf::from("/workspace/src/main.tyr");
    let actions = quick_fixes_for_diagnostics(
        &uri,
        source,
        &current_path,
        &[],
        &[],
        &[type_name_diagnostic(source, "EncodingError")],
    );
    let action = actions
        .iter()
        .find_map(|action| match action {
            CodeActionOrCommand::CodeAction(action)
                if action.title == "Import `EncodingError` from `tyrus:encoding`" =>
            {
                Some(action)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing import quick fix: {actions:#?}"));
    assert_eq!(action.is_preferred, Some(true));
    let edits = &action.edit.as_ref().unwrap().changes.as_ref().unwrap()[&uri];
    let fixed = apply_quick_fix_edits(source, edits);
    assert!(
        fixed.lines().any(|line| line.starts_with("import")
            && line.contains("EncodingError")
            && line.contains("\"tyrus:encoding\"")),
        "fixed source:\n{fixed}"
    );
    assert!(fixed.contains("catch (error: EncodingError)"), "{fixed}");

    let unknown = "const main = (): void => {\n    try {\n        console.log(1);\n    } catch (error: Bogus) {\n        console.log(error);\n    }\n};\n";
    let actions = quick_fixes_for_diagnostics(
        &uri,
        unknown,
        &current_path,
        &[],
        &[],
        &[type_name_diagnostic(unknown, "Bogus")],
    );
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            CodeActionOrCommand::CodeAction(action) if action.title.starts_with("Import")
        )),
        "a type no module exports must not get an import fix: {actions:#?}"
    );
}

#[test]
pub(super) fn an_unimported_task_handle_type_offers_its_import_as_a_quick_fix() {
    let source = "import { channel } from \"tyrus:task\";\n\nasync function produce(tx: Sender<u64>): Promise<void> {\n    await tx.send(1);\n}\n";
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let current_path = std::path::PathBuf::from("/workspace/src/main.tyr");
    let diagnostic = Diagnostic {
        range: Range {
            start: Position {
                line: 2,
                character: 27,
            },
            end: Position {
                line: 2,
                character: 38,
            },
        },
        code: Some(tower_lsp::lsp_types::NumberOrString::String(
            "E0412".to_string(),
        )),
        message: "cannot find type `Sender` in this scope".to_string(),
        ..Diagnostic::default()
    };

    let actions = quick_fixes_for_diagnostics(
        &uri,
        source,
        &current_path,
        &[],
        &[],
        std::slice::from_ref(&diagnostic),
    );
    let action = actions
        .iter()
        .find_map(|action| match action {
            CodeActionOrCommand::CodeAction(action) => Some(action),
            _ => None,
        })
        .expect("the missing task handle type offers its import");
    assert_eq!(action.title, "Import `Sender` from `tyrus:task`");
    let edit = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .and_then(|changes| changes.get(&uri))
        .expect("the fix edits this file");
    assert_eq!(edit.len(), 1);
    assert!(
        edit[0].new_text.contains("Sender"),
        "unexpected edit: {}",
        edit[0].new_text
    );
}

#[test]
pub(super) fn an_unimported_stream_error_offers_its_import_as_a_quick_fix() {
    let source = "function inspect(error: StreamError): void { console.log(error.code); }\n";
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let current_path = std::path::PathBuf::from("/workspace/src/main.tyr");
    let start = source.find("StreamError").unwrap();
    let diagnostic = Diagnostic {
        range: byte_range_to_lsp(source, start..start + "StreamError".len()),
        code: Some(tower_lsp::lsp_types::NumberOrString::String(
            "E0412".to_string(),
        )),
        message: "cannot find type `StreamError` in this scope".to_string(),
        ..Diagnostic::default()
    };

    let actions = quick_fixes_for_diagnostics(
        &uri,
        source,
        &current_path,
        &[],
        &[],
        std::slice::from_ref(&diagnostic),
    );
    let action = actions
        .iter()
        .find_map(|action| match action {
            CodeActionOrCommand::CodeAction(action) => Some(action),
            _ => None,
        })
        .expect("the missing stream error type offers its import");
    assert_eq!(action.title, "Import `StreamError` from `tyrus:stream`");
    let edit = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .and_then(|changes| changes.get(&uri))
        .expect("the fix edits this file");
    assert_eq!(edit.len(), 1);
    assert!(
        edit[0]
            .new_text
            .contains("import { StreamError } from \"tyrus:stream\";")
    );
}

#[test]
pub(super) fn a_protected_import_offers_to_unprotect_the_declaration() {
    let root = std::env::temp_dir().join(format!("tyrus-lsp-protect-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("auth")).unwrap();
    let declaring = root.join("auth/token.tyr");
    let declaring_source = "export protect const verify = (id: i32): i32 => id + 1;\n";
    std::fs::write(&declaring, declaring_source).unwrap();

    let importer = root.join("main.tyr");
    let source = "import { verify } from \"./auth/token\";\nconst total: i32 = verify(1);\n";
    std::fs::write(&importer, source).unwrap();

    let reports = tyrus_lint::lint_paths(&root, &["main.tyr"], &tyrus_lint::LintConfig::default())
        .expect("the linter must analyze the importing file");
    let diagnostic = reports
        .iter()
        .flat_map(|report| &report.diagnostics)
        .find(|diagnostic| diagnostic.code == "E3308")
        .expect("the protected import must be reported");
    let lsp_diagnostic = Diagnostic {
        range: byte_range_to_lsp(source, diagnostic.range.clone()),
        code: Some(NumberOrString::String(diagnostic.code.clone())),
        message: diagnostic.message.clone(),
        ..Diagnostic::default()
    };
    let uri = Url::from_file_path(&importer).unwrap();
    let actions = quick_fixes_for_diagnostics(&uri, source, &importer, &[], &[], &[lsp_diagnostic]);
    let CodeActionOrCommand::CodeAction(action) = actions
        .first()
        .unwrap_or_else(|| panic!("expected one action, got {actions:?}"))
    else {
        panic!("expected a code action, got {actions:?}");
    };
    assert!(
        action.title.contains("Remove `protect`") && action.title.contains("verify"),
        "the title must name the fix and the symbol: {}",
        action.title
    );
    let changes = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .expect("the fix edits a file");
    let declaring_uri = Url::from_file_path(&declaring).unwrap();
    let edits = changes
        .get(&declaring_uri)
        .expect("the edit belongs to the declaring file, not the importing one");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "");
    let start = position_to_byte_offset(declaring_source, edits[0].range.start);
    let end = position_to_byte_offset(declaring_source, edits[0].range.end);
    assert_eq!(&declaring_source[start..end], " protect");
    let mut fixed = declaring_source.to_string();
    fixed.replace_range(start..end, "");
    assert!(
        fixed.starts_with("export const verify"),
        "applying the fix must leave a plain export: {fixed}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

fn analyze_with_archive_imports(source: &str) -> tyrus_core::analysis::DocumentAnalysis {
    analyze_with_archive_exports(
        source,
        &["pack", "ArchiveError", "PackOptions", "ArchiveSummary"],
    )
}

fn analyze_with_archive_exports(
    source: &str,
    names: &[&str],
) -> tyrus_core::analysis::DocumentAnalysis {
    let mut types = tyrus_core::collections::HashMap::default();
    let mut interfaces = tyrus_core::collections::HashMap::default();
    let mut targets = tyrus_core::collections::HashMap::default();
    let throws = tyrus_core::collections::HashMap::default();
    for &name in names {
        let (ty, fields, _) = tyrus_core::module_loader::native_export_info("tyrus:archive", name)
            .expect("archive native export metadata");
        types.insert(name.to_string(), ty);
        if let Some(fields) = fields {
            interfaces.insert(name.to_string(), fields);
        }
        if let Some(target) = tyrus_core::module_loader::native_export_target("tyrus:archive", name)
        {
            targets.insert(name.to_string(), target);
        }
    }
    tyrus_core::analysis::analyze_with_context(
        source,
        tyrus_core::analysis::AnalyzeContext {
            import_types: Some(&types),
            import_targets: Some(&targets),
            import_interfaces: Some(&interfaces),
            import_throws: Some(&throws),
            ..tyrus_core::analysis::AnalyzeContext::default()
        },
    )
}

fn apply_single_quick_fix(
    source: &str,
    uri: &Url,
    code: &str,
) -> (Diagnostic, Vec<CodeActionOrCommand>, String) {
    let analysis = analyze_with_archive_imports(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == code)
        .map(|diagnostic| diagnostic_to_lsp(uri, source, diagnostic))
        .unwrap_or_else(|| panic!("expected {code}: {:#?}", analysis.diagnostics));
    let actions = quick_fixes_for_diagnostics(
        uri,
        source,
        std::path::Path::new("/workspace/src/main.tyr"),
        &[],
        &[],
        std::slice::from_ref(&diagnostic),
    );
    let mut fixed = source.to_string();
    if let Some(CodeActionOrCommand::CodeAction(action)) = actions.first() {
        let edits = action
            .edit
            .as_ref()
            .and_then(|edit| edit.changes.as_ref())
            .and_then(|changes| changes.get(uri))
            .expect("action should carry workspace edits");
        for edit in edits.iter().rev() {
            let start = position_to_byte_offset(source, edit.range.start);
            let end = position_to_byte_offset(source, edit.range.end);
            fixed.replace_range(start..end, &edit.new_text);
        }
    }
    (diagnostic, actions, fixed)
}

#[test]
pub(super) fn archive_extension_diagnostic_points_at_the_literal_and_fixes_only_the_case() {
    let source = "import { ArchiveError, ArchiveSummary, pack } from \"tyrus:archive\";\n\nconst main = async (): Promise<void> => {\n    match (await pack(\"./src\", \"./dist/App.ZIP\")) {\n        Ok(summary) => { print(summary.entries); }\n        Err(error) => { printError(error.code); }\n    }\n};\n";
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let (diagnostic, actions, fixed) = apply_single_quick_fix(source, &uri, "E3311");
    let start = position_to_byte_offset(source, diagnostic.range.start);
    let end = position_to_byte_offset(source, diagnostic.range.end);
    assert_eq!(&source[start..end], "\"./dist/App.ZIP\"");
    assert_eq!(actions.len(), 1, "{actions:#?}");
    let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
        panic!("expected a code action");
    };
    assert_eq!(action.title, "Write the extension as `.zip`");
    assert!(
        fixed.contains("await pack(\"./src\", \"./dist/App.zip\"))"),
        "{fixed}"
    );

    let jar = source.replace("App.ZIP", "app.jar");
    let (_, actions, _) = apply_single_quick_fix(&jar, &uri, "E3311");
    assert!(
        actions.is_empty(),
        "a format must not be guessed: {actions:#?}"
    );
}

#[test]
pub(super) fn archive_level_on_tar_offers_removing_the_property() {
    let source = "import { ArchiveError, ArchiveSummary, PackOptions, pack } from \"tyrus:archive\";\n\nconst main = async (): Promise<void> => {\n    match (await pack(\"./src\", \"./dist/a.tar\", { level: 6, overwrite: true })) {\n        Ok(summary) => { print(summary.entries); }\n        Err(error) => { printError(error.code); }\n    }\n};\n";
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let (diagnostic, actions, fixed) = apply_single_quick_fix(source, &uri, "E3312");
    let start = position_to_byte_offset(source, diagnostic.range.start);
    let end = position_to_byte_offset(source, diagnostic.range.end);
    assert_eq!(&source[start..end], "6");
    assert_eq!(actions.len(), 1, "{actions:#?}");
    assert!(
        fixed.contains("await pack(\"./src\", \"./dist/a.tar\", { overwrite: true }))"),
        "{fixed}"
    );

    let out_of_range = source.replace("a.tar\", { level: 6", "a.zip\", { level: 12");
    let (diagnostic, actions, _) = apply_single_quick_fix(&out_of_range, &uri, "E3312");
    let start = position_to_byte_offset(&out_of_range, diagnostic.range.start);
    let end = position_to_byte_offset(&out_of_range, diagnostic.range.end);
    assert_eq!(&out_of_range[start..end], "12");
    assert!(
        actions.is_empty(),
        "a level must not be guessed: {actions:#?}"
    );
}

#[test]
pub(super) fn compress_level_out_of_range_anchors_the_literal_without_a_fix() {
    let source = "import { CompressError, CompressFormat, CompressOptions, compress } from \"tyrus:compress\";\n\nconst main = async (): Promise<void> => {\n    match (await compress(\"x\", CompressFormat.Brotli, { level: 12 })) {\n        Ok(bytes) => { print(bytes.length); }\n        Err(error) => { printError(error.code); }\n    }\n};\n";
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let mut types = tyrus_core::collections::HashMap::default();
    let mut interfaces = tyrus_core::collections::HashMap::default();
    let mut targets = tyrus_core::collections::HashMap::default();
    let throws = tyrus_core::collections::HashMap::default();
    for name in [
        "compress",
        "CompressError",
        "CompressFormat",
        "CompressOptions",
    ] {
        let (ty, fields, _) = tyrus_core::module_loader::native_export_info("tyrus:compress", name)
            .expect("compress native export metadata");
        types.insert(name.to_string(), ty);
        if let Some(fields) = fields {
            interfaces.insert(name.to_string(), fields);
        }
        if let Some(target) =
            tyrus_core::module_loader::native_export_target("tyrus:compress", name)
        {
            targets.insert(name.to_string(), target);
        }
    }
    let enums = tyrus_core::collections::HashMap::from_iter([(
        "CompressFormat".to_string(),
        tyrus_core::compress::compress_format_enum(),
    )]);
    let analysis = tyrus_core::analysis::analyze_with_context(
        source,
        tyrus_core::analysis::AnalyzeContext {
            import_types: Some(&types),
            import_targets: Some(&targets),
            import_interfaces: Some(&interfaces),
            import_throws: Some(&throws),
            import_enums: Some(&enums),
            ..tyrus_core::analysis::AnalyzeContext::default()
        },
    );
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3312")
        .map(|diagnostic| diagnostic_to_lsp(&uri, source, diagnostic))
        .unwrap_or_else(|| panic!("expected E3312: {:#?}", analysis.diagnostics));
    let actions = quick_fixes_for_diagnostics(
        &uri,
        source,
        std::path::Path::new("/workspace/src/main.tyr"),
        &[],
        &[],
        std::slice::from_ref(&diagnostic),
    );
    let start = position_to_byte_offset(source, diagnostic.range.start);
    let end = position_to_byte_offset(source, diagnostic.range.end);
    assert_eq!(&source[start..end], "12");
    assert!(
        diagnostic
            .message
            .contains("`0` to `11` of `CompressFormat.Brotli`"),
        "{}",
        diagnostic.message
    );
    assert!(
        actions.is_empty(),
        "a level must not be guessed: {actions:#?}"
    );
}

#[test]
pub(super) fn an_unimported_native_options_interface_offers_its_import() {
    let source = "import { ArchiveError, ArchiveSummary, pack } from \"tyrus:archive\";\n\nconst main = async (): Promise<void> => {\n    match (await pack(\"./src\", \"./a.zip\", { level: 9 })) {\n        Ok(summary) => { print(summary.entries); }\n        Err(error) => { printError(error.code); }\n    }\n};\n";
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let analysis =
        analyze_with_archive_exports(source, &["pack", "ArchiveError", "ArchiveSummary"]);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0412")
        .map(|diagnostic| diagnostic_to_lsp(&uri, source, diagnostic))
        .unwrap_or_else(|| panic!("expected E0412: {:#?}", analysis.diagnostics));
    let start = position_to_byte_offset(source, diagnostic.range.start);
    let end = position_to_byte_offset(source, diagnostic.range.end);
    assert_eq!(&source[start..end], "{ level: 9 }");
    let actions = quick_fixes_for_diagnostics(
        &uri,
        source,
        std::path::Path::new("/workspace/src/main.tyr"),
        &[],
        &[],
        std::slice::from_ref(&diagnostic),
    );
    let edits: Vec<String> = actions
        .iter()
        .filter_map(|action| match action {
            CodeActionOrCommand::CodeAction(action) => action.edit.as_ref(),
            CodeActionOrCommand::Command(_) => None,
        })
        .filter_map(|edit| edit.changes.as_ref())
        .filter_map(|changes| changes.get(&uri))
        .flat_map(|edits| edits.iter().map(|edit| edit.new_text.clone()))
        .collect();
    assert!(
        edits.iter().any(|text| text.contains("PackOptions")),
        "expected an import of `PackOptions`: {actions:#?}"
    );
}

#[test]
pub(super) fn an_async_local_of_an_unimported_native_interface_offers_its_import() {
    let source = "import { ArchiveError, list, pack } from \"tyrus:archive\";\n\nasync function inspect(): Promise<Result<void, ArchiveError>> {\n    const entries = await list(\"./a.zip\")?;\n    await pack(\"./src\", \"./b.zip\")?;\n    print(entries.length);\n    return Ok();\n}\n\nconst main = (): void => {};\n";
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let analysis = analyze_with_archive_exports(source, &["list", "pack", "ArchiveError"]);
    let codes: Vec<&str> = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code.starts_with('E'))
        .map(|diagnostic| diagnostic.code.as_str())
        .collect();
    assert_eq!(codes, ["E0412"], "{:#?}", analysis.diagnostics);
    let diagnostic = &analysis.diagnostics[0];
    assert_eq!(
        diagnostic.message,
        "cannot find type `ArchiveEntry` in this scope"
    );
    assert_eq!(
        diagnostic.help,
        vec!["`entries` has type `ArchiveEntry[]`, and `ArchiveEntry` is exported by `tyrus:archive`; add `import { ArchiveEntry } from \"tyrus:archive\";`".to_string()]
    );
    let diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    let start = position_to_byte_offset(source, diagnostic.range.start);
    let end = position_to_byte_offset(source, diagnostic.range.end);
    assert_eq!(start, source.find("entries = ").unwrap());
    assert_eq!(&source[start..end], "entries");
    let actions = quick_fixes_for_diagnostics(
        &uri,
        source,
        std::path::Path::new("/workspace/src/main.tyr"),
        &[],
        &[],
        std::slice::from_ref(&diagnostic),
    );
    let edits: Vec<String> = actions
        .iter()
        .filter_map(|action| match action {
            CodeActionOrCommand::CodeAction(action) => action.edit.as_ref(),
            CodeActionOrCommand::Command(_) => None,
        })
        .filter_map(|edit| edit.changes.as_ref())
        .filter_map(|changes| changes.get(&uri))
        .flat_map(|edits| edits.iter().map(|edit| edit.new_text.clone()))
        .collect();
    assert!(
        edits.iter().any(|text| text.contains("ArchiveEntry")),
        "expected an import of `ArchiveEntry`: {actions:#?}"
    );
}
