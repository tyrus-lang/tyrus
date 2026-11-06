use std::fs;

use tyrus_codegen as codegen;
use tyrus_core::module_loader;

fn emitted_ir(name: &str, source: &str) -> String {
    let dir = std::env::temp_dir().join(format!("tyrus-capture-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("scratch dir");
    let entry = dir.join("main.tyr");
    fs::write(&entry, source).expect("write source");
    let program = module_loader::load_program(&entry, None).expect("load failed");
    let out = dir.join(name);
    codegen::compile(&program, name, out.to_str().unwrap(), false).expect("codegen failed");
    let ir = fs::read_to_string(out.with_extension("ll")).expect("read ir");
    let _ = fs::remove_dir_all(&dir);
    ir
}

fn functions_containing(ir: &str, needle: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut current: Option<String> = None;
    for line in ir.lines() {
        if line.starts_with("define ") {
            current = line.contains(needle).then(|| String::from(line));
        } else if let Some(body) = current.as_mut() {
            body.push('\n');
            body.push_str(line);
            if line == "}" {
                found.push(current.take().unwrap());
            }
        }
    }
    found
}

const SOURCE: &str = r#"
import { Mutex, spawn } from "tyrus:task";

const batch = async (): Promise<void> => {
    const counter = Mutex<u32>(0);
    const task = spawn(async (): Promise<void> => {
        const guard = await counter.lock();
        guard.value += 42;
        console.log(guard.value);
    });
    await task.join();
};

function main(): void {
    console.log(1);
}
"#;

#[test]
fn a_captured_mutex_has_independent_environment_and_frame_references() {
    let ir = emitted_ir("captured_mutex", SOURCE);

    let environment_drops = functions_containing(&ir, "_environment_drop");
    assert!(
        !environment_drops.is_empty(),
        "the closure should have an environment drop:\n{ir}"
    );
    assert!(
        environment_drops
            .iter()
            .any(|body| body.contains("@tyrus_mutex_free")),
        "the environment owns the capture and must release it: {environment_drops:?}"
    );
    assert!(
        ir.matches("async.closure.capture.mutex.retain").count() >= 2,
        "the environment and async frame each need an independent Mutex reference:\n{ir}"
    );

    let arrow_frame_drops = functions_containing(&ir, "__tyrus_async_drop___tyrus_async_arrow");
    assert!(
        !arrow_frame_drops.is_empty(),
        "the async closure should have frame drop glue:\n{ir}"
    );
    for body in &arrow_frame_drops {
        assert!(
            body.contains("@tyrus_mutex_free"),
            "the frame owns its retained capture reference and must release it: {body}"
        );
    }
}
