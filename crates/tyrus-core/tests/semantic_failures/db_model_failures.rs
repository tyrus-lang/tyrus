use tyrus_core::parser::Parser;
use tyrus_core::type_checker::{CheckSeverity, TypeChecker};

fn db_error_codes(source: &str, backend: tyrus_core::db::DatabaseBackend) -> Vec<&'static str> {
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = TypeChecker::new();
    checker.set_db_backend(Some(backend));
    checker
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn rejects_identifier_strategies_with_incompatible_backend_or_field_types() {
    assert!(
        db_error_codes(
            r#"@model("users") interface User { @id({ strategy: "auto_increment" }) id: i32; }"#,
            tyrus_core::db::DatabaseBackend::Mysql,
        )
        .contains(&"E3304")
    );
    assert!(
        db_error_codes(
            r#"@model("users") interface User { @id id: i32; }"#,
            tyrus_core::db::DatabaseBackend::Postgresql,
        )
        .contains(&"E3304")
    );
    assert!(
        db_error_codes(
            r#"@model("users") interface User { @id({ strategy: "uuid_short" }) id: i64; }"#,
            tyrus_core::db::DatabaseBackend::Mysql,
        )
        .contains(&"E3304")
    );
    assert!(
        db_error_codes(
            r#"@model("users") interface User { @id({ strategy: "uuid7" }) id: Uuid; }"#,
            tyrus_core::db::DatabaseBackend::Postgresql,
        )
        .contains(&"E3304")
    );
    assert!(
        db_error_codes(
            r#"@model("users") interface User { @id({ strategy: "uuid" }) id: string; }"#,
            tyrus_core::db::DatabaseBackend::Mongodb,
        )
        .contains(&"E3304")
    );
}
