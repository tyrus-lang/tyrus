use super::*;
use crate::ast::ExprKind;

fn parse_ok(source: &str) -> Vec<EnvVarDecl> {
    let (vars, diagnostics) = parse_env_source(source);
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );
    vars
}

fn parse_errors(source: &str) -> Vec<AnalysisDiagnostic> {
    let (_, diagnostics) = parse_env_source(source);
    diagnostics
}

#[test]
fn a_env_diagnostic_never_prints_the_value_it_rejects() {
    const SECRET: &str = "sk-live-super-secret-abc123";
    let source = format!("@type(i32)\n@port()\nAPI_TOKEN={SECRET}\n");
    let vars = parse_ok(&source);
    let diagnostics = check_env_schema(&vars);
    assert!(!diagnostics.is_empty(), "the value is not a valid `i32`");

    for diagnostic in &diagnostics {
        assert!(
            !diagnostic.message.contains(SECRET),
            "the message published the secret: {}",
            diagnostic.message
        );
        let rendered =
            render_env_diagnostic(std::path::Path::new(".env"), &source, &vars, diagnostic);
        assert!(
            !rendered.contains(SECRET),
            "the rendered diagnostic published the secret:\n{rendered}"
        );
        assert!(
            rendered.contains("API_TOKEN=<redacted>"),
            "the frame must still show the variable it is about:\n{rendered}"
        );
    }
}

#[test]
fn redacting_a_value_keeps_the_other_ranges_pointing_at_the_right_place() {
    let source = "@type(i32)\nFIRST=a-very-long-invalid-value\n\n@type(i32)\nSECOND=also-invalid\n";
    let vars = parse_ok(source);
    let diagnostics = check_env_schema(&vars);
    let second = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.message.contains("SECOND"))
        .expect("`SECOND` does not hold an `i32` either");

    let rendered = render_env_diagnostic(std::path::Path::new(".env"), source, &vars, second);
    assert!(
        rendered.contains("SECOND=<redacted>"),
        "the caret drifted off the variable it is about:\n{rendered}"
    );
    assert!(!rendered.contains("also-invalid"));
    assert!(!rendered.contains("a-very-long-invalid-value"));
}

#[test]
fn splits_key_and_value_at_the_first_equals_sign() {
    let vars = parse_ok("SECRET=key=with=multiple=equals\n");
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].name, "SECRET");
    assert_eq!(vars[0].raw_value, "key=with=multiple=equals");
    assert!(matches!(vars[0].declared_type, Type::Str));
}

#[test]
fn records_name_and_value_ranges() {
    let source = "PORT=3000\n";
    let vars = parse_ok(source);
    assert_eq!(&source[vars[0].name_range.clone()], "PORT");
    assert_eq!(&source[vars[0].value_range.clone()], "3000");
}

#[test]
fn accepts_raw_and_comment_prefixed_decorators() {
    let source = "@min(1024)\n# @max(49151)\n@type(u32)\nPORT=3000\n";
    let vars = parse_ok(source);
    assert_eq!(vars.len(), 1);
    assert!(matches!(vars[0].declared_type, Type::U32));
    let names: Vec<&str> = vars[0]
        .decorators
        .iter()
        .map(|decorator| decorator.name.as_str())
        .collect();
    assert_eq!(names, ["min", "max"]);
    assert_eq!(&source[vars[0].decorator_ranges[0].clone()], "@min(1024)");
    assert_eq!(&source[vars[0].decorator_ranges[1].clone()], "@max(49151)");
}

#[test]
fn parses_type_decorator_with_message() {
    let vars = parse_ok("@type(u32, \"the port must be a positive number\")\nPORT=3000\n");
    assert!(matches!(vars[0].declared_type, Type::U32));
    assert_eq!(
        vars[0].type_message.as_deref(),
        Some("the port must be a positive number")
    );
}

#[test]
fn type_message_may_contain_commas_and_equals() {
    let vars = parse_ok("@type(string, \"one, two = three\")\nGREETING=hello\n");
    assert_eq!(vars[0].type_message.as_deref(), Some("one, two = three"));
}

#[test]
fn variables_without_type_default_to_string() {
    let vars = parse_ok("TYRUS_ENV=development\n");
    assert!(matches!(vars[0].declared_type, Type::Str));
    assert!(vars[0].type_message.is_none());
    assert!(vars[0].decorators.is_empty());
}

#[test]
fn empty_value_is_an_error() {
    let source = "@type(string)\nTOKEN=\n";
    let (vars, parse_diagnostics) = parse_env_source(source);
    assert!(parse_diagnostics.is_empty());
    assert_eq!(vars[0].raw_value, "");
    let diagnostics = check_env_schema(&vars);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "E3148");
    assert_eq!(&source[diagnostics[0].range.clone()], "TOKEN");
}

#[test]
fn requires_a_type_declaration() {
    let source = "SECRET=abc\n";
    let (vars, parse_diagnostics) = parse_env_source(source);
    assert!(parse_diagnostics.is_empty());
    let diagnostics = check_env_schema(&vars);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "E3147");
    assert_eq!(&source[diagnostics[0].range.clone()], "SECRET");
    assert!(!diagnostics[0].help.is_empty());
}

#[test]
fn plain_comments_and_blank_lines_are_ignored() {
    let vars = parse_ok("# database settings\n\nDB_HOST=localhost\n");
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].name, "DB_HOST");
}

#[test]
fn reports_unknown_type() {
    let source = "@type(u16)\nPORT=3000\n";
    let diagnostics = parse_errors(source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "E3149");
    assert_eq!(&source[diagnostics[0].range.clone()], "u16");
    assert!(!diagnostics[0].help.is_empty());
}

#[test]
fn reports_duplicate_type_decorator() {
    let diagnostics = parse_errors("@type(u32)\n@type(i64)\nPORT=3000\n");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "E3146");
    assert!(diagnostics[0].message.contains("duplicate `@type`"));
}

#[test]
fn reports_duplicate_variable_with_related_first_declaration() {
    let source = "PORT=3000\nPORT=4000\n";
    let diagnostics = parse_errors(source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "E3142");
    assert_eq!(&source[diagnostics[0].range.clone()], "PORT");
    assert_eq!(
        diagnostics[0].range.start,
        source.find("PORT=4000").unwrap()
    );
    assert_eq!(diagnostics[0].related.len(), 1);
    assert_eq!(diagnostics[0].related[0].range.start, 0);
}

#[test]
fn reports_orphan_decorators_at_end_of_file() {
    let source = "PORT=3000\n@min(1)\n";
    let diagnostics = parse_errors(source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "E3146");
    assert_eq!(&source[diagnostics[0].range.clone()], "@min(1)");
}

#[test]
fn reports_invalid_lines_and_names() {
    let diagnostics = parse_errors("not a variable line\n");
    assert_eq!(diagnostics[0].code, "E3146");

    let diagnostics = parse_errors("MY-NAME=value\n");
    assert_eq!(diagnostics[0].code, "E3146");
    assert!(diagnostics[0].message.contains("MY-NAME"));
}

#[test]
fn checks_value_against_declared_type() {
    let source = "@type(u32)\nPORT=not-a-number\n";
    let (vars, parse_diagnostics) = parse_env_source(source);
    assert!(parse_diagnostics.is_empty());
    let diagnostics = check_env_schema(&vars);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "E3143");
    assert_eq!(&source[diagnostics[0].range.clone()], "not-a-number");
    assert_eq!(diagnostics[0].related.len(), 1);
    assert_eq!(
        &source[diagnostics[0].related[0].range.clone()],
        "@type(u32)"
    );
}

#[test]
fn type_failure_uses_the_custom_type_message() {
    let (vars, _) = parse_env_source("@type(u32, \"the port must be a number\")\nPORT=abc\n");
    let diagnostics = check_env_schema(&vars);
    assert!(diagnostics[0].message.contains("the port must be a number"));
}

#[test]
fn checks_value_against_validator_decorators() {
    let source = "@type(u32)\n@min(1024, \"use safe ports above 1024\")\nPORT=80\n";
    let (vars, _) = parse_env_source(source);
    let diagnostics = check_env_schema(&vars);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "E3143");
    assert!(diagnostics[0].message.contains("use safe ports above 1024"));
    assert_eq!(&source[diagnostics[0].range.clone()], "80");
    assert_eq!(
        &source[diagnostics[0].related[0].range.clone()],
        "@min(1024, \"use safe ports above 1024\")"
    );
}

#[test]
fn string_validators_apply_to_string_variables() {
    let (vars, _) = parse_env_source("@type(string)\n@email\nCONTACT=not-an-email\n");
    let diagnostics = check_env_schema(&vars);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "E3143");
}

#[test]
fn string_length_validator_rejects_invalid_env_values() {
    let source = "@type(string)\n@minLength(5)\nTEST=test\n";
    let (vars, parse_diagnostics) = parse_env_source(source);
    assert!(parse_diagnostics.is_empty());

    let diagnostics = check_env_schema(&vars);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "E3143");
    assert_eq!(&source[diagnostics[0].range.clone()], "test");
    assert_eq!(
        &source[diagnostics[0].related[0].range.clone()],
        "@minLength(5)"
    );
}

#[test]
fn port_validator_rejects_unregistered_env_ports() {
    let source = "@type(u32)\n@port\nPORT=1000\n";
    let (vars, parse_diagnostics) = parse_env_source(source);
    assert!(parse_diagnostics.is_empty());

    let diagnostics = check_env_schema(&vars);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "E3143");
    assert_eq!(&source[diagnostics[0].range.clone()], "1000");
    assert_eq!(&source[diagnostics[0].related[0].range.clone()], "@port");
}

#[test]
fn reports_validator_incompatible_with_declared_type() {
    let source = "@type(u32)\n@minLength(3)\nPORT=3000\n";
    let (vars, _) = parse_env_source(source);
    let diagnostics = check_env_schema(&vars);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "E3161");
    assert_eq!(&source[diagnostics[0].range.clone()], "@minLength(3)");
}

#[test]
fn reports_unknown_validator() {
    let source = "@type(u32)\n@bogus(1)\nPORT=3000\n";
    let (vars, _) = parse_env_source(source);
    let diagnostics = check_env_schema(&vars);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "E3160");
    assert_eq!(&source[diagnostics[0].range.clone()], "@bogus(1)");
}

#[test]
fn reports_contradictory_validator_set() {
    let source = "@type(u32)\n@min(2000)\n@max(1000)\nPORT=1500\n";
    let (vars, _) = parse_env_source(source);
    let diagnostics = check_env_schema(&vars);
    assert!(diagnostics.iter().any(|d| d.code == "E3163"));
}

#[test]
fn env_example_clears_values_and_preserves_structure() {
    let source = "# service settings\n@type(u32, \"a port number\")\n# @min(1024)\nPORT=3000\n\nSECRET=key=with=equals\nEMPTY=\n";
    let expected = "# service settings\n@type(u32, \"a port number\")\n# @min(1024)\nPORT=\n\nSECRET=\nEMPTY=\n";
    assert_eq!(render_env_example(source), expected);
}

#[test]
fn env_example_preserves_crlf_and_missing_final_newline() {
    assert_eq!(render_env_example("PORT=3000\r\nHOST=x"), "PORT=\r\nHOST=");
}

#[test]
fn object_type_exposes_declared_variables_as_readonly_fields() {
    let (vars, _) = parse_env_source("@type(u32)\nPORT=3000\nTYRUS_ENV=development\n");
    let schema = EnvSchema {
        env_file: ".env".to_string(),
        file_path: std::path::PathBuf::from(".env"),
        source: String::new(),
        vars,
    };
    let fields = schema.object_type();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "PORT");
    assert!(matches!(fields[0].ty, Type::U32));
    assert!(fields[0].readonly);
    assert!(!fields[0].optional);
    assert_eq!(fields[1].name, "TYRUS_ENV");
    assert!(matches!(fields[1].ty, Type::Str));
    assert!(schema.var("PORT").is_some());
    assert!(schema.var("MISSING").is_none());
}

#[test]
fn parses_boolean_and_float_values() {
    let (vars, _) = parse_env_source("@type(boolean)\nDEBUG=true\n@type(f64)\nRATIO=0.5\n");
    assert!(check_env_schema(&vars).is_empty());
    assert!(matches!(
        parse_env_value("true", &Type::Bool),
        Some(Expr {
            kind: ExprKind::BoolLit(true),
            ..
        })
    ));
    assert!(parse_env_value("yes", &Type::Bool).is_none());
    assert!(parse_env_value("4294967296", &Type::U32).is_none());
    assert!(matches!(
        parse_env_value("4294967295", &Type::U32),
        Some(Expr {
            kind: ExprKind::IntLit(4294967295),
            ..
        })
    ));
}

#[test]
fn bare_key_without_equals_requires_a_value() {
    let source = "@type(string)\nGOOGLE_SECRET_KEY\n";
    let (vars, diagnostics) = parse_env_source(source);
    assert!(diagnostics.is_empty(), "unexpected: {diagnostics:?}");
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].name, "GOOGLE_SECRET_KEY");
    assert_eq!(vars[0].raw_value, "");
    let diagnostics = check_env_schema(&vars);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "E3148");
    assert_eq!(&source[diagnostics[0].range.clone()], "GOOGLE_SECRET_KEY");
}

#[test]
fn bare_key_is_preserved_in_the_env_example() {
    let source = "@type(string)\nGOOGLE_SECRET_KEY\n\n@type(u32)\nPORT=3000\n";
    let expected = "@type(string)\nGOOGLE_SECRET_KEY\n\n@type(u32)\nPORT=\n";
    assert_eq!(render_env_example(source), expected);
}

#[test]
fn strips_surrounding_quotes_from_values() {
    let vars = parse_ok("@type(string)\nA=\"hello world\"\n@type(string)\nB='literal'\n");
    assert_eq!(vars[0].raw_value, "hello world");
    assert_eq!(vars[1].raw_value, "literal");
    let source = "@type(string)\nA=\"hello world\"\n";
    let (v, _) = parse_env_source(source);
    assert_eq!(&source[v[0].value_range.clone()], "\"hello world\"");
}

#[test]
fn double_quotes_process_escapes_single_quotes_do_not() {
    let vars = parse_ok("@type(string)\nA=\"a\\nb\"\n@type(string)\nB='a\\nb'\n");
    assert_eq!(vars[0].raw_value, "a\nb");
    assert_eq!(vars[1].raw_value, "a\\nb");
}

#[test]
fn expands_variable_interpolation() {
    let vars = parse_ok(
        "@type(string)\nHOST=localhost\n@type(u32)\nPORT=3000\n@type(string)\nURL=http://${HOST}:${PORT}/api\n",
    );
    assert_eq!(vars[2].raw_value, "http://localhost:3000/api");
    assert!(check_env_schema(&vars).is_empty());
}

#[test]
fn single_quoted_values_are_not_interpolated() {
    let vars = parse_ok("@type(string)\nA=x\n@type(string)\nB='${A}'\n");
    assert_eq!(vars[1].raw_value, "${A}");
}

#[test]
fn reports_undefined_interpolation_reference() {
    let source = "@type(string)\nURL=${MISSING}/api\n";
    let diagnostics = parse_errors(source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "E3150");
    assert_eq!(&source[diagnostics[0].range.clone()], "${MISSING}/api");
    assert!(diagnostics[0].message.contains("MISSING"));
}

#[test]
fn reports_cyclic_interpolation() {
    let source = "@type(string)\nA=${B}\n@type(string)\nB=${A}\n";
    let diagnostics = parse_errors(source);
    assert!(diagnostics.iter().all(|d| d.code == "E3151"));
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn interpolated_value_is_validated_against_the_type() {
    let source = "@type(string)\nA=abc\n@type(u32)\nPORT=${A}\n";
    let (vars, _) = parse_env_source(source);
    let diagnostics = check_env_schema(&vars);
    assert!(diagnostics.iter().any(|d| d.code == "E3143"));
}

#[test]
fn accepts_export_prefix_with_commented_decorators() {
    let source = "# @type(u32)\n# @port()\nexport PORT=3000\n";
    let vars = parse_ok(source);
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].name, "PORT");
    assert_eq!(vars[0].raw_value, "3000");
    assert_eq!(&source[vars[0].name_range.clone()], "PORT");
    assert!(check_env_schema(&vars).is_empty());
}

#[test]
fn export_flags_every_raw_decorator_in_the_file() {
    let source = "@type(string)\nHOST=localhost\n\n@type(u32)\n@port()\nexport PORT=3000\n";
    let diagnostics = parse_errors(source);
    assert_eq!(diagnostics.len(), 3);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code == "E3152")
    );
    assert_eq!(&source[diagnostics[0].range.clone()], "@type(string)");
    assert_eq!(&source[diagnostics[1].range.clone()], "@type(u32)");
    assert_eq!(&source[diagnostics[2].range.clone()], "@port()");
    assert!(diagnostics[0].help.iter().any(|help| help.contains("# ")));
}

#[test]
fn raw_decorators_are_fine_without_export() {
    let vars = parse_ok("@type(u32)\n@port()\nPORT=3000\n");
    assert_eq!(vars[0].name, "PORT");
    assert!(check_env_schema(&vars).is_empty());
}
