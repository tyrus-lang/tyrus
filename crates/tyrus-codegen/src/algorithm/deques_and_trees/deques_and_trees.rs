use super::*;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_deque_tree_call(
        &mut self,
        target: &str,
        args: &[Expr],
        element_type: Option<&Type>,
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let result = match target {
            tyrus_core::algorithm::DEQUE_NEW_TARGET => {
                let elem_ty = element_type.ok_or_else(|| {
                    "error: `Deque` requires a type argument like `Deque<string>()`".to_string()
                })?;
                let element_size = self.type_size(elem_ty)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_deque_new",
                    &[self.usize_type().into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(fn_val, &[element_size.into()], "deque.new")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: deque_new did not return a value")?
            }
            tyrus_core::algorithm::DEQUE_PUSH_FRONT_TARGET => {
                if args.len() != 2 {
                    return Err("error: `pushFront` expects 1 argument".into());
                }
                let deque = self.emit_expr(&args[0])?;
                let elem_ty = element_type.ok_or_else(|| {
                    "error: cannot infer element type for `pushFront`".to_string()
                })?;
                let item = self.emit_expr_as(&args[1], elem_ty)?;
                let slot = self.emit_value_slot(item, elem_ty);
                let fn_val = self.declare_libc_fn(
                    "tyrus_deque_push_front",
                    &[pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(fn_val, &[deque.into(), slot.into()], "deque.push_front")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::DEQUE_PUSH_BACK_TARGET => {
                if args.len() != 2 {
                    return Err("error: `pushBack` expects 1 argument".into());
                }
                let deque = self.emit_expr(&args[0])?;
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `pushBack`".to_string())?;
                let item = self.emit_expr_as(&args[1], elem_ty)?;
                let slot = self.emit_value_slot(item, elem_ty);
                let fn_val = self.declare_libc_fn(
                    "tyrus_deque_push_back",
                    &[pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(fn_val, &[deque.into(), slot.into()], "deque.push_back")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::DEQUE_POP_FRONT_TARGET => {
                if args.len() != 1 {
                    return Err("error: `popFront` expects 0 arguments".into());
                }
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `popFront`".to_string())?;
                let deque = self.emit_expr(&args[0])?;
                let llvm_elem_ty = self.llvm_type(elem_ty);
                let output = self
                    .builder
                    .build_alloca(llvm_elem_ty, "deque.pop_front.output")
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_deque_pop_front",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let ok = self
                    .builder
                    .build_call(fn_val, &[deque.into(), output.into()], "deque.pop_front")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: deque_pop_front did not return a value")?
                    .into_int_value();
                self.emit_binary_heap_optional(ok, output, elem_ty, "deque.pop_front")?
            }
            tyrus_core::algorithm::DEQUE_POP_BACK_TARGET => {
                if args.len() != 1 {
                    return Err("error: `popBack` expects 0 arguments".into());
                }
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `popBack`".to_string())?;
                let deque = self.emit_expr(&args[0])?;
                let llvm_elem_ty = self.llvm_type(elem_ty);
                let output = self
                    .builder
                    .build_alloca(llvm_elem_ty, "deque.pop_back.output")
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_deque_pop_back",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let ok = self
                    .builder
                    .build_call(fn_val, &[deque.into(), output.into()], "deque.pop_back")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: deque_pop_back did not return a value")?
                    .into_int_value();
                self.emit_binary_heap_optional(ok, output, elem_ty, "deque.pop_back")?
            }
            tyrus_core::algorithm::DEQUE_PEEK_FRONT_TARGET => {
                if args.len() != 1 {
                    return Err("error: `peekFront` expects 0 arguments".into());
                }
                let elem_ty = element_type.ok_or_else(|| {
                    "error: cannot infer element type for `peekFront`".to_string()
                })?;
                let deque = self.emit_expr(&args[0])?;
                let llvm_elem_ty = self.llvm_type(elem_ty);
                let output = self
                    .builder
                    .build_alloca(llvm_elem_ty, "deque.peek_front.output")
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_deque_peek_front",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let ok = self
                    .builder
                    .build_call(fn_val, &[deque.into(), output.into()], "deque.peek_front")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: deque_peek_front did not return a value")?
                    .into_int_value();
                self.emit_binary_heap_optional(ok, output, elem_ty, "deque.peek_front")?
            }
            tyrus_core::algorithm::DEQUE_PEEK_BACK_TARGET => {
                if args.len() != 1 {
                    return Err("error: `peekBack` expects 0 arguments".into());
                }
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `peekBack`".to_string())?;
                let deque = self.emit_expr(&args[0])?;
                let llvm_elem_ty = self.llvm_type(elem_ty);
                let output = self
                    .builder
                    .build_alloca(llvm_elem_ty, "deque.peek_back.output")
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_deque_peek_back",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let ok = self
                    .builder
                    .build_call(fn_val, &[deque.into(), output.into()], "deque.peek_back")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: deque_peek_back did not return a value")?
                    .into_int_value();
                self.emit_binary_heap_optional(ok, output, elem_ty, "deque.peek_back")?
            }
            tyrus_core::algorithm::DEQUE_LEN_TARGET => {
                if args.len() != 1 {
                    return Err("error: `len` expects 0 arguments".into());
                }
                let deque = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_deque_len",
                    &[pointer.into()],
                    Some(self.context.i64_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[deque.into()], "deque.len")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: deque_len did not return a value")?
            }
            tyrus_core::algorithm::DEQUE_IS_EMPTY_TARGET => {
                if args.len() != 1 {
                    return Err("error: `isEmpty` expects 0 arguments".into());
                }
                let deque = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_deque_is_empty",
                    &[pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[deque.into()], "deque.is_empty")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: deque_is_empty did not return a value")?
            }
            tyrus_core::algorithm::DEQUE_CLEAR_TARGET => {
                if args.len() != 1 {
                    return Err("error: `clear` expects 0 arguments".into());
                }
                let deque = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn("tyrus_deque_clear", &[pointer.into()], None);
                self.builder
                    .build_call(fn_val, &[deque.into()], "deque.clear")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::FENWICK_TREE_NEW_TARGET => {
                if args.len() != 1 {
                    return Err("error: `FenwickTree` expects 1 argument".into());
                }
                let size = self.emit_expr_as(&args[0], &Type::Usize)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_fenwick_tree_new",
                    &[self.usize_type().into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(fn_val, &[size.into()], "ft.new")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: fenwick_tree_new did not return a value")?
            }
            tyrus_core::algorithm::BUBBLE_SORT_NEW_TARGET => {
                if args.len() != 1 {
                    return Err("error: `BubbleSort` expects 1 argument".into());
                }
                let array_ty = Type::Array(Box::new(Type::I32));
                let values = self.emit_expr_as(&args[0], &array_ty)?;
                let input = self
                    .builder
                    .build_alloca(self.llvm_type(&array_ty), "bubble_sort.input")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(input, values)
                    .map_err(|error| error.to_string())?;
                let function = self.declare_libc_fn(
                    "tyrus_bubble_sort_new",
                    &[pointer.into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(function, &[input.into()], "bubble_sort.new")
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: BubbleSort constructor returned no value")?
            }
            tyrus_core::algorithm::BUBBLE_SORT_STEP_TARGET
            | tyrus_core::algorithm::BUBBLE_SORT_DONE_TARGET
            | tyrus_core::algorithm::BUBBLE_SORT_SWAPPED_TARGET => {
                if args.len() != 1 {
                    return Err("error: BubbleSort method expects 0 arguments".into());
                }
                let handle = self.emit_expr(&args[0])?;
                let runtime = match target {
                    tyrus_core::algorithm::BUBBLE_SORT_STEP_TARGET => "tyrus_bubble_sort_step",
                    tyrus_core::algorithm::BUBBLE_SORT_DONE_TARGET => "tyrus_bubble_sort_done",
                    _ => "tyrus_bubble_sort_swapped",
                };
                let function = self.declare_libc_fn(
                    runtime,
                    &[pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(function, &[handle.into()], "bubble_sort.bool")
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: BubbleSort method returned no value")?
            }
            tyrus_core::algorithm::BUBBLE_SORT_LEFT_TARGET
            | tyrus_core::algorithm::BUBBLE_SORT_RIGHT_TARGET
            | tyrus_core::algorithm::BUBBLE_SORT_PASS_TARGET
            | tyrus_core::algorithm::BUBBLE_SORT_COMPARISONS_TARGET
            | tyrus_core::algorithm::BUBBLE_SORT_SWAPS_TARGET => {
                if args.len() != 1 {
                    return Err("error: BubbleSort method expects 0 arguments".into());
                }
                let handle = self.emit_expr(&args[0])?;
                let runtime = match target {
                    tyrus_core::algorithm::BUBBLE_SORT_LEFT_TARGET => "tyrus_bubble_sort_left",
                    tyrus_core::algorithm::BUBBLE_SORT_RIGHT_TARGET => "tyrus_bubble_sort_right",
                    tyrus_core::algorithm::BUBBLE_SORT_PASS_TARGET => "tyrus_bubble_sort_pass",
                    tyrus_core::algorithm::BUBBLE_SORT_COMPARISONS_TARGET => {
                        "tyrus_bubble_sort_comparisons"
                    }
                    _ => "tyrus_bubble_sort_swaps",
                };
                let function = self.declare_libc_fn(
                    runtime,
                    &[pointer.into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(function, &[handle.into()], "bubble_sort.metric")
                    .map_err(|error| error.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: BubbleSort metric returned no value")?
            }
            tyrus_core::algorithm::BUBBLE_SORT_RUN_TARGET
            | tyrus_core::algorithm::BUBBLE_SORT_VALUES_TARGET => {
                if args.len() != 1 {
                    return Err("error: BubbleSort method expects 0 arguments".into());
                }
                let handle = self.emit_expr(&args[0])?;
                let array_ty = Type::Array(Box::new(Type::I32));
                let output = self
                    .builder
                    .build_alloca(self.llvm_type(&array_ty), "bubble_sort.output")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(output, self.llvm_type(&array_ty).const_zero())
                    .map_err(|error| error.to_string())?;
                let runtime = if target == tyrus_core::algorithm::BUBBLE_SORT_RUN_TARGET {
                    "tyrus_bubble_sort_run"
                } else {
                    "tyrus_bubble_sort_values"
                };
                let function = self.declare_libc_fn(
                    runtime,
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(
                        function,
                        &[handle.into(), output.into()],
                        "bubble_sort.array",
                    )
                    .map_err(|error| error.to_string())?;
                let result = self
                    .builder
                    .build_load(
                        self.llvm_type(&array_ty),
                        output,
                        "bubble_sort.array.result",
                    )
                    .map_err(|error| error.to_string())?;
                self.record_owned_native_result(result, array_ty);
                result
            }
            tyrus_core::algorithm::BUBBLE_SORT_RESET_TARGET => {
                if args.len() != 1 {
                    return Err("error: `reset` expects 0 arguments".into());
                }
                let handle = self.emit_expr(&args[0])?;
                let function =
                    self.declare_libc_fn("tyrus_bubble_sort_reset", &[pointer.into()], None);
                self.builder
                    .build_call(function, &[handle.into()], "bubble_sort.reset")
                    .map_err(|error| error.to_string())?;
                self.context.i32_type().const_zero().into()
            }
            tyrus_core::algorithm::FENWICK_TREE_ADD_TARGET => {
                if args.len() != 3 {
                    return Err("error: `add` expects 2 arguments".into());
                }
                let ft = self.emit_expr(&args[0])?;
                let index = self.emit_expr_as(&args[1], &Type::Usize)?;
                let delta = self.emit_expr_as(&args[2], &Type::I32)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_fenwick_tree_add",
                    &[
                        pointer.into(),
                        self.usize_type().into(),
                        self.context.i32_type().into(),
                    ],
                    None,
                );
                self.builder
                    .build_call(fn_val, &[ft.into(), index.into(), delta.into()], "ft.add")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::FENWICK_TREE_SUM_TARGET => {
                if args.len() != 2 {
                    return Err("error: `sum` expects 1 argument".into());
                }
                let ft = self.emit_expr(&args[0])?;
                let index = self.emit_expr_as(&args[1], &Type::Usize)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_fenwick_tree_sum",
                    &[pointer.into(), self.usize_type().into()],
                    Some(self.context.i32_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[ft.into(), index.into()], "ft.sum")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: fenwick_tree_sum did not return a value")?
            }
            tyrus_core::algorithm::FENWICK_TREE_LEN_TARGET => {
                if args.len() != 1 {
                    return Err("error: `len` expects 0 arguments".into());
                }
                let ft = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_fenwick_tree_len",
                    &[pointer.into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[ft.into()], "ft.len")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: fenwick_tree_len did not return a value")?
            }
            tyrus_core::algorithm::FENWICK_TREE_IS_EMPTY_TARGET => {
                if args.len() != 1 {
                    return Err("error: `isEmpty` expects 0 arguments".into());
                }
                let ft = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_fenwick_tree_is_empty",
                    &[pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[ft.into()], "ft.is_empty")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: fenwick_tree_is_empty did not return a value")?
            }
            tyrus_core::algorithm::SEGMENT_TREE_UPDATE_TARGET => {
                if args.len() != 3 {
                    return Err("error: `update` expects 2 arguments".into());
                }
                let st = self.emit_expr(&args[0])?;
                let index = self.emit_expr_as(&args[1], &Type::Usize)?;
                let value = self.emit_expr_as(&args[2], &Type::I32)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_segment_tree_update",
                    &[
                        pointer.into(),
                        self.usize_type().into(),
                        self.context.i32_type().into(),
                    ],
                    None,
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[st.into(), index.into(), value.into()],
                        "st.update",
                    )
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::SEGMENT_TREE_QUERY_TARGET => {
                if args.len() != 3 {
                    return Err("error: `query` expects 2 arguments".into());
                }
                let st = self.emit_expr(&args[0])?;
                let from = self.emit_expr_as(&args[1], &Type::Usize)?;
                let to = self.emit_expr_as(&args[2], &Type::Usize)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_segment_tree_query",
                    &[
                        pointer.into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                    ],
                    Some(self.context.i32_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[st.into(), from.into(), to.into()], "st.query")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: segment_tree_query did not return a value")?
            }
            tyrus_core::algorithm::SEGMENT_TREE_LEN_TARGET => {
                if args.len() != 1 {
                    return Err("error: `len` expects 0 arguments".into());
                }
                let st = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_segment_tree_len",
                    &[pointer.into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[st.into()], "st.len")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: segment_tree_len did not return a value")?
            }
            tyrus_core::algorithm::SEGMENT_TREE_IS_EMPTY_TARGET => {
                if args.len() != 1 {
                    return Err("error: `isEmpty` expects 0 arguments".into());
                }
                let st = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_segment_tree_is_empty",
                    &[pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[st.into()], "st.is_empty")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: segment_tree_is_empty did not return a value")?
            }
            tyrus_core::algorithm::SPARSE_TABLE_QUERY_TARGET => {
                if args.len() != 3 {
                    return Err("error: `query` expects 2 arguments".into());
                }
                let st = self.emit_expr(&args[0])?;
                let from = self.emit_expr_as(&args[1], &Type::Usize)?;
                let to = self.emit_expr_as(&args[2], &Type::Usize)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_sparse_table_query",
                    &[
                        pointer.into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                    ],
                    Some(self.context.i32_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[st.into(), from.into(), to.into()], "st.query")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: sparse_table_query did not return a value")?
            }
            tyrus_core::algorithm::SPARSE_TABLE_LEN_TARGET => {
                if args.len() != 1 {
                    return Err("error: `len` expects 0 arguments".into());
                }
                let st = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_sparse_table_len",
                    &[pointer.into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[st.into()], "st.len")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: sparse_table_len did not return a value")?
            }
            tyrus_core::algorithm::SPARSE_TABLE_IS_EMPTY_TARGET => {
                if args.len() != 1 {
                    return Err("error: `isEmpty` expects 0 arguments".into());
                }
                let st = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_sparse_table_is_empty",
                    &[pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[st.into()], "st.is_empty")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: sparse_table_is_empty did not return a value")?
            }
            _ => return Ok(None),
        };
        Ok(Some(result))
    }
}
