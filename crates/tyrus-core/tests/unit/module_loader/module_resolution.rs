use super::*;

#[test]
fn resolves_local_and_native_named_imports() {
    let root = std::env::temp_dir().join(format!("tyrus-modules-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("math.tyr"),
        "export const add = (left: i32, right: i32): i32 => left + right;",
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { add as sum } from "./math";
            import { readFileSync } from "tyrus:fs";
            const result: i32 = sum(20, 22);
            const text = readFileSync("message.txt");
        "#,
    )
    .unwrap();

    let program = load_program(&root.join("main.tyr"), None).unwrap();
    assert!(program.imports.is_empty());
    assert!(program.exports.is_empty());
    assert!(matches!(
        &program.items[0],
        Item::FnDecl { name, .. } if name.starts_with("__tyrus_module_") && name.ends_with("_add")
    ));
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { value, .. },
        ..
    }) = &program.items[1]
    else {
        panic!("expected imported function call");
    };
    assert!(matches!(
        value,
        Some(Expr { kind: ExprKind::FnCall { callee, .. }, .. }) if callee.starts_with("__tyrus_module_") && callee.ends_with("_add")
    ));
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { value, .. },
        ..
    }) = &program.items[2]
    else {
        panic!("expected native function call inside try block");
    };
    assert!(matches!(
        value,
        Some(Expr { kind: ExprKind::FnCall { callee, .. }, .. }) if callee == "__tyrus_fs_readFileSync"
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn inferred_orm_onfail_exposes_db_error_fields_without_db_error_import() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-db-onfail-fields-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("user.tyr"),
        "@model(\"users\") export interface User { name: string; }",
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { User } from "./user";
            const main = async (): Promise<void> => {
                const data = await User.insertMany([{ name: "Ada Lovelace" }]) onfail {
                    printError(`${data.code}: ${data.message}`);
                    return;
                };
            };
        "#,
    )
    .unwrap();

    let program = crate::module_loader::load_program_for_db_migrations(&root.join("main.tyr"));

    assert!(
        program.is_ok(),
        "inferred DbError fields should resolve for imported ORM Models: {program:#?}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn resolves_directory_import_aliases_from_the_project_manifest() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-module-import-aliases-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src/utils")).unwrap();
    fs::write(
        root.join("tyrus.json"),
        r##"{
            "imports": {
                "#config": "./src/config",
                "#utils": "./src/utils"
            }
        }"##,
    )
    .unwrap();
    fs::create_dir_all(root.join("src/config")).unwrap();
    fs::write(
        root.join("src/config/index.tyr"),
        "export const port: i32 = 3000;",
    )
    .unwrap();
    fs::write(
        root.join("src/utils/math.tyr"),
        "export const add = (left: i32, right: i32): i32 => left + right;",
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        "import { port } from \"#config\"; import { add } from \"#utils/math\"; const total: i32 = add(port, 12);",
    )
    .unwrap();

    let program = load_program(&root.join("main.tyr"), None).unwrap();
    assert!(program.items.len() >= 3);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn import_aliases_report_missing_modules_exports_and_cycles() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-module-import-alias-errors-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src/module")).unwrap();
    fs::create_dir_all(root.join("src/first")).unwrap();
    fs::create_dir_all(root.join("src/second")).unwrap();
    fs::write(
        root.join("tyrus.json"),
        r##"{
            "imports": {
                "#missing": "./src/missing",
                "#module": "./src/module",
                "#first": "./src/first",
                "#second": "./src/second"
            }
        }"##,
    )
    .unwrap();

    let entry = root.join("main.tyr");
    fs::write(&entry, "import { value } from \"#missing\";").unwrap();
    let error = load_program(&entry, None).expect_err("the missing alias target must fail");
    assert!(error.contains("#missing"), "got: {error}");

    fs::write(
        root.join("src/module/index.tyr"),
        "export const other: i32 = 1;",
    )
    .unwrap();
    fs::write(&entry, "import { value } from \"#module\";").unwrap();
    let error = load_program(&entry, None).expect_err("a missing export must fail");
    assert!(error.contains("does not export `value`"), "got: {error}");

    fs::write(
        root.join("src/first/index.tyr"),
        "import { second } from \"#second\"; export const first: i32 = second;",
    )
    .unwrap();
    fs::write(
        root.join("src/second/index.tyr"),
        "import { first } from \"#first\"; export const second: i32 = first;",
    )
    .unwrap();
    fs::write(&entry, "import { first } from \"#first\";").unwrap();
    let error = load_program(&entry, None).expect_err("an alias cycle must fail");
    assert!(error.contains("circular module import"), "got: {error}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn projects_comptime_results_before_codegen_receives_the_program() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-comptime-projection-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        "comptime function double(value: i32): i32 { return value * 2; } const answer: i32 = comptime(double(21));",
    )
    .unwrap();

    let program = load_program(&root.join("main.tyr"), None).unwrap();
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl {
            value: Some(value), ..
        },
        ..
    }) = &program.items[1]
    else {
        panic!("expected projected comptime binding");
    };
    assert!(matches!(value.kind(), ExprKind::IntLit(42)));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn projects_structured_comptime_results_before_codegen_receives_the_program() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-structured-comptime-projection-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        "enum Mode { Fast = 1 } interface Config { name: string; values: i32[]; mode: Mode; } comptime function build(): Config { return { name: `ready`, values: [20, 22], mode: Mode.Fast }; } const config: Config = comptime(build());",
    )
    .unwrap();

    let program = load_program(&root.join("main.tyr"), None).unwrap();
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl {
            value: Some(value), ..
        },
        ..
    }) = &program.items[3]
    else {
        panic!("expected projected structured comptime binding");
    };
    let ExprKind::ObjectLit(fields) = value.kind() else {
        panic!("expected projected object literal");
    };
    assert!(fields.iter().any(|(name, value)| {
        name == "mode"
            && matches!(
                value.kind(),
                ExprKind::FieldAccess { object, field }
                    if field == "Fast" && matches!(object.kind(), ExprKind::Ident(name) if name == "__tyrus_module_0_Mode")
            )
    }));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_the_removed_source_extension() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-removed-extension-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let removed_extension = ["rs", "ts"].concat();
    let entry = root.join(format!("main.{removed_extension}"));
    fs::write(&entry, "const answer: i32 = 42;").unwrap();

    let error = load_program(&entry, None).expect_err("the removed extension must be rejected");
    assert!(
        error.contains("must use the `.tyr` extension"),
        "got: {error}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_the_removed_native_module_prefix() {
    let removed_prefix = ["rs", "ts:crypto"].concat();
    assert!(native_export_info(&removed_prefix, "digest").is_none());
}

#[test]
fn exposes_task_sleep_as_a_synchronous_native_function() {
    let (ty, fields, alias) = native_export_info("tyrus:task", "sleep").unwrap();
    assert_eq!(ty, crate::task::sleep_type());
    assert!(fields.is_none());
    assert!(alias.is_none());
    let (ty, fields, alias) = native_export_info("tyrus:net", "reverseLookup").unwrap();
    assert!(matches!(ty, Type::Function { .. }));
    assert!(fields.is_none());
    assert!(alias.is_none());
}

#[test]
fn exposes_task_sleep_async_as_a_native_promise_function() {
    let (ty, fields, alias) = native_export_info("tyrus:task", "sleepAsync").unwrap();
    assert_eq!(ty, crate::task::sleep_async_type());
    assert!(fields.is_none());
    assert!(alias.is_none());
}

#[test]
fn does_not_resolve_the_removed_plural_task_module() {
    let removed_module = ["tyrus", ":", "tasks"].concat();
    assert!(native_export_info(&removed_module, "sleep").is_none());
}

#[test]
fn exposes_tyrus_net_address_and_error_types() {
    for export in [
        "NetError",
        "IpAddress",
        "SocketAddress",
        "TcpStream",
        "TcpListener",
        "UdpSocket",
        "UdpPacket",
    ] {
        let (ty, fields, alias) = native_export_info("tyrus:net", export)
            .unwrap_or_else(|| panic!("missing tyrus:net export `{export}`"));
        assert_eq!(ty, Type::Named(export.to_string()));
        assert!(fields.is_some(), "missing fields for `{export}`");
        assert!(alias.is_none());
    }
    let (ty, fields, alias) = native_export_info("tyrus:net", "parseIpAddress").unwrap();
    assert!(matches!(ty, Type::Function { .. }));
    assert!(fields.is_none());
    assert!(alias.is_none());
    let (ty, fields, alias) = native_export_info("tyrus:net", "parseSocketAddress").unwrap();
    assert!(matches!(ty, Type::Function { .. }));
    assert!(fields.is_none());
    assert!(alias.is_none());
    let (ty, fields, alias) = native_export_info("tyrus:net", "resolve").unwrap();
    let Type::Function { ret, .. } = ty else {
        panic!("resolve must be a function");
    };
    let Type::Apply { name, args } = *ret else {
        panic!("resolve must return a promise");
    };
    assert_eq!(name, "Promise");
    assert!(
        matches!(args.as_slice(), [Type::Result { success, error }] if **success == Type::Array(Box::new(Type::Str)) && **error == Type::Named("NetError".to_string()))
    );
    assert!(fields.is_none());
    assert!(alias.is_none());
    let (ty, fields, alias) = native_export_info("tyrus:net", "reverseLookup").unwrap();
    let Type::Function { ret, .. } = ty else {
        panic!("reverseLookup must be a function");
    };
    let Type::Apply { name, args } = *ret else {
        panic!("reverseLookup must return a promise");
    };
    assert_eq!(name, "Promise");
    assert!(
        matches!(args.as_slice(), [Type::Result { success, error }] if **success == Type::Str && **error == Type::Named("NetError".to_string()))
    );
    assert!(fields.is_none());
    assert!(alias.is_none());
    for (export, success_type) in [
        ("ping", Type::U64),
        ("traceroute", Type::Array(Box::new(Type::Str))),
    ] {
        let (ty, fields, alias) = native_export_info("tyrus:net", export)
            .unwrap_or_else(|| panic!("missing tyrus:net export `{export}`"));
        let Type::Function { ret, .. } = ty else {
            panic!("{export} must be a function");
        };
        let Type::Apply { name, args } = *ret else {
            panic!("{export} must return a promise");
        };
        assert_eq!(name, "Promise");
        assert!(
            matches!(args.as_slice(), [Type::Result { success, error }] if **success == success_type && **error == Type::Named("NetError".to_string()))
        );
        assert!(fields.is_none());
        assert!(alias.is_none());
    }
    for (export, success_type) in [
        ("tcpConnect", Type::Named("TcpStream".to_string())),
        ("tcpListen", Type::Named("TcpListener".to_string())),
    ] {
        let (ty, fields, alias) = native_export_info("tyrus:net", export)
            .unwrap_or_else(|| panic!("missing tyrus:net export `{export}`"));
        let Type::Function { ret, .. } = ty else {
            panic!("{export} must be a function");
        };
        let Type::Apply { name, args } = *ret else {
            panic!("{export} must return a promise");
        };
        assert_eq!(name, "Promise");
        assert!(
            matches!(args.as_slice(), [Type::Result { success, error }] if **success == success_type && **error == Type::Named("NetError".to_string()))
        );
        assert!(fields.is_none());
        assert!(alias.is_none());
    }
    for export in ["udpBind", "ping", "traceroute", "supportsIcmp"] {
        let (ty, fields, alias) = native_export_info("tyrus:net", export)
            .unwrap_or_else(|| panic!("missing tyrus:net export `{export}`"));
        assert!(matches!(ty, Type::Function { .. }));
        assert!(fields.is_none());
        assert!(alias.is_none());
    }
}

#[test]
fn exposes_tasks_cancel_as_a_generic_native_function() {
    let (ty, fields, alias) = native_export_info("tyrus:task", "cancel").unwrap();
    assert_eq!(ty, crate::task::cancel_type());
    assert!(fields.is_none());
    assert!(alias.is_none());
}

#[test]
fn exposes_tasks_spawn_as_a_generic_native_function() {
    let (ty, fields, alias) = native_export_info("tyrus:task", "spawn").unwrap();
    assert_eq!(ty, crate::task::spawn_type());
    assert!(fields.is_none());
    assert!(alias.is_none());
}

#[test]
fn exposes_tasks_join_all_as_a_generic_native_function() {
    let (ty, fields, alias) = native_export_info("tyrus:task", "joinAll").unwrap();
    assert_eq!(ty, crate::task::join_all_type());
    assert!(fields.is_none());
    assert!(alias.is_none());
}

#[test]
fn exposes_tasks_race_as_a_generic_native_function() {
    let (ty, fields, alias) = native_export_info("tyrus:task", "race").unwrap();
    assert_eq!(ty, crate::task::race_type());
    assert!(fields.is_none());
    assert!(alias.is_none());
}

#[test]
fn exposes_tasks_timeout_as_a_generic_native_function() {
    let (ty, fields, alias) = native_export_info("tyrus:task", "timeout").unwrap();
    assert_eq!(ty, crate::task::timeout_type());
    assert!(fields.is_none());
    assert!(alias.is_none());
}

#[test]
fn exposes_tasks_channel_as_a_generic_native_function() {
    let (ty, fields, alias) = native_export_info("tyrus:task", "channel").unwrap();
    assert_eq!(ty, crate::task::channel_type());
    assert!(fields.is_none());
    assert!(alias.is_none());
}

#[test]
fn exposes_tasks_mutex_as_a_generic_native_function() {
    let (ty, fields, alias) = native_export_info("tyrus:task", "Mutex").unwrap();
    assert_eq!(ty, crate::task::mutex_type());
    assert!(fields.is_none());
    assert!(alias.is_none());
}

#[test]
fn exposes_tasks_rwlock_as_a_generic_native_function() {
    let (ty, fields, alias) = native_export_info("tyrus:task", "RwLock").unwrap();
    assert_eq!(ty, crate::task::rwlock_type());
    assert!(fields.is_none());
    assert!(alias.is_none());
}

#[test]
fn exposes_tasks_semaphore_as_a_native_function() {
    let (ty, fields, alias) = native_export_info("tyrus:task", "Semaphore").unwrap();
    assert_eq!(ty, crate::task::semaphore_type());
    assert!(fields.is_none());
    assert!(alias.is_none());
}
