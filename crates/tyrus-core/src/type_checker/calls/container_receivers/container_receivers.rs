use super::*;

impl TypeChecker {
    #[allow(clippy::too_many_lines)]
    pub(super) fn check_container_receiver_method(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
        object_ty: &ValueType,
        optional_receiver: bool,
    ) -> Option<ValueType> {
        if let ValueType::Known(Type::Map(key, value)) = &object_ty {
            let (key, value) = (key.as_ref().clone(), value.as_ref().clone());
            let result = self.check_map_method(object, &key, &value, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::Set(element)) = &object_ty {
            let element = element.as_ref().clone();
            let result = self.check_set_method(object, &element, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::BinaryHeap(element)) = &object_ty {
            let element = element.as_ref().clone();
            let result = self.check_binary_heap_method(object, &element, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::BinomialHeap(element)) = &object_ty {
            let element = element.as_ref().clone();
            let result = self.check_binomial_heap_method(object, &element, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::UnionFind(element)) = &object_ty {
            let element = element.as_ref().clone();
            let result = self.check_union_find_method(object, &element, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::Counter(element)) = &object_ty {
            let element = element.as_ref().clone();
            let result = self.check_counter_method(object, &element, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::Deque(element)) = &object_ty {
            let element = element.as_ref().clone();
            let result = self.check_deque_method(object, &element, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::FenwickTree) = &object_ty {
            let result = self.check_fenwick_tree_method(object, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::BubbleSort) = &object_ty {
            let result = self.check_bubble_sort_method(method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::SegmentTree) = &object_ty {
            let result = self.check_segment_tree_method(object, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::SparseTable) = &object_ty {
            let result = self.check_sparse_table_method(object, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::SegmentTreeLazy) = &object_ty {
            let result = self.check_segment_tree_lazy_method(object, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::BitSet) = &object_ty {
            let result = self.check_bit_set_method(object, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::Trie) = &object_ty {
            let result = self.check_trie_method(object, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::SuffixTree) = &object_ty {
            let result = self.check_suffix_tree_method(object, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::Lca) = &object_ty {
            let result = self.check_lca_method(object, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::Hld) = &object_ty {
            let result = self.check_hld_method(object, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::Centroid) = &object_ty {
            let result = self.check_centroid_method(object, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::BloomFilter) = &object_ty {
            let result = self.check_bloom_filter_method(object, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::LinkedList(element)) = &object_ty {
            let element = element.as_ref().clone();
            let result = self.check_linked_list_method(object, &element, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::DoubleLinkedList(element)) = &object_ty {
            let element = element.as_ref().clone();
            let result = self.check_double_linked_list_method(object, &element, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(ty) = &object_ty {
            let (element, type_name) = match ty {
                Type::AvlTree(element) => (Some(element.as_ref().clone()), "AvlTree"),
                Type::RedBlackTree(element) => (Some(element.as_ref().clone()), "RedBlackTree"),
                Type::SplayTree(element) => (Some(element.as_ref().clone()), "SplayTree"),
                Type::Treap(element) => (Some(element.as_ref().clone()), "Treap"),
                _ => (None, ""),
            };
            if let Some(element) = element {
                let result =
                    self.check_ordered_tree_method(object, &element, type_name, method, args);
                return Some(optional_value_type(result, optional_receiver));
            }
        }
        if let ValueType::Known(Type::SortedSet(element)) = &object_ty {
            let element = element.as_ref().clone();
            let result = self.check_sorted_set_method(object, &element, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::SkipList(element)) = &object_ty {
            let element = element.as_ref().clone();
            let result = self.check_skip_list_method(object, &element, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::SortedMap(key, value)) = &object_ty {
            let key = key.as_ref().clone();
            let value = value.as_ref().clone();
            let result = self.check_sorted_map_method(object, &key, &value, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::Graph(node, weight)) = &object_ty {
            let node = node.as_ref().clone();
            let weight = weight.as_ref().clone();
            let result = self.check_graph_method(object, &node, &weight, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let ValueType::Known(Type::Range(element)) = &object_ty {
            let element = element.as_ref().clone();
            let result = self.check_range_method(object, &element, method, args);
            return Some(optional_value_type(result, optional_receiver));
        }
        if let Some(SequenceReceiver {
            element,
            readonly: readonly_receiver,
            fixed_length,
        }) = match &object_ty {
            ValueType::Known(array_ty) => sequence_receiver(array_ty),
            _ => None,
        } {
            let element = &element;
            if fixed_length && is_array_mutating_method(method) {
                self.error_at(
                    "E2543",
                    format!(
                        "cannot call mutating method `{method}` on a tuple: its length is fixed"
                    ),
                    object.span,
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return Some(ValueType::Error);
            }
            if is_functional_array_method(method) {
                if readonly_receiver && is_array_mutating_method(method) {
                    self.error_at(
                        "E2540",
                        format!("cannot call mutating method `{method}` on readonly array"),
                        object.span,
                    );
                }
                let result = self.check_functional_array_method(object, element, method, args);
                return Some(optional_value_type(result, optional_receiver));
            }
            if matches!(method, "sortBy" | "sortedBy") {
                let mutates = method == "sortBy";
                if mutates && readonly_receiver {
                    self.error_at(
                        "E2540",
                        format!("cannot call mutating method `{method}` on readonly array"),
                        object.span,
                    );
                }
                if mutates
                    && matches!(object.kind(), ExprKind::Ident(name) if self.lookup(name).is_some_and(|symbol| !symbol.mutable))
                {
                    self.error_at(
                        "E0384",
                        format!("cannot call mutating method `{method}` on immutable array"),
                        object.span,
                    );
                }
                let result = self.check_array_keyed_sort(element, method, args);
                return Some(optional_value_type(result, optional_receiver));
            }
            if method == "count" {
                let result = self.check_array_count(element, args);
                return Some(optional_value_type(result, optional_receiver));
            }
            if matches!(method, "sumBy" | "avgBy") {
                let result = self.check_array_keyed_aggregate(element, method, args);
                return Some(optional_value_type(result, optional_receiver));
            }
            if matches!(method, "toggle" | "toggleAll") {
                let result = self.check_array_toggle(element, method, args);
                return Some(optional_value_type(result, optional_receiver));
            }
            let (minimum, maximum, return_type, mutates) = match method {
                "push" | "unshift" => (0, usize::MAX, Type::Usize, true),
                "pop" | "shift" => (0, 0, Type::Optional(Box::new(element.clone())), true),
                "first" | "last" => (0, 0, Type::Optional(Box::new(element.clone())), false),
                "random" => (0, 0, element.clone(), false),
                "sum" | "product" => {
                    if !is_numeric_type(element) {
                        self.error(
                            "E0277",
                            format!("array method `{method}` requires a numeric element type, found `{element}`"),
                            Some(method),
                        );
                    }
                    (0, 0, element.clone(), false)
                }
                "avg" => {
                    if !is_numeric_type(element) {
                        self.error(
                            "E0277",
                            format!("array method `avg` requires a numeric element type, found `{element}`"),
                            Some(method),
                        );
                    }
                    let average_type = if matches!(transparent_type(element), Type::F128) {
                        Type::F128
                    } else {
                        Type::F64
                    };
                    (0, 0, Type::Optional(Box::new(average_type)), false)
                }
                "rotate" => (1, 1, Type::Array(Box::new(element.clone())), true),
                "splice" => (1, usize::MAX, Type::Array(Box::new(element.clone())), true),
                "reverse" => (0, 0, Type::Array(Box::new(element.clone())), true),
                "at" => (1, 1, Type::Optional(Box::new(element.clone())), false),
                "keys" => (0, 0, Type::Array(Box::new(Type::Usize)), false),
                "values" => (0, 0, Type::Array(Box::new(element.clone())), false),
                "entries" => (
                    0,
                    0,
                    Type::Array(Box::new(Type::Tuple(vec![Type::Usize, element.clone()]))),
                    false,
                ),
                "includes" => (1, 1, Type::Bool, false),
                "equals" => {
                    let scalar = is_numeric_type(element)
                        || matches!(
                            transparent_type(element),
                            Type::Str | Type::Bool | Type::Char
                        );
                    if !scalar {
                        self.error(
                            "E0277",
                            format!(
                                "array method `equals` compares arrays of numbers, `boolean`, `char` or `string`, but the elements are `{element}`"
                            ),
                            Some(method),
                        );
                    }
                    (1, 1, Type::Bool, false)
                }
                "indexOf" | "lastIndexOf" => (1, 1, Type::I64, false),
                "copy" => (0, 0, Type::Array(Box::new(element.clone())), false),
                "slice" => (0, 2, Type::Array(Box::new(element.clone())), false),
                "concat" => (1, 1, Type::Array(Box::new(element.clone())), false),
                "fill" => (1, 3, Type::Array(Box::new(element.clone())), true),
                "copyWithin" => (2, 3, Type::Array(Box::new(element.clone())), true),
                "toReversed" => (0, 0, Type::Array(Box::new(element.clone())), false),
                "shuffle" => (0, 0, Type::Array(Box::new(element.clone())), false),
                "sample" => (1, 2, Type::Array(Box::new(element.clone())), false),
                "with" => (2, 2, Type::Array(Box::new(element.clone())), false),
                "splitAt" => (
                    1,
                    1,
                    Type::Tuple(vec![Type::Array(Box::new(element.clone())); 2]),
                    false,
                ),
                "join" if matches!(element, Type::Str) => (0, 1, Type::Str, false),
                "join" => {
                    self.error(
                        "E0277",
                        "`join` currently requires a `string[]` receiver".to_string(),
                        Some(method),
                    );
                    (0, 1, Type::Str, false)
                }
                _ => {
                    self.error(
                        "E0599",
                        format!(
                            "no method named `{method}` for type `{object_ty_display}`",
                            object_ty_display = object_ty.display()
                        ),
                        Some(method),
                    );
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return Some(ValueType::Error);
                }
            };
            if args.len() < minimum || args.len() > maximum {
                self.error(
                    "E0061",
                    format!("method `{method}` expects {minimum} to {maximum} argument(s), but {} were provided", args.len()),
                    Some(method),
                );
            }
            if mutates {
                if readonly_receiver {
                    self.error_at(
                        "E2540",
                        format!("cannot call mutating method `{method}` on readonly array"),
                        object.span,
                    );
                }
                if let ExprKind::Ident(name) = object.kind()
                    && self.lookup(name).is_some_and(|symbol| !symbol.mutable)
                {
                    self.error_at(
                        "E0384",
                        format!(
                            "cannot call mutating method `{method}` on immutable array `{name}`"
                        ),
                        object.span,
                    );
                }
            }
            for (index, arg) in args.iter().enumerate() {
                let actual = self.check_expr(arg);
                let expected = match method {
                    "push" | "unshift" => Some(element.clone()),
                    "splice" if index < 2 => Some(Type::I64),
                    "splice" => Some(element.clone()),
                    "includes" | "indexOf" | "lastIndexOf" | "fill" => {
                        (index == 0).then(|| element.clone())
                    }
                    "equals" => (index == 0).then(|| Type::Array(Box::new(element.clone()))),
                    "at" | "copyWithin" | "rotate" => Some(Type::I64),
                    "concat" => None,
                    "with" => (index == 1).then(|| element.clone()),
                    "sample" if index == 0 => Some(Type::Usize),
                    "sample" => Some(Type::Object(vec![Field {
                        name: "repeat".to_string(),
                        ty: Type::Bool,
                        readonly: false,
                        optional: true,
                        decorators: Vec::new(),
                        span: Span::DUMMY,
                    }])),
                    "join" => Some(Type::Str),
                    _ => None,
                };
                if method == "concat" {
                    self.require_assignable(
                        &Type::Array(Box::new(element.clone())),
                        &actual,
                        method,
                        "E0308",
                    );
                } else if let Some(expected) = expected {
                    self.require_assignable(&expected, &actual, method, "E0308");
                } else {
                    self.require_assignable(&Type::Usize, &actual, method, "E0308");
                }
            }
            if method == "splitAt"
                && let Some(len) = self.known_array_len(object, object_ty)
            {
                self.check_split_at_bounds(&args[0], len);
            }
            if method == "sample"
                && let Some(len) = self.known_array_len(object, object_ty)
            {
                self.check_sample_bounds(&args[0], len);
            }
            if matches!(method, "random" | "shuffle")
                && let Some(len) = self.known_array_len(object, object_ty)
            {
                self.check_random_array_minimum_length(method, object, len);
            }
            return Some(optional_value_type(
                ValueType::Known(return_type),
                optional_receiver,
            ));
        }
        None
    }
}
