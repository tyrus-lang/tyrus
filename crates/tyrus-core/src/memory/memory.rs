use crate::ast::{Type, TypeParam};
use crate::collections::HashMap;
use crate::module_loader::ExportInfo;

pub const ZEROIZE_TARGET: &str = "__tyrus_memory_zeroize";
pub const DROP_TARGET: &str = "__tyrus_memory_drop";
pub const SIZE_OF_TARGET: &str = "__tyrus_memory_size_of";
pub const ALIGN_OF_TARGET: &str = "__tyrus_memory_align_of";
pub const SWAP_TARGET: &str = "__tyrus_memory_swap";
pub const REPLACE_TARGET: &str = "__tyrus_memory_replace";

pub fn zeroize_type() -> Type {
    Type::GenericFunction {
        type_params: vec![TypeParam {
            name: "T".to_string(),
            constraint: None,
        }],
        params: vec![Type::Generic("T".to_string())],
        ret: Box::new(Type::Result {
            success: Box::new(Type::Void),
            error: Box::new(Type::Named("MemoryError".to_string())),
        }),
    }
}

pub fn drop_type() -> Type {
    Type::GenericFunction {
        type_params: vec![TypeParam {
            name: "T".to_string(),
            constraint: None,
        }],
        params: vec![Type::Generic("T".to_string())],
        ret: Box::new(Type::Void),
    }
}

pub fn layout_query_type() -> Type {
    Type::GenericFunction {
        type_params: vec![TypeParam {
            name: "T".to_string(),
            constraint: None,
        }],
        params: Vec::new(),
        ret: Box::new(Type::Usize),
    }
}

pub fn swap_type() -> Type {
    Type::GenericFunction {
        type_params: vec![TypeParam {
            name: "T".to_string(),
            constraint: None,
        }],
        params: vec![
            Type::Generic("T".to_string()),
            Type::Generic("T".to_string()),
        ],
        ret: Box::new(Type::Void),
    }
}

pub fn replace_type() -> Type {
    Type::GenericFunction {
        type_params: vec![TypeParam {
            name: "T".to_string(),
            constraint: None,
        }],
        params: vec![
            Type::Generic("T".to_string()),
            Type::Generic("T".to_string()),
        ],
        ret: Box::new(Type::Generic("T".to_string())),
    }
}

pub fn has_stable_layout(ty: &Type) -> bool {
    matches!(
        ty,
        Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::I128
            | Type::Isize
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::Usize
            | Type::F32
            | Type::F64
            | Type::F128
            | Type::Bool
            | Type::Char
    )
}

pub fn supports_drop(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Str
            | Type::Regex
            | Type::Array(_)
            | Type::Map(_, _)
            | Type::Set(_)
            | Type::BinaryHeap(_)
            | Type::BinomialHeap(_)
            | Type::UnionFind(_)
            | Type::Counter(_)
            | Type::Deque(_)
            | Type::FenwickTree
            | Type::BubbleSort
            | Type::SegmentTree
            | Type::SegmentTreeLazy
            | Type::SparseTable
            | Type::BitSet
            | Type::Trie
            | Type::BloomFilter
            | Type::SuffixTree
            | Type::Lca
            | Type::Hld
            | Type::Centroid
            | Type::LinkedList(_)
            | Type::DoubleLinkedList(_)
            | Type::AvlTree(_)
            | Type::RedBlackTree(_)
            | Type::SplayTree(_)
            | Type::Treap(_)
            | Type::SortedSet(_)
            | Type::SkipList(_)
            | Type::SortedMap(_, _)
            | Type::Graph(_, _)
    ) || matches!(
        ty,
        Type::Named(name) if matches!(name.as_str(), "DigestStream" | "Semaphore" | "ChildProcess" | "WebSocket")
    ) || matches!(
        ty,
        Type::Apply { name, args }
            if args.len() == 1 && matches!(name.as_str(), "Sender" | "Receiver" | "Mutex" | "RwLock")
    )
}

pub(crate) fn memory_builtins() -> HashMap<String, ExportInfo> {
    [
        (
            "MemoryError".to_string(),
            ExportInfo {
                target: "MemoryError".to_string(),
                ty: Type::Named("MemoryError".to_string()),
                interface_fields: Some(crate::builtin_interfaces::memory_error_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "zeroize".to_string(),
            ExportInfo {
                target: ZEROIZE_TARGET.to_string(),
                ty: zeroize_type(),
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "drop".to_string(),
            ExportInfo {
                target: DROP_TARGET.to_string(),
                ty: drop_type(),
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "sizeOf".to_string(),
            ExportInfo {
                target: SIZE_OF_TARGET.to_string(),
                ty: layout_query_type(),
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "alignOf".to_string(),
            ExportInfo {
                target: ALIGN_OF_TARGET.to_string(),
                ty: layout_query_type(),
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "swap".to_string(),
            ExportInfo {
                target: SWAP_TARGET.to_string(),
                ty: swap_type(),
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "replace".to_string(),
            ExportInfo {
                target: REPLACE_TARGET.to_string(),
                ty: replace_type(),
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
    ]
    .into_iter()
    .collect()
}
