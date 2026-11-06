use super::*;

#[test]
pub(super) fn emits_length_guard_before_array_match_element_accesses() {
    let ir = compile_module_to_ir(
        r#"
            function main(): void {
                const values: i32[] = [1, 2, 3];
                match (values) {
                    [first, ...rest] => { console.log(first, rest.length); }
                    _ => { return; }
                }
            }
        "#,
        "array_match_length_guard",
    );
    let guard = ir
        .find("match.array.0.length.matches")
        .expect("array match must emit a length guard");
    let element = ir
        .find("match.array.0.element.0")
        .expect("array match must access its first element only in the guarded block");
    assert!(
        guard < element,
        "length guard must precede element access:\n{ir}"
    );
    assert!(
        ir.contains("tyrus_array_slice"),
        "rest binding must copy a slice:\n{ir}"
    );
}

#[test]
pub(super) fn emits_automatic_json_for_arrays_and_nested_data_interfaces() {
    let ir = compile_to_ir(
        r#"
            interface Address { city: string; }
            interface User { address: Address; scores: i32[]; }
            function main(): void {
                const user: User = { address: { city: "Ada" }, scores: [10, 20] };
                console.log(user.toJson());
            }
        "#,
        "automatic_json_arrays",
    );
    assert!(
        ir.contains("tyrus_json_array_builder_finish") && ir.contains("tyrus_json_builder_add_raw"),
        "toJson must serialize arrays and nested objects as JSON values:\n{ir}"
    );
}

#[test]
pub(super) fn emits_automatic_json_decoding_for_scalar_arrays() {
    let ir = compile_to_ir(
        r#"
            interface Scores { values: i32[]; }
            function main(): void {
                const decoded = Scores.fromJson("{\\\"values\\\":[1,2,3]}");
                if (decoded) {
                    const scores: Scores = decoded;
                    console.log(scores.values.length);
                }
            }
        "#,
        "automatic_json_decode_arrays",
    );
    assert!(
        ir.contains("tyrus_json_document_get_array") && ir.contains("tyrus_json_array_get_i32"),
        "fromJson must decode scalar arrays through checked runtime handles:\n{ir}"
    );
}

#[test]
pub(super) fn emits_automatic_json_decoding_for_arrays_of_data_interfaces() {
    let ir = compile_to_ir(
        r#"
            interface Address { city: string; }
            interface Directory { entries: Address[]; }
            function main(): void {
                const decoded = Directory.fromJson("{\"entries\":[{\"city\":\"Ada\"}]} ");
                if (decoded) { const directory: Directory = decoded; console.log(directory.entries[0].city); }
            }
        "#,
        "automatic_json_decode_nested_arrays",
    );
    assert!(
        ir.contains("tyrus_json_array_get_raw") && ir.contains("automatic.json.array.nested"),
        "fromJson must recursively decode data interfaces inside arrays:\n{ir}"
    );
}

#[test]
pub(super) fn emits_automatic_debug_for_array_fields() {
    let ir = compile_to_ir(
        r#"
            interface Report {
                title: string;
                scores: i32[];
            }
            function main(): void {
                const report: Report = { title: "result", scores: [7, 9, 10] };
                console.log(report.toString());
            }
        "#,
        "automatic_debug_array",
    );
    assert!(
        ir.contains("automatic.debug.array.has_element"),
        "toString must walk every array element:\n{ir}"
    );
}

#[test]
pub(super) fn emits_string_literal_comparison_for_array_match_patterns() {
    let ir = compile_to_ir(
        r#"
            function main(): void {
                const parts: string[] = ["go", "north"];
                match (parts) {
                    ["go", direction] => { console.log(direction); }
                    _ => { return; }
                }
            }
        "#,
        "array_match_string_literal",
    );
    assert!(
        ir.contains("match.array.literal.value"),
        "missing literal comparison:\n{ir}"
    );
    assert!(
        ir.contains("@memcmp"),
        "string pattern must compare string contents:\n{ir}"
    );
}

#[test]
pub(super) fn releases_array_match_rest_when_a_guard_rejects_the_arm() {
    let ir = compile_to_ir(
        r#"
            function main(): void {
                const values: i32[] = [-1, 2, 3];
                match (values) {
                    [first, ...rest] if (first > 0) => { console.log(rest.length); }
                    _ => { return; }
                }
            }
        "#,
        "array_match_guard_cleanup",
    );
    let cleanup = ir
        .find("match.array.rest.cleanup.0:")
        .expect("a rejected guard must release the owned rest binding");
    let free = ir[cleanup..]
        .find("call void @free")
        .expect("the rest cleanup block must free the copied array buffer");
    assert!(
        free < 600,
        "expected nearby rest cleanup:\n{}",
        &ir[cleanup..]
    );
}

#[test]
pub(super) fn emits_net_resolve_as_an_async_string_array_promise() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-codegen-net-resolve-source-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("main.tyr"),
        r#"
            import { NetError, resolve, reverseLookup } from "tyrus:net";
            async function load(): Promise<void> {
                const addresses: Result<string[], NetError> = await resolve("localhost");
                match (addresses) {
                    Ok(values) => { console.log(values.length); }
                    Err(error) => { console.error(error.code); }
                }
                const name: Result<string, NetError> = await reverseLookup("127.0.0.1");
                match (name) {
                    Ok(value) => { console.log(value); }
                    Err(error) => { console.error(error.code); }
                }
            }
        "#,
    )
    .unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-codegen-net-resolve-{}", std::process::id()));
    compile(&program, "net_resolve", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("@tyrus_net_resolve"), "{ir}");
    assert!(ir.contains("@tyrus_net_reverse_lookup"), "{ir}");
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
pub(super) fn encoding_array_results_are_owned_by_their_bindings() {
    let source = r#"
        import { utf8, utf16 } from "tyrus:encoding";
        function main(): void {
            const bytes: u8[] = utf8.encode("owned bytes");
            const words: u16[] = utf16.encode("owned words");
            console.log(bytes.length + words.length);
        }
    "#;
    let ir = compile_to_ir(source, "owned-encoding-arrays");
    assert!(ir.contains("tyrus_encoding_utf8_encode"), "{ir}");
    assert!(ir.contains("tyrus_encoding_utf16_encode"), "{ir}");
    assert!(
        ir.matches("call void @free(").count() >= 2,
        "encoding array bindings must release their runtime buffers:\n{ir}"
    );
}

#[test]
pub(super) fn emits_memory_zeroize_for_owned_strings_and_u8_arrays() {
    let source = r#"
        function main(): void {
            let secret: string = "token".slice(0);
            let bytes: u8[] = [1, 2, 3];
            __tyrus_memory_zeroize(secret);
            __tyrus_memory_zeroize(bytes);
        }
    "#;
    let ir = compile_to_ir(source, "memory-zeroize");
    assert!(ir.contains("tyrus_memory_zeroize_string_result"), "{ir}");
    assert!(ir.contains("tyrus_memory_zeroize_u8_array_result"), "{ir}");
}

#[test]
pub(super) fn emits_memory_drop_for_owned_strings_and_u8_arrays() {
    let source = r#"
        function main(): void {
            let secret: string = "token".slice(0);
            let bytes: u8[] = [1, 2, 3];
            __tyrus_memory_drop(secret);
            __tyrus_memory_drop(bytes);
        }
    "#;
    let ir = compile_to_ir(source, "memory-drop");
    assert!(ir.contains("tyrus_string_release_buffer"), "{ir}");
    assert!(ir.contains("call void @free"), "{ir}");
}

#[test]
pub(super) fn array_element_in_dispatches_to_the_same_helper_as_includes() {
    let source = r#"
        function main() {
            const nums: i32[] = [1, 2, 3];
            console.log(2 in nums);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-in-array-{}", std::process::id()));
    compile(&program, "in_array", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("call i64 @tyrus_array_index_of("),
        "expected a call to tyrus_array_index_of:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn for_of_over_a_range_emits_a_counting_loop_with_no_array_materialization() {
    let source = r#"
        function main() {
            let sum: i32 = 0;
            for (const i of 0..5) {
                sum += i;
            }
            console.log(sum);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-forof-range-{}", std::process::id()));
    compile(&program, "forof_range", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("forof_range.cond") && ir.contains("forof_range.body"),
        "expected the range counting-loop blocks:\n{ir}"
    );
    assert!(
        !ir.contains("forof_data") && !ir.contains("@malloc("),
        "a for...of over a Range must not materialize an array:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn string_array_slice_concat_toreversed_with_and_values_clone_their_elements() {
    // A raw byte copy only duplicates each `string` pointer, never the string
    // it points to, the destination array still frees its own elements at
    // scope exit, corrupting memory (confirmed under valgrind: an invalid
    // free pointing at a `.rodata` segment for a literal source). Each of
    // these five methods must instead call the owned-string runtime variant,
    // which `strdup`s every element.
    let source = r#"
        function main() {
            const letters: string[] = ["a", "b", "c", "d", "e"];
            console.log(letters[1..4]);
            console.log(letters.toReversed());
            console.log(letters.values());
            console.log(letters.concat(["f", "g"]));
            console.log(letters.with(0, "z"));
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-string-array-owned-{}", std::process::id()));
    compile(
        &program,
        "string_array_owned",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    for expected in [
        "call i32 @tyrus_array_slice_owned_strings(",
        "call i32 @tyrus_array_concat_owned_strings(",
        "call i32 @tyrus_array_with_owned_strings(",
    ] {
        assert!(
            ir.contains(expected),
            "expected a call to {expected}:\n{ir}"
        );
    }
    assert!(
        !ir.contains("call i32 @tyrus_array_slice(")
            && !ir.contains("call i32 @tyrus_array_concat(")
            && !ir.contains("call i32 @tyrus_array_with("),
        "a string[] receiver must never take the raw byte-copy path:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn values_registers_its_result_as_owned_like_every_other_allocating_array_method() {
    // `values()` allocates a fresh array through the exact same codegen as
    // `slice`/`toReversed`, but was missing from `array_method_allocates`, so
    // its result was never registered for cleanup at all, a leak for any
    // element type, and (after the fix above) unreachable code for owned
    // strings specifically, since nothing ever released the cloned elements
    // either.
    let source = r#"
        function main() {
            const nums: i32[] = [1, 2, 3];
            console.log(nums.values());
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-values-owned-{}", std::process::id()));
    compile(&program, "values_owned", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("call void @free("),
        "expected the result of .values() to be released:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn array_map_releases_an_owned_callback_result_after_cloning_it() {
    let source = r#"
        function main() {
            const values: string[] = ["alpha", "beta"];
            const mapped: string[] = values.map((value) => value.toUpperCase());
            console.log(mapped[0]);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-array-map-owned-callback-result-{}",
        std::process::id()
    ));
    compile(
        &program,
        "array_map_owned_callback_result",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    let callback = ir
        .find("callback_call")
        .expect("expected the map callback call");
    let push = ir[callback..]
        .find("tyrus_array_push_owned_string")
        .map(|offset| callback + offset)
        .expect("expected the mapped string to be cloned into the output array");
    let release = ir[push..]
        .find("tyrus_string_release_buffer")
        .map(|offset| push + offset)
        .expect("expected the callback's owned result to be released after the clone");
    assert!(release > push);
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn persists_copy_array_rest_and_destructured_async_parameters() {
    let source = r#"
        interface Pair { left: i32; right: i32; }

        async function fromArray([left, right]: i32[]): Promise<i32> {
            await Promise.resolve();
            return left + right;
        }

        async function fromObject({ left, right }: Pair): Promise<i32> {
            await Promise.resolve();
            return left + right;
        }

        async function fromRest(...values: i32[]): Promise<i32> {
            await Promise.resolve();
            return values[0] + values[1];
        }

        async function main(): Promise<void> {
            const arrayValue: i32 = await fromArray([20, 22]);
            const objectValue: i32 = await fromObject({ left: 20, right: 22 });
            const restValue: i32 = await fromRest(20, 22);
            console.log(arrayValue + objectValue + restValue);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-pattern-param-{}", std::process::id()));
    compile(
        &program,
        "async_pattern_param",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.param.array.clone"));
    assert!(ir.contains("async.param.array.data"));
    assert!(ir.contains("async.drop.__tyrus_async_param_0.data"));
    assert!(ir.contains("__tyrus_async_poll_fromObject"));
    assert!(ir.contains("__tyrus_async_poll_fromRest"));
    // A rest parameter of Copy elements must clone into the frame like every
    // other array parameter. Adopting the caller's buffer by pointer gave it
    // two owners, the caller releases the synthesized rest array right after
    // the call, and the frame's drop glue released it again, and, because an
    // async frame outlives the call, reading `values[i]` afterwards was a
    // use-after-free.
    let rest_poll = ir
        .split("define")
        .find(|function| function.contains("__tyrus_async_poll_fromRest"))
        .expect("missing `fromRest` poll function in generated IR");
    assert!(
        rest_poll.contains("tyrus_array_slice"),
        "rest parameter adopts the caller's buffer instead of cloning it"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn deeply_clones_and_releases_string_array_async_parameters() {
    let source = r#"
        async function fromArray(values: string[]): Promise<usize> {
            await Promise.resolve();
            return values[0].length + values[1].length;
        }

        async function fromPattern([left, right]: string[]): Promise<usize> {
            await Promise.resolve();
            return left.length + right.length;
        }

        async function fromRest(...values: string[]): Promise<usize> {
            await Promise.resolve();
            return values[0].length + values[1].length;
        }

        async function main(): Promise<void> {
            const direct: usize = await fromArray(["twenty", "two"]);
            const destructured: usize = await fromPattern(["twenty", "two"]);
            const rest: usize = await fromRest("twenty", "two");
            console.log(direct + destructured + rest);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-string-array-{}", std::process::id()));
    compile(
        &program,
        "async_string_array",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("@tyrus_array_clone_strings"));
    assert!(ir.contains("async.param.string.array.clone"));
    assert!(ir.contains("@tyrus_array_drop_string_contents"));
    assert!(ir.contains("async.return.source.array.data"));
    assert!(ir.contains("__tyrus_async_poll_fromPattern"));
    assert!(ir.contains("__tyrus_async_poll_fromRest"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn emits_owned_copy_array_payload_and_transfers_it_through_return_await() {
    let source = r#"
        async function values(): Promise<i32[]> {
            return [1, 2, 3];
        }

        async function delegated(): Promise<i32[]> {
            return await values();
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-async-array-{}", std::process::id()));
    compile(&program, "async_array", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("tyrus_drop_async_copy_array"));
    assert!(ir.contains("tyrus_promise_take_value"));
    assert!(ir.contains("async.await.payload"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn clones_and_drops_owned_string_array_async_payloads() {
    let source = r#"
        async function labels(): Promise<string[]> {
            return ["ready", "done"];
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-string-array-payload-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_string_array_payload",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.payload.string.array.clone"));
    assert!(ir.contains("tyrus_drop_async_string_array"));
    // The payload is cloned into the promise before the returned
    // temporary is released, so a value built inside the `return` is never
    // read after it was freed.
    assert!(ir.contains("owned.async.return"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn clones_and_drops_arrays_of_owned_objects_in_async_payloads_and_parameters() {
    let source = r#"
        interface User {
            name: string;
        }

        async function users(values: User[]): Promise<User[]> {
            return [{ name: "Ada" }, { name: "Grace" }];
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-owned-object-array-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_owned_object_array",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("tyrus_array_clone_owned"));
    assert!(ir.contains("tyrus_array_drop_owned_contents"));
    assert!(ir.contains("__tyrus_clone_async_element_"));
    assert!(ir.contains("__tyrus_drop_async_element_"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn stores_and_releases_const_array_literals_across_awaits() {
    let source = r#"
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            const numbers: i32[] = [20, 22];
            const labels: string[] = ["twenty", "two"];
            await load();
            console.log(numbers[0] + numbers[1]);
            console.log(labels[0]);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-array-locals-{}", std::process::id()));
    compile(
        &program,
        "async_array_locals",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.frame.local.numbers"));
    assert!(ir.contains("async.frame.local.labels"));
    assert!(ir.contains("async.drop.numbers.data"));
    assert!(ir.contains("async.drop.labels.data"));
    assert!(ir.contains("release.local.array"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn deeply_releases_owned_string_array_locals_across_awaits() {
    let source = r#"
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            const labels: string[] = "twenty,two".split(",");
            await load();
            console.log(labels[0]);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-owned-string-array-local-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_owned_string_array_local",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.frame.local.labels"));
    assert!(ir.contains("@tyrus_array_drop_string_contents"));
    assert!(ir.contains("async.drop.labels.slot"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn releases_fresh_copy_array_method_results_across_awaits() {
    let source = r#"
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            const source: i32[] = [20, 22];
            const reversed: i32[] = source.toReversed();
            await load();
            console.log(reversed[0] + reversed[1]);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-copy-array-method-local-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_copy_array_method_local",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.frame.local.reversed"));
    assert!(ir.contains("async.drop.reversed.data"));
    assert!(ir.contains("release.local.array"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn replaces_and_releases_mutable_copy_array_locals_across_awaits() {
    let source = r#"
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            let values: i32[] = [20, 22];
            await load();
            values = values.toReversed();
            await load();
            console.log(values[0]);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-mutable-copy-array-local-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_mutable_copy_array_local",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.frame.local.values"));
    assert!(ir.contains("release.reassigned.async.array"));
    assert!(ir.contains("async.local.values.old.data"));
    assert!(ir.contains("async.drop.values.data"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn tracks_shallow_and_deep_mutable_string_array_cleanup_across_awaits() {
    let source = r#"
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            let labels: string[] = ["static", "array"];
            await load();
            labels[0] = "cloned";
            labels.push("pushed".toUpperCase());
            labels.unshift(labels[0]);
            labels.reverse();
            labels.fill("filled".toUpperCase(), 0, 1);
            labels.copyWithin(1, 0, 1);
            labels.sort();
            const removed: string[] = labels.splice(0, 1, "replacement".toUpperCase());
            await load();
            console.log(removed[0]);
            const removedLast: string? = labels.pop();
            const removedFirst: string? = labels.shift();
            await load();
            console.log(removedLast ?? "missing");
            console.log(removedFirst ?? "missing");
            labels = "owned,array".split(",");
            await load();
            labels[1] = labels[0];
            labels = ["final", "static"];
            await load();
            console.log(labels[0]);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-mutable-string-array-local-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_mutable_string_array_local",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.frame.local.labels.array.ownership"));
    assert!(ir.contains("normalize.async.string.array.for.mutation"));
    assert!(ir.contains("async.string.array.mutation.clone.status"));
    assert!(ir.contains("async.string.array.element.clone"));
    assert!(ir.contains("async.string.array.element.previous"));
    assert!(ir.contains("@tyrus_array_push_owned_string"));
    assert!(ir.contains("@tyrus_array_unshift_owned_string"));
    assert!(ir.contains("@tyrus_array_pop"));
    assert!(ir.contains("@tyrus_array_shift"));
    assert!(ir.contains("@tyrus_array_reverse"));
    assert!(ir.contains("@tyrus_array_fill_owned_string"));
    assert!(ir.contains("@tyrus_array_copy_within_owned_strings"));
    assert!(ir.contains("stable_sort.take.right"));
    assert!(ir.contains("@tyrus_array_splice_owned_strings"));
    assert!(ir.contains("release.reassigned.async.string.array.deep"));
    assert!(ir.contains("release.reassigned.async.string.array.shallow"));
    assert!(ir.contains("release.tracked.local.string.array.deep"));
    assert!(ir.contains("release.tracked.local.string.array.shallow"));
    assert!(ir.contains("@tyrus_array_drop_string_contents"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn stores_static_object_locals_and_releases_nested_array_buffers_across_awaits() {
    let source = r#"
        interface Snapshot {
            count: i32;
            label: string;
            tags: string[];
        }
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            const snapshot: Snapshot = {
                count: 42,
                label: "ready",
                tags: ["async", "frame"],
            };
            await load();
            console.log(snapshot.count);
            console.log(snapshot.label);
            console.log(snapshot.tags[0]);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-static-object-local-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_static_object_local",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.frame.local.snapshot"));
    assert!(ir.contains("async.drop.snapshot.tags.slot"));
    assert!(ir.contains("async.drop.snapshot.tags.data"));
    assert!(ir.contains("release.static.object.string.array"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn emits_narrow_integer_conversions_parsing_and_arrays() {
    let source = r#"
        const main = (): void => {
            const wide: i32 = 200;
            const byte: u8 = wide as u8;
            const maybe: u8? = 300 as u8?;
            const parsed: u8? = parseInt("42");
            const text: string = byte.toString();
            const bytes: u8[] = [1, 2, 255];
            console.log(byte, maybe, parsed, text, bytes);
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-narrow-conv-{}", std::process::id()));
    compile(&program, "narrow_conv", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("cast.range.error"));
    assert!(ir.contains("tyrus_parse_u8"));
    assert!(ir.contains("i8"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn expands_array_spread_with_runtime_length_and_memcpy() {
    let source = r#"
        const main = (): void => {
            const a: i32[] = [1, 2, 3];
            const b: i32[] = [...a, 4, 5];
            console.log(b.length);
            console.log(b[0]);
            console.log(b[3]);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-array-spread-{}", std::process::id()));
    compile(&program, "array_spread", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("llvm.memcpy"), "spread should emit a memcpy");
    assert!(
        ir.contains("array_length"),
        "spread length is computed at run time"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn emits_conversions_array_methods_and_process_exit() {
    let source = r#"
        const main = (): void => {
            const n: i32 = 42;
            console.log(n.toString());
            console.log(String(true));
            const i: i64? = parseInt("42");
            const f: f64? = parseFloat("3.5");
            const num: f64? = Number("2.5");
            console.log(i ?? -1);
            console.log(f ?? 0.0);
            console.log(num ?? 0.0);
            const a: i32[] = Array.of(1, 2, 3);
            console.log(a.at(-1) ?? 0);
            const ks: usize[] = a.keys();
            console.log(ks.length);
            const es: [usize, i32][] = a.entries();
            console.log(es.length);
            process.exit(0);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-tier1-{}", std::process::id()));
    compile(&program, "tier1", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    for symbol in [
        "tyrus_format_i128",
        "tyrus_parse_i64",
        "tyrus_parse_f64_leading",
        "tyrus_parse_f64_whole",
        "tyrus_array_at",
        "tyrus_array_keys",
        "@exit",
    ] {
        assert!(ir.contains(symbol), "expected `{symbol}` in IR");
    }
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn emits_native_typescript_array_methods() {
    let source = r#"
        interface Score {
            score: i32;
            id: i32;
        }
        const main = (): void => {
            let values: i32[] = [1, 2, 3];
            const first: i32? = values.first();
            const last: i32? = values.last();
            const total: i32 = values.sum();
            const multiplied: i32 = values.product();
            const average: f64? = values.avg();
            let scores: Score[] = [
                { score: 30, id: 3 },
                { score: 10, id: 1 },
                { score: 20, id: 2 },
            ];
            scores.sortBy("score", "desc");
            const byId: Score[] = scores.sortedBy("id");
            let rotated: i32[] = [1, 2, 3, 4];
            rotated.rotate(1);
            rotated.rotate(-1);
            console.log(first ?? -1);
            console.log(last ?? -1);
            console.log(total);
            console.log(multiplied);
            console.log(Array.isArray(values));
            console.log(Array.isArray("values"));
            values.push(4);
            values.unshift(0);
            const removed: i32[] = values.splice(1, 2, 8, 9, 10);
            console.log(removed.length);
            console.log(values.pop() ?? -1);
            console.log(values.shift() ?? -1);
            console.log(values.includes(2));
            const part: i32[] = values.slice(1, 3);
            const combined: i32[] = values.concat(part);
            values.reverse();
            values.fill(7, 0, 1);
            values.copyWithin(1, 2, 3);
            const reversed: i32[] = values.toReversed();
            const replaced: i32[] = values.with(0, 9);
            const random: i32? = values.random();
            const shuffled: i32[] = values.shuffle();
            const sample: i32[] = values.sample(2, { repeat: true });
            const words: string[] = ["tyrus", "arrays"];
            console.log(words.join(" "));
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-array-methods-{}", std::process::id()));
    compile(
        &program,
        "array_methods_test",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(
        ir.contains("@bool.true"),
        "missing static true result for Array.isArray(array)"
    );
    assert!(
        ir.contains("ptr @bool.false."),
        "missing static false result for Array.isArray(non-array)"
    );
    for symbol in [
        "tyrus_array_push",
        "tyrus_array_unshift",
        "tyrus_array_pop",
        "tyrus_array_shift",
        "tyrus_array_splice",
        "tyrus_array_index_of",
        "tyrus_array_slice",
        "tyrus_array_concat",
        "tyrus_array_reverse",
        "tyrus_array_fill",
        "tyrus_array_copy_within",
        "tyrus_array_with",
        "tyrus_array_random_index",
        "tyrus_array_shuffle",
        "tyrus_array_sample",
        "tyrus_array_join",
    ] {
        assert!(ir.contains(symbol), "missing native call `{symbol}`");
    }
    assert!(ir.contains("array_extreme.present"));
    assert!(ir.contains("array_extreme.done"));
    assert!(ir.contains("array_numeric.condition"));
    assert!(ir.contains("array_sum_next"));
    assert!(ir.contains("array_product_next"));
    assert!(ir.contains("array_average.condition"));
    assert!(ir.contains("stable_sort_key_left"));
    assert!(ir.contains("stable_sort_key_right"));
    assert!(ir.contains("array_rotate.reverse.condition"));
    assert!(ir.contains("array_rotate.done"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn emits_contextual_synchronous_functional_array_methods() {
    let source = r#"
        interface Item { group: i32; value: i32; }
        interface WorkItem { title: string; completed: boolean; category: string; }
        const main = (): void => {
            const values: i32[] = [1, 2, 3, 4];
            const partitioned: [i32[], i32[]] = values.partition(value % 2 === 0);
            const windowed: i32[][] = values.windows(2);
            const scanned: i32[] = values.scan(0, (sum, value) => sum + value);
            const products: [i32, i32][] = values.cartesianProduct([10, 20]);
            const matrix: i32[][] = [[1, 2], [3, 4]];
            const transposed: i32[][] = matrix.transpose();
            const items: Item[] = [{ group: 1, value: 10 }, { group: 2, value: 20 }, { group: 1, value: 30 }];
            const tasks: WorkItem[] = [
                { title: "Plan", completed: true, category: "work" },
                { title: "Read", completed: false, category: "learning" },
                { title: "Review", completed: true, category: "work" }
            ];
            const [completed, pending] = tasks.partition("completed");
            const [focused, deferred] = tasks.partition("category", "work", "learning");
            const uniqueByGroup: Item[] = items.uniqueBy("group");
            const grouped: Map<i32, Item[]> = items.groupBy("group");
            const uniqueByParityCallback: i32[] = values.uniqueBy(value => value % 2);
            const uniqueByParityImplicit: i32[] = values.uniqueBy(value % 2);
            const groupedParityCallback: Map<i32, i32[]> = values.groupBy(value => value % 2);
            const groupedParityImplicit: Map<i32, i32[]> = values.groupBy(value % 2);
            const mapped: i32[] = values.map(value => value * 2);
            const filtered: i32[] = mapped.filter(value => value > 2);
            const filteredImplicit: i32[] = mapped.filter(value > 2);
            const hasLarge: boolean = values.some(value > 3);
            const allPositive: boolean = values.every(value > 0);
            const found: i32? = filtered.findLast(value => value === 8);
            const foundImplicit: i32? = values.find(value === 3);
            const foundLastImplicit: i32? = values.findLast(value > 1);
            const firstLargeIndex: i64 = values.findIndex(value > 2);
            const lastLargeIndex: i64 = values.findLastIndex(value > 2);
            const total: i32 = filtered.reduce((sum, value) => sum + value, 0);
            const right: i32 = filtered.reduceRight((sum, value) => sum - value);
            let sortable: i32[] = [3, 1, 2];
            sortable.sort((left, right) => left - right);
            const sorted: i32[] = sortable.toSorted();
            const nested: i32[][] = [[1], [2, 3]];
            const flat: i32[] = nested.flat();
            const expanded: i32[] = values.flatMap(value => [value, value]);
            filtered.forEach(value => { console.log(value); });
            console.log(found ?? 0);
            console.log(total);
            console.log(right);
            console.log(sorted[0]);
            console.log(flat[0]);
            console.log(expanded[0]);
            console.log(partitioned[0][0]);
            console.log(windowed[0][0]);
            console.log(scanned[4]);
            console.log(products[0][1]);
            console.log(transposed[0][1]);
            console.log(uniqueByGroup[1].value);
            console.log(completed[0].title);
            console.log(pending[0].title);
            console.log(focused[1].title);
            console.log(deferred.length);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!(
        "tyrus-functional-array-methods-{}",
        std::process::id()
    ));
    compile(
        &program,
        "functional_array_methods",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    for marker in [
        "array.map.condition",
        "array.filter.condition",
        "array.partition.condition",
        "partition.accepted",
        "partition.rejected",
        "array.windows.condition",
        "array.scan.condition",
        "array.cartesian.outer.condition",
        "array.transpose.columns.condition",
        "array.unique_by.condition",
        "array.group_by.condition",
        "array.find.done",
        "reduce.condition",
        "stable_sort.pass.condition",
        "stable_sort.merge.condition",
        "array.flat.condition",
        "array.flatMap.condition",
        "array.forEach.condition",
    ] {
        assert!(ir.contains(marker), "missing LLVM marker `{marker}`");
    }
    assert!(
        !ir.contains("closure_environment"),
        "synchronous callbacks must not use heap-allocated closure environments"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn moves_fresh_windows_and_transpose_rows_into_their_outer_arrays() {
    let source = r#"
        const main = (): void => {
            const values: i32[] = [1, 2, 3, 4];
            const windows: i32[][] = values.windows(2);
            const matrix: i32[][] = [[1, 2], [3, 4]];
            const transposed: i32[][] = matrix.transpose();
            console.log(windows.length);
            console.log(transposed.length);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-functional-array-owned-moves-{}",
        std::process::id()
    ));
    compile(
        &program,
        "functional_array_owned_moves",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("windows_slice"));
    assert!(ir.contains("transpose_column_value"));
    assert!(
        !ir.contains("call void @__tyrus_clone_async_element_array_i32"),
        "fresh nested arrays must move into the outer array instead of being cloned:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn emits_runtime_validator_check_for_array_decorators() {
    let source = r#"
        interface BatchInput {
            @minItems(2)
            tags: string[];

            @notEmptyItems()
            ids: i32[];
        }

        const main = (): void => {
            const tags: string[] = ["rust", "typescript"];
            const ids: i32[] = [1];
            const input: BatchInput = {
                tags: tags,
                ids: ids,
            };
            console.log(input.tags.length);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-validator-array-{}", std::process::id()));
    compile(&program, "validator_array", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("validator.array_len"));
    assert!(ir.contains(
        "tyrus validation error: `@minItems(2)` failed for `tags`: array must contain at least 2 items"
    ));
    assert!(ir.contains(
        "tyrus validation error: `@notEmptyItems()` failed for `ids`: array must not be empty"
    ));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
pub(super) fn emits_runtime_validator_check_for_unique_array_decorator() {
    let source = r#"
        interface BatchInput {
            @unique()
            tags: string[];

            @unique()
            ids: i32[];
        }

        const main = (): void => {
            const tags: string[] = ["rust", "typescript"];
            const ids: i32[] = [1, 2];
            const input: BatchInput = {
                tags: tags,
                ids: ids,
            };
            console.log(input.tags.length);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-validator-unique-{}", std::process::id()));
    compile(
        &program,
        "validator_unique",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("tyrus_array_unique"));
    assert!(ir.contains("validator.unique"));
    assert!(ir.contains(
        "tyrus validation error: `@unique()` failed for `tags`: array items must be unique"
    ));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
pub(super) fn emits_runtime_validator_check_for_unique_object_array_decorator() {
    let source = r#"
        interface Point {
            x: i32;
            y: i32;
        }

        interface ShapeInput {
            @unique()
            points: Point[];
        }

        const main = (): void => {
            const first: Point = { x: 1, y: 2 };
            const second: Point = { x: 3, y: 4 };
            const points: Point[] = [first, second];
            const input: ShapeInput = { points: points };
            console.log(input.points.length);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-validator-unique-object-{}",
        std::process::id()
    ));
    compile(
        &program,
        "validator_unique_object",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("validator.unique.outer"));
    assert!(ir.contains("validator.unique_eq_field"));
    assert!(!ir.contains("call i64 @tyrus_array_unique"));
    assert!(ir.contains(
        "tyrus validation error: `@unique()` failed for `points`: array items must be unique"
    ));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
pub(super) fn emits_runtime_validator_check_for_unique_tuple_array_decorator() {
    let source = r#"
        interface GridInput {
            @unique()
            cells: [i32, i32][];
        }

        const main = (): void => {
            const first: [i32, i32] = [1, 2];
            const second: [i32, i32] = [3, 4];
            const cells: [i32, i32][] = [first, second];
            const input: GridInput = { cells: cells };
            console.log(input.cells.length);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-validator-unique-tuple-{}",
        std::process::id()
    ));
    compile(
        &program,
        "validator_unique_tuple",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("validator.unique.outer"));
    assert!(ir.contains("validator.unique_tuple_left"));
    assert!(!ir.contains("call i64 @tyrus_array_unique"));
    assert!(ir.contains(
        "tyrus validation error: `@unique()` failed for `cells`: array items must be unique"
    ));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
pub(super) fn emits_destructured_array_parameter() {
    let source = r#"
        const sumPair = ([a, b]: i32[]): i32 => a + b;
        const main = (): void => {
            console.log(sumPair([3, 7]));
        };
    "#;
    let tokens: Vec<Token> = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-destructure-arr-{}", std::process::id()));
    compile(&program, "destructure_arr", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("sumPair"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
pub(super) fn emits_for_of_loop_over_array() {
    let source = r#"
        const main = (): void => {
            const values: i32[] = [1, 2, 3];
            let total: i32 = 0;
            for (const value of values) {
                total += value;
            }
            console.log(total);
        };
    "#;
    let tokens: Vec<Token> = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-for-of-{}", std::process::id()));
    compile(&program, "for_of", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("forof.cond"));
    assert!(ir.contains("forof.body"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
pub(super) fn lowers_rest_parameters_to_a_single_array_argument() {
    let source = r#"
        function sum(...xs: i32[]): i32 {
            let total: i32 = 0;
            for (const x of xs) {
                total += x;
            }
            return total;
        }
        const values: i32[] = [1, 2, 3];
        console.log(sum(1, 2, 3));
        console.log(sum());
        console.log(sum(...values));
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-rest-{}", std::process::id()));
    compile(&program, "rest_params", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("define i32 @sum({ ptr, i64, i64 }"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
pub(super) fn releases_allocated_strings_and_arrays_at_scope_exit() {
    // A method that builds a fresh string or array hands its allocation to the
    // caller. Nothing released it, so every such call leaked, and inside a
    // loop the leak grew with the iteration count, which is memory exhaustion
    // for a long-running program rather than a tidy-up detail. Measured with
    // valgrind before the fix: 3 bytes per iteration, exactly proportional.
    let source = r#"
        const main = (): void => {
            let total: usize = 0;
            const numbers: i32[] = [1, 2, 3];
            for (let i: i32 = 0; i < 10; i += 1) {
                const shouted: string = "abc".toUpperCase();
                const doubled: i32[] = numbers.map((value: i32): i32 => value * 2);
                total += shouted.length + doubled.length;
            }
            console.log(total);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-scope-release-{}", std::process::id()));
    compile(&program, "scope_release", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    let releases = ir.matches("call void @free(").count();
    assert!(
        releases >= 2,
        "the string and the array built inside the loop must both be released, \
         found {releases} call(s) to `free`"
    );
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
pub(super) fn array_count_with_no_argument_emits_no_runtime_call_at_all() {
    let source = r#"
        function main() {
            const numbers: i32[] = [1, 2, 3];
            console.log(numbers.count());
        }
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-count-length-{}", std::process::id()));
    compile(&program, "count_length", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(
        !ir.contains("tyrus_array_count_equal") && !ir.contains("array.count"),
        "expected no runtime call and no loop for a bare `.count()`:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn array_count_with_a_value_emits_tyrus_array_count_equal() {
    let source = r#"
        function main() {
            const numbers: i32[] = [1, 2, 3];
            console.log(numbers.count(3));
        }
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-count-value-{}", std::process::id()));
    compile(&program, "count_value", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(
        ir.contains("call i64 @tyrus_array_count_equal("),
        "expected a call to tyrus_array_count_equal:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn array_count_with_an_implicit_condition_emits_a_loop_that_never_exits_early() {
    let source = r#"
        function main() {
            const numbers: i32[] = [1, 2, 3, 4, 5, 6];
            console.log(numbers.count(number % 2 === 0));
        }
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-count-condition-{}", std::process::id()));
    compile(&program, "count_condition", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(
        ir.contains("array.count.condition") || ir.contains("array.count"),
        "expected the dedicated `array.count` loop:\n{ir}"
    );
    assert!(
        !ir.contains("predicate.terminal"),
        "count must not short-circuit like some()/every() do:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn string_count_with_an_explicit_type_annotation_does_not_take_the_array_only_path() {
    let source = r#"
        function main() {
            const text: string = "banana";
            const len: usize = text.count();
            console.log(len);
        }
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-string-count-annotated-{}",
        std::process::id()
    ));
    compile(
        &program,
        "string_count_annotated",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
pub(super) fn array_join_keys_and_entries_register_their_result_as_owned() {
    let source = r#"
        function main() {
            const words: string[] = ["a", "b"];
            const joined: string = words.join(",");
            const idx: usize[] = words.keys();
            const pairs: [usize, string][] = words.entries();
            console.log(joined);
            console.log(idx.length);
            console.log(pairs.length);
        }
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-array-join-keys-entries-owned-{}",
        std::process::id()
    ));
    compile(
        &program,
        "array_join_keys_entries_owned",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    let free_calls = ir.matches("call void @free(").count()
        + ir.matches("call void @tyrus_string_release_buffer(")
            .count();
    assert!(
        free_calls >= 3,
        "expected join/keys/entries results to each be released at scope exit, found {free_calls} frees:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn array_entries_never_recurses_into_its_tuples_own_string_element() {
    // `entries()` copies each element's *value* into a fresh `[usize, T]`
    // tuple, for a non-`Copy` `T` that value is a pointer, so the copy is
    // an alias of the receiver's own element, never a clone. Registering
    // the result as `Deep` (the default every other method here uses)
    // recurses into that alias and frees it: a segfault freeing a
    // `.rodata` address for a static-string receiver (`["a", "b"]`), and a
    // double free for a receiver that already owns its own elements
    // (`["a".toUpperCase(), "b".toUpperCase()]`) once the receiver's own
    // cleanup runs, confirmed with both under valgrind before this test
    // existed. Closed with a plain `Buffer` release for `entries()`
    // specifically, regardless of the receiver's own plan, since `entries()`
    // never clones an element either way. This asserts the fix at the IR
    // level: the tuple-element drop helper must never be called for either
    // receiver shape.
    let source = r#"
        function main() {
            const words: string[] = ["a", "b", "c"];
            const pairs: [usize, string][] = words.entries();
            console.log(pairs.length);

            const computed: string[] = ["a".toUpperCase(), "b".toUpperCase()];
            const pairs2: [usize, string][] = computed.entries();
            console.log(pairs2.length);
        }
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-array-entries-no-element-recursion-{}",
        std::process::id()
    ));
    compile(
        &program,
        "array_entries_no_element_recursion",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        !ir.contains("drop_owned_contents") && !ir.contains("drop_async_element_tuple"),
        "expected entries() to never recurse into its tuples' string element:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn array_pop_on_a_static_string_literal_frees_only_the_box_not_the_element() {
    // `pop`/`shift` box a fresh `T?`. A first attempt always freed the boxed
    // element too (`ReleasePlan::Deep`), reasoning that removal transfers
    // ownership, correct for a dynamically-built array, but a pure
    // `string[]` literal never `strdup`s its elements at all, so this froze
    // a `.rodata` address: "Invalid free()" under valgrind, a genuine
    // double free rather than a leak. Closed by looking up the receiver's
    // own registered plan and only using `Deep` when it proves the elements
    // were themselves allocations. This asserts the box alone is freed here
    // (`ReleasePlan::Value`'s single unconditional `free`, not the branching
    // `Deep` release a heap string would need).
    let source = r#"
        function main() {
            const words: string[] = ["alpha", "beta", "gamma"];
            const popped: string? = words.pop();
            console.log(popped ?? "none");
        }
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-array-pop-static-literal-{}",
        std::process::id()
    ));
    compile(
        &program,
        "array_pop_static_literal",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("call void @free("),
        "expected the popped box to be freed:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn array_pop_on_a_dynamically_built_array_still_frees_the_element_deeply() {
    let source = r#"
        function main() {
            const words: string[] = ["alpha".toUpperCase(), "beta".toUpperCase()];
            const popped: string? = words.pop();
            console.log(popped ?? "none");
        }
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-array-pop-dynamic-{}", std::process::id()));
    compile(
        &program,
        "array_pop_dynamic",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    let free_calls = ir.matches("call void @free(").count()
        + ir.matches("call void @tyrus_string_release_buffer(")
            .count();
    assert!(
        free_calls >= 2,
        "expected both the box and the popped element to be freed, found {free_calls}:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn mixed_array_literal_releases_the_buffer_and_only_the_owned_elements() {
    // A `string[]` literal mixing static and freshly-allocated elements
    // (`["alpha", "beta".toUpperCase(), "gamma"]`) could previously only be
    // claimed all-or-nothing: every element owned (`Deep`) or none (`Buffer`).
    // A mix was left entirely unclaimed, both the buffer and the owned
    // element leaked, confirmed via valgrind even with zero method calls on
    // the array. Closed by `ReleasePlan::ArrayElements`, naming the buffer
    // offsets that are individually owned, mirroring what `Fields` already
    // does for a mixed object.
    let source = r#"
        function main() {
            const words: string[] = ["alpha", "beta".toUpperCase(), "gamma"];
            console.log(words.length);
        }
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-mixed-array-literal-{}", std::process::id()));
    compile(
        &program,
        "mixed_array_literal",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("call void @tyrus_array_drop_string_contents("),
        "expected the owned element to be released with the array:\n{ir}"
    );
    assert!(
        ir.contains("call void @free("),
        "expected the buffer to be freed:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn fuses_a_run_of_pushes_and_emits_no_array_growth() {
    let ir = compile_to_ir(
        r#"
            const build = (seed: i32): i32 => {
                let values: i32[] = [];
                values.push(seed);
                values.push(seed + 1);
                values.push(seed + 2);
                values.push(seed + 3);

                return ((values[0] + values[1]) + values[2]) + values[3];
            };

            const main = (): void => {
                console.log(build(1));
            };
        "#,
        "fused-array-pushes",
    );
    assert!(
        !ir.contains("tyrus_array_push"),
        "expected the pushes to be fused into the literal, got:\n{ir}"
    );
}

#[test]
pub(super) fn keeps_array_growth_when_an_element_reads_the_array() {
    let ir = compile_to_ir(
        r#"
            const build = (seed: i32): i32 => {
                let values: i32[] = [];
                values.push(seed);
                values.push(values[0] + 1);

                return values[0] + values[1];
            };

            const main = (): void => {
                console.log(build(1));
            };
        "#,
        "unfused-array-pushes",
    );
    assert!(
        ir.contains("tyrus_array_push"),
        "expected the self-reading push to survive, got:\n{ir}"
    );
}

#[test]
pub(super) fn emits_boyer_moore_study_result_with_owned_arrays() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-codegen-boyer-moore-info-source-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("main.tyr"),
        r#"
            import { BoyerMooreInfo, StringSearchInfo, boyerMooreInfo } from "tyrus:algorithm";

            function main(): void {
                const info: BoyerMooreInfo = boyerMooreInfo("ababa", "aba");
                console.log(info.matches, info.badCharacterTable, info.appliedShifts);
            }
        "#,
    )
    .unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-codegen-boyer-moore-info-{}",
        std::process::id()
    ));
    compile(
        &program,
        "boyer_moore_info",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("@tyrus_algorithm_boyer_moore_info"), "{ir}");
    assert!(ir.contains("boyer.info.output"), "{ir}");
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
pub(super) fn emits_aho_corasick_study_result_with_owned_string_arrays() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-codegen-aho-corasick-info-source-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("main.tyr"),
        r#"
            import { AhoCorasickInfo, ahoCorasickInfo } from "tyrus:algorithm";

            function main(): void {
                const info: AhoCorasickInfo = ahoCorasickInfo("ababa", ["aba", "ba"]);
                console.log(info.patterns, info.matches, info.visitedStates);
            }
        "#,
    )
    .unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-codegen-aho-corasick-info-{}",
        std::process::id()
    ));
    compile(
        &program,
        "aho_corasick_info",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("@tyrus_algorithm_aho_corasick_info"), "{ir}");
    assert!(ir.contains("aho.info.output"), "{ir}");
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
pub(super) fn emits_suffix_array_study_result_with_owned_snapshot() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-codegen-suffix-array-info-source-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("main.tyr"),
        r#"
            import { SuffixArrayInfo, suffixArrayInfo } from "tyrus:algorithm";

            function main(): void {
                const info: SuffixArrayInfo = suffixArrayInfo("banana");
                console.log(info.suffixArray, info.comparisonCount, info.comparedByteCount);
            }
        "#,
    )
    .unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-codegen-suffix-array-info-{}",
        std::process::id()
    ));
    compile(
        &program,
        "suffix_array_info",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("@tyrus_suffix_array_info"), "{ir}");
    assert!(ir.contains("suffix_array.info.output"), "{ir}");
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
pub(super) fn an_index_into_a_mutable_array_keeps_its_bounds_check() {
    let ir = emitted_ir(
        "bce_constant_index_unproved",
        r#"
        readonly const TABLE: i32[] = [10, 20, 30, 40];

        function main(): void {
            let values: i32[] = [1, 2];
            values.push(3);
            console.log(TABLE[2] + values[1]);
        }
        "#,
    );
    assert_eq!(
        ir.matches("array.bounds.valid:").count(),
        1,
        "expected exactly the mutable array to keep a check:\n{ir}"
    );
}

#[test]
pub(super) fn growing_the_array_inside_the_loop_keeps_the_bounds_check() {
    let ir = emitted_ir(
        "bce_growing_array",
        r#"
        function sweep(): i32 {
            let values: i32[] = [1, 2, 3];
            let sum = 0;
            for (let i: usize = 0; i < values.length; i += 1) {
                values.push(1);
                sum += values[i];
            }
            return sum;
        }

        function main(): void {
            console.log(sweep());
        }
        "#,
    );
    assert!(
        ir.contains("array.bounds.error"),
        "a loop that grows its own array must keep the check:\n{ir}"
    );
}

#[test]
pub(super) fn a_string_array_held_by_an_object_is_released_deeply() {
    // An array literal records its release plan at construction: `Deep` when
    // its elements are allocations, `Buffer` when they are `.rodata` pointers.
    // `[]` has no elements, so it recorded `Buffer`, and a later `push` of a
    // computed string made that answer wrong, leaving the element to leak. A
    // plain local escapes this because mutating one promotes its own runtime
    // ownership mode; an array inside an object has no such slot, so the plan
    // is the only answer it gets.
    //
    // A `string[]` now records `Deep` regardless, which is safe for static
    // elements because `TyrusString::release` returns immediately for a value
    // that does not own its buffer. The deep release is what this asserts;
    // `tests/leaks/clean/push-computed-string-into-an-object-field-array.tyr`
    // asserts that it actually reaches zero bytes.
    let ir = compile_to_ir(
        r#"
        interface Bucket {
            labels: string[];
        }

        const main = (): void => {
            let bucket: Bucket = { labels: [] };
            bucket.labels.push("key".toUpperCase());
            console.log(bucket.labels[0]);
        };
        "#,
        "object-field-string-array-deep-release",
    );
    assert!(
        ir.contains("@tyrus_array_drop_string_contents"),
        "expected the object's string array to be released deeply:\n{ir}"
    );
}

#[test]
pub(super) fn emits_fixed_array_initializers_with_both_literal_forms() {
    let ir = compile_to_ir(
        r#"
        function main(): void {
            const repeated: [i32; 5] = [0; 5];
            const values: [i32; 5] = [1, 2, 3, 4, 5];
            console.log(repeated);
            console.log(values);
        }
        "#,
        "fixed_array_initializers",
    );
    assert!(
        ir.contains("i32 0"),
        "expected repeated zero elements:\n{ir}"
    );
    for value in 1..=5 {
        assert!(
            ir.contains(&format!("i32 {value}")),
            "expected value {value}:\n{ir}"
        );
    }
}

#[test]
pub(super) fn emits_contained_array_repeat_in_stack_storage() {
    let ir = compile_to_ir(
        r#"
        function main(): void {
            const xs: i32[] = [0; 5];
            console.log(xs.length);
        }
        "#,
        "array_repeat_loop",
    );
    assert!(
        ir.contains("array_repeat_data_stack"),
        "contained array repeat should use stack storage:\n{ir}"
    );
    assert!(
        !ir.contains("array_allocation_size"),
        "contained array repeat must not emit a heap allocation:\n{ir}"
    );
    assert!(
        ir.contains("store i32 0"),
        "array repeat should store each element:\n{ir}"
    );
    assert!(
        ir.contains("i64 5"),
        "array repeat should set length and capacity to 5:\n{ir}"
    );
}

#[test]
pub(super) fn emits_array_repeat_with_constant_count_expression() {
    let ir = compile_to_ir(
        r#"
        function main(): void {
            const xs: i32[] = [0; 1 + 2];
            console.log(xs.length);
        }
        "#,
        "array_repeat_constant_count",
    );
    assert!(
        ir.contains("i64 3"),
        "array repeat should evaluate its constant count before codegen:\n{ir}"
    );
}

#[test]
pub(super) fn emits_array_repeat_string_value_with_clone() {
    let ir = compile_to_ir(
        r#"
        function main(): void {
            const xs: string[] = ["a"; 3];
            console.log(xs.length);
        }
        "#,
        "array_repeat_string",
    );
    assert!(
        ir.contains("getelementptr") && ir.contains("store { ptr, i64 }"),
        "array repeat of string literals should store the same literal pointer:\n{ir}"
    );
}

#[test]
pub(super) fn emits_array_repeat_nested_with_deep_clone() {
    let ir = compile_to_ir(
        r#"
        function main(): void {
            const inner: i32[] = [0; 3];
            const matrix: i32[][] = [inner; 2];
            console.log(matrix.length);
        }
        "#,
        "array_repeat_nested",
    );
    assert!(
        ir.contains("tyrus_array_clone_owned"),
        "array repeat of arrays should use deep clone:\n{ir}"
    );
}

#[test]
pub(super) fn emits_toggle_on_string_array() {
    let ir = compile_to_ir(
        r#"
            function main(): void {
                let names: string[] = ["alice", "bob"];
                const result: string[] = names.toggle("charlie");
                console.log(result.length);
            }
        "#,
        "toggle_string",
    );
    assert!(
        ir.contains("tyrus_array_toggle"),
        "toggle on string array must call tyrus_array_toggle:\n{ir}"
    );
}

#[test]
pub(super) fn emits_toggle_on_object_array() {
    let ir = compile_to_ir(
        r#"
            interface Item {
                id: i32;
                name: string;
            }
            function main(): void {
                let items: Item[] = [
                    { id: 1, name: "a" },
                    { id: 2, name: "b" },
                ];
                const result: Item[] = items.toggle({ id: 3, name: "c" });
                console.log(result.length);
            }
        "#,
        "toggle_object",
    );
    assert!(
        ir.contains("tyrus_array_toggle"),
        "toggle on object array must call tyrus_array_toggle:\n{ir}"
    );
    assert!(
        ir.contains("tyrus_array_toggle(ptr"),
        "object toggle must be emitted with the 7-argument signature:\n{ir}"
    );
}

#[test]
pub(super) fn emits_an_empty_argument_array_for_an_absent_process_argument_list() {
    let ir = compile_module_to_ir(
        r#"
        import { ChildProcess, exec, ProcessError } from "tyrus:process";

        async function run(args?: string[]): Promise<Result<Result<i32, ProcessError>, ProcessError>> {
            const child: ChildProcess = exec("tool", args)?;
            return Ok(await child.wait());
        }

        function main(): void {}
        "#,
        "process_exec_optional_args",
    );
    for block in [
        "process.args.absent",
        "process.args.present",
        "process.args.merge",
    ] {
        assert!(
            ir.contains(block),
            "missing `{block}` optional-argument branch:\n{ir}"
        );
    }
    assert!(
        ir.contains("tyrus_process_exec_result"),
        "missing process ABI dispatch:\n{ir}"
    );
}

#[test]
pub(super) fn releases_process_exec_argument_array_after_the_native_call() {
    let ir = compile_module_to_ir(
        r#"
        import { ChildProcess, exec } from "tyrus:process";

        async function main(): Promise<void> {
            const child: ChildProcess = exec("tool", ["--version"]) onfail {
                return;
            };
            await child.wait();
        }
        "#,
        "process_exec_argument_cleanup",
    );
    let dispatch = ir
        .find("call void @tyrus_process_exec_result")
        .expect("process exec must dispatch to its native ABI");
    let argument_cleanup = ir
        .find("call void @tyrus_array_drop_string_contents(ptr %native.cleanup.owned)")
        .unwrap_or_else(|| panic!("the temporary argument array must be released:\n{ir}"));
    assert!(
        dispatch < argument_cleanup,
        "the argument array must remain alive through the native call, then be released:\n{ir}"
    );
}
