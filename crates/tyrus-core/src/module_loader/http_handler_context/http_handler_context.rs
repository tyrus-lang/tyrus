use super::*;

#[derive(Clone, PartialEq)]
pub(super) struct HandlerContext {
    request: Type,
    reply: Type,
    pub(super) interfaces: HashMap<String, Vec<Field>>,
    pub(super) targets: HashMap<String, String>,
}

impl ModuleLoader<'_> {
    pub(super) fn prepare_http_handler_contexts(
        &mut self,
        path: &Path,
        program: &Program,
    ) -> Result<(), String> {
        for item in &program.items {
            let body = match item {
                Item::Stmt(Stmt {
                    kind:
                        StmtKind::VarDecl {
                            value:
                                Some(Expr {
                                    kind: ExprKind::ArrowFn { body, .. },
                                    ..
                                }),
                            ..
                        },
                    ..
                }) => body,
                Item::FnDecl { body, .. } => body,
                _ => continue,
            };
            for statement in body {
                let StmtKind::Expr(Expr {
                    kind: ExprKind::MethodCall { method, args, .. },
                    ..
                }) = statement.kind()
                else {
                    continue;
                };
                if !matches!(
                    method.as_str(),
                    "get"
                        | "post"
                        | "put"
                        | "patch"
                        | "delete"
                        | "head"
                        | "options"
                        | "connect"
                        | "trace"
                        | "query"
                ) {
                    continue;
                }
                let Some(Expr {
                    kind: ExprKind::ObjectLit(options),
                    ..
                }) = args.get(1)
                else {
                    continue;
                };
                let Some((
                    _,
                    Expr {
                        kind: ExprKind::Ident(handler),
                        ..
                    },
                )) = options.iter().find(|(name, _)| name == "handler")
                else {
                    continue;
                };
                let Some(import) = program.imports.iter().find(|import| {
                    import
                        .specifiers
                        .iter()
                        .any(|specifier| specifier.local == *handler)
                }) else {
                    continue;
                };
                if import.source.starts_with("tyrus:") {
                    continue;
                }
                let Some(specifier) = import
                    .specifiers
                    .iter()
                    .find(|specifier| specifier.local == *handler)
                else {
                    continue;
                };
                let Some(context) = self.route_handler_context(path, program, options)? else {
                    continue;
                };
                let dependency = resolve_import_path_in_project(
                    &self.root,
                    &self.imports,
                    path,
                    &import.source,
                )?;
                if self.cache.contains_key(&dependency) {
                    continue;
                }
                if !export_needs_route_context(&dependency, &specifier.imported)? {
                    continue;
                }
                let contexts = self.handler_contexts.entry(dependency.clone()).or_default();
                if let Some(existing) = contexts.get(&specifier.imported) {
                    if existing != &context {
                        return Err(format!(
                            "error: imported HTTP handler `{handler}` is used by routes with different request or response contracts in `{}`",
                            path.display()
                        ));
                    }
                } else {
                    contexts.insert(specifier.imported.clone(), context);
                }
            }
        }
        Ok(())
    }

    fn route_handler_context(
        &mut self,
        path: &Path,
        program: &Program,
        options: &[(String, Expr)],
    ) -> Result<Option<HandlerContext>, String> {
        let mut request = Vec::new();
        let mut response = Vec::new();
        let mut names = HashSet::default();
        for (name, value) in options {
            if matches!(
                name.as_str(),
                "body" | "params" | "query" | "headers" | "cookies"
            ) {
                let ExprKind::Ident(schema) = value.kind() else {
                    return Ok(None);
                };
                names.insert(schema.clone());
                request.push(context_field(
                    name,
                    crate::http_body::request_channel_type(name, schema),
                ));
            } else if name == "response" {
                match value.kind() {
                    ExprKind::ObjectLit(statuses) => {
                        for (status, schema) in statuses {
                            let Some(ty) = response_schema_type(schema) else {
                                return Ok(None);
                            };
                            collect_named_types(&ty, &mut names);
                            response.push(context_field(status, ty));
                        }
                    }
                    _ => {
                        let Some(ty) = response_schema_type(value) else {
                            return Ok(None);
                        };
                        collect_named_types(&ty, &mut names);
                        response.push(context_field("200", ty));
                    }
                }
            }
        }
        if response.is_empty() {
            return Ok(None);
        }
        let mut interfaces = HashMap::default();
        let mut targets = HashMap::default();
        for name in names {
            let Some((import, specifier)) = program.imports.iter().find_map(|import| {
                import
                    .specifiers
                    .iter()
                    .find(|specifier| specifier.local == name)
                    .map(|specifier| (import, specifier))
            }) else {
                continue;
            };
            if import.source.starts_with("tyrus:") {
                continue;
            }
            let dependency =
                resolve_import_path_in_project(&self.root, &self.imports, path, &import.source)?;
            let exports = self.resolve_module(&dependency, false)?;
            if let Some(export) = exports.get(&specifier.imported)
                && let Some(fields) = &export.interface_fields
            {
                targets.insert(name.clone(), export.target.clone());
                interfaces.insert(name, fields.clone());
            }
        }
        Ok(Some(HandlerContext {
            request: Type::Object(request),
            reply: Type::Apply {
                name: "HttpReply".to_string(),
                args: vec![Type::Object(response)],
            },
            interfaces,
            targets,
        }))
    }
}

pub(super) fn apply_handler_contexts(
    program: &mut Program,
    contexts: &HashMap<String, HandlerContext>,
) {
    for (exported, context) in contexts {
        let Some(local) = program.exports.iter().find_map(|export| match export {
            ExportDecl::Named {
                exported: name,
                local,
                ..
            } if name == exported => Some(local.clone()),
            _ => None,
        }) else {
            continue;
        };
        let Some(item) = program
            .items
            .iter_mut()
            .find(|item| item_name(item) == Some(local.as_str()))
        else {
            continue;
        };
        let (params, return_type, is_async) = match item {
            Item::FnDecl {
                params,
                ret_type,
                is_async,
                ..
            } => (params, ret_type, *is_async),
            Item::Stmt(Stmt {
                kind:
                    StmtKind::VarDecl {
                        value:
                            Some(Expr {
                                kind:
                                    ExprKind::ArrowFn {
                                        params,
                                        ret_type,
                                        is_async,
                                        ..
                                    },
                                ..
                            }),
                        ..
                    },
                ..
            }) => (params, ret_type, *is_async),
            _ => continue,
        };
        if params.len() == 2 {
            for (parameter, inferred) in params.iter_mut().zip([&context.request, &context.reply]) {
                if matches!(parameter.ty, Type::Infer) {
                    parameter.ty = inferred.clone();
                }
            }
        }
        if matches!(return_type, Type::Infer) {
            *return_type = if is_async {
                Type::Apply {
                    name: "Promise".to_string(),
                    args: vec![Type::Void],
                }
            } else {
                Type::Void
            };
        }
    }
}

fn context_field(name: &str, ty: Type) -> Field {
    Field {
        name: name.to_string(),
        ty,
        readonly: true,
        optional: false,
        decorators: Vec::new(),
        span: Span::DUMMY,
    }
}

fn response_schema_type(value: &Expr) -> Option<Type> {
    match value.kind() {
        ExprKind::Ident(name) => Some(match name.as_str() {
            "void" => Type::Void,
            "string" => Type::Str,
            "boolean" => Type::Bool,
            "i32" => Type::I32,
            "i64" => Type::I64,
            "u32" => Type::U32,
            "u64" => Type::U64,
            "f32" => Type::F32,
            "f64" => Type::F64,
            _ => Type::Named(name.clone()),
        }),
        ExprKind::TypeAssert { ty, .. } => Some(ty.clone()),
        _ => None,
    }
}

fn collect_named_types(ty: &Type, names: &mut HashSet<String>) {
    match ty {
        Type::Named(name) => {
            names.insert(name.clone());
        }
        Type::Array(inner) | Type::Optional(inner) | Type::Readonly(inner) => {
            collect_named_types(inner, names)
        }
        _ => {}
    }
}

fn export_needs_route_context(path: &Path, exported: &str) -> Result<bool, String> {
    let source = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let (_, program) = crate::analysis::parse_for_analysis(&source).map_err(|_| {
        format!(
            "error: could not parse HTTP controller `{}`",
            path.display()
        )
    })?;
    let Some(local) = program.exports.iter().find_map(|export| match export {
        ExportDecl::Named {
            exported: name,
            local,
            ..
        } if name == exported => Some(local),
        _ => None,
    }) else {
        return Ok(false);
    };
    let params = program.items.iter().find_map(|item| {
        if item_name(item) != Some(local.as_str()) {
            return None;
        }
        match item {
            Item::FnDecl { params, .. } => Some(params.as_slice()),
            Item::Stmt(Stmt {
                kind:
                    StmtKind::VarDecl {
                        value:
                            Some(Expr {
                                kind: ExprKind::ArrowFn { params, .. },
                                ..
                            }),
                        ..
                    },
                ..
            }) => Some(params.as_slice()),
            _ => None,
        }
    });
    Ok(params.is_some_and(|params| {
        params.len() == 2 && params.iter().any(|param| matches!(param.ty, Type::Infer))
    }))
}
