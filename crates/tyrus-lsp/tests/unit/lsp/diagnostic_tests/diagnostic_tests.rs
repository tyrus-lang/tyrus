use super::*;

#[test]
pub(super) fn reports_assignment_to_const_without_initializer() {
    let source = "const { PORT, TEST, SECRET } = process.env;\n\nprint(\"PORT:\", PORT);\nprint(TEST);\nprint(SECRET);\n\nconst isLiquid: boolean;\nisLiquid = true;\nprint(isLiquid);\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0002")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let declaration_start = source
        .find("const isLiquid")
        .expect("declaration is present");
    let declaration_end = declaration_start
        + source[declaration_start..]
            .find(';')
            .expect("declaration terminator is present");
    assert_eq!(diagnostic.range, declaration_end..declaration_end + 1);
}

#[test]
pub(super) fn quick_fix_code_lenses_mirror_action_titles_and_diagnostic_ranges() {
    let fixed_range = Range {
        start: Position {
            line: 2,
            character: 4,
        },
        end: Position {
            line: 2,
            character: 7,
        },
    };
    let actions = vec![
        CodeActionOrCommand::CodeAction(CodeAction {
            title: "refactor suggestion".to_string(),
            kind: Some(CodeActionKind::REFACTOR),
            edit: Some(WorkspaceEdit::default()),
            ..CodeAction::default()
        }),
        synthetic_quickfix_action("replace `let` with `const`", fixed_range, false),
    ];
    let lenses = quick_fix_code_lenses(&actions);
    assert_eq!(lenses.len(), 1);
    assert_eq!(lenses[0].range, fixed_range);
    let command = lenses[0]
        .command
        .as_ref()
        .expect("lens carries its command");
    assert_eq!(command.title, "replace `let` with `const`");
    assert_eq!(command.command, "tyrus.applyFix");
    let arguments = command.arguments.as_deref().expect("edit argument present");
    assert_eq!(arguments.len(), 1);
    assert!(arguments[0].get("changes").is_some());
}

#[test]
pub(super) fn quick_fixes_for_diagnostics_dispatches_lint_fixes_and_skips_unknown_codes() {
    let source = "const main = (): void => {\n    let x = 10;\n};\n";
    let lints = tyrus_lint::lint_rules_source(source, &tyrus_lint::LintConfig::default());
    let lint = lints
        .iter()
        .find(|lint| lint.code == "prefer-const")
        .expect("prefer-const should fire for the never-reassigned binding");
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let lsp_diagnostic = lint_diagnostic_to_lsp(source, lint);

    let current_path = std::path::PathBuf::from("/workspace/src/main.tyr");
    let actions =
        quick_fixes_for_diagnostics(&uri, source, &current_path, &[], &lints, &[lsp_diagnostic]);
    assert_eq!(actions.len(), 1);
    let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
        panic!("expected a code action, got {actions:?}");
    };
    assert_eq!(
        action.title,
        lint.fix.as_ref().expect("prefer-const is fixable").message
    );
    assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
    assert!(action.edit.is_some());

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
    assert!(
        quick_fixes_for_diagnostics(&uri, source, &current_path, &[], &lints, &[unknown],)
            .is_empty()
    );
}

#[test]
pub(super) fn rotated_array_binding_does_not_publish_prefer_const() {
    let source = r#"
const main = (): void => {
    let rotated: i32[] = [1, 2, 3, 4, 5];
    rotated.rotate(-2);
    print(rotated);
};
"#;
    let diagnostics = tyrus_lint::lint_rules_source(source, &tyrus_lint::LintConfig::default());
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "prefer-const"),
        "rotate mutates the array and must suppress prefer-const: {diagnostics:#?}"
    );
}

#[test]
pub(super) fn sort_by_array_binding_does_not_publish_prefer_const() {
    let source = r#"
interface Score {
    name: string;
    points: i32;
}
const main = (): void => {
    let scores: Score[] = [
        { name: "Ada", points: 30 },
        { name: "Grace", points: 10 },
        { name: "Linus", points: 20 },
    ];
    scores.sortBy("points");
    print(scores);
};
"#;
    let diagnostics = tyrus_lint::lint_rules_source(source, &tyrus_lint::LintConfig::default());
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "prefer-const"),
        "sortBy mutates the array and must suppress prefer-const: {diagnostics:#?}"
    );
}

#[test]
pub(super) fn defer_control_flow_diagnostics_publish_related_information_without_unsafe_quick_fixes()
 {
    let source = "const run = (): void => { defer { return; } };";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0D02")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    assert!(lsp_diagnostic.related_information.is_some());
    let actions = quick_fixes_for_diagnostics(
        &uri,
        source,
        std::path::Path::new("/workspace/src/main.tyr"),
        &[],
        &[],
        &[lsp_diagnostic],
    );
    assert!(
        actions.is_empty(),
        "removing control flow would change behavior"
    );
}

#[test]
pub(super) fn array_match_subject_diagnostic_publishes_the_pattern_range_to_lsp() {
    let source = "const value: i32 = 1;\nmatch (value) {\n    [] => { return; }\n}\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0529")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    let pattern = source.find("[]").expect("array pattern is present");
    assert_eq!(
        lsp_diagnostic.range,
        byte_range_to_lsp(source, pattern..pattern + "[]".len())
    );
}

#[test]
pub(super) fn array_match_literal_diagnostic_publishes_the_literal_range_to_lsp() {
    let source = "const values: i32[] = [1];\nmatch (values) {\n    [\"one\"] => { return; }\n}\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0308")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    let literal = source.find("\"one\"").expect("literal is present");
    assert_eq!(
        lsp_diagnostic.range,
        byte_range_to_lsp(source, literal..literal + "\"one\"".len())
    );
}

#[test]
pub(super) fn array_match_rest_parser_diagnostic_publishes_range_and_help_to_lsp() {
    let source =
        "const values: i32[] = [1];\nmatch (values) {\n    [...rest, last] => { return; }\n}\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0792")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    let rest = source.find("...rest").expect("rest pattern is present");
    assert_eq!(
        lsp_diagnostic.range,
        byte_range_to_lsp(source, rest..rest + "...rest".len())
    );
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help == "move the rest pattern to the end of the array pattern")
    );
}

#[test]
pub(super) fn anonymous_object_diagnostic_finds_literal_fields() {
    let source = r#"
        const isAdmin = { a: 1, b: "yes", c: true };
        console.log(isAdmin);
    "#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0310")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let fields = backend::anonymous_object_binding_fields(source, diagnostic)
        .expect("anonymous object fields should be recovered");
    assert_eq!(fields.len(), 3);
    assert!(backend::exact_interface_match(
        &[
            Field {
                name: "a".to_string(),
                ty: Type::I32,
                readonly: false,
                optional: false,
                decorators: Vec::new(),
                span: Span::DUMMY,
            },
            Field {
                name: "b".to_string(),
                ty: Type::Str,
                readonly: false,
                optional: false,
                decorators: Vec::new(),
                span: Span::DUMMY,
            },
            Field {
                name: "c".to_string(),
                ty: Type::Bool,
                readonly: false,
                optional: false,
                decorators: Vec::new(),
                span: Span::DUMMY,
            },
        ],
        &fields
    ));
}

#[test]
pub(super) fn diagnostic_to_lsp_keeps_help_out_of_message() {
    let source = "const isAdmin = { a: 1 };\n";
    let diagnostic = AnalysisDiagnostic {
        code: "E0310".to_string(),
        range: source.find("isAdmin").unwrap()..source.find("isAdmin").unwrap() + "isAdmin".len(),
        severity: DiagnosticSeverity::Error,
        message: "object literal `isAdmin` requires an explicit interface annotation".to_string(),
        related: Vec::new(),
        help: vec![
            "import `{ AdminStatus }` from `./types` or annotate the binding as `: AdminStatus`"
                .to_string(),
        ],
    };
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let lsp = diagnostic_to_lsp(&uri, source, &diagnostic);
    assert!(!lsp.message.contains("help:"));
    let help = lsp
        .data
        .as_ref()
        .and_then(|data| data.get("help"))
        .and_then(|help| help.as_array())
        .expect("help should be stored in diagnostic data");
    assert_eq!(
        help[0],
        "import `{ AdminStatus }` from `./types` or annotate the binding as `: AdminStatus`"
    );
}

#[test]
pub(super) fn the_new_invalid_target_diagnostics_can_be_fixed_by_removing_the_decorator() {
    for (source, fixed) in [
        (
            "interface Conf { @port() value: f64; }\n",
            "interface Conf { value: f64; }\n",
        ),
        (
            "interface Conf { @powerOfTwo() value: f64; }\n",
            "interface Conf { value: f64; }\n",
        ),
        (
            "interface Conf { @statusCode() value: f64; }\n",
            "interface Conf { value: f64; }\n",
        ),
        (
            "interface Grid { @unique() rows: i32[][]; }\n",
            "interface Grid { rows: i32[][]; }\n",
        ),
    ] {
        let analysis = tyrus_core::analysis::analyze(source);
        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "E3161")
            .unwrap_or_else(|| panic!("expected an invalid-target diagnostic for `{source}`"));

        let range = Range::new(
            byte_offset_to_position(source, diagnostic.range.start),
            byte_offset_to_position(source, diagnostic.range.end),
        );
        let edit = crate::server::remove_validator_decorator_edit(source, range)
            .unwrap_or_else(|| panic!("expected a removal edit for `{source}`"));

        let mut applied = source.to_string();
        let start = position_to_byte_offset(source, edit.range.start);
        let end = position_to_byte_offset(source, edit.range.end);
        applied.replace_range(start..end, &edit.new_text);
        assert_eq!(applied, fixed);
        assert!(
            tyrus_core::analysis::analyze(&applied)
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != "E3161")
        );
    }
}

#[test]
pub(super) fn object_projection_diagnostic_keeps_the_key_range_in_lsp() {
    let source = "interface Account { id: i32; }\nconst run = (account: Account): void => { account.pick([\"missing\"]); };\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0609")
        .expect("missing projection key diagnostic");
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let lsp = diagnostic_to_lsp(&uri, source, diagnostic);
    assert_eq!(lsp.range.start.line, 1);
    assert_eq!(lsp.range.start.character, 56);
    assert_eq!(lsp.range.end.character, 65);
}

#[test]
pub(super) fn diagnostics_include_related_information() {
    let source = r#"
        const main = (): void => {
            const name: string = "Ada";
            const moved: string = name;
            console.log(moved);
            console.log(name);
        };
    "#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0382")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp = diagnostic_to_lsp(&uri, source, diagnostic);
    assert_eq!(lsp.message, "use of moved value: `name`");
    assert!(!lsp.message.contains("line "));
    assert!(!lsp.message.contains("console.log(name);"));
    assert!(!lsp.message.contains("^^^^"));
    assert!(!lsp.message.contains(" --> "));
    assert!(!lsp.message.contains(" | "));
    let related = lsp.related_information.unwrap();
    assert_eq!(related.len(), 1);
    assert_eq!(related[0].location.uri, uri);
    assert!(related[0].message.contains("value moved here"));
}

#[test]
pub(super) fn import_alias_diagnostics_point_to_the_module_specifier_and_manifest_entry() {
    let source = "import { value } from \"#missing/value\";\n";
    let declaration_start = source.find("value").unwrap();
    let range = import_source_range(
        source,
        &(declaration_start..declaration_start + 5),
        "#missing/value",
    )
    .expect("the import source literal must have a precise range");
    assert_eq!(&source[range.clone()], "\"#missing/value\"");

    let root = std::env::temp_dir().join(format!(
        "tyrus-lsp-import-alias-related-test-{}",
        std::process::id()
    ));
    let src = root.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        root.join("tyrus.json"),
        r##"{ "imports": { "#missing": "./src/missing" } }"##,
    )
    .unwrap();
    let uri = Url::from_file_path(src.join("main.tyr")).unwrap();
    let analysis = AnalysisDiagnostic {
        code: "E0432".to_string(),
        range,
        severity: DiagnosticSeverity::Error,
        message: "import alias `#missing/value` is not defined in tyrus.json".to_string(),
        related: Vec::new(),
        help: Vec::new(),
    };
    let mut diagnostic = diagnostic_to_lsp(&uri, source, &analysis);
    attach_import_alias_related_information(&mut diagnostic, &uri, source, &analysis);
    let related = diagnostic
        .related_information
        .expect("manifest entry related info");
    assert_eq!(related.len(), 1);
    assert_eq!(
        related[0].location.uri,
        Url::from_file_path(root.join("tyrus.json")).unwrap()
    );
    assert_eq!(
        related[0].message,
        "import alias `#missing` is declared here"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
pub(super) fn fixed_array_length_mismatch_publishes_the_initializer_range_and_help() {
    let source = "const values: [i32; 5] = [1, 2, 3, 4];\nconsole.log(values);\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0308")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    assert_eq!(
        diagnostic.message,
        "fixed-length type requires 5 elements, but this initializer contains 4"
    );
    assert_eq!(
        diagnostic.help,
        ["provide exactly 5 elements in this initializer"]
    );
    let uri = Url::parse("file:///fixed-array.tyr").unwrap();
    let lsp = diagnostic_to_lsp(&uri, source, diagnostic);
    let start = source.find("[1, 2, 3, 4]").expect("initializer exists");
    assert_eq!(
        lsp.range,
        byte_range_to_lsp(source, start..start + "[1, 2, 3, 4]".len())
    );
}

#[test]
pub(super) fn await_non_promise_diagnostic_exposes_await_range_and_related_expression() {
    let source = "async function load(value: i32): Promise<i32> { return await value; }\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3001")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp = diagnostic_to_lsp(&uri, source, diagnostic);
    let await_start = source.find("await").unwrap();
    let value_start = source.rfind("value").unwrap();
    assert_eq!(
        lsp.range,
        byte_range_to_lsp(source, await_start..await_start + "await".len())
    );
    let related = lsp.related_information.unwrap();
    assert_eq!(related.len(), 1);
    assert_eq!(related[0].location.uri, uri);
    assert_eq!(
        related[0].location.range,
        byte_range_to_lsp(source, value_start..value_start + "value".len())
    );
    assert!(related[0].message.contains("not a `Promise<T>`"));
}

#[test]
pub(super) fn external_thenable_diagnostic_is_shared_with_the_lsp() {
    let source = "interface Thenable { then: (value: i32) => void; } async function load(value: Thenable): Promise<void> { await value; }\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3008")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp = diagnostic_to_lsp(&uri, source, diagnostic);
    let value = source.rfind("value").unwrap();
    assert_eq!(
        lsp.range,
        byte_range_to_lsp(source, value..value + "value".len())
    );
}

#[test]
pub(super) fn async_return_diagnostics_publish_precise_ranges_and_related_information() {
    let source = "async function load(): Promise<i32> { return \"wrong\"; }\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3296")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp = diagnostic_to_lsp(&uri, source, diagnostic);
    let value_start = source.find("\"wrong\"").unwrap();
    let annotation_start = source.find("Promise<i32>").unwrap();
    assert_eq!(
        lsp.range,
        byte_range_to_lsp(source, value_start..value_start + "\"wrong\"".len())
    );
    let related = lsp.related_information.unwrap();
    assert!(related.iter().any(|information| {
        information.location.range
            == byte_range_to_lsp(
                source,
                annotation_start..annotation_start + "Promise<i32>".len(),
            )
            && information.message.contains("declared here")
    }));
}

#[test]
pub(super) fn unused_promise_diagnostic_publishes_the_discarded_call_range() {
    let source = "async function load(): Promise<void> {}\nasync function main(): Promise<void> { load(); }\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3005")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp = diagnostic_to_lsp(&uri, source, diagnostic);
    let call_start = source.rfind("load()").unwrap();
    assert_eq!(
        lsp.range,
        byte_range_to_lsp(source, call_start..call_start + "load()".len())
    );
}

#[test]
pub(super) fn await_missing_expression_diagnostic_exposes_await_range() {
    let source = "async function load(): Promise<void> { await; }\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "async-await-missing-expression")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp = diagnostic_to_lsp(&uri, source, diagnostic);
    let await_start = source.find("await").unwrap();
    assert_eq!(
        lsp.range,
        byte_range_to_lsp(source, await_start..await_start + "await".len())
    );
    assert_eq!(lsp.message, "`await` must be followed by an expression");
}

#[test]
pub(super) fn async_borrow_across_await_diagnostic_exposes_related_await() {
    let source = r#"
        async function main(ready: Promise<void>): Promise<void> {
            const name: string = "Ada";
            const borrowed: &string = &name;
            await ready;
            console.log(borrowed);
        }
    "#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3082")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp = diagnostic_to_lsp(&uri, source, diagnostic);
    let await_start = source.find("await").unwrap();
    let related = lsp.related_information.unwrap();
    assert_eq!(related.len(), 1);
    assert_eq!(
        related[0].location.range,
        byte_range_to_lsp(source, await_start..await_start + "await".len())
    );
    assert!(related[0].message.contains("suspension point"));
}

#[test]
pub(super) fn async_closure_capture_diagnostic_exposes_precise_reference_and_declaration() {
    let source = "function setup(): void {\nconst values: Set<i32> = Set<i32>();\nconst calculate = async (): Promise<i32> => values.size;\nconsole.log(values.size);\n}\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3085")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp = diagnostic_to_lsp(&uri, source, diagnostic);
    let closure_start = source.find("async").unwrap();
    let captured_start = source[closure_start..].find("values").unwrap() + closure_start;
    assert_eq!(
        lsp.range,
        byte_range_to_lsp(source, captured_start..captured_start + "values".len())
    );
    let related = lsp.related_information.unwrap();
    assert_eq!(related.len(), 1);
    let declaration_start = source.find("values").unwrap();
    assert_eq!(
        related[0].location.range,
        byte_range_to_lsp(
            source,
            declaration_start..declaration_start + "values".len()
        )
    );
    assert!(related[0].message.contains("declared here"));
}

#[test]
pub(super) fn null_literal_diagnostic_is_exposed_in_lsp() {
    let source = "const value = null;\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3080")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp = diagnostic_to_lsp(&uri, source, diagnostic);

    assert_eq!(lsp.code, Some(NumberOrString::String("E3080".to_string())));
    assert_eq!(lsp.source.as_deref(), Some("tyrus"));
    assert_eq!(
        lsp.severity,
        Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR)
    );
}

#[test]
pub(super) fn lint_diagnostics_publish_warning_severity() {
    let source = "const main = (): void => {\n    const unused: string = \"x\";\n};\n";
    let diagnostics = tyrus_lint::lint_source(source, &tyrus_lint::LintConfig::default());
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "no-unused-vars")
        .unwrap_or_else(|| panic!("diagnostics: {diagnostics:#?}"));
    let lsp = lint_diagnostic_to_lsp(source, diagnostic);

    assert_eq!(
        lsp.severity,
        Some(tower_lsp::lsp_types::DiagnosticSeverity::WARNING)
    );
    assert_eq!(lsp.source.as_deref(), Some("tyrus-lint"));
    assert_eq!(
        lsp.code,
        Some(NumberOrString::String("no-unused-vars".to_string()))
    );
}

#[test]
pub(super) fn definite_assignment_diagnostics_are_lint_owned_in_lsp() {
    let source =
        "const main = (): void => {\n    let message: string;\n    console.log(message);\n};\n";
    let analysis = analyze(source);
    let core = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0381")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    assert!(backend::is_lint_owned_analysis_diagnostic(core));

    let diagnostics = tyrus_lint::lint_source(source, &tyrus_lint::LintConfig::default());
    let lint = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "no-uninitialized-variable")
        .unwrap_or_else(|| panic!("diagnostics: {diagnostics:#?}"));
    let lsp = lint_diagnostic_to_lsp(source, lint);

    assert_eq!(lsp.source.as_deref(), Some("tyrus-lint"));
    assert_eq!(
        lsp.code,
        Some(NumberOrString::String(
            "no-uninitialized-variable".to_string()
        ))
    );
    assert_eq!(
        lsp.severity,
        Some(tower_lsp::lsp_types::DiagnosticSeverity::WARNING)
    );
}

#[test]
pub(super) fn moved_value_diagnostics_remain_compiler_owned_in_lsp() {
    let source = r#"
        const main = (): void => {
            const name: string = "Ada";
            const moved: string = name;
            console.log(moved);
            console.log(name);
        };
    "#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "E0382" && diagnostic.message.contains("use of moved value")
        })
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));

    assert!(!backend::is_lint_owned_analysis_diagnostic(diagnostic));
}

#[test]
pub(super) fn dropped_value_diagnostic_uses_the_invalid_use_range_in_lsp() {
    let source = r#"
        import { drop } from "tyrus:memory";
        const main = (): void => {
            const token: string = "secret";
            drop(token);
            console.log(token);
        };
    "#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.message.contains("use of dropped value: `token`"))
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    let lsp = diagnostic_to_lsp(&uri, source, diagnostic);
    let invalid_use = source.rfind("token").unwrap();
    assert_eq!(
        lsp.range,
        byte_range_to_lsp(source, invalid_use..invalid_use + "token".len())
    );
    assert_eq!(lsp.related_information.as_ref().map(Vec::len), Some(1));
}

#[test]
pub(super) fn reference_type_mismatch_diagnostics_include_related_information() {
    let source = r#"
        const copyNewName: string = "TitenQ";
        const copyBorrowedNewName: string = &copyNewName;
    "#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "E0308" && diagnostic.message.contains("copyBorrowedNewName")
        })
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let uri = Url::parse("file:///test.tyr").unwrap();
    let lsp = diagnostic_to_lsp(&uri, source, diagnostic);
    assert_eq!(
        lsp.message,
        "mismatched types for `copyBorrowedNewName`: expected `string`, found `&string`"
    );
    assert!(!lsp.message.contains("change the annotation to `&string`"));
    assert!(!lsp.message.contains("line "));
    assert!(
        !lsp.message
            .contains("const copyBorrowedNewName: string = &copyNewName;")
    );
    assert!(!lsp.message.contains("^^^^"));
    assert!(!lsp.message.contains(" --> "));
    assert!(!lsp.message.contains(" | "));
    let related = lsp.related_information.unwrap();
    assert_eq!(related.len(), 1);
    assert_eq!(related[0].location.uri, uri);
    assert!(related[0].message.contains("creates a borrow"));
}

#[test]
pub(super) fn env_schema_diagnostic_maps_to_lsp_range_on_the_env_file() {
    let env_source = "@type(u32)\nPORT=not-a-number\n";
    let (vars, parse_diagnostics) = tyrus_core::env::parse_env_source(env_source);
    assert!(parse_diagnostics.is_empty());
    let diagnostics = tyrus_core::env::check_env_schema(&vars);
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3143")
        .expect("expected E3143");
    let env_uri = Url::parse("file:///project/.env").unwrap();
    let lsp = diagnostic_to_lsp(&env_uri, env_source, diagnostic);
    assert_eq!(lsp.range.start.line, 1);
    assert_eq!(lsp.range.start.character, 5);
    assert_eq!(lsp.range.end.character, 5 + "not-a-number".len() as u32);
    assert_eq!(lsp.code, Some(NumberOrString::String("E3143".to_string())));
}

#[test]
pub(super) fn env_validator_diagnostic_maps_to_lsp_value_range() {
    let env_source = "@type(string)\n@minLength(5)\nTEST=test\n";
    let (vars, parse_diagnostics) = tyrus_core::env::parse_env_source(env_source);
    assert!(parse_diagnostics.is_empty());
    let diagnostics = tyrus_core::env::check_env_schema(&vars);
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3143")
        .expect("expected the invalid environment value to be diagnosed");
    let env_uri = Url::parse("file:///project/.env").unwrap();
    let lsp = diagnostic_to_lsp(&env_uri, env_source, diagnostic);
    assert_eq!(lsp.range.start.line, 2);
    assert_eq!(lsp.range.start.character, 5);
    assert_eq!(lsp.range.end.character, 9);
    assert_eq!(lsp.code, Some(NumberOrString::String("E3143".to_string())));
    assert_eq!(lsp.related_information.as_ref().map(Vec::len), Some(1));
}

#[test]
pub(super) fn env_file_missing_diagnostic_anchors_on_the_env_file_value() {
    let dir = std::env::temp_dir().join(format!("tyrus-env-missing-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let config = "{\n  \"project\": { \"name\": \"x\", \"version\": \"0.1.0\" },\n  \"build\": { \"entry\": \"src/main.tyr\" },\n  \"security\": { \"envFile\": \".env\" }\n}\n";
    std::fs::write(dir.join("tyrus.json"), config).unwrap();
    let diagnostic =
        crate::backend::env_file_missing_diagnostic(&dir, "env file `.env` was not found")
            .expect("expected E3083 diagnostic");
    let value_start = config.find("\".env\"").unwrap() + 1;
    let before = &config[..value_start];
    let line = before.matches('\n').count() as u32;
    let character = (value_start - before.rfind('\n').map_or(0, |i| i + 1)) as u32;
    assert_eq!(diagnostic.range.start.line, line);
    assert_eq!(diagnostic.range.start.character, character);
    assert_eq!(
        diagnostic.code,
        Some(NumberOrString::String("E3083".to_string()))
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
pub(super) fn figlet_diagnostics_point_at_the_erroring_source() {
    let mut import_types = tyrus_core::collections::HashMap::default();
    import_types.insert("figlet".to_string(), Type::Named("__Figlet".to_string()));
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
    let unknown = "import { figlet } from \"tyrus:utils\";\n\
                   const ok = figlet(\"banner\", \"small\");\n\
                   const bad = figlet(\"banner\", \"wide\");\n\
                   console.log(bad);\n";
    assert_eq!(range_of(unknown, "E3277"), "\"wide\"");

    let non_literal = "import { figlet } from \"tyrus:utils\";\n\
                       const font: string = \"slant\";\n\
                       const bad = figlet(\"banner\", font);\n\
                       console.log(font);\n\
                       console.log(bad);\n";
    assert_eq!(range_of(non_literal, "E3276"), "font");

    let arity = "import { figlet } from \"tyrus:utils\";\n\
                 const bad = figlet(\"banner\", \"small\", \"extra\");\n\
                 console.log(bad);\n";
    assert_eq!(
        range_of(arity, "E3274"),
        "figlet(\"banner\", \"small\", \"extra\")"
    );
}

#[test]
pub(super) fn test_modifier_diagnostic_publishes_the_member_range_and_help_to_lsp() {
    use tyrus_core::analysis::{AnalyzeContext, analyze_with_context};
    use tyrus_core::test::{TEST_TARGET, test_type};

    let source = "import { test } from \"tyrus:test\";\n\ntest.wat(\"unknown\", () => {});\ntest(\"later\", () => {});\n";
    let import_types = [("test".to_string(), test_type())].into_iter().collect();
    let import_targets = [("test".to_string(), TEST_TARGET.to_string())]
        .into_iter()
        .collect();
    let analysis = analyze_with_context(
        source,
        AnalyzeContext {
            import_types: Some(&import_types),
            import_targets: Some(&import_targets),
            ..AnalyzeContext::default()
        },
    );
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0599")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    assert_eq!(diagnostic.message, "no modifier named `wat` on `test`");
    let uri = Url::parse("file:///suite.test.tyr").unwrap();
    let lsp_diagnostic = diagnostic_to_lsp(&uri, source, diagnostic);
    let member = source.find("wat").expect("member is present");
    assert_eq!(
        lsp_diagnostic.range,
        byte_range_to_lsp(source, member..member + "wat".len())
    );
    assert_eq!(
        diagnostic.help,
        ["the available modifiers are `test.skip`, `test.only`, `test.todo` or `test.each`"]
    );
}

#[test]
pub(super) fn custom_validator_diagnostics_point_to_the_signature_or_decorator() {
    let source = "type Validator = { isValid: true } | { isValid: false; message: string };\nvalidator function missingParameter(): boolean { return true; }\nvalidator function textOnly(value: string): Validator { return { isValid: true }; }\ninterface Account { @textOnly amount: i32; }\n";
    let analysis = analyze(source);
    let uri = Url::parse("file:///workspace/src/main.tyr").unwrap();
    for (code, expected) in [
        (
            "E3168",
            "validator function missingParameter(): boolean { return true; }",
        ),
        ("E3169", "@textOnly"),
    ] {
        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == code)
            .unwrap_or_else(|| panic!("expected {code}"));
        assert_eq!(&source[diagnostic.range.clone()], expected);
        let lsp = diagnostic_to_lsp(&uri, source, diagnostic);
        let start = position_to_byte_offset(source, lsp.range.start);
        let end = position_to_byte_offset(source, lsp.range.end);
        assert_eq!(&source[start..end], expected);
        assert!(!diagnostic.help.is_empty());
    }
}
