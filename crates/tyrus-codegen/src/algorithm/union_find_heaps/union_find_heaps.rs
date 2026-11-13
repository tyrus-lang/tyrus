use super::*;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_union_find_heap_call(
        &mut self,
        target: &str,
        args: &[Expr],
        element_type: Option<&Type>,
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let result = match target {
            tyrus_core::algorithm::UNION_FIND_NEW_TARGET => {
                if args.len() != 1 {
                    return Err("error: `UnionFind` expects 1 argument".into());
                }
                let size = self.emit_expr_as(&args[0], &Type::Usize)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_union_find_new",
                    &[self.usize_type().into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(fn_val, &[size.into()], "uf.new")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::UNION_FIND_FIND_TARGET => {
                if args.len() != 2 {
                    return Err("error: `find` expects 1 argument".into());
                }
                let uf = self.emit_expr(&args[0])?;
                let x = self.emit_expr_as(&args[1], &Type::Usize)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_union_find_find",
                    &[pointer.into(), self.usize_type().into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[uf.into(), x.into()], "uf.find")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::UNION_FIND_UNION_TARGET => {
                if args.len() != 3 {
                    return Err("error: `union` expects 2 arguments".into());
                }
                let uf = self.emit_expr(&args[0])?;
                let x = self.emit_expr_as(&args[1], &Type::Usize)?;
                let y = self.emit_expr_as(&args[2], &Type::Usize)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_union_find_union",
                    &[
                        pointer.into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                    ],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[uf.into(), x.into(), y.into()], "uf.union")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::UNION_FIND_CONNECTED_TARGET => {
                if args.len() != 3 {
                    return Err("error: `connected` expects 2 arguments".into());
                }
                let uf = self.emit_expr(&args[0])?;
                let x = self.emit_expr_as(&args[1], &Type::Usize)?;
                let y = self.emit_expr_as(&args[2], &Type::Usize)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_union_find_connected",
                    &[
                        pointer.into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                    ],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[uf.into(), x.into(), y.into()], "uf.connected")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::UNION_FIND_SET_COUNT_TARGET => {
                if args.len() != 1 {
                    return Err("error: `setCount` expects 0 arguments".into());
                }
                let uf = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_union_find_set_count",
                    &[pointer.into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[uf.into()], "uf.set_count")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::BINARY_HEAP_NEW_TARGET => {
                let elem_ty = element_type.ok_or_else(|| {
                    "error: `BinaryHeap` requires a type argument like `BinaryHeap<i32>()`"
                        .to_string()
                })?;
                let element_size = self.type_size(elem_ty)?;
                let saved_block = self.builder.get_insert_block();
                let compare_fn = self.emit_ordered_comparator(elem_ty)?;
                if let Some(block) = saved_block {
                    self.builder.position_at_end(block);
                }
                let fn_val = self.declare_libc_fn(
                    "tyrus_binary_heap_new",
                    &[self.context.i64_type().into(), pointer.into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(fn_val, &[element_size.into(), compare_fn.into()], "bh.new")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::BINARY_HEAP_PUSH_TARGET => {
                if args.len() != 2 {
                    return Err("error: `push` expects 1 argument".into());
                }
                let heap = self.emit_expr(&args[0])?;
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `push`".to_string())?;
                let item = self.emit_expr_as(&args[1], elem_ty)?;
                let slot = self.emit_value_slot(item, elem_ty);
                let fn_val = self.declare_libc_fn(
                    "tyrus_binary_heap_push",
                    &[pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(fn_val, &[heap.into(), slot.into()], "bh.push")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::BINARY_HEAP_POP_TARGET => {
                if args.len() != 1 {
                    return Err("error: `pop` expects 0 arguments".into());
                }
                let heap = self.emit_expr(&args[0])?;
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `pop`".to_string())?;
                let llvm_elem_ty = self.llvm_type(elem_ty);
                let output = self
                    .builder
                    .build_alloca(llvm_elem_ty, "bh.pop.output")
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_binary_heap_pop",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let ok = self
                    .builder
                    .build_call(fn_val, &[heap.into(), output.into()], "bh.pop")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                self.emit_binary_heap_optional(ok, output, elem_ty, "bh.pop")?
            }
            tyrus_core::algorithm::BINARY_HEAP_PEEK_TARGET => {
                if args.len() != 1 {
                    return Err("error: `peek` expects 0 arguments".into());
                }
                let heap = self.emit_expr(&args[0])?;
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `peek`".to_string())?;
                let llvm_elem_ty = self.llvm_type(elem_ty);
                let output = self
                    .builder
                    .build_alloca(llvm_elem_ty, "bh.peek.output")
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_binary_heap_peek",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let ok = self
                    .builder
                    .build_call(fn_val, &[heap.into(), output.into()], "bh.peek")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                self.emit_binary_heap_optional(ok, output, elem_ty, "bh.peek")?
            }
            tyrus_core::algorithm::BINARY_HEAP_LEN_TARGET => {
                if args.len() != 1 {
                    return Err("error: `len` expects 0 arguments".into());
                }
                let heap = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_binary_heap_len",
                    &[pointer.into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[heap.into()], "bh.len")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::BINARY_HEAP_IS_EMPTY_TARGET => {
                if args.len() != 1 {
                    return Err("error: `isEmpty` expects 0 arguments".into());
                }
                let heap = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_binary_heap_is_empty",
                    &[pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[heap.into()], "bh.is_empty")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::BINARY_HEAP_CLEAR_TARGET => {
                if args.len() != 1 {
                    return Err("error: `clear` expects 0 arguments".into());
                }
                let heap = self.emit_expr(&args[0])?;
                let fn_val =
                    self.declare_libc_fn("tyrus_binary_heap_clear", &[pointer.into()], None);
                self.builder
                    .build_call(fn_val, &[heap.into()], "bh.clear")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::BINOMIAL_HEAP_NEW_TARGET => {
                let elem_ty = element_type.ok_or_else(|| {
                    "error: `BinomialHeap` requires a type argument like `BinomialHeap<i32>()`"
                        .to_string()
                })?;
                let element_size = self.type_size(elem_ty)?;
                let saved_block = self.builder.get_insert_block();
                let compare_fn = self.emit_ordered_comparator(elem_ty)?;
                if let Some(block) = saved_block {
                    self.builder.position_at_end(block);
                }
                let fn_val = self.declare_libc_fn(
                    "tyrus_binomial_heap_new",
                    &[self.context.i64_type().into(), pointer.into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[element_size.into(), compare_fn.into()],
                        "binh.new",
                    )
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::BINOMIAL_HEAP_PUSH_TARGET => {
                if args.len() != 2 {
                    return Err("error: `push` expects 1 argument".into());
                }
                let heap = self.emit_expr(&args[0])?;
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `push`".to_string())?;
                let item = self.emit_expr_as(&args[1], elem_ty)?;
                let slot = self.emit_value_slot(item, elem_ty);
                let fn_val = self.declare_libc_fn(
                    "tyrus_binomial_heap_push",
                    &[pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(fn_val, &[heap.into(), slot.into()], "binh.push")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::BINOMIAL_HEAP_POP_TARGET => {
                if args.len() != 1 {
                    return Err("error: `pop` expects 0 arguments".into());
                }
                let heap = self.emit_expr(&args[0])?;
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `pop`".to_string())?;
                let llvm_elem_ty = self.llvm_type(elem_ty);
                let output = self
                    .builder
                    .build_alloca(llvm_elem_ty, "binh.pop.output")
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_binomial_heap_pop",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let ok = self
                    .builder
                    .build_call(fn_val, &[heap.into(), output.into()], "binh.pop")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                self.emit_binary_heap_optional(ok, output, elem_ty, "binh.pop")?
            }
            tyrus_core::algorithm::BINOMIAL_HEAP_PEEK_TARGET => {
                if args.len() != 1 {
                    return Err("error: `peek` expects 0 arguments".into());
                }
                let heap = self.emit_expr(&args[0])?;
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `peek`".to_string())?;
                let llvm_elem_ty = self.llvm_type(elem_ty);
                let output = self
                    .builder
                    .build_alloca(llvm_elem_ty, "binh.peek.output")
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_binomial_heap_peek",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let ok = self
                    .builder
                    .build_call(fn_val, &[heap.into(), output.into()], "binh.peek")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                self.emit_binary_heap_optional(ok, output, elem_ty, "binh.peek")?
            }
            tyrus_core::algorithm::BINOMIAL_HEAP_LEN_TARGET => {
                if args.len() != 1 {
                    return Err("error: `len` expects 0 arguments".into());
                }
                let heap = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_binomial_heap_len",
                    &[pointer.into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[heap.into()], "binh.len")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::BINOMIAL_HEAP_IS_EMPTY_TARGET => {
                if args.len() != 1 {
                    return Err("error: `isEmpty` expects 0 arguments".into());
                }
                let heap = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_binomial_heap_is_empty",
                    &[pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[heap.into()], "binh.is_empty")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::BINOMIAL_HEAP_CLEAR_TARGET => {
                if args.len() != 1 {
                    return Err("error: `clear` expects 0 arguments".into());
                }
                let heap = self.emit_expr(&args[0])?;
                let fn_val =
                    self.declare_libc_fn("tyrus_binomial_heap_clear", &[pointer.into()], None);
                self.builder
                    .build_call(fn_val, &[heap.into()], "binh.clear")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::COUNTER_NEW_TARGET => {
                let elem_ty = element_type.ok_or_else(|| {
                    "error: `Counter` requires a type argument like `Counter<string>()`".to_string()
                })?;
                let element_size = self.type_size(elem_ty)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_counter_new",
                    &[self.usize_type().into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(fn_val, &[element_size.into()], "counter.new")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::COUNTER_ADD_TARGET => {
                if args.len() < 2 || args.len() > 3 {
                    return Err("error: `add` expects 1 or 2 arguments".into());
                }
                let counter = self.emit_expr(&args[0])?;
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `add`".to_string())?;
                let item = self.emit_expr_as(&args[1], elem_ty)?;
                let slot = self.emit_value_slot(item, elem_ty);
                let count = if args.len() > 2 {
                    self.emit_expr_as(&args[2], &Type::I32)?
                } else {
                    self.context.i32_type().const_int(1, true).into()
                };
                let fn_val = self.declare_libc_fn(
                    "tyrus_counter_add",
                    &[
                        pointer.into(),
                        pointer.into(),
                        self.context.i32_type().into(),
                    ],
                    None,
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[counter.into(), slot.into(), count.into()],
                        "counter.add",
                    )
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::COUNTER_REMOVE_TARGET => {
                if args.len() < 2 || args.len() > 3 {
                    return Err("error: `remove` expects 1 or 2 arguments".into());
                }
                let counter = self.emit_expr(&args[0])?;
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `remove`".to_string())?;
                let item = self.emit_expr_as(&args[1], elem_ty)?;
                let slot = self.emit_value_slot(item, elem_ty);
                let count = if args.len() > 2 {
                    self.emit_expr_as(&args[2], &Type::I32)?
                } else {
                    self.context.i32_type().const_int(1, true).into()
                };
                let fn_val = self.declare_libc_fn(
                    "tyrus_counter_remove",
                    &[
                        pointer.into(),
                        pointer.into(),
                        self.context.i32_type().into(),
                    ],
                    None,
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[counter.into(), slot.into(), count.into()],
                        "counter.remove",
                    )
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::COUNTER_COUNT_TARGET => {
                if args.len() != 2 {
                    return Err("error: `count` expects 1 argument".into());
                }
                let counter = self.emit_expr(&args[0])?;
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `count`".to_string())?;
                let item = self.emit_expr_as(&args[1], elem_ty)?;
                let slot = self.emit_value_slot(item, elem_ty);
                let fn_val = self.declare_libc_fn(
                    "tyrus_counter_count",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.i32_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[counter.into(), slot.into()], "counter.count")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::COUNTER_MOST_COMMON_TARGET => {
                if args.len() != 2 {
                    return Err("error: `mostCommon` expects 1 argument".into());
                }
                let counter = self.emit_expr(&args[0])?;
                let k = self.emit_expr_as(&args[1], &Type::Usize)?;
                let elem_ty = element_type.ok_or_else(|| {
                    "error: cannot infer element type for `mostCommon`".to_string()
                })?;
                let (element_size, element_align) = self.type_layout(elem_ty);
                let count_align = std::mem::align_of::<i32>();
                let count_offset = (element_size + count_align - 1) & !(count_align - 1);
                let pair_align = element_align.max(count_align);
                let pair_stride = (count_offset + std::mem::size_of::<i32>() + pair_align - 1)
                    & !(pair_align - 1);
                let result_ty =
                    Type::Array(Box::new(Type::Tuple(vec![elem_ty.clone(), Type::I32])));
                let array_type = self.llvm_type(&result_ty);
                let output = self
                    .builder
                    .build_alloca(array_type, "counter.most_common.output")
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_counter_most_common",
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
                let count_offset = self.usize_type().const_int(count_offset as u64, false);
                self.builder
                    .build_call(
                        fn_val,
                        &[
                            counter.into(),
                            k.into(),
                            pair_stride.into(),
                            count_offset.into(),
                            output.into(),
                        ],
                        "counter.most_common",
                    )
                    .map_err(|e| e.to_string())?;
                let result = self
                    .builder
                    .build_load(array_type, output, "counter.most_common.result")
                    .map_err(|e| e.to_string())?;
                self.record_owned_native_result(result, result_ty);
                result
            }
            tyrus_core::algorithm::COUNTER_TOTAL_TARGET => {
                if args.len() != 1 {
                    return Err("error: `total` expects 0 arguments".into());
                }
                let counter = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_counter_total",
                    &[pointer.into()],
                    Some(self.context.i32_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[counter.into()], "counter.total")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::COUNTER_UNIQUE_TARGET => {
                if args.len() != 1 {
                    return Err("error: `unique` expects 0 arguments".into());
                }
                let counter = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_counter_unique",
                    &[pointer.into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[counter.into()], "counter.unique")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::COUNTER_IS_EMPTY_TARGET => {
                if args.len() != 1 {
                    return Err("error: `isEmpty` expects 0 arguments".into());
                }
                let counter = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_counter_is_empty",
                    &[pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[counter.into()], "counter.is_empty")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::COUNTER_CLEAR_TARGET => {
                if args.len() != 1 {
                    return Err("error: `clear` expects 0 arguments".into());
                }
                let counter = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn("tyrus_counter_clear", &[pointer.into()], None);
                self.builder
                    .build_call(fn_val, &[counter.into()], "counter.clear")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            _ => return Ok(None),
        };
        Ok(Some(result))
    }
}
