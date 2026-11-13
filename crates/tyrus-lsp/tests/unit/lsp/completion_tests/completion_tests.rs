use super::*;

#[test]
pub(super) fn declared_type_completion_reuses_the_analysis_program() {
    let source = "/** Public account fields. */\ninterface Account { id: string; }\ntype AccountId = string;\nenum AccountState { Active, Closed }";
    let analysis = tyrus_core::analysis::analyze(source);
    let items = crate::completion::type_completion_items(source, analysis.program.as_ref());

    assert!(items.iter().any(|item| item.label == "Account"));
    assert!(items.iter().any(|item| item.label == "AccountId"));
    assert!(items.iter().any(|item| item.label == "AccountState"));
    assert!(
        items
            .iter()
            .find(|item| item.label == "Account")
            .and_then(|item| item.documentation.as_ref())
            .is_some()
    );
}

#[test]
pub(super) fn parse_decimal_builtin_has_completion_hover_and_signature_metadata() {
    use crate::backend::global_builtin_call_signature;
    use crate::completion::global_builtin_completion_items;
    use crate::server::global_builtin_hover_at;

    let completion = global_builtin_completion_items()
        .into_iter()
        .find(|item| item.label == "parseDecimal")
        .unwrap();
    assert_eq!(
        completion.detail.as_deref(),
        Some("(text: string) => decimal?")
    );
    assert_eq!(
        completion.insert_text.as_deref(),
        Some("parseDecimal(${1:text})")
    );
    let signature = global_builtin_call_signature("parseDecimal").unwrap();
    assert_eq!(signature.parameters, ["text"]);
    assert_eq!(
        signature.detail.as_deref(),
        Some("(text: string) => decimal?")
    );
    let source = "const value = parseDecimal(\"19.3\");";
    let analysis = tyrus_core::analysis::analyze(source);
    let (range, markdown) = global_builtin_hover_at(&analysis, 18).unwrap();
    assert_eq!(&source[range], "parseDecimal");
    assert!(markdown.contains("decimal?"));
}

#[test]
pub(super) fn is_nan_builtin_has_completion_hover_and_signature_metadata() {
    use crate::backend::global_builtin_call_signature;
    use crate::completion::global_builtin_completion_items;
    use crate::server::global_builtin_hover_at;

    let completion = global_builtin_completion_items()
        .into_iter()
        .find(|item| item.label == "isNaN")
        .unwrap();
    assert_eq!(
        completion.detail.as_deref(),
        Some("(value: f32 | f64 | f128) => boolean")
    );
    assert_eq!(completion.insert_text.as_deref(), Some("isNaN(${1:value})"));
    let signature = global_builtin_call_signature("isNaN").unwrap();
    assert_eq!(signature.parameters, ["value"]);
    assert_eq!(
        signature.detail.as_deref(),
        Some("(value: f32 | f64 | f128) => boolean")
    );
    let source = "const invalid = isNaN(NaN);";
    let analysis = tyrus_core::analysis::analyze(source);
    let (range, markdown) = global_builtin_hover_at(&analysis, 18).unwrap();
    assert_eq!(&source[range], "isNaN");
    assert!(markdown.contains("boolean"));
}

#[test]
pub(super) fn print_builtins_have_global_completion_items() {
    use crate::completion::global_builtin_completion_items;

    let items = global_builtin_completion_items();
    for (name, detail) in [
        ("print", "write a line to stdout"),
        ("printError", "write an error line to stderr"),
        ("printWarn", "write a warning line to stderr"),
        ("printRaw", "write to stdout without a newline"),
        ("printCR", "update the current stdout line"),
    ] {
        let item = items
            .iter()
            .find(|item| item.label == name)
            .unwrap_or_else(|| panic!("missing completion for `{name}`"));
        let expected_insert_text = format!("{name}($0);");
        assert_eq!(item.detail.as_deref(), Some(detail));
        assert_eq!(
            item.insert_text.as_deref(),
            Some(expected_insert_text.as_str())
        );
    }
}

#[test]
pub(super) fn structural_object_completion_exposes_fields_and_projection_methods() {
    let source = "interface Account { readonly id: i32; nickname?: string; }";
    let analysis = analyze(source);
    let fields = match analysis
        .symbol_named("Account")
        .unwrap()
        .semantic_type
        .as_ref()
    {
        Some(Type::Object(fields)) => fields.clone(),
        other => panic!("expected object type, got {other:?}"),
    };
    let items = member_completion_items_for_type(&Type::Object(fields)).unwrap();
    let id = items.iter().find(|item| item.label == "id").unwrap();
    assert_eq!(id.kind, Some(CompletionItemKind::FIELD));
    assert_eq!(id.detail.as_deref(), Some("readonly i32"));
    let nickname = items.iter().find(|item| item.label == "nickname").unwrap();
    assert_eq!(nickname.detail.as_deref(), Some("string?"));
    for method in ["pick", "omit"] {
        let item = items.iter().find(|item| item.label == method).unwrap();
        assert_eq!(item.kind, Some(CompletionItemKind::METHOD));
        assert!(item.detail.as_deref().unwrap().contains("\"id\""));
        assert_eq!(item.insert_text_format, Some(InsertTextFormat::SNIPPET));
    }

    let source = "interface Account { id: i32; nickname?: string; }\nconst account: Account = { id: 1 };\naccount.";
    let analysis = analyze(source);
    let items = member_completion_items(source, source.len(), &analysis).unwrap();
    assert!(items.iter().any(|item| item.label == "id"));
    assert!(items.iter().any(|item| item.label == "pick"));
}

#[test]
pub(super) fn object_projection_key_completion_recognizes_only_a_direct_receiver() {
    use crate::completion::{
        object_projection_key_completion_receiver, object_projection_type_key_completion_receiver,
    };

    assert_eq!(
        object_projection_key_completion_receiver("account.pick([\"", "account.pick([\"".len()),
        Some("account".to_string())
    );
    assert_eq!(
        object_projection_key_completion_receiver("account.omit([", "account.omit([".len()),
        Some("account".to_string())
    );
    assert!(
        object_projection_key_completion_receiver(
            "account.pick([\"id\"])",
            "account.pick([\"id\"])".len()
        )
        .is_none()
    );
    assert!(
        object_projection_key_completion_receiver(
            "makeAccount().pick([",
            "makeAccount().pick([".len()
        )
        .is_none()
    );
    assert_eq!(
        object_projection_type_key_completion_receiver(
            "type Public = Pick<Account, \"",
            "type Public = Pick<Account, \"".len()
        ),
        Some("Account".to_string())
    );
    assert_eq!(
        object_projection_type_key_completion_receiver(
            "type Stored = Optional<Account, ",
            "type Stored = Optional<Account, ".len()
        ),
        Some("Account".to_string())
    );
}

#[test]
pub(super) fn completion_includes_documentation_for_symbols() {
    let source = r#"/**
 * Returns a constant.
 *
 * @param value The input value
 * @returns The same value plus one
 */
const answer = (value: i32): i32 => value + 1;"#;
    let analysis = analyze(source);
    let symbol = analysis.symbol_named("answer").unwrap();
    let item = symbol_completion_item(symbol);
    let docs = match item.documentation {
        Some(Documentation::MarkupContent(content)) => content.value,
        _ => panic!("expected markup documentation"),
    };
    assert!(docs.contains("Returns a constant."));
    assert!(docs.contains("@param value, The input value"));
    assert!(docs.contains("@returns The same value plus one"));
}

#[test]
pub(super) fn validator_completion_offers_decorators_after_at_sign() {
    let source = "interface UserInput {\n    @em\n    email: string;\n}\n";
    let offset = source.find("@em").unwrap() + "@em".len();
    let analysis = analyze(source);
    let items =
        validator_completion_items(source, offset, &analysis).expect("validator completions");
    let email = items
        .iter()
        .find(|item| item.label == "email")
        .expect("email completion");
    assert_eq!(email.kind, Some(CompletionItemKind::FUNCTION));
    assert_eq!(email.insert_text.as_deref(), Some("email($0)"));
    assert!(
        email
            .detail
            .as_deref()
            .is_some_and(|detail| detail.starts_with("@email("))
    );
}

#[test]
pub(super) fn custom_validator_completion_uses_core_symbols_and_configuration_parameters() {
    let source = "type Validator = { isValid: true } | { isValid: false; message: string };\nvalidator const isPublic = (value: string, constraint: string): Validator => { return { isValid: true }; };\ninterface Account { @isPu username: string; }\n";
    let offset = source.find("@isPu").unwrap() + "@isPu".len();
    let analysis = analyze(source);
    let items = validator_completion_items(source, offset, &analysis)
        .expect("custom validator completions");
    let item = items
        .iter()
        .find(|item| item.label == "isPublic")
        .expect("custom validator completion");
    assert_eq!(item.insert_text.as_deref(), Some("isPublic(${1})"));
    assert_eq!(
        item.detail.as_deref(),
        Some("validator function isPublic(value: string, constraint: string): Validator")
    );
}

#[test]
pub(super) fn abrupt_completions_through_suspending_finally_are_accepted_by_lsp_analysis() {
    let source = "async function pause(): Promise<void> { return; }\nasync function selected(): Promise<i32> { try { return 7; } catch (error: Error) { return 0; } finally { await pause(); } }\nasync function transferred(flag: boolean): Promise<void> { while (flag) { try { if (flag) { continue; } break; } catch (error: Error) {} finally { await pause(); } } }\n";
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
pub(super) fn array_static_completion_respects_local_shadowing() {
    let source = "const Array: string = \"local\"; Array.";
    let analysis = analyze(source);
    let items = member_completion_items(source, source.len(), &analysis).unwrap();
    assert!(!items.iter().any(|item| item.label == "isArray"));
}

#[test]
pub(super) fn member_completion_does_not_fall_back_to_globals_for_dangling_dot() {
    let source = r#"
        const message: string = "Hello";

        console.log(message.
    "#;
    let analysis = analyze(source);
    let items = member_completion_items(source, source.len(), &analysis).unwrap();

    assert!(items.iter().any(|item| item.label == "charAt"));
    assert!(items.iter().any(|item| item.label == "length"));
    assert!(!items.iter().any(|item| item.label == "import"));
    assert!(!items.iter().any(|item| item.label == "i32"));
}

#[test]
pub(super) fn member_completion_offers_regexp_members() {
    let source = r#"
        const pattern: Regex = /tyrus/i;

        console.log(pattern.
    "#;
    let analysis = analyze(source);
    let items = member_completion_items(source, source.len(), &analysis).unwrap();

    assert!(items.iter().any(|item| item.label == "test"));
    assert!(items.iter().any(|item| item.label == "exec"));
    assert!(items.iter().any(|item| item.label == "source"));
    assert!(items.iter().any(|item| item.label == "lastIndex"));
    assert!(!items.iter().any(|item| item.label == "charAt"));
}

#[test]
pub(super) fn promise_static_completion_lists_native_methods() {
    let source = "const value = Promise.";
    let analysis = analyze(source);
    let items = member_completion_items(source, source.len(), &analysis).unwrap();
    for expected in [
        "resolve",
        "reject",
        "all",
        "allSettled",
        "any",
        "race",
        "withResolvers",
    ] {
        assert!(
            items.iter().any(|item| item.label == expected),
            "missing Promise static completion `{expected}`"
        );
    }
    assert!(
        items
            .iter()
            .find(|item| item.label == "all")
            .and_then(|item| item.documentation.as_ref())
            .is_some()
    );
}

#[test]
pub(super) fn promise_instance_completion_lists_native_methods() {
    let source = "const task: Promise<i32> = getTask();\ntask.";
    let analysis = analyze(source);
    let items = member_completion_items(source, source.len(), &analysis).unwrap();
    for expected in ["then", "catch", "finally"] {
        assert!(
            items.iter().any(|item| item.label == expected),
            "missing Promise instance completion `{expected}`"
        );
    }
    let then = items.iter().find(|item| item.label == "then").unwrap();
    assert!(
        then.detail
            .as_deref()
            .is_some_and(|detail| detail.contains("value: i32"))
    );
}

#[test]
pub(super) fn join_handle_completion_exposes_join_and_abort() {
    let items = crate::completion::join_handle_completion_items(&Type::I32);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].label, "join");
    assert_eq!(items[0].detail.as_deref(), Some("() => Promise<i32>"));
    assert_eq!(items[1].label, "abort");
    assert_eq!(items[1].detail.as_deref(), Some("() => void"));
}

#[test]
pub(super) fn detects_statement_completion_context() {
    assert!(completion_starts_statement(
        "const main = (): void => {\n    run",
        "const main = (): void => {\n    run".len()
    ));
    assert!(completion_starts_statement(
        "foo();\nrun",
        "foo();\nrun".len()
    ));
    assert!(!completion_starts_statement(
        "const result = run",
        "const result = run".len()
    ));
    assert!(!completion_starts_statement(
        "return run",
        "return run".len()
    ));
}

#[test]
pub(super) fn env_completion_offers_type_and_validators() {
    let source = "@t\nPORT=3000\n";
    let offset = source.find("@t").unwrap() + "@t".len();
    let items = crate::completion::env_completion_items(source, offset)
        .expect("expected env completions at an `@` prefix");
    assert!(items.iter().any(|item| item.label == "type"));
    assert!(items.iter().any(|item| item.label == "min"));
    assert!(items.iter().any(|item| item.label == "email"));
    let type_item = items.iter().find(|item| item.label == "type").unwrap();
    assert_eq!(type_item.insert_text.as_deref(), Some("type(${1:string})"));
}

#[test]
pub(super) fn env_completion_absent_without_an_at_prefix() {
    assert!(crate::completion::env_completion_items("PORT=3000\n", 4).is_none());
}

#[test]
pub(super) fn env_completion_suggests_type_names_inside_type_decorator() {
    let source = "@type(\nPORT=3000\n";
    let offset = source.find("@type(").unwrap() + "@type(".len();
    let items = crate::completion::env_completion_items(source, offset)
        .expect("expected type-name completions inside @type(");
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(labels.contains(&"string"));
    assert!(labels.contains(&"u32"));
    assert!(labels.contains(&"boolean"));
    assert!(!labels.contains(&"type"));
    assert!(!labels.contains(&"min"));
}

#[test]
pub(super) fn style_text_completion_offers_all_styles() {
    use crate::completion::style_text_completion_items;
    let inserts = |source: &str| -> Vec<String> {
        style_text_completion_items(source, source.len())
            .expect("expected style completions")
            .into_iter()
            .filter(|item| item.label == "green")
            .map(|item| item.insert_text.unwrap_or(item.label))
            .collect()
    };
    let src = "const a = styleText.";
    let items = style_text_completion_items(src, src.len()).expect("styles after styleText.");
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"green") && labels.contains(&"bold") && labels.contains(&"bgRed"));
    assert_eq!(inserts("const a = styleText."), ["green"]);
    assert_eq!(inserts("const a = styleText.green."), ["green"]);
    assert_eq!(inserts("const a = styleText("), ["\"green\""]);
    assert_eq!(inserts("const a = styleText(\""), ["green"]);
    assert_eq!(inserts("const a = styleText([\"bold\", \""), ["green"]);
    assert_eq!(inserts("const a = styleText([\"bold\", "), ["\"green\""]);
    let text_arg = "const a = styleText(\"green\", \"";
    assert!(style_text_completion_items(text_arg, text_arg.len()).is_none());
    let member = "const a = foo.";
    assert!(style_text_completion_items(member, member.len()).is_none());
}

#[test]
pub(super) fn response_header_completion_offers_the_security_headers() {
    use crate::completion::response_header_completion_items;
    let source = "reply.header(\"";
    let items = response_header_completion_items(source, source.len())
        .expect("header names inside `reply.header(\"`");
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    for header in [
        "x-content-type-options",
        "referrer-policy",
        "x-frame-options",
        "cross-origin-resource-policy",
    ] {
        assert!(labels.contains(&header), "`{header}` is not offered");
    }
    assert_eq!(labels[0], "x-content-type-options");

    let inside = |source: &str, label: &str| -> String {
        response_header_completion_items(source, source.len())
            .expect("header names")
            .into_iter()
            .find(|item| item.label == label)
            .and_then(|item| item.insert_text)
            .expect("an insert text")
    };
    assert_eq!(
        inside("reply.header(\"", "x-frame-options"),
        "x-frame-options"
    );
    assert_eq!(
        inside("reply.header(", "x-frame-options"),
        "\"x-frame-options\", \"DENY\""
    );
    assert_eq!(
        inside("reply.header(", "content-security-policy"),
        "\"content-security-policy\""
    );

    for method in ["appendHeader", "removeHeader"] {
        let source = format!("reply.{method}(\"");
        assert!(
            response_header_completion_items(&source, source.len()).is_some(),
            "`{method}` offers no header names"
        );
    }
    let value = "reply.header(\"etag\", \"";
    assert!(response_header_completion_items(value, value.len()).is_none());
    let closed = "reply.header(\"etag\") ";
    assert!(response_header_completion_items(closed, closed.len()).is_none());
}

#[test]
pub(super) fn figlet_completion_offers_builtin_fonts() {
    use crate::completion::figlet_font_completion_items;
    let inserts = |source: &str| -> Vec<String> {
        figlet_font_completion_items(source, source.len())
            .expect("expected font completions")
            .into_iter()
            .filter(|item| item.label == "slant")
            .map(|item| item.insert_text.unwrap_or(item.label))
            .collect()
    };
    let source = "const a = figlet(\"hi\", ";
    let items = figlet_font_completion_items(source, source.len()).expect("fonts after comma");
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert_eq!(labels, ["standard", "small", "big", "slant"]);
    assert_eq!(inserts("const a = figlet(\"hi\", "), ["\"slant\""]);
    assert_eq!(inserts("const a = figlet(\"hi\", \""), ["slant"]);
    assert_eq!(
        inserts("const a = figlet(\"hello, world\", "),
        ["\"slant\""]
    );
    let no_font = "const a = figlet(\"hi\"";
    assert!(figlet_font_completion_items(no_font, no_font.len()).is_none());
    let style_text = "const a = styleText(\"";
    assert!(figlet_font_completion_items(style_text, style_text.len()).is_none());
}

#[test]
pub(super) fn encoding_member_hover_and_completion_support_aliased_imports() {
    let source = r#"import { utf16 as windowsText } from "tyrus:encoding";
const bytes: u8[] = windowsText.encodeLE("A😀");
windowsText.
"#;
    let offset = source.find("encodeLE").unwrap() + 3;
    let (range, markdown) = native_namespace_member_hover_at(source, offset).unwrap();
    let start = source.find("encodeLE").unwrap();
    assert_eq!(range, start..start + "encodeLE".len());
    assert!(markdown.contains("explicitly selected byte order"));
    let analysis = analyze(source);
    let items = member_completion_items(source, source.len(), &analysis).unwrap();
    let decode_le = items.iter().find(|item| item.label == "decodeLE").unwrap();
    assert!(decode_le.detail.as_deref().unwrap().contains("u8[]"));
    let Some(Documentation::String(documentation)) = &decode_le.documentation else {
        panic!("expected UTF-16 completion documentation");
    };
    assert!(documentation.contains("EncodingError"));
}

#[test]
pub(super) fn hex_namespace_hover_and_completion_document_members() {
    let source = r#"import { hex } from "tyrus:encoding";
const text: string = hex.decode("deadbeef");
hex.
"#;
    let offset = source.find("decode").unwrap() + 3;
    let (range, markdown) = native_namespace_member_hover_at(source, offset).unwrap();
    let start = source.find("decode").unwrap();
    assert_eq!(range, start..start + "decode".len());
    assert!(markdown.contains("EncodingError"));
    let analysis = analyze(source);
    let items = member_completion_items(source, source.len(), &analysis).unwrap();
    let encode = items.iter().find(|item| item.label == "encode").unwrap();
    assert!(encode.detail.as_deref().unwrap().contains("string"));
    let Some(Documentation::String(documentation)) = &encode.documentation else {
        panic!("expected hex encode completion documentation");
    };
    assert!(documentation.contains("lowercase hexadecimal"));
}

#[test]
pub(super) fn crypto_member_completion_documents_md5_as_legacy_only() {
    let source = "import { digest as hashes } from \"tyrus:crypto\";\nhashes.";
    let analysis = analyze(source);
    let items = member_completion_items(source, source.len(), &analysis).unwrap();
    let md5 = items.iter().find(|item| item.label == "md5").unwrap();
    let Some(Documentation::String(documentation)) = &md5.documentation else {
        panic!("expected MD5 completion documentation");
    };
    assert!(documentation.contains("cryptographically broken"));
    assert!(documentation.contains("Legacy compatibility"));
}

#[test]
pub(super) fn crypto_argon2_completion_documents_pepper_storage_and_limits() {
    let source = "import { argon2 as passwords } from \"tyrus:crypto\";\npasswords.";
    let analysis = analyze(source);
    let items = member_completion_items(source, source.len(), &analysis).unwrap();
    let hash = items.iter().find(|item| item.label == "hash").unwrap();
    assert!(hash.detail.as_deref().unwrap().contains("secret"));
    let Some(Documentation::String(documentation)) = &hash.documentation else {
        panic!("expected Argon2 completion documentation");
    };
    assert!(documentation.contains("bounded blocking pool"));
    assert!(documentation.contains("rejects a missing `secret`"));
    assert!(documentation.contains("stored separately"));
    let algorithm = items.iter().find(|item| item.label == "argon2id").unwrap();
    assert!(algorithm.detail.as_deref().unwrap().contains("string"));
}

#[test]
pub(super) fn crypto_jwt_completion_uses_compiler_metadata_for_jwks_methods() {
    let source = "import { jwt as tokens } from \"tyrus:crypto\";\ntokens.";
    let analysis = analyze(source);
    let items = member_completion_items(source, source.len(), &analysis).unwrap();
    for expected in ["fetchJwks", "sign", "verify", "verifyWithJwks"] {
        assert!(
            items.iter().any(|item| item.label == expected),
            "missing `{expected}`"
        );
    }
    let fetch = items.iter().find(|item| item.label == "fetchJwks").unwrap();
    assert!(
        fetch
            .detail
            .as_deref()
            .unwrap()
            .contains("Promise<Result<string, CryptoError>>")
    );
    let Some(Documentation::String(documentation)) = &fetch.documentation else {
        panic!("expected JWKS fetch documentation");
    };
    assert!(documentation.contains("HTTPS"));
    assert!(documentation.contains("DNS"));
    assert!(documentation.contains("single-flight"));
}
