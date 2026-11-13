use super::*;
use tyrus_core::ast::ExprKind;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn collection_ptr_type(&self) -> inkwell::types::PointerType<'ctx> {
        self.context.ptr_type(inkwell::AddressSpace::default())
    }

    pub(super) fn type_size(&self, ty: &Type) -> Result<IntValue<'ctx>, String> {
        self.llvm_type(ty)
            .size_of()
            .map(|size| self.truncate_to_usize(size))
            .ok_or_else(|| format!("error: collection element type `{ty}` has no native size"))
    }

    fn collection_kind(&self, ty: &Type) -> IntValue<'ctx> {
        let kind = match transparent_codegen_type(ty) {
            Type::Str => 1,
            Type::F32 => 2,
            Type::F64 => 3,
            _ => 0,
        };
        self.context.i32_type().const_int(kind, false)
    }

    pub(super) fn emit_value_slot(
        &mut self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
    ) -> PointerValue<'ctx> {
        let slot = self
            .builder
            .build_alloca(self.llvm_type(ty), "collection_slot")
            .unwrap();
        self.builder.build_store(slot, value).unwrap();
        slot
    }

    fn build_call_ptr(
        &self,
        function: FunctionValue<'ctx>,
        args: &[BasicMetadataValueEnum<'ctx>],
        name: &str,
    ) -> BasicValueEnum<'ctx> {
        self.builder
            .build_call(function, args, name)
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
    }

    pub(super) fn emit_map_new(
        &mut self,
        key_ty: &Type,
        value_ty: &Type,
    ) -> Result<PointerValue<'ctx>, String> {
        let ptr = self.collection_ptr_type();
        let usize_type = self.usize_type();
        let key_size = self.type_size(key_ty)?;
        let value_size = self.type_size(value_ty)?;
        let kind = self.collection_kind(key_ty);
        let new_fn = self.declare_libc_fn(
            "tyrus_map_new",
            &[
                usize_type.into(),
                self.context.i32_type().into(),
                usize_type.into(),
            ],
            Some(ptr.into()),
        );
        Ok(self
            .build_call_ptr(
                new_fn,
                &[key_size.into(), kind.into(), value_size.into()],
                "map_new",
            )
            .into_pointer_value())
    }

    pub(super) fn emit_map_literal(
        &mut self,
        values: &[(String, Expr)],
        value_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let handle = self.emit_map_new(&Type::Str, value_ty)?;
        for (key, value) in values {
            let key = self.const_string_named(key, "map_key");
            self.emit_map_store(handle, key.into(), &Type::Str, value, value_ty)?;
        }
        self.record_owned_native_result(
            handle.into(),
            Type::Map(Box::new(Type::Str), Box::new(value_ty.clone())),
        );
        Ok(handle.into())
    }

    pub(super) fn emit_new_map(
        &mut self,
        key_ty: &Type,
        value_ty: &Type,
        initial: Option<&Expr>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let handle = self.emit_map_new(key_ty, value_ty)?;
        if let Some(Expr {
            kind: ExprKind::ArrayLit(entries),
            ..
        }) = initial
        {
            let entry_ty = Type::Tuple(vec![key_ty.clone(), value_ty.clone()]);
            for entry in entries {
                let pair = self.emit_expr_as(entry, &entry_ty)?.into_struct_value();
                let key = self
                    .builder
                    .build_extract_value(pair, 0, "entry_key")
                    .unwrap();
                let value = self
                    .builder
                    .build_extract_value(pair, 1, "entry_value")
                    .unwrap();
                let key_ptr = self.emit_value_slot(key, key_ty);
                let value_ptr = self.emit_value_slot(value, value_ty);
                self.emit_map_insert_ptrs(handle, key_ptr, value_ptr);
            }
        }
        Ok(handle.into())
    }

    fn emit_map_insert_ptrs(
        &mut self,
        handle: PointerValue<'ctx>,
        key_ptr: PointerValue<'ctx>,
        value_ptr: PointerValue<'ctx>,
    ) {
        let ptr = self.collection_ptr_type();
        let insert_fn = self.declare_libc_fn(
            "tyrus_map_insert",
            &[ptr.into(), ptr.into(), ptr.into()],
            None,
        );
        self.builder
            .build_call(
                insert_fn,
                &[handle.into(), key_ptr.into(), value_ptr.into()],
                "map_insert",
            )
            .unwrap();
    }

    fn emit_map_store(
        &mut self,
        handle: PointerValue<'ctx>,
        key: BasicValueEnum<'ctx>,
        key_ty: &Type,
        value: &Expr,
        value_ty: &Type,
    ) -> Result<(), String> {
        let key_ptr = self.emit_value_slot(key, key_ty);
        let stored = self.emit_expr_as(value, value_ty)?;
        let value_ptr = self.emit_value_slot(stored, value_ty);
        self.emit_map_insert_ptrs(handle, key_ptr, value_ptr);
        Ok(())
    }

    pub(super) fn emit_map_insert(
        &mut self,
        handle: PointerValue<'ctx>,
        key: &Expr,
        key_ty: &Type,
        value: &Expr,
        value_ty: &Type,
    ) -> Result<(), String> {
        let key_value = self.emit_expr_as(key, key_ty)?;
        self.emit_map_store(handle, key_value, key_ty, value, value_ty)
    }

    pub(super) fn emit_map_get(
        &mut self,
        object: &Expr,
        key: &Expr,
        key_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr = self.collection_ptr_type();
        let handle = self.emit_expr(object)?.into_pointer_value();
        let key_value = self.emit_expr_as(key, key_ty)?;
        let key_ptr = self.emit_value_slot(key_value, key_ty);
        let get_fn =
            self.declare_libc_fn("tyrus_map_get", &[ptr.into(), ptr.into()], Some(ptr.into()));
        Ok(self.build_call_ptr(get_fn, &[handle.into(), key_ptr.into()], "map_get"))
    }

    pub(super) fn emit_map_size(&mut self, object: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr = self.collection_ptr_type();
        let usize_type = self.usize_type();
        let handle = self.emit_expr(object)?.into_pointer_value();
        let size_fn =
            self.declare_libc_fn("tyrus_map_size", &[ptr.into()], Some(usize_type.into()));
        Ok(self.build_call_ptr(size_fn, &[handle.into()], "map_size"))
    }

    fn emit_collection_array(
        &mut self,
        handle: PointerValue<'ctx>,
        filler: &str,
        element_ty: &Type,
    ) -> BasicValueEnum<'ctx> {
        let ptr = self.collection_ptr_type();
        let array_type = self.llvm_type(&Type::Array(Box::new(element_ty.clone())));
        let output = self
            .builder
            .build_alloca(array_type, "collection_array")
            .unwrap();
        let fill_fn = self.declare_libc_fn(filler, &[ptr.into(), ptr.into()], None);
        self.builder
            .build_call(fill_fn, &[handle.into(), output.into()], filler)
            .unwrap();
        self.builder
            .build_load(array_type, output, "collection_array_value")
            .unwrap()
    }

    fn emit_map_entries(
        &mut self,
        handle: PointerValue<'ctx>,
        key_ty: &Type,
        value_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let usize_type = self.usize_type();
        let keys = self
            .emit_collection_array(handle, "tyrus_map_keys", key_ty)
            .into_struct_value();
        let values = self
            .emit_collection_array(handle, "tyrus_map_values", value_ty)
            .into_struct_value();
        let length = self
            .builder
            .build_extract_value(keys, 1, "entries_len")
            .unwrap()
            .into_int_value();
        let key_data = self
            .builder
            .build_extract_value(keys, 0, "keys_data")
            .unwrap()
            .into_pointer_value();
        let value_data = self
            .builder
            .build_extract_value(values, 0, "values_data")
            .unwrap()
            .into_pointer_value();

        let tuple_ty = Type::Tuple(vec![key_ty.clone(), value_ty.clone()]);
        let tuple_llvm = self.llvm_type(&tuple_ty);
        let key_llvm = self.llvm_type(key_ty);
        let value_llvm = self.llvm_type(value_ty);
        let entry_size = tuple_llvm
            .size_of()
            .ok_or_else(|| "error: map entry type has no native size".to_string())?;
        let alloc_size = self
            .builder
            .build_int_mul(entry_size, length, "entries_bytes")
            .unwrap();
        let (malloc, alloc_size) = self.declare_malloc(alloc_size);
        let data = self
            .build_call_ptr(malloc, &[alloc_size.into()], "entries_data")
            .into_pointer_value();

        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or_else(|| "internal: entries loop outside a function".to_string())?;
        let index_slot = self.builder.build_alloca(usize_type, "entries_i").unwrap();
        self.builder
            .build_store(index_slot, usize_type.const_zero())
            .unwrap();
        let cond = self.context.append_basic_block(function, "entries.cond");
        let body = self.context.append_basic_block(function, "entries.body");
        let end = self.context.append_basic_block(function, "entries.end");
        self.builder.build_unconditional_branch(cond).unwrap();

        self.builder.position_at_end(cond);
        let index = self
            .builder
            .build_load(usize_type, index_slot, "entries_idx")
            .unwrap()
            .into_int_value();
        let more = self
            .builder
            .build_int_compare(IntPredicate::ULT, index, length, "entries_more")
            .unwrap();
        self.builder
            .build_conditional_branch(more, body, end)
            .unwrap();

        self.builder.position_at_end(body);
        let key_slot = unsafe {
            self.builder
                .build_gep(key_llvm, key_data, &[index], "entry_key_ptr")
                .unwrap()
        };
        let key_value = self
            .builder
            .build_load(key_llvm, key_slot, "entry_key")
            .unwrap();
        let value_slot = unsafe {
            self.builder
                .build_gep(value_llvm, value_data, &[index], "entry_value_ptr")
                .unwrap()
        };
        let value_value = self
            .builder
            .build_load(value_llvm, value_slot, "entry_value")
            .unwrap();
        let mut tuple = tuple_llvm.into_struct_type().get_undef();
        tuple = self
            .builder
            .build_insert_value(tuple, key_value, 0, "entry_tuple_k")
            .unwrap()
            .into_struct_value();
        tuple = self
            .builder
            .build_insert_value(tuple, value_value, 1, "entry_tuple_v")
            .unwrap()
            .into_struct_value();
        let dest = unsafe {
            self.builder
                .build_gep(tuple_llvm, data, &[index], "entry_dest")
                .unwrap()
        };
        self.builder.build_store(dest, tuple).unwrap();
        let next = self
            .builder
            .build_int_add(index, usize_type.const_int(1, false), "entries_next")
            .unwrap();
        self.builder.build_store(index_slot, next).unwrap();
        self.builder.build_unconditional_branch(cond).unwrap();

        self.builder.position_at_end(end);
        self.emit_owned_native_buffer_free(keys.into());
        self.emit_owned_native_buffer_free(values.into());
        let array_type = self.llvm_type(&Type::Array(Box::new(tuple_ty)));
        let mut array = array_type.into_struct_type().get_undef();
        array = self
            .builder
            .build_insert_value(array, data, 0, "entries_arr_data")
            .unwrap()
            .into_struct_value();
        array = self
            .builder
            .build_insert_value(array, length, 1, "entries_arr_len")
            .unwrap()
            .into_struct_value();
        array = self
            .builder
            .build_insert_value(array, length, 2, "entries_arr_cap")
            .unwrap()
            .into_struct_value();
        Ok(array.into())
    }

    pub(super) fn emit_map_method(
        &mut self,
        object: &Expr,
        key_ty: &Type,
        value_ty: &Type,
        method: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr = self.collection_ptr_type();
        let handle = self.emit_expr(object)?.into_pointer_value();
        match method {
            "get" => {
                let key_value = self.emit_expr_as(&args[0], key_ty)?;
                let key_ptr = self.emit_value_slot(key_value, key_ty);
                let get_fn = self.declare_libc_fn(
                    "tyrus_map_get",
                    &[ptr.into(), ptr.into()],
                    Some(ptr.into()),
                );
                Ok(self.build_call_ptr(get_fn, &[handle.into(), key_ptr.into()], "map_get"))
            }
            "set" => {
                let key_value = self.emit_expr_as(&args[0], key_ty)?;
                let key_ptr = self.emit_value_slot(key_value, key_ty);
                let stored = self.emit_expr_as(&args[1], value_ty)?;
                let value_ptr = self.emit_value_slot(stored, value_ty);
                self.emit_map_insert_ptrs(handle, key_ptr, value_ptr);
                Ok(handle.into())
            }
            "getOrInsert" => self.emit_map_get_or_insert(handle, key_ty, value_ty, args),
            "has" | "delete" => {
                let key_value = self.emit_expr_as(&args[0], key_ty)?;
                let key_ptr = self.emit_value_slot(key_value, key_ty);
                let name = if method == "has" {
                    "tyrus_map_has"
                } else {
                    "tyrus_map_delete"
                };
                let function = self.declare_libc_fn(
                    name,
                    &[ptr.into(), ptr.into()],
                    Some(self.context.bool_type().into()),
                );
                Ok(self.build_call_ptr(function, &[handle.into(), key_ptr.into()], method))
            }
            "keys" => Ok(self.emit_collection_array(handle, "tyrus_map_keys", key_ty)),
            "values" => Ok(self.emit_collection_array(handle, "tyrus_map_values", value_ty)),
            "entries" => self.emit_map_entries(handle, key_ty, value_ty),
            other => Err(format!("error: unknown map method `{other}`")),
        }
    }

    fn emit_map_get_or_insert(
        &mut self,
        handle: PointerValue<'ctx>,
        key_ty: &Type,
        value_ty: &Type,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr = self.collection_ptr_type();
        let key_value = self.emit_expr_as(&args[0], key_ty)?;
        let key_ptr = self.emit_value_slot(key_value, key_ty);
        let has_fn = self.declare_libc_fn(
            "tyrus_map_has",
            &[ptr.into(), ptr.into()],
            Some(self.context.bool_type().into()),
        );
        let present = self.build_call_ptr(
            has_fn,
            &[handle.into(), key_ptr.into()],
            "map_get_or_insert_has",
        );
        let current = self
            .builder
            .get_insert_block()
            .ok_or_else(|| "error: map getOrInsert has no insertion block".to_string())?;
        let function = current
            .get_parent()
            .ok_or_else(|| "error: map getOrInsert has no parent function".to_string())?;
        let hit = self
            .context
            .append_basic_block(function, "map.get_or_insert.hit");
        let miss = self
            .context
            .append_basic_block(function, "map.get_or_insert.miss");
        let merge = self
            .context
            .append_basic_block(function, "map.get_or_insert.merge");
        self.builder
            .build_conditional_branch(present.into_int_value(), hit, miss)
            .unwrap();

        self.builder.position_at_end(hit);
        let get_fn =
            self.declare_libc_fn("tyrus_map_get", &[ptr.into(), ptr.into()], Some(ptr.into()));
        let stored = self
            .build_call_ptr(
                get_fn,
                &[handle.into(), key_ptr.into()],
                "map_get_or_insert_get",
            )
            .into_pointer_value();
        let value = self
            .builder
            .build_load(self.llvm_type(value_ty), stored, "map_get_or_insert_value")
            .unwrap();
        let free = self.declare_libc_fn("free", &[ptr.into()], None);
        self.builder
            .build_call(free, &[stored.into()], "map_get_or_insert_free")
            .unwrap();
        let hit_end = self.builder.get_insert_block().unwrap();
        self.builder.build_unconditional_branch(merge).unwrap();

        self.builder.position_at_end(miss);
        let fallback = self.emit_expr_as(&args[1], value_ty)?;
        let fallback_ptr = self.emit_value_slot(fallback, value_ty);
        self.emit_map_insert_ptrs(handle, key_ptr, fallback_ptr);
        let miss_end = self.builder.get_insert_block().unwrap();
        self.builder.build_unconditional_branch(merge).unwrap();

        self.builder.position_at_end(merge);
        let phi = self
            .builder
            .build_phi(self.llvm_type(value_ty), "map_get_or_insert_value")
            .unwrap();
        phi.add_incoming(&[(&value, hit_end), (&fallback, miss_end)]);
        Ok(phi.as_basic_value())
    }

    pub(super) fn emit_new_set(
        &mut self,
        element_ty: &Type,
        initial: Option<&Expr>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr = self.collection_ptr_type();
        let usize_type = self.usize_type();
        let element_size = self.type_size(element_ty)?;
        let kind = self.collection_kind(element_ty);
        let new_fn = self.declare_libc_fn(
            "tyrus_set_new",
            &[usize_type.into(), self.context.i32_type().into()],
            Some(ptr.into()),
        );
        let handle = self
            .build_call_ptr(new_fn, &[element_size.into(), kind.into()], "set_new")
            .into_pointer_value();
        if let Some(Expr {
            kind: ExprKind::ArrayLit(values),
            ..
        }) = initial
        {
            for value in values {
                let element = self.emit_expr_as(value, element_ty)?;
                self.emit_set_add_value(handle, element, element_ty);
            }
        }
        Ok(handle.into())
    }

    fn emit_set_add_value(
        &mut self,
        handle: PointerValue<'ctx>,
        value: BasicValueEnum<'ctx>,
        element_ty: &Type,
    ) {
        let ptr = self.collection_ptr_type();
        let slot = self.emit_value_slot(value, element_ty);
        let add_fn = self.declare_libc_fn("tyrus_set_add", &[ptr.into(), ptr.into()], None);
        self.builder
            .build_call(add_fn, &[handle.into(), slot.into()], "set_add")
            .unwrap();
    }

    pub(super) fn emit_set_size(&mut self, object: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr = self.collection_ptr_type();
        let usize_type = self.usize_type();
        let handle = self.emit_expr(object)?.into_pointer_value();
        let size_fn =
            self.declare_libc_fn("tyrus_set_size", &[ptr.into()], Some(usize_type.into()));
        Ok(self.build_call_ptr(size_fn, &[handle.into()], "set_size"))
    }

    pub(super) fn emit_set_method(
        &mut self,
        object: &Expr,
        element_ty: &Type,
        method: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr = self.collection_ptr_type();
        let handle = self.emit_expr(object)?.into_pointer_value();
        match method {
            "add" => {
                let value = self.emit_expr_as(&args[0], element_ty)?;
                self.emit_set_add_value(handle, value, element_ty);
                Ok(handle.into())
            }
            "has" | "delete" => {
                let value = self.emit_expr_as(&args[0], element_ty)?;
                let slot = self.emit_value_slot(value, element_ty);
                let name = if method == "has" {
                    "tyrus_set_has"
                } else {
                    "tyrus_set_delete"
                };
                let function = self.declare_libc_fn(
                    name,
                    &[ptr.into(), ptr.into()],
                    Some(self.context.bool_type().into()),
                );
                Ok(self.build_call_ptr(function, &[handle.into(), slot.into()], method))
            }
            "values" => Ok(self.emit_collection_array(handle, "tyrus_set_values", element_ty)),
            other => Err(format!("error: unknown set method `{other}`")),
        }
    }

    pub(super) fn emit_collection_iterable(
        &mut self,
        object: &Expr,
        collection_ty: &Type,
    ) -> Result<Option<(BasicValueEnum<'ctx>, Type)>, String> {
        match transparent_codegen_type(collection_ty) {
            Type::Set(element) => {
                let element = element.as_ref().clone();
                let handle = self.emit_expr(object)?.into_pointer_value();
                let array = self.emit_collection_array(handle, "tyrus_set_values", &element);
                Ok(Some((array, element)))
            }
            Type::BinaryHeap(element) => {
                let element = element.as_ref().clone();
                let handle = self.emit_expr(object)?.into_pointer_value();
                let array =
                    self.emit_collection_array(handle, "tyrus_binary_heap_values", &element);
                Ok(Some((array, element)))
            }
            Type::BinomialHeap(element) => {
                let element = element.as_ref().clone();
                let handle = self.emit_expr(object)?.into_pointer_value();
                let array =
                    self.emit_collection_array(handle, "tyrus_binomial_heap_values", &element);
                Ok(Some((array, element)))
            }
            Type::UnionFind(element) => {
                let element = element.as_ref().clone();
                let handle = self.emit_expr(object)?.into_pointer_value();
                let ptr = self.collection_ptr_type();
                let array_type = self.llvm_type(&Type::Array(Box::new(element.clone())));
                let output = self
                    .builder
                    .build_alloca(array_type, "union_find_values")
                    .unwrap();
                let function = self.declare_libc_fn(
                    "tyrus_union_find_values",
                    &[ptr.into(), self.usize_type().into(), ptr.into()],
                    None,
                );
                let size = self.type_size(&element)?;
                self.builder
                    .build_call(
                        function,
                        &[handle.into(), size.into(), output.into()],
                        "tyrus_union_find_values",
                    )
                    .unwrap();
                Ok(Some((
                    self.builder
                        .build_load(array_type, output, "union_find_values_result")
                        .unwrap(),
                    element,
                )))
            }
            Type::Counter(element) => {
                let element = element.as_ref().clone();
                let handle = self.emit_expr(object)?.into_pointer_value();
                let ptr = self.collection_ptr_type();
                let array_type = self.llvm_type(&Type::Array(Box::new(element.clone())));
                let output = self
                    .builder
                    .build_alloca(array_type, "counter_keys")
                    .unwrap();
                let function =
                    self.declare_libc_fn("tyrus_counter_keys", &[ptr.into(), ptr.into()], None);
                self.builder
                    .build_call(
                        function,
                        &[handle.into(), output.into()],
                        "tyrus_counter_keys",
                    )
                    .unwrap();
                Ok(Some((
                    self.builder
                        .build_load(array_type, output, "counter_keys_result")
                        .unwrap(),
                    element,
                )))
            }
            Type::Deque(element) => {
                let element = element.as_ref().clone();
                let handle = self.emit_expr(object)?.into_pointer_value();
                let ptr = self.collection_ptr_type();
                let array_type = self.llvm_type(&Type::Array(Box::new(element.clone())));
                let output = self.builder.build_alloca(array_type, "deque_keys").unwrap();
                let function =
                    self.declare_libc_fn("tyrus_deque_keys", &[ptr.into(), ptr.into()], None);
                self.builder
                    .build_call(
                        function,
                        &[handle.into(), output.into()],
                        "tyrus_deque_keys",
                    )
                    .unwrap();
                Ok(Some((
                    self.builder
                        .build_load(array_type, output, "deque_keys_result")
                        .unwrap(),
                    element,
                )))
            }
            Type::Map(key, value) => {
                let key = key.as_ref().clone();
                let value = value.as_ref().clone();
                let handle = self.emit_expr(object)?.into_pointer_value();
                let entries = self.emit_map_entries(handle, &key, &value)?;
                Ok(Some((entries, Type::Tuple(vec![key, value]))))
            }
            _ => Ok(None),
        }
    }
}
