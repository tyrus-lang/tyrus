use super::*;

#[test]
pub(super) fn accepts_async_function_returning_promise_payload() {
    let source = r#"
        async function load(): Promise<i32> {
            return 42;
        }

        const useLoad = async (value: Promise<i32>): Promise<i32> => {
            return await value;
        };

        const inferred = async () => 42;
        const inferredVoid = async (): Promise<void> => {};

        async function direct(value: Promise<i32>): Promise<i32> {
            return value;
        }

        const inferredDirect = async (value: Promise<i32>) => value;
        const adopted: Promise<i32> = inferredDirect(direct(Promise.resolve(42)));
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_owned_string_arrays_and_nested_objects_as_async_payloads() {
    let source = r#"
        interface Metadata { label: string; aliases: string[]; }
        interface Result { status: i32; metadata: Metadata; }

        async function labels(): Promise<string[]> {
            return ["ready", "done"];
        }

        async function result(): Promise<Result> {
            return {
                status: 200,
                metadata: { label: "ready", aliases: ["ok"] },
            };
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn reports_async_closure_captures_before_codegen() {
    let source = r#"
        function setup(): void {
            const values: Set<i32> = Set<i32>();
            const calculate = async (): Promise<i32> => values.size;
            console.log(values.size);
        }
    "#;
    let diagnostics = diagnostics(source);
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3085")
        .unwrap_or_else(|| panic!("diagnostics: {diagnostics:#?}"));
    assert!(diagnostic.message.contains("`values`"));
    assert_eq!(diagnostic.subject.as_deref(), Some("values"));
}

#[test]
pub(super) fn accepts_mutable_owned_async_string_locals() {
    let source = r#"
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            let message: string = "before".toUpperCase();
            await load();
            message = "static";
            message = "after".toUpperCase();
            await load();
            console.log(message);
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_mutable_async_string_assignment_from_owned_function_return() {
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
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_owned_const_string_async_locals() {
    let source = r#"
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            const message: string = "before".toUpperCase();
            await load();
            console.log(message);
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_const_string_async_locals_from_owned_function_return() {
    let source = r#"
        function message(): string { return "static"; }
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            const value: string = message();
            await load();
            console.log(value);
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_const_array_literals_across_async_suspensions() {
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
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_owned_dynamic_array_async_locals() {
    let source = r#"
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            const labels: string[] = "twenty,two".split(",");
            const source: i32[] = [20, 22];
            const reversed: i32[] = source.toReversed();
            await load();
            console.log(labels[0]);
            console.log(reversed[0] + reversed[1]);
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_mutable_owned_copy_array_async_locals() {
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
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_mutable_string_array_async_locals_with_tracked_ownership() {
    let source = r#"
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            let labels: string[] = ["static", "array"];
            await load();
            labels = "owned,array".split(",");
            await load();
            labels = ["final", "static"];
            console.log(labels[0]);
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_owned_string_array_mutations_across_async_suspension() {
    let indexed = r#"
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            let labels: string[] = ["static"];
            await load();
            labels[0] = "changed";
            console.log(labels[0]);
        }
    "#;
    assert_eq!(diagnostics(indexed), Vec::<CheckDiagnostic>::new());

    let pushed = r#"
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            let labels: string[] = ["static"];
            await load();
            labels.push("changed");
            labels.unshift("first");
            labels.reverse();
            const last: string? = labels.pop();
            const first: string? = labels.shift();
            console.log(last ?? "missing");
            console.log(first ?? "missing");
        }
    "#;
    assert_eq!(diagnostics(pushed), Vec::<CheckDiagnostic>::new());

    let in_place = r#"
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            let labels: string[] = ["static", "array", "values"];
            await load();
            labels.fill("changed");
            labels.copyWithin(1, 0, 1);
            labels.sort();
            const removed: string[] = labels.splice(0, 1, "replacement");
            console.log(removed[0]);
            console.log(labels[0]);
        }
    "#;
    assert_eq!(diagnostics(in_place), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_static_object_literals_across_async_suspensions() {
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
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_mutable_copy_object_locals_across_async_suspensions() {
    let source = r#"
        interface Cursor {
            row: i32;
            column: i32;
            visible: boolean;
        }
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            let cursor: Cursor = { row: 1, column: 2, visible: true };
            await load();
            cursor.row = 3;
            cursor = { row: 4, column: 5, visible: false };
            await load();
            console.log(cursor.row);
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_whole_reassignment_of_mutable_owned_objects_across_async_suspensions() {
    let source = r#"
        interface Snapshot {
            count: i32;
            label: string;
            tags: string[];
            note?: string;
        }
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            let snapshot: Snapshot = { count: 1, label: "first", tags: ["static"] };
            await load();
            snapshot.count = 2;
            snapshot = {
                count: 3,
                label: "second".toUpperCase(),
                tags: "owned,array".split(","),
                note: "present".toUpperCase(),
            };
            await load();
            console.log(snapshot.label);
            console.log(snapshot.tags[0]);
            console.log(snapshot.note ?? "missing");
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_direct_owned_field_replacement_on_async_objects() {
    let source = r#"
        interface Snapshot { label: string; tags: string[]; }
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            const seed: Snapshot = { label: "first", tags: ["static"] };
            let snapshot: Snapshot = seed;
            await load();
            snapshot.label = snapshot.label.toUpperCase();
            snapshot.tags = "changed,value".split(",");
            snapshot.tags.push("tail");
            console.log(snapshot.tags[0]);
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_recursive_owned_object_locals_across_async_suspensions() {
    let dynamic = r#"
        interface Snapshot { label: string; }
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            const snapshot: Snapshot = { label: "ready".toUpperCase() };
            await load();
            console.log(snapshot.label);
        }
    "#;
    assert_eq!(diagnostics(dynamic), Vec::<CheckDiagnostic>::new());

    let optional = r#"
        interface Snapshot { count?: i32; }
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            const snapshot: Snapshot = {};
            await load();
            console.log(snapshot.count ?? 0);
        }
    "#;
    assert_eq!(diagnostics(optional), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_copy_awaits_nested_in_async_expressions() {
    let source = r#"
        async function value(): Promise<i32> { return 20; }
        function add(left: i32, right: i32): i32 { return left + right; }
        async function total(): Promise<i32> {
            const doubled: i32 = (await value()) * 2;
            console.log(add(doubled, await value()));
            return add(await value(), 2);
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_recursively_nested_async_control_flow() {
    let source = r#"
        async function pause(): Promise<void> {}
        async function nestedIf(flag: boolean): Promise<void> {
            if (flag) { if (flag) { await pause(); } }
        }
        async function nestedLoops(): Promise<void> {
            let outer: i32 = 0;
            while (outer < 1) { let inner: i32 = 0; while (inner < 1) { await pause(); inner += 1; } outer += 1; }
        }
        async function bothBranchesInLoop(flag: boolean): Promise<void> {
            while (flag) { if (flag) { await pause(); break; } else { await pause(); break; } }
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_recursive_async_try_with_synchronous_handlers() {
    let source = r#"
        async function step(): Promise<void> {}
        async function run(flag: boolean): Promise<i32> {
            let result: i32 = 0;
            while (flag) {
                try {
                    if (flag) { await step(); return 7; }
                } catch (error: Error) { result = -1; }
                finally { result += 1; }
            }
            return result;
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_recursive_async_try_with_a_suspending_catch() {
    let source = r#"
        async function step(shouldFail: boolean): Promise<void> throws Error {
            if (shouldFail) { const failure: Error = { message: "boom", code: "ERR_BOOM" }; throw failure; }
        }
        async function pause(): Promise<void> {}
        async function run(): Promise<i32> {
            let result: i32 = 0;
            while (result < 1) {
                try {
                    await step(true);
                } catch (error: Error) {
                    await pause();
                    await pause();
                    result += 1;
                } finally {
                    console.log("cleanup");
                }
            }
            return result;
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_recursive_async_try_with_a_suspending_finally() {
    let source = r#"
        async function pause(): Promise<void> {}
        async function run(): Promise<i32> {
            let result: i32 = 0;
            while (result < 2) {
                try {
                    await pause();
                    if (result === 0) { continue; }
                    return result;
                } catch (error: Error) {
                    await pause();
                } finally {
                    await pause();
                    await pause();
                    result += 1;
                }
            }
            return result;
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_switches_inside_recursive_async_control_flow() {
    let source = r#"
        async function step(shouldFail: boolean): Promise<void> throws Error {
            if (shouldFail) { const failure: Error = { message: "boom", code: "ERR_BOOM" }; throw failure; }
        }
        async function run(shouldFail: boolean): Promise<i32> {
            let result: i32 = 0;
            while (result < 2) {
                result += 1;
                try {
                    switch (result) {
                        case 1: { await step(shouldFail); }
                        default: { await step(false); }
                    }
                } catch (error: Error) { console.log(error.message); }
                finally { console.log("cleanup"); }
            }
            return result;
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_recursive_owned_async_parameter_frames() {
    let source = r#"
        async function optional(value?: i32): Promise<void> {}
        async function rest(...values: i32[]): Promise<void> {}
        async function destructured([value]: i32[]): Promise<void> {}
        async function stringArray(values: string[]): Promise<void> {}
        async function stringRest(...values: string[]): Promise<void> {}
        async function stringPattern([value]: string[]): Promise<void> {}
        async function nativeSize(value: usize): Promise<usize> { return value; }
        interface Point { x: i32; y: i32; }
        async function object({ x, y }: Point): Promise<void> {}
        interface User { name: string; }
        async function owned(value: User): Promise<void> {}
        async function ownedPattern({ name }: User): Promise<void> {}
        interface OwnedList { values: string[]; }
        async function ownedList(value: OwnedList): Promise<void> {}
        interface OwnedGroup { users: User[]; }
        async function ownedGroup(value: OwnedGroup): Promise<void> {}
        async function optionalOwned(value?: User): Promise<void> {}
    "#;
    let diagnostics = diagnostics(source);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3086"),
        "{diagnostics:#?}"
    );
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E3087")
    );
}

#[test]
pub(super) fn accepts_non_async_promise_delegation_and_rejects_a_non_promise_value() {
    assert_eq!(
        diagnostics(
            r#"
                function forward(value: Promise<i32>): Promise<i32> {
                    return value;
                }
            "#,
        ),
        Vec::<CheckDiagnostic>::new()
    );
    assert_error("function invalid(): Promise<i32> { return 1; }", "E0308");
}

#[test]
pub(super) fn reserves_native_async_error_type_names() {
    assert_error("interface Error { message: string; }", "E3009");
    assert_error("type AggregateError = string;", "E3009");
}

#[test]
pub(super) fn rejects_async_function_without_promise_return() {
    assert_error("async function load(): i32 { return 42; }", "E3003");
}

#[test]
pub(super) fn rejects_await_outside_async_function() {
    assert_error(
        "const load = (value: Promise<i32>): i32 => await value;",
        "E3000",
    );
}

#[test]
pub(super) fn rejects_await_in_a_synchronous_function_nested_in_an_async_function() {
    assert_error(
        "async function outer(): Promise<void> { function inner(value: Promise<i32>): i32 { return await value; } }",
        "E3000",
    );
}

#[test]
pub(super) fn reports_async_specific_return_diagnostics() {
    assert_error(
        "async function load(): Promise<i32> { return \"wrong\"; }",
        "E3296",
    );
    assert_error(
        "async function load(flag: boolean): Promise<i32> { if (flag) { return 42; } }",
        "E3004",
    );
}

#[test]
pub(super) fn keeps_sync_return_diagnostics_inside_async_functions() {
    assert_error(
        "async function load(): Promise<void> { const nested = (): i32 => { return \"wrong\"; }; }",
        "E0308",
    );
}

#[test]
pub(super) fn rejects_a_discarded_async_function_promise() {
    assert_error(
        "async function load(): Promise<void> {} async function main(): Promise<void> { load(); }",
        "E3005",
    );
    assert_eq!(
        diagnostics(
            "async function load(): Promise<void> {} async function main(): Promise<void> { await load(); }"
        ),
        Vec::<CheckDiagnostic>::new()
    );
}

#[test]
pub(super) fn accepts_async_try_with_rejection_propagation() {
    assert_eq!(
        diagnostics(
            "async function load(): Promise<void> { try { return; } catch (error: Error) { console.log(error.message); } }"
        ),
        Vec::<CheckDiagnostic>::new()
    );
}

#[test]
pub(super) fn warns_about_a_loop_in_an_async_function_that_never_yields() {
    let source = r#"
        async function main(): Promise<void> {
            let total: i32 = 0;
            while (true) {
                total += 1;
            }
        }
    "#;
    let diagnostics = diagnostics(source);
    let warning = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3032")
        .unwrap_or_else(|| panic!("expected E3032, got {diagnostics:#?}"));
    assert_eq!(warning.severity, CheckSeverity::Warning);
    assert!(
        warning.help.iter().any(|help| help.contains("`await`")),
        "expected a help line naming the fix, got {:#?}",
        warning.help
    );
}

#[test]
pub(super) fn an_async_return_literal_missing_a_field_names_the_field() {
    assert_error(
        "async function reason(): Promise<Error> { return { message: \"async failure\" }; }",
        "E2741",
    );
}
