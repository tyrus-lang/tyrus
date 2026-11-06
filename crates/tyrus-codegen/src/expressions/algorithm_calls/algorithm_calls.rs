use super::*;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_algorithm_constructor_call(
        &mut self,
        callee: &str,
        type_args: &[Type],
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        match callee {
            tyrus_core::crypto::TIMING_SAFE_EQUAL_TARGET => {
                return (self.emit_crypto_runtime_call(callee, args)).map(Some);
            }
            tyrus_core::algorithm::UNION_FIND_NEW_TARGET => {
                let elem_ty = type_args.first().cloned();
                let result =
                    self.emit_algorithm_runtime_call(callee, args, elem_ty.as_ref(), None)?;
                let uf_ty = Type::UnionFind(Box::new(elem_ty.unwrap_or(Type::Usize)));
                self.record_owned_native_result(result, uf_ty);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::BINARY_HEAP_NEW_TARGET => {
                let elem_ty = type_args.first().cloned();
                let result =
                    self.emit_algorithm_runtime_call(callee, args, elem_ty.as_ref(), None)?;
                let heap_ty = Type::BinaryHeap(Box::new(elem_ty.unwrap_or(Type::Unknown)));
                self.record_owned_native_result(result, heap_ty);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::BINOMIAL_HEAP_NEW_TARGET => {
                let elem_ty = type_args.first().cloned();
                let result =
                    self.emit_algorithm_runtime_call(callee, args, elem_ty.as_ref(), None)?;
                let heap_ty = Type::BinomialHeap(Box::new(elem_ty.unwrap_or(Type::Unknown)));
                self.record_owned_native_result(result, heap_ty);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::COUNTER_NEW_TARGET => {
                let elem_ty = type_args.first().cloned();
                let result =
                    self.emit_algorithm_runtime_call(callee, args, elem_ty.as_ref(), None)?;
                let counter_ty = Type::Counter(Box::new(elem_ty.unwrap_or(Type::Unknown)));
                self.record_owned_native_result(result, counter_ty);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::DEQUE_NEW_TARGET => {
                let elem_ty = type_args.first().cloned();
                let result =
                    self.emit_algorithm_runtime_call(callee, args, elem_ty.as_ref(), None)?;
                let deque_ty = Type::Deque(Box::new(elem_ty.unwrap_or(Type::Unknown)));
                self.record_owned_native_result(result, deque_ty);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::FENWICK_TREE_NEW_TARGET => {
                let result = self.emit_algorithm_runtime_call(callee, args, None, None)?;
                self.record_owned_native_result(result, Type::FenwickTree);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::BUBBLE_SORT_NEW_TARGET => {
                let result = self.emit_algorithm_runtime_call(callee, args, None, None)?;
                self.record_owned_native_result(result, Type::BubbleSort);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::SEGMENT_TREE_NEW_TARGET => {
                if args.len() != 1 {
                    return (Err("error: `SegmentTree` expects 1 argument".into())).map(Some);
                }
                let source = &args[0];
                let array_ty = self.expr_type_hint(source).ok_or_else(|| {
                    "error: cannot infer source type of `SegmentTree`".to_string()
                })?;
                let Type::Array(element) = transparent_codegen_type(&array_ty) else {
                    return (Err("error: `SegmentTree` requires an array argument".into()))
                        .map(Some);
                };
                let element = element.as_ref().clone();
                let array = self
                    .emit_expr_as(source, &Type::Array(Box::new(element.clone())))?
                    .into_struct_value();
                let data = self
                    .builder
                    .build_extract_value(array, 0, "segment_tree.data")
                    .unwrap()
                    .into_pointer_value();
                let length = self
                    .builder
                    .build_extract_value(array, 1, "segment_tree.length")
                    .unwrap()
                    .into_int_value();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val = self.declare_libc_fn(
                    "tyrus_segment_tree_new",
                    &[pointer.into(), self.usize_type().into()],
                    Some(pointer.into()),
                );
                let result = self
                    .builder
                    .build_call(fn_val, &[data.into(), length.into()], "segment_tree.new")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: segment_tree_new did not return a value")?;
                self.record_owned_native_result(result, Type::SegmentTree);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::SPARSE_TABLE_NEW_TARGET => {
                if args.len() != 1 {
                    return (Err("error: `SparseTable` expects 1 argument".into())).map(Some);
                }
                let source = &args[0];
                let array_ty = self.expr_type_hint(source).ok_or_else(|| {
                    "error: cannot infer source type of `SparseTable`".to_string()
                })?;
                let Type::Array(element) = transparent_codegen_type(&array_ty) else {
                    return (Err("error: `SparseTable` requires an array argument".into()))
                        .map(Some);
                };
                let element = element.as_ref().clone();
                let array = self
                    .emit_expr_as(source, &Type::Array(Box::new(element.clone())))?
                    .into_struct_value();
                let data = self
                    .builder
                    .build_extract_value(array, 0, "sparse_table.data")
                    .unwrap()
                    .into_pointer_value();
                let length = self
                    .builder
                    .build_extract_value(array, 1, "sparse_table.length")
                    .unwrap()
                    .into_int_value();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val = self.declare_libc_fn(
                    "tyrus_sparse_table_new",
                    &[pointer.into(), self.usize_type().into()],
                    Some(pointer.into()),
                );
                let result = self
                    .builder
                    .build_call(fn_val, &[data.into(), length.into()], "sparse_table.new")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: sparse_table_new did not return a value")?;
                self.record_owned_native_result(result, Type::SparseTable);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::SEGMENT_TREE_LAZY_NEW_TARGET => {
                if args.len() != 1 {
                    return (Err("error: `SegmentTreeLazy` expects 1 argument".into())).map(Some);
                }
                let source = &args[0];
                let array_ty = self.expr_type_hint(source).ok_or_else(|| {
                    "error: cannot infer source type of `SegmentTreeLazy`".to_string()
                })?;
                let Type::Array(element) = transparent_codegen_type(&array_ty) else {
                    return (Err("error: `SegmentTreeLazy` requires an array argument".into()))
                        .map(Some);
                };
                let element = element.as_ref().clone();
                let array = self
                    .emit_expr_as(source, &Type::Array(Box::new(element.clone())))?
                    .into_struct_value();
                let data = self
                    .builder
                    .build_extract_value(array, 0, "segment_tree_lazy.data")
                    .unwrap()
                    .into_pointer_value();
                let length = self
                    .builder
                    .build_extract_value(array, 1, "segment_tree_lazy.length")
                    .unwrap()
                    .into_int_value();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val = self.declare_libc_fn(
                    "tyrus_segment_tree_lazy_new",
                    &[pointer.into(), self.usize_type().into()],
                    Some(pointer.into()),
                );
                let result = self
                    .builder
                    .build_call(
                        fn_val,
                        &[data.into(), length.into()],
                        "segment_tree_lazy.new",
                    )
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: segment_tree_lazy_new did not return a value")?;
                self.record_owned_native_result(result, Type::SegmentTreeLazy);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::BIT_SET_NEW_TARGET => {
                let result = self.emit_algorithm_runtime_call(callee, args, None, None)?;
                self.record_owned_native_result(result, Type::BitSet);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::TRIE_NEW_TARGET => {
                let result = self.emit_algorithm_runtime_call(callee, args, None, None)?;
                self.record_owned_native_result(result, Type::Trie);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::SUFFIX_TREE_NEW_TARGET => {
                let result = self.emit_algorithm_runtime_call(callee, args, None, None)?;
                self.record_owned_native_result(result, Type::SuffixTree);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::LCA_NEW_TARGET => {
                let result = self.emit_algorithm_runtime_call(callee, args, None, None)?;
                self.record_owned_native_result(result, Type::Lca);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::HLD_NEW_TARGET => {
                let result = self.emit_algorithm_runtime_call(callee, args, None, None)?;
                self.record_owned_native_result(result, Type::Hld);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::CENTROID_NEW_TARGET => {
                let result = self.emit_algorithm_runtime_call(callee, args, None, None)?;
                self.record_owned_native_result(result, Type::Centroid);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::BLOOM_FILTER_NEW_TARGET => {
                let result = self.emit_algorithm_runtime_call(callee, args, None, None)?;
                self.record_owned_native_result(result, Type::BloomFilter);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::SUFFIX_ARRAY_INFO_TARGET => {
                if args.len() != 1 {
                    return (Err("error: `suffixArrayInfo` expects 1 argument".into())).map(Some);
                }
                let text = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let data = self.string_data_ptr(text)?;
                let length = self.string_byte_len(text)?;
                let result_ty = Type::Named("SuffixArrayInfo".to_string());
                let output = self
                    .builder
                    .build_alloca(self.llvm_type(&result_ty), "suffix_array.info.output")
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_store(output, self.llvm_type(&result_ty).const_zero())
                    .map_err(|e| e.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_suffix_array_info",
                    &[pointer.into(), self.usize_type().into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        function,
                        &[data.into(), length.into(), output.into()],
                        "suffix_array.info.call",
                    )
                    .map_err(|e| e.to_string())?;
                let result = self
                    .builder
                    .build_load(
                        self.llvm_type(&result_ty),
                        output,
                        "suffix_array.info.result",
                    )
                    .map_err(|e| e.to_string())?;
                self.record_owned_native_result(result, result_ty);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::SUFFIX_ARRAY_TARGET => {
                if args.len() != 1 {
                    return (Err("error: `suffixArray` expects 1 argument".into())).map(Some);
                }
                let s = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let data = self.string_data_ptr(s)?;
                let length = self.string_byte_len(s)?;
                let array_ty = Type::Array(Box::new(Type::Usize));
                let output = self
                    .builder
                    .build_alloca(self.llvm_type(&array_ty), "suffix_array.output")
                    .unwrap();
                self.builder
                    .build_store(output, self.llvm_type(&array_ty).const_zero())
                    .unwrap();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val = self.declare_libc_fn(
                    "tyrus_suffix_array",
                    &[pointer.into(), self.usize_type().into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[data.into(), length.into(), output.into()],
                        "suffix_array",
                    )
                    .map_err(|e| e.to_string())?;
                let result = self
                    .builder
                    .build_load(self.llvm_type(&array_ty), output, "suffix_array.result")
                    .map_err(|e| e.to_string())?;
                self.record_owned_native_result(result, array_ty);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::LINKED_LIST_NEW_TARGET => {
                let elem_ty = type_args.first().cloned();
                let result =
                    self.emit_algorithm_runtime_call(callee, args, elem_ty.as_ref(), None)?;
                let ll_ty = Type::LinkedList(Box::new(elem_ty.unwrap_or(Type::Unknown)));
                self.record_owned_native_result(result, ll_ty);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::DOUBLE_LINKED_LIST_NEW_TARGET => {
                let elem_ty = type_args.first().cloned();
                let result =
                    self.emit_algorithm_runtime_call(callee, args, elem_ty.as_ref(), None)?;
                let dll_ty = Type::DoubleLinkedList(Box::new(elem_ty.unwrap_or(Type::Unknown)));
                self.record_owned_native_result(result, dll_ty);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::AVL_TREE_NEW_TARGET
            | tyrus_core::algorithm::RED_BLACK_TREE_NEW_TARGET
            | tyrus_core::algorithm::SPLAY_TREE_NEW_TARGET
            | tyrus_core::algorithm::TREAP_NEW_TARGET => {
                let elem_ty = type_args.first().cloned();
                let result =
                    self.emit_algorithm_runtime_call(callee, args, elem_ty.as_ref(), None)?;
                let tree_ty = match callee {
                    tyrus_core::algorithm::AVL_TREE_NEW_TARGET => {
                        Type::AvlTree(Box::new(elem_ty.unwrap_or(Type::Unknown)))
                    }
                    tyrus_core::algorithm::RED_BLACK_TREE_NEW_TARGET => {
                        Type::RedBlackTree(Box::new(elem_ty.unwrap_or(Type::Unknown)))
                    }
                    tyrus_core::algorithm::SPLAY_TREE_NEW_TARGET => {
                        Type::SplayTree(Box::new(elem_ty.unwrap_or(Type::Unknown)))
                    }
                    _ => Type::Treap(Box::new(elem_ty.unwrap_or(Type::Unknown))),
                };
                self.record_owned_native_result(result, tree_ty);
                return Ok(Some(result));
            }
            tyrus_core::algorithm::SORTED_SET_NEW_TARGET => {
                let elem_ty = type_args.first().cloned();
                let result =
                    self.emit_algorithm_runtime_call(callee, args, elem_ty.as_ref(), None)?;
                self.record_owned_native_result(
                    result,
                    Type::SortedSet(Box::new(elem_ty.unwrap_or(Type::Unknown))),
                );
                return Ok(Some(result));
            }
            tyrus_core::algorithm::SKIP_LIST_NEW_TARGET => {
                let elem_ty = type_args.first().cloned();
                let result =
                    self.emit_algorithm_runtime_call(callee, args, elem_ty.as_ref(), None)?;
                self.record_owned_native_result(
                    result,
                    Type::SkipList(Box::new(elem_ty.unwrap_or(Type::Unknown))),
                );
                return Ok(Some(result));
            }
            tyrus_core::algorithm::SORTED_MAP_NEW_TARGET => {
                let key_ty = type_args.first().cloned();
                let value_ty = type_args.get(1).cloned();
                let result = self.emit_algorithm_runtime_call(
                    callee,
                    args,
                    key_ty.as_ref(),
                    value_ty.as_ref(),
                )?;
                self.record_owned_native_result(
                    result,
                    Type::SortedMap(
                        Box::new(key_ty.unwrap_or(Type::Unknown)),
                        Box::new(value_ty.unwrap_or(Type::Unknown)),
                    ),
                );
                return Ok(Some(result));
            }
            tyrus_core::algorithm::GRAPH_NEW_TARGET => {
                let node_ty = type_args.first().cloned();
                let weight_ty = type_args.get(1).cloned();
                let result = self.emit_algorithm_runtime_call(
                    callee,
                    args,
                    node_ty.as_ref(),
                    weight_ty.as_ref(),
                )?;
                self.record_owned_native_result(
                    result,
                    Type::Graph(
                        Box::new(node_ty.unwrap_or(Type::Unknown)),
                        Box::new(weight_ty.unwrap_or(Type::Unknown)),
                    ),
                );
                return Ok(Some(result));
            }
            tyrus_core::algorithm::BFS_TARGET
            | tyrus_core::algorithm::DFS_TARGET
            | tyrus_core::algorithm::BFS_INFO_TARGET
            | tyrus_core::algorithm::DFS_INFO_TARGET
            | tyrus_core::algorithm::IDDFS_INFO_TARGET
            | tyrus_core::algorithm::IDDFS_TARGET
            | tyrus_core::algorithm::DIJKSTRA_TARGET
            | tyrus_core::algorithm::DIJKSTRA_INFO_TARGET
            | tyrus_core::algorithm::A_STAR_TARGET
            | tyrus_core::algorithm::A_STAR_INFO_TARGET
            | tyrus_core::algorithm::BELLMAN_FORD_TARGET
            | tyrus_core::algorithm::BELLMAN_FORD_INFO_TARGET
            | tyrus_core::algorithm::FLOYD_WARSHALL_TARGET
            | tyrus_core::algorithm::FLOYD_WARSHALL_INFO_TARGET
            | tyrus_core::algorithm::SPFA_TARGET
            | tyrus_core::algorithm::SPFA_INFO_TARGET
            | tyrus_core::algorithm::JOHNSON_TARGET
            | tyrus_core::algorithm::JOHNSON_INFO_TARGET
            | tyrus_core::algorithm::KRUSKAL_TARGET
            | tyrus_core::algorithm::KRUSKAL_INFO_TARGET
            | tyrus_core::algorithm::PRIM_TARGET
            | tyrus_core::algorithm::PRIM_INFO_TARGET
            | tyrus_core::algorithm::BORUVKA_TARGET
            | tyrus_core::algorithm::BORUVKA_INFO_TARGET
            | tyrus_core::algorithm::STEINER_TREE_TARGET
            | tyrus_core::algorithm::STEINER_TREE_INFO_TARGET
            | tyrus_core::algorithm::EDMONDS_KARP_TARGET
            | tyrus_core::algorithm::DINIC_TARGET
            | tyrus_core::algorithm::PUSH_RELABEL_TARGET
            | tyrus_core::algorithm::MIN_CUT_TARGET
            | tyrus_core::algorithm::MIN_COST_MAX_FLOW_TARGET
            | tyrus_core::algorithm::EDMONDS_KARP_INFO_TARGET
            | tyrus_core::algorithm::DINIC_INFO_TARGET
            | tyrus_core::algorithm::PUSH_RELABEL_INFO_TARGET
            | tyrus_core::algorithm::MIN_CUT_INFO_TARGET
            | tyrus_core::algorithm::MIN_COST_MAX_FLOW_INFO_TARGET
            | tyrus_core::algorithm::TOPOLOGICAL_KAHN_TARGET
            | tyrus_core::algorithm::TOPOLOGICAL_DFS_TARGET
            | tyrus_core::algorithm::TOPOLOGICAL_KAHN_INFO_TARGET
            | tyrus_core::algorithm::TOPOLOGICAL_DFS_INFO_TARGET
            | tyrus_core::algorithm::TARJAN_SCC_INFO_TARGET
            | tyrus_core::algorithm::KOSARAJU_SCC_INFO_TARGET
            | tyrus_core::algorithm::FLEURY_EULERIAN_INFO_TARGET
            | tyrus_core::algorithm::HIERHOLZER_EULERIAN_INFO_TARGET
            | tyrus_core::algorithm::FIND_BRIDGES_INFO_TARGET
            | tyrus_core::algorithm::FIND_CUT_VERTICES_INFO_TARGET
            | tyrus_core::algorithm::BICONNECTED_COMPONENTS_INFO_TARGET
            | tyrus_core::algorithm::EDMONDS_BLOSSOM_INFO_TARGET
            | tyrus_core::algorithm::HUNGARIAN_INFO_TARGET
            | tyrus_core::algorithm::TARJAN_SCC_TARGET
            | tyrus_core::algorithm::KOSARAJU_SCC_TARGET
            | tyrus_core::algorithm::FLEURY_EULERIAN_TARGET
            | tyrus_core::algorithm::HIERHOLZER_EULERIAN_TARGET
            | tyrus_core::algorithm::FIND_BRIDGES_TARGET
            | tyrus_core::algorithm::FIND_CUT_VERTICES_TARGET
            | tyrus_core::algorithm::BICONNECTED_COMPONENTS_TARGET
            | tyrus_core::algorithm::EDMONDS_BLOSSOM_TARGET
            | tyrus_core::algorithm::HUNGARIAN_TARGET => {
                let result = self.emit_algorithm_runtime_call(callee, args, None, None)?;
                return Ok(Some(result));
            }
            tyrus_core::algorithm::MERGE_SORT_TARGET
            | tyrus_core::algorithm::QUICK_SORT_TARGET
            | tyrus_core::algorithm::HEAP_SORT_TARGET
            | tyrus_core::algorithm::COUNTING_SORT_TARGET
            | tyrus_core::algorithm::RADIX_SORT_TARGET
            | tyrus_core::algorithm::BUCKET_SORT_TARGET
            | tyrus_core::algorithm::TIM_SORT_TARGET
            | tyrus_core::algorithm::INSERTION_SORT_TARGET
            | tyrus_core::algorithm::SHELL_SORT_TARGET
            | tyrus_core::algorithm::SELECTION_SORT_TARGET
            | tyrus_core::algorithm::BUBBLE_SORT_TARGET
            | tyrus_core::algorithm::BUBBLE_SORT_INFO_TARGET
            | tyrus_core::algorithm::QUICKSELECT_INFO_TARGET
            | tyrus_core::algorithm::BINARY_SEARCH_INFO_TARGET
            | tyrus_core::algorithm::INTERPOLATION_SEARCH_INFO_TARGET
            | tyrus_core::algorithm::EXPONENTIAL_SEARCH_INFO_TARGET
            | tyrus_core::algorithm::SHAKER_SORT_TARGET
            | tyrus_core::algorithm::MERGE_SORT_INFO_TARGET
            | tyrus_core::algorithm::QUICK_SORT_INFO_TARGET
            | tyrus_core::algorithm::HEAP_SORT_INFO_TARGET
            | tyrus_core::algorithm::COUNTING_SORT_INFO_TARGET
            | tyrus_core::algorithm::RADIX_SORT_INFO_TARGET
            | tyrus_core::algorithm::BUCKET_SORT_INFO_TARGET
            | tyrus_core::algorithm::TIM_SORT_INFO_TARGET
            | tyrus_core::algorithm::INSERTION_SORT_INFO_TARGET
            | tyrus_core::algorithm::SHELL_SORT_INFO_TARGET
            | tyrus_core::algorithm::SELECTION_SORT_INFO_TARGET
            | tyrus_core::algorithm::SHAKER_SORT_INFO_TARGET => {
                let result = self.emit_algorithm_runtime_call(callee, args, None, None)?;
                return Ok(Some(result));
            }
            tyrus_core::algorithm::QUICKSELECT_TARGET
            | tyrus_core::algorithm::BINARY_SEARCH_TARGET
            | tyrus_core::algorithm::INTERPOLATION_SEARCH_TARGET
            | tyrus_core::algorithm::EXPONENTIAL_SEARCH_TARGET => {
                let result = self.emit_algorithm_runtime_call(callee, args, None, None)?;
                return Ok(Some(result));
            }
            tyrus_core::algorithm::KMP_SEARCH_TARGET
            | tyrus_core::algorithm::Z_ALGORITHM_TARGET
            | tyrus_core::algorithm::RABIN_KARP_TARGET
            | tyrus_core::algorithm::BOYER_MOORE_TARGET
            | tyrus_core::algorithm::AHO_CORASICK_TARGET
            | tyrus_core::algorithm::LEVENSHTEIN_TARGET => {
                return (self.emit_string_search_call(callee, args)).map(Some);
            }
            tyrus_core::algorithm::KMP_SEARCH_INFO_TARGET => {
                return (self.emit_kmp_search_info_call(args)).map(Some);
            }
            tyrus_core::algorithm::Z_ALGORITHM_INFO_TARGET => {
                return (self.emit_z_algorithm_info_call(args)).map(Some);
            }
            tyrus_core::algorithm::RABIN_KARP_INFO_TARGET => {
                return (self.emit_rabin_karp_info_call(args)).map(Some);
            }
            tyrus_core::algorithm::BOYER_MOORE_INFO_TARGET => {
                return (self.emit_boyer_moore_info_call(args)).map(Some);
            }
            tyrus_core::algorithm::AHO_CORASICK_INFO_TARGET => {
                return (self.emit_aho_corasick_info_call(args)).map(Some);
            }
            tyrus_core::algorithm::LEVENSHTEIN_INFO_TARGET => {
                return (self.emit_levenshtein_info_call(args)).map(Some);
            }
            _ => {}
        }
        Ok(None)
    }
}
