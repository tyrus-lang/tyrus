use super::*;

#[test]
pub(super) fn releases_every_owned_temporal_string_result() {
    let source = r#"
        import { PlainDateTime, PlainMonthDay, PlainYearMonth, TemporalError, ZonedDateTime, plainDateTimeFrom, plainDateTimeToString, plainMonthDayFrom, plainMonthDayToString, plainYearMonthFrom, plainYearMonthToString, zonedDateTimeFormat, zonedDateTimeFrom, zonedDateTimeToString } from "tyrus:temporal";
        const main = (): Result<void, TemporalError> => {
                const dateTime: PlainDateTime = plainDateTimeFrom("2026-08-07T14:30:00")?;
                const yearMonth: PlainYearMonth = plainYearMonthFrom("2026-08")?;
                const monthDay: PlainMonthDay = plainMonthDayFrom("08-07")?;
                const zoned: ZonedDateTime = zonedDateTimeFrom("2026-08-07T14:30:00-03:00[America/Sao_Paulo]")?;
                const dateTimeText: string = plainDateTimeToString(dateTime)?;
                const yearMonthText: string = plainYearMonthToString(yearMonth)?;
                const monthDayText: string = plainMonthDayToString(monthDay)?;
                const zonedText: string = zonedDateTimeToString(zoned)?;
                const formatted: string = zonedDateTimeFormat(zoned, "yyyy-MM-dd HH:mm")?;
                console.log(dateTimeText, yearMonthText, monthDayText, zonedText, formatted);
                return Ok();
        };
    "#;
    let root = std::env::temp_dir().join(format!(
        "tyrus-codegen-owned-temporal-strings-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.tyr"), source).unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-codegen-owned-temporal-strings-output-{}",
        std::process::id()
    ));
    compile(
        &program,
        "owned_temporal_strings",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = read_ir(output.with_extension("ll")).unwrap();
    assert!(ir.contains("tyrus_temporal_plain_date_time_to_string"));
    assert!(ir.contains("tyrus_temporal_plain_year_month_to_string"));
    assert!(ir.contains("tyrus_temporal_plain_month_day_to_string"));
    assert!(ir.contains("tyrus_temporal_zoned_date_time_to_string"));
    assert!(ir.contains("tyrus_temporal_zoned_date_time_format"));
    assert!(ir.contains(if cfg!(windows) {
        "declare void @tyrus_temporal_plain_date_time_from_result(ptr, ptr)"
    } else {
        "declare void @tyrus_temporal_plain_date_time_from_result({ ptr, i64 }, ptr)"
    }));
    assert!(ir.contains(if cfg!(windows) {
        "declare void @tyrus_temporal_plain_year_month_from_result(ptr, ptr)"
    } else {
        "declare void @tyrus_temporal_plain_year_month_from_result({ ptr, i64 }, ptr)"
    }));
    assert!(ir.contains(if cfg!(windows) {
        "declare void @tyrus_temporal_plain_month_day_from_result(ptr, ptr)"
    } else {
        "declare void @tyrus_temporal_plain_month_day_from_result({ ptr, i64 }, ptr)"
    }));
    assert!(ir.contains(if cfg!(windows) {
        "declare void @tyrus_temporal_zoned_date_time_from_result(ptr, ptr)"
    } else {
        "declare void @tyrus_temporal_zoned_date_time_from_result({ ptr, i64 }, ptr)"
    }));
    assert!(
        ir.matches(string_release_call()).count() >= 5,
        "expected every owned temporal string to be released:\n{ir}"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
pub(super) fn split_at_clones_string_elements_into_both_halves() {
    let source = r#"
        function main() {
            const words: string[] = ["a", "b", "c", "d"];
            const parts = words.splitAt(2);
            console.log(parts[0]);
            console.log(parts[1]);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-split-at-strings-{}", std::process::id()));
    compile(
        &program,
        "split_at_strings_ir",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = read_ir(output.with_extension("ll")).unwrap();
    let owned_slice_calls = ir
        .matches("call i32 @tyrus_array_slice_owned_strings(")
        .count();
    assert!(
        owned_slice_calls >= 2,
        "expected both halves to clone through tyrus_array_slice_owned_strings, found {owned_slice_calls}:\n{ir}"
    );
    assert!(
        !ir.contains("call i32 @tyrus_array_slice("),
        "a string[] receiver must never take the raw byte-copy path:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn clones_the_string_element_of_a_returned_owned_tuple() {
    // A function returning `[i32, string]` is claimed by the caller as fully
    // owned, which relies on `emit_async_clone_value_into` actually cloning
    // the string element rather than copying its raw pointer. `Type::Tuple`
    // had no arm there (only the sibling drop function did, added for
    // `Map.entries()`), so it fell to the catch-all plain `store`: a tuple
    // literal's string field, a pointer to a static constant, never
    // allocated, was copied as-is, and the caller then freed it believing
    // it owned a fresh clone. `munmap_chunk(): invalid pointer` at runtime,
    // not a leak. This asserts the fix at the IR level: a `strdup` call
    // feeding the tuple's second field.
    let source = r#"
        function pair(): [i32, string] {
            return [42, "erro"];
        }

        function main() {
            const par = pair();
            console.log(par[0]);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-tuple-owned-return-clone-{}",
        std::process::id()
    ));
    compile(
        &program,
        "tuple_owned_return_clone",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = read_ir(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("@tyrus_string_share"),
        "expected the tuple's string element to be given its own reference:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn reassigning_a_mutable_string_in_a_loop_emits_a_runtime_release_check() {
    let source = r#"
        function main() {
            let s: string = "";
            for (let i: i32 = 0; i < 200; i += 1) {
                s = "x".toUpperCase();
            }
            console.log(s);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-mutable-string-reassignment-{}",
        std::process::id()
    ));
    compile(
        &program,
        "mutable_string_reassignment",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = read_ir(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("release.mutable.string"),
        "expected a runtime release-if-owned branch inside the loop body:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn string_concatenation_registers_its_own_fresh_buffer_as_owned() {
    // `a + b` built a fresh `calloc`'d buffer via `emit_string_concat` but
    // never registered it as owned anywhere, `emit_binop`'s `BinOp::Add`
    // arm returned it unclaimed, so it leaked whether bound to a name,
    // discarded as a temporary, or reassigned in a loop. Closed by claiming
    // it at `emit_binop`'s own call site, the same way an allocating string
    // method already claims itself immediately after its own low-level
    // call, this asserts the fix at the IR level: a `strdup` is not
    // involved here, but the same `record_owned_native_result`-driven
    // release-at-scope-exit path a plain owned string binding already
    // takes must fire, which the string-concat leak test's valgrind result
    // is the real proof of; this test guards the narrower, faster-to-run
    // claim that concatenation *reaches* that registration at all.
    let source = r#"
        function main() {
            const s: string = "a" + "b";
            console.log(s);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-string-concat-owned-{}", std::process::id()));
    compile(
        &program,
        "string_concat_owned",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = read_ir(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains(string_release_call()),
        "expected the concatenated buffer to be released at scope exit:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn emits_owned_string_payload_with_clone_and_deep_cleanup() {
    let source = r#"
            async function load(): Promise<string> {
                return "value";
            }
        "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-async-string-{}", std::process::id()));
    compile(&program, "async_string", output.to_str().unwrap(), false).unwrap();
    let ir = read_ir(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.payload.string.clone"));
    assert!(ir.contains("tyrus_drop_async_string"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn stores_and_releases_owned_const_string_locals_across_awaits() {
    let source = r#"
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            const message: string = "before".toUpperCase();
            await load();
            console.log(message);
            console.log("assert:", "BEFORE");
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-owned-string-local-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_owned_string_local",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = read_ir(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.frame.local.message"));
    assert!(
        ir.contains("async.drop.message") && ir.contains("@tyrus_string_release_buffer"),
        "an async string local must be released through its ownership bit"
    );
    assert!(
        ir.contains("@tyrus_string_release_buffer"),
        "an async frame local's string must be released through its ownership bit"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn tracks_and_releases_mutable_string_locals_across_awaits_and_reassignments() {
    let source = r#"
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            let message: string = "before".toUpperCase();
            await load();
            message = "static";
            message = "after".toUpperCase();
            await load();
            message += " concat";
            console.log(message);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-mutable-string-local-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_mutable_string_local",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = read_ir(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.frame.local.message.owned"));
    assert!(ir.contains("release.reassigned.async.string"));
    assert!(ir.contains("concat"));
    assert!(ir.contains("release.tracked.local.string"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn accepts_const_string_local_from_owned_function_return() {
    let source = r#"
            function message(): string { return "static"; }
            async function load(): Promise<void> {}
            async function main(): Promise<void> {
                const value: string = message();
                await load();
                console.log(value);
            }
        "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-owned-string-local-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_owned_string_local",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = read_ir(output.with_extension("ll")).unwrap();
    assert!(ir.contains("owned.return"));
    assert!(
        ir.contains("@tyrus_string_release_buffer"),
        "an async frame local's string must be released through its ownership bit"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
}

#[test]
pub(super) fn consecutive_string_literals_fold_into_a_single_printf_call() {
    let source = r#"const main = (): void => { print("a", "b", "c"); };"#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-print-fold-{}", std::process::id()));
    compile(&program, "print_fold", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains(r#"c"a b c\00""#),
        "expected the three literals and their separators folded into one \"a b c\" constant, got:\n{ir}"
    );
    assert_eq!(
        ir.matches("@print.str").count(),
        2,
        "expected exactly one printf call for the whole folded literal run"
    );
    assert_eq!(
        ir.matches("print.arg.space").count(),
        0,
        "an all-literal argument list must never emit separate space literals"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn emits_runtime_string_concatenation() {
    let source = r#"
        export const greetingWithConcatenation = (name: string): void => {
            console.log("Hello, " + name);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-concat-test-{}", std::process::id()));
    compile(&program, "concat_test", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("@llvm.memcpy"),
        "expected the concatenation to copy by length:\n{ir}"
    );
    for absent in ["@strlen", "@strcpy", "@strcat"] {
        assert!(
            !ir.contains(absent),
            "`{absent}` walks to a terminator the value no longer needs:\n{ir}"
        );
    }
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn emits_runtime_string_for_template_literal() {
    let source = r#"
        const greet = (name: string): void => {
            console.log(`Hello, ${name}`);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-template-test-{}", std::process::id()));
    compile(&program, "template_test", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(ir.contains("@llvm.memcpy"));
    assert!(ir.contains("@tyrus_string_alloc"));
    for absent in ["@snprintf", "@calloc"] {
        assert!(
            !ir.contains(absent),
            "a template literal no longer measures its parts with `{absent}`:\n{ir}"
        );
    }
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn interpolates_wide_and_unsigned_primitives_as_strings() {
    let source = r#"
        const main = (): void => {
            const a: u128 = 42;
            const b: usize = 7;
            const c: boolean = true;
            const d: f64 = 1.5;
            const e: i128 = -9000000000;
            console.log(`${a} ${b} ${c} ${d} ${e}`);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-template-wide-{}", std::process::id()));
    compile(&program, "template_wide", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(ir.contains("tyrus_format_u128"));
    assert!(ir.contains("tyrus_format_i128"));
    assert!(ir.contains("tyrus_format_f64"));
    assert!(
        !ir.contains("%lld"),
        "template must not pass raw integer varargs"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn emits_native_string_methods() {
    let source = r#"
        const main = (): void => {
            const name: string = "tyrus";
            const len: i64 = name.length;
            const ch: string = name.charAt(0);
            const code: i32 = name.charCodeAt(1);
            const found: boolean = name.includes("r");
            const sliced: string = name.slice(0, 2);
            const trimmed: string = name.trim();
            const lower: string = name.toLowerCase();
            const upper: string = name.toUpperCase();
            const replaced: string = name.replace("s", "S");
            const regexMatch: string[]? = name.match(/r(st)/);
            const regexMatchAll: string[][] = name.matchAll(/[rs]/);
            const regexSearch: i32 = name.search(/st/);
            const regexReplaced: string = name.replace(/s+/, "S");
            const regexSplit: string[] = name.split(/s/);
            const constructedRegex: Regex = /TYRUS/i;
            const constructedRegexMatch: boolean = constructedRegex.test(name);
            const repeated: string = name.repeat(3);
            const padded: string = name.padStart(10, "-");
            const joined: string = name.concat("!", "?");
            console.log(len);
            console.log(ch);
            console.log(code);
            console.log(found);
            console.log(sliced);
            console.log(trimmed);
            console.log(lower);
            console.log(upper);
            console.log(replaced);
            if (regexMatch) {
                console.log(regexMatch[0]);
                console.log(regexMatch[1]);
            }
            console.log(regexSearch);
            console.log(regexReplaced);
            console.log(constructedRegexMatch);
            console.log(repeated);
            console.log(padded);
            console.log(joined);
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-string-methods-{}", std::process::id()));
    compile(&program, "string_methods", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    for symbol in [
        "tyrus_string_char_at",
        "tyrus_string_char_code_at",
        "tyrus_string_includes",
        "tyrus_string_slice",
        "tyrus_string_trim",
        "tyrus_string_to_lower",
        "tyrus_string_to_upper",
        "tyrus_string_replace",
        "tyrus_regex_exec",
        "tyrus_regex_exec_all",
        "tyrus_string_search_regex",
        "tyrus_regex_replace",
        "tyrus_regex_split",
        "tyrus_string_repeat",
        "tyrus_string_pad_start",
    ] {
        assert!(ir.contains(symbol), "missing native call `{symbol}`");
    }
    assert!(
        ir.contains("tyrus_string_concat"),
        "`.concat()` is still a runtime call:\n{ir}"
    );
    assert!(
        !ir.contains("tyrus_string_len"),
        "`length` should read the character count from the value:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn emits_runtime_validator_check_for_string_decorators() {
    let source = r#"
        interface SignupInput {
            @minLength(3)
            name: string;

            @email()
            email: string;

            @cpf()
            cpf: string;

            @cnpj()
            cnpj: string;
        }

        const main = (): void => {
            const name: string = "Ada";
            const email: string = "ada@example.com";
            const cpf: string = "123.456.789-09";
            const cnpj: string = "AB.CDE.F12/3456-80";
            const input: SignupInput = {
                name: name,
                email: email,
                cpf: cpf,
                cnpj: cnpj,
            };
            console.log(input.name);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-validator-string-{}", std::process::id()));
    compile(
        &program,
        "validator_string",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = read_ir(output.with_extension("ll")).unwrap();
    assert!(
        !ir.contains("tyrus_string_len"),
        "`@minLength` should read the count from the value:\n{ir}"
    );
    assert!(ir.contains("tyrus_string_is_email"));
    assert!(ir.contains("tyrus_string_is_cpf"));
    assert!(ir.contains("tyrus_string_is_cnpj"));
    assert!(ir.contains(
        "tyrus validation error: `@minLength(3)` failed for `name`: value must contain at least 3 characters"
    ));
    assert!(ir.contains(
        "tyrus validation error: `@email()` failed for `email`: value must be a valid e-mail address"
    ));
    assert!(ir.contains(
        "tyrus validation error: `@cpf()` failed for `cpf`: value must be a valid Brazilian CPF"
    ));
    assert!(ir.contains(
        "tyrus validation error: `@cnpj()` failed for `cnpj`: value must be a valid Brazilian CNPJ"
    ));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
pub(super) fn emits_typeof_as_string_constant() {
    let source = r#"
        const main = (): void => {
            const t: string = typeof 42;
            console.log(t);
        };
    "#;
    let tokens: Vec<Token> = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-typeof-{}", std::process::id()));
    compile(&program, "typeof_example", output.to_str().unwrap(), false).unwrap();
    let ir = read_ir(output.with_extension("ll")).unwrap();
    assert!(ir.contains("i32"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
pub(super) fn env_startup_releases_owned_strings_used_only_for_validation() {
    let ir = compile_with_env_to_ir(
        "env_string_cleanup",
        "@type(string)\n@minLength(1)\nTYRUS_ENV=development\n",
        "console.log(\"ready\");",
    );
    let getter = ir
        .find(&format!(
            "call {{ ptr, i64 }} {}",
            native_abi_call("tyrus_env_get_str")
        ))
        .expect("string getter");
    let release = ir[getter..]
        .find("@tyrus_string_release_buffer")
        .expect("validation string release");
    assert!(release > 0);
}

#[test]
pub(super) fn a_string_switch_is_unaffected_by_the_jump_table() {
    let source = r#"
        function classify(s: string): i32 {
            switch (s) {
                case "a": return 1;
                case "b": return 2;
                default: return 0;
            }
        }
        console.log(classify("b"));
    "#;
    let ir = compile_to_ir(source, "switch-string");
    assert!(ir.contains("switch.next"));
    assert!(!ir.contains("switch i32"));
}

#[test]
pub(super) fn a_ternary_over_strings_keeps_its_branches() {
    let source = r#"
        function label(n: i32): string {
            const a = "left";
            const b = "right";
            return n > 0 ? a : b;
        }
        console.log(label(1));
    "#;
    let ir = compile_to_ir(source, "ternary-string");
    assert!(ir.contains("tern.then"));
    assert!(ir.contains("tern.phi"));
}

#[test]
pub(super) fn string_count_reads_the_length_field_or_calls_count_occurrences() {
    let source = r#"
        function main() {
            const text: string = "banana";
            console.log(text.count());
            console.log(text.count("an"));
        }
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-string-count-{}", std::process::id()));
    compile(&program, "string_count_ir", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        !ir.contains("@tyrus_string_len("),
        "`count()` should read the character count from the value:\n{ir}"
    );
    assert!(
        ir.contains("call i64 @tyrus_string_count_occurrences("),
        "expected a call to tyrus_string_count_occurrences:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn string_char_at_registers_its_result_as_owned() {
    // `charAt` builds a fresh single-character string but was missing from
    // `string_method_allocates`, so nothing ever claimed it, confirmed via
    // valgrind to leak 2 bytes per call. This asserts the fix reaches
    // codegen: the bound result must be released at scope exit.
    let source = r#"
        function main() {
            const ch: string = "hello".charAt(1);
            console.log(ch);
        }
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-string-char-at-owned-{}", std::process::id()));
    compile(
        &program,
        "string_char_at_owned",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = read_ir(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains(string_release_call()),
        "expected charAt's result to be released at scope exit:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn string_concat_with_three_arguments_frees_every_intermediate_result() {
    // With N variadic arguments, `concat` built N-1 intermediate buffers
    // before the final result, only the last was ever claimed. Confirmed
    // via valgrind to leak exactly N-1 blocks, scaling with argument count.
    // This asserts the loop frees a superseded intermediate before
    // overwriting it: three `free` calls total (two intermediates plus the
    // final result at scope exit).
    let source = r#"
        function main() {
            const name: string = "TYRUS";
            const s: string = name.concat(" a", " b", " c");
            console.log(s);
        }
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-string-concat-three-args-{}",
        std::process::id()
    ));
    compile(
        &program,
        "string_concat_three_args",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = read_ir(output.with_extension("ll")).unwrap();
    let free_calls = ir.matches(string_release_call()).count();
    assert!(
        free_calls >= 3,
        "expected at least 3 frees (2 intermediates + final result), found {free_calls}:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn reduce_with_a_string_accumulator_frees_every_superseded_intermediate() {
    // `words.reduce((acc, w) => acc.concat(w), "")` built a fresh string on
    // every call, even a bare `(acc, w) => acc` clones on return, since
    // every synchronous closure return already clones whenever the return
    // type is owned-value-supported, but the loop discarded every
    // intermediate accumulator except by coincidence, and never claimed the
    // final result at all. Confirmed via valgrind to leak N allocations for
    // N array elements. Closed with a runtime ownership bit (the loop body
    // compiles once but runs N times, so a compile-time bool cannot track
    // this) freed before each overwrite, and the final value handed to the
    // caller through the same conditionally-owned mechanism a per-path-owned
    // user function return already uses.
    let source = r#"
        function main() {
            const words: string[] = ["a", "b", "c"];
            const joined: string = words.reduce((acc: string, w: string): string => acc.concat(w), "");
            console.log(joined);
        }
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-reduce-string-accumulator-{}",
        std::process::id()
    ));
    compile(
        &program,
        "reduce_string_accumulator",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = read_ir(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("reduce.accumulator.free"),
        "expected a conditional free block inside the reduce loop body:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn a_mutable_string_reassigned_from_a_closure_compiling_call_does_not_desync_scope_stacks()
 {
    // Regression test for a compiler panic (`index out of bounds`), not a
    // leak: `emit_arrow_fn`/`emit_async_arrow_fn` save and restore several
    // parallel scope-stack fields while compiling a closure's body in
    // isolation (`vars`, `owned_native_vars`, `pending_owned_native_results`,
    // ...) but were missing three, `conditionally_owned_vars`,
    // `pending_conditionally_owned_results`, `mutable_string_ownership`.
    // Compiling a second closure in the same outer scope left those three
    // permanently one scope deeper than `vars`, and the next scope-exit
    // cleanup indexed into `vars` with that stale, too-large scope index.
    // Reproduced with no `reduce` involved at all, any two calls compiling
    // a closure, assigned to a mutable `string` binding and reassigned,
    // triggered it.
    let source = r#"
        function main() {
            let acc: string = "x".toUpperCase().concat("y".toUpperCase());
            console.log(acc);
            const words: string[] = ["a", "b", "c"];
            acc = words.map((w: string): string => w.toUpperCase()).join(",");
            console.log(acc);
        }
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-mutable-string-closure-scope-{}",
        std::process::id()
    ));
    compile(
        &program,
        "mutable_string_closure_scope",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
pub(super) fn module_global_string_survives_other_functions() {
    // Regression for the REPL double-free: a template string assigned to a
    // module global used to be freed at the end of *every* function that
    // merely ran after it, leaving the global dangling for the next step.
    let ir = compile_with_env_to_ir(
        "repl_global_owned_survival",
        "",
        r#"
            let name = "Thomas";
            let completeName = `${name}`;

            function unrelated(): i32 {
                console.log("hi");
                return 0;
            }

            function reader(): i32 {
                console.log(completeName);
                return 0;
            }
        "#,
    );
    for fn_name in ["unrelated", "reader"] {
        let body = ir_fn_body(&ir, fn_name);
        assert!(
            !body.contains("@free"),
            "`{fn_name}` must not free the module global:\n{body}"
        );
    }
}

/// The ownership bit is clear on a literal. A `.rodata` buffer that reported
/// itself as freeable would be freed, and freeing a literal is a crash rather
/// than a leak, which is the whole reason the bit exists.
#[test]
pub(super) fn a_string_literal_is_never_owned() {
    let context = Context::create();
    let codegen = Codegen::new(
        &context,
        "string_literal_ownership",
        false,
        CompileTarget::Native,
    );

    for text in ["", "hello", "a\u{0}b", "🚀"] {
        let value = codegen.const_string(text);
        let packed = value
            .get_field_at_index(1)
            .unwrap()
            .into_int_value()
            .get_zero_extended_constant()
            .unwrap();
        assert_eq!(packed & (1 << 31), 0, "`{text}` claimed to own .rodata");
        assert_eq!(packed & 0xffff_ffff, text.len() as u64);
        assert_eq!(packed >> 32, text.chars().count() as u64);
    }
}

#[test]
pub(super) fn a_string_is_two_fields_wide_as_the_c_abi_requires() {
    let context = Context::create();
    let codegen = Codegen::new(&context, "string_shape", false, CompileTarget::Native);

    let string_type = codegen.string_type();
    assert_eq!(string_type.count_fields(), 2);
    assert!(
        string_type
            .get_field_type_at_index(0)
            .unwrap()
            .is_pointer_type()
    );
    assert_eq!(
        string_type
            .get_field_type_at_index(1)
            .unwrap()
            .into_int_type(),
        context.i64_type()
    );
}

#[test]
pub(super) fn a_literal_of_static_strings_in_an_object_still_frees_only_once() {
    // The other half of the same change, and the one that would be corruption
    // rather than a leak if the reasoning were wrong: releasing a literal of
    // `.rodata` strings deeply must free the buffer and leave the elements
    // alone. That is the runtime's job, each element reports that it owns
    // nothing, so the IR is the same single deep release either way.
    let ir = compile_to_ir(
        r#"
        interface Bucket {
            labels: string[];
        }

        const main = (): void => {
            let bucket: Bucket = { labels: ["alpha", "beta"] };
            console.log(bucket.labels[1]);
        };
        "#,
        "object-field-static-string-array",
    );
    assert_eq!(
        ir.matches("call void @tyrus_array_drop_string_contents")
            .count(),
        1,
        "expected exactly one deep release of the static literal:\n{ir}"
    );
}
