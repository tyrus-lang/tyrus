use super::*;

#[test]
fn os_recoverable_operations_return_result_instead_of_throws() {
    for name in [
        "openUrl",
        "trash",
        "revealInFolder",
        "preventSleep",
        "allowSleep",
    ] {
        let info = crate::module_loader::native_export_analysis_info("tyrus:os", name)
            .expect("OS operation is exported");
        assert!(info.throws.is_empty(), "`{name}` must not declare throws");
        assert!(
            matches!(info.ty, Type::Function { ret, .. } if matches!(ret.as_ref(), Type::Result { success, error } if **success == Type::Void && **error == Type::Named("OsError".to_string()))),
            "`{name}` must return Result<void, OsError>"
        );
    }
}

#[test]
fn clipboard_read_returns_an_optional_value_or_a_structured_os_error() {
    let info = crate::module_loader::native_export_analysis_info("tyrus:os", "clipboardRead")
        .expect("clipboardRead is exported");
    assert!(info.throws.is_empty());
    assert!(matches!(
        info.ty,
        Type::Function { ret, .. }
            if matches!(ret.as_ref(), Type::Result { success, error }
                if **success == Type::Optional(Box::new(Type::Str))
                    && **error == Type::Named("OsError".to_string()))
    ));
}

#[test]
fn fs_stat_returns_metadata_or_a_structured_error() {
    let info = crate::module_loader::native_export_analysis_info("tyrus:fs", "stat")
        .expect("stat is exported");
    assert!(info.throws.is_empty());
    assert!(matches!(
        info.ty,
        Type::Function { ret, .. }
            if matches!(ret.as_ref(), Type::Result { success, error }
                if **success == Type::Named("FileStats".to_string())
                    && **error == Type::Named("FsError".to_string()))
    ));
}

#[test]
fn encoding_base_namespaces_are_exported() {
    for namespace in ["base64", "base64Url", "base32"] {
        let (_, fields, _) = native_export_info("tyrus:encoding", namespace).unwrap();
        let names: Vec<String> = fields
            .unwrap()
            .into_iter()
            .map(|field| field.name)
            .collect();
        assert_eq!(names, vec!["encode", "decode"], "namespace `{namespace}`");
        let encode = native_export_info("tyrus:encoding", namespace)
            .unwrap()
            .1
            .unwrap()
            .into_iter()
            .find(|field| field.name == "encode")
            .unwrap();
        assert!(
            matches!(encode.ty, Type::Function { params, .. } if params == vec![Type::Array(Box::new(Type::U8))]),
            "`{namespace}.encode` must take a byte array"
        );
        let decode = native_export_info("tyrus:encoding", namespace)
            .unwrap()
            .1
            .unwrap()
            .into_iter()
            .find(|field| field.name == "decode")
            .unwrap();
        assert!(
            matches!(decode.ty, Type::Function { params, .. } if params == vec![Type::Str]),
            "`{namespace}.decode` must take a string"
        );
    }
    let (_, fields, _) = native_export_info("tyrus:encoding", "radix").unwrap();
    let names: Vec<String> = fields
        .unwrap()
        .into_iter()
        .map(|field| field.name)
        .collect();
    assert_eq!(names, vec!["format", "parseI64", "parseU64"]);
    let format = native_export_info("tyrus:encoding", "radix")
        .unwrap()
        .1
        .unwrap()
        .into_iter()
        .find(|field| field.name == "format")
        .unwrap();
    assert!(
        matches!(format.ty, Type::Function { params, .. } if params == vec![Type::I64, crate::encoding::radix_base_type()]),
        "`radix.format` must take (i64, 2 | 8 | 10 | 16)"
    );
}

fn module_test_root(label: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "tyrus-modules-{label}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn resolves_a_default_import_against_an_export_default() {
    let root = module_test_root("default");
    fs::write(
        root.join("math.tyr"),
        "export const add = (left: i32, right: i32): i32 => left + right;\nexport default add;",
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import sum from "./math";
            const result: i32 = sum(20, 22);
        "#,
    )
    .unwrap();

    let program = load_program(&root.join("main.tyr"), None).unwrap();
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { value, .. },
        ..
    }) = &program.items[1]
    else {
        panic!("expected the default import to be called");
    };
    assert!(matches!(
        value.as_ref().map(|value| value.kind()),
        Some(ExprKind::FnCall { callee, .. })
            if callee.starts_with("__tyrus_module_") && callee.ends_with("_add")
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_namespace_import_binds_every_export_as_a_flat_member() {
    let root = module_test_root("namespace");
    fs::write(
        root.join("math.tyr"),
        "export const add = (left: i32, right: i32): i32 => left + right;",
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import * as math from "./math";
            const result: i32 = math.add(20, 22);
        "#,
    )
    .unwrap();

    let program = load_program(&root.join("main.tyr"), None).unwrap();
    let Item::Stmt(Stmt {
        kind: StmtKind::VarDecl { value, .. },
        ..
    }) = &program.items[1]
    else {
        panic!("expected the namespace member call to survive loading");
    };
    assert!(matches!(
        value.as_ref().map(|value| value.kind()),
        Some(ExprKind::FnCall { callee, .. })
            if callee.starts_with("__tyrus_module_") && callee.ends_with("_add")
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_namespace_member_that_is_not_exported_names_the_module_and_the_member() {
    let root = module_test_root("namespace-missing");
    fs::write(
        root.join("math.tyr"),
        "export const add = (a: i32): i32 => a;",
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import * as math from "./math";
            const result: i32 = math.missing(1);
        "#,
    )
    .unwrap();

    let error = load_program(&root.join("main.tyr"), None).unwrap_err();
    assert!(
        error.contains("module namespace `math` has no export `missing`"),
        "diagnostic must name the module and the member, got: {error}"
    );
    assert!(
        !error.contains("__tyrus_ns"),
        "internal name leaked: {error}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn exports_a_binding_through_a_separate_clause_with_an_alias() {
    let root = module_test_root("export-clause");
    fs::write(
        root.join("math.tyr"),
        "const add = (left: i32, right: i32): i32 => left + right;\nexport { add as plus };",
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { plus } from "./math";
            const result: i32 = plus(20, 22);
        "#,
    )
    .unwrap();

    assert!(load_program(&root.join("main.tyr"), None).is_ok());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_reexport_forwards_a_symbol_without_binding_it_locally() {
    let root = module_test_root("reexport");
    fs::write(
        root.join("math.tyr"),
        "export const add = (left: i32, right: i32): i32 => left + right;",
    )
    .unwrap();
    fs::write(
        root.join("lib.tyr"),
        r#"export { add as plus } from "./math";"#,
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { plus } from "./lib";
            const result: i32 = plus(20, 22);
        "#,
    )
    .unwrap();

    assert!(load_program(&root.join("main.tyr"), None).is_ok());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn export_star_forwards_named_exports_but_never_the_default() {
    let root = module_test_root("export-star");
    fs::write(
        root.join("math.tyr"),
        "export const add = (left: i32, right: i32): i32 => left + right;\nexport default add;",
    )
    .unwrap();
    fs::write(root.join("lib.tyr"), r#"export * from "./math";"#).unwrap();
    fs::write(
        root.join("named.tyr"),
        r#"
            import { add } from "./lib";
            const result: i32 = add(20, 22);
        "#,
    )
    .unwrap();
    fs::write(
        root.join("defaulted.tyr"),
        r#"
            import sum from "./lib";
            const result: i32 = sum(20, 22);
        "#,
    )
    .unwrap();

    assert!(load_program(&root.join("named.tyr"), None).is_ok());
    let error = load_program(&root.join("defaulted.tyr"), None).unwrap_err();
    assert!(
        error.contains("does not export `default`"),
        "`export *` must not forward the default, got: {error}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn two_exports_publishing_the_same_name_are_rejected() {
    let root = module_test_root("duplicate-export");
    fs::write(
        root.join("math.tyr"),
        "export const add = (a: i32): i32 => a;\nconst other = (a: i32): i32 => a;\nexport { other as add };",
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { add } from "./math";
            const result: i32 = add(1);
        "#,
    )
    .unwrap();

    let error = load_program(&root.join("main.tyr"), None).unwrap_err();
    assert!(error.contains("duplicate export `add`"), "got: {error}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn an_exported_binding_without_an_annotation_keeps_its_inferred_type() {
    let root = module_test_root("inferred-export");
    fs::write(root.join("config.tyr"), "export const answer = 42;").unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { answer } from "./config";
            console.log(answer);
        "#,
    )
    .unwrap();

    assert!(load_program(&root.join("main.tyr"), None).is_ok());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn imports_and_renames_behavior_interfaces_with_their_targets() {
    let root = module_test_root("behavior-interface");
    fs::write(
        root.join("model.tyr"),
        r#"
            export interface User { age: u32; }
            export interface UserMethods at User {
                isAdult(user: User): boolean { return user.age >= 18; }
            }
        "#,
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { User, UserMethods } from "./model";
            const accept = (rules: UserMethods): void => {};
            const main = (): void => {
                const user: User = { age: 21 };
                console.log(user.isAdult());
            };
        "#,
    )
    .unwrap();

    let program = load_program(&root.join("main.tyr"), None).unwrap();
    let behavior = program.items.iter().find_map(|item| match item {
        Item::BehaviorInterfaceDecl {
            name,
            targets,
            methods,
            ..
        } => Some((name.clone(), targets.clone(), methods.clone())),
        _ => None,
    });
    let Some((name, targets, methods)) = behavior else {
        panic!("expected imported behavior interface in merged program");
    };
    assert!(name.starts_with("__tyrus_module_"));
    assert!(
        matches!(targets.first(), Some(Type::Named(target)) if target.starts_with("__tyrus_module_"))
    );
    assert_eq!(methods[0].name, "isAdult");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn imports_behavior_interfaces_with_shorthand_bindings() {
    let root = module_test_root("behavior-shorthand-import");
    fs::write(
        root.join("model.tyr"),
        r#"
            export interface User { age: u32; }
            export const ageText = (user: User): string => user.age.toString();
            export interface UserMethods at User { ageText; }
        "#,
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { User, UserMethods } from "./model";
            const accept = (rules: UserMethods): void => {};
            const main = (): void => {
                const user: User = { age: 21 };
                console.log(user.ageText());
            };
        "#,
    )
    .unwrap();

    assert!(load_program(&root.join("main.tyr"), None).is_ok());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn two_modules_may_each_declare_an_enum_with_the_same_name() {
    let root = module_test_root("enum-collision");
    fs::write(
        root.join("a.tyr"),
        "enum Status { Success, Error }\nexport const fromA = (): i32 => Status.Success as i32;",
    )
    .unwrap();
    fs::write(
        root.join("b.tyr"),
        "enum Status { NotFound = 404 }\nexport const fromB = (): i32 => Status.NotFound as i32;",
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { fromA } from "./a";
            import { fromB } from "./b";
            const first: i32 = fromA();
            const second: i32 = fromB();
        "#,
    )
    .unwrap();

    let program = load_program(&root.join("main.tyr"), None).unwrap();
    let enums = program
        .items
        .iter()
        .filter_map(|item| match item {
            Item::EnumDecl { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(enums.len(), 2, "each module keeps its own enum: {enums:?}");
    assert!(
        enums.iter().all(|name| name.starts_with("__tyrus_module_")),
        "both enums must be renamed per module: {enums:?}"
    );
    fs::remove_dir_all(root).unwrap();
}
