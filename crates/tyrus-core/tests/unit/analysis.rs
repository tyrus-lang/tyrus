use super::*;
use crate::span::Span;

#[test]
fn standalone_analysis_matches_compiler_diagnostics_without_project_context() {
    let source = "const answer: string = 42;\n";
    let compiler = analyze(source);
    let standalone = analyze_standalone_source(source);
    let diagnostics = |analysis: &DocumentAnalysis| {
        analysis
            .diagnostics
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic.code.clone(),
                    diagnostic.message.clone(),
                    diagnostic.range.clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(diagnostics(&standalone), diagnostics(&compiler));
}

#[test]
fn onfail_binding_uses_the_error_type_only_inside_its_handler() {
    let source = "function load(ok: boolean): Result<i32, string> { if (ok) { return Ok(1); } return Err(\"failed\"); }\nfunction render(): Result<void, string> { const result = load(false) onfail { console.error(result); }; console.log(result); return Ok(); }\n";
    let analysis = analyze(source);
    let result_symbols = analysis
        .symbols
        .iter()
        .filter(|symbol| symbol.name == "result")
        .collect::<Vec<_>>();
    assert_eq!(result_symbols.len(), 2);
    assert!(result_symbols.iter().any(|symbol| symbol.ty == "i32"));
    assert!(result_symbols.iter().any(|symbol| symbol.ty == "string"));

    let references = analysis
        .references
        .iter()
        .filter(|reference| reference.name == "result")
        .collect::<Vec<_>>();
    assert_eq!(references.len(), 2);
    let handler = references
        .iter()
        .find(|reference| {
            source[reference.range.clone()].starts_with("result")
                && reference.range.start < source.rfind("console.log(result)").unwrap()
        })
        .unwrap();
    let after_handler = references
        .iter()
        .find(|reference| reference.range.start > source.rfind("console.log(result)").unwrap())
        .unwrap();
    let handler_symbol = result_symbols
        .iter()
        .find(|symbol| symbol.declaration == handler.definition.clone().unwrap())
        .unwrap();
    let success_symbol = result_symbols
        .iter()
        .find(|symbol| symbol.declaration == after_handler.definition.clone().unwrap())
        .unwrap();
    assert_eq!(handler_symbol.ty, "string");
    assert_eq!(success_symbol.ty, "i32");
}

#[test]
fn comptime_invalid_target_points_to_the_call_and_relates_the_wrapper() {
    let source = "function runtime(value: i32): i32 { return value; }\nconst result: i32 = comptime(runtime(21));\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == crate::comptime::INVALID_TARGET
                && diagnostic.message.contains("runtime")
        })
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let call_start = source.rfind("runtime(21)").unwrap();
    assert_eq!(
        diagnostic.range,
        call_start..call_start + "runtime(21)".len()
    );
    assert!(diagnostic.related.iter().any(|related| {
        &source[related.range.clone()] == "comptime(runtime(21))"
            && related.message.contains("required by this wrapper")
    }));
    assert!(diagnostic.help.iter().any(|help| help.contains("runtime")));
}

#[test]
fn comptime_effect_analysis_publishes_the_transitive_call_chain() {
    let source = "function runtime(): i32 { return 1; }\ncomptime function leaf(): i32 { return runtime(); }\ncomptime function root(): i32 { return leaf(); }\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.message.contains("`root` transitively"))
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    assert_eq!(diagnostic.code, crate::comptime::INVALID_TARGET);
    assert_eq!(&source[diagnostic.range.clone()], "runtime()");
    assert!(diagnostic.related.iter().any(|related| {
        &source[related.range.clone()] == "leaf()"
            && related.message.contains("`root` calls `leaf`")
    }));
}

#[test]
fn defer_diagnostics_include_the_finalizer_or_return_type_origin() {
    let source = "const run = (): void => { errdefer cleanup(); defer { return; } };";
    let analysis = analyze(source);
    let errdefer = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0D01")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    assert_eq!(&source[errdefer.range.clone()], "errdefer");
    assert!(
        errdefer
            .related
            .iter()
            .any(|related| &source[related.range.clone()] == "void")
    );

    let return_ = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0D02")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    assert_eq!(&source[return_.range.clone()], "return;");
    assert!(
        return_
            .related
            .iter()
            .any(|related| &source[related.range.clone()] == "defer")
    );
    assert!(
        return_
            .help
            .iter()
            .any(|help| help.contains("without `return`"))
    );
}

#[test]
fn finalizer_capture_conflicts_identify_the_registration_and_name_the_binding() {
    let source = "const main = (): void => { const resource: string = \"open\"; defer console.log(resource); const moved: string = resource; };";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0D07")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    assert!(
        diagnostic
            .related
            .iter()
            .any(|related| &source[related.range.clone()] == "defer")
    );
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("`resource`"))
    );
}

#[test]
fn comptime_arrow_mutable_capture_points_to_use_and_declaration() {
    let source = "let seed: i32 = 2;\ncomptime const scale = (value: i32): i32 => value * seed;\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == crate::comptime::NON_CONSTANT)
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let use_start = source.rfind("seed").unwrap();
    assert_eq!(diagnostic.range, use_start..use_start + "seed".len());
    assert!(
        diagnostic
            .related
            .iter()
            .any(|related| { &source[related.range.clone()] == "let seed: i32 = 2;" })
    );
}

#[test]
fn comptime_effect_analysis_allows_local_structured_mutation_only() {
    let source = "let external: i32[] = [1];\ncomptime function local(): i32 { let values: i32[] = [1]; values[0] = 2; let record = { value: values[0] }; record.value += 1; return record.value; }\ncomptime function observable(): void { external[0] = 2; }\nconst result: i32 = comptime(local());\n";
    let analysis = analyze(source);
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("`local`"))
    );
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == crate::comptime::FORBIDDEN_EFFECT
                && diagnostic
                    .message
                    .contains("external indexed value `external`")
        })
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    assert_eq!(&source[diagnostic.range.clone()], "external[0] = 2;");
}

#[test]
fn ambiguous_behavior_method_points_to_call_and_both_origins() {
    let source = r#"
interface User { age: u32; }
interface FirstRules at User { label(user: User): string { return user.age.toString(); } }
interface SecondRules at User { label(user: User): string { return "user"; } }
const main = (): void => {
    const user: User = { age: 21 };
    console.log(user.label());
};
"#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0034")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let call = source.rfind("label").unwrap();
    assert_eq!(diagnostic.range, call..call + "label".len());
    assert_eq!(diagnostic.related.len(), 2);
    assert!(diagnostic.related.iter().any(|related| {
        &source[related.range.clone()] == "FirstRules"
            && related.message.contains("contributes this method")
    }));
    assert!(diagnostic.related.iter().any(|related| {
        &source[related.range.clone()] == "SecondRules"
            && related.message.contains("contributes this method")
    }));
}

#[test]
fn md5_security_warning_points_to_the_method_and_explains_safe_alternatives() {
    let source = r#"import { digest } from "tyrus:crypto";
const legacy: u8[] = digest.md5("data");
console.log(legacy.length);
"#;
    let import_types = [(
        "digest".to_string(),
        Type::Named("DigestNamespace".to_string()),
    )]
    .into_iter()
    .collect();
    let import_interfaces = [(
        "DigestNamespace".to_string(),
        crate::crypto::digest_namespace_fields(),
    )]
    .into_iter()
    .collect();
    let analysis = analyze_with_imports(
        source,
        &import_types,
        &import_interfaces,
        &HashMap::default(),
    );
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3278")
        .expect("expected MD5 security warning");
    let start = source.find("md5").unwrap();
    assert_eq!(diagnostic.range, start..start + 3);
    assert_eq!(diagnostic.severity, DiagnosticSeverity::Warning);
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("digest.sha256"))
    );
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("hmac.sha256"))
    );
}

#[test]
fn invalid_token_diagnostic_uses_lexer_range() {
    let source = "const value = 1_000_;\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0001")
        .expect("expected invalid token diagnostic");
    let start = source.find("1_000_").unwrap();
    assert_eq!(diagnostic.range, start..start + "1_000_".len());
}

#[test]
fn object_projection_method_diagnostics_use_key_and_argument_ranges() {
    let unknown_key_source = r#"
        interface User { name: string; }
        const run = (user: User): void => { user.pick(["missing"]); };
    "#;
    let analysis = analyze(unknown_key_source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0609")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    assert_eq!(&unknown_key_source[diagnostic.range.clone()], "\"missing\"");

    let dynamic_keys_source = r#"
        interface User { name: string; }
        const run = (user: User, keys: string[]): void => { user.omit(keys); };
    "#;
    let analysis = analyze(dynamic_keys_source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0308")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    assert_eq!(&dynamic_keys_source[diagnostic.range.clone()], "keys");
}

#[test]
fn caught_object_update_validation_has_no_unhandled_effect() {
    let source = r#"
        interface Input { @minLength(3) name: string; @email() email: string; }
        const readName = (): string => "a";
        const readEmail = (): string => "invalid";
        const main = (): void => {
            const valid: Input = { name: "Ada", email: "ada@example.com" };
            try {
                const updated: Input = { ...valid, name: readName(), email: readEmail() };
                console.log(updated.name);
            } catch (error: ValidationError) { console.log(error.issues.length); }
        };
    "#;
    let analysis = analyze(source);
    assert!(
        analysis
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "E3011"),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn missing_reply_status_diagnostic_uses_the_send_range() {
    let source = concat!(
        "enum HttpStatus { Ok = 200 }\n",
        "interface Response { ok: boolean; }\n",
        "const app: HttpRouter = __tyrus_http_createRouter();\n",
        "app.get('/users', {\n",
        "    response: { 200: Response },\n",
        "    handler: (request, reply) => { reply.header('x-demo', 'ok').send({ ok: true }); },\n",
        "});\n",
    );
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3101")
        .expect("expected a missing reply status diagnostic");
    let start = source.find(".send(").unwrap() + 1;
    assert_eq!(diagnostic.range, start..start + "send".len());
    assert_eq!(
        diagnostic.message,
        "`reply.send(...)` or `reply.stream()` must be preceded by `reply.status(HttpStatus.X)`"
    );
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains(".status(HttpStatus.X)"))
    );
}

#[test]
fn reply_use_after_writable_stream_marks_the_later_method_range() {
    let source = concat!(
        "enum HttpStatus { Ok = 200 }\n",
        "interface StreamBody {}\n",
        "const app: HttpRouter = __tyrus_http_createRouter();\n",
        "app.get('/events', {\n",
        "    response: { 200: StreamBody },\n",
        "    handler: async (request, reply): Promise<void> => {\n",
        "        const body = reply.status(HttpStatus.Ok).stream();\n",
        "        reply.header('x-late', 'no');\n",
        "        await body.close();\n",
        "    },\n",
        "});\n",
    );
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3119")
        .expect("expected a reply-after-stream diagnostic");
    let start = source.find(".header(").unwrap() + 1;
    assert_eq!(diagnostic.range, start..start + "header".len());
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("WritableStream<u8[]>"))
    );
}

#[test]
fn rejected_reply_header_names_are_diagnosed_at_the_literal() {
    let source = concat!(
        "enum HttpStatus { Ok = 200 }\n",
        "interface Response { ok: boolean; }\n",
        "const app: HttpRouter = __tyrus_http_createRouter();\n",
        "app.get('/users', {\n",
        "    response: { 200: Response },\n",
        "    handler: (request, reply) => {\n",
        "        reply.header('x demo', 'ok');\n",
        "        reply.header('set-cookie', 'a=b');\n",
        "        reply.status(HttpStatus.Ok).send({ ok: true });\n",
        "    },\n",
        "});\n",
    );
    let analysis = analyze(source);

    let invalid = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3117")
        .expect("expected an invalid header name diagnostic");
    let start = source.find("'x demo'").unwrap();
    assert_eq!(invalid.range, start..start + "'x demo'".len());
    assert_eq!(invalid.message, "`x demo` is not a valid HTTP header name");
    assert!(
        invalid
            .help
            .iter()
            .any(|help| help.contains("cannot contain a space"))
    );

    let protected = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3118")
        .expect("expected a protected header diagnostic");
    let start = source.find("'set-cookie'").unwrap();
    assert_eq!(protected.range, start..start + "'set-cookie'".len());
    assert!(
        protected
            .help
            .iter()
            .any(|help| help.contains("reply.setCookie"))
    );
}

#[test]
fn a_rejected_response_value_is_diagnosed_at_the_value() {
    let source = concat!(
        "enum HttpStatus { Ok = 200 }\n",
        "interface UserResponse { id: string; }\n",
        "const app: HttpRouter = __tyrus_http_createRouter();\n",
        "app.get('/u', {\n",
        "    response: { 200: UserResponse },\n",
        "    handler: (request, reply) => {\n",
        "        reply.status(HttpStatus.Ok).send({ wrong: 1 });\n",
        "    },\n",
        "});\n",
    );
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3105")
        .expect("expected a response type mismatch");
    let value = "{ wrong: 1 }";
    let start = source.find(value).unwrap();
    assert_eq!(diagnostic.range, start..start + value.len());
}

#[test]
fn integer_overflow_diagnostic_uses_literal_range() {
    let source = "const tooLarge = 340_282_366_920_938_463_463_374_607_431_768_211_456;\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0002")
        .expect("expected integer overflow parse diagnostic");
    let literal = "340_282_366_920_938_463_463_374_607_431_768_211_456";
    let start = source.find(literal).unwrap();
    assert_eq!(diagnostic.range, start..start + literal.len());
    assert_eq!(diagnostic.message, "integer literal does not fit in u128");
}

#[test]
fn negative_integer_overflow_diagnostic_uses_literal_range() {
    let source = "const tooSmall = -170_141_183_460_469_231_731_687_303_715_884_105_729;\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0002")
        .expect("expected negative integer overflow parse diagnostic");
    let literal = "170_141_183_460_469_231_731_687_303_715_884_105_729";
    let start = source.find(literal).unwrap();
    assert_eq!(diagnostic.range, start..start + literal.len());
    assert_eq!(diagnostic.message, "integer literal does not fit in i128");
}

#[test]
fn wide_unsigned_inference_diagnostic_uses_declaration_range() {
    let source = r#"
        const maxI128 = 170_141_183_460_469_231_731_687_303_715_884_105_728;
        console.log(maxI128);
    "#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0310")
        .expect("expected unsigned inference diagnostic");
    let declaration = source.find("maxI128").unwrap();
    assert_eq!(diagnostic.range, declaration..declaration + "maxI128".len());
}

#[test]
fn anonymous_object_inference_diagnostic_uses_declaration_range() {
    let source = r#"
        const isAdmin = { a: 1, b: "yes", c: true };
        console.log(isAdmin);
    "#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0310")
        .expect("expected anonymous object inference diagnostic");
    let declaration = source.find("isAdmin").unwrap();
    assert_eq!(diagnostic.range, declaration..declaration + "isAdmin".len());
    assert!(
        diagnostic
            .message
            .contains("requires an explicit interface annotation")
    );
}

#[test]
fn negative_unsigned_initializer_diagnostic_uses_declaration_range() {
    let source = r#"
        console.log("test init");
        const test: u128 = -1;
        console.log(test);
    "#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0308")
        .expect("expected negative unsigned initializer diagnostic");
    let declaration = source.find("test: u128").unwrap();
    assert_eq!(diagnostic.range, declaration..declaration + "test".len());
    assert!(
        diagnostic
            .message
            .contains("use `i32`, `i64`, `i128`, or `isize`")
    );
}

#[test]
fn huge_float_diagnostic_uses_literal_range() {
    let source = "const float64 = 10000000000000000000000000000000000000.000000000;\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0002")
        .expect("expected float overflow parse diagnostic");
    let literal = "10000000000000000000000000000000000000.000000000";
    let start = source.find(literal).unwrap();
    assert_eq!(diagnostic.range, start..start + literal.len());
    assert_eq!(diagnostic.message, "float literal does not fit in f64");
}

#[test]
fn unsupported_regex_flag_diagnostic_uses_flag_range() {
    let source = "const slugPattern: Regex = /^[a-z0-9-]+$/g;\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0309")
        .expect("expected unsupported regex flag diagnostic");
    let start = source.find("/g").unwrap() + 1;
    assert_eq!(diagnostic.range, start..start + "g".len());
    assert_eq!(&source[diagnostic.range.clone()], "g");
}

#[test]
fn reports_unused_import_and_unknown_function() {
    let analysis = analyze(
        r#"import { readFileSync } from "tyrus:fs";
           missing();"#,
    );
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message == "unused import `readFileSync`")
    );
    assert!(analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic
            .message
            .contains("undeclared identifier `missing`")
    }));
}

#[test]
fn test_registrar_modifier_calls_count_as_import_uses() {
    let source = concat!(
        "import { it, test } from \"tyrus:test\";\n\n",
        "test.skip(\"later\", () => {});\n",
        "test.todo(\"unicode initials\");\n",
        "it.each(\"clamps\", [-5, 0, 5], (value: i32) => {});\n",
    );
    let registrar_type = Type::Function {
        params: vec![Type::Str, Type::Unknown],
        ret: Box::new(Type::Void),
    };
    let import_types = [
        ("it".to_string(), registrar_type.clone()),
        ("test".to_string(), registrar_type),
    ]
    .into_iter()
    .collect();
    let import_targets = [
        ("it".to_string(), "__tyrus_test_it".to_string()),
        ("test".to_string(), "__tyrus_test_test".to_string()),
    ]
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
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "W0001"),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn assert_throws_calls_count_as_import_uses() {
    let source = concat!(
        "import { alwaysRejects } from \"./bank\";\n\n",
        "assertThrows(alwaysRejects);\n",
        "assertThrows(alwaysRejects, \"insufficient funds\");\n",
    );
    let import_types = [(
        "alwaysRejects".to_string(),
        Type::Function {
            params: Vec::new(),
            ret: Box::new(Type::Void),
        },
    )]
    .into_iter()
    .collect();
    let import_throws = [("alwaysRejects".to_string(), vec![Type::Str])]
        .into_iter()
        .collect();
    let analysis = analyze_with_context(
        source,
        AnalyzeContext {
            import_types: Some(&import_types),
            import_throws: Some(&import_throws),
            ..AnalyzeContext::default()
        },
    );
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "W0001"),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn undeclared_initializer_identifier_points_to_its_initializer_use() {
    let source = "const u8MIN = u8MIN;\nconsole.log(u8MIN);\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0425")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let initializer = source.find("u8MIN;").unwrap();
    assert_eq!(diagnostic.range, initializer..initializer + "u8MIN".len());
}

#[test]
fn validator_diagnostic_anchors_to_the_offending_field() {
    let source = "interface A {\n    @email()\n    a: string;\n}\n\n\
                  interface B {\n    @email()\n    code: i32;\n}\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3161")
        .expect("expected an invalid-target diagnostic");
    let second_email = source.rfind("@email").unwrap();
    assert_eq!(
        diagnostic.range.start, second_email,
        "diagnostic should anchor to the second `@email`, got {:?}",
        diagnostic.range
    );
}

#[test]
fn exposes_types_for_hover() {
    let source = "const greet = (name: &string): void => { console.log(name); };";
    let analysis = analyze(source);
    let offset = source.rfind("name").unwrap();
    let symbol = analysis.symbol_at(offset).unwrap();
    assert_eq!(symbol.name, "name");
    assert_eq!(symbol.ty, "&string");
}

#[test]
fn tuple_literal_hover_does_not_leak_into_the_annotation() {
    let source = "const pair: [i32, string] = [1, \"one\"];\n";
    let analysis = analyze(source);
    let annotation = source.find("i32").unwrap();
    assert!(
        analysis.type_at(annotation).is_none(),
        "annotation should not be covered by the value's expression node"
    );
    let literal = source.find("[1,").unwrap();
    assert_eq!(
        analysis
            .type_at(literal)
            .map(|expression| expression.ty.clone()),
        Some("[i32, string]".to_string())
    );
}

#[test]
fn tuple_destructuring_bindings_get_positional_types() {
    let source = "const pair: [i32, string] = [1, \"one\"];\nconst [count, label] = pair;\n";
    let analysis = analyze(source);
    let count = analysis.symbol_named("count").unwrap();
    assert_eq!(count.kind, SymbolKind::Variable);
    assert_eq!(count.ty, "i32");
    let label = analysis.symbol_named("label").unwrap();
    assert_eq!(label.kind, SymbolKind::Variable);
    assert_eq!(label.ty, "string");
}

#[test]
fn symbol_name_index_preserves_first_match_and_omits_unknown_names() {
    let analysis = analyze(
        "export const alpha: i32 = 1;\nconst beta: string = \"value\";\nexport const gamma: bool = true;\n",
    );
    for symbol in &analysis.symbols {
        let expected = analysis
            .symbols
            .iter()
            .find(|candidate| candidate.name == symbol.name)
            .map(|candidate| candidate.declaration.clone());
        assert_eq!(
            analysis
                .symbol_named(&symbol.name)
                .map(|candidate| candidate.declaration.clone()),
            expected
        );
        let expected_export = analysis
            .symbols
            .iter()
            .find(|candidate| candidate.exported && candidate.name == symbol.name)
            .map(|candidate| candidate.declaration.clone());
        assert_eq!(
            analysis
                .exported_symbol_named(&symbol.name)
                .map(|candidate| candidate.declaration.clone()),
            expected_export
        );
    }
    assert!(analysis.symbol_named("missingSymbol").is_none());
    assert!(analysis.exported_symbol_named("missingSymbol").is_none());
}

#[test]
fn diagnostic_range_index_matches_linear_primary_and_related_lookup() {
    let source = "const value: i32 = \"text\";\nconst other: bool = 1;\n";
    let analysis = analyze(source);
    assert!(!analysis.diagnostics.is_empty());
    for offset in 0..=source.len() {
        let expected = analysis.diagnostics.iter().find(|diagnostic| {
            diagnostic.range.contains(&offset)
                || diagnostic
                    .related
                    .iter()
                    .any(|related| related.range.contains(&offset))
        });
        assert_eq!(
            analysis
                .diagnostic_at(offset)
                .map(|diagnostic| diagnostic.range.clone()),
            expected.map(|diagnostic| diagnostic.range.clone()),
            "diagnostic lookup differs at byte offset {offset}"
        );
    }
}

#[test]
fn call_site_interval_index_matches_linear_narrowest_call_lookup() {
    let source = "function render(value: string): string { return value.trim().toUpperCase(); }\n";
    let analysis = analyze(source);
    assert!(analysis.call_sites.len() > 1);
    for offset in 0..=source.len() {
        let expected = analysis
            .call_sites
            .iter()
            .filter(|call| call.range.contains(&offset))
            .min_by_key(|call| call.range.end - call.range.start);
        assert_eq!(
            analysis.call_site_at(offset).map(|call| call.range.clone()),
            expected.map(|call| call.range.clone()),
            "call-site lookup differs at byte offset {offset}"
        );
    }
}

#[test]
fn token_interval_lookup_matches_linear_inclusive_cursor_lookup() {
    let source = "const value: Result<i32, string> = load(\"input\");\n";
    let analysis = analyze(source);
    for offset in 0..=source.len() {
        let expected = analysis
            .tokens
            .iter()
            .find(|token| token.range.start <= offset && offset <= token.range.end);
        assert_eq!(
            analysis
                .token_at_inclusive(offset)
                .map(|token| token.range.clone()),
            expected.map(|token| token.range.clone()),
            "token lookup differs at byte offset {offset}"
        );
    }
}

#[test]
fn imported_symbol_name_index_matches_linear_import_filtering() {
    let source =
        "import { alpha as localAlpha, beta } from \"./module\";\nconst value = localAlpha;\n";
    let analysis = analyze(source);
    for imported_name in ["alpha", "beta", "missing"] {
        let expected = analysis
            .symbols
            .iter()
            .enumerate()
            .filter(|(_, symbol)| {
                symbol.kind == SymbolKind::Import
                    && symbol.imported_name.as_deref() == Some(imported_name)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        assert_eq!(
            analysis.imported_symbol_indexes_named(imported_name),
            Some(expected.as_slice())
        );
    }
}

#[test]
fn export_name_indexes_match_linear_named_and_wildcard_lookup() {
    let source =
        "export const alpha: i32 = 1;\nexport { alpha as beta };\nexport * from \"./other\";\n";
    let analysis = analyze(source);
    let program = analysis.program.as_ref().unwrap();
    for name in ["alpha", "beta", "missing"] {
        let expected = program
            .exports
            .iter()
            .enumerate()
            .filter(|(_, export)| export.exported_name() == Some(name))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        assert_eq!(analysis.named_export_indexes(name), expected.as_slice());
    }
    let expected_wildcards = program
        .exports
        .iter()
        .enumerate()
        .filter(|(_, export)| matches!(export, crate::ast::ExportDecl::All { .. }))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    assert_eq!(analysis.wildcard_export_indexes(), expected_wildcards);
}

#[test]
fn undeclared_wildcard_read_points_at_the_read_not_the_discard_site() {
    let source =
        "const pair: [i32, string] = [1, \"one\"];\nconst [count, _] = pair;\nconsole.log(_);\n";
    let analysis = analyze(source);
    let read = source.rfind('_').unwrap();
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0425")
        .expect("expected an undeclared-identifier diagnostic");
    assert_eq!(diagnostic.range, read..read + 1);
}

#[test]
fn unresolved_read_after_a_local_borrow_does_not_report_the_borrow_as_invalid() {
    let source = "function test() {\n    const original: string = \"tyrus\";\n    const borrowed: &string = &original;\n    print(borrowed);\n}\n\ntest();\nprint(original);\n";
    let analysis = analyze(source);
    let unresolved_read = source.rfind("original").unwrap();
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0425")
        .expect("expected the out-of-scope read to be diagnosed");

    assert_eq!(
        diagnostic.range,
        unresolved_read..unresolved_read + "original".len()
    );
    assert!(
        analysis
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "E0500"),
        "a valid local borrow must not be blamed for a later unresolved read: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn expression_hover_stays_aligned_after_console_log() {
    let source = "const nested: [[i32, i32], boolean] = [[3, 4], true];\nconsole.log(nested);\nconst pair: [i32, string] = [1, \"two\"];\nconsole.log(pair);\n";
    let analysis = analyze(source);
    let inner = source.find("[3, 4]").unwrap();
    assert_eq!(
        analysis.type_at(inner).map(|e| e.ty.clone()),
        Some("[i32, i32]".to_string())
    );
    let truthy = source.find("true").unwrap();
    assert_eq!(
        analysis.type_at(truthy).map(|e| e.ty.clone()),
        Some("boolean".to_string())
    );
    let two = source.find("\"two\"").unwrap();
    assert_eq!(
        analysis.type_at(two).map(|e| e.ty.clone()),
        Some("string".to_string())
    );
}

#[test]
fn shorthand_object_field_hovers_correctly_and_does_not_corrupt_later_spans() {
    let source = "const name: string = \"Ada\";\nconst age: i32 = 30;\nconst user = { name, age };\nconst pair: [i32, string] = [1, \"two\"];\n";
    let analysis = analyze(source);
    let shorthand_name = source.rfind("name").unwrap();
    assert_eq!(
        analysis.type_at(shorthand_name).map(|e| e.ty.clone()),
        Some("string".to_string())
    );
    let two = source.find("\"two\"").unwrap();
    assert_eq!(
        analysis.type_at(two).map(|e| e.ty.clone()),
        Some("string".to_string())
    );
}

#[test]
fn nested_tuple_literal_is_typed_positionally() {
    let source = "const nested: [[i32, i32], boolean] = [[3, 4], true];\n";
    let analysis = analyze(source);
    assert!(
        analysis.diagnostics.iter().all(|d| d.code != "E0308"),
        "unexpected diagnostics: {:?}",
        analysis.diagnostics
    );
    let inner = source.find("[3,").unwrap();
    assert_eq!(
        analysis
            .type_at(inner)
            .map(|expression| expression.ty.clone()),
        Some("[i32, i32]".to_string())
    );
}

#[test]
fn exposes_contextually_inferred_callback_parameter_types() {
    let source = "const main = (): void => { const values: i32[] = [1]; const mapped: i32[] = values.map(value => value * 2); };";
    let analysis = analyze(source);
    let parameter = analysis
        .symbols
        .iter()
        .find(|symbol| symbol.kind == SymbolKind::Parameter && symbol.name == "value")
        .unwrap_or_else(|| panic!("symbols: {:#?}", analysis.symbols));
    assert_eq!(parameter.ty, "i32");
    assert_eq!(parameter.semantic_type, Some(Type::I32));
}

#[test]
fn attaches_documentation_to_declarations() {
    let source = r#"/**
 * Adds two integers.
 *
 * @param left First operand
 * @param right Second operand
 * @returns Sum of the operands
 */
const add = (left: i32, right: i32): i32 => left + right;"#;
    let analysis = analyze(source);
    let symbol = analysis.symbol_named("add").unwrap();
    let doc = symbol
        .documentation
        .as_ref()
        .expect("expected documentation");
    assert_eq!(doc.summary, "Adds two integers.");
    assert_eq!(
        doc.params.get("left").map(String::as_str),
        Some("First operand")
    );
    assert_eq!(
        doc.params.get("right").map(String::as_str),
        Some("Second operand")
    );
    assert_eq!(doc.returns.as_deref(), Some("Sum of the operands"));
}

#[test]
fn exposes_enum_and_member_symbols() {
    let source = r#"enum Role: string {
    Admin = "admin",
    User = "user",
}"#;
    let analysis = analyze(source);
    let role = analysis.symbol_named("Role").unwrap();
    assert_eq!(role.kind, SymbolKind::Enum);
    assert_eq!(role.ty, "enum Role: string");
    let admin = analysis.symbol_named("Admin").unwrap();
    assert_eq!(admin.kind, SymbolKind::EnumMember);

    let nominal = analyze("enum Status { Success, Error }");
    assert_eq!(nominal.symbol_named("Status").unwrap().ty, "enum Status");
}

#[test]
fn exposes_interface_documentation_symbols() {
    let source = r#"/**
 * User profile.
 */
interface User {
    name: string;
}"#;
    let analysis = analyze(source);
    let symbol = analysis.symbol_named("User").unwrap();
    assert_eq!(symbol.kind, SymbolKind::Interface);
    assert_eq!(
        symbol
            .documentation
            .as_ref()
            .map(|doc| doc.summary.as_str()),
        Some("User profile.")
    );
}

#[test]
fn preserves_function_parameter_names_in_symbol_details() {
    let analysis = analyze("export const sum = (left: i32, right: i32): i32 => left + right;");
    let symbol = analysis.symbol_named("sum").unwrap();

    assert_eq!(symbol.ty, "(left: i32, right: i32) => i32");
    assert!(matches!(symbol.semantic_type, Some(Type::Function { .. })));
}

#[test]
fn infers_exported_block_arrow_function_symbol_type() {
    let source = r#"
        export const double = (n: i64): i64 => {
            return n * 2;
        };
    "#;
    let analysis = analyze(source);
    let symbol = analysis.symbol_named("double").unwrap();

    assert_eq!(symbol.ty, "(n: i64) => i64");
    assert!(matches!(symbol.semantic_type, Some(Type::Function { .. })));
}

#[test]
fn infers_async_function_symbol_type_as_promise() {
    let source = "async function load(value: i32) { return value + 1; }\n";
    let analysis = analyze(source);
    let symbol = analysis.symbol_named("load").unwrap();

    assert_eq!(symbol.ty, "(value: i32) => Promise<i32>");
    assert!(matches!(symbol.semantic_type, Some(Type::Function { .. })));
}

#[test]
fn infers_async_arrow_symbol_type_as_promise() {
    let source = "const load = async (value: i32) => value + 1;\n";
    let analysis = analyze(source);
    let symbol = analysis.symbol_named("load").unwrap();

    assert_eq!(symbol.ty, "(value: i32) => Promise<i32>");
    assert!(matches!(symbol.semantic_type, Some(Type::Function { .. })));
}

#[test]
fn exposes_function_call_argument_ranges() {
    let source = "const sum = (left: i32, right: i32): i32 => left + right; sum(20, 22);";
    let analysis = analyze(source);
    let call = analysis.call_sites.first().unwrap();

    assert_eq!(call.callee, "sum");
    assert_eq!(&source[call.arguments[0].clone()], "20");
    assert_eq!(&source[call.arguments[1].clone()], "22");
}

#[test]
fn assigns_source_ranges_to_all_semantic_nodes() {
    let source = "const add = (left: i32, right: i32): i32 => left + right;";
    let analysis = analyze(source);
    assert!(
        analysis
            .node_spans
            .iter()
            .any(|node| node.kind == AstNodeKind::Program && node.range == (0..source.len()))
    );
    assert!(
        analysis
            .node_spans
            .iter()
            .filter(|node| node.kind == AstNodeKind::Parameter)
            .count()
            >= 2
    );
    assert!(
        analysis
            .node_spans
            .iter()
            .all(|node| node.range.start <= node.range.end && node.range.end <= source.len())
    );
}

#[test]
fn positions_parser_diagnostics_at_the_offending_token() {
    let source = "const value: i32 = ;";
    let analysis = analyze(source);
    let diagnostic = analysis.diagnostics.first().unwrap();
    assert_eq!(diagnostic.code, "E0002");
    assert!(diagnostic.range.start > 0);
}

#[test]
fn positions_type_errors_at_the_declaration() {
    let source = "const age: i32 = \"twenty\";";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0308")
        .unwrap();
    assert_eq!(&source[diagnostic.range.clone()], "age");
}

#[test]
fn numeric_mismatch_at_assignment_relates_the_value_expression() {
    let source = "const main = (): void => {\n    let sum: i32 = 0;\n    const values: i32[] = [0, 1, 2, 3];\n    for (let i = 0; i < values.length; i += 1) {\n        sum += values[i];\n        sum += i;\n    }\n    console.log(sum);\n};\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "E0308" && diagnostic.message.contains("type mismatch for `sum`")
        })
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    assert_eq!(diagnostic.related.len(), 1);
    assert_eq!(&source[diagnostic.related[0].range.clone()], "i");
    assert!(
        diagnostic.related[0]
            .message
            .contains("value of type `usize`")
    );
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("write the conversion explicitly with `as i32`"))
    );
}

#[test]
fn plain_assignment_mismatch_relates_the_value_expression() {
    let source = "const main = (): void => {\n    let width: u32 = 10;\n    let depth: i64 = 1;\n    width = depth;\n    console.log(width, depth);\n};\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "E0308" && diagnostic.message.contains("type mismatch for `width`")
        })
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    assert_eq!(diagnostic.related.len(), 1);
    assert_eq!(&source[diagnostic.related[0].range.clone()], "depth");
    assert!(
        diagnostic.related[0]
            .message
            .contains("value of type `i64`")
    );
}

#[test]
fn initializer_type_mismatch_prefers_declaration_over_later_reference() {
    let source = r#"
        const main = (): void => {
            const message: string = "hello";
            const len: i64 = message.length;
            console.log(len);
        };
    "#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0308")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    assert_eq!(&source[diagnostic.range.clone()], "len");
    assert!(diagnostic.range.start < source.find("console.log(len)").unwrap());
}

#[test]
fn callback_parameter_type_mismatch_points_at_parameter_annotation() {
    let source = "const main = (): void => {\n\
                  const values: i32[] = [1, 2, 3];\n\
                  const filtered: i32[] = values.filter((value: i32, index: i64) => {\n\
                      return index >= 1;\n\
                  });\n\
                  console.log(filtered);\n\
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

    assert_eq!(&source[diagnostic.range.clone()], "i64");
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help == "change the callback parameter type to `usize`")
    );
}

#[test]
fn includes_previous_declaration_in_duplicate_declaration_diagnostic() {
    let source = r#"
        const words: string = "tyrus,hello,world";
        const words: string = "test";
    "#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0428")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    assert!(diagnostic.message.contains("declared more than once"));
    assert_eq!(&source[diagnostic.range.clone()], "words");
    let first = source.find("const words").unwrap();
    let second = source.rfind("const words").unwrap();
    assert_eq!(diagnostic.range.start, second + 6);
    assert_eq!(diagnostic.related.len(), 1);
    assert_eq!(&source[diagnostic.related[0].range.clone()], "words");
    assert_eq!(diagnostic.related[0].range.start, first + 6);
    assert!(
        diagnostic.related[0]
            .message
            .contains("previous declaration")
    );
}

#[test]
fn positions_reference_type_mismatches_at_the_owned_annotation() {
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
    assert_eq!(&source[diagnostic.range.clone()], "string");
    assert!(diagnostic.message.contains("mismatched types"));
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("change the annotation to `&string`"))
    );
    let related = diagnostic.related.first().unwrap();
    assert_eq!(&source[related.range.clone()], "&copyNewName");
    assert!(related.message.contains("creates a borrow"));
}

#[test]
fn positions_missing_object_property_errors_at_the_declaration() {
    let source = r#"
        interface User {
            name: string;
            age: i32;
        }

        const user: User = { name: "Ada" };
        const renamed: User = {
            ...user,
            name: "Grace",
        };
    "#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E2741")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    assert_eq!(&source[diagnostic.range.clone()], "user");
    assert!(diagnostic.range.start < source.find("renamed").unwrap());
}

#[test]
fn positions_mutating_array_method_errors_at_the_receiver() {
    let source = r#"const words: string[] = ["tyrus"]; words.push("test");"#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0384")
        .unwrap();
    assert_eq!(&source[diagnostic.range.clone()], "words");
    assert!(diagnostic.range.start > source.find(';').unwrap());
}

#[test]
fn positions_moved_value_errors_at_the_invalid_use() {
    let source = r#"
        const main = (): void => {
            const newName: string = "TitenQ";
            const copyNewName: string = newName;
            console.log(copyNewName);
            console.log(newName);
        };
    "#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.message.contains("use of moved value: `newName`"))
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let last_use = source.rfind("newName").unwrap();
    let move_site = source.find("= newName").unwrap() + 2;
    assert_eq!(diagnostic.code, "E0382");
    assert_eq!(&source[diagnostic.range.clone()], "newName");
    assert_eq!(diagnostic.range.start, last_use);
    assert_eq!(diagnostic.related.len(), 1);
    assert_eq!(&source[diagnostic.related[0].range.clone()], "newName");
    assert_eq!(diagnostic.related[0].range.start, move_site);
    assert!(diagnostic.related[0].message.contains("value moved here"));
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("borrow `newName`"))
    );
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("const copyNewName: &string = &newName;"))
    );
    let rendered = render_diagnostic(std::path::Path::new("examples.tyr"), source, diagnostic);
    let rendered_lines = rendered.lines().collect::<Vec<_>>();
    let code_line = rendered_lines
        .iter()
        .find(|line| line.contains("console.log(newName);"))
        .unwrap();
    let underline_line = rendered_lines
        .iter()
        .skip_while(|line| *line != code_line)
        .nth(1)
        .unwrap();
    assert_eq!(code_line.find("newName"), underline_line.find('^'));
    assert!(rendered.contains("at examples.tyr:"));
    assert!(!rendered.contains(" | "));
}

#[test]
fn positions_dropped_value_errors_at_the_invalid_use_and_relates_drop_site() {
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
    let invalid_use = source.rfind("token").unwrap();
    let drop_site = source.find("drop(token)").unwrap() + "drop(".len();
    assert_eq!(diagnostic.code, "E0382");
    assert_eq!(diagnostic.range, invalid_use..invalid_use + "token".len());
    assert_eq!(diagnostic.related.len(), 1);
    assert_eq!(
        diagnostic.related[0].range,
        drop_site..drop_site + "token".len()
    );
    assert!(diagnostic.related[0].message.contains("value dropped here"));
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("drop(token)"))
    );
}

#[test]
fn return_path_does_not_make_a_value_dropped_on_the_continuing_path() {
    let source = r#"
        import { drop } from "tyrus:memory";
        const main = (discard: boolean): void => {
            let token: string = "secret";
            if (discard) {
                drop(token);
                return;
            }
            console.log(token);
        };
    "#;
    let analysis = analyze(source);
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("use of dropped value: `token`")),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn async_borrow_across_await_diagnostic_uses_borrow_binding_range() {
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
    let declaration = source.find("borrowed").unwrap();
    let await_start = source.find("await").unwrap();
    assert_eq!(&source[diagnostic.range.clone()], "borrowed");
    assert_eq!(diagnostic.range.start, declaration);
    assert!(diagnostic.message.contains("crosses `await`"));
    assert!(diagnostic.related.iter().any(|related| {
        related.range == (await_start..await_start + "await".len())
            && related.message.contains("suspension point")
    }));
}

#[test]
fn positions_unreachable_match_arm_warnings_at_the_pattern() {
    let source = r#"
        type Status = "ready" | "failed";
        const status: Status = "ready";

        match (status) {
            ("ready") => {
                console.log("ready");
            }
            ("failed") => {
                console.log("failed");
            }
            _ => {
                console.log("any");
            }
        }
    "#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "W0002")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    assert_eq!(diagnostic.message, "unreachable match arm");
    assert_eq!(&source[diagnostic.range.clone()], "_");
}

#[test]
fn warns_for_complex_http_reply_flow() {
    let source = r#"
        enum HttpStatus { Ok = 200 }
        const app: HttpRouter = __tyrus_http_createRouter();
        app.get("/health", {
            response: { 200: Response },
            handler: (request, reply) => {
                while (false) {
                    console.log("skip");
                }
                reply.status(HttpStatus.Ok).send({ ok: true });
            }
        });
    "#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3109")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    assert_eq!(diagnostic.severity, DiagnosticSeverity::Warning);
    assert!(diagnostic.message.contains("complex control flow"));
}

#[test]
fn http_reply_flow_diagnostic_includes_help() {
    let source = r#"
        enum HttpStatus { Ok = 200 }
        interface Response {
            ok: boolean;
        }
        const app: HttpRouter = __tyrus_http_createRouter();
        app.get("/health", {
            response: { 200: Response },
            handler: (request, reply) => {
                const found: boolean = true;
                if (found) {
                    reply.status(HttpStatus.Ok).send({ ok: true });
                }
            }
        });
    "#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3100")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("reply.status(HttpStatus.X).send(value)"))
    );
}

#[test]
fn resolves_shadowed_names_to_their_lexical_scope() {
    let source = r#"
        const main = (): void => {
            const value: i32 = 1;
            if (true) {
                const value: i32 = 2;
                console.log(value);
            }
            console.log(value);
        };
    "#;
    let analysis = analyze(source);
    let declarations = analysis
        .symbols
        .iter()
        .filter(|symbol| symbol.name == "value")
        .collect::<Vec<_>>();
    let references = analysis
        .references
        .iter()
        .filter(|reference| reference.name == "value")
        .collect::<Vec<_>>();
    assert_eq!(declarations.len(), 2);
    assert_eq!(references.len(), 2);
    assert_eq!(
        references[0].definition,
        Some(declarations[1].declaration.clone())
    );
    assert_eq!(
        references[1].definition,
        Some(declarations[0].declaration.clone())
    );
}

#[test]
fn checks_calls_using_resolved_import_types() {
    let source = r#"
        import { add } from "./math";
        add("wrong", 2);
    "#;
    let imports = [(
        "add".to_string(),
        Type::Function {
            params: vec![Type::I32, Type::I32],
            ret: Box::new(Type::I32),
        },
    )]
    .into_iter()
    .collect();
    let analysis = analyze_with_imports(source, &imports, &HashMap::default(), &HashMap::default());
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E0308")
    );
}

#[test]
fn exposes_inferred_expression_types_by_source_range() {
    let source = "const result: i32 = 20 + 22;";
    let analysis = analyze(source);
    let operator = source.find('+').unwrap();
    assert_eq!(analysis.type_at(operator).unwrap().ty, "i32");
    assert_eq!(
        analysis.expression_types.len(),
        analysis
            .node_spans
            .iter()
            .filter(|node| node.kind == AstNodeKind::Expression)
            .count()
    );
}

#[test]
fn keeps_template_interpolation_types_aligned_with_spans() {
    let source = "const message: string = `sum: ${1 + 2}`;";
    let analysis = analyze(source);
    assert_eq!(
        analysis.expression_types.len(),
        analysis
            .node_spans
            .iter()
            .filter(|node| node.kind == AstNodeKind::Expression)
            .count()
    );
    assert_eq!(
        analysis.type_at(source.find('`').unwrap()).unwrap().ty,
        "string"
    );
}

#[test]
fn export_function_keyword_is_exported() {
    let source = r#"
export function greet(name: string): string {
    return "hello " + name;
}
export type Score = i32;
export let mutableCount: i32 = 0;
export const PI: f64 = 3.14159;
export interface Point {
    x: f64;
    y: f64;
}
"#;
    let analysis = analyze(source);
    let greet = analysis
        .symbols
        .iter()
        .find(|s| s.name == "greet")
        .unwrap_or_else(|| {
            panic!(
                "greet not found; symbols: {:?}",
                analysis.symbols.iter().map(|s| &s.name).collect::<Vec<_>>()
            )
        });
    assert_eq!(greet.kind, SymbolKind::Function);
    assert!(greet.exported, "greet must be exported");

    let point = analysis
        .symbols
        .iter()
        .find(|s| s.name == "Point")
        .unwrap_or_else(|| {
            panic!(
                "Point not found; symbols: {:?}",
                analysis.symbols.iter().map(|s| &s.name).collect::<Vec<_>>()
            )
        });
    assert_eq!(point.kind, SymbolKind::Interface);
    assert!(point.exported, "Point must be exported");
    assert!(
        matches!(&point.semantic_type, Some(Type::Object(fields)) if fields.len() == 2),
        "Point must have Object semantic_type with 2 fields, got: {:?}",
        point.semantic_type
    );
}

#[test]
fn object_literal_keys_are_not_parameters() {
    let source = r#"
const p: Point = { x: 1.0, y: 2.0 };
"#;
    let analysis = analyze(source);
    for s in &analysis.symbols {
        if s.name == "x" || s.name == "y" || s.name == "p" {
            assert_ne!(
                s.kind,
                SymbolKind::Parameter,
                "symbol '{}' should not be Parameter (kind={:?})",
                s.name,
                s.kind
            );
        }
    }
    let param_refs = analysis.parameter_refs(source);
    let p_pos = source.find("= p").map(|i| i + 2);
    if let Some(pos) = p_pos {
        assert!(
            !param_refs.iter().any(|r| r.contains(&pos)),
            "p should not appear as parameter reference in semantic tokens"
        );
    }
}

#[test]
fn interface_import_resolves_correctly() {
    let interface_fields = vec![
        Field {
            name: "x".to_string(),
            ty: Type::F64,
            optional: false,
            readonly: false,
            decorators: Vec::new(),
            span: Span::DUMMY,
        },
        Field {
            name: "y".to_string(),
            ty: Type::F64,
            optional: false,
            readonly: false,
            decorators: Vec::new(),
            span: Span::DUMMY,
        },
    ];
    let source = r#"
import { Point } from "./geo";
const p: Point = { x: 1.0, y: 2.0 };
console.log(p);
"#;
    let analysis = analyze_with_imports(
        source,
        &[("Point".to_string(), Type::Named("Point".to_string()))]
            .into_iter()
            .collect(),
        &[("Point".to_string(), interface_fields)]
            .into_iter()
            .collect(),
        &HashMap::default(),
    );
    let has_unused_point = analysis
        .diagnostics
        .iter()
        .any(|d| d.code == "W0001" && d.message.contains("Point"));
    assert!(
        !has_unused_point,
        "Point should not be reported as unused import"
    );
    let has_type_mismatch = analysis
        .diagnostics
        .iter()
        .any(|d| d.code == "E0308" && d.message.contains("Point"));
    assert!(
        !has_type_mismatch,
        "Point = {{x:1.0, y:2.0}} should not produce E0308"
    );
}

#[test]
fn account_interface_with_optional_and_readonly_fields() {
    let fields = vec![
        Field {
            name: "id".to_string(),
            ty: Type::U64,
            readonly: true,
            optional: false,
            decorators: Vec::new(),
            span: Span::DUMMY,
        },
        Field {
            name: "owner".to_string(),
            ty: Type::Str,
            readonly: false,
            optional: false,
            decorators: Vec::new(),
            span: Span::DUMMY,
        },
        Field {
            name: "nickname".to_string(),
            ty: Type::Str,
            readonly: false,
            optional: true,
            decorators: Vec::new(),
            span: Span::DUMMY,
        },
        Field {
            name: "active".to_string(),
            ty: Type::Bool,
            readonly: false,
            optional: false,
            decorators: Vec::new(),
            span: Span::DUMMY,
        },
    ];
    let source = r#"
import { Account } from "./interfaces/examples";
const account: Account = { id: 1, owner: "TitenQ", nickname: "titenq", active: true };
console.log(account);
"#;
    let analysis = analyze_with_imports(
        source,
        &[("Account".to_string(), Type::Named("Account".to_string()))]
            .into_iter()
            .collect(),
        &[("Account".to_string(), fields)].into_iter().collect(),
        &HashMap::default(),
    );
    let errors: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|d| d.code == "E0308" || d.code == "E0277")
        .collect();
    assert!(
        errors.is_empty(),
        "Account object literal should not produce errors: {errors:?}"
    );
}

#[test]
fn validator_diagnostics_point_at_decorator() {
    let source = "interface UserInput { @min(3) name: string; }";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3161")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    assert_eq!(&source[diagnostic.range.clone()], "@min");
}

#[test]
fn async_closure_capture_points_at_the_captured_reference() {
    let source = "function setup(): void {\nconst values: Set<i32> = Set<i32>();\nconst calculate = async (): Promise<i32> => values.size;\nconsole.log(values.size);\n}\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3085")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let closure_start = source.find("async").unwrap();
    let captured_start = source[closure_start..].find("values").unwrap() + closure_start;
    assert_eq!(
        diagnostic.range,
        captured_start..captured_start + "values".len()
    );
    assert_eq!(diagnostic.related.len(), 1);
    assert_eq!(&source[diagnostic.related[0].range.clone()], "values");
    assert!(diagnostic.related[0].message.contains("declared here"));
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("explicit parameter"))
    );
}

#[test]
fn compound_string_assignment_across_await_is_lowerable() {
    let source = "async function load(): Promise<void> {}\nasync function main(): Promise<void> {\nlet value: string = \"before\";\nawait load();\nvalue += \"after\";\nconsole.log(value);\n}\n";
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
fn dynamically_replaced_string_array_async_local_is_lowerable() {
    let source = "async function load(): Promise<void> {}\nasync function main(): Promise<void> {\nlet values: string[] = [\"dynamic\"];\nawait load();\nvalues = values.concat([\"changed\"]);\nconsole.log(values[0]);\n}\n";
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
fn dynamic_object_async_local_uses_owned_frame_storage() {
    let source = "interface Snapshot { label: string; }\nasync function load(): Promise<void> {}\nasync function main(): Promise<void> {\nconst snapshot: Snapshot = { label: \"ready\".toUpperCase() };\nawait load();\nconsole.log(snapshot.label);\n}\n";
    let analysis = analyze(source);
    assert!(
        analysis.diagnostics.is_empty(),
        "{:#?}",
        analysis.diagnostics
    );
}

#[test]
fn repeated_validator_warning_points_at_repeated_decorator() {
    let source = "interface UserInput { @email() @email(\"invalid e-mail\") email: string; }";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3164")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let expected = source.rfind("@email").unwrap();
    assert_eq!(diagnostic.range, expected..expected + "@email".len());
}

#[test]
fn validator_failed_diagnostic_points_at_object_literal_field() {
    let source = r#"
        interface ProductFilter {
            @min(5)
            pageSize: i32;
        }

        const filter: ProductFilter = {
            pageSize: 3,
        };
    "#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3165")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let expected = source.find("pageSize: 3").unwrap() + "pageSize: ".len();
    assert_eq!(diagnostic.range, expected..expected + "3".len());
    assert!(diagnostic.message.contains("@min(5)"));
}

#[test]
fn validator_failed_diagnostic_skips_a_valid_object_literal() {
    let source = r#"
        interface ProductFilter {
            @min(5)
            pageSize: i32;
        }

        const bad: ProductFilter = { pageSize: 3 };
        const good: ProductFilter = { pageSize: 9 };
    "#;
    let analysis = analyze(source);
    let failures = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "E3165")
        .collect::<Vec<_>>();
    assert_eq!(failures.len(), 1, "diagnostics: {failures:#?}");
    let expected = source.find("pageSize: 3").unwrap() + "pageSize: ".len();
    assert_eq!(failures[0].range, expected..expected + "3".len());
}

#[test]
fn validator_failed_diagnostic_skips_a_valid_call_argument() {
    let source = r#"
        const accept = (@min(5) size: i32): void => {
            console.log(size);
        };

        const main = (): void => {
            accept(3);
            accept(9);
            accept(1);
        };
    "#;
    let analysis = analyze(source);
    let failures = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "E3165")
        .collect::<Vec<_>>();
    assert_eq!(failures.len(), 2, "diagnostics: {failures:#?}");
    let first = source.find("accept(3)").unwrap() + "accept(".len();
    let second = source.find("accept(1)").unwrap() + "accept(".len();
    assert_eq!(failures[0].range, first..first + "3".len());
    assert_eq!(failures[1].range, second..second + "1".len());
}

#[test]
fn validator_failed_diagnostic_points_at_literal_call_argument() {
    let source = r#"
        const submit = (@email() email: string): void => {};
        submit("ada.example.com");
    "#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3165")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let expected = source.find("\"ada.example.com\"").unwrap();
    assert_eq!(
        diagnostic.range,
        expected..expected + "\"ada.example.com\"".len()
    );
    assert!(diagnostic.message.contains("@email()"));
}

#[test]
fn force_unwrap_diagnostic_is_error_with_help() {
    let source = r#"
        const value: string? = "x";
        const forced: string = value!;
    "#;
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0716")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let expected = source.find("value!").unwrap();
    assert_eq!(diagnostic.severity, DiagnosticSeverity::Error);
    assert_eq!(diagnostic.range, expected..expected + "value!".len());
    assert!(diagnostic.message.contains("forced unwrap"));
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("if (value)"))
    );
}

#[test]
fn async_missing_promise_return_diagnostic_points_at_return_type_with_help() {
    let source = "async function load(): i32 { return 42; }\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3003")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let expected = source.find("i32").unwrap();
    assert_eq!(diagnostic.range, expected..expected + "i32".len());
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help == "change the return type to `Promise<i32>`")
    );
}

#[test]
fn async_arrow_missing_promise_return_diagnostic_points_at_return_type() {
    let source = "const load = async (): i32 => 42;\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3003")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let expected = source.find("i32").unwrap();
    assert_eq!(diagnostic.range, expected..expected + "i32".len());
}

#[test]
fn promise_shadow_diagnostic_points_at_user_declaration() {
    let source = "type Promise<T> = T;\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3049")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let expected = source.find("Promise").unwrap();
    assert_eq!(diagnostic.range, expected..expected + "Promise".len());
    assert!(diagnostic.message.contains("native async result type"));
}

#[test]
fn promise_import_shadow_diagnostic_points_at_local_alias() {
    let source = "import { LocalPromise as Promise } from \"./promise\";\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3049")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let expected = source.find("Promise }").unwrap();
    assert_eq!(diagnostic.range, expected..expected + "Promise".len());
}

#[test]
fn await_outside_async_diagnostic_points_at_await_keyword() {
    let source = "const load = (value: Promise<i32>): i32 => await value;\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3000")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let expected = source.find("await").unwrap();
    assert_eq!(diagnostic.range, expected..expected + "await".len());
}

#[test]
fn await_non_promise_diagnostic_points_at_await_with_related_expression() {
    let source = "async function load(value: i32): Promise<i32> { return await value; }\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3001")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let await_start = source.find("await").unwrap();
    let value_start = source.rfind("value").unwrap();
    assert_eq!(diagnostic.range, await_start..await_start + "await".len());
    assert!(diagnostic.related.iter().any(|related| {
        related.range == (value_start..value_start + "value".len())
            && related.message.contains("not a `Promise<T>`")
    }));
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("return `Promise<T>`"))
    );
}

#[test]
fn await_in_catch_without_a_try_suspension_lowers() {
    let source = "async function loadMessage(): Promise<string> { return \"boom\"; }\nasync function run(): Promise<void> { try { console.log(\"work\"); } catch (error: Error) { const message = await loadMessage(); console.log(message); } }\n";
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
fn await_inside_an_if_within_a_try_body_lowers() {
    let source = "async function step(): Promise<void> { return; }\nasync function run(flag: boolean): Promise<void> { try { if (flag) { await step(); } } catch (error: Error) { console.log(error.message); } }\n";
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
fn switch_awaits_and_switches_nested_in_control_flow_are_lowerable() {
    let supported = analyze(
        "async function step(): Promise<void> { return; }\nasync function select(flag: boolean): Promise<boolean> { return flag; }\nasync function run(flag: boolean): Promise<void> { switch (await select(flag)) { case true: { await step(); await step(); break; } default: { await step(); break; } } }\n",
    );
    assert!(
        !supported
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3002"),
        "diagnostics: {:#?}",
        supported.diagnostics
    );

    let nested_switch = analyze(
        "async function step(): Promise<void> { return; }\nasync function run(flag: boolean): Promise<void> { if (flag) { switch (flag) { case await Promise.resolve<boolean>(true): { await step(); break; } default: { break; } } } }\n",
    );
    assert!(
        !nested_switch
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3002"),
        "diagnostics: {:#?}",
        nested_switch.diagnostics
    );

    let nested_case_expression = analyze(
        "async function select(flag: boolean): Promise<boolean> { return flag; }\nasync function run(flag: boolean): Promise<void> { switch (flag) { case (await select(flag)) && flag: { break; } default: { break; } } }\n",
    );
    assert!(
        !nested_case_expression
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3002"),
        "diagnostics: {:#?}",
        nested_case_expression.diagnostics
    );

    let nested_clause = analyze(
        "async function step(): Promise<void> { return; }\nasync function run(flag: boolean): Promise<void> { switch (flag) { case true: { if (flag) { await step(); } break; } default: { break; } } }\n",
    );
    assert!(
        !nested_clause
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3002"),
        "diagnostics: {:#?}",
        nested_clause.diagnostics
    );

    let nested_for_of = analyze(
        "async function step(): Promise<void> { return; }\nasync function run(flag: boolean): Promise<void> { const values: i32[] = [1, 2]; switch (flag) { case true: { for (const value of values) { await step(); if (value === 1) { continue; } console.log(value); } break; } default: { break; } } }\n",
    );
    assert!(
        !nested_for_of
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3002"),
        "diagnostics: {:#?}",
        nested_for_of.diagnostics
    );
}

#[test]
fn await_in_finally_after_a_suspending_if_try_body_lowers() {
    let source = "async function step(): Promise<void> { return; }\nasync function run(flag: boolean): Promise<void> { try { if (flag) { await step(); } } catch (error: Error) { console.log(error.message); } finally { await step(); await step(); } }\n";
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
fn await_in_both_branches_of_an_if_lowers() {
    let source = "async function step(): Promise<void> { return; }\nasync function run(flag: boolean): Promise<void> { if (flag) { console.log(\"then\"); await step(); } else { console.log(\"else\"); await step(); } console.log(\"after\"); }\n";
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
fn await_in_both_branches_preserves_distinct_bindings_and_following_statements() {
    let source = "async function value(input: i32): Promise<i32> { return input; }\nasync function run(flag: boolean): Promise<void> { if (flag) { const left: i32 = await value(7); console.log(left); } else { const right: i32 = await value(9); console.log(right); } console.log(\"after\"); }\n";
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
fn both_if_branches_accept_independent_multi_suspend_chains() {
    let source = "async function run(flag: boolean): Promise<void> { if (flag) { const number: i32 = await Promise.resolve<i32>(7); await Promise.resolve(); const text: string = await Promise.resolve<string>(\"then\"); console.log(number); console.log(text); } else { const text: string = await Promise.resolve<string>(\"else\"); const number: i32 = await Promise.resolve<i32>(9); console.log(text); console.log(number); } }\n";
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
fn await_in_both_branches_of_an_if_within_a_try_lowers() {
    let source = "async function step(): Promise<void> { return; }\nasync function run(flag: boolean): Promise<void> { try { if (flag) { await step(); } else { await step(); } } catch (error: Error) { console.log(error.message); } }\n";
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
fn await_in_an_if_condition_lowers() {
    let source = "async function enabled(): Promise<boolean> { return true; }\nasync function value(): Promise<i32> { return 7; }\nasync function run(): Promise<void> { if (await enabled()) { console.log(\"enabled\"); } if ((await value()) > 0) { console.log(\"positive\"); } }\n";
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
fn loop_guard_before_await_lowers_without_reordering_the_operands() {
    let source = "async function next(index: i32): Promise<i32> { return index; }\nasync function run(): Promise<void> { let index: i32 = 0; while (index >= 0 && await next(index) < 3) { index += 1; } }\n";
    let analysis = analyze(source);
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| { matches!(diagnostic.code.as_str(), "E3010" | "E3002") }),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn external_thenable_diagnostic_points_at_awaited_value_with_help() {
    let source = "interface Thenable { then: (value: i32) => void; } async function load(value: Thenable): Promise<void> { await value; }\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3008")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let value = source.rfind("value").unwrap();
    assert_eq!(diagnostic.range, value..value + "value".len());
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("native `Promise<T>`"))
    );
}

#[test]
fn async_return_mismatch_points_at_value_with_declared_payload_and_help() {
    let source =
        "async function load(): Promise<i32> { return \"wrong\"; }\nconst later: i32 = 1;\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3296")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let value_start = source.find("\"wrong\"").unwrap();
    let annotation_start = source.find("Promise<i32>").unwrap();
    assert_eq!(
        diagnostic.range,
        value_start..value_start + "\"wrong\"".len()
    );
    assert!(diagnostic.related.iter().any(|related| {
        related.range == (annotation_start..annotation_start + "Promise<i32>".len())
            && related.message.contains("declared here")
    }));
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help == "return a value assignable to `i32`")
    );
}

#[test]
fn async_missing_return_points_at_annotation_with_fallthrough_and_help() {
    let source = "async function load(flag: boolean): Promise<i32> { if (flag) { return 42; } }\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3004")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let annotation_start = source.find("Promise<i32>").unwrap();
    let closing_brace = source.rfind('}').unwrap();
    assert_eq!(
        diagnostic.range,
        annotation_start..annotation_start + "Promise<i32>".len()
    );
    assert!(diagnostic.related.iter().any(|related| {
        related.range == (closing_brace..closing_brace + 1)
            && related.message.contains("reach the end")
    }));
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help == "return a value assignable to `i32` on every path")
    );
}

#[test]
fn unused_promise_points_at_the_discarded_call_with_shutdown_help() {
    let source = "async function load(): Promise<void> {}\nasync function main(): Promise<void> { load(); }\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3005")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let call_start = source.rfind("load()").unwrap();
    assert_eq!(diagnostic.range, call_start..call_start + "load()".len());
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("await the promise"))
    );
}

#[test]
fn async_try_is_accepted_by_shared_analysis() {
    let source = "async function load(): Promise<void> { try { return; } catch (error: Error) { console.log(error.message); } }\n";
    let analysis = analyze(source);
    assert!(
        analysis.diagnostics.is_empty(),
        "{:#?}",
        analysis.diagnostics
    );
}

#[test]
fn await_missing_expression_diagnostic_points_at_await_keyword() {
    let source = "async function load(): Promise<void> { await; }\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "async-await-missing-expression")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let expected = source.find("await").unwrap();
    assert_eq!(diagnostic.range, expected..expected + "await".len());
    assert_eq!(
        diagnostic.message,
        "`await` must be followed by an expression"
    );
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help == "write an expression after `await` or remove `await`")
    );
}

fn env_schema_fixture(env_source: &str) -> crate::env::EnvSchema {
    let (vars, diagnostics) = crate::env::parse_env_source(env_source);
    assert!(
        diagnostics.is_empty(),
        "invalid env fixture: {diagnostics:?}"
    );
    crate::env::EnvSchema {
        env_file: ".env".to_string(),
        file_path: std::path::PathBuf::from(".env"),
        source: env_source.to_string(),
        vars,
    }
}

#[test]
fn process_env_access_is_clean_with_configured_schema() {
    let schema = env_schema_fixture("@type(u32)\nPORT=3000\n");
    let source = "const port: u32 = process.env.PORT;\nconsole.log(port);\n";
    let analysis = analyze_with_context(
        source,
        AnalyzeContext {
            env_schema: Some(&schema),
            ..AnalyzeContext::default()
        },
    );
    assert!(
        analysis.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        analysis.diagnostics
    );
}

#[test]
fn undeclared_env_variable_diagnostic_anchors_on_the_variable_token() {
    let schema = env_schema_fixture("PORT=3000\n");
    let source = "const HOST = process.env.HOST;\nconsole.log(HOST);\n";
    let analysis = analyze_with_context(
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
    let start = source.find("process.env.HOST").unwrap() + "process.env.".len();
    assert_eq!(diagnostic.range, start..start + "HOST".len());
    assert!(diagnostic.help.iter().any(|help| help.contains(".env")));
}

#[test]
fn repeated_undeclared_env_accesses_claim_distinct_ranges() {
    let schema = env_schema_fixture("PORT=3000\n");
    let source = "const a = process.env.HOST;\nconst b = process.env.HOST;\nconsole.log(a);\nconsole.log(b);\n";
    let analysis = analyze_with_context(
        source,
        AnalyzeContext {
            env_schema: Some(&schema),
            ..AnalyzeContext::default()
        },
    );
    let ranges: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "E3141")
        .map(|diagnostic| diagnostic.range.clone())
        .collect();
    assert_eq!(ranges.len(), 2);
    assert_ne!(ranges[0], ranges[1]);
}

#[test]
fn env_not_configured_diagnostic_anchors_on_process_env() {
    let source = "const port = process.env.PORT;\nconsole.log(port);\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3140")
        .expect("expected E3140");
    let start = source.find("process.env").unwrap();
    assert_eq!(diagnostic.range, start..start + "process.env".len());
    assert!(diagnostic.help.iter().any(|help| help.contains("security")));
}

#[test]
fn invalid_process_use_diagnostic_anchors_on_the_access() {
    let source = "const id = process.bogus;\nconsole.log(id);\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3145")
        .expect("expected E3145");
    let start = source.find("process.bogus").unwrap();
    assert_eq!(diagnostic.range, start..start + "process.bogus".len());

    let source = "const p = process;\nconsole.log(p);\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3145")
        .expect("expected E3145");
    let start = source.find("process;").unwrap();
    assert_eq!(diagnostic.range, start..start + "process".len());
}

#[test]
fn undeclared_env_variable_in_destructure_anchors_on_the_binding() {
    let schema = env_schema_fixture("@type(u32)\nPORT=3000\n");
    let source = "const { PORT, HOST } = process.env;\nconsole.log(PORT);\nconsole.log(HOST);\n";
    let analysis = analyze_with_context(
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
    let binding = source.find("HOST").unwrap();
    assert_eq!(diagnostic.range, binding..binding + "HOST".len());
}

#[test]
fn destructure_bindings_expose_their_resolved_type() {
    let schema = env_schema_fixture("@type(u32)\nPORT=3000\n@type(string)\nTYRUS_ENV=dev\n");
    let source =
        "const { PORT, TYRUS_ENV } = process.env;\nconsole.log(PORT);\nconsole.log(TYRUS_ENV);\n";
    let analysis = analyze_with_context(
        source,
        AnalyzeContext {
            env_schema: Some(&schema),
            ..AnalyzeContext::default()
        },
    );
    let port = analysis
        .symbols
        .iter()
        .find(|symbol| symbol.name == "PORT" && symbol.kind == SymbolKind::Variable)
        .expect("PORT binding");
    assert_eq!(port.ty, "u32");
    let env = analysis
        .symbols
        .iter()
        .find(|symbol| symbol.name == "TYRUS_ENV" && symbol.kind == SymbolKind::Variable)
        .expect("TYRUS_ENV binding");
    assert_eq!(env.ty, "string");
}

#[test]
fn env_index_access_anchors_on_process_env() {
    let schema = env_schema_fixture("@type(string)\nKEY=v\n");
    let source = "const x = process.env[\"KEY\"];\nconsole.log(x);\n";
    let analysis = analyze_with_context(
        source,
        AnalyzeContext {
            env_schema: Some(&schema),
            ..AnalyzeContext::default()
        },
    );
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3144")
        .expect("expected E3144");
    let start = source.find("process.env").unwrap();
    assert_eq!(diagnostic.range, start..start + "process.env".len());
    assert!(diagnostic.message.contains("process.env.KEY"));
}

#[test]
fn pipeline_call_site_offsets_arguments_by_one() {
    let source = "const clamp = (n: i64, max: i64): i64 => n;\nconst r: i64 = 100 |> clamp(50);\nconsole.log(r);\n";
    let analysis = analyze(source);
    let call = analysis
        .call_sites
        .iter()
        .find(|call| call.callee == "clamp")
        .expect("clamp call site");
    assert_eq!(call.arg_offset, 1);
    assert_eq!(call.arguments.len(), 1);
    assert_eq!(
        call.piped_value
            .as_ref()
            .map(|range| &source[range.clone()]),
        Some("100")
    );

    let plain = analyze("const f = (a: i64): i64 => a;\nconsole.log(f(1));\n");
    let call = plain
        .call_sites
        .iter()
        .find(|call| call.callee == "f")
        .unwrap();
    assert_eq!(call.arg_offset, 0);
    assert!(call.piped_value.is_none());
}

#[test]
fn topic_pipeline_call_site_keeps_written_argument_positions() {
    let source = "const clamp = (n: i64, min: i64, max: i64): i64 => n;\nconst r: i64 = 80 |> clamp(30, %, 100);\nconsole.log(r);\n";
    let analysis = analyze(source);
    let call = analysis
        .call_sites
        .iter()
        .find(|call| call.callee == "clamp")
        .expect("clamp call site");
    assert_eq!(call.arg_offset, 0);
    assert!(call.piped_value.is_none());
    assert_eq!(call.arguments.len(), 3);
    assert_eq!(&source[call.arguments[0].clone()], "30");
    assert_eq!(&source[call.arguments[1].clone()], "%");
    assert_eq!(&source[call.arguments[2].clone()], "100");
}

#[test]
fn every_parameter_declaration_is_a_symbol_for_highlighting() {
    let source = "const clamp = (n: i64, max: i64): i64 => n + max;\nconsole.log(clamp(1, 2));\n";
    let analysis = analyze(source);
    let params: Vec<&str> = analysis
        .symbols
        .iter()
        .filter(|symbol| symbol.kind == SymbolKind::Parameter)
        .map(|symbol| &source[symbol.declaration.clone()])
        .collect();
    assert!(params.contains(&"n"));
    assert!(params.contains(&"max"));
}

#[test]
fn keyword_object_key_keeps_later_hover_ranges_aligned() {
    let source = "const config = { default: 1 };\nconst port: i32 = 42;\nconsole.log(port);\n";
    let analysis = analyze(source);
    let offset = source.rfind("port").unwrap();
    let info = analysis
        .type_at(offset)
        .expect("hover should resolve for the later `port` reference");
    assert_eq!(info.ty, "i32");
    assert_eq!(&source[info.range.clone()], "port");
}

#[test]
fn generic_unknown_type_diagnostic_anchors_to_the_type_name() {
    let source = "const run = (): void => {\n    const value: Missing<i32> = 0;\n    console.log(value);\n};\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|d| d.code == "E3202")
        .expect("expected an unknown-generic diagnostic");
    assert_eq!(&source[diagnostic.range.clone()], "Missing");
}

#[test]
fn a_nul_inside_a_string_literal_is_kept() {
    let source = "const greeting: string = \"a\\u{0}b\";\nconsole.log(greeting);\n";
    let analysis = analyze(source);
    assert!(
        analysis.diagnostics.is_empty(),
        "a NUL in a string literal is no longer a diagnostic: {:?}",
        analysis.diagnostics
    );
}

#[test]
fn a_nul_inside_a_template_literal_is_kept() {
    let source = "const greeting: string = `a\\u0000b`;\n";
    let analysis = analyze(source);
    assert!(
        analysis.diagnostics.is_empty(),
        "a NUL in a template literal is no longer a diagnostic: {:?}",
        analysis.diagnostics
    );
}

#[test]
fn future_reserved_keyword_as_binding_name_is_rejected() {
    let source = "const class = 1;\nconsole.log(class);\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0003")
        .expect("expected a reserved-keyword diagnostic");
    let start = source.find("class").unwrap();
    assert_eq!(diagnostic.range, start..start + "class".len());
    assert!(diagnostic.message.contains("reserved for future use"));
    assert!(
        diagnostic.help.iter().any(|line| line.contains("`class`")),
        "help should cite the name: {:?}",
        diagnostic.help
    );
}

#[test]
fn and_or_not_as_binding_names_are_rejected_as_active_keywords() {
    for word in ["and", "or", "not"] {
        let source = format!("const {word} = 1;\n");
        let analysis = analyze(&source);
        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "E0003")
            .unwrap_or_else(|| panic!("expected a reserved-keyword diagnostic for `{word}`"));
        assert!(
            diagnostic.message.contains("reserved keyword"),
            "`{word}` should be reported as an active keyword, got: {}",
            diagnostic.message
        );
        assert!(
            diagnostic
                .help
                .iter()
                .any(|line| line.contains(&format!("`{word}`"))),
            "help should cite the name for `{word}`: {:?}",
            diagnostic.help
        );
    }
}

#[test]
fn active_keyword_as_binding_name_is_rejected() {
    let source = "const interface = 1;\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0003")
        .expect("expected a reserved-keyword diagnostic");
    let start = source.find("interface").unwrap();
    assert_eq!(diagnostic.range, start..start + "interface".len());
    assert!(diagnostic.message.contains("reserved keyword"));
    assert!(!diagnostic.help.is_empty());
}

#[test]
fn reserved_word_as_object_key_is_allowed() {
    let source = "const config = { and: 1, class: 2 };\nconsole.log(config.and);\n";
    let analysis = analyze(source);
    assert!(
        analysis
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "E0003"),
        "object keys must not trigger E0003: {:?}",
        analysis.diagnostics
    );
}

#[test]
fn reserved_keyword_rejected_across_binding_sites() {
    let cases = [
        "function and(): void {}\n",
        "function foo(when: i32): void {}\n",
        "type or = i32;\n",
        "interface get {}\n",
        "enum Color { get }\n",
        "const run = (): void => { const [set] = [1]; };\n",
        "const run = (): void => { const { class: y } = obj; };\n",
        "const run = (): void => { for (const or of [1]) { console.log(or); } };\n",
        "import { foo as class } from \"tyrus:fs\";\n",
    ];
    for source in cases {
        let analysis = analyze(source);
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E0003"),
            "expected E0003 for {source:?}, got {:?}",
            analysis.diagnostics
        );
    }
}

#[test]
fn non_reserved_type_names_are_ordinary_identifiers() {
    let source = "const run = (): void => { const number: i32 = 1; const object: i32 = 2; console.log(number + object); };\n";
    let analysis = analyze(source);
    assert!(
        analysis
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "E0003"),
        "type names must not be reserved: {:?}",
        analysis.diagnostics
    );
}

#[test]
fn reserved_word_in_reference_position_is_rejected() {
    let source = "const run = (): void => { console.log(class); console.log(class); };\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0003")
        .expect("expected a reserved-keyword diagnostic");
    let start = source.find("class").unwrap();
    assert_eq!(diagnostic.range, start..start + "class".len());
    assert!(diagnostic.message.contains("reserved for future use"));
}

#[test]
fn reserved_word_as_arrow_parameter_is_rejected() {
    for source in [
        "const f = (class): void => {};\n",
        "const f = class => class;\n",
    ] {
        let analysis = analyze(source);
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E0003"),
            "expected E0003 for {source:?}, got {:?}",
            analysis.diagnostics
        );
    }
}

#[test]
fn speculative_backtrack_does_not_leak_reserved_error() {
    // `(x as f64)` speculatively parses as arrow parameters, hits `as` (a
    // keyword) mid-guess, then backtracks to a parenthesized expression. The
    // stashed error must not leak: this valid program has no E0003.
    let source = "const run = (): void => { const x: i32 = 42; const a: i32 = (x as f64) as i32; console.log(a); };\n";
    let analysis = analyze(source);
    assert!(
        analysis
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "E0003"),
        "valid type assertion must not report E0003: {:?}",
        analysis.diagnostics
    );
}

#[test]
fn call_site_range_spans_through_closing_paren() {
    let source = "const run = (): void => { greet(a, b); };\n";
    let analysis = analyze(source);
    let call = analysis
        .call_sites
        .iter()
        .find(|call| call.callee == "greet")
        .expect("expected a call site");
    let start = source.find("greet").unwrap();
    let end = source.find("b)").unwrap() + "b)".len();
    assert_eq!(call.range, start..end);
}

#[test]
fn http_handler_with_an_unimported_type_names_it_and_anchors_on_the_handler() {
    let source = concat!(
        "interface OkResponse { ok: boolean; }\n",
        "enum HttpStatus { Ok = 200 }\n",
        "const badHandler = (request: HttpRequest, reply: HttpReply<OkResponse>): void => {\n",
        "    reply.status(HttpStatus.Ok).send({ ok: true });\n",
        "};\n",
        "const app: HttpRouter = __tyrus_http_createRouter();\n",
        "app.get(\"/x\", { response: { 200: OkResponse }, handler: badHandler });\n",
    );
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3110")
        .expect("expected an unimported handler type diagnostic");
    assert!(
        diagnostic.message.contains("`HttpRequest`"),
        "{diagnostic:?}"
    );
    let start = source.rfind("badHandler").unwrap();
    assert_eq!(diagnostic.range, start..start + "badHandler".len());
}

#[test]
fn http_handler_mismatch_diagnostic_anchors_on_handler_name() {
    let source = concat!(
        "interface HttpReply<T> {}\n",
        "interface OkResponse { ok: boolean; }\n",
        "enum HttpStatus { Ok = 200 }\n",
        "const badHandler = (request: { }, reply: HttpReply<OkResponse>): boolean => {\n",
        "    return true;\n",
        "};\n",
        "const app: HttpRouter = __tyrus_http_createRouter();\n",
        "app.get(\"/x\", { response: { 200: OkResponse }, handler: badHandler });\n",
    );
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.message.contains("HttpReply"))
        .expect("expected handler mismatch diagnostic");
    let start = source.rfind("badHandler").unwrap();
    assert_eq!(diagnostic.range, start..start + "badHandler".len());
}

#[test]
fn break_outside_loop_points_at_the_break_keyword() {
    let source = "function main(): void {\n  break;\n}\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0268")
        .expect("expected break-outside-loop diagnostic");
    let start = source.find("break").unwrap();
    assert_eq!(diagnostic.range, start..start + "break".len());
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help == "`break` can only be used inside a loop or switch")
    );
}

#[test]
fn continue_outside_loop_points_at_the_continue_keyword() {
    let source = "function main(): void {\n  continue;\n}\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0268")
        .expect("expected continue-outside-loop diagnostic");
    let start = source.find("continue").unwrap();
    assert_eq!(diagnostic.range, start..start + "continue".len());
}

#[test]
fn break_inside_switch_case_is_a_valid_switch_exit() {
    let source = "function main(): void {\n  for (let i: i32 = 0; i < 3; i += 1) {\n    switch (i) {\n      case 1: { break; }\n      default: { console.log(i); }\n    }\n  }\n}\n";
    let analysis = analyze(source);
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E0268" || diagnostic.code == "E0269"),
        "switch break should be valid: {:?}",
        analysis.diagnostics
    );
}

#[test]
fn break_inside_match_arm_points_at_the_break_and_explains() {
    let source = "function main(): void {\n  for (let i: i32 = 0; i < 3; i += 1) {\n    match (i) {\n      (1) => { break; }\n      _ => { console.log(i); }\n    }\n  }\n}\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0269")
        .expect("expected break-inside-match diagnostic");
    let start = source.find("break").unwrap();
    assert_eq!(diagnostic.range, start..start + "break".len());
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("arms do not fall through"))
    );
}

#[test]
fn switch_break_paths_participate_in_definite_assignment() {
    let assigned_on_every_path = analyze(
        "function main(flag: boolean): void {\n  let value: i32;\n  switch (flag) {\n    case true: { value = 1; break; }\n    default: { value = 2; break; }\n  }\n  console.log(value);\n}\n",
    );
    assert!(
        !assigned_on_every_path
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E0381"),
        "all switch exits assign value: {:?}",
        assigned_on_every_path.diagnostics
    );

    let missing_assignment = analyze(
        "function main(flag: boolean): void {\n  let value: i32;\n  switch (flag) {\n    case true: { value = 1; break; }\n    default: { break; }\n  }\n  console.log(value);\n}\n",
    );
    assert!(
        missing_assignment
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E0381"),
        "missing switch assignment should be diagnosed"
    );
}

#[test]
fn unhandled_throw_anchors_on_the_call_that_is_not_handled() {
    let source = r#"const readAge = (): i32 => 16;

const acceptAge = (@min(18) age: i32): void => {
    console.log(age);
};

const main = (): void => {
    acceptAge(readAge());

    try {
        acceptAge(readAge());
    } catch (error: ValidationError) {
        console.log(error.message);
    }
};
"#;
    let diagnostics = analyze(source).diagnostics;
    let unhandled: Vec<_> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "E3011")
        .collect();
    assert_eq!(unhandled.len(), 1, "expected exactly one unhandled call");

    let anchored = &source[unhandled[0].range.clone()];
    assert_eq!(anchored, "acceptAge(readAge())");
    let first = source.find("acceptAge(readAge())").unwrap();
    assert_eq!(unhandled[0].range.start, first);

    let handled = source.rfind("acceptAge(readAge())").unwrap();
    assert_ne!(unhandled[0].range.start, handled);
}

#[test]
fn every_diagnostic_the_checker_raises_carries_a_span() {
    let sources = [
        "const main = (): void => {\n    const t: i32 = \"x\";\n    console.log(t);\n};\n",
        "interface P { x: i32; }\nconst main = (): void => {\n    const p: P = { x: 1 };\n    console.log(p.y);\n};\n",
        "interface B {\n    @doesNotExist()\n    n: i32;\n}\n",
        "interface C {\n    @min(1)\n    @min(5)\n    n: i32;\n}\n",
        "interface D {\n    @min(3)\n    s: string;\n}\n",
        "const main = (): void => {\n    const x: i32 = 1;\n    const x: i32 = 2;\n    console.log(x);\n};\n",
        "interface A { x: i32; }\ninterface A { y: i32; }\n",
        "const main = (): void => {\n    break;\n};\n",
        "const f = (a: i32): i32 => a;\nconst main = (): void => {\n    console.log(f());\n};\n",
        "const main = (): void => {\n    const v: i32? = null;\n    console.log(v ?? 0);\n};\n",
        "const main = (): void => {\n    const flag: boolean = true;\n    const n: i32 = flag - 1;\n    console.log(n);\n};\n",
    ];
    let mut total = 0;
    for source in sources {
        let program = crate::parser::Parser::from_source(source)
            .unwrap()
            .parse_program()
            .unwrap();
        for diagnostic in crate::type_checker::TypeChecker::new().check_program(&program) {
            total += 1;
            assert!(
                diagnostic.span.is_some(),
                "`{}` was published with no span (subject {:?}) for:\n{source}",
                diagnostic.code,
                diagnostic.subject
            );
        }
    }
    assert!(total >= 10, "expected the corpus to raise diagnostics");
}

#[test]
fn integer_literal_out_of_range_points_at_the_literal_and_suggests_a_wider_type() {
    let source = "const main = (): void => {\n\
                  const byte: u8 = 300;\n\
                  console.log(byte);\n\
                  };\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0706")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));

    assert_eq!(&source[diagnostic.range.clone()], "300");
    assert!(
        diagnostic.message.contains(
            "integer literal `300` does not fit in `u8`, which holds values from 0 to 255"
        )
    );
    assert!(diagnostic.help.iter().any(|help| {
        help == "the narrowest type that holds this value is `u16`; widen the type or change the value to fit"
    }));
}

#[test]
fn integer_literal_out_of_range_is_reported_for_the_wider_types_too() {
    let source = "const main = (): void => {\n\
                  const wide: i32 = 3000000000;\n\
                  console.log(wide);\n\
                  };\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0706")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));

    assert_eq!(&source[diagnostic.range.clone()], "3000000000");
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("the narrowest type that holds this value is `i64`"))
    );
}

#[test]
fn constant_integer_overflow_points_at_the_operation_and_suggests_a_wider_type() {
    let source = "const main = (): void => {\n    const numberOverflow: i32 = i32.MAX + 1;\n    console.log(numberOverflow);\n};\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0706")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));

    assert_eq!(&source[diagnostic.range.clone()], "i32.MAX + 1");
    assert!(
        diagnostic
            .message
            .contains("integer constant result `2147483648`")
    );
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("the narrowest type that holds this value is `i64`"))
    );
}

#[test]
fn pointer_sized_types_have_no_min_or_max() {
    let source = "const main = (): void => {\n\
                  const limit: usize = usize.MAX;\n\
                  console.log(limit);\n\
                  };\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0707")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));

    assert_eq!(&source[diagnostic.range.clone()], "usize.MAX");
    assert!(
        diagnostic
            .message
            .contains("`usize` has no `MAX`: it is as wide as a pointer")
    );
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("use a fixed-width type"))
    );
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("undeclared variable `usize`"))
    );
}

#[test]
fn narrow_integer_types_expose_min_and_max() {
    let source = "const main = (): void => {\n\
                  const low: i8 = i8.MIN;\n\
                  const high: u16 = u16.MAX;\n\
                  console.log(low, high);\n\
                  };\n";
    let analysis = analyze(source);
    let errors = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "diagnostics: {errors:#?}");
}

#[test]
fn char_literal_of_the_wrong_length_points_at_the_literal() {
    let source = "const main = (): void => {\n\
                  const letter: char = 'ab';\n\
                  console.log(letter);\n\
                  };\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0708")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));

    assert_eq!(&source[diagnostic.range.clone()], "'ab'");
    assert!(
        diagnostic
            .message
            .contains("a `char` holds exactly one character, but `ab` has 2")
    );
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("annotate the value with `string`"))
    );
}

#[test]
fn destructured_param_binding_does_not_leak_its_prototype_to_a_later_same_named_local() {
    let source = "interface AsyncUser {\n\
                  \x20   name: string;\n\
                  \x20   aliases: string[];\n\
                  \x20   score: i32;\n\
                  }\n\
                  \n\
                  const readAsyncUserPattern = async ({name, aliases, score}: AsyncUser): Promise<i32> => {\n\
                  \x20   return 1;\n\
                  };\n\
                  \n\
                  const main = async (): Promise<void> => {\n\
                  \x20   let name = \"Thomas\";\n\
                  \x20   console.log(name);\n\
                  };\n";
    let analysis = analyze(source);

    let main_name_offset = source.rfind("let name").unwrap() + 4;
    let symbol = analysis.symbol_at(main_name_offset).unwrap_or_else(|| {
        panic!(
            "no symbol at `main`'s `name`; symbols: {:#?}",
            analysis.symbols
        )
    });
    assert_eq!(symbol.kind, SymbolKind::Variable, "symbol: {symbol:#?}");
    assert_eq!(symbol.ty, "string", "symbol: {symbol:#?}");

    let param_names: Vec<&str> = analysis
        .symbols
        .iter()
        .filter(|s| s.kind == SymbolKind::Parameter)
        .map(|s| s.name.as_str())
        .collect();
    assert!(param_names.contains(&"name"), "params: {param_names:?}");
    assert!(param_names.contains(&"aliases"), "params: {param_names:?}");
    assert!(param_names.contains(&"score"), "params: {param_names:?}");
}

#[test]
fn destructured_object_param_bindings_resolve_to_their_own_field_type() {
    let source = "interface AsyncUser {\n\
                  \x20   name: string;\n\
                  \x20   aliases: string[];\n\
                  \x20   score: i32;\n\
                  }\n\
                  \n\
                  const readAsyncUserPattern = async ({name, aliases, score}: AsyncUser): Promise<i32> => {\n\
                  \x20   return 1;\n\
                  };\n";
    let analysis = analyze(source);
    let by_name = |n: &str| {
        analysis
            .symbols
            .iter()
            .find(|s| s.kind == SymbolKind::Parameter && s.name == n)
            .unwrap_or_else(|| {
                panic!(
                    "no Parameter symbol named `{n}`; symbols: {:#?}",
                    analysis.symbols
                )
            })
    };
    assert_eq!(by_name("name").ty, "string");
    assert_eq!(by_name("aliases").ty, "string[]");
    assert_eq!(by_name("score").ty, "i32");
}

#[test]
fn destructured_array_param_bindings_resolve_to_the_element_type() {
    let source = "const sum = ([a, b]: i32[]): i32 => a + b;\n";
    let analysis = analyze(source);
    for name in ["a", "b"] {
        let symbol = analysis
            .symbols
            .iter()
            .find(|s| s.kind == SymbolKind::Parameter && s.name == name)
            .unwrap_or_else(|| {
                panic!(
                    "no Parameter symbol named `{name}`; symbols: {:#?}",
                    analysis.symbols
                )
            });
        assert_eq!(symbol.ty, "i32", "symbol: {symbol:#?}");
    }
}

#[test]
fn rejected_object_literal_binding_has_no_resolved_type() {
    let source = r#"
interface User {
    name: string;
}

export function main() {
    const user = { name: "Eu" };
    console.log(user);
}
"#;
    let analysis = analyze(source);
    let symbol = analysis
        .symbols
        .iter()
        .find(|s| s.kind == SymbolKind::Variable && s.name == "user")
        .unwrap_or_else(|| {
            panic!(
                "no Variable symbol named `user`; symbols: {:#?}",
                analysis.symbols
            )
        });
    assert_eq!(symbol.ty, "<inferred>", "symbol: {symbol:#?}");
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|d| d.code == "E0310" && d.range == symbol.declaration)
    );
}

#[test]
fn unknown_hovers_as_the_narrowed_type_only_inside_the_proven_branch() {
    let source = "const main = (): void => {\n    const payload: unknown = \"Tyrus\";\n    if (typeof payload === \"string\") {\n        console.log(payload.length);\n    }\n    console.log(payload);\n};\n";
    let analysis = analyze(source);
    assert!(
        analysis
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != DiagnosticSeverity::Error),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
    let inside = source.find("payload.length").unwrap();
    assert_eq!(
        analysis
            .type_at(inside)
            .map(|expression| expression.ty.clone()),
        Some("string".to_string())
    );
    let outside = source.rfind("payload").unwrap();
    assert_eq!(
        analysis
            .type_at(outside)
            .map(|expression| expression.ty.clone()),
        Some("unknown".to_string())
    );
}

#[test]
fn invalid_typeof_category_anchors_on_the_literal_with_the_compared_typeof() {
    let source = "const main = (): void => {\n    const value: unknown = 1;\n    if (typeof value === \"String\") {\n        console.log(value);\n    }\n    if (\"number\" !== typeof value) {\n        console.log(value);\n    }\n};\n";
    let analysis = analyze(source);
    let invalid = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "E0795")
        .collect::<Vec<_>>();
    assert_eq!(invalid.len(), 2, "diagnostics: {:#?}", analysis.diagnostics);

    let capitalized = source.find("\"String\"").unwrap();
    assert_eq!(
        invalid[0].range,
        capitalized..capitalized + "\"String\"".len()
    );
    let first_typeof = source.find("typeof").unwrap();
    assert_eq!(invalid[0].related.len(), 1);
    assert_eq!(
        invalid[0].related[0].range,
        first_typeof..first_typeof + "typeof".len()
    );
    assert_eq!(
        invalid[0].help.first().map(String::as_str),
        Some("replace `\"String\"` with `\"string\"`")
    );

    let number = source.find("\"number\"").unwrap();
    assert_eq!(invalid[1].range, number..number + "\"number\"".len());
    let second_typeof = source.rfind("typeof").unwrap();
    assert_eq!(
        invalid[1].related[0].range,
        second_typeof..second_typeof + "typeof".len()
    );
    assert!(
        invalid[1]
            .help
            .iter()
            .all(|help| !help.starts_with("replace")),
        "`number` has no single replacement: {:?}",
        invalid[1].help
    );
}

fn encoding_imports() -> (HashMap<String, Type>, HashMap<String, Vec<Field>>) {
    let types = [
        ("utf8".to_string(), Type::Named("Utf8Namespace".to_string())),
        (
            "wide".to_string(),
            Type::Named("Utf16Namespace".to_string()),
        ),
        (
            "radix".to_string(),
            Type::Named("RadixNamespace".to_string()),
        ),
        (
            "EncodingError".to_string(),
            Type::Named("EncodingError".to_string()),
        ),
    ]
    .into_iter()
    .collect();
    let interfaces = [(
        "EncodingError".to_string(),
        crate::builtin_interfaces::encoding_error_fields(),
    )]
    .into_iter()
    .collect();
    (types, interfaces)
}

#[test]
fn encoding_strict_decoders_must_be_handled_and_encoders_need_not_be() {
    let source = r#"import { EncodingError, radix, utf16 as wide, utf8 } from "tyrus:encoding";

const main = (): void => {
    const bytes: u8[] = utf8.encode("x");
    const lossy: string = utf8.decodeLossy(bytes);
    console.log(radix.format(255, 16));
    const first: Result<string, EncodingError> = wide.decodeLE(bytes);
    match (first) {
        Ok(value) => { console.log(value); }
        Err(error) => { console.log(error.code); }
    }
    const strict: Result<string, EncodingError> = utf8.decode(bytes);
    match (strict) {
        Ok(value) => { console.log(value); }
        Err(error) => { console.log(error.code); }
    }
    const parsed: Result<u64, EncodingError> = radix.parseU64("ff", 16);
    match (parsed) {
        Ok(value) => { console.log(value); }
        Err(error) => { console.log(error.code); }
    }
};
"#;
    let (types, interfaces) = encoding_imports();
    let analysis = analyze_with_imports(source, &types, &interfaces, &HashMap::default());
    let unhandled = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "E3011")
        .collect::<Vec<_>>();
    assert!(
        unhandled.is_empty(),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(
                |diagnostic| diagnostic.severity == DiagnosticSeverity::Error
                    && diagnostic.code != "E3011"
            ),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn radix_base_accepts_only_the_supported_literal_bases() {
    let source = "import { radix } from \"tyrus:encoding\";\nconst main = (): void => {\n    const base: i32 = 16;\n    console.log(radix.format(255, 16));\n    console.log(radix.format(255, 3));\n    console.log(radix.format(255, base));\n};\n";
    let (types, interfaces) = encoding_imports();
    let analysis = analyze_with_imports(source, &types, &interfaces, &HashMap::default());
    let mismatches = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.code == "E0308" && diagnostic.message.contains("2 | 8 | 10 | 16")
        })
        .count();
    assert_eq!(mismatches, 2, "diagnostics: {:#?}", analysis.diagnostics);
}

#[test]
fn catch_annotation_naming_an_unimported_native_interface_points_at_the_type() {
    let source = "const main = (): void => {\n    try {\n        console.log(1);\n    } catch (error: EncodingError) {\n        console.log(error.code);\n        console.log(error.message);\n    }\n};\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0412")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let start = source.find("EncodingError").unwrap();
    assert_eq!(diagnostic.range, start..start + "EncodingError".len());
    assert_eq!(
        diagnostic.message,
        "cannot find type `EncodingError` in this scope"
    );
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains(r#"import { EncodingError } from "tyrus:encoding";"#)),
        "help: {:?}",
        diagnostic.help
    );
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E0609"),
        "an unresolved catch type must not cascade into field errors: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn catch_annotation_naming_an_unknown_type_offers_no_import() {
    let source = "const main = (): void => {\n    try {\n        console.log(1);\n    } catch (error: Bogus) {\n        console.log(error);\n    }\n};\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0412")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let start = source.find("Bogus").unwrap();
    assert_eq!(diagnostic.range, start..start + "Bogus".len());
    assert!(diagnostic.help.is_empty(), "help: {:?}", diagnostic.help);
}

#[test]
fn native_method_argument_mismatch_points_at_the_argument() {
    let source = "import { radix } from \"tyrus:encoding\";\nconst main = (): void => {\n    const base: i32 = 16;\n    console.log(radix.format(255, 3));\n    console.log(radix.format(255, base));\n    console.log(base);\n};\n";
    let (types, interfaces) = encoding_imports();
    let analysis = analyze_with_imports(source, &types, &interfaces, &HashMap::default());
    let mut ranges = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "E0308")
        .map(|diagnostic| diagnostic.range.clone())
        .collect::<Vec<_>>();
    ranges.sort_by_key(|range| range.start);
    let literal = source.find("255, 3)").unwrap() + "255, ".len();
    let variable = source.find("255, base)").unwrap() + "255, ".len();
    assert_eq!(
        ranges,
        vec![literal..literal + 1, variable..variable + "base".len()],
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

fn plain_field(name: &str, ty: Type) -> Field {
    Field {
        name: name.to_string(),
        ty,
        readonly: false,
        optional: false,
        decorators: Vec::new(),
        span: Span::DUMMY,
    }
}

type ReachableAddressContext = (
    HashMap<String, Type>,
    HashMap<String, Vec<Field>>,
    HashMap<String, Vec<Field>>,
);

fn reachable_address_context() -> ReachableAddressContext {
    let address = vec![plain_field("city", Type::Str)];
    let person = vec![
        plain_field("name", Type::Str),
        plain_field("address", Type::Named("Address".to_string())),
    ];
    let import_types = [("Person".to_string(), Type::Named("Person".to_string()))]
        .into_iter()
        .collect();
    let import_interfaces: crate::collections::HashMap<String, Vec<crate::ast::Field>> =
        [("Person".to_string(), person.clone())]
            .into_iter()
            .collect();
    let mut reachable: crate::collections::HashMap<String, Vec<crate::ast::Field>> =
        crate::collections::HashMap::default();
    let exports: crate::collections::HashMap<String, Vec<crate::ast::Field>> = [
        ("Person".to_string(), person.clone()),
        ("Address".to_string(), address),
    ]
    .into_iter()
    .collect();
    collect_reachable_interfaces(
        &[person],
        &|name| exports.get(name).cloned(),
        &mut reachable,
    );
    (import_types, import_interfaces, reachable)
}

#[test]
fn a_field_of_an_imported_interface_resolves_without_importing_its_type() {
    let (import_types, import_interfaces, reachable) = reachable_address_context();
    assert!(reachable.contains_key("Address"));
    let source =
        "const read = (person: Person): string => {\n    return person.address.city;\n};\n";
    let analysis = analyze_with_context(
        source,
        AnalyzeContext {
            import_types: Some(&import_types),
            import_interfaces: Some(&import_interfaces),
            reachable_interfaces: Some(&reachable),
            ..AnalyzeContext::default()
        },
    );
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn a_reachable_interface_named_without_its_import_is_refused() {
    let (import_types, import_interfaces, reachable) = reachable_address_context();
    let source = "const city = (person: Person): string => {\n    const address: Address = person.address;\n    return address.city;\n};\n";
    let analysis = analyze_with_context(
        source,
        AnalyzeContext {
            import_types: Some(&import_types),
            import_interfaces: Some(&import_interfaces),
            reachable_interfaces: Some(&reachable),
            ..AnalyzeContext::default()
        },
    );
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0412")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    assert!(diagnostic.message.contains("`Address`"));
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("add `Address` to the import"))
    );
}

#[test]
fn a_throwing_function_argument_is_reported_on_the_argument() {
    let source = "function failText(): void throws string {\n    throw \"boom\";\n}\nconst run = (action: () => void): void => {\n    action();\n};\nconst main = (): void => {\n    run(failText);\n};\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3298")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let argument = source.rfind("failText").unwrap();
    assert_eq!(diagnostic.range, argument..argument + "failText".len());
    assert!(diagnostic.message.contains("throws `string`"));
}

#[test]
fn a_throwing_http_route_handler_is_refused_once_with_a_help_it_can_follow() {
    let source = "interface HttpReply<T> {}\ninterface HttpRouter {}\nenum HttpStatus { Ok = 200 }\ninterface UserResponse {\n    id: string;\n}\nconst app: HttpRouter = __tyrus_http_createRouter();\napp.get(\"/users\", {\n    response: { 200: UserResponse },\n    handler: (request, reply) throws string => {\n        throw \"invalid\";\n    }\n});\n";
    let analysis = analyze(source);
    let refusals = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "E3298")
        .collect::<Vec<_>>();
    assert_eq!(
        refusals.len(),
        1,
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
    let handler = source.find("(request, reply)").unwrap();
    assert_eq!(refusals[0].range.start, handler);
    assert!(refusals[0].message.contains("cannot throw `string`"));
    assert!(refusals[0].help.iter().any(|help| {
        help.contains("catch `string` inside the handler and answer with `reply.status(...)`")
    }));
}

#[test]
fn a_named_throwing_http_route_handler_is_refused() {
    let source = "interface HttpReply<T> {}\ninterface HttpRouter {}\nenum HttpStatus { Ok = 200 }\ninterface UserResponse {\n    id: string;\n}\ninterface UsersRequest {}\nfunction users(request: UsersRequest, reply: HttpReply<UserResponse>): void throws string {\n    throw \"invalid\";\n}\nconst app: HttpRouter = __tyrus_http_createRouter();\napp.get(\"/users\", {\n    response: { 200: UserResponse },\n    handler: users\n});\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3298")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let reference = source.rfind("users").unwrap();
    assert_eq!(diagnostic.range, reference..reference + "users".len());
}

#[test]
fn a_synchronous_error_that_would_skip_an_awaiting_finally_is_refused_on_the_call() {
    let source = "interface ReadError {\n    code: string;\n    message: string;\n}\ninterface WriteError {\n    code: string;\n    message: string;\n}\nasync function pause(): Promise<void> {}\nfunction writeNow(): void throws WriteError {\n    const failure: WriteError = { code: \"ERR_WRITE\", message: \"disk is full\" };\n    throw failure;\n}\nasync function copy(): Promise<void> throws WriteError {\n    try {\n        writeNow();\n    } catch (error: ReadError) {\n        console.log(error.code);\n    } finally {\n        await pause();\n    }\n}\n";
    let analysis = analyze(source);
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3299")
        .unwrap_or_else(|| panic!("diagnostics: {:#?}", analysis.diagnostics));
    let call = source.find("writeNow();").unwrap();
    assert!(
        diagnostic.range.start <= call && call < diagnostic.range.end,
        "range {:?} does not cover the call at {call}",
        diagnostic.range
    );
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("catch `WriteError` in the `try` whose `finally` awaits"))
    );
}

#[test]
fn a_synchronous_error_passed_through_the_awaiting_finally_of_a_suspending_try_is_accepted() {
    let source = "interface ReadError {\n    code: string;\n    message: string;\n}\ninterface WriteError {\n    code: string;\n    message: string;\n}\nasync function pause(): Promise<void> {}\nfunction writeNow(): void throws WriteError {\n    const failure: WriteError = { code: \"ERR_WRITE\", message: \"disk is full\" };\n    throw failure;\n}\nasync function copy(): Promise<void> throws WriteError {\n    try {\n        await pause();\n        writeNow();\n    } catch (error: ReadError) {\n        console.log(error.code);\n    } finally {\n        await pause();\n    }\n}\n";
    let analysis = analyze(source);
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3299"),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn a_synchronous_error_caught_by_the_try_whose_finally_awaits_is_accepted() {
    let source = "interface WriteError {\n    code: string;\n    message: string;\n}\nasync function pause(): Promise<void> {}\nfunction writeNow(): void throws WriteError {\n    const failure: WriteError = { code: \"ERR_WRITE\", message: \"disk is full\" };\n    throw failure;\n}\nasync function copy(): Promise<void> {\n    try {\n        writeNow();\n        await pause();\n    } catch (error: WriteError) {\n        console.log(error.code);\n    } finally {\n        await pause();\n    }\n}\n";
    let analysis = analyze(source);
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3299"),
        "diagnostics: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn a_caught_error_narrowed_with_is_has_the_subtype_for_the_editor() {
    let source = "interface WriteError extends Error {\n    path: string;\n}\nfunction write(): void throws WriteError {\n    const failure: WriteError = { message: \"disk is full\", code: \"ERR_WRITE\", path: \"a.txt\" };\n    throw failure;\n}\nconst main = (): void => {\n    try {\n        write();\n    } catch (error) {\n        if (error is WriteError) {\n            console.log(error.path);\n        }\n        console.log(error.code);\n    }\n};\n";
    let analysis = analyze(source);
    let narrowed = source.find("error.path").unwrap();
    assert_eq!(
        analysis.type_at(narrowed).map(|info| info.ty.clone()),
        Some("WriteError".to_string()),
        "types: {:#?}",
        analysis.expression_types
    );
    let outside = source.find("error.code").unwrap();
    assert_eq!(
        analysis.type_at(outside).map(|info| info.ty.clone()),
        Some("Error".to_string())
    );
}

#[test]
fn an_intersection_alias_keeps_later_declarations_in_place() {
    let source = "interface B { id: i32; }\ninterface C { name: string; }\ntype A = B & C;\ninterface Later { tag: string; }\n";
    let analysis = analyze(source);
    let alias = analysis
        .symbols
        .iter()
        .find(|symbol| symbol.name == "A")
        .expect("the alias must produce a symbol");
    assert_eq!(alias.ty, "type A = B & C");
    let declaration = &source[alias.declaration.clone()];
    assert_eq!(declaration, "A");

    // The alias declares no field block. Seeking one used to run past the
    // declaration and drag every later symbol out of position.
    let later = analysis
        .symbols
        .iter()
        .find(|symbol| symbol.name == "Later")
        .expect("the interface after the alias must still produce a symbol");
    assert_eq!(&source[later.declaration.clone()], "Later");
    let tag = analysis
        .symbols
        .iter()
        .find(|symbol| symbol.name == "tag")
        .expect("the field after the alias must still produce a symbol");
    assert_eq!(&source[tag.declaration.clone()], "tag");
}
