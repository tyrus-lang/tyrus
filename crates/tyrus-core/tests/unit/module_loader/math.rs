use super::*;

#[test]
fn exposes_math_constants() {
    let names = crate::module_loader::math_constant_names();
    assert!(!names.is_empty());
    for constant in names {
        let (ty, _, _) = native_export_info("tyrus:math", constant)
            .unwrap_or_else(|| panic!("math constant `{constant}` is named but not exported"));
        assert_eq!(ty, Type::F64, "math constant `{constant}` must be f64");
        assert!(
            crate::module_loader::math_constant_value(constant).is_some(),
            "math constant `{constant}` is exported but has no value"
        );
    }
    assert!(native_export_info("tyrus:math", "UNKNOWN").is_none());
    assert!(crate::module_loader::math_constant_value("UNKNOWN").is_none());
}

#[test]
fn the_golden_ratio_satisfies_its_own_definition() {
    let phi = crate::module_loader::math_constant_value("PHI").expect("PHI");
    assert_eq!(phi * phi - phi, 1.0);
    assert_eq!(phi, f64::midpoint(1.0, 5.0_f64.sqrt()));
}

#[test]
#[allow(clippy::too_many_lines)]
fn exposes_math_functions() {
    let functions = [
        (
            "abs",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "sign",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "sqrt",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "cbrt",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "floor",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "ceil",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "trunc",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "round",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "sin",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "cos",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "tan",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "asin",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "acos",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "atan",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "atan2",
            Type::Function {
                params: vec![Type::F64, Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "sinh",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "cosh",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "tanh",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "exp",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "log",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "log10",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "log2",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "pow",
            Type::Function {
                params: vec![Type::F64, Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "max",
            Type::Function {
                params: vec![Type::F64, Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "min",
            Type::Function {
                params: vec![Type::F64, Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "imul",
            Type::Function {
                params: vec![Type::I32, Type::I32],
                ret: Box::new(Type::I32),
            },
        ),
        (
            "clz32",
            Type::Function {
                params: vec![Type::I32],
                ret: Box::new(Type::I32),
            },
        ),
        (
            "approxEqual",
            Type::Function {
                params: vec![Type::F64, Type::F64, Type::Optional(Box::new(Type::F64))],
                ret: Box::new(Type::Bool),
            },
        ),
        (
            "clamp",
            Type::Function {
                params: vec![Type::F64, Type::F64, Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "clampMin",
            Type::Function {
                params: vec![Type::F64, Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "clampMax",
            Type::Function {
                params: vec![Type::F64, Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "lerp",
            Type::Function {
                params: vec![Type::F64, Type::F64, Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "inverseLerp",
            Type::Function {
                params: vec![Type::F64, Type::F64, Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "remap",
            Type::Function {
                params: vec![Type::F64, Type::F64, Type::F64, Type::F64, Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "degToRad",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "radToDeg",
            Type::Function {
                params: vec![Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "gcd",
            Type::Function {
                params: vec![Type::U32, Type::U32],
                ret: Box::new(Type::U32),
            },
        ),
        (
            "lcm",
            Type::Function {
                params: vec![Type::U32, Type::U32],
                ret: Box::new(Type::U32),
            },
        ),
        (
            "randomRange",
            Type::Function {
                params: vec![Type::F64, Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "smoothstep",
            Type::Function {
                params: vec![Type::F64, Type::F64, Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "roundTo",
            Type::Function {
                params: vec![Type::F64, Type::I32],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "mean",
            Type::Function {
                params: vec![Type::Array(Box::new(Type::F64))],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "median",
            Type::Function {
                params: vec![Type::Array(Box::new(Type::F64))],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "mode",
            Type::Function {
                params: vec![Type::Array(Box::new(Type::F64))],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "variance",
            Type::Function {
                params: vec![Type::Array(Box::new(Type::F64))],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "stdDev",
            Type::Function {
                params: vec![Type::Array(Box::new(Type::F64))],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "percentile",
            Type::Function {
                params: vec![Type::F64, Type::Array(Box::new(Type::F64))],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "factorial",
            Type::Function {
                params: vec![Type::U128],
                ret: Box::new(Type::U128),
            },
        ),
        (
            "permutation",
            Type::Function {
                params: vec![Type::U128, Type::U128],
                ret: Box::new(Type::U128),
            },
        ),
        (
            "combination",
            Type::Function {
                params: vec![Type::U128, Type::U128],
                ret: Box::new(Type::U128),
            },
        ),
        (
            "isPrime",
            Type::Function {
                params: vec![Type::U64],
                ret: Box::new(Type::Bool),
            },
        ),
        (
            "nextPrime",
            Type::Function {
                params: vec![Type::U64],
                ret: Box::new(Type::U64),
            },
        ),
        (
            "primeFactors",
            Type::Function {
                params: vec![Type::U64],
                ret: Box::new(Type::Array(Box::new(Type::U64))),
            },
        ),
        (
            "fibonacci",
            Type::Function {
                params: vec![Type::U64],
                ret: Box::new(Type::U64),
            },
        ),
        (
            "hypot",
            Type::Function {
                params: vec![Type::F64, Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "copysign",
            Type::Function {
                params: vec![Type::F64, Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "fmod",
            Type::Function {
                params: vec![Type::F64, Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "logBase",
            Type::Function {
                params: vec![Type::F64, Type::F64],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "powi",
            Type::Function {
                params: vec![Type::F64, Type::I32],
                ret: Box::new(Type::F64),
            },
        ),
        (
            "toPolar",
            Type::Function {
                params: vec![Type::F64, Type::F64],
                ret: Box::new(Type::Tuple(vec![Type::F64, Type::F64])),
            },
        ),
        (
            "toCartesian",
            Type::Function {
                params: vec![Type::F64, Type::F64],
                ret: Box::new(Type::Tuple(vec![Type::F64, Type::F64])),
            },
        ),
    ];
    for (name, expected_ty) in functions {
        let (ty, _, _) = native_export_info("tyrus:math", name).unwrap();
        assert_eq!(ty, expected_ty, "math function `{name}` type mismatch");
    }
}

#[test]
fn resolves_math_named_imports() {
    let root = std::env::temp_dir().join(format!("tyrus-math-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { PI, TAU } from "tyrus:math";
            const circle: f64 = PI * 2.0;
            const fullCircle: f64 = TAU;
        "#,
    )
    .unwrap();

    let program = load_program(&root.join("main.tyr"), None).unwrap();
    assert!(program.imports.is_empty());
    assert!(program.exports.is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn resolves_math_namespace_imports() {
    let root = std::env::temp_dir().join(format!("tyrus-math-ns-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import * as math from "tyrus:math";
            const circle: f64 = math.PI * 2.0;
        "#,
    )
    .unwrap();

    let program = load_program(&root.join("main.tyr"), None).unwrap();
    assert!(program.imports.is_empty());
    assert!(program.exports.is_empty());
    fs::remove_dir_all(root).unwrap();
}
