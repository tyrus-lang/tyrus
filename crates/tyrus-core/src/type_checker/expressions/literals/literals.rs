use super::*;

impl TypeChecker {
    pub(super) fn check_literal_expr(&mut self, expr: &Expr, expected: Option<&Type>) -> ValueType {
        match expr.kind() {
            ExprKind::StrLit(_) | ExprKind::TemplateLit(_) => {
                if let ExprKind::TemplateLit(parts) = expr.kind() {
                    for part in parts {
                        if let TemplatePart::Expr(expr, spec) = part {
                            let ty = self.check_expr(expr);
                            if let Some(spec) = spec {
                                let matches_spec = matches!(&ty, ValueType::Known(known) if match spec {
                                    FormatSpec::FixedPoint(_) => is_float_type(transparent_type(known)),
                                    FormatSpec::ZeroPadded(_) => is_integer_type(transparent_type(known)),
                                });
                                if !matches_spec {
                                    let kind = match spec {
                                        FormatSpec::FixedPoint(_) => "a float",
                                        FormatSpec::ZeroPadded(_) => "an integer",
                                    };
                                    self.error(
                                        "E0717",
                                        format!(
                                            "format specifier `:{}` requires {kind}, found `{}`",
                                            spec.as_written(),
                                            ty.display()
                                        ),
                                        expression_subject(expr),
                                    );
                                }
                            } else if !is_template_value(&ty) {
                                self.error(
                                    "E0277",
                                    format!(
                                        "type `{}` cannot be interpolated into a string",
                                        ty.display()
                                    ),
                                    expression_subject(expr),
                                );
                            }
                        }
                    }
                }
                ValueType::Known(Type::Str)
            }
            ExprKind::RegexLit { flags, .. } => {
                if let Err(message) = regex_runtime_pattern_with_context("", flags, "Regex") {
                    self.error("E0309", message, None);
                }
                ValueType::Known(Type::Regex)
            }
            ExprKind::NewSet { ty, initial } => {
                self.check_new_set(ty.as_ref(), initial.as_deref(), expected)
            }
            ExprKind::NewMap {
                key,
                value,
                initial,
            } => self.check_new_map(key.as_ref(), value.as_ref(), initial.as_deref(), expected),
            ExprKind::IntLit(value) if *value < 0 => ValueType::NegativeIntegerLiteral,
            ExprKind::IntLit(_) => ValueType::IntegerLiteral,
            ExprKind::UIntLit(_) => ValueType::UnsignedIntegerLiteral,
            ExprKind::FloatLit(_) => ValueType::FloatLiteral,
            ExprKind::DecimalLit(_) => ValueType::Known(Type::Decimal),
            ExprKind::BoolLit(_) => ValueType::Known(Type::Bool),
            ExprKind::Null => {
                self.error(
                    "E3080",
                    "`null` is not a value in tyrus; use `T?` for nullable types and omit optional values instead".to_string(),
                    Some("null"),
                );
                ValueType::Error
            }
            ExprKind::None => ValueType::Null,
            ExprKind::ObjectLit(fields) => ValueType::Object(
                fields
                    .iter()
                    .map(|(name, value)| (name.clone(), self.check_object_field_expr(value)))
                    .collect(),
            ),
            ExprKind::ObjectUpdate { base, fields } => {
                let base_ty = self.check_expr(base);
                let Some(mut object_fields) = self.resolve_value_object_fields(&base_ty) else {
                    self.error(
                        "E0308",
                        format!(
                            "object update requires an object, found `{}`",
                            base_ty.display()
                        ),
                        expression_subject(base),
                    );
                    for (_, value) in fields {
                        self.check_expr(value);
                    }
                    return ValueType::Error;
                };

                for (name, value) in fields {
                    let Some(index) = object_fields
                        .iter()
                        .position(|(candidate, _)| candidate == name)
                    else {
                        self.error(
                            "E0609",
                            format!("no field `{name}` in object update"),
                            Some(name),
                        );
                        self.check_expr(value);
                        continue;
                    };
                    let actual = self.check_expr(value);
                    let expected = value_type_to_type(&object_fields[index].1);
                    if let Some(expected) = expected {
                        self.require_assignable(&expected, &actual, name, "E0308");
                        self.require_integer_literal_fits(&expected, value);
                    }
                    object_fields[index].1 = actual;
                }

                if let ValueType::Known(base_type) = &base_ty {
                    self.validate_object_literal_decorators(base_type, fields);
                }

                match base_ty {
                    ValueType::Known(_) => base_ty,
                    _ => ValueType::Object(object_fields),
                }
            }
            ExprKind::ArrayLit(values) => {
                let mut element_types = vec![];
                for value in values {
                    match value.kind() {
                        ExprKind::Spread(inner) => {
                            let inner_ty = self.check_expr(inner);
                            let spread_elem = match &inner_ty {
                                ValueType::Array(elems) if !elems.is_empty() => elems[0].clone(),
                                ValueType::Array(_) => ValueType::Unknown,
                                _ => {
                                    if let Some(Type::Array(elem_ty)) =
                                        transparent_value_type(&inner_ty)
                                    {
                                        ValueType::Known(*elem_ty.clone())
                                    } else if matches!(inner_ty, ValueType::Unknown) {
                                        ValueType::Unknown
                                    } else {
                                        self.error(
                                            "E0308",
                                            format!(
                                                "Spread requires an array, found `{}`",
                                                inner_ty.display()
                                            ),
                                            expression_subject(inner),
                                        );
                                        ValueType::Known(Type::I64)
                                    }
                                }
                            };
                            element_types.push(spread_elem);
                        }
                        _ => {
                            element_types.push(self.check_expr(value));
                        }
                    }
                }
                ValueType::Array(element_types)
            }
            ExprKind::ArrayRepeat { value, count } => {
                let repeat_count = match integer_constant_value(count) {
                    None => {
                        self.error(
                            "E0308",
                            "Array repeat count must be a constant integer".to_string(),
                            expression_subject(count),
                        );
                        None
                    }
                    Some(Err(())) => {
                        self.error(
                            "E0308",
                            "Array repeat count overflows".to_string(),
                            expression_subject(count),
                        );
                        None
                    }
                    Some(Ok(value)) if !(1..=MAX_ARRAY_REPEAT_COUNT as i128).contains(&value) => {
                        self.error(
                            "E0308",
                            format!(
                                "Array repeat count must be between 1 and {MAX_ARRAY_REPEAT_COUNT}"
                            ),
                            expression_subject(count),
                        );
                        None
                    }
                    Some(Ok(value)) => usize::try_from(value).ok(),
                };
                let element_type = self.check_expr(value);
                ValueType::Array(vec![element_type; repeat_count.unwrap_or(1)])
            }
            _ => unreachable!("internal: not a literal expression"),
        }
    }
}
