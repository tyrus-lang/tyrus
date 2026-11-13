use super::*;

#[path = "compact_structures/compact_structures.rs"]
mod compact_structures;
#[path = "deques_and_trees/deques_and_trees.rs"]
mod deques_and_trees;
#[path = "graph_algorithms/graph_algorithms.rs"]
mod graph_algorithms;
#[path = "graph_containers/graph_containers.rs"]
mod graph_containers;
#[path = "linked_lists/linked_lists.rs"]
mod linked_lists;
#[path = "sorted_containers/sorted_containers.rs"]
mod sorted_containers;
#[path = "text_algorithms/text_algorithms.rs"]
mod text_algorithms;
#[path = "union_find_heaps/union_find_heaps.rs"]
mod union_find_heaps;

impl<'ctx> Codegen<'ctx> {
    fn emit_optional_array_result(
        &mut self,
        result: PointerValue<'ctx>,
        element: Type,
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let array_ty = Type::Array(Box::new(element));
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("error: optional graph result is not inside a function")?;
        let present = self
            .context
            .append_basic_block(function, &format!("{name}.present"));
        let absent = self
            .context
            .append_basic_block(function, &format!("{name}.absent"));
        let merge = self
            .context
            .append_basic_block(function, &format!("{name}.merge"));
        let is_present = self
            .builder
            .build_is_not_null(result, &format!("{name}.present"))
            .map_err(|e| e.to_string())?;
        self.builder
            .build_conditional_branch(is_present, present, absent)
            .map_err(|e| e.to_string())?;
        self.builder.position_at_end(absent);
        let null = pointer.const_null();
        self.builder
            .build_unconditional_branch(merge)
            .map_err(|e| e.to_string())?;
        let absent_end = self.builder.get_insert_block().unwrap();
        self.builder.position_at_end(present);
        let loaded = self
            .builder
            .build_load(self.llvm_type(&array_ty), result, &format!("{name}.value"))
            .map_err(|e| e.to_string())?;
        let boxed = self.emit_optional_box(loaded, &array_ty)?;
        let free = self.declare_libc_fn("free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[result.into()], &format!("{name}.free"))
            .map_err(|e| e.to_string())?;
        self.builder
            .build_unconditional_branch(merge)
            .map_err(|e| e.to_string())?;
        let present_end = self.builder.get_insert_block().unwrap();
        self.builder.position_at_end(merge);
        let phi = self
            .builder
            .build_phi(pointer, &format!("{name}.optional"))
            .map_err(|e| e.to_string())?;
        phi.add_incoming(&[(&null, absent_end), (&boxed, present_end)]);
        let value = phi.as_basic_value();
        self.record_owned_native_result_with(
            value,
            Type::Optional(Box::new(array_ty)),
            crate::ReleasePlan::Box(Box::new(crate::ReleasePlan::Deep)),
        );
        Ok(value)
    }

    fn emit_euler_info_call(
        &mut self,
        target: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let (name, result_ty) = match target {
            tyrus_core::algorithm::FLEURY_EULERIAN_INFO_TARGET => (
                "tyrus_algorithm_fleury_eulerian_info",
                Type::Named("FleuryEulerianInfo".to_string()),
            ),
            tyrus_core::algorithm::HIERHOLZER_EULERIAN_INFO_TARGET => (
                "tyrus_algorithm_hierholzer_eulerian_info",
                Type::Named("HierholzerEulerianInfo".to_string()),
            ),
            _ => return Err("error: unknown Eulerian info algorithm".into()),
        };
        let output = self
            .builder
            .build_alloca(self.llvm_type(&result_ty), "euler.info.output")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(output, self.llvm_type(&result_ty).const_zero())
            .map_err(|e| e.to_string())?;
        let input_ty = self
            .expr_type_hint(&args[0])
            .ok_or("error: cannot infer Eulerian input type")?;
        let graph_input = matches!(transparent_codegen_type(&input_ty), Type::Graph(_, _));
        let (name, params, call_args) = if graph_input {
            (
                format!("{name}_graph"),
                vec![pointer.into(), pointer.into()],
                vec![self.emit_expr(&args[0])?, output.into()],
            )
        } else {
            let (data, length) =
                self.emit_path_array_argument(&args[0], &input_ty, "euler.info.input")?;
            (
                name.to_string(),
                vec![pointer.into(), self.usize_type().into(), pointer.into()],
                vec![data, length, output.into()],
            )
        };
        let function = self.declare_libc_fn(&name, &params, None);
        let metadata: Vec<BasicMetadataValueEnum<'ctx>> =
            call_args.into_iter().map(Into::into).collect();
        self.builder
            .build_call(function, &metadata, "euler.info.call")
            .map_err(|e| e.to_string())?;
        let result = self
            .builder
            .build_load(self.llvm_type(&result_ty), output, "euler.info.result")
            .map_err(|e| e.to_string())?;
        self.record_owned_native_result(result, result_ty);
        Ok(result)
    }

    fn emit_structure_algorithm_call(
        &mut self,
        target: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let input_ty = self
            .expr_type_hint(&args[0])
            .ok_or("error: cannot infer graph structure input type")?;
        let graph_input = matches!(transparent_codegen_type(&input_ty), Type::Graph(_, _));
        let usize_array = Type::Array(Box::new(Type::Usize));
        let pair_array = Type::Array(Box::new(Type::Tuple(vec![Type::Usize, Type::Usize])));
        let nested_usize = Type::Array(Box::new(usize_array.clone()));
        let nested_pairs = Type::Array(Box::new(pair_array.clone()));
        let optional = matches!(
            target,
            tyrus_core::algorithm::TOPOLOGICAL_KAHN_TARGET
                | tyrus_core::algorithm::TOPOLOGICAL_DFS_TARGET
                | tyrus_core::algorithm::FLEURY_EULERIAN_TARGET
                | tyrus_core::algorithm::HIERHOLZER_EULERIAN_TARGET
        );
        let (element, result_ty) = match target {
            tyrus_core::algorithm::TARJAN_SCC_TARGET
            | tyrus_core::algorithm::KOSARAJU_SCC_TARGET => (usize_array.clone(), nested_usize),
            tyrus_core::algorithm::BICONNECTED_COMPONENTS_TARGET => {
                (pair_array.clone(), nested_pairs)
            }
            tyrus_core::algorithm::FIND_BRIDGES_TARGET
            | tyrus_core::algorithm::EDMONDS_BLOSSOM_TARGET => {
                (Type::Tuple(vec![Type::Usize, Type::Usize]), pair_array)
            }
            tyrus_core::algorithm::FIND_CUT_VERTICES_TARGET => (Type::Usize, usize_array),
            _ => (Type::Usize, usize_array),
        };
        let graph = if graph_input {
            Some(self.emit_expr(&args[0])?)
        } else {
            None
        };
        let mut output = None;
        let (name, params, call_args) = if optional {
            let name = match target {
                tyrus_core::algorithm::TOPOLOGICAL_KAHN_TARGET => {
                    "tyrus_algorithm_topological_sort_kahn"
                }
                tyrus_core::algorithm::TOPOLOGICAL_DFS_TARGET => {
                    "tyrus_algorithm_topological_sort_dfs"
                }
                tyrus_core::algorithm::FLEURY_EULERIAN_TARGET => "tyrus_algorithm_fleury_eulerian",
                _ => "tyrus_algorithm_hierholzer_eulerian",
            };
            if graph_input {
                (
                    format!("{name}_graph"),
                    vec![pointer.into()],
                    vec![graph.unwrap()],
                )
            } else {
                let (data, length) =
                    self.emit_path_array_argument(&args[0], &input_ty, "structure.input")?;
                (
                    name.to_string(),
                    vec![pointer.into(), self.usize_type().into()],
                    vec![data, length],
                )
            }
        } else {
            let output_ptr = self
                .builder
                .build_alloca(self.llvm_type(&result_ty), "structure.output")
                .map_err(|e| e.to_string())?;
            output = Some(output_ptr);
            let name = match target {
                tyrus_core::algorithm::TARJAN_SCC_TARGET => "tyrus_algorithm_tarjan_scc",
                tyrus_core::algorithm::KOSARAJU_SCC_TARGET => "tyrus_algorithm_kosaraju_scc",
                tyrus_core::algorithm::FIND_BRIDGES_TARGET => "tyrus_algorithm_find_bridges",
                tyrus_core::algorithm::FIND_CUT_VERTICES_TARGET => {
                    "tyrus_algorithm_find_cut_vertices"
                }
                tyrus_core::algorithm::BICONNECTED_COMPONENTS_TARGET => {
                    "tyrus_algorithm_biconnected_components"
                }
                _ => "tyrus_algorithm_edmonds_blossom",
            };
            if graph_input {
                (
                    format!("{name}_graph"),
                    vec![pointer.into(), pointer.into()],
                    vec![graph.unwrap(), output_ptr.into()],
                )
            } else {
                let (data, length) =
                    self.emit_path_array_argument(&args[0], &input_ty, "structure.input")?;
                (
                    name.to_string(),
                    vec![pointer.into(), self.usize_type().into(), pointer.into()],
                    vec![data, length, output_ptr.into()],
                )
            }
        };
        let function = self.declare_libc_fn(
            &name,
            &params,
            if optional { Some(pointer.into()) } else { None },
        );
        let metadata: Vec<BasicMetadataValueEnum<'ctx>> =
            call_args.into_iter().map(Into::into).collect();
        let call = self
            .builder
            .build_call(function, &metadata, "structure.call")
            .map_err(|e| e.to_string())?;
        if optional {
            return self.emit_optional_array_result(
                call.try_as_basic_value()
                    .basic()
                    .ok_or("error: optional structure algorithm returned no value")?
                    .into_pointer_value(),
                element,
                "structure.optional",
            );
        }
        let value = self
            .builder
            .build_load(
                self.llvm_type(&result_ty),
                output.unwrap(),
                "structure.result",
            )
            .map_err(|e| e.to_string())?;
        self.record_owned_native_result(value, result_ty);
        Ok(value)
    }

    fn emit_hungarian_algorithm_call(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let matrix_ty = Type::Array(Box::new(Type::Array(Box::new(Type::I32))));
        let pair_ty = Type::Array(Box::new(Type::Tuple(vec![Type::Usize, Type::Usize])));
        let result_ty = Type::Tuple(vec![Type::I32, pair_ty.clone()]);
        let (data, length) =
            self.emit_path_array_argument(&args[0], &matrix_ty, "hungarian.input")?;
        let cost = self
            .builder
            .build_alloca(self.llvm_type(&Type::I32), "hungarian.cost")
            .map_err(|e| e.to_string())?;
        let pairs = self
            .builder
            .build_alloca(self.llvm_type(&pair_ty), "hungarian.pairs")
            .map_err(|e| e.to_string())?;
        let function = self.declare_libc_fn(
            "tyrus_algorithm_hungarian",
            &[
                pointer.into(),
                self.usize_type().into(),
                pointer.into(),
                pointer.into(),
            ],
            None,
        );
        let metadata: Vec<BasicMetadataValueEnum<'ctx>> =
            vec![data, length, cost.into(), pairs.into()]
                .into_iter()
                .map(Into::into)
                .collect();
        self.builder
            .build_call(function, &metadata, "hungarian.call")
            .map_err(|e| e.to_string())?;
        let value = self
            .builder
            .build_load(self.llvm_type(&Type::I32), cost, "hungarian.cost.result")
            .map_err(|e| e.to_string())?;
        let matching = self
            .builder
            .build_load(self.llvm_type(&pair_ty), pairs, "hungarian.pairs.result")
            .map_err(|e| e.to_string())?;
        let tuple = self.llvm_type(&result_ty).into_struct_type().const_zero();
        let tuple = self
            .builder
            .build_insert_value(tuple, value, 0, "hungarian.result.cost")
            .map_err(|e| e.to_string())?;
        let result = self
            .builder
            .build_insert_value(tuple, matching, 1, "hungarian.result.pairs")
            .map_err(|e| e.to_string())?
            .into_struct_value()
            .into();
        self.record_owned_native_result(result, result_ty);
        Ok(result)
    }

    fn emit_sorting_algorithm_call(
        &mut self,
        target: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.is_empty()
            || (target == tyrus_core::algorithm::COUNTING_SORT_TARGET && args.len() != 2)
            || (target != tyrus_core::algorithm::COUNTING_SORT_TARGET && args.len() != 1)
        {
            return Err("error: sorting algorithm received an invalid number of arguments".into());
        }
        let array_ty = Type::Array(Box::new(Type::I32));
        let input = self.emit_expr_as(&args[0], &array_ty)?;
        let input_ptr = self
            .builder
            .build_alloca(self.llvm_type(&array_ty), "sort.input")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(input_ptr, input)
            .map_err(|e| e.to_string())?;
        let output = self
            .builder
            .build_alloca(self.llvm_type(&array_ty), "sort.output")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(output, self.llvm_type(&array_ty).const_zero())
            .map_err(|e| e.to_string())?;
        let runtime_name = match target {
            tyrus_core::algorithm::MERGE_SORT_TARGET => "tyrus_algorithm_merge_sort",
            tyrus_core::algorithm::QUICK_SORT_TARGET => "tyrus_algorithm_quick_sort",
            tyrus_core::algorithm::HEAP_SORT_TARGET => "tyrus_algorithm_heap_sort",
            tyrus_core::algorithm::COUNTING_SORT_TARGET => "tyrus_algorithm_counting_sort",
            tyrus_core::algorithm::RADIX_SORT_TARGET => "tyrus_algorithm_radix_sort",
            tyrus_core::algorithm::BUCKET_SORT_TARGET => "tyrus_algorithm_bucket_sort",
            tyrus_core::algorithm::TIM_SORT_TARGET => "tyrus_algorithm_tim_sort",
            tyrus_core::algorithm::INSERTION_SORT_TARGET => "tyrus_algorithm_insertion_sort",
            tyrus_core::algorithm::SHELL_SORT_TARGET => "tyrus_algorithm_shell_sort",
            tyrus_core::algorithm::SELECTION_SORT_TARGET => "tyrus_algorithm_selection_sort",
            tyrus_core::algorithm::BUBBLE_SORT_TARGET => "tyrus_algorithm_bubble_sort",
            tyrus_core::algorithm::SHAKER_SORT_TARGET => "tyrus_algorithm_shaker_sort",
            _ => return Err("error: unknown sorting algorithm".into()),
        };
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let mut values: Vec<BasicMetadataValueEnum<'ctx>> = vec![input_ptr.into()];
        let params = if target == tyrus_core::algorithm::COUNTING_SORT_TARGET {
            values.push(self.emit_expr_as(&args[1], &Type::I32)?.into());
            vec![
                pointer.into(),
                self.context.i32_type().into(),
                pointer.into(),
            ]
        } else {
            vec![pointer.into(), pointer.into()]
        };
        values.push(output.into());
        let function =
            self.declare_libc_fn(runtime_name, &params, Some(self.context.i32_type().into()));
        self.builder
            .build_call(function, &values, "sort.call")
            .map_err(|e| e.to_string())?;
        let result = self
            .builder
            .build_load(self.llvm_type(&array_ty), output, "sort.result")
            .map_err(|e| e.to_string())?;
        self.record_owned_native_result(result, array_ty);
        Ok(result)
    }

    fn emit_search_algorithm_call(
        &mut self,
        target: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 2 {
            return Err("error: search algorithm expects 2 arguments".into());
        }
        let array_ty = Type::Array(Box::new(Type::I32));
        let input = self.emit_expr_as(&args[0], &array_ty)?;
        let input_ptr = self
            .builder
            .build_alloca(self.llvm_type(&array_ty), "search.input")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(input_ptr, input)
            .map_err(|e| e.to_string())?;
        let index_result = target != tyrus_core::algorithm::QUICKSELECT_TARGET;
        let element_ty = if index_result { Type::Usize } else { Type::I32 };
        let query = if index_result {
            self.emit_expr_as(&args[1], &Type::I32)?
        } else {
            self.emit_expr_as(&args[1], &Type::Usize)?
        };
        let runtime_name = match target {
            tyrus_core::algorithm::QUICKSELECT_TARGET => "tyrus_algorithm_quickselect",
            tyrus_core::algorithm::BINARY_SEARCH_TARGET => "tyrus_algorithm_binary_search",
            tyrus_core::algorithm::INTERPOLATION_SEARCH_TARGET => {
                "tyrus_algorithm_interpolation_search"
            }
            tyrus_core::algorithm::EXPONENTIAL_SEARCH_TARGET => {
                "tyrus_algorithm_exponential_search"
            }
            _ => return Err("error: unknown search algorithm".into()),
        };
        let output = self
            .builder
            .build_alloca(self.llvm_type(&element_ty), "search.output")
            .map_err(|e| e.to_string())?;
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let function = self.declare_libc_fn(
            runtime_name,
            &[pointer.into(), query.get_type(), pointer.into()],
            Some(self.context.i32_type().into()),
        );
        let status = self
            .builder
            .build_call(
                function,
                &[input_ptr.into(), query.into(), output.into()],
                "search.call",
            )
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("error: search algorithm returned no status")?
            .into_int_value();
        let present = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::NE,
                status,
                self.context.i32_type().const_zero(),
                "search.present",
            )
            .map_err(|e| e.to_string())?;
        self.emit_binary_heap_optional(present, output, &element_ty, "search")
    }

    pub(super) fn emit_algorithm_static_method(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        let ExprKind::Ident(type_name) = object.kind() else {
            return None;
        };
        let type_name = match type_name.as_str() {
            "BitSet" | tyrus_core::algorithm::BIT_SET_NEW_TARGET => "BitSet",
            "LinkedList" | tyrus_core::algorithm::LINKED_LIST_NEW_TARGET => "LinkedList",
            "DoubleLinkedList" | tyrus_core::algorithm::DOUBLE_LINKED_LIST_NEW_TARGET => {
                "DoubleLinkedList"
            }
            _ => return None,
        };
        if method != "fromArray" {
            return Some(Err(format!(
                "error: unknown static method `{type_name}.{method}`"
            )));
        }
        let Some(source) = args.first() else {
            return Some(Err(format!(
                "error: `{type_name}.fromArray` expects 1 argument"
            )));
        };
        Some((|| {
            let array_ty = self
                .expr_type_hint(source)
                .ok_or_else(|| format!("error: cannot infer `{type_name}.fromArray` argument"))?;
            let Type::Array(element) = transparent_codegen_type(&array_ty) else {
                return Err(format!(
                    "error: `{type_name}.fromArray` requires an array argument"
                ));
            };
            let element = element.as_ref().clone();
            let array = self.emit_expr_as(source, &array_ty)?.into_struct_value();
            let data = self
                .builder
                .build_extract_value(array, 0, "algorithm.from_array.data")
                .map_err(|error| error.to_string())?
                .into_pointer_value();
            let length = self
                .builder
                .build_extract_value(array, 1, "algorithm.from_array.length")
                .map_err(|error| error.to_string())?
                .into_int_value();
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let (function, arguments, parameter_types, result_ty): (
                &str,
                Vec<BasicMetadataValueEnum<'ctx>>,
                Vec<BasicTypeEnum<'ctx>>,
                Type,
            ) = match type_name {
                "BitSet" => (
                    "tyrus_bit_set_from_array",
                    vec![data.into(), length.into()],
                    vec![pointer.into(), self.usize_type().into()],
                    Type::BitSet,
                ),
                "LinkedList" => (
                    "tyrus_linked_list_from_array",
                    vec![self.type_size(&element)?.into(), data.into(), length.into()],
                    vec![
                        self.usize_type().into(),
                        pointer.into(),
                        self.usize_type().into(),
                    ],
                    Type::LinkedList(Box::new(element)),
                ),
                "DoubleLinkedList" => (
                    "tyrus_double_linked_list_from_array",
                    vec![self.type_size(&element)?.into(), data.into(), length.into()],
                    vec![
                        self.usize_type().into(),
                        pointer.into(),
                        self.usize_type().into(),
                    ],
                    Type::DoubleLinkedList(Box::new(element)),
                ),
                _ => unreachable!(),
            };
            let function = self.declare_libc_fn(function, &parameter_types, Some(pointer.into()));
            let result = self
                .builder
                .build_call(function, &arguments, "algorithm.from_array")
                .map_err(|error| error.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or_else(|| format!("error: `{type_name}.fromArray` returned no value"))?;
            self.record_owned_native_result(result, result_ty);
            Ok(result)
        })())
    }

    pub(super) fn emit_algorithm_method_call(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        let obj_ty = self.expr_type_hint(object)?;
        let (ty_name, element_ty) = match &obj_ty {
            Type::BinaryHeap(element) => ("BinaryHeap".to_string(), Some(element.as_ref().clone())),
            Type::BinomialHeap(element) => {
                ("BinomialHeap".to_string(), Some(element.as_ref().clone()))
            }
            Type::UnionFind(element) => ("UnionFind".to_string(), Some(element.as_ref().clone())),
            Type::Counter(element) => ("Counter".to_string(), Some(element.as_ref().clone())),
            Type::Deque(element) => ("Deque".to_string(), Some(element.as_ref().clone())),
            Type::LinkedList(element) => ("LinkedList".to_string(), Some(element.as_ref().clone())),
            Type::DoubleLinkedList(element) => (
                "DoubleLinkedList".to_string(),
                Some(element.as_ref().clone()),
            ),
            Type::AvlTree(element) => ("AvlTree".to_string(), Some(element.as_ref().clone())),
            Type::RedBlackTree(element) => {
                ("RedBlackTree".to_string(), Some(element.as_ref().clone()))
            }
            Type::SplayTree(element) => ("SplayTree".to_string(), Some(element.as_ref().clone())),
            Type::Treap(element) => ("Treap".to_string(), Some(element.as_ref().clone())),
            Type::SortedSet(element) => ("SortedSet".to_string(), Some(element.as_ref().clone())),
            Type::SkipList(element) => ("SkipList".to_string(), Some(element.as_ref().clone())),
            Type::SortedMap(key, _) => ("SortedMap".to_string(), Some(key.as_ref().clone())),
            Type::Graph(node, _) => ("Graph".to_string(), Some(node.as_ref().clone())),
            Type::FenwickTree => ("FenwickTree".to_string(), None),
            Type::BubbleSort => ("BubbleSort".to_string(), None),
            Type::SegmentTree => ("SegmentTree".to_string(), None),
            Type::SparseTable => ("SparseTable".to_string(), None),
            Type::SegmentTreeLazy => ("SegmentTreeLazy".to_string(), None),
            Type::BitSet => ("BitSet".to_string(), None),
            Type::Trie => ("Trie".to_string(), None),
            Type::BloomFilter => ("BloomFilter".to_string(), None),
            Type::SuffixTree => ("SuffixTree".to_string(), None),
            Type::Lca => ("Lca".to_string(), None),
            Type::Hld => ("Hld".to_string(), None),
            Type::Centroid => ("Centroid".to_string(), None),
            Type::Named(name)
                if name == "BinaryHeap"
                    || name == "BinomialHeap"
                    || name == "UnionFind"
                    || name == "Counter"
                    || name == "Deque"
                    || name == "LinkedList"
                    || name == "DoubleLinkedList"
                    || name == "AvlTree"
                    || name == "RedBlackTree"
                    || name == "SplayTree"
                    || name == "Treap"
                    || name == "SortedSet"
                    || name == "SkipList"
                    || name == "SortedMap"
                    || name == "FenwickTree"
                    || name == "BubbleSort"
                    || name == "SegmentTree"
                    || name == "SparseTable"
                    || name == "SegmentTreeLazy"
                    || name == "BitSet"
                    || name == "Trie"
                    || name == "BloomFilter"
                    || name == "SuffixTree"
                    || name == "Lca"
                    || name == "Hld"
                    || name == "Centroid" =>
            {
                (name.clone(), None)
            }
            _ => return None,
        };

        if let Type::Graph(node, weight) = &obj_ty {
            if method == "edges" {
                return Some((|| {
                    let handle = self.emit_expr(object)?.into_pointer_value();
                    self.emit_graph_edges(handle, node, weight)
                })());
            }
            if method == "mapEdges"
                && let Some(callback) = args.first()
            {
                return Some((|| {
                    let handle = self.emit_expr(object)?.into_pointer_value();
                    let new_weight = self
                        .infer_callback_return_type(callback, &[(**weight).clone()])
                        .ok_or_else(|| {
                            "error: cannot infer `mapEdges` callback return type".to_string()
                        })?;
                    self.emit_graph_map_edges(handle, node, weight, &new_weight, callback)
                })());
            }
            if method == "mapNodes"
                && let Some(callback) = args.first()
            {
                return Some((|| {
                    let handle = self.emit_expr(object)?.into_pointer_value();
                    let new_node = self
                        .infer_callback_return_type(callback, &[(**node).clone()])
                        .ok_or_else(|| {
                            "error: cannot infer `mapNodes` callback return type".to_string()
                        })?;
                    self.emit_graph_map_nodes(handle, node, weight, &new_node, callback)
                })());
            }
        }
        if let Some(target) = tyrus_core::algorithm::algorithm_method_target(&ty_name, method) {
            let mut full_args = vec![object.clone()];
            full_args.extend(args.iter().cloned());
            let map_value_ty = match &obj_ty {
                Type::SortedMap(_, value) => Some(value.as_ref()),
                Type::Graph(_, weight) => Some(weight.as_ref()),
                _ => None,
            };
            let result = self.emit_algorithm_runtime_call(
                target,
                &full_args,
                element_ty.as_ref(),
                map_value_ty,
            );
            if matches!(
                target,
                tyrus_core::algorithm::BIT_SET_UNION_TARGET
                    | tyrus_core::algorithm::BIT_SET_INTERSECTION_TARGET
                    | tyrus_core::algorithm::BIT_SET_DIFFERENCE_TARGET
                    | tyrus_core::algorithm::BIT_SET_COMPLEMENT_TARGET
            ) && let Ok(value) = result
            {
                self.record_owned_native_result(value, Type::BitSet);
            }
            let optional_ty = if matches!(
                target,
                tyrus_core::algorithm::SORTED_SET_MIN_TARGET
                    | tyrus_core::algorithm::SORTED_SET_MAX_TARGET
            ) {
                match &obj_ty {
                    Type::SortedSet(element) => Some(Type::Optional(Box::new((**element).clone()))),
                    _ => None,
                }
            } else if matches!(
                target,
                tyrus_core::algorithm::SKIP_LIST_MIN_TARGET
                    | tyrus_core::algorithm::SKIP_LIST_MAX_TARGET
            ) {
                match &obj_ty {
                    Type::SkipList(element) => Some(Type::Optional(Box::new((**element).clone()))),
                    _ => None,
                }
            } else if matches!(
                target,
                tyrus_core::algorithm::SORTED_MAP_MIN_TARGET
                    | tyrus_core::algorithm::SORTED_MAP_MAX_TARGET
            ) {
                match &obj_ty {
                    Type::SortedMap(key, _) => Some(Type::Optional(Box::new((**key).clone()))),
                    _ => None,
                }
            } else if matches!(target, tyrus_core::algorithm::SORTED_MAP_GET_TARGET) {
                match &obj_ty {
                    Type::SortedMap(_, value) => Some(Type::Optional(Box::new((**value).clone()))),
                    _ => None,
                }
            } else if matches!(target, tyrus_core::algorithm::GRAPH_EDGE_TARGET) {
                match &obj_ty {
                    Type::Graph(_, weight) => Some(Type::Optional(Box::new((**weight).clone()))),
                    _ => None,
                }
            } else {
                None
            };
            if let Some(optional_ty) = optional_ty
                && let Ok(value) = result
            {
                self.record_owned_native_result_with(value, optional_ty, crate::ReleasePlan::Value);
            }
            return Some(result);
        }

        None
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_algorithm_runtime_call(
        &mut self,
        target: &str,
        args: &[Expr],
        element_type: Option<&Type>,
        value_type: Option<&Type>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());

        let sort_info_runtime = match target {
            tyrus_core::algorithm::MERGE_SORT_INFO_TARGET => {
                Some("tyrus_algorithm_merge_sort_info")
            }
            tyrus_core::algorithm::QUICK_SORT_INFO_TARGET => {
                Some("tyrus_algorithm_quick_sort_info")
            }
            tyrus_core::algorithm::HEAP_SORT_INFO_TARGET => Some("tyrus_algorithm_heap_sort_info"),
            tyrus_core::algorithm::COUNTING_SORT_INFO_TARGET => {
                Some("tyrus_algorithm_counting_sort_info")
            }
            tyrus_core::algorithm::RADIX_SORT_INFO_TARGET => {
                Some("tyrus_algorithm_radix_sort_info")
            }
            tyrus_core::algorithm::BUCKET_SORT_INFO_TARGET => {
                Some("tyrus_algorithm_bucket_sort_info")
            }
            tyrus_core::algorithm::TIM_SORT_INFO_TARGET => Some("tyrus_algorithm_tim_sort_info"),
            tyrus_core::algorithm::INSERTION_SORT_INFO_TARGET => {
                Some("tyrus_algorithm_insertion_sort_info")
            }
            tyrus_core::algorithm::SHELL_SORT_INFO_TARGET => {
                Some("tyrus_algorithm_shell_sort_info")
            }
            tyrus_core::algorithm::SELECTION_SORT_INFO_TARGET => {
                Some("tyrus_algorithm_selection_sort_info")
            }
            tyrus_core::algorithm::SHAKER_SORT_INFO_TARGET => {
                Some("tyrus_algorithm_shaker_sort_info")
            }
            _ => None,
        };
        if let Some(runtime_name) = sort_info_runtime {
            let counting = target == tyrus_core::algorithm::COUNTING_SORT_INFO_TARGET;
            if args.len() != if counting { 2 } else { 1 } {
                return Err(
                    "error: sorting study function received an invalid argument count".into(),
                );
            }
            let array_ty = Type::Array(Box::new(Type::I32));
            let input = self.emit_expr_as(&args[0], &array_ty)?;
            let input_ptr = self
                .builder
                .build_alloca(self.llvm_type(&array_ty), "sort_info.input")
                .map_err(|error| error.to_string())?;
            self.builder
                .build_store(input_ptr, input)
                .map_err(|error| error.to_string())?;
            let result_ty = Type::Named("SortInfo".to_string());
            let llvm_result_ty = self.llvm_type(&result_ty);
            let output = self
                .builder
                .build_alloca(llvm_result_ty, "sort_info.output")
                .map_err(|error| error.to_string())?;
            self.builder
                .build_store(output, llvm_result_ty.const_zero())
                .map_err(|error| error.to_string())?;
            let max_value = if counting {
                self.emit_expr_as(&args[1], &Type::I32)?
            } else {
                self.context.i32_type().const_zero().into()
            };
            let function = self.declare_libc_fn(
                runtime_name,
                &[
                    pointer.into(),
                    self.context.i32_type().into(),
                    pointer.into(),
                ],
                Some(self.context.i32_type().into()),
            );
            self.builder
                .build_call(
                    function,
                    &[input_ptr.into(), max_value.into(), output.into()],
                    "sort_info.call",
                )
                .map_err(|error| error.to_string())?;
            let result = self
                .builder
                .build_load(llvm_result_ty, output, "sort_info.result")
                .map_err(|error| error.to_string())?;
            self.record_owned_native_result(result, result_ty);
            return Ok(result);
        }

        if matches!(
            target,
            tyrus_core::algorithm::BUBBLE_SORT_INFO_TARGET
                | tyrus_core::algorithm::BINARY_SEARCH_INFO_TARGET
                | tyrus_core::algorithm::INTERPOLATION_SEARCH_INFO_TARGET
                | tyrus_core::algorithm::EXPONENTIAL_SEARCH_INFO_TARGET
                | tyrus_core::algorithm::QUICKSELECT_INFO_TARGET
        ) {
            let bubble = target == tyrus_core::algorithm::BUBBLE_SORT_INFO_TARGET;
            if args.len() != if bubble { 1 } else { 2 } {
                return Err("error: study algorithm received an invalid argument count".into());
            }
            let array_ty = Type::Array(Box::new(Type::I32));
            let input = self.emit_expr_as(&args[0], &array_ty)?;
            let input_slot = self
                .builder
                .build_alloca(self.llvm_type(&array_ty), "bubble_sort_info.input")
                .map_err(|error| error.to_string())?;
            self.builder
                .build_store(input_slot, input)
                .map_err(|error| error.to_string())?;
            let (result_name, runtime_name) = match target {
                tyrus_core::algorithm::BUBBLE_SORT_INFO_TARGET => {
                    ("BubbleSortInfo", "tyrus_algorithm_bubble_sort_info")
                }
                tyrus_core::algorithm::BINARY_SEARCH_INFO_TARGET => {
                    ("SearchInfo", "tyrus_algorithm_binary_search_info")
                }
                tyrus_core::algorithm::INTERPOLATION_SEARCH_INFO_TARGET => (
                    "InterpolationSearchInfo",
                    "tyrus_algorithm_interpolation_search_info",
                ),
                tyrus_core::algorithm::EXPONENTIAL_SEARCH_INFO_TARGET => (
                    "ExponentialSearchInfo",
                    "tyrus_algorithm_exponential_search_info",
                ),
                tyrus_core::algorithm::QUICKSELECT_INFO_TARGET => {
                    ("SelectionInfo", "tyrus_algorithm_quickselect_info")
                }
                _ => unreachable!(),
            };
            let result_ty = Type::Named(result_name.to_string());
            let llvm_result_ty = self.llvm_type(&result_ty);
            let output = self
                .builder
                .build_alloca(llvm_result_ty, "study_info.output")
                .map_err(|error| error.to_string())?;
            self.builder
                .build_store(output, llvm_result_ty.const_zero())
                .map_err(|error| error.to_string())?;
            let query = if bubble {
                None
            } else if target == tyrus_core::algorithm::QUICKSELECT_INFO_TARGET {
                Some(self.emit_expr_as(&args[1], &Type::Usize)?)
            } else {
                Some(self.emit_expr_as(&args[1], &Type::I32)?)
            };
            let mut parameter_types = vec![pointer.into()];
            if let Some(query) = query {
                parameter_types.push(query.get_type());
            }
            parameter_types.push(pointer.into());
            let function = self.declare_libc_fn(
                runtime_name,
                &parameter_types,
                Some(self.context.i32_type().into()),
            );
            let mut call_args = vec![input_slot.into()];
            if let Some(query) = query {
                call_args.push(query.into());
            }
            call_args.push(output.into());
            self.builder
                .build_call(function, &call_args, "study_info.call")
                .map_err(|error| error.to_string())?;
            let result = self
                .builder
                .build_load(llvm_result_ty, output, "study_info.result")
                .map_err(|error| error.to_string())?;
            self.record_owned_native_result(result, result_ty);
            return Ok(result);
        }

        if matches!(
            target,
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
                | tyrus_core::algorithm::SHAKER_SORT_INFO_TARGET
        ) {
            if target == tyrus_core::algorithm::MERGE_SORT_INFO_TARGET
                || target == tyrus_core::algorithm::QUICK_SORT_INFO_TARGET
                || target == tyrus_core::algorithm::HEAP_SORT_INFO_TARGET
                || target == tyrus_core::algorithm::COUNTING_SORT_INFO_TARGET
                || target == tyrus_core::algorithm::RADIX_SORT_INFO_TARGET
                || target == tyrus_core::algorithm::BUCKET_SORT_INFO_TARGET
                || target == tyrus_core::algorithm::TIM_SORT_INFO_TARGET
                || target == tyrus_core::algorithm::INSERTION_SORT_INFO_TARGET
                || target == tyrus_core::algorithm::SHELL_SORT_INFO_TARGET
                || target == tyrus_core::algorithm::SELECTION_SORT_INFO_TARGET
                || target == tyrus_core::algorithm::SHAKER_SORT_INFO_TARGET
            {
                return self.emit_algorithm_runtime_call(target, args, None, None);
            }
            return self.emit_sorting_algorithm_call(target, args);
        }
        if matches!(
            target,
            tyrus_core::algorithm::QUICKSELECT_TARGET
                | tyrus_core::algorithm::BINARY_SEARCH_TARGET
                | tyrus_core::algorithm::INTERPOLATION_SEARCH_TARGET
                | tyrus_core::algorithm::EXPONENTIAL_SEARCH_TARGET
                | tyrus_core::algorithm::QUICKSELECT_INFO_TARGET
                | tyrus_core::algorithm::BINARY_SEARCH_INFO_TARGET
                | tyrus_core::algorithm::INTERPOLATION_SEARCH_INFO_TARGET
                | tyrus_core::algorithm::EXPONENTIAL_SEARCH_INFO_TARGET
        ) {
            return self.emit_search_algorithm_call(target, args);
        }

        if matches!(
            target,
            tyrus_core::algorithm::IDDFS_TARGET
                | tyrus_core::algorithm::IDDFS_INFO_TARGET
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
                | tyrus_core::algorithm::HUNGARIAN_TARGET
        ) {
            return if target == tyrus_core::algorithm::HUNGARIAN_TARGET {
                self.emit_hungarian_algorithm_call(args)
            } else if target == tyrus_core::algorithm::TOPOLOGICAL_KAHN_INFO_TARGET {
                self.emit_topological_kahn_info_call(args)
            } else if target == tyrus_core::algorithm::TOPOLOGICAL_DFS_INFO_TARGET {
                self.emit_topological_dfs_info_call(args)
            } else if matches!(
                target,
                tyrus_core::algorithm::TARJAN_SCC_INFO_TARGET
                    | tyrus_core::algorithm::KOSARAJU_SCC_INFO_TARGET
            ) {
                self.emit_scc_info_call(target, args)
            } else if matches!(
                target,
                tyrus_core::algorithm::FLEURY_EULERIAN_INFO_TARGET
                    | tyrus_core::algorithm::HIERHOLZER_EULERIAN_INFO_TARGET
            ) {
                self.emit_euler_info_call(target, args)
            } else if matches!(
                target,
                tyrus_core::algorithm::FIND_BRIDGES_INFO_TARGET
                    | tyrus_core::algorithm::FIND_CUT_VERTICES_INFO_TARGET
                    | tyrus_core::algorithm::BICONNECTED_COMPONENTS_INFO_TARGET
            ) {
                self.emit_connectivity_info_call(target, args)
            } else if matches!(
                target,
                tyrus_core::algorithm::EDMONDS_BLOSSOM_INFO_TARGET
                    | tyrus_core::algorithm::HUNGARIAN_INFO_TARGET
            ) {
                self.emit_matching_info_call(target, args)
            } else if matches!(
                target,
                tyrus_core::algorithm::TOPOLOGICAL_KAHN_TARGET
                    | tyrus_core::algorithm::TOPOLOGICAL_DFS_TARGET
                    | tyrus_core::algorithm::TARJAN_SCC_TARGET
                    | tyrus_core::algorithm::KOSARAJU_SCC_TARGET
                    | tyrus_core::algorithm::FLEURY_EULERIAN_TARGET
                    | tyrus_core::algorithm::HIERHOLZER_EULERIAN_TARGET
                    | tyrus_core::algorithm::FIND_BRIDGES_TARGET
                    | tyrus_core::algorithm::FIND_CUT_VERTICES_TARGET
                    | tyrus_core::algorithm::BICONNECTED_COMPONENTS_TARGET
                    | tyrus_core::algorithm::EDMONDS_BLOSSOM_TARGET
            ) {
                self.emit_structure_algorithm_call(target, args)
            } else if matches!(
                target,
                tyrus_core::algorithm::EDMONDS_KARP_TARGET
                    | tyrus_core::algorithm::DINIC_TARGET
                    | tyrus_core::algorithm::PUSH_RELABEL_TARGET
                    | tyrus_core::algorithm::MIN_CUT_TARGET
                    | tyrus_core::algorithm::MIN_COST_MAX_FLOW_TARGET
            ) {
                self.emit_flow_algorithm_call(target, args)
            } else if matches!(
                target,
                tyrus_core::algorithm::EDMONDS_KARP_INFO_TARGET
                    | tyrus_core::algorithm::DINIC_INFO_TARGET
                    | tyrus_core::algorithm::PUSH_RELABEL_INFO_TARGET
                    | tyrus_core::algorithm::MIN_CUT_INFO_TARGET
                    | tyrus_core::algorithm::MIN_COST_MAX_FLOW_INFO_TARGET
            ) {
                self.emit_flow_info_algorithm_call(target, args)
            } else if matches!(
                target,
                tyrus_core::algorithm::KRUSKAL_TARGET
                    | tyrus_core::algorithm::KRUSKAL_INFO_TARGET
                    | tyrus_core::algorithm::PRIM_TARGET
                    | tyrus_core::algorithm::PRIM_INFO_TARGET
                    | tyrus_core::algorithm::BORUVKA_TARGET
                    | tyrus_core::algorithm::BORUVKA_INFO_TARGET
                    | tyrus_core::algorithm::STEINER_TREE_TARGET
                    | tyrus_core::algorithm::STEINER_TREE_INFO_TARGET
            ) {
                self.emit_mst_algorithm_call(target, args)
            } else {
                self.emit_path_algorithm_call(target, args)
            };
        }

        if let Some(result) = self.emit_union_find_heap_call(target, args, element_type)? {
            return Ok(result);
        }
        if let Some(result) = self.emit_deque_tree_call(target, args, element_type)? {
            return Ok(result);
        }
        if let Some(result) =
            self.emit_sorted_container_call(target, args, element_type, value_type)?
        {
            return Ok(result);
        }
        if let Some(result) =
            self.emit_graph_container_call(target, args, element_type, value_type)?
        {
            return Ok(result);
        }
        if let Some(result) = self.emit_compact_structure_call(target, args)? {
            return Ok(result);
        }
        if let Some(result) = self.emit_linked_list_call(target, args, element_type)? {
            return Ok(result);
        }
        let result = match target {
            tyrus_core::algorithm::BFS_TARGET
            | tyrus_core::algorithm::DFS_TARGET
            | tyrus_core::algorithm::BFS_INFO_TARGET
            | tyrus_core::algorithm::DFS_INFO_TARGET => {
                if args.len() != 2 {
                    return Err("error: graph traversal expects 2 arguments".into());
                }
                let input_ty = self
                    .expr_type_hint(&args[0])
                    .ok_or("error: cannot infer graph traversal input type")?;
                let output_ty = match target {
                    tyrus_core::algorithm::BFS_INFO_TARGET
                    | tyrus_core::algorithm::DFS_INFO_TARGET => {
                        Type::Named("TraversalInfo".to_string())
                    }
                    _ => Type::Array(Box::new(Type::Usize)),
                };
                let output = self
                    .builder
                    .build_alloca(self.llvm_type(&output_ty), "path.traversal.output")
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_store(output, self.llvm_type(&output_ty).const_zero())
                    .map_err(|e| e.to_string())?;
                let start = self.emit_expr_as(&args[1], &Type::Usize)?;
                let runtime_name = match target {
                    tyrus_core::algorithm::BFS_TARGET => "tyrus_algorithm_bfs",
                    tyrus_core::algorithm::DFS_TARGET => "tyrus_algorithm_dfs",
                    tyrus_core::algorithm::BFS_INFO_TARGET => "tyrus_algorithm_bfs_info",
                    _ => "tyrus_algorithm_dfs_info",
                };
                let result = match transparent_codegen_type(&input_ty) {
                    Type::Graph(_, _) => {
                        let graph = self.emit_expr(&args[0])?;
                        let fn_val = self.declare_libc_fn(
                            &format!("{runtime_name}_graph"),
                            &[pointer.into(), self.usize_type().into(), pointer.into()],
                            None,
                        );
                        self.builder
                            .build_call(
                                fn_val,
                                &[graph.into(), start.into(), output.into()],
                                "path.traversal",
                            )
                            .map_err(|e| e.to_string())?;
                        self.builder
                            .build_load(self.llvm_type(&output_ty), output, "path.traversal.result")
                            .map_err(|e| e.to_string())?
                    }
                    Type::Array(_) => {
                        let adjacency = self.emit_expr_as(&args[0], &input_ty)?.into_struct_value();
                        let data = self
                            .builder
                            .build_extract_value(adjacency, 0, "path.adj.data")
                            .map_err(|e| e.to_string())?;
                        let length = self
                            .builder
                            .build_extract_value(adjacency, 1, "path.adj.length")
                            .map_err(|e| e.to_string())?;
                        let fn_val = self.declare_libc_fn(
                            runtime_name,
                            &[
                                pointer.into(),
                                self.usize_type().into(),
                                self.usize_type().into(),
                                pointer.into(),
                            ],
                            None,
                        );
                        self.builder
                            .build_call(
                                fn_val,
                                &[data.into(), length.into(), start.into(), output.into()],
                                "path.traversal",
                            )
                            .map_err(|e| e.to_string())?;
                        self.builder
                            .build_load(self.llvm_type(&output_ty), output, "path.traversal.result")
                            .map_err(|e| e.to_string())?
                    }
                    _ => {
                        return Err(
                            "error: graph traversal requires an adjacency array or Graph".into(),
                        );
                    }
                };
                self.record_owned_native_result(result, output_ty);
                result
            }
            _ => {
                return Err(format!(
                    "error: unknown algorithm runtime target `{target}`"
                ));
            }
        };

        Ok(result)
    }

    fn emit_binary_heap_optional(
        &mut self,
        present: inkwell::values::IntValue<'ctx>,
        output: inkwell::values::PointerValue<'ctx>,
        element_type: &Type,
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: heap optional outside function")?;
        let yes = self
            .context
            .append_basic_block(function, &format!("{name}.present"));
        let no = self
            .context
            .append_basic_block(function, &format!("{name}.absent"));
        let merge = self
            .context
            .append_basic_block(function, &format!("{name}.merge"));
        self.builder
            .build_conditional_branch(present, yes, no)
            .map_err(|e| e.to_string())?;
        self.builder.position_at_end(no);
        let null = self
            .context
            .ptr_type(inkwell::AddressSpace::default())
            .const_null();
        self.builder
            .build_unconditional_branch(merge)
            .map_err(|e| e.to_string())?;
        let no_end = self.builder.get_insert_block().unwrap();
        self.builder.position_at_end(yes);
        let value = self
            .builder
            .build_load(
                self.llvm_type(element_type),
                output,
                &format!("{name}.value"),
            )
            .map_err(|e| e.to_string())?;
        let boxed = self.emit_optional_box(value, element_type)?;
        self.builder
            .build_unconditional_branch(merge)
            .map_err(|e| e.to_string())?;
        let yes_end = self.builder.get_insert_block().unwrap();
        self.builder.position_at_end(merge);
        let phi = self
            .builder
            .build_phi(
                self.context.ptr_type(inkwell::AddressSpace::default()),
                &format!("{name}.optional"),
            )
            .map_err(|e| e.to_string())?;
        phi.add_incoming(&[(&null, no_end), (&boxed, yes_end)]);
        let result = phi.as_basic_value();
        self.record_owned_native_result_with(
            result,
            Type::Optional(Box::new(element_type.clone())),
            crate::ReleasePlan::Value,
        );
        Ok(result)
    }

    /// Emits an `extern "C"` comparator function for an ordered element type.
    ///
    /// The function signature is `fn(*const u8, *const u8) -> i32`:
    /// - returns > 0 if a > b
    /// - returns < 0 if a < b
    /// - returns 0 if a == b
    ///
    /// The function is cached per element type in the current module so that
    /// BinaryHeap, the balanced trees and other ordered containers share it.
    fn emit_ordered_comparator(
        &mut self,
        element_type: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i8_ptr = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_type = self.context.i32_type();

        let fn_name = format!("__tyrus_cmp_{element_type}");
        if let Some(existing) = self.module.get_function(&fn_name) {
            return Ok(existing.as_global_value().as_pointer_value().into());
        }

        let function_type = i32_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
        let function = self.module.add_function(&fn_name, function_type, None);

        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        let a_ptr = function.get_nth_param(0).unwrap().into_pointer_value();
        let b_ptr = function.get_nth_param(1).unwrap().into_pointer_value();

        let llvm_ty = self.llvm_type(element_type);
        let typed_ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let a_typed = self
            .builder
            .build_pointer_cast(a_ptr, typed_ptr_ty, "a.typed")
            .map_err(|e| e.to_string())?;
        let b_typed = self
            .builder
            .build_pointer_cast(b_ptr, typed_ptr_ty, "b.typed")
            .map_err(|e| e.to_string())?;

        let a_val = self
            .builder
            .build_load(llvm_ty, a_typed, "a.val")
            .map_err(|e| e.to_string())?;
        let b_val = self
            .builder
            .build_load(llvm_ty, b_typed, "b.val")
            .map_err(|e| e.to_string())?;

        let one = i32_type.const_int(1, false);
        let neg_one = i32_type.const_int(u64::MAX, false);
        let zero = i32_type.const_zero();

        let cmp_result = match element_type {
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 | Type::Isize => {
                let a_int = a_val.into_int_value();
                let b_int = b_val.into_int_value();
                let gt = self
                    .builder
                    .build_int_compare(inkwell::IntPredicate::SGT, a_int, b_int, "gt")
                    .map_err(|e| e.to_string())?;
                let eq = self
                    .builder
                    .build_int_compare(inkwell::IntPredicate::EQ, a_int, b_int, "eq")
                    .map_err(|e| e.to_string())?;
                let select_gt = self
                    .builder
                    .build_select(gt, one, neg_one, "sel")
                    .map_err(|e| e.to_string())?
                    .into_int_value();
                self.builder
                    .build_select(eq, zero, select_gt, "cmp")
                    .map_err(|e| e.to_string())?
                    .into_int_value()
            }
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::Usize => {
                let a_int = a_val.into_int_value();
                let b_int = b_val.into_int_value();
                let gt = self
                    .builder
                    .build_int_compare(inkwell::IntPredicate::UGT, a_int, b_int, "gt")
                    .map_err(|e| e.to_string())?;
                let eq = self
                    .builder
                    .build_int_compare(inkwell::IntPredicate::EQ, a_int, b_int, "eq")
                    .map_err(|e| e.to_string())?;
                let select_gt = self
                    .builder
                    .build_select(gt, one, neg_one, "sel")
                    .map_err(|e| e.to_string())?
                    .into_int_value();
                self.builder
                    .build_select(eq, zero, select_gt, "cmp")
                    .map_err(|e| e.to_string())?
                    .into_int_value()
            }
            Type::F32 | Type::F64 | Type::F128 => {
                let a_float = a_val.into_float_value();
                let b_float = b_val.into_float_value();
                let gt = self
                    .builder
                    .build_float_compare(inkwell::FloatPredicate::OGT, a_float, b_float, "gt")
                    .map_err(|e| e.to_string())?;
                let eq = self
                    .builder
                    .build_float_compare(inkwell::FloatPredicate::OEQ, a_float, b_float, "eq")
                    .map_err(|e| e.to_string())?;
                let select_gt = self
                    .builder
                    .build_select(gt, one, neg_one, "sel")
                    .map_err(|e| e.to_string())?
                    .into_int_value();
                self.builder
                    .build_select(eq, zero, select_gt, "cmp")
                    .map_err(|e| e.to_string())?
                    .into_int_value()
            }
            Type::Bool => {
                let a_int = self
                    .builder
                    .build_int_z_extend(a_val.into_int_value(), i32_type, "a.ext")
                    .map_err(|e| e.to_string())?;
                let b_int = self
                    .builder
                    .build_int_z_extend(b_val.into_int_value(), i32_type, "b.ext")
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_int_sub(a_int, b_int, "cmp")
                    .map_err(|e| e.to_string())?
            }
            Type::Str => self.emit_string_compare(
                a_val.into_struct_value(),
                b_val.into_struct_value(),
                "cmp",
            )?,
            _ => {
                return Err(format!(
                    "error: ordered container does not support element type `{element_type}`"
                ));
            }
        };

        self.builder
            .build_return(Some(&cmp_result))
            .map_err(|e| e.to_string())?;

        Ok(function.as_global_value().as_pointer_value().into())
    }
}

fn graph_type_tag(ty: &Type) -> u64 {
    let ty = transparent_codegen_type(ty);
    if matches!(ty, Type::Str) {
        tyrus_core::algorithm::GRAPH_TAG_STRING
    } else if matches!(ty, Type::Bool) {
        tyrus_core::algorithm::GRAPH_TAG_BOOL
    } else if is_unsigned_int_type(ty) {
        tyrus_core::algorithm::GRAPH_TAG_UNSIGNED
    } else if is_float_type(ty) {
        tyrus_core::algorithm::GRAPH_TAG_FLOAT
    } else {
        tyrus_core::algorithm::GRAPH_TAG_SIGNED
    }
}
