use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_literal_pattern_condition(
        &mut self,
        pattern: &MatchPattern,
        guard: &Option<Expr>,
        subject_expr: &Expr,
        subject: BasicValueEnum<'ctx>,
        is_string_match: bool,
        index: usize,
    ) -> Result<IntValue<'ctx>, String> {
        match pattern {
            MatchPattern::Lit(lit) => {
                let subject_ty = self
                    .expr_type_hint(subject_expr)
                    .map(|ty| self.resolve_type(&ty));
                let is_optional_none_pattern =
                    matches!(lit.kind(), ExprKind::Null | ExprKind::None)
                        && subject_ty.as_ref().is_some_and(|ty| {
                            matches!(transparent_codegen_type(ty), Type::Optional(_))
                        });
                let union_variants =
                    subject_ty
                        .as_ref()
                        .and_then(|ty| match transparent_codegen_type(ty) {
                            Type::Union(variants) => Some(variants),
                            _ => None,
                        });
                let eq = if is_optional_none_pattern {
                    self.builder
                        .build_is_null(subject.into_pointer_value(), &format!("match.none.{index}"))
                        .unwrap()
                } else if let Some(variants) = union_variants {
                    let wanted = variants
                        .iter()
                        .position(|variant| {
                            matches!(variant, Type::Lit(value) if value.kind() == lit.kind())
                        })
                        .ok_or_else(|| {
                            format!(
                                "internal: `{:?}` is not a variant of the union being matched",
                                lit.kind()
                            )
                        })?;
                    let tag = self
                        .builder
                        .build_extract_value(
                            subject.into_struct_value(),
                            0,
                            &format!("match.union.tag.{index}"),
                        )
                        .map_err(|error| error.to_string())?
                        .into_int_value();
                    self.builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            tag,
                            self.context.i8_type().const_int(wanted as u64, false),
                            &format!("match.union.is.{index}"),
                        )
                        .map_err(|error| error.to_string())?
                } else if is_string_match {
                    let case_val = self.emit_expr(lit)?;
                    self.emit_string_equality(
                        subject.into_struct_value(),
                        case_val.into_struct_value(),
                        true,
                    )?
                    .into_int_value()
                } else {
                    let case_val = self.emit_expr(lit)?;
                    self.builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            subject.into_int_value(),
                            case_val.into_int_value(),
                            &format!("match.cmp.{index}"),
                        )
                        .unwrap()
                };
                if let Some(guard_expr) = guard {
                    let guard_val = self.emit_expr(guard_expr)?.into_int_value();
                    Ok(self
                        .builder
                        .build_and(eq, guard_val, &format!("match.guard.{index}"))
                        .unwrap())
                } else {
                    Ok(eq)
                }
            }
            MatchPattern::Wildcard => {
                if let Some(guard_expr) = guard {
                    let guard_val = self.emit_expr(guard_expr)?.into_int_value();
                    Ok(guard_val)
                } else {
                    Ok(self.context.bool_type().const_all_ones())
                }
            }
            MatchPattern::ResultVariant {
                success,
                binding: Some(name),
            } if name == "_" => {
                let tag = self
                    .builder
                    .build_extract_value(subject.into_struct_value(), 0, "match.result.tag")
                    .unwrap()
                    .into_int_value();
                let matches_variant = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        tag,
                        self.context
                            .i8_type()
                            .const_int(usize::from(!success) as u64, false),
                        "match.result.variant",
                    )
                    .unwrap();
                if let Some(guard_expr) = guard {
                    let guard_val = self.emit_expr(guard_expr)?.into_int_value();
                    Ok(self
                        .builder
                        .build_and(matches_variant, guard_val, "match.result.guard")
                        .unwrap())
                } else {
                    Ok(matches_variant)
                }
            }
            MatchPattern::Binding(name)
            | MatchPattern::ResultVariant {
                binding: Some(name),
                ..
            } => {
                if matches!(pattern, MatchPattern::ResultVariant { .. })
                    && let Some(bindings) = self.match_result_bindings.last_mut()
                {
                    bindings.insert(name.clone());
                }
                let subject_type = self.expr_type_hint(subject_expr).ok_or_else(|| {
                    format!("error: cannot infer type for match binding `{name}`")
                })?;
                let resolved_subject_ty = self.resolve_type(&subject_type);
                let (resolved_ty, bound_value) =
                    match transparent_codegen_type(&resolved_subject_ty) {
                        Type::Optional(inner) => {
                            let payload_ty = inner.as_ref().clone();
                            let payload = self
                                .builder
                                .build_load(
                                    self.llvm_type(&payload_ty),
                                    subject.into_pointer_value(),
                                    &format!("match.binding.{name}.payload"),
                                )
                                .unwrap();
                            (payload_ty, Some(payload))
                        }
                        Type::Result { success, error } => {
                            let variants = vec![success.as_ref().clone(), error.as_ref().clone()];
                            let variant_index = match pattern {
                                MatchPattern::ResultVariant { success, .. } => {
                                    usize::from(!success)
                                }
                                _ => index.min(1),
                            };
                            let payload_ty = variants[variant_index].clone();
                            if matches!(transparent_codegen_type(&payload_ty), Type::Void) {
                                (payload_ty, None)
                            } else {
                                let slot = self.native_value_slot(
                                    self.llvm_type(&resolved_subject_ty),
                                    subject,
                                    &format!("match.binding.{name}.result"),
                                )?;
                                let payload = self.load_union_payload(
                                    &variants,
                                    slot,
                                    &payload_ty,
                                    &format!("match.binding.{name}.payload"),
                                )?;
                                (payload_ty, Some(payload))
                            }
                        }
                        _ => (resolved_subject_ty.clone(), Some(subject)),
                    };
                if let Some(bound_value) = bound_value {
                    let llvm_ty = self.llvm_type(&resolved_ty);
                    let alloca = self.entry_alloca(llvm_ty, name)?;
                    self.builder.build_store(alloca, bound_value).unwrap();
                    self.store_var(name, alloca, resolved_ty);
                }

                if let Some(guard_expr) = guard {
                    let guard_val = self.emit_expr(guard_expr)?.into_int_value();
                    if matches!(
                        transparent_codegen_type(&resolved_subject_ty),
                        Type::Result { .. }
                    ) {
                        let tag = self
                            .builder
                            .build_extract_value(subject.into_struct_value(), 0, "match.result.tag")
                            .unwrap()
                            .into_int_value();
                        let variant_index = match pattern {
                            MatchPattern::ResultVariant { success, .. } => usize::from(!success),
                            _ => index.min(1),
                        };
                        let variant = self
                            .context
                            .i8_type()
                            .const_int(variant_index as u64, false);
                        let matches_variant = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                tag,
                                variant,
                                "match.result.variant",
                            )
                            .unwrap();
                        Ok(self
                            .builder
                            .build_and(matches_variant, guard_val, "match.result.guard")
                            .unwrap())
                    } else {
                        Ok(guard_val)
                    }
                } else {
                    if matches!(
                        transparent_codegen_type(&resolved_subject_ty),
                        Type::Result { .. }
                    ) {
                        let tag = self
                            .builder
                            .build_extract_value(subject.into_struct_value(), 0, "match.result.tag")
                            .unwrap()
                            .into_int_value();
                        Ok(self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                tag,
                                self.context.i8_type().const_int(
                                    match pattern {
                                        MatchPattern::ResultVariant { success, .. } => {
                                            usize::from(!success)
                                        }
                                        _ => index.min(1),
                                    } as u64,
                                    false,
                                ),
                                "match.result.variant",
                            )
                            .unwrap())
                    } else {
                        Ok(self.context.bool_type().const_all_ones())
                    }
                }
            }
            MatchPattern::ResultVariant {
                success,
                binding: None,
            } => {
                let tag = self
                    .builder
                    .build_extract_value(subject.into_struct_value(), 0, "match.result.tag")
                    .unwrap()
                    .into_int_value();
                let matches_variant = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        tag,
                        self.context
                            .i8_type()
                            .const_int(usize::from(!success) as u64, false),
                        "match.result.variant",
                    )
                    .unwrap();
                if let Some(guard_expr) = guard {
                    let guard_val = self.emit_expr(guard_expr)?.into_int_value();
                    Ok(self
                        .builder
                        .build_and(matches_variant, guard_val, "match.result.guard")
                        .unwrap())
                } else {
                    Ok(matches_variant)
                }
            }
            _ => unreachable!("internal: not a literal, wildcard or binding pattern"),
        }
    }
}
