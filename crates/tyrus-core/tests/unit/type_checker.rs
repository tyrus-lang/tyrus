use super::*;
use crate::parser::Parser;
use crate::span::Span;

#[path = "type_checker/array_tests/array_tests.rs"]
mod array_tests;
#[path = "type_checker/async_tests/async_tests.rs"]
mod async_tests;

fn diagnostics(source: &str) -> Vec<CheckDiagnostic> {
    let tokens = crate::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    TypeChecker::new().check_program(&program)
}

fn spanned_diagnostics(source: &str) -> Vec<CheckDiagnostic> {
    let spanned = crate::lexer::lex_spanned(source).unwrap();
    let tokens = spanned.iter().map(|(token, _)| token.clone()).collect();
    let ranges = spanned.iter().map(|(_, range)| range.clone()).collect();
    let program = Parser::new_with_ranges(tokens, ranges)
        .parse_program()
        .unwrap();
    TypeChecker::new().check_program(&program)
}

fn assert_error(source: &str, code: &str) {
    let diagnostics = diagnostics(source);
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code == code),
        "expected {code}, got {diagnostics:#?}"
    );
}

#[test]
fn simd_add_returns_result_and_err_match_exposes_error_fields() {
    let source = r#"
        const inspect = (result: Result<f32[], { code: string }>): void => {
            match (result) {
                Ok(values) => { console.log("ok"); }
                Err(error) => { console.log(error.code); }
            }
        };
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}
#[test]
fn accepts_well_typed_program() {
    let source = r#"
        const add = (left: i32, right: i32): i32 => left + right;
        const result: i32 = add(20, 22);
        console.log(result);
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn credentialed_wildcard_cors_anchors_the_wildcard_with_actionable_help() {
    let source = "interface CorsOptions { origin: string[]; methods: string[]; allowedHeaders: string[]; exposedHeaders: string[]; credentials: boolean; maxAge: i32; }\nconst app: HttpRouter = __tyrus_http_createRouter();\napp.cors({ origin: [\"*\"], methods: [\"GET\"], allowedHeaders: [], exposedHeaders: [], credentials: true, maxAge: 0 });";
    let diagnostic = spanned_diagnostics(source)
        .into_iter()
        .find(|diagnostic| diagnostic.code == "E2742")
        .expect("expected credentialed wildcard CORS diagnostic");
    let wildcard = source.rfind("\"*\"").expect("wildcard origin is present");
    assert_eq!(diagnostic.span, Some(Span::new(wildcard, wildcard + 3)));
    assert_eq!(
        diagnostic.help,
        vec![
            "a browser refuses this pair, so every credentialed request would fail; list the exact origins instead, or set `credentials: false`"
        ]
    );

    let named = "interface CorsOptions { origin: string[]; methods: string[]; allowedHeaders: string[]; exposedHeaders: string[]; credentials: boolean; maxAge: i32; }\nconst cors: CorsOptions = { origin: [\"*\"], methods: [\"GET\"], allowedHeaders: [], exposedHeaders: [], credentials: true, maxAge: 0 };";
    let diagnostic = spanned_diagnostics(named)
        .into_iter()
        .find(|diagnostic| diagnostic.code == "E2742")
        .expect("a named `CorsOptions` value is checked too");
    let wildcard = named.rfind("\"*\"").expect("wildcard origin is present");
    assert_eq!(diagnostic.span, Some(Span::new(wildcard, wildcard + 3)));
}

#[test]
fn onfail_binds_result_error_only_inside_its_handler() {
    let source = r#"
        interface User { name: string; }
        interface LoadError { message: string; }

        const loadUser = (): Result<User, LoadError> => {
            const user: User = { name: "Ada" };
            return Ok(user);
        };

        const render = (): Result<void, LoadError> => {
            const result = loadUser() onfail {
                console.error(result.message);
            };
            console.log(result.name);
            return Ok();
        };
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

const ONFAIL_LOADER: &str = r#"
    interface LoadError { message: string; }
    const load = (): Result<i32, LoadError> => {
        const error: LoadError = { message: "gone" };
        return Err(error);
    };
"#;

#[test]
fn onfail_on_a_result_leaves_a_void_function_at_the_end_of_its_handler() {
    let source = format!(
        "{ONFAIL_LOADER}
        const render = (): void => {{
            const value = load() onfail console.error(value.message);
            console.log(value);
        }};"
    );
    assert_eq!(diagnostics(&source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn onfail_on_a_result_accepts_any_function_when_its_handler_always_exits() {
    let source = format!(
        "{ONFAIL_LOADER}
        const count = (): i32 => {{
            const value = load() onfail {{
                console.error(value.message);
                return 0;
            }};
            return value;
        }};
        const other = (): Result<i32, string> => {{
            const value = load() onfail {{
                return Err(value.message);
            }};
            return Ok(value);
        }};
        const total = (): i32 => {{
            let sum = 0;
            for (let round = 0; round < 3; round += 1) {{
                const value = load() onfail {{
                    continue;
                }};
                sum += value;
            }}
            return sum;
        }};"
    );
    assert_eq!(diagnostics(&source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn onfail_handler_that_reaches_its_end_needs_somewhere_to_send_the_error() {
    let source = format!(
        "{ONFAIL_LOADER}
        const count = (): i32 => {{
            const value = load() onfail console.error(value.message);
            return value;
        }};"
    );
    let diagnostic = spanned_diagnostics(&source)
        .into_iter()
        .find(|diagnostic| diagnostic.code == "E0277")
        .expect("expected onfail diagnostic");
    assert_eq!(
        diagnostic.message,
        "the `onfail` handler for `value` can reach its end, but the enclosing function returns `i32`, so the error cannot propagate implicitly"
    );
    let start = source.find("onfail").expect("expected onfail keyword");
    let end = start + source[start..].find(';').expect("expected terminator");
    assert_eq!(diagnostic.span, Some(Span::new(start, end)));
    assert_eq!(
        diagnostic.help,
        vec![
            "end the handler with `return <value>;` or `throw`, or make the enclosing function return `Result<U, LoadError>` or `void`"
                .to_string()
        ]
    );
}

#[test]
fn onfail_handler_that_reaches_its_end_propagates_only_the_same_error_type() {
    let source = format!(
        "{ONFAIL_LOADER}
        const other = (): Result<i32, string> => {{
            const value = load() onfail console.error(value.message);
            return Ok(value);
        }};"
    );
    let diagnostic = spanned_diagnostics(&source)
        .into_iter()
        .find(|diagnostic| diagnostic.code == "E0308")
        .expect("expected onfail diagnostic");
    let start = source.find("onfail").expect("expected onfail keyword");
    let end = start + source[start..].find(';').expect("expected terminator");
    assert_eq!(diagnostic.span, Some(Span::new(start, end)));
    assert_eq!(
        diagnostic.help,
        vec![
            "reaching the end of the handler propagates `Err(value)`; end the handler with a `return` of the enclosing function's type instead"
                .to_string()
        ]
    );
}

#[test]
fn onfail_handler_cannot_suspend_inside_an_async_function() {
    let source = r#"
        const load = async (): Promise<Result<i32, string>> => {
            return Err("gone");
        };
        const pause = async (): Promise<void> => {};
        const run = async (): Promise<void> => {
            for (let round = 0; round < 2; round += 1) {
                const value = await load() onfail {
                    await pause();
                    return;
                };
                console.log(value);
            }
        };
    "#;
    let diagnostic = spanned_diagnostics(source)
        .into_iter()
        .find(|diagnostic| diagnostic.code == "E3002")
        .expect("expected an unsupported await diagnostic");
    let start = source.find("await pause").expect("expected handler await");
    assert_eq!(
        diagnostic.message,
        "this `await` inside an `onfail` handler requires continuation lowering that is not implemented yet"
    );
    assert_eq!(
        diagnostic.span.map(|span| span.start),
        Some(start),
        "the diagnostic anchors the `await` inside the handler"
    );
}

#[test]
fn onfail_requires_a_result_initializer() {
    assert_error(
        "const render = (): void => { const result = 1 onfail console.error(result); };",
        "E0277",
    );
}

#[test]
fn onfail_non_result_diagnostic_points_to_the_handler_keyword() {
    let source = "const render = (): void => { const result = 1 onfail console.error(result); };";
    let diagnostic = spanned_diagnostics(source)
        .into_iter()
        .find(|diagnostic| diagnostic.code == "E0277")
        .expect("expected onfail diagnostic");
    let start = source.find("onfail").expect("expected onfail keyword");
    let end = start
        + source[start..]
            .find(';')
            .expect("expected statement terminator");
    assert_eq!(diagnostic.span, Some(Span::new(start, end)));
}

#[test]
fn types_special_floating_point_literals_and_is_nan() {
    let source = "const a: f32 = Infinity; const b: f64 = -Infinity; const c: f128 = NaN; const invalid: boolean = isNaN(c); const equal: boolean = NaN === NaN; const unequal: boolean = NaN !== NaN;";
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
    assert_error("const invalid = isNaN(42);", "E0277");
    assert_error("const invalid = isNaN();", "E0061");
}

#[test]
fn validates_defer_control_flow_and_errdefer_return_context() {
    assert_error("const run = (): void => { errdefer cleanup(); };", "E0D01");
    assert_error("const run = (): void => { defer { return; } };", "E0D02");
    assert_error("const run = (): void => { defer { break; } };", "E0D03");
    assert_error("const run = (): void => { defer { continue; } };", "E0D04");
    assert_error(
        "const run = (): Result<i32, string> => { defer cleanup()?; return Ok(0); };",
        "E0D06",
    );
    assert_error("const run = (): void => { defer { throw 1; } };", "E0D08");
    assert_error(
        "const run = async (): Promise<void> => { defer cleanup(); };",
        "E0D09",
    );
}

#[test]
fn defer_diagnostics_anchor_the_keyword_or_control_token() {
    let source = "const run = (): void => { errdefer cleanup(); defer { return; } };";
    let diagnostics = spanned_diagnostics(source);
    let errdefer = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0D01")
        .expect("expected errdefer diagnostic");
    assert_eq!(
        &source[errdefer.span.expect("missing errdefer span").to_range()],
        "errdefer"
    );
    let return_ = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0D02")
        .expect("expected return diagnostic");
    assert_eq!(
        &source[return_.span.expect("missing return span").to_range()],
        "return;"
    );
}

#[test]
fn scope_exit_reasons_select_errdefer_only_for_error_paths() {
    use crate::ast::ScopeExitReason;

    for reason in [
        ScopeExitReason::Normal,
        ScopeExitReason::Return,
        ScopeExitReason::Break,
        ScopeExitReason::Continue,
    ] {
        assert!(!reason.runs_errdefer(), "{reason:?} must not run errdefer");
    }
    assert!(ScopeExitReason::ErrorReturn.runs_errdefer());
    assert!(ScopeExitReason::TryPropagation.runs_errdefer());
}

#[test]
fn accepts_drop_for_native_collections() {
    let source = r#"
        import { drop } from "tyrus:memory";
        const main = (): void => {
            let entries = Map<string, i32>();
            drop(entries);
        };
    "#;
    let collection_diagnostics = diagnostics(source);
    assert!(
        collection_diagnostics.is_empty(),
        "unexpected diagnostics: {collection_diagnostics:#?}"
    );
}

#[test]
fn types_algebraic_float_methods_only_for_matching_binary_floats() {
    let source = r#"
        const add = (left: f64, right: f64): f64 => left.algebraicAdd(right);
        const remainder = (left: f32, right: f32): f32 => left.algebraicRem(right);
        console.log(add(1.0, 2.0), remainder(5.5, 2.0));
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
    assert_error(
        "const value: i32 = 1; const result = value.algebraicAdd(2);",
        "E0599",
    );
    assert_error(
        "const value: f64 = 1.0; const result = value.algebraicMul(2);",
        "E0308",
    );
    assert_error(
        "const value: f64 = 1.0; const result = value.algebraicDiv();",
        "E0061",
    );
}

#[test]
fn accepts_partition_by_boolean_and_string_object_fields() {
    let source = r#"
        interface Ticket { title: string; completed: boolean; category: string; }
        const tickets: Ticket[] = [
            { title: "Plan", completed: true, category: "work" },
            { title: "Read", completed: false, category: "learning" }
        ];
        const [completed, pending] = tickets.partition("completed");
        const [focused, deferred] = tickets.partition("category", "work", "learning");
        console.log(completed[0].title, pending[0].title, focused.length, deferred.length);
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn rejects_partition_property_that_is_not_a_field() {
    assert_error(
        r#"
            interface Ticket { completed: boolean; }
            const tickets: Ticket[] = [{ completed: true }];
            const result = tickets.partition("archived");
        "#,
        "E0609",
    );
}

#[test]
fn rejects_partition_boolean_form_for_non_boolean_field() {
    assert_error(
        r#"
            interface Ticket { category: string; }
            const tickets: Ticket[] = [{ category: "work" }];
            const result = tickets.partition("category");
        "#,
        "E0277",
    );
}

#[test]
fn infers_generic_arguments_nested_inside_promise() {
    let source = r#"
        function cancel<T>(promise: Promise<T>, reason: string): boolean {
            console.log(reason);
            return true;
        }
        const operation: Promise<void> = Promise.resolve();
        const cancelled: boolean = cancel(operation, "Cancelled by caller");
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn requires_decorated_call_to_be_caught_or_propagated() {
    let source = r#"
        const acceptAge = (@min(18) age: i32): void => {};
        const run = (age: i32): void => { acceptAge(age); };
    "#;
    assert_error(source, "E3011");
}

#[test]
fn accepts_validation_error_propagation_and_catch() {
    let source = r#"
        const acceptAge = (@min(18) age: i32): void => {};
        const register = (age: i32): void throws ValidationError => { acceptAge(age); };
        const main = (): void => {
            try { register(21); } catch (error: ValidationError) { console.log(error.issues.length); }
        };
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn rejects_throws_on_main() {
    assert_error(
        "const main = (): void throws ValidationError => {};",
        "E3012",
    );
}

#[test]
fn requires_dynamic_decorated_objects_to_handle_validation_error() {
    let source = r#"
        interface Input { @min(18) age: i32; }
        const readAge = (): i32 => 16;
        const main = (): void => { const input: Input = { age: readAge() }; console.log(input.age); };
    "#;
    assert_error(source, "E3011");
}

#[test]
fn accepts_caught_dynamic_decorated_object_validation() {
    let source = r#"
        interface Input { @min(18) age: i32; }
        const readAge = (): i32 => 16;
        const main = (): void => {
            try { const input: Input = { age: readAge() }; console.log(input.age); }
            catch (error: ValidationError) { console.log(error.issues.length); }
        };
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn accepts_caught_dynamic_decorated_object_update_validation() {
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
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn rejects_variable_type_mismatch() {
    assert_error("const age: i32 = \"twenty\";", "E0308");
}

#[test]
fn infers_primitive_initializer_types() {
    let source = r#"
        const count = 42;
        const ratio = 3.14;
        const label = "tyrus";
        const active = true;
        const usedCount: i32 = count;
        const usedRatio: f64 = ratio;
        const usedLabel: string = label;
        const usedActive: boolean = active;
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn accepts_regular_expression_literals() {
    assert_eq!(
        diagnostics("const slugPattern: Regex = /^[a-z0-9-]+$/i;"),
        Vec::<CheckDiagnostic>::new()
    );
}

#[test]
fn rejects_unsupported_regular_expression_flags_during_type_check() {
    let diagnostics = diagnostics("const slugPattern: Regex = /^[a-z0-9-]+$/g;");
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E0309"
                && diagnostic
                    .message
                    .contains("`Regex` flag `g` is not supported")
        }),
        "expected unsupported Regex flag diagnostic, got {diagnostics:#?}"
    );
}

#[test]
fn infers_integer_literal_widening_to_i64() {
    assert_eq!(
        diagnostics("const big = 5000000000; const same: i64 = big;"),
        Vec::<CheckDiagnostic>::new()
    );
    assert_error("const big = 5000000000; const small: i32 = big;", "E0308");
    assert_eq!(
        diagnostics("const small = 42; const ok: i32 = small;"),
        Vec::<CheckDiagnostic>::new()
    );
    assert_eq!(
        diagnostics("const huge = 3_000_000_000_000_000_000_000; const same: i128 = huge;"),
        Vec::<CheckDiagnostic>::new()
    );
    assert_error(
        "const huge = 3_000_000_000_000_000_000_000; const small: i64 = huge;",
        "E0308",
    );
}

#[test]
fn infers_integer_literal_boundary_types() {
    assert_eq!(
        diagnostics(
            r#"
            const minI32 = -2_147_483_648;
            const maxI32 = 2_147_483_647;
            const sameMinI32: i32 = minI32;
            const sameMaxI32: i32 = maxI32;

            const minI64 = -9_223_372_036_854_775_808;
            const maxI64 = 9_223_372_036_854_775_807;
            const sameMinI64: i64 = minI64;
            const sameMaxI64: i64 = maxI64;

            const minI128 = -170_141_183_460_469_231_731_687_303_715_884_105_728;
            const maxI128 = 170_141_183_460_469_231_731_687_303_715_884_105_727;
            const sameMinI128: i128 = minI128;
            const sameMaxI128: i128 = maxI128;

            const firstU128: u128 = 170_141_183_460_469_231_731_687_303_715_884_105_728;
            const maxU128: u128 = 340_282_366_920_938_463_463_374_607_431_768_211_455;
            const sameFirstU128: u128 = firstU128;
            const sameMaxU128: u128 = maxU128;
            "#
        ),
        Vec::<CheckDiagnostic>::new()
    );
    assert_error(
        "const tooLargeForI128: i128 = 170_141_183_460_469_231_731_687_303_715_884_105_728;",
        "E0308",
    );
    assert_error(
        "const needsAnnotation = 170_141_183_460_469_231_731_687_303_715_884_105_728;",
        "E0310",
    );
}

#[test]
fn rejects_negative_literals_for_unsigned_types() {
    assert_error("const value: u32 = -1;", "E0308");
    assert_error("const value: u64 = -1;", "E0308");
    assert_error("const value: u128 = -1;", "E0308");
    assert_error(
        "const value: u128 = -170_141_183_460_469_231_731_687_303_715_884_105_728;",
        "E0308",
    );
}

#[test]
fn rejects_uninferable_initializer() {
    assert_error("const empty = [];", "E0310");
}

#[test]
fn rejects_anonymous_object_binding_without_annotation() {
    let diagnostics = diagnostics(r#"const user = { name: "Ada", active: true };"#);
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E0310"
                && diagnostic
                    .message
                    .contains("requires an explicit interface annotation")
                && diagnostic.subject.as_deref() == Some("user")
        }),
        "expected anonymous object inference error at binding name, got {diagnostics:#?}"
    );
}

#[test]
fn an_object_literal_names_the_interface_it_builds() {
    assert_eq!(
        diagnostics(
            r#"
            interface User {
                name: string;
                active: boolean;
            }

            const user: User = { name: "Ada", active: true };
            const same: User = user;
            "#
        ),
        Vec::<CheckDiagnostic>::new()
    );
}

#[test]
fn an_unannotated_object_literal_is_rejected_with_the_interface_to_use() {
    let diagnostics = diagnostics(
        r#"
            interface User {
                name: string;
                active: boolean;
            }

            const user = { name: "Ada", active: true };
            "#,
    );
    let annotation = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0310")
        .expect("expected the missing annotation to be reported");
    assert_eq!(annotation.subject.as_deref(), Some("user"));
    assert_eq!(annotation.help, vec!["annotate it as `: User`".to_string()]);
}

#[test]
fn an_ambiguous_object_literal_offers_every_matching_interface() {
    let diagnostics = diagnostics(
        r#"
            interface Point { x: i32; y: i32 }
            interface Coord { x: i32; y: i32 }

            const p = { x: 1, y: 2 };
            "#,
    );
    let annotation = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0310")
        .expect("expected the missing annotation to be reported");
    assert_eq!(
        annotation.help,
        vec![
            "annotate it as `: Coord`".to_string(),
            "annotate it as `: Point`".to_string(),
        ]
    );
}

#[test]
fn rejects_invalid_binary_operands() {
    assert_error("const result: boolean = 10 + false;", "E0369");
}

#[test]
fn validates_validator_decorators() {
    assert_eq!(
        diagnostics(
            r#"
            interface UserInput {
                @minLength(3)
                @regex(/^[a-z]+$/)
                name: string;
            }
            const divide = (a: f64, @notEqual(0) b: f64): f64 => a / b;
            "#,
        ),
        Vec::<CheckDiagnostic>::new()
    );
    assert_error(
        "interface UserInput { @doesNotExist() name: string; }",
        "E3160",
    );
    assert_error("interface UserInput { @min(3) name: string; }", "E3161");
    assert_error(
        "interface UserInput { @minLength(\"3\") name: string; }",
        "E3162",
    );
    assert_error(
        "interface UserInput { @regex(\"^[a-z]+$\") name: string; }",
        "E3162",
    );
    assert_error(
        "interface UserInput { @regex(/(?=admin).*/) name: string; }",
        "E3162",
    );
}

#[test]
fn resolves_membership_validator_constants_and_enum_members() {
    assert_eq!(
        diagnostics(
            r#"
            enum Role: string {
                Admin = "admin",
                User = "user",
            }

            const allowedRoles: Role[] = [Role.Admin, Role.User];

            interface AccessInput {
                @in(allowedRoles)
                role: Role;
            }

            const input: AccessInput = { role: Role.Admin };
            "#,
        ),
        Vec::<CheckDiagnostic>::new()
    );
}

#[test]
fn rejects_runtime_and_heterogeneous_membership_validator_values() {
    assert_error(
        r#"
        let allowedRoles: string[] = ["admin", "user"];
        const authorize = (@in(allowedRoles) role: string): void => {};
        "#,
        "E3162",
    );
    assert_error(
        r#"
        const authorize = (@in(["admin", true]) role: string): void => {};
        "#,
        "E3162",
    );
    assert_error(
        r#"
        interface Account { id: string; }
        const authorize = (@in(["admin"]) account: Account): void => {};
        "#,
        "E3162",
    );
}

#[test]
fn validates_decorated_call_arguments_when_literal() {
    assert_error(
        r#"
        const divideWithMessage = (a: i32, @notEqual(0, "division by zero is not allowed") b: i32): i32 => {
            return a / b;
        };
        console.log(divideWithMessage(12, 0));
        "#,
        "E3165",
    );
    assert_error(
        r#"
            const divideWithMessage = (a: i32, @notEqual(0, "division by zero is not allowed") b: i32): i32 => {
                return a / b;
            };
            const divisor: i32 = 0;
            console.log(divideWithMessage(12, divisor));
            "#,
        "E3011",
    );
}

#[test]
fn validates_optional_validator() {
    assert_eq!(
        diagnostics("interface Session { @optional() token: string?; }"),
        Vec::<CheckDiagnostic>::new()
    );
    assert_error("interface Bad { @optional() name: string; }", "E3161");
}

#[test]
fn validates_port_validator() {
    for port in ["80", "1023", "49152", "70000"] {
        assert_error(
            &format!("interface Env {{ @port() PORT: i32; }} const e: Env = {{ PORT: {port} }};"),
            "E3165",
        );
    }
    for port in ["1024", "8080", "49151"] {
        assert_eq!(
            diagnostics(&format!(
                "interface Env {{ @port() PORT: i32; }} const e: Env = {{ PORT: {port} }};"
            )),
            Vec::<CheckDiagnostic>::new(),
            "expected `{port}` to be a valid port"
        );
    }
    assert_error("interface Env { @port() PORT: string; }", "E3161");
}

#[test]
fn a_numeric_validator_is_checked_in_the_arithmetic_of_its_target() {
    assert_error(
        "interface B { @min(9007199254740993) value: i64; } const b: B = { value: 9007199254740992 };",
        "E3165",
    );
    assert_error(
        "interface E { @multipleOf(2) value: i64; } const e: E = { value: 9007199254740993 };",
        "E3165",
    );
    assert_eq!(
        diagnostics(
            "interface B { @min(9007199254740993) value: i64; } const b: B = { value: 9007199254740993 };"
        ),
        Vec::<CheckDiagnostic>::new(),
    );
}

#[test]
fn an_integral_float_literal_satisfies_the_integer_validator() {
    assert_eq!(
        diagnostics("interface M { @integer() value: f64; } const m: M = { value: 3.0 };"),
        Vec::<CheckDiagnostic>::new(),
    );
    assert_error(
        "interface M { @integer() value: f64; } const m: M = { value: 3.5 };",
        "E3165",
    );
}

#[test]
fn a_whole_number_validator_rejects_a_float_target() {
    for source in [
        "interface C { @port() value: f64; }",
        "interface C { @powerOfTwo() value: f64; }",
        "interface C { @statusCode() value: f64; }",
    ] {
        assert_error(source, "E3161");
    }
    assert_eq!(
        diagnostics("interface C { @port() value: i32; }"),
        Vec::<CheckDiagnostic>::new(),
    );
}

#[test]
fn a_negative_literal_is_proven_at_compile_time_like_any_other() {
    assert_eq!(
        diagnostics("interface T { @negative() value: i32; } const t: T = { value: -1 };"),
        Vec::<CheckDiagnostic>::new(),
    );
    assert_error(
        "interface T { @positive() value: i32; } const t: T = { value: -1 };",
        "E3165",
    );
}

#[test]
fn unique_rejects_an_element_that_is_not_compared_by_value() {
    assert_error("interface G { @unique() rows: i32[][]; }", "E3161");
    assert_eq!(
        diagnostics("interface G { @unique() names: string[]; }"),
        Vec::<CheckDiagnostic>::new(),
    );
}

#[test]
fn flags_repeated_bound_validators_as_redundant() {
    assert_error("interface F { @min(1) @min(5) pageSize: i32; }", "E3164");
    assert_error(
        "interface F { @maxLength(80) @maxLength(120) name: string; }",
        "E3164",
    );
    assert_eq!(
        diagnostics("interface F { @min(5) pageSize: i32; }"),
        Vec::<CheckDiagnostic>::new()
    );
}

#[test]
fn validates_validator_redundancy_and_contradictions() {
    assert_error(
        "interface UserInput { @email() @email(\"invalid e-mail\") email: string; }",
        "E3164",
    );
    assert_error(
        "interface UserInput { @min(10) @max(5) limit: i32; }",
        "E3163",
    );
    assert_error(
        "interface UserInput { @minLength(10) @maxLength(5) name: string; }",
        "E3163",
    );
    assert_error(
        "interface UserInput { @equal(1) @notEqual(1) value: i32; }",
        "E3163",
    );
}

#[test]
fn validates_decorated_object_literal_values_when_literal() {
    assert_eq!(
        diagnostics(
            r#"
            interface ProductFilter {
                @min(5)
                limit: i32;

                @minLength(3)
                query: string;
            }

            const filter: ProductFilter = {
                limit: 5,
                query: "notebook",
            };
            "#,
        ),
        Vec::<CheckDiagnostic>::new()
    );
    let default_message_diagnostics = diagnostics(
        r#"
        interface ProductFilter {
            @min(5)
            limit: i32;
        }

        const filter: ProductFilter = {
            limit: 3,
        };
        "#,
    );
    assert!(
        default_message_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3165"
                && diagnostic.message.contains("@min(5)")
                && diagnostic
                    .message
                    .contains("value must be greater than or equal to 5")),
        "expected validator label in default message, got {default_message_diagnostics:#?}"
    );
    let custom_message_diagnostics = diagnostics(
        r#"
        interface ProductFilter {
            @min(5, "limit must be at least 5")
            limit: i32;
        }

        const filter: ProductFilter = {
            limit: 3,
        };
        "#,
    );
    assert!(
        custom_message_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3165"
                && diagnostic.message.contains("@min(5")
                && diagnostic.message.contains("limit must be at least 5")),
        "expected custom validator message, got {custom_message_diagnostics:#?}"
    );
    let unicode_message_diagnostics = diagnostics(
        r#"
        interface ProductFilter {
            @min(5, "Valor m\u{ed}nimo permitido \u{e9} 5")
            limit: i32;
        }

        const filter: ProductFilter = {
            limit: 3,
        };
        "#,
    );
    assert!(
        unicode_message_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3165"
                && diagnostic.message.contains("Valor mínimo permitido é 5")
                && !diagnostic.message.contains("\\u{")),
        "expected decoded unicode validator message, got {unicode_message_diagnostics:#?}"
    );
}

#[test]
fn validates_decorated_inferred_object_literal_values() {
    let diagnostics = diagnostics(
        r#"
        interface ProductFilter {
            @min(5, "limit must be at least 5")
            limit: i32;
        }

        const filter: ProductFilter = {
            limit: 3,
        };
        "#,
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3165"
                && diagnostic.message.contains("limit must be at least 5")),
        "expected validator error for inferred interface object, got {diagnostics:#?}"
    );
}

#[test]
fn validates_format_decorators_on_string_literals() {
    assert_eq!(
        diagnostics(
            r#"
            interface BrazilianDocuments {
                @email()
                email: string;

                @cpf()
                cpf: string;

                @cnpj()
                cnpj: string;
            }

            const documents: BrazilianDocuments = {
                email: "ada@example.com",
                cpf: "123.456.789-09",
                cnpj: "AB.CDE.F12/3456-80",
            };
            "#,
        ),
        Vec::<CheckDiagnostic>::new()
    );
    for source in [
        r#"
        interface Input { @email() email: string; }
        const input: Input = { email: "ada.example.com" };
        "#,
        r#"
        interface Input { @cpf() cpf: string; }
        const input: Input = { cpf: "123.456.789-00" };
        "#,
        r#"
        interface Input { @cnpj() cnpj: string; }
        const input: Input = { cnpj: "AB.CDE.F12/3456-AA" };
        "#,
        r#"
        interface Input { @cnpj() cnpj: string; }
        const input: Input = { cnpj: "AB.CDE.F12/3456-81" };
        "#,
    ] {
        assert_error(source, "E3165");
    }
}

#[test]
fn validates_extended_format_decorators_on_string_literals() {
    assert_eq!(
        diagnostics(
            r#"
            interface Formats {
                @url()
                website: string;

                @uuid(7)
                id: string;

                @ip()
                ip: string;

                @ipv4()
                ipv4: string;

                @ipv6()
                ipv6: string;

                @e164()
                phone: string;

                @regex(/^[a-z0-9-]+$/i)
                slug: string;
            }

            const formats: Formats = {
                website: "https://example.com/docs",
                id: "01890f3d-7d2b-7cc8-98f1-97a7c8f1a111",
                ip: "::1",
                ipv4: "127.0.0.1",
                ipv6: "::1",
                phone: "+15551234567",
                slug: "Release-2026",
            };
            "#,
        ),
        Vec::<CheckDiagnostic>::new()
    );
    for source in [
        r#"interface Input { @url() value: string; } const input: Input = { value: "mailto:ada@example.com" };"#,
        r#"interface Input { @uuid(4) value: string; } const input: Input = { value: "01890f3d-7d2b-7cc8-98f1-97a7c8f1a111" };"#,
        r#"interface Input { @ip() value: string; } const input: Input = { value: "not-an-ip" };"#,
        r#"interface Input { @ipv4() value: string; } const input: Input = { value: "::1" };"#,
        r#"interface Input { @ipv6() value: string; } const input: Input = { value: "127.0.0.1" };"#,
        r#"interface Input { @e164() value: string; } const input: Input = { value: "15551234567" };"#,
        r#"interface Input { @regex(/^[a-z]+$/) value: string; } const input: Input = { value: "abc123" };"#,
    ] {
        assert_error(source, "E3165");
    }
}

#[test]
fn validates_safe_integer_literals_with_tyrus_native_semantics() {
    assert_eq!(
        diagnostics(
            r#"
            interface NativeIntegers {
                @safeInteger()
                id: i128;
            }

            const value: NativeIntegers = {
                id: 170141183460469231731687303715884105727,
            };
            "#,
        ),
        Vec::<CheckDiagnostic>::new()
    );
    assert_eq!(
        diagnostics(
            r#"
            interface FloatInteger {
                @safeInteger()
                value: f64;
            }

            const value: FloatInteger = {
                value: 42.0,
            };
            "#,
        ),
        Vec::<CheckDiagnostic>::new()
    );
    assert_error(
        r#"
        interface FloatInteger {
            @safeInteger()
            value: f64;
        }

        const value: FloatInteger = {
            value: 42.5,
        };
        "#,
        "E3165",
    );
}

#[test]
fn reuses_validator_facts_for_impossible_conditions() {
    let diagnostics = diagnostics(
        r#"
        const check = (@notEqual(0) value: i32): void => {
            if (value === 0) {
                console.log(value);
            }
            if (value !== 0) {
                console.log(value);
            }
        };
        "#,
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E3166" && diagnostic.message.contains("value === 0")
        }),
        "expected impossible condition warning, got {diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E3167" && diagnostic.message.contains("value !== 0")
        }),
        "expected redundant condition warning, got {diagnostics:#?}"
    );
}

#[test]
fn reuses_validator_bounds_and_length_facts_for_conditions() {
    let diagnostics = diagnostics(
        r#"
        const check = (@min(1) @max(10) count: i32, @length(3) code: string): void => {
            if (count < 1) {
                console.log(count);
            }
            if (count <= 10) {
                console.log(count);
            }
            if (code.length !== 3) {
                console.log(code);
            }
        };
        "#,
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E3166" && diagnostic.message.contains("count < 1")
        }),
        "expected impossible numeric bound warning, got {diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E3167" && diagnostic.message.contains("count <= 10")
        }),
        "expected redundant numeric bound warning, got {diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E3166" && diagnostic.message.contains("code.length !== 3")
        }),
        "expected impossible length warning, got {diagnostics:#?}"
    );
}

#[test]
fn narrows_not_null_decorated_optional_parameters() {
    assert_eq!(
        diagnostics(
            r#"
            const useName = (@notNull() name: string?): void => {
                const concrete: string = name;
                console.log(concrete);
            };
            "#,
        ),
        Vec::<CheckDiagnostic>::new()
    );
}

#[test]
fn validates_format_decorators_on_literal_call_arguments() {
    assert_eq!(
        diagnostics(
            r#"
            const submit = (@email() email: string, @cpf() cpf: string, @cnpj() cnpj: string): void => {};
            submit("ada@example.com", "123.456.789-09", "AB.CDE.F12/3456-80");
            "#,
        ),
        Vec::<CheckDiagnostic>::new()
    );
    assert_error(
        r#"
        const submit = (@email() email: string): void => {};
        submit("ada.example.com");
        "#,
        "E3165",
    );
    assert_error(
        r#"
        const submit = (@cpf() cpf: string): void => {};
        submit("111.111.111-11");
        "#,
        "E3165",
    );
    assert_error(
        r#"
        const submit = (@cnpj() cnpj: string): void => {};
        submit("AB.CDE.F12/3456-81");
        "#,
        "E3165",
    );
}

#[test]
fn skips_static_validator_diagnostics_for_dynamic_values() {
    let diagnostics = diagnostics(
        r#"
            interface ProductFilter {
                @min(5)
                limit: i32;

                @minItems(2)
                tags: string[];
            }

            const limit: i32 = 3;
            const tags: string[] = ["a"];
            const filter: ProductFilter = {
                limit: limit,
                tags: tags,
            };
            "#,
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "E3165")
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3011")
    );
}

#[test]
fn validates_decorated_inline_object_literal_values() {
    assert_error(
        r#"
        type ProductFilter = {
            @notEmpty()
            query: string;
        };

        const filter: ProductFilter = {
            query: "",
        };
        "#,
        "E3165",
    );
}

#[test]
fn validates_decorated_object_literal_arguments_when_literal() {
    assert_error(
        r#"
        interface ProductFilter {
            @maxItems(2)
            tags: string[];
        }

        const search = (filter: ProductFilter): void => {};
        search({
            tags: ["a", "b", "c"],
        });
        "#,
        "E3165",
    );
}

#[test]
fn rejects_wrong_call_arity_and_argument_type() {
    let source = r#"
        const add = (a: i32, b: i32): i32 => a + b;
        add("10");
    "#;
    assert_error(source, "E0061");
    assert_error(source, "E0308");
}

#[test]
fn rejects_wrong_return_type() {
    assert_error("const greet = (): i32 => { return \"hello\"; };", "E0308");
}

#[test]
fn rejects_missing_return_path() {
    assert_error("const value = (): i32 => { const x: i32 = 1; };", "E0310");
}

#[test]
fn accepts_a_suspending_if_nested_in_a_while_with_break_and_continue() {
    let source = r#"
        async function pause(): Promise<void> {}
        async function main(): Promise<void> {
            let index: i32 = 0;
            let total: i32 = 0;
            while (index < 6) {
                index += 1;
                if (index % 2 === 0) {
                    await pause();
                    if (index === 2) { continue; }
                }
                total += index;
                if (index === 5) { break; }
            }
            console.log(total);
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn accepts_suspending_if_branches_in_every_supported_loop() {
    let source = r#"
        async function pause(): Promise<void> {}
        async function main(values: i32[]): Promise<void> {
            let index: i32 = 0;
            do { if (index === 0) { await pause(); } index += 1; } while (index < 1);
            for (let current: i32 = 0; current < 1; current += 1) { if (current === 0) { await pause(); } }
            for (const value of values) { if (value > 0) { await pause(); } }
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn accepts_abrupt_completions_through_suspending_finally() {
    let source = r#"
        async function pause(): Promise<void> {}
        async function returnsThroughFinally(): Promise<i32> {
            try { return 7; } catch (error: Error) { return 0; } finally { await pause(); }
        }
        async function transfersThroughFinally(): Promise<i32> {
            let total: i32 = 0;
            for (let index: i32 = 0; index < 4; index += 1) {
                try {
                    if (index === 1) { continue; }
                    if (index === 3) { break; }
                    total += index;
                } catch (error: Error) {} finally { await pause(); }
            }
            return total;
        }
        async function throwsThroughFinally(): Promise<void> throws Error {
            try { throw { message: "boom", code: "ERR_BOOM" }; } catch (error: Error) { throw error; } finally { await pause(); }
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn accepts_a_direct_await_inside_an_if_branch() {
    let source = r#"
        async function pause(): Promise<void> {}
        async function run(enabled: boolean): Promise<void> {
            if (enabled) {
                console.log("before");
                await pause();
                console.log("after");
            } else {
                console.log("disabled");
            }
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn accepts_a_direct_await_inside_a_while_body() {
    let source = r#"
        async function pause(): Promise<void> {}
        async function run(): Promise<void> {
            let count: i32 = 0;
            while (count < 3) {
                await pause();
                count += 1;
            }
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn narrows_optional_and_union_payloads_before_the_owned_local_codegen_boundary() {
    let source = r#"
        async function inspectOptional(value: Promise<i32?>): Promise<void> {
            const result = await value;
            if (result) {
                console.log(result + 1);
            }
        }

        async function inspectUnion(value: Promise<boolean | i32>): Promise<void> {
            const result = await value;
            if (typeof result === "boolean") {
                console.log(result);
            } else {
                console.log(result + 1);
            }
        }
    "#;
    let diagnostics = diagnostics(source);
    assert_eq!(diagnostics, Vec::<CheckDiagnostic>::new());
}

#[test]
fn rejects_promise_with_wrong_arity() {
    assert_error("const value: Promise<i32, string> = 1;", "E3048");
}

#[test]
fn rejects_user_declared_promise_type_name() {
    assert_error("type Promise<T> = T;", "E3049");
    assert_error("interface Promise<T> { value: T; }", "E3049");
    assert_error("const Promise = 1;", "E3049");
}

#[test]
fn rejects_imported_promise_alias() {
    assert_error(
        "import { LocalPromise as Promise } from \"./promise\";\n",
        "E3049",
    );
}

#[test]
fn rejects_user_declared_join_handle_type_name() {
    assert_error("type JoinHandle<T> = T;", "E3042");
    assert_error("interface JoinHandle<T> { value: T; }", "E3042");
    assert_error("const JoinHandle = 1;", "E3042");
    assert_error(
        "import { LocalJoinHandle as JoinHandle } from \"./tasks\";\n",
        "E3042",
    );
}

#[test]
fn accepts_panic_as_a_bare_statement_with_a_string_message() {
    assert_eq!(
        diagnostics(
            r#"
                function validate(age: i32): void {
                    if (age < 0) {
                        panic("age cannot be negative");
                    }
                    console.log(age);
                }
            "#,
        ),
        Vec::<CheckDiagnostic>::new()
    );
}

#[test]
fn rejects_panic_with_the_wrong_arity_or_message_type() {
    assert_error(
        r#"
            function main(): void {
                panic();
            }
        "#,
        "E0061",
    );
    assert_error(
        r#"
            function main(): void {
                panic(42);
            }
        "#,
        "E0308",
    );
}

#[test]
fn rejects_panic_used_outside_statement_position() {
    assert_error(
        r#"
            function main(): void {
                const x: i32 = panic("boom");
                console.log(x);
            }
        "#,
        "E0425",
    );
}

#[test]
fn rejects_panic_redeclaration_as_an_ambient_builtin() {
    for source in ["function panic(): void {}", "const panic = 1;"] {
        let tokens = crate::lexer::lex(source).unwrap();
        let error = Parser::new(tokens)
            .parse_program()
            .expect_err(&format!("expected a parse error for `{source}`"));
        assert_eq!(error.code, Some("E0003"));
    }
}

#[test]
fn rejects_await_on_non_promise_value() {
    assert_error(
        "async function load(value: i32): Promise<i32> { return await value; }",
        "E3001",
    );
}

#[test]
fn rejects_external_thenables_with_a_dedicated_diagnostic() {
    assert_error(
        "interface Thenable { then: (value: i32) => void; } async function load(value: Thenable): Promise<void> { await value; }",
        "E3008",
    );
}

#[test]
fn types_native_promise_static_and_instance_methods() {
    let source = r#"
        async function ready(): Promise<i32> { return 21; }
        function testCapability(): void {
            const capability = Promise.withResolvers<i32>();
            capability.resolve(42);
            capability.reject({ message: "expected", code: "ERR_EXPECTED" });
        }
        async function main(): Promise<void> {
            const direct = Promise.resolve(21);
            const empty = Promise.resolve().then((): void => {});
            const doubled = direct.then((value: i32): i32 => value * 2);
            const recovered = doubled.catch((error: Error): i32 => 0);
            const finalized = recovered.finally((): void => {});
            const combined = Promise.all([ready(), ready()]);
            const settled = Promise.allSettled([ready(), ready()]);
            const fastest = Promise.race([ready(), ready()]);
            const first = Promise.any([ready(), ready()]);
            const explicitlyRejected = Promise.reject<i32>({ message: "expected", code: "ERR_EXPECTED" });
            await finalized;
            await combined;
            await settled;
            await fastest;
            await first;
            await explicitlyRejected;
            await empty;
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn promise_reject_requires_a_native_error_reason() {
    assert_error("const rejected = Promise.reject(42);", "E0308");
    assert_eq!(
        diagnostics(
            "const failure: Error = { message: \"boom\", code: \"ERR_BOOM\" }; const rejected = Promise.reject(failure);"
        ),
        Vec::<CheckDiagnostic>::new()
    );
}

#[test]
fn rejects_undeclared_identifier() {
    assert_error("missing();", "E0425");
}

#[test]
fn rejects_duplicate_declaration_in_scope() {
    assert_error(
        "const main = (): void => { const x: i32 = 1; const x: i32 = 2; };",
        "E0428",
    );
}

#[test]
fn accepts_full_numeric_type_set_constants_and_explicit_conversions() {
    let source = r#"
        const signed: i128 = i128.MAX;
        const tiny: i8 = i8.MIN;
        const small: i16 = i16.MAX;
        const byte: u8 = u8.MAX;
        const word: u16 = u16.MAX;
        // `isize`/`usize` are as wide as a pointer on the target machine, so they
        // expose no `MIN`/`MAX` (see `pointer_sized_types_have_no_min_or_max`).
        const pointerSigned: isize = -1;
        const unsigned: u128 = u128.MAX;
        const pointerUnsigned: usize = 0;
        const unsignedMin: u64 = u64.MIN;
        const floatMax: f128 = f128.MAX;
        const decimalMin: decimal = decimal.MIN;
        const decimalMax: decimal = decimal.MAX;
        const promoted: f128 = unsigned as f128;
        const narrowed: u32 = promoted as u32;
        const signedAgain: i64 = narrowed as i64;
        let names: string[] = ["Ada", "Grace"];
        const count: usize = names.length;
        const capacity: usize = names.capacity;
        const textLength: usize = "tyrus".length;
        const pushed: usize = names.push("Linus");
        names.forEach((value: string, index: usize): void => {});
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn accepts_parse_decimal_as_an_optional_decimal() {
    assert_eq!(
        diagnostics("const value: decimal? = parseDecimal(\"19.3\");"),
        Vec::<CheckDiagnostic>::new()
    );
}

#[test]
fn rejects_implicit_decimal_mixing_with_integers_and_floats() {
    assert_error("const value: decimal = 1d + 2;", "E0369");
    assert_error("const value: decimal = 1d + 2.0;", "E0369");
    assert_error("const equal: boolean = 1d === 1;", "E0308");
    assert_error("const ordered: boolean = 1d < 1.0;", "E0369");
}

#[test]
fn rejects_integer_literal_that_does_not_fit_its_annotation() {
    assert_error("const byte: u8 = 300;", "E0706");
    assert_error("const small: i16 = 40000;", "E0706");
    assert_error("const tiny: i8 = -129;", "E0706");
    assert_error("const wide: i32 = 3000000000;", "E0706");
}

#[test]
fn rejects_out_of_range_integer_literals_in_typed_arrays() {
    assert_error("const values: i8[] = [255; 5];", "E0706");
    assert_error("const values: i8[] = [120, 128];", "E0706");
    assert_error("const values: [i8; 5] = [255; 5];", "E0706");
}

#[test]
fn rejects_out_of_range_literal_for_pointer_sized_integers() {
    assert_error("const size: usize = 18446744073709551616;", "E0706");
    assert_error("const offset: isize = 9223372036854775808;", "E0706");
    assert_error("const negative: usize = -1;", "E0706");
}

#[test]
fn rejects_constant_integer_expression_that_overflows_its_annotation() {
    assert_error("const numberOverflow: i32 = i32.MAX + 1;", "E0706");
}

#[test]
fn accepts_integer_literals_at_the_bounds_of_the_narrow_types() {
    let source = r#"
        const tiny: i8 = -128;
        const tinyMax: i8 = 127;
        const small: i16 = -32768;
        const byte: u8 = 255;
        const word: u16 = 65535;
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn rejects_out_of_range_literal_in_argument_and_object_field() {
    assert_error(
        "const take = (byte: u8): void => {};\nconst main = (): void => { take(300); };",
        "E0706",
    );
    assert_error(
        "interface Pixel { red: u8; }\nconst p: Pixel = { red: 300 };",
        "E0706",
    );
}

#[test]
fn accepts_a_char_written_as_a_one_character_literal() {
    let source = r#"
        const letter: char = 'A';
        const quoted: char = "B";
        const emoji: char = '😀';
        const code: u32 = letter as u32;
        const back: char = code as char;
        const maybe: char? = code as char?;
        const text: string = letter as string;
        const upper: char = letter.toUpperCase();
        const digit: boolean = letter.isDigit();
        const letters: char[] = "hello".chars();
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn rejects_a_char_literal_that_is_not_one_character() {
    assert_error("const letter: char = 'ab';", "E0708");
    assert_error("const letter: char = '';", "E0708");
}

#[test]
fn rejects_unrelated_conversions_involving_char() {
    assert_error(
        "const text: string = \"ab\";\nconst letter: char = text as char;",
        "E0605",
    );
    assert_error(
        "const ratio: f64 = 1.5;\nconst letter: char = ratio as char;",
        "E0605",
    );
}

#[test]
fn rejects_min_and_max_on_pointer_sized_types() {
    assert_error("const limit: usize = usize.MAX;", "E0707");
    assert_error("const low: isize = isize.MIN;", "E0707");
}

#[test]
fn accepts_optional_object_properties_and_immutable_updates() {
    let source = r#"
        interface User {
            name: string;
            age: i32;
            nickname?: string;
        }

        const user: User = { name: "Ada", age: 36 };
        const renamed: User = { ...user, name: "Grace" };
        const withNickname: User = { ...renamed, nickname: "Amazing Grace" };
        const nickname: string? = withNickname.nickname;
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn accepts_const_assertions_with_literal_and_readonly_types() {
    let source = r#"
        interface Config {
            readonly mode: "dev";
            readonly ports: readonly i32[];
        }

        const status: "ready" = "ready" as const;
        const config: Config = { mode: "dev", ports: [3000, 3001] } as const;
        const mode: "dev" = config.mode;
        const first: i32 = config.ports[0];
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn accepts_a_const_asserted_object_assigned_to_a_mutable_interface() {
    let source = r#"
        interface Config {
            mode: string;
            port: i32;
        }

        const config: Config = { mode: "dev", port: 3000 } as const;
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn reads_fields_through_a_deeply_readonly_value() {
    let source = r#"
        interface Address {
            city: string;
        }

        interface User {
            name: string;
            tags: string[];
            address: Address;
        }

        function getUser(): User {
            return { name: "Ada", tags: ["a"], address: { city: "Rio" } };
        }

        const user: readonly User = getUser();
        const name: string = user.name;
        const city: string = user.address.city;
        const present: boolean = user.tags.includes("a");
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn classifies_a_uniform_tuple_as_a_contiguous_run() {
    let sequence = tuple_sequence(&[Type::I32, Type::I32]);
    assert!(sequence.is_uniform());
    assert_eq!(sequence.element(), Some(&Type::I32));
}

#[test]
fn classifies_literal_positions_by_their_stored_type() {
    let one = Type::Lit(Box::new(Expr::synthetic(ExprKind::IntLit(1))));
    let two = Type::Lit(Box::new(Expr::synthetic(ExprKind::IntLit(2))));
    let sequence = tuple_sequence(&[one, two]);
    assert!(sequence.is_uniform());
    assert_eq!(sequence.element(), Some(&Type::I32));
}

#[test]
fn classifies_a_mixed_tuple_as_a_union_of_its_positions() {
    let sequence = tuple_sequence(&[Type::I32, Type::Str]);
    assert!(!sequence.is_uniform());
    assert_eq!(
        sequence.element(),
        Some(&Type::Union(vec![Type::I32, Type::Str]))
    );
}

#[test]
fn classifies_positions_of_different_widths_as_mixed() {
    let sequence = tuple_sequence(&[Type::I32, Type::I64]);
    assert!(!sequence.is_uniform());
}

#[test]
fn classifies_an_empty_tuple_as_having_no_element() {
    let sequence = tuple_sequence(&[]);
    assert_eq!(sequence, TupleSequence::Empty);
    assert_eq!(sequence.element(), None);
}

#[test]
fn a_transforming_method_on_a_tuple_does_not_produce_a_tuple() {
    let source = r#"
        const point: [i32, i32, i32] = [10, 20, 30];
        const same: [i32, i32, i32] = point.slice(0);
    "#;
    assert_error(source, "E0308");
}

#[test]
fn rejects_a_mutating_method_on_a_tuple() {
    let source = r#"
        let point: [i32, i32] = [1, 2];
        point.push(3);
    "#;
    assert_error(source, "E2543");
}

#[test]
fn rejects_sort_on_a_tuple_through_the_functional_path() {
    let source = r#"
        let point: [i32, i32] = [1, 2];
        point.sort();
    "#;
    assert_error(source, "E2543");
}

#[test]
fn offers_no_sequence_surface_on_a_mixed_tuple() {
    let source = r#"
        const pair: [i32, string] = [1, "a"];
        const has: boolean = pair.includes(1);
    "#;
    assert_error(source, "E0599");
}

#[test]
fn iterates_a_tuple_with_for_of() {
    let source = r#"
        const point: [i32, i32, i32] = [10, 20, 30];
        for (const value of point) {
            const current: i32 = value;
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn rejects_iterating_a_mixed_tuple() {
    let source = r#"
        const pair: [i32, string] = [1, "a"];
        for (const value of pair) {
            console.log(value);
        }
    "#;
    assert_error(source, "E0277");
}

#[test]
fn keeps_readonly_transitive_when_iterating_a_readonly_tuple() {
    let source = r#"
        interface User {
            tags: string[];
        }

        function build(): [User, User] {
            return [{ tags: ["a"] }, { tags: ["b"] }];
        }

        const users: readonly [User, User] = build();
        for (const user of users) {
            user.tags.push("c");
        }
    "#;
    assert_error(source, "E2540");
}

#[test]
fn rejects_mutating_method_on_a_readonly_for_of_element() {
    let source = r#"
        interface User {
            tags: string[];
        }

        function getUsers(): User[] {
            return [{ tags: ["a"] }];
        }

        const users: readonly User[] = getUsers();
        for (const user of users) {
            user.tags.push("b");
        }
    "#;
    assert_error(source, "E2540");
}

#[test]
fn reads_fields_of_a_readonly_for_of_element() {
    let source = r#"
        interface User {
            name: string;
            tags: string[];
        }

        function getUsers(): User[] {
            return [{ name: "Ada", tags: ["a"] }];
        }

        const users: readonly User[] = getUsers();
        for (const user of users) {
            const name: string = user.name;
            const present: boolean = user.tags.includes("a");
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn accepts_field_assignment_on_a_mutable_object() {
    let source = r#"
        interface Point {
            x: i64;
            y: i64;
        }

        let p: Point = { x: 1, y: 2 };
        p.x = 10;
        p.y += 5;
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn rejects_assignment_to_a_readonly_property() {
    let source = r#"
        interface User {
            readonly id: i64;
            name: string;
        }

        let user: User = { id: 1, name: "Ada" };
        user.id = 2;
    "#;
    assert_error(source, "E2542");
}

#[test]
fn rejects_field_assignment_through_a_deeply_readonly_value() {
    let source = r#"
        interface User {
            id: i64;
        }

        function build(): User {
            return { id: 1 };
        }

        let user: readonly User = build();
        user.id = 2;
    "#;
    assert_error(source, "E2541");
}

#[test]
fn rejects_field_assignment_through_a_const_binding() {
    let source = r#"
        interface User {
            id: i64;
        }

        const user: User = { id: 1 };
        user.id = 2;
    "#;
    assert_error(source, "E0384");
}

#[test]
fn rejects_field_assignment_with_a_mismatched_value() {
    let source = r#"
        interface User {
            id: i64;
        }

        let user: User = { id: 1 };
        user.id = "two";
    "#;
    assert_error(source, "E0308");
}

#[test]
fn rejects_const_assertion_on_non_literal_expression() {
    let source = r#"
        const value: i32 = 42;
        const frozen: i32 = value as const;
    "#;
    assert_error(source, "E1355");
}

#[test]
fn rejects_borrow_expression_assigned_to_owned_type() {
    let source = r#"
        const name: string = "TitenQ";
        const borrowed: string = &name;
    "#;
    assert_error(source, "E0308");
}

#[test]
fn accepts_explicit_reference_bindings() {
    let source = r#"
        let count: i32 = 1;
        const borrowed: &i32 = &count;
        const mutableBorrowed: &mut i32 = &mut count;
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn rejects_excess_object_properties() {
    let source = r#"
        interface User {
            name: string;
        }

        const user: User = { name: "Ada", age: 36 };
    "#;
    assert_error(source, "E2353");
}

#[test]
fn rejects_missing_required_object_property_with_specific_diagnostic() {
    let source = r#"
        interface User {
            name: string;
            age: i32;
        }

        const user: User = { name: "Ada" };
    "#;
    let diagnostics = diagnostics(source);
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E2741"
                && diagnostic.message
                    == "property `age` is missing in object literal for type `User`"
                && diagnostic.subject.as_deref() == Some("user")
        }),
        "expected missing-property diagnostic on `user`, got {diagnostics:#?}"
    );
}

#[test]
fn rejects_unknown_object_update_property() {
    let source = r#"
        interface User {
            name: string;
        }

        const user: User = { name: "Ada" };
        const renamed: User = { ...user, nickname: "Ada" };
    "#;
    assert_error(source, "E0609");
}

#[test]
fn accepts_exhaustive_match_over_discriminated_union() {
    let source = r#"
        type Result =
            | { tag: "success"; data: string }
            | { tag: "error"; message: string };

        const result: Result = { tag: "success", data: "ok" };
        match (result) {
            ({ tag: "success", data }) => {
                console.log(data);
            }
            ({ tag: "error", message }) => {
                console.log(message);
            }
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn rejects_non_exhaustive_match_over_discriminated_union() {
    let source = r#"
        type Result =
            | { tag: "success"; data: string }
            | { tag: "error"; message: string };

        const result: Result = { tag: "success", data: "ok" };
        match (result) {
            ({ tag: "success", data }) => {
                console.log(data);
            }
        }
    "#;
    assert_error(source, "E0004");
}

#[test]
fn warns_on_unreachable_match_arm() {
    let source = r#"
        type Status = "ready" | "failed";
        const status: Status = "ready";

        match (status) {
            _ => {
                console.log("any");
            }
            ("ready") => {
                console.log("ready");
            }
        }
    "#;
    assert_error(source, "W0002");
}

#[test]
fn guarded_match_arm_does_not_make_match_exhaustive() {
    let source = r#"
        type Result =
            | { tag: "success"; data: string }
            | { tag: "error"; message: string };

        const result: Result = { tag: "success", data: "ok" };
        match (result) {
            ({ tag: "success", data }) if (data === "ok") => {
                console.log(data);
            }
            ({ tag: "error", message }) => {
                console.log(message);
            }
        }
    "#;
    assert_error(source, "E0004");
}

#[test]
fn throw_type_checked_against_catch_clause() {
    let source = r#"
        const main = (): void => {
            try {
                throw "error message";
            } catch (e: string) {
                console.log(e);
            }
        };
    "#;
    let d = diagnostics(source);
    assert!(d.is_empty(), "expected no type errors: {d:#?}");
}

#[test]
fn throw_type_mismatch_is_an_error() {
    let source = r#"
        const main = (): void => {
            try {
                throw 42;
            } catch (e: string) {
                console.log(e);
            }
        };
    "#;
    assert_error(source, "E0308");
}

#[test]
fn catch_binding_is_in_scope() {
    let source = r#"
        const main = (): void => {
            try {
                throw "oops";
            } catch (err: string) {
                console.log(err);
            }
        };
    "#;
    let d = diagnostics(source);
    assert!(d.is_empty(), "catch binding should be in scope: {d:#?}");
}

#[test]
fn try_catch_finally_no_errors() {
    let source = r#"
        const main = (): void => {
            try {
                throw "problem";
            } catch (e: string) {
                console.log(e);
            } finally {
                console.log("done");
            }
        };
    "#;
    let d = diagnostics(source);
    assert!(
        d.is_empty(),
        "try/catch/finally should type-check cleanly: {d:#?}"
    );
}

#[test]
fn print_and_print_raw_accept_any_printable_argument_count() {
    let source = r#"
        const main = (): void => {
            print();
            print("Ano:", 2026, true);
            printRaw("sem quebra de linha");
        };
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn print_rejects_a_non_printable_argument() {
    let source = r#"
        const add = (left: i32, right: i32): i32 => left + right;
        const main = (): void => { print(add); };
    "#;
    assert_error(source, "E0277");
}

#[test]
fn print_raw_rejects_a_non_printable_argument() {
    let source = r#"
        const add = (left: i32, right: i32): i32 => left + right;
        const main = (): void => { printRaw(add); };
    "#;
    assert_error(source, "E0277");
}

#[test]
fn print_cr_accepts_any_printable_argument_count() {
    let source = r#"
        const main = (): void => {
            printCR();
            printCR("Loading", 42, true);
        };
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn print_cr_rejects_a_non_printable_argument() {
    let source = r#"
        const add = (left: i32, right: i32): i32 => left + right;
        const main = (): void => { printCR(add); };
    "#;
    assert_error(source, "E0277");
}

#[test]
fn loop_expression_with_break_value_typechecks() {
    let source = r#"
        const search = (limit: i32): i32 => {
            let index: i32 = 0;
            const found: i32 = loop {
                index += 1;
                if (index === limit) { break index; }
                continue;
            };
            return found;
        };
        const main = (): void => {
            console.log(search(3));
        };
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn loop_statement_form_has_void_type() {
    let source = r#"
        const main = (): void => {
            let counter: i32 = 0;
            loop {
                counter += 1;
                if (counter > 3) { break; }
            }
            console.log(counter);
        };
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn loop_without_break_value_cannot_be_used_as_a_value() {
    let source = r#"
        const main = (): void => {
            const result: i32 = loop { break; };
        };
    "#;
    assert_error(source, "E0308");
}

#[test]
fn break_with_a_value_is_rejected_outside_a_loop_expression() {
    for source in [
        "const f = (): i32 => { let i: i32 = 0; while (i < 3) { i += 1; break 1; } return i; };",
        "const f = (): i32 => { for (let i: i32 = 0; i < 3; i++) { break 1; } return 0; };",
        "const f = (): i32 => { switch (1) { case 1: break 2; default: break; } return 0; };",
    ] {
        assert_error(source, "E0571");
    }
}

#[test]
fn loop_break_values_must_agree_on_type() {
    let source = r#"
        const main = (): void => {
            const result = loop {
                if (true) { break 42; }
                break "text";
            };
            console.log(result);
        };
    "#;
    assert_error(source, "E0308");
}

#[test]
fn print_console_and_assert_cannot_be_redeclared() {
    for source in [
        "function print(label: string): void { console.log(label); }",
        "const printRaw = 1;",
        "function main(): void { let console: i32 = 5; }",
        "function assert(x: boolean): void {}",
        "const assertEqual = 1;",
        "function main(): void { const f = print => print + 1; }",
    ] {
        let tokens = crate::lexer::lex(source).unwrap();
        let error = Parser::new(tokens)
            .parse_program()
            .expect_err(&format!("expected a parse error for: {source}"));
        assert_eq!(error.code, Some("E0003"), "source: {source}");
    }
}

fn assert_no_diagnostic(source: &str, code: &str) {
    let diagnostics = diagnostics(source);
    assert!(
        !diagnostics.iter().any(|diagnostic| diagnostic.code == code),
        "expected no {code}, got {diagnostics:#?}"
    );
}

#[test]
fn accepts_a_loop_that_awaits_inside_its_body() {
    let source = r#"
        async function tick(): Promise<void> {}

        async function main(): Promise<void> {
            while (true) {
                await tick();
            }
        }
    "#;
    assert_no_diagnostic(source, "E3032");
}

#[test]
fn accepts_a_loop_that_can_break_out_of_itself() {
    let source = r#"
        async function main(): Promise<void> {
            let total: i32 = 0;
            while (true) {
                total += 1;
                if (total > 3) { break; }
            }
        }
    "#;
    assert_no_diagnostic(source, "E3032");
}

#[test]
fn accepts_a_do_while_that_returns_from_its_body() {
    let source = r#"
        async function main(): Promise<void> {
            let total: i32 = 0;
            do {
                total += 1;
                if (total > 3) { return; }
            } while (true);
        }
    "#;
    assert_no_diagnostic(source, "E3032");
}

#[test]
fn leaves_a_never_ending_loop_in_a_synchronous_function_alone() {
    let source = r#"
        function main(): void {
            let total: i32 = 0;
            while (true) {
                total += 1;
            }
        }
    "#;
    assert_no_diagnostic(source, "E3032");
}

#[test]
fn leaves_a_loop_with_a_real_condition_alone() {
    let source = r#"
        async function main(): Promise<void> {
            let total: i32 = 0;
            while (total < 10) {
                total += 1;
            }
        }
    "#;
    assert_no_diagnostic(source, "E3032");
}

#[test]
fn e0436_anchors_on_the_discarded_write() {
    let source = r#"function make(): () => i32 {
    let counter = 0;
    const f = (): i32 => {
        counter = counter + 1;
        return counter;
    };
    return f;
}
"#;
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let diagnostics = TypeChecker::new().check_program(&program);
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0436")
        .unwrap_or_else(|| panic!("expected E0436, got {diagnostics:#?}"));
    let span = diagnostic
        .span
        .expect("E0436 must publish a span for the LSP");
    assert_eq!(
        &source[span.start..span.end],
        "counter = counter + 1;",
        "the caret belongs on the write, not on a later `counter`"
    );
    assert_eq!(diagnostic.subject.as_deref(), Some("counter"));
    assert_eq!(diagnostic.help.len(), 2);
    assert!(diagnostic.help[1].contains("Mutex"));
}

#[test]
fn e0436_leaves_a_confined_closure_alone() {
    let source = r#"
        function main(): void {
            let counter = 0;
            const bump = (): i32 => {
                counter = counter + 1;
                return counter;
            };
            console.log(bump());
        }
    "#;
    let diagnostics = diagnostics(source);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E0436"),
        "confined closure reported: {diagnostics:#?}"
    );
}

#[test]
fn e0436_does_not_fire_on_a_read_only_capture() {
    let source = r#"
        function make(): () => i32 {
            let n = 5;
            const f = (): i32 => n;
            return f;
        }
    "#;
    let diagnostics = diagnostics(source);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E0436"),
        "read-only capture reported: {diagnostics:#?}"
    );
}

#[test]
fn e0436_ignores_a_local_of_the_closure() {
    let source = r#"
        function make(): () => i32 {
            const f = (): i32 => {
                let local = 0;
                local = local + 1;
                return local;
            };
            return f;
        }
    "#;
    let diagnostics = diagnostics(source);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E0436"),
        "closure local reported: {diagnostics:#?}"
    );
}

#[test]
fn infers_a_for_counter_from_the_loop_bound() {
    let source = r#"
        const nums = [10, 20, 30];
        const text = "tyrus";
        const limit: usize = 2;
        for (let i = 0; i < nums.length; i++) {
            print(nums[i]);
        }
        for (let i = 0; i < text.length; i++) {
            print(i);
        }
        for (let i = 0; i < nums.length - 1; i++) {
            print(nums[i]);
        }
        for (let i = 0; i < limit; i++) {
            print(i);
        }
        for (let i = 0; nums.length > i; i++) {
            print(i);
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn keeps_inferring_i32_for_a_counter_with_a_signed_bound() {
    let source = r#"
        const takes = (n: i32): void => { print(n); };
        const limit: i32 = 3;
        for (let i = 0; i < 3; i++) {
            const below: i32 = i - 5;
            takes(i);
            print(below);
        }
        for (let i = 0; i < limit; i++) {
            takes(i);
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn leaves_an_annotated_for_counter_alone() {
    assert_error(
        r#"
        const nums = [10, 20, 30];
        for (let i: i32 = 0; i < nums.length; i++) {
            print(i);
        }
        "#,
        "E0369",
    );
}

#[test]
fn does_not_infer_a_for_counter_from_an_unrecognised_bound() {
    assert_error(
        r#"
        const size = (): usize => { return 3; };
        const nums = [10, 20, 30];
        for (let i = 0; i < size(); i++) {
            print(nums[i]);
        }
        "#,
        "E0369",
    );
}

#[test]
fn rejects_nested_mutation_through_the_readonly_utility_type() {
    let source = r#"
        interface Inner {
            port: i32;
        }

        interface Cfg {
            inner: Inner;
        }

        type Frozen = Readonly<Cfg>;

        let config: Frozen = { inner: { port: 1 } };
        config.inner.port = 2;
    "#;
    assert_error(source, "E2541");
}

#[test]
fn rejects_surface_mutation_through_the_readonly_utility_type() {
    let source = r#"
        interface Cfg {
            host: string;
        }

        type Frozen = Readonly<Cfg>;

        let config: Frozen = { host: "localhost" };
        config.host = "127.0.0.1";
    "#;
    assert_error(source, "E2542");
}

#[test]
fn accepts_reading_through_a_readonly_const_binding() {
    let source = r#"
        readonly const DAYS: i32[] = [1, 2, 3];
        const first: i32 = DAYS[0];
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn rejects_a_mutating_method_on_a_readonly_const_binding() {
    let source = r#"
        readonly const DAYS: i32[] = [1, 2, 3];
        DAYS.push(4);
    "#;
    assert_error(source, "E2540");
}

#[test]
fn rejects_nested_mutation_through_a_readonly_const_binding() {
    let source = r#"
        interface Inner { port: i32; }
        interface Cfg { inner: Inner; }

        readonly const CONFIG: Cfg = { inner: { port: 1 } };
        CONFIG.inner.port = 2;
    "#;
    assert_error(source, "E2541");
}

#[test]
fn a_literal_union_accepts_the_operations_its_variants_do() {
    for source in [
        r#"
type Status = "active" | "inactive";
function ask(status: Status): boolean {
    return status === "active";
}
"#,
        r#"
type Status = "active" | "inactive";
function code(status: Status): i32 {
    match (status) {
        "active" => {
            return 1;
        }
        "inactive" => {
            return 2;
        }
    }
}
"#,
        r#"
type Status = "active" | "inactive";
function choose(flag: boolean): Status {
    const chosen: Status = flag ? "active" : "inactive";
    return chosen;
}
"#,
        r#"
type Status = "active" | "inactive";
function table(): Status[] {
    const rows: Status[] = ["active", "inactive"];
    return rows;
}
"#,
        r#"
type Status = "active" | "inactive";
function width(status: Status): usize {
    return status.length;
}
"#,
    ] {
        assert_eq!(
            spanned_diagnostics(source),
            Vec::<CheckDiagnostic>::new(),
            "expected no diagnostics for:{source}"
        );
    }
}

#[test]
fn a_literal_union_still_rejects_what_is_not_a_variant() {
    assert_error(
        r#"
type Status = "active" | "inactive";
function choose(flag: boolean): Status {
    const chosen: Status = flag ? "active" : "bogus";
    return chosen;
}
"#,
        "E0308",
    );
    assert_error(
        r#"
type Status = "active" | "inactive";
function table(): Status[] {
    const rows: Status[] = ["active", "bogus"];
    return rows;
}
"#,
        "E0308",
    );
    assert_error(
        r#"
type Status = "active" | "inactive" | "pending";
function code(status: Status): i32 {
    match (status) {
        "active" => {
            return 1;
        }
        "inactive" => {
            return 2;
        }
    }
}
"#,
        "E0004",
    );
}

#[test]
fn a_literal_key_on_a_struct_is_a_field_access() {
    assert_eq!(
        diagnostics(
            r#"
interface Person { name: string; age: i32; }
function greet(person: Person): string {
    return person["name"];
}
"#
        ),
        Vec::<CheckDiagnostic>::new()
    );
    assert_error(
        r#"
interface Person { name: string; }
function greet(person: Person, key: string): string {
    return person[key];
}
"#,
        "E0608",
    );
    assert_error(
        r#"
interface Person { name: string; }
function greet(person: Person): string {
    return person["nope"];
}
"#,
        "E0609",
    );
    assert_error(
        r#"
interface Person { age: i32; }
function birthday(): void {
    const person: Person = { age: 1 };
    person["age"] = 2;
}
"#,
        "E0384",
    );
}

#[test]
fn process_exec_accepts_a_call_without_its_optional_argument_list_as_result() {
    let source = r#"
        import { exec } from "tyrus:process";
        interface ChildProcess { pid: i32; }
        interface ProcessError { code: string; message: string; }
        const main = (): void => {
            const result: Result<ChildProcess, ProcessError> = exec("cat");
            match (result) {
                Ok(_child) => {}
                Err(error) => { console.error(error.code); }
            }
        };
    "#;
    let errors = process_diagnostics(source);
    assert!(
        errors.is_empty(),
        "unexpected process result diagnostic: {errors:#?}"
    );
}

#[test]
fn child_process_wait_and_kill_return_process_results() {
    let source = r#"
        interface ChildProcess { pid: i32; }
        interface ProcessError { code: string; message: string; }
        function waitChild(child: ChildProcess): Promise<Result<i32, ProcessError>> {
            return child.wait();
        }
        function killChild(child: ChildProcess): Promise<Result<void, ProcessError>> {
            return child.kill();
        }
    "#;
    let errors = process_diagnostics(source);
    assert!(
        errors.is_empty(),
        "unexpected child process result diagnostic: {errors:#?}"
    );
}

fn process_diagnostics(source: &str) -> Vec<CheckDiagnostic> {
    let tokens = crate::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let imports = [(
        "exec".to_string(),
        Type::Function {
            params: vec![
                Type::Str,
                Type::Optional(Box::new(Type::Array(Box::new(Type::Str)))),
            ],
            ret: Box::new(Type::Result {
                success: Box::new(Type::Named("ChildProcess".to_string())),
                error: Box::new(Type::Named("ProcessError".to_string())),
            }),
        },
    )]
    .into_iter()
    .collect();
    TypeChecker::new().check_program_with_imports(&program, &imports)
}

#[test]
fn an_outer_catch_that_names_the_effect_handles_a_synchronous_call() {
    let source = r#"
        interface ReadError { code: string; message: string; }
        interface WriteError { code: string; message: string; }

        function read(): i32 throws ReadError {
            return 1;
        }

        function write(): i32 throws WriteError {
            return 2;
        }

        const main = (): void => {
            try {
                try {
                    console.log(read());
                    console.log(write());
                } catch (error: ReadError) {
                    console.log(error.code);
                }
            } catch (error: WriteError) {
                console.log(error.code);
            }
        };
    "#;
    let unhandled: Vec<_> = diagnostics(source)
        .into_iter()
        .filter(|diagnostic| diagnostic.code == "E3011")
        .collect();
    assert!(
        unhandled.is_empty(),
        "the outer catch handles it: {unhandled:#?}"
    );
}

#[test]
fn an_awaited_rejection_reaches_the_outer_catch_that_names_it() {
    let source = r#"
        interface ReadError { code: string; message: string; }
        interface WriteError { code: string; message: string; }

        async function read(): Promise<i32> throws ReadError {
            return 1;
        }

        async function write(): Promise<i32> throws WriteError {
            return 2;
        }

        async function main(): Promise<void> {
            try {
                try {
                    console.log(await read());
                    console.log(await write());
                } catch (error: ReadError) {
                    console.log(error.code);
                }
            } catch (error: WriteError) {
                console.log(error.code);
            }
        }
    "#;
    let unhandled: Vec<_> = diagnostics(source)
        .into_iter()
        .filter(|diagnostic| diagnostic.code == "E3011")
        .collect();
    assert!(unhandled.is_empty(), "unexpected E3011: {unhandled:#?}");
}

#[test]
fn a_try_around_a_closure_declaration_does_not_handle_the_closure_body() {
    let source = r#"
        function risky(): i32 throws string {
            throw "boom";
        }

        const main = (): void => {
            try {
                const run = (): void => {
                    console.log(risky());
                };
                console.log("declared");
            } catch (error: string) {
                console.log(error);
            }
        };
    "#;
    assert_error(source, "E3011");
}

#[test]
fn a_union_of_interfaces_exposes_the_fields_they_share() {
    let source = r#"
        interface ReadError { code: string; message: string; }
        interface WriteError { code: string; message: string; }

        function describe(error: ReadError | WriteError): string {
            return error.code + error.message;
        }

        const main = (): void => {
            console.log("ok");
        };
    "#;
    let missing: Vec<_> = diagnostics(source)
        .into_iter()
        .filter(|diagnostic| diagnostic.code == "E0609")
        .collect();
    assert!(missing.is_empty(), "unexpected E0609: {missing:#?}");
}

#[test]
fn a_throw_an_outer_catch_accepts_is_handled_by_it() {
    let source = r#"
        interface ReadError { code: string; message: string; }
        interface WriteError { code: string; message: string; }

        const main = (): void => {
            try {
                try {
                    const failure: WriteError = { code: "ERR_WRITE", message: "cannot write" };
                    throw failure;
                } catch (error: ReadError) {
                    console.log(error.code);
                }
            } catch (error: WriteError) {
                console.log(error.code);
            }
        };
    "#;
    let mismatches: Vec<_> = diagnostics(source)
        .into_iter()
        .filter(|diagnostic| matches!(diagnostic.code, "E0308" | "E3011"))
        .collect();
    assert!(
        mismatches.is_empty(),
        "the outer catch accepts it: {mismatches:#?}"
    );
}

#[test]
fn a_throw_no_catch_accepts_is_refused() {
    let source = r#"
        interface ReadError { code: string; message: string; }
        interface WriteError { code: string; message: string; }

        const main = (): void => {
            try {
                const failure: WriteError = { code: "ERR_WRITE", message: "cannot write" };
                throw failure;
            } catch (error: ReadError) {
                console.log(error.code);
            }
        };
    "#;
    let mismatches: Vec<_> = diagnostics(source)
        .into_iter()
        .filter(|diagnostic| diagnostic.code == "E0308")
        .collect();
    assert!(!mismatches.is_empty(), "expected the throw to be refused");
}

const WEBSOCKET_PRELUDE: &str = "interface WebSocket {}\ninterface WebSocketMessage { binary: boolean; text: string; bytes: u8[]; }\ninterface HttpError { code: string; message: string; }\n";

#[test]
fn a_websocket_route_handler_must_take_the_socket_and_return_a_void_promise() {
    let source = format!(
        "{WEBSOCKET_PRELUDE}function wrong(value: i32): void {{}}\nconst app: HttpRouter = __tyrus_http_createRouter();\napp.ws(\"/chat\", {{ handler: wrong }});"
    );
    assert_error(&source, "E0308");
}

#[test]
fn a_websocket_route_without_a_handler_is_refused() {
    let source = format!(
        "{WEBSOCKET_PRELUDE}const app: HttpRouter = __tyrus_http_createRouter();\napp.ws(\"/chat\", {{ origins: [\"https://example.com\"] }});"
    );
    assert_error(&source, "E2741");
}

#[test]
fn a_websocket_route_option_it_does_not_know_is_refused() {
    let source = format!(
        "{WEBSOCKET_PRELUDE}async function chat(socket: WebSocket): Promise<void> {{}}\nconst app: HttpRouter = __tyrus_http_createRouter();\napp.ws(\"/chat\", {{ handler: chat, body: 1 }});"
    );
    assert_error(&source, "E0609");
}

#[test]
fn a_websocket_route_is_refused_without_the_socket_type_in_scope() {
    let source = "async function chat(socket: i32): Promise<void> {}\nconst app: HttpRouter = __tyrus_http_createRouter();\napp.ws(\"/chat\", { handler: chat });";
    assert_error(source, "E0412");
}

#[test]
fn websocket_receive_returns_result_without_throw_effects() {
    let source = format!(
        "{WEBSOCKET_PRELUDE}async function chat(socket: WebSocket): Promise<void> {{\n    const message: Result<WebSocketMessage?, HttpError> = await socket.receive();\n}}"
    );
    let found = diagnostics(&source);
    assert!(
        found.iter().all(|diagnostic| diagnostic.code != "E3011"),
        "receive should return Result instead of requiring throws: {found:#?}"
    );
}

#[test]
fn websocket_send_and_close_return_results_without_throw_effects() {
    let source = format!(
        "{WEBSOCKET_PRELUDE}async function chat(socket: WebSocket): Promise<void> {{\n    const stream: Result<DuplexStream<u8[], u8[]>, HttpError> = await socket.stream();\n    const sent: Result<void, HttpError> = await socket.sendText(\"hello\");\n    const closed: Result<void, HttpError> = await socket.close();\n}}"
    );
    let found = diagnostics(&source);
    assert!(
        found.iter().all(|diagnostic| diagnostic.code != "E3011"),
        "send and close should return Result instead of requiring throws: {found:#?}"
    );
}

#[test]
fn a_websocket_method_that_does_not_exist_is_refused() {
    let source = format!(
        "{WEBSOCKET_PRELUDE}async function chat(socket: WebSocket): Promise<void> {{\n    socket.broadcast(\"hello\");\n}}"
    );
    assert_error(&source, "E0599");
}

#[test]
fn a_well_formed_websocket_route_and_handler_are_accepted() {
    let source = format!(
        "{WEBSOCKET_PRELUDE}async function chat(socket: WebSocket): Promise<void> throws HttpError {{\n    const message: Result<WebSocketMessage?, HttpError> = await socket.receive();\n    const closeCode: i32? = socket.closeCode();\n    const sent: Result<void, HttpError> = await socket.sendText(socket.protocol());\n    const closed: Result<void, HttpError> = await socket.close();\n}}\nconst app: HttpRouter = __tyrus_http_createRouter();\napp.ws(\"/chat\", {{ handler: chat, origins: [\"https://example.com\"], subprotocols: [\"chat\"], maxMessageBytes: 65536 }});"
    );
    let found = diagnostics(&source);
    assert!(
        found.iter().all(|diagnostic| {
            (diagnostic.code == "E0412" && diagnostic.message.contains("HttpRouter"))
                || !matches!(
                    diagnostic.code,
                    "E0308" | "E0412" | "E0599" | "E0609" | "E2741" | "E3011"
                )
        }),
        "unexpected diagnostics: {found:#?}"
    );
}

#[test]
fn a_websocket_route_config_accepts_only_a_rate_limit() {
    let source = format!(
        "{WEBSOCKET_PRELUDE}async function chat(socket: WebSocket): Promise<void> {{}}\nconst app: HttpRouter = __tyrus_http_createRouter();\napp.ws(\"/chat\", {{ handler: chat, config: {{ bodyLimit: \"1mb\" }} }});"
    );
    let found = diagnostics(&source);
    assert!(
        found.iter().any(
            |diagnostic| diagnostic.code == "E0609" && diagnostic.message.contains("bodyLimit")
        ),
        "expected the unknown option to be refused: {found:#?}"
    );
}

#[test]
fn docs_options_are_checked_and_an_unknown_one_is_refused() {
    let accepted = "const app: HttpRouter = __tyrus_http_createRouter();\napp.docs({ path: \"/reference\", title: \"Users API\", version: \"2.0.0\", description: \"Users\" });\napp.docs();";
    let found = diagnostics(accepted);
    assert!(
        found
            .iter()
            .all(|diagnostic| !matches!(diagnostic.code, "E0308" | "E0609" | "E0061")),
        "unexpected diagnostics: {found:#?}"
    );
    assert_error(
        "const app: HttpRouter = __tyrus_http_createRouter();\napp.docs({ roles: [] });",
        "E0609",
    );
    assert_error(
        "const app: HttpRouter = __tyrus_http_createRouter();\napp.docs({ title: 1 });",
        "E0308",
    );
}

#[test]
fn fallback_registration_returns_result_without_corrupting_route_types() {
    let source = r#"
        interface HttpReply<T> {}
        interface HttpRouter {}
        interface HttpError {}
        interface HealthResponse { ok: boolean; }
        enum HttpStatus { Ok = 200 }
        const app: HttpRouter = __tyrus_http_createRouter();
        const routeResult: Result<void, HttpError> = app.fallback({});
        const route: Result<void, HttpError> = app.get("/health", {
            response: { 200: HealthResponse },
            handler: (request, reply): void => {}
        });
    "#;
    let found = diagnostics(source);
    assert!(
        found.iter().all(|diagnostic| diagnostic.code != "E0500"),
        "fallback registration must not corrupt the following route's response types: {found:#?}"
    );
}

#[test]
fn fallback_registration_checks_its_argument_expression() {
    let source = "interface HttpRouter {}\nconst app: HttpRouter = __tyrus_http_createRouter();\napp.fallback(missingHandler);";
    assert_error(source, "E0425");
}

#[test]
fn docs_auth_is_a_literal_naming_its_schemes() {
    let accepted = "const app: HttpRouter = __tyrus_http_createRouter();\napp.docs({ auth: { bearer: true, bearerFormat: \"JWT\", apiKey: { name: \"x-api-key\", in: \"header\" }, oauth2: { authorizationUrl: \"https://auth.example.com/authorize\", tokenUrl: \"https://auth.example.com/token\", scopes: [\"read\"] } }, css: \".topbar { display: none; }\" });";
    let found = diagnostics(accepted);
    assert!(
        found
            .iter()
            .all(|diagnostic| !matches!(diagnostic.code, "E0308" | "E0609" | "E2741")),
        "unexpected diagnostics: {found:#?}"
    );
    let router = "const app: HttpRouter = __tyrus_http_createRouter();\n";
    assert_error(
        &format!("{router}app.docs({{ auth: {{ bearer: true, basic: true }} }});"),
        "E0609",
    );
    assert_error(
        &format!(
            "{router}app.docs({{ auth: {{ bearerFormat: \"JWT\", apiKey: {{ name: \"k\", in: \"header\" }} }} }});"
        ),
        "E0308",
    );
    assert_error(
        &format!("{router}app.docs({{ auth: {{ apiKey: {{ name: \"k\", in: \"body\" }} }} }});"),
        "E0308",
    );
    assert_error(
        &format!(
            "{router}app.docs({{ auth: {{ oauth2: {{ authorizationUrl: \"https://a\" }} }} }});"
        ),
        "E2741",
    );
    assert_error(&format!("{router}app.docs({{ auth: {{}} }});"), "E0308");
    assert_error(&format!("{router}app.docs({{ css: 1 }});"), "E0308");
}

#[test]
fn a_catch_without_an_annotation_handles_an_error_and_reads_its_code() {
    let source = r#"
        function risky(): void throws Error {
            const failure: Error = { message: "boom", code: "ERR_BOOM" };
            throw failure;
        }
        function run(): string {
            try {
                risky();
            } catch (error) {
                return error.code + ": " + error.message;
            }
            return "";
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn a_catch_without_an_annotation_does_not_handle_a_thrown_string() {
    let source = r#"
        function risky(): void throws string {
            throw "boom";
        }
        function run(): void {
            try {
                risky();
            } catch (error) {
                console.log(error.message);
            }
        }
    "#;
    assert_error(source, "E3011");
}

#[test]
fn a_catch_without_an_annotation_handles_a_native_module_error() {
    let source = r#"
        interface FsError { code: string; message: string; }
        function readFileSync(path: string): string throws FsError {
            return path;
        }
        function run(): string {
            try {
                return readFileSync("config.json");
            } catch (error) {
                return error.code;
            }
        }
    "#;
    let unhandled: Vec<_> = diagnostics(source)
        .into_iter()
        .filter(|diagnostic| diagnostic.code == "E3011")
        .collect();
    assert!(unhandled.is_empty(), "{unhandled:#?}");
}

#[test]
fn an_interface_that_extends_error_is_handled_by_a_catch_without_an_annotation() {
    let source = r#"
        interface WriteError extends Error { path: string; }
        function write(): void throws WriteError {
            const failure: WriteError = { message: "disk is full", code: "ERR_WRITE", path: "a.txt" };
            throw failure;
        }
        function run(): string {
            try {
                write();
            } catch (error) {
                return error.code;
            }
            return "";
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn is_narrows_a_caught_error_to_its_subtype() {
    let source = r#"
        interface WriteError extends Error { path: string; }
        function write(): void throws WriteError {
            const failure: WriteError = { message: "disk is full", code: "ERR_WRITE", path: "a.txt" };
            throw failure;
        }
        function run(): string {
            try {
                write();
            } catch (error) {
                if (error is WriteError) {
                    return error.path;
                }
                return error.code;
            }
            return "";
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
fn is_refuses_a_type_an_error_can_never_be() {
    let source = r#"
        interface Point { x: i32; y: i32; }
        function run(): boolean {
            try {
                return false;
            } catch (error) {
                return error is Point;
            }
        }
    "#;
    assert_error(source, "E0308");
}

#[test]
fn is_on_an_error_that_did_not_come_from_a_catch_is_refused() {
    let source = r#"
        interface WriteError extends Error { path: string; }
        function describe(error: Error): boolean {
            return error is WriteError;
        }
    "#;
    assert_error(source, "E3297");
}

#[test]
fn a_throwing_function_is_refused_where_the_function_type_does_not_declare_it() {
    let source = r#"
        function failText(): void throws string {
            throw "boom";
        }
        const run = (action: () => void): void => {
            action();
        };
        const main = (): void => {
            run(failText);
        };
    "#;
    assert_error(source, "E3298");
}

#[test]
fn a_function_type_that_declares_throws_requires_handling_its_call() {
    let handled = r#"
        function failText(): void throws string {
            throw "boom";
        }
        const run = (action: () => void throws string): void => {
            try {
                action();
            } catch (error: string) {
                console.log(error);
            }
        };
        const main = (): void => {
            run(failText);
        };
    "#;
    assert_eq!(diagnostics(handled), Vec::<CheckDiagnostic>::new());
    let unhandled = r#"
        const run = (action: () => void throws string): void => {
            action();
        };
    "#;
    assert_error(unhandled, "E3011");
}

#[test]
fn checks_custom_validator_signatures_and_applications() {
    let valid = r#"
        type Validator = { isValid: true } | { isValid: false; message: string };
        validator function usernameWithoutSpaces(value: string): Validator {
            if (value.includes(" ")) {
                return { isValid: false, message: "Username cannot contain spaces" };
            }
            return { isValid: true };
        }
        validator function min(value: u32, constraint: u32): Validator {
            if (value < constraint) {
                return { isValid: false, message: "Value is below the constraint" };
            }
            return { isValid: true };
        }
        validator const startsWith = (value: string, constraint: string): Validator => {
            if (!value.startsWith(constraint)) {
                return { isValid: false, message: "Value has the wrong prefix" };
            }
            return { isValid: true };
        };
        interface Account {
            @usernameWithoutSpaces()
            username: string;
            @min(5)
            age: u32;
            @startsWith("user_")
            handle: string;
        }
        function register(@usernameWithoutSpaces() value: string): void {}
        const registerArrow = (@startsWith("user_") value: string): void => {};
    "#;
    assert_eq!(diagnostics(valid), Vec::<CheckDiagnostic>::new());
    assert_error(
        "type Validator = { isValid: true } | { isValid: false; message: string }; validator function invalid(): Validator { return { isValid: true }; }",
        "E3168",
    );
    assert_error(
        "type Validator = { isValid: true } | { isValid: false; message: string }; validator function invalid(value): Validator { return { isValid: true }; }",
        "E3168",
    );
    assert_error(
        "validator function invalid(value: string): boolean { return true; }",
        "E3168",
    );
    assert_error(
        "type Validator = { isValid: true } | { isValid: false; message: string }; validator function invalid(value: string, constraint): Validator { return { isValid: true }; }",
        "E3168",
    );
    assert_error(
        "type Validator = { isValid: true } | { isValid: false; message: string }; validator function username(value: string): Validator { return { isValid: true }; } interface Account { @username username: i32; }",
        "E3169",
    );
    assert_error(
        "type Validator = { isValid: true } | { isValid: false; message: string }; validator function min(value: u32, constraint: u32): Validator { return { isValid: true }; } interface Account { @min(\"five\") age: u32; }",
        "E3169",
    );
    assert_error(
        "type Validator = { isValid: true } | { isValid: false; message: string }; validator function min(value: u32, constraint: u32): Validator { return { isValid: true }; } interface Account { @min(5, 6) age: u32; }",
        "E3169",
    );
    assert_error(
        "function ordinary(value: string): boolean { return true; } interface Account { @ordinary username: string; }",
        "E3160",
    );
    assert_eq!(
        diagnostics(
            "type Validator = { isValid: true } | { isValid: false; message: string }; validator function username(value: string): Validator { return { isValid: true }; } function register(@username() value: string): void {}"
        ),
        Vec::<CheckDiagnostic>::new()
    );
}
