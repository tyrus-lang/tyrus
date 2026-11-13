use super::*;

#[test]
fn skips_long_unicode_block_comments_without_exhausting_the_stack() {
    let rule = char::from_u32(0x2500).expect("box drawing character");
    let source = format!(
        "/* {} */ const answer = 42;",
        rule.to_string().repeat(1_521)
    );
    let tokens = crate::lexer::lex(&source).expect("lex source after block comment");
    assert_eq!(tokens.first(), Some(&Token::Const));
}

#[test]
fn reports_an_unterminated_block_comment_at_its_source_range() {
    let source = "const answer = 42; /* unfinished";
    let error = crate::lexer::lex(source).expect_err("unterminated block comment");
    assert_eq!(error.range, 19..source.len());
}

#[test]
fn accepts_single_and_double_quoted_strings() {
    let source = r#""It's tyrus" 'She said "hello"'"#;
    let tokens = Token::lexer(source).collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(
        tokens,
        vec![
            Token::StringLit("It's tyrus".to_string()),
            Token::StringLit("She said \"hello\"".to_string()),
        ]
    );
}

#[test]
fn tokenizes_special_floating_point_literals() {
    let tokens = crate::lexer::lex("Infinity NaN").unwrap();
    assert!(
        matches!(tokens[0], Token::SpecialFloatLit(ref value) if value.value() == f64::INFINITY && value.text() == "Infinity")
    );
    assert!(
        matches!(tokens[1], Token::SpecialFloatLit(ref value) if value.value().is_nan() && value.text() == "NaN")
    );
}

#[test]
fn tokenizes_regular_expression_literals() {
    let tokens = crate::lexer::lex(r"/^[a-z0-9-]+$/i").unwrap();
    assert_eq!(
        tokens,
        vec![Token::RegexLit(crate::lexer::RegexLiteral {
            pattern: "^[a-z0-9-]+$".to_string(),
            flags: "i".to_string(),
        })]
    );
}

#[test]
fn keeps_spaced_slash_as_division_operator() {
    let tokens = crate::lexer::lex("a / b").unwrap();
    assert_eq!(
        tokens,
        vec![
            Token::Ident("a".to_string()),
            Token::Slash,
            Token::Ident("b".to_string()),
        ]
    );
}

#[test]
fn tokenizes_logical_assignment_operators() {
    let tokens = Token::lexer("&&= ||= ??=")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        tokens,
        vec![Token::AndEq, Token::OrEq, Token::NullCoalesceEq]
    );
}

#[test]
fn written_logical_operators_lex_to_the_same_tokens_as_symbols() {
    let symbolic = Token::lexer("&& || !")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let written = Token::lexer("and or not")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(symbolic, vec![Token::And, Token::Or, Token::Bang]);
    assert_eq!(written, symbolic);
}

#[test]
fn tokenizes_array_delimiters() {
    let tokens = Token::lexer("string[] [\"GET\"]")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        tokens,
        vec![
            Token::TyString,
            Token::LBracket,
            Token::RBracket,
            Token::LBracket,
            Token::StringLit("GET".to_string()),
            Token::RBracket,
        ]
    );
}

#[test]
fn tokenizes_full_numeric_type_set() {
    let tokens = Token::lexer("i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 f128")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        tokens,
        vec![
            Token::TyI8,
            Token::TyI16,
            Token::TyI32,
            Token::TyI64,
            Token::TyI128,
            Token::TyIsize,
            Token::TyU8,
            Token::TyU16,
            Token::TyU32,
            Token::TyU64,
            Token::TyU128,
            Token::TyUsize,
            Token::TyF32,
            Token::TyF64,
            Token::TyF128,
        ]
    );
}

#[test]
#[allow(clippy::approx_constant)]
fn tokenizes_typed_numeric_literals_as_single_tokens() {
    assert_eq!(
        crate::lexer::lex("42u8 1000i16 3.14f32 2.0f128").unwrap(),
        vec![
            Token::TypedUIntLit((42, crate::ast::Type::U8)),
            Token::TypedIntLit((1000, crate::ast::Type::I16)),
            Token::TypedFloatLit((
                crate::ast::FloatLiteral::new(3.14, "3.14"),
                crate::ast::Type::F32,
            )),
            Token::TypedFloatLit((
                crate::ast::FloatLiteral::new(2.0, "2.0"),
                crate::ast::Type::F128,
            )),
        ]
    );
}

#[test]
fn rejects_unknown_numeric_suffixes() {
    assert!(crate::lexer::lex("42x").is_err());
}

#[test]
fn accepts_u128_literals_above_i128_max() {
    assert_eq!(
        crate::lexer::lex("340282366920938463463374607431768211455u128").unwrap(),
        vec![Token::TypedUIntLit((u128::MAX, crate::ast::Type::U128))]
    );
}

#[test]
fn tokenizes_the_char_type_keyword() {
    let tokens = Token::lexer("char").collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(tokens, vec![Token::TyChar]);
}

#[test]
fn tokenizes_defer_keywords_as_dedicated_tokens() {
    let tokens = Token::lexer("defer errdefer deferred errdeferred")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        tokens,
        vec![
            Token::Defer,
            Token::ErrDefer,
            Token::Ident("deferred".to_string()),
            Token::Ident("errdeferred".to_string()),
        ]
    );
}

#[test]
fn tokenizes_onfail_as_a_dedicated_keyword() {
    let tokens = Token::lexer("onfail onfailure")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(
        tokens,
        vec![Token::OnFail, Token::Ident("onfailure".to_string())]
    );
}

#[test]
fn tokenizes_integer_literals_wider_than_i64() {
    let tokens = Token::lexer("3_000_000_000_000_000_000_000")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(tokens, vec![Token::IntLit(3_000_000_000_000_000_000_000)]);
}

#[test]
fn decodes_escaped_delimiters_and_control_characters() {
    let source = r#""She said \"hello\"\n" 'It\'s tyrus\t!'"#;
    let tokens = Token::lexer(source).collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(
        tokens,
        vec![
            Token::StringLit("She said \"hello\"\n".to_string()),
            Token::StringLit("It's tyrus\t!".to_string()),
        ]
    );
}

#[test]
fn decodes_unicode_codepoint_escapes_in_strings() {
    let source = r#""Valor m\u{ed}nimo permitido \u00e9 3 \u{1f600}" "\uD83D\uDE00""#;
    let tokens = Token::lexer(source).collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(
        tokens,
        vec![
            Token::StringLit("Valor mínimo permitido é 3 😀".to_string()),
            Token::StringLit("😀".to_string()),
        ]
    );
}

#[test]
fn tokenizes_hexadecimal_numbers() {
    let tokens = Token::lexer("0xFF 0x1A 0xff 0xFFF_123")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        tokens,
        vec![
            Token::IntLit(255),
            Token::IntLit(26),
            Token::IntLit(255),
            Token::IntLit(0xFFF123),
        ]
    );
}

#[test]
fn tokenizes_binary_numbers() {
    let tokens = Token::lexer("0b1010 0B1111 0b10_10")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        tokens,
        vec![Token::IntLit(10), Token::IntLit(15), Token::IntLit(10),]
    );
}

#[test]
fn tokenizes_octal_numbers() {
    let tokens = Token::lexer("0o77 0O755 0o1_2_3")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        tokens,
        vec![Token::IntLit(63), Token::IntLit(493), Token::IntLit(83),]
    );
}

#[test]
fn tokenizes_scientific_notation() {
    let tokens = Token::lexer("1.5e10 2.5E-3 1e5 3E+2")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let literals = tokens
        .iter()
        .map(|token| match token {
            Token::FloatLit(literal) => (literal.value(), literal.text()),
            other => panic!("expected a float literal, found {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        literals,
        vec![
            (1.5e10, "1.5e10"),
            (2.5e-3, "2.5E-3"),
            (1e5, "1e5"),
            (300.0, "3E+2"),
        ]
    );
}

#[test]
fn tokenizes_decimal_with_underscores() {
    let tokens = Token::lexer("1_000 10_000 1_000_000")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        tokens,
        vec![
            Token::IntLit(1000),
            Token::IntLit(10000),
            Token::IntLit(1000000),
        ]
    );
}

#[test]
fn rejects_misplaced_decimal_underscores() {
    for source in ["1_000_", "1_00_0_000", "10_00", "1__000"] {
        assert!(
            Token::lexer(source).collect::<Result<Vec<_>, _>>().is_err(),
            "`{source}` should be rejected as an invalid decimal literal"
        );
        assert!(
            crate::lexer::lex(source).is_err(),
            "`{source}` should fail through the public lexer"
        );
    }
}

#[test]
fn reports_invalid_token_range() {
    let source = "const value = 1_000_;\n";
    let error = crate::lexer::lex(source).expect_err("invalid separator should fail lexing");
    let start = source.find("1_000_").unwrap();
    assert_eq!(error.range, start..start + "1_000_".len());
}

#[test]
fn tokenizes_multiline_string_with_dedent() {
    let source = r#"const query = """
    SELECT id, nome
    FROM usuarios
    WHERE ativo = true
    """;"#;
    let tokens = crate::lexer::lex(source).unwrap();
    assert_eq!(
        tokens,
        vec![
            Token::Const,
            Token::Ident("query".to_string()),
            Token::Eq,
            Token::StringLit("SELECT id, nome\nFROM usuarios\nWHERE ativo = true".to_string()),
            Token::Semicolon,
        ]
    );
}

#[test]
fn tokenizes_multiline_string_with_zero_closing_indent() {
    let source = r#"const x = """
line1
    line2
""";"#;
    let tokens = crate::lexer::lex(source).unwrap();
    assert_eq!(
        tokens,
        vec![
            Token::Const,
            Token::Ident("x".to_string()),
            Token::Eq,
            Token::StringLit("line1\n    line2".to_string()),
            Token::Semicolon,
        ]
    );
}

#[test]
fn tokenizes_multiline_string_with_first_line_empty() {
    let source = r#"const x = """
    line1
    line2
    """;"#;
    let tokens = crate::lexer::lex(source).unwrap();
    assert_eq!(
        tokens,
        vec![
            Token::Const,
            Token::Ident("x".to_string()),
            Token::Eq,
            Token::StringLit("line1\nline2".to_string()),
            Token::Semicolon,
        ]
    );
}

#[test]
fn tokenizes_multiline_string_ignoring_tab_indentation() {
    let source = "const x = \"\"\"\n\tline1\n\tline2\n\t\"\"\";";
    let tokens = crate::lexer::lex(source).unwrap();
    assert_eq!(
        tokens,
        vec![
            Token::Const,
            Token::Ident("x".to_string()),
            Token::Eq,
            Token::StringLit("\tline1\n\tline2".to_string()),
            Token::Semicolon,
        ]
    );
}

#[test]
fn tokenizes_multiline_string_dedent_ignores_trailing_source() {
    let source = r#"function run(): void {
    const query = """
        SELECT id
        FROM usuarios
        """;
    console.log(query);
}"#;
    let tokens = crate::lexer::lex(source).unwrap();
    assert!(tokens.contains(&Token::StringLit("SELECT id\nFROM usuarios".to_string())));
}

#[test]
fn rejects_unterminated_multiline_string() {
    let source = r#"const x = """
    line1
    line2"#;
    let error =
        crate::lexer::lex(source).expect_err("unterminated multiline string should fail lexing");
    let start = source.find("\"\"\"").unwrap();
    assert_eq!(error.range, start..source.len());
}
