use super::*;

#[test]
pub(super) fn hover_markdown_includes_documentation() {
    let doc = DocComment {
        range: 0..1,
        summary: "Greets the caller.".to_string(),
        description: "Greets the caller.".to_string(),
        ..Default::default()
    };
    let markdown = hover_markdown("greet", "(name: string) => void", None, Some(&doc));
    assert!(markdown.starts_with("Greets the caller."));
    assert!(markdown.contains("```tyrus\ngreet: (name: string) => void\n```"));
}

#[test]
pub(super) fn async_hover_markdown_shows_inferred_promise_return() {
    let source = "async function load(value: i32) { return value + 1; }\n";
    let analysis = analyze(source);
    let symbol = analysis.symbol_named("load").unwrap();
    let markdown = hover_markdown(
        &symbol.name,
        &symbol.ty,
        None,
        symbol.documentation.as_ref(),
    );
    assert!(markdown.contains("```tyrus\nload: (value: i32) => Promise<i32>\n```"));
}

#[test]
pub(super) fn interface_hover_renders_fields_and_link() {
    let source = r#"export interface Account {
    id: i32;
    name: string;
    email: string?;
}"#;
    let analysis = analyze(source);
    let symbol = analysis.symbol_named("Account").unwrap();
    let fields = match symbol.semantic_type.as_ref() {
        Some(tyrus_core::ast::Type::Object(fields)) => fields.clone(),
        other => panic!("expected interface object type, got {other:?}"),
    };
    let markdown = interface_hover_markdown("Account", &fields, None, Some("file:///acc.tyr#L1"));
    assert!(markdown.contains("interface Account {"));
    assert!(markdown.contains("id: i32;"));
    assert!(markdown.contains("name: string;"));
    assert!(markdown.contains("email: string?;"));
    assert!(markdown.contains("[Go to definition](file:///acc.tyr#L1)"));
}

#[test]
pub(super) fn interface_hover_without_link_omits_it() {
    let markdown = interface_hover_markdown("Empty", &[], None, None);
    assert!(markdown.contains("interface Empty {}"));
    assert!(!markdown.contains("Go to definition"));
}

#[test]
pub(super) fn custom_validator_decorator_reference_resolves_for_hover() {
    let source = "type Validator = { isValid: true } | { isValid: false; message: string };\nvalidator const noSpaces = (value: string): Validator => { return { isValid: true }; };\ninterface User { @noSpaces() name: string; }";
    let analysis = analyze(source);
    let decorator_offset = source.rfind("noSpaces").unwrap();
    let symbol = analysis
        .symbol_at(decorator_offset)
        .expect("custom validator decorator reference");
    assert_eq!(symbol.name, "noSpaces");
    assert!(symbol.is_validator);
    let markdown = hover_markdown(
        &symbol.name,
        &format!("validator function {}", symbol.ty),
        None,
        symbol.documentation.as_ref(),
    );
    assert!(markdown.contains("noSpaces: validator function"));
}

#[test]
pub(super) fn validator_hover_renders_signature_and_description() {
    let source = "interface UserInput { @email(\"invalid e-mail\") email: string; }";
    let offset = source.find("email").unwrap();
    let (_, markdown) = validator_hover_at(source, offset).expect("validator hover");
    assert!(markdown.contains("Validates a practical ASCII e-mail address string."));
    assert!(markdown.contains("@email("));
    assert!(markdown.contains("Default message: `value must be a valid e-mail address`"));
    assert!(markdown.contains("Concrete message: `invalid e-mail`"));
}

#[test]
pub(super) fn validator_hover_renders_concrete_default_message_with_arguments() {
    let source = "interface ProductFilter { @min(5) pageSize: i32; }";
    let offset = source.find("min").unwrap();
    let (_, markdown) = validator_hover_at(source, offset).expect("validator hover");
    assert!(markdown.contains("Concrete message: `value must be greater than or equal to 5`"));
}

#[test]
pub(super) fn comptime_hover_shows_the_evaluated_type_and_bounded_value() {
    let source = "comptime function build(): i32[] { return [20, 22]; }\nconst result: i32[] = comptime(build());\n";
    let analysis = analyze(source);
    assert!(
        analysis.diagnostics.is_empty(),
        "{:#?}",
        analysis.diagnostics
    );
    let start = source.rfind("comptime").unwrap();
    let (range, markdown) =
        crate::positions::comptime_hover_at(&analysis, start + 2).expect("expected comptime hover");
    assert_eq!(range, start..start + "comptime".len());
    assert!(markdown.contains("Evaluated at compile time."));
    assert!(markdown.contains("```tyrus\ni32[]\n```"), "{markdown}");
    assert!(markdown.contains("Value: `[20, 22]`"));
    assert!(
        crate::positions::comptime_hover_at(&analysis, source.rfind("build").unwrap()).is_none()
    );
}

#[test]
pub(super) fn comptime_hover_truncates_large_values() {
    let value = "a".repeat(200);
    let source = format!("const result: string = comptime(\"{value}\");\n");
    let analysis = analyze(&source);
    assert!(
        analysis.diagnostics.is_empty(),
        "{:#?}",
        analysis.diagnostics
    );
    let start = source.find("comptime").unwrap();
    let (_, markdown) =
        crate::positions::comptime_hover_at(&analysis, start).expect("expected comptime hover");
    assert!(markdown.contains('…'), "{markdown}");
    assert!(markdown.chars().count() < 180, "{markdown}");
}

#[test]
pub(super) fn diagnostic_hover_uses_markdown_with_tyrus_fix_example() {
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
    let markdown = diagnostic_hover_markdown(source, diagnostic);
    assert!(markdown.contains("**use of moved value: `name`**"));
    assert!(markdown.contains("`E0382`"));
    assert!(markdown.contains("```\n            console.log(name);"));
    assert!(markdown.contains("^^^^"));
    assert!(markdown.contains("value moved here"));
    assert!(markdown.contains("help: borrow `name` at the move site"));
    assert!(markdown.contains("```tyrus\nconst moved: &string = &name;\n```"));
}

#[test]
pub(super) fn native_promise_hover_describes_current_support() {
    let source = "const value: Promise<i32> = getValue();\n";
    let analysis = analyze(source);
    let offset = source.find("Promise").unwrap();
    let (range, markdown) = crate::server::native_promise_hover_at(&analysis, offset).unwrap();
    assert_eq!(range, offset..offset + "Promise".len());
    assert!(markdown.contains("Promise<T>"));
    assert!(markdown.contains("native asynchronous result type"));
    assert!(markdown.contains("failure propagation"));
}

#[test]
pub(super) fn result_hover_describes_success_and_error_payloads() {
    let source = "const value: Result<i32, string> = load();\n";
    let analysis = analyze(source);
    let offset = source.find("Result").unwrap();
    let (range, markdown) = crate::server::native_promise_hover_at(&analysis, offset).unwrap();
    assert_eq!(range, offset..offset + "Result".len());
    assert!(markdown.contains("Result<T, E>"));
    assert!(markdown.contains("success value"));
    assert!(markdown.contains("`match`"));
}

#[test]
pub(super) fn native_async_error_hovers_show_public_fields() {
    let source = "type Failure = Error | AggregateError;\n";
    let analysis = analyze(source);
    let error_offset = source.find("Error").unwrap();
    let (_, error) = crate::server::native_promise_hover_at(&analysis, error_offset).unwrap();
    assert!(error.contains("readonly message: string"));
    let aggregate_offset = source.find("AggregateError").unwrap();
    let (_, aggregate) =
        crate::server::native_promise_hover_at(&analysis, aggregate_offset).unwrap();
    assert!(aggregate.contains("readonly errors: Error[]"));
}

#[test]
pub(super) fn crypto_member_hover_exposes_security_contract_for_aliased_imports() {
    let source = r#"import { jwt as tokens } from "tyrus:crypto";
const claims: string = tokens.decodeUnverified(token);
"#;
    let offset = source.find("decodeUnverified").unwrap() + 3;
    let (range, markdown) = native_namespace_member_hover_at(source, offset).unwrap();
    let start = source.find("decodeUnverified").unwrap();
    assert_eq!(range, start..start + "decodeUnverified".len());
    assert!(markdown.contains("does not authenticate"));
    assert!(markdown.contains("must never be used for authorization"));
}

#[test]
pub(super) fn test_framework_hover_describes_matchers_and_modifiers() {
    let source = "import { describe, test, expect } from \"tyrus:test\";\n\ntest.skip(\"later\", () => {\n    expect(2).not.toBeCloseTo(3);\n});\n";
    let at = |needle: &str| {
        let start = source.find(needle).expect("fragment is present");
        start + 1
    };

    let (range, markdown) = test_framework_hover_at(source, at("skip(")).expect("modifier hover");
    assert_eq!(&source[range], "skip");
    assert!(markdown.contains("test.skip(name, callback)"), "{markdown}");
    assert!(markdown.contains("Test modifier."), "{markdown}");

    let (range, markdown) =
        test_framework_hover_at(source, at("toBeCloseTo(")).expect("matcher hover");
    assert_eq!(&source[range], "toBeCloseTo");
    assert!(markdown.contains("toBeCloseTo(expected)"), "{markdown}");
    assert!(markdown.contains("within `0.005`"), "{markdown}");

    let (range, markdown) = test_framework_hover_at(source, at("not.")).expect("not hover");
    assert_eq!(&source[range], "not");
    assert!(markdown.contains("Inverts the matcher"), "{markdown}");

    let settling = "import { expect } from \"tyrus:test\";\n\nexpect(load()).rejects.toThrow();\nexpect(load()).resolves.toBe(1);\n";
    let (range, markdown) =
        test_framework_hover_at(settling, settling.find("rejects").unwrap() + 1)
            .expect("rejects hover");
    assert_eq!(&settling[range], "rejects");
    assert!(markdown.contains("asserts that it failed"), "{markdown}");
    let (range, markdown) =
        test_framework_hover_at(settling, settling.find("resolves").unwrap() + 1)
            .expect("resolves hover");
    assert_eq!(&settling[range], "resolves");
    assert!(markdown.contains("the value it resolved to"), "{markdown}");

    let plain = "const value = 1;\nvalue.toBe(2);\n";
    assert!(test_framework_hover_at(plain, plain.find("toBe").unwrap() + 1).is_none());
}
