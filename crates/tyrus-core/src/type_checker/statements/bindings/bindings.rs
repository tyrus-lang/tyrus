use super::*;

impl TypeChecker {
    #[allow(clippy::too_many_lines)]
    pub(super) fn check_binding_stmt(&mut self, statement: &Stmt) {
        match statement.kind() {
            StmtKind::VarDecl {
                mutable,
                name,
                ty,
                value,
                onfail,
            } => {
                let index_hint = self.for_index_hint.take();
                self.reject_bare_literal_annotation(ty, Some(name));
                let anchor = self.current_span().unwrap_or(statement.span);
                self.check_type_name(ty, anchor);
                let Some(value) = value else {
                    let resolved_ty = match ty {
                        Type::Named(alias) => {
                            self.type_aliases.get(alias).cloned().unwrap_or(ty.clone())
                        }
                        _ => ty.clone(),
                    };
                    self.resolved_bindings.push(ResolvedBinding {
                        name: name.clone(),
                        span: statement.span,
                        ty: resolved_ty.clone(),
                    });
                    self.declare(
                        name,
                        Symbol {
                            ty: ValueType::known(&resolved_ty),
                            mutable: *mutable,
                            kind: SymbolKind::Variable,
                            used: false,
                            array_len: None,
                        },
                    );
                    self.record_constant_integer(name, *mutable, None);
                    return;
                };
                let expected_ty = (!matches!(ty, Type::Infer))
                    .then(|| self.resolve_type_annotation(ty))
                    .or(index_hint);
                let onfail_exception = onfail
                    .as_ref()
                    .and_then(|_| self.direct_onfail_exception(value));
                if let Some(exception) = &onfail_exception {
                    self.try_catch_type.push(exception.clone());
                    self.try_body_suspends.push(false);
                    self.try_catch_suspends.push(false);
                    self.try_finally_suspends.push(false);
                }
                let actual = match expected_ty.as_ref() {
                    Some(expected) => self.check_expr_as(value, expected),
                    None => self.check_expr(value),
                };
                if onfail_exception.is_some() {
                    self.try_finally_suspends.pop();
                    self.try_catch_suspends.pop();
                    self.try_body_suspends.pop();
                    self.try_catch_type.pop();
                }
                let actual = if let Some(onfail) = onfail {
                    self.check_onfail(name, onfail, actual, onfail_exception)
                } else {
                    actual
                };
                let resolved_ty = expected_ty
                    .unwrap_or_else(|| self.resolve_declared_type(name, ty, value, &actual));
                self.resolved_bindings.push(ResolvedBinding {
                    name: name.clone(),
                    span: statement.span,
                    ty: resolved_ty.clone(),
                });
                self.require_assignable_on_declaration(&resolved_ty, &actual, name, "E0308");
                self.require_integer_literal_fits(&resolved_ty, value);
                if matches!(ty, Type::Infer)
                    && let ExprKind::ObjectLit(fields) = value.kind()
                {
                    self.validate_object_literal_decorators(&resolved_ty, fields);
                }
                let kind = if matches!(actual, ValueType::Known(Type::Function { .. })) {
                    SymbolKind::Function
                } else {
                    SymbolKind::Variable
                };
                if is_mutex_guard_typed(&resolved_ty) || is_rwlock_guard_typed(&resolved_ty) {
                    self.mark_mutex_guard_local(name);
                }
                let array_len = self.initial_array_len(&resolved_ty, value);
                self.declare(
                    name,
                    Symbol {
                        ty: ValueType::known(&resolved_ty),
                        mutable: *mutable,
                        kind,
                        used: false,
                        array_len,
                    },
                );
                self.record_constant_integer(name, *mutable, Some(value));
            }
            StmtKind::ArrayDestructure {
                mutable,
                bindings,
                value,
            } => {
                let actual = self.check_expr(value);
                if let ValueType::Known(Type::Tuple(elements)) = &actual {
                    for (index, binding) in bindings.iter().enumerate() {
                        let Some(element_ty) = elements.get(index).cloned() else {
                            self.error(
                                "E2493",
                                format!(
                                    "tuple type of length {} has no element at index {index}",
                                    elements.len()
                                ),
                                Some(&binding.name),
                            );
                            continue;
                        };
                        let binding_type = binding.ty.clone();
                        let resolved = if matches!(binding_type, Type::Infer) {
                            element_ty.clone()
                        } else {
                            self.require_assignable(
                                &binding_type,
                                &ValueType::known(&element_ty),
                                &binding.name,
                                "E0308",
                            );
                            binding_type
                        };
                        if matches!(binding.ty, Type::Infer) {
                            self.contextual_parameters.push(InferredParameter {
                                name: binding.name.clone(),
                                ty: resolved.clone(),
                            });
                        }
                        self.declare(
                            &binding.name,
                            Symbol {
                                ty: ValueType::known(&resolved),
                                mutable: *mutable || binding.mutable,
                                kind: SymbolKind::Variable,
                                used: false,
                                array_len: None,
                            },
                        );
                    }
                    return;
                }
                let element_ty = match &actual {
                    ValueType::Known(Type::Array(element)) => Some(element.as_ref().clone()),
                    ValueType::Known(Type::Readonly(inner)) => match inner.as_ref() {
                        Type::Array(element) => Some(element.as_ref().clone()),
                        _ => None,
                    },
                    ValueType::Array(values) => infer_array_element_type(values),
                    _ => None,
                };
                match element_ty {
                    Some(element_ty) => {
                        for binding in bindings {
                            let binding_type = binding.ty.clone();
                            let resolved = if matches!(binding_type, Type::Infer) {
                                element_ty.clone()
                            } else {
                                self.require_assignable(
                                    &binding_type,
                                    &ValueType::known(&element_ty),
                                    &binding.name,
                                    "E0308",
                                );
                                binding_type
                            };
                            self.declare(
                                &binding.name,
                                Symbol {
                                    ty: ValueType::known(&resolved),
                                    mutable: *mutable,
                                    kind: SymbolKind::Variable,
                                    used: false,
                                    array_len: None,
                                },
                            );
                        }
                    }
                    None => match &actual {
                        ValueType::Error | ValueType::Unknown => {
                            for binding in bindings {
                                self.declare(
                                    &binding.name,
                                    Symbol {
                                        ty: ValueType::Unknown,
                                        mutable: *mutable,
                                        kind: SymbolKind::Variable,
                                        used: false,
                                        array_len: None,
                                    },
                                );
                            }
                        }
                        other => {
                            self.error(
                                "E0308",
                                format!("cannot destructure array from type `{}`", other.display()),
                                expression_subject(value),
                            );
                        }
                    },
                }
            }
            StmtKind::ObjectDestructure {
                mutable,
                bindings,
                value,
            } => {
                let actual = self.check_expr(value);
                match &actual {
                    ValueType::Object(fields) => {
                        for binding in bindings {
                            let field_type = fields
                                .iter()
                                .find(|(name, _)| name == &binding.name)
                                .map(|(_, ty)| ty.clone())
                                .unwrap_or(ValueType::Unknown);
                            let binding_type = binding.ty.clone();
                            let resolved = if matches!(binding_type, Type::Infer) {
                                match &field_type {
                                    ValueType::Known(ty) => ty.clone(),
                                    _ => {
                                        self.error(
                                            "E0425",
                                            format!(
                                                "cannot destructure field `{}` from object literal",
                                                binding.name
                                            ),
                                            Some(&binding.name),
                                        );
                                        Type::Void
                                    }
                                }
                            } else {
                                self.require_assignable(
                                    &binding_type,
                                    &field_type,
                                    &binding.name,
                                    "E0308",
                                );
                                binding_type
                            };
                            self.contextual_parameters.push(InferredParameter {
                                name: binding.name.clone(),
                                ty: resolved.clone(),
                            });
                            self.declare(
                                &binding.name,
                                Symbol {
                                    ty: ValueType::known(&resolved),
                                    mutable: *mutable,
                                    kind: SymbolKind::Variable,
                                    used: false,
                                    array_len: None,
                                },
                            );
                        }
                    }
                    ValueType::Known(Type::Named(name)) => {
                        let name_str = name.clone();
                        let field_map = {
                            let iface = self.interfaces.get(name);
                            iface
                                .map(|f| {
                                    f.iter()
                                        .map(|f| (f.name.clone(), field_value_type(f)))
                                        .collect::<std::collections::HashMap<String, Type>>()
                                })
                                .unwrap_or_default()
                        };
                        for binding in bindings {
                            let field_type =
                                field_map.get(&binding.name).cloned().unwrap_or(Type::Void);
                            if matches!(field_type, Type::Void) {
                                self.error(
                                    "E0425",
                                    format!(
                                        "cannot destructure field `{}` from type `{name_str}`",
                                        binding.name
                                    ),
                                    Some(&binding.name),
                                );
                            }
                            let binding_type = binding.ty.clone();
                            let resolved = if matches!(binding_type, Type::Infer) {
                                field_type
                            } else {
                                self.require_assignable(
                                    &binding_type,
                                    &ValueType::known(&field_type),
                                    &binding.name,
                                    "E0308",
                                );
                                binding_type
                            };
                            if !matches!(resolved, Type::Void) {
                                self.contextual_parameters.push(InferredParameter {
                                    name: binding.name.clone(),
                                    ty: resolved.clone(),
                                });
                            }
                            self.declare(
                                &binding.name,
                                Symbol {
                                    ty: ValueType::known(&resolved),
                                    mutable: *mutable,
                                    kind: SymbolKind::Variable,
                                    used: false,
                                    array_len: None,
                                },
                            );
                        }
                    }
                    ValueType::Known(Type::Object(fields)) => {
                        let from_process_env = self.is_process_env(value);
                        for binding in bindings {
                            let field = fields
                                .iter()
                                .find(|candidate| candidate.name == binding.name)
                                .cloned();
                            let Some(field_type) = field.as_ref().map(field_value_type) else {
                                if from_process_env {
                                    self.error_on_declaration(
                                        "E3141",
                                        format!(
                                            "environment variable `{}` is not declared in the env file",
                                            binding.name
                                        ),
                                        Some(&binding.name),
                                    );
                                } else {
                                    self.error(
                                        "E0425",
                                        format!(
                                            "cannot destructure field `{}` from object type",
                                            binding.name
                                        ),
                                        Some(&binding.name),
                                    );
                                }
                                self.declare(
                                    &binding.name,
                                    Symbol {
                                        ty: ValueType::Unknown,
                                        mutable: *mutable,
                                        kind: SymbolKind::Variable,
                                        used: false,
                                        array_len: None,
                                    },
                                );
                                continue;
                            };
                            let binding_type = binding.ty.clone();
                            let resolved = if matches!(binding_type, Type::Infer) {
                                field_type
                            } else {
                                self.require_assignable(
                                    &binding_type,
                                    &ValueType::known(&field_type),
                                    &binding.name,
                                    "E0308",
                                );
                                binding_type
                            };
                            self.contextual_parameters.push(InferredParameter {
                                name: binding.name.clone(),
                                ty: resolved.clone(),
                            });
                            self.declare(
                                &binding.name,
                                Symbol {
                                    ty: ValueType::known(&resolved),
                                    mutable: *mutable,
                                    kind: SymbolKind::Variable,
                                    used: false,
                                    array_len: None,
                                },
                            );
                            if let Some(field) = field {
                                self.declare_validator_facts(
                                    &binding.name,
                                    facts_from_decorators(&field.decorators),
                                );
                            }
                        }
                    }
                    ValueType::Error | ValueType::Unknown => {
                        for binding in bindings {
                            self.declare(
                                &binding.name,
                                Symbol {
                                    ty: ValueType::Unknown,
                                    mutable: *mutable,
                                    kind: SymbolKind::Variable,
                                    used: false,
                                    array_len: None,
                                },
                            );
                        }
                    }
                    other => {
                        self.error(
                            "E0308",
                            format!("cannot destructure object from type `{}`", other.display()),
                            expression_subject(value),
                        );
                    }
                }
            }
            StmtKind::Assign { name, op, value } => {
                let target = self.lookup(name).cloned();
                let Some(target) = target else {
                    self.error(
                        "E0425",
                        format!("cannot assign to undeclared identifier `{name}`"),
                        Some(name),
                    );
                    self.check_expr(value);
                    return;
                };
                if !target.mutable {
                    self.error(
                        "E0384",
                        format!("cannot assign to immutable variable `{name}`"),
                        Some(name),
                    );
                }
                let actual = self.check_expr(value);
                let ValueType::Known(target_ty) = &target.ty else {
                    return;
                };
                if matches!(op, crate::ast::AssignOp::Assign) {
                    self.require_assignable(target_ty, &actual, name, "E0308");
                    let target_ty = target_ty.clone();
                    self.require_integer_literal_fits(&target_ty, value);
                } else {
                    self.check_compound_assignment(name, target_ty, &actual, op);
                }
                let new_len = if self.is_length_volatile() {
                    None
                } else {
                    self.initial_array_len(target_ty, value)
                };
                self.set_array_len(name, new_len);
            }
            StmtKind::IndexAssign { name, index, value } => {
                let target = self.lookup(name).cloned();
                let Some(target) = target else {
                    self.error(
                        "E0425",
                        format!("cannot assign to undeclared identifier `{name}`"),
                        Some(name),
                    );
                    self.check_expr(index);
                    self.check_expr(value);
                    return;
                };
                if !target.mutable {
                    self.error(
                        "E0384",
                        format!("cannot assign through immutable `{name}`"),
                        Some(name),
                    );
                }
                if let ExprKind::StrLit(field) = index.kind()
                    && self.is_struct_like(&target.ty)
                {
                    let field_ty = self.check_field_on_value(target.ty.clone(), field);
                    if let ValueType::Known(expected) = field_ty {
                        let actual = self.check_expr_as(value, &expected);
                        self.require_assignable(&expected, &actual, field, "E0308");
                    } else {
                        self.check_expr(value);
                    }
                    return;
                }
                if let ValueType::Known(Type::Map(key_ty, value_ty)) = &target.ty {
                    let key_ty = key_ty.as_ref().clone();
                    let value_ty = value_ty.as_ref().clone();
                    let index_ty = self.check_expr_as(index, &key_ty);
                    self.require_assignable(&key_ty, &index_ty, name, "E0308");
                    let actual = self.check_expr_as(value, &value_ty);
                    self.require_assignable(&value_ty, &actual, name, "E0308");
                    return;
                }
                let index_ty = self.check_expr(index);
                if !matches!(index_ty, ValueType::IntegerLiteral)
                    && !matches!(index_ty, ValueType::Known(ref ty) if is_integer_type(transparent_type(ty)))
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
                let actual = self.check_expr(value);
                match target.ty {
                    ValueType::Known(Type::Array(element)) => {
                        self.require_assignable(&element, &actual, name, "E0308");
                    }
                    ValueType::Known(Type::Tuple(elements)) => {
                        if let ValueType::Known(element) = tuple_index_type(&elements, index, self)
                        {
                            self.require_assignable(&element, &actual, name, "E0308");
                        }
                    }
                    ValueType::Known(Type::Readonly(inner))
                        if matches!(inner.as_ref(), Type::Array(_) | Type::Tuple(_)) =>
                    {
                        self.error(
                            "E2540",
                            format!("cannot assign through readonly `{name}`"),
                            Some(name),
                        );
                    }
                    other => self.error(
                        "E0608",
                        format!("cannot index into type `{}`", other.display()),
                        Some(name),
                    ),
                }
            }
            StmtKind::FieldAssign {
                object,
                field,
                op,
                value,
            } => {
                if let Some(root) = crate::ast::place_root_ident(object) {
                    let is_mutex_guard_value = field == "value"
                        && self.lookup(root).is_some_and(|sym| {
                            matches!(&sym.ty, ValueType::Known(ty) if is_mutex_guard_typed(ty) || is_rwlock_write_guard_typed(ty))
                        });
                    if !is_mutex_guard_value {
                        match self.lookup(root) {
                            None => {
                                self.error(
                                    "E0425",
                                    format!("cannot assign to undeclared identifier `{root}`"),
                                    Some(root),
                                );
                                self.check_expr(value);
                                return;
                            }
                            Some(sym) if !sym.mutable => {
                                self.error(
                                    "E0384",
                                    format!("cannot assign through immutable `{root}`"),
                                    Some(root),
                                );
                            }
                            _ => {}
                        }
                    }
                }

                let object_ty = self.check_expr(object);

                if let ValueType::Known(Type::Readonly(_)) = &object_ty {
                    self.error(
                        "E2541",
                        format!("cannot assign to `{field}` through a readonly value"),
                        Some(field),
                    );
                    self.check_expr(value);
                    return;
                }

                if self.field_is_readonly(&object_ty, field) {
                    self.error(
                        "E2542",
                        format!("cannot assign to readonly property `{field}`"),
                        Some(field),
                    );
                    self.check_expr(value);
                    return;
                }

                let field_ty = self.check_field_on_value(object_ty.clone(), field);
                if let ValueType::Known(field_ty) = field_ty {
                    let actual = self.check_expr(value);
                    if matches!(op, crate::ast::AssignOp::Assign) {
                        self.require_assignable(&field_ty, &actual, field, "E0308");
                        self.require_integer_literal_fits(&field_ty, value);
                    } else {
                        self.check_compound_assignment(field, &field_ty, &actual, op);
                    }
                } else {
                    self.check_expr(value);
                }
                self.check_assigned_field_decorators(&object_ty, field, value);
            }
            _ => {}
        }
    }
}
