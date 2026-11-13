use super::*;

#[path = "fn_call_hints/fn_call_hints.rs"]
mod fn_call_hints;
#[path = "method_call_hints/method_call_hints.rs"]
mod method_call_hints;

impl Codegen<'_> {
    fn mongo_model_row_type(&self, model_name: &str) -> Option<Type> {
        let mongodb = self
            .database_path
            .as_deref()
            .and_then(|target| serde_json::from_str::<serde_json::Value>(target).ok())
            .and_then(|target| target.get("backend")?.as_str().map(str::to_owned))
            .is_some_and(|backend| backend == "mongodb");
        let fields = self.declared_interfaces.get(model_name)?;
        Some(if mongodb {
            tyrus_core::db::mongo_model_result_type(fields)
        } else {
            Type::Named(model_name.to_string())
        })
    }

    pub(super) fn db_find_many_result_type(&self, type_args: &[Type]) -> Option<Type> {
        let Type::Named(row_type) = type_args.first()? else {
            return None;
        };
        let row_type = self
            .mongo_model_row_type(row_type)
            .unwrap_or_else(|| Type::Named(row_type.clone()));
        Some(Type::Result {
            success: Box::new(Type::Array(Box::new(row_type))),
            error: Box::new(Type::Named("DbError".to_string())),
        })
    }

    pub(super) fn db_find_one_result_type(&self, type_args: &[Type]) -> Option<Type> {
        let Type::Named(row_type) = type_args.first()? else {
            return None;
        };
        let row_type = self
            .mongo_model_row_type(row_type)
            .unwrap_or_else(|| Type::Named(row_type.clone()));
        Some(Type::Result {
            success: Box::new(Type::Optional(Box::new(row_type))),
            error: Box::new(Type::Named("DbError".to_string())),
        })
    }

    pub(super) fn db_count_result_type(&self) -> Type {
        Type::Result {
            success: Box::new(Type::U64),
            error: Box::new(Type::Named("DbError".to_string())),
        }
    }

    pub(super) fn db_query_result_type(&self, type_args: &[Type], args: &[Expr]) -> Option<Type> {
        let is_select = args
            .first()
            .and_then(|arg| match arg.kind() {
                ExprKind::StrLit(sql) => Some(sql),
                _ => None,
            })
            .and_then(|sql| tyrus_core::db::tokenize_sql(sql).ok())
            .and_then(|tokens| {
                tokens
                    .first()
                    .map(|token| token.text.eq_ignore_ascii_case("SELECT"))
            })
            .unwrap_or(false);
        let success = if is_select {
            let Type::Named(row_type) = type_args.first()? else {
                return None;
            };
            Type::Array(Box::new(Type::Named(row_type.clone())))
        } else {
            Type::Void
        };
        Some(Type::Result {
            success: Box::new(success),
            error: Box::new(Type::Named("DbError".to_string())),
        })
    }

    pub(crate) fn expr_type_hint(&self, expr: &Expr) -> Option<Type> {
        let declared = self.declared_expr_type_hint(expr)?;
        let Type::Optional(inner) = transparent_codegen_type(&declared) else {
            return Some(declared);
        };
        let inner = (**inner).clone();
        let narrowed = super::statements::codegen_expression_path(expr)
            .is_some_and(|path| self.path_is_narrowed(&path));
        Some(if narrowed { inner } else { declared })
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn declared_expr_type_hint(&self, expr: &Expr) -> Option<Type> {
        if let ExprKind::MethodCall {
            object,
            method,
            args,
            ..
        } = expr.kind()
            && let ExprKind::Ident(model_name) = object.kind()
            && self.db_models.contains_key(model_name)
        {
            let success = match method.as_str() {
                "findMany" => {
                    let row = self
                        .mongo_model_row_type(model_name)
                        .unwrap_or_else(|| Type::Named(model_name.clone()));
                    if args.get(1).is_some_and(|options| {
                                matches!(options.kind(), ExprKind::ObjectLit(fields) if fields.iter().any(|(name, value)| name == "paginate" && matches!(value.kind(), ExprKind::BoolLit(true))))
                            }) {
                                let fields = tyrus_core::builtin_interfaces::db_page_fields()
                                    .into_iter()
                                    .map(|mut field| {
                                        if field.name == "items" {
                                            field.ty = Type::Array(Box::new(row.clone()));
                                        }
                                        field
                                    })
                                    .collect();
                                Type::Object(fields)
                            } else {
                                Type::Array(Box::new(row))
                            }
                }
                "findOne" => Type::Optional(Box::new(
                    self.mongo_model_row_type(model_name)
                        .unwrap_or_else(|| Type::Named(model_name.clone())),
                )),
                "count" => Type::U64,
                "insertOne" | "insertMany" | "updateOne" | "updateMany" | "deleteOne"
                | "deleteMany" => {
                    let mongodb = self
                        .database_path
                        .as_deref()
                        .and_then(|target| serde_json::from_str::<serde_json::Value>(target).ok())
                        .and_then(|target| target.get("backend")?.as_str().map(str::to_owned))
                        .is_some_and(|backend| backend == "mongodb");
                    if mongodb {
                        tyrus_core::db::mongo_model_mutation_result_type(
                            method,
                            self.declared_interfaces.get(model_name)?,
                        )?
                    } else {
                        tyrus_core::db::model_mutation_result_type(method, model_name)?
                    }
                }
                _ => return None,
            };
            return Some(Type::Apply {
                name: "DbPromise".to_string(),
                args: vec![Type::Result {
                    success: Box::new(success),
                    error: Box::new(Type::Named("DbError".to_string())),
                }],
            });
        }
        if let ExprKind::FnCall {
            callee,
            type_args,
            args,
        } = expr.kind()
        {
            let result = if callee == tyrus_core::db::QUERY_TARGET {
                self.db_query_result_type(type_args, args)
            } else if callee == tyrus_core::db::FIND_MANY_TARGET {
                let Type::Named(row_type) = type_args.first()? else {
                    return None;
                };
                let success = if args.get(1).is_some_and(|options| {
                    matches!(options.kind(), ExprKind::ObjectLit(fields) if fields.iter().any(|(name, value)| name == "paginate" && matches!(value.kind(), ExprKind::BoolLit(true))))
                }) {
                    let fields = tyrus_core::builtin_interfaces::db_page_fields()
                        .into_iter()
                        .map(|mut field| {
                            if field.name == "items" {
                                field.ty = Type::Array(Box::new(Type::Named(row_type.clone())));
                            }
                            field
                        })
                        .collect();
                    Type::Object(fields)
                } else {
                    Type::Array(Box::new(Type::Named(row_type.clone())))
                };
                Some(Type::Result {
                    success: Box::new(success),
                    error: Box::new(Type::Named("DbError".to_string())),
                })
            } else if callee == tyrus_core::db::FIND_ONE_TARGET {
                self.db_find_one_result_type(type_args)
            } else if callee == tyrus_core::db::COUNT_TARGET {
                Some(self.db_count_result_type())
            } else if tyrus_core::db::model_method_name(callee).is_some() {
                Some(Type::Result {
                    success: Box::new(tyrus_core::db::model_mutation_result_type(
                        tyrus_core::db::model_method_name(callee)?,
                        type_args.first().and_then(|ty| match ty {
                            Type::Named(name) => Some(name.as_str()),
                            _ => None,
                        })?,
                    )?),
                    error: Box::new(Type::Named("DbError".to_string())),
                })
            } else {
                None
            };
            if let Some(result) = result {
                return Some(Type::Apply {
                    name: "DbPromise".to_string(),
                    args: vec![result],
                });
            }
        }
        if let ExprKind::MethodCall { object, method, .. } = expr.kind() {
            if matches!(object.kind(), ExprKind::Ident(name) if name == "__tyrus_http_fetch")
                && tyrus_core::http_body::fetch_verb_method(method).is_some()
            {
                return tyrus_core::module_loader::native_target_return_type("__tyrus_http_fetch");
            }
            if matches!(object.kind(), ExprKind::Ident(name) if name == "__tyrus_http_fetch")
                && method == "stream"
            {
                return Some(Type::Apply {
                    name: "Promise".to_string(),
                    args: vec![Type::Named("HttpClientStreamResponse".to_string())],
                });
            }
        }
        if let ExprKind::MethodCall { object, method, .. } = expr.kind()
            && matches!(
                self.declared_expr_type_hint(object),
                Some(Type::Named(ref name)) if name == "HttpClientStreamResponse"
            )
        {
            return match method.as_str() {
                "status" => Some(Type::I32),
                "ok" => Some(Type::Bool),
                "header" => Some(Type::Optional(Box::new(Type::Str))),
                "stream" => Some(Type::Apply {
                    name: "Promise".to_string(),
                    args: vec![Type::Apply {
                        name: "ReadableStream".to_string(),
                        args: vec![Type::Array(Box::new(Type::U8))],
                    }],
                }),
                _ => None,
            };
        }
        if let ExprKind::MethodCall {
            object,
            method,
            type_args,
            ..
        } = expr.kind()
            && matches!(
                self.declared_expr_type_hint(object),
                Some(Type::Named(ref name)) if name == "HttpClientResponse"
            )
        {
            return match method.as_str() {
                "status" => Some(Type::I32),
                "ok" => Some(Type::Bool),
                "text" => Some(Type::Str),
                "bytes" => Some(Type::Array(Box::new(Type::U8))),
                "header" => Some(Type::Optional(Box::new(Type::Str))),
                "json" => match type_args.as_slice() {
                    [Type::Named(name)] => Some(Type::Result {
                        success: Box::new(Type::Named(name.clone())),
                        error: Box::new(Type::Named("JsonError".to_string())),
                    }),
                    _ => None,
                },
                _ => None,
            };
        }
        let is_style_text = |name: &str| name == "styleText" && self.load_var(name).is_none();
        if matches!(
            tyrus_core::style_text::as_style_text(expr, &is_style_text),
            Some(tyrus_core::style_text::StyleTextForm::Styled { .. })
        ) {
            return Some(Type::Str);
        }
        let is_figlet = |name: &str| name == "figlet" && self.load_var(name).is_none();
        if matches!(
            tyrus_core::figlet::as_figlet(expr, &is_figlet),
            Some(tyrus_core::figlet::FigletForm::Rendered { .. })
        ) {
            return Some(Type::Str);
        }
        if let Expr {
            kind:
                ExprKind::MethodCall {
                    object,
                    method,
                    type_args,
                    args,
                    ..
                },
            ..
        } = expr
        {
            if method == "fromJson"
                && let ExprKind::Ident(type_name) = object.as_ref().kind()
                && self.declared_interfaces.contains_key(type_name.as_str())
            {
                return Some(Type::Result {
                    success: Box::new(Type::Named(type_name.clone())),
                    error: Box::new(Type::Named("JsonError".to_string())),
                });
            }
            if let ExprKind::Ident(type_name) = object.as_ref().kind()
                && let Some(enum_info) = self.enums.get(type_name)
                && enum_info.members.iter().any(|(member, _)| member == method)
            {
                return Some(Type::Named(type_name.clone()));
            }
            if matches!(object.as_ref().kind(), ExprKind::Ident(name) if name == "Array")
                && self.load_var("Array").is_none()
            {
                match method.as_str() {
                    "isArray" => return Some(Type::Bool),
                    "of" => {
                        return self
                            .infer_array_literal_type(args)
                            .map(|element| Type::Array(Box::new(element)));
                    }
                    "from" => {
                        return args
                            .first()
                            .and_then(|source| self.expr_type_hint(source))
                            .and_then(|ty| match transparent_codegen_type(&ty) {
                                Type::Array(element) => {
                                    Some(Type::Array(Box::new(element.as_ref().clone())))
                                }
                                _ => None,
                            });
                    }
                    "withCapacity" => {
                        return None;
                    }
                    _ => {}
                }
            }
            if matches!(object.as_ref().kind(), ExprKind::Ident(name) if name == "Promise")
                && self.load_var("Promise").is_none()
            {
                match method.as_str() {
                    "reject" => {
                        return Some(Type::Apply {
                            name: "Promise".to_string(),
                            args: vec![type_args.first().cloned().unwrap_or(Type::Unknown)],
                        });
                    }
                    "resolve" => {
                        let payload = type_args.first().cloned().unwrap_or_else(|| {
                            args.first()
                                .and_then(|value| self.expr_type_hint(value))
                                .unwrap_or(Type::Void)
                        });
                        return Some(match payload {
                            Type::Apply { ref name, .. } if name == "Promise" => payload,
                            payload => Type::Apply {
                                name: "Promise".to_string(),
                                args: vec![payload],
                            },
                        });
                    }
                    "all" => {
                        let payload = type_args
                            .first()
                            .cloned()
                            .or_else(|| self.promise_combinator_payload(args))
                            .unwrap_or(Type::Unknown);
                        return Some(Type::Apply {
                            name: "Promise".to_string(),
                            args: vec![Type::Array(Box::new(payload))],
                        });
                    }
                    "allSettled" => {
                        let payload = type_args
                            .first()
                            .cloned()
                            .or_else(|| self.promise_combinator_payload(args))
                            .unwrap_or(Type::Unknown);
                        return Some(Type::Apply {
                            name: "Promise".to_string(),
                            args: vec![Type::Array(Box::new(promise_settled_codegen_type(
                                payload,
                            )))],
                        });
                    }
                    "race" | "any" => {
                        return Some(Type::Apply {
                            name: "Promise".to_string(),
                            args: vec![
                                type_args
                                    .first()
                                    .cloned()
                                    .or_else(|| self.promise_combinator_payload(args))
                                    .unwrap_or(Type::Unknown),
                            ],
                        });
                    }
                    "withResolvers" => {
                        let payload = type_args.first().cloned().unwrap_or(Type::Unknown);
                        return Some(Type::Object(vec![
                            Field {
                                name: "promise".to_string(),
                                ty: Type::Apply {
                                    name: "Promise".to_string(),
                                    args: vec![payload.clone()],
                                },
                                readonly: false,
                                optional: false,
                                decorators: Vec::new(),
                                span: Span::DUMMY,
                            },
                            Field {
                                name: "resolve".to_string(),
                                ty: Type::Function {
                                    params: vec![payload],
                                    ret: Box::new(Type::Void),
                                },
                                readonly: false,
                                optional: false,
                                decorators: Vec::new(),
                                span: Span::DUMMY,
                            },
                            Field {
                                name: "reject".to_string(),
                                ty: Type::Function {
                                    params: vec![Type::Named("Error".to_string())],
                                    ret: Box::new(Type::Void),
                                },
                                readonly: false,
                                optional: false,
                                decorators: Vec::new(),
                                span: Span::DUMMY,
                            },
                        ]));
                    }
                    _ => {}
                }
            }
            if matches!(object.as_ref().kind(), ExprKind::Ident(name) if name == "console")
                && self.load_var("console").is_none()
                && matches!(method.as_str(), "log" | "error" | "warn")
            {
                return Some(Type::Void);
            }
            if matches!(object.as_ref().kind(), ExprKind::Ident(name) if name == "process")
                && self.load_var("process").is_none()
            {
                match method.as_str() {
                    "cwd" => return Some(Type::Str),
                    "uptime" => return Some(Type::U64),
                    "memoryUsage" => return Some(Type::Named("MemoryInfo".to_string())),
                    "cpuUsage" => return Some(Type::Named("CpuUsage".to_string())),
                    _ => {}
                }
            }
            if let Some(ty) = self.extension_method_return_type(object, method) {
                return Some(ty);
            }
            if let Some((automatic, receiver)) = self.automatic_data_method_for(object, method) {
                return match automatic {
                    tyrus_core::automatic_data_methods::AutomaticDataMethod::Copy => Some(receiver),
                    tyrus_core::automatic_data_methods::AutomaticDataMethod::Eq => Some(Type::Bool),
                    tyrus_core::automatic_data_methods::AutomaticDataMethod::Debug => {
                        Some(Type::Str)
                    }
                    _ => None,
                };
            }
        }
        match expr.kind() {
            ExprKind::ObjectLit(_) => None,
            ExprKind::Try(inner) => self.expr_type_hint(inner).and_then(|ty| match ty {
                Type::Result { success, .. } => Some(*success),
                _ => None,
            }),
            ExprKind::CallValue { callee, .. } => match self.expr_type_hint(callee) {
                Some(Type::Function { ret, .. }) => Some(*ret),
                _ => None,
            },
            ExprKind::ObjectUpdate { base, .. } => self.expr_type_hint(base),
            ExprKind::ArrayLit(values) => self
                .infer_array_literal_type(values)
                .map(|element| Type::Array(Box::new(element))),
            ExprKind::ArrayRepeat { value, .. } => self
                .expr_type_hint(value)
                .map(|element| Type::Array(Box::new(element))),
            ExprKind::FieldAccess { object, field } => {
                if self.is_process_env(object) {
                    return self.env_declared_type(field);
                }
                if self.is_process_namespace(object) {
                    match field.as_str() {
                        "argv" => return Some(Type::Array(Box::new(Type::Str))),
                        "pid" | "ppid" => return Some(Type::U32),
                        _ => {}
                    }
                }
                if let ExprKind::Ident(type_name) = object.as_ref().kind() {
                    if let Some(ty) = numeric_constant_type(type_name, field) {
                        return Some(ty);
                    }
                    if let Some(enum_info) = self.enums.get(type_name)
                        && enum_info.members.iter().any(|(member, _)| member == field)
                    {
                        return Some(enum_info.repr.clone());
                    }
                }
                let object_type = self.expr_type_hint(object)?;
                self.field_type_on(&object_type, field)
            }
            ExprKind::OptionalFieldAccess { object, field } => {
                let Type::Optional(inner) = self.expr_type_hint(object)? else {
                    return None;
                };
                let field_type = self.field_type_on(&inner, field)?;
                Some(match field_type {
                    already @ Type::Optional(_) => already,
                    field_type => Type::Optional(Box::new(field_type)),
                })
            }
            ExprKind::IndexAccess { object, .. } => {
                let object_type = self.expr_type_hint(object)?;
                match transparent_codegen_type(&object_type) {
                    Type::Array(element) => Some((**element).clone()),
                    Type::Map(_, value) => Some(Type::Optional(value.clone())),
                    Type::Tuple(elements) => {
                        let Expr {
                            kind: ExprKind::IndexAccess { index, .. },
                            ..
                        } = expr
                        else {
                            return None;
                        };
                        let Expr {
                            kind: ExprKind::IntLit(index),
                            ..
                        } = index.as_ref()
                        else {
                            return None;
                        };
                        elements.get(*index as usize).cloned()
                    }
                    _ => None,
                }
            }
            ExprKind::Ident(name) | ExprKind::Borrow { name, .. } => self
                .load_var(name)
                .map(|(_, ty)| {
                    if matches!(
                        transparent_codegen_type(&ty),
                        Type::Union(_) | Type::Optional(_)
                    ) {
                        self.load_concrete_type(name).unwrap_or(ty)
                    } else {
                        ty
                    }
                })
                .or_else(|| self.async_prescan_types.get(name).cloned())
                .or_else(|| match name.as_str() {
                    "digest" => Some(Type::Named("DigestNamespace".to_string())),
                    "hmac" => Some(Type::Named("HmacNamespace".to_string())),
                    "cipher" => Some(Type::Named("CipherNamespace".to_string())),
                    "rand" => Some(Type::Named("RandNamespace".to_string())),
                    "argon2" => Some(Type::Named("Argon2Namespace".to_string())),
                    "jwt" => Some(Type::Named("JwtNamespace".to_string())),
                    _ => None,
                })
                .or_else(|| {
                    self.fn_signatures
                        .get(name)
                        .map(|(params, ret)| Type::Function {
                            params: params.clone(),
                            ret: Box::new(ret.clone()),
                        })
                }),
            ExprKind::IntLit(value) => Some(int_literal_type(*value)),
            ExprKind::UIntLit(_) => Some(Type::U128),
            ExprKind::FloatLit(_) => Some(Type::F64),
            ExprKind::DecimalLit(_) => Some(Type::Decimal),
            ExprKind::StrLit(_) | ExprKind::TemplateLit(_) => Some(Type::Str),
            ExprKind::RegexLit { .. } => Some(Type::Regex),
            ExprKind::NewSet { ty, initial } => {
                let element = ty
                    .clone()
                    .or_else(|| self.new_set_element_type(initial.as_deref()))?;
                Some(Type::Set(Box::new(element)))
            }
            ExprKind::NewMap {
                key,
                value,
                initial,
            } => {
                let (key, value) = match (key, value) {
                    (Some(key), Some(value)) => (key.clone(), value.clone()),
                    _ => self.new_map_entry_types(initial.as_deref())?,
                };
                Some(Type::Map(Box::new(key), Box::new(value)))
            }
            ExprKind::BoolLit(_) | ExprKind::Not(_) => Some(Type::Bool),
            ExprKind::Typeof(_) => Some(Type::Str),
            ExprKind::FnCall { .. } => self.declared_fn_call_type_hint(expr),
            ExprKind::BinOp { op, left, right } => match op {
                BinOp::Gt
                | BinOp::Lt
                | BinOp::Gte
                | BinOp::Lte
                | BinOp::Eq
                | BinOp::Neq
                | BinOp::And
                | BinOp::Or => Some(Type::Bool),
                BinOp::NullCoalesce => match self.expr_type_hint(left) {
                    Some(Type::Optional(inner)) => Some(*inner),
                    other => other.or_else(|| self.expr_type_hint(right)),
                },
                _ => self
                    .expr_type_hint(left)
                    .or_else(|| self.expr_type_hint(right)),
            },
            ExprKind::Pipeline { left, right } => {
                if tyrus_core::ast::contains_outer_topic(right) {
                    self.expr_type_hint(right)
                } else {
                    let call = tyrus_core::ast::desugar_pipeline(left, right)?;
                    self.expr_type_hint(&call)
                }
            }
            ExprKind::Topic => self.topic_values.last().map(|(_, ty)| ty.clone()),
            ExprKind::Unwrap(inner) => self.expr_type_hint(inner).map(|ty| match ty {
                Type::Optional(inner) => *inner,
                other => other,
            }),
            ExprKind::MethodCall { .. } | ExprKind::OptionalMethodCall { .. } => {
                self.declared_method_call_type_hint(expr)
            }
            ExprKind::Null | ExprKind::None | ExprKind::ArrowFn { .. } | ExprKind::If { .. } => {
                None
            }
            ExprKind::Loop { body } => self.loop_value_type_hint(body),
            ExprKind::Neg(inner) | ExprKind::BitNot(inner) => self.expr_type_hint(inner),
            ExprKind::PrefixIncr(inner)
            | ExprKind::PostfixIncr(inner)
            | ExprKind::PrefixDecr(inner)
            | ExprKind::PostfixDecr(inner) => self.expr_type_hint(inner),
            ExprKind::Ternary { then_expr, .. } => self.expr_type_hint(then_expr),
            ExprKind::TypeAssert { ty, .. } => Some(ty.clone()),
            ExprKind::ConstAssert(inner) => {
                Some(tyrus_core::type_checker::const_assertion_type(inner))
            }
            ExprKind::Comptime(inner) => self.expr_type_hint(inner),
            ExprKind::Spread(inner) => self.expr_type_hint(inner),
            ExprKind::Await(inner) => {
                let promise = self.expr_type_hint(inner)?;
                match transparent_codegen_type(&promise) {
                    Type::Apply { name, args }
                        if matches!(name.as_str(), "Promise" | "DbPromise") && args.len() == 1 =>
                    {
                        args.first().cloned()
                    }
                    _ => None,
                }
            }
            ExprKind::In { .. } => Some(Type::Bool),
            ExprKind::Is { .. } => Some(Type::Bool),
            ExprKind::Range { start, end, .. } => {
                let element = self
                    .expr_type_hint(start)
                    .or_else(|| self.expr_type_hint(end))?;
                Some(Type::Range(Box::new(element)))
            }
            ExprKind::ChainedComparison { .. } => Some(Type::Bool),
        }
    }

    pub(super) fn loop_value_type_hint(&self, body: &[Stmt]) -> Option<Type> {
        for statement in body {
            match statement.kind() {
                StmtKind::Break(Some(value)) => {
                    let ty = self.expr_type_hint(value);
                    if ty.is_some() {
                        return ty;
                    }
                }
                StmtKind::Break(None) | StmtKind::Continue => return None,
                StmtKind::Expr(expression) => {
                    if let ExprKind::If {
                        cond: _,
                        then_block,
                        else_block,
                    } = expression.kind()
                    {
                        if let Some(ty) = self.loop_value_type_hint(then_block) {
                            return Some(ty);
                        }
                        if let Some(else_block) = else_block
                            && let Some(ty) = self.loop_value_type_hint(else_block)
                        {
                            return Some(ty);
                        }
                    }
                }
                StmtKind::TryCatch {
                    try_body,
                    catch_body,
                    finally_body,
                    ..
                } => {
                    if let Some(ty) = self.loop_value_type_hint(try_body) {
                        return Some(ty);
                    }
                    if let Some(ty) = self.loop_value_type_hint(catch_body) {
                        return Some(ty);
                    }
                    if let Some(finally_body) = finally_body
                        && let Some(ty) = self.loop_value_type_hint(finally_body)
                    {
                        return Some(ty);
                    }
                }
                _ => {}
            }
        }
        None
    }

    pub(super) fn infer_decl_type(&self, value: &Expr) -> Option<Type> {
        if matches!(value.kind(), ExprKind::UIntLit(_)) {
            return None;
        }
        if let Some(literal) = codegen_const_int(value) {
            if literal < i128::from(i64::MIN) || literal > i128::from(i64::MAX) {
                return Some(Type::I128);
            }
            if literal < i128::from(i32::MIN) || literal > i128::from(i32::MAX) {
                return Some(Type::I64);
            }
        }
        match value.kind() {
            ExprKind::FnCall {
                callee,
                type_args,
                args,
                ..
            } if callee == tyrus_core::db::QUERY_TARGET => self.db_query_result_type(type_args, args),
            ExprKind::FnCall {
                callee,
                type_args,
                ..
            } if callee == tyrus_core::db::FIND_MANY_TARGET => {
                self.db_find_many_result_type(type_args)
            }
            ExprKind::FnCall {
                callee,
                type_args,
                ..
            } if callee == tyrus_core::db::FIND_ONE_TARGET => {
                self.db_find_one_result_type(type_args)
            }
            ExprKind::FnCall { callee, .. } if callee == tyrus_core::db::COUNT_TARGET => {
                Some(self.db_count_result_type())
            }
            ExprKind::MethodCall { object, method, .. } if method == "toJson" => self
                .expr_type_hint(object)
                .filter(|ty| matches!(transparent_codegen_type(ty), Type::Named(name) if self.declared_interfaces.contains_key(name)))
                .map(|_| Type::Str),
            ExprKind::MethodCall { object, method, .. }
                if method == "fromJson"
                    && matches!(object.kind(), ExprKind::Ident(name) if self.declared_interfaces.contains_key(name)) =>
            {
                let ExprKind::Ident(name) = object.kind() else {
                    unreachable!("the guard requires an interface identifier");
                };
                Some(Type::Result {
                    success: Box::new(Type::Named(name.clone())),
                    error: Box::new(Type::Named("JsonError".to_string())),
                })
            }
            ExprKind::ObjectLit(fields) => {
                let mut inferred = Vec::with_capacity(fields.len());
                for (name, field_value) in fields {
                    inferred.push(Field {
                        name: name.clone(),
                        ty: self.infer_decl_type(field_value)?,
                        readonly: false,
                        optional: false,
                        decorators: Vec::new(),
                        span: Span::DUMMY,
                    });
                }
                self.infer_exact_interface_type(&inferred)
            }
            _ => self.expr_type_hint(value),
        }
    }

    pub(super) fn infer_exact_interface_type(&self, actual_fields: &[Field]) -> Option<Type> {
        let mut matches = self
            .interfaces
            .iter()
            .filter(|(_, fields)| exact_codegen_interface_match(fields, actual_fields))
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        matches.sort();
        (matches.len() == 1).then(|| Type::Named(matches.remove(0)))
    }
}
