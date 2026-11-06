use crate::ast::{Type, TypeParam};
use crate::collections::HashMap;
use crate::module_loader::ExportInfo;

pub const ADD_TARGET: &str = "__tyrus_simd_add";
pub const SUB_TARGET: &str = "__tyrus_simd_sub";
pub const MUL_TARGET: &str = "__tyrus_simd_mul";
pub const DIV_TARGET: &str = "__tyrus_simd_div";

pub(crate) fn simd_builtins() -> HashMap<String, ExportInfo> {
    let number = Type::Union(vec![
        Type::I8,
        Type::I16,
        Type::I32,
        Type::I64,
        Type::U8,
        Type::U16,
        Type::U32,
        Type::U64,
        Type::F32,
        Type::F64,
    ]);
    let mut exports = [
        ("simdAdd", ADD_TARGET),
        ("simdSub", SUB_TARGET),
        ("simdMul", MUL_TARGET),
        ("simdDiv", DIV_TARGET),
    ]
    .into_iter()
    .map(|(name, target)| {
        let generic = Type::Generic("T".to_string());
        (
            name.to_string(),
            ExportInfo {
                target: target.to_string(),
                ty: Type::GenericFunction {
                    type_params: vec![TypeParam {
                        name: "T".to_string(),
                        constraint: Some(number.clone()),
                    }],
                    params: vec![
                        Type::Array(Box::new(generic.clone())),
                        Type::Array(Box::new(generic.clone())),
                    ],
                    ret: Box::new(Type::Result {
                        success: Box::new(Type::Array(Box::new(generic))),
                        error: Box::new(Type::Named("SimdError".to_string())),
                    }),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        )
    })
    .collect::<HashMap<_, _>>();
    exports.insert(
        "SimdError".to_string(),
        ExportInfo {
            target: "SimdError".to_string(),
            ty: Type::Named("SimdError".to_string()),
            interface_fields: Some(crate::builtin_interfaces::simd_error_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: Vec::new(),
            db_model: None,
        },
    );
    exports
}
