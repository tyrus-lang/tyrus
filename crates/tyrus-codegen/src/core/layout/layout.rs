use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(crate) fn union_llvm_type(&self, variants: &[Type]) -> StructType<'ctx> {
        self.context.struct_type(
            &[
                self.context.i8_type().into(),
                self.union_payload_type(variants),
            ],
            false,
        )
    }

    pub(super) fn align_to_union_payload(
        instruction: inkwell::values::InstructionValue<'ctx>,
    ) -> Result<(), String> {
        instruction
            .set_alignment(UNION_PAYLOAD_ALIGNMENT)
            .map_err(|error| format!("union payload alignment: {error}"))
    }

    pub(super) fn layout_weight(&self, ty: &Type, seen: &mut HashSet<String>) -> (u32, u32) {
        let resolved = self.resolve_type(ty);
        let pointer = (8, 8);
        match transparent_codegen_type(&resolved) {
            Type::Bool | Type::I8 | Type::U8 => (1, 1),
            Type::I16 | Type::U16 => (2, 2),
            Type::I32 | Type::U32 | Type::F32 | Type::Char => (4, 4),
            Type::I64 | Type::U64 | Type::F64 | Type::Isize | Type::Usize => (8, 8),
            Type::I128 | Type::U128 | Type::F128 => (16, 16),
            Type::Decimal => (16, 32),
            Type::Array(_) => (8, 24),
            Type::Str | Type::Regex => (8, 16),
            Type::Range(element) => {
                let (align, size) = self.layout_weight(element, seen);
                (align.max(1), size * 2 + 1)
            }
            Type::Tuple(elements) => {
                let mut align = 1;
                let mut size = 0;
                for element in elements {
                    let (element_align, element_size) = self.layout_weight(element, seen);
                    align = align.max(element_align);
                    size += element_size;
                }
                (align, size)
            }
            Type::Object(fields) => self.aggregate_weight(&fields.clone(), seen),
            Type::Named(name) => {
                if !seen.insert(name.clone()) {
                    return pointer;
                }
                match self.interfaces.get(name.as_str()) {
                    Some(fields) => self.aggregate_weight(&fields.clone(), seen),
                    None => pointer,
                }
            }
            _ => pointer,
        }
    }

    pub(crate) fn llvm_type(&self, ty: &Type) -> BasicTypeEnum<'ctx> {
        let resolved = self.resolve_type(ty);
        if &resolved != ty {
            return self.llvm_type(&resolved);
        }
        match ty {
            Type::Infer => unreachable!("contextual type reached LLVM code generation"),
            Type::Throws { function, .. } => self.llvm_type(function),
            Type::I8 => self.context.i8_type().into(),
            Type::I16 => self.context.i16_type().into(),
            Type::I32 => self.context.i32_type().into(),
            Type::I64 => self.context.i64_type().into(),
            Type::I128 => self.context.i128_type().into(),
            Type::Isize => self.usize_type().into(),
            Type::U8 => self.context.i8_type().into(),
            Type::U16 => self.context.i16_type().into(),
            Type::U32 => self.context.i32_type().into(),
            Type::U64 => self.context.i64_type().into(),
            Type::U128 => self.context.i128_type().into(),
            Type::Usize => self.usize_type().into(),
            Type::F32 => self.context.f32_type().into(),
            Type::F64 => self.context.f64_type().into(),
            Type::F128 => self.context.f128_type().into(),
            Type::Decimal => self
                .context
                .struct_type(
                    &[
                        self.context.i128_type().into(),
                        self.context.i32_type().into(),
                    ],
                    false,
                )
                .into(),
            Type::Bool => self.context.bool_type().into(),
            Type::Char => self.context.i32_type().into(),
            Type::Apply { name, args }
                if matches!(
                    name.as_str(),
                    "Promise"
                        | "DbPromise"
                        | "JoinHandle"
                        | "Sender"
                        | "Receiver"
                        | "Mutex"
                        | "MutexGuard"
                        | "RwLock"
                        | "RwReadGuard"
                        | "RwWriteGuard"
                        | "ReadableStream"
                        | "WritableStream"
                ) && args.len() == 1 =>
            {
                self.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into()
            }
            Type::Apply { name, args }
                if matches!(name.as_str(), "DuplexStream" | "TransformStream")
                    && args.len() == 2 =>
            {
                self.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into()
            }
            Type::Array(_) => self
                .context
                .struct_type(
                    &[
                        self.context
                            .ptr_type(inkwell::AddressSpace::default())
                            .into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                    ],
                    false,
                )
                .into(),
            Type::Tuple(elements) => self
                .context
                .struct_type(
                    &elements
                        .iter()
                        .map(|element| self.llvm_type(element))
                        .collect::<Vec<_>>(),
                    false,
                )
                .into(),
            Type::Named(name) => {
                if name == "ObjectId" {
                    return self.llvm_type(&Type::Str);
                }
                if tyrus_core::native_handles::is_opaque_handle_name(name) {
                    return self
                        .context
                        .ptr_type(inkwell::AddressSpace::default())
                        .into();
                }
                if let Some(enum_info) = self.enums.get(name)
                    && enum_info.is_algebraic
                {
                    return self.enum_llvm_type(name, enum_info);
                }
                let fields = self
                    .interfaces
                    .get(name)
                    .unwrap_or_else(|| panic!("unknown interface `{name}`"));
                self.context
                    .struct_type(
                        &fields
                            .iter()
                            .map(|field| self.llvm_type(&field_storage_type(field)))
                            .collect::<Vec<_>>(),
                        false,
                    )
                    .into()
            }
            Type::Ref(inner) | Type::MutRef(inner) | Type::Readonly(inner) => self.llvm_type(inner),
            Type::Apply { name, .. } if name == "HttpReply" || name == "__HttpReplyStatus" => self
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
            Type::Function { .. } | Type::GenericFunction { .. } => self
                .context
                .struct_type(
                    &[
                        self.context
                            .ptr_type(inkwell::AddressSpace::default())
                            .into(),
                        self.context
                            .ptr_type(inkwell::AddressSpace::default())
                            .into(),
                    ],
                    false,
                )
                .into(),
            Type::Str | Type::Regex => self.string_type().into(),
            Type::Optional(_) | Type::Generic(_) => self
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
            Type::Map(_, _)
            | Type::Set(_)
            | Type::BinaryHeap(_)
            | Type::BinomialHeap(_)
            | Type::UnionFind(_)
            | Type::Counter(_)
            | Type::Deque(_)
            | Type::FenwickTree
            | Type::BubbleSort
            | Type::SegmentTree
            | Type::SparseTable
            | Type::SegmentTreeLazy
            | Type::BitSet
            | Type::Trie
            | Type::SuffixTree
            | Type::Lca
            | Type::Hld
            | Type::Centroid
            | Type::BloomFilter
            | Type::LinkedList(_)
            | Type::DoubleLinkedList(_)
            | Type::AvlTree(_)
            | Type::RedBlackTree(_)
            | Type::SplayTree(_)
            | Type::Treap(_)
            | Type::SortedSet(_)
            | Type::SkipList(_)
            | Type::SortedMap(_, _)
            | Type::Graph(_, _) => self
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
            Type::Range(element) => {
                let element = self.llvm_type(element);
                self.context
                    .struct_type(&[element, element, self.context.bool_type().into()], false)
                    .into()
            }
            Type::Object(fields) => self
                .context
                .struct_type(
                    &fields
                        .iter()
                        .map(|field| self.llvm_type(&field_storage_type(field)))
                        .collect::<Vec<_>>(),
                    false,
                )
                .into(),
            Type::Result { success, error } => self
                .union_llvm_type(&[success.as_ref().clone(), error.as_ref().clone()])
                .into(),
            Type::Union(variants) => self.union_llvm_type(variants).into(),
            Type::Unknown => self
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
            Type::Never => unreachable!("never has no runtime value representation"),
            Type::Lit(expr) => self.llvm_type(&self.lit_underlying_type(expr)),
            Type::TypeQuery(expr) => self.llvm_type(&self.type_query_underlying_type(expr)),
            Type::Void => unreachable!("void has no value representation"),
            Type::Apply { .. } | Type::KeyOf(_) | Type::Conditional { .. } | Type::InferVar(_) => {
                unreachable!("unresolved utility type reached LLVM code generation: {ty}")
            }
        }
    }

    pub(super) fn enum_llvm_type(
        &self,
        _name: &str,
        enum_info: &tyrus_core::ast::ResolvedEnum,
    ) -> BasicTypeEnum<'ctx> {
        let mut max_payload_size = 0;
        for member_kind in &enum_info.member_kinds {
            match member_kind {
                EnumPayload::Unit => {}
                EnumPayload::Tuple(types) => {
                    let size: u64 = types
                        .iter()
                        .map(|ty| self.type_layout(ty).0 as u64)
                        .sum::<u64>();
                    max_payload_size = max_payload_size.max(size);
                }
                EnumPayload::Struct(fields) => {
                    let size: u64 = fields
                        .iter()
                        .map(|field| self.type_layout(&field.ty).0 as u64)
                        .sum::<u64>();
                    max_payload_size = max_payload_size.max(size);
                }
            }
        }

        let tag_ty = self.context.i8_type();
        let payload_ty = self.context.i8_type().array_type(max_payload_size as u32);

        self.context
            .struct_type(&[tag_ty.into(), payload_ty.into()], false)
            .into()
    }

    pub(crate) fn type_layout(&self, ty: &Type) -> (usize, usize) {
        match transparent_codegen_type(&self.resolve_type(ty)) {
            Type::Throws { function, .. } => self.type_layout(function),
            Type::Bool => (1, 1),
            Type::I8 | Type::U8 => (1, 1),
            Type::I16 | Type::U16 => (2, 2),
            Type::I32 | Type::U32 | Type::F32 | Type::Char => (4, 4),
            Type::I64
            | Type::Isize
            | Type::U64
            | Type::Usize
            | Type::Optional(_)
            | Type::Generic(_) => (8, 8),
            Type::Str | Type::Regex => (16, 8),
            Type::I128 | Type::U128 | Type::F128 => (16, 16),
            Type::Decimal => (32, 16),
            Type::F64 => (8, 8),
            Type::Map(_, _)
            | Type::Set(_)
            | Type::BinaryHeap(_)
            | Type::BinomialHeap(_)
            | Type::UnionFind(_)
            | Type::Counter(_)
            | Type::Deque(_)
            | Type::FenwickTree
            | Type::BubbleSort
            | Type::SegmentTree
            | Type::SparseTable
            | Type::SegmentTreeLazy
            | Type::BitSet
            | Type::Trie
            | Type::SuffixTree
            | Type::Lca
            | Type::Hld
            | Type::Centroid
            | Type::BloomFilter
            | Type::LinkedList(_)
            | Type::DoubleLinkedList(_)
            | Type::AvlTree(_)
            | Type::RedBlackTree(_)
            | Type::SplayTree(_)
            | Type::Treap(_)
            | Type::SortedSet(_)
            | Type::SkipList(_)
            | Type::SortedMap(_, _)
            | Type::Graph(_, _) => (8, 8),
            Type::Range(element) => {
                let element = element.as_ref().clone();
                self.aggregate_layout([element.clone(), element, Type::Bool].iter())
            }
            Type::Array(_) | Type::Function { .. } | Type::GenericFunction { .. } => (24, 8),
            Type::Ref(_) | Type::MutRef(_) => (8, 8),
            Type::Readonly(inner) => self.type_layout(inner),
            Type::Tuple(elements) => self.aggregate_layout(elements.iter()),
            Type::Object(fields) => self.aggregate_layout(
                fields
                    .iter()
                    .map(field_storage_type)
                    .collect::<Vec<_>>()
                    .iter(),
            ),
            Type::Named(name) => {
                if name == "HttpRouter" || name == "HttpServer" || name == "DigestStream" {
                    return (8, 8);
                }
                let Some(fields) = self.interfaces.get(name) else {
                    return (8, 8);
                };
                self.aggregate_layout(
                    fields
                        .iter()
                        .map(field_storage_type)
                        .collect::<Vec<_>>()
                        .iter(),
                )
            }
            Type::Union(variants) => {
                let payload = variants
                    .iter()
                    .map(|variant| self.type_layout(variant))
                    .max_by_key(|(size, _)| *size)
                    .unwrap_or((16, 16));
                let offset = align_to(1, payload.1);
                let size = align_to(offset + payload.0, payload.1);
                (size, payload.1)
            }
            Type::Result { success, error } => self.type_layout(&Type::Union(vec![
                success.as_ref().clone(),
                error.as_ref().clone(),
            ])),
            Type::Lit(expr) => self.type_layout(&self.lit_underlying_type(expr)),
            Type::TypeQuery(expr) => self.type_layout(&self.type_query_underlying_type(expr)),
            Type::Infer | Type::Void | Type::Unknown | Type::Never => (0, 1),
            Type::Apply { name, .. } if name == "HttpReply" || name == "__HttpReplyStatus" => {
                (8, 8)
            }
            Type::Apply { .. } if tyrus_core::native_handles::is_handle_type(ty) => (8, 8),
            Type::Apply { .. } | Type::KeyOf(_) | Type::Conditional { .. } | Type::InferVar(_) => {
                unreachable!("unresolved utility type reached codegen layout")
            }
        }
    }

    pub(super) fn aggregate_layout<'a, I>(&self, fields: I) -> (usize, usize)
    where
        I: IntoIterator<Item = &'a Type>,
    {
        let mut size = 0;
        let mut aggregate_align = 1;
        for field in fields {
            let (field_size, field_align) = self.type_layout(field);
            aggregate_align = aggregate_align.max(field_align);
            size = align_to(size, field_align) + field_size;
        }
        (align_to(size, aggregate_align), aggregate_align)
    }

    pub(super) fn catch_binding_has_a_layout(&self, ty: &Type) -> bool {
        let resolved = self.resolve_type(ty);
        match transparent_codegen_type(&resolved) {
            Type::Named(name) => {
                self.interfaces.contains_key(name.as_str())
                    || self.declared_interfaces.contains_key(name.as_str())
                    || self.enums.contains_key(name.as_str())
            }
            _ => true,
        }
    }
}
