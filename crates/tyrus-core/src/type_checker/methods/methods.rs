use super::*;

impl TypeChecker {
    pub(super) fn check_map_method(
        &mut self,
        object: &Expr,
        key: &Type,
        value: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let _ = object;
        let (minimum, maximum, return_type) = match method {
            "set" => (
                2,
                2,
                Type::Map(Box::new(key.clone()), Box::new(value.clone())),
            ),
            "get" => (1, 1, Type::Optional(Box::new(value.clone()))),
            "getOrInsert" => (2, 2, Type::MutRef(Box::new(value.clone()))),
            "has" | "delete" => (1, 1, Type::Bool),
            "keys" => (0, 0, Type::Array(Box::new(key.clone()))),
            "values" => (0, 0, Type::Array(Box::new(value.clone()))),
            "entries" => (
                0,
                0,
                Type::Array(Box::new(Type::Tuple(vec![key.clone(), value.clone()]))),
            ),
            _ => {
                self.error(
                    "E0599",
                    format!(
                        "no method named `{method}` for a map; use `set`, `get`, `getOrInsert`, `has`, `delete`, `keys`, `values` or `entries` (iterate entries with `for...of`)"
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        for (index, arg) in args.iter().enumerate() {
            let expected = if index == 0 { key } else { value };
            let actual = self.check_expr_as(arg, expected);
            self.require_assignable(expected, &actual, method, "E0308");
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_set_method(
        &mut self,
        _object: &Expr,
        element: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let (minimum, maximum, return_type) = match method {
            "add" => (1, 1, Type::Set(Box::new(element.clone()))),
            "has" | "delete" => (1, 1, Type::Bool),
            "values" => (0, 0, Type::Array(Box::new(element.clone()))),
            _ => {
                self.error(
                    "E0599",
                    format!(
                        "no method named `{method}` for a set; use `add`, `has`, `delete` or `values`"
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        for arg in args {
            let actual = self.check_expr_as(arg, element);
            self.require_assignable(element, &actual, method, "E0308");
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_union_find_method(
        &mut self,
        _object: &Expr,
        element: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let (minimum, maximum, return_type) = match method {
            "find" => (1, 1, element.clone()),
            "union" | "connected" => (2, 2, Type::Bool),
            "setCount" => (0, 0, Type::Usize),
            _ => {
                self.error(
                    "E0599",
                    format!(
                        "no method named `{method}` for `UnionFind<{element}>`; use `find`, `union`, `connected`, or `setCount`"
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        for arg in args {
            let actual = self.check_expr_as(arg, element);
            self.require_assignable(element, &actual, method, "E0308");
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_binomial_heap_method(
        &mut self,
        _object: &Expr,
        element: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let (minimum, maximum, return_type) = match method {
            "push" => (1, 1, Type::Void),
            "pop" | "peek" => (0, 0, Type::Optional(Box::new(element.clone()))),
            "len" => (0, 0, Type::Usize),
            "isEmpty" => (0, 0, Type::Bool),
            "clear" => (0, 0, Type::Void),
            _ => {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for `BinomialHeap<{element}>`"),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        for arg in args {
            let actual = self.check_expr_as(arg, element);
            self.require_assignable(element, &actual, method, "E0308");
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_counter_method(
        &mut self,
        _object: &Expr,
        element: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let i32_ty = Type::I32;
        let (minimum, maximum, return_type) = match method {
            "add" => (1, 2, Type::Void),
            "remove" => (1, 2, Type::Void),
            "count" => (1, 1, Type::I32),
            "mostCommon" => (
                1,
                1,
                Type::Array(Box::new(Type::Tuple(vec![element.clone(), Type::I32]))),
            ),
            "total" => (0, 0, Type::I32),
            "unique" => (0, 0, Type::Usize),
            "isEmpty" => (0, 0, Type::Bool),
            "clear" => (0, 0, Type::Void),
            _ => {
                self.error(
                    "E0599",
                    format!(
                        "no method named `{method}` for `Counter<{element}>`; use `add`, `remove`, `count`, `mostCommon`, `total`, `unique`, `isEmpty`, or `clear`"
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        match method {
            "add" | "remove" => {
                if let Some(item_arg) = args.first() {
                    let actual = self.check_expr_as(item_arg, element);
                    self.require_assignable(element, &actual, method, "E0308");
                }
                if let Some(count_arg) = args.get(1) {
                    let actual = self.check_expr_as(count_arg, &i32_ty);
                    self.require_assignable(&i32_ty, &actual, method, "E0308");
                }
            }
            "count" => {
                if let Some(item_arg) = args.first() {
                    let actual = self.check_expr_as(item_arg, element);
                    self.require_assignable(element, &actual, method, "E0308");
                }
            }
            "mostCommon" => {
                if let Some(k_arg) = args.first() {
                    let actual = self.check_expr_as(k_arg, &Type::Usize);
                    self.require_assignable(&Type::Usize, &actual, method, "E0308");
                }
            }
            _ => {}
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_deque_method(
        &mut self,
        _object: &Expr,
        element: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let (minimum, maximum, return_type) = match method {
            "pushFront" | "pushBack" => (1, 1, Type::Void),
            "popFront" | "popBack" => (0, 0, Type::Optional(Box::new(element.clone()))),
            "peekFront" | "peekBack" => (0, 0, Type::Optional(Box::new(element.clone()))),
            "len" => (0, 0, Type::Usize),
            "isEmpty" => (0, 0, Type::Bool),
            "clear" => (0, 0, Type::Void),
            _ => {
                self.error(
                    "E0599",
                    format!(
                        "no method named `{method}` for `Deque<{element}>`; use `pushFront`, `pushBack`, `popFront`, `popBack`, `peekFront`, `peekBack`, `len`, `isEmpty`, or `clear`"
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        match method {
            "pushFront" | "pushBack" => {
                if let Some(item_arg) = args.first() {
                    let actual = self.check_expr_as(item_arg, element);
                    self.require_assignable(element, &actual, method, "E0308");
                }
            }
            _ => {}
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_fenwick_tree_method(
        &mut self,
        _object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let usize_ty = Type::Usize;
        let (minimum, maximum, return_type) = match method {
            "add" => (2, 2, Type::Void),
            "sum" => (1, 1, Type::I32),
            "len" => (0, 0, Type::Usize),
            "isEmpty" => (0, 0, Type::Bool),
            _ => {
                self.error(
                    "E0599",
                    format!(
                        "no method named `{method}` for `FenwickTree`; use `add`, `sum`, `len`, or `isEmpty`"
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        match method {
            "add" => {
                if let Some(index_arg) = args.first() {
                    let actual = self.check_expr_as(index_arg, &usize_ty);
                    self.require_assignable(&usize_ty, &actual, method, "E0308");
                }
                if let Some(delta_arg) = args.get(1) {
                    let actual = self.check_expr_as(delta_arg, &Type::I32);
                    self.require_assignable(&Type::I32, &actual, method, "E0308");
                }
            }
            "sum" => {
                if let Some(index_arg) = args.first() {
                    let actual = self.check_expr_as(index_arg, &usize_ty);
                    self.require_assignable(&usize_ty, &actual, method, "E0308");
                }
            }
            _ => {}
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_bubble_sort_method(&mut self, method: &str, args: &[Expr]) -> ValueType {
        let return_type = match method {
            "step" | "done" | "swapped" => Type::Bool,
            "run" | "values" => Type::Array(Box::new(Type::I32)),
            "left" | "right" | "pass" | "comparisons" | "swaps" => Type::Usize,
            "reset" => Type::Void,
            _ => {
                self.error(
                    "E0599",
                    format!(
                        "no method named `{method}` for `BubbleSort`; use `step`, `run`, `values`, `done`, `left`, `right`, `swapped`, `pass`, `comparisons`, `swaps`, or `reset`"
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if !args.is_empty() {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects 0 arguments, but {} were provided",
                    args.len()
                ),
                Some(method),
            );
            for arg in args {
                self.check_expr(arg);
            }
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_segment_tree_method(
        &mut self,
        _object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let usize_ty = Type::Usize;
        let (minimum, maximum, return_type) = match method {
            "update" => (2, 2, Type::Void),
            "query" => (2, 2, Type::I32),
            "len" => (0, 0, Type::Usize),
            "isEmpty" => (0, 0, Type::Bool),
            _ => {
                self.error(
                    "E0599",
                    format!(
                        "no method named `{method}` for `SegmentTree`; use `update`, `query`, `len`, or `isEmpty`"
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        match method {
            "update" => {
                if let Some(index_arg) = args.first() {
                    let actual = self.check_expr_as(index_arg, &usize_ty);
                    self.require_assignable(&usize_ty, &actual, method, "E0308");
                }
                if let Some(value_arg) = args.get(1) {
                    let actual = self.check_expr_as(value_arg, &Type::I32);
                    self.require_assignable(&Type::I32, &actual, method, "E0308");
                }
            }
            "query" => {
                for arg in args {
                    let actual = self.check_expr_as(arg, &usize_ty);
                    self.require_assignable(&usize_ty, &actual, method, "E0308");
                }
            }
            _ => {}
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_segment_tree_lazy_method(
        &mut self,
        _object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let usize_ty = Type::Usize;
        let (minimum, maximum, return_type) = match method {
            "updateRange" => (3, 3, Type::Void),
            "query" => (2, 2, Type::I32),
            "len" => (0, 0, Type::Usize),
            "isEmpty" => (0, 0, Type::Bool),
            _ => {
                self.error(
                    "E0599",
                    format!(
                        "no method named `{method}` for `SegmentTreeLazy`; use `updateRange`, `query`, `len`, or `isEmpty`"
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        match method {
            "updateRange" => {
                for arg in args.iter().take(2) {
                    let actual = self.check_expr_as(arg, &usize_ty);
                    self.require_assignable(&usize_ty, &actual, method, "E0308");
                }
                if let Some(delta_arg) = args.get(2) {
                    let actual = self.check_expr_as(delta_arg, &Type::I32);
                    self.require_assignable(&Type::I32, &actual, method, "E0308");
                }
            }
            "query" => {
                for arg in args {
                    let actual = self.check_expr_as(arg, &usize_ty);
                    self.require_assignable(&usize_ty, &actual, method, "E0308");
                }
            }
            _ => {}
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_sparse_table_method(
        &mut self,
        _object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let usize_ty = Type::Usize;
        let (minimum, maximum, return_type) = match method {
            "query" => (2, 2, Type::I32),
            "len" => (0, 0, Type::Usize),
            "isEmpty" => (0, 0, Type::Bool),
            _ => {
                self.error(
                    "E0599",
                    format!(
                        "no method named `{method}` for `SparseTable`; use `query`, `len`, or `isEmpty`"
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        if method == "query" {
            for arg in args {
                let actual = self.check_expr_as(arg, &usize_ty);
                self.require_assignable(&usize_ty, &actual, method, "E0308");
            }
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_bit_set_method(
        &mut self,
        _object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let usize_ty = Type::Usize;
        let (minimum, maximum, return_type) = match method {
            "set" => (1, 2, Type::Void),
            "flip" => (1, 1, Type::Void),
            "clear" => (0, 0, Type::Void),
            "get" => (1, 1, Type::Bool),
            "count" => (0, 0, Type::Usize),
            "isEmpty" => (0, 0, Type::Bool),
            "isSubset" | "isSuperset" => (1, 1, Type::Bool),
            "union" | "intersection" | "difference" => (1, 1, Type::BitSet),
            "complement" => (0, 0, Type::BitSet),
            _ => {
                self.error(
                    "E0599",
                    format!(
                        "no method named `{method}` for `BitSet`; use `set`, `flip`, `clear`, `get`, `count`, `isEmpty`, `isSubset`, `isSuperset`, `union`, `intersection`, `difference`, or `complement`"
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        match method {
            "set" => {
                if let Some(index_arg) = args.first() {
                    let actual = self.check_expr_as(index_arg, &usize_ty);
                    self.require_assignable(&usize_ty, &actual, method, "E0308");
                }
                if let Some(value_arg) = args.get(1) {
                    let actual = self.check_expr_as(value_arg, &Type::Bool);
                    self.require_assignable(&Type::Bool, &actual, method, "E0308");
                }
            }
            "flip" | "get" => {
                if let Some(index_arg) = args.first() {
                    let actual = self.check_expr_as(index_arg, &usize_ty);
                    self.require_assignable(&usize_ty, &actual, method, "E0308");
                }
            }
            "isSubset" | "isSuperset" | "union" | "intersection" | "difference" => {
                if let Some(other_arg) = args.first() {
                    let actual = self.check_expr_as(other_arg, &Type::BitSet);
                    self.require_assignable(&Type::BitSet, &actual, method, "E0308");
                }
            }
            _ => {}
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_trie_method(
        &mut self,
        _object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let (minimum, maximum, return_type) = match method {
            "insert" => (1, 1, Type::Void),
            "search" | "startsWith" => (1, 1, Type::Bool),
            "len" => (0, 0, Type::Usize),
            "isEmpty" => (0, 0, Type::Bool),
            "clear" => (0, 0, Type::Void),
            _ => {
                self.error(
                    "E0599",
                    format!(
                        "no method named `{method}` for `Trie`; use `insert`, `search`, `startsWith`, `len`, `isEmpty`, or `clear`"
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        match method {
            "insert" | "search" | "startsWith" => {
                if let Some(word_arg) = args.first() {
                    let actual = self.check_expr_as(word_arg, &Type::Str);
                    self.require_assignable(&Type::Str, &actual, method, "E0308");
                }
            }
            _ => {}
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_suffix_tree_method(
        &mut self,
        _object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let (minimum, maximum, return_type) = match method {
            "getSuffixArray" => (0, 0, Type::Array(Box::new(Type::Usize))),
            _ => {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for `SuffixTree`; use `getSuffixArray`"),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_lca_method(
        &mut self,
        _object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let (minimum, maximum, return_type) = match method {
            "query" => (2, 2, Type::Usize),
            _ => {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for `Lca`; use `query`"),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        for arg in args.iter().take(maximum) {
            let actual = self.check_expr_as(arg, &Type::Usize);
            self.require_assignable(&Type::Usize, &actual, method, "E0308");
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_hld_method(
        &mut self,
        _object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let range_type = Type::Array(Box::new(Type::Tuple(vec![Type::Usize, Type::Usize])));
        let (minimum, maximum, return_type, arg_types) = match method {
            "queryPath" => (2, 2, range_type, vec![Type::Usize, Type::Usize]),
            "position" => (1, 1, Type::Usize, vec![Type::Usize]),
            "head" => (1, 1, Type::Usize, vec![Type::Usize]),
            _ => {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for `Hld`; use `queryPath`"),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        for (arg, expected) in args.iter().zip(arg_types.iter()) {
            let actual = self.check_expr_as(arg, expected);
            self.require_assignable(expected, &actual, method, "E0308");
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_centroid_method(
        &mut self,
        _object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let parent_type = Type::Optional(Box::new(Type::Usize));
        let (minimum, maximum, return_type, arg_types) = match method {
            "parentOf" => (1, 1, parent_type, vec![Type::Usize]),
            "depth" => (1, 1, Type::Usize, vec![Type::Usize]),
            _ => {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for `Centroid`; use `parentOf`"),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        for (arg, expected) in args.iter().zip(arg_types.iter()) {
            let actual = self.check_expr_as(arg, expected);
            self.require_assignable(expected, &actual, method, "E0308");
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_bloom_filter_method(
        &mut self,
        _object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let (minimum, maximum, return_type, arg_types) = match method {
            "insert" => (1, 1, Type::Void, vec![Type::Str]),
            "contains" => (1, 1, Type::Bool, vec![Type::Str]),
            _ => {
                self.error(
                    "E0599",
                    format!(
                        "no method named `{method}` for `BloomFilter`; use `insert` or `contains`"
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        for (arg, expected) in args.iter().zip(arg_types.iter()) {
            let actual = self.check_expr_as(arg, expected);
            self.require_assignable(expected, &actual, method, "E0308");
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_linked_list_method(
        &mut self,
        _object: &Expr,
        element: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let (minimum, maximum, return_type) = match method {
            "pushFront" | "pushBack" => (1, 1, Type::Void),
            "popFront" | "popBack" | "front" | "back" => {
                (0, 0, Type::Optional(Box::new(element.clone())))
            }
            "len" => (0, 0, Type::Usize),
            "isEmpty" => (0, 0, Type::Bool),
            "clear" => (0, 0, Type::Void),
            _ => {
                self.error(
                    "E0599",
                    format!(
                        "no method named `{method}` for `LinkedList<{element}>`; use `pushFront`, `pushBack`, `popFront`, `popBack`, `front`, `back`, `len`, `isEmpty`, or `clear`"
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        match method {
            "pushFront" | "pushBack" => {
                if let Some(item_arg) = args.first() {
                    let actual = self.check_expr_as(item_arg, element);
                    self.require_assignable(element, &actual, method, "E0308");
                }
            }
            _ => {}
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_double_linked_list_method(
        &mut self,
        _object: &Expr,
        element: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let (minimum, maximum, return_type) = match method {
            "pushFront" | "pushBack" => (1, 1, Type::Void),
            "popFront" | "popBack" | "front" | "back" => {
                (0, 0, Type::Optional(Box::new(element.clone())))
            }
            "len" => (0, 0, Type::Usize),
            "isEmpty" => (0, 0, Type::Bool),
            "clear" => (0, 0, Type::Void),
            _ => {
                self.error(
                    "E0599",
                    format!(
                        "no method named `{method}` for `DoubleLinkedList<{element}>`; use `pushFront`, `pushBack`, `popFront`, `popBack`, `front`, `back`, `len`, `isEmpty`, or `clear`"
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        match method {
            "pushFront" | "pushBack" => {
                if let Some(item_arg) = args.first() {
                    let actual = self.check_expr_as(item_arg, element);
                    self.require_assignable(element, &actual, method, "E0308");
                }
            }
            _ => {}
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_ordered_tree_method(
        &mut self,
        _object: &Expr,
        element: &Type,
        type_name: &str,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let (minimum, maximum, return_type) = match method {
            "insert" => (1, 1, Type::Void),
            "contains" => (1, 1, Type::Bool),
            "len" => (0, 0, Type::Usize),
            "isEmpty" => (0, 0, Type::Bool),
            _ => {
                self.error(
                    "E0599",
                    format!(
                        "no method named `{method}` for `{type_name}<{element}>`; use `insert`, `contains`, `len`, or `isEmpty`"
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        if (method == "insert" || method == "contains")
            && let Some(item_arg) = args.first()
        {
            let actual = self.check_expr_as(item_arg, element);
            self.require_assignable(element, &actual, method, "E0308");
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_sorted_set_method(
        &mut self,
        _object: &Expr,
        element: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let (minimum, maximum, return_type) = match method {
            "insert" | "remove" | "contains" => (1, 1, Type::Bool),
            "min" | "max" => (0, 0, Type::Optional(Box::new(element.clone()))),
            "len" => (0, 0, Type::Usize),
            "isEmpty" => (0, 0, Type::Bool),
            "clear" => (0, 0, Type::Void),
            _ => {
                self.error(
                    "E0599",
                    format!(
                        "no method named `{method}` for `SortedSet<{element}>`; use `insert`, `remove`, `contains`, `min`, `max`, `len`, `isEmpty`, or `clear`"
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        if (method == "insert" || method == "remove" || method == "contains")
            && let Some(item_arg) = args.first()
        {
            let actual = self.check_expr_as(item_arg, element);
            self.require_assignable(element, &actual, method, "E0308");
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_skip_list_method(
        &mut self,
        _object: &Expr,
        element: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let (minimum, maximum, return_type) = match method {
            "insert" | "remove" | "contains" => (1, 1, Type::Bool),
            "min" | "max" => (0, 0, Type::Optional(Box::new(element.clone()))),
            "len" => (0, 0, Type::Usize),
            "isEmpty" => (0, 0, Type::Bool),
            "clear" => (0, 0, Type::Void),
            _ => {
                self.error(
                    "E0599",
                    format!(
                        "no method named `{method}` for `SkipList<{element}>`; use `insert`, `remove`, `contains`, `min`, `max`, `len`, `isEmpty`, or `clear`"
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        if (method == "insert" || method == "remove" || method == "contains")
            && let Some(item_arg) = args.first()
        {
            let actual = self.check_expr_as(item_arg, element);
            self.require_assignable(element, &actual, method, "E0308");
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_sorted_map_method(
        &mut self,
        _object: &Expr,
        key: &Type,
        value: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let (minimum, maximum, return_type) = match method {
            "set" => (2, 2, Type::Void),
            "get" => (1, 1, Type::Optional(Box::new(value.clone()))),
            "has" | "delete" => (1, 1, Type::Bool),
            "min" | "max" => (0, 0, Type::Optional(Box::new(key.clone()))),
            "len" => (0, 0, Type::Usize),
            "isEmpty" => (0, 0, Type::Bool),
            "clear" => (0, 0, Type::Void),
            _ => {
                self.error(
                    "E0599",
                    format!(
                        "no method named `{method}` for `SortedMap<{key}, {value}>`; use `set`, `get`, `has`, `delete`, `min`, `max`, `len`, `isEmpty`, or `clear`"
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        if method == "set" {
            if let Some(key_arg) = args.first() {
                let actual = self.check_expr_as(key_arg, key);
                self.require_assignable(key, &actual, method, "E0308");
            }
            if let Some(value_arg) = args.get(1) {
                let actual = self.check_expr_as(value_arg, value);
                self.require_assignable(value, &actual, method, "E0308");
            }
        } else if (method == "get" || method == "has" || method == "delete")
            && let Some(key_arg) = args.first()
        {
            let actual = self.check_expr_as(key_arg, key);
            self.require_assignable(key, &actual, method, "E0308");
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_graph_method(
        &mut self,
        _object: &Expr,
        node: &Type,
        weight: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let (minimum, maximum, return_type) = match method {
            "addEdge" => (3, 3, Type::Void),
            "addNode" => (1, 1, Type::Void),
            "removeNode" | "hasNode" => (1, 1, Type::Bool),
            "removeEdge" | "hasEdge" => (2, 2, Type::Bool),
            "edge" => (2, 2, Type::Optional(Box::new(weight.clone()))),
            "neighbors" => (1, 1, Type::Array(Box::new(node.clone()))),
            "degree" => (1, 1, Type::Usize),
            "nodeCount" | "edgeCount" => (0, 0, Type::Usize),
            "isEmpty" => (0, 0, Type::Bool),
            "nodes" => (0, 0, Type::Array(Box::new(node.clone()))),
            "edges" => (
                0,
                0,
                Type::Array(Box::new(Type::Tuple(vec![
                    node.clone(),
                    node.clone(),
                    weight.clone(),
                ]))),
            ),
            "reverse" => (
                0,
                0,
                Type::Graph(Box::new(node.clone()), Box::new(weight.clone())),
            ),
            "toDot" | "toString" => (0, 0, Type::Str),
            "mapEdges" => {
                let callback = match args.first() {
                    Some(callback) => callback,
                    None => {
                        self.error(
                            "E0061",
                            format!(
                                "`{method}` expects 1 argument, but {} were provided",
                                args.len()
                            ),
                            Some(method),
                        );
                        return ValueType::Error;
                    }
                };
                let new_weight = self
                    .check_array_callback(
                        callback,
                        std::slice::from_ref(weight),
                        &Type::Infer,
                        method,
                    )
                    .unwrap_or(weight.clone());
                if !is_numeric_type(&new_weight) {
                    self.error(
                        "E0277",
                        format!("`{method}` callback must return a numeric weight type, found `{new_weight}`"),
                        Some(method),
                    );
                    return ValueType::Error;
                }
                (
                    1,
                    1,
                    Type::Graph(Box::new(node.clone()), Box::new(new_weight)),
                )
            }
            "mapNodes" => {
                let callback = match args.first() {
                    Some(callback) => callback,
                    None => {
                        self.error(
                            "E0061",
                            format!(
                                "`{method}` expects 1 argument, but {} were provided",
                                args.len()
                            ),
                            Some(method),
                        );
                        return ValueType::Error;
                    }
                };
                let new_node = self
                    .check_array_callback(
                        callback,
                        std::slice::from_ref(node),
                        &Type::Infer,
                        method,
                    )
                    .unwrap_or(node.clone());
                if !ordered_tree_element_allowed(&new_node) {
                    self.error(
                        "E0277",
                        format!("`{method}` callback must return an ordered node type, found `{new_node}`"),
                        Some(method),
                    );
                    return ValueType::Error;
                }
                (
                    1,
                    1,
                    Type::Graph(Box::new(new_node), Box::new(weight.clone())),
                )
            }
            _ => {
                self.error(
                    "E0599",
                    format!(
                        "no method named `{method}` for `Graph<{node}, {weight}>`; use `addEdge`, `addNode`, `removeNode`, `removeEdge`, `hasNode`, `hasEdge`, `edge`, `neighbors`, `degree`, `nodeCount`, `edgeCount`, `isEmpty`, `nodes`, `edges`, `reverse`, `mapEdges`, `mapNodes`, `toDot`, or `toString`"
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
        };
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects {minimum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        match method {
            "addEdge" => {
                for (index, arg) in args.iter().enumerate() {
                    let expected = if index < 2 { node } else { weight };
                    let actual = self.check_expr_as(arg, expected);
                    self.require_assignable(expected, &actual, method, "E0308");
                }
            }
            "addNode" | "removeNode" | "hasNode" | "neighbors" | "degree" => {
                if let Some(arg) = args.first() {
                    let actual = self.check_expr_as(arg, node);
                    self.require_assignable(node, &actual, method, "E0308");
                }
            }
            "removeEdge" | "hasEdge" | "edge" => {
                for arg in args.iter().take(2) {
                    let actual = self.check_expr_as(arg, node);
                    self.require_assignable(node, &actual, method, "E0308");
                }
            }
            _ => {}
        }
        ValueType::Known(return_type)
    }

    pub(super) fn check_algorithm_static_method(
        &mut self,
        type_name: &str,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        if method != "fromArray" {
            self.error(
                "E0599",
                format!("no static method named `{method}` on `{type_name}`; use `fromArray`"),
                Some(method),
            );
            for arg in args {
                self.check_expr(arg);
            }
            return ValueType::Error;
        }
        if args.len() != 1 {
            self.error(
                "E0061",
                format!(
                    "`{type_name}.fromArray` expects 1 argument, but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        let source = args.first();
        let source_ty = source.map(|arg| self.check_expr(arg));
        let source_as_type = source_ty.as_ref().and_then(value_type_to_type);
        let array_element = source_as_type.as_ref().and_then(|ty| match ty {
            Type::Array(element) => Some(element.as_ref().clone()),
            _ => None,
        });
        if let Some(ty) = &source_ty
            && !matches!(value_type_to_type(ty), Some(Type::Array(_)))
        {
            self.error(
                "E0277",
                format!("`{type_name}.fromArray` requires an array argument"),
                expression_subject(source.as_ref().unwrap()).or(Some(method)),
            );
        }
        match type_name {
            "BitSet" => {
                if let Some(element) = &array_element {
                    self.require_assignable(
                        &Type::Usize,
                        &ValueType::Known(element.clone()),
                        method,
                        "E0308",
                    );
                }
                ValueType::Known(Type::BitSet)
            }
            "LinkedList" => {
                let element = array_element.unwrap_or(Type::Unknown);
                if element == Type::Unknown {
                    self.error(
                        "E0282",
                        "cannot infer element type of `LinkedList.fromArray`; the array argument must have a settled element type"
                            .to_string(),
                        source.and_then(expression_subject),
                    );
                }
                ValueType::Known(Type::LinkedList(Box::new(element)))
            }
            "DoubleLinkedList" => {
                let element = array_element.unwrap_or(Type::Unknown);
                if element == Type::Unknown {
                    self.error(
                        "E0282",
                        "cannot infer element type of `DoubleLinkedList.fromArray`; the array argument must have a settled element type"
                            .to_string(),
                        source.and_then(expression_subject),
                    );
                }
                ValueType::Known(Type::DoubleLinkedList(Box::new(element)))
            }
            _ => ValueType::Error,
        }
    }

    pub(super) fn check_range_method(
        &mut self,
        _object: &Expr,
        element: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        if method != "contains" {
            self.error(
                "E0599",
                format!("no method named `{method}` for a Range; use `contains`"),
                Some(method),
            );
            for arg in args {
                self.check_expr(arg);
            }
            return ValueType::Error;
        }
        if args.len() != 1 {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects 1 argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        for arg in args {
            let actual = self.check_expr_as(arg, element);
            self.require_assignable(element, &actual, method, "E0308");
        }
        ValueType::Known(Type::Bool)
    }

    pub(super) fn check_join_handle_instance_method(
        &mut self,
        payload: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        if !args.is_empty() {
            self.error(
                "E0061",
                format!(
                    "`JoinHandle.{method}` expects 0 arguments, but {} were provided",
                    args.len()
                ),
                Some(method),
            );
            for argument in args {
                self.check_expr(argument);
            }
        }
        match method {
            "join" => ValueType::Known(promise_of(payload.clone())),
            "abort" => ValueType::Known(Type::Void),
            _ => {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for `JoinHandle<{payload}>`"),
                    Some(method),
                );
                ValueType::Error
            }
        }
    }

    pub(super) fn check_sender_instance_method(
        &mut self,
        payload: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        match method {
            "send" => {
                if args.len() != 1 {
                    self.error(
                        "E0061",
                        format!(
                            "`Sender.send` expects 1 argument, but {} were provided",
                            args.len()
                        ),
                        Some(method),
                    );
                }
                for (index, arg) in args.iter().enumerate() {
                    let actual = self.check_expr(arg);
                    if index == 0 {
                        self.require_assignable(payload, &actual, method, "E0308");
                    }
                }
                ValueType::Known(promise_of(Type::Void))
            }
            "trySend" => {
                if args.len() != 1 {
                    self.error(
                        "E0061",
                        format!(
                            "`Sender.trySend` expects 1 argument, but {} were provided",
                            args.len()
                        ),
                        Some(method),
                    );
                }
                for (index, arg) in args.iter().enumerate() {
                    let actual = self.check_expr(arg);
                    if index == 0 {
                        self.require_assignable(payload, &actual, method, "E0308");
                    }
                }
                ValueType::Known(Type::Bool)
            }
            _ => {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for `Sender<{payload}>`"),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                ValueType::Error
            }
        }
    }

    pub(super) fn check_receiver_instance_method(
        &mut self,
        payload: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        if !args.is_empty() {
            self.error(
                "E0061",
                format!(
                    "`Receiver.{method}` expects 0 arguments, but {} were provided",
                    args.len()
                ),
                Some(method),
            );
            for argument in args {
                self.check_expr(argument);
            }
        }
        match method {
            "recv" => ValueType::Known(promise_of(Type::Optional(Box::new(payload.clone())))),
            "tryRecv" => ValueType::Known(Type::Optional(Box::new(payload.clone()))),
            "close" => ValueType::Known(Type::Void),
            _ => {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for `Receiver<{payload}>`"),
                    Some(method),
                );
                ValueType::Error
            }
        }
    }

    pub(super) fn check_readable_stream_instance_method(
        &mut self,
        payload: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        match method {
            "read" | "close" if args.is_empty() => ValueType::Known(if method == "read" {
                promise_of(Type::Optional(Box::new(payload.clone())))
            } else {
                promise_of(Type::Void)
            }),
            "pipeTo" if args.len() == 1 => {
                self.check_expr_as(
                    &args[0],
                    &Type::Apply {
                        name: "WritableStream".to_string(),
                        args: vec![payload.clone()],
                    },
                );
                ValueType::Known(promise_of(Type::Void))
            }
            "read" | "close" | "pipeTo" => {
                self.error(
                    "E0061",
                    format!(
                        "`ReadableStream.{method}` expects {} argument(s), but {} were provided",
                        usize::from(method == "pipeTo"),
                        args.len()
                    ),
                    Some(method),
                );
                for argument in args {
                    self.check_expr(argument);
                }
                ValueType::Error
            }
            _ => {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for `ReadableStream<{payload}>`"),
                    Some(method),
                );
                for argument in args {
                    self.check_expr(argument);
                }
                ValueType::Error
            }
        }
    }

    pub(super) fn check_writable_stream_instance_method(
        &mut self,
        payload: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        match method {
            "write" if args.len() == 1 => {
                self.check_expr_as(&args[0], payload);
                ValueType::Known(promise_of(Type::Void))
            }
            "close" if args.is_empty() => ValueType::Known(promise_of(Type::Void)),
            "write" | "close" => {
                self.error(
                    "E0061",
                    format!(
                        "`WritableStream.{method}` expects {} argument(s), but {} were provided",
                        usize::from(method == "write"),
                        args.len()
                    ),
                    Some(method),
                );
                for argument in args {
                    self.check_expr(argument);
                }
                ValueType::Error
            }
            _ => {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for `WritableStream<{payload}>`"),
                    Some(method),
                );
                for argument in args {
                    self.check_expr(argument);
                }
                ValueType::Error
            }
        }
    }

    pub(super) fn check_composite_stream_instance_method(
        &mut self,
        stream_name: &str,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        if method == "close" && args.is_empty() {
            return ValueType::Known(promise_of(Type::Void));
        }
        if method == "close" {
            self.error(
                "E0061",
                format!(
                    "`{stream_name}.close` expects 0 arguments, but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        } else {
            self.error(
                "E0599",
                format!("no method named `{method}` for `{stream_name}`"),
                Some(method),
            );
        }
        for argument in args {
            self.check_expr(argument);
        }
        ValueType::Error
    }

    pub(super) fn check_mutex_instance_method(
        &mut self,
        payload: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        if !args.is_empty() {
            self.error(
                "E0061",
                format!(
                    "`Mutex.{method}` expects 0 arguments, but {} were provided",
                    args.len()
                ),
                Some(method),
            );
            for argument in args {
                self.check_expr(argument);
            }
        }
        let guard_of = |payload: &Type| Type::Apply {
            name: "MutexGuard".to_string(),
            args: vec![payload.clone()],
        };
        match method {
            "lock" => ValueType::Known(promise_of(guard_of(payload))),
            "tryLock" => ValueType::Known(Type::Optional(Box::new(guard_of(payload)))),
            _ => {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for `Mutex<{payload}>`"),
                    Some(method),
                );
                ValueType::Error
            }
        }
    }

    pub(super) fn check_rwlock_instance_method(
        &mut self,
        payload: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        if !args.is_empty() {
            self.error(
                "E0061",
                format!(
                    "`RwLock.{method}` expects 0 arguments, but {} were provided",
                    args.len()
                ),
                Some(method),
            );
            for argument in args {
                self.check_expr(argument);
            }
        }
        let read_guard_of = |payload: &Type| Type::Apply {
            name: "RwReadGuard".to_string(),
            args: vec![payload.clone()],
        };
        let write_guard_of = |payload: &Type| Type::Apply {
            name: "RwWriteGuard".to_string(),
            args: vec![payload.clone()],
        };
        match method {
            "read" => ValueType::Known(promise_of(read_guard_of(payload))),
            "write" => ValueType::Known(promise_of(write_guard_of(payload))),
            "tryRead" => ValueType::Known(Type::Optional(Box::new(read_guard_of(payload)))),
            "tryWrite" => ValueType::Known(Type::Optional(Box::new(write_guard_of(payload)))),
            _ => {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for `RwLock<{payload}>`"),
                    Some(method),
                );
                ValueType::Error
            }
        }
    }

    pub(super) fn check_semaphore_instance_method(
        &mut self,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        if !args.is_empty() {
            self.error(
                "E0061",
                format!(
                    "`Semaphore.{method}` expects 0 arguments, but {} were provided",
                    args.len()
                ),
                Some(method),
            );
            for argument in args {
                self.check_expr(argument);
            }
        }
        let permit = Type::Named("SemaphorePermit".to_string());
        match method {
            "acquire" => ValueType::Known(promise_of(permit)),
            "tryAcquire" => ValueType::Known(Type::Optional(Box::new(permit))),
            _ => {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for type `Semaphore`"),
                    Some(method),
                );
                ValueType::Error
            }
        }
    }

    pub(super) fn check_object_projection_method(
        &mut self,
        object: &Expr,
        receiver_ty: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let fields = match transparent_type(receiver_ty) {
            Type::Object(fields) => Some(fields.clone()),
            Type::Named(name) => self.interfaces.get(name).cloned(),
            _ => None,
        };
        let Some(fields) = fields else {
            self.error_at(
                "E0599",
                format!(
                    "no method named `{method}` for type `{receiver_ty}`; `{method}` requires an object receiver"
                ),
                object.span,
            );
            for arg in args {
                self.check_expr(arg);
            }
            return ValueType::Error;
        };
        if args.len() != 1 {
            self.error(
                "E0061",
                format!(
                    "`{method}` expects 1 argument (an array of static field names), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
            for arg in args {
                self.check_expr(arg);
            }
            return ValueType::Error;
        }
        let Some(keys) = self.static_object_projection_keys(&args[0], method) else {
            return ValueType::Error;
        };
        let mut valid = true;
        for key in &keys {
            if !fields.iter().any(|field| field.name == key.name) {
                self.error_at(
                    "E0609",
                    format!(
                        "no field `{}` on `{receiver_ty}`; `{method}` only accepts fields of its receiver",
                        key.name
                    ),
                    key.span,
                );
                valid = false;
            }
        }
        if !valid {
            return ValueType::Error;
        }
        let mode = if method == "pick" {
            ObjectProjectionMode::Pick
        } else {
            ObjectProjectionMode::Omit
        };
        ValueType::Known(Type::Object(project_object_fields(
            fields,
            keys.into_iter().map(|key| key.name),
            mode,
        )))
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn check_test_assertion_method(
        &mut self,
        method: &str,
        method_span: Option<Span>,
        args: &[Expr],
        expectation: Option<(&Expr, crate::test::Expectation)>,
    ) -> ValueType {
        let (actual, chain) = match expectation {
            Some((actual, chain)) => (Some(actual), chain),
            None => (None, crate::test::Expectation::default()),
        };
        if chain.resolves && chain.rejects {
            self.error_at_member(
                "E0599",
                "`.resolves` and `.rejects` ask opposite questions of the same promise".to_string(),
                method_span,
                method,
            );
            for arg in args {
                self.check_expr(arg);
            }
            return ValueType::Error;
        }
        if chain.resolves && method == "toThrow" {
            self.error_at_member(
                "E0599",
                "`.toThrow()` does not follow `.resolves`".to_string(),
                method_span,
                method,
            );
            if let Some(last) = self.diagnostics.last_mut() {
                last.help =
                    vec!["assert that a promise failed with `.rejects.toThrow(...)`".to_string()];
            }
            for arg in args {
                self.check_expr(arg);
            }
            return ValueType::Error;
        }
        if let Some((actual, modifier)) = actual.zip(match (chain.resolves, chain.rejects) {
            (true, _) => Some(crate::test::RESOLVES_MODIFIER),
            (_, true) => Some(crate::test::REJECTS_MODIFIER),
            _ => None,
        }) {
            let before = self.diagnostics.len();
            let settled = value_type_to_type(&self.check_expr(actual)).unwrap_or(Type::Unknown);
            self.diagnostics.truncate(before);
            match promise_payload(&settled) {
                Some(Type::Void) if chain.resolves => {
                    self.error_on_expr(
                        "E0277",
                        "`.resolves` needs a promise that produces a value, and this one produces `void`".to_string(),
                        actual,
                    );
                    if let Some(last) = self.diagnostics.last_mut() {
                        last.help = vec![
                            "assert that it did not fail with `.rejects` and `.not`, as in `expect(run()).not.rejects.toThrow()`"
                                .to_string(),
                        ];
                    }
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return ValueType::Error;
                }
                Some(_) => {}
                None if matches!(settled, Type::Unknown | Type::Never) => {}
                None => {
                    self.error_on_expr(
                        "E0277",
                        format!(
                            "`.{modifier}` expects a promise, but `expect` was given `{settled}`"
                        ),
                        actual,
                    );
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return ValueType::Error;
                }
            }
        }
        if chain.rejects {
            if method != "toThrow" {
                self.error_at_member(
                    "E0599",
                    format!("`.{method}()` does not follow `.rejects`; only `toThrow` does"),
                    method_span,
                    method,
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
            if args.len() > 1 {
                self.error_at_member(
                    "E0061",
                    format!(
                        "`.{method}()` expects 0 or 1 arguments, but {} were provided",
                        args.len()
                    ),
                    method_span,
                    method,
                );
            }
            for arg in args {
                let actual = self.check_expr(arg);
                self.require_assignable(&Type::Str, &actual, method, "E0308");
            }
            return ValueType::Known(Type::Void);
        }
        use crate::test::matcher_def;
        if method == "not" {
            if !args.is_empty() {
                self.error_at_member(
                    "E0061",
                    format!(
                        "`.not` expects 0 arguments, but {} were provided",
                        args.len()
                    ),
                    method_span,
                    method,
                );
                for arg in args {
                    self.check_expr(arg);
                }
            }
            return ValueType::Known(Type::Named(crate::test::ASSERTION_TYPE.to_string()));
        }
        let Some(def) = matcher_def(method) else {
            let inapplicable = crate::test::inapplicable_matcher(method);
            let message = match inapplicable {
                Some((reason, _)) => format!("`{method}` does not apply in tyrus: {reason}"),
                None => {
                    format!("no test matcher named `{method}` on the value returned by `expect`")
                }
            };
            self.error_at_member("E0599", message, method_span, method);
            if let Some((_, help)) = inapplicable
                && let Some(last) = self.diagnostics.last_mut()
            {
                last.help = vec![help.to_string()];
            }
            for arg in args {
                self.check_expr(arg);
            }
            return ValueType::Error;
        };
        match def.arg {
            crate::test::MatcherArg::None => {
                if !args.is_empty() {
                    self.error_at_member(
                        "E0061",
                        format!(
                            "`.{method}()` expects 0 arguments, but {} were provided",
                            args.len()
                        ),
                        method_span,
                        method,
                    );
                    for arg in args {
                        self.check_expr(arg);
                    }
                }
            }
            crate::test::MatcherArg::Any => {
                if args.len() != 1 {
                    self.error_at_member(
                        "E0061",
                        format!(
                            "`.{method}()` expects 1 argument, but {} were provided",
                            args.len()
                        ),
                        method_span,
                        method,
                    );
                }
                for arg in args {
                    self.check_expr(arg);
                }
            }
            crate::test::MatcherArg::Numeric => {
                if args.len() != 1 {
                    self.error_at_member(
                        "E0061",
                        format!(
                            "`.{method}()` expects 1 argument, but {} were provided",
                            args.len()
                        ),
                        method_span,
                        method,
                    );
                }
                for arg in args {
                    let actual = self.check_expr(arg);
                    if !is_numeric(&actual) {
                        self.error(
                            "E0308",
                            format!(
                                "`.{method}()` expects a numeric argument, found `{}`",
                                actual.display()
                            ),
                            expression_subject(arg),
                        );
                    }
                }
            }
            crate::test::MatcherArg::Length => {
                if args.len() != 1 {
                    self.error_at_member(
                        "E0061",
                        format!(
                            "`.{method}()` expects 1 argument, but {} were provided",
                            args.len()
                        ),
                        method_span,
                        method,
                    );
                }
                for arg in args {
                    let actual = self.check_expr(arg);
                    if !is_integer_value(&actual) {
                        self.error(
                            "E0308",
                            format!(
                                "`.{method}()` expects an integer length, found `{}`",
                                actual.display()
                            ),
                            expression_subject(arg),
                        );
                    }
                }
            }
            crate::test::MatcherArg::Throws => {
                if args.len() > 1 {
                    self.error_at_member(
                        "E0061",
                        format!(
                            "`.{method}()` expects 0 or 1 arguments, but {} were provided",
                            args.len()
                        ),
                        method_span,
                        method,
                    );
                }
                let thrown =
                    actual.and_then(|actual| self.assert_throws_thrown_type(actual, method));
                if let Some(expected) = args.first() {
                    let expected_ty = self.check_expr(expected);
                    match &thrown {
                        Some(Type::Str) => {
                            self.require_assignable(&Type::Str, &expected_ty, method, "E0308");
                        }
                        Some(other) => self.error_on_expr("E0308",
                            format!(
                                "`.{method}()` can only compare a message when the value thrown is a `string`, but `{other}` is thrown"
                            ), expected),
                        None => {}
                    }
                }
                for arg in args.iter().skip(1) {
                    self.check_expr(arg);
                }
            }
            crate::test::MatcherArg::Contained => {
                if args.len() != 1 {
                    self.error_at_member(
                        "E0061",
                        format!(
                            "`.{method}()` expects 1 argument, but {} were provided",
                            args.len()
                        ),
                        method_span,
                        method,
                    );
                }
                for arg in args {
                    self.check_expr(arg);
                }
            }
            crate::test::MatcherArg::Text => {
                if args.len() != 1 {
                    self.error_at_member(
                        "E0061",
                        format!(
                            "`.{method}()` expects 1 argument, but {} were provided",
                            args.len()
                        ),
                        method_span,
                        method,
                    );
                }
                for arg in args {
                    let actual = self.check_expr(arg);
                    let is_text = matches!(&actual, ValueType::Known(Type::Str))
                        || matches!(actual, ValueType::Unknown | ValueType::Error);
                    if !is_text {
                        self.error(
                            "E0308",
                            format!(
                                "`.{method}()` expects a string argument, found `{}`",
                                actual.display()
                            ),
                            expression_subject(arg),
                        );
                    }
                }
            }
        }
        ValueType::Known(Type::Void)
    }

    pub(super) fn check_array_static_method(&mut self, method: &str, args: &[Expr]) -> ValueType {
        if method == "of" {
            let actuals = args
                .iter()
                .map(|arg| self.check_expr(arg))
                .collect::<Vec<_>>();
            let Some(element) = infer_array_element_type(&actuals) else {
                self.error(
                    "E0282",
                    "cannot infer element type of `Array.of`; provide at least one argument"
                        .to_string(),
                    Some(method),
                );
                return ValueType::Error;
            };
            return ValueType::Known(Type::Array(Box::new(element)));
        }
        if method == "from" {
            if args.len() != 1 {
                self.error(
                    "E0061",
                    format!(
                        "`Array.from` expects 1 argument, but {} were provided",
                        args.len()
                    ),
                    Some(method),
                );
            }
            let source = args.first().map(|arg| self.check_expr(arg));
            return match source.as_ref().map(transparent_value_type) {
                Some(Some(Type::Array(element))) => {
                    ValueType::Known(Type::Array(Box::new(element.as_ref().clone())))
                }
                Some(Some(_)) | None => {
                    self.error(
                        "E0277",
                        "`Array.from` requires an array argument".to_string(),
                        args.first().and_then(expression_subject),
                    );
                    ValueType::Error
                }
                _ => ValueType::Error,
            };
        }
        if method == "withCapacity" {
            if args.len() != 1 {
                self.error(
                    "E0061",
                    format!(
                        "`Array.withCapacity` expects 1 argument, but {} were provided",
                        args.len()
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
            let actual = self.check_expr_as(args.first().unwrap(), &Type::Usize);
            self.require_assignable(&Type::Usize, &actual, "capacity", "E0308");
            return ValueType::Known(Type::Array(Box::new(Type::Unknown)));
        }
        if method != "isArray" {
            self.error(
                "E0599",
                format!("unknown Array method `{method}`"),
                Some(method),
            );
            for arg in args {
                self.check_expr(arg);
            }
            return ValueType::Error;
        }
        if args.len() != 1 {
            self.error(
                "E0061",
                format!(
                    "method `Array.isArray` expects 1 argument, but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        for arg in args {
            self.check_expr(arg);
        }
        ValueType::Known(Type::Bool)
    }

    pub(super) fn update_len_after_method(&mut self, object: &Expr, method: &str, args: &[Expr]) {
        let Expr {
            kind: ExprKind::Ident(name),
            ..
        } = object
        else {
            return;
        };
        if !ARRAY_MUTATING_METHODS.contains(&method) {
            return;
        }
        let new_len = if self.is_length_volatile() {
            None
        } else {
            match array_method_len_delta(method, args) {
                Some(delta) => self
                    .lookup(name)
                    .and_then(|symbol| symbol.array_len)
                    .map(|len| (len as i64 + delta).max(0) as usize),
                None => None,
            }
        };
        self.set_array_len(name, new_len);
    }

    pub(super) fn check_automatic_data_method(
        &mut self,
        receiver: &ValueType,
        method: &str,
        method_span: Span,
        args: &[Expr],
    ) -> Option<ValueType> {
        let automatic = crate::automatic_data_methods::AutomaticDataMethod::instance(method)?;
        let ValueType::Known(Type::Named(name)) = receiver else {
            return None;
        };
        if !self.data_interfaces.contains(name) {
            return None;
        }
        let supported = match automatic {
            crate::automatic_data_methods::AutomaticDataMethod::Copy => {
                crate::automatic_data_methods::supports_copy(
                    &Type::Named(name.clone()),
                    &self.interfaces,
                    &self.data_interfaces,
                )
            }
            crate::automatic_data_methods::AutomaticDataMethod::Eq => {
                crate::automatic_data_methods::supports_eq(
                    &Type::Named(name.clone()),
                    &self.interfaces,
                    &self.data_interfaces,
                )
            }
            crate::automatic_data_methods::AutomaticDataMethod::Debug => {
                crate::automatic_data_methods::supports_debug(
                    &Type::Named(name.clone()),
                    &self.interfaces,
                    &self.data_interfaces,
                )
            }
            crate::automatic_data_methods::AutomaticDataMethod::JsonEncode => {
                crate::automatic_data_methods::supports_json_encode(
                    &Type::Named(name.clone()),
                    &self.interfaces,
                    &self.data_interfaces,
                )
            }
            _ => return None,
        };
        if !supported {
            self.error_at_with_help(
                "E0277",
                format!("automatic data method `{method}` is not supported by interface `{name}`"),
                method_span,
                vec![format!(
                    "use a behavior interface to provide `{method}` for `{name}` explicitly"
                )],
            );
            for arg in args {
                self.check_expr(arg);
            }
            return Some(ValueType::Error);
        }
        let valid_arity = match automatic {
            crate::automatic_data_methods::AutomaticDataMethod::Eq => args.len() == 1,
            crate::automatic_data_methods::AutomaticDataMethod::JsonEncode => args.len() <= 1,
            _ => args.is_empty(),
        };
        if !valid_arity {
            let expected_args = match automatic {
                crate::automatic_data_methods::AutomaticDataMethod::JsonEncode => "0 or 1",
                crate::automatic_data_methods::AutomaticDataMethod::Eq => "1",
                _ => "0",
            };
            self.error_at(
                "E0061",
                format!(
                    "automatic data method `{method}` expects {expected_args} argument(s), but {} were provided",
                    args.len()
                ),
                method_span,
            );
        }
        for arg in args {
            let actual = self.check_expr(arg);
            if matches!(
                automatic,
                crate::automatic_data_methods::AutomaticDataMethod::Eq
            ) {
                self.require_assignable(&Type::Named(name.clone()), &actual, method, "E0308");
            } else if matches!(
                automatic,
                crate::automatic_data_methods::AutomaticDataMethod::JsonEncode
            ) {
                self.require_assignable(
                    &Type::Object(vec![Field {
                        name: "pretty".to_string(),
                        ty: Type::Bool,
                        readonly: false,
                        optional: true,
                        decorators: Vec::new(),
                        span: Span::DUMMY,
                    }]),
                    &actual,
                    method,
                    "E0308",
                );
            }
        }
        Some(ValueType::Known(match automatic {
            crate::automatic_data_methods::AutomaticDataMethod::Copy => Type::Named(name.clone()),
            crate::automatic_data_methods::AutomaticDataMethod::Eq => Type::Bool,
            crate::automatic_data_methods::AutomaticDataMethod::Debug => Type::Str,
            crate::automatic_data_methods::AutomaticDataMethod::JsonEncode => Type::Str,
            _ => unreachable!("only Phase 2 automatic methods reach this branch"),
        }))
    }

    pub(super) fn check_automatic_data_static_method(
        &mut self,
        type_name: &str,
        method: &str,
        method_span: Span,
        args: &[Expr],
    ) -> Option<ValueType> {
        let automatic = crate::automatic_data_methods::AutomaticDataMethod::static_method(method)?;
        if !matches!(
            automatic,
            crate::automatic_data_methods::AutomaticDataMethod::JsonDecode
        ) {
            return None;
        }
        let supported = crate::automatic_data_methods::supports_json(
            &Type::Named(type_name.to_string()),
            &self.interfaces,
            &self.data_interfaces,
        );
        if !supported {
            let help = format!(
                "use a behavior interface or a supported JSON field type for `{type_name}`"
            );
            self.error_at_with_help(
                "E0277",
                format!(
                    "automatic data method `{method}` is not supported by interface `{type_name}`"
                ),
                method_span,
                vec![help],
            );
        }
        if args.len() != 1 {
            self.error_at(
                "E0061",
                format!(
                    "automatic data method `{type_name}.{method}` expects 1 argument, but {} were provided",
                    args.len()
                ),
                method_span,
            );
        }
        for argument in args {
            let actual = self.check_expr(argument);
            self.require_assignable(&Type::Str, &actual, method, "E0308");
        }
        Some(ValueType::Known(Type::Result {
            success: Box::new(Type::Named(type_name.to_string())),
            error: Box::new(Type::Named("JsonError".to_string())),
        }))
    }
}
