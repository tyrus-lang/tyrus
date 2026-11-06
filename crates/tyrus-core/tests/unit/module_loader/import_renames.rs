use super::*;

#[test]
fn renames_imported_interface_declarations_with_their_references() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-module-interface-rename-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("model.tyr"),
        r#"
            export interface User {
                id: string;
            }
        "#,
    )
    .unwrap();
    fs::write(
        root.join("repository.tyr"),
        r#"
            import { User } from "./model";
            export function findUser(): User {
                return { id: "user-1" };
            }
        "#,
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { findUser } from "./repository";
            import { User } from "./model";
            const user: User = findUser();
            console.log(user.id);
        "#,
    )
    .unwrap();

    let program = load_program(&root.join("main.tyr"), None).unwrap();
    let renamed_user = program
        .items
        .iter()
        .find_map(|item| match item {
            Item::InterfaceDecl { name, .. }
                if name.starts_with("__tyrus_module_") && name.ends_with("_User") =>
            {
                Some(name.clone())
            }
            _ => None,
        })
        .expect("expected renamed imported interface");
    assert!(program.items.iter().any(|item| matches!(
        item,
        Item::FnDecl { ret_type, .. } if ret_type == &Type::Named(renamed_user.clone())
    )));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn renames_generic_call_types_inside_imported_module_expressions() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-module-generic-expression-rename-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("tokens.tyr"),
        r#"
            import { CryptoError, jwt } from "tyrus:crypto";

            interface Claims {
                sub: string;
                exp: i64;
            }

            export function createToken(): Result<string, CryptoError> {
                return jwt.sign<Claims>(
                    { sub: "user-1", exp: 4102444800 },
                    "0123456789abcdef0123456789abcdef",
                );
            }
        "#,
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { createToken } from "./tokens";
            import { CryptoError } from "tyrus:crypto";

            function main(): void {
                const token: Result<string, CryptoError> = createToken();
                match (token) {
                    Ok(value) => { console.log(value); }
                    Err(error) => { console.error(error.code); }
                }
            }
        "#,
    )
    .unwrap();

    let program = load_program(&root.join("main.tyr"), None).unwrap();
    let renamed_claims = program
        .items
        .iter()
        .find_map(|item| match item {
            Item::InterfaceDecl { name, .. }
                if name.starts_with("__tyrus_module_") && name.ends_with("_Claims") =>
            {
                Some(name.clone())
            }
            _ => None,
        })
        .expect("expected renamed imported claims interface");
    let type_args = program
        .items
        .iter()
        .find_map(|item| {
            let Item::FnDecl { name, body, .. } = item else {
                return None;
            };
            if !name.ends_with("_createToken") {
                return None;
            }
            body.iter().find_map(|statement| {
                let StmtKind::Return(Some(expression)) = statement.kind() else {
                    return None;
                };
                let ExprKind::MethodCall { type_args, .. } = expression.kind() else {
                    return None;
                };
                Some(type_args.clone())
            })
        })
        .expect("expected imported generic JWT method call");
    assert_eq!(type_args, vec![Type::Named(renamed_claims)]);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_circular_imports() {
    let root = std::env::temp_dir().join(format!("tyrus-module-cycle-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("a.tyr"),
        "import { b } from \"./b\"; export const a: i32 = b;",
    )
    .unwrap();
    fs::write(
        root.join("b.tyr"),
        "import { a } from \"./a\"; export const b: i32 = a;",
    )
    .unwrap();

    let error = load_program(&root.join("a.tyr"), None).unwrap_err();
    assert!(error.contains("circular module import"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_invalid_arguments_across_modules_with_source_location() {
    let root = std::env::temp_dir().join(format!("tyrus-module-types-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("math.tyr"),
        "export const add = (left: i32, right: i32): i32 => left + right;",
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        "import { add } from \"./math\";\nadd(\"wrong\", 2);",
    )
    .unwrap();

    let error = load_program(&root.join("main.tyr"), None).unwrap_err();
    assert!(error.contains("error[E0308]"));
    assert!(error.contains("main.tyr:2:1"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn requires_imported_throws_to_be_handled() {
    let root =
        std::env::temp_dir().join(format!("tyrus-module-throws-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("validation.tyr"),
        "export const accept = (@min(18) age: i32): void => {};",
    )
    .unwrap();
    fs::write(root.join("main.tyr"), "import { accept } from \"./validation\";\nconst readAge = (): i32 => 16;\nconst main = (): void => { accept(readAge()); };").unwrap();
    let error = load_program(&root.join("main.tyr"), None)
        .expect_err("imported ValidationError must be handled");
    assert!(error.contains("E3011"), "got: {error}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn import_escaping_the_project_root_is_rejected() {
    let base = std::env::temp_dir().join(format!("tyrus-traversal-{}", std::process::id()));
    let proj = base.join("proj");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&proj).unwrap();
    fs::write(proj.join("tyrus.json"), "{}").unwrap();
    fs::write(base.join("secret.tyr"), "export const secret: i32 = 1;").unwrap();
    fs::write(
        proj.join("main.tyr"),
        "import { secret } from \"../secret\";\nconst x: i32 = secret;\n",
    )
    .unwrap();

    let error = load_program(&proj.join("main.tyr"), None)
        .expect_err("an import escaping the project root must be rejected");
    assert!(error.contains("outside the project root"), "got: {error}");

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn renames_return_type_of_a_nested_arrow_in_an_imported_module() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-module-nested-arrow-rename-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("model.tyr"),
        r#"
            export interface Account {
                id: i32;
            }
            export const run = (): void => {
                const make = (): Account => {
                    return { id: 1 };
                };
                console.log(make().id);
            };
        "#,
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { run } from "./model";
            const main = (): void => {
                run();
            };
        "#,
    )
    .unwrap();

    let program = load_program(&root.join("main.tyr"), None).unwrap();
    let mut nested_ret_types = Vec::new();
    for item in &program.items {
        if let Item::FnDecl { body, .. } = item {
            collect_nested_arrow_return_types(body, &mut nested_ret_types);
        }
    }
    assert!(
        !nested_ret_types.is_empty(),
        "expected to find a nested arrow with a return type"
    );
    assert!(
        nested_ret_types
            .iter()
            .all(|ty| !matches!(ty, Type::Named(name) if name == "Account")),
        "a nested arrow kept the unrenamed interface `Account`: {nested_ret_types:?}"
    );
    fs::remove_dir_all(root).unwrap();
}

fn collect_nested_arrow_return_types(body: &[Stmt], out: &mut Vec<Type>) {
    for statement in body {
        if let Stmt {
            kind:
                StmtKind::VarDecl {
                    value:
                        Some(Expr {
                            kind: ExprKind::ArrowFn { ret_type, .. },
                            ..
                        }),
                    ..
                },
            ..
        } = statement
        {
            out.push(ret_type.clone());
        }
    }
    assert!(native_export_info("tyrus:encoding", "utf8").is_some());
    assert!(native_export_info("tyrus:encoding", "utf16").is_some());
    assert!(native_export_info("tyrus:encoding", "hex").is_some());
    let (_, fields, _) = native_export_info("tyrus:encoding", "hex").unwrap();
    let names: Vec<String> = fields
        .unwrap()
        .into_iter()
        .map(|field| field.name)
        .collect();
    assert_eq!(names, vec!["encode", "decode"]);
}
