use super::*;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_linked_list_call(
        &mut self,
        target: &str,
        args: &[Expr],
        element_type: Option<&Type>,
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let result = match target {
            tyrus_core::algorithm::LINKED_LIST_NEW_TARGET => {
                let elem_ty = element_type.ok_or_else(|| {
                    "error: cannot infer element type for `LinkedList`".to_string()
                })?;
                let element_size = self.type_size(elem_ty)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_linked_list_new",
                    &[self.usize_type().into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(fn_val, &[element_size.into()], "ll.new")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: linked_list_new did not return a value")?
            }
            tyrus_core::algorithm::LINKED_LIST_PUSH_FRONT_TARGET => {
                if args.len() != 2 {
                    return Err("error: `pushFront` expects 1 argument".into());
                }
                let elem_ty = element_type.ok_or_else(|| {
                    "error: cannot infer element type for `pushFront`".to_string()
                })?;
                let list = self.emit_expr(&args[0])?;
                let item = self.emit_expr_as(&args[1], elem_ty)?;
                let slot = self.emit_value_slot(item, elem_ty);
                let fn_val = self.declare_libc_fn(
                    "tyrus_linked_list_push_front",
                    &[pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(fn_val, &[list.into(), slot.into()], "ll.push_front")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::LINKED_LIST_PUSH_BACK_TARGET => {
                if args.len() != 2 {
                    return Err("error: `pushBack` expects 1 argument".into());
                }
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `pushBack`".to_string())?;
                let list = self.emit_expr(&args[0])?;
                let item = self.emit_expr_as(&args[1], elem_ty)?;
                let slot = self.emit_value_slot(item, elem_ty);
                let fn_val = self.declare_libc_fn(
                    "tyrus_linked_list_push_back",
                    &[pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(fn_val, &[list.into(), slot.into()], "ll.push_back")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::LINKED_LIST_POP_FRONT_TARGET => {
                if args.len() != 1 {
                    return Err("error: `popFront` expects 0 arguments".into());
                }
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `popFront`".to_string())?;
                let list = self.emit_expr(&args[0])?;
                let llvm_elem_ty = self.llvm_type(elem_ty);
                let output = self
                    .builder
                    .build_alloca(llvm_elem_ty, "ll.pop_front.output")
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_linked_list_pop_front",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let ok = self
                    .builder
                    .build_call(fn_val, &[list.into(), output.into()], "ll.pop_front")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: linked_list_pop_front did not return a value")?
                    .into_int_value();
                self.emit_binary_heap_optional(ok, output, elem_ty, "ll.pop_front")?
            }
            tyrus_core::algorithm::LINKED_LIST_POP_BACK_TARGET => {
                if args.len() != 1 {
                    return Err("error: `popBack` expects 0 arguments".into());
                }
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `popBack`".to_string())?;
                let list = self.emit_expr(&args[0])?;
                let llvm_elem_ty = self.llvm_type(elem_ty);
                let output = self
                    .builder
                    .build_alloca(llvm_elem_ty, "ll.pop_back.output")
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_linked_list_pop_back",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let ok = self
                    .builder
                    .build_call(fn_val, &[list.into(), output.into()], "ll.pop_back")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: linked_list_pop_back did not return a value")?
                    .into_int_value();
                self.emit_binary_heap_optional(ok, output, elem_ty, "ll.pop_back")?
            }
            tyrus_core::algorithm::LINKED_LIST_FRONT_TARGET => {
                if args.len() != 1 {
                    return Err("error: `front` expects 0 arguments".into());
                }
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `front`".to_string())?;
                let list = self.emit_expr(&args[0])?;
                let llvm_elem_ty = self.llvm_type(elem_ty);
                let output = self
                    .builder
                    .build_alloca(llvm_elem_ty, "ll.front.output")
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_linked_list_front",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let ok = self
                    .builder
                    .build_call(fn_val, &[list.into(), output.into()], "ll.front")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: linked_list_front did not return a value")?
                    .into_int_value();
                self.emit_binary_heap_optional(ok, output, elem_ty, "ll.front")?
            }
            tyrus_core::algorithm::LINKED_LIST_BACK_TARGET => {
                if args.len() != 1 {
                    return Err("error: `back` expects 0 arguments".into());
                }
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `back`".to_string())?;
                let list = self.emit_expr(&args[0])?;
                let llvm_elem_ty = self.llvm_type(elem_ty);
                let output = self
                    .builder
                    .build_alloca(llvm_elem_ty, "ll.back.output")
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_linked_list_back",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let ok = self
                    .builder
                    .build_call(fn_val, &[list.into(), output.into()], "ll.back")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: linked_list_back did not return a value")?
                    .into_int_value();
                self.emit_binary_heap_optional(ok, output, elem_ty, "ll.back")?
            }
            tyrus_core::algorithm::LINKED_LIST_LEN_TARGET => {
                if args.len() != 1 {
                    return Err("error: `len` expects 0 arguments".into());
                }
                let list = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_linked_list_len",
                    &[pointer.into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[list.into()], "ll.len")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: linked_list_len did not return a value")?
            }
            tyrus_core::algorithm::LINKED_LIST_IS_EMPTY_TARGET => {
                if args.len() != 1 {
                    return Err("error: `isEmpty` expects 0 arguments".into());
                }
                let list = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_linked_list_is_empty",
                    &[pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[list.into()], "ll.is_empty")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: linked_list_is_empty did not return a value")?
            }
            tyrus_core::algorithm::LINKED_LIST_CLEAR_TARGET => {
                if args.len() != 1 {
                    return Err("error: `clear` expects 0 arguments".into());
                }
                let list = self.emit_expr(&args[0])?;
                let fn_val =
                    self.declare_libc_fn("tyrus_linked_list_clear", &[pointer.into()], None);
                self.builder
                    .build_call(fn_val, &[list.into()], "ll.clear")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::DOUBLE_LINKED_LIST_NEW_TARGET => {
                let elem_ty = element_type.ok_or_else(|| {
                    "error: cannot infer element type for `DoubleLinkedList`".to_string()
                })?;
                let element_size = self.type_size(elem_ty)?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_double_linked_list_new",
                    &[self.usize_type().into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(fn_val, &[element_size.into()], "dll.new")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: double_linked_list_new did not return a value")?
            }
            tyrus_core::algorithm::DOUBLE_LINKED_LIST_PUSH_FRONT_TARGET => {
                if args.len() != 2 {
                    return Err("error: `pushFront` expects 1 argument".into());
                }
                let elem_ty = element_type.ok_or_else(|| {
                    "error: cannot infer element type for `pushFront`".to_string()
                })?;
                let list = self.emit_expr(&args[0])?;
                let item = self.emit_expr_as(&args[1], elem_ty)?;
                let slot = self.emit_value_slot(item, elem_ty);
                let fn_val = self.declare_libc_fn(
                    "tyrus_double_linked_list_push_front",
                    &[pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(fn_val, &[list.into(), slot.into()], "dll.push_front")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::DOUBLE_LINKED_LIST_PUSH_BACK_TARGET => {
                if args.len() != 2 {
                    return Err("error: `pushBack` expects 1 argument".into());
                }
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `pushBack`".to_string())?;
                let list = self.emit_expr(&args[0])?;
                let item = self.emit_expr_as(&args[1], elem_ty)?;
                let slot = self.emit_value_slot(item, elem_ty);
                let fn_val = self.declare_libc_fn(
                    "tyrus_double_linked_list_push_back",
                    &[pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(fn_val, &[list.into(), slot.into()], "dll.push_back")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::DOUBLE_LINKED_LIST_POP_FRONT_TARGET => {
                if args.len() != 1 {
                    return Err("error: `popFront` expects 0 arguments".into());
                }
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `popFront`".to_string())?;
                let list = self.emit_expr(&args[0])?;
                let llvm_elem_ty = self.llvm_type(elem_ty);
                let output = self
                    .builder
                    .build_alloca(llvm_elem_ty, "dll.pop_front.output")
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_double_linked_list_pop_front",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let ok = self
                    .builder
                    .build_call(fn_val, &[list.into(), output.into()], "dll.pop_front")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: double_linked_list_pop_front did not return a value")?
                    .into_int_value();
                self.emit_binary_heap_optional(ok, output, elem_ty, "dll.pop_front")?
            }
            tyrus_core::algorithm::DOUBLE_LINKED_LIST_POP_BACK_TARGET => {
                if args.len() != 1 {
                    return Err("error: `popBack` expects 0 arguments".into());
                }
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `popBack`".to_string())?;
                let list = self.emit_expr(&args[0])?;
                let llvm_elem_ty = self.llvm_type(elem_ty);
                let output = self
                    .builder
                    .build_alloca(llvm_elem_ty, "dll.pop_back.output")
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_double_linked_list_pop_back",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let ok = self
                    .builder
                    .build_call(fn_val, &[list.into(), output.into()], "dll.pop_back")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: double_linked_list_pop_back did not return a value")?
                    .into_int_value();
                self.emit_binary_heap_optional(ok, output, elem_ty, "dll.pop_back")?
            }
            tyrus_core::algorithm::DOUBLE_LINKED_LIST_FRONT_TARGET => {
                if args.len() != 1 {
                    return Err("error: `front` expects 0 arguments".into());
                }
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `front`".to_string())?;
                let list = self.emit_expr(&args[0])?;
                let llvm_elem_ty = self.llvm_type(elem_ty);
                let output = self
                    .builder
                    .build_alloca(llvm_elem_ty, "dll.front.output")
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_double_linked_list_front",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let ok = self
                    .builder
                    .build_call(fn_val, &[list.into(), output.into()], "dll.front")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: double_linked_list_front did not return a value")?
                    .into_int_value();
                self.emit_binary_heap_optional(ok, output, elem_ty, "dll.front")?
            }
            tyrus_core::algorithm::DOUBLE_LINKED_LIST_BACK_TARGET => {
                if args.len() != 1 {
                    return Err("error: `back` expects 0 arguments".into());
                }
                let elem_ty = element_type
                    .ok_or_else(|| "error: cannot infer element type for `back`".to_string())?;
                let list = self.emit_expr(&args[0])?;
                let llvm_elem_ty = self.llvm_type(elem_ty);
                let output = self
                    .builder
                    .build_alloca(llvm_elem_ty, "dll.back.output")
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_double_linked_list_back",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                let ok = self
                    .builder
                    .build_call(fn_val, &[list.into(), output.into()], "dll.back")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: double_linked_list_back did not return a value")?
                    .into_int_value();
                self.emit_binary_heap_optional(ok, output, elem_ty, "dll.back")?
            }
            tyrus_core::algorithm::DOUBLE_LINKED_LIST_LEN_TARGET => {
                if args.len() != 1 {
                    return Err("error: `len` expects 0 arguments".into());
                }
                let list = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_double_linked_list_len",
                    &[pointer.into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[list.into()], "dll.len")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: double_linked_list_len did not return a value")?
            }
            tyrus_core::algorithm::DOUBLE_LINKED_LIST_IS_EMPTY_TARGET => {
                if args.len() != 1 {
                    return Err("error: `isEmpty` expects 0 arguments".into());
                }
                let list = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_double_linked_list_is_empty",
                    &[pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[list.into()], "dll.is_empty")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: double_linked_list_is_empty did not return a value")?
            }
            tyrus_core::algorithm::DOUBLE_LINKED_LIST_CLEAR_TARGET => {
                if args.len() != 1 {
                    return Err("error: `clear` expects 0 arguments".into());
                }
                let list = self.emit_expr(&args[0])?;
                let fn_val =
                    self.declare_libc_fn("tyrus_double_linked_list_clear", &[pointer.into()], None);
                self.builder
                    .build_call(fn_val, &[list.into()], "dll.clear")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::TRIE_INSERT_TARGET => {
                if args.len() != 2 {
                    return Err("error: `insert` expects 1 argument".into());
                }
                let trie = self.emit_expr(&args[0])?;
                let word = self.emit_expr(&args[1])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_trie_insert",
                    &[pointer.into(), self.string_abi_type()],
                    None,
                );
                self.builder
                    .build_call(fn_val, &[trie.into(), word.into()], "trie.insert")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::TRIE_SEARCH_TARGET => {
                if args.len() != 2 {
                    return Err("error: `search` expects 1 argument".into());
                }
                let trie = self.emit_expr(&args[0])?;
                let word = self.emit_expr(&args[1])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_trie_search",
                    &[pointer.into(), self.string_abi_type()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[trie.into(), word.into()], "trie.search")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: trie_search did not return a value")?
            }
            tyrus_core::algorithm::TRIE_STARTS_WITH_TARGET => {
                if args.len() != 2 {
                    return Err("error: `startsWith` expects 1 argument".into());
                }
                let trie = self.emit_expr(&args[0])?;
                let prefix = self.emit_expr(&args[1])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_trie_starts_with",
                    &[pointer.into(), self.string_abi_type()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[trie.into(), prefix.into()], "trie.starts_with")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: trie_starts_with did not return a value")?
            }
            tyrus_core::algorithm::TRIE_LEN_TARGET => {
                if args.len() != 1 {
                    return Err("error: `len` expects 0 arguments".into());
                }
                let trie = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_trie_len",
                    &[pointer.into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[trie.into()], "trie.len")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: trie_len did not return a value")?
            }
            tyrus_core::algorithm::TRIE_IS_EMPTY_TARGET => {
                if args.len() != 1 {
                    return Err("error: `isEmpty` expects 0 arguments".into());
                }
                let trie = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_trie_is_empty",
                    &[pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[trie.into()], "trie.is_empty")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: trie_is_empty did not return a value")?
            }
            tyrus_core::algorithm::TRIE_CLEAR_TARGET => {
                if args.len() != 1 {
                    return Err("error: `clear` expects 0 arguments".into());
                }
                let trie = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn("tyrus_trie_clear", &[pointer.into()], None);
                self.builder
                    .build_call(fn_val, &[trie.into()], "trie.clear")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            _ => return Ok(None),
        };
        Ok(Some(result))
    }
}
