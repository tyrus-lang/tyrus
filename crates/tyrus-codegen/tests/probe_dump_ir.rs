use std::fs;
use std::path::Path;

use tyrus_codegen as codegen;
use tyrus_core::module_loader;

#[test]
fn probe_dump_full_ir() {
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = Path::new(&root);
    let entry = root.join("src/probe_main.tyr");
    let _ = fs::create_dir_all(entry.parent().unwrap());
    fs::write(
        &entry,
        "\
let counter: i32 = 0;
const add = (a: i32, b: i32): i32 => a + b;
const __repl_step_1 = (): void => { counter = 5; console.log(add(counter, 1)); };
const __repl_step_2 = (): void => { console.log(\"hello\".includes(\"he\")); };
",
    )
    .unwrap();
    let program = module_loader::load_program(&entry, None).expect("load failed");
    let out = root.join("probe_full.ll");
    codegen::compile(&program, "probe", out.to_str().unwrap(), false).expect("codegen failed");
    fs::remove_file(&entry).ok();
}
