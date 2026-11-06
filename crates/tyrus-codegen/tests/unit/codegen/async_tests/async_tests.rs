use super::*;

#[test]
pub(super) fn materializes_structured_net_error_bindings_in_async_catches() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-codegen-net-error-source-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("main.tyr"),
        r#"
            import { NetError, tcpConnect } from "tyrus:net";
            async function connectOrReport(): Promise<void> {
                match (await tcpConnect("127.0.0.1:1", 1)) {
                    Ok(stream) => { stream.close(); }
                    Err(error) => { console.log(error.code, error.message); }
                }
            }
        "#,
    )
    .unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-codegen-net-error-{}", std::process::id()));
    compile(&program, "net_error", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("tyrus_net_tcp_connect"), "{ir}");
    assert!(ir.contains("%code = extractvalue"), "{ir}");
    assert!(ir.contains("%message = extractvalue"), "{ir}");
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
pub(super) fn releases_owned_array_success_payload_after_awaited_result_propagation() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-async-result-array-source-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("main.tyr"),
        r#"
            import { ArchiveEntry, ArchiveError, list } from "tyrus:archive";
            import { sleepAsync } from "tyrus:task";

            async function inspect(path: string): Promise<Result<void, ArchiveError>> {
                const entries: ArchiveEntry[] = await list(path)?;
                await sleepAsync(1);
                print(entries.length);
                return Ok();
            }
        "#,
    )
    .unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-result-array-{}", std::process::id()));
    compile(
        &program,
        "async_result_array",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    let success = ir
        .split("result.success:")
        .nth(1)
        .and_then(|block| block.split("result.error:").next())
        .unwrap_or_default();
    assert!(
        success.contains(
            "store { ptr, i64, i64 } %result.success.value, ptr %async.frame.local.entries"
        ),
        "an owned array extracted from an awaited Result must move into its async local:\n{success}"
    );
    assert!(
        !success.contains("tyrus_array_clone_owned"),
        "moving an awaited Result payload must not clone and prematurely release it:\n{success}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
pub(super) fn automatically_drops_digest_streams_stored_across_async_suspension() {
    let source = r#"
        import { CryptoError, argon2, digest } from "tyrus:crypto";

        async function hashWithStream(password: string): Promise<Result<u8[], CryptoError>> {
            const stream: DigestStream = digest.create("sha256")?;
            stream.update("prefix")?;
            await argon2.hash(password)?;
            stream.update("suffix")?;
            return Ok(stream.digest()?);
        }
    "#;
    let root =
        std::env::temp_dir().join(format!("tyrus-async-digest-source-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.tyr"), source).unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-async-digest-{}", std::process::id()));
    compile(&program, "async_digest", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("__tyrus_async_drop___tyrus_module_0_hashWithStream"));
    assert!(ir.contains("async.payload.drop.digest.stream"));
    assert!(ir.contains("call void @tyrus_crypto_digest_stream_free"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
pub(super) fn emits_parameterless_named_async_void_function() {
    let source = r#"
        async function load(): Promise<void> {
            console.log("loaded");
            return;
        }

        async function main(): Promise<void> {
            await load();
            console.log("done");
            return;
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-async-function-{}", std::process::id()));
    compile(&program, "async_function", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("__tyrus_async_poll_load"));
    assert!(ir.contains("define ptr @load()"));
    assert!(ir.contains("tyrus_async_task_await"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn stores_copy_parameters_in_named_async_frames() {
    let source = r#"
        async function load(value: i32): Promise<void> {
            console.log(value);
        }

        async function main(): Promise<void> {
            await load(41);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-async-param-{}", std::process::id()));
    compile(&program, "async_param", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("define ptr @load(i32"));
    assert!(ir.contains("async.frame.param.value"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn retains_promise_parameters_and_releases_async_frame_handles() {
    let source = r#"
        async function forward(value: Promise<i32>): Promise<i32> {
            return await value;
        }

        async function main(): Promise<void> {
            const source: Promise<i32> = Promise.resolve(41);
            const value: i32 = await forward(source);
            console.log(value);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-promise-param-{}", std::process::id()));
    compile(
        &program,
        "async_promise_param",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.frame.param.retain"));
    assert!(ir.contains("async.await.retain"));
    assert!(ir.contains("__tyrus_async_drop_forward"));
    assert!(ir.contains("async.drop.dependency.present"));
    assert!(ir.contains("async.drop.value.present"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn adopts_direct_promise_returns_without_an_extra_async_frame_suspension() {
    let source = r#"
        async function source(): Promise<i32> {
            return 42;
        }

        async function fromParameter(value: Promise<i32>): Promise<i32> {
            return value;
        }

        async function fromCall(): Promise<i32> {
            return source();
        }

        async function inferred() {
            return source();
        }

        async function main(): Promise<void> {
            const first: i32 = await fromParameter(source());
            const second: i32 = await fromCall();
            const third: i32 = await inferred();
            console.log(first + second + third);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-direct-adopt-{}", std::process::id()));
    compile(
        &program,
        "async_direct_adopt",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("tyrus_promise_adopt"));
    assert!(ir.contains("async.return.adopt"));
    assert!(ir.contains("call void @tyrus_promise_free(ptr %call)"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn supports_defaulted_and_validated_copy_async_parameters() {
    let source = r#"
        async function load(@min(1) value: i32 = 41): Promise<i32> {
            return value;
        }

        async function main(): Promise<void> {
            const value: i32 = await load();
            console.log(value);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-default-param-{}", std::process::id()));
    compile(
        &program,
        "async_default_param",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.frame.param.value"));
    assert!(ir.contains("@min(1)"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn clones_and_releases_optional_copy_async_parameters() {
    let source = r#"
        async function valueOrDefault(value?: i32): Promise<i32> {
            return value ?? 41;
        }

        async function main(): Promise<void> {
            const missing: i32 = await valueOrDefault();
            const present: i32 = await valueOrDefault(42);
            console.log(missing + present);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-optional-param-{}", std::process::id()));
    compile(
        &program,
        "async_optional_param",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.optional.clone"));
    assert!(ir.contains("async.drop.value.present"));
    assert!(ir.contains("release.optional"));
    assert!(ir.contains("async.argument.optional.release"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn clones_and_releases_string_fields_in_async_object_parameters() {
    let source = r#"
        interface User { name: string; aliases: string[]; age: usize; }

        async function fromObject(value: User): Promise<usize> {
            await Promise.resolve();
            return value.name.length + value.aliases[0].length + value.age;
        }

        async function fromPattern({ name, aliases, age }: User): Promise<usize> {
            await Promise.resolve();
            return name.length + aliases[0].length + age;
        }

        async function main(): Promise<void> {
            const direct: usize = await fromObject({ name: "Ada", aliases: ["A"], age: 38 });
            const destructured: usize = await fromPattern({ name: "Ada", aliases: ["A"], age: 38 });
            console.log(direct + destructured);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-object-param-{}", std::process::id()));
    compile(
        &program,
        "async_object_param",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.payload.name.value"));
    assert!(ir.contains("async.payload.aliases.value"));
    assert!(ir.contains("async.payload.string.array.clone"));
    assert!(ir.contains("async.payload.drop.name.slot"));
    assert!(ir.contains("async.payload.drop.aliases.slot"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn deeply_clones_and_releases_nested_async_object_parameters() {
    let source = r#"
        interface Profile { name: string; aliases: string[]; }
        interface User { id: i32; profile: Profile; }

        async function userName(value: User): Promise<string> {
            await Promise.resolve();
            return value.profile.name;
        }

        async function main(): Promise<void> {
            await userName({
                id: 1,
                profile: { name: "Ada", aliases: ["A"] },
            });
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-nested-object-param-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_nested_object_param",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.payload.profile.value"));
    assert!(ir.contains("async.payload.name.value"));
    assert!(ir.contains("async.payload.aliases.value"));
    assert!(ir.contains("async.payload.drop.profile.slot"));
    assert!(ir.contains("async.payload.drop.name.slot"));
    assert!(ir.contains("async.payload.drop.aliases.slot"));
    assert!(ir.contains("async.return.source.array.data"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn emits_copy_payload_for_named_async_function() {
    let source = r#"
        async function load(): Promise<i32> {
            return 41;
        }

        async function main(): Promise<void> {
            await load();
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-async-payload-{}", std::process::id()));
    compile(&program, "async_payload", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.payload"));
    assert!(ir.contains("async.fulfill.value"));
    assert!(ir.contains("call i32 @tyrus_promise_fulfill"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn emits_named_async_await_and_return_await_payload() {
    let source = r#"
        async function source(): Promise<i32> {
            return 41;
        }

        async function delegated(): Promise<i32> {
            return await source();
        }

        async function main(): Promise<void> {
            await delegated();
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-return-await-{}", std::process::id()));
    compile(&program, "return_await", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("__tyrus_async_poll_delegated"));
    assert!(ir.contains("async.await.payload"));
    assert!(ir.contains("async.fulfill.value"));
    assert!(ir.contains("tyrus_promise_propagate_failure"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn stores_awaited_payload_in_an_async_binding() {
    let source = r#"
        async function source(): Promise<i32> {
            return 41;
        }

        async function main(): Promise<void> {
            const value: i32 = await source();
            console.log(value);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-await-binding-{}", std::process::id()));
    compile(&program, "await_binding", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.await.binding.payload"));
    assert!(ir.contains("async.frame.local.value"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn transfers_owned_awaited_bindings_and_assignments_into_the_async_frame() {
    let source = r#"
        async function source(value: string): Promise<string> {
            return value;
        }

        async function main(): Promise<void> {
            let value: string = await source("first");
            value = await source("second");
            console.log(value);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-owned-await-binding-assignment-{}",
        std::process::id()
    ));
    compile(
        &program,
        "owned_await_binding_assignment",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.await.binding.payload"));
    assert!(ir.contains("async.payload.drop.string"));
    assert!(
        ir.contains("async.payload.drop.string") && ir.contains("@tyrus_string_release_buffer"),
        "an async payload's string must be released through its ownership bit"
    );
    assert!(
        ir.contains("async.payload.drop.string") && ir.contains("@tyrus_string_release_buffer"),
        "an awaited binding's string must be released through its ownership bit"
    );
    assert!(
        ir.contains("@tyrus_string_release_buffer"),
        "a captured string must be released through its ownership bit"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn emits_async_try_catch_finally_for_rejected_await() {
    let source = r#"
        async function main(): Promise<void> {
            try {
                const failure: Error = { message: "boom", code: "ERR_BOOM" };
                console.log("before");
                await Promise.reject(failure);
                console.log("unreachable");
            } catch (error: Error) {
                console.log(error.message);
            } finally {
                console.log("cleanup");
            }
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-async-try-{}", std::process::id()));
    compile(&program, "async_try", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("promise.reject"));
    assert!(ir.contains("async.error.message.clone"));
    assert!(ir.contains("async.try.recovered"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn lowers_multiple_suspensions_inside_one_async_try() {
    let source = r#"
        async function pause(): Promise<void> {}

        async function main(): Promise<void> {
            try {
                await pause();
                console.log("between");
                await pause();
                console.log("after");
            } catch (error: Error) {
                console.error(error.message);
            } finally {
                console.log("cleanup");
            }
            console.log("joined");
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-multiple-try-awaits-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_multiple_try_awaits",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.state.2"));
    assert!(ir.contains("async.try.recovered"));
    assert!(ir.matches("async.await.state").count() >= 2);
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn emits_a_noncapturing_async_closure() {
    let source = r#"
        function increment(value: i32): i32 {
            return value + 1;
        }

        function setup(): void {
            const task = async (value: i32): Promise<i32> => {
                return increment(value);
            };
        }

        async function main(): Promise<void> {
            setup();
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-async-closure-{}", std::process::id()));
    compile(&program, "async_closure", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("__tyrus_async_arrow_"));
    assert!(ir.contains("__tyrus_async_poll___tyrus_async_arrow_"));
    assert!(ir.contains("__tyrus_async_arrow_0_closure"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn emits_copy_capture_for_async_closure() {
    let source = r#"
            function setup(): void {
                const captured: i32 = 41;
                const task = async (): Promise<i32> => {
                    return captured + 1;
                };
            }
        "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-async-capture-{}", std::process::id()));
    compile(&program, "async_capture", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.closure.environment"));
    assert!(ir.contains("async.closure.capture.value"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn clones_and_drops_owned_capture_for_async_closure() {
    let source = r#"
        function setup(): void {
            const captured: string = "ready";
            const task = async (): Promise<string> => {
                return captured;
            };
            task();
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-owned-capture-{}", std::process::id()));
    compile(
        &program,
        "async_owned_capture",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.payload.string.clone"));
    assert!(ir.contains("_environment_drop"));
    assert!(ir.contains("async.closure.capture.drop"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn clones_and_drops_nested_object_async_payloads() {
    let source = r#"
        interface Metadata {
            label: string;
            aliases: string[];
        }

        interface Result {
            status: i32;
            metadata: Metadata;
        }

        async function result(): Promise<Result> {
            return {
                status: 200,
                metadata: { label: "ready", aliases: ["ok", "done"] },
            };
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-object-payload-{}", std::process::id()));
    compile(
        &program,
        "async_object_payload",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.payload.label.value"));
    assert!(ir.contains("async.payload.aliases.value"));
    assert!(ir.contains("__tyrus_drop_async_payload_"));
    assert!(ir.contains("tyrus_array_drop_string_contents"));
    // The payload is cloned into the promise before the returned
    // temporary is released, so a value built inside the `return` is never
    // read after it was freed.
    assert!(ir.contains("owned.async.return"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn clones_and_drops_owned_optional_async_payloads_and_parameters() {
    let source = r#"
        interface User {
            name: string;
        }

        async function optionalUser(value?: User): Promise<User?> {
            return value;
        }

        async function main(): Promise<void> {
            await optionalUser({ name: "ada".toUpperCase() });
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-owned-optional-{}", std::process::id()));
    compile(
        &program,
        "async_owned_optional",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.optional.clone"));
    assert!(ir.contains("async.payload.drop.optional"));
    assert!(ir.contains("async.frame.param.store"));
    assert!(ir.contains("async.argument.optional.release"));
    assert!(ir.contains("async.argument.optional.inner"));
    assert!(ir.contains("async.return.source.name"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn lowers_a_copy_await_in_an_async_condition() {
    let source = r#"
        async function enabled(): Promise<boolean> {
            return true;
        }

        async function main(): Promise<void> {
            if (await enabled()) {
                console.log("enabled");
            } else {
                console.log("disabled");
            }
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-condition-await-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_condition_await",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.if.selected"));
    assert!(ir.contains("async.if.alternative"));
    assert!(ir.contains("async.if.after"));
    assert!(!ir.contains("async.frame.await.value"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn clones_and_releases_owned_string_async_parameters() {
    let source = r#"
        async function load(value: string): Promise<void> {
            console.log(value);
        }

        async function main(): Promise<void> {
            await load("owned");
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-string-param-{}", std::process::id()));
    compile(
        &program,
        "async_string_param",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.frame.param.string.clone"));
    assert!(
        ir.contains("async.drop.value") && ir.contains("@tyrus_string_release_buffer"),
        "an async string parameter must be released through its ownership bit"
    );
    assert!(
        ir.contains("@tyrus_string_release_buffer"),
        "a captured string must be released through its ownership bit"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn emits_multiple_top_level_await_states_in_async_main() {
    let source = r#"
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            console.log("before");
            await load();
            console.log("between");
            await load();
            console.log("after");
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-async-states-{}", std::process::id()));
    compile(&program, "async_states", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.state.0"));
    assert!(ir.contains("async.state.1"));
    assert!(ir.contains("async.state.2"));
    assert!(ir.contains("async.is_state.2"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn stores_copy_locals_in_the_async_frame_across_awaits() {
    let source = r#"
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            let count: i32 = 0;
            const label: string = "completed";
            await load();
            count += 1;
            await load();
            console.log(count);
            console.log(label);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-async-locals-{}", std::process::id()));
    compile(&program, "async_locals", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.frame.local.count"));
    assert!(ir.contains("async.frame.local.label"));
    assert!(ir.contains("async.state.2"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn accepts_mutable_async_string_reassignment_from_owned_function_return() {
    let source = r#"
            function message(): string { return "static"; }
            async function load(): Promise<void> {}
            async function main(): Promise<void> {
                let value: string = "before";
                await load();
                value = message();
                console.log(value);
            }
        "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-owned-string-assignment-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_owned_string_assignment",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("owned.return"));
    assert!(ir.contains("release.reassigned.async.string"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
}

#[test]
pub(super) fn emits_async_main_for_the_tokio_runtime() {
    let source = r#"
        async function main(): Promise<void> {
            console.log("async main");
            console.log("assert:", "async main");
            return;
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-async-main-{}", std::process::id()));
    compile(&program, "async_main", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    for symbol in [
        "__tyrus_async_poll_main",
        "tyrus_promise_pending",
        "tyrus_async_task_new",
        "tyrus_async_task_schedule",
        "tyrus_async_block_on",
        "tyrus_promise_fulfill",
    ] {
        assert!(ir.contains(symbol), "missing `{symbol}` in async main IR");
    }
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn registers_an_async_inline_handler_from_inside_a_function() {
    let source = r#"
        enum HttpStatus { Ok = 200 }
        interface OkResponse {
            ok: boolean;
        }
        const main = (): void => {
            const app: HttpRouter = __tyrus_http_createRouter();
            app.get("/ok", {
                response: { 200: OkResponse },
                handler: async (request, reply) => {
                    reply.status(HttpStatus.Ok).send({ ok: true });
                }
            });
            app.listen(3000);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-http-async-inline-test-{}",
        std::process::id()
    ));
    compile(
        &program,
        "http_async_inline_test",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("@__tyrus_http_inline_handler_"));
    assert!(ir.contains("@tyrus_http_router_listen"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn emits_async_http_handlers_that_return_promises_to_the_runtime() {
    let source = r#"
        interface HealthResponse {
            status: string;
        }
        enum HttpStatus { Ok = 200 }
        async function health(request: {}, reply: HttpReply<HealthResponse>): Promise<void> {
            await __tyrus_task_sleepAsync(1);
            reply.status(HttpStatus.Ok).send({ status: "ok" });
        }
        const app: HttpRouter = __tyrus_http_createRouter();
        app.get("/health", {
            response: { 200: HealthResponse },
            handler: health
        });
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-http-async-test-{}", std::process::id()));
    compile(&program, "http_async_test", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();

    assert!(ir.contains("@__tyrus_async_poll_health"));
    assert!(ir.contains("@__tyrus_http_callback_health"));
    assert!(ir.contains("call ptr @health"));
    assert!(ir.contains("ret ptr %response"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn lowers_await_inside_an_async_closure_stored_in_a_local() {
    let source = r#"
        async function load(value: i32): Promise<i32> {
            return value * 2;
        }

        async function main(): Promise<void> {
            const factor: i32 = 20;
            const task = async (): Promise<i32> => {
                const doubled = await load(factor);
                return doubled + 1;
            };
            const value = await task();
            console.log("value:", value);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-closure-await-{}", std::process::id()));
    compile(
        &program,
        "async_closure_await",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("__tyrus_async_poll___tyrus_async_arrow_0"));
    assert!(ir.contains("async.closure.capture.value"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn async_closures_do_not_clean_up_locals_of_the_enclosing_frame() {
    let source = r#"
        async function main(): Promise<void> {
            const one = async (): Promise<i32> => {
                return 1;
            };
            const two = async (): Promise<i32> => {
                return 2;
            };
            const first = await one();
            const second = await two();
            console.log("sum:", first + second);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-two-closures-{}", std::process::id()));
    compile(
        &program,
        "async_two_closures",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    let closure_poll = ir
        .split("define i32 @__tyrus_async_poll___tyrus_async_arrow_1")
        .nth(1)
        .expect("second closure must have a poll function");
    let closure_poll = closure_poll.split("\n}").next().unwrap();
    assert!(
        !closure_poll.contains("async.frame.local.one"),
        "closure poll reached into the caller's frame:\n{closure_poll}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn lowers_multiple_awaits_inside_an_async_catch_with_owned_error_storage() {
    let source = r#"
        async function run(): Promise<void> {
            try {
                await Promise.reject<void>({ message: "boom", code: "ERR_BOOM" });
            } catch (error: Error) {
                const first: string = await Promise.resolve<string>("first");
                const second: string = await Promise.resolve<string>(first);
                console.log(error.message, second);
            } finally {
                console.log("done");
            }
        }

        async function main(): Promise<void> {
            await run();
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-catch-await-{}", std::process::id()));
    compile(
        &program,
        "async_catch_await",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.catch.0.state.0"));
    assert!(ir.contains("async.catch.0.state.1"));
    assert!(ir.contains("async.frame.local.__tyrus_async_catch_error_0"));
    assert!(ir.contains("async.catch.propagate"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn lowers_multiple_awaits_inside_async_finally_with_pending_completion_state() {
    let source = r#"
        function step(shouldFail: boolean): Promise<void> {
            return shouldFail ? Promise.reject<void>({ message: "boom", code: "ERR_BOOM" }) : Promise.resolve();
        }

        async function run(shouldFail: boolean): Promise<void> {
            try {
                await step(shouldFail);
            } catch (error: Error) {
                console.log(error.message);
            } finally {
                const first: string = await Promise.resolve<string>("first");
                const second: string = await Promise.resolve<string>(first);
                console.log(second);
            }
        }

        async function main(): Promise<void> {
            await run(false);
            await run(true);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-finally-await-{}", std::process::id()));
    compile(
        &program,
        "async_finally_await",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.finally.0.state.0"));
    assert!(ir.contains("async.finally.0.state.1"));
    assert!(ir.contains("async.frame.local.__tyrus_async_finally_mode_0"));
    assert!(ir.contains("async.frame.local.__tyrus_async_finally_failure_0"));
    assert!(ir.contains("async.finally.propagate.pending"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn lowers_async_finally_after_a_suspending_if_try_body() {
    let source = r#"
        function step(shouldFail: boolean): Promise<void> {
            return shouldFail ? Promise.reject<void>({ message: "boom", code: "ERR_BOOM" }) : Promise.resolve();
        }

        async function run(shouldFail: boolean): Promise<void> {
            try {
                if (shouldFail) {
                    await step(true);
                } else {
                    await step(false);
                }
            } catch (error: Error) {
                console.log(error.message);
            } finally {
                await Promise.resolve();
                await Promise.resolve();
                console.log("cleanup");
            }
        }

        async function main(): Promise<void> {
            await run(false);
            await run(true);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-if-finally-{}", std::process::id()));
    compile(
        &program,
        "async_if_finally",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.if.after"));
    assert!(ir.contains("async.finally.0.state.0"));
    assert!(ir.contains("async.finally.0.state.1"));
    assert!(ir.contains("async.finally.propagate.pending"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn rejects_the_promise_when_an_async_function_throws() {
    let source = r#"
        async function failing(): Promise<void> throws Error {
            const boom: Error = { message: "boom", code: "ERR_BOOM" };
            throw boom;
        }

        async function main(): Promise<void> {
            try {
                await failing();
            } catch (error: Error) {
                console.log("caught:", error.message);
            }
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-async-throw-{}", std::process::id()));
    compile(&program, "async_throw", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    let poll = ir
        .split("define i32 @__tyrus_async_poll_failing")
        .nth(1)
        .expect("the async function must have a poll function")
        .split("\n}")
        .next()
        .unwrap();
    assert!(
        poll.contains("tyrus_promise_reject_pending"),
        "the throw did not reject the Promise:\n{poll}"
    );
    assert!(
        !poll.contains("uncaught exception"),
        "the throw still aborts the process:\n{poll}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn printing_an_async_owned_handle_keeps_its_frame_drop_armed() {
    let ir = compile_to_ir(
        r#"
            import { SemaphorePermit } from "tyrus:task";

            async function show(permit: SemaphorePermit): Promise<void> {
                const local: SemaphorePermit = permit;
                print(local);
            }
        "#,
        "async-print-owned-handle",
    );
    let after_print_load = ir
        .split("%local = load")
        .nth(1)
        .expect("local permit must be loaded for printing");
    let before_fulfill = after_print_load
        .split("async.fulfill.void")
        .next()
        .unwrap_or(after_print_load);
    assert!(
        !before_fulfill.contains("store i1 false, ptr %async.frame.local.local.initialized"),
        "printing must borrow the permit without disarming its frame cleanup:\n{ir}"
    );
    assert!(
        ir.contains("call void @tyrus_semaphore_permit_release("),
        "async frame drop must release the printed permit:\n{ir}"
    );
}

#[test]
pub(super) fn an_owned_temporary_in_an_await_free_async_statement_is_released() {
    // Inside an `async` function `record_owned_native_result` used to be a
    // no-op for every statement, so a temporary consumed by a borrowing
    // operation had nobody to release it: the frame never took it, and the
    // statement-end sweep that releases exactly this synchronously was off.
    //
    // The blanket rule only ever protected a value moved into the heap frame
    // across a suspension. This statement contains no `await`, so it is emitted
    // as straight-line code in a single state-machine block and the sweep
    // dominates the concatenation it allocated.
    //
    // `tests/leaks/clean/async-owned-temporary-never-bound.tyr` asserts that
    // this and its four sibling shapes actually reach zero bytes.
    let ir = compile_to_ir(
        r#"
        export async function main(): Promise<void> {
            const base: string = "payload";
            const total: usize = (base + "!").length;
            console.log(total);
        }
        "#,
        "async-owned-temporary-never-bound",
    );
    assert!(
        ir.contains("@tyrus_string_release_buffer"),
        "expected the discarded concatenation to be released:\n{ir}"
    );
}
