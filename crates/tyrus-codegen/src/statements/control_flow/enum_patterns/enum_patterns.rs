use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_enum_variant_condition(
        &mut self,
        pattern: &MatchPattern,
        guard: &Option<Expr>,
        subject_expr: &Expr,
        subject: BasicValueEnum<'ctx>,
        is_string_match: bool,
        index: usize,
    ) -> Result<IntValue<'ctx>, String> {
        match pattern {
            MatchPattern::EnumVariant {
                enum_name,
                variant,
                binding,
            } => {
                let subject_type = self.expr_type_hint(subject_expr).ok_or_else(|| {
                    "error: cannot infer type for match enum variant pattern".to_string()
                })?;
                let resolved_subject_ty = self.resolve_type(&subject_type);

                let mut condition = if let Type::Named(type_name) =
                    transparent_codegen_type(&resolved_subject_ty)
                {
                    if let Some(enum_info) = self.enums.get(type_name) {
                        if enum_info.is_algebraic {
                            let variant_index = enum_info
                                .members
                                .iter()
                                .position(|(name, _)| name == variant)
                                .ok_or_else(|| {
                                    format!(
                                        "error: variant `{variant}` not found in enum `{type_name}`"
                                    )
                                })?;
                            let tag = self
                                .builder
                                .build_extract_value(
                                    subject.into_struct_value(),
                                    0,
                                    &format!("match.enum.{enum_name}.{variant}.tag"),
                                )
                                .unwrap()
                                .into_int_value();
                            let expected_tag = self
                                .context
                                .i8_type()
                                .const_int(variant_index as u64, false);
                            Some(
                                self.builder
                                    .build_int_compare(
                                        IntPredicate::EQ,
                                        tag,
                                        expected_tag,
                                        &format!("match.enum.{enum_name}.{variant}.cond"),
                                    )
                                    .unwrap(),
                            )
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(binding) = binding {
                    let single_field_payload = if let Type::Named(type_name) =
                        transparent_codegen_type(&resolved_subject_ty)
                    {
                        self.enums.get(type_name).is_some_and(|enum_info| {
                            enum_info
                                .members
                                .iter()
                                .position(|(name, _)| name == variant)
                                .is_some_and(|index| {
                                    matches!(
                                        &enum_info.member_kinds[index],
                                        EnumPayload::Tuple(types) if types.len() == 1
                                    )
                                })
                        })
                    } else {
                        false
                    };
                    let normalized;
                    let binding = match binding.as_ref() {
                        pattern @ (MatchPattern::Binding(_) | MatchPattern::Wildcard)
                            if single_field_payload =>
                        {
                            normalized = MatchPattern::Tuple(vec![pattern.clone()]);
                            &normalized
                        }
                        pattern => pattern,
                    };
                    match binding {
                        MatchPattern::Tuple(elements) => {
                            if let Type::Named(type_name) =
                                transparent_codegen_type(&resolved_subject_ty)
                                && let Some(enum_info) = self.enums.get(type_name).cloned()
                                && let Some(member_idx) = enum_info
                                    .members
                                    .iter()
                                    .position(|(name, _)| name == variant)
                                && let EnumPayload::Tuple(payload_types) =
                                    &enum_info.member_kinds[member_idx]
                                && payload_types.len() == elements.len()
                            {
                                let tag_size = self
                                    .context
                                    .i8_type()
                                    .size_of()
                                    .get_zero_extended_constant()
                                    .unwrap_or(1);
                                let byte_ptr_ty =
                                    self.context.ptr_type(inkwell::AddressSpace::default());
                                let payload_alloca = self
                                    .builder
                                    .build_alloca(
                                        self.llvm_type(&Type::Named(type_name.clone())),
                                        &format!("match.{enum_name}.{variant}.payload.alloca"),
                                    )
                                    .unwrap();
                                self.builder.build_store(payload_alloca, subject).unwrap();
                                let payload_struct_ptr = self
                                    .builder
                                    .build_pointer_cast(
                                        payload_alloca,
                                        byte_ptr_ty,
                                        &format!("match.{enum_name}.{variant}.payload.ptr"),
                                    )
                                    .map_err(|e| format!("pointer cast: {e}"))?;
                                let mut offset: u64 = tag_size;
                                for (elem, payload_ty) in elements.iter().zip(payload_types.iter())
                                {
                                    if let MatchPattern::Binding(name) = elem {
                                        let elem_llvm_ty = self.llvm_type(payload_ty);
                                        let elem_size = self.type_layout(payload_ty).0 as u64;
                                        let elem_ptr = unsafe {
                                            self.builder
                                                .build_gep(
                                                    self.context.i8_type(),
                                                    payload_struct_ptr,
                                                    &[self
                                                        .context
                                                        .i64_type()
                                                        .const_int(offset, false)],
                                                    &format!(
                                                        "match.{enum_name}.{variant}.{name}.ptr"
                                                    ),
                                                )
                                                .unwrap()
                                        };
                                        let alloca = self.entry_alloca(elem_llvm_ty, name)?;
                                        let alloca_byte_ptr = self
                                            .builder
                                            .build_pointer_cast(
                                                alloca,
                                                byte_ptr_ty,
                                                &format!(
                                                    "match.{enum_name}.{variant}.{name}.alloca.ptr"
                                                ),
                                            )
                                            .map_err(|e| format!("pointer cast: {e}"))?;
                                        let size_val =
                                            self.context.i64_type().const_int(elem_size, false);
                                        self.builder
                                            .build_memcpy(alloca_byte_ptr, 1, elem_ptr, 1, size_val)
                                            .map_err(|e| format!("memcpy: {e}"))?;
                                        self.store_var(name, alloca, payload_ty.clone());
                                        offset += elem_size;
                                    }
                                }
                            }
                        }
                        MatchPattern::Object(fields) => {
                            if let Type::Named(type_name) =
                                transparent_codegen_type(&resolved_subject_ty)
                                && let Some(enum_info) = self.enums.get(type_name).cloned()
                                && let Some(member_idx) = enum_info
                                    .members
                                    .iter()
                                    .position(|(name, _)| name == variant)
                                && let EnumPayload::Struct(payload_fields) =
                                    &enum_info.member_kinds[member_idx]
                            {
                                let byte_ptr_ty =
                                    self.context.ptr_type(inkwell::AddressSpace::default());
                                let payload_alloca = self
                                    .builder
                                    .build_alloca(
                                        self.llvm_type(&Type::Named(type_name.clone())),
                                        &format!("match.{enum_name}.{variant}.payload.alloca"),
                                    )
                                    .unwrap();
                                self.builder.build_store(payload_alloca, subject).unwrap();
                                let payload_ptr = self
                                    .builder
                                    .build_pointer_cast(
                                        payload_alloca,
                                        byte_ptr_ty,
                                        &format!("match.{enum_name}.{variant}.payload.ptr"),
                                    )
                                    .map_err(|e| format!("pointer cast: {e}"))?;
                                let mut offset = self
                                    .context
                                    .i8_type()
                                    .size_of()
                                    .get_zero_extended_constant()
                                    .unwrap_or(1);
                                for payload_field in payload_fields {
                                    let field_llvm_ty = self.llvm_type(&payload_field.ty);
                                    let field_size = self.type_layout(&payload_field.ty).0 as u64;
                                    if let Some(pattern_field) =
                                        fields.iter().find(|field| field.key == payload_field.name)
                                        && let Some(name) = &pattern_field.binding
                                    {
                                        let field_ptr = unsafe {
                                            self.builder
                                                .build_gep(
                                                    self.context.i8_type(),
                                                    payload_ptr,
                                                    &[self
                                                        .context
                                                        .i64_type()
                                                        .const_int(offset, false)],
                                                    &format!(
                                                        "match.{enum_name}.{variant}.{name}.ptr"
                                                    ),
                                                )
                                                .unwrap()
                                        };
                                        let alloca = self.entry_alloca(field_llvm_ty, name)?;
                                        let alloca_byte_ptr = self
                                            .builder
                                            .build_pointer_cast(
                                                alloca,
                                                byte_ptr_ty,
                                                &format!(
                                                    "match.{enum_name}.{variant}.{name}.alloca.ptr"
                                                ),
                                            )
                                            .map_err(|e| format!("pointer cast: {e}"))?;
                                        let size_val =
                                            self.context.i64_type().const_int(field_size, false);
                                        self.builder
                                            .build_memcpy(
                                                alloca_byte_ptr,
                                                1,
                                                field_ptr,
                                                1,
                                                size_val,
                                            )
                                            .map_err(|e| format!("memcpy: {e}"))?;
                                        self.store_var(name, alloca, payload_field.ty.clone());
                                    }
                                    offset += field_size;
                                }
                            }
                        }
                        _ => {
                            let binding_cond = self.emit_match_condition(
                                binding,
                                guard,
                                subject_expr,
                                subject,
                                is_string_match,
                                index,
                            )?;
                            condition = Some(match condition {
                                Some(c) => self
                                    .builder
                                    .build_and(
                                        c,
                                        binding_cond,
                                        &format!("match.{enum_name}.{variant}.combined"),
                                    )
                                    .unwrap(),
                                None => binding_cond,
                            });
                        }
                    }
                }

                match condition {
                    Some(c) => Ok(c),
                    None => Ok(self.context.bool_type().const_all_ones()),
                }
            }
            _ => unreachable!("internal: not an enum variant pattern"),
        }
    }
}
