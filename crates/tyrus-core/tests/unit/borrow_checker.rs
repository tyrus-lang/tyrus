use super::*;
use crate::lexer::Token;
use crate::parser::Parser;
use logos::Logos;

fn errors_for(source: &str) -> Vec<String> {
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    BorrowChecker::new().check_program(&program)
}

#[test]
fn optional_force_unwrap_is_not_a_borrow_checker_warning() {
    let tokens = Token::lexer(
        "const show = (mut value: string?): void => { value = \"updated\"; console.log(value!); };",
    )
    .filter_map(Result::ok)
    .collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert!(checker.check_program(&program).is_empty());
    assert!(checker.warnings().is_empty());
}

#[test]
fn accepts_copy_capture_in_arrow_function_value() {
    let errors = errors_for(
        "const main = (): void => { const offset: i32 = 1; const add: (i32) => i32 = (value: i32): i32 => value + offset; };",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn rejects_moving_a_value_captured_by_a_pending_finalizer() {
    let errors = errors_for(
        "const main = (): void => { const resource: string = \"open\"; defer console.log(resource); console.log(resource); const moved: string = resource; };",
    );
    assert!(
        errors.iter().any(|error| error
            .contains("cannot move `resource` because it is captured by a pending finalizer")),
        "expected finalizer capture error, got {errors:?}"
    );
}

#[test]
fn rejects_double_consumption_scheduled_by_finalizers() {
    let errors = errors_for(
        r#"
        import { drop } from "tyrus:memory";
        const main = (): void => {
            let values = [1, 2];
            defer drop(values);
            errdefer drop(values);
        };
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("pending finalizer already consumes it")),
        "expected scheduled double-consumption diagnostic, got {errors:#?}"
    );
}

#[test]
fn accepts_reading_an_inferred_copy_binding_after_aliasing_it() {
    let errors = errors_for(
        "const main = (): void => { const number = 40; const copyNumber = number; console.log(number); console.log(copyNumber); };",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn accepts_reading_inferred_copy_bindings_of_every_literal_kind_after_aliasing_them() {
    let errors = errors_for(
        "const main = (): void => { \
            const flag = true; const copyFlag = flag; console.log(flag); console.log(copyFlag); \
            const ratio = 3.14; const copyRatio = ratio; console.log(ratio); console.log(copyRatio); \
        };",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn distinguishes_dropped_bindings_from_moved_bindings() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                const moved: string = "moved";
                const alias: string = moved;
                console.log(moved);

                const dropped: string = "dropped";
                __tyrus_memory_drop(dropped);
                console.log(dropped);

                const erased: string = "erased";
                __tyrus_memory_zeroize(erased);
                console.log(erased);
                console.log(alias);
            };
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("use of moved value: `moved`")),
        "{errors:?}"
    );
    assert_eq!(
        errors
            .iter()
            .filter(|error| error.contains("use of dropped value:"))
            .count(),
        2,
        "{errors:?}"
    );
}

#[test]
fn preserves_dropped_state_only_when_every_branch_drops_the_binding() {
    let errors = errors_for(
        r#"
            const main = (flag: boolean): void => {
                const alwaysDropped: string = "always";
                if (flag) {
                    __tyrus_memory_drop(alwaysDropped);
                } else {
                    __tyrus_memory_drop(alwaysDropped);
                }
                console.log(alwaysDropped);

                const conditionallyDropped: string = "sometimes";
                if (flag) {
                    __tyrus_memory_drop(conditionallyDropped);
                }
                console.log(conditionallyDropped);
            };
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("use of dropped value: `alwaysDropped`")),
        "{errors:?}"
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("use of moved value: `conditionallyDropped`")),
        "{errors:?}"
    );
}

#[test]
fn merges_try_and_catch_ownership_states_before_running_finally() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                const alwaysDropped: string = "always";
                try {
                    __tyrus_memory_drop(alwaysDropped);
                } catch (error: string) {
                    __tyrus_memory_drop(alwaysDropped);
                }
                console.log(alwaysDropped);

                const conditionallyDropped: string = "sometimes";
                try {
                    __tyrus_memory_drop(conditionallyDropped);
                } catch (error: string) {
                    console.log(error);
                } finally {
                    console.log("complete");
                }
                console.log(conditionallyDropped);
            };
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("use of dropped value: `alwaysDropped`")),
        "{errors:?}"
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("use of moved value: `conditionallyDropped`")),
        "{errors:?}"
    );
}

#[test]
fn ignores_dropped_states_on_paths_that_return_or_throw() {
    let errors = errors_for(
        r#"
            const fromReturn = (flag: boolean): void => {
                const token: string = "token";
                if (flag) {
                    __tyrus_memory_drop(token);
                    return;
                }
                console.log(token);
            };

            const fromThrow = (flag: boolean): void => {
                const token: string = "token";
                try {
                    if (flag) {
                        __tyrus_memory_drop(token);
                        throw "failed";
                    }
                } catch (error: string) {
                    return;
                }
                console.log(token);
            };
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn rejects_mismatched_arrow_function_type() {
    let errors = errors_for(
        "const main = (): void => { const convert: (i32) => boolean = (value: i32): i32 => value; };",
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("arrow function has type"))
    );
}

#[test]
fn accepts_contextually_typed_arrow_function_value() {
    let errors = errors_for(
        "const main = (): void => { const factor: i32 = 2; const transform: (i32) => i32 = value => value * factor; const values: i32[] = [1, 2]; const mapped: i32[] = values.map(transform); };",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn function_calls_do_not_move_non_copy_arguments() {
    let errors = errors_for(
        r#"
            interface User {
                name: string;
            }

            const greet = (name: string): void => {
                console.log(name);
            };

            const showUser = (user: User): void => {
                console.log(user.name);
            };

            const main = (): void => {
                const name: string = "Ada";
                greet(name);
                console.log(name);

                const user: User = { name: "Grace" };
                showUser(user);
                console.log(user.name);
            };
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn rejects_returning_reference_to_local_variable() {
    let errors = errors_for(
        r#"
            const bad = (): &string => {
                const name: string = "Ada";
                return &name;
            };
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("cannot return reference to local variable `name`")),
        "expected local reference return error, got {errors:?}"
    );
}

#[test]
fn rejects_returning_stored_reference_to_local_variable() {
    let errors = errors_for(
        r#"
            const bad = (): &string => {
                const name: string = "Ada";
                const borrowed: &string = &name;
                return borrowed;
            };
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("cannot return reference to local variable `name`")),
        "expected stored local reference return error, got {errors:?}"
    );
}

#[test]
fn accepts_returning_reference_parameter() {
    let errors = errors_for(
        r#"
            const identity = (name: &string): &string => {
                return name;
            };
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn rejects_assigning_inner_reference_to_outer_binding() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                const owner: string = "owner";
                let current: &string = &owner;
                if (true) {
                    const local: string = "local";
                    current = &local;
                }
                console.log(current);
            };
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("borrowed value `local` does not live long enough")),
        "expected escaped inner borrow error, got {errors:?}"
    );
}

#[test]
fn releases_temporary_borrows_after_statement() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                let name: string = "Ada";
                console.log(&name);
                name = "Grace";
            };
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn keeps_stored_borrows_until_binding_leaves_scope() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                let name: string = "Ada";
                const borrowed: &string = &name;
                name = "Grace";
                console.log(borrowed);
            };
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("while it is borrowed")),
        "expected stored borrow error, got {errors:?}"
    );
}

#[test]
fn releases_stored_borrows_after_their_last_use() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                let name: string = "Ada";
                const borrowed: &string = &name;
                console.log(borrowed);
                name = "Grace";
            };
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn keeps_a_borrow_alive_through_a_later_loop() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                let name: string = "Ada";
                const borrowed: &string = &name;
                let index: i32 = 0;
                while (index < 2) {
                    name = "Grace";
                    console.log(borrowed);
                    index = index + 1;
                }
            };
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("while it is borrowed")),
        "expected the borrow to outlive the loop, got {errors:?}"
    );
}

#[test]
fn keeps_a_borrow_alive_while_a_closure_still_captures_it() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                let name: string = "Ada";
                const borrowed: &string = &name;
                name = "Grace";
                const show: () => void = (): void => {
                    console.log(borrowed);
                };
                show();
            };
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("while it is borrowed")),
        "expected the capture to keep the borrow alive, got {errors:?}"
    );
}

#[test]
fn keeps_a_borrow_alive_while_a_closure_declared_earlier_still_holds_it() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                let name: string = "Ada";
                const borrowed: &string = &name;
                const show: () => void = (): void => {
                    console.log(borrowed);
                };
                name = "Grace";
                show();
            };
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("while it is borrowed")),
        "expected the closure to carry the borrow to its own last use, got {errors:?}"
    );
}

#[test]
fn keeps_a_borrow_alive_until_scope_exit_when_a_defer_reads_it() {
    let errors = errors_for(
        r#"
            function work(): void {
                let name: string = "Ada";
                const borrowed: &string = &name;

                defer { console.log(borrowed); }

                name = "Grace";
                return;
            }
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("while it is borrowed")),
        "expected the deferred read to hold the borrow to the closing brace, got {errors:?}"
    );
}

#[test]
fn a_defer_that_does_not_read_the_borrow_still_releases_it() {
    let errors = errors_for(
        r#"
            function work(): void {
                let name: string = "Ada";
                const borrowed: &string = &name;
                console.log(borrowed);

                defer { console.log("done"); }

                name = "Grace";
                return;
            }
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn accepts_borrow_that_ends_before_await_without_a_nested_block() {
    let errors = errors_for(
        r#"
            async function main(ready: Promise<void>): Promise<void> {
                const name: string = "Ada";
                const borrowed: &string = &name;
                console.log(borrowed);
                await ready;
                console.log(name);
            }
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn releases_stored_borrows_after_branch_scope() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                let name: string = "Ada";
                if (true) {
                    const borrowed: &string = &name;
                    console.log(borrowed);
                }
                name = "Grace";
            };
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn rejects_stored_borrow_across_await() {
    let errors = errors_for(
        r#"
            async function main(ready: Promise<void>): Promise<void> {
                const name: string = "Ada";
                const borrowed: &string = &name;
                await ready;
                console.log(borrowed);
            }
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("borrow `borrowed` of `name` crosses `await`")),
        "expected async borrow crossing error, got {errors:?}"
    );
}

#[test]
fn rejects_reference_parameter_across_await() {
    let errors = errors_for(
        r#"
            async function main(name: &string, ready: Promise<void>): Promise<void> {
                await ready;
                console.log(name);
            }
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("borrow `name` of `name` crosses `await`")),
        "expected async reference parameter crossing error, got {errors:?}"
    );
}

#[test]
fn rejects_async_closure_capturing_reference_across_await() {
    let errors = errors_for(
        r#"
            const main = (ready: Promise<void>): void => {
                const name: string = "Ada";
                const borrowed: &string = &name;
                const run: () => Promise<void> = async (): Promise<void> => {
                    await ready;
                    console.log(borrowed);
                };
            };
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("borrow `borrowed` of `borrowed` crosses `await`")),
        "expected async closure capture crossing error, got {errors:?}"
    );
}

#[test]
fn accepts_owned_and_copy_values_across_await() {
    let errors = errors_for(
        r#"
            async function main(ready: Promise<void>): Promise<void> {
                const name: string = "Ada";
                const count: i32 = 1;
                await ready;
                console.log(name);
                console.log(count);
            }
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn accepts_borrow_that_ends_before_await() {
    let errors = errors_for(
        r#"
            async function main(ready: Promise<void>): Promise<void> {
                const name: string = "Ada";
                if (true) {
                    const borrowed: &string = &name;
                    console.log(borrowed);
                }
                await ready;
                console.log(name);
            }
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn rejects_use_after_conditional_move() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                const name: string = "Ada";
                if (true) {
                    const moved: string = name;
                    console.log(moved);
                }
                console.log(name);
            };
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("use of moved value: `name`")),
        "expected conditional move error, got {errors:?}"
    );
}

#[test]
fn rejects_move_after_switch_branch_move() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                const name: string = "Ada";
                switch (1) {
                    case 1: {
                        const moved: string = name;
                        console.log(moved);
                    }
                    default:
                        console.log("default");
                }
                console.log(name);
            };
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("use of moved value: `name`")),
        "expected switch branch move error, got {errors:?}"
    );
}

#[test]
fn rejects_use_after_move_across_switch_fallthrough() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                const name: string = "Ada";
                switch (1) {
                    case 1: {
                        const moved: string = name;
                        console.log(moved);
                    }
                    case 2: {
                        console.log(name);
                        break;
                    }
                }
            };
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("use of moved value: `name`")),
        "expected fallthrough move error, got {errors:?}"
    );
}

#[test]
fn switch_break_prevents_a_false_fallthrough_move_error() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                const name: string = "Ada";
                switch (1) {
                    case 1: {
                        const moved: string = name;
                        console.log(moved);
                        break;
                    }
                    case 2: {
                        console.log(name);
                        break;
                    }
                }
            };
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn rejects_use_of_non_copy_after_escaping_closure_capture() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                const label: string = "hello";
                const f: () => void = (): void => {
                    console.log(label);
                };
                console.log(label);
            };
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("use of moved value: `label`")),
        "expected move error for non-Copy capture, got {errors:?}"
    );
}

#[test]
fn accepts_copy_type_after_escaping_closure_capture() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                const count: i32 = 42;
                const f: () => i32 = (): i32 => {
                    return count;
                };
                const again: i32 = count;
            };
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn accepts_non_copy_capture_in_sync_array_callback() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                const prefix: string = "item";
                const numbers: i32[] = [1, 2, 3];
                numbers.forEach((n: i32): void => {
                    console.log(prefix);
                });
                console.log(prefix);
            };
        "#,
    );
    assert!(
        errors.is_empty(),
        "sync callback must not move non-Copy captures: {errors:?}"
    );
}

#[test]
fn accepts_non_copy_capture_in_map_callback() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                const suffix: string = "px";
                const values: i32[] = [1, 2, 3];
                const result: string[] = values.map((n: i32): string => {
                    return suffix;
                });
                console.log(suffix);
            };
        "#,
    );
    assert!(
        errors.is_empty(),
        "map callback must not move non-Copy captures: {errors:?}"
    );
}

#[test]
fn rejects_capture_of_already_moved_value_in_closure() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                const name: string = "Ada";
                const moved: string = name;
                const f: () => void = (): void => {
                    console.log(name);
                };
            };
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("use of moved value: `name`")),
        "expected error capturing already-moved value, got {errors:?}"
    );
}

#[test]
fn allows_copy_type_capture_across_multiple_closures() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                const x: i32 = 10;
                const add: (i32) => i32 = (n: i32): i32 => n + x;
                const mul: (i32) => i32 = (n: i32): i32 => n * x;
            };
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn closure_body_can_use_captured_non_copy_value() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                const greeting: string = "hello";
                const f: () => void = (): void => {
                    console.log(greeting);
                };
            };
        "#,
    );
    assert!(
        errors.is_empty(),
        "closure body should access captured value without errors: {errors:?}"
    );
}

#[test]
fn throw_inside_try_does_not_consume_catch_binding() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                try {
                    throw "something went wrong";
                } catch (e: string) {
                    console.log(e);
                }
            };
        "#,
    );
    assert!(errors.is_empty(), "unexpected borrow errors: {errors:?}");
}

#[test]
fn finally_body_is_checked() {
    let errors = errors_for(
        r#"
            const main = (): void => {
                try {
                    throw "err";
                } catch (e: string) {
                    console.log(e);
                } finally {
                    console.log("cleanup");
                }
            };
        "#,
    );
    assert!(errors.is_empty(), "unexpected borrow errors: {errors:?}");
}

#[test]
fn tuple_of_copy_types_is_copy_not_moved() {
    let errors = errors_for(
        "const a: [i32, i32] = [1, 2]; const b: [i32, i32] = a; console.log(a[0]); console.log(b[0]);",
    );
    assert!(errors.is_empty(), "unexpected borrow errors: {errors:?}");
}

#[test]
fn tuple_with_non_copy_element_moves() {
    let errors = errors_for(
        "const a: [i32, string] = [1, \"x\"]; const b: [i32, string] = a; console.log(a[1]);",
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("use of moved value")),
        "expected a move error, got: {errors:?}"
    );
}

#[test]
fn result_propagation_moves_non_copy_source() {
    let errors = errors_for(
        "function load(): Result<string, string> { return Ok(\"value\"); } function main(): Result<string, string> { const result: Result<string, string> = load(); const value: string = result?; console.log(result); return Ok(value); }",
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("use of moved value: `result`")),
        "expected Result propagation to move its source, got: {errors:?}"
    );
}

#[test]
fn result_match_moves_non_copy_subject() {
    let errors = errors_for(
        "function load(): Result<string, string> { return Ok(\"value\"); } function main(): void { const result: Result<string, string> = load(); match (result) { Ok(value) => { console.log(value); } Err(error) => { console.log(error); } } console.log(result); }",
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("use of moved value: `result`")),
        "expected Result match to move its subject, got: {errors:?}"
    );
}

#[test]
fn array_match_bindings_are_scoped_to_their_arm() {
    let errors = errors_for(
        "function main(values: string[]): void { match (values) { [first, ...rest] => { console.log(first); console.log(rest); } _ => { return; } } }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn returning_non_copy_result_moves_parameter() {
    let errors = errors_for(
        "function identity(result: Result<string, string>): Result<string, string> { return result; console.log(result); } function main(): void { const result: Result<string, string> = identity(\"value\"); console.log(result); }",
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("use of moved value: `result`")),
        "expected returning Result to move its parameter, got: {errors:?}"
    );
}

#[test]
fn call_returning_a_function_stays_callable_without_an_annotation() {
    let errors = errors_for(
        "const makeAdder = (base: i32): (value: i32) => i32 => { return (value: i32): i32 => base + value; };\
         const main = (): void => { const add5 = makeAdder(5); console.log(add5(3)); };",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn call_returning_a_non_function_is_still_not_callable() {
    let errors = errors_for(
        "const getNumber = (): i32 => 7;\
         const main = (): void => { const value = getNumber(); console.log(value(3)); };",
    );
    assert!(
        errors.iter().any(|error| error.contains("is not callable")),
        "expected a not-callable error, got: {errors:?}"
    );
}

#[test]
fn native_promise_static_methods_are_not_treated_as_undeclared_variables() {
    let errors = errors_for(
        "async function main(): Promise<void> { const failure: Error = { message: \"boom\", code: \"ERR_BOOM\" }; await Promise.reject(failure); }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn chained_comparison_reads_a_non_copy_middle_term_only_once() {
    let errors = errors_for(
        "const main = (): void => { \
            const a: string = \"x\"; \
            const b: string = \"x\"; \
            const c: string = \"x\"; \
            console.log(a === b === c); \
            console.log(b); \
        };",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

const BASKET: &str = "interface Basket { items: string[]; label: string; }";

#[test]
fn a_field_moved_out_leaves_the_other_fields_usable() {
    let errors = errors_for(&format!(
        "{BASKET} const main = (): void => {{ const basket: Basket = {{ items: [\"a\"], label: \"x\" }}; const taken: string[] = basket.items; console.log(taken.length); console.log(basket.label); }};"
    ));
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn a_moved_field_cannot_be_read_again() {
    let errors = errors_for(&format!(
        "{BASKET} const main = (): void => {{ const basket: Basket = {{ items: [\"a\"], label: \"x\" }}; const taken: string[] = basket.items; console.log(basket.items.length); }};"
    ));
    assert!(
        errors
            .iter()
            .any(|error| error.contains("use of moved value: `basket.items`")),
        "errors: {errors:?}"
    );
}

#[test]
fn a_value_with_a_moved_field_cannot_be_used_whole() {
    let errors = errors_for(&format!(
        "{BASKET} const keep = (basket: Basket): void => {{}}; const main = (): void => {{ const basket: Basket = {{ items: [\"a\"], label: \"x\" }}; const taken: string[] = basket.items; keep(basket); }};"
    ));
    assert!(
        errors.iter().any(|error| error
            .contains("use of moved value: `basket` (its field `items` was moved out)")),
        "errors: {errors:?}"
    );
}

#[test]
fn assigning_a_moved_field_gives_it_back() {
    let errors = errors_for(&format!(
        "{BASKET} const main = (): void => {{ let basket: Basket = {{ items: [\"a\"], label: \"x\" }}; const taken: string[] = basket.items; basket.items = []; console.log(basket.items.length); console.log(taken.length); }};"
    ));
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn a_field_without_an_annotation_is_moved_too() {
    let errors = errors_for(&format!(
        "{BASKET} const main = (): void => {{ const basket: Basket = {{ items: [\"a\"], label: \"x\" }}; const taken = basket.items; console.log(basket.items.length); }};"
    ));
    assert!(
        errors
            .iter()
            .any(|error| error.contains("use of moved value: `basket.items`")),
        "errors: {errors:?}"
    );
}

#[test]
fn a_field_of_a_validated_value_is_not_moved_out() {
    let errors = errors_for(
        "interface Account { @minLength(1) name: string; tags: string[]; } const main = (): void => { const account: Account = { name: \"ada\", tags: [] }; const tags: string[] = account.tags; };",
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("cannot move out of `account`")
                && error.contains("validation rules")),
        "errors: {errors:?}"
    );
}

#[test]
fn a_binding_declared_with_another_type_than_its_field_is_not_moved() {
    let errors = errors_for(
        "interface Profile { nickname?: string; } const main = (): void => { const profile: Profile = { nickname: \"ada\" }; if (profile.nickname) { const nickname: string = profile.nickname; } };",
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("cannot move out of `profile`")
                && error.contains("change shape")),
        "errors: {errors:?}"
    );
}

#[test]
fn a_move_on_one_branch_counts_after_the_branch() {
    let errors = errors_for(&format!(
        "{BASKET} const main = (flag: boolean): void => {{ const basket: Basket = {{ items: [\"a\"], label: \"x\" }}; if (flag) {{ const taken: string[] = basket.items; }} console.log(basket.items.length); }};"
    ));
    assert!(
        errors
            .iter()
            .any(|error| error.contains("use of moved value: `basket.items`")),
        "errors: {errors:?}"
    );
}

#[test]
fn moving_a_binding_from_outside_a_loop_is_refused() {
    let errors = errors_for(
        "const main = (): void => { const names: string[] = [\"a\"]; let round: i32 = 0; while (round < 2) { const taken: string[] = names; round += 1; } };",
    );
    assert!(
        errors.iter().any(|error| error
            .contains("use of moved value: `names` (moved in the previous iteration of the loop)")),
        "errors: {errors:?}"
    );
}

#[test]
fn moving_a_field_from_outside_a_loop_is_refused() {
    let errors = errors_for(&format!(
        "{BASKET} const main = (): void => {{ const basket: Basket = {{ items: [\"a\"], label: \"x\" }}; for (const round of [1, 2]) {{ const taken: string[] = basket.items; }} }};"
    ));
    assert!(
        errors.iter().any(|error| error.contains(
            "use of moved value: `basket.items` (moved in the previous iteration of the loop)"
        )),
        "errors: {errors:?}"
    );
}

#[test]
fn a_loop_that_gives_the_field_back_each_pass_may_move_it() {
    let errors = errors_for(&format!(
        "{BASKET} const main = (): void => {{ let basket: Basket = {{ items: [\"a\"], label: \"x\" }}; for (const round of [1, 2]) {{ const taken: string[] = basket.items; basket.items = []; }} }};"
    ));
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn a_loop_that_leaves_right_after_the_move_may_move() {
    let errors = errors_for(
        "const main = (): void => { const names: string[] = [\"a\"]; while (true) { const taken: string[] = names; break; } };",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn nll_scenario_1_mutable_borrow_followed_by_mutation() {
    let errors = errors_for(
        r#"
        const main = (): void => {
            let x: i32 = 5;
            const r: &mut i32 = &mut x;
            console.log(r);
            x += 1;
        };
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn nll_scenario_2_immutable_borrow_followed_by_mutation() {
    let errors = errors_for(
        r#"
        const main = (): void => {
            let v: i32[] = [1, 2, 3];
            const a: &i32 = &v[0];
            console.log(a);
            v.push(4);
        };
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn nll_scenario_3_borrow_inside_branch_with_subsequent_mutation() {
    let errors = errors_for(
        r#"
        const main = (cond: bool): void => {
            let x: i32 = 5;
            if (cond) {
                const r: &i32 = &x;
                console.log(r);
            }
            x += 1;
        };
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn nll_scenario_4_match_over_reference_with_subsequent_mutation() {
    let errors = errors_for(
        r#"
        interface User { nome: string; }
        const editar = (u: &mut User): void => {};
        const main = (): void => {
            let u_val: User = { nome: "Ada" };
            match (&u_val) {
                _ => {}
            }
            editar(&mut u_val);
        };
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn nll_scenario_5_move_after_borrow() {
    let errors = errors_for(
        r#"
        const consumir = (s: string[]): void => {};
        const main = (): void => {
            const s: string[] = ["texto"];
            const r: &string[] = &s;
            console.log(r[0]);
            consumir(s);
        };
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn nll_scenario_6_partial_move_and_reuse_of_rest() {
    let errors = errors_for(
        r#"
        interface User { nome: string[]; idade: i32; }
        const main = (): void => {
            const u: User = { nome: ["Ada"], idade: 30 };
            const nome: string[] = u.nome;
            console.log(u.idade);
        };
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn nll_scenario_7_loop_with_borrow_per_iteration() {
    let errors = errors_for(
        r#"
        const main = (): void => {
            let v: i32[] = [1, 2, 3];
            for (const x of v) {
                console.log(x);
            }
            v.push(4);
        };
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn nll_scenario_8_rejection_of_escaping_local_reference() {
    let errors = errors_for(
        r#"
        const escape = (): &i32 => {
            const x: i32 = 5;
            return &x;
        };
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("cannot return reference to local variable")),
        "expected error returning local reference, got: {errors:?}"
    );
}

#[test]
fn nll_scenario_9_reassignment_releases_previous_target() {
    let errors = errors_for(
        r#"
        const main = (): void => {
            let a: i32 = 1;
            let b: i32 = 2;
            let r: &mut i32 = &mut a;
            console.log(r);
            r = &mut b;
            a += 1;
            console.log(r);
        };
        "#,
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn nll_negative_mutation_while_borrow_still_used_later() {
    let errors = errors_for(
        r#"
        const main = (): void => {
            let x: i32 = 5;
            const r: &mut i32 = &mut x;
            x += 1;
            console.log(r);
        };
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("cannot assign to `x` while it is borrowed")),
        "expected borrow error, got: {errors:?}"
    );
}

#[test]
fn nll_closure_keeps_borrow_live_until_closure_last_used() {
    let errors = errors_for(
        r#"
        const main = (): void => {
            let x: i32 = 5;
            const r: &i32 = &x;
            const print_r = (): void => {
                console.log(r);
            };
            x += 1;
            print_r();
        };
        "#,
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("cannot assign to `x` while it is borrowed")),
        "expected borrow error across closure call, got: {errors:?}"
    );
}
