use super::*;
use crate::ast::StmtKind;
use crate::ast::{DeferKind, ExprKind, OnFailHandler};
use crate::parser::ParseError;
use logos::Logos;

fn parse_error_spanned(source: &str) -> ParseError {
    let spanned = Token::lexer(source)
        .spanned()
        .filter_map(|(token, range)| token.ok().map(|token| (token, range)))
        .collect::<Vec<_>>();
    let tokens = spanned.iter().map(|(token, _)| token.clone()).collect();
    let ranges = spanned.iter().map(|(_, range)| range.clone()).collect();
    Parser::new_with_ranges(tokens, ranges)
        .parse_program()
        .unwrap_err()
}

fn parse(source: &str) -> Program {
    let tokens = crate::lexer::lex(source).unwrap();
    Parser::new(tokens).parse_program().unwrap()
}

#[test]
fn type_alias_intersection_desugars_to_an_interface_with_operands() {
    let program = parse("interface B { id: i32; }\ninterface C { name: string; }\ntype A = B & C;");
    let Item::InterfaceDecl {
        name,
        extends,
        fields,
        from_type_alias,
        ..
    } = &program.items[2]
    else {
        panic!("expected the alias to desugar into an interface declaration");
    };
    assert_eq!(name, "A");
    assert!(*from_type_alias);
    assert!(fields.is_empty());
    assert_eq!(
        extends,
        &vec![Type::Named("B".into()), Type::Named("C".into())]
    );
}

#[test]
fn type_alias_intersection_keeps_inline_objects_in_written_order() {
    let program = parse("interface B { id: i32; }\ntype A = { name: string; } & B;");
    let Item::InterfaceDecl { extends, .. } = &program.items[1] else {
        panic!("expected an interface declaration");
    };
    assert_eq!(extends.len(), 2);
    assert!(matches!(&extends[0], Type::Object(fields) if fields[0].name == "name"));
    assert_eq!(extends[1], Type::Named("B".into()));
}

#[test]
fn type_alias_intersection_chains_more_than_two_operands() {
    let program = parse("type A = { name: string; } & B & C;");
    let Item::InterfaceDecl { extends, .. } = &program.items[0] else {
        panic!("expected an interface declaration");
    };
    assert_eq!(extends.len(), 3);
}

#[test]
fn a_leading_ampersand_still_parses_as_a_reference_type() {
    let program = parse("type A = &i32;");
    let Item::TypeAlias { ty, .. } = &program.items[0] else {
        panic!("expected a plain type alias");
    };
    assert_eq!(ty, &Type::Ref(Box::new(Type::I32)));
}

#[test]
fn an_intersection_in_an_annotation_is_rejected_with_its_own_code() {
    let mut parser = Parser::from_source("function show(v: B & C): void {}").unwrap();
    let error = parser.parse_program().expect_err("expected a parse error");
    assert_eq!(error.code, Some("E3215"));
    assert!(error.span.is_some(), "the operator must carry a span");
    assert!(!error.help.is_empty());
}

#[test]
fn mixing_an_intersection_with_a_union_is_rejected_with_its_own_code() {
    for source in [
        "type A = B & C | D;",
        "type A = B | C & D;",
        "type A = B & (C | D);",
        "type A = (B & C) | D;",
    ] {
        let mut parser = Parser::from_source(source).unwrap();
        let error = parser
            .parse_program()
            .expect_err("mixing `&` and `|` must be rejected");
        assert_eq!(error.code, Some("E3217"), "{source}");
        assert!(
            error.span.is_some(),
            "`{source}` must anchor on the operator"
        );
        assert_eq!(error.help.len(), 2, "{source}");
        assert!(
            error.help[0].contains("own name"),
            "the actionable fix comes first: {:?}",
            error.help
        );
    }
}

#[test]
fn a_named_intersection_is_allowed_inside_a_union() {
    let program = parse("type BC = B & C;\ntype A = BC | D;");
    let Item::TypeAlias { ty, .. } = &program.items[1] else {
        panic!("expected a union alias");
    };
    assert_eq!(
        ty,
        &Type::Union(vec![Type::Named("BC".into()), Type::Named("D".into())])
    );
}

#[test]
fn a_declaration_without_a_semicolon_is_rejected() {
    for (source, what) in [
        ("type A = B", "type alias"),
        ("const X: i32 = 1", "declaration"),
        ("let Y: i32 = 1", "declaration"),
        ("const main = (): void => { print(1); }", "declaration"),
        ("const f = function (): void {}", "declaration"),
        (
            "const main = (): void => { const x: i32 = 1 };",
            "declaration",
        ),
        ("const main = (): void => { print(1) };", "statement"),
        ("const main = (): i32 => { return 1 };", "return"),
        (
            "const main = (): void => { let x: i32 = 0; x = 1 };",
            "assignment",
        ),
        (
            "const main = (): void => { let xs: i32[] = [1]; xs[0] = 2 };",
            "assignment",
        ),
        (
            "const main = (): void => { const [a, b] = [1, 2] };",
            "destructuring",
        ),
        (
            "const main = (): void => { while (true) { break } };",
            "break",
        ),
        (
            "const main = (): void => { while (true) { continue } };",
            "continue",
        ),
        (
            "const main = (): void => { defer print(1) print(2); };",
            "deferred statement",
        ),
        ("import { a } from \"./m\"", "import"),
    ] {
        let mut parser = Parser::from_source(source).unwrap();
        let error = parser
            .parse_program()
            .expect_err("a missing `;` must be rejected");
        assert_eq!(error.code, Some("E3218"), "{source}");
        assert!(
            error.message.contains(what),
            "`{source}` must name the construct: {}",
            error.message
        );
        assert!(!error.help.is_empty(), "{source}");
    }
}

/// Everything that closes with its own `}`, plus the two clauses a delimiter
/// closes: a `do ... while (cond)` and the update of a classic `for`.
#[test]
fn a_construct_that_closes_itself_needs_no_semicolon() {
    for source in [
        "const main = (): void => { if (true) { print(1); } print(2); };",
        "const main = (): void => { if (true) { print(1); } else { print(2); } print(3); };",
        "const main = (): void => { while (false) { print(1); } print(2); };",
        "const main = (): void => { let i: i32 = 0; do { i += 1; } while (i < 2) print(i); };",
        "const main = (): void => { for (let i = 0; i < 2; i++) { print(i); } print(9); };",
        "const main = (): void => { const xs = [1]; for (const x of xs) { print(x); } print(9); };",
        "interface O { a: i32; }\nconst main = (): void => { const o: O = { a: 1 }; for (const k in o) { print(k); } print(9); };",
        "const main = (): void => { const n: i32 = 1; match (n) { 1 => { print(1); } _ => { print(0); } } print(9); };",
        "const main = (): void => { try { print(1); } catch (e) { print(2); } print(3); };",
        "const main = (): void => { const n: i32 = 1; switch (n) { case 1: { print(1); } default: { print(0); } } print(9); };",
        "const main = (): void => { let i: i32 = 0; loop { i += 1; if (i > 1) { break; } } print(i); };",
        "const main = (): void => { defer { print(1); } print(2); };",
        "interface A { id: i32 }",
        "function f(): i32 { return 1; }",
        "enum Color { Red, Green }",
        "interface A { id: i32; }\ninterface AM at A { twice(a: A): i32 { return a.id * 2; } }",
    ] {
        let mut parser = Parser::from_source(source).unwrap();
        assert!(
            parser.parse_program().is_ok(),
            "`{source}` must parse without a trailing `;`"
        );
    }
}

#[test]
fn omitted_annotation_parses_as_infer() {
    let program = parse("const count = 42;");
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { ty, value, .. },
        ..
    }) = &program.items[0]
    else {
        panic!("expected variable declaration");
    };
    assert!(matches!(ty, Type::Infer));
    assert!(matches!(
        value,
        Some(Expr {
            kind: ExprKind::IntLit(42),
            ..
        })
    ));
}

#[test]
fn model_timestamps_are_synthetic_by_default_and_respect_model_options() {
    let default = parse("@model(\"users\") interface User { id: i64; }");
    let Item::InterfaceDecl {
        fields,
        db_model: Some(model),
        ..
    } = &default.items[0]
    else {
        panic!("expected a database model");
    };
    assert!(model.timestamps);
    assert_eq!(
        fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["id", "createdAt", "updatedAt"]
    );
    assert_eq!(
        model
            .fields
            .iter()
            .map(|field| field.column.as_str())
            .collect::<Vec<_>>(),
        ["id", "createdAt", "updatedAt"]
    );
    assert!(matches!(fields[1].ty, Type::Named(ref name) if name == "Instant"));

    let snake_case = parse("@model(\"users\", { snakeCase: true }) interface User { id: i64; }");
    let Item::InterfaceDecl {
        db_model: Some(model),
        ..
    } = &snake_case.items[0]
    else {
        panic!("expected a database model");
    };
    assert_eq!(model.fields[1].column, "created_at");
    assert_eq!(model.fields[2].column, "updated_at");

    let disabled = parse("@model(\"users\", { timestamps: false }) interface User { id: i64; }");
    let Item::InterfaceDecl {
        fields,
        db_model: Some(model),
        ..
    } = &disabled.items[0]
    else {
        panic!("expected a database model");
    };
    assert!(!model.timestamps);
    assert_eq!(fields.len(), 1);
    assert_eq!(model.fields.len(), 1);
}

#[test]
fn parses_special_floating_point_literals_and_unary_negative_infinity() {
    let program =
        parse("const positive = Infinity; const negative = -Infinity; const invalid = NaN;");
    let values = program
        .items
        .iter()
        .map(|item| match item {
            Item::Stmt(Stmt {
                kind:
                    StmtKind::VarDecl {
                        value: Some(value), ..
                    },
                ..
            }) => value,
            _ => panic!("expected variable declaration"),
        })
        .collect::<Vec<_>>();
    assert!(
        matches!(values[0].kind(), ExprKind::FloatLit(value) if value.value() == f64::INFINITY)
    );
    assert!(
        matches!(values[1].kind(), ExprKind::Neg(value) if matches!(value.kind(), ExprKind::FloatLit(inner) if inner.value() == f64::INFINITY))
    );
    assert!(matches!(values[2].kind(), ExprKind::FloatLit(value) if value.value().is_nan()));
}

#[test]
fn parses_defer_expression_and_block_forms() {
    let program = parse("defer cleanup(); defer { first(); second(); }");
    let Item::Stmt(Stmt {
        kind:
            StmtKind::Defer {
                kind: DeferKind::Always,
                body,
                ..
            },
        ..
    }) = &program.items[0]
    else {
        panic!("expected expression-form defer");
    };
    assert_eq!(body.len(), 1);

    let Item::Stmt(Stmt {
        kind:
            StmtKind::Defer {
                kind: DeferKind::Always,
                body,
                ..
            },
        ..
    }) = &program.items[1]
    else {
        panic!("expected block-form defer");
    };
    assert_eq!(body.len(), 2);
}

#[test]
fn parses_defer_inside_a_function_body() {
    parse(
        r#"
function value(): i32 {
    defer cleanup();
    defer { first(); second(); }
    return 7;
}
"#,
    );
}

#[test]
fn parses_multiple_defer_forms_inside_a_function_body() {
    parse(
        r#"
function value(): i32 {
    defer console.log("outer-a");
    defer { console.log("outer-b"); }
    if (true) {
        defer console.log("inner");
        console.log("body");
    }
    return 7;
}
"#,
    );
}

#[test]
fn parses_errdefer_expression_and_block_forms() {
    let program = parse("errdefer rollback(); errdefer { first(); second(); }");
    for (index, expected_len) in [(0, 1), (1, 2)] {
        let Item::Stmt(Stmt {
            kind:
                StmtKind::Defer {
                    kind: DeferKind::OnError,
                    body,
                    ..
                },
            ..
        }) = &program.items[index]
        else {
            panic!("expected errdefer");
        };
        assert_eq!(body.len(), expected_len);
    }
}

#[test]
fn defer_spans_distinguish_the_keyword_body_and_statement() {
    let source = "defer { cleanup(); }\nerrdefer rollback();\n";
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let Item::Stmt(Stmt {
        kind: StmtKind::Defer {
            keyword_span, body, ..
        },
        span,
    }) = &program.items[0]
    else {
        panic!("expected block-form defer");
    };
    assert_eq!(&source[keyword_span.to_range()], "defer");
    assert_eq!(&source[body[0].span.to_range()], "cleanup();");
    assert_eq!(&source[span.to_range()], "defer { cleanup(); }");

    let Item::Stmt(Stmt {
        kind: StmtKind::Defer { keyword_span, .. },
        ..
    }) = &program.items[1]
    else {
        panic!("expected expression-form errdefer");
    };
    assert_eq!(&source[keyword_span.to_range()], "errdefer");
}

#[test]
fn rejects_defer_without_an_expression_or_block() {
    let message = parse_error("defer;");
    assert!(message.contains("Expected an expression"), "{message}");
}

#[test]
fn parses_typed_numeric_literals_without_losing_their_type() {
    let program = parse("const a = 10u8; const b = 3.14f64; const c = -5i32;");
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { value: Some(a), .. },
        ..
    }) = &program.items[0]
    else {
        panic!("expected first declaration");
    };
    assert!(matches!(
        a.kind(),
        ExprKind::TypeAssert { ty: Type::U8, .. }
    ));
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { value: Some(b), .. },
        ..
    }) = &program.items[1]
    else {
        panic!("expected second declaration");
    };
    assert!(matches!(
        b.kind(),
        ExprKind::TypeAssert { ty: Type::F64, .. }
    ));
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { value: Some(c), .. },
        ..
    }) = &program.items[2]
    else {
        panic!("expected third declaration");
    };
    assert!(
        matches!(c.kind(), ExprKind::Neg(inner) if matches!(inner.kind(), ExprKind::TypeAssert { ty: Type::I32, .. }))
    );
}

#[test]
fn parses_decimal_as_a_primitive_type() {
    let program = parse("const identity = (value: decimal): decimal => value;");
    let Item::FnDecl {
        params, ret_type, ..
    } = &program.items[0]
    else {
        panic!("expected function declaration");
    };
    assert_eq!(params[0].ty, Type::Decimal);
    assert_eq!(*ret_type, Type::Decimal);
}

#[test]
fn parses_decimal_literals_without_a_float_approximation() {
    let program = parse("const a = 0.1d; const b = 12D; const c = 1_000.25_0e-2d;");
    for item in &program.items {
        let Item::Stmt(Stmt {
            kind: StmtKind::VarDecl {
                value: Some(value), ..
            },
            ..
        }) = item
        else {
            panic!("expected decimal declaration");
        };
        assert!(matches!(value.kind(), ExprKind::DecimalLit(_)));
    }
}

#[test]
fn decimal_arithmetic_keeps_operator_precedence_and_parentheses() {
    let program = parse("const first = 1d + 2d * 3d; const second = (1d + 2d) * 3d;");
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl {
            value: Some(first), ..
        },
        ..
    }) = &program.items[0]
    else {
        panic!("expected first decimal declaration");
    };
    assert!(matches!(
        first.kind(),
        ExprKind::BinOp { op: BinOp::Add, right, .. }
            if matches!(right.kind(), ExprKind::BinOp { op: BinOp::Mul, .. })
    ));
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl {
            value: Some(second),
            ..
        },
        ..
    }) = &program.items[1]
    else {
        panic!("expected second decimal declaration");
    };
    assert!(matches!(
        second.kind(),
        ExprKind::BinOp { op: BinOp::Mul, left, .. }
            if matches!(left.kind(), ExprKind::BinOp { op: BinOp::Add, .. })
    ));
}

#[test]
fn parses_behavior_interfaces_and_extension_methods() {
    let program = parse(
        "interface User { age: u32; }\n\
         interface UserMethods at User {\n\
             isAdult(user: User): boolean { return user.age >= 18; }\n\
         }",
    );
    let Item::BehaviorInterfaceDecl {
        name,
        targets,
        methods,
        ..
    } = &program.items[1]
    else {
        panic!("expected behavior interface declaration");
    };
    assert_eq!(name, "UserMethods");
    assert_eq!(targets, &[Type::Named("User".to_string())]);
    assert_eq!(methods.len(), 1);
    assert_eq!(methods[0].name, "isAdult");
    assert_eq!(methods[0].params.len(), 1);
}

#[test]
fn parses_behavior_interface_shorthand_bindings() {
    let program = parse("interface NumberMethods at i32 { isEven; }");
    let Item::BehaviorInterfaceDecl {
        bindings, methods, ..
    } = &program.items[0]
    else {
        panic!("expected behavior interface declaration");
    };
    assert_eq!(bindings, &["isEven"]);
    assert!(methods.is_empty());
}

#[test]
fn explicit_annotation_is_preserved() {
    let program = parse("const count: i64 = 42;");
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { ty, .. },
        ..
    }) = &program.items[0]
    else {
        panic!("expected variable declaration");
    };
    assert!(matches!(ty, Type::I64));
}

#[test]
fn parses_regular_expression_literals() {
    let program = parse("const slugPattern: Regex = /^[a-z0-9-]+$/i;");
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { ty, value, .. },
        ..
    }) = &program.items[0]
    else {
        panic!("expected variable declaration");
    };
    assert!(matches!(ty, Type::Regex));
    assert!(matches!(
        value,
        Some(Expr { kind: ExprKind::RegexLit { pattern, flags }, .. })
            if pattern == "^[a-z0-9-]+$" && flags == "i"
    ));
}

#[test]
fn deferred_let_without_annotation_is_rejected() {
    let tokens = Token::lexer("let x;").filter_map(Result::ok).collect();
    let error = Parser::new(tokens).parse_program().unwrap_err();
    assert!(
        error.message.contains("type annotation required"),
        "got: {error}"
    );
}

#[test]
fn boolean_is_the_primitive_keyword() {
    let program = parse("const flag: boolean = true;");
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { ty, .. },
        ..
    }) = &program.items[0]
    else {
        panic!("expected variable declaration");
    };
    assert!(matches!(ty, Type::Bool));
}

#[test]
fn rejects_null_as_a_type() {
    let tokens = Token::lexer("const x: null = null;")
        .filter_map(Result::ok)
        .collect();
    let error = Parser::new(tokens).parse_program().unwrap_err();
    assert!(
        error.message.contains("`null` is not a type"),
        "got: {error}"
    );
    assert!(error.message.contains("T?"), "got: {error}");

    let tokens = Token::lexer("type MaybeName = string | null;")
        .filter_map(Result::ok)
        .collect();
    let error = Parser::new(tokens).parse_program().unwrap_err();
    assert!(
        error.message.contains("`null` is not a type"),
        "got: {error}"
    );
}

#[test]
fn parses_validator_decorators_on_fields_and_parameters() {
    let program = parse(
        r#"
            interface UserInput {
                @minLength(3, "too short")
                @isTrimmed()
                name: string;
            }

            const divide = (a: f64, @notEqual(0) b: f64): f64 => a / b;
        "#,
    );
    let Item::InterfaceDecl { fields, .. } = &program.items[0] else {
        panic!("expected interface declaration");
    };
    assert_eq!(fields[0].decorators.len(), 2);
    assert_eq!(fields[0].decorators[0].name, "minLength");
    assert_eq!(fields[0].decorators[0].args.len(), 2);
    assert_eq!(fields[0].decorators[1].name, "isTrimmed");

    let Item::FnDecl { params, .. } = &program.items[1] else {
        panic!("expected function declaration");
    };
    assert!(params[0].decorators.is_empty());
    assert_eq!(params[1].decorators.len(), 1);
    assert_eq!(params[1].decorators[0].name, "notEqual");
}

#[test]
fn preserves_validator_decorator_argument_ranges() {
    let source = r#"
        interface ProductFilter {
            @range(5, 100, "page size is out of range")
            pageSize: i32;
        }
    "#;
    let spanned = Token::lexer(source)
        .spanned()
        .filter_map(|(token, range)| token.ok().map(|token| (token, range)))
        .collect::<Vec<_>>();
    let tokens = spanned.iter().map(|(token, _)| token.clone()).collect();
    let ranges = spanned.iter().map(|(_, range)| range.clone()).collect();
    let program = Parser::new_with_ranges(tokens, ranges)
        .parse_program()
        .unwrap();
    let Item::InterfaceDecl { fields, .. } = &program.items[0] else {
        panic!("expected interface declaration");
    };
    let decorator = &fields[0].decorators[0];

    assert_eq!(decorator.argument_ranges.len(), 3);
    assert_eq!(&source[decorator.argument_ranges[0].clone()], "5");
    assert_eq!(&source[decorator.argument_ranges[1].clone()], "100");
    assert_eq!(
        &source[decorator.argument_ranges[2].clone()],
        "\"page size is out of range\""
    );
}

#[test]
fn parses_in_keyword_as_validator_name_after_at_sign() {
    let program = parse(
        r#"
            interface AccessInput {
                @in(["admin", "user"])
                role: string;
            }
        "#,
    );
    let Item::InterfaceDecl { fields, .. } = &program.items[0] else {
        panic!("expected interface declaration");
    };
    assert_eq!(fields[0].decorators.len(), 1);
    assert_eq!(fields[0].decorators[0].name, "in");
}

#[test]
fn parses_while_loop() {
    let program = parse("const main = (): void => { let i: i32 = 0; while (i < 3) { i += 1; } };");
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    assert!(matches!(body[1].kind(), StmtKind::While { .. }));
}

#[test]
fn parses_typescript_style_for_loop() {
    let program = parse(
        "const main = (): void => { for (let i: i32 = 0; i < 3; i += 1) { console.log(i); } };",
    );
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    assert!(matches!(body[0].kind(), StmtKind::For { .. }));
}

#[test]
fn parses_for_of_loop() {
    let program = parse(
        "const main = (): void => { const values: i32[] = [1, 2]; for (const value of values) { console.log(value); } };",
    );
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    let StmtKind::ForOf {
        mutable,
        name,
        iterable,
        ..
    } = body[1].kind()
    else {
        panic!("expected for...of loop");
    };
    assert!(!mutable);
    assert_eq!(name, "value");
    assert!(matches!(iterable.kind(), ExprKind::Ident(name) if name == "values"));
}

#[test]
fn parses_typeof_expression() {
    let program = parse("const main = (): void => { const t: string = typeof 42; };");
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    assert!(matches!(
        body[0].kind(),
        StmtKind::VarDecl {
            value: Some(Expr {
                kind: ExprKind::Typeof(..),
                ..
            }),
            ..
        }
    ));
}

#[test]
fn parses_typeof_type_query() {
    let program = parse("type T = typeof 42; const main = (): void => { const x: T = 42; };");
    let Item::TypeAlias { ty, .. } = &program.items[0] else {
        panic!("expected type alias");
    };
    assert!(matches!(ty, Type::TypeQuery(..)));
}

#[test]
fn parses_typescript_style_tuple_types() {
    let program = parse("const main = (): void => { const point: [f64, f64] = [10.0, 20.0]; };");
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    assert!(matches!(
            body[0].kind(),
        StmtKind::VarDecl {
            ty: Type::Tuple(elements),
            value: Some(Expr { kind: ExprKind::ArrayLit(values), .. }),
            ..
        } if matches!(elements.as_slice(), [Type::F64, Type::F64]) && values.len() == 2
    ));
}

#[test]
fn parses_fixed_array_types_as_homogeneous_tuples() {
    let program =
        parse("const repeated: [i32; 5] = [0; 5]; const values: [i32; 5] = [1, 2, 3, 4, 5];");
    let types = program
        .items
        .iter()
        .map(|item| match item {
            Item::Stmt(Stmt {
                kind: StmtKind::VarDecl { ty, .. },
                ..
            }) => ty,
            _ => panic!("expected variable declaration"),
        })
        .collect::<Vec<_>>();
    let expected = Type::Tuple(vec![Type::I32; 5]);
    assert_eq!(types, vec![&expected, &expected]);
}

#[test]
fn parses_empty_tuples_and_rejects_invalid_fixed_array_lengths() {
    let program = parse("const empty: [] = []; ");
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { ty, .. },
        ..
    }) = &program.items[0]
    else {
        panic!("expected variable declaration");
    };
    assert_eq!(ty, &Type::Tuple(Vec::new()));

    for length in ["0", "65537"] {
        let source = format!("const invalid: [i32; {length}] = []; ");
        let tokens = crate::lexer::lex(&source).unwrap();
        let error = Parser::new(tokens).parse_program().unwrap_err();
        assert!(
            error.message.contains("fixed array length"),
            "unexpected parser error for length {length}: {}",
            error.message
        );
    }
}

#[test]
fn parses_generic_tuple_optional_and_range_types() {
    let program = parse(
        "const main = (): void => { const pair: Tuple<string, i32> = [\"one\", 1]; const count: Optional<i32> = 1; const span: Range<i32> = 1..4; };",
    );
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    assert!(
        matches!(body[0].kind(), StmtKind::VarDecl { ty: Type::Apply { name, args }, .. }
        if name == "Tuple" && args.as_slice() == [Type::Str, Type::I32])
    );
    assert!(
        matches!(body[1].kind(), StmtKind::VarDecl { ty: Type::Apply { name, args }, .. }
        if name == "Optional" && args.as_slice() == [Type::I32])
    );
    assert!(
        matches!(body[2].kind(), StmtKind::VarDecl { ty: Type::Apply { name, args }, .. }
        if name == "Range" && args.as_slice() == [Type::I32])
    );
}

#[test]
fn parses_full_numeric_types_and_numeric_constants() {
    let program = parse(
        "const main = (): void => { const a: i128 = i128.MAX; const b: usize = usize.MIN; const c: f128 = f128.MAX; const d: isize = isize.MAX; const e: decimal = decimal.MIN; };",
    );
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    assert!(matches!(
            body[0].kind(),
        StmtKind::VarDecl {
            ty: Type::I128,
            value: Some(Expr { kind: ExprKind::FieldAccess { field, .. }, .. }),
            ..
        } if field == "MAX"
    ));
    assert!(matches!(
            body[1].kind(),
        StmtKind::VarDecl {
            ty: Type::Usize,
            value: Some(Expr { kind: ExprKind::FieldAccess { field, .. }, .. }),
            ..
        } if field == "MIN"
    ));
    assert!(matches!(
            body[2].kind(),
        StmtKind::VarDecl {
            ty: Type::F128,
            value: Some(Expr { kind: ExprKind::FieldAccess { field, .. }, .. }),
            ..
        } if field == "MAX"
    ));
    assert!(matches!(
            body[4].kind(),
        StmtKind::VarDecl {
            ty: Type::Decimal,
            value: Some(Expr { kind: ExprKind::FieldAccess { field, .. }, .. }),
            ..
        } if field == "MIN"
    ));
    assert!(matches!(
            body[3].kind(),
        StmtKind::VarDecl {
            ty: Type::Isize,
            value: Some(Expr { kind: ExprKind::FieldAccess { field, .. }, .. }),
            ..
        } if field == "MAX"
    ));
}

#[test]
fn parses_narrow_integer_types() {
    let program = parse(
        "const main = (): void => { const a: i8 = 1; const b: i16 = 2; const c: u8 = 3; const d: u16 = 4; };",
    );
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    let types = body
        .iter()
        .map(|stmt| match stmt.kind() {
            StmtKind::VarDecl { ty, .. } => ty.clone(),
            other => panic!("expected variable declaration, got {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(types, vec![Type::I8, Type::I16, Type::U8, Type::U16]);
}

#[test]
fn parses_the_char_type() {
    let program = parse("const main = (): void => { const letter: char = 'A'; };");
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    assert!(matches!(
        body[0].kind(),
        StmtKind::VarDecl { ty: Type::Char, .. }
    ));
}

#[test]
fn parses_optional_object_fields_and_object_update() {
    let program = parse(
        r#"
            interface User {
                name: string;
                nickname?: string;
            }
            const user: User = { name: "Ada" };
            const renamed: User = { ...user, name: "Grace" };
        "#,
    );
    let Item::InterfaceDecl { fields, .. } = &program.items[0] else {
        panic!("expected interface declaration");
    };
    assert!(!fields[0].optional);
    assert!(fields[1].optional);

    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { value, .. },
        ..
    }) = &program.items[2]
    else {
        panic!("expected updated object declaration");
    };
    let Some(Expr {
        kind: ExprKind::ObjectUpdate { base, fields },
        ..
    }) = value
    else {
        panic!("expected object update");
    };
    assert!(matches!(base.as_ref().kind(), ExprKind::Ident(name) if name == "user"));
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].0, "name");
}

#[test]
fn parses_arrow_function_as_a_typed_value() {
    let program = parse(
        "const main = (): void => { const double: (i32) => i32 = (value: i32): i32 => value * 2; };",
    );
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    let StmtKind::VarDecl { ty, value, .. } = body[0].kind() else {
        panic!("expected variable declaration");
    };
    assert!(matches!(ty, Type::Function { .. }));
    assert!(matches!(
        value,
        Some(Expr {
            kind: ExprKind::ArrowFn { .. },
            ..
        })
    ));
}

#[test]
fn parses_a_parenthesized_initializer_that_is_not_an_arrow_function() {
    let program = parse("const main = (): void => { const value = (1 + 2); console.log(value); };");
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    let StmtKind::VarDecl { value, .. } = body[0].kind() else {
        panic!("expected variable declaration");
    };
    assert!(matches!(
        value,
        Some(Expr {
            kind: ExprKind::BinOp { .. },
            ..
        })
    ));
}

#[test]
fn parses_a_parenthesized_top_level_initializer() {
    let program = parse("const value = (1 + 2);");
    let Item::Stmt(statement) = &program.items[0] else {
        panic!("expected a variable declaration, not a function");
    };
    let StmtKind::VarDecl { value, .. } = statement.kind() else {
        panic!("expected variable declaration");
    };
    assert!(matches!(
        value,
        Some(Expr {
            kind: ExprKind::BinOp { .. },
            ..
        })
    ));
}

#[test]
fn parses_template_literal_with_interpolation() {
    let program =
        parse("export const greet = (name: &string): void => { console.log(`Hello, ${name}`); };");
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    let StmtKind::Expr(Expr {
        kind: ExprKind::MethodCall { args, .. },
        ..
    }) = body[0].kind()
    else {
        panic!("expected console method call");
    };
    let ExprKind::TemplateLit(parts) = &args[0].kind() else {
        panic!("expected template literal");
    };
    assert!(matches!(&parts[0], TemplatePart::Text(text) if text == "Hello, "));
    assert!(
        matches!(&parts[1], TemplatePart::Expr(Expr { kind: ExprKind::Ident(name), .. }, _) if name == "name")
    );
}

#[test]
fn parses_multiline_template_literal_with_unicode_escapes() {
    let program = parse(
        "const name: string = \"Ada\";\nconst message: string = `Ol\\u00e1, ${name}\nwelcome`;",
    );
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { value, .. },
        ..
    }) = &program.items[1]
    else {
        panic!("expected variable declaration");
    };
    let Some(Expr {
        kind: ExprKind::TemplateLit(parts),
        ..
    }) = value
    else {
        panic!("expected template literal");
    };
    assert!(matches!(&parts[0], TemplatePart::Text(text) if text == "Olá, "));
    assert!(
        matches!(&parts[1], TemplatePart::Expr(Expr { kind: ExprKind::Ident(name), .. }, _) if name == "name")
    );
    assert!(matches!(&parts[2], TemplatePart::Text(text) if text == "\nwelcome"));
}

#[test]
fn parses_fixed_point_format_specifier_on_a_float_interpolation() {
    let program = parse("const message = `Valor: ${pi:.2f}`;");
    let value = var_decl_value(&program);
    let ExprKind::TemplateLit(parts) = value.kind() else {
        panic!("expected template literal");
    };
    let TemplatePart::Expr(expr, spec) = &parts[1] else {
        panic!("expected an interpolation part");
    };
    assert!(matches!(expr.kind(), ExprKind::Ident(name) if name == "pi"));
    assert_eq!(*spec, Some(FormatSpec::FixedPoint(2)));
}

#[test]
fn parses_zero_padded_format_specifier_on_an_integer_interpolation() {
    let program = parse("const message = `Codigo: ${id:04d}`;");
    let value = var_decl_value(&program);
    let ExprKind::TemplateLit(parts) = value.kind() else {
        panic!("expected template literal");
    };
    let TemplatePart::Expr(expr, spec) = &parts[1] else {
        panic!("expected an interpolation part");
    };
    assert!(matches!(expr.kind(), ExprKind::Ident(name) if name == "id"));
    assert_eq!(*spec, Some(FormatSpec::ZeroPadded(4)));
}

#[test]
fn interpolation_without_a_trailing_colon_has_no_format_specifier() {
    let program = parse("const message = `Valor: ${pi}`;");
    let value = var_decl_value(&program);
    let ExprKind::TemplateLit(parts) = value.kind() else {
        panic!("expected template literal");
    };
    let TemplatePart::Expr(_, spec) = &parts[1] else {
        panic!("expected an interpolation part");
    };
    assert_eq!(*spec, None);
}

#[test]
fn a_ternarys_own_colon_is_not_mistaken_for_a_format_specifier() {
    let program = parse("const message = `Valor: ${flag ? 1 : 2}`;");
    let value = var_decl_value(&program);
    let ExprKind::TemplateLit(parts) = value.kind() else {
        panic!("expected template literal");
    };
    let TemplatePart::Expr(expr, spec) = &parts[1] else {
        panic!("expected an interpolation part");
    };
    assert!(matches!(expr.kind(), ExprKind::Ternary { .. }));
    assert_eq!(*spec, None);
}

#[test]
fn parses_array_types_literals_indexing_and_length() {
    let program = parse(
        r#"
            const methods: string[] = ["GET", "POST"];
            const statuses: Array<i32> = [200, 201];
            const first: string = methods[0];
            const count: i64 = methods.length;
            let mutable: i32[] = [1, 2];
            mutable[0] = 3;
        "#,
    );
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { ty, value, .. },
        ..
    }) = &program.items[0]
    else {
        panic!("expected array declaration");
    };
    assert_eq!(ty, &Type::Array(Box::new(Type::Str)));
    assert!(
        matches!(value, Some(Expr { kind: ExprKind::ArrayLit(values), .. }) if values.len() == 2)
    );
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { value, .. },
        ..
    }) = &program.items[2]
    else {
        panic!("expected indexed declaration");
    };
    assert!(matches!(
        value,
        Some(Expr {
            kind: ExprKind::IndexAccess { .. },
            ..
        })
    ));
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { value, .. },
        ..
    }) = &program.items[3]
    else {
        panic!("expected length declaration");
    };
    assert!(
        matches!(value, Some(Expr { kind: ExprKind::FieldAccess { field, .. }, .. }) if field == "length")
    );
    assert!(matches!(
        program.items[5],
        Item::Stmt(Stmt {
            kind: StmtKind::IndexAssign { .. },
            ..
        })
    ));
}

#[test]
fn parses_member_and_nested_place_assignments() {
    let program = parse("obj.field = 1; obj.inner.value += 2; items[0].count = 3;");

    let Item::Stmt(Stmt {
        kind: StmtKind::FieldAssign {
            object, field, op, ..
        },
        ..
    }) = &program.items[0]
    else {
        panic!("expected a field assignment");
    };
    assert_eq!(field, "field");
    assert!(matches!(op, AssignOp::Assign));
    assert!(matches!(object.kind, ExprKind::Ident(ref name) if name == "obj"));

    let Item::Stmt(Stmt {
        kind: StmtKind::FieldAssign {
            object, field, op, ..
        },
        ..
    }) = &program.items[1]
    else {
        panic!("expected a nested field assignment");
    };
    assert_eq!(field, "value");
    assert!(matches!(op, AssignOp::AddAssign));
    assert!(matches!(object.kind, ExprKind::FieldAccess { .. }));

    let Item::Stmt(Stmt {
        kind: StmtKind::FieldAssign { object, field, .. },
        ..
    }) = &program.items[2]
    else {
        panic!("expected a field assignment on an indexed element");
    };
    assert_eq!(field, "count");
    assert!(matches!(object.kind, ExprKind::IndexAccess { .. }));
}

#[test]
fn parses_typescript_style_generic_functions_and_calls() {
    let program =
        parse("const identity = <T>(value: T): T => value; const result: i32 = identity<i32>(42);");
    let Item::FnDecl {
        type_params,
        params,
        ret_type,
        ..
    } = &program.items[0]
    else {
        panic!("expected generic function");
    };
    assert_eq!(type_params[0].name, "T");
    assert_eq!(params[0].ty, Type::Generic("T".to_string()));
    assert_eq!(ret_type, &Type::Generic("T".to_string()));
    let Item::Stmt(Stmt {
        kind:
            StmtKind::VarDecl {
                value:
                    Some(Expr {
                        kind: ExprKind::FnCall { type_args, .. },
                        ..
                    }),
                ..
            },
        ..
    }) = &program.items[1]
    else {
        panic!("expected generic call");
    };
    assert_eq!(type_args, &vec![Type::I32]);
}

#[test]
fn parses_contextually_typed_arrow_functions() {
    let program = parse(
        "const main = (): void => { const values: i32[] = [1]; const mapped: i32[] = values.map((value, index) => value + index); const filtered: i32[] = mapped.filter(value => value > 0); };",
    );
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    let StmtKind::VarDecl {
        value:
            Some(Expr {
                kind: ExprKind::MethodCall { args, .. },
                ..
            }),
        ..
    } = body[1].kind()
    else {
        panic!("expected map call");
    };
    let ExprKind::ArrowFn {
        params, ret_type, ..
    } = args[0].kind()
    else {
        panic!("expected arrow callback");
    };
    assert_eq!(params.len(), 2);
    assert!(params.iter().all(|param| param.ty == Type::Infer));
    assert_eq!(ret_type, &Type::Infer);
}

#[test]
fn preserves_async_function_markers() {
    let program = parse(
        "async function load(): Promise<i32> { return 1; } const compute = async (): Promise<i32> => 2; const main = (): void => { const nested = async function (): Promise<i32> { return 3; }; };",
    );
    let Item::FnDecl {
        is_async, ret_type, ..
    } = &program.items[0]
    else {
        panic!("expected async function declaration");
    };
    assert!(*is_async);
    assert_eq!(
        ret_type,
        &Type::Apply {
            name: "Promise".to_string(),
            args: vec![Type::I32],
        }
    );
    let Item::FnDecl { is_async, .. } = &program.items[1] else {
        panic!("expected async arrow declaration");
    };
    assert!(*is_async);
    let Item::FnDecl { body, .. } = &program.items[2] else {
        panic!("expected main function");
    };
    let StmtKind::VarDecl {
        value:
            Some(Expr {
                kind: ExprKind::ArrowFn { is_async, .. },
                ..
            }),
        ..
    } = body[0].kind()
    else {
        panic!("expected async function expression");
    };
    assert!(*is_async);
}

#[test]
fn preserves_comptime_declarations_and_expressions() {
    let program = parse(
        "comptime function double(value: i32): i32 { return value * 2; } comptime const mask = (): i32 => 3; const result = comptime(double(mask()));",
    );
    let Item::FnDecl {
        name,
        is_comptime,
        is_async,
        ..
    } = &program.items[0]
    else {
        panic!("expected comptime function declaration");
    };
    assert_eq!(name, "double");
    assert!(*is_comptime);
    assert!(!*is_async);
    let Item::FnDecl {
        name, is_comptime, ..
    } = &program.items[1]
    else {
        panic!("expected comptime arrow declaration");
    };
    assert_eq!(name, "mask");
    assert!(*is_comptime);
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl {
            value: Some(value), ..
        },
        ..
    }) = &program.items[2]
    else {
        panic!("expected result declaration");
    };
    assert!(matches!(value.kind(), ExprKind::Comptime(_)));
}

#[test]
fn rejects_invalid_comptime_declaration_shapes() {
    for (source, expected) in [
        (
            "comptime const value = 1;",
            "`comptime const` requires an arrow function initializer",
        ),
        (
            "comptime let value = (): i32 => 1;",
            "`comptime` may only qualify `function` or a `const` arrow function",
        ),
    ] {
        let tokens = crate::lexer::lex(source).unwrap();
        let error = Parser::new(tokens).parse_program().unwrap_err();
        assert_eq!(error.message, expected);
    }
}

#[test]
fn parses_await_expression() {
    let program =
        parse("async function load(value: Promise<i32>): Promise<i32> { return await value; }");
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    assert!(matches!(
            body[0].kind(),
        StmtKind::Return(Some(Expr { kind: ExprKind::Await(inner), .. })) if matches!(inner.as_ref().kind(), ExprKind::Ident(name) if name == "value")
    ));
}

#[test]
fn parses_modulo_and_right_associative_exponentiation() {
    let program = parse("const value: i32 = 10 % 4 * 2 ** 3 ** 2;");
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { value, .. },
        ..
    }) = &program.items[0]
    else {
        panic!("expected variable declaration");
    };
    let Some(Expr {
        kind:
            ExprKind::BinOp {
                op: BinOp::Mul,
                left,
                right,
            },
        ..
    }) = value
    else {
        panic!("expected multiplication at the root");
    };
    assert!(matches!(
        left.as_ref().kind(),
        ExprKind::BinOp { op: BinOp::Mod, .. }
    ));
    let ExprKind::BinOp {
        op: BinOp::Pow,
        right: nested_power,
        ..
    } = right.kind()
    else {
        panic!("expected exponentiation");
    };
    assert!(matches!(
        nested_power.kind(),
        ExprKind::BinOp { op: BinOp::Pow, .. }
    ));
}

#[test]
fn parses_modulo_and_power_assignments() {
    let program =
        parse("const main = (): void => { let value: i32 = 10; value %= 4; value **= 3; };");
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    assert!(matches!(
        body[1].kind(),
        StmtKind::Assign {
            op: AssignOp::ModAssign,
            ..
        }
    ));
    assert!(matches!(
        body[2].kind(),
        StmtKind::Assign {
            op: AssignOp::PowAssign,
            ..
        }
    ));
}

#[test]
fn parses_logical_assignments() {
    let program = parse(
        "const main = (): void => { let active: boolean = true; active &&= false; active ||= true; let name: string? = \"Ada\"; name ??= \"guest\"; };",
    );
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    assert!(matches!(
        body[1].kind(),
        StmtKind::Assign {
            op: AssignOp::AndAssign,
            ..
        }
    ));
    assert!(matches!(
        body[2].kind(),
        StmtKind::Assign {
            op: AssignOp::OrAssign,
            ..
        }
    ));
    assert!(matches!(
        body[4].kind(),
        StmtKind::Assign {
            op: AssignOp::NullCoalesceAssign,
            ..
        }
    ));
}

#[test]
fn parses_named_imports_aliases_and_exports() {
    let program = parse(
        r#"
            import { add as sum, factor } from "./math";
            export const answer: i32 = sum(factor, 22);
        "#,
    );
    assert_eq!(program.imports.len(), 1);
    assert_eq!(program.imports[0].source, "./math");
    assert_eq!(program.imports[0].specifiers[0].imported, "add");
    assert_eq!(program.imports[0].specifiers[0].local, "sum");
    assert_eq!(program.imports[0].specifiers[1].local, "factor");
    assert_eq!(
        program
            .exports
            .iter()
            .filter_map(|export| export.exported_name())
            .collect::<Vec<_>>(),
        vec!["answer"]
    );
}

#[test]
fn parses_default_namespace_and_mixed_imports() {
    let program = parse(
        r#"
            import sum from "./math";
            import * as math from "./math";
            import total, { minus as sub } from "./math";
        "#,
    );
    assert_eq!(program.imports[0].specifiers[0].kind, ImportKind::Default);
    assert_eq!(program.imports[0].specifiers[0].imported, "default");
    assert_eq!(program.imports[0].specifiers[0].local, "sum");

    assert_eq!(program.imports[1].specifiers[0].kind, ImportKind::Namespace);
    assert_eq!(program.imports[1].specifiers[0].local, "math");

    assert_eq!(program.imports[2].specifiers[0].kind, ImportKind::Default);
    assert_eq!(program.imports[2].specifiers[0].local, "total");
    assert_eq!(program.imports[2].specifiers[1].kind, ImportKind::Named);
    assert_eq!(program.imports[2].specifiers[1].imported, "minus");
    assert_eq!(program.imports[2].specifiers[1].local, "sub");
}

#[test]
fn parses_export_clauses_aliases_reexports_and_default() {
    let program = parse(
        r#"
            const local: i32 = 1;
            export { local };
            export { local as alias };
            export { add as plus } from "./math";
            export * from "./extra";
            export default local;
        "#,
    );
    let names = program
        .exports
        .iter()
        .filter_map(|export| export.exported_name())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["local", "alias", "plus", "default"]);

    assert!(matches!(
        &program.exports[0],
        ExportDecl::Named { inline: false, .. }
    ));
    assert!(matches!(
        &program.exports[2],
        ExportDecl::Re { source, imported, .. } if source == "./math" && imported == "add"
    ));
    assert!(matches!(
        &program.exports[3],
        ExportDecl::All { source, .. } if source == "./extra"
    ));
}

#[test]
fn export_default_requires_a_name_when_the_value_is_not_a_function() {
    let tokens = Token::lexer("export default 42;")
        .filter_map(Result::ok)
        .collect();
    let error = Parser::new(tokens).parse_program().unwrap_err();
    assert!(error.message.contains("export default"), "got: {error}");
    assert!(
        error.message.contains("const name"),
        "help must be concrete: {error}"
    );
}

/// A field key is not a binding, so a reserved word there is unambiguous and has
/// to be accepted, the way the object literal and member access positions already
/// did. Without this, a shape as ordinary as `{ "type": "user" }` had no
/// expressible type at all.
#[test]
fn a_reserved_word_is_a_valid_field_name() {
    let program = parse(
        "interface Event { type: string; from: i32; default: i32; protect: boolean; match?: i32; }",
    );
    let Item::InterfaceDecl { fields, .. } = &program.items[0] else {
        panic!("expected an interface declaration");
    };
    let names = fields
        .iter()
        .map(|field| field.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec!["type", "from", "default", "protect", "match"],
        "every reserved word must survive as a field name"
    );
}

#[test]
fn readonly_is_a_modifier_only_when_a_field_name_follows() {
    let program = parse("interface Frozen { readonly const: string; readonly: string; }");
    let Item::InterfaceDecl { fields, .. } = &program.items[0] else {
        panic!("expected an interface declaration");
    };
    assert_eq!(fields[0].name, "const");
    assert!(fields[0].readonly, "`readonly const` is a frozen field");
    assert_eq!(fields[1].name, "readonly");
    assert!(
        !fields[1].readonly,
        "`readonly: string` is a field of that name, not a modifier"
    );
}

#[test]
fn a_reserved_word_is_a_valid_field_name_in_an_inline_object_type() {
    let program = parse("type Inline = { type: string; interface: i32; };");
    let Item::InterfaceDecl { fields, .. } = &program.items[0] else {
        panic!("expected the alias to desugar to an interface");
    };
    let names = fields
        .iter()
        .map(|field| field.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["type", "interface"]);
}

#[test]
fn a_reserved_word_is_a_valid_key_in_an_object_match_pattern() {
    parse(
        "interface Event { type: string; }\nconst describe = (event: Event): string => {\n    match (event) {\n        ({ type: \"user\" }) => { return \"a\"; }\n        (_) => { return \"b\"; }\n    }\n};",
    );
}

#[test]
fn a_reserved_word_is_still_refused_as_a_binding_name() {
    for source in [
        "const type = 1;",
        "let from = 1;",
        "const { type } = shape;",
        "const handler = (type: i32): void => {};",
    ] {
        let error = parse_error_spanned(source);
        assert_eq!(
            error.code,
            Some("E0003"),
            "a binding position must stay reserved: {source}"
        );
    }
}

#[test]
fn parses_protect_on_every_exportable_declaration() {
    let program = parse(
        r#"
            export protect const answer: i32 = 42;
            export protect function helper(): void {}
            export protect interface Shape { id: i32; }
            export protect type Alias = i32;
            export protect enum Color { Red = 0 }
        "#,
    );
    assert_eq!(program.exports.len(), 5);
    for export in &program.exports {
        assert!(
            matches!(
                export,
                ExportDecl::Named {
                    inline: true,
                    visibility: Visibility::Protect,
                    ..
                }
            ),
            "every declaration should carry the modifier: {export:?}"
        );
    }
}

#[test]
fn plain_export_stays_public() {
    let program = parse("export const answer: i32 = 42;");
    assert!(matches!(
        &program.exports[0],
        ExportDecl::Named {
            visibility: Visibility::Public,
            ..
        }
    ));
}

#[test]
fn parses_protect_on_a_local_export_clause() {
    let program = parse(
        r#"
            const first: i32 = 1;
            const second: i32 = 2;
            export protect { first, second as alias };
            export { first as open };
        "#,
    );
    let visibilities = program
        .exports
        .iter()
        .map(|export| match export {
            ExportDecl::Named { visibility, .. } => *visibility,
            other => panic!("expected a named export, got {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        visibilities,
        vec![Visibility::Protect, Visibility::Protect, Visibility::Public]
    );
}

#[test]
fn rejects_protect_written_before_export() {
    let tokens = crate::lexer::lex("protect export const answer: i32 = 1;").unwrap();
    let error = Parser::new(tokens).parse_program().unwrap_err();
    assert!(
        error.code != Some("E3309") && error.code != Some("E3310"),
        "the reversed order is a plain syntax error: {error}"
    );
}

#[test]
fn rejects_protect_combined_with_a_default_export() {
    for source in [
        "const local: i32 = 1;\nexport protect default local;",
        "const local: i32 = 1;\nexport default protect local;",
    ] {
        let error = parse_error_spanned(source);
        assert_eq!(error.code, Some("E3309"), "got: {error}");
        assert!(
            error.message.contains("`protect`") && error.message.contains("default export"),
            "message must name the modifier: {error}"
        );
        assert!(
            error
                .help
                .iter()
                .any(|help| help.contains("export protect")),
            "help must be concrete: {error:?}"
        );
        let span = error.span.clone().expect("the modifier needs a span");
        assert_eq!(
            &source[span.clone()],
            "protect",
            "the range must cover the modifier, not the whole statement"
        );
    }
}

#[test]
fn rejects_protect_on_a_re_export() {
    for source in [
        "export protect { add } from \"./math\";",
        "export protect * from \"./math\";",
    ] {
        let error = parse_error_spanned(source);
        assert_eq!(error.code, Some("E3310"), "got: {error}");
        assert!(
            error.message.contains("re-export"),
            "message must name the cause: {error}"
        );
        assert!(
            error
                .help
                .iter()
                .any(|help| help.contains("keeps the scope of the original declaration")),
            "help must explain the propagation rule: {error:?}"
        );
        let span = error.span.clone().expect("the modifier needs a span");
        assert_eq!(&source[span.clone()], "protect");
    }
}

fn initializer_of(source: &str) -> Expr {
    let program = parse(source);
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl {
            value: Some(value), ..
        },
        ..
    }) = &program.items[0]
    else {
        panic!("expected a declaration with an initializer");
    };
    value.clone()
}

/// A `?` used to become a `Try` only before a fixed list of tokens, so any
/// operator after it was read as the start of a ternary.
#[test]
fn question_mark_before_a_binary_operator_is_a_try() {
    for (source, expected) in [
        ("const a = load()? + 1;", BinOp::Add),
        ("const a = load()? - 1;", BinOp::Sub),
        ("const a = load()? * 2;", BinOp::Mul),
    ] {
        let value = initializer_of(source);
        let ExprKind::BinOp { op, left, .. } = value.kind() else {
            panic!("`{source}` should be a binary operation, got {value:?}");
        };
        assert_eq!(*op, expected, "{source}");
        assert!(
            matches!(left.kind(), ExprKind::Try(_)),
            "the left operand of `{source}` should be the try, got {left:?}"
        );
    }
}

#[test]
fn a_ternary_is_still_a_ternary_next_to_the_try() {
    for source in [
        "const a = c ? 1 : 2;",
        "const a = c ? -1 : 2;",
        "const a = c ? d ? 1 : 2 : 3;",
    ] {
        let value = initializer_of(source);
        assert!(
            matches!(value.kind(), ExprKind::Ternary { .. }),
            "`{source}` should stay a ternary, got {value:?}"
        );
    }
    let value = initializer_of("const a = c ? load()? : 2;");
    let ExprKind::Ternary { then_expr, .. } = value.kind() else {
        panic!("expected a ternary, got {value:?}");
    };
    assert!(
        matches!(then_expr.kind(), ExprKind::Try(_)),
        "the branch should hold the try, got {then_expr:?}"
    );
}

/// In `c ? load()? - x : y` the inner `?` sees `- x` followed by `:` and could
/// open a ternary of its own, taking the `:` the outer ternary needs. Only the
/// outer reading completes, so the inner `?` has to end up a try.
#[test]
fn an_inner_question_mark_does_not_steal_the_outer_colon() {
    let value = initializer_of("const a = c ? load()? - x : y;");
    let ExprKind::Ternary {
        then_expr,
        else_expr,
        ..
    } = value.kind()
    else {
        panic!("expected the outer ternary, got {value:?}");
    };
    let ExprKind::BinOp { op, left, .. } = then_expr.kind() else {
        panic!("the branch should be the subtraction, got {then_expr:?}");
    };
    assert_eq!(*op, BinOp::Sub);
    assert!(
        matches!(left.kind(), ExprKind::Try(_)),
        "the inner `?` should be the try, got {left:?}"
    );
    assert!(matches!(else_expr.kind(), ExprKind::Ident(name) if name == "y"));

    let nested = initializer_of("const a = c ? d ? -1 : 2 : 3;");
    let ExprKind::Ternary { then_expr, .. } = nested.kind() else {
        panic!("expected a ternary, got {nested:?}");
    };
    assert!(
        matches!(then_expr.kind(), ExprKind::Ternary { .. }),
        "a genuine inner ternary keeps its own `:`, got {then_expr:?}"
    );
}

fn nested_ternary_source(depth: usize) -> String {
    let mut source = String::from("const a = ");
    for _ in 0..depth {
        source.push_str("c ? (");
    }
    source.push('1');
    for _ in 0..depth {
        source.push_str(") : 2");
    }
    source.push(';');
    source
}

/// Each ambiguous `?` is decided once; without the cache, a ternary nested in
/// the branch of another is re-read once per enclosing speculation.
#[test]
fn nested_ternaries_in_branches_parse_in_bounded_time() {
    let started = std::time::Instant::now();
    let value = initializer_of(&nested_ternary_source(8));
    assert!(matches!(value.kind(), ExprKind::Ternary { .. }));
    assert!(
        started.elapsed() < std::time::Duration::from_secs(2),
        "parsing nested ternaries took {:?}",
        started.elapsed()
    );
}

/// A speculation that fails must not decide the `?`: the recursion limit hit
/// while reading a branch used to be cached as "this is a try", and the real
/// parse then failed with an unrelated `Expected an expression, found Colon`.
#[test]
fn a_failed_speculation_keeps_the_real_error() {
    let tokens = crate::lexer::lex(&nested_ternary_source(40)).unwrap();
    let error = Parser::new(tokens).parse_program().unwrap_err();
    assert!(
        error.message.contains("nested too deeply"),
        "the recursion limit should be reported as such: {error}"
    );
}

#[test]
fn parses_bitwise_operators() {
    let program =
        parse("const main = (): void => { const a: i32 = 5 & 3 | 2 ^ 1 << 4 >> 2 >>> 1; };");
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    let StmtKind::VarDecl { value, .. } = body[0].kind() else {
        panic!("expected variable declaration");
    };
    let Some(Expr {
        kind:
            ExprKind::BinOp {
                op: _op,
                left: _left,
                right: _right,
            },
        ..
    }) = value
    else {
        panic!("expected binary operation");
    };
}

#[test]
fn parses_bitwise_not() {
    let program = parse("const main = (): void => { const a: i32 = ~1; };");
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    let StmtKind::VarDecl { value, .. } = body[0].kind() else {
        panic!("expected variable declaration");
    };
    assert!(matches!(
        value,
        Some(Expr {
            kind: ExprKind::BitNot(_),
            ..
        })
    ));
}

#[test]
fn parses_bitwise_compound_assignments() {
    let program = parse(
        "const main = (): void => { let x: i32 = 5; x &= 3; x |= 2; x ^= 1; x <<= 1; x >>= 1; x >>>= 1; };",
    );
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    assert!(matches!(
        body[1].kind(),
        StmtKind::Assign {
            op: AssignOp::BitAndAssign,
            ..
        }
    ));
    assert!(matches!(
        body[2].kind(),
        StmtKind::Assign {
            op: AssignOp::BitOrAssign,
            ..
        }
    ));
    assert!(matches!(
        body[3].kind(),
        StmtKind::Assign {
            op: AssignOp::BitXorAssign,
            ..
        }
    ));
    assert!(matches!(
        body[4].kind(),
        StmtKind::Assign {
            op: AssignOp::ShlAssign,
            ..
        }
    ));
    assert!(matches!(
        body[5].kind(),
        StmtKind::Assign {
            op: AssignOp::ShrAssign,
            ..
        }
    ));
    assert!(matches!(
        body[6].kind(),
        StmtKind::Assign {
            op: AssignOp::UShrAssign,
            ..
        }
    ));
}

#[test]
fn parses_interfaces_object_literals_and_field_access() {
    let program = parse(
        r#"
            interface User { name: string; active: boolean; }
            const user: User = { name: 'Ada', active: true };
            console.log(user.name);
        "#,
    );
    assert!(matches!(
        &program.items[0],
        Item::InterfaceDecl { name, fields, .. } if name == "User" && fields.len() == 2
    ));
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { value, .. },
        ..
    }) = &program.items[1]
    else {
        panic!("expected object declaration");
    };
    assert!(
        matches!(value, Some(Expr { kind: ExprKind::ObjectLit(fields), .. }) if fields.len() == 2)
    );
}

#[test]
fn parses_a_borrow_of_a_field() {
    let program = parse(
        "interface Basket { items: i32[]; } const main = (): void => { const basket: Basket = { items: [1] }; const borrowed: &i32[] = &basket.items; };",
    );
    assert!(!program.items.is_empty());
}

#[test]
fn parses_const_assertions() {
    let program = parse(
        r#"
            const status: "ready" = "ready" as const;
            const config: { readonly mode: "dev"; readonly ports: readonly i32[] } = { mode: "dev", ports: [3000, 3001] } as const;
        "#,
    );
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { value, .. },
        ..
    }) = &program.items[0]
    else {
        panic!("expected variable declaration");
    };
    assert!(
        matches!(value, Some(Expr { kind: ExprKind::ConstAssert(inner), .. }) if matches!(inner.as_ref().kind(), ExprKind::StrLit(value) if value == "ready"))
    );
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { value, .. },
        ..
    }) = &program.items[1]
    else {
        panic!("expected variable declaration");
    };
    assert!(
        matches!(value, Some(Expr { kind: ExprKind::ConstAssert(inner), .. }) if matches!(inner.as_ref().kind(), ExprKind::ObjectLit(fields) if fields.len() == 2))
    );
}

#[test]
fn pipe_operator_is_left_associative() {
    let program = parse("const r = x |> a |> b;");
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl {
            value: Some(value), ..
        },
        ..
    }) = &program.items[0]
    else {
        panic!("expected variable declaration");
    };
    let ExprKind::Pipeline { left, right } = value.kind() else {
        panic!("expected pipeline, got {value:?}");
    };
    assert!(matches!(right.as_ref().kind(), ExprKind::Ident(name) if name == "b"));
    assert!(matches!(left.as_ref().kind(), ExprKind::Pipeline { .. }));
}

#[test]
fn pipe_operator_binds_looser_than_arithmetic() {
    let program = parse("const r = a + b |> f;");
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl {
            value: Some(value), ..
        },
        ..
    }) = &program.items[0]
    else {
        panic!("expected variable declaration");
    };
    let ExprKind::Pipeline { left, right } = value.kind() else {
        panic!("expected pipeline");
    };
    assert!(matches!(left.as_ref().kind(), ExprKind::BinOp { .. }));
    assert!(matches!(right.as_ref().kind(), ExprKind::Ident(name) if name == "f"));
}

#[test]
fn pipe_operator_right_can_be_a_call() {
    let program = parse("const r = x |> style([1, 2]);");
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl {
            value: Some(value), ..
        },
        ..
    }) = &program.items[0]
    else {
        panic!("expected variable declaration");
    };
    let ExprKind::Pipeline { right, .. } = value.kind() else {
        panic!("expected pipeline");
    };
    assert!(matches!(right.as_ref().kind(), ExprKind::FnCall { callee, .. } if callee == "style"));
}

#[test]
fn pipe_operator_right_can_use_topic_token() {
    let program = parse("const r = x |> clamp(0, %, 100);");
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl {
            value: Some(value), ..
        },
        ..
    }) = &program.items[0]
    else {
        panic!("expected variable declaration");
    };
    let ExprKind::Pipeline { right, .. } = value.kind() else {
        panic!("expected pipeline");
    };
    let ExprKind::FnCall { args, .. } = right.as_ref().kind() else {
        panic!("expected topic inside call");
    };
    assert!(matches!(&args[1].kind(), ExprKind::Topic));
}

#[test]
fn desugar_pipeline_injects_first_argument() {
    use crate::ast::desugar_pipeline;
    let x = Expr::synthetic(ExprKind::Ident("x".to_string()));
    let call = desugar_pipeline(&x, &Expr::synthetic(ExprKind::Ident("f".to_string()))).unwrap();
    let ExprKind::FnCall { callee, args, .. } = &call.kind() else {
        panic!("expected FnCall");
    };
    assert_eq!(callee, "f");
    assert_eq!(args.len(), 1);
    assert!(matches!(&args[0].kind(), ExprKind::Ident(name) if name == "x"));
    let member = Expr::synthetic(ExprKind::FieldAccess {
        object: Box::new(Expr::synthetic(ExprKind::Ident("a".to_string()))),
        field: "b".to_string(),
    });
    let method = desugar_pipeline(&x, &member).unwrap();
    assert!(
        matches!(&method.kind(), ExprKind::MethodCall { method, args, .. } if method == "b" && args.len() == 1)
    );
    let existing = Expr::synthetic(ExprKind::FnCall {
        callee: "f".to_string(),
        type_args: Vec::new(),
        args: vec![Expr::synthetic(ExprKind::Ident("y".to_string()))],
    });
    let injected = desugar_pipeline(&x, &existing).unwrap();
    let ExprKind::FnCall { args, .. } = &injected.kind() else {
        panic!("expected FnCall");
    };
    assert_eq!(args.len(), 2);
    assert!(matches!(&args[0].kind(), ExprKind::Ident(name) if name == "x"));
    assert!(matches!(&args[1].kind(), ExprKind::Ident(name) if name == "y"));
    assert!(desugar_pipeline(&x, &Expr::synthetic(ExprKind::IntLit(5))).is_none());
}

#[test]
fn object_literal_allows_keyword_property_names() {
    let program = parse("const options = { default: 1, type: 2, for: 3 };");
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl {
            value: Some(value), ..
        },
        ..
    }) = &program.items[0]
    else {
        panic!("expected variable declaration");
    };
    let ExprKind::ObjectLit(fields) = value.kind() else {
        panic!("expected object literal");
    };
    let names: Vec<&str> = fields.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["default", "type", "for"]);
}

#[test]
fn object_literal_allows_numeric_property_names() {
    let program = parse("const responses = { 200: OkResponse, 404: ApiError };");
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl {
            value: Some(value), ..
        },
        ..
    }) = &program.items[0]
    else {
        panic!("expected variable declaration");
    };
    let ExprKind::ObjectLit(fields) = value.kind() else {
        panic!("expected object literal");
    };
    let names: Vec<&str> = fields.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["200", "404"]);
}

#[test]
fn shorthand_object_field_desugars_to_a_matching_ident_value() {
    let program = parse("const user = { name, age };");
    let value = var_decl_value(&program);
    let ExprKind::ObjectLit(fields) = value.kind() else {
        panic!("expected object literal");
    };
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].0, "name");
    assert!(matches!(fields[0].1.kind(), ExprKind::Ident(name) if name == "name"));
    assert_eq!(fields[1].0, "age");
    assert!(matches!(fields[1].1.kind(), ExprKind::Ident(name) if name == "age"));
}

#[test]
fn shorthand_and_explicit_object_fields_mix_freely() {
    let program = parse("const user = { name, age: 34, active: true };");
    let value = var_decl_value(&program);
    let ExprKind::ObjectLit(fields) = value.kind() else {
        panic!("expected object literal");
    };
    assert!(matches!(fields[0].1.kind(), ExprKind::Ident(name) if name == "name"));
    assert!(matches!(fields[1].1.kind(), ExprKind::IntLit(34)));
    assert!(matches!(fields[2].1.kind(), ExprKind::BoolLit(true)));
}

#[test]
fn shorthand_object_field_works_inside_a_spread_update() {
    let program = parse("const updated = { ...base, name, age: 34 };");
    let value = var_decl_value(&program);
    let ExprKind::ObjectUpdate { fields, .. } = value.kind() else {
        panic!("expected object update");
    };
    assert!(matches!(fields[0].1.kind(), ExprKind::Ident(name) if name == "name"));
    assert!(matches!(fields[1].1.kind(), ExprKind::IntLit(34)));
}

#[test]
fn object_field_without_a_name_matching_ident_has_no_shorthand_form() {
    let tokens = crate::lexer::lex("const bad = { 5 };").unwrap();
    let error = Parser::new(tokens).parse_program().unwrap_err();
    assert!(
        error.message.contains("has no shorthand form"),
        "got: {error:?}"
    );
}

#[test]
fn function_type_parameters_may_be_named() {
    let program =
        parse("interface H { onClick: (x: i32) => i32; combine: (a: i32, b?: i32) => i32; }");
    let Item::InterfaceDecl { fields, .. } = &program.items[0] else {
        panic!("expected interface");
    };
    assert!(matches!(
        &fields[0].ty,
        Type::Function { params, ret }
            if params.as_slice() == [Type::I32] && **ret == Type::I32
    ));
    assert!(matches!(
        &fields[1].ty,
        Type::Function { params, .. } if params.as_slice() == [Type::I32, Type::I32]
    ));
}

#[test]
fn parses_utility_type_forms() {
    let program = parse(
        "type A = Partial<User>;\n\
         type B = Record<string, Array<i32>>;\n\
         type C = keyof User;\n\
         type D = T extends (x: i32) => infer R ? R : never;",
    );
    let alias_ty = |index: usize| -> Type {
        match &program.items[index] {
            Item::TypeAlias { ty, .. } => ty.clone(),
            other => panic!("expected type alias, got {other:?}"),
        }
    };
    assert!(matches!(&alias_ty(0), Type::Apply { name, args }
        if name == "Partial" && args.as_slice() == [Type::Named("User".into())]));
    assert!(matches!(&alias_ty(1), Type::Apply { name, args }
        if name == "Record"
            && args[0] == Type::Str
            && matches!(&args[1], Type::Array(el) if **el == Type::I32)));
    assert!(matches!(&alias_ty(2), Type::KeyOf(inner) if **inner == Type::Named("User".into())));
    assert!(
        matches!(&alias_ty(3), Type::Conditional { bound, then, otherwise, .. }
        if matches!(bound.as_ref(), Type::Function { ret, .. } if matches!(ret.as_ref(), Type::InferVar(r) if r == "R"))
            && **then == Type::Named("R".into())
            && **otherwise == Type::Never)
    );
}

#[test]
fn parses_generic_interfaces_aliases_and_extends() {
    let program = parse(
        "interface Box<T> { value: T; }\n\
         interface Admin extends User, Timestamps { role: string; }\n\
         type Pair<A, B> = [A, B];",
    );
    let Item::InterfaceDecl {
        type_params,
        fields,
        extends,
        ..
    } = &program.items[0]
    else {
        panic!("expected interface");
    };
    assert_eq!(type_params.len(), 1);
    assert_eq!(type_params[0].name, "T");
    assert!(extends.is_empty());
    assert!(matches!(&fields[0].ty, Type::Generic(t) if t == "T"));
    let Item::InterfaceDecl { extends, .. } = &program.items[1] else {
        panic!("expected interface");
    };
    assert_eq!(
        extends,
        &[Type::Named("User".into()), Type::Named("Timestamps".into())]
    );
    let Item::TypeAlias {
        type_params, ty, ..
    } = &program.items[2]
    else {
        panic!("expected type alias");
    };
    assert_eq!(type_params.len(), 2);
    assert!(matches!(ty, Type::Tuple(elements)
        if elements.as_slice() == [Type::Generic("A".into()), Type::Generic("B".into())]));
}

#[test]
fn deeply_nested_input_errors_instead_of_overflowing() {
    let source = format!(
        "const x: i32 = {}1{};\n",
        "(".repeat(5000),
        ")".repeat(5000)
    );
    let tokens = crate::lexer::lex(&source).unwrap();
    let error = Parser::new(tokens)
        .parse_program()
        .expect_err("deep nesting should be rejected");
    assert!(error.message.contains("nested too deeply"), "got: {error}");
}

#[test]
fn deeply_nested_types_error_instead_of_overflowing() {
    let source = format!("const x: i32{} = 1;\n", "[]".repeat(5000));
    let tokens = crate::lexer::lex(&source).unwrap();
    let error = Parser::new(tokens)
        .parse_program()
        .expect_err("deep type nesting should be rejected");
    assert!(error.message.contains("nested too deeply"), "got: {error}");
}

#[test]
fn moderately_nested_input_still_parses() {
    let source = format!("const x: i32 = {}1{};\n", "(".repeat(20), ")".repeat(20));
    let tokens = crate::lexer::lex(&source).unwrap();
    assert!(Parser::new(tokens).parse_program().is_ok());
}

#[test]
fn oversized_input_is_rejected() {
    let tokens = vec![crate::lexer::Token::Semicolon; crate::parser::MAX_TOKENS + 1];
    let error = Parser::new(tokens)
        .parse_program()
        .expect_err("oversized input should be rejected");
    assert!(error.message.contains("too large"), "got: {error}");
}

#[test]
fn parses_typed_throws_clause() {
    let program = parse("const load = (): void throws ValidationError | FileError => {};");
    let Item::FnDecl { throws, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    assert_eq!(
        throws,
        &vec![
            Type::Named("ValidationError".to_string()),
            Type::Named("FileError".to_string())
        ]
    );
}

#[test]
fn every_parsed_expression_carries_the_span_it_was_written_at() {
    let source = "const total: i32 = 1 + 22 * 333;\n";
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl {
            value: Some(value), ..
        },
        ..
    }) = &program.items[0]
    else {
        panic!("expected a variable declaration");
    };

    assert_eq!(&source[value.span.to_range()], "1 + 22 * 333");
    let ExprKind::BinOp { left, right, .. } = value.kind() else {
        panic!("expected an addition");
    };
    assert_eq!(&source[left.span.to_range()], "1");
    assert_eq!(&source[right.span.to_range()], "22 * 333");
    let ExprKind::BinOp { left, right, .. } = right.kind() else {
        panic!("expected a multiplication");
    };
    assert_eq!(&source[left.span.to_range()], "22");
    assert_eq!(&source[right.span.to_range()], "333");
}

#[test]
fn a_span_distinguishes_two_uses_of_the_same_name() {
    let source = "const main = (): void => {\n    log(value);\n    log(value);\n};\n";
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let mut spans = Vec::new();
    for item in &program.items {
        let Item::FnDecl { body, .. } = item else {
            continue;
        };
        for statement in body {
            let StmtKind::Expr(expr) = statement.kind() else {
                continue;
            };
            let ExprKind::FnCall { args, .. } = expr.kind() else {
                continue;
            };
            spans.push(args[0].span);
        }
    }

    assert_eq!(spans.len(), 2, "expected both mentions of `value`");
    assert_ne!(spans[0], spans[1], "each mention must carry its own span");
    assert_eq!(&source[spans[0].to_range()], "value");
    assert_eq!(&source[spans[1].to_range()], "value");
    assert!(spans[0].end < spans[1].start);
}

#[test]
fn every_parsed_statement_carries_the_span_it_was_written_at() {
    let source = "const main = (): void => {\n    const total: i32 = 7;\n    return;\n};\n";
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected a function declaration");
    };

    assert_eq!(&source[body[0].span.to_range()], "const total: i32 = 7;");
    assert_eq!(&source[body[1].span.to_range()], "return;");

    let StmtKind::VarDecl {
        value: Some(value), ..
    } = body[0].kind()
    else {
        panic!("expected a variable declaration");
    };
    assert_eq!(&source[value.span.to_range()], "7");
    assert!(body[0].span.start < value.span.start);
    assert!(value.span.end <= body[0].span.end);
}

#[test]
fn a_parameter_span_covers_its_decorators() {
    let source = "const accept = (@min(18) age: i32, name: string): void => {};\n";
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let Item::FnDecl { params, .. } = &program.items[0] else {
        panic!("expected a function declaration");
    };

    assert_eq!(&source[params[0].span.to_range()], "@min(18) age: i32");
    assert_eq!(&source[params[1].span.to_range()], "name: string");
}

#[test]
fn an_interface_field_span_covers_its_decorators() {
    let source = "interface Filter {\n    @min(5)\n    pageSize: i32;\n\n    name: string;\n}\n";
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let Item::InterfaceDecl { fields, .. } = &program.items[0] else {
        panic!("expected an interface declaration");
    };

    assert_eq!(
        &source[fields[0].span.to_range()],
        "@min(5)\n    pageSize: i32"
    );
    assert_eq!(&source[fields[1].span.to_range()], "name: string");
}

#[test]
fn parses_explicit_type_arguments_on_method_calls() {
    let program = Parser::from_source("const capability = Promise.withResolvers<i32>();")
        .unwrap()
        .parse_program()
        .unwrap();
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl {
            value: Some(value), ..
        },
        ..
    }) = &program.items[0]
    else {
        panic!("expected a variable declaration");
    };
    assert!(matches!(
        value.kind(),
        ExprKind::MethodCall {
            method,
            type_args,
            args,
            ..
        } if method == "withResolvers" && type_args == &[Type::I32] && args.is_empty()
    ));
}

#[test]
fn preserves_the_default_position_inside_a_switch() {
    let program = Parser::from_source(
        "switch (1) { case 1: { break; } default: { console.log(\"d\"); } case 2: { break; } }",
    )
    .unwrap()
    .parse_program()
    .unwrap();
    let Item::Stmt(Stmt {
        kind:
            StmtKind::Switch {
                cases,
                default,
                default_index,
                ..
            },
        ..
    }) = &program.items[0]
    else {
        panic!("expected a switch statement");
    };
    assert_eq!(cases.len(), 2);
    assert!(default.is_some());
    assert_eq!(*default_index, Some(1));
}

#[test]
fn rejects_duplicate_switch_defaults() {
    let error = Parser::from_source(
        "switch (1) { default: { console.log(\"a\"); } default: { console.log(\"b\"); } }",
    )
    .unwrap()
    .parse_program()
    .unwrap_err();
    assert!(error.message.contains("Only one default clause"));
}

#[test]
fn parses_a_call_on_the_result_of_a_call() {
    let program = parse("const main = (): void => { makeAdder(5)(3); };");
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    let StmtKind::Expr(expr) = body[0].kind() else {
        panic!("expected an expression statement");
    };
    let ExprKind::CallValue { callee, args, .. } = expr.kind() else {
        panic!("expected a call on a value, got {:?}", expr.kind());
    };
    assert_eq!(args.len(), 1);
    assert!(matches!(
        callee.kind(),
        ExprKind::FnCall { callee, .. } if callee == "makeAdder"
    ));
}

#[test]
fn parses_a_braceless_switch_clause_with_several_statements() {
    let program = parse(
        "const main = (): void => {\n\
             let n: i32 = 0;\n\
             switch (n) {\n\
                 case 0:\n\
                     n = 1;\n\
                     break;\n\
                 case 1:\n\
                 case 2:\n\
                     n = 2;\n\
                     break;\n\
                 default:\n\
                     n = 3;\n\
             }\n\
         };",
    );
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    let StmtKind::Switch { cases, default, .. } = body[1].kind() else {
        panic!("expected a switch statement");
    };
    assert_eq!(cases.len(), 3);
    assert_eq!(cases[0].body.len(), 2);
    assert!(cases[1].body.is_empty());
    assert_eq!(cases[2].body.len(), 2);
    assert_eq!(default.as_ref().map(std::vec::Vec::len), Some(1));
}

#[test]
fn parses_match_arms_without_when() {
    let program = parse(
        "const main = (value: i32): void => { match (value) { (1) => { return; } _ => { return; } } };",
    );
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    let StmtKind::Match { arms, .. } = body[0].kind() else {
        panic!("expected match statement");
    };
    assert_eq!(arms.len(), 2);
    assert!(matches!(arms[0].pattern, MatchPattern::Lit(_)));
    assert!(matches!(arms[1].pattern, MatchPattern::Wildcard));
}

#[test]
fn parses_compact_match_arms_as_implicit_returns() {
    let program = parse(
        "const main = (value: i32): string => { match (value) { 1 => \"one\"; _ => \"other\"; } };",
    );
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    let StmtKind::Match { arms, .. } = body[0].kind() else {
        panic!("expected match statement");
    };
    assert!(arms.iter().all(|arm| arm.compact));
    assert!(matches!(arms[0].body[0].kind(), StmtKind::Return(Some(_))));
}

#[test]
fn parses_optional_semicolon_after_block_match_arm() {
    let program = parse(
        "const main = (value: i32): void => { match (value) { 1 => { return; }; _ => { return; } } };",
    );
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    let StmtKind::Match { arms, .. } = body[0].kind() else {
        panic!("expected match statement");
    };
    assert!(arms.iter().all(|arm| !arm.compact));
}

#[test]
fn rejects_compact_match_arm_without_semicolon() {
    let tokens = crate::lexer::lex("match (value) { 1 => \"one\" }").unwrap();
    let error = Parser::new(tokens).parse_program().unwrap_err();
    assert!(error.message.contains("Expected Semicolon"), "{error:?}");
}

#[test]
fn parses_array_match_pattern_with_a_final_rest_binding() {
    let program = parse(
        "const main = (values: i32[]): void => { match (values) { [first, ...rest] => { console.log(first); } _ => { return; } } };",
    );
    let patterns = match_arm_patterns(&program);
    let MatchPattern::Array { elements, .. } = &patterns[0] else {
        panic!("expected an array pattern");
    };
    assert_eq!(elements.len(), 2);
    assert!(matches!(elements[0].pattern, MatchPattern::Binding(ref name) if name == "first"));
    assert!(elements[1].is_rest());
    assert!(matches!(elements[1].pattern, MatchPattern::Binding(ref name) if name == "rest"));
}

#[test]
fn parses_empty_fixed_and_nested_array_match_patterns() {
    let program = parse(
        "const main = (values: i32[][]): void => { match (values) { [] => { return; } [first, second] => { return; } [[nested]] => { return; } _ => { return; } } };",
    );
    let patterns = match_arm_patterns(&program);
    assert!(matches!(&patterns[0], MatchPattern::Array { elements, .. } if elements.is_empty()));
    assert!(
        matches!(&patterns[1], MatchPattern::Array { elements, .. } if elements.len() == 2 && elements.iter().all(|element| !element.is_rest()))
    );
    assert!(
        matches!(&patterns[2], MatchPattern::Array { elements, .. } if matches!(elements.first().map(|element| &element.pattern), Some(MatchPattern::Array { elements: nested, .. }) if nested.len() == 1))
    );
}

#[test]
fn rejects_array_match_rest_that_is_not_last() {
    let tokens = crate::lexer::lex("match (values) { [...rest, last] => { return; } }").unwrap();
    let error = Parser::new(tokens).parse_program().unwrap_err();
    assert!(
        error
            .message
            .contains("rest pattern in an array match pattern must be the last element"),
        "{error:?}"
    );
}

#[test]
fn rejects_array_match_rest_without_a_binding_or_wildcard() {
    let tokens = crate::lexer::lex("match (values) { [...1] => { return; } }").unwrap();
    let error = Parser::new(tokens).parse_program().unwrap_err();
    assert!(
        error
            .message
            .contains("array rest pattern must bind an identifier or use `_`"),
        "{error:?}"
    );
}

#[test]
fn array_match_rest_errors_publish_stable_codes_help_and_precise_spans() {
    let cases = [
        (
            "match (values) { [...first, ...second] => { return; } }",
            "E0791",
            "...second",
            "remove the extra rest pattern",
        ),
        (
            "match (values) { [...first, last] => { return; } }",
            "E0792",
            "...first",
            "move the rest pattern to the end of the array pattern",
        ),
        (
            "match (values) { [...1] => { return; } }",
            "E0793",
            "...1",
            "replace the rest pattern with an identifier or `_`",
        ),
    ];
    for (source, code, offending, help) in cases {
        let error = Parser::from_source(source)
            .expect("source must lex")
            .parse_program()
            .expect_err("source is deliberately malformed");
        assert_eq!(error.code, Some(code));
        let span = error.span.expect("array rest error must have a span");
        assert_eq!(&source[span], offending);
        assert_eq!(error.help, vec![help]);
    }
}

fn match_arm_patterns(program: &Program) -> Vec<MatchPattern> {
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    let StmtKind::Match { arms, .. } = body[0].kind() else {
        panic!("expected match statement");
    };
    arms.iter().map(|arm| arm.pattern.clone()).collect()
}

#[test]
fn closed_range_pattern_parses_with_both_bounds() {
    let program = parse(
        "const main = (value: i32): void => { match (value) { 0..10 => { return; } _ => { return; } } };",
    );
    let patterns = match_arm_patterns(&program);
    let MatchPattern::Range {
        start,
        end,
        inclusive,
    } = &patterns[0]
    else {
        panic!("expected a range pattern");
    };
    assert!(!inclusive);
    assert!(matches!(
        start.as_ref().unwrap().kind(),
        ExprKind::IntLit(0)
    ));
    assert!(matches!(end.as_ref().unwrap().kind(), ExprKind::IntLit(10)));
}

#[test]
fn inclusive_range_pattern_sets_the_inclusive_flag() {
    let program = parse(
        "const main = (value: i32): void => { match (value) { 90..=100 => { return; } _ => { return; } } };",
    );
    let patterns = match_arm_patterns(&program);
    let MatchPattern::Range { inclusive, .. } = &patterns[0] else {
        panic!("expected a range pattern");
    };
    assert!(inclusive);
}

#[test]
fn open_start_range_pattern_has_no_start_bound() {
    let program = parse(
        "const main = (value: i32): void => { match (value) { ..60 => { return; } _ => { return; } } };",
    );
    let patterns = match_arm_patterns(&program);
    let MatchPattern::Range { start, end, .. } = &patterns[0] else {
        panic!("expected a range pattern");
    };
    assert!(start.is_none());
    assert!(matches!(end.as_ref().unwrap().kind(), ExprKind::IntLit(60)));
}

#[test]
fn open_end_range_pattern_has_no_end_bound() {
    let program = parse(
        "const main = (value: i32): void => { match (value) { 100.. => { return; } _ => { return; } } };",
    );
    let patterns = match_arm_patterns(&program);
    let MatchPattern::Range { start, end, .. } = &patterns[0] else {
        panic!("expected a range pattern");
    };
    assert!(matches!(
        start.as_ref().unwrap().kind(),
        ExprKind::IntLit(100)
    ));
    assert!(end.is_none());
}

#[test]
fn open_end_range_pattern_with_a_guard_still_finds_the_arrow() {
    let program = parse(
        "const main = (value: i32): void => { match (value) { 100.. if (value < 200) => { return; } _ => { return; } } };",
    );
    let patterns = match_arm_patterns(&program);
    let MatchPattern::Range { end, .. } = &patterns[0] else {
        panic!("expected a range pattern");
    };
    assert!(end.is_none());
}

#[test]
fn plain_literal_pattern_still_parses_as_lit() {
    let program = parse(
        "const main = (value: i32): void => { match (value) { 5 => { return; } _ => { return; } } };",
    );
    let patterns = match_arm_patterns(&program);
    assert!(matches!(patterns[0], MatchPattern::Lit(_)));
}

#[test]
fn when_remains_reserved_but_cannot_prefix_a_match_arm() {
    let tokens = Token::lexer("when").collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(tokens, vec![Token::Ident("when".to_string())]);

    Parser::from_source(
        "const main = (value: i32): void => { match (value) { when (1) => { return; } } };",
    )
    .unwrap()
    .parse_program()
    .unwrap_err();
}

fn var_decl_value(program: &Program) -> &Expr {
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { value, .. },
        ..
    }) = &program.items[0]
    else {
        panic!("expected variable declaration");
    };
    value.as_ref().expect("expected an initializer")
}

#[test]
fn two_term_relational_comparison_stays_a_plain_binop() {
    let program = parse("const value = 1 < 2;");
    assert!(matches!(
        var_decl_value(&program).kind(),
        ExprKind::BinOp { op: BinOp::Lt, .. }
    ));
}

#[test]
fn three_term_relational_chain_parses_as_chained_comparison() {
    let program = parse("const value = 1 < 2 < 3;");
    let ExprKind::ChainedComparison { terms, ops } = var_decl_value(&program).kind() else {
        panic!("expected a chained comparison");
    };
    assert_eq!(ops, &[BinOp::Lt, BinOp::Lt]);
    assert_eq!(terms.len(), 3);
    assert!(matches!(terms[0].kind(), ExprKind::IntLit(1)));
    assert!(matches!(terms[1].kind(), ExprKind::IntLit(2)));
    assert!(matches!(terms[2].kind(), ExprKind::IntLit(3)));
}

#[test]
fn chained_comparison_mixes_relational_and_strict_equality_operators() {
    let program = parse("const value = 18 <= idade === 65;");
    let ExprKind::ChainedComparison { ops, .. } = var_decl_value(&program).kind() else {
        panic!("expected a chained comparison");
    };
    assert_eq!(ops, &[BinOp::Lte, BinOp::Eq]);
}

#[test]
fn loose_inequality_never_chains() {
    let program = parse("const value = (1 < 2) !== (2 < 3);");
    assert!(matches!(
        var_decl_value(&program).kind(),
        ExprKind::BinOp { op: BinOp::Neq, .. }
    ));
}

#[test]
fn four_term_chain_parses_as_one_chained_comparison() {
    let program = parse("const value = a < b <= c < d;");
    let ExprKind::ChainedComparison { terms, ops } = var_decl_value(&program).kind() else {
        panic!("expected a chained comparison");
    };
    assert_eq!(terms.len(), 4);
    assert_eq!(ops, &[BinOp::Lt, BinOp::Lte, BinOp::Lt]);
}

#[test]
fn exclusive_range_parses_as_range_expr() {
    let program = parse("const value = 0..10;");
    let ExprKind::Range {
        start,
        end,
        inclusive,
    } = var_decl_value(&program).kind()
    else {
        panic!("expected a range expression");
    };
    assert!(!inclusive);
    assert!(matches!(start.kind(), ExprKind::IntLit(0)));
    assert!(matches!(end.kind(), ExprKind::IntLit(10)));
}

#[test]
fn inclusive_range_parses_with_inclusive_flag() {
    let program = parse("const value = 1..=5;");
    let ExprKind::Range { inclusive, .. } = var_decl_value(&program).kind() else {
        panic!("expected a range expression");
    };
    assert!(inclusive);
}

#[test]
fn range_bounds_parse_through_addition_precedence() {
    let program = parse("const value = a + 1..b * 2;");
    let ExprKind::Range { start, end, .. } = var_decl_value(&program).kind() else {
        panic!("expected a range expression");
    };
    assert!(matches!(
        start.kind(),
        ExprKind::BinOp { op: BinOp::Add, .. }
    ));
    assert!(matches!(end.kind(), ExprKind::BinOp { op: BinOp::Mul, .. }));
}

#[test]
fn plain_in_still_parses_as_in_expr() {
    let program = parse(r#"const value = "x" in obj;"#);
    assert!(matches!(
        var_decl_value(&program).kind(),
        ExprKind::In { .. }
    ));
}

#[test]
fn not_in_desugars_to_negated_in_expr() {
    let program = parse(r#"const value = "x" not in obj;"#);
    let ExprKind::Not(inner) = var_decl_value(&program).kind() else {
        panic!("expected `not in` to desugar to ExprKind::Not(In)");
    };
    assert!(matches!(inner.kind(), ExprKind::In { .. }));
}

#[test]
fn postfix_unwrap_still_parses_when_not_followed_by_in() {
    let program = parse("const value = maybe!;");
    assert!(matches!(
        var_decl_value(&program).kind(),
        ExprKind::Unwrap(_)
    ));
}

fn slice_call(expr: &Expr) -> (&str, &[Expr]) {
    let ExprKind::MethodCall { method, args, .. } = expr.kind() else {
        panic!("expected a MethodCall, got {expr:?}");
    };
    (method.as_str(), args.as_slice())
}

#[test]
fn plain_index_still_parses_as_index_access() {
    let program = parse("const value = xs[0];");
    assert!(matches!(
        var_decl_value(&program).kind(),
        ExprKind::IndexAccess { .. }
    ));
}

#[test]
fn exclusive_slice_desugars_to_slice_call_with_both_bounds() {
    let program = parse("const value = xs[1..4];");
    let (method, args) = slice_call(var_decl_value(&program));
    assert_eq!(method, "slice");
    assert_eq!(args.len(), 2);
    assert!(matches!(args[0].kind(), ExprKind::IntLit(1)));
    assert!(matches!(args[1].kind(), ExprKind::IntLit(4)));
}

#[test]
fn inclusive_slice_folds_a_literal_end_instead_of_adding_at_runtime() {
    let program = parse("const value = xs[1..=3];");
    let (method, args) = slice_call(var_decl_value(&program));
    assert_eq!(method, "slice");
    assert_eq!(args.len(), 2);
    assert!(matches!(args[0].kind(), ExprKind::IntLit(1)));
    assert!(matches!(args[1].kind(), ExprKind::IntLit(4)));
}

#[test]
fn inclusive_slice_with_a_non_literal_end_adds_one_at_runtime() {
    let program = parse("const value = xs[1..=n];");
    let (method, args) = slice_call(var_decl_value(&program));
    assert_eq!(method, "slice");
    assert_eq!(args.len(), 2);
    assert!(matches!(
        args[1].kind(),
        ExprKind::BinOp { op: BinOp::Add, .. }
    ));
}

#[test]
fn open_ended_start_slice_desugars_to_a_single_argument_slice_call() {
    let program = parse("const value = xs[2..];");
    let (method, args) = slice_call(var_decl_value(&program));
    assert_eq!(method, "slice");
    assert_eq!(args.len(), 1);
    assert!(matches!(args[0].kind(), ExprKind::IntLit(2)));
}

#[test]
fn open_ended_end_slice_synthesizes_a_zero_start() {
    let program = parse("const value = xs[..3];");
    let (method, args) = slice_call(var_decl_value(&program));
    assert_eq!(method, "slice");
    assert_eq!(args.len(), 2);
    assert!(matches!(args[0].kind(), ExprKind::IntLit(0)));
    assert!(matches!(args[1].kind(), ExprKind::IntLit(3)));
}

#[test]
fn full_range_slice_desugars_to_a_zero_argument_slice_call() {
    let program = parse("const value = xs[..];");
    let (method, args) = slice_call(var_decl_value(&program));
    assert_eq!(method, "slice");
    assert!(args.is_empty());
}

#[test]
fn wildcard_in_array_destructure_gets_a_synthesized_name_and_is_marked_discarded() {
    let program = parse("const [dados, _] = pair();");
    let Item::Stmt(Stmt {
        kind: StmtKind::ArrayDestructure { bindings, .. },
        ..
    }) = &program.items[0]
    else {
        panic!("expected an array destructure");
    };
    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[0].name, "dados");
    assert!(!bindings[0].discarded);
    assert!(bindings[1].name.starts_with("__tyrus_discard_"));
    assert!(bindings[1].discarded);
}

#[test]
fn two_wildcards_in_the_same_array_destructure_get_distinct_synthesized_names() {
    let program = parse("const [_, _] = pair();");
    let Item::Stmt(Stmt {
        kind: StmtKind::ArrayDestructure { bindings, .. },
        ..
    }) = &program.items[0]
    else {
        panic!("expected an array destructure");
    };
    assert_eq!(bindings.len(), 2);
    assert!(bindings[0].discarded);
    assert!(bindings[1].discarded);
    assert_ne!(bindings[0].name, bindings[1].name);
}

#[test]
fn object_destructure_never_discards_even_a_field_bound_to_underscore() {
    let program = parse("const { _ } = pair();");
    let Item::Stmt(Stmt {
        kind: StmtKind::ObjectDestructure { bindings, .. },
        ..
    }) = &program.items[0]
    else {
        panic!("expected an object destructure");
    };
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].name, "_");
    assert!(!bindings[0].discarded);
}

#[test]
fn a_plain_parse_error_is_anchored_on_the_token_that_failed() {
    let cases = [
        ("const x: i32 = ;\n", ";"),
        ("const main = (): void => { if (x > 0 { } };\n", "{"),
        ("const z: = 2;\n", "="),
    ];
    for (source, offending) in cases {
        let error = Parser::from_source(source)
            .expect("lexes")
            .parse_program()
            .expect_err("the source is deliberately malformed");
        let span = error
            .span
            .clone()
            .unwrap_or_else(|| panic!("no span for: {source}"));
        assert_eq!(
            &source[span.clone()],
            offending,
            "the caret must land on `{offending}` in: {source}"
        );
    }
}

#[test]
fn a_span_that_would_contradict_its_message_is_not_published() {
    let source = "const p = { a: 1, : 2 };\n";
    let error = Parser::from_source(source)
        .expect("lexes")
        .parse_program()
        .expect_err("the source is deliberately malformed");
    assert!(
        error.message.contains("Colon"),
        "unexpected message: {}",
        error.message
    );
    if let Some(span) = error.span.clone() {
        assert_eq!(
            &source[span], ":",
            "a published caret must point at the token the message names"
        );
    }
}

#[test]
fn an_unexpected_token_reads_without_the_option_wrapper() {
    let error = Parser::from_source("const x: i32 = ;\n")
        .expect("lexes")
        .parse_program()
        .expect_err("the source is deliberately malformed");
    assert_eq!(error.message, "Expected an expression, found Semicolon");

    let at_end = Parser::from_source("const main = (): void => { console.log(1\n")
        .expect("lexes")
        .parse_program()
        .expect_err("the source is deliberately malformed");
    assert!(
        at_end.message.contains("EOF") || at_end.message.contains("end of file"),
        "a failure at end of input must say so: {}",
        at_end.message
    );
}

#[test]
fn a_parser_without_source_ranges_reports_no_span() {
    let tokens = crate::lexer::lex("const x: i32 = ;\n").expect("lexes");
    let error = Parser::new(tokens)
        .parse_program()
        .expect_err("the source is deliberately malformed");
    assert!(
        error.span.is_none(),
        "dummy ranges must not become a caret: {:?}",
        error.span
    );
}

#[test]
fn running_out_of_input_is_reported_as_a_flag_not_as_wording() {
    let incomplete = [
        "const x = (",
        "const main = (): void => {",
        "const numbers: i32[] = [1, 2,",
        "const y: ",
    ];
    for source in incomplete {
        let error = Parser::from_source(source)
            .expect("lexes")
            .parse_program()
            .expect_err("an incomplete prefix does not parse");
        assert!(
            error.at_end_of_input,
            "`{source}` is an incomplete prefix, got: {}",
            error.message
        );
    }

    let genuine = ["console.log(numbers[1];", ")", "const z: = 2;"];
    for source in genuine {
        let error = Parser::from_source(source)
            .expect("lexes")
            .parse_program()
            .expect_err("the source is deliberately malformed");
        assert!(
            !error.at_end_of_input,
            "`{source}` trips on a token, not on the end, got: {}",
            error.message
        );
    }
}

fn parse_error(source: &str) -> String {
    let tokens = crate::lexer::lex(source).unwrap();
    match Parser::new(tokens).parse_program() {
        Ok(_) => panic!("expected a parse error for: {source}"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn readonly_const_wraps_the_annotation_in_readonly() {
    let program = parse("readonly const DAYS: i32[] = [1, 2, 3];");
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { mutable, ty, .. },
        ..
    }) = &program.items[0]
    else {
        panic!("expected variable declaration");
    };
    assert!(!mutable);
    assert!(matches!(ty, Type::Readonly(inner) if matches!(inner.as_ref(), Type::Array(_))));
}

#[test]
fn readonly_const_does_not_double_wrap_a_readonly_annotation() {
    let program = parse("readonly const DAYS: readonly i32[] = [1, 2, 3];");
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { ty, .. },
        ..
    }) = &program.items[0]
    else {
        panic!("expected variable declaration");
    };
    assert!(matches!(ty, Type::Readonly(inner) if matches!(inner.as_ref(), Type::Array(_))));
}

#[test]
fn readonly_const_is_recognized_inside_a_function_body() {
    let program = parse("const f = (): void => { readonly const d: i32[] = [1]; };");
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    let StmtKind::VarDecl { ty, .. } = body[0].kind() else {
        panic!("expected variable declaration");
    };
    assert!(matches!(ty, Type::Readonly(_)));
}

#[test]
fn rejects_const_readonly_with_the_order_as_the_fix() {
    let message = parse_error("const readonly DAYS: i32[] = [1, 2, 3];");
    assert!(
        message.contains("`readonly` goes before `const`"),
        "{message}"
    );
}

#[test]
fn rejects_readonly_let() {
    let message = parse_error("readonly let days: i32[] = [1, 2, 3];");
    assert!(
        message.contains("qualifies `const`, not `let`"),
        "{message}"
    );
}

#[test]
fn rejects_readonly_const_without_an_annotation() {
    let message = parse_error("readonly const DAYS = [1, 2, 3];");
    assert!(message.contains("requires a type annotation"), "{message}");
}

#[test]
fn rejects_readonly_const_on_a_function() {
    let message = parse_error("readonly const f = (): void => {};");
    assert!(
        message.contains("declares a value, not a function"),
        "{message}"
    );
}

#[test]
fn rejects_readonly_const_destructuring() {
    let message = parse_error("readonly const [a, b]: i32[] = [1, 2];");
    assert!(message.contains("cannot destructure"), "{message}");
}

#[test]
fn readonly_in_type_position_is_not_a_binding_qualifier() {
    let program = parse("const f = (xs: readonly i32[]): void => {};");
    assert!(matches!(&program.items[0], Item::FnDecl { .. }));
}

#[test]
fn parentheses_group_a_type() {
    for source in [
        "const fns: ((n: i32) => i32)[] = [];",
        "const grouped: (i32)[] = [];",
        "const nested: (i32)[][] = [];",
        "let maybe: (i32)?;",
        "const fn: (n: i32) => i32 = double;",
        "const bare: (i32) => i32 = double;",
        "const many: (i32, string) => i32 = pair;",
        "const rest: (...args: i32[]) => i32 = sum;",
    ] {
        let tokens = crate::lexer::lex(source).expect("lexes");
        let parsed = Parser::new(tokens).parse_program();
        assert!(
            parsed.is_ok(),
            "expected `{source}` to parse, got {:?}",
            parsed.err()
        );
    }
}

#[test]
fn parses_array_repeat_syntax() {
    let program = parse(r#"const zeros: i32[] = [0; 5];"#);
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { ty, value, .. },
        ..
    }) = &program.items[0]
    else {
        panic!("expected var decl");
    };
    assert_eq!(ty, &Type::Array(Box::new(Type::I32)));
    let value = value.as_ref().expect("expected initializer");
    let Expr {
        kind: ExprKind::ArrayRepeat { value: elem, count },
        ..
    } = value
    else {
        panic!("expected ArrayRepeat, got {:?}", value.kind);
    };
    assert!(
        matches!(elem.kind, ExprKind::IntLit(0)),
        "expected literal 0, got {:?}",
        elem.kind
    );
    assert!(
        matches!(count.kind, ExprKind::IntLit(5)),
        "expected literal 5, got {:?}",
        count.kind
    );
}

#[test]
fn parses_array_repeat_with_expression_value() {
    let program = parse(r#"const xs: i32[] = [1 + 2; 3];"#);
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { value, .. },
        ..
    }) = &program.items[0]
    else {
        panic!("expected var decl");
    };
    let value = value.as_ref().expect("expected initializer");
    let Expr {
        kind: ExprKind::ArrayRepeat { value: elem, count },
        ..
    } = value
    else {
        panic!("expected ArrayRepeat, got {:?}", value.kind);
    };
    assert!(
        matches!(elem.kind, ExprKind::BinOp { .. }),
        "expected binary op"
    );
    assert!(matches!(count.kind, ExprKind::IntLit(3)));
}

#[test]
fn parses_array_repeat_with_const_count() {
    let program = parse(
        r#"
        const N: i32 = 4;
        const xs: i32[] = [0; N];
    "#,
    );
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { value, .. },
        ..
    }) = &program.items[1]
    else {
        panic!("expected var decl");
    };
    let value = value.as_ref().expect("expected initializer");
    let Expr {
        kind: ExprKind::ArrayRepeat { value: elem, count },
        ..
    } = value
    else {
        panic!("expected ArrayRepeat, got {:?}", value.kind);
    };
    assert!(matches!(elem.kind, ExprKind::IntLit(0)));
    assert!(
        matches!(count.kind, ExprKind::Ident(ref s) if s == "N"),
        "expected identifier N, got {:?}",
        count.kind
    );
}

#[test]
fn parses_onfail_block_as_a_variable_declaration_handler() {
    let program = parse("const result = loadUser() onfail { console.error(result.message); };");
    let Item::Stmt(Stmt {
        kind:
            StmtKind::VarDecl {
                name,
                value: Some(value),
                onfail: Some(onfail),
                ..
            },
        ..
    }) = &program.items[0]
    else {
        panic!("expected variable declaration with an onfail handler");
    };

    assert_eq!(name, "result");
    assert!(matches!(value.kind(), ExprKind::FnCall { .. }));
    assert!(matches!(&onfail.handler, OnFailHandler::Block(body) if body.len() == 1));
}

#[test]
fn parses_onfail_expression_as_a_variable_declaration_handler() {
    let program = parse("let result = loadUser() onfail report(result);");
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl {
            onfail: Some(onfail),
            ..
        },
        ..
    }) = &program.items[0]
    else {
        panic!("expected variable declaration with an onfail handler");
    };

    assert!(matches!(
        &onfail.handler,
        OnFailHandler::Expr(Expr {
            kind: ExprKind::FnCall { .. },
            ..
        })
    ));
}

#[test]
fn parses_toggle_with_single_argument() {
    let program = parse(
        "const main = (): void => { let values: i32[] = [1, 2, 3]; const result: i32[] = values.toggle(4); };",
    );
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    let StmtKind::VarDecl {
        value:
            Some(Expr {
                kind: ExprKind::MethodCall { method, args, .. },
                ..
            }),
        ..
    } = body[1].kind()
    else {
        panic!("expected method call");
    };
    assert_eq!(method, "toggle");
    assert_eq!(args.len(), 1);
}

#[test]
fn parses_toggle_with_key_argument() {
    let program = parse(
        r#"interface Item { id: i32; name: string; } const main = (): void => { let items: Item[] = []; const result: Item[] = items.toggle({ id: 1, name: "a" }, "id"); };"#,
    );
    let Item::FnDecl { body, .. } = &program.items[1] else {
        panic!("expected function declaration");
    };
    let StmtKind::VarDecl {
        value:
            Some(Expr {
                kind: ExprKind::MethodCall { method, args, .. },
                ..
            }),
        ..
    } = body[1].kind()
    else {
        panic!("expected method call");
    };
    assert_eq!(method, "toggle");
    assert_eq!(args.len(), 2);
}

#[test]
fn parses_toggle_all() {
    let program = parse(
        "const main = (): void => { let values: i32[] = [1, 2, 3]; const result: i32[] = values.toggleAll([2, 4]); };",
    );
    let Item::FnDecl { body, .. } = &program.items[0] else {
        panic!("expected function declaration");
    };
    let StmtKind::VarDecl {
        value:
            Some(Expr {
                kind: ExprKind::MethodCall { method, args, .. },
                ..
            }),
        ..
    } = body[1].kind()
    else {
        panic!("expected method call");
    };
    assert_eq!(method, "toggleAll");
    assert_eq!(args.len(), 1);
}

#[test]
fn a_catch_without_an_annotation_binds_error_with_an_empty_type_span() {
    let source = "try { risky(); } catch (error) { report(error); }";
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let Item::Stmt(Stmt {
        kind:
            StmtKind::TryCatch {
                catch_binding,
                catch_type,
                catch_type_span,
                ..
            },
        ..
    }) = &program.items[0]
    else {
        panic!("expected try/catch");
    };
    assert_eq!(catch_binding, "error");
    assert!(matches!(catch_type, Type::Named(name) if name == "Error"));
    let binding_end = source.find("error)").unwrap() + "error".len();
    assert_eq!(catch_type_span.start, binding_end);
    assert_eq!(catch_type_span.end, binding_end);
}

#[test]
fn a_catch_annotation_keeps_the_span_of_the_written_type() {
    let source = "try { risky(); } catch (error: Error) { report(error); }";
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let Item::Stmt(Stmt {
        kind: StmtKind::TryCatch {
            catch_type_span, ..
        },
        ..
    }) = &program.items[0]
    else {
        panic!("expected try/catch");
    };
    assert_eq!(&source[catch_type_span.start..catch_type_span.end], "Error");
}

#[test]
fn not_is_after_an_identifier_is_a_negated_type_check_and_not_an_unwrap() {
    let program = parse("const negated = value not is string;");
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl {
            value: Some(value), ..
        },
        ..
    }) = &program.items[0]
    else {
        panic!("expected variable declaration");
    };
    let ExprKind::Not(inner) = value.kind() else {
        panic!("expected `not`, found {:?}", value.kind());
    };
    let ExprKind::Is { expr, ty } = inner.kind() else {
        panic!("expected `is`, found {:?}", inner.kind());
    };
    assert!(matches!(expr.kind(), ExprKind::Ident(name) if name == "value"));
    assert!(matches!(ty, Type::Str));
}

#[test]
fn a_function_type_can_declare_what_it_throws() {
    let program = parse("type Load = (path: string) => string throws FsError | StreamError;");
    let Item::TypeAlias { ty, .. } = &program.items[0] else {
        panic!("expected a type alias, found {:?}", program.items[0]);
    };
    let Type::Throws { function, effects } = ty else {
        panic!("expected a throwing function type, found {ty:?}");
    };
    assert!(matches!(function.as_ref(), Type::Function { .. }));
    assert_eq!(effects.len(), 2);
    assert_eq!(
        ty.to_string(),
        "(string) => string throws FsError | StreamError"
    );
}

#[test]
fn parses_validator_function_keyword_and_preserves_identifier_prefixes() {
    let program = parse(
        r#"validator function usernameWithoutSpaces(value: string): boolean { return !value.includes(" "); } const validatorValue: string = "ok";"#,
    );
    assert!(matches!(
        &program.items[0],
        Item::FnDecl {
            name,
            is_validator: true,
            ..
        } if name == "usernameWithoutSpaces"
    ));
    assert!(matches!(
        &program.items[1],
        Item::Stmt(Stmt {
            kind: StmtKind::VarDecl { name, .. },
            ..
        }) if name == "validatorValue"
    ));
}

#[test]
fn parses_validator_const_arrow_declaration() {
    let program = parse(
        r#"type Validator = { isValid: true } | { isValid: false; message: string }; validator const rule = (value: string, constraint: string): Validator => { if (!value.startsWith(constraint)) { return { isValid: false, message: "Value has the wrong prefix" }; } return { isValid: true }; };"#,
    );
    assert!(matches!(
        &program.items[1],
        Item::FnDecl {
            name,
            is_validator: true,
            params,
            ..
        } if name == "rule" && params.len() == 2
    ));
}

#[test]
fn rejects_validator_modifier_without_function_declaration() {
    let tokens = crate::lexer::lex("validator const rule = true;").unwrap();
    let error = Parser::new(tokens).parse_program().unwrap_err();
    assert!(
        error
            .message
            .contains("validator const` must be initialized with a synchronous arrow function")
    );
}

#[test]
fn keyword_validator_remains_available_as_a_member_name() {
    let program =
        parse("const readRule = (error: ValidationError): string => error.issues[0].validator;");
    assert_eq!(program.items.len(), 1);
}

/// `await` covers its operand up to the first propagating `?`, which then
/// applies to the awaited `Result`; explicit parentheses keep their meaning.
#[test]
fn await_binds_before_the_try_operator() {
    let value = initializer_of("const a = await load()?;");
    let ExprKind::Try(inner) = value.kind() else {
        panic!("`await load()?` should be a try of the await, got {value:?}");
    };
    assert!(matches!(inner.kind(), ExprKind::Await(_)), "{inner:?}");

    let value = initializer_of("const a = await load()? + 1;");
    let ExprKind::BinOp { left, .. } = value.kind() else {
        panic!("expected an addition, got {value:?}");
    };
    let ExprKind::Try(inner) = left.kind() else {
        panic!("the left operand should be the try, got {left:?}");
    };
    assert!(matches!(inner.kind(), ExprKind::Await(_)), "{inner:?}");

    let value = initializer_of("const a = (await load()?).name;");
    let ExprKind::FieldAccess { object, .. } = value.kind() else {
        panic!("expected a field access, got {value:?}");
    };
    assert!(matches!(object.kind(), ExprKind::Try(_)), "{object:?}");

    let value = initializer_of("const a = await (load()?);");
    let ExprKind::Await(inner) = value.kind() else {
        panic!("parentheses should keep the await outside, got {value:?}");
    };
    assert!(matches!(inner.kind(), ExprKind::Try(_)), "{inner:?}");

    let value = initializer_of("const a = await load(check()?);");
    let ExprKind::Await(inner) = value.kind() else {
        panic!("a try inside an argument stays inside, got {value:?}");
    };
    assert!(matches!(inner.kind(), ExprKind::FnCall { .. }), "{inner:?}");

    let value = initializer_of("const a = await client.fetch().json()?;");
    let ExprKind::Try(inner) = value.kind() else {
        panic!("the whole call chain is awaited first, got {value:?}");
    };
    let ExprKind::Await(chain) = inner.kind() else {
        panic!("expected the await under the try, got {inner:?}");
    };
    assert!(
        matches!(chain.kind(), ExprKind::MethodCall { .. }),
        "{chain:?}"
    );
}
