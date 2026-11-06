use std::path::PathBuf;
use tyrus_codegen::CompileTarget;

fn project_with(label: &str, source: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "tyrus-wasm-ir-{label}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("tyrus.json"), "{}").unwrap();
    std::fs::write(root.join("src/main.tyr"), source).unwrap();
    root
}

fn ir_for(label: &str, source: &str, target: CompileTarget) -> String {
    let root = project_with(label, source);
    let program = tyrus_core::module_loader::load_program(&root.join("src/main.tyr"), None)
        .unwrap_or_else(|error| {
            let _ = std::fs::remove_dir_all(&root);
            panic!("`{label}`: the front end rejected this program: {error:?}");
        });
    let output = root.join("out");
    let result = tyrus_codegen::compile_with_env_options_and_target(
        &program,
        "wasm_ir",
        output.to_str().unwrap(),
        false,
        None,
        None,
        target,
    );
    if let Err(error) = result {
        let _ = std::fs::remove_dir_all(&root);
        panic!("`{label}` failed to emit for {target:?}: {error}");
    }
    let ir = std::fs::read_to_string(output.with_extension("ll"))
        .unwrap_or_else(|error| panic!("`{label}` did not produce readable IR: {error}"));
    let _ = std::fs::remove_dir_all(&root);
    ir
}

fn emits_for(label: &str, source: &str, target: CompileTarget) {
    let _ = ir_for(label, source, target);
}

const LENGTH_ARITHMETIC: &[(&str, &str)] = &[
    (
        "map",
        r#"
            const main = (): void => {
                const a: i32[] = [1, 2, 3];
                console.log(a.map((n: i32): i32 => n * 2).length);
            };
        "#,
    ),
    (
        "filter",
        r#"
            const main = (): void => {
                const a: i32[] = [1, 2, 3];
                console.log(a.filter((n: i32): boolean => n > 1).length);
            };
        "#,
    ),
    (
        "reduce",
        r#"
            const main = (): void => {
                const a: i32[] = [1, 2, 3];
                console.log(a.reduce((x: i32, y: i32): i32 => x + y, 0));
            };
        "#,
    ),
    (
        "sort",
        r#"
            const main = (): void => {
                let a: i32[] = [3, 1, 2];
                a.sort();
                console.log(a[0]);
            };
        "#,
    ),
    (
        "push-and-length",
        r#"
            const main = (): void => {
                let a: i32[] = [1];
                a.push(2);
                console.log(a.length);
            };
        "#,
    ),
    (
        "tuple-borrowed-view",
        r#"
            const main = (): void => {
                const pair: [i32, i32] = [1, 2];
                console.log(pair.map((n: i32): i32 => n + 1).length);
            };
        "#,
    ),
    (
        "set-literal",
        r#"
            const main = (): void => {
                const roles = Set<string>(["admin", "editor", "admin"]);
                console.log(roles.size);
            };
        "#,
    ),
    (
        "map-literal",
        r#"
            const main = (): void => {
                const ages: Map<string, i32> = Map<string, i32>();
                ages.set("ada", 36);
                console.log(ages.size);
            };
        "#,
    ),
    (
        "group-by",
        r#"
            const main = (): void => {
                const grouped = [1, 2, 3].groupBy((n: i32): boolean => n % 2 === 0);
                console.log(grouped.size);
            };
        "#,
    ),
    (
        "owned-element-array",
        r#"
            const main = (): void => {
                const nested: i32[][] = [[1, 2], [3]];
                const part: i32[][] = nested.slice(0, 1);
                console.log(part.length);
            };
        "#,
    ),
    (
        "validation-error-describe",
        r#"
            interface Account {
                @min(18)
                age: i32;
            }
            const main = (): void => {
                const account: Account = { age: 20 };
                console.log(account.age);
            };
        "#,
    ),
];

#[test]
fn length_arithmetic_emits_valid_ir_for_wasm() {
    for (label, source) in LENGTH_ARITHMETIC {
        emits_for(label, source, CompileTarget::Wasm);
    }
}

#[test]
fn length_arithmetic_emits_valid_ir_for_native() {
    for (label, source) in LENGTH_ARITHMETIC {
        emits_for(label, source, CompileTarget::Native);
    }
}

const SHARED_RUNTIME_BUILTINS: &[(&str, &str)] = &[
    (
        "array-slice-splice-with",
        r#"
            const main = (): void => {
                let a: i32[] = [1, 2, 3, 4, 5];
                console.log(a.slice(1, 3).length);
                console.log(a.with(0, 9)[0]);
                console.log(a.keys().length);
                a.splice(1, 1);
                console.log(a.length);
            };
        "#,
    ),
    (
        "array-unique",
        r#"
            const main = (): void => {
                const a: i32[] = [1, 2, 2, 3];
                console.log(a.unique().length);
            };
        "#,
    ),
    (
        "math-statistics",
        r#"
            import { median, variance, stdDev, mode, percentile } from "tyrus:math";
            const main = (): void => {
                const data: f64[] = [1.0, 2.0, 2.0, 3.0];
                console.log(median(data));
                console.log(variance(data));
                console.log(stdDev(data));
                console.log(mode(data));
                console.log(percentile(50.0, data));
            };
        "#,
    ),
    (
        "math-combinatorics",
        r#"
            import { combination, permutation, primeFactors, roundTo, logBase } from "tyrus:math";
            const main = (): void => {
                console.log(combination(5, 2));
                console.log(permutation(5, 2));
                const factors: u64[] = primeFactors(12);
                console.log(factors.length);
                console.log(roundTo(3.14159, 2));
                console.log(logBase(8.0, 2.0));
            };
        "#,
    ),
];

#[test]
fn shared_runtime_builtins_emit_for_wasm() {
    for (label, source) in SHARED_RUNTIME_BUILTINS {
        emits_for(label, source, CompileTarget::Wasm);
    }
}

#[test]
fn shared_runtime_builtins_emit_for_native() {
    for (label, source) in SHARED_RUNTIME_BUILTINS {
        emits_for(label, source, CompileTarget::Native);
    }
}

const PLAYGROUND_EXAMPLES: &[(&str, &str)] = &[
    (
        "fibonacci",
        r#"
            const fib = (n: i32): i32 => {
                if (n < 2) { return n; }
                return fib(n - 1) + fib(n - 2);
            };
            const main = (): void => {
                for (let i: i32 = 0; i < 10; i += 1) {
                    const label: string = "fib";
                    console.log(`${label}(${i.toString()}) = ${fib(i).toString()}`);
                }
            };
        "#,
    ),
    (
        "match-with-option",
        r#"
            const describe = (value: i32?): string => {
                if (value) {
                    const inner: string = value.toString();
                    return `some ${inner}`;
                }
                return "none at all";
            };
            const main = (): void => {
                let present: i32? = 7;
                console.log(describe(present));
                let absent: i32?;
                console.log(describe(absent));
            };
        "#,
    ),
    (
        "loops",
        r#"
            const main = (): void => {
                const names: string[] = ["ada", "grace"];
                for (const name of names) {
                    console.log(`hello, ${name}`);
                }
                let count: i32 = 0;
                while (count < 3) {
                    console.log(`tick ${count.toString()}`);
                    count += 1;
                }
            };
        "#,
    ),
    (
        "pipe-operator",
        r#"
            const shout = (text: string): string => text.toUpperCase();
            const exclaim = (text: string): string => `${text}!`;
            const main = (): void => {
                const greeting: string = "hello";
                const loud: string = greeting |> shout |> exclaim;
                console.log(`said: ${loud}`);
                console.log(`again: ${loud}`);
                console.log(`third: ${loud}`);
            };
        "#,
    ),
];

#[test]
fn playground_examples_emit_for_wasm() {
    for (label, source) in PLAYGROUND_EXAMPLES {
        emits_for(label, source, CompileTarget::Wasm);
    }
}

#[test]
fn playground_examples_emit_for_native() {
    for (label, source) in PLAYGROUND_EXAMPLES {
        emits_for(label, source, CompileTarget::Native);
    }
}

#[test]
fn wasm_string_runtime_calls_use_indirect_abi_thunks() {
    let ir = ir_for(
        "string-abi-thunks",
        r#"
            const main = (): void => {
                const value: string = "runtime".toUpperCase();
                print(`value: ${value}`);
            };
        "#,
        CompileTarget::Wasm,
    );

    assert!(
        ir.contains("@__tyrus_wasm_string_abi_tyrus_print_str"),
        "the generated call must go through the indirect-argument thunk:\n{ir}"
    );
    assert!(
        ir.contains("declare void @tyrus_print_str(ptr, i32)"),
        "the real runtime declaration must receive a string address:\n{ir}"
    );
    assert!(
        ir.contains("@__tyrus_wasm_string_abi_tyrus_string_to_upper"),
        "a string-returning call must go through the sret thunk:\n{ir}"
    );
    assert!(
        ir.contains("declare void @tyrus_string_to_upper(ptr, ptr)"),
        "the real runtime declaration must receive the result and argument addresses:\n{ir}"
    );
}

#[test]
fn wasm_string_comparison_uses_target_sized_memcmp_length() {
    let ir = ir_for(
        "string-comparison-memcmp-size",
        r#"
            const main = (): void => {
                const matches: boolean = "fast-result" === "slow-result";
                print(matches);
            };
        "#,
        CompileTarget::Wasm,
    );

    assert!(
        ir.contains("call i32 @memcmp(ptr"),
        "WASM string comparison must call memcmp with target-sized length values:\n{ir}"
    );
    assert!(
        ir.contains("declare i32 @memcmp(ptr, ptr, i32)"),
        "WASM memcmp must not receive an i64 size_t:\n{ir}"
    );
}
