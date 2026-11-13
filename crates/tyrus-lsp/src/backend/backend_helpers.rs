use super::*;

pub(crate) fn global_builtin_call_signature(callee: &str) -> Option<CallSignature> {
    match callee {
        "parseDecimal" => Some(CallSignature {
            parameters: vec!["text".to_string()],
            detail: Some("(text: string) => decimal?".to_string()),
        }),
        "isNaN" => Some(CallSignature {
            parameters: vec!["value".to_string()],
            detail: Some("(value: f32 | f64 | f128) => boolean".to_string()),
        }),
        _ => None,
    }
}

pub(crate) fn native_crypto_call_signature(
    analysis: &DocumentAnalysis,
    callee: &str,
) -> Option<CallSignature> {
    let (receiver, rest) = callee.split_once('.')?;
    let method = rest.rsplit_once('.').map_or(rest, |(_, method)| method);
    let symbol = analysis.symbol_named(receiver).filter(|symbol| {
        symbol.kind == SymbolKind::Import
            && matches!(
                symbol.import_source.as_deref(),
                Some("tyrus:crypto" | "tyrus:encoding")
            )
    })?;
    let module = symbol.import_source.as_deref()?;
    let imported = symbol.imported_name.as_deref().unwrap_or(&symbol.name);
    let namespace = if module == "tyrus:crypto" && callee.contains(".aes.") {
        "cipher.aes"
    } else {
        imported
    };
    let fields = if module == "tyrus:crypto" && namespace == "cipher.aes" {
        Some(tyrus_core::crypto::cipher_aes_namespace_fields())
    } else {
        tyrus_core::module_loader::native_export_info(module, imported)
            .and_then(|(_, fields, _)| fields)
    }?;
    let names = if module == "tyrus:encoding" {
        match namespace {
            "hex" => match method {
                "encode" => vec!["bytes"],
                "decode" => vec!["text"],
                _ => return None,
            },
            "base64" | "base64Url" | "base32" => match method {
                "encode" => vec!["bytes"],
                "decode" => vec!["text"],
                _ => return None,
            },
            "radix" => match method {
                "format" => vec!["value", "base"],
                "parseI64" | "parseU64" => vec!["text", "base"],
                _ => return None,
            },
            _ => match method {
                "encode" | "encodeLE" | "encodeBE" => vec!["text"],
                "decode" | "decodeLossy" | "decodeLE" | "decodeLELossy" | "decodeBE"
                | "decodeBELossy" => vec!["bytes"],
                _ => return None,
            },
        }
    } else {
        tyrus_core::crypto::crypto_method_parameter_names(namespace, method)?.to_vec()
    };
    let field = fields.into_iter().find(|field| field.name == method)?;
    Some(CallSignature {
        parameters: names.into_iter().map(str::to_string).collect(),
        detail: Some(field.ty.to_string()),
    })
}

pub(crate) fn receiver_semantic_type(analysis: &DocumentAnalysis, receiver: &str) -> Option<Type> {
    analysis
        .symbols
        .iter()
        .find(|symbol| symbol.name == receiver)
        .and_then(|symbol| symbol.semantic_type.clone())
}

pub(crate) fn tasks_instance_call_signature(
    receiver: &Type,
    method: &str,
) -> Option<CallSignature> {
    if !is_tasks_native_type(receiver) {
        return None;
    }
    let detail = crate::completion::member_completion_items_for_type(receiver)?
        .into_iter()
        .find(|item| item.label == method)?
        .detail?;
    Some(CallSignature {
        parameters: signature_parameter_names(&detail),
        detail: Some(detail),
    })
}

pub(crate) fn automatic_data_call_signature(
    receiver: &Type,
    method: &str,
) -> Option<CallSignature> {
    let Type::Named(name) = receiver else {
        return None;
    };
    let (parameters, detail) = match method {
        "copy" => (Vec::new(), format!("() => {name}")),
        "toString" => (Vec::new(), "() => string".to_string()),
        "toJson" => (
            vec!["options?: { pretty: boolean }".to_string()],
            "(options?: { pretty: boolean }) => string".to_string(),
        ),
        "equals" => (
            vec!["other".to_string()],
            format!("(other: {name}) => boolean"),
        ),
        _ => return None,
    };
    Some(CallSignature {
        parameters,
        detail: Some(detail),
    })
}

pub(crate) fn automatic_data_static_call_signature(
    analysis: &DocumentAnalysis,
    receiver: &str,
    method: &str,
) -> Option<CallSignature> {
    if method != "fromJson"
        || !analysis.symbols.iter().any(|symbol| {
            symbol.name == receiver
                && symbol.kind == SymbolKind::Interface
                && matches!(symbol.semantic_type, Some(Type::Object(_)))
        })
    {
        return None;
    }
    Some(CallSignature {
        parameters: vec!["text".to_string()],
        detail: Some(format!("(text: string) => Result<{receiver}, JsonError>")),
    })
}

pub(crate) fn is_tasks_native_type(ty: &Type) -> bool {
    match ty {
        Type::Apply { name, args } if args.len() == 1 => matches!(
            name.as_str(),
            "JoinHandle" | "Sender" | "Receiver" | "Mutex" | "RwLock"
        ),
        Type::Named(name) => name == "Semaphore",
        _ => false,
    }
}

pub(crate) fn signature_parameter_names(detail: &str) -> Vec<String> {
    let Some(open) = detail.find('(') else {
        return Vec::new();
    };
    let Some(close) = detail[open..].find(')').map(|offset| open + offset) else {
        return Vec::new();
    };
    detail[open + 1..close]
        .split(',')
        .filter_map(|parameter| {
            let name = parameter
                .split(':')
                .next()
                .map(str::trim)
                .unwrap_or_default()
                .trim_end_matches('?');
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect()
}

pub(crate) fn native_promise_call_signature(callee: &str) -> Option<CallSignature> {
    let (parameters, detail) = match callee {
        "Promise.resolve" => (vec!["value"], "(value: T) => Promise<T>"),
        "Promise.reject" => (vec!["reason"], "(reason: Error) => Promise<unknown>"),
        "Promise.all" => (vec!["values"], "(values: Promise<T>[]) => Promise<T[]>"),
        "Promise.allSettled" => (
            vec!["values"],
            "(values: Promise<T>[]) => Promise<PromiseSettledResult<T>[]>",
        ),
        "Promise.any" | "Promise.race" => (vec!["values"], "(values: Promise<T>[]) => Promise<T>"),
        "Promise.withResolvers" => (Vec::new(), "() => PromiseWithResolvers<T>"),
        "then" => (
            vec!["onFulfilled"],
            "(onFulfilled: (value: T) => U | Promise<U>) => Promise<U>",
        ),
        "catch" => (
            vec!["onRejected"],
            "(onRejected: (reason: Error) => T | Promise<T>) => Promise<T>",
        ),
        "finally" => (
            vec!["onFinally"],
            "(onFinally: () => void | Promise<void>) => Promise<T>",
        ),
        _ => return None,
    };
    Some(CallSignature {
        parameters: parameters.into_iter().map(str::to_string).collect(),
        detail: Some(detail.to_string()),
    })
}

pub(crate) fn parameter_names(symbol: &Symbol) -> Vec<String> {
    symbol
        .parameters
        .iter()
        .map(|parameter| crate::completion::parameter_pattern_argument(&parameter.pattern))
        .collect()
}

pub(crate) fn anonymous_object_binding_fields(
    source: &str,
    diagnostic: &AnalysisDiagnostic,
) -> Option<Vec<Field>> {
    let name = source.get(diagnostic.range.clone())?;
    let tokens = tyrus_core::lexer::lex(source).ok()?;
    let program = Parser::new(tokens).parse_program().ok()?;
    find_object_binding_fields_in_items(&program.items, name)
}

pub(crate) fn find_object_binding_fields_in_items(
    items: &[Item],
    name: &str,
) -> Option<Vec<Field>> {
    for item in items {
        match item {
            Item::Stmt(statement) => {
                if let Some(fields) = find_object_binding_fields_in_stmt(statement, name) {
                    return Some(fields);
                }
            }
            Item::FnDecl { body, .. } => {
                if let Some(fields) = find_object_binding_fields_in_stmts(body, name) {
                    return Some(fields);
                }
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn find_object_binding_fields_in_stmts(
    statements: &[Stmt],
    name: &str,
) -> Option<Vec<Field>> {
    for statement in statements {
        if let Some(fields) = find_object_binding_fields_in_stmt(statement, name) {
            return Some(fields);
        }
    }
    None
}

pub(crate) fn find_object_binding_fields_in_stmt(
    statement: &Stmt,
    name: &str,
) -> Option<Vec<Field>> {
    match statement.kind() {
        StmtKind::VarDecl {
            name: binding,
            ty,
            value:
                Some(Expr {
                    kind: ExprKind::ObjectLit(fields),
                    ..
                }),
            ..
        } if binding == name && matches!(ty, Type::Infer) => infer_object_literal_fields(fields),
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::ForIn { body, .. }
        | StmtKind::For { body, .. } => find_object_binding_fields_in_stmts(body, name),
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => find_object_binding_fields_in_stmts(try_body, name)
            .or_else(|| find_object_binding_fields_in_stmts(catch_body, name))
            .or_else(|| {
                finally_body
                    .as_deref()
                    .and_then(|body| find_object_binding_fields_in_stmts(body, name))
            }),
        StmtKind::Switch { cases, default, .. } => cases
            .iter()
            .find_map(|case| find_object_binding_fields_in_stmts(&case.body, name))
            .or_else(|| {
                default
                    .as_deref()
                    .and_then(|body| find_object_binding_fields_in_stmts(body, name))
            }),
        StmtKind::Match { arms, default, .. } => arms
            .iter()
            .find_map(|arm| find_object_binding_fields_in_stmts(&arm.body, name))
            .or_else(|| {
                default
                    .as_deref()
                    .and_then(|body| find_object_binding_fields_in_stmts(body, name))
            }),
        _ => None,
    }
}

pub(crate) fn infer_object_literal_fields(fields: &[(String, Expr)]) -> Option<Vec<Field>> {
    fields
        .iter()
        .map(|(name, value)| {
            infer_literal_type(value).map(|ty| Field {
                name: name.clone(),
                ty,
                readonly: false,
                optional: false,
                decorators: Vec::new(),
                span: Span::DUMMY,
            })
        })
        .collect()
}

pub(crate) fn infer_literal_type(expr: &Expr) -> Option<Type> {
    match expr.kind() {
        ExprKind::StrLit(_) | ExprKind::TemplateLit(_) => Some(Type::Str),
        ExprKind::RegexLit { .. } => Some(Type::Regex),
        ExprKind::BoolLit(_) => Some(Type::Bool),
        ExprKind::IntLit(value) => {
            if *value < i128::from(i32::MIN) || *value > i128::from(i32::MAX) {
                Some(Type::I64)
            } else {
                Some(Type::I32)
            }
        }
        ExprKind::FloatLit(_) => Some(Type::F64),
        ExprKind::ArrayLit(values) => {
            let mut inferred = values
                .iter()
                .filter_map(|value| match value {
                    Expr {
                        kind: ExprKind::Spread(_),
                        ..
                    } => None,
                    other => infer_literal_type(other),
                })
                .collect::<Vec<_>>();
            if inferred.is_empty() {
                return None;
            }
            let first = inferred.remove(0);
            inferred
                .iter()
                .all(|ty| ty == &first)
                .then(|| Type::Array(Box::new(first)))
        }
        _ => None,
    }
}

pub(crate) fn exact_interface_match(expected_fields: &[Field], actual_fields: &[Field]) -> bool {
    expected_fields.len() == actual_fields.len()
        && expected_fields.iter().all(|expected| {
            !expected.optional
                && !expected.readonly
                && actual_fields
                    .iter()
                    .find(|actual| actual.name == expected.name)
                    .is_some_and(|actual| {
                        !actual.optional && !actual.readonly && actual.ty == expected.ty
                    })
        })
        && actual_fields.iter().all(|actual| {
            expected_fields
                .iter()
                .any(|expected| expected.name == actual.name)
        })
}

pub(crate) fn object_interface_help(
    matches: &[&ObjectInterfaceCandidate],
    current_path: &Path,
) -> Option<String> {
    let exported = matches
        .iter()
        .copied()
        .filter(|candidate| candidate.exported)
        .collect::<Vec<_>>();
    if exported.len() == 1 {
        let candidate = exported[0];
        let module = import_source_for_export(Some(current_path), &candidate.uri)?;
        return Some(format!(
            "import `{{ {} }}` from `{}` or annotate the binding as `: {}`",
            candidate.name, module, candidate.name
        ));
    }
    if exported.len() > 1 {
        return Some(format!(
            "multiple exported interfaces match this object; add an explicit annotation such as `: {}`",
            exported[0].name
        ));
    }
    if matches.len() == 1 {
        let candidate = matches[0];
        return Some(format!(
            "interface `{}` in `{}` matches this object but is not exported; add `export` before the interface or annotate with a local interface",
            candidate.name,
            candidate.uri.path()
        ));
    }
    (!matches.is_empty()).then(|| {
        "multiple private interfaces match this object; export the intended interface and add an explicit annotation"
            .to_string()
    })
}

pub(crate) fn env_file_missing_diagnostic(root: &Path, message: &str) -> Option<Diagnostic> {
    let config_path = root.join("tyrus.json");
    let source = fs::read_to_string(&config_path).ok()?;
    let key = source.find("\"envFile\"")?;
    let colon = source[key..].find(':')? + key;
    let value_start = source[colon + 1..].find('"')? + colon + 2;
    let value_end = source[value_start..].find('"')? + value_start;
    Some(Diagnostic {
        range: byte_range_to_lsp(&source, value_start..value_end),
        severity: Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR),
        source: Some("tyrus".to_string()),
        code: Some(NumberOrString::String("E3083".to_string())),
        message: message.to_string(),
        related_information: None,
        data: Some(serde_json::json!({
            "help": ["create the env file or update `security.envFile` in tyrus.json"],
        })),
        ..Diagnostic::default()
    })
}

pub(crate) fn env_schema_for_document(file: &Path) -> Option<tyrus_core::env::EnvSchema> {
    let root = tyrus_core::project::find_project_root(file)?;
    let config = tyrus_core::project::load_project(&root).ok()?;
    Some(tyrus_core::env::load_env_schema(&root, &config).ok()??.0)
}

pub(crate) fn is_lint_owned_analysis_diagnostic(diagnostic: &AnalysisDiagnostic) -> bool {
    diagnostic.code == tyrus_core::definite_assignment::USE_BEFORE_ASSIGN
        || (diagnostic.code == tyrus_core::definite_assignment::NEVER_ASSIGNED
            && diagnostic
                .message
                .contains("declared but never assigned a value"))
}

pub(crate) fn update_workspace_files_cache_for_root(
    cache: &mut std::collections::HashMap<PathBuf, WorkspaceFilesCacheEntry>,
    changed_root: &Path,
    generation: WorkspaceGeneration,
    inventory_changed: bool,
) {
    if inventory_changed {
        cache.remove(changed_root);
    } else if let Some(entry) = cache.get_mut(changed_root) {
        entry.generation = generation;
    }
}

pub(crate) fn exported_interface_fields(
    analysis: &DocumentAnalysis,
    name: &str,
) -> Option<Vec<Field>> {
    analysis
        .symbols
        .iter()
        .find(|symbol| symbol.kind == SymbolKind::Interface && symbol.name == name)
        .and_then(|symbol| match &symbol.semantic_type {
            Some(Type::Object(fields)) => Some(fields.clone()),
            _ => None,
        })
}

pub(crate) fn workspace_export_is_visible(export: &IndexedExport, for_file: Option<&Path>) -> bool {
    if !export.is_protected {
        return true;
    }
    let Some(importer) = for_file else {
        return true;
    };
    export.uri.to_file_path().is_ok_and(|declaring_module| {
        tyrus_core::module_loader::protect_scope_allows(importer, &declaring_module)
    })
}

pub(crate) fn analysis_is_cancelled(analysis: &DocumentAnalysis) -> bool {
    analysis.program.is_none()
        && analysis.symbols.is_empty()
        && analysis.references.is_empty()
        && analysis.diagnostics.is_empty()
}

pub(crate) fn analysis_retry_allowed(
    analysis: &DocumentAnalysis,
    document_is_current: bool,
    attempt: usize,
) -> bool {
    analysis_is_cancelled(analysis) && document_is_current && attempt < 2
}
