use super::*;

impl TypeChecker {
    pub fn new() -> Self {
        let interfaces = crate::async_types::error_interfaces();
        Self {
            cancellation: None,
            exhaustive_matches: HashSet::default(),
            confined_closures: crate::escape_analysis::ConfinedClosures::default(),
            item_span: None,
            span_stack: Vec::new(),
            scopes: vec![HashMap::default()],
            constant_integers: vec![HashMap::default()],
            narrowed_paths: vec![HashSet::default()],
            declaration_depths: vec![HashMap::default()],
            import_targets: HashMap::default(),
            source_text: None,
            db_schema: None,
            db_schema_error: None,
            db_backend: None,
            validate_db_schema: true,
            db_models: HashMap::default(),
            expression_types_by_span: HashMap::default(),
            user_validators: HashMap::default(),
            entry_point: false,
            pending_entry_point_return_check: None,
            unresolved_namespaces: Vec::new(),
            validator_facts: vec![HashMap::default()],
            for_index_hint: None,
            diagnostics: Vec::new(),
            expected_return: None,
            inferred_returns: Vec::new(),
            inferred_onfail_results: Vec::new(),
            expressions: Vec::new(),
            contextual_parameters: Vec::new(),
            onfail_bindings: Vec::new(),
            resolved_bindings: Vec::new(),
            interfaces,
            data_interfaces: HashSet::default(),
            reachable_only_types: HashSet::default(),
            type_aliases: HashMap::default(),
            generic_defs: crate::type_resolve::GenericDefs::default(),
            type_param_constraints: Vec::new(),
            type_param_names: Vec::new(),
            enums: HashMap::default(),
            fn_param_metadata: HashMap::default(),
            fn_body_metadata: HashMap::default(),
            extension_methods: Vec::new(),
            behavior_targets: HashMap::default(),
            fn_throws: HashMap::default(),
            async_fns: std::collections::HashSet::default(),
            current_throws: Vec::new(),
            control_stack: Vec::new(),
            try_catch_type: Vec::new(),
            awaited_operand: None,
            try_body_suspends: Vec::new(),
            try_catch_suspends: Vec::new(),
            try_finally_suspends: Vec::new(),
            error_catch_bindings: Vec::new(),
            finalizer_scopes: vec![Vec::new()],
            finalizer_contexts: Vec::new(),
            branch_depth: 0,
            fn_depth: 0,
            env_fields: None,
            topic_stack: Vec::new(),
            async_depth: 0,
            async_function_stack: Vec::new(),
            joined_handle_names: vec![HashMap::default()],
            awaited_promise_names: vec![HashMap::default()],
            mutex_guard_names: vec![HashSet::default()],
        }
    }

    pub fn set_cancellation(
        &mut self,
        cancellation: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) {
        self.cancellation = Some(cancellation);
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancellation
            .as_ref()
            .is_some_and(|signal| signal.load(std::sync::atomic::Ordering::Acquire))
    }

    pub fn set_source_text(&mut self, source: &str) {
        self.source_text = Some(source.to_string());
    }

    pub fn set_db_schema(
        &mut self,
        schema: Option<crate::db::DbSchemaSnapshot>,
        error: Option<&str>,
    ) {
        self.db_schema = schema;
        self.db_schema_error = error.map(str::to_string);
    }

    pub fn set_db_schema_validation(&mut self, enabled: bool) {
        self.validate_db_schema = enabled;
    }

    pub fn set_db_backend(&mut self, backend: Option<crate::db::DatabaseBackend>) {
        self.db_backend = backend;
    }

    pub fn set_import_targets(&mut self, targets: HashMap<String, String>) {
        self.import_targets = targets;
    }

    pub fn set_entry_point(&mut self, entry_point: bool) {
        self.entry_point = entry_point;
    }

    pub fn set_env_schema(&mut self, fields: Vec<Field>) {
        self.env_fields = Some(fields);
    }

    pub fn check_program(self, program: &Program) -> Vec<CheckDiagnostic> {
        let types = HashMap::default();
        let interfaces = HashMap::default();
        let behaviors = HashMap::default();
        let type_aliases = HashMap::default();
        let enums = HashMap::default();
        let throws = HashMap::default();
        let reachable_interfaces = HashMap::default();
        let validators = HashMap::default();
        let db_models = HashMap::default();
        self.analyze_program_with_imports(
            program,
            ImportedProgramSymbols {
                types: &types,
                interfaces: &interfaces,
                behaviors: &behaviors,
                type_aliases: &type_aliases,
                enums: &enums,
                throws: &throws,
                db_models: &db_models,
                validators: &validators,
                reachable_interfaces: &reachable_interfaces,
            },
        )
        .diagnostics
    }

    pub fn check_program_with_imports(
        self,
        program: &Program,
        imports: &HashMap<String, Type>,
    ) -> Vec<CheckDiagnostic> {
        let interfaces = HashMap::default();
        let behaviors = HashMap::default();
        let type_aliases = HashMap::default();
        let enums = HashMap::default();
        let throws = HashMap::default();
        let reachable_interfaces = HashMap::default();
        let validators = HashMap::default();
        let db_models = HashMap::default();
        self.analyze_program_with_imports(
            program,
            ImportedProgramSymbols {
                types: imports,
                interfaces: &interfaces,
                behaviors: &behaviors,
                type_aliases: &type_aliases,
                enums: &enums,
                throws: &throws,
                db_models: &db_models,
                validators: &validators,
                reachable_interfaces: &reachable_interfaces,
            },
        )
        .diagnostics
    }

    pub fn analyze_program_with_imports(
        mut self,
        program: &Program,
        imports: ImportedProgramSymbols<'_>,
    ) -> CheckResult {
        macro_rules! return_if_cancelled {
            () => {
                if self.is_cancelled() {
                    return CheckResult::default();
                }
            };
        }
        return_if_cancelled!();
        let ImportedProgramSymbols {
            types: import_types,
            interfaces: import_interfaces,
            behaviors: import_behaviors,
            type_aliases: import_type_aliases,
            enums: import_enums,
            throws: import_throws,
            db_models: import_db_models,
            reachable_interfaces: import_reachable_interfaces,
            validators: import_validators,
        } = imports;
        let mut program = program.clone();
        self.user_validators.extend(import_validators.clone());
        self.db_models.extend(import_db_models.clone());
        for item in &program.items {
            return_if_cancelled!();
            if let Item::FnDecl {
                name,
                is_validator: true,
                params,
                ret_type,
                ..
            } = item
            {
                self.user_validators.insert(
                    name.clone(),
                    Type::Function {
                        params: params.iter().map(|param| param.ty.clone()).collect(),
                        ret: Box::new(ret_type.clone()),
                    },
                );
            }
        }
        crate::validators::resolve_membership_decorator_constants(&mut program, import_enums);
        let program = &program;
        self.confined_closures = crate::escape_analysis::analyze_confined_closures(program);
        self.check_entry_point(program);
        self.fn_throws.extend(import_throws.clone());
        self.interfaces
            .extend(crate::builtin_interfaces::ambient_builtin_interfaces());
        self.interfaces.insert(
            "DigestStream".to_string(),
            crate::crypto::digest_stream_methods(),
        );
        for (name, fields) in crate::crypto::crypto_namespace_interface_fields() {
            self.interfaces.entry(name).or_insert(fields);
        }
        for (name, fields) in crate::encoding::encoding_namespace_interface_fields() {
            self.interfaces.entry(name).or_insert(fields);
        }
        for interface in crate::cli::option_interfaces() {
            self.interfaces
                .insert(interface.name.to_string(), interface.fields);
        }
        let declared_here = program
            .items
            .iter()
            .filter_map(|item| match item {
                Item::InterfaceDecl { name, .. }
                | Item::EnumDecl { name, .. }
                | Item::TypeAlias { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect::<HashSet<_>>();
        for (name, fields) in import_reachable_interfaces {
            if declared_here.contains(name.as_str())
                || import_interfaces.contains_key(name)
                || import_type_aliases.contains_key(name)
                || import_enums.contains_key(name)
                || self.interfaces.contains_key(name)
            {
                continue;
            }
            self.interfaces.insert(name.clone(), fields.clone());
            self.data_interfaces.insert(name.clone());
            self.reachable_only_types.insert(name.clone());
        }
        let builtins = crate::builtin_interfaces::builtin_interfaces();
        for (name, fields) in import_interfaces {
            self.interfaces
                .entry(name.clone())
                .or_insert_with(|| fields.clone());
            if !builtins.contains_key(name) {
                self.data_interfaces.insert(name.clone());
            }
        }
        let mut behavior_targets = HashMap::default();
        for (name, behavior) in import_behaviors {
            self.interfaces.entry(name.clone()).or_default();
            behavior_targets.insert(name.clone(), behavior.targets.clone());
        }
        for (name, ty) in import_type_aliases {
            self.type_aliases
                .entry(name.clone())
                .or_insert_with(|| ty.clone());
        }
        for (name, enum_info) in import_enums {
            self.enums
                .entry(name.clone())
                .or_insert_with(|| enum_info.clone());
        }
        self.validate_native_promise_name_conflicts(program);
        self.generic_defs = crate::type_resolve::GenericDefs::collect(program);
        for (name, fields) in import_interfaces
            .iter()
            .chain(import_reachable_interfaces.iter())
        {
            self.generic_defs
                .register_imported_interface(name, fields.clone());
        }
        if import_types.contains_key("Page") {
            self.generic_defs.register_db_page();
        }
        for (alias, ty) in import_type_aliases {
            if let Type::Named(canonical) = ty {
                self.generic_defs.register_stream_alias(alias, canonical);
            }
        }
        for item in &program.items {
            return_if_cancelled!();
            if let Item::BehaviorInterfaceDecl { name, targets, .. } = item {
                behavior_targets.insert(name.clone(), targets.clone());
                self.interfaces.entry(name.clone()).or_default();
            }
        }
        for _ in 0..program.items.len() {
            return_if_cancelled!();
            let mut changed = false;
            for (name, behavior) in import_behaviors {
                let inherited = behavior
                    .extends
                    .iter()
                    .filter_map(|ty| match ty {
                        Type::Named(super_name) => behavior_targets.get(super_name).cloned(),
                        _ => None,
                    })
                    .flatten()
                    .collect::<Vec<_>>();
                let entry = behavior_targets.entry(name.clone()).or_default();
                for target in inherited {
                    if !entry.contains(&target) {
                        entry.push(target);
                        changed = true;
                    }
                }
            }
            for item in &program.items {
                return_if_cancelled!();
                let Item::BehaviorInterfaceDecl { name, extends, .. } = item else {
                    continue;
                };
                let inherited = extends
                    .iter()
                    .filter_map(|ty| match ty {
                        Type::Named(super_name) => behavior_targets.get(super_name).cloned(),
                        _ => None,
                    })
                    .flatten()
                    .collect::<Vec<_>>();
                let entry = behavior_targets.entry(name.clone()).or_default();
                for target in inherited {
                    if !entry.contains(&target) {
                        entry.push(target);
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        self.behavior_targets.clone_from(&behavior_targets);
        for (name, behavior) in import_behaviors {
            return_if_cancelled!();
            let targets = behavior_targets
                .get(name)
                .cloned()
                .unwrap_or_else(|| behavior.targets.clone());
            for target in targets {
                for method in behavior.methods.iter().chain(&behavior.binding_methods) {
                    if method.params.is_empty() {
                        continue;
                    }
                    self.extension_methods.push(ExtensionMethod {
                        owner: name.clone(),
                        target: target.clone(),
                        name: method.name.clone(),
                        params: specialize_extension_receiver(&method.params, &target),
                        ret_type: method.ret_type.clone(),
                    });
                }
            }
        }
        for item in &program.items {
            return_if_cancelled!();
            let Item::BehaviorInterfaceDecl { name, methods, .. } = item else {
                continue;
            };
            for target in behavior_targets.get(name).cloned().unwrap_or_default() {
                for method in methods {
                    if method.params.is_empty() {
                        self.error_at(
                            "E0315",
                            format!("extension method `{}` must declare its receiver as the first parameter", method.name),
                            method.span,
                        );
                        continue;
                    }
                    let params = specialize_extension_receiver(&method.params, &target);
                    let receiver = &params[0].ty;
                    if !self.is_assignable(receiver, &ValueType::Known(target.clone())) {
                        self.error_at(
                            "E0308",
                            format!("receiver of extension method `{}` must accept `{target}`, found `{receiver}`", method.name),
                            method.params[0].span,
                        );
                    }
                    self.extension_methods.push(ExtensionMethod {
                        owner: name.clone(),
                        target: target.clone(),
                        name: method.name.clone(),
                        params,
                        ret_type: method.ret_type.clone(),
                    });
                }
            }
        }
        self.register_declared_types(program);
        self.register_imports(program, import_types);
        self.register_function_symbols(program);
        for item in &program.items {
            return_if_cancelled!();
            let Item::BehaviorInterfaceDecl { name, bindings, .. } = item else {
                continue;
            };
            let targets = behavior_targets.get(name).cloned().unwrap_or_default();
            for binding in bindings {
                let Some(ValueType::Known(Type::Function { params, ret })) =
                    self.lookup(binding).map(|symbol| symbol.ty.clone())
                else {
                    self.error_at(
                        "E0433",
                        format!("shorthand extension binding `{binding}` must name a function"),
                        item.span(),
                    );
                    continue;
                };
                if params.is_empty() {
                    self.error_at("E0315", format!("extension binding `{binding}` must accept its receiver as the first parameter"), item.span());
                    continue;
                }
                for target in &targets {
                    if !self.is_assignable(&params[0], &ValueType::Known(target.clone())) {
                        self.error_at("E0308", format!("receiver of extension binding `{binding}` must accept `{target}`, found `{}`", params[0]), item.span());
                    }
                    self.extension_methods.push(ExtensionMethod {
                        owner: name.clone(),
                        target: target.clone(),
                        name: binding.clone(),
                        params: params
                            .iter()
                            .enumerate()
                            .map(|(index, ty)| Param {
                                pattern: ParamPattern::Name(format!("__at_param_{index}")),
                                ty: ty.clone(),
                                mutable: false,
                                optional: false,
                                rest: false,
                                default_value: None,
                                decorators: Vec::new(),
                                span: item.span(),
                            })
                            .collect(),
                        ret_type: (*ret).clone(),
                    });
                }
            }
        }
        for item in &program.items {
            return_if_cancelled!();
            self.item_span = Some(item.span());
            self.check_item(item);
        }
        return_if_cancelled!();
        self.check_entry_point_inferred_return();
        self.report_unused_imports();
        return_if_cancelled!();
        self.diagnostics
            .extend(crate::definite_assignment::check(program));
        let resolved_symbols = self
            .scopes
            .first()
            .map(|scope| {
                scope
                    .iter()
                    .filter_map(|(name, symbol)| match &symbol.ty {
                        ValueType::Known(ty) => Some((name.clone(), ty.clone())),
                        _ => None,
                    })
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();
        CheckResult {
            diagnostics: self.diagnostics,
            expressions: self.expressions,
            contextual_parameters: self.contextual_parameters,
            onfail_bindings: self.onfail_bindings,
            resolved_symbols,
            resolved_bindings: self.resolved_bindings,
        }
    }

    fn register_declared_types(&mut self, program: &Program) {
        let mut db_model_tables = HashMap::default();
        for item in &program.items {
            self.item_span = Some(item.span());
            match item {
                Item::InterfaceDecl {
                    name,
                    fields,
                    type_params,
                    from_type_alias,
                    db_model,
                    ..
                } => {
                    self.validate_field_decorators(fields);
                    if let Some(model) = db_model {
                        self.db_models.insert(name.clone(), model.clone());
                        if let Some(previous) = db_model_tables.insert(model.table.clone(), name) {
                            self.error_at_with_help(
                                "E3304",
                                format!("interfaces `{previous}` and `{name}` both map to database table `{}`", model.table),
                                model.span,
                                vec!["map each database table to exactly one model interface".to_string()],
                            );
                        }
                        self.validate_db_model(name, fields, model);
                    }
                    let merged = if type_params.is_empty() {
                        let mut diagnostics = Vec::new();
                        let fields = self
                            .generic_defs
                            .interface_fields(name, &mut diagnostics)
                            .unwrap_or_else(|| fields.clone());
                        for diagnostic in diagnostics {
                            self.error(
                                diagnostic.code,
                                diagnostic.message,
                                diagnostic.subject.as_deref(),
                            );
                        }
                        fields
                    } else {
                        fields.clone()
                    };
                    if self.interfaces.insert(name.clone(), merged).is_some() {
                        self.error(
                            "E0428",
                            format!("interface `{name}` is declared more than once"),
                            Some(name),
                        );
                    }
                    if !from_type_alias {
                        self.data_interfaces.insert(name.clone());
                    }
                }
                Item::EnumDecl {
                    name,
                    repr,
                    members,
                    ..
                } => {
                    let (resolved, diagnostics) = resolve_enum(repr, members);
                    for diagnostic in diagnostics {
                        self.error(
                            diagnostic.code,
                            diagnostic.message,
                            diagnostic.member.as_deref().or(Some(name.as_str())),
                        );
                    }
                    if self.enums.insert(name.clone(), resolved).is_some() {
                        self.error(
                            "E0428",
                            format!("enum `{name}` is declared more than once"),
                            Some(name),
                        );
                    }
                    self.declare(
                        name,
                        Symbol {
                            ty: ValueType::Known(Type::Named(name.clone())),
                            mutable: false,
                            kind: SymbolKind::Variable,
                            used: false,
                            array_len: None,
                        },
                    );
                }
                _ => {}
            }
        }
        for item in &program.items {
            let Item::InterfaceDecl {
                name,
                db_model: Some(model),
                ..
            } = item
            else {
                continue;
            };
            self.validate_db_model_relations(name, model);
        }
    }

    fn validate_db_model_relations(&mut self, name: &str, model: &crate::ast::DbModelMetadata) {
        for relation in &model.relations {
            let Some(target) = self.db_models.get(&relation.target_model).cloned() else {
                self.error_at_with_help(
                    "E3304",
                    format!(
                        "relation `{}.{}` targets `{}`, which is not a database Model",
                        name, relation.field, relation.target_model
                    ),
                    relation.span,
                    vec![format!(
                        "declare `{}` with `@model(\"table\")`",
                        relation.target_model
                    )],
                );
                continue;
            };
            let (source_field_name, target_field_name) = match relation.kind {
                crate::ast::DbModelRelationKind::BelongsTo => {
                    let Some(local_key) = model
                        .fields
                        .iter()
                        .find(|field| field.field == relation.foreign_key_field)
                    else {
                        self.invalid_db_relation(
                            name,
                            relation,
                            "local foreign-key field is missing from the Model",
                        );
                        continue;
                    };
                    let Some((table, column)) = &local_key.foreign_key else {
                        self.invalid_db_relation(
                            name,
                            relation,
                            "local field has no matching `@foreignKey` metadata",
                        );
                        continue;
                    };
                    if !table.eq_ignore_ascii_case(&target.table) {
                        self.invalid_db_relation(
                            name,
                            relation,
                            "`@foreignKey` target table does not match the related Model",
                        );
                        continue;
                    }
                    let Some(target_key) = target.fields.iter().find(|field| {
                        field.column.eq_ignore_ascii_case(column) && (field.id || field.unique)
                    }) else {
                        self.invalid_db_relation(
                            name,
                            relation,
                            "related key must be marked `@id` or `@unique`",
                        );
                        continue;
                    };
                    (local_key.field.as_str(), target_key.field.as_str())
                }
                crate::ast::DbModelRelationKind::HasOne
                | crate::ast::DbModelRelationKind::HasMany => {
                    let Some(target_key) = target
                        .fields
                        .iter()
                        .find(|field| field.field == relation.foreign_key_field)
                    else {
                        self.invalid_db_relation(
                            name,
                            relation,
                            "related Model foreign-key field is missing",
                        );
                        continue;
                    };
                    let Some((table, column)) = &target_key.foreign_key else {
                        self.invalid_db_relation(
                            name,
                            relation,
                            "related field has no matching `@foreignKey` metadata",
                        );
                        continue;
                    };
                    if !table.eq_ignore_ascii_case(&model.table) {
                        self.invalid_db_relation(
                            name,
                            relation,
                            "related field `@foreignKey` targets a different table",
                        );
                        continue;
                    }
                    let Some(source_key) = model
                        .fields
                        .iter()
                        .find(|field| field.column.eq_ignore_ascii_case(column))
                    else {
                        self.invalid_db_relation(
                            name,
                            relation,
                            "related foreign key references a missing source field",
                        );
                        continue;
                    };
                    if !source_key.id && !source_key.unique {
                        self.invalid_db_relation(
                            name,
                            relation,
                            "related foreign key must reference a unique field or `@id`",
                        );
                        continue;
                    }
                    if relation.kind == crate::ast::DbModelRelationKind::HasOne
                        && !target_key.id
                        && !target_key.unique
                    {
                        self.invalid_db_relation(
                            name,
                            relation,
                            "`@hasOne` requires a unique foreign-key field",
                        );
                        continue;
                    }
                    (source_key.field.as_str(), target_key.field.as_str())
                }
                crate::ast::DbModelRelationKind::ManyToMany => {
                    let source_key = model.fields.iter().find(|field| field.id);
                    let target_key = target.fields.iter().find(|field| field.id);
                    let pivot_valid = !self.validate_db_schema
                        || match (
                            relation.pivot_table.as_deref(),
                            relation.pivot_source_column.as_deref(),
                            relation.pivot_target_column.as_deref(),
                            self.db_schema.as_ref(),
                        ) {
                            (
                                Some(table_name),
                                Some(source_column),
                                Some(target_column),
                                Some(schema),
                            ) => schema
                                .tables
                                .iter()
                                .find(|table| table.name.eq_ignore_ascii_case(table_name))
                                .is_some_and(|table| {
                                    table.foreign_keys.iter().any(|foreign_key| {
                                        foreign_key.columns.len() == 1
                                            && foreign_key.columns[0]
                                                .eq_ignore_ascii_case(source_column)
                                            && foreign_key
                                                .referenced_table
                                                .eq_ignore_ascii_case(&model.table)
                                            && source_key.is_some_and(|key| {
                                                foreign_key.referenced_columns.len() == 1
                                                    && foreign_key.referenced_columns[0]
                                                        .eq_ignore_ascii_case(&key.column)
                                            })
                                    }) && table.foreign_keys.iter().any(|foreign_key| {
                                        foreign_key.columns.len() == 1
                                            && foreign_key.columns[0]
                                                .eq_ignore_ascii_case(target_column)
                                            && foreign_key
                                                .referenced_table
                                                .eq_ignore_ascii_case(&target.table)
                                            && target_key.is_some_and(|key| {
                                                foreign_key.referenced_columns.len() == 1
                                                    && foreign_key.referenced_columns[0]
                                                        .eq_ignore_ascii_case(&key.column)
                                            })
                                    })
                                }),
                            _ => false,
                        };
                    if !pivot_valid {
                        self.invalid_db_relation(
                            name,
                            relation,
                            "pivot table and both foreign keys must match the schema snapshot",
                        );
                        continue;
                    }
                    let (Some(source_key), Some(target_key)) = (source_key, target_key) else {
                        self.invalid_db_relation(
                            name,
                            relation,
                            "many-to-many models require primary keys",
                        );
                        continue;
                    };
                    (source_key.field.as_str(), target_key.field.as_str())
                }
            };
            let source_field = self
                .interfaces
                .get(name)
                .and_then(|fields| fields.iter().find(|field| field.name == source_field_name));
            let target_field = self
                .interfaces
                .get(&relation.target_model)
                .and_then(|fields| fields.iter().find(|field| field.name == target_field_name));
            let (Some(source_field), Some(target_field)) = (source_field, target_field) else {
                self.invalid_db_relation(name, relation, "relationship key type is unavailable");
                continue;
            };
            let source_type = match &source_field.ty {
                crate::ast::Type::Optional(inner) => inner.as_ref(),
                ty => ty,
            };
            let target_type = match &target_field.ty {
                crate::ast::Type::Optional(inner) => inner.as_ref(),
                ty => ty,
            };
            if source_type != target_type {
                self.invalid_db_relation(name, relation, "relationship key types do not match");
            }
        }
    }

    fn invalid_db_relation(
        &mut self,
        model: &str,
        relation: &crate::ast::DbModelRelationMetadata,
        reason: &str,
    ) {
        self.error_at_with_help(
            "E3304",
            format!(
                "relation `{}.{}` is invalid: {reason}",
                model, relation.field
            ),
            relation.span,
            vec![
                "match the relation metadata to Model fields and a compatible `@foreignKey`"
                    .to_string(),
            ],
        );
    }

    fn validate_db_model(
        &mut self,
        name: &str,
        fields: &[crate::ast::Field],
        model: &crate::ast::DbModelMetadata,
    ) {
        self.validate_db_model_id_strategies(name, fields, model);
        for metadata in &model.fields {
            let Some(default) = &metadata.default_value else {
                continue;
            };
            let Some(field) = fields.iter().find(|field| field.name == metadata.field) else {
                continue;
            };
            let field_type = match &field.ty {
                crate::ast::Type::Optional(inner) => inner.as_ref(),
                other => other,
            };
            let compatible = match default.kind() {
                crate::ast::ExprKind::StrLit(_) => *field_type == crate::ast::Type::Str,
                crate::ast::ExprKind::BoolLit(_) => *field_type == crate::ast::Type::Bool,
                crate::ast::ExprKind::IntLit(value) => match field_type {
                    crate::ast::Type::I8
                    | crate::ast::Type::I16
                    | crate::ast::Type::I32
                    | crate::ast::Type::I64
                    | crate::ast::Type::I128
                    | crate::ast::Type::Isize => true,
                    crate::ast::Type::U8
                    | crate::ast::Type::U16
                    | crate::ast::Type::U32
                    | crate::ast::Type::U64
                    | crate::ast::Type::U128
                    | crate::ast::Type::Usize => *value >= 0,
                    crate::ast::Type::F32 | crate::ast::Type::F64 | crate::ast::Type::Decimal => {
                        true
                    }
                    _ => false,
                },
                crate::ast::ExprKind::UIntLit(_) => matches!(
                    field_type,
                    crate::ast::Type::U8
                        | crate::ast::Type::U16
                        | crate::ast::Type::U32
                        | crate::ast::Type::U64
                        | crate::ast::Type::U128
                        | crate::ast::Type::Usize
                        | crate::ast::Type::I8
                        | crate::ast::Type::I16
                        | crate::ast::Type::I32
                        | crate::ast::Type::I64
                        | crate::ast::Type::I128
                        | crate::ast::Type::Isize
                        | crate::ast::Type::F32
                        | crate::ast::Type::F64
                        | crate::ast::Type::Decimal
                ),
                crate::ast::ExprKind::FloatLit(_) => matches!(
                    field_type,
                    crate::ast::Type::F32 | crate::ast::Type::F64 | crate::ast::Type::Decimal
                ),
                _ => false,
            };
            if !compatible {
                self.error_at_with_help(
                    "E3304",
                    format!(
                        "default value for model field `{}.{}` is incompatible with type `{}`",
                        name, metadata.field, field.ty
                    ),
                    metadata.span,
                    vec![format!("use a literal compatible with `{}`", field.ty)],
                );
            }
        }
        if self.db_backend == Some(crate::db::DatabaseBackend::Mongodb) {
            return;
        }
        if !self.validate_db_schema {
            return;
        }
        let Some(snapshot) = self.db_schema.as_ref() else {
            return;
        };
        let Some(table) = snapshot
            .tables
            .iter()
            .find(|table| table.name == model.table)
            .cloned()
        else {
            self.error_at_with_help(
                "E3304",
                format!(
                    "model `{name}` maps to unknown database table `{}`",
                    model.table
                ),
                model.span,
                vec![
                    "create the table and refresh the local snapshot with `tyrus db introspect`"
                        .to_string(),
                ],
            );
            return;
        };
        let mut mapped_columns = std::collections::HashSet::new();
        let mut id_fields = Vec::new();
        for metadata in &model.fields {
            if !mapped_columns.insert(metadata.column.as_str()) {
                self.error_at_with_help(
                    "E3304",
                    format!(
                        "model `{name}` maps more than one field to column `{}`",
                        metadata.column
                    ),
                    metadata.span,
                    vec!["map each model field to a distinct database column".to_string()],
                );
                continue;
            }
            let Some(field) = fields.iter().find(|field| field.name == metadata.field) else {
                continue;
            };
            let Some(column) = table
                .columns
                .iter()
                .find(|column| column.name == metadata.column)
            else {
                self.error_at_with_help(
                    "E3304",
                    format!(
                        "model field `{}.{}` maps to unknown column `{}` in `{}`",
                        name, metadata.field, metadata.column, model.table
                    ),
                    metadata.span,
                    vec!["correct the `@column` mapping or update the schema snapshot".to_string()],
                );
                continue;
            };
            if metadata.id {
                id_fields.push(metadata);
            }
            let (value_type, optional) = match &field.ty {
                crate::ast::Type::Optional(inner) => (inner.as_ref(), true),
                other => (other, false),
            };
            let compatible = match (column.logical_type.as_str(), value_type) {
                ("boolean", crate::ast::Type::Bool) => true,
                ("bytes", crate::ast::Type::Array(inner)) => **inner == crate::ast::Type::U8,
                ("f64", crate::ast::Type::F32 | crate::ast::Type::F64)
                | ("i64", crate::ast::Type::I32 | crate::ast::Type::I64)
                | ("u64", crate::ast::Type::U32 | crate::ast::Type::U64)
                | ("string", crate::ast::Type::Str) => true,
                ("string", crate::ast::Type::Named(type_name))
                    if type_name == "Uuid" || type_name == "Uuid7" =>
                {
                    true
                }
                ("string", crate::ast::Type::Named(type_name))
                    if type_name == "Instant"
                        && model.timestamps
                        && matches!(metadata.field.as_str(), "createdAt" | "updatedAt") =>
                {
                    true
                }
                ("instant", crate::ast::Type::Named(name)) if name == "Instant" => true,
                _ => false,
            };
            if !compatible {
                self.error_at_with_help(
                    "E3304",
                    format!("model field `{}.{}` has type `{}`, incompatible with database column `{}.{}` of type `{}`", name, metadata.field, field.ty, model.table, metadata.column, column.logical_type),
                    field.span,
                    vec!["make the model field type match the column's logical type".to_string()],
                );
            }
            if column.nullable != optional {
                self.error_at_with_help(
                    "E3304",
                    format!(
                        "model field `{}.{}` optionality does not match database column `{}.{}`",
                        name, metadata.field, model.table, metadata.column
                    ),
                    field.span,
                    vec![
                        if column.nullable {
                            "declare the model field with an optional type such as `string?`"
                        } else {
                            "remove the optional type because the database column is NOT NULL"
                        }
                        .to_string(),
                    ],
                );
            }
            if metadata.unique
                && !table
                    .primary_key
                    .iter()
                    .any(|column| column == &metadata.column)
                && !table
                    .unique
                    .iter()
                    .any(|columns| columns.as_slice() == [metadata.column.as_str()])
                && !table.indexes.iter().any(|index| {
                    index.unique
                        && !index.partial
                        && !index.expression
                        && index.columns.as_slice() == [metadata.column.as_str()]
                })
            {
                self.error_at_with_help(
                    "E3304",
                    format!("model field `{}.{}` is marked `@unique`, but database column `{}.{}` has no single-column unique constraint", name, metadata.field, model.table, metadata.column),
                    metadata.span,
                    vec!["add a unique constraint to the database and refresh the snapshot".to_string()],
                );
            }
            if metadata.default_value.is_some() && column.default.is_none() {
                self.error_at_with_help(
                    "E3304",
                    format!("model field `{}.{}` declares `@default`, but database column `{}.{}` has no default", name, metadata.field, model.table, metadata.column),
                    metadata.span,
                    vec!["add a SQLite DEFAULT expression to the column and refresh the snapshot".to_string()],
                );
            }
            if model.timestamps
                && matches!(metadata.field.as_str(), "createdAt" | "updatedAt")
                && !column.default.as_deref().is_some_and(|default| {
                    default
                        .trim_matches(|character| matches!(character, '(' | ')' | '\'' | '"'))
                        .eq_ignore_ascii_case("CURRENT_TIMESTAMP")
                        || default
                            .trim_matches(|character| matches!(character, '(' | ')' | '\'' | '"'))
                            .to_ascii_lowercase()
                            .starts_with("current_timestamp(")
                })
            {
                self.error_at_with_help(
                    "E3304",
                    format!(
                        "timestamp field `{}.{}` must use a `CURRENT_TIMESTAMP` database default",
                        name, metadata.field
                    ),
                    metadata.span,
                    vec![
                        "generate and apply a migration from the Model with automatic timestamps"
                            .to_string(),
                    ],
                );
            }
            if let Some((referenced_table, referenced_column)) = &metadata.foreign_key {
                let matches_schema = table.foreign_keys.iter().any(|foreign_key| {
                    foreign_key.columns.as_slice() == [metadata.column.as_str()]
                        && foreign_key.referenced_table == *referenced_table
                        && foreign_key.referenced_columns.as_slice() == [referenced_column.as_str()]
                });
                if !matches_schema {
                    self.error_at_with_help(
                        "E3304",
                        format!("model field `{}.{}` declares a foreign key to `{}.{}`, but that single-column relationship is absent from the schema snapshot", name, metadata.field, referenced_table, referenced_column),
                        metadata.span,
                        vec!["match `@foreignKey` to the SQLite foreign key and refresh the snapshot".to_string()],
                    );
                }
            }
        }
        if model.timestamps && self.db_backend == Some(crate::db::DatabaseBackend::Sqlite) {
            let trigger_name = format!(
                "_tyrus_timestamps_{}_updated_at",
                model.table.to_ascii_lowercase()
            );
            if !table
                .triggers
                .iter()
                .any(|trigger| trigger.name.eq_ignore_ascii_case(&trigger_name))
            {
                self.error_at_with_help(
                    "E3304",
                    format!(
                        "model `{name}` uses `@timestamps`, but trigger `{trigger_name}` is missing from the schema snapshot"
                    ),
                    model.span,
                    vec!["generate and apply a migration from the Model with `@timestamps`, then run `tyrus db introspect`".to_string()],
                );
            }
        }
        if id_fields.len() != 1
            || table.primary_key.len() != 1
            || id_fields
                .first()
                .is_some_and(|field| table.primary_key[0] != field.column)
        {
            self.error_at_with_help(
                "E3304",
                format!("model `{name}` must mark its single database primary key with exactly one `@id` field"),
                model.span,
                vec!["add `@id` to the field mapped to the table's single primary key".to_string()],
            );
        }
    }

    fn validate_db_model_id_strategies(
        &mut self,
        model_name: &str,
        fields: &[crate::ast::Field],
        model: &crate::ast::DbModelMetadata,
    ) {
        use crate::ast::Type;
        use crate::db::DatabaseBackend;

        let Some(backend) = self.db_backend.clone() else {
            return;
        };
        if backend == DatabaseBackend::Mongodb {
            for metadata in model
                .fields
                .iter()
                .filter(|field| field.id_strategy.is_some())
            {
                self.error_at_with_help(
                    "E3304",
                    format!("MongoDB generates `_id` automatically and does not accept an ID strategy on model field `{model_name}.{}`", metadata.field),
                    metadata.span,
                    vec!["remove the `strategy` option from `@id` or omit the declared ID field".to_string()],
                );
            }
            return;
        }
        for metadata in model.fields.iter().filter(|field| field.id) {
            let Some(field) = fields.iter().find(|field| field.name == metadata.field) else {
                continue;
            };
            let ty = match &field.ty {
                Type::Optional(inner) => inner.as_ref(),
                ty => ty,
            };
            let strategy = metadata.id_strategy.as_deref().unwrap_or(match &backend {
                DatabaseBackend::Postgresql => "identity",
                DatabaseBackend::Mysql => "auto_increment",
                DatabaseBackend::Sqlite => "rowid",
                DatabaseBackend::Mongodb => unreachable!(),
            });
            let compatible = match (&backend, strategy) {
                (DatabaseBackend::Sqlite, "rowid") => *ty == Type::I64,
                (DatabaseBackend::Mysql, "auto_increment")
                | (DatabaseBackend::Postgresql, "identity") => *ty == Type::I64,
                (DatabaseBackend::Mysql, "uuid") | (DatabaseBackend::Postgresql, "uuid") => {
                    matches!(ty, Type::Named(name) if name == "Uuid")
                }
                (DatabaseBackend::Mysql, "uuid_short") => *ty == Type::U64,
                (DatabaseBackend::Postgresql, "uuid7") => {
                    matches!(ty, Type::Named(name) if name == "Uuid7")
                }
                _ => false,
            };
            if !compatible {
                let help = match &backend {
                    DatabaseBackend::Mysql => {
                        "use the default `auto_increment` strategy with `i64`, or choose `uuid` with `Uuid` or `uuid_short` with `u64`"
                    }
                    DatabaseBackend::Postgresql => {
                        "choose `identity` with a signed integer, `uuid` with `Uuid`, or `uuid7` with `Uuid7`"
                    }
                    DatabaseBackend::Sqlite => {
                        "SQLite generates its implicit `id: i64`; remove `@id` and the declared id field"
                    }
                    DatabaseBackend::Mongodb => unreachable!(),
                };
                let message = format!(
                    "model `{model_name}` ID field `{}` uses strategy `{strategy}` with incompatible type `{}`",
                    metadata.field, field.ty
                );
                self.error_at_with_help("E3304", message, metadata.span, vec![help.to_string()]);
            }
            if matches!(field.ty, Type::Optional(_)) {
                self.error_at_with_help(
                    "E3304",
                    format!(
                        "model `{model_name}` primary-key field `{}` cannot be optional",
                        metadata.field
                    ),
                    field.span,
                    vec!["remove `?` from the primary-key field type".to_string()],
                );
            }
            if metadata.default_value.is_some() && metadata.id_strategy.is_some() {
                self.error_at_with_help(
                    "E3304",
                    format!("model ID field `{model_name}.{}` cannot combine `@default` with an ID strategy", metadata.field),
                    metadata.span,
                    vec!["remove `@default` or the `@id` strategy".to_string()],
                );
            }
        }
    }

    fn register_imports(&mut self, program: &Program, import_types: &HashMap<String, Type>) {
        for import in &program.imports {
            for specifier in &import.specifiers {
                if crate::module_loader::is_namespace_specifier(&import.source, specifier) {
                    let prefix =
                        crate::module_loader::namespace_member_binding(&specifier.local, "");
                    if !import_types.keys().any(|name| name.starts_with(&prefix)) {
                        self.unresolved_namespaces.push(prefix);
                    }
                }
                for local in crate::module_loader::specifier_bindings(
                    &import.source,
                    specifier,
                    import_types,
                ) {
                    self.declare(
                        local.as_str(),
                        Symbol {
                            ty: import_types
                                .get(local.as_str())
                                .map(ValueType::known)
                                .unwrap_or(ValueType::Unknown),
                            mutable: false,
                            kind: SymbolKind::Import,
                            used: false,
                            array_len: None,
                        },
                    );
                }
            }
        }
    }

    fn register_function_symbols(&mut self, program: &Program) {
        for item in &program.items {
            self.item_span = Some(item.span());
            match item {
                Item::FnDecl {
                    name,
                    is_async,
                    type_params,
                    params,
                    ret_type,
                    throws,
                    body,
                    ..
                } => {
                    self.validate_param_decorators(params);
                    self.fn_param_metadata.insert(name.clone(), params.clone());
                    self.fn_body_metadata.insert(
                        name.clone(),
                        FunctionBodyMetadata {
                            params: params.clone(),
                            body: body.clone(),
                        },
                    );
                    let mut effects = throws.clone();
                    if params.iter().any(|param| !param.decorators.is_empty())
                        && !effects
                            .iter()
                            .any(|ty| ty == &Type::Named("ValidationError".to_string()))
                    {
                        effects.push(Type::Named("ValidationError".to_string()));
                    }
                    self.fn_throws.insert(name.clone(), effects);
                    if *is_async {
                        self.async_fns.insert(name.clone());
                    }
                    let parameter_types = params.iter().map(|param| param.ty.clone()).collect();
                    let symbol_return = if *is_async && matches!(ret_type, Type::Infer) {
                        promise_of(Type::Infer)
                    } else {
                        ret_type.clone()
                    };
                    let ty = if type_params.is_empty() {
                        Type::Function {
                            params: parameter_types,
                            ret: Box::new(symbol_return),
                        }
                    } else {
                        Type::GenericFunction {
                            type_params: type_params.clone(),
                            params: parameter_types,
                            ret: Box::new(symbol_return),
                        }
                    };
                    self.declare(
                        name,
                        Symbol {
                            ty: ValueType::Known(ty),
                            mutable: false,
                            kind: SymbolKind::Function,
                            used: false,
                            array_len: None,
                        },
                    );
                }
                Item::Stmt(Stmt {
                    kind:
                        StmtKind::VarDecl {
                            mutable,
                            name,
                            ty,
                            value,
                            ..
                        },
                    ..
                }) => {
                    if let Some(Expr {
                        kind: ExprKind::ArrowFn { params, body, .. },
                        ..
                    }) = value
                    {
                        self.fn_body_metadata.insert(
                            name.clone(),
                            FunctionBodyMetadata {
                                params: params.clone(),
                                body: body.clone(),
                            },
                        );
                    }
                    self.declare(
                        name,
                        Symbol {
                            ty: ValueType::known(ty),
                            mutable: *mutable,
                            kind: SymbolKind::Variable,
                            used: false,
                            array_len: value
                                .as_ref()
                                .and_then(|value| array_literal_len(ty, value)),
                        },
                    );
                }
                _ => {}
            }
        }
    }
}
