use super::*;

#[test]
fn emits_vector_add_with_scalar_tail_and_recoverable_errors() {
    let ir = compile_module_to_ir(
        r#"
        import { simdAdd, SimdError } from "tyrus:simd";

        function addValues(left: f32[], right: f32[]): Result<f32[], SimdError> {
            return simdAdd(left, right);
        }

        function addDoubleValues(left: f64[], right: f64[]): Result<f64[], SimdError> {
            return simdAdd(left, right);
        }

        function addI8(left: i8[], right: i8[]): Result<i8[], SimdError> { return simdAdd(left, right); }
        function addI16(left: i16[], right: i16[]): Result<i16[], SimdError> { return simdAdd(left, right); }
        function addI32(left: i32[], right: i32[]): Result<i32[], SimdError> { return simdAdd(left, right); }
        function addI64(left: i64[], right: i64[]): Result<i64[], SimdError> { return simdAdd(left, right); }
        function addU8(left: u8[], right: u8[]): Result<u8[], SimdError> { return simdAdd(left, right); }
        function addU16(left: u16[], right: u16[]): Result<u16[], SimdError> { return simdAdd(left, right); }
        function addU32(left: u32[], right: u32[]): Result<u32[], SimdError> { return simdAdd(left, right); }
        function addU64(left: u64[], right: u64[]): Result<u64[], SimdError> { return simdAdd(left, right); }
        "#,
        "simd-array-add",
    );

    assert!(ir.contains("fadd <4 x float>"));
    assert!(ir.contains("fadd <2 x double>"));
    assert!(ir.contains("add <16 x i8>"));
    assert!(ir.contains("add <8 x i16>"));
    assert!(ir.contains("add <4 x i32>"));
    assert!(ir.contains("add <2 x i64>"));
    assert!(ir.contains("fadd float"));
    let vector_memory_instructions = ir
        .lines()
        .filter(|line| {
            line.contains("load <4 x float>")
                || line.contains("store <4 x float>")
                || line.contains("load <2 x double>")
                || line.contains("store <2 x double>")
        })
        .collect::<Vec<_>>();
    assert_eq!(vector_memory_instructions.len(), 6);
    assert!(
        vector_memory_instructions
            .iter()
            .all(|instruction| instruction.contains("align 1"))
    );
    let integer_vector_memory_instructions = ir
        .lines()
        .filter(|line| {
            line.contains("load <16 x i8>")
                || line.contains("store <16 x i8>")
                || line.contains("load <8 x i16>")
                || line.contains("store <8 x i16>")
                || line.contains("load <4 x i32>")
                || line.contains("store <4 x i32>")
                || line.contains("load <2 x i64>")
                || line.contains("store <2 x i64>")
        })
        .collect::<Vec<_>>();
    assert_eq!(integer_vector_memory_instructions.len(), 24);
    assert!(
        integer_vector_memory_instructions
            .iter()
            .all(|instruction| instruction.contains("align 1"))
    );
    assert!(ir.contains("ERR_SIMD_LENGTH_MISMATCH"));
    assert!(ir.contains("ERR_SIMD_OUT_OF_MEMORY"));
}

#[test]
fn emits_vector_subtraction_and_multiplication_for_integer_and_float_arrays() {
    let ir = compile_module_to_ir(
        r#"
        import { simdSub, simdMul, SimdError } from "tyrus:simd";

        function subtractI32(left: i32[], right: i32[]): Result<i32[], SimdError> { return simdSub(left, right); }
        function multiplyI32(left: i32[], right: i32[]): Result<i32[], SimdError> { return simdMul(left, right); }
        function subtractF32(left: f32[], right: f32[]): Result<f32[], SimdError> { return simdSub(left, right); }
        function multiplyF32(left: f32[], right: f32[]): Result<f32[], SimdError> { return simdMul(left, right); }
        "#,
        "simd-array-sub-mul",
    );

    assert!(ir.contains("sub <4 x i32>"));
    assert!(ir.contains("mul <4 x i32>"));
    assert!(ir.contains("fsub <4 x float>"));
    assert!(ir.contains("fmul <4 x float>"));
    assert!(!ir.contains("simd.mul.overflow_lanes"));
    assert!(!ir.contains("simd.operand_signs_match"));
    assert!(ir.contains("simdSub could not allocate its result array"));
    assert!(ir.contains("simdMul could not allocate its result array"));
}

#[test]
fn accepts_inferred_array_bindings_as_simd_operands() {
    let ir = compile_module_to_ir(
        r#"
        import { simdAdd, SimdError } from "tyrus:simd";

        function addInferredArrays(): Result<i32[], SimdError> {
            const left = [1, 2, 3, 4, 5, 6, 7, 8];
            const right = [8, 7, 6, 5, 4, 3, 2, 1];
            return simdAdd(left, right);
        }
        "#,
        "simd-inferred-arrays",
    );

    assert!(ir.contains("add <4 x i32>"));
}

#[test]
fn emits_vector_division_and_prevalidates_integer_lanes() {
    let ir = compile_module_to_ir(
        r#"
        import { simdDiv, SimdError } from "tyrus:simd";

        function divideI32(left: i32[], right: i32[]): Result<i32[], SimdError> { return simdDiv(left, right); }
        function divideU32(left: u32[], right: u32[]): Result<u32[], SimdError> { return simdDiv(left, right); }
        function divideF32(left: f32[], right: f32[]): Result<f32[], SimdError> { return simdDiv(left, right); }
        "#,
        "simd-array-div",
    );

    assert!(ir.contains("sdiv <4 x i32>"));
    assert!(ir.contains("udiv <4 x i32>"));
    assert!(ir.contains("fdiv <4 x float>"));
    assert!(ir.contains("ERR_SIMD_DIVISION_BY_ZERO"));
    assert!(ir.contains("ERR_SIMD_INTEGER_OVERFLOW"));
    assert!(ir.contains("simd.div.validate.has_more"));
}

#[test]
fn simd_result_arrays_are_released_when_the_result_binding_leaves_scope() {
    let ir = compile_module_to_ir(
        r#"
        import { simdAdd, SimdError } from "tyrus:simd";

        function discardSimdResult(left: i32[], right: i32[]): void {
            const result: Result<i32[], SimdError> = simdAdd(left, right);
        }
        "#,
        "simd-result-array-cleanup",
    );

    assert!(ir.contains("simd.add.result.value"));
    assert!(ir.contains("call void @free"));
}
