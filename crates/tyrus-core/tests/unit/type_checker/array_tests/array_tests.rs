use super::*;

#[test]
pub(super) fn accepts_mutable_copy_array_assignment_from_owned_function_return() {
    let source = r#"
        function values(): i32[] { return [20, 22]; }
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            let current: i32[] = [20, 22];
            await load();
            current = values();
            console.log(current[0]);
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_mutable_string_array_assignment_from_owned_function_return() {
    let source = r#"
        function labels(): string[] { return ["dynamic"]; }
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            let current: string[] = ["static"];
            await load();
            current = labels();
            console.log(current[0]);
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_a_direct_await_inside_a_copy_array_for_of_body() {
    let source = r#"
        async function pause(): Promise<void> {}
        async function run(): Promise<void> {
            const values: i32[] = [1, 2, 3];
            for (const value of values) {
                await pause();
                console.log(value);
            }
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_for_of_over_array() {
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
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn rejects_for_of_over_non_array() {
    let source = r#"
        const main = (): void => {
            const value: i32 = 1;
            for (const item of value) {
                console.log(item);
            }
        };
    "#;
    assert_error(source, "E0308");
}

#[test]
pub(super) fn rejects_mutation_of_const_asserted_array() {
    let source = r#"
        let ports: readonly i32[] = [3000, 3001] as const;
        ports.push(3002);
    "#;
    assert_error(source, "E2540");
}

#[test]
pub(super) fn rejects_assigning_a_readonly_array_to_a_mutable_array() {
    let source = r#"
        const readonlyPorts: readonly i32[] = [3000, 3001];
        let ports: i32[] = readonlyPorts;
    "#;
    assert_error(source, "E0308");
}

#[test]
pub(super) fn rejects_assigning_a_const_asserted_array_to_a_mutable_array() {
    let source = r#"
        const ports: i32[] = [3000, 3001] as const;
    "#;
    assert_error(source, "E0308");
}

#[test]
pub(super) fn accepts_a_const_asserted_array_without_an_annotation() {
    let source = r#"
        const ports = [3000, 3001] as const;
        const first: i32 = ports[0];
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn rejects_assigning_an_inferred_const_asserted_array_to_a_mutable_array() {
    let source = r#"
        const ports = [3000, 3001] as const;
        let mutablePorts: i32[] = ports;
    "#;
    assert_error(source, "E0308");
}

#[test]
pub(super) fn rejects_writing_through_an_inferred_const_asserted_array() {
    let source = r#"
        const ports = [3000, 3001] as const;
        ports[0] = 3002;
    "#;
    assert_error(source, "E2540");
}

#[test]
pub(super) fn accepts_assigning_a_readonly_array_to_a_readonly_array() {
    let source = r#"
        const readonlyPorts: readonly i32[] = [3000, 3001];
        const alias: readonly i32[] = readonlyPorts;
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn rejects_mutating_method_on_a_readonly_array_field() {
    let source = r#"
        interface User {
            tags: string[];
        }

        function getUser(): User {
            return { tags: ["a"] };
        }

        const user: readonly User = getUser();
        user.tags.push("b");
    "#;
    assert_error(source, "E2540");
}

#[test]
pub(super) fn rejects_mutating_method_through_a_readonly_array_index() {
    let source = r#"
        interface User {
            tags: string[];
        }

        function getUsers(): User[] {
            return [{ tags: ["a"] }];
        }

        const users: readonly User[] = getUsers();
        users[0].tags.push("b");
    "#;
    assert_error(source, "E2540");
}

#[test]
pub(super) fn exposes_the_array_read_surface_on_a_tuple() {
    let source = r#"
        const point: [i32, i32, i32] = [10, 20, 30];
        const has: boolean = point.includes(20);
        const at: i64 = point.indexOf(30);
        const size: usize = point.length;
        const doubled: i32[] = point.map((value: i32): i32 => value * 2);
        const big: boolean = point.some((value: i32): boolean => value > 25);
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn fills_an_array_from_a_tuple() {
    let source = r#"
        const point: [i32, i32] = [1, 2];
        const mutable: i32[] = point;
        const frozen: readonly i32[] = point;
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn fills_a_readonly_array_from_a_readonly_tuple() {
    let source = r#"
        const point: readonly [i32, i32] = [1, 2];
        const frozen: readonly i32[] = point;
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn rejects_filling_a_mutable_array_from_a_readonly_tuple() {
    let source = r#"
        const point: readonly [i32, i32] = [1, 2];
        let mutable: i32[] = point;
    "#;
    assert_error(source, "E0308");
}

#[test]
pub(super) fn rejects_filling_an_array_from_a_tuple_whose_positions_do_not_fit() {
    let source = r#"
        const pair: [i32, string] = [1, "a"];
        const numbers: i32[] = pair;
    "#;
    assert_error(source, "E0308");
}

#[test]
pub(super) fn transforming_methods_on_a_tuple_return_an_array() {
    let source = r#"
        const point: [i32, i32, i32] = [10, 20, 30];
        const part: i32[] = point.slice(1);
        const more: i32[] = point.concat([40]);
        const patched: i32[] = point.with(0, 99);
        const flipped: i32[] = point.toReversed();
        const ordered: i32[] = point.toSorted();
        const doubled: i32[] = point.map((value: i32): i32 => value * 2);
        const large: i32[] = point.filter((value: i32): boolean => value > 15);
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn exposes_the_array_surface_on_a_string_tuple() {
    let source = r#"
        const names: [string, string] = ["Ada", "Grace"];
        const joined: string = names.join(", ");
        const has: boolean = names.includes("Ada");
        const upper: string[] = names.map((name: string): string => name.toUpperCase());
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn rejects_sort_on_a_readonly_array_bound_mutably() {
    let source = r#"
        let ports: readonly i32[] = [3, 1, 2];
        ports.sort();
    "#;
    assert_error(source, "E2540");
}

#[test]
pub(super) fn accepts_typed_array_match_bindings_before_codegen() {
    let source = r#"
        const values: i32[] = [1, 2];
        match (values) {
            ([first, ...rest]) => {
                const second: i32 = rest[0];
                console.log(first + second);
            }
        }
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn rejects_array_match_patterns_for_non_array_subjects() {
    let source = r#"
        const value: i32 = 1;
        match (value) {
            [item] => { console.log(item); }
        }
    "#;
    assert_error(source, "E0529");
}

#[test]
pub(super) fn array_match_subject_diagnostic_anchors_the_entire_pattern() {
    let source = "const value: i32 = 1; match (value) { [] => { return; } }";
    let diagnostics = spanned_diagnostics(source);
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0529")
        .expect("expected array match subject diagnostic");
    assert_eq!(
        &source[diagnostic
            .span
            .expect("missing array pattern span")
            .to_range()],
        "[]"
    );
}

#[test]
pub(super) fn rejects_incompatible_array_match_literal() {
    let source = r#"
        const values: i32[] = [1, 2];
        match (values) {
            ["one"] => { return; }
        }
    "#;
    let diagnostics = spanned_diagnostics(source);
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0308")
        .expect("expected incompatible array element diagnostic");
    assert_eq!(
        &source[diagnostic
            .span
            .expect("missing literal diagnostic span")
            .to_range()],
        "\"one\""
    );
}

#[test]
pub(super) fn rejects_nested_array_match_patterns_until_they_have_safe_codegen() {
    let source =
        "const values: i32[][] = [[1]]; match (values) { [[value]] => { console.log(value); } }";
    let diagnostics = spanned_diagnostics(source);
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0658")
        .expect("expected nested array pattern diagnostic");
    assert_eq!(
        &source[diagnostic
            .span
            .expect("missing nested pattern span")
            .to_range()],
        "[value]"
    );
}

#[test]
pub(super) fn rejects_mutating_an_array_field_of_the_readonly_utility_type() {
    let source = r#"
        interface Cfg {
            tags: string[];
        }

        type Frozen = Readonly<Cfg>;

        let config: Frozen = { tags: ["a"] };
        config.tags.push("b");
    "#;
    assert_error(source, "E2540");
}

#[test]
pub(super) fn rejects_assigning_a_readonly_const_binding_to_a_mutable_array() {
    let source = r#"
        readonly const DAYS: i32[] = [1, 2, 3];
        let days: i32[] = DAYS;
    "#;
    assert_error(source, "E0308");
}

#[test]
pub(super) fn accepts_fixed_array_initializers_with_repeated_or_distinct_values() {
    let source = r#"
        const repeated: [i32; 5] = [0; 5];
        const values: [i32; 5] = [1, 2, 3, 4, 5];
        console.log(repeated[4]);
        console.log(values[4]);
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn rejects_fixed_array_initializers_with_the_wrong_length() {
    assert_error("const values: [i32; 5] = [1, 2, 3, 4];", "E0308");
    assert_error("const values: [i32; 5] = [0; 4];", "E0308");
}

#[test]
pub(super) fn fixed_array_length_mismatch_points_to_the_initializer_and_suggests_a_fix() {
    let source = "const values: [i32; 5] = [1, 2, 3, 4];";
    let diagnostic = spanned_diagnostics(source)
        .into_iter()
        .find(|diagnostic| diagnostic.code == "E0308")
        .expect("expected fixed-array length mismatch");
    let start = source.find("[1, 2, 3, 4]").expect("initializer exists");
    assert_eq!(diagnostic.span, Some(Span::new(start, source.len() - 1)));
    assert_eq!(
        diagnostic.message,
        "fixed-length type requires 5 elements, but this initializer contains 4"
    );
    assert_eq!(
        diagnostic.help,
        vec!["provide exactly 5 elements in this initializer"]
    );
}

#[test]
pub(super) fn fixed_array_repeat_length_mismatch_points_to_the_count() {
    let source = "const values: [i32; 5] = [0; 4];";
    let diagnostic = spanned_diagnostics(source)
        .into_iter()
        .find(|diagnostic| diagnostic.code == "E0308")
        .expect("expected fixed-array repeat mismatch");
    let start = source.rfind('4').expect("repeat count exists");
    assert_eq!(diagnostic.span, Some(Span::new(start, start + 1)));
    assert_eq!(diagnostic.help, vec!["change the repeat count to 5"]);
}

#[test]
pub(super) fn accepts_array_repeat_with_literal_count() {
    let source = r#"
        const main = (): void => {
            const zeros: i32[] = [0; 5];
            console.log(zeros.length);
        };
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_array_repeat_with_expression_value() {
    let source = r#"
        const main = (): void => {
            const xs: i32[] = [1 + 2; 3];
            console.log(xs[0]);
        };
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn rejects_array_repeat_with_negative_literal_count() {
    assert_error(
        r#"
            const main = (): void => {
                const xs: i32[] = [0; -1];
            };
        "#,
        "E0308",
    );
}

#[test]
pub(super) fn rejects_array_repeat_with_zero_or_excessive_count() {
    for count in ["0", "65537"] {
        assert_error(
            &format!("const main = (): void => {{ const xs: i32[] = [0; {count}]; }};"),
            "E0308",
        );
    }
}

#[test]
pub(super) fn accepts_array_repeat_with_constant_count_expression() {
    let source = r#"
        const main = (): void => {
            const xs: i32[] = [0; 1 + 2];
            console.log(xs.length);
        };
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn rejects_array_repeat_with_non_integer_count() {
    assert_error(
        r#"
            const main = (): void => {
                const xs: i32[] = [0; 3.14];
            };
        "#,
        "E0308",
    );
}

#[test]
pub(super) fn rejects_array_repeat_with_variable_count() {
    assert_error(
        r#"
            const main = (): void => {
                const n: i32 = 5;
                const xs: i32[] = [0; n];
            };
        "#,
        "E0308",
    );
}

#[test]
pub(super) fn rejects_array_repeat_with_string_value_and_int_array_type() {
    assert_error(
        r#"
            const main = (): void => {
                const xs: i32[] = ["hello"; 3];
            };
        "#,
        "E0308",
    );
}

#[test]
pub(super) fn accepts_comptime_array_repeat_with_literal() {
    let source = r#"
        const TABLE = comptime([0; 10]);
        const main = (): void => {
            console.log(TABLE.length);
        };
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_comptime_array_repeat_with_expression() {
    let source = r#"
        const TABLE = comptime([1 + 2; 5]);
        const main = (): void => {
            console.log(TABLE[0]);
        };
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_comptime_array_repeat_nested() {
    let source = r#"
        const MATRIX: i32[][] = comptime([[0; 3]; 3]);
        const main = (): void => {
            console.log(MATRIX.length);
        };
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_array_with_capacity_with_annotation() {
    let source = r#"
        const main = (): void => {
            const xs: i32[] = Array.withCapacity(10);
            console.log(xs.length);
        };
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn accepts_array_with_capacity_with_string_element() {
    let source = r#"
        const main = (): void => {
            const xs: string[] = Array.withCapacity(5);
            console.log(xs.length);
        };
    "#;
    assert_eq!(diagnostics(source), Vec::<CheckDiagnostic>::new());
}

#[test]
pub(super) fn rejects_array_with_capacity_with_non_usize_arg() {
    assert_error(
        r#"
            const main = (): void => {
                const xs: i32[] = Array.withCapacity("ten");
            };
        "#,
        "E0308",
    );
}

#[test]
pub(super) fn rejects_array_with_capacity_with_wrong_arg_count() {
    assert_error(
        r#"
            const main = (): void => {
                const xs: i32[] = Array.withCapacity();
            };
        "#,
        "E0061",
    );
}

#[test]
pub(super) fn rejects_a_constant_index_after_an_unconditional_array_reassignment() {
    let source = r#"
        const main = (): void => {
            const index = 1;
            let array = [1, 2, 3, 4, 5];
            if (index === 1) {
                array = [];
            }
            console.log(array[2]);
        };
    "#;
    let diagnostic = diagnostics(source)
        .into_iter()
        .find(|diagnostic| diagnostic.code == "E0606")
        .expect("the definitely empty array must reject index 2");
    assert_eq!(
        diagnostic.message,
        "array index 2 is out of bounds: the array is empty (length 0)"
    );
}

#[test]
pub(super) fn does_not_use_a_shadowed_constant_to_prove_an_array_reassignment() {
    let source = r#"
        const index = 1;
        const read = (index: i32): i32 => {
            let array = [1, 2, 3];
            if (index === 1) {
                array = [];
            }
            return array[2];
        };
    "#;
    assert!(
        !diagnostics(source)
            .iter()
            .any(|diagnostic| diagnostic.code == "E0606"),
        "a parameter may differ from the outer constant"
    );
}
