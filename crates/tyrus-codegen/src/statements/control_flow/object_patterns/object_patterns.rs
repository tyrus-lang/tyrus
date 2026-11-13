use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_object_pattern_condition(
        &mut self,
        pattern: &MatchPattern,
        guard: &Option<Expr>,
        subject_expr: &Expr,
        subject: BasicValueEnum<'ctx>,
        index: usize,
    ) -> Result<IntValue<'ctx>, String> {
        match pattern {
            MatchPattern::Object(fields) => {
                let subject_type = self.expr_type_hint(subject_expr).ok_or_else(|| {
                    "error: cannot infer type for match object pattern".to_string()
                })?;

                let resolved_subject_type = self.resolve_type(&subject_type);
                let mut condition: Option<IntValue<'ctx>> = None;
                let (subject_struct, struct_fields): (_, Vec<(String, Type)>) =
                    match transparent_codegen_type(&resolved_subject_type) {
                        Type::Union(variants) => {
                            let variant = fields
                                .iter()
                                .filter_map(|field| {
                                    field.literal.as_ref().map(|lit| (&field.key, lit))
                                })
                                .find_map(|(field, lit)| {
                                    variants.iter().find(|variant| {
                                        Self::variant_discriminant_matches(variant, field, lit)
                                    })
                                })
                                .ok_or_else(|| {
                                    "error: match object pattern over a union requires a literal discriminant"
                                        .to_string()
                                })?;
                            let union_value = subject.into_struct_value();
                            let tag = self
                                .builder
                                .build_extract_value(union_value, 0, "match.union.tag")
                                .map_err(|error| error.to_string())?
                                .into_int_value();
                            let expected_tag = self.context.i8_type().const_int(
                                Self::union_variant_index(variants, variant) as u64,
                                false,
                            );
                            condition = Some(
                                self.builder
                                    .build_int_compare(
                                        IntPredicate::EQ,
                                        tag,
                                        expected_tag,
                                        "match.union.tag.cmp",
                                    )
                                    .unwrap(),
                            );
                            let union_ty = self.union_llvm_type(variants);
                            let union_slot =
                                self.builder.build_alloca(union_ty, "match.union").unwrap();
                            self.builder.build_store(union_slot, subject).unwrap();
                            let payload = self.load_union_payload(
                                variants,
                                union_slot,
                                variant,
                                "match.union.payload",
                            )?;
                            let Type::Object(variant_fields) = transparent_codegen_type(variant)
                            else {
                                return Err(
                                    "error: match object pattern requires an object union variant"
                                        .to_string(),
                                );
                            };
                            (
                                payload.into_struct_value(),
                                variant_fields
                                    .iter()
                                    .map(|field| (field.name.clone(), field_storage_type(field)))
                                    .collect(),
                            )
                        }
                        Type::Named(name) => {
                            let fields = self.interfaces.get(name).ok_or_else(|| {
                                format!("error: unknown interface `{name}` for match pattern")
                            })?;
                            (
                                subject.into_struct_value(),
                                fields
                                    .iter()
                                    .map(|f| (f.name.clone(), field_storage_type(f)))
                                    .collect(),
                            )
                        }
                        Type::Object(fs) => (
                            subject.into_struct_value(),
                            fs.iter()
                                .map(|f| (f.name.clone(), field_storage_type(f)))
                                .collect(),
                        ),
                        _ => {
                            return Err(
                                "error: match object pattern requires an object or interface type"
                                    .to_string(),
                            );
                        }
                    };

                for field in fields {
                    if let Some(lit) = &field.literal {
                        let field_index = struct_fields
                            .iter()
                            .position(|(name, _)| name == &field.key)
                            .ok_or_else(|| {
                                format!(
                                    "error: field `{}` not found in match subject type",
                                    field.key
                                )
                            })?;
                        let field_ty = &struct_fields[field_index].1;
                        let field_val = self
                            .builder
                            .build_extract_value(subject_struct, field_index as u32, &field.key)
                            .unwrap();
                        let lit_val = self.emit_expr(lit)?;
                        let comparison_ty = match field_ty {
                            Type::Lit(expr) => self.lit_underlying_type(expr),
                            ty => ty.clone(),
                        };
                        let is_string_field = matches!(comparison_ty, Type::Str);
                        let eq = if is_string_field {
                            self.emit_string_equality(
                                field_val.into_struct_value(),
                                lit_val.into_struct_value(),
                                true,
                            )?
                            .into_int_value()
                        } else {
                            self.builder
                                .build_int_compare(
                                    IntPredicate::EQ,
                                    field_val.into_int_value(),
                                    lit_val.into_int_value(),
                                    &format!("match.field.cmp.{}", field.key),
                                )
                                .unwrap()
                        };
                        condition = Some(match condition {
                            Some(prev) => self
                                .builder
                                .build_and(prev, eq, &format!("match.field.and.{}", field.key))
                                .unwrap(),
                            None => eq,
                        });
                    }
                }

                for field in fields {
                    if let Some(binding) = &field.binding {
                        let field_index = struct_fields
                            .iter()
                            .position(|(name, _)| name == &field.key)
                            .ok_or_else(|| {
                                format!(
                                    "error: field `{}` not found in match subject type",
                                    field.key
                                )
                            })?;
                        let field_ty = &struct_fields[field_index].1;
                        let field_val = self
                            .builder
                            .build_extract_value(subject_struct, field_index as u32, &field.key)
                            .unwrap();
                        let llvm_ty = self.llvm_type(field_ty);
                        let alloca = self.builder.build_alloca(llvm_ty, binding).unwrap();
                        self.builder.build_store(alloca, field_val).unwrap();
                        self.store_var(binding, alloca, field_ty.clone());
                    }
                }

                if let Some(guard_expr) = guard {
                    let guard_val = self.emit_expr(guard_expr)?.into_int_value();
                    Ok(match condition {
                        Some(cond) => self
                            .builder
                            .build_and(cond, guard_val, &format!("match.guard.{index}"))
                            .unwrap(),
                        None => guard_val,
                    })
                } else {
                    Ok(condition.unwrap_or_else(|| self.context.bool_type().const_all_ones()))
                }
            }
            _ => unreachable!("internal: not an object pattern"),
        }
    }
}
