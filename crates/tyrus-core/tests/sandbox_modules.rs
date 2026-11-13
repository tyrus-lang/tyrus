use std::path::{Path, PathBuf};
use tyrus_core::module_loader::ModuleRestriction;

fn project_with(label: &str, source: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "tyrus-sandbox-{label}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("tyrus.json"), "{}").unwrap();
    std::fs::write(root.join("src/main.tyr"), source).unwrap();
    root
}

fn load(root: &Path, sandbox: ModuleRestriction) -> Result<(), String> {
    tyrus_core::module_loader::load_program_sandboxed(&root.join("src/main.tyr"), None, sandbox)
        .map(|_| ())
}

fn check(label: &str, source: &str, sandbox: ModuleRestriction) -> Result<(), String> {
    let root = project_with(label, source);
    let result = load(&root, sandbox);
    let _ = std::fs::remove_dir_all(&root);
    result
}

const MAIN: &str = "const main = (): void => { console.log(1); };";

#[test]
fn a_sandboxed_build_refuses_every_capability_module() {
    for module in tyrus_core::module_loader::SANDBOX_FORBIDDEN_MODULES {
        let source = format!("import * as m from \"{module}\";\n{MAIN}");
        let error = check("forbidden", &source, ModuleRestriction::Sandbox)
            .expect_err(&format!("`{module}` must be refused in a sandboxed build"));
        assert!(
            error.contains(module) && error.contains("sandboxed build"),
            "the error for `{module}` should name the module and the reason, got: {error}"
        );
    }
}

#[test]
fn a_comment_does_not_disguise_a_forbidden_import() {
    for source in [
        "import { readFile } from /* x */ \"tyrus:fs\";",
        "import { readFile } from\n\"tyrus:fs\";",
        "import { readFile } from\t'tyrus:fs';",
    ] {
        let program = format!("{source}\n{MAIN}");
        assert!(
            check("disguised", &program, ModuleRestriction::Sandbox).is_err(),
            "a sandboxed build must refuse this import regardless of spacing:\n{source}"
        );
    }
}

#[test]
fn the_pure_modules_stay_available_in_a_sandbox() {
    for source in [
        "import { median } from \"tyrus:math\";\nconst main = (): void => { console.log(median([1.0, 2.0])); };",
        "import { instantNow } from \"tyrus:temporal\";\nconst main = (): void => { const now = instantNow(); };",
    ] {
        check("allowed", source, ModuleRestriction::Sandbox)
            .unwrap_or_else(|error| panic!("a pure module must load in a sandbox: {error}"));
    }
}

#[test]
fn an_unsandboxed_build_keeps_every_module() {
    for module in tyrus_core::module_loader::SANDBOX_FORBIDDEN_MODULES {
        let source = format!("import * as m from \"{module}\";\n{MAIN}");
        assert!(
            check("native", &source, ModuleRestriction::None).is_ok(),
            "`{module}` must still load for a native build"
        );
    }
}

#[test]
fn a_webassembly_build_refuses_every_capability_module() {
    for module in tyrus_core::module_loader::SANDBOX_FORBIDDEN_MODULES {
        let source = format!("import * as m from \"{module}\";\n{MAIN}");
        let error = check("wasm", &source, ModuleRestriction::WasmTarget)
            .expect_err("a WebAssembly build must refuse a capability module");
        assert!(
            error.contains(module) && error.contains("WebAssembly"),
            "the diagnostic must name the module and the target: {error}"
        );
        assert!(
            error.contains("help:"),
            "the diagnostic must say what to do instead: {error}"
        );
    }
}

#[test]
fn a_module_that_is_not_portable_yet_says_why_without_claiming_system_access() {
    for module in tyrus_core::module_loader::NOT_YET_PORTABLE_MODULES {
        let source = format!("import * as m from \"{module}\";\n{MAIN}");
        for (restriction, place) in [
            (ModuleRestriction::Sandbox, "in a sandboxed build"),
            (
                ModuleRestriction::WasmTarget,
                "when compiling to WebAssembly",
            ),
        ] {
            let error = check("pending", &source, restriction)
                .expect_err("a module that is not portable yet must be refused");
            assert!(
                error.contains(module) && error.contains(place) && error.contains("help:"),
                "the diagnostic must name the module, the target and a way out: {error}"
            );
            assert!(!error.contains("operating system access"), "{error}");
        }
        assert!(check("pending-native", &source, ModuleRestriction::None).is_ok());
    }
}
