use super::*;

impl TypeChecker {
    #[allow(clippy::too_many_lines)]
    pub(super) fn check_algorithm_constructor_call(
        &mut self,
        callee: &str,
        callee_short: &str,
        type_args: &[Type],
        args: &[Expr],
    ) -> Option<ValueType> {
        let heap_name = if callee_short == "BinaryHeap" {
            Some("BinaryHeap")
        } else if callee_short == "BinomialHeap" {
            Some("BinomialHeap")
        } else {
            None
        };
        if let Some(heap_name) = heap_name {
            let element = if type_args.is_empty() {
                self.error(
                    "E0107",
                    format!("`{heap_name}` requires exactly one explicit element type"),
                    Some(callee),
                );
                return Some(ValueType::Error);
            } else {
                type_args[0].clone()
            };
            if !matches!(
                &element,
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
            ) {
                self.error(
                    "E0277",
                    format!("`{heap_name}` element type must be an ordered numeric type, found `{element}`"),
                    Some(callee),
                );
                return Some(ValueType::Error);
            }
            let heap_ty = if heap_name == "BinaryHeap" {
                Type::BinaryHeap(Box::new(element))
            } else {
                Type::BinomialHeap(Box::new(element))
            };
            return Some(ValueType::Known(heap_ty));
        }
        if callee_short == "UnionFind" {
            let element = if type_args.is_empty() {
                Type::Usize
            } else {
                let element = &type_args[0];
                if !matches!(
                    element,
                    Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::Usize
                ) {
                    self.error(
                        "E3242",
                        format!(
                            "`UnionFind` element type must be an unsigned integer (u8, u16, u32, u64, u128, usize), found `{element}`"
                        ),
                        Some(callee),
                    );
                    return Some(ValueType::Error);
                }
                element.clone()
            };
            return Some(ValueType::Known(Type::UnionFind(Box::new(element))));
        }
        if callee_short == "Counter" {
            let element = if type_args.is_empty() {
                self.error(
                    "E0107",
                    "`Counter` requires exactly one explicit element type".to_string(),
                    Some(callee),
                );
                return Some(ValueType::Error);
            } else {
                type_args[0].clone()
            };
            if !matches!(
                &element,
                Type::Bool
                    | Type::Char
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
            ) {
                self.error(
                    "E0277",
                    format!(
                        "`Counter` element type must be a scalar value type, found `{element}`"
                    ),
                    Some(callee),
                );
                return Some(ValueType::Error);
            }
            return Some(ValueType::Known(Type::Counter(Box::new(element))));
        }
        if callee_short == "Deque" {
            let element = if type_args.is_empty() {
                self.error(
                    "E0107",
                    "`Deque` requires exactly one explicit element type".to_string(),
                    Some(callee),
                );
                return Some(ValueType::Error);
            } else {
                type_args[0].clone()
            };
            if !matches!(
                &element,
                Type::Bool
                    | Type::Char
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
            ) {
                self.error(
                    "E0277",
                    format!("`Deque` element type must be a scalar value type, found `{element}`"),
                    Some(callee),
                );
                return Some(ValueType::Error);
            }
            return Some(ValueType::Known(Type::Deque(Box::new(element))));
        }
        if callee_short == "FenwickTree" {
            if args.len() != 1 {
                self.error(
                    "E0061",
                    format!(
                        "`FenwickTree` expects 1 argument, but {} were provided",
                        args.len()
                    ),
                    Some(callee),
                );
            }
            if let Some(size_arg) = args.first() {
                let actual = self.check_expr_as(size_arg, &Type::Usize);
                self.require_assignable(&Type::Usize, &actual, callee, "E0308");
            }
            return Some(ValueType::Known(Type::FenwickTree));
        }
        if callee_short == "BubbleSort" {
            if !type_args.is_empty() {
                self.error(
                    "E0107",
                    "`BubbleSort` does not accept type arguments".to_string(),
                    Some(callee),
                );
            }
            if args.len() != 1 {
                self.error(
                    "E0061",
                    format!(
                        "`BubbleSort` expects 1 argument, but {} were provided",
                        args.len()
                    ),
                    Some(callee),
                );
            }
            if let Some(values) = args.first() {
                let expected = Type::Array(Box::new(Type::I32));
                let actual = self.check_expr_as(values, &expected);
                self.require_assignable(&expected, &actual, callee, "E0308");
            }
            return Some(ValueType::Known(Type::BubbleSort));
        }
        if callee_short == "SegmentTree" {
            if args.len() != 1 {
                self.error(
                    "E0061",
                    format!(
                        "`SegmentTree` expects 1 argument, but {} were provided",
                        args.len()
                    ),
                    Some(callee),
                );
            }
            if let Some(data_arg) = args.first() {
                let expected = Type::Array(Box::new(Type::I32));
                let actual = self.check_expr_as(data_arg, &expected);
                self.require_assignable(&expected, &actual, callee, "E0308");
            }
            return Some(ValueType::Known(Type::SegmentTree));
        }
        if callee_short == "SparseTable" {
            if args.len() != 1 {
                self.error(
                    "E0061",
                    format!(
                        "`SparseTable` expects 1 argument, but {} were provided",
                        args.len()
                    ),
                    Some(callee),
                );
            }
            if let Some(data_arg) = args.first() {
                let expected = Type::Array(Box::new(Type::I32));
                let actual = self.check_expr_as(data_arg, &expected);
                self.require_assignable(&expected, &actual, callee, "E0308");
            }
            return Some(ValueType::Known(Type::SparseTable));
        }
        if callee_short == "SegmentTreeLazy" {
            if args.len() != 1 {
                self.error(
                    "E0061",
                    format!(
                        "`SegmentTreeLazy` expects 1 argument, but {} were provided",
                        args.len()
                    ),
                    Some(callee),
                );
            }
            if let Some(data_arg) = args.first() {
                let expected = Type::Array(Box::new(Type::I32));
                let actual = self.check_expr_as(data_arg, &expected);
                self.require_assignable(&expected, &actual, callee, "E0308");
            }
            return Some(ValueType::Known(Type::SegmentTreeLazy));
        }
        if callee_short == "BitSet" {
            if args.len() != 1 {
                self.error(
                    "E0061",
                    format!(
                        "`BitSet` expects 1 argument, but {} were provided",
                        args.len()
                    ),
                    Some(callee),
                );
            }
            if let Some(size_arg) = args.first() {
                let actual = self.check_expr_as(size_arg, &Type::Usize);
                self.require_assignable(&Type::Usize, &actual, callee, "E0308");
            }
            return Some(ValueType::Known(Type::BitSet));
        }
        if callee_short == "Trie" {
            if !args.is_empty() {
                self.error(
                    "E0061",
                    format!(
                        "`Trie` expects 0 arguments, but {} were provided",
                        args.len()
                    ),
                    Some(callee),
                );
            }
            return Some(ValueType::Known(Type::Trie));
        }
        if callee_short == "BloomFilter" {
            if args.len() != 2 {
                self.error(
                    "E0061",
                    format!(
                        "`BloomFilter` expects 2 arguments, but {} were provided",
                        args.len()
                    ),
                    Some(callee),
                );
            }
            if let Some(k_arg) = args.first() {
                let actual = self.check_expr_as(k_arg, &Type::Usize);
                self.require_assignable(&Type::Usize, &actual, callee, "E0308");
            }
            if let Some(m_arg) = args.get(1) {
                let actual = self.check_expr_as(m_arg, &Type::Usize);
                self.require_assignable(&Type::Usize, &actual, callee, "E0308");
            }
            return Some(ValueType::Known(Type::BloomFilter));
        }
        if callee_short == "suffixTreeUkkonen" {
            if args.len() != 1 {
                self.error(
                    "E0061",
                    format!(
                        "`suffixTreeUkkonen` expects 1 argument, but {} were provided",
                        args.len()
                    ),
                    Some(callee),
                );
            }
            if let Some(text_arg) = args.first() {
                let actual = self.check_expr_as(text_arg, &Type::Str);
                self.require_assignable(&Type::Str, &actual, callee, "E0308");
            }
            return Some(ValueType::Known(Type::SuffixTree));
        }
        if callee_short == "lcaBinaryLifting" {
            if args.len() != 2 {
                self.error(
                    "E0061",
                    format!(
                        "`lcaBinaryLifting` expects 2 arguments, but {} were provided",
                        args.len()
                    ),
                    Some(callee),
                );
            }
            if let Some(adj_arg) = args.first() {
                let expected = Type::Array(Box::new(Type::Array(Box::new(Type::Usize))));
                let actual = self.check_expr_as(adj_arg, &expected);
                self.require_assignable(&expected, &actual, callee, "E0308");
            }
            if let Some(root_arg) = args.get(1) {
                let actual = self.check_expr_as(root_arg, &Type::Usize);
                self.require_assignable(&Type::Usize, &actual, callee, "E0308");
            }
            return Some(ValueType::Known(Type::Lca));
        }
        if callee_short == "heavyLightDecomposition" {
            if args.len() != 2 {
                self.error(
                    "E0061",
                    format!(
                        "`heavyLightDecomposition` expects 2 arguments, but {} were provided",
                        args.len()
                    ),
                    Some(callee),
                );
            }
            if let Some(adj_arg) = args.first() {
                let expected = Type::Array(Box::new(Type::Array(Box::new(Type::Usize))));
                let actual = self.check_expr_as(adj_arg, &expected);
                self.require_assignable(&expected, &actual, callee, "E0308");
            }
            if let Some(root_arg) = args.get(1) {
                let actual = self.check_expr_as(root_arg, &Type::Usize);
                self.require_assignable(&Type::Usize, &actual, callee, "E0308");
            }
            return Some(ValueType::Known(Type::Hld));
        }
        if callee_short == "centroidDecomposition" {
            if args.len() != 1 {
                self.error(
                    "E0061",
                    format!(
                        "`centroidDecomposition` expects 1 argument, but {} were provided",
                        args.len()
                    ),
                    Some(callee),
                );
            }
            if let Some(adj_arg) = args.first() {
                let expected = Type::Array(Box::new(Type::Array(Box::new(Type::Usize))));
                let actual = self.check_expr_as(adj_arg, &expected);
                self.require_assignable(&expected, &actual, callee, "E0308");
            }
            return Some(ValueType::Known(Type::Centroid));
        }
        if callee_short == "LinkedList" {
            let element = if type_args.is_empty() {
                self.error(
                    "E0107",
                    "`LinkedList` requires exactly one explicit element type".to_string(),
                    Some(callee),
                );
                return Some(ValueType::Error);
            } else {
                type_args[0].clone()
            };
            if !matches!(
                &element,
                Type::Bool
                    | Type::Char
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
            ) {
                self.error(
                    "E0277",
                    format!(
                        "`LinkedList` element type must be a scalar value type, found `{element}`"
                    ),
                    Some(callee),
                );
                return Some(ValueType::Error);
            }
            if !args.is_empty() {
                self.error(
                    "E0061",
                    format!(
                        "`LinkedList` expects 0 arguments, but {} were provided",
                        args.len()
                    ),
                    Some(callee),
                );
            }
            return Some(ValueType::Known(Type::LinkedList(Box::new(element))));
        }
        if callee_short == "DoubleLinkedList" {
            let element = if type_args.is_empty() {
                self.error(
                    "E0107",
                    "`DoubleLinkedList` requires exactly one explicit element type".to_string(),
                    Some(callee),
                );
                return Some(ValueType::Error);
            } else {
                type_args[0].clone()
            };
            if !matches!(
                &element,
                Type::Bool
                    | Type::Char
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
            ) {
                self.error(
                    "E0277",
                    format!(
                        "`DoubleLinkedList` element type must be a scalar value type, found `{element}`"
                    ),
                    Some(callee),
                );
                return Some(ValueType::Error);
            }
            if !args.is_empty() {
                self.error(
                    "E0061",
                    format!(
                        "`DoubleLinkedList` expects 0 arguments, but {} were provided",
                        args.len()
                    ),
                    Some(callee),
                );
            }
            return Some(ValueType::Known(Type::DoubleLinkedList(Box::new(element))));
        }
        let ordered_tree_name = ["AvlTree", "RedBlackTree", "SplayTree", "Treap"]
            .into_iter()
            .find(|name| callee_short == *name);
        if let Some(name) = ordered_tree_name {
            let element = if type_args.is_empty() {
                self.error(
                    "E0107",
                    format!("`{name}` requires exactly one explicit element type"),
                    Some(callee),
                );
                return Some(ValueType::Error);
            } else {
                type_args[0].clone()
            };
            if !ordered_tree_element_allowed(&element) {
                self.error(
                    "E0277",
                    format!("`{name}` element type must be an ordered type (numeric, boolean, or string), found `{element}`"),
                    Some(callee),
                );
                return Some(ValueType::Error);
            }
            if !args.is_empty() {
                self.error(
                    "E0061",
                    format!(
                        "`{name}` expects 0 arguments, but {} were provided",
                        args.len()
                    ),
                    Some(callee),
                );
            }
            return Some(ValueType::Known(match name {
                "AvlTree" => Type::AvlTree(Box::new(element)),
                "RedBlackTree" => Type::RedBlackTree(Box::new(element)),
                "SplayTree" => Type::SplayTree(Box::new(element)),
                _ => Type::Treap(Box::new(element)),
            }));
        }
        if callee_short == "SortedSet" {
            let element = if type_args.is_empty() {
                self.error(
                    "E0107",
                    "`SortedSet` requires exactly one explicit element type".to_string(),
                    Some(callee),
                );
                return Some(ValueType::Error);
            } else {
                type_args[0].clone()
            };
            if !ordered_tree_element_allowed(&element) {
                self.error(
                    "E0277",
                    format!("`SortedSet` element type must be an ordered type (numeric, boolean, or string), found `{element}`"),
                    Some(callee),
                );
                return Some(ValueType::Error);
            }
            if !args.is_empty() {
                self.error(
                    "E0061",
                    format!(
                        "`SortedSet` expects 0 arguments, but {} were provided",
                        args.len()
                    ),
                    Some(callee),
                );
            }
            return Some(ValueType::Known(Type::SortedSet(Box::new(element))));
        }
        if callee_short == "SkipList" {
            let element = if type_args.is_empty() {
                self.error(
                    "E0107",
                    "`SkipList` requires exactly one explicit element type".to_string(),
                    Some(callee),
                );
                return Some(ValueType::Error);
            } else {
                type_args[0].clone()
            };
            if !ordered_tree_element_allowed(&element) {
                self.error(
                    "E0277",
                    format!("`SkipList` element type must be an ordered type (numeric, boolean, or string), found `{element}`"),
                    Some(callee),
                );
                return Some(ValueType::Error);
            }
            if !args.is_empty() {
                self.error(
                    "E0061",
                    format!(
                        "`SkipList` expects 0 arguments, but {} were provided",
                        args.len()
                    ),
                    Some(callee),
                );
            }
            return Some(ValueType::Known(Type::SkipList(Box::new(element))));
        }
        if callee_short == "SortedMap" {
            if type_args.len() < 2 {
                self.error(
                    "E0107",
                    "`SortedMap` requires exactly two explicit type arguments, like `SortedMap<i32, string>()`"
                        .to_string(),
                    Some(callee),
                );
                return Some(ValueType::Error);
            }
            let key = type_args[0].clone();
            let value = type_args[1].clone();
            if !ordered_tree_element_allowed(&key) {
                self.error(
                    "E0277",
                    format!("`SortedMap` key type must be an ordered type (numeric, boolean, or string), found `{key}`"),
                    Some(callee),
                );
                return Some(ValueType::Error);
            }
            if !args.is_empty() {
                self.error(
                    "E0061",
                    format!(
                        "`SortedMap` expects 0 arguments, but {} were provided",
                        args.len()
                    ),
                    Some(callee),
                );
            }
            return Some(ValueType::Known(Type::SortedMap(
                Box::new(key),
                Box::new(value),
            )));
        }
        if callee_short == "Graph" {
            if type_args.len() < 2 {
                self.error(
                    "E0107",
                    "`Graph` requires exactly two explicit type arguments, like `Graph<string, i32>()`"
                        .to_string(),
                    Some(callee),
                );
                return Some(ValueType::Error);
            }
            let node = type_args[0].clone();
            let weight = type_args[1].clone();
            if !ordered_tree_element_allowed(&node) {
                self.error(
                    "E0277",
                    format!("`Graph` node type must be an ordered type (numeric, boolean, or string), found `{node}`"),
                    Some(callee),
                );
                return Some(ValueType::Error);
            }
            if !is_numeric_type(&weight) {
                self.error(
                    "E0277",
                    format!("`Graph` weight type must be a numeric type, found `{weight}`"),
                    Some(callee),
                );
                return Some(ValueType::Error);
            }
            if !args.is_empty() {
                self.error(
                    "E0061",
                    format!(
                        "`Graph` expects 0 arguments, but {} were provided",
                        args.len()
                    ),
                    Some(callee),
                );
            }
            return Some(ValueType::Known(Type::Graph(
                Box::new(node),
                Box::new(weight),
            )));
        }
        None
    }
}
