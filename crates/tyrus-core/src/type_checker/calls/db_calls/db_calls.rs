use super::*;

impl TypeChecker {
    pub(super) fn check_db_query_call(
        &mut self,
        type_args: &[Type],
        args: &[Expr],
        span: Span,
    ) -> Type {
        let error = Box::new(Type::Named("DbError".to_string()));
        if self.db_backend == Some(crate::db::DatabaseBackend::Mongodb) {
            self.error_at_with_help(
                "E3306",
                "`db.query` SQL is not supported when `database.backend` is `mongodb`".to_string(),
                args.first().map_or(span, |argument| argument.span),
                vec![
                    "use a relational backend such as `sqlite`, `postgresql`, or `mysql` for `db.query`".to_string(),
                    "use supported MongoDB Model methods such as `findMany`, `insertOne`, or `updateMany`".to_string(),
                ],
            );
            return db_promise(Type::Result {
                success: Box::new(Type::Void),
                error,
            });
        }
        let Some(source) = self.source_text.as_deref() else {
            return db_promise(Type::Result {
                success: Box::new(Type::Void),
                error,
            });
        };
        let captured = match crate::db::CapturedSqlCall::capture_arguments(
            source,
            &args[..args.len().min(2)],
            span.to_range(),
        ) {
            Ok(captured) => captured,
            Err(capture_error) => {
                self.error_at_with_help(
                    "E3300",
                    capture_error.message,
                    Span::new(capture_error.span.start, capture_error.span.end),
                    vec![
                        "write SQL as a string literal and parameters as an array literal"
                            .to_string(),
                    ],
                );
                return db_promise(Type::Result {
                    success: Box::new(Type::Void),
                    error,
                });
            }
        };
        if captured.parameters.len() > crate::db::MAX_BOUND_PARAMETERS {
            let parameter_span = captured
                .parameter_spans
                .get(crate::db::MAX_BOUND_PARAMETERS)
                .cloned()
                .unwrap_or_else(|| span.to_range());
            self.error_at_with_help(
                "E3305",
                format!(
                    "`db.query` accepts at most {} bound parameters",
                    crate::db::MAX_BOUND_PARAMETERS
                ),
                Span::new(parameter_span.start, parameter_span.end),
                vec![
                    "reduce the parameter array to at most 999 values or split the database work into separate calls".to_string(),
                ],
            );
        }
        let parameter_types = captured
            .parameters
            .iter()
            .map(|parameter| {
                self.expression_types_by_span
                    .get(&parameter.span)
                    .cloned()
                    .unwrap_or(Type::Unknown)
            })
            .collect::<Vec<_>>();
        let first_keyword = crate::db::tokenize_sql(&captured.sql)
            .ok()
            .and_then(|tokens| tokens.first().map(|token| token.text.to_ascii_uppercase()));
        if first_keyword.as_deref() == Some("SELECT") {
            if self.validate_db_schema {
                self.check_db_select_call("db.query", type_args, &args[..args.len().min(2)], span);
            }
            let row_type = type_args.first().cloned().unwrap_or(Type::Unknown);
            return db_promise(Type::Result {
                success: Box::new(Type::Array(Box::new(row_type))),
                error,
            });
        }
        if !type_args.is_empty() {
            self.error_at_with_help(
                "E3303",
                "`db.query` mutation statements do not take a result interface type".to_string(),
                Span::new(span.start, span.end),
                vec![
                    "omit the type argument for `INSERT`, `UPDATE`, and `DELETE` statements"
                        .to_string(),
                ],
            );
        }
        if !self.validate_db_schema {
            return db_promise(Type::Result {
                success: Box::new(Type::Void),
                error,
            });
        }
        if let Some(schema) = self.db_schema.as_ref() {
            let captured = captured.clone();
            if let Err(validation_error) =
                captured.validate_exec_at_source(schema, &parameter_types)
            {
                self.error_at_with_help(
                    "E3302",
                    validation_error.message,
                    Span::new(validation_error.span.start, validation_error.span.end),
                    vec![
                        "correct the SQL or regenerate the snapshot with `tyrus db introspect`"
                            .to_string(),
                    ],
                );
            }
        } else {
            let detail = self
                .db_schema_error
                .as_deref()
                .unwrap_or("database schema snapshot is unavailable");
            self.error_at_with_help(
                "E3301",
                format!("cannot validate `db.query`: {detail}"),
                args.first()
                    .map_or(Span::new(span.start, span.start), |arg| arg.span),
                vec![
                    "configure `database` in tyrus.json and run `tyrus db introspect`".to_string(),
                ],
            );
        }
        db_promise(Type::Result {
            success: Box::new(Type::Void),
            error,
        })
    }

    pub(super) fn check_db_find_many_call(
        &mut self,
        type_args: &[Type],
        args: &[Expr],
        span: Span,
    ) -> Type {
        self.check_db_model_find_call(type_args, args, span, false, false)
    }

    pub(super) fn check_db_find_one_call(
        &mut self,
        type_args: &[Type],
        args: &[Expr],
        span: Span,
    ) -> Type {
        self.check_db_model_find_call(type_args, args, span, true, false)
    }

    pub(super) fn check_db_count_call(
        &mut self,
        type_args: &[Type],
        args: &[Expr],
        span: Span,
    ) -> Type {
        self.check_db_model_find_call(type_args, args, span, false, true)
    }

    fn check_db_find_many_includes(
        &mut self,
        model_name: &str,
        model: &crate::ast::DbModelMetadata,
        args: &[Expr],
    ) -> Vec<crate::db::DbRelationInclude> {
        let mut includes = Vec::new();
        let mut includes_collection_relation = false;
        if args.len() == 2
            && let ExprKind::ObjectLit(options) = args[1].kind()
        {
            if let Some((_, value)) = options.iter().find(|(name, _)| name == "include") {
                if let ExprKind::ArrayLit(items) = value.kind() {
                    let mut included_names = std::collections::HashSet::new();
                    for item in items {
                        let parsed = match item.kind() {
                            ExprKind::StrLit(name) => Some((name.clone(), None)),
                            ExprKind::ObjectLit(properties) if properties.len() == 1 => {
                                let (name, projection) = &properties[0];
                                match projection.kind() {
                                    ExprKind::ArrayLit(selected) if !selected.is_empty() => {
                                        let mut names = Vec::new();
                                        let mut valid = true;
                                        for field in selected {
                                            if let ExprKind::StrLit(field_name) = field.kind() {
                                                names.push(field_name.clone());
                                            } else {
                                                valid = false;
                                                self.error_at_with_help(
                                                        "E3304",
                                                        "relationship projection fields must be string literals".to_string(),
                                                        field.span,
                                                        vec!["use an array such as `{ team: [\"id\", \"name\"] }`".to_string()],
                                                    );
                                            }
                                        }
                                        valid.then_some((name.clone(), Some(names)))
                                    }
                                    ExprKind::ArrayLit(_) => {
                                        self.error_at_with_help(
                                                "E3304",
                                                "relationship projections must select at least one field".to_string(),
                                                projection.span,
                                                vec!["select one or more scalar fields from the related Model".to_string()],
                                            );
                                        None
                                    }
                                    _ => {
                                        self.error_at_with_help(
                                                "E3304",
                                                "relationship projection must be an array of field names".to_string(),
                                                projection.span,
                                                vec!["use `{ team: [\"id\", \"name\"] }` inside `include`".to_string()],
                                            );
                                        None
                                    }
                                }
                            }
                            _ => None,
                        };
                        let Some((name, projection)) = parsed else {
                            self.error_at_with_help(
                                    "E3304",
                                    "each `findMany` include must be a relationship name or a single-field projection object".to_string(),
                                    item.span,
                                    vec!["use `\"team\"` or `{ team: [\"id\", \"name\"] }` in `include`".to_string()],
                                );
                            continue;
                        };
                        if !included_names.insert(name.clone()) {
                            self.error_at_with_help(
                                "E3304",
                                format!("relationship `{name}` is included more than once"),
                                item.span,
                                vec!["include each relationship only once".to_string()],
                            );
                            continue;
                        }
                        if let Some(relation) = model
                            .relations
                            .iter()
                            .find(|relation| relation.field == name)
                        {
                            if self.db_backend.as_ref().is_some_and(|backend| {
                                *backend != crate::db::DatabaseBackend::Sqlite
                            }) {
                                self.error_at_with_help(
                                        "E3307",
                                        format!("relation include `{name}` is not supported by the configured database backend"),
                                        item.span,
                                        vec![
                                            "remove this relation from `include` or configure SQLite, which supports relation includes".to_string(),
                                            "use explicitly supported scalar Model queries until this backend gains relation include support".to_string(),
                                        ],
                                    );
                            }
                            includes_collection_relation |= matches!(
                                relation.kind,
                                crate::ast::DbModelRelationKind::HasMany
                                    | crate::ast::DbModelRelationKind::ManyToMany
                            );
                            if let Some(selected) = &projection {
                                let target_fields = self
                                    .interfaces
                                    .get(&relation.target_model)
                                    .map(|fields| {
                                        fields
                                            .iter()
                                            .map(|field| field.name.clone())
                                            .collect::<std::collections::HashSet<_>>()
                                    })
                                    .unwrap_or_default();
                                let target_relations = self
                                    .db_models
                                    .get(&relation.target_model)
                                    .map(|target| {
                                        target
                                            .relations
                                            .iter()
                                            .map(|nested| nested.field.clone())
                                            .collect::<std::collections::HashSet<_>>()
                                    })
                                    .unwrap_or_default();
                                let mut seen_fields = std::collections::HashSet::new();
                                for selected_name in selected {
                                    if !seen_fields.insert(selected_name) {
                                        self.error_at_with_help(
                                                "E3304",
                                                format!("field `{selected_name}` is projected more than once"),
                                                item.span,
                                                vec!["list each projected related field once".to_string()],
                                            );
                                        continue;
                                    }
                                    if target_relations.contains(selected_name) {
                                        self.error_at_with_help(
                                                "E3304",
                                                format!("nested relationship `{selected_name}` cannot be projected at this depth"),
                                                item.span,
                                                vec!["include relationships only one level deep and select scalar fields".to_string()],
                                            );
                                    } else if !target_fields.contains(selected_name) {
                                        self.error_at_with_help(
                                                "E3304",
                                                format!("field `{selected_name}` does not exist on related Model `{}`", relation.target_model),
                                                item.span,
                                                vec!["select declared scalar fields from the related Model".to_string()],
                                            );
                                    }
                                }
                            }
                            includes.push(crate::db::DbRelationInclude {
                                field: name.clone(),
                                projection,
                            });
                        } else {
                            self.error_at_with_help(
                                    "E3304",
                                    format!("`{name}` is not a relationship field on Model `{model_name}`"),
                                    item.span,
                                    vec!["declare the relationship on the model or remove it from `include`".to_string()],
                                );
                        }
                    }
                } else {
                    self.error_at_with_help(
                            "E3304",
                            "`findMany` `include` must be an array of relationship names or projections"
                                .to_string(),
                            value.span,
                            vec!["pass an array such as `{ include: [\"team\"] }` or `{ include: [{ team: [\"id\"] }] }`".to_string()],
                        );
                }
            }
            let relation_limit = options.iter().find(|(name, _)| name == "relationLimit");
            if let Some((_, value)) = relation_limit {
                let limit = match value.kind() {
                    ExprKind::UIntLit(limit) => u64::try_from(*limit).ok(),
                    ExprKind::IntLit(limit) => u64::try_from(*limit).ok(),
                    _ => None,
                };
                if !limit.is_some_and(|limit| (1..=crate::db::MAX_RELATION_ROWS).contains(&limit)) {
                    self.error_at_with_help(
                        "E3304",
                        format!(
                            "`relationLimit` must be a literal integer between 1 and {}",
                            crate::db::MAX_RELATION_ROWS
                        ),
                        value.span,
                        vec![format!(
                            "set `relationLimit` to a literal from 1 through {}",
                            crate::db::MAX_RELATION_ROWS
                        )],
                    );
                }
                if !includes_collection_relation {
                    self.error_at_with_help(
                            "E3304",
                            "`relationLimit` requires an included `hasMany` or `manyToMany` relationship".to_string(),
                            value.span,
                            vec!["include a to-many relation or remove `relationLimit`".to_string()],
                        );
                }
            } else if includes_collection_relation {
                self.error_at_with_help(
                    "E3304",
                    "including a `hasMany` or `manyToMany` relationship requires `relationLimit`"
                        .to_string(),
                    options
                        .iter()
                        .find(|(name, _)| name == "include")
                        .map_or(args[1].span, |(_, value)| value.span),
                    vec![format!(
                        "pass `{{relationLimit: 100}}` with a literal limit from 1 through {}",
                        crate::db::MAX_RELATION_ROWS
                    )],
                );
            }
        }
        includes
    }

    fn db_model_find_name(
        &mut self,
        method_name: &str,
        type_args: &[Type],
        args: &[Expr],
        span: Span,
    ) -> Option<String> {
        let Some(Type::Named(model_name)) = type_args.first() else {
            self.error_at_with_help(
                "E3304",
                format!("`Model.{method_name}` requires an imported model interface"),
                Span::new(span.start, span.start),
                vec![format!(
                    "import the model and call `User.{method_name}(filter)`"
                )],
            );
            return None;
        };
        if !(1..=2).contains(&args.len()) {
            self.error_at_with_help(
                "E3304",
                format!(
                    "`Model.{method_name}` received an invalid number of arguments: {}",
                    args.len()
                ),
                Span::new(span.start, span.end),
                vec![if method_name == "findMany" {
                    "pass a filter and optional pagination options".to_string()
                } else {
                    "pass one object literal containing the model filter".to_string()
                }],
            );
            return None;
        }
        Some(model_name.clone())
    }

    fn check_db_model_find_call(
        &mut self,
        type_args: &[Type],
        args: &[Expr],
        span: Span,
        single: bool,
        count: bool,
    ) -> Type {
        let error_type = Type::Named("DbError".to_string());
        let row_type = type_args.first().cloned().unwrap_or(Type::Unknown);
        let pagination = !single && !count && self.check_db_find_many_pagination(args);
        let mut result = db_promise(Type::Result {
            success: Box::new(if count {
                Type::U64
            } else if single {
                Type::Optional(Box::new(row_type.clone()))
            } else if pagination {
                db_page_type(&row_type)
            } else {
                Type::Array(Box::new(row_type.clone()))
            }),
            error: Box::new(error_type.clone()),
        });
        let method_name = if count {
            "count"
        } else if single {
            "findOne"
        } else {
            "findMany"
        };
        let Some(model_name) = self.db_model_find_name(method_name, type_args, args, span) else {
            return result;
        };
        if method_name == "findMany"
            && let Some((_, options)) = args.get(1).and_then(|options| match options.kind() {
                ExprKind::ObjectLit(properties) => {
                    properties.iter().find(|(name, _)| name == "transaction")
                }
                _ => None,
            })
        {
            self.check_db_transaction_handle(options);
            if self.db_backend == Some(crate::db::DatabaseBackend::Mongodb) {
                self.error_at_with_help(
                    "E3304",
                    "MongoDB Model operations do not support transaction handles yet".to_string(),
                    options.span,
                    vec!["remove the `transaction` option when using MongoDB".to_string()],
                );
            }
        }
        if pagination && args.get(1).is_some_and(object_has_transaction_option) {
            let options = &args[1];
            self.error_at_with_help(
                "E3304",
                "paginated `findMany` manages its own consistent read transaction".to_string(),
                options.span,
                vec!["omit `transaction` when using pagination".to_string()],
            );
        }
        let Some(model) = self.db_models.get(&model_name).cloned() else {
            self.error_at_with_help(
                "E3304",
                format!("type `{model_name}` is not a database model"),
                args[0].span,
                vec!["declare the interface with `@model(\"table_name\")`".to_string()],
            );
            return result;
        };
        if method_name != "findMany"
            && let Some(options) = args.get(1)
        {
            self.check_db_transaction_options(options);
            if self.db_backend == Some(crate::db::DatabaseBackend::Mongodb)
                && object_has_transaction_option(options)
            {
                self.error_at_with_help(
                    "E3304",
                    "MongoDB Model operations do not support transaction handles yet".to_string(),
                    options.span,
                    vec!["remove the `transaction` option when using MongoDB".to_string()],
                );
            }
        }
        if !self.check_mongo_model_capabilities(&model_name, &model) {
            return result;
        }
        let Some(fields) = self.interfaces.get(&model_name).cloned() else {
            return result;
        };
        let includes = if !single && !count {
            self.check_db_find_many_includes(&model_name, &model, args)
        } else {
            Vec::new()
        };
        let row_type = if includes.iter().any(|include| include.projection.is_some()) {
            match crate::db::project_model_result_type(
                &model_name,
                &self.db_models,
                &self.interfaces,
                &includes,
            ) {
                Ok(projected) => projected,
                Err(message) => {
                    self.error_at_with_help(
                        "E3304",
                        message,
                        args.get(1).map_or(args[0].span, |options| options.span),
                        vec!["check the related Model projection fields".to_string()],
                    );
                    Type::Named(model_name.clone())
                }
            }
        } else if self
            .db_backend
            .as_ref()
            .is_some_and(|backend| *backend == crate::db::DatabaseBackend::Mongodb)
        {
            crate::db::mongo_model_result_type(&fields)
        } else {
            Type::Named(model_name.clone())
        };
        result = db_promise(Type::Result {
            success: Box::new(if count {
                Type::U64
            } else if single {
                Type::Optional(Box::new(row_type.clone()))
            } else if pagination {
                db_page_type(&row_type)
            } else {
                Type::Array(Box::new(row_type))
            }),
            error: Box::new(error_type.clone()),
        });
        let mongo_backend = self.db_backend == Some(crate::db::DatabaseBackend::Mongodb);
        let mongo_parameter_fields = if mongo_backend {
            crate::db::mongo_model_read(
                &model,
                &args[0],
                if count {
                    crate::db::MongoReadKind::Count
                } else if single {
                    crate::db::MongoReadKind::FindOne
                } else {
                    crate::db::MongoReadKind::FindMany
                },
                &self.interfaces,
            )
            .map(|query| query.parameter_fields)
            .unwrap_or_default()
        } else {
            Vec::new()
        };
        let query = match if mongo_backend {
            crate::db::mongo_model_read(
                &model,
                &args[0],
                if count {
                    crate::db::MongoReadKind::Count
                } else if single {
                    crate::db::MongoReadKind::FindOne
                } else {
                    crate::db::MongoReadKind::FindMany
                },
                &self.interfaces,
            )
            .map(|query| crate::db::OrmSelect {
                sql: String::new(),
                parameters: query.parameters,
            })
        } else if count {
            crate::db::find_many_query(&model, &args[0])
        } else if single {
            crate::db::find_one_query(&model, &args[0])
        } else if pagination {
            crate::db::find_many_page_query(&model, &args[0])
        } else {
            crate::db::find_many_query(&model, &args[0])
        } {
            Ok(query) => query,
            Err(message) => {
                self.error_at_with_help(
                    "E3304",
                    message,
                    args[0].span,
                    vec!["use model field names and supported comparison operators".to_string()],
                );
                return result;
            }
        };
        let mut parameter_types = Vec::with_capacity(query.parameters.len());
        for parameter in &query.parameters {
            let known = self.expression_types_by_span.get(&parameter.span).cloned();
            let parameter_type = match known {
                Some(ty) if ty != Type::Unknown => ty,
                _ => {
                    let inferred = self.check_expr(parameter);
                    value_type_to_type(&inferred).unwrap_or(Type::Unknown)
                }
            };
            parameter_types.push(parameter_type);
        }
        if mongo_backend {
            for (parameter, field_name) in query.parameters.iter().zip(&mongo_parameter_fields) {
                let expected = if field_name == "_id" {
                    Some(Type::Str)
                } else {
                    fields
                        .iter()
                        .find(|field| field.name == *field_name)
                        .map(|field| field.ty.clone())
                };
                let Some(expected) = expected else {
                    continue;
                };
                let actual = self
                    .expression_types_by_span
                    .get(&parameter.span)
                    .cloned()
                    .filter(|ty| *ty != Type::Unknown)
                    .unwrap_or_else(|| {
                        value_type_to_type(&self.check_expr(parameter)).unwrap_or(Type::Unknown)
                    });
                if actual != Type::Unknown
                    && !self.is_assignable(&expected, &ValueType::Known(actual.clone()))
                {
                    self.error_at_with_help(
                        "E3304",
                        format!("MongoDB filter for `{field_name}` expects `{expected}`, found `{actual}`"),
                        parameter.span,
                        vec![format!("pass a value compatible with `{expected}` for `{field_name}`")],
                    );
                }
            }
        }
        if mongo_backend || !self.validate_db_schema {
            return result;
        }
        let Some(schema) = self.db_schema.as_ref() else {
            let detail = self
                .db_schema_error
                .as_deref()
                .unwrap_or("database schema snapshot is unavailable");
            self.error_at_with_help(
                "E3301",
                format!("cannot validate `Model.{method_name}`: {detail}"),
                args[0].span,
                vec![
                    "configure `database` in tyrus.json and run `tyrus db introspect`".to_string(),
                ],
            );
            return result;
        };
        let parsed = match crate::db::parse_select_sql(&query.sql) {
            Ok(parsed) => parsed,
            Err(error) => {
                self.error_at_with_help(
                    "E3304",
                    error.message,
                    args[0].span,
                    vec!["use a supported field comparison in the filter".to_string()],
                );
                return result;
            }
        };
        let scalar_fields = fields
            .iter()
            .filter(|field| {
                !model
                    .relations
                    .iter()
                    .any(|relation| relation.field == field.name)
            })
            .cloned()
            .collect::<Vec<_>>();
        let field_columns = model
            .fields
            .iter()
            .map(|field| (field.field.clone(), field.column.clone()))
            .collect::<crate::collections::HashMap<_, _>>();
        if let Err(error) = crate::db::validate_select_against_snapshot_with_mappings(
            schema,
            &parsed,
            &scalar_fields,
            &parameter_types,
            &field_columns,
        ) {
            let source_span = parsed
                .parameter_spans
                .iter()
                .position(|parameter_span| *parameter_span == error.span)
                .and_then(|index| query.parameters.get(index))
                .map_or(args[0].span, |parameter| parameter.span);
            self.error_at_with_help(
                "E3302",
                error.message,
                source_span,
                vec!["correct the filter value types or refresh the schema snapshot".to_string()],
            );
        }
        result
    }

    fn check_db_find_many_pagination(&mut self, args: &[Expr]) -> bool {
        if args.len() < 2 {
            return false;
        }
        let ExprKind::ObjectLit(options) = args[1].kind() else {
            self.error_at_with_help(
                "E3304",
                "`findMany` pagination options must be an object literal".to_string(),
                args[1].span,
                vec!["pass `{ paginate: true, pageSize: 25 }` as the second argument".to_string()],
            );
            return false;
        };
        let allowed = [
            "paginate",
            "pageSize",
            "page",
            "cursor",
            "after",
            "include",
            "relationLimit",
            "transaction",
        ];
        for (name, value) in options {
            if !allowed.contains(&name.as_str()) {
                self.error_at_with_help(
                    "E3304",
                    format!("unknown `findMany` pagination option `{name}`"),
                    value.span,
                    vec!["use `paginate`, `pageSize`, `page`, `cursor`, or `after`".to_string()],
                );
            }
        }
        let paginate = options.iter().find(|(name, _)| name == "paginate");
        let Some((_, paginate_value)) = paginate else {
            if options.iter().any(|(name, _)| name == "transaction") {
                return false;
            }
            if options.iter().any(|(name, _)| name == "include")
                && options.iter().all(|(name, _)| {
                    matches!(name.as_str(), "include" | "relationLimit" | "transaction")
                })
            {
                return false;
            }
            self.error_at_with_help(
                "E3304",
                "pagination options require `paginate: true`".to_string(),
                args[1].span,
                vec!["set `paginate: true` to enable pagination".to_string()],
            );
            return false;
        };
        if !matches!(paginate_value.kind(), ExprKind::BoolLit(true)) {
            self.error_at_with_help(
                "E3304",
                "`findMany` pagination requires the literal `paginate: true`".to_string(),
                paginate_value.span,
                vec!["set `paginate: true` or omit the options argument".to_string()],
            );
            return false;
        }
        let cursor_mode = options
            .iter()
            .find(|(name, _)| name == "cursor")
            .is_some_and(|(_, value)| matches!(value.kind(), ExprKind::BoolLit(true)));
        if let Some((_, cursor)) = options.iter().find(|(name, _)| name == "cursor")
            && !matches!(
                cursor.kind(),
                ExprKind::BoolLit(true) | ExprKind::BoolLit(false)
            )
        {
            self.error_at_with_help(
                "E3304",
                "`cursor` must be the literal `true` when cursor pagination is selected"
                    .to_string(),
                cursor.span,
                vec!["use `cursor: true` or omit `cursor` for numbered pagination".to_string()],
            );
        }
        if options.iter().all(|(name, _)| name != "pageSize") {
            self.error_at_with_help(
                "E3304",
                "numbered pagination requires `pageSize`".to_string(),
                args[1].span,
                vec!["set a positive integer `pageSize`".to_string()],
            );
            return false;
        };
        for name in if cursor_mode {
            ["pageSize", "after"]
        } else {
            ["pageSize", "page"]
        } {
            if let Some((_, value)) = options.iter().find(|(field, _)| field == name) {
                let actual = self.check_expr(value);
                let valid = if name == "after" {
                    matches!(
                        actual,
                        ValueType::Known(Type::Str) | ValueType::Unknown | ValueType::Error
                    )
                } else {
                    is_integer_value(&actual)
                };
                if !valid {
                    self.error_at_with_help(
                        "E3304",
                        if name == "after" {
                            "pagination option `after` must be a string cursor".to_string()
                        } else {
                            format!("pagination option `{name}` must be an integer")
                        },
                        value.span,
                        vec![if name == "after" {
                            "pass a `nextCursor` string returned by a previous cursor page"
                                .to_string()
                        } else {
                            format!("pass a positive integer for `{name}`")
                        }],
                    );
                }
            }
        }
        if cursor_mode && options.iter().any(|(name, _)| name == "page") {
            let page = options.iter().find(|(name, _)| name == "page").unwrap();
            self.error_at_with_help(
                "E3304",
                "`page` cannot be combined with cursor pagination".to_string(),
                page.1.span,
                vec!["remove `page` when `cursor: true` is set".to_string()],
            );
        }
        if !cursor_mode && let Some((_, after)) = options.iter().find(|(name, _)| name == "after") {
            self.error_at_with_help(
                "E3304",
                "`after` is only valid with cursor pagination".to_string(),
                after.span,
                vec!["set `cursor: true` or remove `after`".to_string()],
            );
        }
        true
    }

    fn check_db_transaction_options(&mut self, options: &Expr) {
        let ExprKind::ObjectLit(properties) = options.kind() else {
            self.error_at_with_help(
                "E3304",
                "database transaction options must be an object literal".to_string(),
                options.span,
                vec!["pass `{ transaction }` as the options object".to_string()],
            );
            return;
        };
        for (name, value) in properties {
            if name != "transaction" {
                self.error_at_with_help(
                    "E3304",
                    format!("unknown database transaction option `{name}`"),
                    value.span,
                    vec!["use only the `transaction` option".to_string()],
                );
            } else {
                self.check_db_transaction_handle(value);
            }
        }
    }

    fn check_db_transaction_handle(&mut self, value: &Expr) {
        let actual = self.check_expr(value);
        let expected = Type::U64;
        if !self.is_assignable(&expected, &actual) {
            self.error_at_with_help(
                "E3304",
                format!(
                    "database transaction option expects `{expected}`, found `{}`",
                    actual.display()
                ),
                value.span,
                vec!["pass the handle returned by `db.beginTransaction()`".to_string()],
            );
        }
    }

    pub(super) fn check_db_model_mutation_call(
        &mut self,
        method: &str,
        model_name: &str,
        args: &[Expr],
        span: Span,
    ) -> Type {
        let success = if self.db_backend == Some(crate::db::DatabaseBackend::Mongodb) {
            self.interfaces
                .get(model_name)
                .and_then(|fields| crate::db::mongo_model_mutation_result_type(method, fields))
                .unwrap_or(Type::Void)
        } else {
            crate::db::model_mutation_result_type(method, model_name).unwrap_or(Type::Void)
        };
        let result = db_promise(Type::Result {
            success: Box::new(success),
            error: Box::new(Type::Named("DbError".to_string())),
        });
        let Some(model) = self.db_models.get(model_name).cloned() else {
            self.error_at_with_help(
                "E3304",
                format!("type `{model_name}` is not a database model"),
                span,
                vec!["declare the interface with `@model(\"table_name\")`".to_string()],
            );
            return result;
        };
        let required_arguments = if matches!(
            method,
            "insertOne" | "insertMany" | "deleteOne" | "deleteMany"
        ) {
            1
        } else {
            2
        };
        if args.len() != required_arguments && args.len() != required_arguments + 1 {
            self.error_at_with_help(
                "E3304",
                format!(
                    "`Model.{method}` received an invalid number of arguments: {}",
                    args.len()
                ),
                Span::new(span.start, span.end),
                vec![
                    "pass the required arguments and optional `{ transaction }` options"
                        .to_string(),
                ],
            );
            return result;
        }
        if let Some(options) = args.get(required_arguments) {
            self.check_db_transaction_options(options);
        }
        let mongo_backend = self.db_backend == Some(crate::db::DatabaseBackend::Mongodb);
        if mongo_backend
            && args
                .get(required_arguments)
                .is_some_and(object_has_transaction_option)
        {
            let options = &args[required_arguments];
            self.error_at_with_help(
                "E3304",
                "MongoDB Model operations do not support transaction handles yet".to_string(),
                options.span,
                vec!["remove the `transaction` option when using MongoDB".to_string()],
            );
        }
        if !self.check_mongo_model_capabilities(model_name, &model) {
            return result;
        }
        let mut mongo_parameter_fields = Vec::new();
        let query = match if mongo_backend {
            crate::db::mongo_model_mutation(
                method,
                &model,
                &args[..required_arguments],
                &self.interfaces,
            )
            .map(|query| {
                mongo_parameter_fields = query.parameter_fields;
                crate::db::OrmSelect {
                    sql: String::new(),
                    parameters: query.parameters,
                }
            })
        } else {
            crate::db::mutation_query(method, &model, &args[..required_arguments])
        } {
            Ok(parameters) => parameters,
            Err(message) => {
                self.error_at_with_help(
                    "E3304",
                    message,
                    args.first().map_or(span, |arg| arg.span),
                    vec![
                        "use model fields and a non-empty filter for every update or delete"
                            .to_string(),
                    ],
                );
                return result;
            }
        };
        let parameter_types = query
            .parameters
            .iter()
            .map(|parameter| {
                self.expression_types_by_span
                    .get(&parameter.span)
                    .cloned()
                    .filter(|ty| *ty != Type::Unknown)
                    .or_else(|| value_type_to_type(&self.check_expr(parameter)))
                    .unwrap_or(Type::Unknown)
            })
            .collect::<Vec<_>>();
        if mongo_backend {
            for (parameter, field_name) in query.parameters.iter().zip(&mongo_parameter_fields) {
                let expected = if field_name == "_id" {
                    Some(Type::Str)
                } else {
                    model
                        .fields
                        .iter()
                        .find(|field| field.field == *field_name)
                        .and_then(|_| self.interfaces.get(model_name))
                        .and_then(|fields| fields.iter().find(|field| field.name == *field_name))
                        .map(|field| field.ty.clone())
                };
                let Some(expected) = expected else {
                    continue;
                };
                let actual = self
                    .expression_types_by_span
                    .get(&parameter.span)
                    .cloned()
                    .filter(|ty| *ty != Type::Unknown)
                    .unwrap_or_else(|| {
                        value_type_to_type(&self.check_expr(parameter)).unwrap_or(Type::Unknown)
                    });
                if actual != Type::Unknown
                    && !self.is_assignable(&expected, &ValueType::Known(actual.clone()))
                {
                    self.error_at_with_help(
                        "E3304",
                        format!("MongoDB value for `{field_name}` expects `{expected}`, found `{actual}`"),
                        parameter.span,
                        vec![format!("pass a value compatible with `{expected}` for `{field_name}`")],
                    );
                }
            }
            return result;
        }
        if !self.validate_db_schema {
            return result;
        }
        let Some(schema) = self.db_schema.clone() else {
            let detail = self
                .db_schema_error
                .as_deref()
                .unwrap_or("database schema snapshot is unavailable");
            self.error_at_with_help(
                "E3301",
                format!("cannot validate `Model.{method}`: {detail}"),
                span,
                vec![
                    "configure `database` in tyrus.json and run `tyrus db introspect`".to_string(),
                ],
            );
            return result;
        };
        if method == "insertOne" || method == "insertMany" {
            let rows = match args.first().map(Expr::kind) {
                Some(ExprKind::ObjectLit(fields)) if method == "insertOne" => {
                    vec![fields.as_slice()]
                }
                Some(ExprKind::ArrayLit(rows)) if method == "insertMany" => rows
                    .iter()
                    .filter_map(|row| match row.kind() {
                        ExprKind::ObjectLit(fields) => Some(fields.as_slice()),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            if let Some(table) = schema.tables.iter().find(|table| table.name == model.table) {
                for fields in rows {
                    for metadata in &model.fields {
                        if fields.iter().any(|(name, _)| name == &metadata.field) {
                            continue;
                        }
                        let Some(column) = table
                            .columns
                            .iter()
                            .find(|column| column.name == metadata.column)
                        else {
                            continue;
                        };
                        let generated_primary_key = metadata.id
                            && table.primary_key.len() == 1
                            && table.primary_key[0] == metadata.column;
                        if !column.nullable && column.default.is_none() && !generated_primary_key {
                            self.error_at_with_help(
                                "E3304",
                                format!(
                                    "insert into model `{model_name}` omits required field `{}`",
                                    metadata.field
                                ),
                                args[0].span,
                                vec![format!(
                                    "provide `{}` or give database column `{}.{}` a default",
                                    metadata.field, model.table, metadata.column
                                )],
                            );
                            return result;
                        }
                    }
                }
            }
        }
        let validation = crate::db::parse_exec_sql(&query.sql).and_then(|statement| {
            crate::db::validate_exec_against_snapshot(&schema, &statement, &parameter_types)
        });
        if let Err(error) = validation {
            self.error_at_with_help(
                "E3302",
                error.message,
                span,
                vec![
                    "correct the model values or refresh the database schema snapshot".to_string(),
                ],
            );
        }
        result
    }

    fn check_mongo_model_capabilities(
        &mut self,
        model_name: &str,
        model: &crate::ast::DbModelMetadata,
    ) -> bool {
        if self.db_backend != Some(crate::db::DatabaseBackend::Mongodb) {
            return true;
        }
        let mut supported = true;
        for field in &model.fields {
            if (field.id || field.field == "_id" || field.column == "_id")
                && let Some(declared) = self
                    .interfaces
                    .get(model_name)
                    .and_then(|fields| fields.iter().find(|item| item.name == field.field))
                && !self.is_assignable(&Type::Str, &ValueType::Known(declared.ty.clone()))
            {
                self.error_at_with_help(
                    "E3304",
                    format!(
                        "MongoDB identifier `{}.{}` must use the `ObjectId` type",
                        model_name, field.field
                    ),
                    declared.span,
                    vec![format!(
                        "change `{}.{}` to `ObjectId`",
                        model_name, field.field
                    )],
                );
                supported = false;
            }
            let unsupported = if field.unique {
                Some((
                    "`@unique`",
                    "MongoDB unique indexes must be created and managed explicitly",
                ))
            } else if field.default_value.is_some() {
                Some((
                    "`@default`",
                    "MongoDB Model defaults are not applied by Tyrus yet",
                ))
            } else if field.foreign_key.is_some() {
                Some((
                    "`@foreignKey`",
                    "MongoDB does not enforce relational foreign keys",
                ))
            } else {
                None
            };
            if let Some((metadata, reason)) = unsupported {
                self.error_at_with_help(
                    "E3304",
                    format!("Model `{model_name}` uses unsupported MongoDB metadata {metadata}: {reason}"),
                    field.span,
                    vec![format!("remove {metadata} from Model `{model_name}` or select a relational backend")],
                );
                supported = false;
            }
        }
        for relation in &model.relations {
            self.error_at_with_help(
                "E3304",
                format!(
                    "Model `{model_name}` relationship `{}` is not supported by MongoDB",
                    relation.field
                ),
                relation.span,
                vec![format!(
                    "remove relationship `{}` from Model `{model_name}` or select SQLite",
                    relation.field
                )],
            );
            supported = false;
        }
        supported
    }

    fn check_db_select_call(
        &mut self,
        function_name: &str,
        type_args: &[Type],
        args: &[Expr],
        span: Span,
    ) {
        let Some(source) = self.source_text.as_deref() else {
            return;
        };
        let captured =
            match crate::db::CapturedSqlCall::capture_arguments(source, args, span.to_range()) {
                Ok(captured) => captured,
                Err(error) => {
                    self.error_at_with_help(
                        "E3300",
                        error.message,
                        Span::new(error.span.start, error.span.end),
                        vec![
                        "write the SQL as a string literal and its parameters as an array literal"
                            .to_string(),
                    ],
                    );
                    return;
                }
            };
        let Some(Type::Named(result_name)) = type_args.first() else {
            self.error_at_with_help(
                "E3303",
                format!("`{function_name}` requires an explicit result interface type"),
                Span::new(span.start, span.start),
                vec![format!(
                    "call `{function_name}<UserRow>(sql, parameters)` with a declared interface"
                )],
            );
            return;
        };
        let Some(result_fields) = self.interfaces.get(result_name).cloned() else {
            self.error_at_with_help(
                "E3303",
                format!(
                    "`{function_name}` result type `{result_name}` must be a declared interface"
                ),
                Span::new(span.start, span.start),
                vec![format!(
                    "declare `interface {result_name}` with fields matching the selected columns"
                )],
            );
            return;
        };
        let Some(schema) = self.db_schema.as_ref() else {
            let detail = self
                .db_schema_error
                .as_deref()
                .unwrap_or("database schema snapshot is unavailable");
            self.error_at_with_help(
                "E3301",
                format!("cannot validate `{function_name}`: {detail}"),
                args[0].span,
                vec![
                    "configure `database` in tyrus.json and run `tyrus db introspect`".to_string(),
                ],
            );
            return;
        };
        let parameter_types = captured
            .parameters
            .iter()
            .map(|parameter| {
                self.expression_types_by_span
                    .get(&parameter.span)
                    .cloned()
                    .unwrap_or(Type::Unknown)
            })
            .collect::<Vec<_>>();
        let model_for_query = captured
            .parse_select()
            .ok()
            .filter(|query| query.select_all && query.joins.is_empty())
            .and_then(|query| {
                self.db_models
                    .get(result_name)
                    .filter(|model| model.table == query.table)
            });
        let field_columns = model_for_query
            .map(|model| {
                model
                    .fields
                    .iter()
                    .map(|field| (field.field.clone(), field.column.clone()))
                    .collect::<crate::collections::HashMap<_, _>>()
            })
            .unwrap_or_default();
        let validation = if field_columns.is_empty() {
            captured.validate_select_at_source(schema, &result_fields, &parameter_types)
        } else {
            captured.validate_select_at_source_with_mappings(
                schema,
                &result_fields,
                &parameter_types,
                &field_columns,
            )
        };
        if let Err(error) = validation {
            self.error_at_with_help(
                "E3302",
                error.message,
                Span::new(error.span.start, error.span.end),
                vec![
                    "correct the SQL or regenerate the snapshot with `tyrus db introspect`"
                        .to_string(),
                ],
            );
        }
    }
}

use crate::ast::{Expr, ExprKind, Type};

pub(super) fn db_page_type(row_type: &Type) -> Type {
    let fields = crate::builtin_interfaces::db_page_fields()
        .into_iter()
        .map(|mut field| {
            if field.name == "items" {
                field.ty = Type::Array(Box::new(row_type.clone()));
            }
            field
        })
        .collect();
    Type::Object(fields)
}

pub(super) fn db_promise(result: Type) -> Type {
    Type::Apply {
        name: "DbPromise".to_string(),
        args: vec![result],
    }
}

pub(super) fn object_has_transaction_option(expression: &Expr) -> bool {
    matches!(expression.kind(), ExprKind::ObjectLit(properties) if properties.iter().any(|(name, _)| name == "transaction"))
}
