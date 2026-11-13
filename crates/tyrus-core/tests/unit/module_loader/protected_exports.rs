use super::*;

#[test]
fn the_protect_modifier_range_covers_only_the_modifier() {
    let source = concat!(
        "export const open = (): i32 => 1;\n",
        "export protect const verify = (): i32 => 2;\n",
        "export protect function helper(): void {}\n",
        "export protect interface Shape { id: i32; }\n",
        "const first: i32 = 3;\n",
        "export protect { first as alias };\n",
    );
    for name in ["verify", "helper", "Shape", "alias"] {
        let range = crate::module_loader::protect_modifier_range(source, name)
            .unwrap_or_else(|| panic!("`{name}` should resolve to a modifier range"));
        assert_eq!(
            &source[range.clone()],
            " protect",
            "the edit must remove the modifier and keep `export` for `{name}`"
        );
        assert!(
            source[..range.start].ends_with("export"),
            "the range must start right after `export` for `{name}`"
        );
    }
    assert!(
        crate::module_loader::protect_modifier_range(source, "open").is_none(),
        "a public export has no modifier to remove"
    );
}

#[test]
fn the_editor_index_helpers_agree_with_the_loader() {
    let root = protect_fixture("index-helpers");
    let source = std::fs::read_to_string(root.join("auth/token.tyr")).unwrap();
    assert_eq!(
        crate::module_loader::protected_export_names(&source),
        vec!["verify".to_string()],
        "only the protected name belongs in the filter, not the public one"
    );

    let declaring = root.join("auth/token.tyr");
    for (importer, allowed) in [
        (root.join("auth/login.tyr"), true),
        (root.join("auth/session/refresh.tyr"), true),
        (root.join("http/handler.tyr"), false),
        (root.join("main.tyr"), false),
    ] {
        std::fs::write(&importer, "const placeholder: i32 = 1;\n").unwrap();
        assert_eq!(
            crate::module_loader::protect_scope_allows(&importer, &declaring),
            allowed,
            "wrong scope answer for {}",
            importer.display()
        );
    }

    // The editor resolvers produce paths that still contain `..`, which is what
    // made the linter refuse a legitimate subfolder import before.
    let unnormalized = root.join("auth/session/../token.tyr");
    assert!(
        crate::module_loader::protect_scope_allows(&root.join("auth/login.tyr"), &unnormalized),
        "a `..` in the declaring path must not change the answer"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn files_without_protect_skip_export_parsing() {
    let source = format!(
        "/* {} */\nconsole.log(\"workspace file\");",
        "comment\n".repeat(20_000)
    );
    assert!(crate::module_loader::protected_export_names(&source).is_empty());
}

fn protect_fixture(label: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("tyrus-protect-{}-{label}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("auth/session")).unwrap();
    fs::create_dir_all(root.join("http")).unwrap();
    fs::write(
        root.join("auth/token.tyr"),
        concat!(
            "export protect const verify = (id: i32): i32 => id + 1;\n",
            "export const open = (id: i32): i32 => id + 2;\n",
        ),
    )
    .unwrap();
    fs::write(
        root.join("auth/session/keys.tyr"),
        "export protect const secret = (): i32 => 7;\n",
    )
    .unwrap();
    root
}

#[test]
fn a_protect_export_is_visible_in_its_folder_and_below() {
    let root = protect_fixture("below");
    fs::write(
        root.join("auth/login.tyr"),
        concat!(
            "import { verify } from \"./token\";\n",
            "export const login = (id: i32): i32 => verify(id);\n",
        ),
    )
    .unwrap();
    fs::write(
        root.join("auth/session/refresh.tyr"),
        concat!(
            "import { verify } from \"../token\";\n",
            "export const refresh = (id: i32): i32 => verify(id);\n",
        ),
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        concat!(
            "import { login } from \"./auth/login\";\n",
            "import { refresh } from \"./auth/session/refresh\";\n",
            "const total: i32 = login(1) + refresh(2);\n",
        ),
    )
    .unwrap();
    load_program(&root.join("main.tyr"), None).expect("the folder and its subfolders may import");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_protect_export_is_refused_from_a_sibling_folder() {
    let root = protect_fixture("sibling");
    fs::write(
        root.join("http/handler.tyr"),
        concat!(
            "import { verify } from \"../auth/token\";\n",
            "export const handle = (id: i32): i32 => verify(id);\n",
        ),
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        concat!(
            "import { handle } from \"./http/handler\";\n",
            "const total: i32 = handle(1);\n",
        ),
    )
    .unwrap();
    let error = load_program(&root.join("main.tyr"), None).unwrap_err();
    assert!(error.contains("error[E3308]"), "got: {error}");
    assert!(
        error.contains("`verify`"),
        "the name must be cited: {error}"
    );
    assert!(
        error.contains("http/handler.tyr"),
        "the importing file must be cited: {error}"
    );
    assert!(
        error.contains("note:") && error.contains("subfolders"),
        "the note must explain the rule: {error}"
    );
    assert!(
        error.contains("help:") && error.contains("drop `protect`"),
        "the help must be concrete: {error}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_protect_export_is_refused_from_the_parent_folder() {
    let root = protect_fixture("parent");
    fs::write(
        root.join("main.tyr"),
        concat!(
            "import { verify } from \"./auth/token\";\n",
            "const total: i32 = verify(1);\n",
        ),
    )
    .unwrap();
    let error = load_program(&root.join("main.tyr"), None).unwrap_err();
    assert!(error.contains("error[E3308]"), "got: {error}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_deeper_protect_is_not_visible_in_the_parent_folder() {
    let root = protect_fixture("deeper");
    fs::write(
        root.join("auth/login.tyr"),
        concat!(
            "import { secret } from \"./session/keys\";\n",
            "export const login = (): i32 => secret();\n",
        ),
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        concat!(
            "import { login } from \"./auth/login\";\n",
            "const total: i32 = login();\n",
        ),
    )
    .unwrap();
    let error = load_program(&root.join("main.tyr"), None).unwrap_err();
    assert!(error.contains("error[E3308]"), "got: {error}");
    assert!(error.contains("`secret`"), "got: {error}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_named_re_export_keeps_the_protected_scope() {
    let root = protect_fixture("re-export-named");
    fs::write(
        root.join("auth/index.tyr"),
        "export { verify } from \"./token\";\n",
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        concat!(
            "import { verify } from \"./auth/index\";\n",
            "const total: i32 = verify(1);\n",
        ),
    )
    .unwrap();
    let error = load_program(&root.join("main.tyr"), None).unwrap_err();
    assert!(
        error.contains("error[E3308]"),
        "a re-export must not widen the scope: {error}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_star_re_export_keeps_the_protected_scope() {
    let root = protect_fixture("re-export-star");
    fs::write(root.join("auth/index.tyr"), "export * from \"./token\";\n").unwrap();
    fs::write(
        root.join("main.tyr"),
        concat!(
            "import { open, verify } from \"./auth/index\";\n",
            "const total: i32 = open(1) + verify(2);\n",
        ),
    )
    .unwrap();
    let error = load_program(&root.join("main.tyr"), None).unwrap_err();
    assert!(
        error.contains("error[E3308]") && error.contains("`verify`"),
        "`export *` must not widen the scope: {error}"
    );

    fs::write(
        root.join("main.tyr"),
        concat!(
            "import { open } from \"./auth/index\";\n",
            "const total: i32 = open(1);\n",
        ),
    )
    .unwrap();
    load_program(&root.join("main.tyr"), None)
        .expect("the public name forwarded by the same `export *` stays importable");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_re_export_inside_the_folder_still_resolves() {
    let root = protect_fixture("re-export-inside");
    fs::write(root.join("auth/index.tyr"), "export * from \"./token\";\n").unwrap();
    fs::write(
        root.join("auth/login.tyr"),
        concat!(
            "import { verify } from \"./index\";\n",
            "export const login = (id: i32): i32 => verify(id);\n",
        ),
    )
    .unwrap();
    fs::write(
        root.join("main.tyr"),
        concat!(
            "import { login } from \"./auth/login\";\n",
            "const total: i32 = login(1);\n",
        ),
    )
    .unwrap();
    load_program(&root.join("main.tyr"), None)
        .expect("a sibling inside the folder may go through the barrel");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_namespace_import_omits_protected_symbols() {
    let root = protect_fixture("namespace");
    fs::write(
        root.join("main.tyr"),
        concat!(
            "import * as auth from \"./auth/token\";\n",
            "const total: i32 = auth.open(1);\n",
        ),
    )
    .unwrap();
    load_program(&root.join("main.tyr"), None)
        .expect("the public name stays reachable through the namespace");

    fs::write(
        root.join("main.tyr"),
        concat!(
            "import * as auth from \"./auth/token\";\n",
            "const total: i32 = auth.verify(1);\n",
        ),
    )
    .unwrap();
    assert!(
        load_program(&root.join("main.tyr"), None).is_err(),
        "a namespace import must not smuggle a protected symbol out of its folder"
    );
    fs::remove_dir_all(root).unwrap();
}
