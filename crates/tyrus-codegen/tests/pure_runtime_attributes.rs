use std::fs;

use tyrus_codegen as codegen;
use tyrus_core::module_loader;

fn emitted_ir(name: &str, source: &str) -> String {
    let dir = std::env::temp_dir().join(format!("tyrus-attrs-{name}-{}", std::process::id()));
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

fn attribute_group(ir: &str, symbol: &str) -> String {
    let line = ir
        .lines()
        .find(|line| line.starts_with("declare") && line.contains(symbol))
        .unwrap_or_else(|| panic!("no declaration of `{symbol}` in:\n{ir}"));
    let Some(hash) = line.rsplit(' ').next().filter(|last| last.starts_with('#')) else {
        return String::new();
    };
    ir.lines()
        .find(|line| line.starts_with(&format!("attributes {hash} =")))
        .unwrap_or_default()
        .to_string()
}

#[test]
fn a_pure_and_total_helper_is_declared_as_pure() {
    let ir = emitted_ir(
        "pure_total",
        r#"
import { sqrt } from "tyrus:math";

function main(): void {
    console.log(sqrt(2.0));
}
"#,
    );
    let group = attribute_group(&ir, "@tyrus_math_sqrt");
    assert!(group.contains("memory(none)"), "sqrt: {group}");
    assert!(group.contains("willreturn"), "sqrt: {group}");
    assert!(group.contains("nounwind"), "sqrt: {group}");
}

#[test]
fn a_helper_that_can_abort_gets_nounwind_and_nothing_more() {
    let ir = emitted_ir(
        "pure_aborting",
        r#"
import { factorial } from "tyrus:math";

function main(): void {
    console.log(factorial(5));
}
"#,
    );
    let group = attribute_group(&ir, "@tyrus_math_factorial");
    assert!(group.contains("nounwind"), "factorial: {group}");
    assert!(!group.contains("memory(none)"), "factorial: {group}");
    assert!(!group.contains("willreturn"), "factorial: {group}");
}

#[test]
fn a_helper_that_draws_randomness_is_not_described_as_pure() {
    let ir = emitted_ir(
        "impure_random",
        r#"
import { randomRange } from "tyrus:math";

function main(): void {
    console.log(randomRange(0.0, 1.0));
}
"#,
    );
    let group = attribute_group(&ir, "@tyrus_math_random_range");
    assert!(!group.contains("memory(none)"), "randomRange: {group}");
    assert!(!group.contains("willreturn"), "randomRange: {group}");
}

#[test]
fn a_helper_that_only_reads_its_arguments_says_so() {
    let ir = emitted_ir(
        "reading_total",
        r#"
import { mean } from "tyrus:math";

function main(): void {
    const data: f64[] = [1.0, 2.0, 3.0];
    console.log(mean(data));
}
"#,
    );
    let group = attribute_group(&ir, "@tyrus_math_mean");
    assert!(group.contains("memory(read)"), "mean: {group}");
    assert!(group.contains("willreturn"), "mean: {group}");
    assert!(group.contains("nounwind"), "mean: {group}");
    assert!(
        !group.contains("memory(none)"),
        "mean reads its argument buffer: {group}"
    );
}

#[test]
fn a_helper_that_allocates_to_sort_is_not_described_as_reading_only() {
    let ir = emitted_ir(
        "allocating_sort",
        r#"
import { median } from "tyrus:math";

function main(): void {
    const data: f64[] = [3.0, 1.0, 2.0];
    console.log(median(data));
}
"#,
    );
    let group = attribute_group(&ir, "@tyrus_math_median");
    assert!(!group.contains("memory(read)"), "median: {group}");
    assert!(!group.contains("memory(none)"), "median: {group}");
}
