use std::path::PathBuf;

fn project_with(label: &str, source: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "tyrus-invariant-{label}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("tyrus.json"), "{}").unwrap();
    std::fs::write(root.join("src/main.tyr"), source).unwrap();
    root
}

#[must_use]
fn check_implies_codegen(label: &str, source: &str) -> bool {
    let root = project_with(label, source);
    let Ok(program) = tyrus_core::module_loader::load_program(&root.join("src/main.tyr"), None)
    else {
        let _ = std::fs::remove_dir_all(&root);
        return false;
    };
    let output = root.join("out");
    let result = tyrus_codegen::compile(&program, "invariant", output.to_str().unwrap(), false);
    let _ = std::fs::remove_dir_all(&root);
    if let Err(error) = result {
        panic!(
            "`{label}`: the front end approved this program but code generation rejected it.\n\
             This is the `tyrus check` contract being broken: the restriction below belongs in \
             the front end, with a stable code, a precise range and a `help:`.\n\
             backend said: {error}"
        );
    }
    true
}

#[test]
fn a_closure_capturing_two_levels_out_is_diagnosed_by_the_front_end() {
    let _ = check_implies_codegen(
        "transitive-capture",
        r#"
            const triple = (a: i32): (i32) => (i32) => i32 => {
                return (b: i32): (i32) => i32 => {
                    return (c: i32): i32 => a + b + c;
                };
            };

            const main = (): void => {
                console.log(triple(1)(2)(3));
            };
        "#,
    );
}

#[test]
fn native_module_arguments_resolved_at_compile_time_are_checked_by_the_front_end() {
    for (label, source) in [
        (
            "fs-encoding-literal",
            r#"
                import { readFileSync } from "tyrus:fs";
                const main = (): void => {
                    console.log(readFileSync("a.txt", "latin1"));
                };
            "#,
        ),
        (
            "fs-encoding-variable",
            r#"
                import { readFileSync as rf } from "tyrus:fs";
                const main = (): void => {
                    const encoding: string = "utf8";
                    console.log(rf("a.txt", encoding));
                };
            "#,
        ),
    ] {
        let _ = check_implies_codegen(label, source);
    }
}

#[test]
fn cli_is_tty_generates_after_check_approves() {
    let _ = check_implies_codegen(
        "cli-is-tty",
        r#"
            import { isTTY } from "tyrus:cli";
            const main = (): void => {
                const interactive: boolean = isTTY();
                console.log(interactive);
            };
        "#,
    );
}

#[test]
fn ordinary_programs_hold_the_invariant() {
    let corpus: &[(&str, &str)] = &[
        (
            "closure-one-level",
            r#"
                const makeAdder = (base: i32): (i32) => i32 => {
                    return (value: i32): i32 => base + value;
                };
                const main = (): void => {
                    console.log(makeAdder(5)(3));
                };
            "#,
        ),
        (
            "strings-and-arrays",
            r#"
                const main = (): void => {
                    const parts: string[] = "a,b,c".split(",");
                    const upper: string = parts[0].toUpperCase();
                    const doubled: i32[] = [1, 2, 3].map((v: i32): i32 => v * 2);
                    const widened: i64[] = [1, 2];
                    console.log(upper);
                    console.log(doubled.length);
                    console.log(widened[0]);
                };
            "#,
        ),
        (
            "interfaces-and-optionals",
            r#"
                interface User { id: i32; nickname?: string }
                const main = (): void => {
                    const user: User = { id: 1 };
                    console.log(user.nickname ?? "anonymous");
                };
            "#,
        ),
        (
            "at-least-one-object-union",
            r#"
                interface Form { name: string; email?: string; phone?: string; }
                type ValidForm = AtLeastOne<Form, "email" | "phone">;
                const main = (): void => {
                    const byEmail: ValidForm = { name: "Ada", email: "ada@example.com" };
                    const byBoth: ValidForm = { name: "Grace", email: "grace@example.com", phone: "555-0100" };
                    console.log(byEmail.name);
                    console.log(byBoth.name);
                };
            "#,
        ),
        (
            "switch-and-loops",
            r#"
                const classify = (n: i32): string => {
                    let label: string = "";
                    switch (n) {
                        case 0:
                            label = "zero";
                            break;
                        default:
                            label = "other";
                    }
                    return label;
                };
                const main = (): void => {
                    for (let i: i32 = 0; i < 3; i += 1) {
                        console.log(classify(i));
                    }
                };
            "#,
        ),
        (
            "generics-and-tuples",
            r#"
                function first<T>(values: T[]): T {
                    return values[0];
                }
                const main = (): void => {
                    const pair: [i32, string] = [1, "one"];
                    console.log(first<i32>([10, 20]));
                    console.log(pair[1]);
                };
            "#,
        ),
        (
            "error-handling",
            r#"
                const divide = (a: f64, b: f64): f64 throws string => {
                    if (b === 0.0) {
                        throw "division by zero";
                    }
                    return a / b;
                };
                const main = (): void => {
                    try {
                        console.log(divide(1.0, 0.0));
                    } catch (error: string) {
                        console.log(error);
                    }
                };
            "#,
        ),
        (
            "closures-over-owned-values",
            r#"
                interface User { id: i32 }
                const holdString = (text: string): () => string => {
                    return (): string => text;
                };
                const holdObject = (user: User): () => i32 => {
                    return (): i32 => user.id;
                };
                const main = (): void => {
                    const user: User = { id: 1 };
                    console.log(holdString("a")());
                    console.log(holdObject(user)());
                };
            "#,
        ),
        (
            "collections-of-composites",
            r#"
                interface User { id: i32 }
                const main = (): void => {
                    const users = Map<string, User>();
                    const user: User = { id: 1 };
                    users.set("a", user);
                    const names = Set<string>();
                    names.add("a");
                    for (const name of names) {
                        console.log(name);
                    }
                    const pairs: [i32, string][] = [[1, "a"]];
                    const grid: i32[][] = [[1], [2]];
                    console.log(users.size);
                    console.log(pairs.length);
                    console.log(grid.length);
                };
            "#,
        ),
        (
            "async-over-owned-payloads",
            r#"
                interface User { id: i32 }
                const loadUser = async (): Promise<User> => {
                    const user: User = { id: 1 };
                    return user;
                };
                const loadNames = async (): Promise<string[]> => {
                    return "a,b".split(",");
                };
                const main = async (): Promise<void> => {
                    const user: User = await loadUser();
                    const names: string[] = await loadNames();
                    console.log(user.id);
                    console.log(names.length);
                };
            "#,
        ),
        (
            "generics-over-arrays-and-tuples",
            r#"
                function firstOf<T>(values: T[]): T {
                    return values[0];
                }
                function pairOf<A, B>(left: A, right: B): [A, B] {
                    return [left, right];
                }
                const main = (): void => {
                    const pair: [i32, string] = pairOf<i32, string>(1, "a");
                    console.log(firstOf<string>(["a", "b"]));
                    console.log(pair[0]);
                };
            "#,
        ),
        (
            "numeric-conversions-both-forms",
            r#"
                const main = (): void => {
                    const wide: i64 = 42;
                    const narrow: i32 = wide as i32;
                    const missing: i32? = u64.MAX as i32?;
                    console.log(narrow);
                    console.log(missing ?? -1);
                };
            "#,
        ),
        (
            "math-constants",
            r#"
                import { PI, TAU } from "tyrus:math";
                const main = (): void => {
                    console.log(TAU * 3.0);
                };
            "#,
        ),
    ];
    for (label, source) in corpus {
        assert!(
            check_implies_codegen(label, source),
            "`{label}` is a valid program, so it must reach code generation. \
             Being rejected by the front end would make this suite pass without \
             ever exercising the backend."
        );
    }
}

fn rejected_by_front_end(label: &str, source: &str) -> Option<bool> {
    let root = project_with(label, source);
    let loaded = tyrus_core::module_loader::load_program(&root.join("src/main.tyr"), None);
    let answer = match loaded {
        Err(_) => Some(true),
        Ok(program) => {
            let output = root.join("out");
            match tyrus_codegen::compile(&program, "invariant", output.to_str().unwrap(), false) {
                Err(_) => Some(false),
                Ok(()) => None,
            }
        }
    };
    let _ = std::fs::remove_dir_all(&root);
    answer
}

#[test]
fn invalid_programs_are_rejected_by_the_front_end_and_not_by_the_backend() {
    let invalid: &[(&str, &str)] = &[
        (
            "tuple-index-out-of-bounds",
            r#"const main = (): void => { const t: [i32, string] = [1, "a"]; console.log(t[5]); };"#,
        ),
        (
            "tuple-index-not-a-literal",
            r#"const main = (): void => { const t: [i32, i32] = [1, 2]; const i: i32 = 0; console.log(t[i]); };"#,
        ),
        (
            "index-into-a-non-array",
            r#"const main = (): void => { const n: i32 = 5; console.log(n[0]); };"#,
        ),
        (
            "optional-chain-on-a-non-optional",
            r#"const main = (): void => { const s: string = "a"; console.log(s?.length); };"#,
        ),
        (
            "call-a-non-function",
            r#"const main = (): void => { const n: i32 = 5; console.log(n(1)); };"#,
        ),
        (
            "mixed-numeric-operands",
            r#"const main = (): void => { const a: i32 = 5; const b: i64 = 2; console.log(a % b); };"#,
        ),
        (
            "unknown-field",
            r#"interface U { id: i32 }
               const main = (): void => { const u: U = { id: 1 }; console.log(u.nope); };"#,
        ),
        (
            "array-interpolated-into-a-string",
            r#"const main = (): void => { const a: i32[] = [1]; console.log(`${a}`); };"#,
        ),
        (
            "async-main-with-a-parameter",
            r#"const main = async (x: i32): Promise<void> => { console.log(x); };"#,
        ),
        (
            "main-returning-a-value",
            r#"const main = (): i32 => { return 1; };"#,
        ),
        (
            "entry-module-without-main",
            r#"const helper = (): void => {};"#,
        ),
        (
            "unsupported-file-encoding",
            r#"import { readFileSync } from "tyrus:fs";
               const main = (): void => { console.log(readFileSync("a.txt", "latin1")); };"#,
        ),
        (
            "file-encoding-from-a-variable",
            r#"import { readFileSync } from "tyrus:fs";
               const main = (): void => { const e: string = "utf8"; console.log(readFileSync("a.txt", e)); };"#,
        ),
        (
            "capture-from-two-functions-out",
            r#"const triple = (a: i32): (i32) => (i32) => i32 => {
                   return (b: i32): (i32) => i32 => {
                       return (c: i32): i32 => a + b + c;
                   };
               };
               const main = (): void => { console.log(triple(1)(2)(3)); };"#,
        ),
    ];
    for (label, source) in invalid {
        match rejected_by_front_end(label, source) {
            Some(true) => {}
            Some(false) => panic!(
                "`{label}`: only code generation rejected this program. The restriction belongs \
                 in the front end, with a stable code, a precise range and a `help:`, so that \
                 `tyrus check` does not report success on a program that cannot be built."
            ),
            None => panic!(
                "`{label}` is meant to be invalid but compiled. Either the program is no longer \
                 a good example of the restriction, or a check was lost."
            ),
        }
    }
}

#[test]
fn async_locals_the_frame_cannot_adopt_are_diagnosed_by_the_front_end() {
    // A non-Copy array local that lives across an `await` is adopted by the
    // frame only from a literal or a call to a module-level function. The
    // front end once accepted any initializer, and a native call, a method
    // call or a local closure reached code generation and failed there.
    let prelude = r#"
        import { CpuInfo, cpus } from "tyrus:os";
        import { sleepAsync } from "tyrus:task";

        interface Paint {
            name: string;
            shade: i32;
        }

        function declared(): Paint[] {
            return [{ name: "declared", shade: 2 }];
        }

        const loadCores = (): CpuInfo[] => cpus();
    "#;
    let mut reached_codegen = 0;
    for (label, body) in [
        (
            "native-call",
            "const v: CpuInfo[] = cpus(); await sleepAsync(1); print(v.length);",
        ),
        (
            "method-call",
            "const all: Paint[] = declared(); const v: Paint[] = all.filter((p: Paint): boolean => p.shade > 0); await sleepAsync(1); print(v.length);",
        ),
        (
            "local-closure",
            "const local = (): Paint[] => [{ name: \"l\", shade: 1 }]; const v: Paint[] = local(); await sleepAsync(1); print(v[0].name);",
        ),
        (
            "declared-function",
            "const v: Paint[] = declared(); await sleepAsync(1); print(v[0].name);",
        ),
        (
            "wrapped-native-call",
            "const v: CpuInfo[] = loadCores(); await sleepAsync(1); print(v.length);",
        ),
        (
            "literal",
            "const v: Paint[] = [{ name: \"lit\", shade: 5 }]; await sleepAsync(1); print(v[0].name);",
        ),
    ] {
        let source = format!(
            "{prelude}\nasync function run(): Promise<void> {{ {body} }}\nconst main = async (): Promise<void> => {{ await run(); }};\n"
        );
        if check_implies_codegen(label, &source) {
            reached_codegen += 1;
        }
    }
    assert_eq!(
        reached_codegen, 3,
        "the declared-function, wrapped-native-call and literal forms must reach code generation"
    );
}

#[test]
fn closures_written_inside_a_try_generate_valid_code() {
    // A closure is a function of its own, but it inherited the enclosing
    // `try` bookkeeping: its `return` restored the outer `try` context from a
    // value of the enclosing function, and LLVM rejected the whole module.
    for (label, source) in [
        (
            "sync-closure",
            r#"
                const main = (): void => {
                    try {
                        const add = (value: i32): i32 => value + 1;
                        print(add(1));
                    } catch (error) {
                        printError(error.message);
                    }
                };
            "#,
        ),
        (
            "callback-with-finally",
            r#"
                const main = (): void => {
                    try {
                        const values: i32[] = [1, 2, 3];
                        print(values.filter((value: i32): boolean => { return value > 1; }).length);
                    } catch (error) {
                        printError(error.message);
                    } finally {
                        print("finally");
                    }
                };
            "#,
        ),
        (
            "closure-in-catch",
            r#"
                function fail(): void throws string {
                    throw "boom";
                }
                const main = (): void => {
                    try {
                        fail();
                    } catch (error: string) {
                        const shout = (text: string): string => text.toUpperCase();
                        print(shout(error));
                    }
                };
            "#,
        ),
        (
            "async-try-callback",
            r#"
                import { sleepAsync } from "tyrus:task";
                const main = async (): Promise<void> => {
                    try {
                        await sleepAsync(1);
                        const values: i32[] = [1, 2, 3];
                        print(values.filter((value: i32): boolean => value > 1).length);
                        await sleepAsync(1);
                    } catch (error) {
                        printError(error.message);
                    }
                };
            "#,
        ),
        (
            "async-closure-in-try",
            r#"
                const main = async (): Promise<void> => {
                    try {
                        const compute = async (value: i32): Promise<i32> => { return value * 3; };
                        const value: i32 = await compute(2);
                        print(value);
                    } catch (error) {
                        printError(error.message);
                    }
                };
            "#,
        ),
    ] {
        assert!(
            check_implies_codegen(label, source),
            "`{label}` must reach code generation"
        );
    }
}

#[test]
fn async_for_of_over_non_copy_elements_reaches_code_generation() {
    for (label, body) in [
        (
            "no-await-in-body",
            "const paints: Paint[] = [{ name: \"a\", shade: 1 }]; for (const paint of paints) { print(paint.name); } await sleepAsync(1);",
        ),
        (
            "await-in-body",
            "const paints: Paint[] = [{ name: \"a\", shade: 1 }]; for (const paint of paints) { await sleepAsync(1); print(paint.name); }",
        ),
        (
            "string-elements",
            "const words: string[] = [\"a\", \"b\"]; for (const word of words) { await sleepAsync(1); print(word); }",
        ),
    ] {
        let source = format!(
            "import {{ sleepAsync }} from \"tyrus:task\";\ninterface Paint {{\n    name: string;\n    shade: i32;\n}}\nasync function run(): Promise<void> {{ {body} }}\nconst main = async (): Promise<void> => {{ await run(); }};\n"
        );
        assert!(
            check_implies_codegen(label, &source),
            "`{label}` must reach code generation"
        );
    }
}

#[test]
fn question_mark_inside_an_async_function_reaches_code_generation() {
    for (label, body) in [
        (
            "sync-result",
            "const value = parse(ok)?; return Ok(value + 1);",
        ),
        (
            "awaited-local",
            "const result = await load(ok); const value = result?; return Ok(value);",
        ),
    ] {
        let source = format!(
            "const parse = (ok: boolean): Result<i32, string> => {{ if (ok) {{ return Ok(1); }} return Err(\"no\"); }};\nconst load = async (ok: boolean): Promise<Result<i32, string>> => {{ return parse(ok); }};\nconst run = async (ok: boolean): Promise<Result<i32, string>> => {{ {body} }};\nconst main = async (): Promise<void> => {{ const result = await run(true); match (result) {{ Ok(value) => {{ print(value); }} Err(error) => {{ print(error); }} }} }};\n"
        );
        assert!(
            check_implies_codegen(label, &source),
            "`{label}` must reach code generation"
        );
    }
}

#[test]
fn hoisted_awaits_and_strings_read_from_fields_reach_code_generation() {
    for (label, body) in [
        (
            "match-subject",
            "match (await load(true)) { Ok(value) => { print(value); } Err(error) => { print(error); } }",
        ),
        (
            "argument-in-a-loop",
            "for (let i = 0; i < 2; i += 1) { show(await load(i === 0)); }",
        ),
        ("if-condition", "if (await ready()) { print(1); }"),
        (
            "string-from-a-field",
            "const user: User = { name: `a${1}` }; const name = user.name; await ready(); print(name);",
        ),
    ] {
        let source = format!(
            "interface User {{\n    name: string;\n}}\nconst load = async (ok: boolean): Promise<Result<i32, string>> => {{ if (ok) {{ return Ok(1); }} return Err(\"no\"); }};\nconst ready = async (): Promise<boolean> => {{ return true; }};\nconst show = (result: Result<i32, string>): void => {{ match (result) {{ Ok(value) => {{ print(value); }} Err(error) => {{ print(error); }} }} }};\nconst main = async (): Promise<void> => {{ {body} }};\n"
        );
        assert!(
            check_implies_codegen(label, &source),
            "`{label}` must reach code generation"
        );
    }
    let source = "const load = async (ok: boolean): Promise<Result<i32, string>> => { return Ok(1); };\nconst next = async (ok: boolean): Promise<Result<i32, string>> => { return Ok(await load(ok)? + 1); };\nconst main = async (): Promise<void> => { const result = await next(true); match (result) { Ok(value) => { print(value); } Err(error) => { print(error); } } };\n";
    assert!(check_implies_codegen("await-then-try", source));
}

#[test]
fn void_results_and_borrowed_arrays_in_async_functions_reach_code_generation() {
    let source = "import { sleepAsync } from \"tyrus:task\";\ninterface Paint {\n    name: string;\n}\ninterface Palette {\n    paints: Paint[];\n}\nconst load = async (): Promise<Result<Paint[], string>> => { const paints: Paint[] = [{ name: `a${1}` }]; return Ok(paints); };\nconst check = async (): Promise<Result<void, string>> => { const paints = await load()?; const palette: Palette = { paints: paints }; const copy = palette.paints; await sleepAsync(1); print(copy.length); return Ok(); };\nconst main = async (): Promise<void> => { match (await check()) { Ok() => {} Err(error) => { print(error); } } };\n";
    assert!(check_implies_codegen("void-result-borrowed-array", source));
}

#[test]
fn onfail_over_results_reaches_code_generation() {
    for (label, body) in [
        (
            "awaited-into-a-void-function",
            "const value = await load(false) onfail { print(value); return; }; print(value.length);",
        ),
        (
            "awaited-with-an-expression-handler",
            "const value = await load(false) onfail print(value); print(value.length);",
        ),
        (
            "awaited-inside-a-loop",
            "for (let i = 0; i < 2; i += 1) { const value = await load(i === 0) onfail { print(value); continue; }; print(value[0]); }",
        ),
        (
            "synchronous-in-an-async-function",
            "const value = parse(false) onfail print(value); print(value);",
        ),
        (
            "compact-match-arm",
            "match (await load(true)) { Ok(value) => print(value.length); Err(error) => print(error); }",
        ),
        (
            "array-mixing-literals",
            "const names: string[] = [`a${1}`, \"b\"]; await load(true); print(names[0]);",
        ),
    ] {
        let source = format!(
            "const load = async (ok: boolean): Promise<Result<string[], string>> => {{ if (ok) {{ const names: string[] = [`a${{1}}`]; return Ok(names); }} return Err(`e${{2}}`); }};\nconst parse = (ok: boolean): Result<i32, string> => {{ if (ok) {{ return Ok(1); }} return Err(\"no\"); }};\nconst propagate = async (): Promise<Result<usize, string>> => {{ const value = await load(false) onfail print(value); return Ok(value.length); }};\nconst main = async (): Promise<void> => {{ {body} }};\n"
        );
        assert!(
            check_implies_codegen(label, &source),
            "`{label}` must reach code generation"
        );
    }
}

#[test]
fn compress_calls_reach_code_generation() {
    for (label, body) in [
        (
            "text",
            "const packed = await compress(\"x\", Format.Gzip) onfail { return; }; print(packed.length);",
        ),
        (
            "bytes-literal",
            "match (await decompress([1, 2, 3])) { Ok(bytes) => { print(bytes.length); } Err(error) => { print(error.code); } }",
        ),
        (
            "union-parameter",
            "match (await pack(\"x\")) { Ok(size) => { print(size); } Err(error) => { print(error.code); } }",
        ),
        (
            "options-variable",
            "const options: DecompressOptions = { format: Format.Brotli, maxRatio: 500 }; const packed = await compress(\"x\", Format.Brotli, { level: 9 }) onfail { return; }; const back = await decompress(packed, options) onfail { return; }; print(back.length);",
        ),
        (
            "format-variable",
            "const format: Format = Format.DeflateRaw; const packed = await compress(\"x\", format) onfail { return; }; print(packed.length);",
        ),
    ] {
        let source = format!(
            "import {{ CompressError, CompressFormat as Format, CompressOptions, DecompressOptions, compress, decompress }} from \"tyrus:compress\";\nconst pack = async (data: u8[] | string): Promise<Result<usize, CompressError>> => {{ const packed = await compress(data, Format.Deflate)?; return Ok(packed.length); }};\nconst main = async (): Promise<void> => {{ {body} }};\n"
        );
        assert!(
            check_implies_codegen(label, &source),
            "`{label}` must reach code generation"
        );
    }
}

#[test]
fn compression_streams_reach_code_generation() {
    for (label, body) in [
        (
            "compression",
            "const packer = compressionStream(Format.Gzip); await packer.writable.write([1, 2, 3]); await packer.writable.close(); const first: u8[]? = await packer.readable.read(); print((first ?? []).length);",
        ),
        (
            "compression-options",
            "const packer = compressionStream(Format.Brotli, { level: 9 }); await packer.close();",
        ),
        (
            "decompression",
            "const options: DecompressOptions = { format: Format.Brotli, maxOutputBytes: 1_000 }; const unpacker = decompressionStream(options); await unpacker.writable.close();",
        ),
        (
            "detected",
            "const unpacker = decompressionStream(); try { const first: u8[]? = await unpacker.readable.read(); print((first ?? []).length); } catch (error: CompressError) { print(error.code); }",
        ),
        (
            "piped",
            "const packer = compressionStream(Format.Deflate); const unpacker = decompressionStream(); await packer.writable.close(); await packer.readable.pipeTo(unpacker.writable);",
        ),
    ] {
        let source = format!(
            "import {{ CompressError, CompressFormat as Format, CompressOptions, DecompressOptions, compressionStream, decompressionStream }} from \"tyrus:compress\";\nimport {{ StreamError, TransformStream }} from \"tyrus:stream\";\nconst main = async (): Promise<void> => {{ {body} }};\n"
        );
        assert!(
            check_implies_codegen(label, &source),
            "`{label}` must reach code generation"
        );
    }
}

#[test]
fn archive_streams_reach_code_generation() {
    for (label, body) in [
        (
            "pack-stream",
            "const archive: ReadableStream<u8[]> = packStream(\"./site\", Format.TarGz); await archive.close();",
        ),
        (
            "pack-stream-options",
            "const archive: ReadableStream<u8[]> = packStream(\"./site\", Format.Zip, { level: 9, reproducible: true }); const first: u8[]? = await archive.read(); print((first ?? []).length);",
        ),
        (
            "unpack-stream",
            "const archive: ReadableStream<u8[]> = packStream(\"./site\", Format.Tar); match (await unpackStream(archive, \"./out\")) { Ok(summary) => { print(summary.entries); } Err(error) => { print(error.code); } }",
        ),
        (
            "unpack-stream-try",
            "const archive: ReadableStream<u8[]> = packStream(\"./site\", Format.Tar); const summary = await unpackStream(archive, \"./out\", { maxEntries: 10 }) onfail { return; }; print(summary.bytes);",
        ),
        (
            "unpack-stream-temporary",
            "match (await unpackStream(packStream(\"./site\", Format.TarGz), \"./out\")) { Ok(summary) => { print(summary.entries); } Err(error) => { print(error.code); } }",
        ),
    ] {
        let source = format!(
            "import {{ ArchiveError, ArchiveFormat as Format, ArchiveSummary, PackOptions, UnpackOptions, packStream, unpackStream }} from \"tyrus:archive\";\nimport {{ ReadableStream, StreamError }} from \"tyrus:stream\";\nconst main = async (): Promise<void> => {{ {body} }};\n"
        );
        assert!(
            check_implies_codegen(label, &source),
            "`{label}` must reach code generation"
        );
    }
}
