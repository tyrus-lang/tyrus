use super::*;

impl TypeChecker {
    pub(super) fn check_access_expr(&mut self, expr: &Expr) -> ValueType {
        match expr.kind() {
            ExprKind::FieldAccess { object, field } => {
                if matches!(object.as_ref().kind(), ExprKind::Ident(name) if name == "process")
                    && !self.has_symbol("process")
                {
                    if field == "argv" {
                        return ValueType::Known(Type::Array(Box::new(Type::Str)));
                    }
                    if field == "pid" || field == "ppid" {
                        return ValueType::Known(Type::U32);
                    }
                    if field != "env" {
                        self.error(
                            "E3145",
                            format!("`process` has no member `{field}`; only `process.env`, `process.argv`, `process.pid`, and `process.ppid` are available"),
                            Some(field),
                        );
                        return ValueType::Error;
                    }
                    return match &self.env_fields {
                        Some(fields) => ValueType::Known(Type::Object(fields.clone())),
                        None => {
                            self.error(
                                "E3140",
                                "`process.env` requires an env file configured in tyrus.json under `security.envFile`"
                                    .to_string(),
                                None,
                            );
                            ValueType::Error
                        }
                    };
                }
                if self.is_process_env(object) {
                    let object_ty = self.check_expr(object);
                    if let ValueType::Known(Type::Object(fields)) = &object_ty {
                        if let Some(field_ty) = fields
                            .iter()
                            .find(|candidate| candidate.name == *field)
                            .map(field_value_type)
                        {
                            return ValueType::Known(field_ty);
                        }
                        self.error(
                            "E3141",
                            format!(
                                "environment variable `{field}` is not declared in the env file"
                            ),
                            Some(field),
                        );
                    }
                    return ValueType::Error;
                }
                if let ExprKind::Ident(type_name) = object.as_ref().kind() {
                    if let Some(ty) = numeric_constant_type(type_name, field) {
                        return ValueType::Known(ty);
                    }
                    if matches!(type_name.as_str(), "isize" | "usize")
                        && matches!(field.as_str(), "MIN" | "MAX")
                    {
                        self.error(
                            "E0707",
                            format!(
                                "`{type_name}` has no `{field}`: it is as wide as a pointer on the machine the program is built for, so its bounds are not fixed by the language"
                            ),
                            Some(type_name),
                        );
                        return ValueType::Error;
                    }
                    if self.enums.get(type_name).is_some_and(|enum_info| {
                        enum_info.members.iter().any(|(member, _)| member == field)
                    }) {
                        self.mark_used(type_name);
                        return ValueType::Known(Type::Named(type_name.clone()));
                    }
                    if self.enums.contains_key(type_name) {
                        self.error(
                            "E0609",
                            format!("no member `{field}` on enum `{type_name}`"),
                            Some(field),
                        );
                        return ValueType::Error;
                    }
                }
                let object_ty = self.check_expr(object);
                let field_ty = self.check_field_on_value(object_ty, field);
                match (&field_ty, expression_path(expr)) {
                    (ValueType::Known(Type::Optional(inner)), Some(path))
                        if self.path_is_narrowed(&path) =>
                    {
                        ValueType::Known(inner.as_ref().clone())
                    }
                    _ => field_ty,
                }
            }
            ExprKind::OptionalFieldAccess { object, field } => {
                let object_ty = self.check_expr(object);
                if matches!(object_ty, ValueType::Error) {
                    return ValueType::Error;
                }
                let Some(Type::Optional(inner)) = concrete_value_type(&object_ty) else {
                    self.error(
                        "E0277",
                        format!(
                            "optional chaining with `?.` requires an optional receiver, found `{}`",
                            object_ty.display()
                        ),
                        expression_subject(object),
                    );
                    return ValueType::Error;
                };
                match self.check_field_on_value(ValueType::Known(*inner), field) {
                    ValueType::Known(Type::Optional(field_ty)) => {
                        ValueType::Known(Type::Optional(field_ty))
                    }
                    ValueType::Known(field_ty) => {
                        ValueType::Known(Type::Optional(Box::new(field_ty)))
                    }
                    other => other,
                }
            }
            ExprKind::IndexAccess { object, index } => {
                if self.is_process_env(object) {
                    let suggestion = match index.as_ref().kind() {
                        ExprKind::StrLit(name) => format!("process.env.{name}"),
                        _ => "process.env.<NAME>".to_string(),
                    };
                    self.error(
                        "E3144",
                        format!("`process.env` cannot be indexed; use `{suggestion}`"),
                        None,
                    );
                    return ValueType::Error;
                }
                let object_ty = self.check_expr(object);
                if let ExprKind::StrLit(field) = index.kind()
                    && self.is_struct_like(&object_ty)
                {
                    return self.check_field_on_value(object_ty, field);
                }
                if let ValueType::Known(Type::Map(key, value)) = &object_ty {
                    let key = key.as_ref().clone();
                    let value = value.as_ref().clone();
                    let index_ty = self.check_expr_as(index, &key);
                    self.require_assignable(&key, &index_ty, "index", "E0308");
                    return ValueType::Known(Type::Optional(Box::new(value)));
                }
                let index_ty = self.check_expr(index);
                if !matches!(index_ty, ValueType::IntegerLiteral)
                    && !matches!(
                        index_ty,
                        ValueType::Known(ref ty) if is_integer_type(transparent_type(ty))
                    )
                    && !matches!(index_ty, ValueType::Error | ValueType::Unknown)
                {
                    self.error(
                        "E0277",
                        format!(
                            "array index must be an integer, found `{}`",
                            index_ty.display()
                        ),
                        expression_subject(index),
                    );
                }
                if let Some(len) = self.known_array_len(object, &object_ty) {
                    self.check_constant_index_bounds(index, len, expression_subject(object));
                }
                match object_ty {
                    ValueType::Known(Type::Tuple(elements)) => {
                        tuple_index_type(&elements, index, self)
                    }
                    ValueType::Known(Type::Array(element)) => ValueType::Known(*element),
                    ValueType::Known(Type::Readonly(inner)) => match *inner {
                        Type::Tuple(elements) => match tuple_index_type(&elements, index, self) {
                            ValueType::Known(ty) => ValueType::Known(deep_readonly(ty)),
                            other => other,
                        },
                        Type::Array(element) => ValueType::Known(deep_readonly(*element)),
                        other => {
                            self.error(
                                "E0608",
                                format!("cannot index into type `{other}`"),
                                expression_subject(object),
                            );
                            ValueType::Error
                        }
                    },
                    ValueType::Known(Type::Ref(inner) | Type::MutRef(inner)) => match *inner {
                        Type::Array(element) => ValueType::Known(*element),
                        Type::Tuple(elements) => tuple_index_type(&elements, index, self),
                        other => {
                            self.error(
                                "E0608",
                                format!("cannot index into type `{other}`"),
                                expression_subject(object),
                            );
                            ValueType::Error
                        }
                    },
                    ValueType::Array(values) => infer_array_element_type(&values)
                        .map(ValueType::Known)
                        .unwrap_or(ValueType::Unknown),
                    ValueType::Error => ValueType::Error,
                    other => {
                        self.error(
                            "E0608",
                            format!("cannot index into type `{}`", other.display()),
                            expression_subject(object),
                        );
                        ValueType::Error
                    }
                }
            }
            _ => unreachable!("internal: not a field or index access"),
        }
    }
}
