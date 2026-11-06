use super::*;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_sorted_container_call(
        &mut self,
        target: &str,
        args: &[Expr],
        element_type: Option<&Type>,
        value_type: Option<&Type>,
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let result = match target {
            // Ordered tree construction: AvlTree, RedBlackTree, SplayTree, Treap.
            // The element type is erased at the C ABI boundary; the runtime gets
            // the element size and an ordering comparator function.
            tyrus_core::algorithm::AVL_TREE_NEW_TARGET
            | tyrus_core::algorithm::RED_BLACK_TREE_NEW_TARGET
            | tyrus_core::algorithm::SPLAY_TREE_NEW_TARGET
            | tyrus_core::algorithm::TREAP_NEW_TARGET => {
                let elem_ty = element_type.ok_or_else(|| {
                    "error: ordered tree requires a type argument like `AvlTree<i32>()`".to_string()
                })?;
                let element_size = self.type_size(elem_ty)?;
                let saved_block = self.builder.get_insert_block();
                let compare_fn = self.emit_ordered_comparator(elem_ty)?;
                if let Some(block) = saved_block {
                    self.builder.position_at_end(block);
                }
                let (runtime_name, label) = match target {
                    tyrus_core::algorithm::AVL_TREE_NEW_TARGET => ("tyrus_avl_tree_new", "avl.new"),
                    tyrus_core::algorithm::RED_BLACK_TREE_NEW_TARGET => {
                        ("tyrus_red_black_tree_new", "rbtree.new")
                    }
                    tyrus_core::algorithm::SPLAY_TREE_NEW_TARGET => {
                        ("tyrus_splay_tree_new", "splay.new")
                    }
                    _ => ("tyrus_treap_new", "treap.new"),
                };
                let fn_val = self.declare_libc_fn(
                    runtime_name,
                    &[self.context.i64_type().into(), pointer.into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(fn_val, &[element_size.into(), compare_fn.into()], label)
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::SORTED_SET_NEW_TARGET => {
                let elem_ty = element_type.ok_or_else(|| {
                    "error: `SortedSet` requires a type argument like `SortedSet<i32>()`"
                        .to_string()
                })?;
                let element_size = self.type_size(elem_ty)?;
                let saved_block = self.builder.get_insert_block();
                let compare_fn = self.emit_ordered_comparator(elem_ty)?;
                if let Some(block) = saved_block {
                    self.builder.position_at_end(block);
                }
                let fn_val = self.declare_libc_fn(
                    "tyrus_sorted_set_new",
                    &[self.context.i64_type().into(), pointer.into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[element_size.into(), compare_fn.into()],
                        "sset.new",
                    )
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::SKIP_LIST_NEW_TARGET => {
                let elem_ty = element_type.ok_or_else(|| {
                    "error: `SkipList` requires a type argument like `SkipList<i32>()`".to_string()
                })?;
                let element_size = self.type_size(elem_ty)?;
                let saved_block = self.builder.get_insert_block();
                let compare_fn = self.emit_ordered_comparator(elem_ty)?;
                if let Some(block) = saved_block {
                    self.builder.position_at_end(block);
                }
                let fn_val = self.declare_libc_fn(
                    "tyrus_skip_list_new",
                    &[self.context.i64_type().into(), pointer.into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[element_size.into(), compare_fn.into()],
                        "skip_list.new",
                    )
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::SORTED_MAP_NEW_TARGET => {
                let key_ty = element_type.ok_or_else(|| {
                    "error: `SortedMap` requires type arguments like `SortedMap<i32, string>()`"
                        .to_string()
                })?;
                let key_size = self.type_size(key_ty)?;
                let value_ty = value_type
                    .ok_or_else(|| "error: cannot infer value type for `SortedMap`".to_string())?;
                let value_size = self.type_size(value_ty)?;
                let saved_block = self.builder.get_insert_block();
                let compare_fn = self.emit_ordered_comparator(key_ty)?;
                if let Some(block) = saved_block {
                    self.builder.position_at_end(block);
                }
                let fn_val = self.declare_libc_fn(
                    "tyrus_sorted_map_new",
                    &[
                        self.context.i64_type().into(),
                        self.context.i64_type().into(),
                        pointer.into(),
                    ],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[key_size.into(), value_size.into(), compare_fn.into()],
                        "smap.new",
                    )
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::AVL_TREE_INSERT_TARGET
            | tyrus_core::algorithm::RED_BLACK_TREE_INSERT_TARGET
            | tyrus_core::algorithm::SPLAY_TREE_INSERT_TARGET
            | tyrus_core::algorithm::TREAP_INSERT_TARGET => {
                if args.len() != 2 {
                    return Err("error: `insert` expects 1 argument".into());
                }
                let tree = self.emit_expr(&args[0])?;
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `insert`".to_string())?;
                let item = self.emit_expr_as(&args[1], elem_ty)?;
                let slot = self.emit_value_slot(item, elem_ty);
                let (runtime_name, label) = match target {
                    tyrus_core::algorithm::AVL_TREE_INSERT_TARGET => {
                        ("tyrus_avl_tree_insert", "avl.insert")
                    }
                    tyrus_core::algorithm::RED_BLACK_TREE_INSERT_TARGET => {
                        ("tyrus_red_black_tree_insert", "rbtree.insert")
                    }
                    tyrus_core::algorithm::SPLAY_TREE_INSERT_TARGET => {
                        ("tyrus_splay_tree_insert", "splay.insert")
                    }
                    _ => ("tyrus_treap_insert", "treap.insert"),
                };
                let fn_val =
                    self.declare_libc_fn(runtime_name, &[pointer.into(), pointer.into()], None);
                self.builder
                    .build_call(fn_val, &[tree.into(), slot.into()], label)
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::AVL_TREE_CONTAINS_TARGET
            | tyrus_core::algorithm::RED_BLACK_TREE_CONTAINS_TARGET
            | tyrus_core::algorithm::SPLAY_TREE_CONTAINS_TARGET
            | tyrus_core::algorithm::TREAP_CONTAINS_TARGET => {
                if args.len() != 2 {
                    return Err("error: `contains` expects 1 argument".into());
                }
                let tree = self.emit_expr(&args[0])?;
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `contains`".to_string())?;
                let item = self.emit_expr_as(&args[1], elem_ty)?;
                let slot = self.emit_value_slot(item, elem_ty);
                let (runtime_name, label) = match target {
                    tyrus_core::algorithm::AVL_TREE_CONTAINS_TARGET => {
                        ("tyrus_avl_tree_contains", "avl.contains")
                    }
                    tyrus_core::algorithm::RED_BLACK_TREE_CONTAINS_TARGET => {
                        ("tyrus_red_black_tree_contains", "rbtree.contains")
                    }
                    tyrus_core::algorithm::SPLAY_TREE_CONTAINS_TARGET => {
                        ("tyrus_splay_tree_contains", "splay.contains")
                    }
                    _ => ("tyrus_treap_contains", "treap.contains"),
                };
                let fn_val = self.declare_libc_fn(
                    runtime_name,
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[tree.into(), slot.into()], label)
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::AVL_TREE_LEN_TARGET
            | tyrus_core::algorithm::RED_BLACK_TREE_LEN_TARGET
            | tyrus_core::algorithm::SPLAY_TREE_LEN_TARGET
            | tyrus_core::algorithm::TREAP_LEN_TARGET => {
                if args.len() != 1 {
                    return Err("error: `len` expects 0 arguments".into());
                }
                let tree = self.emit_expr(&args[0])?;
                let (runtime_name, label) = match target {
                    tyrus_core::algorithm::AVL_TREE_LEN_TARGET => ("tyrus_avl_tree_len", "avl.len"),
                    tyrus_core::algorithm::RED_BLACK_TREE_LEN_TARGET => {
                        ("tyrus_red_black_tree_len", "rbtree.len")
                    }
                    tyrus_core::algorithm::SPLAY_TREE_LEN_TARGET => {
                        ("tyrus_splay_tree_len", "splay.len")
                    }
                    _ => ("tyrus_treap_len", "treap.len"),
                };
                let fn_val = self.declare_libc_fn(
                    runtime_name,
                    &[pointer.into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[tree.into()], label)
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::AVL_TREE_IS_EMPTY_TARGET
            | tyrus_core::algorithm::RED_BLACK_TREE_IS_EMPTY_TARGET
            | tyrus_core::algorithm::SPLAY_TREE_IS_EMPTY_TARGET
            | tyrus_core::algorithm::TREAP_IS_EMPTY_TARGET => {
                if args.len() != 1 {
                    return Err("error: `isEmpty` expects 0 arguments".into());
                }
                let tree = self.emit_expr(&args[0])?;
                let (runtime_name, label) = match target {
                    tyrus_core::algorithm::AVL_TREE_IS_EMPTY_TARGET => {
                        ("tyrus_avl_tree_is_empty", "avl.is_empty")
                    }
                    tyrus_core::algorithm::RED_BLACK_TREE_IS_EMPTY_TARGET => {
                        ("tyrus_red_black_tree_is_empty", "rbtree.is_empty")
                    }
                    tyrus_core::algorithm::SPLAY_TREE_IS_EMPTY_TARGET => {
                        ("tyrus_splay_tree_is_empty", "splay.is_empty")
                    }
                    _ => ("tyrus_treap_is_empty", "treap.is_empty"),
                };
                let fn_val = self.declare_libc_fn(
                    runtime_name,
                    &[pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[tree.into()], label)
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::SORTED_SET_INSERT_TARGET
            | tyrus_core::algorithm::SORTED_SET_REMOVE_TARGET
            | tyrus_core::algorithm::SORTED_SET_CONTAINS_TARGET => {
                if args.len() != 2 {
                    return Err("error: `insert`/`remove`/`contains` expect 1 argument".into());
                }
                let set = self.emit_expr(&args[0])?;
                let elem_ty = element_type.ok_or_else(|| {
                    "error: cannot infer element type for `SortedSet` method".to_string()
                })?;
                let item = self.emit_expr_as(&args[1], elem_ty)?;
                let slot = self.emit_value_slot(item, elem_ty);
                let (runtime_name, label) = match target {
                    tyrus_core::algorithm::SORTED_SET_INSERT_TARGET => {
                        ("tyrus_sorted_set_insert", "sset.insert")
                    }
                    tyrus_core::algorithm::SORTED_SET_REMOVE_TARGET => {
                        ("tyrus_sorted_set_remove", "sset.remove")
                    }
                    _ => ("tyrus_sorted_set_contains", "sset.contains"),
                };
                let fn_val = self.declare_libc_fn(
                    runtime_name,
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[set.into(), slot.into()], label)
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::SORTED_SET_MIN_TARGET
            | tyrus_core::algorithm::SORTED_SET_MAX_TARGET => {
                if args.len() != 1 {
                    return Err("error: `min`/`max` expect 0 arguments".into());
                }
                let set = self.emit_expr(&args[0])?;
                let (runtime_name, label) = match target {
                    tyrus_core::algorithm::SORTED_SET_MIN_TARGET => {
                        ("tyrus_sorted_set_min", "sset.min")
                    }
                    _ => ("tyrus_sorted_set_max", "sset.max"),
                };
                let fn_val =
                    self.declare_libc_fn(runtime_name, &[pointer.into()], Some(pointer.into()));
                self.builder
                    .build_call(fn_val, &[set.into()], label)
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::SORTED_SET_LEN_TARGET => {
                if args.len() != 1 {
                    return Err("error: `len` expects 0 arguments".into());
                }
                let set = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_sorted_set_len",
                    &[pointer.into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[set.into()], "sset.len")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::SORTED_SET_IS_EMPTY_TARGET => {
                if args.len() != 1 {
                    return Err("error: `isEmpty` expects 0 arguments".into());
                }
                let set = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_sorted_set_is_empty",
                    &[pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[set.into()], "sset.is_empty")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::SORTED_SET_CLEAR_TARGET => {
                if args.len() != 1 {
                    return Err("error: `clear` expects 0 arguments".into());
                }
                let set = self.emit_expr(&args[0])?;
                let fn_val =
                    self.declare_libc_fn("tyrus_sorted_set_clear", &[pointer.into()], None);
                self.builder
                    .build_call(fn_val, &[set.into()], "sset.clear")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::SKIP_LIST_INSERT_TARGET
            | tyrus_core::algorithm::SKIP_LIST_REMOVE_TARGET
            | tyrus_core::algorithm::SKIP_LIST_CONTAINS_TARGET => {
                if args.len() != 2 {
                    return Err("error: `insert`/`remove`/`contains` expect 1 argument".into());
                }
                let list = self.emit_expr(&args[0])?;
                let elem_ty = element_type.ok_or_else(|| {
                    "error: cannot infer element type for `SkipList` method".to_string()
                })?;
                let item = self.emit_expr_as(&args[1], elem_ty)?;
                let slot = self.emit_value_slot(item, elem_ty);
                let (runtime_name, label) = match target {
                    tyrus_core::algorithm::SKIP_LIST_INSERT_TARGET => {
                        ("tyrus_skip_list_insert", "skip_list.insert")
                    }
                    tyrus_core::algorithm::SKIP_LIST_REMOVE_TARGET => {
                        ("tyrus_skip_list_remove", "skip_list.remove")
                    }
                    _ => ("tyrus_skip_list_contains", "skip_list.contains"),
                };
                let fn_val = self.declare_libc_fn(
                    runtime_name,
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[list.into(), slot.into()], label)
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::SKIP_LIST_MIN_TARGET
            | tyrus_core::algorithm::SKIP_LIST_MAX_TARGET => {
                if args.len() != 1 {
                    return Err("error: `min`/`max` expect 0 arguments".into());
                }
                let list = self.emit_expr(&args[0])?;
                let (runtime_name, label) = match target {
                    tyrus_core::algorithm::SKIP_LIST_MIN_TARGET => {
                        ("tyrus_skip_list_min", "skip_list.min")
                    }
                    _ => ("tyrus_skip_list_max", "skip_list.max"),
                };
                let fn_val =
                    self.declare_libc_fn(runtime_name, &[pointer.into()], Some(pointer.into()));
                self.builder
                    .build_call(fn_val, &[list.into()], label)
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::SKIP_LIST_LEN_TARGET => {
                if args.len() != 1 {
                    return Err("error: `len` expects 0 arguments".into());
                }
                let list = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_skip_list_len",
                    &[pointer.into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[list.into()], "skip_list.len")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::SKIP_LIST_IS_EMPTY_TARGET => {
                if args.len() != 1 {
                    return Err("error: `isEmpty` expects 0 arguments".into());
                }
                let list = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_skip_list_is_empty",
                    &[pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[list.into()], "skip_list.is_empty")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::SKIP_LIST_CLEAR_TARGET => {
                if args.len() != 1 {
                    return Err("error: `clear` expects 0 arguments".into());
                }
                let list = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn("tyrus_skip_list_clear", &[pointer.into()], None);
                self.builder
                    .build_call(fn_val, &[list.into()], "skip_list.clear")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::SORTED_MAP_SET_TARGET => {
                if args.len() != 3 {
                    return Err("error: `set` expects 2 arguments".into());
                }
                let map = self.emit_expr(&args[0])?;
                let key_ty = element_type.ok_or_else(|| {
                    "error: cannot infer key type for `SortedMap.set`".to_string()
                })?;
                let key = self.emit_expr_as(&args[1], key_ty)?;
                let key_slot = self.emit_value_slot(key, key_ty);
                let value_ty = value_type.ok_or_else(|| {
                    "error: cannot infer value type for `SortedMap.set`".to_string()
                })?;
                let value = self.emit_expr_as(&args[2], value_ty)?;
                let value_slot = self.emit_value_slot(value, value_ty);
                let fn_val = self.declare_libc_fn(
                    "tyrus_sorted_map_set",
                    &[pointer.into(), pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[map.into(), key_slot.into(), value_slot.into()],
                        "smap.set",
                    )
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::SORTED_MAP_GET_TARGET => {
                if args.len() != 2 {
                    return Err("error: `get` expects 1 argument".into());
                }
                let map = self.emit_expr(&args[0])?;
                let key_ty = element_type.ok_or_else(|| {
                    "error: cannot infer key type for `SortedMap.get`".to_string()
                })?;
                let key = self.emit_expr_as(&args[1], key_ty)?;
                let key_slot = self.emit_value_slot(key, key_ty);
                let fn_val = self.declare_libc_fn(
                    "tyrus_sorted_map_get",
                    &[pointer.into(), pointer.into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(fn_val, &[map.into(), key_slot.into()], "smap.get")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::SORTED_MAP_HAS_TARGET
            | tyrus_core::algorithm::SORTED_MAP_DELETE_TARGET => {
                if args.len() != 2 {
                    return Err("error: `has`/`delete` expect 1 argument".into());
                }
                let map = self.emit_expr(&args[0])?;
                let key_ty = element_type.ok_or_else(|| {
                    "error: cannot infer key type for `SortedMap.has`/`delete`".to_string()
                })?;
                let key = self.emit_expr_as(&args[1], key_ty)?;
                let key_slot = self.emit_value_slot(key, key_ty);
                let (runtime_name, label) = match target {
                    tyrus_core::algorithm::SORTED_MAP_HAS_TARGET => {
                        ("tyrus_sorted_map_has", "smap.has")
                    }
                    _ => ("tyrus_sorted_map_delete", "smap.delete"),
                };
                let fn_val = self.declare_libc_fn(
                    runtime_name,
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[map.into(), key_slot.into()], label)
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::SORTED_MAP_MIN_TARGET
            | tyrus_core::algorithm::SORTED_MAP_MAX_TARGET => {
                if args.len() != 1 {
                    return Err("error: `min`/`max` expect 0 arguments".into());
                }
                let map = self.emit_expr(&args[0])?;
                let (runtime_name, label) = match target {
                    tyrus_core::algorithm::SORTED_MAP_MIN_TARGET => {
                        ("tyrus_sorted_map_min", "smap.min")
                    }
                    _ => ("tyrus_sorted_map_max", "smap.max"),
                };
                let fn_val =
                    self.declare_libc_fn(runtime_name, &[pointer.into()], Some(pointer.into()));
                self.builder
                    .build_call(fn_val, &[map.into()], label)
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::SORTED_MAP_LEN_TARGET => {
                if args.len() != 1 {
                    return Err("error: `len` expects 0 arguments".into());
                }
                let map = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_sorted_map_len",
                    &[pointer.into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[map.into()], "smap.len")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::SORTED_MAP_IS_EMPTY_TARGET => {
                if args.len() != 1 {
                    return Err("error: `isEmpty` expects 0 arguments".into());
                }
                let map = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_sorted_map_is_empty",
                    &[pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[map.into()], "smap.is_empty")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::SORTED_MAP_CLEAR_TARGET => {
                if args.len() != 1 {
                    return Err("error: `clear` expects 0 arguments".into());
                }
                let map = self.emit_expr(&args[0])?;
                let fn_val =
                    self.declare_libc_fn("tyrus_sorted_map_clear", &[pointer.into()], None);
                self.builder
                    .build_call(fn_val, &[map.into()], "smap.clear")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            _ => return Ok(None),
        };
        Ok(Some(result))
    }
}
