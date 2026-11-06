use super::*;

#[test]
fn original_global_name_undoes_only_a_real_module_prefix() {
    use crate::module_loader::original_global_name;

    assert_eq!(
        original_global_name("__tyrus_module_1_ownerText"),
        "ownerText"
    );
    assert_eq!(original_global_name("__tyrus_module_42_a_b"), "a_b");
    assert_eq!(original_global_name("main"), "main");
    assert_eq!(
        original_global_name("__tyrus_module_x_ownerText"),
        "__tyrus_module_x_ownerText"
    );
    assert_eq!(original_global_name("__tyrus_module_"), "__tyrus_module_");
}

#[test]
fn a_parse_error_with_a_span_is_rendered_as_a_code_frame() {
    let source = "const render = (out: string): string => out;\n";
    let error = Parser::from_source(source)
        .expect("lexes")
        .parse_program()
        .expect_err("`out` is reserved and must not parse");
    let rendered = render_parse_error(Path::new("main.tyr"), source, &error);

    assert!(
        rendered.starts_with("error[E0003]:"),
        "the parser's own code must survive rendering, got: {rendered}"
    );
    assert!(
        rendered.contains("main.tyr:1:17"),
        "the frame must point at `out`, got: {rendered}"
    );
    assert!(
        rendered.contains("^^^"),
        "the caret must underline the offending name, got: {rendered}"
    );
    assert!(
        rendered.contains("help: rename `out`"),
        "the help must survive and name the identifier, got: {rendered}"
    );
}

#[test]
fn a_parse_error_without_a_span_keeps_the_plain_message() {
    let error = ParseError::new("Unexpected token in expression: Some(Semicolon)");
    let rendered = render_parse_error(Path::new("main.tyr"), "const x: i32 = ;\n", &error);
    assert_eq!(
        rendered,
        "error[parse] in `main.tyr`: Unexpected token in expression: Some(Semicolon)"
    );
}

#[test]
fn entry_declarations_are_mangled_except_main() {
    let root = std::env::temp_dir().join(format!("tyrus-entry-mangling-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            const read = (): i32 => 42;
            const main = (): void => {
                console.log(read());
            };
        "#,
    )
    .unwrap();

    let program = load_program(&root.join("main.tyr"), None).unwrap();
    let names = program
        .items
        .iter()
        .filter_map(|item| match item {
            Item::FnDecl { name, .. } => Some(name.as_str()),
            Item::Stmt(Stmt {
                kind: StmtKind::VarDecl { name, .. },
                ..
            }) => Some(name.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["__tyrus_module_0_read", "main"]);
    assert_eq!(entry_symbol("read"), "__tyrus_module_0_read");
    assert_eq!(entry_symbol("main"), "main");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn switch_case_labels_are_renamed_like_any_other_expression() {
    let root = std::env::temp_dir().join(format!("tyrus-switch-rename-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            const label = (): string => "first";
            const main = (): void => {
                switch ("first") {
                    case label(): {
                        console.log("matched");
                        break;
                    }
                    default: {
                        break;
                    }
                }
            };
        "#,
    )
    .unwrap();

    let program = load_program(&root.join("main.tyr"), None).unwrap();
    let body = match program.items.last() {
        Some(Item::FnDecl { body, .. }) => body.clone(),
        Some(Item::Stmt(Stmt {
            kind: StmtKind::VarDecl {
                value: Some(main), ..
            },
            ..
        })) => match main.kind() {
            ExprKind::ArrowFn { body, .. } => body.clone(),
            other => panic!("expected `main` to be an arrow function, got {other:?}"),
        },
        other => panic!("expected `main` to be the last item, got {other:?}"),
    };
    let Some(Stmt {
        kind: StmtKind::Switch { cases, .. },
        ..
    }) = body.first()
    else {
        panic!("expected a `switch` statement in `main`");
    };
    assert!(matches!(
        cases[0].value.kind(),
        ExprKind::FnCall { callee, .. } if callee == "__tyrus_module_0_label"
    ));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_native_export_answers_with_the_return_type_it_declares() {
    use crate::ast::Type;
    use crate::module_loader::native_target_return_type;

    assert_eq!(
        native_target_return_type("__tyrus_http_createRouter"),
        Some(Type::Named("HttpRouter".to_string())),
    );
    assert_eq!(
        native_target_return_type("__tyrus_http_fetch"),
        Some(Type::Apply {
            name: "Promise".to_string(),
            args: vec![Type::Named("HttpClientResponse".to_string())],
        }),
    );
    assert_eq!(
        native_target_return_type("__tyrus_http_connectWebSocket"),
        Some(Type::Apply {
            name: "Promise".to_string(),
            args: vec![Type::Result {
                success: Box::new(Type::Named("WebSocket".to_string())),
                error: Box::new(Type::Named("HttpError".to_string())),
            }],
        }),
    );
    assert_eq!(native_target_return_type("__tyrus_not_an_export"), None);
}

#[test]
fn an_aliased_native_import_may_leave_out_its_trailing_optional_arguments() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-aliased-optional-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { DirEntry, FsError, readdir as list } from "tyrus:fs";
            try {
                const entries: Result<DirEntry[], FsError> = list(".");
                match (entries) {
                    Ok(value) => { console.log(value.length); }
                    Err(error) => { console.error(error.code); }
                }
            } catch (error: FsError) {
                console.error(error.code);
            }
        "#,
    )
    .unwrap();
    let result = load_program(&root.join("main.tyr"), None);
    fs::remove_dir_all(&root).unwrap();
    result.expect("an alias accepts the call its export accepts");
}

#[test]
fn resolves_exported_custom_validator_through_import_alias() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-validator-module-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("validators.tyr"),
        r#"type Validator = { isValid: true } | { isValid: false; message: string }; export validator function noSpaces(value: string): Validator { if (value.includes(" ")) { return { isValid: false, message: "Value cannot contain spaces" }; } return { isValid: true }; }"#,
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"import { noSpaces as usernameRule } from "./validators"; interface Account { @usernameRule username: string; }"#,
    )
    .unwrap();
    let program = load_program(&root.join("main.tyr"), None).unwrap();
    let validator_name = program
        .items
        .iter()
        .find_map(|item| match item {
            Item::FnDecl {
                name,
                is_validator: true,
                ..
            } => Some(name.clone()),
            _ => None,
        })
        .expect("expected the imported validator declaration");
    let field = program
        .items
        .iter()
        .find_map(|item| match item {
            Item::InterfaceDecl { name, fields, .. }
                if crate::module_loader::original_global_name(name) == "Account" =>
            {
                fields.first()
            }
            _ => None,
        })
        .expect("expected the imported validator target field");
    assert_eq!(field.decorators[0].name, validator_name);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn db_error_is_an_import_only_native_interface() {
    let (ty, fields, alias) = native_export_info("tyrus:db", "DbError").unwrap();
    assert_eq!(ty, Type::Named("DbError".to_string()));
    let fields = fields.unwrap();
    assert_eq!(
        fields
            .iter()
            .map(|field| (field.name.as_str(), field.ty.clone(), field.readonly))
            .collect::<Vec<_>>(),
        vec![("code", Type::Str, true), ("message", Type::Str, true),]
    );
    assert_eq!(fields, crate::builtin_interfaces::db_error_fields());
    assert_eq!(
        crate::builtin_interfaces::builtin_interfaces().get("DbError"),
        Some(&fields)
    );
    assert!(alias.is_none());
    assert!(native_export_info("tyrus:db", "queryOne").is_none());

    let root = std::env::temp_dir().join(format!("tyrus-db-error-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { DbError as DatabaseError } from "tyrus:db";
            function errorCode(error: DatabaseError): string {
                return error.code;
            }
        "#,
    )
    .unwrap();
    load_program(&root.join("main.tyr"), None).unwrap();

    fs::write(
        root.join("main.tyr"),
        r#"
            function errorCode(error: DbError): string {
                return error.code;
            }
        "#,
    )
    .unwrap();
    let error = load_program(&root.join("main.tyr"), None).unwrap_err();
    assert!(error.contains("DbError"), "unexpected diagnostics: {error}");
    let _ = fs::remove_dir_all(root);
}
