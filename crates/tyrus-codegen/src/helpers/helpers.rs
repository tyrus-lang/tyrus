use super::*;
use tyrus_core::ast::ExprKind;
use tyrus_core::ast::FloatLiteral;
use tyrus_core::ast::StmtKind;

fn is_plain_decimal_literal(text: &str) -> bool {
    let text = text.strip_prefix(['+', '-']).unwrap_or(text);
    let (mantissa, exponent) = match text.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, Some(exponent)),
        None => (text, None),
    };
    let (integer, fraction) = match mantissa.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (mantissa, None),
    };
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    digits(integer)
        && fraction.is_none_or(digits)
        && exponent
            .is_none_or(|exponent| digits(exponent.strip_prefix(['+', '-']).unwrap_or(exponent)))
}

pub(super) fn const_float_literal<'ctx>(
    ty: FloatType<'ctx>,
    literal: &FloatLiteral,
) -> FloatValue<'ctx> {
    if is_plain_decimal_literal(literal.text()) {
        // SAFETY: `const_float_from_string` is unsafe because LLVM parses
        // without reporting failure; it is only reached with text just proven
        // to be a finite decimal literal, and never with the empty string its
        // own assertion rejects.
        unsafe { ty.const_float_from_string(literal.text()) }
    } else {
        ty.const_float(literal.value())
    }
}

pub(super) fn closure_captures(params: &[Param], body: &[Stmt]) -> HashSet<String> {
    let mut names = tyrus_core::ast::block_referenced_identifiers(body);
    for parameter in params {
        for name in parameter.pattern.binding_names() {
            names.remove(name);
        }
    }
    for statement in body {
        match statement.kind() {
            StmtKind::VarDecl { name, .. } => {
                names.remove(name);
            }
            StmtKind::ArrayDestructure { bindings, .. }
            | StmtKind::ObjectDestructure { bindings, .. } => {
                for binding in bindings {
                    names.remove(&binding.name);
                }
            }
            _ => {}
        }
    }
    names
}

pub(super) fn transparent_codegen_type(ty: &Type) -> &Type {
    match ty {
        Type::Infer => ty,
        Type::Ref(inner) | Type::MutRef(inner) | Type::Readonly(inner) => {
            transparent_codegen_type(inner)
        }
        Type::Throws { function, .. } => transparent_codegen_type(function),
        _ => ty,
    }
}

pub(super) fn is_algorithm_value_type(ty: &Type) -> bool {
    matches!(
        transparent_codegen_type(ty),
        Type::BinaryHeap(_)
            | Type::BinomialHeap(_)
            | Type::UnionFind(_)
            | Type::Counter(_)
            | Type::Deque(_)
            | Type::FenwickTree
            | Type::BubbleSort
            | Type::SegmentTree
            | Type::SparseTable
            | Type::SegmentTreeLazy
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
    )
}

pub(super) fn substitute_type(ty: &Type, substitutions: &HashMap<String, Type>) -> Type {
    match ty {
        Type::Generic(name) => substitutions
            .get(name)
            .cloned()
            .unwrap_or_else(|| ty.clone()),
        Type::Array(inner) => Type::Array(Box::new(substitute_type(inner, substitutions))),
        Type::Map(key, value) => Type::Map(
            Box::new(substitute_type(key, substitutions)),
            Box::new(substitute_type(value, substitutions)),
        ),
        Type::Set(value) => Type::Set(Box::new(substitute_type(value, substitutions))),
        Type::BinaryHeap(value) => {
            Type::BinaryHeap(Box::new(substitute_type(value, substitutions)))
        }
        Type::BinomialHeap(value) => {
            Type::BinomialHeap(Box::new(substitute_type(value, substitutions)))
        }
        Type::UnionFind(value) => Type::UnionFind(Box::new(substitute_type(value, substitutions))),
        Type::Counter(value) => Type::Counter(Box::new(substitute_type(value, substitutions))),
        Type::Deque(value) => Type::Deque(Box::new(substitute_type(value, substitutions))),
        Type::LinkedList(value) => {
            Type::LinkedList(Box::new(substitute_type(value, substitutions)))
        }
        Type::DoubleLinkedList(value) => {
            Type::DoubleLinkedList(Box::new(substitute_type(value, substitutions)))
        }
        Type::AvlTree(value) => Type::AvlTree(Box::new(substitute_type(value, substitutions))),
        Type::RedBlackTree(value) => {
            Type::RedBlackTree(Box::new(substitute_type(value, substitutions)))
        }
        Type::SplayTree(value) => Type::SplayTree(Box::new(substitute_type(value, substitutions))),
        Type::Treap(value) => Type::Treap(Box::new(substitute_type(value, substitutions))),
        Type::SortedSet(value) => Type::SortedSet(Box::new(substitute_type(value, substitutions))),
        Type::SkipList(value) => Type::SkipList(Box::new(substitute_type(value, substitutions))),
        Type::SortedMap(key, value) => Type::SortedMap(
            Box::new(substitute_type(key, substitutions)),
            Box::new(substitute_type(value, substitutions)),
        ),
        Type::Graph(node, weight) => Type::Graph(
            Box::new(substitute_type(node, substitutions)),
            Box::new(substitute_type(weight, substitutions)),
        ),
        Type::Tuple(elements) => Type::Tuple(
            elements
                .iter()
                .map(|element| substitute_type(element, substitutions))
                .collect(),
        ),
        Type::Ref(inner) => Type::Ref(Box::new(substitute_type(inner, substitutions))),
        Type::MutRef(inner) => Type::MutRef(Box::new(substitute_type(inner, substitutions))),
        Type::Optional(inner) => Type::Optional(Box::new(substitute_type(inner, substitutions))),
        Type::Throws { function, effects } => Type::Throws {
            function: Box::new(substitute_type(function, substitutions)),
            effects: effects
                .iter()
                .map(|effect| substitute_type(effect, substitutions))
                .collect(),
        },
        Type::Function { params, ret } => Type::Function {
            params: params
                .iter()
                .map(|parameter| substitute_type(parameter, substitutions))
                .collect(),
            ret: Box::new(substitute_type(ret, substitutions)),
        },
        Type::GenericFunction {
            type_params,
            params,
            ret,
        } => Type::GenericFunction {
            type_params: type_params.clone(),
            params: params
                .iter()
                .map(|parameter| substitute_type(parameter, substitutions))
                .collect(),
            ret: Box::new(substitute_type(ret, substitutions)),
        },
        _ => ty.clone(),
    }
}

pub(super) fn infer_codegen_generics(
    expected: &Type,
    actual: &Type,
    substitutions: &mut HashMap<String, Type>,
) -> Result<(), String> {
    match (expected, actual) {
        (Type::Generic(name), actual) => match substitutions.get(name) {
            Some(existing) if existing != actual => Err(format!(
                "error: conflicting inferred types `{existing}` and `{actual}` for `{name}`"
            )),
            Some(_) => Ok(()),
            None => {
                substitutions.insert(name.clone(), actual.clone());
                Ok(())
            }
        },
        (Type::Array(expected), Type::Array(actual))
        | (Type::Ref(expected), Type::Ref(actual))
        | (Type::MutRef(expected), Type::MutRef(actual))
        | (Type::Optional(expected), Type::Optional(actual)) => {
            infer_codegen_generics(expected, actual, substitutions)
        }
        (Type::Tuple(expected), Type::Tuple(actual)) if expected.len() == actual.len() => expected
            .iter()
            .zip(actual)
            .try_for_each(|(expected, actual)| {
                infer_codegen_generics(expected, actual, substitutions)
            }),
        (
            Type::Function {
                params: expected_params,
                ret: expected_ret,
            },
            Type::Function {
                params: actual_params,
                ret: actual_ret,
            },
        ) => {
            for (expected, actual) in expected_params.iter().zip(actual_params) {
                infer_codegen_generics(expected, actual, substitutions)?;
            }
            infer_codegen_generics(expected_ret, actual_ret, substitutions)
        }
        _ => Ok(()),
    }
}

pub(super) fn mangle_type(ty: &Type) -> String {
    match ty {
        Type::Infer => "inferred".to_string(),
        Type::Throws { function, .. } => mangle_type(function),
        Type::I8 => "i8".to_string(),
        Type::I16 => "i16".to_string(),
        Type::I32 => "i32".to_string(),
        Type::I64 => "i64".to_string(),
        Type::I128 => "i128".to_string(),
        Type::Isize => "isize".to_string(),
        Type::U8 => "u8".to_string(),
        Type::U16 => "u16".to_string(),
        Type::U32 => "u32".to_string(),
        Type::U64 => "u64".to_string(),
        Type::U128 => "u128".to_string(),
        Type::Usize => "usize".to_string(),
        Type::F32 => "f32".to_string(),
        Type::F64 => "f64".to_string(),
        Type::F128 => "f128".to_string(),
        Type::Decimal => "decimal".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Char => "char".to_string(),
        Type::Regex => "regexp".to_string(),
        Type::Str => "string".to_string(),
        Type::Void => "void".to_string(),
        Type::Named(name) | Type::Generic(name) => name
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character
                } else {
                    '_'
                }
            })
            .collect(),
        Type::Array(inner) => format!("array_{}", mangle_type(inner)),
        Type::Map(key, value) => format!("map_{}_to_{}", mangle_type(key), mangle_type(value)),
        Type::Set(value) => format!("set_{}", mangle_type(value)),
        Type::BinaryHeap(value) => format!("binary_heap_{}", mangle_type(value)),
        Type::BinomialHeap(value) => format!("binomial_heap_{}", mangle_type(value)),
        Type::UnionFind(value) => format!("union_find_{}", mangle_type(value)),
        Type::Counter(value) => format!("counter_{}", mangle_type(value)),
        Type::Deque(value) => format!("deque_{}", mangle_type(value)),
        Type::FenwickTree => "fenwick_tree".to_string(),
        Type::BubbleSort => "bubble_sort".to_string(),
        Type::SegmentTree => "segment_tree".to_string(),
        Type::SparseTable => "sparse_table".to_string(),
        Type::SegmentTreeLazy => "segment_tree_lazy".to_string(),
        Type::BitSet => "bit_set".to_string(),
        Type::Trie => "trie".to_string(),
        Type::BloomFilter => "bloom_filter".to_string(),
        Type::SuffixTree => "suffix_tree".to_string(),
        Type::Lca => "lca".to_string(),
        Type::Hld => "hld".to_string(),
        Type::Centroid => "centroid".to_string(),
        Type::LinkedList(value) => format!("linked_list_{}", mangle_type(value)),
        Type::DoubleLinkedList(value) => format!("double_linked_list_{}", mangle_type(value)),
        Type::AvlTree(value) => format!("avl_tree_{}", mangle_type(value)),
        Type::RedBlackTree(value) => format!("red_black_tree_{}", mangle_type(value)),
        Type::SplayTree(value) => format!("splay_tree_{}", mangle_type(value)),
        Type::Treap(value) => format!("treap_{}", mangle_type(value)),
        Type::SortedSet(value) => format!("sorted_set_{}", mangle_type(value)),
        Type::SkipList(value) => format!("skip_list_{}", mangle_type(value)),
        Type::SortedMap(key, value) => {
            format!("sorted_map_{}_{}", mangle_type(key), mangle_type(value))
        }
        Type::Graph(node, weight) => {
            format!("graph_{}_{}", mangle_type(node), mangle_type(weight))
        }
        Type::Range(element) => format!("range_{}", mangle_type(element)),
        Type::Tuple(elements) => format!(
            "tuple_{}",
            elements
                .iter()
                .map(mangle_type)
                .collect::<Vec<_>>()
                .join("_")
        ),
        Type::Readonly(inner) => format!("readonly_{}", mangle_type(inner)),
        Type::Ref(inner) => format!("ref_{}", mangle_type(inner)),
        Type::MutRef(inner) => format!("mut_ref_{}", mangle_type(inner)),
        Type::Optional(inner) => format!("optional_{}", mangle_type(inner)),
        Type::Result { success, error } => {
            format!("result_{}_or_{}", mangle_type(success), mangle_type(error))
        }
        Type::Function { params, ret } | Type::GenericFunction { params, ret, .. } => format!(
            "fn_{}_to_{}",
            params.iter().map(mangle_type).collect::<Vec<_>>().join("_"),
            mangle_type(ret)
        ),
        Type::Object(_) => "object".to_string(),
        Type::Union(variants) => format!(
            "union_{}",
            variants
                .iter()
                .map(mangle_type)
                .collect::<Vec<_>>()
                .join("_or_")
        ),
        Type::Unknown => "unknown".to_string(),
        Type::Never => "never".to_string(),
        Type::Lit(expr) => {
            let ut = match expr.as_ref().kind() {
                ExprKind::StrLit(_) => Type::Str,
                ExprKind::RegexLit { .. } => Type::Regex,
                ExprKind::IntLit(_) => Type::I32,
                ExprKind::UIntLit(_) => Type::U128,
                ExprKind::FloatLit(_) => Type::F64,
                ExprKind::BoolLit(_) => Type::Bool,
                _ => Type::Never,
            };
            format!("lit_{}", mangle_type(&ut))
        }
        Type::TypeQuery(expr) => format!("typeof_{}", expr_display(expr)),
        Type::Apply { name, args } => {
            let args = args.iter().map(mangle_type).collect::<Vec<_>>().join("_");
            format!("{name}_of_{args}")
        }
        Type::KeyOf(inner) => format!("keyof_{}", mangle_type(inner)),
        Type::Conditional { .. } => "conditional".to_string(),
        Type::InferVar(name) => format!("infer_{name}"),
    }
}

pub(super) fn field_storage_type(field: &Field) -> Type {
    if field.optional && !matches!(field.ty, Type::Optional(_)) {
        Type::Optional(Box::new(field.ty.clone()))
    } else {
        field.ty.clone()
    }
}

pub(super) fn is_integer_type(ty: &Type) -> bool {
    tyrus_core::numeric_types::is_integer_type(transparent_codegen_type(ty))
}

pub(super) fn is_unsigned_int_type(ty: &Type) -> bool {
    tyrus_core::numeric_types::is_unsigned_integer_type(transparent_codegen_type(ty))
}

pub(super) fn is_float_type(ty: &Type) -> bool {
    tyrus_core::numeric_types::is_float_type(transparent_codegen_type(ty))
}

pub(super) fn numeric_constant_type(type_name: &str, field: &str) -> Option<Type> {
    tyrus_core::numeric_types::numeric_constant_type(type_name, field)
}

pub(super) fn is_functional_array_method(method: &str) -> bool {
    tyrus_core::type_checker::is_functional_array_method_surface(method) || method == "count"
}

/// Whether `ty` is an opaque native collection handle, a pointer the runtime
/// allocated and only the runtime can free.
///
/// The companion of `emit_owned_native_value_free`'s collection
/// arms: this answers *whether* one of them applies, that match answers *which*
/// one. The two lists have to agree, and a type present there but missing here
/// is a leak rather than a crash, a handle nobody claims is a handle nobody
/// releases, which is exactly how a `SortedSet` returned out of a function
/// leaked (`tests/leaks/`). The match has no wildcard arm for that reason: a
/// `Type` variant added later stops compiling here until someone says which
/// side it belongs to, rather than defaulting to the silent, leaking one.
///
/// These types are deliberately outside `async_owned_value_supported`: that
/// predicate also drives frame layout and deep-clone glue, neither of which an
/// opaque handle has. Widening it was tried and reverted for exactly that
/// reason; ownership at a call boundary is a separate question and gets this
/// separate answer.
pub(super) fn native_collection_handle(ty: &Type) -> bool {
    match ty {
        Type::Throws { function, .. } => native_collection_handle(function),
        Type::Map(_, _)
        | Type::Set(_)
        | Type::BinaryHeap(_)
        | Type::BinomialHeap(_)
        | Type::UnionFind(_)
        | Type::Counter(_)
        | Type::Deque(_)
        | Type::FenwickTree
        | Type::BubbleSort
        | Type::SegmentTree
        | Type::SparseTable
        | Type::SegmentTreeLazy
        | Type::BitSet
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
        | Type::Trie
        | Type::BloomFilter
        | Type::SuffixTree
        | Type::Lca
        | Type::Hld
        | Type::Centroid => true,
        Type::Infer
        | Type::I8
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
        | Type::Decimal
        | Type::Bool
        | Type::Char
        | Type::Regex
        | Type::Str
        | Type::Void
        | Type::Named(_)
        | Type::Generic(_)
        | Type::Array(_)
        | Type::Tuple(_)
        | Type::Readonly(_)
        | Type::Ref(_)
        | Type::MutRef(_)
        | Type::Optional(_)
        | Type::Result { .. }
        | Type::Range(_)
        | Type::Function { .. }
        | Type::GenericFunction { .. }
        | Type::Object(_)
        | Type::Union(_)
        | Type::Unknown
        | Type::Never
        | Type::Lit(_)
        | Type::TypeQuery(_)
        | Type::Apply { .. }
        | Type::KeyOf(_)
        | Type::Conditional { .. }
        | Type::InferVar(_) => false,
    }
}
