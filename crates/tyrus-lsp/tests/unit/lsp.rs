use super::*;
use crate::backend::{
    InitializationOptions, attach_import_alias_related_information, import_source_range,
    is_entry_point_uri, is_lint_owned_analysis_diagnostic,
};
use crate::server::{
    call_sites_in_range, completion_identifier_prefix, diagnostic_has_quick_fix_candidate,
    inferred_type_inlay_hint, is_newer_document_version, is_valid_inlay_argument_range,
    parameter_inlay_label, quick_fix_code_lenses, quick_fixes_for_diagnostics,
    quick_fixes_need_workspace_exports,
};
use tyrus_core::doc_comments::DocComment;
use tyrus_core::span::Span;

#[test]
fn quick_fix_coverage_marks_only_diagnostics_with_safe_fix_paths() {
    let diagnostic = |code: &str, source: &str| Diagnostic {
        code: Some(NumberOrString::String(code.to_string())),
        source: Some(source.to_string()),
        ..Diagnostic::default()
    };
    assert!(diagnostic_has_quick_fix_candidate(&diagnostic(
        "prefer-const",
        "tyrus-lint"
    )));
    assert!(diagnostic_has_quick_fix_candidate(&diagnostic(
        "http-no-raw-status-code",
        "tyrus-lint"
    )));
    assert!(diagnostic_has_quick_fix_candidate(&diagnostic(
        "no-unused-vars",
        "tyrus-lint"
    )));
    assert!(diagnostic_has_quick_fix_candidate(&diagnostic(
        "E0308", "tyrus"
    )));
    assert!(!diagnostic_has_quick_fix_candidate(&diagnostic(
        "E9999", "tyrus"
    )));
}

#[test]
fn inlay_hint_cache_bounds_the_number_of_visible_ranges() {
    let mut cache = std::collections::HashMap::new();
    for line in 0..5 {
        crate::cache_inlay_hints(&mut cache, (line, 0, line + 1, 0), Vec::new());
    }
    assert_eq!(cache.len(), 4);
    assert!(cache.contains_key(&(4, 0, 5, 0)));
    assert!(!cache.contains_key(&(0, 0, 1, 0)));
    assert!(cache.contains_key(&(1, 0, 2, 0)));
}

#[test]
fn call_parameter_cache_is_bounded_and_caches_missing_signatures() {
    let mut cache = std::collections::HashMap::new();
    for index in 0..MAX_CACHED_CALL_PARAMETERS {
        cache_call_parameters(&mut cache, format!("callee{index}"), Some(Vec::new()));
    }
    cache_call_parameters(&mut cache, "unknown".to_string(), None);
    assert_eq!(cache.len(), MAX_CACHED_CALL_PARAMETERS);
    assert_eq!(cache.get("unknown"), Some(&None));
    assert!(!cache.contains_key("callee0"));
    assert!(cache.contains_key("callee1"));
}

#[path = "lsp/completion_tests/completion_tests.rs"]
mod completion_tests;
#[path = "lsp/diagnostic_tests/diagnostic_tests.rs"]
mod diagnostic_tests;
#[path = "lsp/hover_tests/hover_tests.rs"]
mod hover_tests;
#[path = "lsp/quick_fix_tests/quick_fix_tests.rs"]
mod quick_fix_tests;

fn synthetic_quickfix_action(title: &str, range: Range, is_preferred: bool) -> CodeActionOrCommand {
    CodeActionOrCommand::CodeAction(CodeAction {
        title: title.to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![Diagnostic {
            range,
            message: format!("diagnostic behind {title}"),
            ..Diagnostic::default()
        }]),
        edit: Some(WorkspaceEdit {
            changes: Some(
                [(
                    Url::parse("file:///workspace/src/main.tyr").unwrap(),
                    vec![TextEdit {
                        range,
                        new_text: "fixed".to_string(),
                    }],
                )]
                .into_iter()
                .collect(),
            ),
            ..WorkspaceEdit::default()
        }),
        is_preferred: Some(is_preferred),
        ..CodeAction::default()
    })
}

#[test]
fn a_cors_origin_with_a_path_offers_to_lose_it() {
    let source = "const main = (): void => {\n    app.cors({ origin: [\"https://example.com/api\"] });\n};\n";
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let current_path = std::path::PathBuf::from("/workspace/src/main.tyr");
    let start = source.find("\"https").expect("the origin literal");
    let end = start + "\"https://example.com/api\"".len();
    let diagnostic = Diagnostic {
        range: byte_range_to_lsp(source, start..end),
        code: Some(NumberOrString::String("E2743".to_string())),
        message: "`https://example.com/api` is not a valid CORS origin".to_string(),
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
        .expect("a quick fix for the origin");
    assert_eq!(action.title, "Trim the origin to `https://example.com`");
    let edits = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .and_then(|changes| changes.get(&uri))
        .expect("one edit");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "\"https://example.com\"");
    assert_eq!(edits[0].range, diagnostic.range);

    let start = source.find("\"https").expect("the origin literal");
    let no_scheme = Diagnostic {
        range: byte_range_to_lsp(source, start..start + 1),
        code: Some(NumberOrString::String("E2743".to_string())),
        message: "`example.com` is not a valid CORS origin".to_string(),
        ..Diagnostic::default()
    };
    let actions = quick_fixes_for_diagnostics(&uri, source, &current_path, &[], &[], &[no_scheme]);
    assert!(
        quick_fix_code_lenses(&actions).is_empty(),
        "a guessed scheme must not be offered as a fix"
    );
}

fn initialization_options(json: &str) -> Option<serde_json::Value> {
    Some(serde_json::from_str(json).unwrap())
}

#[test]
fn initialization_options_default_to_enabled_when_missing() {
    let options = InitializationOptions::parse(&None);
    assert!(options.format_enable);
    assert!(options.code_lens_quick_fixes_enable);
}

#[test]
fn initialization_options_parse_each_toggle_independently() {
    let options =
        InitializationOptions::parse(&initialization_options(r#"{"formatEnable": false}"#));
    assert!(!options.format_enable);
    assert!(options.code_lens_quick_fixes_enable);

    let options = InitializationOptions::parse(&initialization_options(
        r#"{"codeLensQuickFixesEnable": false}"#,
    ));
    assert!(options.format_enable);
    assert!(!options.code_lens_quick_fixes_enable);
}

#[test]
fn analysis_cache_requires_the_exact_document_text_and_version() {
    let document = DocumentState {
        text: "const value = 1;".to_string(),
        version: 4,
        analysis: Some(CachedAnalysis {
            context_generation: 2,
            analysis: Arc::new(DocumentAnalysis::default()),
            line_index: Arc::new(LineIndex::new("const value = 1;")),
            import_behaviors: Arc::new(tyrus_core::collections::HashMap::default()),
            lint_diagnostics: None,
            semantic_tokens: None,
            call_parameters: std::collections::HashMap::default(),
            inlay_hints: std::collections::HashMap::new(),
        }),
        published_diagnostics: Vec::new(),
    };
    assert!(
        document
            .cached_analysis("const value = 1;", Some(4), 2)
            .is_some()
    );
    assert!(
        document
            .cached_analysis("const value = 2;", Some(4), 2)
            .is_none()
    );
    assert!(
        document
            .cached_analysis("const value = 1;", Some(5), 2)
            .is_none()
    );
    assert!(
        document
            .cached_analysis("const value = 1;", Some(4), 3)
            .is_none()
    );
}

#[test]
fn inlay_call_site_range_index_keeps_enclosing_calls_and_skips_distant_calls() {
    let source = "const value: i32 = outer(inner(1));\nconst other: i32 = later();\n";
    let analysis = tyrus_core::analysis::analyze(source);
    let inner_start = source.find("inner").unwrap();
    let inner_end = inner_start + "inner(1)".len();
    let calls = call_sites_in_range(&analysis, inner_start, inner_end);
    let names = calls
        .iter()
        .map(|call| call.callee.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["outer", "inner"]);
}

#[test]
fn line_index_preserves_utf16_positions_for_multibyte_characters() {
    let source = "first\n😀value\nlast";
    let index = LineIndex::new(source);
    let value_offset = source.find("value").unwrap();
    assert_eq!(
        index.byte_offset_to_position(source, value_offset),
        Position::new(1, 2)
    );
    assert_eq!(
        index.position_to_byte_offset(source, Position::new(1, 2)),
        value_offset
    );
    assert_eq!(
        index.position_to_byte_offset(source, Position::new(1, 0)),
        source.find('😀').unwrap()
    );
    assert_eq!(
        index.position_to_byte_offset(source, Position::new(1, 1)),
        source.find('😀').unwrap() + '😀'.len_utf8()
    );
}

#[test]
fn workspace_exports_are_only_needed_for_undeclared_identifier_fixes() {
    let local_fix = Diagnostic {
        message: "type mismatch".to_string(),
        ..Diagnostic::default()
    };
    assert!(!quick_fixes_need_workspace_exports(&[local_fix]));

    let import_fix = Diagnostic {
        code: Some(NumberOrString::String("E0425".to_string())),
        message: "use of undeclared identifier `formatUser`".to_string(),
        ..Diagnostic::default()
    };
    assert!(quick_fixes_need_workspace_exports(&[import_fix]));
}

#[test]
fn completion_identifier_prefix_ignores_whitespace_and_stops_at_punctuation() {
    let source = "const value = al";
    assert_eq!(
        completion_identifier_prefix(source, source.len()),
        Some("al")
    );
    assert_eq!(completion_identifier_prefix("const value = ", 14), None);
    assert_eq!(
        completion_identifier_prefix("object.member", 13),
        Some("member")
    );
    assert_eq!(completion_identifier_prefix("object.", 7), None);
}

#[test]
fn document_changes_must_advance_the_version() {
    assert!(is_newer_document_version(None, 0));
    assert!(is_newer_document_version(Some(4), 5));
    assert!(!is_newer_document_version(Some(4), 4));
    assert!(!is_newer_document_version(Some(5), 4));
}

#[test]
fn initialization_options_ignore_non_boolean_values() {
    let options = InitializationOptions::parse(&initialization_options(
        r#"{"formatEnable": "no", "codeLensQuickFixesEnable": 0}"#,
    ));
    assert!(options.format_enable);
    assert!(options.code_lens_quick_fixes_enable);
}

#[test]
fn diagnostic_debounce_option_defaults_and_is_bounded() {
    assert_eq!(
        InitializationOptions::parse(&None).diagnostic_debounce_ms,
        120
    );
    let options =
        InitializationOptions::parse(&initialization_options(r#"{"diagnosticDebounceMs": 2400}"#));
    assert_eq!(options.diagnostic_debounce_ms, 1_000);
}

#[test]
fn publishes_an_out_of_bounds_index_after_a_constant_control_flow_reassignment() {
    let source = r#"const index = 1;
let array = [1, 2, 3, 4, 5];

if (index === 1) {
    array = [];
}

print(array[2]);
"#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0606")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    let index_access = source
        .find("array[2]")
        .expect("array index access is present");
    assert_eq!(
        lsp_diagnostic.range,
        byte_range_to_lsp(source, index_access..index_access + "array[2]".len())
    );
    assert_eq!(
        lsp_diagnostic.code,
        Some(NumberOrString::String("E0606".to_string()))
    );
}

fn scratch_project(unique: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("tyrus-lsp-entry-point-test-{unique}"));
    let src = root.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(root.join("tyrus.json"), "{}\n").unwrap();
    std::fs::write(src.join("main.tyr"), "const main = (): void => {};\n").unwrap();
    root
}

#[test]
fn is_entry_point_uri_recognizes_the_project_entry_module() {
    let root = scratch_project("recognizes");
    let uri = Url::from_file_path(root.join("src").join("main.tyr")).unwrap();
    assert!(is_entry_point_uri(&uri));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn is_entry_point_uri_rejects_a_non_entry_module_in_the_same_project() {
    let root = scratch_project("rejects");
    std::fs::write(root.join("src").join("helper.tyr"), "export const x = 1;\n").unwrap();
    let uri = Url::from_file_path(root.join("src").join("helper.tyr")).unwrap();
    assert!(!is_entry_point_uri(&uri));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn validator_failed_is_language_owned_not_lint_owned() {
    let source = "const divide = (a: i32, @notEqual(0) b: i32): i32 => a / b;\n\
                  const main = (): void => {\n    console.log(divide(10, 0));\n};\n";
    let analysis = analyze(source);
    let count = |code: &str| {
        analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == code)
            .count()
    };
    assert_eq!(count("E3165"), 1);

    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3165")
        .unwrap();
    assert!(!is_lint_owned_analysis_diagnostic(diagnostic));

    assert_eq!(
        tyrus_lint::lint_rules_source(source, &tyrus_lint::LintConfig::default())
            .iter()
            .filter(|diagnostic| diagnostic.code == "E3165")
            .count(),
        0
    );
}

#[test]
fn validator_redundant_is_language_owned_not_lint_owned() {
    let source = "interface ProductFilter { @min(1) @min(5) pageSize: i32; }\n";
    let analysis = analyze(source);
    let count = |code: &str| {
        analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == code)
            .count()
    };
    assert_eq!(count("E3164"), 1);

    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3164")
        .unwrap();
    assert!(!is_lint_owned_analysis_diagnostic(diagnostic));

    assert_eq!(
        tyrus_lint::lint_rules_source(source, &tyrus_lint::LintConfig::default())
            .iter()
            .filter(|diagnostic| diagnostic.code == "E3164")
            .count(),
        0
    );
}

#[test]
fn undeclared_initializer_identifier_publishes_its_own_lsp_range() {
    let source = "const u8MIN = u8MIN;\nconsole.log(u8MIN);\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0425")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    let initializer = source.find("u8MIN;").unwrap();
    assert_eq!(
        lsp_diagnostic.range,
        byte_range_to_lsp(source, initializer..initializer + "u8MIN".len())
    );
}

#[test]
fn onfail_handler_without_an_exit_publishes_the_handler_range_to_lsp() {
    let source = "function load(): Result<i32, string> { return Err(\"gone\"); }\nfunction count(): i32 {\n    const value = load() onfail console.log(value);\n    return value;\n}\nconsole.log(count());\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0277")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    let handler = source.find("onfail").unwrap();
    let end = handler + source[handler..].find(';').unwrap();
    assert_eq!(
        lsp_diagnostic.range,
        byte_range_to_lsp(source, handler..end)
    );
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("end the handler with `return <value>;` or `throw`"))
    );
}

#[test]
fn await_inside_an_onfail_handler_publishes_the_await_range_to_lsp() {
    let source = "async function load(): Promise<Result<i32, string>> { return Err(\"gone\"); }\nasync function pause(): Promise<void> {}\nasync function run(): Promise<void> {\n    const value = await load() onfail {\n        await pause();\n        return;\n    };\n    console.log(value);\n}\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3002")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    let inner = source.find("await pause").unwrap();
    assert_eq!(
        lsp_diagnostic.range,
        byte_range_to_lsp(source, inner..inner + "await".len())
    );
}

#[test]
fn credentialed_wildcard_cors_publishes_the_wildcard_range_to_lsp() {
    let source = "interface CorsOptions { origin: string[]; methods: string[]; allowedHeaders: string[]; exposedHeaders: string[]; credentials: boolean; maxAge: i32; }\ninterface HttpRouter {}\nconst app: HttpRouter = __tyrus_http_createRouter();\napp.cors({ origin: [\"*\"], methods: [\"GET\"], allowedHeaders: [], exposedHeaders: [], credentials: true, maxAge: 0 });\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E2742")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    let wildcard = source.rfind("\"*\"").expect("wildcard origin is present");
    assert_eq!(
        lsp_diagnostic.range,
        byte_range_to_lsp(source, wildcard..wildcard + "\"*\"".len())
    );
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("list the exact origins instead"))
    );
}

#[test]
fn auto_import_skips_already_imported_even_when_file_is_broken() {
    let source = "import { Account } from \"./interfaces/examples\";\n\nconst a: Account = {\n    Account\n};\n";
    assert!(imported_names(source).contains("Account"));
    let edit = auto_import_edit(source, "./interfaces/examples", "Account");
    assert!(
        edit.is_none(),
        "should not re-import an already-imported name"
    );
}

#[test]
fn auto_import_merges_into_existing_statement() {
    let source = "import { Account } from \"./interfaces/examples\";\n\nconst x = 1;\n";
    let edit = auto_import_edit(source, "./interfaces/examples", "Role")
        .expect("a new name from the same module produces an edit");
    assert!(
        edit.new_text
            .contains("import { Account, Role } from \"./interfaces/examples\";")
    );
    assert_eq!(edit.new_text.matches("import {").count(), 1);
}

#[test]
fn object_interface_help_suggests_import_for_exported_match() {
    let candidate = backend::ObjectInterfaceCandidate {
        name: "AdminStatus".to_string(),
        fields: Vec::new(),
        exported: true,
        uri: Url::parse("file:///workspace/src/types.tyr").unwrap(),
    };
    let help = backend::object_interface_help(&[&candidate], Path::new("/workspace/src/main.tyr"))
        .expect("exported interface should produce import help");
    assert!(help.contains("import `{ AdminStatus }` from `./types`"));
    assert!(help.contains("`: AdminStatus`"));

    let parsed = crate::server::diagnostic_object_import_help(&format!(
        "object literal `isAdmin` requires an explicit interface annotation; {help}"
    ))
    .expect("diagnostic help should be parsed for quick fix");
    assert_eq!(parsed, ("AdminStatus".to_string(), "./types".to_string()));
}

#[test]
fn object_binding_annotation_edit_inserts_type_before_initializer() {
    let source = "const isAdmin = { a: 1, b: \"yes\", c: true };\n";
    let binding_end = source.find("isAdmin").unwrap() + "isAdmin".len();
    let edit = crate::server::object_binding_annotation_edit(source, binding_end, "AdminStatus")
        .expect("annotation edit should be created");
    let start = position_to_byte_offset(source, edit.range.start);
    let end = position_to_byte_offset(source, edit.range.end);
    let mut fixed = source.to_string();
    fixed.replace_range(start..end, &edit.new_text);
    assert_eq!(
        fixed,
        "const isAdmin: AdminStatus = { a: 1, b: \"yes\", c: true };\n"
    );
}

#[test]
fn object_interface_help_suggests_export_for_private_match() {
    let candidate = backend::ObjectInterfaceCandidate {
        name: "AdminStatus".to_string(),
        fields: Vec::new(),
        exported: false,
        uri: Url::parse("file:///workspace/src/types.tyr").unwrap(),
    };
    let help = backend::object_interface_help(&[&candidate], Path::new("/workspace/src/main.tyr"))
        .expect("private interface should produce export help");
    assert!(help.contains("interface `AdminStatus`"));
    assert!(help.contains("is not exported"));
    assert!(help.contains("add `export`"));

    let parsed = crate::server::diagnostic_object_export_help(&help)
        .expect("private interface help should be parsed for quick fix");
    assert_eq!(
        parsed,
        (
            "AdminStatus".to_string(),
            "/workspace/src/types.tyr".to_string()
        )
    );
}

#[test]
fn signature_return_type_parser_keeps_promise_payload() {
    assert_eq!(
        crate::server::signature_return_type_from_detail("(value: i32) => Promise<i32>").as_deref(),
        Some("Promise<i32>")
    );
}

#[test]
fn export_interface_edit_inserts_export_before_private_interface() {
    let source = "interface AdminStatus {\n    a: i32;\n}\n";
    let edit = crate::server::export_interface_edit(source, "AdminStatus")
        .expect("private interface should produce export edit");
    let start = position_to_byte_offset(source, edit.range.start);
    let end = position_to_byte_offset(source, edit.range.end);
    let mut fixed = source.to_string();
    fixed.replace_range(start..end, &edit.new_text);
    assert_eq!(fixed, "export interface AdminStatus {\n    a: i32;\n}\n");
}

#[test]
fn inferred_type_hints_only_for_omitted_annotations() {
    let source = concat!(
        "const count = 42;\n",
        "const big = 5000000000;\n",
        "const huge = 3_000_000_000_000_000_000_000;\n",
        "const ratio = 3.14;\n",
        "const label = \"tyrus\";\n",
        "const active = true;\n",
        "const names = [\"a\"];\n",
        "const maybeNames = \"a\".match(/(a)/);\n",
        "const re = /a+/;\n",
        "const annotated: i32 = 7;\n",
    );
    let analysis = analyze(source);
    let hints = inferred_type_hints(source, &analysis);
    let types: Vec<&str> = hints.iter().map(|(_, ty)| ty.as_str()).collect();
    assert_eq!(
        types,
        vec![
            "i32",
            "i64",
            "i128",
            "f64",
            "string",
            "boolean",
            "string[]",
            "string[]?",
            "Regex"
        ]
    );
    assert!(!hints.iter().any(|(offset, _)| {
        let prefix = &source[..*offset];
        prefix.ends_with("annotated")
    }));
    let count_offset = source.find("count").unwrap() + "count".len();
    assert!(
        hints
            .iter()
            .any(|(offset, ty)| *offset == count_offset && ty == "i32")
    );
}

#[test]
fn fn_return_type_hints_show_the_inferred_return_type_including_void() {
    let source = concat!(
        "function hello() {\n",
        "    console.log(\"hi\");\n",
        "}\n",
        "function double(x: i32) {\n",
        "    return x * 2;\n",
        "}\n",
        "function annotated(): string {\n",
        "    return \"x\";\n",
        "}\n",
    );
    let analysis = analyze(source);
    let hints = fn_return_type_hints(source, &analysis);
    let types: Vec<&str> = hints.iter().map(|(_, ty)| ty.as_str()).collect();
    assert_eq!(types, vec!["void", "i32"]);
    let hello_paren = source.find("hello()").unwrap() + "hello(".len() + 1;
    assert!(
        hints
            .iter()
            .any(|(offset, ty)| *offset == hello_paren && ty == "void")
    );
    assert!(!hints.iter().any(|(offset, _)| {
        let prefix = &source[..*offset];
        prefix.ends_with("annotated()")
    }));
}

#[test]
fn inferred_type_hints_use_pipeline_initializer_types() {
    let source = concat!(
        "const double = (value: i32): i32 => value * 2;\n",
        "const increment = (value: i32): i32 => value + 1;\n",
        "const range = (value: i32, max: i32): i32 => value > max ? max : value;\n",
        "const result = 5 |> double |> increment |> double;\n",
        "const ranged = 100 |> range(50);\n",
        "42 |> double |> console.log;\n",
    );
    let analysis = analyze(source);
    let hints = inferred_type_hints(source, &analysis);

    for expected in ["result", "ranged"] {
        let offset = source.find(expected).unwrap() + expected.len();
        assert!(
            hints
                .iter()
                .any(|(hint_offset, ty)| *hint_offset == offset && ty == "i32"),
            "missing inferred pipeline type hint for `{expected}`"
        );
    }
}

#[test]
fn inferred_type_hints_skip_arrow_function_initializers() {
    let source = "const load = async (value: i32) => value + 1;\n";
    let analysis = analyze(source);
    let hints = inferred_type_hints(source, &analysis);
    let offset = source.find("load").unwrap() + "load".len();
    assert!(!hints.iter().any(|(hint_offset, _)| *hint_offset == offset));
}

#[test]
fn inferred_type_hints_skip_arrow_function_with_explicit_return_type() {
    let source = "const main = async (): Promise<void> => {};\n";
    let analysis = analyze(source);
    let hints = inferred_type_hints(source, &analysis);
    let offset = source.find("main").unwrap() + "main".len();
    assert!(!hints.iter().any(|(hint_offset, _)| *hint_offset == offset));
}

#[test]
fn inferred_type_hints_skip_bindings_rejected_by_e0310() {
    let source = concat!(
        "interface User {\n",
        "    name: string;\n",
        "}\n",
        "\n",
        "export function main() {\n",
        "    const user = { name: \"Eu\" };\n",
        "    console.log(user);\n",
        "}\n",
    );
    let analysis = analyze(source);
    let hints = inferred_type_hints(source, &analysis);
    let offset = source.find("user").unwrap() + "user".len();
    assert!(
        !hints.iter().any(|(hint_offset, _)| *hint_offset == offset),
        "unexpected inlay hint for a binding the checker rejected: {hints:?}"
    );
}

#[test]
fn inferred_type_hints_use_imported_pipeline_function_types() {
    let source = concat!(
        "import { double, range, clamp } from \"./pipe/examples\";\n",
        "const result = 5 |> double |> double;\n",
        "const ranged = 100 |> range(50);\n",
        "const clamped = 80 |> clamp(30, 100);\n",
    );
    let mut import_types = tyrus_core::collections::HashMap::default();
    import_types.insert(
        "double".to_string(),
        Type::Function {
            params: vec![Type::I64],
            ret: Box::new(Type::I64),
        },
    );
    import_types.insert(
        "range".to_string(),
        Type::Function {
            params: vec![Type::I64, Type::I64],
            ret: Box::new(Type::I64),
        },
    );
    import_types.insert(
        "clamp".to_string(),
        Type::Function {
            params: vec![Type::I64, Type::I64, Type::I64],
            ret: Box::new(Type::I64),
        },
    );
    let analysis = tyrus_core::analysis::analyze_with_imports(
        source,
        &import_types,
        &tyrus_core::collections::HashMap::default(),
        &tyrus_core::collections::HashMap::default(),
    );
    let hints = inferred_type_hints(source, &analysis);

    for expected in ["result", "ranged", "clamped"] {
        let offset = source.find(expected).unwrap() + expected.len();
        assert!(
            hints
                .iter()
                .any(|(hint_offset, ty)| *hint_offset == offset && ty == "i64"),
            "missing imported pipeline type hint for `{expected}`"
        );
    }
}

#[test]
fn inferred_type_inlay_hints_keep_space_before_following_tokens() {
    let source = "const array = [\"a\"];\n";
    let offset = source.find("array").unwrap() + "array".len();
    let hint = inferred_type_inlay_hint(source, offset, "string[]");

    match hint.label {
        InlayHintLabel::String(label) => assert_eq!(label, ": string[]"),
        other => panic!("expected string inlay hint label, got {other:?}"),
    }
    assert_eq!(hint.kind, Some(InlayHintKind::TYPE));
    assert_eq!(hint.padding_right, Some(true));
}

#[test]
fn detects_object_literal_interface_context() {
    let source = "import { Account } from \"./x\";\n\nconst main = (): void => {\n    const account: Account = {\n        \n    };\n};\n";
    let offset = source.find("= {").unwrap() + "= {\n        ".len();
    let (name, existing) = object_literal_context(source, offset).expect("object literal context");
    assert_eq!(name, "Account");
    assert!(existing.is_empty());
}

#[test]
fn object_literal_context_collects_existing_keys() {
    let source = "const a: Account = {\n    id: 1,\n    \n};\n";
    let offset = source.find("id: 1,").unwrap() + "id: 1,\n    ".len();
    let (name, existing) = object_literal_context(source, offset).expect("context");
    assert_eq!(name, "Account");
    assert!(existing.contains("id"));
}

#[test]
fn object_literal_context_ignored_outside_typed_literal() {
    let source = "const main = (): void => {\n    \n};\n";
    let offset = source.find("=> {").unwrap() + "=> {\n    ".len();
    assert!(object_literal_context(source, offset).is_none());
}

#[test]
fn object_field_items_show_modifiers_types_and_all() {
    let source = "interface Account { readonly id: i32; name: string; nickname?: string; }";
    let analysis = analyze(source);
    let fields = match analysis
        .symbol_named("Account")
        .unwrap()
        .semantic_type
        .as_ref()
    {
        Some(tyrus_core::ast::Type::Object(fields)) => fields.clone(),
        other => panic!("expected object type, got {other:?}"),
    };
    let items = object_field_completion_items(&fields, &std::collections::HashSet::new());
    let labels: Vec<_> = items.iter().map(|item| item.label.clone()).collect();
    assert_eq!(labels, vec!["all", "id", "name", "nickname?"]);

    let all = &items[0];
    assert_eq!(all.kind, Some(CompletionItemKind::SNIPPET));
    let all_text = all.insert_text.clone().unwrap();
    assert!(all_text.contains("id: ${1},"));
    assert!(all_text.contains("nickname: ${3},"));

    let id = &items[1];
    assert_eq!(id.detail.as_deref(), Some("readonly i32"));
    assert_eq!(id.insert_text.as_deref(), Some("id: "));
    let nickname = items.last().unwrap();
    assert_eq!(nickname.detail.as_deref(), Some("string"));
    assert_eq!(nickname.filter_text.as_deref(), Some("nickname"));
}

#[test]
fn object_field_items_skip_existing_keys() {
    let source = "interface Account { id: i32; name: string; }";
    let analysis = analyze(source);
    let fields = match analysis
        .symbol_named("Account")
        .unwrap()
        .semantic_type
        .as_ref()
    {
        Some(tyrus_core::ast::Type::Object(fields)) => fields.clone(),
        other => panic!("expected object type, got {other:?}"),
    };
    let mut existing = std::collections::HashSet::new();
    existing.insert("id".to_string());
    let items = object_field_completion_items(&fields, &existing);
    let labels: Vec<_> = items.iter().map(|item| item.label.clone()).collect();
    assert_eq!(labels, vec!["name"]);
}

#[test]
fn interface_object_snippet_fills_fields() {
    let source = r#"export interface Account {
    id: i32;
    name: string;
}"#;
    let analysis = analyze(source);
    let symbol = analysis.symbol_named("Account").unwrap();
    let fields = match symbol.semantic_type.as_ref() {
        Some(tyrus_core::ast::Type::Object(fields)) => fields.clone(),
        other => panic!("expected interface object type, got {other:?}"),
    };
    let item = interface_object_snippet_item("Account", &fields);
    assert_eq!(item.filter_text.as_deref(), Some("Account"));
    assert_eq!(item.insert_text_format, Some(InsertTextFormat::SNIPPET));
    let text = item.insert_text.unwrap();
    assert!(text.contains("const ${1:value}: Account = {"));
    assert!(text.contains("id: ${2},"));
    assert!(text.contains("name: ${3},"));
}

#[test]
fn interface_object_snippet_handles_empty_interface() {
    let item = interface_object_snippet_item("Empty", &[]);
    assert_eq!(
        item.insert_text.as_deref(),
        Some("const ${1:value}: Empty = {};$0")
    );
}

#[test]
fn handles_utf16_positions() {
    let source = "Hello, name";
    assert_eq!(
        byte_offset_to_position(source, source.find("name").unwrap()),
        Position::new(0, 7)
    );
    assert_eq!(
        position_to_byte_offset(source, Position::new(0, 7)),
        source.find("name").unwrap()
    );
}

#[test]
fn nested_switch_for_of_await_is_accepted_by_lsp_analysis() {
    let source = "async function step(): Promise<void> { return; }\nasync function run(flag: boolean): Promise<void> { const values: i32[] = [1]; switch (flag) { case true: { for (const value of values) { await step(); console.log(value); } break; } default: { break; } } }\n";
    let analysis = analyze(source);
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3002"),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn nested_if_loop_awaits_are_accepted_by_lsp_analysis() {
    let source = "async function step(): Promise<void> { return; }\nasync function run(values: i32[]): Promise<void> { let index: i32 = 0; while (index < 1) { if (index === 0) { await step(); } index += 1; } do { if (index === 1) { await step(); } index += 1; } while (index < 2); for (let current: i32 = 0; current < 1; current += 1) { if (current === 0) { await step(); } } for (const value of values) { if (value > 0) { await step(); } } }\n";
    let analysis = analyze(source);
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3002"),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn recursively_nested_sibling_await_paths_are_accepted_by_lsp_analysis() {
    let source = "async function step(): Promise<void> { return; }\nasync function run(flag: boolean): Promise<void> { let outer: i32 = 0; while (outer < 2) { outer += 1; let inner: i32 = 0; while (inner < 3) { inner += 1; if (inner === 1) { await step(); continue; } if (inner === 2) { await step(); } } } while (flag) { if (outer > 0) { await step(); break; } else { await step(); break; } } }\n";
    let analysis = analyze(source);
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3002"),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn independent_multi_suspend_branches_are_accepted_by_lsp_analysis() {
    let source = "async function run(flag: boolean): Promise<void> { while (flag) { await Promise.resolve(); if (flag) { const number: i32 = await Promise.resolve<i32>(7); await Promise.resolve(); const text: string = await Promise.resolve<string>(\"then\"); console.log(number); console.log(text); } else { const text: string = await Promise.resolve<string>(\"else\"); const number: i32 = await Promise.resolve<i32>(9); console.log(text); console.log(number); } await Promise.resolve(); break; } }\n";
    let analysis = analyze(source);
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3002"),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn recursive_switch_try_awaits_are_accepted_by_lsp_analysis() {
    let source = "async function step(): Promise<void> { return; }\nasync function run(values: i32[]): Promise<void> { for (const value of values) { try { switch (value) { case 1: { await step(); break; } default: { await step(); } } } catch (error: Error) { console.log(error.message); } finally { console.log(\"cleanup\"); } } }\n";
    let analysis = analyze(source);
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3002"),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn recursive_suspending_catches_are_accepted_by_lsp_analysis() {
    let source = "async function step(shouldFail: boolean): Promise<void> throws Error { if (shouldFail) { const failure: Error = { message: \"boom\", code: \"ERR_BOOM\" }; throw failure; } }\nasync function pause(): Promise<void> { return; }\nasync function run(): Promise<void> { let index: i32 = 0; while (index < 1) { index += 1; try { await step(true); } catch (error: Error) { await pause(); await pause(); console.log(error.message); } finally { console.log(\"cleanup\"); } } }\n";
    let analysis = analyze(source);
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3002"),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn recursive_suspending_finally_is_accepted_by_lsp_analysis() {
    let source = r#"
async function pause(): Promise<void> {}
async function run(): Promise<void> {
    while (true) {
        try { await pause(); break; }
        catch (error: Error) { await pause(); }
        finally { await pause(); }
    }
}
"#;
    assert!(analyze(source).diagnostics.is_empty());
}

#[test]
fn a_direct_await_inside_switch_is_accepted_by_lsp_analysis() {
    let source = "async function step(): Promise<void> { return; }\nasync function select(flag: boolean): Promise<boolean> { return flag; }\nasync function run(flag: boolean): Promise<void> { if (flag) { switch (await select(flag)) { case await select(flag): { await step(); await step(); break; } default: { await step(); break; } } } }\n";
    let analysis = analyze(source);
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3002"),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn async_try_is_accepted_by_the_lsp_analysis() {
    let source = "async function load(): Promise<void> { try { return; } catch (error: Error) { console.log(error.message); } }\n";
    let analysis = analyze(source);
    assert!(
        analysis.diagnostics.is_empty(),
        "{:#?}",
        analysis.diagnostics
    );
}

#[test]
fn deeply_cloned_string_array_rebind_is_accepted_by_lsp_analysis() {
    let source = "async function load(): Promise<void> {}\nasync function main(): Promise<void> {\nlet values: string[] = [\"before\"];\nawait load();\nvalues = values.concat([\"after\"]);\nconsole.log(values[0]);\n}\n";
    let analysis = analyze(source);
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3088"),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn owned_async_object_field_replacement_is_accepted_by_lsp_analysis() {
    let source = "interface Snapshot { label: string; tags: string[]; }\nasync function load(): Promise<void> {}\nasync function main(): Promise<void> {\nlet snapshot: Snapshot = { label: \"first\", tags: [\"static\"] };\nawait load();\nsnapshot.label = snapshot.label.toUpperCase();\nsnapshot.tags = \"owned,array\".split(\",\");\nsnapshot.tags.push(\"tail\");\nconsole.log(snapshot.tags[2]);\n}\n";
    let analysis = analyze(source);
    assert!(
        analysis.diagnostics.is_empty(),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn recursively_owned_promise_payload_delegation_is_accepted_by_lsp_analysis() {
    let source = "interface Metadata { label: string; aliases: string[]; note?: string; }\ninterface Result { status: i32; metadata: Metadata; }\nasync function source(label: string): Promise<Result> { return { status: 200, metadata: { label: label.toUpperCase(), aliases: \"ready,done\".split(\",\"), note: label } }; }\nasync function delegated(label: string): Promise<Result> { return await source(label); }\nasync function adopted(label: string): Promise<Result> { return delegated(label); }\nasync function main(): Promise<void> { const result: Result = await adopted(\"value\"); console.log(result.metadata.aliases[1]); await source(\"discarded\"); }\n";
    let analysis = analyze(source);
    assert!(
        analysis.diagnostics.is_empty(),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn recursive_async_parameter_forms_are_accepted_by_lsp_analysis() {
    let source = "interface Profile { name: string; aliases: string[]; }\nasync function pause(): Promise<void> {}\nasync function defaultProfile(value: Profile = { name: \"Default\", aliases: [\"D\"] }): Promise<string> { await pause(); return value.name; }\nasync function optionalProfile(value?: Profile): Promise<string> { await pause(); return value?.name ?? \"missing\"; }\nasync function arrayPattern([left, right]: string[]): Promise<string> { await pause(); return left + right; }\nasync function objectPattern({ name, aliases }: Profile): Promise<string> { await pause(); return name + aliases[0]; }\nasync function objectRest(...values: Profile[]): Promise<string> { await pause(); return values[0].name; }\n";
    let analysis = analyze(source);
    assert!(
        analysis.diagnostics.is_empty(),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn multiple_async_closures_with_owned_captures_are_accepted_by_lsp_analysis() {
    let source = "async function pause(): Promise<void> {}\nasync function main(): Promise<void> { const firstPrefix: string = \"first\".toUpperCase(); const secondPrefix: string = \"second\".toUpperCase(); const first = async (suffix: string): Promise<string> => { await pause(); return firstPrefix + suffix; }; const second = async (suffix: string): Promise<string> => { await pause(); return secondPrefix + suffix; }; console.log(await first(\"!\")); console.log(await second(\"?\")); }\n";
    let analysis = analyze(source);
    assert!(
        analysis.diagnostics.is_empty(),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn return_await_publishes_the_safe_direct_adoption_fix() {
    let source = "async function load(value: Promise<i32>): Promise<i32> { return await value; }\n";
    let diagnostics = tyrus_lint::lint_source(source, &tyrus_lint::LintConfig::default());
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "no-unnecessary-await")
        .unwrap_or_else(|| panic!("diagnostics: {diagnostics:#?}"));
    let fix = diagnostic
        .fix
        .as_ref()
        .expect("expected safe direct-adoption fix");
    assert_eq!(&source[fix.range.clone()], "await");
    assert_eq!(fix.replacement, "");
}

#[test]
fn comptime_dynamic_value_publishes_range_related_help_and_wrapper_fix() {
    let source = "let dynamic: i32 = 21;\nconst result: i32 = comptime(dynamic + 1);\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == tyrus_core::comptime::NON_CONSTANT)
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let dynamic_use = source.rfind("dynamic").unwrap();
    assert_eq!(diagnostic.range, dynamic_use..dynamic_use + "dynamic".len());
    assert!(!diagnostic.related.is_empty());
    assert!(!diagnostic.help.is_empty());

    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    assert_eq!(
        lsp_diagnostic.range,
        byte_range_to_lsp(source, diagnostic.range.clone())
    );
    assert!(
        lsp_diagnostic
            .related_information
            .as_ref()
            .is_some_and(|related| !related.is_empty())
    );
    let mut actions = Vec::new();
    crate::server::push_remove_comptime_wrapper_code_action(
        &mut actions,
        &uri,
        source,
        &lsp_diagnostic,
    );
    let [CodeActionOrCommand::CodeAction(action)] = actions.as_slice() else {
        panic!("expected one comptime code action");
    };
    assert_eq!(action.title, "Evaluate this expression at runtime");
    let edits = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .and_then(|changes| changes.get(&uri))
        .expect("expected wrapper edits");
    assert_eq!(edits.len(), 2);
    assert_eq!(
        edits[0].range,
        byte_range_to_lsp(
            source,
            source.find("comptime(").unwrap()
                ..source.find("comptime(").unwrap() + "comptime(".len()
        )
    );
    let closing = source.rfind(')').unwrap();
    assert_eq!(
        edits[1].range,
        byte_range_to_lsp(source, closing..closing + 1)
    );
    assert!(edits.iter().all(|edit| edit.new_text.is_empty()));
}

#[test]
fn comptime_evaluation_failure_publishes_the_cause_range_related_info_and_help() {
    let source = "const result: i32 = comptime(42 / 0);\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == tyrus_core::comptime::UNSUPPORTED_VALUE)
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let zero = source.rfind('0').unwrap();
    assert_eq!(diagnostic.range, zero..zero + 1);
    assert!(diagnostic.message.contains("division by zero"));
    assert!(
        diagnostic
            .related
            .iter()
            .any(|related| related.message.contains("wrapper"))
    );
    assert!(diagnostic.help.iter().any(|help| help.contains("non-zero")));

    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    assert_eq!(
        lsp_diagnostic.range,
        byte_range_to_lsp(source, zero..zero + 1)
    );
    assert!(
        lsp_diagnostic
            .related_information
            .as_ref()
            .is_some_and(|related| !related.is_empty())
    );
}

#[test]
fn lint_rules_reuse_resolved_editor_analysis_for_native_imports() {
    let source = "import { styleText } from \"tyrus:utils\";\n\n\
                  export const runStyleTextExamples = (): void => {\n\
                      const banner: string = \"Ready\" |> styleText.greenBright.bold;\n\
                      console.log(banner);\n\
                  };\n";
    let mut import_types = tyrus_core::collections::HashMap::default();
    import_types.insert(
        "styleText".to_string(),
        Type::Named(tyrus_core::style_text::STYLE_TEXT_MARKER.to_string()),
    );
    let analysis = tyrus_core::analysis::analyze_with_imports(
        source,
        &import_types,
        &tyrus_core::collections::HashMap::default(),
        &tyrus_core::collections::HashMap::default(),
    );
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E0599"),
        "analysis diagnostics: {:#?}",
        analysis.diagnostics
    );

    let diagnostics = tyrus_lint::lint_rules_source_with_analysis(
        source,
        &analysis,
        &tyrus_lint::LintConfig::default(),
    );
    assert!(diagnostics.is_empty(), "lint diagnostics: {diagnostics:#?}");
}

#[test]
fn completes_http_router_methods() {
    assert_eq!(
        member_completion_receiver("app.", "app.".len()).as_deref(),
        Some("app")
    );
    assert_eq!(
        member_completion_receiver("app?.", "app?.".len()).as_deref(),
        Some("app")
    );
    let items = http_router_completion_items();
    assert_eq!(items.len(), 18);
    for method in ["ws", "docs"] {
        assert!(
            items.iter().any(|item| item.label == method),
            "`{method}` is a router method and must be offered"
        );
    }
    for policy in ["cors", "fetchPolicy", "rateLimit", "trustedProxies"] {
        assert!(
            items.iter().any(|item| item.label == policy),
            "`{policy}` is a server policy and belongs on the router"
        );
    }
    assert!(
        !items.iter().any(|item| item.label == "securityHeaders"),
        "security headers are written with `reply.header`, not by a router method"
    );
    let start = items
        .iter()
        .find(|item| item.label == "start")
        .expect("`start` is part of the router surface");
    assert_eq!(
        start.detail.as_deref(),
        Some("(port: i32, options?: ServerOptions) => HttpServer")
    );
    let trusted = items
        .iter()
        .find(|item| item.label == "trustedProxies")
        .expect("trustedProxies is part of the router surface");
    assert_eq!(
        trusted.detail.as_deref(),
        Some("(proxies: string[], identityHeader?: string) => void")
    );
    let fetch_policy = items
        .iter()
        .find(|item| item.label == "fetchPolicy")
        .expect("fetchPolicy is part of the router surface");
    assert_eq!(
        fetch_policy.detail.as_deref(),
        Some("(options: FetchPolicyOptions) => void")
    );
    let get = items.iter().find(|item| item.label == "get").unwrap();
    assert_eq!(get.kind, Some(CompletionItemKind::METHOD));
    assert_eq!(
        get.detail.as_deref(),
        Some("(path: string, options: HttpRouteOptions) => void")
    );
    assert_eq!(
        get.insert_text.as_deref(),
        Some("get(${1:path}, ${2:options})")
    );
    let query = items.iter().find(|item| item.label == "query").unwrap();
    assert_eq!(query.kind, Some(CompletionItemKind::METHOD));
    assert_eq!(
        query.detail.as_deref(),
        Some("(path: string, options: HttpRouteOptions) => void")
    );
    assert_eq!(
        query.insert_text.as_deref(),
        Some("query(${1:path}, ${2:options})")
    );
}

#[test]
fn completes_console_methods_as_statement_snippets() {
    let source = "console.";
    let analysis = analyze(source);
    let items = member_completion_items(source, source.len(), &analysis).unwrap();
    let log = items.iter().find(|item| item.label == "log").unwrap();
    assert_eq!(log.kind, Some(CompletionItemKind::METHOD));
    assert_eq!(log.insert_text.as_deref(), Some("log($0);"));
    assert_eq!(log.insert_text_format, Some(InsertTextFormat::SNIPPET));
}

#[test]
fn completes_array_methods() {
    let items = array_completion_items();
    assert_eq!(items.len(), 46);
    let equals = items.iter().find(|item| item.label == "equals").unwrap();
    assert_eq!(equals.detail.as_deref(), Some("(other: T[]) => boolean"));
    assert_eq!(equals.insert_text.as_deref(), Some("equals(${1:other})"));
    let push = items.iter().find(|item| item.label == "push").unwrap();
    assert_eq!(push.kind, Some(CompletionItemKind::METHOD));
    assert_eq!(push.insert_text.as_deref(), Some("push(${1:item})"));
    let zip = items.iter().find(|item| item.label == "zip").unwrap();
    assert_eq!(zip.detail.as_deref(), Some("<U>(other: U[]) => [T, U][]"));
    assert_eq!(zip.insert_text.as_deref(), Some("zip(${1:other})"));
    for removal in ["pop", "shift"] {
        let item = items.iter().find(|item| item.label == removal).unwrap();
        assert_eq!(item.detail.as_deref(), Some("() => T?"));
        let expected = format!("{removal}()");
        assert_eq!(item.insert_text.as_deref(), Some(expected.as_str()));
    }
    let random = items.iter().find(|item| item.label == "random").unwrap();
    assert_eq!(random.detail.as_deref(), Some("() => T?"));
    assert_eq!(random.insert_text.as_deref(), Some("random()"));
    let shuffle = items.iter().find(|item| item.label == "shuffle").unwrap();
    assert_eq!(shuffle.detail.as_deref(), Some("() => T[]"));
    assert_eq!(shuffle.insert_text.as_deref(), Some("shuffle()"));
    let sample = items.iter().find(|item| item.label == "sample").unwrap();
    assert_eq!(
        sample.detail.as_deref(),
        Some("(count: usize, options?: { repeat?: boolean }) => T[]")
    );
    assert_eq!(
        sample.insert_text.as_deref(),
        Some("sample(${1:count}, ${2:options})")
    );
    let map = items.iter().find(|item| item.label == "map").unwrap();
    assert!(map.detail.as_deref().unwrap().contains("<U>"));
    assert_eq!(map.insert_text.as_deref(), Some("map(${1:callback})"));
    assert_eq!(
        items
            .iter()
            .find(|item| item.label == "capacity")
            .unwrap()
            .kind,
        Some(CompletionItemKind::FIELD)
    );
}

#[test]
fn hides_mutating_methods_on_readonly_arrays() {
    let items = non_mutating_array_completion_items();
    for mutating in [
        "push",
        "pop",
        "shift",
        "unshift",
        "splice",
        "sort",
        "reverse",
        "fill",
        "copyWithin",
    ] {
        assert!(
            !items.iter().any(|item| item.label == mutating),
            "readonly array completion should not offer `{mutating}`"
        );
    }
    for keep in [
        "includes",
        "map",
        "filter",
        "toReversed",
        "toSorted",
        "slice",
    ] {
        assert!(
            items.iter().any(|item| item.label == keep),
            "readonly array completion should still offer `{keep}`"
        );
    }
}

#[test]
fn offers_the_non_mutating_array_surface_on_a_uniform_tuple() {
    let items = member_completion_items_for_type(&Type::Tuple(vec![Type::I32, Type::I32])).unwrap();
    assert!(items.iter().any(|item| item.label == "includes"));
    assert!(items.iter().any(|item| item.label == "map"));
    assert!(!items.iter().any(|item| item.label == "push"));
    assert!(!items.iter().any(|item| item.label == "sort"));
}

#[test]
fn offers_no_surface_on_a_mixed_tuple() {
    assert!(member_completion_items_for_type(&Type::Tuple(vec![Type::I32, Type::Str])).is_none());
}

#[test]
fn completes_segment_tree_lazy_methods() {
    let items = member_completion_items_for_type(&Type::SegmentTreeLazy).unwrap();
    let update_range = items
        .iter()
        .find(|item| item.label == "updateRange")
        .unwrap();
    assert_eq!(
        update_range.detail.as_deref(),
        Some("(from: usize, to: usize, delta: i32) => void")
    );
    assert_eq!(
        update_range.insert_text.as_deref(),
        Some("updateRange(${1:from}, ${2:to}, ${3:delta})")
    );
    assert!(items.iter().any(|item| item.label == "query"));
    assert!(items.iter().any(|item| item.label == "len"));
    assert!(items.iter().any(|item| item.label == "isEmpty"));
}

#[test]
fn completes_sparse_table_methods() {
    let items = member_completion_items_for_type(&Type::SparseTable).unwrap();
    let query = items.iter().find(|item| item.label == "query").unwrap();
    assert_eq!(
        query.detail.as_deref(),
        Some("(from: usize, to: usize) => i32")
    );
    assert_eq!(
        query.insert_text.as_deref(),
        Some("query(${1:from}, ${2:to})")
    );
    assert!(items.iter().any(|item| item.label == "len"));
    assert!(items.iter().any(|item| item.label == "isEmpty"));
    assert!(!items.iter().any(|item| item.label == "update"));
}

#[test]
fn completes_trie_methods() {
    let items = member_completion_items_for_type(&Type::Trie).unwrap();
    let insert = items.iter().find(|item| item.label == "insert").unwrap();
    assert_eq!(insert.detail.as_deref(), Some("(word: string) => void"));
    assert_eq!(insert.insert_text.as_deref(), Some("insert(${1:word})"));
    let starts_with = items
        .iter()
        .find(|item| item.label == "startsWith")
        .unwrap();
    assert_eq!(
        starts_with.detail.as_deref(),
        Some("(prefix: string) => boolean")
    );
    assert_eq!(
        starts_with.insert_text.as_deref(),
        Some("startsWith(${1:prefix})")
    );
    assert!(items.iter().any(|item| item.label == "search"));
    assert!(items.iter().any(|item| item.label == "len"));
    assert!(items.iter().any(|item| item.label == "isEmpty"));
    assert!(items.iter().any(|item| item.label == "clear"));
}

#[test]
fn completes_suffix_tree_methods() {
    let items = member_completion_items_for_type(&Type::SuffixTree).unwrap();
    let get_suffix_array = items
        .iter()
        .find(|item| item.label == "getSuffixArray")
        .unwrap();
    assert_eq!(get_suffix_array.detail.as_deref(), Some("() => usize[]"));
    assert_eq!(
        get_suffix_array.insert_text.as_deref(),
        Some("getSuffixArray()")
    );
    assert_eq!(items.len(), 1);
}

#[test]
fn completes_lca_methods() {
    let items = member_completion_items_for_type(&Type::Lca).unwrap();
    let query = items.iter().find(|item| item.label == "query").unwrap();
    assert_eq!(
        query.detail.as_deref(),
        Some("(u: usize, v: usize) => usize")
    );
    assert_eq!(query.insert_text.as_deref(), Some("query(${1:u}, ${2:v})"));
    assert_eq!(items.len(), 1);
}

#[test]
fn completes_hld_methods() {
    let items = member_completion_items_for_type(&Type::Hld).unwrap();
    let query_path = items.iter().find(|item| item.label == "queryPath").unwrap();
    assert_eq!(
        query_path.detail.as_deref(),
        Some("(u: usize, v: usize) => [usize, usize][]")
    );
    assert_eq!(
        query_path.insert_text.as_deref(),
        Some("queryPath(${1:u}, ${2:v})")
    );
    let position = items.iter().find(|item| item.label == "position").unwrap();
    assert_eq!(position.detail.as_deref(), Some("(v: usize) => usize"));
    assert_eq!(position.insert_text.as_deref(), Some("position(${1:v})"));
    let head = items.iter().find(|item| item.label == "head").unwrap();
    assert_eq!(head.detail.as_deref(), Some("(v: usize) => usize"));
    assert_eq!(head.insert_text.as_deref(), Some("head(${1:v})"));
    assert_eq!(items.len(), 3);
}

#[test]
fn completes_centroid_methods() {
    let items = member_completion_items_for_type(&Type::Centroid).unwrap();
    let parent_of = items.iter().find(|item| item.label == "parentOf").unwrap();
    assert_eq!(parent_of.detail.as_deref(), Some("(v: usize) => usize?"));
    assert_eq!(parent_of.insert_text.as_deref(), Some("parentOf(${1:v})"));
    let depth = items.iter().find(|item| item.label == "depth").unwrap();
    assert_eq!(depth.detail.as_deref(), Some("(v: usize) => usize"));
    assert_eq!(depth.insert_text.as_deref(), Some("depth(${1:v})"));
    assert_eq!(items.len(), 2);
}

#[test]
fn completes_array_static_methods() {
    let source = "const result = Array.";
    let analysis = analyze(source);
    let items = member_completion_items(source, source.len(), &analysis).unwrap();
    let is_array = items.iter().find(|item| item.label == "isArray").unwrap();
    assert_eq!(is_array.kind, Some(CompletionItemKind::METHOD));
    assert_eq!(is_array.detail.as_deref(), Some("(value) => boolean"));
    assert_eq!(is_array.insert_text.as_deref(), Some("isArray(${1:value})"));
}

#[test]
fn completes_array_methods_for_inferred_array_literals() {
    let source = "const values = [1, 2, 3];\nvalues.";
    let analysis = analyze(source);
    let items = member_completion_items(source, source.len(), &analysis).unwrap();

    for expected in ["map", "filter", "reduce", "find"] {
        assert!(
            items.iter().any(|item| item.label == expected),
            "missing array method completion `{expected}`"
        );
    }
}

#[test]
fn completes_string_members() {
    let items = string_completion_items();
    let length = items.iter().find(|item| item.label == "length").unwrap();
    assert_eq!(length.kind, Some(CompletionItemKind::FIELD));
    assert_eq!(length.detail.as_deref(), Some("i64"));

    let replace = items.iter().find(|item| item.label == "replace").unwrap();
    assert_eq!(replace.kind, Some(CompletionItemKind::METHOD));
    assert_eq!(
        replace.insert_text.as_deref(),
        Some("replace(${1:search}, ${2:replacement})")
    );
    let trim = items.iter().find(|item| item.label == "trim").unwrap();
    assert_eq!(trim.insert_text.as_deref(), Some("trim()"));

    let match_item = items.iter().find(|item| item.label == "match").unwrap();
    assert_eq!(
        match_item.detail.as_deref(),
        Some("(pattern: Regex) => string[]?")
    );
    assert_eq!(
        match_item.insert_text.as_deref(),
        Some("match(${1:pattern})")
    );
    let split = items.iter().find(|item| item.label == "split").unwrap();
    assert_eq!(
        split.detail.as_deref(),
        Some("(separator: string | Regex) => string[]")
    );
}

#[test]
fn completes_regexp_members() {
    let items = regex_completion_items();
    let exec = items.iter().find(|item| item.label == "exec").unwrap();
    assert_eq!(exec.kind, Some(CompletionItemKind::METHOD));
    assert_eq!(exec.detail.as_deref(), Some("(input: string) => string[]?"));
    assert_eq!(exec.insert_text.as_deref(), Some("exec(${1:input})"));

    let source = items.iter().find(|item| item.label == "source").unwrap();
    assert_eq!(source.kind, Some(CompletionItemKind::FIELD));
    assert_eq!(source.detail.as_deref(), Some("string"));
    let last_index = items.iter().find(|item| item.label == "lastIndex").unwrap();
    assert_eq!(last_index.detail.as_deref(), Some("i32"));
}

#[test]
fn completes_keywords_literals_and_types() {
    let keywords = keyword_completion_items();
    for expected in [
        "type",
        "match",
        "ifelse",
        "forof",
        "forin",
        "trycatch",
        "readonly",
        "of",
        "new",
        "None",
        "true",
        "async",
        "await",
        "asyncfn",
        "asyncconst",
    ] {
        assert!(
            keywords.iter().any(|item| item.label == expected),
            "missing keyword completion `{expected}`"
        );
    }
    let if_item = keywords.iter().find(|item| item.label == "if").unwrap();
    assert_eq!(if_item.kind, Some(CompletionItemKind::SNIPPET));
    assert_eq!(
        if_item.insert_text.as_deref(),
        Some("if (${1:condition}) {\n    $0\n}")
    );
    let ifelse = keywords.iter().find(|item| item.label == "ifelse").unwrap();
    assert_eq!(ifelse.kind, Some(CompletionItemKind::SNIPPET));
    assert!(
        ifelse
            .insert_text
            .as_deref()
            .is_some_and(|text| text.contains("} else {"))
    );
    let forin = keywords.iter().find(|item| item.label == "forin").unwrap();
    assert_eq!(
        forin.insert_text.as_deref(),
        Some("for (const ${1:key} in ${2:object}) {\n    $0\n}")
    );
    let match_item = keywords.iter().find(|item| item.label == "match").unwrap();
    assert_eq!(match_item.kind, Some(CompletionItemKind::SNIPPET));
    assert_eq!(
        match_item.insert_text.as_deref(),
        Some(
            "match (${1:value}) {\n    ${2:case} => {\n        ${3}\n    }\n    _ => {\n        $0\n    }\n}"
        )
    );
    let asyncfn = keywords
        .iter()
        .find(|item| item.label == "asyncfn")
        .unwrap();
    assert_eq!(asyncfn.kind, Some(CompletionItemKind::SNIPPET));
    assert_eq!(
        asyncfn.insert_text.as_deref(),
        Some("async function ${1:name}(${2}): Promise<${3:void}> {\n    $0\n}")
    );
    let asyncconst = keywords
        .iter()
        .find(|item| item.label == "asyncconst")
        .unwrap();
    assert_eq!(asyncconst.kind, Some(CompletionItemKind::SNIPPET));
    assert_eq!(
        asyncconst.insert_text.as_deref(),
        Some("const ${1:name} = async (${2}): Promise<${3:void}> => {\n    $0\n};")
    );
    let console = keywords
        .iter()
        .find(|item| item.label == "console.log")
        .unwrap();
    assert_eq!(console.kind, Some(CompletionItemKind::FUNCTION));
    assert_eq!(console.insert_text.as_deref(), Some("console.log($0);"));

    let source = r#"
        interface User { name: string; }
    "#;
    let analysis = tyrus_core::analysis::analyze(source);
    let types = type_completion_items(source, analysis.program.as_ref());
    let result = types.iter().find(|item| item.label == "Result").unwrap();
    assert_eq!(
        result.detail.as_deref(),
        Some("success-or-error result type")
    );
    for expected in [
        "i32",
        "i128",
        "u32",
        "u64",
        "u128",
        "f128",
        "decimal",
        "string",
        "unknown",
        "never",
        "Array",
        "Promise",
        "Error",
        "AggregateError",
        "HttpRouter",
        "User",
        "Result",
    ] {
        assert!(
            types.iter().any(|item| item.label == expected),
            "missing type completion `{expected}`"
        );
    }
}

#[test]
fn promise_calls_expose_native_signature_help_metadata() {
    let source = "const failure: Error = { message: \"boom\", code: \"ERR_BOOM\" }; Promise.reject(failure);";
    let analysis = analyze(source);
    let call = analysis
        .call_sites
        .iter()
        .find(|call| call.callee == "Promise.reject")
        .unwrap();
    assert_eq!(call.arguments.len(), 1);
    let signature = crate::backend::native_promise_call_signature(&call.callee).unwrap();
    assert_eq!(signature.parameters, vec!["reason"]);
    assert!(signature.detail.unwrap().contains("reason: Error"));
    let then = crate::backend::native_promise_call_signature("then").unwrap();
    assert_eq!(then.parameters, vec!["onFulfilled"]);
    assert!(then.detail.unwrap().contains("Promise<U>"));
}

#[test]
fn direct_fixed_array_simd_arguments_have_no_dynamic_array_diagnostic() {
    let source = "import { simdAdd, SimdError } from \"tyrus:simd\";\n\
const main = (): void => {\n\
    const left: [i32; 5] = [1, 2, 3, 4, 5];\n\
    const right: [i32; 5] = [10, 20, 30, 40, 50];\n\
    const result: Result<i32[], SimdError> = simdAdd(left, right);\n\
};\n";
    let mut types = tyrus_core::collections::HashMap::default();
    let mut interfaces = tyrus_core::collections::HashMap::default();
    let mut targets = tyrus_core::collections::HashMap::default();
    for name in ["simdAdd", "SimdError"] {
        let (ty, fields, _) = tyrus_core::module_loader::native_export_info("tyrus:simd", name)
            .expect("SIMD native export metadata");
        types.insert(name.to_string(), ty);
        if let Some(fields) = fields {
            interfaces.insert(name.to_string(), fields);
        }
        if let Some(target) = tyrus_core::module_loader::native_export_target("tyrus:simd", name) {
            targets.insert(name.to_string(), target);
        }
    }
    let analysis = tyrus_core::analysis::analyze_with_context(
        source,
        tyrus_core::analysis::AnalyzeContext {
            import_types: Some(&types),
            import_targets: Some(&targets),
            import_interfaces: Some(&interfaces),
            ..tyrus_core::analysis::AnalyzeContext::default()
        },
    );
    assert!(
        analysis.diagnostics.iter().all(|diagnostic| {
            !diagnostic.code.eq("E0308") || !diagnostic.message.contains("dynamic array")
        }),
        "fixed-size SIMD arguments should not publish a dynamic-array diagnostic: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn fixed_size_simd_length_mismatch_publishes_the_second_argument_range_and_help() {
    let source = "import { simdAdd, SimdError } from \"tyrus:simd\";\n\
const main = (): void => {\n\
    const left: [i32; 5] = [1, 2, 3, 4, 5];\n\
    const right: [i32; 4] = [10, 20, 30, 40];\n\
    const result: Result<i32[], SimdError> = simdAdd(left, right);\n\
    print(right);\n\
};\n";
    let mut types = tyrus_core::collections::HashMap::default();
    let mut interfaces = tyrus_core::collections::HashMap::default();
    let mut targets = tyrus_core::collections::HashMap::default();
    for name in ["simdAdd", "SimdError"] {
        let (ty, fields, _) = tyrus_core::module_loader::native_export_info("tyrus:simd", name)
            .expect("SIMD native export metadata");
        types.insert(name.to_string(), ty);
        if let Some(fields) = fields {
            interfaces.insert(name.to_string(), fields);
        }
        if let Some(target) = tyrus_core::module_loader::native_export_target("tyrus:simd", name) {
            targets.insert(name.to_string(), target);
        }
    }
    let analysis = tyrus_core::analysis::analyze_with_context(
        source,
        tyrus_core::analysis::AnalyzeContext {
            import_types: Some(&types),
            import_targets: Some(&targets),
            import_interfaces: Some(&interfaces),
            ..tyrus_core::analysis::AnalyzeContext::default()
        },
    );
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "E0308" && diagnostic.message.contains("fixed length")
        })
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let argument_start = source.rfind("left, right").unwrap() + "left, ".len();
    let argument_range = argument_start..argument_start + "right".len();
    assert_eq!(diagnostic.range, argument_range);
    assert!(!diagnostic.help.is_empty());

    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let published = diagnostic_to_lsp(&uri, source, diagnostic);
    assert_eq!(published.range, byte_range_to_lsp(source, argument_range));
}

#[test]
fn simd_native_metadata_exposes_generic_signatures_parameters_and_examples() {
    for (name, local, example) in [
        (
            "simdAdd",
            "add",
            "const result: Result<i32[], SimdError> = add([1, 2], [3, 4]);",
        ),
        (
            "simdSub",
            "sub",
            "const result: Result<i32[], SimdError> = sub([4, 3], [1, 2]);",
        ),
        (
            "simdMul",
            "mul",
            "const result: Result<i32[], SimdError> = mul([2, 3], [4, 5]);",
        ),
        (
            "simdDiv",
            "div",
            "const result: Result<i32[], SimdError> = div([8, 9], [2, 3]);",
        ),
    ] {
        assert_eq!(
            crate::native::native_import_detail("tyrus:simd", name),
            Some("<T>(left: T[], right: T[]) => Result<T[], SimdError>")
        );
        assert_eq!(
            crate::native::native_parameter_names("tyrus:simd", name),
            Some(vec!["left".to_string(), "right".to_string()])
        );
        assert_eq!(
            crate::native::native_import_example("tyrus:simd", name, local),
            Some(example.to_string())
        );
        assert!(matches!(
            crate::native::native_import_type("tyrus:simd", name),
            Some(tyrus_core::ast::Type::GenericFunction { .. })
        ));
        assert!(crate::native::native_exports().iter().any(|export| {
            export.name == name
                && export.kind == SymbolKind::Function
                && export.uri.as_str() == "tyrus:simd"
        }));
    }
}

#[test]
fn task_sleep_native_metadata_exposes_synchronous_signature() {
    assert_eq!(
        crate::native::native_import_detail("tyrus:task", "sleep"),
        Some("(ms: u64) => void")
    );
    assert_eq!(
        crate::native::native_parameter_names("tyrus:task", "sleep"),
        Some(vec!["ms".to_string()])
    );
    assert_eq!(
        crate::native::native_import_example("tyrus:task", "sleep", "delay"),
        Some("delay(500);".to_string())
    );
}

#[test]
fn task_sleep_async_native_metadata_exposes_promise_signature() {
    assert_eq!(
        crate::native::native_import_detail("tyrus:task", "sleepAsync"),
        Some("(ms: u64) => Promise<void>")
    );
    assert_eq!(
        crate::native::native_parameter_names("tyrus:task", "sleepAsync"),
        Some(vec!["ms".to_string()])
    );
    assert_eq!(
        crate::native::native_import_example("tyrus:task", "sleepAsync", "delay"),
        Some("await delay(500);".to_string())
    );
}

#[test]
fn tasks_cancel_native_metadata_exposes_generic_promise_signature() {
    assert_eq!(
        crate::native::native_import_detail("tyrus:task", "cancel"),
        Some("<T>(promise: Promise<T>, reason: string) => boolean")
    );
    assert_eq!(
        crate::native::native_parameter_names("tyrus:task", "cancel"),
        Some(vec!["promise".to_string(), "reason".to_string()])
    );
    assert_eq!(
        crate::native::native_import_example("tyrus:task", "cancel", "stop"),
        Some("const cancelled: boolean = stop(operation, \"Cancelled by caller\");".to_string())
    );
}

#[test]
fn tasks_spawn_native_metadata_exposes_generic_join_handle_signature() {
    assert_eq!(
        crate::native::native_import_detail("tyrus:task", "spawn"),
        Some("<T>(task: () => Promise<T>) => JoinHandle<T>")
    );
    assert_eq!(
        crate::native::native_parameter_names("tyrus:task", "spawn"),
        Some(vec!["task".to_string()])
    );
    assert!(
        crate::native::native_import_example("tyrus:task", "spawn", "run")
            .unwrap()
            .contains("run(async")
    );
}

#[test]
fn rejects_empty_or_synthetic_inlay_argument_ranges() {
    let source = "const main = (): void => {\n    formatMessage(message);\n};";
    let valid = source.find("message").unwrap()..source.find("message").unwrap() + "message".len();
    let synthetic = source.len()..source.len();

    assert!(is_valid_inlay_argument_range(
        source,
        &valid,
        0,
        source.len()
    ));
    assert!(!is_valid_inlay_argument_range(
        source,
        &synthetic,
        0,
        source.len()
    ));
    assert!(!is_valid_inlay_argument_range(
        source,
        &(10..10),
        0,
        source.len()
    ));

    let closing_brace = source.find("};").unwrap()..source.find("};").unwrap() + 1;
    assert!(!is_valid_inlay_argument_range(
        source,
        &closing_brace,
        0,
        source.len()
    ));
}

#[test]
fn builds_relative_module_paths() {
    let current = Path::new("/workspace/src/main.tyr");
    let target = Path::new("/workspace/src/functions/greetings.tyr");
    assert_eq!(
        relative_module_source(current, target).as_deref(),
        Some("./functions/greetings")
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn exposes_native_module_signatures() {
    assert_eq!(
        native_import_type("tyrus:fs", "writeFileSync")
            .unwrap()
            .to_string(),
        "(string, string, string) => void"
    );
    assert_eq!(
        native_import_detail("tyrus:fs", "writeFileSync"),
        Some("(path: string, data: string, encoding: string) => void")
    );
    assert!(native_import_type("tyrus:fs", "missing").is_none());
    assert_eq!(
        native_import_detail("tyrus:http", "createRouter"),
        Some("() => HttpRouter")
    );
    assert_eq!(
        native_import_type("tyrus:http", "HttpStatus")
            .unwrap()
            .to_string(),
        "HttpStatus"
    );
    assert_eq!(
        native_import_detail("tyrus:http", "HttpStatus"),
        Some("enum HttpStatus")
    );
    assert_eq!(
        native_import_type("tyrus:http", "HttpReply")
            .unwrap()
            .to_string(),
        "HttpReply"
    );
    assert_eq!(
        native_import_detail("tyrus:http", "HttpReply"),
        Some("type HttpReply<T>")
    );
    assert_eq!(
        native_import_type("tyrus:http", "HttpRouter")
            .unwrap()
            .to_string(),
        "HttpRouter"
    );
    assert_eq!(
        native_import_detail("tyrus:http", "HttpRouter"),
        Some("type HttpRouter")
    );
    for name in [
        "HttpRequest",
        "HttpResponse",
        "CorsOptions",
        "CookieOptions",
    ] {
        let detail = format!("interface {name}");
        assert_eq!(
            native_import_type("tyrus:http", name).unwrap().to_string(),
            name
        );
        assert_eq!(
            native_import_detail("tyrus:http", name),
            Some(detail.as_str())
        );
    }
    assert!(native_import_type("tyrus:http", "ErrorResponse").is_none());
    assert!(native_import_detail("tyrus:http", "ErrorResponse").is_none());
    assert!(native_import_type("tyrus:http", "serve").is_none());
    assert!(native_import_detail("tyrus:http", "serve").is_none());
    assert_eq!(
        native_import_detail("tyrus:http", "connectWebSocket"),
        Some("(url: string, protocols?: string[]) => Promise<Result<WebSocket, HttpError>>")
    );
    assert_eq!(
        native_import_detail("tyrus:http", "fetch"),
        Some(
            "(url: string, options?: FetchOptions) => Promise<HttpClientResponse>; streaming: fetch.stream(url, options?) => Promise<HttpClientStreamResponse>; verbs: fetch.get, fetch.post, fetch.put, fetch.patch, fetch.delete, fetch.head, fetch.options, fetch.connect, fetch.query, fetch.trace"
        )
    );
    assert_eq!(
        native_import_type("tyrus:net", "ping").unwrap().to_string(),
        "(string, u64?) => Promise<Result<u64, NetError>>"
    );
    assert_eq!(
        native_import_detail("tyrus:net", "UdpSocket"),
        Some("type UdpSocket")
    );
    assert_eq!(
        native_import_detail("tyrus:net", "tcpConnect"),
        Some("(address: string, timeoutMs?: u64) => Promise<Result<TcpStream, NetError>>")
    );
    assert_eq!(
        native_import_detail("tyrus:net", "udpBind"),
        Some("(address: string) => Promise<Result<UdpSocket, NetError>>")
    );
    assert_eq!(
        native_import_detail("tyrus:net", "ping"),
        Some("(address: string, timeoutMs?: u64) => Promise<Result<u64, NetError>>")
    );
    assert_eq!(
        crate::native::native_parameter_names("tyrus:net", "ping"),
        Some(vec!["address".to_string(), "timeoutMs".to_string()])
    );
    assert_eq!(
        native_import_type("tyrus:net", "NetError")
            .unwrap()
            .to_string(),
        "NetError"
    );
    assert_eq!(
        native_import_type("tyrus:temporal", "PlainDate")
            .unwrap()
            .to_string(),
        "PlainDate"
    );
    assert_eq!(
        native_import_detail("tyrus:temporal", "plainDateFrom"),
        Some("(value: string) => Result<PlainDate, TemporalError>")
    );
    assert_eq!(
        native_import_detail("tyrus:temporal", "instantFrom"),
        Some("(value: string) => Result<Instant, TemporalError>")
    );
    assert_eq!(
        native_import_type("tyrus:temporal", "plainDateAddDays")
            .unwrap()
            .to_string(),
        "(PlainDate, i64) => Result<PlainDate, TemporalError>"
    );
    assert_eq!(
        native_import_detail("tyrus:temporal", "plainDateDaysUntil"),
        Some("(start: PlainDate, end: PlainDate) => Result<i64, TemporalError>")
    );
    assert_eq!(
        native_import_detail("tyrus:temporal", "plainDateDayOfWeek"),
        Some("(date: PlainDate) => Result<u32, TemporalError>")
    );
    assert_eq!(
        native_import_type("tyrus:temporal", "PlainTime")
            .unwrap()
            .to_string(),
        "PlainTime"
    );
    assert_eq!(
        native_import_detail("tyrus:temporal", "plainTimeFrom"),
        Some("(value: string) => Result<PlainTime, TemporalError>")
    );
    assert_eq!(
        native_import_detail("tyrus:temporal", "plainTimeSecondsUntil"),
        Some("(start: PlainTime, end: PlainTime) => Result<i64, TemporalError>")
    );
    assert_eq!(
        native_import_detail("tyrus:temporal", "plainTimeAddNanoseconds"),
        Some("(time: PlainTime, nanoseconds: i64) => Result<PlainTime, TemporalError>")
    );
    let base64_decode =
        crate::native::native_namespace_member_documentation("tyrus:encoding", "base64", "decode")
            .unwrap();
    assert!(base64_decode.contains("returns `Err(EncodingError)`"));
    assert!(!base64_decode.contains("throws"));
    assert!(
        native_enum_member_names("tyrus:temporal", "TimeZone")
            .unwrap()
            .iter()
            .any(|name| name == "Europe_Paris")
    );
    assert_eq!(
        native_import_type("tyrus:cli", "multiselect")
            .unwrap()
            .to_string(),
        "(MultiSelectOptions) => string[]"
    );
    assert_eq!(
        native_import_detail("tyrus:cli", "confirm"),
        Some("(options: ConfirmOptions) => boolean")
    );
    assert_eq!(
        native_import_type("tyrus:cli", "isTTY")
            .unwrap()
            .to_string(),
        "() => boolean"
    );
    assert_eq!(
        native_import_detail("tyrus:cli", "isTTY"),
        Some("() => boolean - true when stdout is an interactive terminal")
    );
    assert_eq!(
        native_import_type("tyrus:utils", "figlet")
            .unwrap()
            .to_string(),
        "__Figlet"
    );
    assert_eq!(
        native_import_detail("tyrus:utils", "figlet"),
        Some("(text: string, font?: \"standard\" | \"small\" | \"big\" | \"slant\") => string")
    );
    assert_eq!(
        native_import_type("tyrus:algorithm", "suffixArray")
            .unwrap()
            .to_string(),
        "(string) => usize[]"
    );
    assert_eq!(
        native_import_detail("tyrus:algorithm", "suffixArray"),
        Some(
            "(text: string) => usize[], starting indices of every suffix sorted lexicographically (direct O(n² log n) construction)"
        )
    );
    assert_eq!(
        native_import_example("tyrus:algorithm", "suffixArray", "suffixArray").as_deref(),
        Some("const indices: usize[] = suffixArray(\"banana\");")
    );
    assert_eq!(
        native_import_type("tyrus:algorithm", "suffixTreeUkkonen")
            .unwrap()
            .to_string(),
        "(string) => SuffixTree"
    );
    assert_eq!(
        native_import_detail("tyrus:algorithm", "suffixTreeUkkonen"),
        Some(
            "(text: string) => SuffixTree, suffix tree over a string built with Ukkonen's linear-time algorithm (getSuffixArray)"
        )
    );
    assert_eq!(
        native_import_type("tyrus:algorithm", "SuffixTree")
            .unwrap()
            .to_string(),
        "SuffixTree"
    );
    assert_eq!(
        native_import_example("tyrus:algorithm", "suffixTreeUkkonen", "suffixTreeUkkonen")
            .as_deref(),
        Some("const tree: SuffixTree = suffixTreeUkkonen(\"banana\");")
    );
    assert_eq!(
        native_import_type("tyrus:algorithm", "lcaBinaryLifting")
            .unwrap()
            .to_string(),
        "(usize[][], usize) => Lca"
    );
    assert_eq!(
        native_import_detail("tyrus:algorithm", "lcaBinaryLifting"),
        Some(
            "(adj: usize[][], root: usize) => Lca, lowest common ancestor tables built with binary lifting (query)"
        )
    );
    assert_eq!(
        native_import_type("tyrus:algorithm", "Lca")
            .unwrap()
            .to_string(),
        "Lca"
    );
    assert_eq!(
        native_import_example("tyrus:algorithm", "lcaBinaryLifting", "lcaBinaryLifting").as_deref(),
        Some("const lca: Lca = lcaBinaryLifting([[1], [0, 2], [1]], 0);")
    );
    assert_eq!(
        native_import_type("tyrus:algorithm", "heavyLightDecomposition")
            .unwrap()
            .to_string(),
        "(usize[][], usize) => Hld"
    );
    assert_eq!(
        native_import_detail("tyrus:algorithm", "heavyLightDecomposition"),
        Some(
            "(adj: usize[][], root: usize) => Hld, heavy-light decomposition over a tree (queryPath, position, head)"
        )
    );
    assert_eq!(
        native_import_type("tyrus:algorithm", "Hld")
            .unwrap()
            .to_string(),
        "Hld"
    );
    assert_eq!(
        native_import_example(
            "tyrus:algorithm",
            "heavyLightDecomposition",
            "heavyLightDecomposition"
        )
        .as_deref(),
        Some("const hld: Hld = heavyLightDecomposition([[1], [0, 2], [1]], 0);")
    );
    assert_eq!(
        native_import_type("tyrus:algorithm", "centroidDecomposition")
            .unwrap()
            .to_string(),
        "(usize[][]) => Centroid"
    );
    assert_eq!(
        native_import_detail("tyrus:algorithm", "centroidDecomposition"),
        Some("(adj: usize[][]) => Centroid, centroid decomposition over a tree (parentOf, depth)")
    );
    assert_eq!(
        native_import_type("tyrus:algorithm", "Centroid")
            .unwrap()
            .to_string(),
        "Centroid"
    );
    assert_eq!(
        native_import_example(
            "tyrus:algorithm",
            "centroidDecomposition",
            "centroidDecomposition"
        )
        .as_deref(),
        Some("const ct: Centroid = centroidDecomposition([[1], [0, 2], [1]]);")
    );
    assert_eq!(
        native_import_type("tyrus:algorithm", "Graph")
            .unwrap()
            .to_string(),
        "() => Graph"
    );
    assert_eq!(
        native_import_type("tyrus:algorithm", "UnionFind")
            .unwrap()
            .to_string(),
        "(usize) => UnionFind"
    );
    assert!(native_import_type("tyrus:cli", "missing").is_none());
    assert!(
        native_exports()
            .iter()
            .any(|export| export.name == "select")
    );
    assert!(
        native_exports()
            .iter()
            .any(|export| export.name == "suffixArray" && export.kind == SymbolKind::Function)
    );
    assert!(native_exports()
        .iter()
        .any(|export| export.name == "suffixTreeUkkonen" && export.kind == SymbolKind::Function));
    assert!(
        native_exports()
            .iter()
            .any(|export| export.name == "lcaBinaryLifting" && export.kind == SymbolKind::Function)
    );
    assert!(
        native_exports()
            .iter()
            .any(|export| export.name == "Lca" && export.kind == SymbolKind::TypeAlias)
    );
    assert!(
        native_exports()
            .iter()
            .any(|export| export.name == "heavyLightDecomposition"
                && export.kind == SymbolKind::Function)
    );
    assert!(
        native_exports()
            .iter()
            .any(|export| export.name == "Hld" && export.kind == SymbolKind::TypeAlias)
    );
    assert!(native_exports().iter().any(
        |export| export.name == "centroidDecomposition" && export.kind == SymbolKind::Function
    ));
    assert!(
        native_exports()
            .iter()
            .any(|export| export.name == "Centroid" && export.kind == SymbolKind::TypeAlias)
    );
    assert!(
        native_exports()
            .iter()
            .any(|export| export.name == "HttpStatus" && export.kind == SymbolKind::Enum)
    );
    assert!(
        native_exports()
            .iter()
            .any(|export| export.name == "HttpReply" && export.kind == SymbolKind::TypeAlias)
    );
    assert!(
        native_exports()
            .iter()
            .any(|export| export.name == "HttpRouter" && export.kind == SymbolKind::TypeAlias)
    );
    for name in [
        "HttpRequest",
        "HttpResponse",
        "CorsOptions",
        "CookieOptions",
    ] {
        assert!(
            native_exports()
                .iter()
                .any(|export| export.name == name && export.kind == SymbolKind::Interface)
        );
    }
    assert!(
        !native_exports()
            .iter()
            .any(|export| export.name == "ErrorResponse")
    );
}

#[test]
fn exposes_stream_capability_types() {
    for (name, detail, example) in [
        (
            "ReadableStream",
            "type ReadableStream<T>",
            "type Input = ReadableStream<u8[]>;",
        ),
        (
            "WritableStream",
            "type WritableStream<T>",
            "type Output = WritableStream<u8[]>;",
        ),
        (
            "DuplexStream",
            "type DuplexStream<TRead, TWrite>",
            "type Connection = DuplexStream<u8[], u8[]>;",
        ),
        (
            "TransformStream",
            "type TransformStream<TInput, TOutput>",
            "type Codec = TransformStream<string, u8[]>;",
        ),
    ] {
        assert_eq!(
            native_import_type("tyrus:stream", name)
                .unwrap()
                .to_string(),
            name
        );
        assert_eq!(native_import_detail("tyrus:stream", name), Some(detail));
        assert_eq!(
            native_parameter_names("tyrus:stream", name),
            Some(Vec::new())
        );
        assert_eq!(
            native_import_example("tyrus:stream", name, name).as_deref(),
            Some(example)
        );
        assert!(
            native_exports()
                .iter()
                .any(|export| export.uri.as_str() == "tyrus:stream"
                    && export.name == name
                    && export.kind == SymbolKind::TypeAlias)
        );
    }
    assert_eq!(
        native_import_detail("tyrus:stream", "StreamError"),
        Some("interface StreamError { readonly code: string, readonly message: string }")
    );
    assert!(
        native_exports()
            .iter()
            .any(|export| export.uri.as_str() == "tyrus:stream"
                && export.name == "StreamError"
                && export.kind == SymbolKind::Interface)
    );
}

#[test]
fn completes_stream_capability_members_with_substituted_types() {
    let readable = member_completion_items_for_type(&Type::Apply {
        name: "ReadableStream".to_string(),
        args: vec![Type::Array(Box::new(Type::U8))],
    })
    .unwrap();
    assert!(
        readable
            .iter()
            .any(|item| item.label == "read"
                && item.detail.as_deref() == Some("() => Promise<u8[]?>"))
    );
    assert!(readable.iter().any(|item| item.label == "pipeTo"
        && item.detail.as_deref() == Some("(destination: WritableStream<u8[]>) => Promise<void>")));

    let transform = member_completion_items_for_type(&Type::Apply {
        name: "TransformStream".to_string(),
        args: vec![Type::Str, Type::Array(Box::new(Type::U8))],
    })
    .unwrap();
    assert!(
        transform.iter().any(|item| item.label == "writable"
            && item.detail.as_deref() == Some("WritableStream<string>"))
    );
    assert!(
        transform.iter().any(|item| item.label == "readable"
            && item.detail.as_deref() == Some("ReadableStream<u8[]>"))
    );
}

#[test]
fn temporal_object_annotation_needs_interface_fields_to_avoid_e0412() {
    let source = r#"
        import { Duration } from "tyrus:temporal";

        const oneDay: Duration = {
            years: 0,
            months: 0,
            weeks: 0,
            days: 1,
            hours: 0,
            minutes: 0,
            seconds: 0,
            milliseconds: 0,
            microseconds: 0,
            nanoseconds: 0,
        };
    "#;
    let import_types: tyrus_core::collections::HashMap<String, Type> =
        [("Duration".to_string(), Type::Named("Duration".to_string()))]
            .into_iter()
            .collect();

    let (_, fields, _) =
        tyrus_core::module_loader::native_export_info("tyrus:temporal", "Duration")
            .expect("tyrus:temporal should export `Duration`");
    let fields = fields.expect("`Duration` should carry interface fields");

    let without_fields = tyrus_core::analysis::analyze_with_imports(
        source,
        &import_types,
        &tyrus_core::collections::HashMap::default(),
        &tyrus_core::collections::HashMap::default(),
    );
    assert!(
        without_fields
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E0412"
                && diagnostic.message.contains("Duration")),
        "expected E0412 without interface fields: {:#?}",
        without_fields.diagnostics
    );

    let import_interfaces: tyrus_core::collections::HashMap<String, Vec<Field>> =
        [("Duration".to_string(), fields)].into_iter().collect();
    let with_fields = tyrus_core::analysis::analyze_with_imports(
        source,
        &import_types,
        &import_interfaces,
        &tyrus_core::collections::HashMap::default(),
    );
    assert!(
        !with_fields
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E0412"),
        "unexpected E0412 with interface fields: {:#?}",
        with_fields.diagnostics
    );
}

#[test]
fn indexes_native_exports_for_auto_import() {
    let exports = native_exports();
    let write_file = exports
        .iter()
        .find(|export| export.name == "writeFileSync")
        .unwrap();

    assert_eq!(
        write_file.ty,
        "(path: string, data: string, encoding: string) => void"
    );
    assert_eq!(
        import_source_for_export(None, &write_file.uri).as_deref(),
        Some("tyrus:fs")
    );
}

#[test]
fn native_algorithm_study_result_metadata_exposes_fields() {
    let source = "import { BubbleSortInfo, bubbleSortInfo } from \"tyrus:algorithm\";\nconst run = (): void => { const info: BubbleSortInfo = bubbleSortInfo([4, 1, 3, 2]); console.log(info.comparisonCount, info.passCount); };\n";
    let names = ["BubbleSortInfo", "bubbleSortInfo"];
    let mut types = tyrus_core::collections::HashMap::default();
    let mut interfaces = tyrus_core::collections::HashMap::default();
    for name in names {
        types.insert(
            name.to_string(),
            native_import_type("tyrus:algorithm", name).expect("native export type"),
        );
        if let Some((_, Some(fields), _)) =
            tyrus_core::module_loader::native_export_info("tyrus:algorithm", name)
        {
            interfaces.insert(name.to_string(), fields);
        }
    }
    let analysis = tyrus_core::analysis::analyze_with_context(
        source,
        tyrus_core::analysis::AnalyzeContext {
            import_types: Some(&types),
            import_interfaces: Some(&interfaces),
            ..tyrus_core::analysis::AnalyzeContext::default()
        },
    );
    assert!(
        analysis.diagnostics.is_empty(),
        "analysis diagnostics: {:#?}",
        analysis.diagnostics
    );
    assert_eq!(
        native_import_detail("tyrus:algorithm", "bubbleSortInfo"),
        Some(
            "(values: i32[]) => BubbleSortInfo, sorted and original snapshots with deterministic Bubble Sort metrics"
        )
    );
    assert!(
        native_exports().iter().any(|export| {
            export.name == "BubbleSortInfo" && export.kind == SymbolKind::Interface
        })
    );
    assert!(
        !native_exports().iter().any(
            |export| export.name == "SortInfoMethods" || export.name == "BubbleSortInfoMethods"
        )
    );
    for (function, interface, detail, parameters) in [
        (
            "kmpSearchInfo",
            "KmpSearchInfo",
            "(text: string, pattern: string) => KmpSearchInfo, prefix-function search metrics",
            vec!["text".to_string(), "pattern".to_string()],
        ),
        (
            "zAlgorithmInfo",
            "ZAlgorithmInfo",
            "(input: string) => ZAlgorithmInfo, Z-table construction metrics",
            vec!["input".to_string()],
        ),
        (
            "rabinKarpInfo",
            "RabinKarpInfo",
            "(text: string, pattern: string) => RabinKarpInfo, rolling-hash search metrics",
            vec!["text".to_string(), "pattern".to_string()],
        ),
    ] {
        assert_eq!(
            native_import_detail("tyrus:algorithm", function),
            Some(detail)
        );
        assert_eq!(
            native_parameter_names("tyrus:algorithm", function),
            Some(parameters)
        );
        assert!(native_import_example("tyrus:algorithm", function, function).is_some());
        assert!(
            native_exports()
                .iter()
                .any(|export| { export.name == function && export.kind == SymbolKind::Function })
        );
        assert!(
            native_exports()
                .iter()
                .any(|export| { export.name == interface && export.kind == SymbolKind::Interface })
        );
    }
    assert!(native_exports().iter().any(|export| {
        export.name == "StringSearchInfo" && export.kind == SymbolKind::Interface
    }));
    assert_eq!(
        native_import_detail("tyrus:algorithm", "boyerMooreInfo"),
        Some("(text: string, pattern: string) => BoyerMooreInfo, bad-character search metrics")
    );
    assert!(
        native_exports().iter().any(|export| {
            export.name == "BoyerMooreInfo" && export.kind == SymbolKind::Interface
        })
    );
    assert_eq!(
        native_import_detail("tyrus:algorithm", "ahoCorasickInfo"),
        Some(
            "(text: string, patterns: string[]) => AhoCorasickInfo, multi-pattern automaton metrics"
        )
    );
    assert!(native_exports().iter().any(|export| {
        export.name == "AhoCorasickInfo" && export.kind == SymbolKind::Interface
    }));
    assert_eq!(
        native_import_detail("tyrus:algorithm", "suffixArrayInfo"),
        Some(
            "(text: string) => SuffixArrayInfo, deterministic suffix sorting rounds and comparison metrics"
        )
    );
    assert!(native_exports().iter().any(|export| {
        export.name == "SuffixArrayInfo" && export.kind == SymbolKind::Interface
    }));

    let invalid_source = "import { SuffixArrayInfo, suffixArrayInfo } from \"tyrus:algorithm\";\nconst run = (): void => { const info: SuffixArrayInfo = suffixArrayInfo(\"banana\"); console.log(info.unknownMetric); };\n";
    let mut invalid_types = tyrus_core::collections::HashMap::default();
    let mut invalid_interfaces = tyrus_core::collections::HashMap::default();
    for name in ["SuffixArrayInfo", "suffixArrayInfo"] {
        invalid_types.insert(
            name.to_string(),
            native_import_type("tyrus:algorithm", name).expect("native export type"),
        );
        if let Some((_, Some(fields), _)) =
            tyrus_core::module_loader::native_export_info("tyrus:algorithm", name)
        {
            invalid_interfaces.insert(name.to_string(), fields);
        }
    }
    let invalid_analysis = tyrus_core::analysis::analyze_with_context(
        invalid_source,
        tyrus_core::analysis::AnalyzeContext {
            import_types: Some(&invalid_types),
            import_interfaces: Some(&invalid_interfaces),
            ..tyrus_core::analysis::AnalyzeContext::default()
        },
    );
    assert!(invalid_analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "E0609"
            && diagnostic
                .message
                .contains("no field `unknownMetric` on type `SuffixArrayInfo`")
    }));
}

#[test]
fn builds_usage_examples_for_native_and_internal_functions() {
    assert_eq!(
        native_import_example("tyrus:fs", "writeFileSync", "writeFileSync").as_deref(),
        Some("writeFileSync(\"file.txt\", \"Hello from tyrus\", \"utf8\");")
    );
    assert_eq!(
        native_import_example("tyrus:utils", "figlet", "figlet").as_deref(),
        Some("const banner: string = figlet(\"HELLO\", \"slant\");")
    );

    let analysis = analyze("const sum = (left: i32, right: i32): i32 => left + right;");
    assert_eq!(
        function_usage_example("sum", analysis.symbol_named("sum").unwrap()).as_deref(),
        Some("sum(0, 0);")
    );
}

#[test]
fn inserts_function_calls_as_snippets() {
    let analysis = analyze("const sum = (left: i32, right: i32): i32 => left + right;");
    let item = symbol_completion_item(analysis.symbol_named("sum").unwrap());

    assert_eq!(
        item.insert_text.as_deref(),
        Some("sum(${1:left}, ${2:right})")
    );
    assert_eq!(item.insert_text_format, Some(InsertTextFormat::SNIPPET));

    let analysis = analyze("const readPair = ([first, second]: i32[]): i32 => first + second;");
    let item = symbol_completion_item(analysis.symbol_named("readPair").unwrap());
    assert_eq!(
        item.insert_text.as_deref(),
        Some("readPair(${1:[first, second]})")
    );

    let analysis = analyze("const readPoint = ({x, y}: Point): i32 => x + y;");
    let item = symbol_completion_item(analysis.symbol_named("readPoint").unwrap());
    assert_eq!(item.insert_text.as_deref(), Some("readPoint(${1:{x, y}})"));

    let mut native = CompletionItem {
        label: "writeFileSync".to_string(),
        kind: Some(CompletionItemKind::FUNCTION),
        ..CompletionItem::default()
    };
    apply_function_snippet(
        &mut native,
        "writeFileSync",
        &native_parameter_names("tyrus:fs", "writeFileSync").unwrap(),
    );
    assert_eq!(
        native.insert_text.as_deref(),
        Some("writeFileSync(${1:path}, ${2:data}, ${3:encoding})")
    );

    apply_function_snippet_with_terminator(
        &mut native,
        "writeFileSync",
        &native_parameter_names("tyrus:fs", "writeFileSync").unwrap(),
        true,
    );
    assert_eq!(
        native.insert_text.as_deref(),
        Some("writeFileSync(${1:path}, ${2:data}, ${3:encoding});$0")
    );
}

#[test]
fn suppresses_parameter_inlay_hints_for_destructuring_patterns() {
    assert_eq!(parameter_inlay_label("value"), Some("value"));
    assert_eq!(parameter_inlay_label("[first, second]"), None);
    assert_eq!(parameter_inlay_label("{x, y}"), None);
}

#[test]
fn creates_first_auto_import_with_a_blank_line() {
    let source = "writeFileSync(\"file.txt\", \"content\", \"utf8\");";
    let edit = auto_import_edit(source, "tyrus:fs", "writeFileSync").unwrap();

    assert_eq!(
        edit.range,
        Range::new(Position::new(0, 0), Position::new(0, 0))
    );
    assert_eq!(
        edit.new_text,
        "import { writeFileSync } from \"tyrus:fs\";\n\n"
    );
}

#[test]
fn merges_auto_imports_from_the_same_module() {
    let source = "import { writeFileSync } from \"tyrus:fs\";\n\nwriteFileSync(\"file.txt\", \"content\", \"utf8\");";
    let edit = auto_import_edit(source, "tyrus:fs", "readFileSync").unwrap();
    let start = position_to_byte_offset(source, edit.range.start);
    let end = position_to_byte_offset(source, edit.range.end);
    let mut updated = source.to_string();
    updated.replace_range(start..end, &edit.new_text);

    assert!(updated.starts_with("import { readFileSync, writeFileSync } from \"tyrus:fs\";\n\n"));
}

#[test]
fn organizes_native_and_internal_auto_imports() {
    let source = r#"import { zebra } from "../../zebra/tools";
import { alpha } from "../alpha/math";
import { readFileSync } from "tyrus:fs";

sum(1, 2);"#;
    let edit = auto_import_edit(source, "tyrus:assert", "assert").unwrap();
    let start = position_to_byte_offset(source, edit.range.start);
    let end = position_to_byte_offset(source, edit.range.end);
    let mut updated = source.to_string();
    updated.replace_range(start..end, &edit.new_text);

    assert_eq!(
        updated,
        r#"import { assert } from "tyrus:assert";
import { readFileSync } from "tyrus:fs";

import { alpha } from "../alpha/math";
import { zebra } from "../../zebra/tools";

sum(1, 2);"#
    );
}

#[test]
fn env_undeclared_variable_publishes_precise_range() {
    let env_source = "PORT=3000\n";
    let (vars, _) = tyrus_core::env::parse_env_source(env_source);
    let schema = tyrus_core::env::EnvSchema {
        env_file: ".env".to_string(),
        file_path: std::path::PathBuf::from("/project/.env"),
        source: env_source.to_string(),
        vars,
    };
    let source = "const HOST = process.env.HOST;\nconsole.log(HOST);\n";
    let analysis = tyrus_core::analysis::analyze_with_context(
        source,
        AnalyzeContext {
            env_schema: Some(&schema),
            ..AnalyzeContext::default()
        },
    );
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3141")
        .expect("expected E3141");
    let uri = Url::parse("file:///project/src/main.tyr").unwrap();
    let lsp = diagnostic_to_lsp(&uri, source, diagnostic);
    let start = source.find("process.env.HOST").unwrap() + "process.env.".len();
    assert_eq!(
        lsp.range.start.character,
        "const HOST = process.env.".len() as u32
    );
    assert_eq!(lsp.range.start.character as usize, start);
    assert!(
        lsp.data
            .and_then(|data| data.get("help").cloned())
            .is_some()
    );
}

#[test]
fn style_text_arity_error_points_at_the_call() {
    let source = "import { styleText } from \"tyrus:utils\";\n\
                  const ok = styleText(\"bold\", \"hi\");\n\
                  const arr = styleText([\"bold\", \"white\", \"bgRed\"], \" CRITICAL \");\n\
                  const piped: string = \"data\" |> styleText([\"cyan\"]);\n\
                  const text = styleText([\"yellow\", \"bold\", \"blink\"]);\n\
                  console.log(ok);\n\
                  console.log(arr);\n\
                  console.log(piped);\n\
                  console.log(text);\n";
    let mut import_types = tyrus_core::collections::HashMap::default();
    import_types.insert(
        "styleText".to_string(),
        Type::Named("__StyleText".to_string()),
    );
    let analysis = tyrus_core::analysis::analyze_with_imports(
        source,
        &import_types,
        &tyrus_core::collections::HashMap::default(),
        &tyrus_core::collections::HashMap::default(),
    );
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3270")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let slice = &source[diagnostic.range.clone()];
    assert_eq!(slice, "styleText([\"yellow\", \"bold\", \"blink\"])");
}

#[test]
fn style_text_non_literal_and_incomplete_pick_the_erroring_call() {
    let mut import_types = tyrus_core::collections::HashMap::default();
    import_types.insert(
        "styleText".to_string(),
        Type::Named("__StyleText".to_string()),
    );
    let empty_interfaces = tyrus_core::collections::HashMap::default();
    let empty_aliases = tyrus_core::collections::HashMap::default();
    let range_of = |source: &str, code: &str| -> String {
        let analysis = tyrus_core::analysis::analyze_with_imports(
            source,
            &import_types,
            &empty_interfaces,
            &empty_aliases,
        );
        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == code)
            .unwrap_or_else(|| panic!("no {code} in: {:#?}", analysis.diagnostics));
        source[diagnostic.range.clone()].to_string()
    };
    let non_literal = "import { styleText } from \"tyrus:utils\";\n\
                       const fmt: string = \"green\";\n\
                       const ok = styleText(\"bold\", \"hi\");\n\
                       const bad = styleText(fmt, \"hi\");\n\
                       console.log(ok);\n\
                       console.log(bad);\n";
    assert_eq!(range_of(non_literal, "E3272"), "styleText(fmt, \"hi\")");
    let incomplete = "import { styleText } from \"tyrus:utils\";\n\
                      const ok = styleText.green(\"hi\");\n\
                      const bad = styleText.red.bold;\n\
                      console.log(ok);\n\
                      console.log(bad);\n";
    assert_eq!(range_of(incomplete, "E3271"), "styleText.red.bold");
}

#[test]
fn format_document_edits_replaces_only_the_changed_region() {
    let config = tyrus_fmt::config::Config::default();
    let source = "const   x:i32   =1;\n";
    let expected = tyrus_fmt::format_source(source, &config);
    assert_ne!(expected, source, "sample must be reformatted by tyrus-fmt");
    let edits = crate::positions::format_document_edits(source, &config)
        .expect("unformatted input should produce an edit");
    assert_eq!(edits.len(), 1);
    assert!(edits[0].range.start.character > 0);
    assert!(edits[0].new_text.len() < expected.len());
    let start = crate::positions::position_to_byte_offset(source, edits[0].range.start);
    let end = crate::positions::position_to_byte_offset(source, edits[0].range.end);
    let mut applied = source.to_string();
    applied.replace_range(start..end, &edits[0].new_text);
    assert_eq!(applied, expected);
}

#[test]
fn format_document_edits_preserves_utf8_boundaries() {
    let config = tyrus_fmt::config::Config::default();
    let source = "const x:string = \"é\";\n";
    let expected = tyrus_fmt::format_source(source, &config);
    let edits = crate::positions::format_document_edits(source, &config)
        .expect("unformatted input should produce an edit");
    let start = crate::positions::position_to_byte_offset(source, edits[0].range.start);
    let end = crate::positions::position_to_byte_offset(source, edits[0].range.end);
    let mut applied = source.to_string();
    applied.replace_range(start..end, &edits[0].new_text);
    assert_eq!(applied, expected);
}

#[test]
fn format_document_edits_is_noop_when_already_formatted() {
    let config = tyrus_fmt::config::Config::default();
    let formatted = tyrus_fmt::format_source("const x: i32 = 1;\n", &config);
    assert!(crate::positions::format_document_edits(&formatted, &config).is_none());
}

#[test]
fn reserved_keyword_ranges_marks_reserved_words_outside_member_position() {
    let source = "const class = 1;\n";
    let ranges = crate::positions::reserved_keyword_ranges(source);
    let start = source.find("class").unwrap();
    assert_eq!(ranges, vec![start..start + "class".len()]);

    assert!(crate::positions::reserved_keyword_ranges("const value = 1;\n").is_empty());
}

#[test]
fn reserved_keyword_ranges_skips_member_access() {
    let source = "const x = obj.get;\nconst y = obj?.set;\n";
    assert!(crate::positions::reserved_keyword_ranges(source).is_empty());
}

#[test]
fn comptime_keyword_ranges_marks_expressions_functions_and_arrow_declarations() {
    let source = "comptime function build(): i32 { return 42; }\ncomptime const arrow = (): i32 => 42;\nconst result = comptime(build());\n";
    let ranges = crate::positions::comptime_keyword_ranges(source);
    let words = ranges
        .iter()
        .map(|range| &source[range.clone()])
        .collect::<Vec<_>>();
    assert_eq!(words, ["comptime", "comptime", "const", "comptime"]);
}

#[test]
fn encode_semantic_tokens_delta_encodes_by_type() {
    let source = "class public\n";
    let entries = vec![
        (0usize..5usize, crate::positions::SEMANTIC_TOKEN_KEYWORD),
        (6usize..12usize, crate::positions::SEMANTIC_TOKEN_KEYWORD),
    ];
    let tokens = crate::positions::encode_semantic_tokens(source, entries);
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].delta_line, 0);
    assert_eq!(tokens[0].delta_start, 0);
    assert_eq!(tokens[0].length, 5);
    assert_eq!(
        tokens[0].token_type,
        crate::positions::SEMANTIC_TOKEN_KEYWORD
    );
    assert_eq!(tokens[1].delta_line, 0);
    assert_eq!(tokens[1].delta_start, 6);
    assert_eq!(tokens[1].length, 6);
}

#[test]
fn symbol_occurrence_ranges_includes_declaration_and_references() {
    let source = "const greeting = 1;\nconsole.log(greeting);\nconsole.log(greeting);\n";
    let analysis = analyze(source);
    let decl = source.find("greeting").unwrap();
    let symbol = analysis.symbol_at(decl).expect("symbol at declaration");
    let ranges = crate::positions::symbol_occurrence_ranges(&analysis, symbol);
    let expected: Vec<_> = source
        .match_indices("greeting")
        .map(|(index, _)| index..index + "greeting".len())
        .collect();
    assert_eq!(expected.len(), 3, "declaration + two uses");
    assert_eq!(ranges, expected);
}

#[test]
fn custom_validator_references_include_the_decorator_occurrence() {
    let source = "type Validator = { isValid: true } | { isValid: false; message: string };\nvalidator const noSpaces = (value: string): Validator => { return { isValid: true }; };\ninterface User { @noSpaces() name: string; }\n";
    let analysis = analyze(source);
    let declaration = source.find("noSpaces").unwrap();
    let symbol = analysis
        .symbol_at(declaration)
        .expect("validator declaration symbol");
    let ranges = crate::positions::symbol_occurrence_ranges(&analysis, symbol);
    let expected: Vec<_> = source
        .match_indices("noSpaces")
        .map(|(index, _)| index..index + "noSpaces".len())
        .collect();
    assert_eq!(ranges, expected, "declaration + decorator reference");
}

#[test]
fn signature_active_parameter_tracks_the_cursor() {
    let source = "const run = (): void => { greet(x, y); };\n";
    let analysis = analyze(source);
    let call = analysis
        .call_sites
        .iter()
        .find(|call| call.callee == "greet")
        .expect("call site");
    let first_arg = source.find("(x").unwrap() + 1;
    assert_eq!(
        crate::positions::signature_active_parameter(call, first_arg, 2),
        0
    );
    let second_arg = source.find("y)").unwrap();
    assert_eq!(
        crate::positions::signature_active_parameter(call, second_arg, 2),
        1
    );
}

#[test]
fn symbol_occurrence_ranges_covers_parameter_uses() {
    let source = "const add = (amount: i32): i32 => amount + amount;\n";
    let analysis = analyze(source);
    let decl = source.find("amount").unwrap();
    let symbol = analysis.symbol_at(decl).expect("parameter symbol");
    let ranges = crate::positions::symbol_occurrence_ranges(&analysis, symbol);
    let expected: Vec<_> = source
        .match_indices("amount")
        .map(|(index, _)| index..index + "amount".len())
        .collect();
    assert_eq!(expected.len(), 3, "declaration + two uses");
    assert_eq!(ranges, expected);
}

#[test]
fn cross_file_rename_renames_non_aliased_imports_everywhere() {
    let math_uri = Url::from_file_path("/proj/math.tyr").unwrap();
    let main_uri = Url::from_file_path("/proj/main.tyr").unwrap();
    let math_src = "export const add = (a: i32, b: i32): i32 => a + b;\n";
    let main_src = "import { add } from \"./math\";\nconst r: i32 = add(1, 2);\nconsole.log(r);\n";
    let files = vec![
        (math_uri.clone(), math_src.to_string()),
        (main_uri.clone(), main_src.to_string()),
    ];
    let occ = crate::positions::cross_file_occurrences_in(&files, &math_uri, "add", true);
    let texts = |uri: &Url, src: &str| -> Vec<String> {
        occ.iter()
            .find(|(candidate, _)| candidate == uri)
            .map(|(_, ranges)| ranges.iter().map(|r| src[r.clone()].to_string()).collect())
            .unwrap_or_default()
    };
    assert_eq!(texts(&math_uri, math_src), vec!["add"]);
    assert_eq!(texts(&main_uri, main_src), vec!["add", "add"]);
}

#[test]
fn cross_file_rename_splits_aliased_imports() {
    let math_uri = Url::from_file_path("/proj/math.tyr").unwrap();
    let main_uri = Url::from_file_path("/proj/main.tyr").unwrap();
    let math_src = "export const add = (a: i32, b: i32): i32 => a + b;\n";
    let main_src =
        "import { add as plus } from \"./math\";\nconst r: i32 = plus(1, 2);\nconsole.log(r);\n";
    let files = vec![
        (math_uri.clone(), math_src.to_string()),
        (main_uri.clone(), main_src.to_string()),
    ];
    let main_texts = |rename: bool| -> Vec<String> {
        crate::positions::cross_file_occurrences_in(&files, &math_uri, "add", rename)
            .iter()
            .find(|(uri, _)| uri == &main_uri)
            .map(|(_, ranges)| {
                ranges
                    .iter()
                    .map(|r| main_src[r.clone()].to_string())
                    .collect()
            })
            .unwrap_or_default()
    };
    assert_eq!(main_texts(true), vec!["add"]);
    assert_eq!(main_texts(false), vec!["add", "plus", "plus"]);
}

#[test]
fn nearest_project_root_walks_up_to_tyrus_json() {
    let base = std::env::temp_dir().join(format!("tyrus-proj-{}", std::process::id()));
    let nested = base.join("src").join("feature");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(base.join("tyrus.json"), "{}").unwrap();

    let file = nested.join("mod.tyr");
    assert_eq!(
        crate::workspace::nearest_project_root(&file),
        Some(base.clone())
    );
    assert_eq!(
        crate::workspace::nearest_project_root(&std::env::temp_dir().join("loose.tyr")),
        None
    );

    std::fs::remove_dir_all(&base).ok();
}

#[test]
fn constant_integer_overflow_publishes_the_operation_range_and_widens_the_annotation() {
    let source = "const main = (): void => {\n    const numberOverflow: i32 = i32.MAX + 1;\n    console.log(numberOverflow);\n};\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0706")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    let operation = source.find("i32.MAX + 1").unwrap();
    assert_eq!(
        lsp_diagnostic.range,
        byte_range_to_lsp(source, operation..operation + "i32.MAX + 1".len())
    );

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
    assert_eq!(action.title, "Change type annotation to `i64`");
}

#[test]
fn out_of_range_literal_in_array_repeat_publishes_the_literal_range() {
    let source = "const unsignedLeft: i8[] = [255; 5];";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0706")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    let literal = source.find("255").unwrap();
    assert_eq!(
        lsp_diagnostic.range,
        byte_range_to_lsp(source, literal..literal + "255".len())
    );
}

#[test]
fn crypto_method_signature_help_preserves_aliases_and_parameter_names() {
    let source = r#"import { jwt as tokens } from "tyrus:crypto";
interface Claims { sub: string; }
const key: string = "0123456789abcdef0123456789abcdef";
const claims: Claims = { sub: "user-42" };
import { CryptoError } from "tyrus:crypto";
const token: Result<string, CryptoError> = tokens.sign(claims, key);
"#;
    let analysis = analyze(source);
    let sign = crate::backend::native_crypto_call_signature(&analysis, "tokens.sign").unwrap();
    assert_eq!(sign.parameters, ["payload", "key", "options"]);
    assert!(sign.detail.unwrap().contains("<T>"));
    let fetch =
        crate::backend::native_crypto_call_signature(&analysis, "tokens.fetchJwks").unwrap();
    assert_eq!(fetch.parameters, ["url", "options"]);
    assert!(
        fetch
            .detail
            .unwrap()
            .contains("Promise<Result<string, CryptoError>>")
    );
}

#[test]
fn encoding_method_signature_help_preserves_aliases_and_parameter_names() {
    let source = r#"import { utf16 as windowsText } from "tyrus:encoding";
const bytes: u8[] = windowsText.encodeLE("A😀");
const text: string = windowsText.decodeLE(bytes);
"#;
    let analysis = analyze(source);
    let encode =
        crate::backend::native_crypto_call_signature(&analysis, "windowsText.encodeLE").unwrap();
    assert_eq!(encode.parameters, ["text"]);
    assert!(encode.detail.unwrap().contains("u8[]"));
    let decode =
        crate::backend::native_crypto_call_signature(&analysis, "windowsText.decodeLE").unwrap();
    assert_eq!(decode.parameters, ["bytes"]);
    assert!(decode.detail.unwrap().contains("string"));
}

#[test]
fn hex_signature_help_uses_byte_and_text_parameter_names() {
    let source = r#"import { hex } from "tyrus:encoding";
const text: string = hex.encode([0xde, 0xad]);
const bytes: u8[] = hex.decode(text);
"#;
    let analysis = analyze(source);
    let encode = crate::backend::native_crypto_call_signature(&analysis, "hex.encode").unwrap();
    assert_eq!(encode.parameters, ["bytes"]);
    assert!(encode.detail.unwrap().contains("string"));
    let decode = crate::backend::native_crypto_call_signature(&analysis, "hex.decode").unwrap();
    assert_eq!(decode.parameters, ["text"]);
    assert!(decode.detail.unwrap().contains("u8[]"));
}

fn spawn_quick_fix_edit(
    source: &str,
    call: &str,
    message: &str,
) -> Option<(String, tower_lsp::lsp_types::Range)> {
    let start = source
        .find(call)
        .expect("the call must appear in the source");
    let uri = Url::parse("file:///test.tyr").unwrap();
    let diagnostic = Diagnostic {
        range: byte_range_to_lsp(source, start..start + call.len()),
        severity: Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("E3031".to_string())),
        message: message.to_string(),
        ..Diagnostic::default()
    };
    let mut actions = Vec::new();
    crate::server::push_call_task_directly_code_action(&mut actions, &uri, source, &diagnostic);
    let action = match actions.first()? {
        CodeActionOrCommand::CodeAction(action) => action,
        CodeActionOrCommand::Command(_) => panic!("expected a code action"),
    };
    assert_eq!(
        action.title,
        "Call the task directly instead of spawning it"
    );
    let edit = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .and_then(|changes| changes.get(&uri))
        .and_then(|edits| edits.first())
        .expect("expected edit");
    Some((edit.new_text.clone(), edit.range))
}

const AWAIT_SPAWN_DIRECTLY: &str =
    "awaiting `spawn(...)` directly defeats the purpose of running it in the background";

#[test]
fn awaiting_spawn_of_a_named_task_offers_calling_it_directly() {
    let source = "async function main(): Promise<void> { const value = await spawn(work); }\n";
    let (new_text, range) =
        spawn_quick_fix_edit(source, "spawn(work)", AWAIT_SPAWN_DIRECTLY).expect("expected a fix");
    assert_eq!(new_text, "work()");
    assert_eq!(
        &source[position_to_byte_offset(source, range.start)
            ..position_to_byte_offset(source, range.end)],
        "spawn(work)"
    );
}

#[test]
fn awaiting_spawn_of_a_literal_task_parenthesises_it_before_calling() {
    let source = "async function main(): Promise<void> { const value = await spawn(async (): Promise<i32> => { return 1; }); }\n";
    let (new_text, _) = spawn_quick_fix_edit(
        source,
        "spawn(async (): Promise<i32> => { return 1; })",
        AWAIT_SPAWN_DIRECTLY,
    )
    .expect("expected a fix");
    assert_eq!(new_text, "(async (): Promise<i32> => { return 1; })()");
}

#[test]
fn tasks_method_calls_record_their_receiver_for_signature_help() {
    let source = concat!(
        "async function main(): Promise<void> {\n",
        "    const numbers: i32[] = [1, 2];\n",
        "    console.log(numbers.join(\", \"));\n",
        "}\n",
    );
    let analysis = analyze(source);
    let call = analysis
        .call_sites
        .iter()
        .find(|call| call.callee == "join")
        .expect("expected a `join` call site");
    assert_eq!(call.receiver.as_deref(), Some("numbers"));
}

#[test]
fn join_signature_help_follows_the_receiver_type() {
    let handle = Type::Apply {
        name: "JoinHandle".to_string(),
        args: vec![Type::I32],
    };
    let signature = crate::backend::tasks_instance_call_signature(&handle, "join")
        .expect("expected a `JoinHandle<T>.join()` signature");
    assert!(signature.parameters.is_empty());
    assert_eq!(signature.detail.as_deref(), Some("() => Promise<i32>"));

    assert!(
        crate::backend::tasks_instance_call_signature(&Type::Array(Box::new(Type::I32)), "join")
            .is_none(),
        "an array must keep `Array.prototype.join`'s own signature"
    );
}

#[test]
fn tasks_instance_signature_help_names_payload_parameters() {
    let sender = Type::Apply {
        name: "Sender".to_string(),
        args: vec![Type::Str],
    };
    let send = crate::backend::tasks_instance_call_signature(&sender, "send")
        .expect("expected a `Sender<T>.send()` signature");
    assert_eq!(send.parameters, vec!["value"]);
    assert_eq!(
        send.detail.as_deref(),
        Some("(value: string) => Promise<void>")
    );

    let semaphore = Type::Named("Semaphore".to_string());
    let acquire = crate::backend::tasks_instance_call_signature(&semaphore, "acquire")
        .expect("expected a `Semaphore.acquire()` signature");
    assert!(acquire.parameters.is_empty());

    assert!(
        crate::backend::tasks_instance_call_signature(&semaphore, "release").is_none(),
        "a method the type does not have has no signature"
    );
}

#[test]
fn signature_parameter_names_reads_a_rendered_parameter_list() {
    assert_eq!(
        crate::backend::signature_parameter_names("(value: T, retries: i32) => void"),
        vec!["value", "retries"]
    );
    assert!(crate::backend::signature_parameter_names("() => Promise<i32>").is_empty());
    assert_eq!(
        crate::backend::signature_parameter_names("(capacity?: usize) => Semaphore"),
        vec!["capacity"]
    );
}

#[test]
fn moved_value_with_an_explicit_annotation_only_offers_the_annotated_declaration() {
    let source = "const main = (): void => {\n    const a = \"a\";\n    const b: string = a;\n\n    console.log(a);\n    console.log(b);\n};\n";
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
    assert_eq!(actions.len(), 1, "actions: {actions:#?}");
    let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
        panic!("expected a code action, got {:#?}", actions[0]);
    };
    assert_eq!(
        action.title,
        "Replace the declaration with `const b: &string = &a;`"
    );
    assert_eq!(action.is_preferred, Some(true));

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
    assert!(fixed.contains("const b: &string = &a;"), "fixed: {fixed}");
    let repaired = analyze(&fixed);
    assert!(
        repaired.diagnostics.is_empty(),
        "the applied fix must leave no diagnostics: {:#?}",
        repaired.diagnostics
    );
}

#[test]
fn a_binding_annotated_without_a_borrow_offers_changing_the_annotation() {
    let source = "const main = (): void => {\n    const a = \"a\";\n    const b: string = &a;\n\n    console.log(a);\n    console.log(b);\n};\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0308")
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
    let CodeActionOrCommand::CodeAction(action) = actions
        .iter()
        .find(|action| match action {
            CodeActionOrCommand::CodeAction(action) => {
                action.title == "Change the annotation to `&string`"
            }
            CodeActionOrCommand::Command(_) => false,
        })
        .unwrap_or_else(|| panic!("actions: {actions:#?}"))
    else {
        unreachable!()
    };

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
    assert!(fixed.contains("const b: &string = &a;"), "fixed: {fixed}");
    let repaired = analyze(&fixed);
    assert!(
        repaired.diagnostics.is_empty(),
        "the applied fix must leave no diagnostics: {:#?}",
        repaired.diagnostics
    );
}

#[test]
fn borrow_annotation_help_is_only_read_from_its_own_message() {
    assert_eq!(
        crate::server::diagnostic_change_annotation_to_borrow_help(
            "change the annotation to `&string` if this binding should store a borrow"
        )
        .as_deref(),
        Some("&string")
    );
    assert!(
        crate::server::diagnostic_change_annotation_to_borrow_help(
            "change the return type to `Promise<i32>`"
        )
        .is_none()
    );
}

#[test]
fn inferred_type_hint_shows_a_for_counter_as_its_bound_type() {
    let source = concat!(
        "const items = [1, 2, 3];\n",
        "for (let i = 0; i < items.length; i++) {\n",
        "    print(items[i]);\n",
        "}\n",
    );
    let analysis = analyze(source);
    let hints = inferred_type_hints(source, &analysis);
    let types: Vec<&str> = hints.iter().map(|(_, ty)| ty.as_str()).collect();
    assert_eq!(types, vec!["i32[]", "usize"]);
}

#[test]
fn inferred_type_hint_keeps_a_signed_for_counter_as_i32() {
    let source = concat!(
        "const limit: i32 = 3;\n",
        "for (let i = 0; i < limit; i++) {\n",
        "    print(i);\n",
        "}\n",
    );
    let analysis = analyze(source);
    let hints = inferred_type_hints(source, &analysis);
    let types: Vec<&str> = hints.iter().map(|(_, ty)| ty.as_str()).collect();
    assert_eq!(types, vec!["i32"]);
}

#[test]
fn test_registrar_modifiers_are_valid_with_lsp_import_targets() {
    use tyrus_core::analysis::AnalyzeContext;
    use tyrus_core::test::{IT_TARGET, TEST_TARGET, test_type};

    let source = concat!(
        "import { it, test } from \"tyrus:test\";\n\n",
        "it.skip(\"later\", () => {});\n",
        "test.todo(\"unicode initials\");\n",
        "test.each(\"clamps\", [-5, 0, 5], (value: i32) => {});\n",
    );
    let import_types = [
        ("it".to_string(), test_type()),
        ("test".to_string(), test_type()),
    ]
    .into_iter()
    .collect();
    let import_targets = [
        ("it".to_string(), IT_TARGET.to_string()),
        ("test".to_string(), TEST_TARGET.to_string()),
    ]
    .into_iter()
    .collect();
    let analysis = tyrus_core::analysis::analyze_with_context(
        source,
        AnalyzeContext {
            import_types: Some(&import_types),
            import_targets: Some(&import_targets),
            ..AnalyzeContext::default()
        },
    );
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E0599"),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

fn apply_edits(source: &str, edits: &[tower_lsp::lsp_types::TextEdit]) -> String {
    let mut text = source.to_string();
    let mut ranges: Vec<_> = edits
        .iter()
        .map(|edit| {
            (
                crate::positions::position_to_byte_offset(source, edit.range.start),
                crate::positions::position_to_byte_offset(source, edit.range.end),
                edit.new_text.clone(),
            )
        })
        .collect();
    ranges.sort_by_key(|(start, ..)| std::cmp::Reverse(*start));
    for (start, end, new_text) in ranges {
        text.replace_range(start..end, &new_text);
    }
    text
}

fn apply_quick_fix_edits(source: &str, edits: &[TextEdit]) -> String {
    let offset = |position: Position| {
        let line_start = source
            .split_inclusive('\n')
            .take(position.line as usize)
            .map(str::len)
            .sum::<usize>();
        line_start + position.character as usize
    };
    let mut ordered = edits.to_vec();
    ordered.sort_by_key(|edit| std::cmp::Reverse(offset(edit.range.start)));
    let mut fixed = source.to_string();
    for edit in ordered {
        fixed.replace_range(
            offset(edit.range.start)..offset(edit.range.end),
            &edit.new_text,
        );
    }
    fixed
}

fn type_name_diagnostic(source: &str, name: &str) -> Diagnostic {
    let start = source.find(name).unwrap();
    let line = source[..start].matches('\n').count() as u32;
    let character = (start - source[..start].rfind('\n').map_or(0, |index| index + 1)) as u32;
    Diagnostic {
        range: Range {
            start: Position { line, character },
            end: Position {
                line,
                character: character + name.len() as u32,
            },
        },
        code: Some(NumberOrString::String("E0412".to_string())),
        message: format!("cannot find type `{name}` in this scope"),
        ..Diagnostic::default()
    }
}

#[test]
fn every_http_export_is_known_to_editor_support() {
    let indexed = crate::native::native_exports()
        .into_iter()
        .map(|export| export.name)
        .collect::<Vec<_>>();

    for name in tyrus_core::module_loader::native_export_names("tyrus:http") {
        assert!(
            crate::native::native_import_type("tyrus:http", &name).is_some(),
            "`{name}` is exported by `tyrus:http` but has no type for hover or completion"
        );
        assert!(
            crate::native::native_import_detail("tyrus:http", &name).is_some(),
            "`{name}` is exported by `tyrus:http` but has no signature detail"
        );
        assert!(
            crate::native::native_import_example("tyrus:http", &name, &name).is_some(),
            "`{name}` is exported by `tyrus:http` but has no import example"
        );
        assert!(
            indexed.contains(&name),
            "`{name}` is exported by `tyrus:http` but is missing from the export index"
        );
    }
}

#[test]
fn a_throwing_function_argument_offers_to_declare_throws_on_the_parameter_type() {
    let source = "function failText(): void throws string {\n    throw \"boom\";\n}\nconst run = (action: () => void): void => {\n    action();\n};\nconst main = (): void => {\n    run(failText);\n};\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3298")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    let mut actions = Vec::new();
    crate::server::push_function_type_throws_code_action(
        &mut actions,
        &uri,
        source,
        &lsp_diagnostic,
    );
    let action = actions
        .iter()
        .find_map(|action| match action {
            CodeActionOrCommand::CodeAction(action) => Some(action),
            _ => None,
        })
        .expect("the parameter annotation is in this file");
    assert_eq!(action.title, "Declare `throws string` in the function type");
    let edit = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .and_then(|changes| changes.get(&uri))
        .expect("the fix edits this file");
    assert_eq!(edit.len(), 1);
    assert_eq!(edit[0].new_text, " throws string");
    let annotation_end = source.find("() => void").unwrap() + "() => void".len();
    assert_eq!(
        edit[0].range,
        crate::positions::byte_range_to_lsp(source, annotation_end..annotation_end)
    );
}

#[test]
fn a_throwing_function_initializer_adds_its_effect_to_a_type_that_already_throws() {
    let source = "function failText(): void throws string {\n    throw \"boom\";\n}\nfunction failNumber(): void throws i32 {\n    throw 1;\n}\nconst main = (): void => {\n    const first: () => void throws string = failText;\n    const second: () => void throws string = failNumber;\n};\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3298")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    let mut actions = Vec::new();
    crate::server::push_function_type_throws_code_action(
        &mut actions,
        &uri,
        source,
        &lsp_diagnostic,
    );
    let CodeActionOrCommand::CodeAction(action) = actions.first().expect("an action") else {
        panic!("expected a code action");
    };
    let edit = &action.edit.as_ref().unwrap().changes.as_ref().unwrap()[&uri];
    assert_eq!(edit[0].new_text, " | i32");
    let second = source.find("const second").unwrap();
    let annotation_end = second + source[second..].find(" = failNumber").unwrap();
    assert_eq!(
        edit[0].range,
        crate::positions::byte_range_to_lsp(source, annotation_end..annotation_end)
    );
}

#[test]
fn completes_the_fields_of_a_type_intersection() {
    let source = "interface B { id: i32; }\ninterface C { name: string; }\ntype A = B & C;\nconst value: A = { id: 1, name: \"x\" };\nvalue.";
    let analysis = analyze(source);
    let items = member_completion_items(source, source.len(), &analysis).unwrap();
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(labels.contains(&"id"), "{labels:?}");
    assert!(labels.contains(&"name"), "{labels:?}");
}

#[test]
fn completes_the_inherited_fields_of_an_interface() {
    let source = "interface B { id: i32; }\ninterface A extends B { name: string; }\nconst value: A = { id: 1, name: \"x\" };\nvalue.";
    let analysis = analyze(source);
    let items = member_completion_items(source, source.len(), &analysis).unwrap();
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(labels.contains(&"id"), "{labels:?}");
    assert!(labels.contains(&"name"), "{labels:?}");
}
