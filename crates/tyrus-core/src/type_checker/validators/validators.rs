use super::*;

impl TypeChecker {
    pub(super) fn warn_validator_fact_condition(&mut self, op: &BinOp, left: &Expr, right: &Expr) {
        let Some((name, fact_kind, literal, op)) = fact_condition_parts(op, left, right) else {
            return;
        };
        let Some(facts) = self.lookup_validator_facts(&name) else {
            return;
        };
        let result = match fact_kind {
            FactKind::Value => facts.evaluate_value_condition(&op, &literal),
            FactKind::Length => facts.evaluate_length_condition(&op, &literal),
        };
        let Some(result) = result else {
            return;
        };
        let literal = crate::ast::expr_display(&literal);
        let condition = match fact_kind {
            FactKind::Value => format!("`{name} {} {literal}`", binop_display_name(&op)),
            FactKind::Length => format!("`{name}.length {} {literal}`", binop_display_name(&op)),
        };
        match result {
            FactCondition::AlwaysTrue => self.warning(
                "E3167",
                format!("{condition} is always true because of Validator Decorations"),
                Some(&name),
            ),
            FactCondition::AlwaysFalse => self.warning(
                "E3166",
                format!("{condition} is impossible because of Validator Decorations"),
                Some(&name),
            ),
        }
    }

    pub(super) fn declare_validator_facts(&mut self, name: &str, facts: ValidatorFacts) {
        if !facts.is_empty() {
            self.validator_facts
                .last_mut()
                .unwrap()
                .insert(name.to_string(), facts);
        }
    }

    pub(super) fn lookup_validator_facts(&self, name: &str) -> Option<&ValidatorFacts> {
        self.validator_facts
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .filter(|facts| !facts.is_empty())
    }

    pub(super) fn check_assigned_field_decorators(
        &mut self,
        object_ty: &ValueType,
        field: &str,
        value: &Expr,
    ) {
        let Some(fields) = self.decorated_object_fields_from_value(object_ty) else {
            return;
        };
        let Some(declared) = fields.iter().find(|candidate| candidate.name == field) else {
            return;
        };
        if declared.decorators.is_empty() {
            return;
        }
        let enum_literal = matches!(transparent_type(&declared.ty), Type::Named(enum_name) if self.enums.contains_key(enum_name))
            && matches!(value.kind(), ExprKind::FieldAccess { object, .. } if matches!(object.as_ref().kind(), ExprKind::Ident(name) if self.enums.contains_key(name)));
        let has_custom_validator = declared
            .decorators
            .iter()
            .any(|decorator| self.user_validators.contains_key(&decorator.name));
        if (!is_validator_literal(value) || has_custom_validator) && !enum_literal {
            self.check_unhandled_effect(
                &Type::Named("ValidationError".to_string()),
                &format!("assignment to validated field `{field}`"),
                value.span,
                true,
            );
        }
        for decorator in &declared.decorators.clone() {
            if let Some(message) = validate_literal_value(decorator, value, &declared.ty) {
                let validator = validator_display(decorator);
                self.error_at(
                    "E3165",
                    format!("`{validator}` failed for `{field}`: {message}"),
                    value.span,
                );
            }
        }
    }

    pub(super) fn decorated_object_fields_from_value(
        &self,
        object_ty: &ValueType,
    ) -> Option<Vec<Field>> {
        match object_ty {
            ValueType::Known(ty) => self.decorated_object_fields(ty),
            _ => None,
        }
    }

    pub(super) fn validate_object_literal_decorators(
        &mut self,
        expected: &Type,
        actual: &[(String, Expr)],
    ) {
        let Some(expected_fields) = self.decorated_object_fields(expected) else {
            return;
        };
        for field in expected_fields {
            if field.decorators.is_empty() {
                continue;
            }
            let Some((_, value)) = actual.iter().find(|(name, _)| name == &field.name) else {
                continue;
            };
            let enum_literal = matches!(transparent_type(&field.ty), Type::Named(enum_name) if self.enums.contains_key(enum_name))
                && matches!(value.kind(), ExprKind::FieldAccess { object, .. } if matches!(object.as_ref().kind(), ExprKind::Ident(name) if self.enums.contains_key(name)));
            let has_custom_validator = field
                .decorators
                .iter()
                .any(|decorator| self.user_validators.contains_key(&decorator.name));
            if (!is_validator_literal(value) || has_custom_validator) && !enum_literal {
                self.check_unhandled_effect(
                    &Type::Named("ValidationError".to_string()),
                    &format!("validation of field `{}`", field.name),
                    value.span,
                    true,
                );
            }
            for decorator in &field.decorators {
                if let Some(message) = validate_literal_value(decorator, value, &field.ty) {
                    let validator = validator_display(decorator);
                    self.error_at(
                        "E3165",
                        format!("`{validator}` failed for `{}`: {message}", field.name),
                        value.span,
                    );
                }
            }
        }
    }

    pub(super) fn decorated_object_fields(&self, ty: &Type) -> Option<Vec<Field>> {
        match transparent_type(ty) {
            Type::Named(name) => self.interfaces.get(name).cloned().or_else(|| {
                self.type_aliases
                    .get(name)
                    .and_then(|alias| match transparent_type(alias) {
                        Type::Object(fields) => Some(fields.clone()),
                        _ => None,
                    })
            }),
            Type::Object(fields) => Some(fields.clone()),
            _ => None,
        }
    }

    pub(super) fn validate_field_decorators(&mut self, fields: &[Field]) {
        for field in fields {
            self.validate_decorators_for_target(&field.name, &field.decorators, &field.ty);
        }
    }

    pub(super) fn validate_param_decorators(&mut self, params: &[Param]) {
        for param in params {
            let name = param
                .pattern
                .binding_names()
                .first()
                .copied()
                .unwrap_or("parameter")
                .to_string();
            self.validate_decorators_for_target(&name, &param.decorators, &param.ty);
        }
    }

    pub(super) fn validate_decorators_for_target(
        &mut self,
        target: &str,
        decorators: &[ValidatorDecorator],
        ty: &Type,
    ) {
        for (index, decorator) in decorators.iter().enumerate() {
            if let Some(Type::Function { params, ret }) =
                self.user_validators.get(&decorator.name).cloned()
            {
                self.validate_user_validator_application(target, decorator, ty, &params, &ret);
                continue;
            }
            let occurrence = decorators[..=index]
                .iter()
                .filter(|other| other.name == decorator.name)
                .count();
            let validation_type = if matches!(decorator.name.as_str(), "in" | "notIn") {
                match ty {
                    Type::Named(name) => self
                        .enums
                        .get(name)
                        .map(|resolved| &resolved.repr)
                        .unwrap_or(ty),
                    _ => ty,
                }
            } else {
                ty
            };
            for mut diagnostic in validate_decorator(decorator, validation_type) {
                diagnostic.subject =
                    Some(format!("{target}\u{1f}@{}#{occurrence}", decorator.name));
                self.push_validator_diagnostic(diagnostic);
            }
            self.check_unique_element_is_comparable(decorator, ty);
        }
        for mut diagnostic in validate_decorator_set(decorators) {
            if let Some(subject) = diagnostic.subject.take() {
                diagnostic.subject = Some(format!("{target}\u{1f}{subject}"));
            }
            self.push_validator_diagnostic(diagnostic);
        }
    }

    pub(super) fn validate_user_validator_signature(
        &mut self,
        name: &str,
        is_async: bool,
        is_comptime: bool,
        type_params: &[crate::ast::TypeParam],
        params: &[Param],
        ret_type: &Type,
        throws: &[Type],
        span: Span,
    ) {
        let resolved_return = self.resolve_type_annotation(ret_type);
        let issue = if is_async {
            Some((span, "must be synchronous"))
        } else if is_comptime {
            Some((span, "cannot be a compile-time function"))
        } else if !type_params.is_empty() {
            Some((span, "cannot declare type parameters in the first version"))
        } else if params.is_empty() {
            Some((span, "must declare the decorated value parameter"))
        } else if matches!(params[0].ty, Type::Infer) {
            Some((
                params[0].span,
                "must declare an explicit type for its value parameter",
            ))
        } else if params[0].optional || params[0].rest || params[0].default_value.is_some() {
            Some((
                params[0].span,
                "must take the decorated value as a required parameter",
            ))
        } else if params.iter().any(|param| {
            matches!(param.ty, Type::Infer)
                || param.rest
                || !param.decorators.is_empty()
                || param.optional
                || param.default_value.is_some()
        }) {
            Some((
                span,
                "must use explicitly typed, non-rest configuration parameters",
            ))
        } else if !Self::is_validator_result_type(&resolved_return) {
            Some((span, "must return `Validator`"))
        } else if !throws.is_empty() {
            Some((span, "cannot declare `throws` effects"))
        } else {
            None
        };
        if let Some((span, reason)) = issue {
            self.error_at_with_help(
                "E3168",
                format!("validator function `{name}` {reason}"),
                span,
                vec![format!("declare `validator function {name}(value: <type>, ...): Validator` with an explicitly typed value parameter and no effects")],
            );
        }
    }

    fn validate_user_validator_application(
        &mut self,
        target: &str,
        decorator: &ValidatorDecorator,
        target_ty: &Type,
        params: &[Type],
        ret: &Type,
    ) {
        if params.is_empty() || !Self::is_validator_result_type(&self.resolve_type_annotation(ret))
        {
            return;
        }
        let config_params = &params[1..];
        if decorator.args.len() != config_params.len() {
            self.error_at_with_help(
                "E3169",
                format!(
                    "custom validator `@{}` expects {} configuration argument(s), found {}",
                    decorator.display_name,
                    config_params.len(),
                    decorator.args.len()
                ),
                decorator.span,
                vec![format!(
                    "write `@{}(...)` with {} argument(s)",
                    decorator.display_name,
                    config_params.len()
                )],
            );
            return;
        }
        for (index, argument) in decorator.args.iter().enumerate() {
            if !is_validator_literal(argument) {
                self.error_at_with_help(
                    "E3169",
                    format!(
                        "configuration argument {} of `@{}` must be a literal",
                        index + 1,
                        decorator.display_name
                    ),
                    argument.span,
                    vec!["use a literal value in the validator decorator".to_string()],
                );
                continue;
            }
            let actual = self.check_expr(argument);
            let expected = self.resolve_type_annotation(&config_params[index]);
            if !self.is_assignable(&expected, &actual) {
                self.error_at_with_help(
                    "E3169",
                    format!(
                        "configuration argument {} of `@{}` expects `{expected}`, found `{}`",
                        index + 1,
                        decorator.display_name,
                        actual.display()
                    ),
                    argument.span,
                    vec![format!("pass a literal assignable to `{expected}`")],
                );
            }
        }
        let resolved_target = self.resolve_type_query(target_ty);
        let value_ty = match resolved_target {
            Type::Optional(inner) => *inner,
            other => other,
        };
        let value_param = self.resolve_type_annotation(&params[0]);
        if !self.is_assignable(&value_param, &ValueType::Known(value_ty.clone())) {
            self.error_at_with_help(
                "E3169",
                format!("custom validator `@{}` expects `{}`, but `{target}` has type `{value_ty}`", decorator.display_name, value_param),
                decorator.span,
                vec![format!("change the first parameter of `{}` to accept `{value_ty}`, or use a compatible validator", decorator.display_name)],
            );
        }
    }

    pub(super) fn push_validator_diagnostic(
        &mut self,
        diagnostic: crate::validators::ValidatorDiagnostic,
    ) {
        self.diagnostics.push(CheckDiagnostic {
            code: diagnostic.code,
            severity: match diagnostic.severity {
                ValidatorDiagnosticSeverity::Error => CheckSeverity::Error,
                ValidatorDiagnosticSeverity::Warning => CheckSeverity::Warning,
            },
            message: diagnostic.message,
            span: (!diagnostic.span.is_dummy()).then_some(diagnostic.span),
            subject: diagnostic.subject,
            help: Vec::new(),
            prefer_declaration: false,
        });
    }

    fn is_validator_result_type(ty: &Type) -> bool {
        let Type::Union(variants) = ty else {
            return false;
        };
        if variants.len() != 2 {
            return false;
        }
        let mut has_valid = false;
        let mut has_invalid = false;
        for variant in variants {
            let Type::Object(fields) = variant else {
                return false;
            };
            let Some(is_valid) = fields.iter().find(|field| field.name == "isValid") else {
                return false;
            };
            let valid = matches!(
                &is_valid.ty,
                Type::Lit(value) if matches!(value.kind(), ExprKind::BoolLit(true))
            );
            let invalid = matches!(
                &is_valid.ty,
                Type::Lit(value) if matches!(value.kind(), ExprKind::BoolLit(false))
            );
            if valid && fields.len() == 1 {
                has_valid = true;
            } else if invalid
                && fields.len() == 2
                && fields
                    .iter()
                    .any(|field| field.name == "message" && matches!(field.ty, Type::Str))
            {
                has_invalid = true;
            } else {
                return false;
            }
        }
        has_valid && has_invalid
    }
}
