use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_field_access_expr(
        &mut self,
        expr: &Expr,
        object: &Expr,
        field: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if self.is_process_env(object) {
            let ty = self.env_declared_type(field).ok_or_else(|| {
                format!("internal: env variable `{field}` missing from the env schema")
            })?;
            let value = self.emit_env_get(field, &ty)?;
            if matches!(self.resolve_type(&ty), Type::Str) {
                self.record_owned_native_result(value, Type::Str);
            }
            return Ok(value);
        }
        if field == "argv" && self.is_process_namespace(object) {
            return self.emit_process_argv();
        }
        if field == "pid" && self.is_process_namespace(object) {
            return self.emit_process_pid();
        }
        if field == "ppid" && self.is_process_namespace(object) {
            return self.emit_process_ppid();
        }
        if let ExprKind::Ident(type_name) = object.kind() {
            if let Some(value) = self.emit_numeric_constant(type_name, field) {
                return Ok(value);
            }
            if let Some(enum_info) = self.enums.get(type_name)
                && let Some((_, value)) =
                    enum_info.members.iter().find(|(member, _)| member == field)
            {
                if enum_info.is_algebraic {
                    let enum_info = enum_info.clone();
                    return self.emit_enum_variant(type_name, field, &enum_info);
                }
                let value = value.clone();
                let repr = enum_info.repr.clone();
                return self.emit_expr_as(&value, &repr);
            }
        }
        if let Some(Type::Named(namespace)) = self.expr_type_hint(object)
            && let Some(value) = tyrus_core::crypto::crypto_namespace_constant(&namespace, field)
        {
            return Ok(self.const_string_named(value, "crypto.constant").into());
        }
        let object_type = self
            .expr_type_hint(object)
            .ok_or_else(|| format!("error: cannot infer object type for field `{field}`"))?;
        if let Type::Tuple(elements) = transparent_codegen_type(&object_type)
            && field == "length"
        {
            return Ok(self
                .usize_type()
                .const_int(elements.len() as u64, false)
                .into());
        }
        if let Type::Array(_) = transparent_codegen_type(&object_type)
            && matches!(field, "length" | "capacity")
        {
            let array = self.emit_expr(object)?.into_struct_value();
            return self
                .builder
                .build_extract_value(
                    array,
                    if field == "length" { 1 } else { 2 },
                    if field == "length" {
                        "array_length"
                    } else {
                        "array_capacity"
                    },
                )
                .map_err(|error| error.to_string());
        }
        if let Some((variants, underlying)) = self.payload_free_union_underlying(&object_type)
            && matches!(underlying, Type::Str)
            && field == "length"
        {
            let union_value = self.emit_expr(object)?;
            let text = self.emit_union_underlying_value(union_value, &variants, "union.text")?;
            return Ok(self.string_char_len(text.into_struct_value())?.into());
        }
        if matches!(transparent_codegen_type(&object_type), Type::Str) && field == "length" {
            let s = self.emit_expr(object)?.into_struct_value();
            return Ok(self.string_char_len(s)?.into());
        }
        if matches!(transparent_codegen_type(&object_type), Type::Regex) {
            return self.emit_regex_field_access(object, field);
        }
        if matches!(transparent_codegen_type(&object_type), Type::Named(name) if name == "ChildProcess")
        {
            let child = self.emit_expr(object)?.into_pointer_value();
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let (symbol, result_type, stream_type) = match field {
                "pid" => (
                    "tyrus_child_process_pid",
                    self.context.i32_type().into(),
                    None,
                ),
                "stdin" => (
                    "tyrus_child_process_stdin",
                    pointer.into(),
                    Some(Type::Apply {
                        name: "WritableStream".to_string(),
                        args: vec![Type::Array(Box::new(Type::U8))],
                    }),
                ),
                "stdout" => (
                    "tyrus_child_process_stdout",
                    pointer.into(),
                    Some(Type::Apply {
                        name: "ReadableStream".to_string(),
                        args: vec![Type::Array(Box::new(Type::U8))],
                    }),
                ),
                "stderr" => (
                    "tyrus_child_process_stderr",
                    pointer.into(),
                    Some(Type::Apply {
                        name: "ReadableStream".to_string(),
                        args: vec![Type::Array(Box::new(Type::U8))],
                    }),
                ),
                _ => {
                    return Err(format!("error: unsupported ChildProcess field `{field}`"));
                }
            };
            let accessor = self.declare_libc_fn(symbol, &[pointer.into()], Some(result_type));
            let value = self
                .builder
                .build_call(accessor, &[child.into()], "process.field")
                .map_err(|error| error.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or_else(|| {
                    format!("internal: ChildProcess field `{field}` did not return a value")
                })?;
            if let Some(stream_type) = stream_type {
                self.record_owned_native_result(value, stream_type);
            }
            return Ok(value);
        }
        if let Type::Apply { name, args } = transparent_codegen_type(&object_type) {
            if matches!(name.as_str(), "DuplexStream" | "TransformStream")
                && args.len() == 2
                && matches!(field, "readable" | "writable")
            {
                let stream = self.emit_expr(object)?.into_pointer_value();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let symbol = match (name.as_str(), field) {
                    ("DuplexStream", "readable") => "tyrus_duplex_stream_readable",
                    ("DuplexStream", "writable") => "tyrus_duplex_stream_writable",
                    ("TransformStream", "readable") => "tyrus_transform_stream_readable",
                    ("TransformStream", "writable") => "tyrus_transform_stream_writable",
                    _ => unreachable!(),
                };
                let accessor =
                    self.declare_libc_fn(symbol, &[pointer.into()], Some(pointer.into()));
                let value = self
                    .builder
                    .build_call(accessor, &[stream.into()], "stream.capability")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                let payload = match (name.as_str(), field) {
                    ("DuplexStream", "readable") => args[0].clone(),
                    ("DuplexStream", "writable") => args[1].clone(),
                    ("TransformStream", "readable") => args[1].clone(),
                    ("TransformStream", "writable") => args[0].clone(),
                    _ => unreachable!(),
                };
                let capability = Type::Apply {
                    name: if field == "readable" {
                        "ReadableStream".to_string()
                    } else {
                        "WritableStream".to_string()
                    },
                    args: vec![payload],
                };
                self.record_owned_native_result(value, capability);
                return Ok(value);
            }
            if name == "MutexGuard" && args.len() == 1 && field == "value" {
                let payload_ty = self.resolve_type(&args[0]);
                let guard = self.emit_expr(object)?.into_pointer_value();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let storage_fn = self.declare_libc_fn(
                    "tyrus_mutex_guard_storage",
                    &[pointer.into()],
                    Some(pointer.into()),
                );
                let storage = self
                    .builder
                    .build_call(storage_fn, &[guard.into()], "mutex.guard.storage")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_pointer_value();
                return self
                    .builder
                    .build_load(self.llvm_type(&payload_ty), storage, "mutex.guard.value")
                    .map_err(|error| error.to_string());
            }
            if matches!(name.as_str(), "RwReadGuard" | "RwWriteGuard")
                && args.len() == 1
                && field == "value"
            {
                let payload_ty = self.resolve_type(&args[0]);
                let guard = self.emit_expr(object)?.into_pointer_value();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let storage_symbol = if name == "RwReadGuard" {
                    "tyrus_rwlock_read_guard_storage"
                } else {
                    "tyrus_rwlock_write_guard_storage"
                };
                let storage_fn =
                    self.declare_libc_fn(storage_symbol, &[pointer.into()], Some(pointer.into()));
                let storage = self
                    .builder
                    .build_call(storage_fn, &[guard.into()], "rwlock.guard.storage")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_pointer_value();
                return self
                    .builder
                    .build_load(self.llvm_type(&payload_ty), storage, "rwlock.guard.value")
                    .map_err(|error| error.to_string());
            }
        }
        if matches!(transparent_codegen_type(&object_type), Type::Map(_, _)) && field == "size" {
            return self.emit_map_size(object);
        }
        if matches!(
            transparent_codegen_type(&object_type),
            Type::Set(_) | Type::BinaryHeap(_) | Type::BinomialHeap(_) | Type::UnionFind(_)
        ) && field == "size"
        {
            return self.emit_set_size(object);
        }
        let extracted = match transparent_codegen_type(&object_type) {
            Type::Named(name) => {
                let index = self
                    .interfaces
                    .get(name)
                    .and_then(|fields| fields.iter().position(|candidate| candidate.name == *field))
                    .ok_or_else(|| format!("error: interface `{name}` has no field `{field}`"))?;
                let value = self.emit_expr(object)?.into_struct_value();
                self.builder
                    .build_extract_value(value, index as u32, field)
                    .map_err(|error| error.to_string())
            }
            Type::Object(_) => {
                let index = self
                    .struct_field_index(&object_type, field)
                    .ok_or_else(|| format!("error: object has no field `{field}`"))?;
                let value = self.emit_expr(object)?.into_struct_value();
                self.builder
                    .build_extract_value(value, index, field)
                    .map_err(|error| error.to_string())
            }
            Type::Union(variants) => {
                let variants = variants.clone();
                let union_value = self.emit_expr(object)?;
                let union_ptr =
                    self.entry_alloca(self.union_llvm_type(&variants), "union.field.source")?;
                self.builder
                    .build_store(union_ptr, union_value)
                    .map_err(|error| error.to_string())?;
                self.emit_union_common_field(&variants, union_ptr, field)
            }
            _ => Err("error: field access requires an object value".to_string()),
        }?;
        self.emit_narrowed_field_payload(expr, &object_type, field, extracted)
    }
}
