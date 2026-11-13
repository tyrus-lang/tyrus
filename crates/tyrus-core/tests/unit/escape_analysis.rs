use super::*;
use crate::lexer;
use crate::parser::Parser;

fn contained_count(source: &str) -> usize {
    contained_names(source).len()
}

fn contained_names(source: &str) -> Vec<String> {
    let tokens = lexer::lex(source).expect("lexes");
    let program = Parser::new(tokens).parse_program().expect("parses");
    analyze(&program).names().to_vec()
}

fn confined_names(source: &str) -> Vec<String> {
    let tokens = lexer::lex(source).expect("lexes");
    let program = Parser::new(tokens).parse_program().expect("parses");
    let mut names = analyze_confined_closures(&program).names().to_vec();
    names.sort();
    names
}

#[test]
fn an_array_only_indexed_is_contained() {
    assert_eq!(
        contained_count(
            r#"
const main = (): void => {
    let total: i64 = 0;
    for (let i: i32 = 0; i < 10; i += 1) {
        const numbers: i32[] = [1, 2, 3];
        total += numbers[0] as i64;
    }
    console.log(total);
};
"#,
        ),
        1
    );
}

#[test]
fn indexing_and_length_together_are_contained() {
    assert_eq!(
        contained_count(
            r#"
const main = (): void => {
    const numbers: i32[] = [1, 2, 3];
    console.log(numbers[numbers.length - 1]);
};
"#,
        ),
        1
    );
}

#[test]
fn assigning_to_an_element_is_contained() {
    assert_eq!(
        contained_count(
            r#"
const main = (): void => {
    let numbers: i32[] = [1, 2, 3];
    numbers[0] = 9;
    console.log(numbers[0]);
};
"#,
        ),
        1
    );
}

#[test]
fn every_escaping_shape_is_refused() {
    let cases = [
        (
            "returned",
            r#"
const build = (): i32[] => {
    const numbers: i32[] = [1, 2, 3];
    return numbers;
};
"#,
        ),
        (
            "assigned to another name",
            r#"
const main = (): void => {
    const numbers: i32[] = [1, 2, 3];
    const alias: i32[] = numbers;
    console.log(alias[0]);
};
"#,
        ),
        (
            "passed to a function",
            r#"
const take = (values: i32[]): i32 => values[0];

const main = (): void => {
    const numbers: i32[] = [1, 2, 3];
    console.log(take(numbers));
};
"#,
        ),
        (
            "grown by push",
            r#"
const main = (): void => {
    let numbers: i32[] = [1, 2, 3];
    numbers.push(4);
    console.log(numbers[0]);
};
"#,
        ),
        (
            "any method call at all",
            r#"
const main = (): void => {
    const numbers: i32[] = [1, 2, 3];
    console.log(numbers.join(","));
};
"#,
        ),
        (
            "iterated, which hands the buffer to the loop",
            r#"
const main = (): void => {
    const numbers: i32[] = [1, 2, 3];
    for (const value of numbers) {
        console.log(value);
    }
};
"#,
        ),
        (
            "printed, which is a call",
            r#"
const main = (): void => {
    const numbers: i32[] = [1, 2, 3];
    console.log(numbers);
};
"#,
        ),
    ];
    for (label, source) in cases {
        assert_eq!(
            contained_count(source),
            0,
            "an array {label} must not be treated as contained"
        );
    }
}

#[test]
fn an_array_put_inside_another_is_refused() {
    let contained = contained_names(
        r#"
const main = (): void => {
    const numbers: i32[] = [1, 2, 3];
    const nested: i32[][] = [numbers];
    console.log(nested[0][0]);
};
"#,
    );
    assert!(
        !contained.iter().any(|name| name == "numbers"),
        "an array put inside another must not be contained, got {contained:?}"
    );
}

#[test]
fn a_spread_literal_is_refused() {
    assert_eq!(
        contained_count(
            r#"
const main = (): void => {
    const source: i32[] = [1, 2];
    const numbers: i32[] = [0, ...source];
    console.log(numbers[0]);
};
"#,
        ),
        0
    );
}

#[test]
fn two_declarations_of_one_name_are_judged_separately() {
    assert_eq!(
        contained_count(
            r#"
const build = (): i32[] => {
    const numbers: i32[] = [1, 2, 3];
    if (numbers[0] === 1) {
        const numbers: i32[] = [4, 5, 6];
        console.log(numbers[0]);
    }
    return numbers;
};
"#,
        ),
        1
    );
}

#[test]
fn a_captured_array_escapes_and_a_closure_local_does_not() {
    assert_eq!(
        contained_count(
            r#"
const main = (): void => {
    const captured: i32[] = [1, 2, 3];
    const read = (): i32 => captured[0];
    console.log(read());
};
"#,
        ),
        0,
        "an array a closure captures outlives the frame that declared it"
    );
    assert_eq!(
        contained_count(
            r#"
const main = (): void => {
    const read = (): i32 => {
        const inside: i32[] = [1, 2, 3];
        return inside[0];
    };
    console.log(read());
};
"#,
        ),
        1,
        "an array declared inside a closure is contained by that closure"
    );
}

/// An `async` function's locals live in a heap frame that outlives the stack
/// each poll runs on, so nothing declared in one can be contained, reserving
/// the slot on the stack and reading it back after a suspension was a
/// use-after-return that ended in SIGSEGV.
#[test]
fn an_array_in_an_async_function_is_never_contained() {
    assert_eq!(
        contained_count(
            r#"
async function main(): Promise<void> {
    const numbers: i32[] = [1, 2, 3];
    await sleepAsync(1);
    console.log(numbers[0]);
};
"#,
        ),
        0
    );
}

#[test]
fn an_array_in_an_async_arrow_is_never_contained() {
    assert_eq!(
        contained_count(
            r#"
const main = (): void => {
    const task = async (): Promise<i32> => {
        const numbers: i32[] = [1, 2, 3];
        return numbers[0];
    };
};
"#,
        ),
        0
    );
}

#[test]
fn an_array_in_a_closure_nested_in_an_async_function_is_not_contained() {
    assert_eq!(
        contained_count(
            r#"
async function main(): Promise<void> {
    const compute = (): i32 => {
        const numbers: i32[] = [1, 2, 3];
        return numbers[0];
    };
    console.log(compute());
};
"#,
        ),
        0
    );
}

#[test]
fn a_synchronous_function_beside_an_async_one_is_still_contained() {
    assert_eq!(
        contained_count(
            r#"
async function load(): Promise<i32> {
    const skipped: i32[] = [1, 2, 3];
    return skipped[0];
}

const main = (): void => {
    const numbers: i32[] = [1, 2, 3];
    console.log(numbers[0]);
};
"#,
        ),
        1
    );
}

#[test]
fn a_closure_that_is_only_called_is_confined() {
    assert_eq!(
        confined_names(
            r#"
function main(): void {
    let counter = 0;
    const bump = (): i32 => {
        counter = counter + 1;
        return counter;
    };
    console.log(bump());
}
"#,
        ),
        vec!["bump".to_string()]
    );
}

#[test]
fn a_returned_closure_is_not_confined() {
    assert!(
        confined_names(
            r#"
function make(): () => i32 {
    let n = 5;
    const f = (): i32 => n;
    return f;
}
"#,
        )
        .is_empty()
    );
}

#[test]
fn a_closure_passed_as_an_argument_is_not_confined() {
    assert!(
        confined_names(
            r#"
function run(f: () => i32): i32 {
    return f();
}
function main(): void {
    let n = 1;
    const f = (): i32 => n;
    console.log(run(f));
}
"#,
        )
        .is_empty()
    );
}

#[test]
fn an_aliased_closure_is_not_confined() {
    assert!(
        confined_names(
            r#"
function main(): void {
    let n = 1;
    const f = (): i32 => n;
    const g = f;
    console.log(g());
}
"#,
        )
        .is_empty()
    );
}

#[test]
fn an_async_closure_is_never_confined() {
    assert!(
        confined_names(
            r#"
async function main(): Promise<void> {
    let n = 1;
    const f = async (): Promise<i32> => n;
    console.log(await f());
}
"#,
        )
        .is_empty()
    );
}

#[test]
fn a_closure_captured_by_another_closure_is_not_confined() {
    assert_eq!(
        confined_names(
            r#"
function main(): void {
    let acc = 0;
    const inner = (): void => { acc = acc + 1; };
    const hold = (): void => { keep(inner); };
    hold();
}
function keep(f: () => void): void {
    f();
}
"#,
        ),
        vec!["hold".to_string()]
    );
}

#[test]
fn a_closure_nested_in_a_closure_is_judged_by_its_own_body() {
    assert_eq!(
        confined_names(
            r#"
function main(): void {
    const outer = (): i32 => {
        let inner_total = 0;
        const add = (): void => { inner_total = inner_total + 1; };
        add();
        return inner_total;
    };
    console.log(outer());
}
"#,
        ),
        vec!["add".to_string(), "outer".to_string()]
    );
}

#[test]
fn assigned_captures_names_the_writes_and_not_the_reads() {
    let tokens = lexer::lex(
        r#"
function main(): void {
    let counter = 0;
    let other = 0;
    const f = (n: i32): i32 => {
        let local = 1;
        local = local + 1;
        counter = counter + n;
        return counter + other + local;
    };
    console.log(f(1));
}
"#,
    )
    .expect("lexes");
    let program = Parser::new(tokens).parse_program().expect("parses");
    let mut found: Vec<String> = Vec::new();
    collect_arrow_assignments(&program, &mut found);
    assert_eq!(found, vec!["counter".to_string()]);
}

fn collect_arrow_assignments(program: &Program, found: &mut Vec<String>) {
    fn from_stmts(stmts: &[Stmt], found: &mut Vec<String>) {
        for stmt in stmts {
            if let StmtKind::VarDecl {
                value: Some(value), ..
            } = stmt.kind()
                && let ExprKind::ArrowFn { params, body, .. } = value.kind()
            {
                found.extend(
                    closure_assigned_captures(params, body)
                        .into_iter()
                        .map(|(name, _)| name),
                );
            }
            for nested in nested_bodies(stmt) {
                from_stmts(nested, found);
            }
        }
    }
    for item in &program.items {
        match item {
            Item::FnDecl { body, .. } => from_stmts(body, found),
            Item::Stmt(stmt) => from_stmts(std::slice::from_ref(stmt), found),
            _ => {}
        }
    }
}

/// Assigning to the binding replaces the buffer the declaration was judged on.
///
/// The walker never saw it: an assignment's target is a name, not an
/// expression, so no arm reached it. The declaration stayed contained, the
/// literal went on the stack, and the assignment's own heap buffer registered a
/// release against the slot, which on the path where the assignment never runs
/// hands a stack address to `free`. Reproduced as a SIGSEGV before the fix; the
/// case lives in `tests/leaks/clean/contained-array-replaced-by-assignment.tyr`.
#[test]
fn an_array_replaced_by_assignment_is_not_contained() {
    assert_eq!(
        contained_count(
            r#"
const main = (): void => {
    let numbers: i32[] = [1, 2, 3];
    console.log(numbers[0]);
    numbers = [4, 5, 6];
    console.log(numbers[0]);
};
"#,
        ),
        0
    );
}

#[test]
fn an_array_conditionally_replaced_by_assignment_is_not_contained() {
    assert_eq!(
        contained_count(
            r#"
const main = (): void => {
    let numbers: i32[] = [1, 2, 3];
    if (numbers.length > 100) {
        numbers = [4, 5, 6];
    }
    console.log(numbers[0]);
};
"#,
        ),
        0
    );
}

fn constant_global_names(source: &str) -> Vec<String> {
    let tokens = lexer::lex(source).expect("lexes");
    let program = Parser::new(tokens).parse_program().expect("parses");
    let mut names = analyze_constant_global_arrays(&program).names().to_vec();
    names.sort();
    names
}

#[test]
fn a_global_const_table_read_by_index_is_constant() {
    assert_eq!(
        constant_global_names(
            r#"
const PRIMES: i32[] = [2, 3, 5];
const NAMES: string[] = ["alpha", "beta"];
const main = (): void => {
    console.log(PRIMES[1]);
    console.log(PRIMES.length);
    console.log(NAMES[0]);
};
"#
        ),
        vec!["NAMES".to_string(), "PRIMES".to_string()]
    );
}

/// Moving the table into a mutable binding is the shape that must never be
/// promoted: `push` reallocates, and reallocating a pointer into read-only
/// memory is not a leak but a fault.
#[test]
fn a_global_moved_into_a_binding_is_not_constant() {
    assert!(
        constant_global_names(
            r#"
const PRIMES: i32[] = [2, 3, 5];
const main = (): void => {
    let xs: i32[] = PRIMES;
    xs.push(9);
    console.log(xs.length);
};
"#
        )
        .is_empty()
    );
}

#[test]
fn a_global_moved_inside_another_function_is_not_constant() {
    assert!(
        constant_global_names(
            r#"
const PRIMES: i32[] = [2, 3, 5];
function steal(): usize {
    let xs: i32[] = PRIMES;
    xs.push(9);
    return xs.length;
}
const main = (): void => {
    console.log(steal());
};
"#
        )
        .is_empty()
    );
}

#[test]
fn a_mutable_global_is_never_constant() {
    assert!(
        constant_global_names(
            r#"
let PRIMES: i32[] = [2, 3, 5];
const main = (): void => {
    PRIMES.push(9);
    console.log(PRIMES.length);
};
"#
        )
        .is_empty()
    );
}

#[test]
fn a_global_of_computed_elements_is_not_constant() {
    assert!(
        constant_global_names(
            r#"
const NAMES: string[] = ["a".toUpperCase(), "b"];
const main = (): void => {
    console.log(NAMES[0]);
};
"#
        )
        .is_empty()
    );
}

#[test]
fn a_readonly_global_const_table_is_still_constant() {
    assert_eq!(
        constant_global_names(
            r#"
readonly const PRIMES: i32[] = [2, 3, 5];
const NAMES: readonly string[] = ["alpha", "beta"];
const main = (): void => {
    console.log(PRIMES[1]);
    console.log(NAMES[0]);
};
"#
        ),
        vec!["NAMES".to_string(), "PRIMES".to_string()]
    );
}
