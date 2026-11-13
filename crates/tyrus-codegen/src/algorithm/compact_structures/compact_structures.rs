use super::*;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_compact_structure_call(
        &mut self,
        target: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let result = match target {
            tyrus_core::algorithm::SEGMENT_TREE_LAZY_UPDATE_RANGE_TARGET => {
                if args.len() != 4 {
                    return Err("error: `updateRange` expects 3 arguments".into());
                }
                let st = self.emit_expr(&args[0])?;
                let from = self.emit_expr_as(&args[1], &Type::Usize)?;
                let to = self.emit_expr_as(&args[2], &Type::Usize)?;
                let delta = self.emit_expr_as(&args[3], &Type::I32)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_segment_tree_lazy_update_range",
                    &[
                        pointer.into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                        self.context.i32_type().into(),
                    ],
                    None,
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[st.into(), from.into(), to.into(), delta.into()],
                        "segment_tree_lazy.update_range",
                    )
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::SEGMENT_TREE_LAZY_QUERY_TARGET => {
                if args.len() != 3 {
                    return Err("error: `query` expects 2 arguments".into());
                }
                let st = self.emit_expr(&args[0])?;
                let from = self.emit_expr_as(&args[1], &Type::Usize)?;
                let to = self.emit_expr_as(&args[2], &Type::Usize)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_segment_tree_lazy_query",
                    &[
                        pointer.into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                    ],
                    Some(self.context.i32_type().into()),
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[st.into(), from.into(), to.into()],
                        "segment_tree_lazy.query",
                    )
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: segment_tree_lazy_query did not return a value")?
            }
            tyrus_core::algorithm::SEGMENT_TREE_LAZY_LEN_TARGET => {
                if args.len() != 1 {
                    return Err("error: `len` expects 0 arguments".into());
                }
                let st = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_segment_tree_lazy_len",
                    &[pointer.into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[st.into()], "segment_tree_lazy.len")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: segment_tree_lazy_len did not return a value")?
            }
            tyrus_core::algorithm::SEGMENT_TREE_LAZY_IS_EMPTY_TARGET => {
                if args.len() != 1 {
                    return Err("error: `isEmpty` expects 0 arguments".into());
                }
                let st = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_segment_tree_lazy_is_empty",
                    &[pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[st.into()], "segment_tree_lazy.is_empty")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: segment_tree_lazy_is_empty did not return a value")?
            }
            tyrus_core::algorithm::BIT_SET_NEW_TARGET => {
                if args.len() != 1 {
                    return Err("error: `BitSet` expects 1 argument".into());
                }
                let size = self.emit_expr_as(&args[0], &Type::Usize)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_bit_set_new",
                    &[self.usize_type().into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(fn_val, &[size.into()], "bitset.new")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: bit_set_new did not return a value")?
            }
            tyrus_core::algorithm::TRIE_NEW_TARGET => {
                if !args.is_empty() {
                    return Err("error: `Trie` expects 0 arguments".into());
                }
                let fn_val = self.declare_libc_fn("tyrus_trie_new", &[], Some(pointer.into()));
                self.builder
                    .build_call(fn_val, &[], "trie.new")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: trie_new did not return a value")?
            }
            tyrus_core::algorithm::SUFFIX_TREE_NEW_TARGET => {
                if args.len() != 1 {
                    return Err("error: `suffixTreeUkkonen` expects 1 argument".into());
                }
                let text_value = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let text = self.string_data_ptr(text_value)?;
                let _strlen = self.declare_libc_fn(
                    "strlen",
                    &[self
                        .context
                        .ptr_type(inkwell::AddressSpace::default())
                        .into()],
                    Some(self.usize_type().into()),
                );
                let length = self.string_byte_len(text_value)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_suffix_tree_new",
                    &[pointer.into(), self.usize_type().into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(fn_val, &[text.into(), length.into()], "suffix_tree.new")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: suffix_tree_new did not return a value")?
            }
            tyrus_core::algorithm::SUFFIX_TREE_GET_SUFFIX_ARRAY_TARGET => {
                if args.len() != 1 {
                    return Err("error: `getSuffixArray` expects 0 arguments".into());
                }
                let tree = self.emit_expr(&args[0])?;
                let array_ty = Type::Array(Box::new(Type::Usize));
                let output = self
                    .builder
                    .build_alloca(self.llvm_type(&array_ty), "suffix_tree.suffix_array.output")
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_store(output, self.llvm_type(&array_ty).const_zero())
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_suffix_tree_get_suffix_array",
                    &[pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[tree.into(), output.into()],
                        "suffix_tree.get_suffix_array",
                    )
                    .map_err(|e| e.to_string())?;
                let result = self
                    .builder
                    .build_load(
                        self.llvm_type(&array_ty),
                        output,
                        "suffix_tree.suffix_array.result",
                    )
                    .map_err(|e| e.to_string())?;
                self.record_owned_native_result(result, array_ty);
                result
            }
            tyrus_core::algorithm::LCA_NEW_TARGET => {
                if args.len() != 2 {
                    return Err("error: `lcaBinaryLifting` expects 2 arguments".into());
                }
                let adj_ty = Type::Array(Box::new(Type::Array(Box::new(Type::Usize))));
                let adj = self.emit_expr_as(&args[0], &adj_ty)?.into_struct_value();
                let adj_data = self
                    .builder
                    .build_extract_value(adj, 0, "lca.adj.data")
                    .map_err(|e| e.to_string())?
                    .into_pointer_value();
                let adj_length = self
                    .builder
                    .build_extract_value(adj, 1, "lca.adj.length")
                    .map_err(|e| e.to_string())?
                    .into_int_value();
                let root = self.emit_expr_as(&args[1], &Type::Usize)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_lca_new",
                    &[
                        pointer.into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                    ],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[adj_data.into(), adj_length.into(), root.into()],
                        "lca.new",
                    )
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: lca_new did not return a value")?
            }
            tyrus_core::algorithm::LCA_QUERY_TARGET => {
                if args.len() != 3 {
                    return Err("error: `query` expects 2 arguments".into());
                }
                let lca = self.emit_expr(&args[0])?;
                let u = self.emit_expr_as(&args[1], &Type::Usize)?;
                let v = self.emit_expr_as(&args[2], &Type::Usize)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_lca_query",
                    &[
                        pointer.into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                    ],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[lca.into(), u.into(), v.into()], "lca.query")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: lca_query did not return a value")?
            }
            tyrus_core::algorithm::HLD_NEW_TARGET => {
                if args.len() != 2 {
                    return Err("error: `heavyLightDecomposition` expects 2 arguments".into());
                }
                let adj_ty = Type::Array(Box::new(Type::Array(Box::new(Type::Usize))));
                let adj = self.emit_expr_as(&args[0], &adj_ty)?.into_struct_value();
                let adj_data = self
                    .builder
                    .build_extract_value(adj, 0, "hld.adj.data")
                    .map_err(|e| e.to_string())?
                    .into_pointer_value();
                let adj_length = self
                    .builder
                    .build_extract_value(adj, 1, "hld.adj.length")
                    .map_err(|e| e.to_string())?
                    .into_int_value();
                let root = self.emit_expr_as(&args[1], &Type::Usize)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_hld_new",
                    &[
                        pointer.into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                    ],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[adj_data.into(), adj_length.into(), root.into()],
                        "hld.new",
                    )
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: hld_new did not return a value")?
            }
            tyrus_core::algorithm::HLD_QUERY_PATH_TARGET => {
                if args.len() != 3 {
                    return Err("error: `queryPath` expects 2 arguments".into());
                }
                let hld = self.emit_expr(&args[0])?;
                let u = self.emit_expr_as(&args[1], &Type::Usize)?;
                let v = self.emit_expr_as(&args[2], &Type::Usize)?;
                let elem_ty = Type::Tuple(vec![Type::Usize, Type::Usize]);
                let pair_stride = std::mem::size_of::<usize>() * 2;
                let array_type = self.llvm_type(&Type::Array(Box::new(elem_ty)));
                let output = self
                    .builder
                    .build_alloca(array_type, "hld.query_path.output")
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_hld_query_path",
                    &[
                        pointer.into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                        pointer.into(),
                    ],
                    None,
                );
                let pair_stride = self.usize_type().const_int(pair_stride as u64, false);
                self.builder
                    .build_call(
                        fn_val,
                        &[
                            hld.into(),
                            u.into(),
                            v.into(),
                            pair_stride.into(),
                            output.into(),
                        ],
                        "hld.query_path",
                    )
                    .map_err(|e| e.to_string())?;
                let result = self
                    .builder
                    .build_load(array_type, output, "hld.query_path.result")
                    .map_err(|e| e.to_string())?;
                let array_ty = Type::Array(Box::new(Type::Tuple(vec![Type::Usize, Type::Usize])));
                self.record_owned_native_result(result, array_ty);
                result
            }
            tyrus_core::algorithm::HLD_POSITION_TARGET => {
                if args.len() != 2 {
                    return Err("error: `position` expects 1 argument".into());
                }
                let hld = self.emit_expr(&args[0])?;
                let v = self.emit_expr_as(&args[1], &Type::Usize)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_hld_position",
                    &[pointer.into(), self.usize_type().into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[hld.into(), v.into()], "hld.position")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: hld_position did not return a value")?
            }
            tyrus_core::algorithm::HLD_HEAD_TARGET => {
                if args.len() != 2 {
                    return Err("error: `head` expects 1 argument".into());
                }
                let hld = self.emit_expr(&args[0])?;
                let v = self.emit_expr_as(&args[1], &Type::Usize)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_hld_head",
                    &[pointer.into(), self.usize_type().into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[hld.into(), v.into()], "hld.head")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: hld_head did not return a value")?
            }
            tyrus_core::algorithm::CENTROID_NEW_TARGET => {
                if args.len() != 1 {
                    return Err("error: `centroidDecomposition` expects 1 argument".into());
                }
                let adj_ty = Type::Array(Box::new(Type::Array(Box::new(Type::Usize))));
                let adj = self.emit_expr_as(&args[0], &adj_ty)?.into_struct_value();
                let adj_data = self
                    .builder
                    .build_extract_value(adj, 0, "centroid.adj.data")
                    .map_err(|e| e.to_string())?
                    .into_pointer_value();
                let adj_length = self
                    .builder
                    .build_extract_value(adj, 1, "centroid.adj.length")
                    .map_err(|e| e.to_string())?
                    .into_int_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_centroid_new",
                    &[pointer.into(), self.usize_type().into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[adj_data.into(), adj_length.into()],
                        "centroid.new",
                    )
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: centroid_new did not return a value")?
            }
            tyrus_core::algorithm::CENTROID_PARENT_OF_TARGET => {
                if args.len() != 2 {
                    return Err("error: `parentOf` expects 1 argument".into());
                }
                let centroid = self.emit_expr(&args[0])?;
                let v = self.emit_expr_as(&args[1], &Type::Usize)?;
                let llvm_elem_ty = self.llvm_type(&Type::Usize);
                let output = self
                    .builder
                    .build_alloca(llvm_elem_ty, "centroid.parent.output")
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_centroid_parent_of",
                    &[pointer.into(), self.usize_type().into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let ok = self
                    .builder
                    .build_call(
                        fn_val,
                        &[centroid.into(), v.into(), output.into()],
                        "centroid.parent_of",
                    )
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                self.emit_binary_heap_optional(ok, output, &Type::Usize, "centroid.parent_of")?
            }
            tyrus_core::algorithm::CENTROID_DEPTH_TARGET => {
                if args.len() != 2 {
                    return Err("error: `depth` expects 1 argument".into());
                }
                let centroid = self.emit_expr(&args[0])?;
                let v = self.emit_expr_as(&args[1], &Type::Usize)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_centroid_depth",
                    &[pointer.into(), self.usize_type().into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[centroid.into(), v.into()], "centroid.depth")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: centroid_depth did not return a value")?
            }
            tyrus_core::algorithm::BLOOM_FILTER_NEW_TARGET => {
                if args.len() != 2 {
                    return Err("error: `BloomFilter` expects 2 arguments".into());
                }
                let k = self.emit_expr_as(&args[0], &Type::Usize)?;
                let m = self.emit_expr_as(&args[1], &Type::Usize)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_bloom_filter_new",
                    &[self.usize_type().into(), self.usize_type().into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(fn_val, &[k.into(), m.into()], "bloom_filter.new")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: bloom_filter_new did not return a value")?
            }
            tyrus_core::algorithm::BLOOM_FILTER_INSERT_TARGET => {
                if args.len() != 2 {
                    return Err("error: `insert` expects 1 argument".into());
                }
                let bf = self.emit_expr(&args[0])?;
                let key = self.emit_expr(&args[1])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_bloom_filter_insert",
                    &[pointer.into(), self.string_abi_type()],
                    None,
                );
                self.builder
                    .build_call(fn_val, &[bf.into(), key.into()], "bloom_filter.insert")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::BLOOM_FILTER_CONTAINS_TARGET => {
                if args.len() != 2 {
                    return Err("error: `contains` expects 1 argument".into());
                }
                let bf = self.emit_expr(&args[0])?;
                let key = self.emit_expr(&args[1])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_bloom_filter_contains",
                    &[pointer.into(), self.string_abi_type()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[bf.into(), key.into()], "bloom_filter.contains")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: bloom_filter_contains did not return a value")?
            }
            tyrus_core::algorithm::BIT_SET_SET_TARGET => {
                if args.len() < 2 || args.len() > 3 {
                    return Err("error: `set` expects 1 or 2 arguments".into());
                }
                let bitset = self.emit_expr(&args[0])?;
                let index = self.emit_expr_as(&args[1], &Type::Usize)?;
                let value = if args.len() > 2 {
                    self.emit_expr_as(&args[2], &Type::Bool)?
                } else {
                    self.context.bool_type().const_int(1, false).into()
                };
                let fn_val = self.declare_libc_fn(
                    "tyrus_bit_set_set",
                    &[
                        pointer.into(),
                        self.usize_type().into(),
                        self.context.bool_type().into(),
                    ],
                    None,
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[bitset.into(), index.into(), value.into()],
                        "bitset.set",
                    )
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::BIT_SET_FLIP_TARGET => {
                if args.len() != 2 {
                    return Err("error: `flip` expects 1 argument".into());
                }
                let bitset = self.emit_expr(&args[0])?;
                let index = self.emit_expr_as(&args[1], &Type::Usize)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_bit_set_flip",
                    &[pointer.into(), self.usize_type().into()],
                    None,
                );
                self.builder
                    .build_call(fn_val, &[bitset.into(), index.into()], "bitset.flip")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::BIT_SET_CLEAR_TARGET => {
                if args.len() != 1 {
                    return Err("error: `clear` expects 0 arguments".into());
                }
                let bitset = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn("tyrus_bit_set_clear", &[pointer.into()], None);
                self.builder
                    .build_call(fn_val, &[bitset.into()], "bitset.clear")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::BIT_SET_GET_TARGET => {
                if args.len() != 2 {
                    return Err("error: `get` expects 1 argument".into());
                }
                let bitset = self.emit_expr(&args[0])?;
                let index = self.emit_expr_as(&args[1], &Type::Usize)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_bit_set_get",
                    &[pointer.into(), self.usize_type().into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[bitset.into(), index.into()], "bitset.get")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: bit_set_get did not return a value")?
            }
            tyrus_core::algorithm::BIT_SET_COUNT_TARGET => {
                if args.len() != 1 {
                    return Err("error: `count` expects 0 arguments".into());
                }
                let bitset = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_bit_set_count",
                    &[pointer.into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[bitset.into()], "bitset.count")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: bit_set_count did not return a value")?
            }
            tyrus_core::algorithm::BIT_SET_IS_EMPTY_TARGET => {
                if args.len() != 1 {
                    return Err("error: `isEmpty` expects 0 arguments".into());
                }
                let bitset = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_bit_set_is_empty",
                    &[pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[bitset.into()], "bitset.is_empty")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: bit_set_is_empty did not return a value")?
            }
            tyrus_core::algorithm::BIT_SET_IS_SUBSET_TARGET => {
                if args.len() != 2 {
                    return Err("error: `isSubset` expects 1 argument".into());
                }
                let bitset = self.emit_expr(&args[0])?;
                let other = self.emit_expr(&args[1])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_bit_set_is_subset",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[bitset.into(), other.into()], "bitset.is_subset")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: bit_set_is_subset did not return a value")?
            }
            tyrus_core::algorithm::BIT_SET_IS_SUPERSET_TARGET => {
                if args.len() != 2 {
                    return Err("error: `isSuperset` expects 1 argument".into());
                }
                let bitset = self.emit_expr(&args[0])?;
                let other = self.emit_expr(&args[1])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_bit_set_is_superset",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[bitset.into(), other.into()], "bitset.is_superset")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: bit_set_is_superset did not return a value")?
            }
            tyrus_core::algorithm::BIT_SET_UNION_TARGET => {
                if args.len() != 2 {
                    return Err("error: `union` expects 1 argument".into());
                }
                let bitset = self.emit_expr(&args[0])?;
                let other = self.emit_expr(&args[1])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_bit_set_union",
                    &[pointer.into(), pointer.into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(fn_val, &[bitset.into(), other.into()], "bitset.union")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: bit_set_union did not return a value")?
            }
            tyrus_core::algorithm::BIT_SET_INTERSECTION_TARGET => {
                if args.len() != 2 {
                    return Err("error: `intersection` expects 1 argument".into());
                }
                let bitset = self.emit_expr(&args[0])?;
                let other = self.emit_expr(&args[1])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_bit_set_intersection",
                    &[pointer.into(), pointer.into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[bitset.into(), other.into()],
                        "bitset.intersection",
                    )
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: bit_set_intersection did not return a value")?
            }
            tyrus_core::algorithm::BIT_SET_DIFFERENCE_TARGET => {
                if args.len() != 2 {
                    return Err("error: `difference` expects 1 argument".into());
                }
                let bitset = self.emit_expr(&args[0])?;
                let other = self.emit_expr(&args[1])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_bit_set_difference",
                    &[pointer.into(), pointer.into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(fn_val, &[bitset.into(), other.into()], "bitset.difference")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: bit_set_difference did not return a value")?
            }
            tyrus_core::algorithm::BIT_SET_COMPLEMENT_TARGET => {
                if args.len() != 1 {
                    return Err("error: `complement` expects 0 arguments".into());
                }
                let bitset = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_bit_set_complement",
                    &[pointer.into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(fn_val, &[bitset.into()], "bitset.complement")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: bit_set_complement did not return a value")?
            }
            _ => return Ok(None),
        };
        Ok(Some(result))
    }
}
