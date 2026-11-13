use super::*;
use crate::backend::*;
use tyrus_core::collections::HashMap;
use tyrus_core::doc_comments::DocComment;

#[derive(Default)]
pub(super) struct ImportedModuleContext {
    pub(crate) types: HashMap<String, Type>,
    pub(crate) interfaces: HashMap<String, Vec<Field>>,
    pub(crate) behaviors: HashMap<String, tyrus_core::ast::BehaviorInterfaceInfo>,
    pub(crate) type_aliases: HashMap<String, Type>,
    pub(crate) enums: HashMap<String, tyrus_core::ast::ResolvedEnum>,
    pub(crate) throws: HashMap<String, Vec<Type>>,
    pub(crate) db_models: HashMap<String, tyrus_core::ast::DbModelMetadata>,
    pub(crate) validators: HashMap<String, Type>,
    pub(crate) targets: HashMap<String, String>,
    pub(crate) reachable_interfaces: HashMap<String, Vec<Field>>,
}

pub(crate) struct ImportResolutionState<'a> {
    pub(crate) db_backend: Option<&'a tyrus_core::db::DatabaseBackend>,
    pub(crate) dependency_updates: &'a mut HashMap<Url, HashSet<Url>>,
    pub(crate) module_stack: &'a mut HashSet<Url>,
    pub(crate) cancellation: &'a Arc<std::sync::atomic::AtomicBool>,
}

fn indexed_parameters(parameters: &[tyrus_core::ast::Param]) -> Vec<IndexedParameter> {
    parameters
        .iter()
        .map(|parameter| IndexedParameter {
            pattern: parameter.pattern.clone(),
            ty: parameter.ty.clone(),
            optional: parameter.optional,
            rest: parameter.rest,
            validators: parameter.decorators.clone(),
        })
        .collect()
}

fn indexed_export_location(source: &str, uri: &Url, symbol: &Symbol) -> Location {
    Location {
        uri: uri.clone(),
        range: byte_range_to_lsp(source, symbol.declaration.clone()),
    }
}

fn indexed_export_metadata(analysis: &DocumentAnalysis, symbol: &Symbol) -> IndexedExportMetadata {
    let Some(program) = analysis.program.as_ref() else {
        return IndexedExportMetadata::Value {
            ty: symbol.semantic_type.clone(),
        };
    };
    let Some(item) = program.items.iter().find(|item| {
        let same_name = match item {
            tyrus_core::ast::Item::FnDecl { name, .. }
            | tyrus_core::ast::Item::InterfaceDecl { name, .. }
            | tyrus_core::ast::Item::BehaviorInterfaceDecl { name, .. }
            | tyrus_core::ast::Item::TypeAlias { name, .. }
            | tyrus_core::ast::Item::EnumDecl { name, .. } => name == &symbol.name,
            tyrus_core::ast::Item::Stmt(statement) => matches!(
                statement.kind(),
                tyrus_core::ast::StmtKind::VarDecl { name, .. } if name == &symbol.name
            ),
        };
        let item_range = item.span().to_range();
        same_name
            && item_range.start <= symbol.declaration.start
            && symbol.declaration.end <= item_range.end
    }) else {
        return IndexedExportMetadata::Value {
            ty: symbol.semantic_type.clone(),
        };
    };
    match item {
        tyrus_core::ast::Item::FnDecl {
            is_validator,
            type_params,
            params,
            ret_type,
            throws,
            ..
        } => IndexedExportMetadata::Function {
            is_validator: *is_validator,
            type_params: type_params.clone(),
            parameters: indexed_parameters(params),
            return_type: ret_type.clone(),
            throws: throws.clone(),
        },
        tyrus_core::ast::Item::InterfaceDecl {
            type_params,
            extends,
            fields,
            db_model,
            ..
        } => IndexedExportMetadata::Interface {
            type_params: type_params.clone(),
            extends: extends.clone(),
            fields: fields.clone(),
            db_model: db_model.clone(),
        },
        tyrus_core::ast::Item::BehaviorInterfaceDecl {
            type_params,
            targets,
            extends,
            methods,
            bindings,
            ..
        } => IndexedExportMetadata::Behavior {
            type_params: type_params.clone(),
            targets: targets.clone(),
            extends: extends.clone(),
            methods: methods
                .iter()
                .map(|method| IndexedBehaviorMethod {
                    name: method.name.clone(),
                    type_params: method.type_params.clone(),
                    parameters: indexed_parameters(&method.params),
                    return_type: method.ret_type.clone(),
                    throws: method.throws.clone(),
                })
                .collect(),
            bindings: bindings.clone(),
        },
        tyrus_core::ast::Item::TypeAlias {
            type_params,
            ty,
            merged_fields,
            ..
        } => IndexedExportMetadata::TypeAlias {
            type_params: type_params.clone(),
            ty: ty.clone(),
            merged_fields: merged_fields.clone(),
        },
        tyrus_core::ast::Item::EnumDecl { repr, members, .. } => IndexedExportMetadata::Enum {
            repr: repr.clone(),
            members: members.clone(),
        },
        tyrus_core::ast::Item::Stmt(_) => IndexedExportMetadata::Value {
            ty: symbol.semantic_type.clone(),
        },
    }
}

impl Backend {
    pub(super) async fn add_import_diagnostics(
        &self,
        uri: &Url,
        source_text: &str,
        analysis: &mut DocumentAnalysis,
        import_depth: usize,
        state: &mut ImportResolutionState<'_>,
    ) -> ImportedModuleContext {
        state.dependency_updates.entry(uri.clone()).or_default();
        let mut reachable_interfaces = HashMap::default();
        let mut import_types = HashMap::default();
        let mut import_interfaces = HashMap::default();
        let mut import_behaviors = HashMap::default();
        let mut import_type_aliases = HashMap::default();
        let mut import_enums = HashMap::default();
        let mut import_throws = HashMap::default();
        let mut import_db_models = HashMap::default();
        let mut import_validators = HashMap::default();
        let mut import_targets = HashMap::default();
        let Ok(document_path) = uri.to_file_path() else {
            return ImportedModuleContext {
                types: import_types,
                interfaces: import_interfaces,
                behaviors: import_behaviors,
                type_aliases: import_type_aliases,
                enums: import_enums,
                throws: import_throws,
                db_models: import_db_models,
                validators: import_validators,
                targets: import_targets,
                reachable_interfaces,
            };
        };
        for import in analysis
            .symbols
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::Import)
        {
            if state
                .cancellation
                .load(std::sync::atomic::Ordering::Acquire)
            {
                break;
            }
            let Some(source) = import.import_source.as_deref() else {
                continue;
            };
            let import_range = import_source_range(source_text, &import.declaration, source)
                .unwrap_or_else(|| import.declaration.clone());
            let imported = import.imported_name.as_deref().unwrap_or(&import.name);
            if source.starts_with("tyrus:") {
                if source == tyrus_core::test::MODULE
                    && !tyrus_core::test::is_test_file(&document_path)
                {
                    let name = document_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("this file");
                    let stem = document_path
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .unwrap_or("example");
                    analysis.diagnostics.push(AnalysisDiagnostic {
                        code: "E0434".to_string(),
                        range: import_range.clone(),
                        severity: DiagnosticSeverity::Error,
                        message: format!(
                            "`{source}` can only be imported by a test file, and `{name}` is not one"
                        ),
                        related: Vec::new(),
                        help: vec![format!(
                            "rename it to `{stem}.test.tyr`, or move the import into a `*.test.tyr` file such as `__tests__/{stem}.test.tyr`"
                        )],
                    });
                    continue;
                }
                if let Some(ty) = native_import_type(source, imported) {
                    if let Some(target) =
                        tyrus_core::module_loader::native_export_target(source, imported)
                    {
                        import_targets.insert(import.name.clone(), target);
                    }
                    let ty = if tyrus_core::module_loader::native_export_enum(source, imported)
                        .is_some()
                    {
                        Type::Named(import.name.clone())
                    } else {
                        ty
                    };
                    import_types.insert(import.name.clone(), ty);
                    if source == "tyrus:db" && imported == "db" {
                        {
                            let member = "query";
                            let binding = tyrus_core::module_loader::namespace_member_binding(
                                &import.name,
                                member,
                            );
                            if let Some(member_ty) = native_import_type(source, member) {
                                import_types.insert(binding.clone(), member_ty);
                            }
                            if let Some(target) =
                                tyrus_core::module_loader::native_export_target(source, member)
                            {
                                import_targets.insert(binding, target);
                            }
                        }
                        if let Some((_, Some(fields), _)) =
                            tyrus_core::module_loader::native_export_info(source, "DbError")
                        {
                            import_interfaces.insert("DbError".to_string(), fields);
                        }
                    }
                    if let Some((_, interface_fields, type_alias)) =
                        tyrus_core::module_loader::native_export_info(source, imported)
                    {
                        if let Some(fields) = interface_fields {
                            import_interfaces.insert(import.name.clone(), fields.clone());
                            import_interfaces
                                .entry(imported.to_string())
                                .or_insert(fields);
                            if import.name != imported {
                                import_type_aliases
                                    .insert(import.name.clone(), Type::Named(imported.to_string()));
                            }
                        }
                        if let Some(alias) = type_alias {
                            import_type_aliases.insert(import.name.clone(), alias);
                        }
                    }
                    if let Some(behavior) =
                        tyrus_core::module_loader::native_export_behavior(source, imported)
                    {
                        import_behaviors.insert(import.name.clone(), behavior);
                    }
                    if let Some(enum_info) =
                        tyrus_core::module_loader::native_export_enum(source, imported)
                    {
                        import_enums.insert(import.name.clone(), enum_info);
                    }
                    if let Some((_, Some(fields), _)) =
                        tyrus_core::module_loader::native_export_info(source, imported)
                    {
                        import_interfaces.insert(import.name.clone(), fields);
                    }
                } else {
                    analysis.diagnostics.push(AnalysisDiagnostic {
                        code: "E0432".to_string(),
                        range: import_range.clone(),
                        severity: DiagnosticSeverity::Error,
                        message: format!("module `{source}` does not export `{imported}`"),
                        related: Vec::new(),
                        help: Vec::new(),
                    });
                }
                continue;
            }
            let alias_resolution = if source.starts_with('#') {
                match self.project_root_for(uri).await {
                    Some(root) => self
                        .import_map_for_root(&root)
                        .await
                        .and_then(|imports| imports.resolve(&root, source)),
                    None => Err("project root is unavailable".to_string()),
                }
            } else {
                Ok(resolve_import_path(&document_path, source))
            };
            let target = match alias_resolution {
                Ok(Some(target)) => target,
                Ok(None) => {
                    analysis.diagnostics.push(AnalysisDiagnostic {
                        code: "E0432".to_string(),
                        range: import_range.clone(),
                        severity: DiagnosticSeverity::Error,
                        message: format!("unsupported module `{source}`"),
                        related: Vec::new(),
                        help: vec![
                            "use a relative module path such as `./module`, an import alias declared in tyrus.json, or a supported `tyrus:` module"
                                .to_string(),
                        ],
                    });
                    continue;
                }
                Err(message) => {
                    analysis.diagnostics.push(AnalysisDiagnostic {
                        code: "E0432".to_string(),
                        range: import_range.clone(),
                        severity: DiagnosticSeverity::Error,
                        message,
                        related: Vec::new(),
                        help: vec![
                            "declare the alias in tyrus.json or use a relative module path such as `./module`"
                                .to_string(),
                        ],
                    });
                    continue;
                }
            };
            let Ok(target_uri) = Url::from_file_path(&target) else {
                continue;
            };
            state
                .dependency_updates
                .entry(uri.clone())
                .or_default()
                .insert(target_uri.clone());
            let Some(target_source) = self.source(&target_uri).await else {
                analysis.diagnostics.push(AnalysisDiagnostic {
                    code: "E0583".to_string(),
                    range: import_range.clone(),
                    severity: DiagnosticSeverity::Error,
                    message: format!("cannot resolve module `{source}`"),
                    related: Vec::new(),
                    help: Vec::new(),
                });
                continue;
            };
            if let Some(violation) = tyrus_core::module_loader::protected_import_violation(
                &document_path,
                &target,
                imported,
            ) {
                trace_lsp_event("protected_import_violation");
                analysis.diagnostics.push(AnalysisDiagnostic {
                    code: "E3308".to_string(),
                    range: import.declaration.clone(),
                    severity: DiagnosticSeverity::Error,
                    message: violation.message(imported, source),
                    related: Vec::new(),
                    help: violation.help(imported),
                });
                continue;
            }
            let mut transitive_interfaces = HashMap::default();
            let target_analysis =
                if import_depth < 32 && state.module_stack.insert(target_uri.clone()) {
                    let mut dependency_analysis = self
                        .workspace_analysis_for(&target_uri, &target_source)
                        .await
                        .as_ref()
                        .clone();
                    let ImportedModuleContext {
                        types: dependency_types,
                        interfaces: dependency_interfaces,
                        behaviors: dependency_behaviors,
                        type_aliases: dependency_type_aliases,
                        enums: dependency_enums,
                        throws: dependency_throws,
                        db_models: dependency_db_models,
                        validators: dependency_validators,
                        targets: dependency_targets,
                        reachable_interfaces: dependency_reachable_interfaces,
                    } = Box::pin(self.add_import_diagnostics(
                        &target_uri,
                        &target_source,
                        &mut dependency_analysis,
                        import_depth + 1,
                        state,
                    ))
                    .await;
                    state.module_stack.remove(&target_uri);
                    transitive_interfaces.extend(dependency_interfaces.clone());
                    transitive_interfaces.extend(dependency_reachable_interfaces.clone());
                    let has_dependency_context = !dependency_types.is_empty()
                        || !dependency_targets.is_empty()
                        || !dependency_interfaces.is_empty()
                        || !dependency_behaviors.is_empty()
                        || !dependency_type_aliases.is_empty()
                        || !dependency_enums.is_empty()
                        || !dependency_throws.is_empty()
                        || !dependency_db_models.is_empty()
                        || !dependency_validators.is_empty()
                        || !dependency_reachable_interfaces.is_empty();
                    if !has_dependency_context && state.db_backend.is_none() {
                        dependency_analysis
                    } else {
                        tyrus_core::analysis::analyze_with_context_cancellable(
                            &target_source,
                            tyrus_core::analysis::AnalyzeContext {
                                import_types: Some(&dependency_types),
                                import_targets: Some(&dependency_targets),
                                entry_point: false,
                                import_interfaces: Some(&dependency_interfaces),
                                import_behaviors: Some(&dependency_behaviors),
                                import_type_aliases: Some(&dependency_type_aliases),
                                import_enums: Some(&dependency_enums),
                                import_throws: Some(&dependency_throws),
                                import_db_models: Some(&dependency_db_models),
                                import_validators: Some(&dependency_validators),
                                reachable_interfaces: Some(&dependency_reachable_interfaces),
                                env_schema: None,
                                db_schema: None,
                                db_schema_error: None,
                                db_backend: state.db_backend,
                            },
                            Arc::clone(state.cancellation),
                        )
                        .unwrap_or_default()
                    }
                } else {
                    if import_depth < 32 && state.module_stack.contains(&target_uri) {
                        trace_lsp_event("import_cycle_cutoff");
                    }
                    tyrus_core::analysis::analyze_with_context_cancellable(
                        &target_source,
                        tyrus_core::analysis::AnalyzeContext {
                            db_backend: state.db_backend,
                            ..tyrus_core::analysis::AnalyzeContext::default()
                        },
                        Arc::clone(state.cancellation),
                    )
                    .unwrap_or_default()
                };
            let target_program = target_analysis.program.as_ref();
            let resolved_export_name = target_program.and_then(|program| {
                if imported == tyrus_core::ast::DEFAULT_EXPORT {
                    return program.exports.iter().find_map(|export| match export {
                        tyrus_core::ast::ExportDecl::Named {
                            exported, local, ..
                        } if exported == tyrus_core::ast::DEFAULT_EXPORT => Some(local.as_str()),
                        tyrus_core::ast::ExportDecl::Re {
                            exported, imported, ..
                        } if exported == tyrus_core::ast::DEFAULT_EXPORT => Some(imported.as_str()),
                        _ => None,
                    });
                }
                program.exports.iter().find_map(|export| match export {
                    tyrus_core::ast::ExportDecl::Named {
                        exported, local, ..
                    } if exported == imported => Some(local.as_str()),
                    tyrus_core::ast::ExportDecl::Re {
                        exported,
                        imported: original,
                        ..
                    } if exported == imported => Some(original.as_str()),
                    tyrus_core::ast::ExportDecl::All { .. } => Some(imported),
                    _ => None,
                })
            });
            if let Some(target) = resolved_export_name.and_then(|name| {
                target_analysis
                    .symbols
                    .iter()
                    .find(|symbol| symbol.exported && symbol.name == name)
            }) {
                if target.kind == SymbolKind::Interface {
                    if let Some(Type::Object(fields)) = &target.semantic_type {
                        import_interfaces.insert(import.name.clone(), fields.clone());
                        import_types.insert(import.name.clone(), Type::Named(imported.to_string()));
                        tyrus_core::analysis::collect_reachable_interfaces(
                            std::slice::from_ref(fields),
                            &|name| {
                                transitive_interfaces
                                    .get(name)
                                    .cloned()
                                    .or_else(|| exported_interface_fields(&target_analysis, name))
                            },
                            &mut reachable_interfaces,
                        );
                    }
                } else if target.kind == SymbolKind::TypeAlias {
                    if let Some(ty) = &target.semantic_type {
                        import_type_aliases.insert(import.name.clone(), ty.clone());
                        import_types.insert(import.name.clone(), Type::Named(imported.to_string()));
                    }
                } else if let Some(ty) = &target.semantic_type {
                    import_types.insert(import.name.clone(), ty.clone());
                }
                if target.kind == SymbolKind::Interface {
                    if let Some(model) = target_program.and_then(|program| {
                        imported_db_model_metadata(program, imported, state.db_backend)
                    }) {
                        import_db_models.insert(import.name.clone(), model);
                    }
                    if let Some(program) = target_program
                        && let Some(item) = program.items.iter().find(|item| {
                            matches!(item, Item::BehaviorInterfaceDecl { name, .. } if name == imported)
                        })
                            && let Some(behavior) =
                                tyrus_core::module_loader::behavior_interface_info(
                                    item,
                                    &program.items,
                                )
                            {
                                import_types.insert(
                                    import.name.clone(),
                                    Type::Named(imported.to_string()),
                                );
                                import_behaviors.insert(import.name.clone(), behavior);
                            }
                }
                if let Some(ty) = target.semantic_type.as_ref() {
                    let is_validator =
                        target_program.is_some_and(|program| {
                            program.items.iter().any(|item| matches!(
                                item,
                                Item::FnDecl { name, is_validator: true, .. } if name == imported
                            ))
                        });
                    if is_validator {
                        import_validators.insert(import.name.clone(), ty.clone());
                    }
                }
                if let Some(effects) = target_program
                    .and_then(|program| function_throws_from_program(program, imported))
                    && !effects.is_empty()
                {
                    import_throws.insert(import.name.clone(), effects);
                }
            } else {
                let default_export_local = target_program.and_then(|program| {
                    program.exports.iter().find_map(|export| match export {
                        tyrus_core::ast::ExportDecl::Named {
                            exported, local, ..
                        } if exported == tyrus_core::ast::DEFAULT_EXPORT => Some(local.as_str()),
                        _ => None,
                    })
                });
                let has_named_export = target_program.is_some_and(|program| {
                    program
                        .exports
                        .iter()
                        .any(|export| export.exported_name().is_some_and(|name| name == imported))
                });
                let is_default_only_import =
                    default_export_local == Some(imported) && !has_named_export;
                let (message, help) = if is_default_only_import {
                    (
                        format!(
                            "module `{source}` exports `{imported}` as default, not as a named export"
                        ),
                        vec![format!(
                            "remove the braces and import the default export: `import {imported} from \"{source}\";`"
                        )],
                    )
                } else {
                    (
                        format!("module `{source}` does not export `{imported}`"),
                        Vec::new(),
                    )
                };
                analysis.diagnostics.push(AnalysisDiagnostic {
                    code: "E0432".to_string(),
                    range: import.declaration.clone(),
                    severity: DiagnosticSeverity::Error,
                    message,
                    related: Vec::new(),
                    help,
                });
            }
        }
        let reexport_sources = analysis
            .program
            .as_ref()
            .into_iter()
            .flat_map(|program| program.exports.iter())
            .filter_map(|export| match export {
                tyrus_core::ast::ExportDecl::Re { source, .. }
                | tyrus_core::ast::ExportDecl::All { source, .. } => Some(source.clone()),
                tyrus_core::ast::ExportDecl::Named { .. } => None,
            })
            .collect::<Vec<_>>();
        for source in reexport_sources {
            if state
                .cancellation
                .load(std::sync::atomic::Ordering::Acquire)
            {
                break;
            }
            let target = if source.starts_with('#') {
                match self.project_root_for(uri).await {
                    Some(root) => self
                        .import_map_for_root(&root)
                        .await
                        .ok()
                        .and_then(|imports| imports.resolve(&root, &source).ok().flatten()),
                    None => None,
                }
            } else {
                resolve_import_path(&document_path, &source)
            };
            let Some(target) = target else {
                continue;
            };
            let Ok(target_uri) = Url::from_file_path(&target) else {
                continue;
            };
            state
                .dependency_updates
                .entry(uri.clone())
                .or_default()
                .insert(target_uri.clone());
            if import_depth >= 32 || !state.module_stack.insert(target_uri.clone()) {
                continue;
            }
            let Some(target_source) = self.source(&target_uri).await else {
                state.module_stack.remove(&target_uri);
                continue;
            };
            let mut target_analysis = self
                .workspace_analysis_for(&target_uri, &target_source)
                .await
                .as_ref()
                .clone();
            let _ = Box::pin(self.add_import_diagnostics(
                &target_uri,
                &target_source,
                &mut target_analysis,
                import_depth + 1,
                state,
            ))
            .await;
            state.module_stack.remove(&target_uri);
        }
        if import_db_models.values().any(|model| model.timestamps) {
            reachable_interfaces
                .entry("Instant".to_string())
                .or_insert_with(tyrus_core::builtin_interfaces::instant_fields);
        }
        ImportedModuleContext {
            types: import_types,
            interfaces: import_interfaces,
            behaviors: import_behaviors,
            type_aliases: import_type_aliases,
            enums: import_enums,
            throws: import_throws,
            db_models: import_db_models,
            validators: import_validators,
            targets: import_targets,
            reachable_interfaces,
        }
    }

    pub(super) async fn exported_symbols(
        &self,
        for_file: Option<&Path>,
        prefix: Option<&str>,
        progress_token: Option<ProgressToken>,
    ) -> Vec<IndexedExport> {
        self.exported_symbols_inner(for_file, prefix, false, progress_token)
            .await
            .0
    }

    async fn workspace_root_for_export_index(&self, for_file: Option<&Path>) -> Option<PathBuf> {
        if let Some(path) = for_file {
            let uri = Url::from_file_path(path).ok()?;
            self.project_root_for(&uri).await
        } else {
            self.workspace_root.read().await.clone()
        }
    }

    async fn cached_workspace_exports(
        &self,
        root: &Path,
        generation: WorkspaceGeneration,
        prefix: Option<&str>,
        include_interfaces: bool,
    ) -> Option<(Vec<IndexedExport>, Vec<ObjectInterfaceCandidate>)> {
        let lock_wait_started = std::time::Instant::now();
        let cache = self.workspace_exports_cache.read().await;
        trace_lsp_stage("workspace_exports_cache_lock_wait", lock_wait_started);
        let lock_hold_started = std::time::Instant::now();
        let cached = cache
            .get(root)
            .filter(|entry| entry.generation == generation)
            .map(|entry| {
                (
                    Self::filter_indexed_exports_with_name_order(
                        &entry.exports,
                        &entry.export_name_order,
                        prefix,
                    ),
                    if include_interfaces {
                        entry.interfaces.clone()
                    } else {
                        Vec::new()
                    },
                )
            });
        drop(cache);
        trace_lsp_stage("workspace_exports_cache_lock_hold", lock_hold_started);
        cached
    }

    async fn exported_symbols_inner(
        &self,
        for_file: Option<&Path>,
        prefix: Option<&str>,
        include_interfaces: bool,
        progress_token: Option<ProgressToken>,
    ) -> (Vec<IndexedExport>, Vec<ObjectInterfaceCandidate>) {
        let index_started = std::time::Instant::now();
        let mut exports = Self::filter_indexed_exports(&native_exports(), prefix);
        let workspace_root = self.workspace_root_for_export_index(for_file).await;
        let Some(root) = workspace_root else {
            return (exports, Vec::new());
        };
        let generation = self.workspace_index_generation_for(&root).await;
        let cached = self
            .cached_workspace_exports(&root, generation, prefix, include_interfaces)
            .await;
        let (workspace_exports, workspace_interfaces) = if let Some(cached) = cached {
            trace_lsp_stage("workspace_exports_cache_hit", index_started);
            cached
        } else {
            let files = self.workspace_files(&root, generation).await;
            let progress = self
                .begin_workspace_index_progress(progress_token, files.len())
                .await;
            let analysis_lock_wait_started = std::time::Instant::now();
            let analyses_cache = self.workspace_analysis_cache.read().await;
            trace_lsp_stage(
                "workspace_analysis_cache_lock_wait",
                analysis_lock_wait_started,
            );
            let analysis_lock_hold_started = std::time::Instant::now();
            let mut cached_analyses = HashMap::default();
            for path in &files {
                let Ok(uri) = Url::from_file_path(path) else {
                    continue;
                };
                if let Some((source, analysis)) = analyses_cache.get(&uri) {
                    cached_analyses.insert(uri, (source.clone(), Arc::clone(analysis)));
                }
            }
            drop(analyses_cache);
            trace_lsp_stage(
                "workspace_analysis_cache_lock_hold",
                analysis_lock_hold_started,
            );
            let Ok(permit) = Arc::clone(&self.workspace_index_semaphore)
                .acquire_owned()
                .await
            else {
                self.end_workspace_index_progress(progress).await;
                return (exports, Vec::new());
            };
            let documents = {
                let documents = self.documents.read().await;
                files
                    .iter()
                    .filter_map(|path| {
                        let uri = Url::from_file_path(path).ok()?;
                        documents
                            .get(&uri)
                            .map(|document| (uri, document.text.clone()))
                    })
                    .collect::<HashMap<_, _>>()
            };
            let (progress_sender, progress_reporter) = if let Some(token) = progress.clone() {
                let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<usize>();
                let client = self.client.clone();
                let total_files = files.len();
                let reporter = tokio::spawn(async move {
                    while let Some(completed) = receiver.recv().await {
                        let percentage = completed.saturating_mul(100) / total_files.max(1);
                        client
                            .send_notification::<notification::Progress>(ProgressParams {
                                token: token.clone(),
                                value: ProgressParamsValue::WorkDone(WorkDoneProgress::Report(
                                    WorkDoneProgressReport {
                                        cancellable: Some(false),
                                        message: Some(format!(
                                            "Indexing file {completed} of {total_files}"
                                        )),
                                        percentage: Some(percentage.min(99) as u32),
                                    },
                                )),
                            })
                            .await;
                    }
                });
                (Some(sender), Some(reporter))
            } else {
                (None, None)
            };
            let total_files = files.len();
            let mut import_maps = HashMap::default();
            let mut module_roots = HashMap::default();
            for path in &files {
                let Ok(uri) = Url::from_file_path(path) else {
                    continue;
                };
                let module_root = nearest_project_root(path).unwrap_or_else(|| root.clone());
                module_roots.insert(uri, module_root.clone());
                if !import_maps.contains_key(&module_root)
                    && let Ok(import_map) = self.import_map_for_root(&module_root).await
                {
                    import_maps.insert(module_root, import_map);
                }
            }
            let (built, interfaces, analyses, resolved_imports, module_fingerprints) =
                tokio::task::spawn_blocking(move || {
                    let _permit = permit;
                    let mut exports = Vec::new();
                    let mut interfaces = Vec::new();
                    let mut analyses = Vec::new();
                    let mut resolved_imports = std::collections::HashMap::new();
                    let mut module_fingerprints = std::collections::HashMap::new();
                    for (index, path) in files.into_iter().enumerate() {
                        if index % 32 == 0
                            && let Some(sender) = &progress_sender
                        {
                            let _ = sender.send(index + 1);
                        }
                        let Ok(uri) = Url::from_file_path(&path) else {
                            continue;
                        };
                        let source = read_workspace_source(&path, &uri, &documents);
                        let Some(source) = source else { continue };
                        let protected_names =
                            tyrus_core::module_loader::protected_export_names(&source);
                        let analysis = cached_analyses
                            .get(&uri)
                            .filter(|(cached_source, _)| cached_source == &source)
                            .map(|(_, analysis)| Arc::clone(analysis))
                            .unwrap_or_else(|| Arc::new(analyze(&source)));
                        module_fingerprints.insert(
                            uri.clone(),
                            WorkspaceModuleFingerprint {
                                canonical_path: std::fs::canonicalize(&path)
                                    .unwrap_or_else(|_| path.clone()),
                                content: workspace_content_fingerprint(&source),
                            },
                        );
                        for symbol in &analysis.symbols {
                            if symbol.kind == SymbolKind::Import
                                && let Some(import_source) = symbol.import_source.as_deref()
                            {
                                let target = if import_source.starts_with('#') {
                                    module_roots.get(&uri).and_then(|root| {
                                        import_maps.get(root).and_then(|map| {
                                            map.resolve(root, import_source).ok().flatten()
                                        })
                                    })
                                } else if import_source.starts_with("tyrus:") {
                                    None
                                } else {
                                    resolve_import_path(&path, import_source)
                                };
                                if let Some(target) = target
                                    && let Ok(target_uri) = Url::from_file_path(target)
                                {
                                    resolved_imports.insert(
                                        (uri.clone(), import_source.to_string()),
                                        target_uri,
                                    );
                                }
                            }
                            if symbol.kind == SymbolKind::Interface
                                && let Some(Type::Object(fields)) = symbol.semantic_type.as_ref()
                            {
                                interfaces.push(ObjectInterfaceCandidate {
                                    name: symbol.name.clone(),
                                    fields: fields.clone(),
                                    exported: symbol.exported,
                                    uri: uri.clone(),
                                });
                            }
                            if symbol.exported {
                                exports.push(IndexedExport {
                                    is_protected: protected_names.contains(&symbol.name),
                                    name: symbol.name.clone(),
                                    is_default: analysis.program.as_ref().is_some_and(|program| {
                                        program.exports.iter().any(|export| matches!(
                                            export,
                                            tyrus_core::ast::ExportDecl::Named { exported, local, .. }
                                                if exported == tyrus_core::ast::DEFAULT_EXPORT && local == &symbol.name
                                        ))
                                    }),
                                    module_source: None,
                                    ty: symbol.ty.clone(),
                                    parameters: symbol
                                        .parameters
                                        .iter()
                                        .map(|parameter| {
                                            parameter
                                                .pattern
                                                .binding_names()
                                                .first()
                                                .copied()
                                                .unwrap_or("param")
                                                .to_string()
                                        })
                                        .collect(),
                                    kind: symbol.kind,
                                    uri: uri.clone(),
                                    definition: Some(indexed_export_location(
                                        &source, &uri, symbol,
                                    )),
                                    metadata: Arc::new(indexed_export_metadata(&analysis, symbol)),
                                });
                            }
                        }
                        analyses.push((uri, source, analysis));
                    }
                    if let Some(sender) = &progress_sender {
                        let _ = sender.send(total_files);
                    }
                    (
                        exports,
                        interfaces,
                        analyses,
                        resolved_imports,
                        module_fingerprints,
                    )
                })
                .await
                .unwrap_or_default();
            if let Some(reporter) = progress_reporter {
                let _ = reporter.await;
            }
            {
                let mut cache = self.workspace_analysis_cache.write().await;
                for (uri, source, analysis) in analyses {
                    insert_cached_workspace_analysis(&mut cache, uri, &source, analysis);
                }
            }
            let weight = estimated_workspace_export_weight(
                &built,
                &interfaces,
                &resolved_imports,
                &module_fingerprints,
            );
            trace_lsp_count("workspace_modules_indexed", module_fingerprints.len());
            trace_lsp_count("workspace_import_edges_indexed", resolved_imports.len());
            let returned_interfaces = if include_interfaces {
                interfaces.clone()
            } else {
                Vec::new()
            };
            let export_name_order = Self::indexed_export_name_order(&built);
            let filtered_exports =
                Self::filter_indexed_exports_with_name_order(&built, &export_name_order, prefix);
            self.end_workspace_index_progress(progress).await;
            let returned_interfaces = if self.workspace_index_generation_for(&root).await
                == generation
            {
                let mut cache = self.workspace_exports_cache.write().await;
                if weight <= MAX_CACHED_WORKSPACE_EXPORT_WEIGHT {
                    evict_workspace_root_cache_entries(&mut cache, &root);
                    let cached_weight = cache
                        .iter()
                        .filter(|(cached_root, _)| *cached_root != &root)
                        .fold(0usize, |total, (_, entry)| {
                            total.saturating_add(entry.estimated_weight)
                        });
                    if cached_weight.saturating_add(weight) <= MAX_CACHED_WORKSPACE_EXPORTS_WEIGHT {
                        cache.insert(
                            root.clone(),
                            WorkspaceExportCacheEntry {
                                generation,
                                estimated_weight: weight,
                                exports: built.clone(),
                                export_name_order,
                                interfaces,
                                resolved_imports,
                                module_fingerprints,
                            },
                        );
                    }
                }
                returned_interfaces
            } else {
                Vec::new()
            };
            trace_lsp_stage("workspace_exports_build", index_started);
            (filtered_exports, returned_interfaces)
        };
        for mut export in workspace_exports {
            if !workspace_export_is_visible(&export, for_file) {
                continue;
            }
            export.is_protected = false;
            exports.push(export);
        }
        (exports, workspace_interfaces)
    }

    pub(super) fn filter_indexed_exports(
        exports: &[IndexedExport],
        prefix: Option<&str>,
    ) -> Vec<IndexedExport> {
        exports
            .iter()
            .filter(|export| prefix.is_none_or(|prefix| export.name.starts_with(prefix)))
            .cloned()
            .collect()
    }

    pub(super) fn indexed_export_name_order(exports: &[IndexedExport]) -> Vec<usize> {
        let mut order = (0..exports.len()).collect::<Vec<_>>();
        order.sort_unstable_by(|left, right| {
            exports[*left]
                .name
                .cmp(&exports[*right].name)
                .then_with(|| left.cmp(right))
        });
        order
    }

    pub(super) fn filter_indexed_exports_with_name_order(
        exports: &[IndexedExport],
        name_order: &[usize],
        prefix: Option<&str>,
    ) -> Vec<IndexedExport> {
        let Some(prefix) = prefix else {
            return exports.to_vec();
        };
        if prefix.is_empty() {
            return exports.to_vec();
        }
        let start = name_order.partition_point(|index| exports[*index].name.as_str() < prefix);
        let mut matching_indices = Vec::new();
        for index in &name_order[start..] {
            if !exports[*index].name.starts_with(prefix) {
                break;
            }
            matching_indices.push(*index);
        }
        matching_indices.sort_unstable();
        matching_indices
            .into_iter()
            .map(|index| exports[index].clone())
            .collect()
    }

    pub(super) async fn add_object_interface_match_hints(
        &self,
        uri: &Url,
        source: &str,
        analysis: &mut DocumentAnalysis,
    ) {
        let Ok(current_path) = uri.to_file_path() else {
            return;
        };
        if !analysis.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E0310"
                && diagnostic
                    .message
                    .contains("requires an explicit interface annotation")
        }) {
            return;
        }
        let matches_by_fields = self.object_interface_candidates(uri).await;
        for diagnostic in &mut analysis.diagnostics {
            if diagnostic.code != "E0310"
                || !diagnostic
                    .message
                    .contains("requires an explicit interface annotation")
            {
                continue;
            }
            let Some(actual_fields) = anonymous_object_binding_fields(source, diagnostic) else {
                continue;
            };
            let matches = matches_by_fields
                .iter()
                .filter(|candidate| exact_interface_match(&candidate.fields, &actual_fields))
                .collect::<Vec<_>>();
            if matches.is_empty() {
                continue;
            }
            let help = object_interface_help(&matches, &current_path);
            let Some(help) = help else { continue };
            if !diagnostic.help.contains(&help) {
                diagnostic.help.push(help);
            }
        }
    }

    async fn object_interface_candidates(
        &self,
        current_uri: &Url,
    ) -> Vec<ObjectInterfaceCandidate> {
        let Some(path) = current_uri.to_file_path().ok() else {
            return Vec::new();
        };
        self.exported_symbols_inner(Some(&path), None, true, None)
            .await
            .1
            .into_iter()
            .filter(|candidate| &candidate.uri != current_uri)
            .collect()
    }

    pub(super) async fn interface_info(
        &self,
        document_uri: &Url,
        symbol: &Symbol,
        source: &str,
    ) -> Option<InterfaceInfo> {
        if symbol.kind == SymbolKind::Interface
            && let Some(Type::Object(fields)) = &symbol.semantic_type
        {
            let line = byte_range_to_lsp(source, symbol.declaration.clone())
                .start
                .line
                + 1;
            return Some(InterfaceInfo {
                name: symbol.name.clone(),
                fields: fields.clone(),
                link: format!("{document_uri}#L{line}"),
                documentation: symbol.documentation.clone(),
            });
        }
        if symbol.kind == SymbolKind::Import {
            let imported = symbol.imported_name.as_deref().unwrap_or(&symbol.name);
            let location = self.imported_definition(document_uri, symbol).await?;
            let target_source = self.source(&location.uri).await?;
            let analysis = self
                .workspace_analysis_for(&location.uri, &target_source)
                .await;
            let target = analysis
                .symbols
                .iter()
                .find(|candidate| candidate.name == imported)?;
            if target.kind == SymbolKind::Interface
                && let Some(Type::Object(fields)) = &target.semantic_type
            {
                return Some(InterfaceInfo {
                    name: imported.to_string(),
                    fields: fields.clone(),
                    link: format!("{}#L{}", location.uri, location.range.start.line + 1),
                    documentation: target.documentation.clone(),
                });
            }
        }
        None
    }

    pub(super) async fn object_type_fields(
        &self,
        document_uri: &Url,
        symbol: &Symbol,
    ) -> Option<Vec<Field>> {
        let object_fields = |symbol: &Symbol| match (&symbol.kind, &symbol.semantic_type) {
            (SymbolKind::Interface | SymbolKind::TypeAlias, Some(Type::Object(fields))) => {
                Some(fields.clone())
            }
            _ => None,
        };
        if let Some(fields) = object_fields(symbol) {
            return Some(fields);
        }
        if symbol.kind == SymbolKind::Import {
            let imported = symbol.imported_name.as_deref().unwrap_or(&symbol.name);
            let location = self.imported_definition(document_uri, symbol).await?;
            let target_source = self.source(&location.uri).await?;
            let analysis = self
                .workspace_analysis_for(&location.uri, &target_source)
                .await;
            let target = analysis
                .symbols
                .iter()
                .find(|candidate| candidate.name == imported)?;
            return object_fields(target);
        }
        None
    }

    pub(super) async fn interface_markdown(
        &self,
        document_uri: &Url,
        symbol: &Symbol,
        source: &str,
        current_program: Option<&tyrus_core::ast::Program>,
    ) -> Option<String> {
        if symbol.kind == SymbolKind::Import {
            let module = symbol.import_source.as_deref()?;
            if module.starts_with("tyrus:") {
                let name = symbol.imported_name.as_deref().unwrap_or(&symbol.name);
                if tyrus_core::module_loader::native_export_enum(module, name).is_some() {
                    return None;
                }
                let (ty, fields, alias) =
                    tyrus_core::module_loader::native_export_info(module, name)?;
                if let Some(fields) = fields {
                    return Some(interface_hover_markdown(name, &fields, None, None));
                }
                if let Some(alias) = alias {
                    return Some(format!("```tyrus\ntype {name} = {alias}\n```"));
                }
                if matches!(ty, Type::Named(_)) {
                    return Some(format!("```tyrus\ntype {name}\n```"));
                }
                return None;
            }
        }
        let (name, target_source, link, documentation, target_analysis) =
            if symbol.kind == SymbolKind::Import {
                let location = self.imported_definition(document_uri, symbol).await?;
                let target_source = self.source(&location.uri).await?;
                let target_analysis = self
                    .workspace_analysis_for(&location.uri, &target_source)
                    .await;
                (
                    symbol.imported_name.as_deref().unwrap_or(&symbol.name),
                    target_source,
                    format!("{}#L{}", location.uri, location.range.start.line + 1),
                    None,
                    Some(target_analysis),
                )
            } else {
                (
                    symbol.name.as_str(),
                    source.to_string(),
                    String::new(),
                    symbol.documentation.clone(),
                    None,
                )
            };
        let behavior_program = target_analysis
            .as_ref()
            .and_then(|analysis| analysis.program.as_ref())
            .or(current_program);
        let behavior = if let Some(program) = behavior_program {
            program.items.iter().find_map(|item| match item {
                tyrus_core::ast::Item::BehaviorInterfaceDecl {
                    name: item_name, ..
                } if item_name == name => {
                    tyrus_core::module_loader::behavior_interface_info(item, &program.items)
                }
                _ => None,
            })
        } else {
            Parser::from_source(&target_source)
                .ok()
                .and_then(|mut parser| parser.parse_program().ok())
                .and_then(|program| {
                    program.items.iter().find_map(|item| match item {
                        tyrus_core::ast::Item::BehaviorInterfaceDecl {
                            name: item_name, ..
                        } if item_name == name => {
                            tyrus_core::module_loader::behavior_interface_info(item, &program.items)
                        }
                        _ => None,
                    })
                })
        };
        if let Some(behavior) = behavior {
            return Some(Self::behavior_interface_hover_markdown(
                name,
                &behavior.methods,
                &behavior.binding_methods,
                documentation.as_ref(),
                (!link.is_empty()).then_some(link.as_str()),
            ));
        }
        let info = self.interface_info(document_uri, symbol, source).await?;
        Some(interface_hover_markdown(
            &info.name,
            &info.fields,
            info.documentation.as_ref(),
            Some(&info.link),
        ))
    }

    fn behavior_interface_hover_markdown(
        name: &str,
        methods: &[tyrus_core::ast::BehaviorMethod],
        binding_methods: &[tyrus_core::ast::BehaviorMethod],
        documentation: Option<&DocComment>,
        definition_link: Option<&str>,
    ) -> String {
        let mut sections = Vec::new();
        if let Some(doc) = documentation {
            let markdown = doc.to_markdown();
            if !markdown.is_empty() {
                sections.push(markdown);
            }
        }
        let mut body = format!("interface {name} {{");
        for method in methods.iter().chain(binding_methods) {
            let params = method
                .params
                .iter()
                .map(|param| {
                    let name = match &param.pattern {
                        tyrus_core::ast::ParamPattern::Name(name) => name.as_str(),
                        _ => "value",
                    };
                    format!("{name}: {}", param.ty)
                })
                .collect::<Vec<_>>()
                .join(", ");
            body.push_str(&format!(
                "\n    {}({}): {};",
                method.name, params, method.ret_type
            ));
        }
        body.push_str("\n}");
        sections.push(format!("```tyrus\n{body}\n```"));
        if let Some(link) = definition_link {
            sections.push(format!("[Go to definition]({link})"));
        }
        sections.join("\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_export_location_uses_the_source_utf16_range() {
        let source = "const label = \"🦀\";\nexport const value = 1;";
        let uri = Url::parse("file:///workspace/module.tyr").unwrap();
        let analysis = tyrus_core::analysis::analyze(source);
        let symbol = analysis
            .symbols
            .iter()
            .find(|symbol| symbol.exported && symbol.name == "value")
            .expect("exported value exists");

        let location = indexed_export_location(source, &uri, symbol);

        assert_eq!(location.uri, uri);
        assert_eq!(location.range.start.line, 1);
        assert_eq!(location.range.start.character, 13);
        assert_eq!(location.range.end.character, 18);
    }

    #[test]
    fn indexed_exports_retain_type_checker_metadata() {
        let source = r#"
@model("users", { timestamps: false }) export interface User { name: string; }
export const displayName = (user: User): string => user.name;
export interface UserMethods at User {
    displayName(user: User): string { return user.name; }
}
export type UserId = string;
export enum UserState { Active = 0, Disabled = 1 }
export function loadUser(): User throws Error {}
"#;
        let analysis = tyrus_core::analysis::analyze(source);
        let metadata_for = |name: &str| {
            let symbol = analysis
                .symbols
                .iter()
                .find(|symbol| symbol.exported && symbol.name == name)
                .expect("exported symbol exists");
            indexed_export_metadata(&analysis, symbol)
        };

        assert!(matches!(
            metadata_for("loadUser"),
            IndexedExportMetadata::Function {
                is_validator: false,
                throws,
                ..
            } if !throws.is_empty()
        ));
        assert!(matches!(
            metadata_for("User"),
            IndexedExportMetadata::Interface {
                db_model: Some(_),
                fields,
                ..
            } if !fields.is_empty()
        ));
        assert!(matches!(
            metadata_for("UserMethods"),
            IndexedExportMetadata::Behavior { methods, .. } if !methods.is_empty()
        ));
        assert!(matches!(
            metadata_for("UserId"),
            IndexedExportMetadata::TypeAlias { .. }
        ));
        assert!(matches!(
            metadata_for("UserState"),
            IndexedExportMetadata::Enum { members, .. } if !members.is_empty()
        ));
    }
}
