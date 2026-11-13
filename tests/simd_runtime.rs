use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

static PROJECT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct SimdProject {
    root: PathBuf,
    name: String,
}

impl SimdProject {
    fn new(source: &str) -> Self {
        let suffix = format!(
            "{}-{}-{}",
            std::process::id(),
            PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let name = format!("simd-runtime-{suffix}");
        let root = std::env::temp_dir().join(&name);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(
                "{{\"project\":{{\"name\":\"{name}\",\"version\":\"0.1.0\",\"authors\":[],\"description\":\"SIMD runtime test\"}},\"dependencies\":{{}}}}"
            ),
        )
        .unwrap();
        fs::write(root.join("src/main.tyr"), source).unwrap();
        Self { root, name }
    }

    fn build(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_tyrus"))
            .arg("build")
            .current_dir(&self.root)
            .output()
            .unwrap()
    }

    fn build_and_run(&self) -> Output {
        let build = self.build();
        assert!(
            build.status.success(),
            "build failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
        Command::new(self.root.join("target/debug").join(&self.name))
            .output()
            .unwrap()
    }
}

impl Drop for SimdProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn integer_add_supports_signed_unsigned_lanes_and_scalar_tails() {
    let project = SimdProject::new(
        r#"
import { simdAdd, SimdError } from "tyrus:simd";

const main = (): void => {
    const i8Left: i8[] = [1; 17];
    const i8Right: i8[] = [2; 17];
    const i8Result: Result<i8[], SimdError> = simdAdd(i8Left, i8Right);
    match (i8Result) { Ok(values) => { print(values[16]); } Err(error) => { print(error.code); } }

    const i16Left: i16[] = [1; 9];
    const i16Right: i16[] = [2; 9];
    const i16Result: Result<i16[], SimdError> = simdAdd(i16Left, i16Right);
    match (i16Result) { Ok(values) => { print(values[8]); } Err(error) => { print(error.code); } }

    const i32Left: i32[] = [1; 5];
    const i32Right: i32[] = [2; 5];
    const i32Result: Result<i32[], SimdError> = simdAdd(i32Left, i32Right);
    match (i32Result) { Ok(values) => { print(values[4]); } Err(error) => { print(error.code); } }

    const i64Left: i64[] = [1; 3];
    const i64Right: i64[] = [2; 3];
    const i64Result: Result<i64[], SimdError> = simdAdd(i64Left, i64Right);
    match (i64Result) { Ok(values) => { print(values[2]); } Err(error) => { print(error.code); } }

    const u8Left: u8[] = [255; 17];
    const u8Right: u8[] = [1; 17];
    const u8Result: Result<u8[], SimdError> = simdAdd(u8Left, u8Right);
    match (u8Result) { Ok(values) => { print(values[16]); } Err(error) => { print(error.code); } }

    const u16Left: u16[] = [65535; 9];
    const u16Right: u16[] = [1; 9];
    const u16Result: Result<u16[], SimdError> = simdAdd(u16Left, u16Right);
    match (u16Result) { Ok(values) => { print(values[8]); } Err(error) => { print(error.code); } }

    const u32Left: u32[] = [4294967295; 5];
    const u32Right: u32[] = [1; 5];
    const u32Result: Result<u32[], SimdError> = simdAdd(u32Left, u32Right);
    match (u32Result) { Ok(values) => { print(values[4]); } Err(error) => { print(error.code); } }

    const u64Left: u64[] = [18446744073709551615; 3];
    const u64Right: u64[] = [1; 3];
    const u64Result: Result<u64[], SimdError> = simdAdd(u64Left, u64Right);
    match (u64Result) { Ok(values) => { print(values[2]); } Err(error) => { print(error.code); } }
};
"#,
    );
    let output = project.build_and_run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "3\n3\n3\n3\n0\n0\n0\n0\n"
    );

    let negative = SimdProject::new(
        r#"
import { simdAdd, SimdError } from "tyrus:simd";

const main = (): void => {
    const left: i8[] = [-100; 16];
    const right: i8[] = [-20; 16];
    const result: Result<i8[], SimdError> = simdAdd(left, right);
    match (result) { Ok(values) => { print(values[0]); } Err(error) => { print(error.code); } }
};
"#,
    );
    let output = negative.build_and_run();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "-120\n");
}

#[test]
fn signed_integer_simd_add_wraps_in_vector_lanes_and_scalar_tail() {
    for (left, right, index, expected) in [
        ("[120; 17]", "[120; 17]", 0, "-16\n"),
        ("[120; 17]", "[120; 17]", 16, "-16\n"),
        ("[120; 1]", "[120; 1]", 0, "-16\n"),
        ("[127; 17]", "[1; 17]", 16, "-128\n"),
    ] {
        let source = format!(
            r#"
import {{ simdAdd, SimdError }} from "tyrus:simd";

const main = (): void => {{
    const left: i8[] = {left};
    const right: i8[] = {right};
    const result: Result<i8[], SimdError> = simdAdd(left, right);
    match (result) {{ Ok(values) => {{ print(values[{index}]); }} Err(error) => {{ printError(error.code, error.message); }} }}
}};
"#
        );
        let output = SimdProject::new(&source).build_and_run();
        assert!(
            output.status.success(),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
    }
}

#[test]
fn fixed_size_arrays_convert_to_dynamic_arrays() {
    let project = SimdProject::new(
        r#"
import { simdAdd, SimdError } from "tyrus:simd";

const main = (): void => {
    const fixed: [i32; 5] = [1, 2, 3, 4, 5];
    const otherFixed: [i32; 5] = [10, 20, 30, 40, 50];
    const dynamic: i32[] = fixed;
    const otherDynamic: i32[] = otherFixed;
    print(dynamic);
    const result: Result<i32[], SimdError> = simdAdd(dynamic, otherDynamic);
    match (result) { Ok(values) => { print(values); } Err(error) => { printError(error.code, error.message); } }
};
"#,
    );
    let output = project.build_and_run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[1, 2, 3, 4, 5]\n[11, 22, 33, 44, 55]\n"
    );
}

#[test]
fn fixed_size_arrays_work_as_direct_simd_arguments() {
    let project = SimdProject::new(
        r#"
import { simdAdd, SimdError } from "tyrus:simd";

const main = (): void => {
    const left: [i32; 5] = [1, 2, 3, 4, 5];
    const right: [i32; 5] = [10, 20, 30, 40, 50];
    const result: Result<i32[], SimdError> = simdAdd(left, right);
    match (result) { Ok(values) => { print(values); } Err(error) => { printError(error.code, error.message); } }
    const explicit: Result<i32[], SimdError> = simdAdd<i32>(left, right);
    match (explicit) { Ok(values) => { print(values); } Err(error) => { printError(error.code, error.message); } }
};
"#,
    );
    let output = project.build_and_run();
    assert!(
        output.status.success(),
        "program failed with status {:?}:\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[11, 22, 33, 44, 55]\n[11, 22, 33, 44, 55]\n"
    );
}

#[test]
fn fixed_size_simd_arrays_with_different_lengths_fail_at_compile_time() {
    let source = r#"
import { simdAdd, SimdError } from "tyrus:simd";

const main = (): void => {
    const left: [i32; 5] = [1, 2, 3, 4, 5];
    const right: [i32; 4] = [10, 20, 30, 40];
    const result: Result<i32[], SimdError> = simdAdd(left, right);
};
"#;
    let build = SimdProject::new(source).build();
    assert!(!build.status.success(), "different fixed lengths must fail");
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stderr.contains("error[E0308]"),
        "unexpected diagnostic: {stderr}"
    );
    assert!(
        stderr.contains(
            "argument 2 of `simdAdd` has fixed length 4, but argument 1 has fixed length 5"
        ),
        "unexpected diagnostic: {stderr}"
    );
}

#[test]
fn simd_length_mismatch_returns_error_and_empty_arrays_succeed() {
    let project = SimdProject::new(
        r#"
import { simdAdd, SimdError } from "tyrus:simd";

const main = (): void => {
    const emptyLeft: i32[] = [];
    const emptyRight: i32[] = [];
    const emptyResult: Result<i32[], SimdError> = simdAdd(emptyLeft, emptyRight);
    match (emptyResult) { Ok(values) => { print(values); } Err(error) => { printError(error.code, error.message); } }

    const left: i32[] = [1; 2];
    const right: i32[] = [3; 3];
    const mismatch: Result<i32[], SimdError> = simdAdd(left, right);
    match (mismatch) { Ok(values) => { print(values); } Err(error) => { print(error.code); } }
};
"#,
    );
    let output = project.build_and_run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[]\nERR_SIMD_LENGTH_MISMATCH\n"
    );
}

#[test]
fn simd_full_vector_blocks_write_contiguous_results() {
    let project = SimdProject::new(
        r#"
import { simdAdd, SimdError } from "tyrus:simd";

const main = (): void => {
    const left: i32[] = [1; 8];
    const right: i32[] = [2; 8];
    const result: Result<i32[], SimdError> = simdAdd(left, right);
    match (result) { Ok(values) => { print(values); } Err(error) => { printError(error.code, error.message); } }
};
"#,
    );
    let output = project.build_and_run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[3, 3, 3, 3, 3, 3, 3, 3]\n"
    );
}

#[test]
fn simd_example_fixture_runs_and_prints_expected_integer_values() {
    let example_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/simd/examples.tyr");
    let mut source = fs::read_to_string(example_path).unwrap();
    source.push_str("\nconst main = (): void => { runSimdArrayAddExamples(); };\n");
    let project = SimdProject::new(&source);
    let output = project.build_and_run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout,
        "11.0\n22.0\n33.0\n44.0\n55.0\n11\n55\n0\n[-16, -16, -16, -16, -16]\n[17, 26, 35, 44, 53]\n[6.0, 12.0, 20.0, 30.0, 42.0]\n[11, 22, 33, 44, 55]\n[-9, -18, -27, -36, -45]\n[10, 40, 90, 160, 250]\n[10, 9, 9, 11, 7]\n"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERR_SIMD_LENGTH_MISMATCH"));
}

#[test]
fn simd_integer_division_vectorizes_and_checks_all_lanes_before_dividing() {
    let project = SimdProject::new(
        r#"
import { simdDiv, SimdError } from "tyrus:simd";

const main = (): void => {
    const left: i32[] = [20, 40, 60, 80, 100];
    const right: i32[] = [2, 4, 5, 8, 10];
    const result: Result<i32[], SimdError> = simdDiv(left, right);
    match (result) { Ok(values) => { print(values); } Err(error) => { printError(error.code, error.message); } }

    const fixedLeft: [i32; 5] = [100, 81, 72, 55, 42];
    const fixedRight: [i32; 5] = [10, 9, 8, 5, 6];
    const fixedResult: Result<i32[], SimdError> = simdDiv(fixedLeft, fixedRight);
    match (fixedResult) { Ok(values) => { print(values); } Err(error) => { printError(error.code, error.message); } }

    const zeroInVector: Result<i32[], SimdError> = simdDiv([8; 5], [2, 2, 0, 2, 2]);
    match (zeroInVector) { Ok(values) => { print(values); } Err(error) => { print(error.code); } }

    const zeroInTail: Result<i32[], SimdError> = simdDiv([8; 5], [2, 2, 2, 2, 0]);
    match (zeroInTail) { Ok(values) => { print(values); } Err(error) => { print(error.code); } }

    const unsignedLeft: u8[] = [255; 5];
    const unsignedRight: u8[] = [5; 5];
    const unsigned: Result<u8[], SimdError> = simdDiv(unsignedLeft, unsignedRight);
    match (unsigned) { Ok(values) => { print(values); } Err(error) => { printError(error.code, error.message); } }

    const overflowLeft: i8[] = [-128; 1];
    const overflowRight: i8[] = [-1; 1];
    const overflow: Result<i8[], SimdError> = simdDiv(overflowLeft, overflowRight);
    match (overflow) { Ok(values) => { print(values); } Err(error) => { print(error.code); } }

    const floatLeft: f32[] = [1.0, -1.0, 0.0];
    const floatRight: f32[] = [0.0; 3];
    const floats: Result<f32[], SimdError> = simdDiv(floatLeft, floatRight);
    match (floats) { Ok(values) => { print(values[0] > 100.0); print(values[1] < -100.0); print(values[2] === values[2]); } Err(error) => { printError(error.code, error.message); } }
};
"#,
    );
    let output = project.build_and_run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[10, 10, 12, 10, 10]\n[10, 9, 9, 11, 7]\nERR_SIMD_DIVISION_BY_ZERO\nERR_SIMD_DIVISION_BY_ZERO\n[51, 51, 51, 51, 51]\nERR_SIMD_INTEGER_OVERFLOW\ntrue\ntrue\nfalse\n"
    );
}

#[test]
fn simd_subtraction_and_multiplication_support_numeric_vectors_and_tails() {
    let project = SimdProject::new(
        r#"
import { simdSub, simdMul, SimdError } from "tyrus:simd";

const main = (): void => {
    const i32Sub: Result<i32[], SimdError> = simdSub([20; 5], [3; 5]);
    const i32Mul: Result<i32[], SimdError> = simdMul([3; 5], [4; 5]);
    const f32Left: f32[] = [6.5; 5];
    const f32Right: f32[] = [1.5; 5];
    const f32MulLeft: f32[] = [2.0; 5];
    const f32MulRight: f32[] = [3.0; 5];
    const u8Left: u8[] = [0; 17];
    const u8Right: u8[] = [1; 17];
    const u8MulLeft: u8[] = [255; 17];
    const u8MulRight: u8[] = [2; 17];
    const i64Left: i64[] = [4; 3];
    const i64Right: i64[] = [5; 3];
    const f32Sub: Result<f32[], SimdError> = simdSub(f32Left, f32Right);
    const f32Mul: Result<f32[], SimdError> = simdMul(f32MulLeft, f32MulRight);
    const u8Sub: Result<u8[], SimdError> = simdSub(u8Left, u8Right);
    const u8Mul: Result<u8[], SimdError> = simdMul(u8MulLeft, u8MulRight);
    const i64Mul: Result<i64[], SimdError> = simdMul(i64Left, i64Right);
    match (i32Sub) { Ok(values) => { print(values[4]); } Err(error) => { printError(error.code, error.message); } }
    match (i32Mul) { Ok(values) => { print(values[4]); } Err(error) => { printError(error.code, error.message); } }
    match (f32Sub) { Ok(values) => { print(values[4]); } Err(error) => { printError(error.code, error.message); } }
    match (f32Mul) { Ok(values) => { print(values[4]); } Err(error) => { printError(error.code, error.message); } }
    match (u8Sub) { Ok(values) => { print(values[16]); } Err(error) => { printError(error.code, error.message); } }
    match (u8Mul) { Ok(values) => { print(values[16]); } Err(error) => { printError(error.code, error.message); } }
    match (i64Mul) { Ok(values) => { print(values[2]); } Err(error) => { printError(error.code, error.message); } }
};
"#,
    );
    let output = project.build_and_run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "17\n12\n5.0\n6.0\n255\n254\n20\n"
    );
}

#[test]
fn signed_integer_simd_sub_and_mul_wrap_in_vector_lanes_and_scalar_tail() {
    for (operation, left, right, index, expected) in [
        ("simdSub", "[-128; 17]", "[1; 17]", 0, "127\n"),
        ("simdSub", "[-128; 17]", "[1; 17]", 16, "127\n"),
        ("simdSub", "[-128; 1]", "[1; 1]", 0, "127\n"),
        ("simdMul", "[50; 17]", "[3; 17]", 0, "-106\n"),
        ("simdMul", "[50; 17]", "[3; 17]", 16, "-106\n"),
        ("simdMul", "[50; 1]", "[3; 1]", 0, "-106\n"),
    ] {
        let source = format!(
            r#"
import {{ simdSub, simdMul, SimdError }} from "tyrus:simd";

const main = (): void => {{
    const left: i8[] = {left};
    const right: i8[] = {right};
    const result: Result<i8[], SimdError> = {operation}(left, right);
    match (result) {{ Ok(values) => {{ print(values[{index}]); }} Err(error) => {{ printError(error.code, error.message); }} }}
}};
"#
        );
        let output = SimdProject::new(&source).build_and_run();
        assert!(
            output.status.success(),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
    }
}
