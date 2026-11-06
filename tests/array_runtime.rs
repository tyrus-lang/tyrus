use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Tells this binary's scratch projects apart. A name built only from a
/// timestamp collided whenever two tests reached it inside the same tick, and
/// since `Drop` removes the whole directory, the loser's binary was gone by the
/// time it tried to run: `NotFound` from a build that had just succeeded. The
/// process id and this counter make the name unique by construction, and the
/// timestamp stays to keep a directory left behind by a crashed run, under a
/// process id the system later reused, from being adopted as if it were ours.
static PROJECT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct ProjectRoot(PathBuf);

impl Deref for ProjectRoot {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<Path> for ProjectRoot {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Drop for ProjectRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn project_root(name: &str) -> ProjectRoot {
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let root = ProjectRoot(std::env::temp_dir().join(format!("tyrus-{name}-{suffix}")));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("tyrus.json"),
        format!(
            r#"{{
  "project": {{
    "name": "{name}",
    "version": "0.1.0",
    "authors": [],
    "description": "array element ownership runtime test"
  }},
  "dependencies": {{}}
}}
"#
        ),
    )
    .unwrap();
    root
}

fn build_and_run(root: &std::path::Path, binary_name: &str) -> std::process::Output {
    let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    Command::new(root.join("target/debug").join(binary_name))
        .output()
        .unwrap()
}

fn run_program(name: &str, source: &str) -> String {
    let root = project_root(name);
    fs::write(root.join("src/main.tyr"), source).unwrap();
    let output = build_and_run(&root, name);
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// The reported crash: pushing a freshly computed string in a loop and reading
/// the elements back. `push` used to copy the argument's raw pointer bytes into
/// the array without taking ownership, so the temporary was released as an
/// ordinary statement temporary right after the call and every element became a
/// dangling pointer into recycled heap memory, printing punctuation from
/// whatever the allocator had written into the freed blocks, and segfaulting in
/// `strlen` under the HTTP dispatcher.
#[test]
fn push_of_a_computed_string_in_a_loop_keeps_every_element_readable() {
    let stdout = run_program(
        "array-push-computed-loop",
        r#"const main = (): void => {
    let parts: string[] = [];
    let i: i32 = 0;
    while (i < 5) {
        parts.push("item-" + i.toString());
        i = i + 1;
    }
    console.log(parts.join(","));
};
"#,
    );
    assert_eq!(stdout, "item-0,item-1,item-2,item-3,item-4\n");
}

/// The same ownership transfer, through a method call rather than a
/// concatenation, and read back by index instead of `join`, neither the loop
/// nor `join` was load-bearing for the crash.
#[test]
fn push_of_a_method_result_survives_the_statement_that_pushed_it() {
    let stdout = run_program(
        "array-push-method-result",
        r#"const main = (): void => {
    let parts: string[] = [];
    parts.push("last".toUpperCase());
    console.log(parts[0]);
};
"#,
    );
    assert_eq!(stdout, "LAST\n");
}

/// `unshift` shares `push`'s emission path and shared the same defect.
#[test]
fn unshift_of_a_computed_string_keeps_the_element_readable() {
    let stdout = run_program(
        "array-unshift-computed",
        r#"const main = (): void => {
    let parts: string[] = ["tail"];
    parts.unshift("item-" + 1.toString());
    console.log(parts.join(","));
};
"#,
    );
    assert_eq!(stdout, "item-1,tail\n");
}

/// An array reached through an object field takes a different receiver path in
/// the codegen (`emit_place_ptr`) than a bare identifier, and was equally
/// affected.
#[test]
fn push_of_a_computed_string_into_an_object_field_array() {
    let stdout = run_program(
        "array-push-object-field",
        r#"interface Bucket {
    labels: string[];
}

const main = (): void => {
    let bucket: Bucket = { labels: [] };
    bucket.labels.push("key".toUpperCase());
    console.log(bucket.labels[0]);
};
"#,
    );
    assert_eq!(stdout, "KEY\n");
}

/// Static string literals were never the broken case, they point at `.rodata`
/// and stayed valid, so this guards the fix against regressing them while the
/// array now stores a clone of each pushed element.
#[test]
fn push_of_static_string_literals_still_reads_back() {
    let stdout = run_program(
        "array-push-literals",
        r#"const main = (): void => {
    let parts: string[] = [];
    parts.push("alpha");
    parts.push("beta");
    console.log(parts.join(","));
};
"#,
    );
    assert_eq!(stdout, "alpha,beta\n");
}

/// Pushing a named binding must clone rather than alias it: the array's element
/// and the original binding are independent afterwards, so releasing the
/// binding at scope exit cannot leave the array dangling and cannot double-free.
#[test]
fn push_of_a_named_binding_clones_instead_of_aliasing_it() {
    let stdout = run_program(
        "array-push-named-binding",
        r#"const main = (): void => {
    let parts: string[] = [];
    const label: string = "item-" + 1.toString();
    parts.push(label);
    console.log(parts[0]);
    console.log(label);
};
"#,
    );
    assert_eq!(stdout, "item-1\nitem-1\n");
}

/// Array match arms must distinguish exact lengths from a final rest binding,
/// evaluate guards only after their bindings exist, and retain independently
/// owned rest elements after the source array leaves the arm's scope.
#[test]
fn array_match_patterns_run_with_guards_and_owned_rest_bindings() {
    let stdout = run_program(
        "array-match-patterns",
        r#"function describe(values: i32[]): void {
    match (values) {
        [] => { console.log("empty"); }
        [only] => { console.log("one", only); }
        [first, second] => { console.log("pair", first + second); }
        [first, ...rest] if (first > 0 && rest.length === 2) => {
            console.log("many", first, rest[0], rest[1]);
        }
        _ => { console.log("other"); }
    }
}

function main(): void {
    describe([]);
    describe([7]);
    describe([2, 3]);
    describe([4, 5, 6]);
    describe([-1, 5, 6]);

    const words: string[] = ["go", "north", "fast"];
    match (words) {
        ["go", direction, ...rest] if (rest.length === 1) => {
            console.log(direction);
            console.log(rest[0]);
        }
        _ => { console.log("wrong direction"); }
    }

    const nested: i32[][] = [[1], [2], [3]];
    match (nested) {
        [head, ...tail] => {
            console.log(head[0], tail[0][0], tail[1][0]);
        }
        _ => { console.log("missing nested values"); }
    }
}
"#,
    );
    assert_eq!(
        stdout,
        "empty\none 7\npair 5\nmany 4 5 6\nother\nnorth\nfast\n1 2 3\n"
    );
}

/// `windows` used to reserve capacity on a freshly `alloca`ed window before
/// filling it, so `tyrus_array_reserve` read whatever the stack happened to
/// hold and called `realloc` on a garbage pointer. The IR verified and the
/// markers were all in place, only running it showed the crash.
#[test]
fn windows_produces_every_window_without_reading_uninitialized_memory() {
    let root = project_root("array-windows");
    fs::write(
        root.join("src/main.tyr"),
        r#"const main = (): void => {
    const readings: i32[] = [2, 4, 6, 8];
    const threes: i32[][] = readings.windows(3);
    const ones: i32[][] = readings.windows(1);
    const whole: i32[][] = readings.windows(4);
    console.log(threes);
    console.log(ones);
    console.log(whole);
    const words: string[] = ["a", "b", "c"];
    const pairs: string[][] = words.windows(2);
    console.log(pairs);
};
"#,
    )
    .unwrap();

    let output = build_and_run(&root, "array-windows");
    assert!(
        output.status.success(),
        "program failed: {:?}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[[2, 4, 6], [4, 6, 8]]\n[[2], [4], [6], [8]]\n[[2, 4, 6, 8]]\n[[\"a\", \"b\"], [\"b\", \"c\"]]\n"
    );
    let _ = fs::remove_dir_all(root);
}

/// The window slot is reused across iterations rather than `alloca`ed inside
/// the loop, so the stack stays flat however many windows the input produces.
#[test]
fn windows_over_a_long_array_keeps_the_stack_flat() {
    let root = project_root("array-windows-long");
    fs::write(
        root.join("src/main.tyr"),
        r#"const main = (): void => {
    let readings: i32[] = [];
    for (let i = 0; i < 300000; i++) {
        readings.push(i);
    }
    const windowed: i32[][] = readings.windows(2);
    console.log(windowed.length);
    console.log(windowed[299998][1]);
};
"#,
    )
    .unwrap();

    let output = build_and_run(&root, "array-windows-long");
    assert!(
        output.status.success(),
        "program failed: {:?}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "299999\n299999\n"
    );
    let _ = fs::remove_dir_all(root);
}

/// Every index in the string API counts characters. `.length` used to report
/// bytes (`strlen`), slicing across a multi-byte character panicked in the
/// runtime, a one-argument `slice` dropped the last character, and a negative
/// index was zero-extended into a huge positive one.
#[test]
fn string_indices_count_characters_end_to_end() {
    let root = project_root("string-char-indices");
    fs::write(
        root.join("src/main.tyr"),
        r#"const main = (): void => {
    const accented: string = "héllo";
    const launch: string = "a🚀b";
    console.log(accented.length);
    console.log(launch.length);
    console.log(accented.charAt(1));
    console.log(launch.charAt(1));
    console.log(accented.slice(0, 2));
    console.log(accented.slice(1));
    console.log(accented.slice(-2));
    console.log(accented.substring(1, 3));
    console.log(accented.indexOf("l"));
    console.log(accented.lastIndexOf("l"));
    console.log(accented.padStart(7, "."));
    console.log(launch.padEnd(5, "."));
};
"#,
    )
    .unwrap();

    let output = build_and_run(&root, "string-char-indices");
    assert!(
        output.status.success(),
        "program failed: {:?}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "5\n3\né\n🚀\nhé\néllo\nlo\nél\n2\n3\n..héllo\na🚀b..\n"
    );
    let _ = fs::remove_dir_all(root);
}

/// A byte copy of an array whose elements own memory left the source and the
/// copy sharing one allocation, and both freed it at scope exit: printing the
/// result aborted with `free(): double free detected`.
#[test]
fn copying_methods_give_owning_elements_their_own_allocation() {
    let root = project_root("array-owning-copies");
    fs::write(
        root.join("src/main.tyr"),
        r#"interface Item { name: string; }

const main = (): void => {
    const nested: i32[][] = [[1, 2], [3, 4]];
    console.log(nested.toReversed());
    console.log(nested.slice(0, 1));
    console.log(nested.concat(nested));
    console.log(nested.with(0, [9, 9]));
    console.log(nested.values());
    console.log(nested);

    const items: Item[] = [{ name: "a" }, { name: "b" }];
    const reversed: Item[] = items.toReversed();
    console.log(reversed[0].name);
    console.log(items[0].name);
};
"#,
    )
    .unwrap();

    let output = build_and_run(&root, "array-owning-copies");
    assert!(
        output.status.success(),
        "program failed: {:?}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[[3, 4], [1, 2]]\n[[1, 2]]\n[[1, 2], [3, 4], [1, 2], [3, 4]]\n[[9, 9], [3, 4]]\n\
         [[1, 2], [3, 4]]\n[[1, 2], [3, 4]]\nb\na\n"
    );
    let _ = fs::remove_dir_all(root);
}

/// `console.log` dispatches on the argument's inferred type. The functional
/// array methods were missing from that table, so their results were typed
/// `Unknown` and printed as nothing at all.
#[test]
fn functional_method_results_print_without_being_bound_first() {
    let root = project_root("array-temporary-print");
    fs::write(
        root.join("src/main.tyr"),
        r#"const main = (): void => {
    const values: i32[] = [1, 2, 3, 4];
    const nested: i32[][] = [[1, 2], [3, 4]];
    console.log(values.windows(3));
    console.log(values.partition(value % 2 === 0));
    console.log(values.scan(0, (sum, value) => sum + value));
    console.log(values.unique());
    console.log(values.cartesianProduct([10, 20]));
    console.log(nested.transpose());
    console.log(values.first() ?? 0);
    console.log(values.last() ?? 0);
    console.log(values.avg() ?? 0.0);
};
"#,
    )
    .unwrap();

    let output = build_and_run(&root, "array-temporary-print");
    assert!(
        output.status.success(),
        "program failed: {:?}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[[1, 2, 3], [2, 3, 4]]\n[[2, 4], [1, 3]]\n[0, 1, 3, 6, 10]\n[1, 2, 3, 4]\n\
         [[1, 10], [1, 20], [2, 10], [2, 20], [3, 10], [3, 20], [4, 10], [4, 20]]\n\
         [[1, 3], [2, 4]]\n1\n4\n2.5\n"
    );
    let _ = fs::remove_dir_all(root);
}

/// A loop whose bounds check was removed still has to compute what it always
/// computed. The IR-level proof that the check is gone lives in the code
/// generator's own tests; this one is about the answer.
#[test]
fn a_counting_loop_reads_every_element_without_its_bounds_check() {
    let stdout = run_program(
        "bce-counting-loop",
        r#"function sweep(values: i32[]): i32 {
    let sum = 0;
    for (let i: usize = 0; i < values.length; i += 1) {
        sum += values[i];
    }
    return sum;
}

function doubled(): i32 {
    let values: i32[] = [1, 2, 3, 4];
    for (let i: usize = 0; i < values.length; i += 1) {
        values[i] = values[i] * 2;
    }
    let sum = 0;
    for (let i: usize = 0; i < values.length; i += 1) {
        sum += values[i];
    }
    return sum;
}

function main(): void {
    const values: i32[] = [1, 2, 3, 4];
    console.log(sweep(values));
    console.log(doubled());

    const empty: i32[] = [];
    console.log(sweep(empty));
}
"#,
    );
    assert_eq!(stdout, "10\n20\n0\n");
}

/// A fixed table read at literal slots, and swept by a loop, has to produce the
/// same numbers with its checks gone as it did with them. Both proofs are in
/// play here: the literal indexes and the counting loop over a global.
#[test]
fn a_fixed_table_reads_the_same_values_without_its_bounds_checks() {
    let stdout = run_program(
        "bce-fixed-table",
        r#"readonly const TABLE: i32[] = [10, 20, 30, 40];

function total(): i32 {
    let sum = 0;
    for (let i: usize = 0; i < TABLE.length; i += 1) {
        sum += TABLE[i];
    }
    return sum;
}

function main(): void {
    console.log(TABLE[0]);
    console.log(TABLE[3]);
    console.log(total());

    const local: i32[] = [7, 8, 9];
    console.log(local[2]);
}
"#,
    );
    assert_eq!(stdout, "10\n40\n100\n9\n");
}

/// The elimination is per access, not per program: an index nothing proves is
/// still checked, and still stops the program with the same message.
#[test]
fn an_index_the_loop_does_not_prove_still_reports_out_of_bounds() {
    let root = project_root("bce-unproved-index");
    fs::write(
        root.join("src/main.tyr"),
        r#"function sweep(values: i32[]): i32 {
    let sum = 0;
    for (let i: usize = 0; i < values.length; i += 1) {
        sum += values[i];
    }
    return sum;
}

function pick(values: i32[], slot: usize): i32 {
    return values[slot];
}

function main(): void {
    const values: i32[] = [1, 2, 3];
    console.log(sweep(values));
    console.log(pick(values, 9));
}
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "bce-unproved-index");
    assert!(
        !output.status.success(),
        "the bad index must stop the program"
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "6\n");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("array index out of bounds"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A loop that grows its own array keeps the check, and the check is what turns
/// a read past the end into a diagnostic instead of into whatever byte follows
/// the buffer.
#[test]
fn a_loop_that_shrinks_its_own_array_still_reports_out_of_bounds() {
    let root = project_root("bce-shrinking-array");
    fs::write(
        root.join("src/main.tyr"),
        r#"function drain(): i32 {
    let values: i32[] = [1, 2, 3, 4];
    let sum = 0;
    for (let i: usize = 0; i < values.length; i += 1) {
        sum += values[i];
        values.pop();
        values.pop();
        sum += values[i];
    }
    return sum;
}

function main(): void {
    console.log(drain());
}
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "bce-shrinking-array");
    assert!(
        !output.status.success(),
        "reading past the shrunk end must stop the program"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("array index out of bounds"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// String equality settles on the pointers when two values share a buffer, and
/// that fast path must not change a single answer. Assigning a `string` shares
/// its buffer rather than copying, and identical literals resolve to one
/// global, so the shortcut fires constantly, a wrong answer here would be a
/// silent, widespread miscompare rather than a crash.
///
/// The cases that matter are the ones where pointer identity and text equality
/// disagree in either direction: equal text over different buffers (built at
/// run time), unequal text of the same length, unequal text of different
/// lengths, and text carrying an interior NUL, which the length-delimited
/// representation stores as an ordinary character.
#[test]
fn string_equality_agrees_whether_or_not_the_buffers_are_shared() {
    let stdout = run_program(
        "string-equality-shared-buffer",
        r#"const main = (): void => {
    const original = "shared";
    const alias = original;
    console.log(alias === "shared");

    const built = "sha" + "red";
    console.log(built === "shared");

    console.log("abc" === "abd");
    console.log("abc" === "ab");
    console.log("" === "");

    const accented = "héllo";
    const accentedAlias = accented;
    console.log(accentedAlias === "héllo");

    const withNul = "a\u{0}b";
    console.log(withNul === "a\u{0}b");
    console.log(withNul === "a\u{0}c");
};
"#,
    );
    assert_eq!(
        stdout,
        "true\ntrue\nfalse\nfalse\ntrue\ntrue\ntrue\nfalse\n"
    );
}

#[test]
fn array_repeat_creates_correct_length_and_values() {
    let stdout = run_program(
        "array-repeat-basic",
        r#"const main = (): void => {
    const zeros: i32[] = [0; 5];
    console.log(zeros.length);
    console.log(zeros[0]);
    console.log(zeros[4]);

    const xs: i32[] = [1 + 2; 3];
    console.log(xs.length);
    console.log(xs[0]);
    console.log(xs[2]);

    const folded: i32[] = [9; 1 + 2];
    console.log(folded.length);
    console.log(folded[2]);
};
"#,
    );
    assert_eq!(stdout, "5\n0\n0\n3\n3\n3\n3\n9\n");
}

#[test]
fn array_repeat_with_string_elements() {
    let stdout = run_program(
        "array-repeat-strings",
        r#"const main = (): void => {
    const xs: string[] = ["hi"; 3];
    console.log(xs.length);
    console.log(xs[0]);
    console.log(xs[1]);
    console.log(xs[2]);
};
"#,
    );
    assert_eq!(stdout, "3\nhi\nhi\nhi\n");
}

#[test]
fn array_repeat_with_const_count() {
    let stdout = run_program(
        "array-repeat-const-count",
        r#"const main = (): void => {
    const xs: i32[] = [7; 4];
    console.log(xs.length);
    for (let i: usize = 0; i < xs.length; i += 1) {
        console.log(xs[i]);
    }
};
"#,
    );
    assert_eq!(stdout, "4\n7\n7\n7\n7\n");
}

#[test]
fn array_with_capacity_basic() {
    let stdout = run_program(
        "array-with-capacity-basic",
        r#"const main = (): void => {
    let xs: i32[] = Array.withCapacity(10);
    console.log(xs.length);
    console.log(xs.capacity);
    xs.push(42);
    xs.push(99);
    console.log(xs.length);
    console.log(xs.capacity);
    console.log(xs[0]);
    console.log(xs[1]);
};
"#,
    );
    assert_eq!(stdout, "0\n10\n2\n10\n42\n99\n");
}

#[test]
fn array_with_capacity_grows_after_its_initial_capacity_is_exhausted() {
    let stdout = run_program(
        "array-with-capacity-growth",
        r#"const main = (): void => {
    let xs: i32[] = Array.withCapacity(2);
    xs.push(10);
    xs.push(20);
    xs.push(30);
    console.log(xs.length);
    console.log(xs.capacity);
    console.log(xs[2]);
};
"#,
    );
    assert_eq!(stdout, "3\n8\n30\n");
}

#[test]
fn zeroize_releases_dynamic_values_and_reports_static_strings() {
    let stdout = run_program(
        "memory-zeroize-runtime",
        r#"import { MemoryError, zeroize } from "tyrus:memory";

const main = (): void => {
    let token: string = "runtime-token".slice(0);
    let bytes: u8[] = [7, 8, 9];
    const tokenResult: Result<void, MemoryError> = zeroize(token);
    match (tokenResult) {
        Ok() => {}
        Err(error) => { console.log(error.code); }
    }
    const bytesResult: Result<void, MemoryError> = zeroize(bytes);
    match (bytesResult) {
        Ok() => { console.log("erased"); }
        Err(error) => { console.log(error.code); }
    }

    const literal = "static-token";
    const literalResult: Result<void, MemoryError> = zeroize(literal);
    match (literalResult) {
        Ok() => {}
        Err(error) => { console.log(error.code); }
    }

};
"#,
    );
    assert_eq!(stdout, "erased\nERR_MEMORY_NOT_OWNED\n");
}

/// The fused and unfused forms must be the same program.
///
/// `array_literal_fusion` rewrites a straight run of `push` into the array
/// literal it is equivalent to, and the whole claim is that "equivalent" is
/// exact: same elements, same order, same length, and an array that still grows
/// afterwards. This runs both forms in one program and checks they agree.
#[test]
fn fusing_a_run_of_pushes_preserves_elements_order_and_later_growth() {
    let stdout = run_program(
        "array-push-fusion-equivalence",
        r#"const seed = (): i32 => {
    return 10;
};

const main = (): void => {
    // Fused: a straight run of pushes onto a fresh empty array.
    let fused: i32[] = [];
    fused.push(seed());
    fused.push(seed() + 1);
    fused.push(seed() + 2);

    // Not fused: the second element reads the array being built, so the run
    // stops at the first push.
    let selfReading: i32[] = [];
    selfReading.push(seed());
    selfReading.push(selfReading[0] + 1);
    selfReading.push(selfReading[1] + 1);

    // Fused, then grown further: the literal must still accept a push.
    let grown: i32[] = [];
    grown.push(1);
    grown.push(2);
    grown.push(3);
    grown.push(4);
    grown[0] = 9;

    console.log(`${fused[0]},${fused[1]},${fused[2]}`);
    console.log(fused.length);
    console.log(`${selfReading[0]},${selfReading[1]},${selfReading[2]}`);
    console.log(selfReading.length);
    console.log(`${grown[0]},${grown[1]},${grown[2]},${grown[3]}`);
    console.log(grown.length);
};
"#,
    );
    assert_eq!(
        stdout, "10,11,12\n3\n10,11,12\n3\n9,2,3,4\n4\n",
        "the fused and unfused forms disagree"
    );
}

/// Fusing must not smuggle an array onto a frame that dies before it does.
///
/// The rewrite exists so the escape analysis can put the array on the stack,
/// and the safety of that rests on the analysis still seeing every way the
/// array leaves. A returned array is the plainest of those: fused into a
/// literal it looks exactly like the fixed-size local the analysis promotes, so
/// this is the case where a promotion that ignored the `return` would hand the
/// caller a pointer into a frame that is already gone.
#[test]
fn a_fused_array_that_is_returned_still_outlives_the_frame_that_built_it() {
    let stdout = run_program(
        "array-push-fusion-escape",
        r#"const build = (seed: i32): i32[] => {
    let values: i32[] = [];
    values.push(seed);
    values.push(seed + 1);
    values.push(seed + 2);

    return values;
};

const main = (): void => {
    const first: i32[] = build(10);
    const second: i32[] = build(20);

    console.log(`${first[0]},${first[1]},${first[2]}`);
    console.log(`${second[0]},${second[1]},${second[2]}`);
    console.log(first.length + second.length);
};
"#,
    );
    assert_eq!(stdout, "10,11,12\n20,21,22\n6\n");
}

#[test]
fn encoding_array_literals_use_the_native_element_width() {
    let stdout = run_program(
        "encoding-literal-width",
        r#"import { EncodingError, hex, base64, base64Url, base32, utf8, utf16 } from "tyrus:encoding";
const main = (): void => {
    const bytes: u8[] = [0xde, 0xad, 0xbe, 0xef];
    console.log(hex.encode(bytes));
    console.log(hex.encode([0xde, 0xad, 0xbe, 0xef]));
    console.log(hex.encode([0xde, 0xad, 0xbe, 0xef] as u8[]));
    console.log(base64.encode([0xde, 0xad, 0xbe, 0xef]));
    console.log(base64Url.encode([0xde, 0xad, 0xbe, 0xef]));
    console.log(base32.encode([0xde, 0xad, 0xbe, 0xef]));
    const decodedUtf8: Result<string, EncodingError> = utf8.decode([65, 66, 67]);
    match (decodedUtf8) {
        Ok(value) => { console.log(value); }
        Err(error) => { console.log(error.code); }
    }
    const decodedUtf16: Result<string, EncodingError> = utf16.decode([65, 0x20ac, 66]);
    match (decodedUtf16) {
        Ok(value) => { console.log(value); }
        Err(error) => { console.log(error.code); }
    }
    console.log(hex.encode([]));
    console.log(hex.encode([0xde; 4]));
    const repeatedUtf16: Result<string, EncodingError> = utf16.decode([0x20ac; 2]);
    match (repeatedUtf16) {
        Ok(value) => { console.log(value); }
        Err(error) => { console.log(error.code); }
    }
};
"#,
    );
    assert_eq!(
        stdout,
        "deadbeef\ndeadbeef\ndeadbeef\n3q2+7w==\n3q2-7w\n32W353Y=\nABC\nA€B\n\ndededede\n€€\n"
    );
}

#[test]
fn asserted_array_literals_preserve_values_and_element_widths() {
    let stdout = run_program(
        "asserted-array-width",
        r#"const firstByte = (bytes: u8[]): u8 => bytes[1];
const main = (): void => {
    console.log([0xde, 0xad, 0xbe, 0xef] as u8[]);
    console.log([256, 65535] as u16[]);
    console.log([-127, 127] as i8[]);
    console.log([1.5, 2.5] as f32[]);
    console.log([[222, 173], [190, 239]] as u8[][]);
    console.log([] as u8[]);
    console.log(firstByte([222, 173]));
    console.log(firstByte([222, 173] as u8[]));
    console.log([222; 4] as u8[]);
    console.log([65535; 2] as u16[]);
};
"#,
    );
    assert_eq!(
        stdout,
        "[222, 173, 190, 239]\n[256, 65535]\n[-127, 127]\n[1.5, 2.5]\n[[222, 173], [190, 239]]\n[]\n173\n173\n[222, 222, 222, 222]\n[65535, 65535]\n"
    );
}

#[test]
fn crypto_array_literals_match_typed_byte_inputs() {
    let stdout = run_program(
        "crypto-literal-width",
        r#"import { CryptoError, digest, timingSafeEqual } from "tyrus:crypto";



import { hex } from "tyrus:encoding";
function cryptoBytes(result: Result<u8[], CryptoError>): u8[] {
    match (result) {
        Ok(value) => { return value; }
        Err(error) => { printError(error.code); return []; }
    }
}
const main = (): void => {
    const bytes: u8[] = [0xde, 0xad, 0xbe, 0xef];
    console.log(timingSafeEqual(bytes, [0xde, 0xad, 0xbe, 0xef]));
    console.log(timingSafeEqual(bytes, [0xde, 0xad, 0xbe, 0xef] as u8[]));
    console.log(hex.encode(cryptoBytes(digest.sha256([0xde, 0xad, 0xbe, 0xef]))) === hex.encode(cryptoBytes(digest.sha256(bytes))));
    console.log(timingSafeEqual([222; 2], [222, 222]));
    console.log(timingSafeEqual([], []));
};"#,
    );
    assert_eq!(stdout, "true\ntrue\ntrue\ntrue\ntrue\n");
}

#[test]
fn signed_minimum_literals_are_negated_before_array_element_narrowing() {
    let stdout = run_program(
        "signed-minimum-array",
        r#"type SignedByte = i8;
const acceptByte = (value: i8): i8 => value;
const returnByte = (): i8 => -128;
const main = (): void => {
    const bytes: i8[] = [-128, -127, 0, 127];
    const words: i16[] = [-32768, -32767, 32767];
    const integers: i32[] = [-2147483648, 2147483647];
    const longs: i64[] = [-9223372036854775808, 9223372036854775807];
    const huge: i128[] = [-170141183460469231731687303715884105728, 170141183460469231731687303715884105727];
    const alias: SignedByte[] = [-128, 127];
    const scalar: i8 = -128;
    console.log(bytes);
    console.log(words);
    console.log(integers);
    console.log(longs);
    console.log(huge);
    console.log(alias);
    console.log([-128, 127] as i8[]);
    console.log([-32768, 32767] as i16[]);
    console.log([-128; 3] as i8[]);
    console.log(scalar);
    console.log(acceptByte(-128));
    console.log(returnByte());
};
"#,
    );
    assert_eq!(
        stdout,
        "[-128, -127, 0, 127]\n[-32768, -32767, 32767]\n[-2147483648, 2147483647]\n[-9223372036854775808, 9223372036854775807]\n[-170141183460469231731687303715884105728, 170141183460469231731687303715884105727]\n[-128, 127]\n[-128, 127]\n[-32768, 32767]\n[-128, -128, -128]\n-128\n-128\n-128\n"
    );
}

#[test]
fn signed_array_element_narrowing_still_rejects_out_of_range_values() {
    for (name, value) in [
        ("signed-array-below-min", -129),
        ("signed-array-above-max", 128),
    ] {
        let root = project_root(name);
        fs::write(
            root.join("src/main.tyr"),
            format!(
                "const main = (): void => {{\n    const input: i32 = {value};\n    const bytes: i8[] = [input as i8];\n    console.log(bytes);\n}};\n"
            ),
        )
        .unwrap();
        let output = build_and_run(&root, name);
        assert!(
            !output.status.success(),
            "out-of-range value {value} was accepted"
        );
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("value out of range for `i8`"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn zip_pairs_equal_length_arrays_into_a_tuple_array() {
    let stdout = run_program(
        "array-zip-ear",
        r#"const main = (): void => {
    const names: string[] = ["ada", "bob"];
    const scores: i32[] = [10, 20];
    try {
        const pairs: [string, i32][] = names.zip(scores);
        console.log(pairs.length);
        console.log(pairs[0][0]);
        console.log(pairs[0][1]);
        console.log(pairs[1][0]);
        console.log(pairs[1][1]);
        console.log("after");
    } catch (error: ZipError) {
        console.log(error.code);
    }
};
"#,
    );
    assert_eq!(stdout, "2\nada\n10\nbob\n20\nafter\n");
}

#[test]
fn zip_of_mismatched_lengths_throws_a_recoverable_zip_error() {
    let stdout = run_program(
        "array-zip-mismatch",
        r#"const main = (): void => {
    const names: string[] = ["ada", "bob", "carl"];
    const scores: i32[] = [10, 20];
    try {
        const pairs: [string, i32][] = names.zip(scores);
        console.log(pairs.length);
    } catch (error: ZipError) {
        console.log(error.code);
        console.log(error.message);
        console.log(names.length);
        console.log(scores.length);
    }
    console.log("done");
};
"#,
    );
    assert_eq!(
        stdout,
        "ERR_ARRAY_ZIP_MISMATCH\nzip: arrays passed to zip must have the same length\n3\n2\ndone\n"
    );
}

/// `equals` compares two arrays by their elements: the same length, the same
/// elements, in the same order. Element equality is the one `includes` and
/// `indexOf` already use, so `-0.0` equals `0.0` and `NaN` equals nothing.
#[test]
fn array_equals_compares_elements_in_order() {
    let output = run_program(
        "array-equals",
        r#"const main = (): void => {
    const left: i32[] = [1, 2, 3];
    console.log(left.equals([1, 2, 3]));
    console.log(left.equals([3, 2, 1]));
    console.log(left.equals([1, 2]));
    const names: string[] = ["ada", "grace"];
    console.log(names.equals(["ada", "grace"]));
    console.log(names.equals(["ada", "hopper"]));
    const empty: i32[] = [];
    console.log(empty.equals([]));
    const zero: f64[] = [0.0];
    console.log(zero.equals([-0.0]));
    const nan: f64[] = [0.0 / 0.0];
    console.log(nan.equals(nan));
};
"#,
    );
    assert_eq!(
        output, "true\nfalse\nfalse\ntrue\nfalse\ntrue\ntrue\nfalse\n",
        "unexpected equality results"
    );
}
