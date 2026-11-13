use crate::{LintContext, LintFix, RuleDiagnostic, SpannedToken};
use std::ops::Range;
use std::sync::atomic::{AtomicBool, Ordering};
use tyrus_core::analysis::SymbolKind;
use tyrus_core::ast::{Expr, ExprKind, ImportKind, Item, Program, Stmt, StmtKind, Type};
use tyrus_core::collections::HashSet;
use tyrus_core::lexer::Token;
use tyrus_core::type_checker::is_mutating_method;

pub fn run_cancellable(
    rule: &str,
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    if cancellation_requested(cancellation) {
        return Vec::new();
    }
    match rule {
        "no-unused-vars" => return no_unused_vars_cancellable(ctx, cancellation),
        "no-unused-imports" => return no_unused_imports_cancellable(ctx, cancellation),
        "no-unused-functions" => return no_unused_functions_cancellable(ctx, cancellation),
        "no-null-literal" => return no_null_literal_cancellable(ctx, cancellation),
        "no-duplicate-imports" => return no_duplicate_imports_cancellable(ctx, cancellation),
        "no-force-unwrap" => return no_force_unwrap_cancellable(ctx, cancellation),
        "no-unreachable-code" => return no_unreachable_code_cancellable(ctx, cancellation),
        "no-duplicate-case" => return no_duplicate_case_cancellable(ctx, cancellation),
        "no-shadow" => return no_shadow_cancellable(ctx, cancellation),
        "prefer-const" => return prefer_const_cancellable(ctx, cancellation),
        "prefer-nullish-coalescing" => {
            return prefer_nullish_coalescing_cancellable(ctx, cancellation);
        }
        "no-unnecessary-allocation" => {
            return no_unnecessary_allocation_cancellable(ctx, cancellation);
        }
        "no-unnecessary-await" => return no_unnecessary_await_cancellable(ctx, cancellation),
        "no-string-concat-in-loop" => {
            return no_string_concat_in_loop_cancellable(ctx, cancellation);
        }
        "no-uninitialized-variable" => {
            return no_uninitialized_variable_cancellable(ctx, cancellation);
        }
        "no-unsafe-finally" => return no_unsafe_finally_cancellable(ctx, cancellation),
        "no-empty-catch" => return no_empty_catch_cancellable(ctx, cancellation),
        "no-empty-block" => return no_empty_block_cancellable(ctx, cancellation),
        "use-caught-error" => return use_caught_error_cancellable(ctx, cancellation),
        _ => {}
    }
    let diagnostics = match rule {
        "api-no-verb-in-path" => api_no_verb_in_path(ctx, cancellation),
        "api-prefer-resource-nouns" => api_prefer_resource_nouns(ctx, cancellation),
        "controller-no-try-catch" => controller_no_try_catch(ctx, cancellation),
        "db-explicit-select-columns" => db_explicit_select_columns(ctx, cancellation),
        "db-no-extra-select-columns" => db_no_extra_select_columns(ctx, cancellation),
        "http-no-raw-status-code" => http_no_raw_status_code(ctx, cancellation),
        "export-protect-no-effect" => export_protect_no_effect(ctx, cancellation),
        "no-layer-skip" => no_layer_skip(ctx, cancellation),
        "no-unused-vars" => no_unused_vars(ctx),
        "no-unused-imports" => no_unused_imports(ctx),
        "no-unused-functions" => no_unused_functions(ctx),
        "no-duplicate-imports" => no_duplicate_imports(ctx),
        "no-upward-imports" => no_upward_imports_cancellable(ctx, cancellation),
        "prefer-const" => prefer_const(ctx),
        "no-shadow" => no_shadow(ctx),
        "no-null-literal" => no_null_literal(ctx),
        "no-force-unwrap" => no_force_unwrap(ctx),
        "no-unreachable-code" => no_unreachable_code(ctx),
        "no-unsafe-finally" => no_unsafe_finally(ctx),
        "no-empty-catch" => no_empty_catch(ctx),
        "no-empty-block" => no_empty_block(ctx),
        "use-caught-error" => use_caught_error(ctx),
        "no-duplicate-case" => no_duplicate_case(ctx),
        "prefer-nullish-coalescing" => prefer_nullish_coalescing(ctx),
        "no-unnecessary-await" => no_unnecessary_await(ctx),
        "no-unnecessary-allocation" => no_unnecessary_allocation(ctx),
        "no-uninitialized-variable" => no_uninitialized_variable_cancellable(ctx, cancellation),
        "no-string-concat-in-loop" => no_string_concat_in_loop(ctx),
        "repository-no-http-error" => repository_no_http_error(ctx, cancellation),
        "route-no-business-logic" => route_no_business_logic(ctx, cancellation),
        "service-no-reply" => service_no_reply(ctx, cancellation),
        _ => Vec::new(),
    };
    if cancellation_requested(cancellation) {
        Vec::new()
    } else {
        diagnostics
    }
}

fn cancellation_requested(cancellation: Option<&AtomicBool>) -> bool {
    cancellation.is_some_and(|signal| signal.load(Ordering::Acquire))
}

fn db_no_extra_select_columns(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let mut diagnostics = Vec::new();
    for index in 0..ctx.tokens.len().saturating_sub(7) {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if !matches!(&ctx.tokens[index].token, Token::Ident(name) if name == "db")
            || !matches!(ctx.tokens[index + 1].token, Token::Dot)
            || !matches!(&ctx.tokens[index + 2].token, Token::Ident(name) if name == "query")
            || !matches!(ctx.tokens[index + 3].token, Token::Lt)
        {
            continue;
        }
        let Token::Ident(interface_name) = &ctx.tokens[index + 4].token else {
            continue;
        };
        if !matches!(ctx.tokens[index + 5].token, Token::Gt)
            || !matches!(ctx.tokens[index + 6].token, Token::LParen)
        {
            continue;
        }
        let Some(sql_token) = ctx.tokens.get(index + 7) else {
            continue;
        };
        let Token::StringLit(sql) = &sql_token.token else {
            continue;
        };
        let Ok(select) = tyrus_core::db::parse_select_sql(sql) else {
            continue;
        };
        if select.select_all {
            continue;
        }
        let Some((fields, model)) = ctx.program.items.iter().find_map(|item| match item {
            Item::InterfaceDecl {
                name,
                fields,
                db_model,
                ..
            } if name == interface_name => Some((fields, db_model.as_ref())),
            _ => None,
        }) else {
            continue;
        };
        if fields.is_empty() {
            continue;
        }
        let mapped_columns = model
            .map(|model| {
                model
                    .fields
                    .iter()
                    .map(|metadata| (metadata.field.as_str(), metadata.column.as_str()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let outputs = fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<HashSet<_>>();
        for projection in select.projections {
            let output_name = projection.alias.as_deref().unwrap_or_else(|| {
                mapped_columns
                    .iter()
                    .find(|(_, column)| *column == projection.name)
                    .map_or(projection.name.as_str(), |(field, _)| *field)
            });
            if outputs.contains(output_name) {
                continue;
            }
            let range = projection.alias_span.unwrap_or(projection.span);
            let start = sql_token.range.start + 1 + range.start;
            let end = sql_token.range.start + 1 + range.end;
            diagnostics.push(RuleDiagnostic {
                range: start..end,
                message: format!(
                    "SQL projection `{output_name}` is not declared by `{interface_name}`"
                ),
                help: Some(format!(
                    "remove `{output_name}` from the SELECT list or add it to `{interface_name}`"
                )),
                note: Some(
                    "the row decoder only retains fields declared by the result interface"
                        .to_string(),
                ),
                fix: None,
            });
        }
    }
    diagnostics
}

fn db_explicit_select_columns(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let mut diagnostics = Vec::new();
    for index in 0..ctx.tokens.len().saturating_sub(6) {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if !matches!(&ctx.tokens[index].token, Token::Ident(name) if name == "db")
            || !matches!(ctx.tokens[index + 1].token, Token::Dot)
            || !matches!(&ctx.tokens[index + 2].token, Token::Ident(name) if name == "query")
            || !matches!(ctx.tokens[index + 3].token, Token::Lt)
        {
            continue;
        }
        let Token::Ident(interface_name) = &ctx.tokens[index + 4].token else {
            continue;
        };
        if !matches!(ctx.tokens[index + 5].token, Token::Gt)
            || !matches!(ctx.tokens[index + 6].token, Token::LParen)
        {
            continue;
        }
        let Some((sql, sql_range)) = ctx.tokens.get(index + 7).and_then(|token| {
            let Token::StringLit(sql) = &token.token else {
                return None;
            };
            Some((sql, token.range.clone()))
        }) else {
            continue;
        };
        let Ok(sql_tokens) = tyrus_core::db::tokenize_sql(sql) else {
            continue;
        };
        if !matches!(
            sql_tokens.as_slice(),
            [select, star, from, ..]
                if select.text.eq_ignore_ascii_case("SELECT")
                    && star.kind == tyrus_core::db::SqlTokenKind::Star
                    && from.text.eq_ignore_ascii_case("FROM")
        ) {
            continue;
        }
        let Some((fields, db_model)) = ctx.program.items.iter().find_map(|item| match item {
            Item::InterfaceDecl {
                name,
                fields,
                db_model,
                ..
            } if name == interface_name => Some((fields, db_model.as_ref())),
            _ => None,
        }) else {
            continue;
        };
        if fields.is_empty() {
            continue;
        }
        let raw = ctx.source.get(sql_range.clone()).unwrap_or_default();
        if !raw.starts_with('"') || raw.get(1..raw.len().saturating_sub(1)) != Some(sql.as_str()) {
            continue;
        }
        let columns = fields
            .iter()
            .filter(|field| {
                !(db_model.is_some_and(|model| model.timestamps)
                    && matches!(field.name.as_str(), "createdAt" | "updatedAt"))
            })
            .map(|field| {
                let name = db_model
                    .and_then(|model| model.fields.iter().find(|meta| meta.field == field.name))
                    .map_or(field.name.as_str(), |meta| meta.column.as_str());
                format!("\"{}\"", name.replace('"', "\"\""))
            })
            .collect::<Vec<_>>()
            .join(", ");
        let star_range = sql_range.start + 1 + sql_tokens[1].span.start
            ..sql_range.start + 1 + sql_tokens[1].span.end;
        let has_join = sql_tokens.iter().any(|token| {
            token.kind == tyrus_core::db::SqlTokenKind::Word
                && token.text.eq_ignore_ascii_case("JOIN")
        });
        diagnostics.push(RuleDiagnostic {
            range: star_range.clone(),
            message: "prefer an explicit projection over `SELECT *`".to_string(),
            help: Some("select only the fields required by the result interface".to_string()),
            note: Some(format!(
                "`{interface_name}` declares the exact result fields"
            )),
            fix: (!has_join).then(|| LintFix {
                message: "Replace `*` with the fields declared by the result interface".to_string(),
                range: star_range,
                replacement: columns,
            }),
        });
    }
    diagnostics
}

const HTTP_ROUTE_METHODS: &[&str] = &[
    "get", "post", "put", "patch", "delete", "head", "options", "connect", "trace", "all",
];

const PATH_VERBS: &[&str] = &[
    "add", "create", "delete", "edit", "fetch", "get", "list", "remove", "save", "search", "set",
    "update",
];

#[derive(Debug, Clone)]
struct RoutePath<'a> {
    path: &'a str,
    range: Range<usize>,
}

fn route_paths<'a>(ctx: &'a LintContext, cancellation: Option<&AtomicBool>) -> Vec<RoutePath<'a>> {
    let mut paths = Vec::new();
    let tokens = ctx.tokens;
    for index in 0..tokens.len() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if !matches!(tokens[index].token, Token::Dot) {
            continue;
        }
        let is_route_method = matches!(tokens.get(index + 1).map(|token| &token.token), Some(Token::Ident(method)) if HTTP_ROUTE_METHODS.contains(&method.as_str()));
        if !is_route_method
            || !matches!(
                tokens.get(index + 2).map(|token| &token.token),
                Some(Token::LParen)
            )
        {
            continue;
        }
        let Some(path_token) = tokens.get(index + 3) else {
            continue;
        };
        let Token::StringLit(path) = &path_token.token else {
            continue;
        };
        if !path.starts_with('/') {
            continue;
        }
        paths.push(RoutePath {
            path,
            range: path_token.range.clone(),
        });
    }
    paths
}

fn route_static_segments(path: &str) -> Vec<&str> {
    path.trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .filter(|segment| !segment.starts_with(':') && !segment.starts_with('{'))
        .collect()
}

fn segment_starts_with_verb(segment: &str) -> Option<&'static str> {
    let original = segment.trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
    let normalized = segment
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric())
        .to_ascii_lowercase();
    PATH_VERBS.iter().copied().find(|verb| {
        normalized == *verb
            || normalized
                .strip_prefix(*verb)
                .is_some_and(|rest| rest.starts_with('-') || rest.starts_with('_'))
            || original.strip_prefix(*verb).is_some_and(|rest| {
                rest.chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_uppercase())
            })
    })
}

fn api_no_verb_in_path(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    route_paths(ctx, cancellation)
        .into_iter()
        .filter_map(|route| {
            let verb = route_static_segments(route.path)
                .into_iter()
                .find_map(segment_starts_with_verb)?;
            Some(RuleDiagnostic {
                range: route.range,
                message: format!("avoid action verb `{verb}` in HTTP route path"),
                help: Some(
                    "model the route as a resource path and choose the action with the HTTP method"
                        .to_string(),
                ),
                note: None,
                fix: None,
            })
        })
        .collect()
}

fn api_prefer_resource_nouns(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    route_paths(ctx, cancellation)
        .into_iter()
        .filter_map(|route| {
            let first = route_static_segments(route.path).into_iter().next()?;
            let verb = segment_starts_with_verb(first)?;
            Some(RuleDiagnostic {
                range: route.range,
                message: format!("HTTP route path should start with a resource noun, not `{verb}`"),
                help: Some("prefer paths such as `/users`, `/products`, or `/orders`".to_string()),
                note: None,
                fix: None,
            })
        })
        .collect()
}

fn http_no_raw_status_code(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let tokens = ctx.tokens;
    let mut diagnostics = Vec::new();
    for index in 0..tokens.len() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if !matches!(tokens[index].token, Token::Dot) {
            continue;
        }
        let is_status_call = matches!(tokens.get(index + 1).map(|token| &token.token), Some(Token::Ident(method)) if method == "status")
            && matches!(
                tokens.get(index + 2).map(|token| &token.token),
                Some(Token::LParen)
            );
        if !is_status_call {
            continue;
        }
        let Some(status_token) = tokens.get(index + 3) else {
            continue;
        };
        let Token::IntLit(status) = status_token.token else {
            continue;
        };
        let help = http_status_member(status).map_or_else(
            || "use a named `HttpStatus` member instead of a raw number".to_string(),
            |member| format!("use `reply.status(HttpStatus.{member})` instead"),
        );
        diagnostics.push(RuleDiagnostic {
            range: status_token.range.clone(),
            message: format!("avoid raw HTTP status code `{status}`"),
            help: Some(help),
            note: None,
            fix: http_status_member(status).map(|member| LintFix {
                message: format!("Replace `{status}` with `HttpStatus.{member}`"),
                range: status_token.range.clone(),
                replacement: format!("HttpStatus.{member}"),
            }),
        });
    }
    diagnostics
}

fn http_status_member(status: i128) -> Option<&'static str> {
    match status {
        200 => Some("Ok"),
        201 => Some("Created"),
        202 => Some("Accepted"),
        204 => Some("NoContent"),
        400 => Some("BadRequest"),
        401 => Some("Unauthorized"),
        403 => Some("Forbidden"),
        404 => Some("NotFound"),
        409 => Some("Conflict"),
        422 => Some("UnprocessableEntity"),
        500 => Some("InternalServerError"),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ApiLayer {
    Routes,
    Controllers,
    Services,
    Repositories,
}

impl ApiLayer {
    fn label(self) -> &'static str {
        match self {
            ApiLayer::Routes => "routes",
            ApiLayer::Controllers => "controllers",
            ApiLayer::Services => "services",
            ApiLayer::Repositories => "repositories",
        }
    }
}

fn current_api_layer(ctx: &LintContext) -> Option<ApiLayer> {
    ctx.file.and_then(|file| {
        file.components()
            .find_map(|component| layer_from_segment(&component.as_os_str().to_string_lossy()))
    })
}

fn layer_from_segment(segment: &str) -> Option<ApiLayer> {
    match segment {
        "routes" => Some(ApiLayer::Routes),
        "controllers" => Some(ApiLayer::Controllers),
        "services" => Some(ApiLayer::Services),
        "repositories" => Some(ApiLayer::Repositories),
        _ => None,
    }
}

fn import_api_layer(source: &str) -> Option<ApiLayer> {
    source
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != "." && *segment != "..")
        .find_map(layer_from_segment)
}

fn api_layer_imports<'a>(
    ctx: &'a LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<(&'a tyrus_core::analysis::Symbol, ApiLayer)> {
    let mut imports = Vec::new();
    for (index, symbol) in ctx.analysis.symbols.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if symbol.kind != SymbolKind::Import {
            continue;
        }
        let Some(source) = symbol.import_source.as_deref() else {
            continue;
        };
        if let Some(layer) = import_api_layer(source) {
            imports.push((symbol, layer));
        }
    }
    imports
}

fn controller_no_try_catch(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    if current_api_layer(ctx) != Some(ApiLayer::Controllers) {
        return Vec::new();
    }
    let mut diagnostics = Vec::new();
    for (index, token) in ctx.tokens.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if !matches!(token.token, Token::Try | Token::Catch) {
            continue;
        }
        diagnostics.push(RuleDiagnostic {
            range: token.range.clone(),
            message: "controller module should not catch expected errors".to_string(),
            help: Some(
                "let the service return domain results or throw domain errors, then map them at the controller boundary"
                    .to_string(),
            ),
            note: None,
            fix: None,
        });
    }
    diagnostics
}

fn no_layer_skip(ctx: &LintContext, cancellation: Option<&AtomicBool>) -> Vec<RuleDiagnostic> {
    let Some(current) = current_api_layer(ctx) else {
        return Vec::new();
    };
    let imports = api_layer_imports(ctx, cancellation);
    let mut diagnostics = Vec::new();
    for (index, (symbol, target)) in imports.into_iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        let skips_layer = matches!(
            (current, target),
            (
                ApiLayer::Routes,
                ApiLayer::Services | ApiLayer::Repositories
            ) | (ApiLayer::Controllers, ApiLayer::Repositories)
        );
        if skips_layer {
            diagnostics.push(RuleDiagnostic {
                range: symbol.declaration.clone(),
                message: format!(
                    "`{}` imports `{}` and skips the expected API layer",
                    current.label(),
                    target.label()
                ),
                help: Some(
                    "depend on the next layer down instead of jumping across API layers"
                        .to_string(),
                ),
                note: None,
                fix: None,
            });
        }
    }
    diagnostics
}

fn no_upward_imports_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let Some(current) = current_api_layer(ctx) else {
        return Vec::new();
    };
    let imports = api_layer_imports(ctx, cancellation);
    let mut diagnostics = Vec::new();
    for (index, (symbol, target)) in imports.into_iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if target >= current {
            continue;
        }
        diagnostics.push(RuleDiagnostic {
            range: symbol.declaration.clone(),
            message: format!(
                "`{}` module must not import upward from `{}`",
                current.label(),
                target.label()
            ),
            help: Some(
                "move shared code to schemas/models or depend only on lower API layers".to_string(),
            ),
            note: None,
            fix: None,
        });
    }
    diagnostics
}

fn used_declarations_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Option<Vec<Range<usize>>> {
    let mut declarations = Vec::with_capacity(ctx.analysis.references.len());
    for (index, reference) in ctx.analysis.references.iter().enumerate() {
        if index % 256 == 0 && cancellation.is_some_and(|signal| signal.load(Ordering::Acquire)) {
            return None;
        }
        if let Some(definition) = &reference.definition {
            declarations.push(definition.clone());
        }
    }
    declarations.sort_unstable_by_key(|range| (range.start, range.end));
    declarations.dedup();
    (!cancellation.is_some_and(|signal| signal.load(Ordering::Acquire))).then_some(declarations)
}

fn reference_offsets_by_name_with_cancellation(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Option<tyrus_core::collections::HashMap<String, Vec<usize>>> {
    let mut offsets = tyrus_core::collections::HashMap::<String, Vec<usize>>::default();
    for (index, reference) in ctx.analysis.references.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return None;
        }
        offsets
            .entry(reference.name.clone())
            .or_default()
            .push(reference.range.start);
    }
    for references in offsets.values_mut() {
        references.sort_unstable();
    }
    (!cancellation_requested(cancellation)).then_some(offsets)
}

fn reference_exists_in_range(
    offsets: &tyrus_core::collections::HashMap<String, Vec<usize>>,
    name: &str,
    range: &Range<usize>,
) -> bool {
    let Some(references) = offsets.get(name) else {
        return false;
    };
    let index = references.partition_point(|offset| *offset < range.start);
    references
        .get(index)
        .is_some_and(|offset| *offset < range.end)
}

fn is_used(used: &[Range<usize>], declaration: &Range<usize>) -> bool {
    used.binary_search_by_key(&(declaration.start, declaration.end), |range| {
        (range.start, range.end)
    })
    .is_ok()
}

fn onfail_used_initializer_bindings(
    ctx: &LintContext,
    used: &[Range<usize>],
    cancellation: Option<&AtomicBool>,
) -> HashSet<Range<usize>> {
    fn visit(
        ctx: &LintContext,
        used: &[Range<usize>],
        statements: &[tyrus_core::ast::Stmt],
        bindings: &mut HashSet<Range<usize>>,
        cancellation: Option<&AtomicBool>,
    ) {
        for statement in statements {
            if cancellation.is_some_and(|signal| signal.load(Ordering::Acquire)) {
                return;
            }
            if let StmtKind::VarDecl {
                name,
                onfail: Some(onfail),
                ..
            } = statement.kind()
            {
                let statement_range = statement.span.to_range();
                let handler_range = onfail.span.to_range();
                let mut handler_is_used = false;
                for handler_binding in &ctx.analysis.symbols {
                    if cancellation.is_some_and(|signal| signal.load(Ordering::Acquire)) {
                        return;
                    }
                    if handler_binding.name == *name
                        && handler_binding.declaration == handler_range
                        && is_used(used, &handler_binding.declaration)
                    {
                        handler_is_used = true;
                        break;
                    }
                }
                if handler_is_used {
                    for symbol in &ctx.analysis.symbols {
                        if cancellation.is_some_and(|signal| signal.load(Ordering::Acquire)) {
                            return;
                        }
                        if symbol.name == *name
                            && statement_range.start <= symbol.declaration.start
                            && symbol.declaration.end <= statement_range.end
                        {
                            bindings.insert(symbol.declaration.clone());
                        }
                    }
                }
            }

            match statement.kind() {
                StmtKind::Defer { body, .. }
                | StmtKind::While { body, .. }
                | StmtKind::DoWhile { body, .. }
                | StmtKind::ForOf { body, .. }
                | StmtKind::ForIn { body, .. } => visit(ctx, used, body, bindings, cancellation),
                StmtKind::For {
                    init, update, body, ..
                } => {
                    visit(
                        ctx,
                        used,
                        std::slice::from_ref(init),
                        bindings,
                        cancellation,
                    );
                    visit(
                        ctx,
                        used,
                        std::slice::from_ref(update),
                        bindings,
                        cancellation,
                    );
                    visit(ctx, used, body, bindings, cancellation);
                }
                StmtKind::Switch { cases, default, .. } => {
                    for case in cases {
                        visit(ctx, used, &case.body, bindings, cancellation);
                    }
                    if let Some(body) = default {
                        visit(ctx, used, body, bindings, cancellation);
                    }
                }
                StmtKind::Match { arms, default, .. } => {
                    for arm in arms {
                        visit(ctx, used, &arm.body, bindings, cancellation);
                    }
                    if let Some(body) = default {
                        visit(ctx, used, body, bindings, cancellation);
                    }
                }
                StmtKind::TryCatch {
                    try_body,
                    catch_body,
                    finally_body,
                    ..
                } => {
                    visit(ctx, used, try_body, bindings, cancellation);
                    visit(ctx, used, catch_body, bindings, cancellation);
                    if let Some(body) = finally_body {
                        visit(ctx, used, body, bindings, cancellation);
                    }
                }
                StmtKind::Expr(Expr {
                    kind:
                        ExprKind::If {
                            then_block,
                            else_block,
                            ..
                        },
                    ..
                }) => {
                    visit(ctx, used, then_block, bindings, cancellation);
                    if let Some(body) = else_block {
                        visit(ctx, used, body, bindings, cancellation);
                    }
                }
                _ => {}
            }

            for_each_value_expr(statement, &mut |expr| {
                if cancellation.is_some_and(|signal| signal.load(Ordering::Acquire)) {
                    return;
                }
                if let ExprKind::ArrowFn { body, .. } = expr.kind() {
                    visit(ctx, used, body, bindings, cancellation);
                }
            });
        }
    }

    let mut bindings = HashSet::default();
    for item in &ctx.program.items {
        if cancellation.is_some_and(|signal| signal.load(Ordering::Acquire)) {
            return bindings;
        }
        match item {
            Item::FnDecl { body, .. } => visit(ctx, used, body, &mut bindings, cancellation),
            Item::Stmt(statement) => visit(
                ctx,
                used,
                std::slice::from_ref(statement),
                &mut bindings,
                cancellation,
            ),
            _ => {}
        }
    }
    bindings
}

fn deferred_var_names_cancellable(
    program: &Program,
    cancellation: Option<&AtomicBool>,
) -> Option<HashSet<String>> {
    let mut names = HashSet::default();
    for (index, item) in program.items.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return None;
        }
        match item {
            Item::FnDecl { body, .. } => {
                if !collect_deferred(body, &mut names, cancellation) {
                    return None;
                }
            }
            Item::Stmt(statement)
                if !collect_deferred_stmt(statement, &mut names, cancellation) =>
            {
                return None;
            }
            _ => {}
        }
    }
    (!cancellation_requested(cancellation)).then_some(names)
}

fn collect_deferred(
    body: &[Stmt],
    names: &mut HashSet<String>,
    cancellation: Option<&AtomicBool>,
) -> bool {
    for (index, statement) in body.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return false;
        }
        if !collect_deferred_stmt(statement, names, cancellation) {
            return false;
        }
    }
    true
}

fn collect_deferred_stmt(
    statement: &Stmt,
    names: &mut HashSet<String>,
    cancellation: Option<&AtomicBool>,
) -> bool {
    if cancellation_requested(cancellation) {
        return false;
    }
    if let Stmt {
        kind: StmtKind::VarDecl {
            name, value: None, ..
        },
        ..
    } = statement
    {
        names.insert(name.clone());
    }
    if let Stmt {
        kind:
            StmtKind::Expr(Expr {
                kind:
                    ExprKind::If {
                        then_block,
                        else_block,
                        ..
                    },
                ..
            }),
        ..
    } = statement
    {
        if !collect_deferred(then_block, names, cancellation) {
            return false;
        }
        if let Some(else_block) = else_block
            && !collect_deferred(else_block, names, cancellation)
        {
            return false;
        }
    }
    match statement.kind() {
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::ForIn { body, .. } => {
            if !collect_deferred(body, names, cancellation) {
                return false;
            }
        }
        StmtKind::For {
            init, update, body, ..
        } => {
            if !collect_deferred_stmt(init, names, cancellation)
                || !collect_deferred_stmt(update, names, cancellation)
                || !collect_deferred(body, names, cancellation)
            {
                return false;
            }
        }
        StmtKind::Switch { cases, default, .. } => {
            for (index, case) in cases.iter().enumerate() {
                if index % 256 == 0 && cancellation_requested(cancellation) {
                    return false;
                }
                if !collect_deferred(&case.body, names, cancellation) {
                    return false;
                }
            }
            if let Some(default) = default
                && !collect_deferred(default, names, cancellation)
            {
                return false;
            }
        }
        StmtKind::Match { arms, default, .. } => {
            for (index, arm) in arms.iter().enumerate() {
                if index % 256 == 0 && cancellation_requested(cancellation) {
                    return false;
                }
                if !collect_deferred(&arm.body, names, cancellation) {
                    return false;
                }
            }
            if let Some(default) = default
                && !collect_deferred(default, names, cancellation)
            {
                return false;
            }
        }
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            if !collect_deferred(try_body, names, cancellation)
                || !collect_deferred(catch_body, names, cancellation)
            {
                return false;
            }
            if let Some(finally_body) = finally_body
                && !collect_deferred(finally_body, names, cancellation)
            {
                return false;
            }
        }
        _ => {}
    }
    let mut complete = true;
    for_each_value_expr(statement, &mut |expr| {
        if cancellation_requested(cancellation) {
            complete = false;
            return;
        }
        if let ExprKind::ArrowFn { body, .. } = expr.kind() {
            complete &= collect_deferred(body, names, cancellation);
        }
    });
    complete && !cancellation_requested(cancellation)
}

fn no_null_literal(ctx: &LintContext) -> Vec<RuleDiagnostic> {
    no_null_literal_cancellable(ctx, None)
}

fn no_null_literal_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let mut diagnostics = Vec::new();
    for (index, token) in ctx.tokens.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if matches!(token.token, Token::Null) {
            diagnostics.push(RuleDiagnostic {
                range: token.range.clone(),
                message: "`null` is not a value in tyrus".to_string(),
                help: Some(
                    "use `T?` for nullable types and omit optional values instead".to_string(),
                ),
                note: Some(
                    "absence is represented by optional fields, optional parameters, and external boundaries"
                        .to_string(),
                ),
                fix: None,
            });
        }
    }
    diagnostics
}

fn for_each_value_expr(statement: &Stmt, visit: &mut dyn FnMut(&Expr)) {
    match statement.kind() {
        StmtKind::VarDecl { value: Some(v), .. }
        | StmtKind::Assign { value: v, .. }
        | StmtKind::Return(Some(v))
        | StmtKind::Throw(v)
        | StmtKind::Expr(v) => visit(v),
        _ => {}
    }
}

fn no_uninitialized_variable_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let mut diagnostics = Vec::new();
    for (index, diagnostic) in ctx.analysis.diagnostics.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if diagnostic.code == tyrus_core::definite_assignment::USE_BEFORE_ASSIGN
            || (diagnostic.code == tyrus_core::definite_assignment::NEVER_ASSIGNED
                && diagnostic
                    .message
                    .contains("declared but never assigned a value"))
        {
            diagnostics.push(RuleDiagnostic {
                range: diagnostic.range.clone(),
                message: diagnostic.message.clone(),
                help: diagnostic.help.first().cloned(),
                note: None,
                fix: None,
            });
        }
    }
    if cancellation_requested(cancellation) {
        Vec::new()
    } else {
        diagnostics
    }
}

fn no_unused_vars(ctx: &LintContext) -> Vec<RuleDiagnostic> {
    no_unused_vars_cancellable(ctx, None)
}

fn no_unused_vars_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let Some(deferred) = deferred_var_names_cancellable(ctx.program, cancellation) else {
        return Vec::new();
    };
    let Some(used) = used_declarations_cancellable(ctx, cancellation) else {
        return Vec::new();
    };
    let onfail_used = onfail_used_initializer_bindings(ctx, &used, cancellation);
    let mut diagnostics = Vec::new();
    for symbol in &ctx.analysis.symbols {
        if cancellation.is_some_and(|signal| signal.load(Ordering::Acquire)) {
            return diagnostics;
        }
        if symbol.kind != SymbolKind::Variable
            || symbol.exported
            || symbol.name.starts_with('_')
            || deferred.contains(&symbol.name)
            || is_used(&used, &symbol.declaration)
            || onfail_used.contains(&symbol.declaration)
        {
            continue;
        }
        diagnostics.push(RuleDiagnostic {
            range: symbol.declaration.clone(),
            message: format!("variable `{}` is never used", symbol.name),
            help: Some(format!(
                "remove the declaration or prefix it with `_{}`",
                symbol.name
            )),
            note: None,
            fix: unused_variable_rename_fix(ctx, symbol),
        });
    }
    diagnostics
}

fn unused_variable_rename_fix(
    ctx: &LintContext,
    symbol: &tyrus_core::analysis::Symbol,
) -> Option<LintFix> {
    if ctx.source.get(symbol.declaration.clone())? != symbol.name {
        return None;
    }
    let token_index = ctx
        .tokens
        .binary_search_by_key(&symbol.declaration.start, |token| token.range.start)
        .ok()?;
    if ctx.tokens[token_index].range != symbol.declaration {
        return None;
    }
    if token_index == 0 || !matches!(ctx.tokens[token_index - 1].token, Token::Const | Token::Let) {
        return None;
    }
    let replacement = format!("_{}", symbol.name);
    if ctx
        .analysis
        .symbols
        .iter()
        .any(|candidate| candidate.name == replacement && candidate.scope == symbol.scope)
    {
        return None;
    }
    Some(LintFix {
        message: format!("Rename `{}` to `{replacement}`", symbol.name),
        range: symbol.declaration.clone(),
        replacement,
    })
}

fn no_unused_imports(ctx: &LintContext) -> Vec<RuleDiagnostic> {
    no_unused_imports_cancellable(ctx, None)
}

fn no_unused_imports_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let Some(used) = used_declarations_cancellable(ctx, cancellation) else {
        return Vec::new();
    };
    let used_as_type = ctx
        .analysis
        .symbols
        .iter()
        .filter(|symbol| symbol.kind != SymbolKind::Import)
        .flat_map(|symbol| {
            symbol
                .ty
                .split(|character: char| !character.is_alphanumeric() && character != '_')
        })
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect::<HashSet<_>>();
    let mut diagnostics = Vec::new();
    for symbol in &ctx.analysis.symbols {
        if cancellation.is_some_and(|signal| signal.load(Ordering::Acquire)) {
            return diagnostics;
        }
        if symbol.kind != SymbolKind::Import
            || symbol.name.starts_with('_')
            || is_used(&used, &symbol.declaration)
            || used_as_type.contains(&symbol.name)
        {
            continue;
        }
        diagnostics.push(RuleDiagnostic {
            range: symbol.declaration.clone(),
            message: format!("import `{}` is never used", symbol.name),
            help: Some("remove the unused import".to_string()),
            note: None,
            fix: unused_import_fix(ctx, &symbol.name),
        });
    }
    diagnostics
}

fn unused_import_fix(ctx: &LintContext, name: &str) -> Option<LintFix> {
    for import in &ctx.program.imports {
        let Some(index) = import
            .specifiers
            .iter()
            .position(|specifier| specifier.local == name)
        else {
            continue;
        };
        let specifier = &import.specifiers[index];
        if import.specifiers.len() > 1
            && import
                .specifiers
                .iter()
                .any(|specifier| specifier.kind != ImportKind::Named)
        {
            return None;
        }
        let range = if import.specifiers.len() == 1 {
            import.span.start..import.span.end
        } else if let Some(next) = import.specifiers.get(index + 1) {
            let separator = ctx.source.get(specifier.span.end..next.span.start)?;
            if separator.contains("//") || separator.contains("/*") {
                return None;
            }
            let comma = separator.find(',')?;
            let mut end = specifier.span.end + comma + 1;
            while ctx
                .source
                .as_bytes()
                .get(end)
                .is_some_and(u8::is_ascii_whitespace)
            {
                end += 1;
            }
            specifier.span.start..end
        } else {
            let previous = &import.specifiers[index - 1];
            let separator = ctx.source.get(previous.span.end..specifier.span.start)?;
            if separator.contains("//") || separator.contains("/*") {
                return None;
            }
            let comma = separator.rfind(',')?;
            previous.span.end + comma..specifier.span.end
        };
        return Some(LintFix {
            message: format!("remove unused import `{name}`"),
            range,
            replacement: String::new(),
        });
    }
    None
}

fn no_unused_functions(ctx: &LintContext) -> Vec<RuleDiagnostic> {
    no_unused_functions_cancellable(ctx, None)
}

fn no_unused_functions_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let Some(used) = used_declarations_cancellable(ctx, cancellation) else {
        return Vec::new();
    };
    let mut diagnostics = Vec::new();
    for symbol in &ctx.analysis.symbols {
        if cancellation.is_some_and(|signal| signal.load(Ordering::Acquire)) {
            return diagnostics;
        }
        if symbol.kind != SymbolKind::Function
            || symbol.exported
            || symbol.name == "main"
            || symbol.name.starts_with('_')
            || is_used(&used, &symbol.declaration)
        {
            continue;
        }
        diagnostics.push(RuleDiagnostic {
            range: symbol.declaration.clone(),
            message: format!("function `{}` is never used", symbol.name),
            help: Some("remove it, or export it if it is part of the public API".to_string()),
            note: None,
            fix: None,
        });
    }
    diagnostics
}

fn repository_no_http_error(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    if current_api_layer(ctx) != Some(ApiLayer::Repositories) {
        return Vec::new();
    }
    let mut diagnostics = Vec::new();
    for (index, symbol) in ctx.analysis.symbols.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if symbol.kind == SymbolKind::Import
            && symbol.import_source.as_deref() == Some("tyrus:http")
            && symbol.imported_name.as_deref().unwrap_or(&symbol.name) == "HttpReply"
        {
            diagnostics.push(RuleDiagnostic {
                range: symbol.declaration.clone(),
                message: "repository module should not depend on `HttpReply`".to_string(),
                help: Some(
                    "throw a database/domain error or return null and let the service decide"
                        .to_string(),
                ),
                note: None,
                fix: None,
            });
        }
    }
    for (index, reference) in ctx.analysis.references.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if reference.name == "HttpReply" {
            diagnostics.push(RuleDiagnostic {
                range: reference.range.clone(),
                message: "repository module should not construct or use `HttpReply`".to_string(),
                help: Some(
                    "throw a database/domain error or return null and let the service decide"
                        .to_string(),
                ),
                note: None,
                fix: None,
            });
        }
    }
    diagnostics
}

fn route_no_business_logic(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    if current_api_layer(ctx) != Some(ApiLayer::Routes) {
        return Vec::new();
    }
    let mut diagnostics = Vec::new();
    for (index, (symbol, target)) in api_layer_imports(ctx, cancellation).into_iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if matches!(target, ApiLayer::Services | ApiLayer::Repositories) {
            diagnostics.push(RuleDiagnostic {
                range: symbol.declaration.clone(),
                message: format!(
                    "route module should not import `{}` directly",
                    target.label()
                ),
                help: Some(
                    "keep route files declarative and call controllers from route handlers"
                        .to_string(),
                ),
                note: None,
                fix: None,
            });
        }
    }
    diagnostics.extend(inline_route_handler_diagnostics(ctx, cancellation));
    diagnostics
}

fn inline_route_handler_diagnostics(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let mut diagnostics = Vec::new();
    for (index, token) in ctx.tokens.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if !matches!(&token.token, Token::Ident(name) if name == "handler") {
            continue;
        }
        let Some(next) = ctx.tokens.get(index + 1) else {
            continue;
        };
        if !matches!(next.token, Token::Colon) {
            continue;
        }
        let mut depth = 0i32;
        let mut cursor = index + 2;
        while let Some(candidate) = ctx.tokens.get(cursor) {
            if cursor % 256 == 0 && cancellation_requested(cancellation) {
                return Vec::new();
            }
            match candidate.token {
                Token::LParen | Token::LBracket | Token::LBrace => depth += 1,
                Token::RParen | Token::RBracket | Token::RBrace => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                Token::Comma if depth == 0 => break,
                Token::Arrow => {
                    diagnostics.push(RuleDiagnostic {
                        range: token.range.clone(),
                        message: "route module should not define inline handler business logic".to_string(),
                        help: Some("move the handler body to a controller and reference the controller from the route".to_string()),
                        note: None,
                        fix: None,
                    });
                    break;
                }
                _ => {}
            }
            cursor += 1;
        }
    }
    diagnostics
}

fn service_no_reply(ctx: &LintContext, cancellation: Option<&AtomicBool>) -> Vec<RuleDiagnostic> {
    if !is_service_file(ctx) {
        return Vec::new();
    }
    let mut diagnostics = Vec::new();
    for (index, symbol) in ctx.analysis.symbols.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if symbol.kind == SymbolKind::Import
            && symbol.import_source.as_deref() == Some("tyrus:http")
            && matches!(
                symbol.imported_name.as_deref().unwrap_or(&symbol.name),
                "HttpReply" | "HttpStatus"
            )
        {
            diagnostics.push(RuleDiagnostic {
                range: symbol.declaration.clone(),
                message: format!(
                    "service module must not depend on HTTP response API `{}`",
                    symbol.name
                ),
                help: Some(
                    "move response handling to a controller and keep services transport-agnostic"
                        .to_string(),
                ),
                note: None,
                fix: None,
            });
        }
        if matches!(symbol.kind, SymbolKind::Parameter | SymbolKind::Variable)
            && symbol.name == "reply"
        {
            diagnostics.push(RuleDiagnostic {
                range: symbol.declaration.clone(),
                message: "service module must not receive or create `reply`".to_string(),
                help: Some("return domain data from the service and let the controller send the HTTP response".to_string()),
                note: None,
                fix: None,
            });
        }
    }
    for (index, reference) in ctx.analysis.references.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if reference.name == "reply" {
            diagnostics.push(RuleDiagnostic {
                range: reference.range.clone(),
                message: "service module must not use `reply`".to_string(),
                help: Some(
                    "return domain data from the service and let the controller send the HTTP response"
                        .to_string(),
                ),
                note: None,
                fix: None,
            });
        }
    }
    diagnostics
}

/// `export protect` scopes a name to the declaring directory and its
/// descendants. In the source root that is every file the compiler would accept
/// anyway, so the modifier is dead weight rather than a restriction.
fn export_protect_no_effect(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let Some(file) = ctx.file else {
        return Vec::new();
    };
    let Some(directory) = file.parent() else {
        return Vec::new();
    };
    let Some(root) = tyrus_core::project::find_project_root(file) else {
        return Vec::new();
    };
    if directory != root && directory != root.join("src") {
        return Vec::new();
    }
    let mut diagnostics = Vec::new();
    for (index, export) in ctx.program.exports.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        let tyrus_core::ast::ExportDecl::Named {
            exported,
            visibility: tyrus_core::ast::Visibility::Protect,
            ..
        } = export
        else {
            continue;
        };
        let Some(range) = tyrus_core::module_loader::protect_modifier_range(ctx.source, exported)
        else {
            continue;
        };
        diagnostics.push(RuleDiagnostic {
            range,
            message: format!(
                "`export protect` on `{exported}` narrows nothing: this file sits in the source root, so its folder already covers the whole project"
            ),
            help: Some(format!(
                "drop `protect` from `{exported}`, or move the declaration into a subfolder that should own it"
            )),
            note: None,
            fix: None,
        });
    }
    diagnostics
}

fn is_service_file(ctx: &LintContext) -> bool {
    ctx.file.is_some_and(|file| {
        file.components()
            .any(|component| component.as_os_str() == "services")
    })
}

fn no_shadow(ctx: &LintContext) -> Vec<RuleDiagnostic> {
    no_shadow_cancellable(ctx, None)
}

fn no_shadow_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let bindings: Vec<_> = ctx
        .analysis
        .symbols
        .iter()
        .filter(|symbol| matches!(symbol.kind, SymbolKind::Variable | SymbolKind::Parameter))
        .collect();
    let mut diagnostics = Vec::new();
    let mut comparisons = 0usize;
    for inner in &bindings {
        if cancellation_requested(cancellation) {
            return Vec::new();
        }
        let is_onfail_binding = ctx
            .source
            .get(inner.declaration.clone())
            .is_some_and(|text| text.trim_start().starts_with("onfail"));
        if is_onfail_binding {
            continue;
        }
        let mut outer = None;
        for candidate in &bindings {
            comparisons += 1;
            if comparisons.is_multiple_of(256) && cancellation_requested(cancellation) {
                return Vec::new();
            }
            if candidate.name == inner.name
                && candidate.declaration != inner.declaration
                && candidate.scope.start <= inner.scope.start
                && inner.scope.end <= candidate.scope.end
                && (candidate.scope.start < inner.scope.start
                    || inner.scope.end < candidate.scope.end)
                && candidate.declaration.start < inner.declaration.start
            {
                outer = Some(candidate);
                break;
            }
        }
        if let Some(outer) = outer {
            let (line, _) = crate::line_column(ctx.source, outer.declaration.start);
            diagnostics.push(RuleDiagnostic {
                range: inner.declaration.clone(),
                message: format!("`{}` shadows a binding from an outer scope", inner.name),
                help: Some("rename one of the bindings to avoid confusion".to_string()),
                note: Some(format!("outer `{}` is declared on line {line}", inner.name)),
                fix: None,
            });
        }
    }
    diagnostics
}

fn prefer_const(ctx: &LintContext) -> Vec<RuleDiagnostic> {
    prefer_const_cancellable(ctx, None)
}

fn prefer_const_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    if cancellation_requested(cancellation) {
        return Vec::new();
    }
    let mutated = collect_mutated_names(ctx.program);
    if cancellation_requested(cancellation) {
        return Vec::new();
    }
    let Some(deferred) = deferred_var_names_cancellable(ctx.program, cancellation) else {
        return Vec::new();
    };
    if cancellation_requested(cancellation) {
        return Vec::new();
    }
    let mut diagnostics = Vec::new();
    for (index, symbol) in ctx.analysis.symbols.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if symbol.kind != SymbolKind::Variable || symbol.exported {
            continue;
        }
        if deferred.contains(&symbol.name) {
            continue;
        }
        let Some(let_range) = preceding_let_keyword(ctx.tokens, &symbol.declaration) else {
            continue;
        };
        if mutated.contains(&symbol.name) {
            continue;
        }
        diagnostics.push(RuleDiagnostic {
            range: symbol.declaration.clone(),
            message: format!("variable `{}` is never reassigned", symbol.name),
            help: Some("use `const` instead of `let`".to_string()),
            note: None,
            fix: Some(LintFix {
                message: "replace `let` with `const`".to_string(),
                range: let_range,
                replacement: "const".to_string(),
            }),
        });
    }
    if cancellation_requested(cancellation) {
        Vec::new()
    } else {
        diagnostics
    }
}

fn preceding_let_keyword(
    tokens: &[SpannedToken],
    declaration: &Range<usize>,
) -> Option<Range<usize>> {
    let position = tokens
        .iter()
        .position(|token| token.range == *declaration)?;
    let previous = tokens.get(position.checked_sub(1)?)?;
    matches!(previous.token, Token::Let).then(|| previous.range.clone())
}

fn collect_mutated_names(program: &Program) -> HashSet<String> {
    let mut names = HashSet::default();
    for item in &program.items {
        match item {
            Item::Stmt(statement) => walk_stmt_assignments(statement, &mut names),
            Item::FnDecl { body, .. } => {
                for statement in body {
                    walk_stmt_assignments(statement, &mut names);
                }
            }
            _ => {}
        }
    }
    names
}

fn walk_stmt_assignments(statement: &Stmt, names: &mut HashSet<String>) {
    match statement.kind() {
        StmtKind::Assign { name, value, .. } => {
            names.insert(name.clone());
            walk_expr_assignments(value, names);
        }
        StmtKind::IndexAssign { name, index, value } => {
            names.insert(name.clone());
            walk_expr_assignments(index, names);
            walk_expr_assignments(value, names);
        }
        StmtKind::FieldAssign { object, value, .. } => {
            if let Some(name) = mutable_place_root(object) {
                names.insert(name.to_string());
            }
            walk_expr_assignments(object, names);
            walk_expr_assignments(value, names);
        }
        StmtKind::VarDecl {
            value: Some(value), ..
        } => walk_expr_assignments(value, names),
        StmtKind::ArrayDestructure { value, .. } | StmtKind::ObjectDestructure { value, .. } => {
            walk_expr_assignments(value, names);
        }
        StmtKind::Return(Some(value)) | StmtKind::Throw(value) | StmtKind::Expr(value) => {
            walk_expr_assignments(value, names);
        }
        StmtKind::While { cond, body } | StmtKind::DoWhile { cond, body } => {
            walk_expr_assignments(cond, names);
            walk_block_assignments(body, names);
        }
        StmtKind::For {
            init,
            cond,
            update,
            body,
        } => {
            walk_stmt_assignments(init, names);
            walk_expr_assignments(cond, names);
            walk_stmt_assignments(update, names);
            walk_block_assignments(body, names);
        }
        StmtKind::ForOf { iterable, body, .. } => {
            walk_expr_assignments(iterable, names);
            walk_block_assignments(body, names);
        }
        StmtKind::ForIn { object, body, .. } => {
            walk_expr_assignments(object, names);
            walk_block_assignments(body, names);
        }
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            walk_block_assignments(try_body, names);
            walk_block_assignments(catch_body, names);
            if let Some(finally_body) = finally_body {
                walk_block_assignments(finally_body, names);
            }
        }
        StmtKind::Switch {
            expr,
            cases,
            default,
            ..
        } => {
            walk_expr_assignments(expr, names);
            for case in cases {
                walk_block_assignments(&case.body, names);
            }
            if let Some(default) = default {
                walk_block_assignments(default, names);
            }
        }
        StmtKind::Match {
            expr,
            arms,
            default,
        } => {
            walk_expr_assignments(expr, names);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    walk_expr_assignments(guard, names);
                }
                walk_block_assignments(&arm.body, names);
            }
            if let Some(default) = default {
                walk_block_assignments(default, names);
            }
        }
        _ => {}
    }
}

fn mutable_place_root(expr: &Expr) -> Option<&str> {
    match expr.kind() {
        ExprKind::Ident(name) => Some(name),
        ExprKind::FieldAccess { object, .. } | ExprKind::IndexAccess { object, .. } => {
            mutable_place_root(object)
        }
        _ => None,
    }
}

fn walk_block_assignments(body: &[Stmt], names: &mut HashSet<String>) {
    for statement in body {
        walk_stmt_assignments(statement, names);
    }
}

fn walk_expr_assignments(expr: &Expr, names: &mut HashSet<String>) {
    match expr.kind() {
        ExprKind::PrefixIncr(inner)
        | ExprKind::PrefixDecr(inner)
        | ExprKind::PostfixIncr(inner)
        | ExprKind::PostfixDecr(inner) => {
            if let ExprKind::Ident(name) = inner.as_ref().kind() {
                names.insert(name.clone());
            }
            walk_expr_assignments(inner, names);
        }
        ExprKind::BinOp { left, right, .. } => {
            walk_expr_assignments(left, names);
            walk_expr_assignments(right, names);
        }
        ExprKind::Not(inner)
        | ExprKind::Neg(inner)
        | ExprKind::BitNot(inner)
        | ExprKind::Unwrap(inner)
        | ExprKind::Typeof(inner)
        | ExprKind::Spread(inner)
        | ExprKind::ConstAssert(inner) => walk_expr_assignments(inner, names),
        ExprKind::FieldAccess { object, .. } => walk_expr_assignments(object, names),
        ExprKind::IndexAccess { object, index } => {
            walk_expr_assignments(object, names);
            walk_expr_assignments(index, names);
        }
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            walk_expr_assignments(cond, names);
            walk_expr_assignments(then_expr, names);
            walk_expr_assignments(else_expr, names);
        }
        ExprKind::TypeAssert { value, .. } => walk_expr_assignments(value, names),
        ExprKind::ArrayLit(values) => values
            .iter()
            .for_each(|value| walk_expr_assignments(value, names)),
        ExprKind::ObjectLit(fields) => fields
            .iter()
            .for_each(|(_, value)| walk_expr_assignments(value, names)),
        ExprKind::ObjectUpdate { base, fields } => {
            walk_expr_assignments(base, names);
            fields
                .iter()
                .for_each(|(_, value)| walk_expr_assignments(value, names));
        }
        ExprKind::FnCall { args, .. } => args
            .iter()
            .for_each(|arg| walk_expr_assignments(arg, names)),
        ExprKind::MethodCall {
            object,
            method,
            args,
            ..
        }
        | ExprKind::OptionalMethodCall {
            object,
            method,
            args,
            ..
        } => {
            if let ExprKind::Ident(name) = object.as_ref().kind()
                && is_mutating_method(method)
            {
                names.insert(name.clone());
            }
            walk_expr_assignments(object, names);
            args.iter()
                .for_each(|arg| walk_expr_assignments(arg, names));
        }
        ExprKind::Borrow {
            mutable: true,
            name,
            ..
        } => {
            names.insert(name.clone());
        }
        ExprKind::ArrowFn { body, .. } | ExprKind::Loop { body } => {
            walk_block_assignments(body, names);
        }
        ExprKind::If {
            cond,
            then_block,
            else_block,
        } => {
            walk_expr_assignments(cond, names);
            walk_block_assignments(then_block, names);
            if let Some(else_block) = else_block {
                walk_block_assignments(else_block, names);
            }
        }
        ExprKind::In { key, object } => {
            walk_expr_assignments(key, names);
            walk_expr_assignments(object, names);
        }
        ExprKind::TemplateLit(parts) => {
            for part in parts {
                if let tyrus_core::ast::TemplatePart::Expr(expr, _) = part {
                    walk_expr_assignments(expr, names);
                }
            }
        }
        _ => {}
    }
}

fn no_duplicate_imports(ctx: &LintContext) -> Vec<RuleDiagnostic> {
    no_duplicate_imports_cancellable(ctx, None)
}

fn no_duplicate_imports_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let mut seen: HashSet<String> = HashSet::default();
    let mut diagnostics = Vec::new();
    for (index, spanned) in ctx.tokens.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if !matches!(spanned.token, Token::From) {
            continue;
        }
        let Some(source) = ctx.tokens.get(index + 1) else {
            continue;
        };
        let Token::StringLit(module) = &source.token else {
            continue;
        };
        if !seen.insert(module.clone()) {
            diagnostics.push(RuleDiagnostic {
                range: source.range.clone(),
                message: format!("module `{module}` is imported more than once"),
                help: Some(
                    "merge the imports from this module into a single statement".to_string(),
                ),
                note: None,
                fix: None,
            });
        }
    }
    diagnostics
}

fn no_force_unwrap(ctx: &LintContext) -> Vec<RuleDiagnostic> {
    no_force_unwrap_cancellable(ctx, None)
}

fn no_force_unwrap_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let mut diagnostics = Vec::new();
    for (index, spanned) in ctx.tokens.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if !matches!(spanned.token, Token::Bang) {
            continue;
        }
        let is_postfix = index
            .checked_sub(1)
            .and_then(|prev| ctx.tokens.get(prev))
            .is_some_and(|prev| {
                matches!(
                    prev.token,
                    Token::Ident(_) | Token::RParen | Token::RBracket
                )
            });
        if is_postfix {
            diagnostics.push(RuleDiagnostic {
                range: spanned.range.clone(),
                message: "avoid force unwrap on an optional value".to_string(),
                help: Some(
                    "check for null before using the value, or provide a fallback with `??`"
                        .to_string(),
                ),
                note: None,
                fix: None,
            });
        }
    }
    diagnostics
}

fn no_unreachable_code(ctx: &LintContext) -> Vec<RuleDiagnostic> {
    no_unreachable_code_cancellable(ctx, None)
}

fn no_unreachable_code_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let tokens = ctx.tokens;
    let mut diagnostics = Vec::new();
    for (index, spanned) in tokens.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if !matches!(
            spanned.token,
            Token::Return | Token::Throw | Token::Break | Token::Continue
        ) {
            continue;
        }
        let mut depth = 0i32;
        let mut cursor = index + 1;
        let mut ended_with_semicolon = false;
        while let Some(token) = tokens.get(cursor) {
            if cursor % 256 == 0 && cancellation_requested(cancellation) {
                return Vec::new();
            }
            match token.token {
                Token::LParen | Token::LBracket | Token::LBrace => depth += 1,
                Token::RParen | Token::RBracket => depth -= 1,
                Token::RBrace if depth == 0 => break,
                Token::RBrace => depth -= 1,
                Token::Semicolon if depth == 0 => {
                    ended_with_semicolon = true;
                    break;
                }
                _ => {}
            }
            cursor += 1;
        }
        if !ended_with_semicolon {
            continue;
        }
        let Some(next) = tokens.get(cursor + 1) else {
            continue;
        };
        if matches!(next.token, Token::RBrace | Token::Case | Token::Default) {
            continue;
        }
        diagnostics.push(RuleDiagnostic {
            range: next.range.clone(),
            message: "this code is unreachable".to_string(),
            help: Some("remove it, or move it before the terminating statement".to_string()),
            note: None,
            fix: None,
        });
    }
    diagnostics
}

fn no_unsafe_finally(ctx: &LintContext) -> Vec<RuleDiagnostic> {
    no_unsafe_finally_cancellable(ctx, None)
}

fn no_unsafe_finally_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let tokens = ctx.tokens;
    let mut diagnostics = Vec::new();
    for (index, spanned) in tokens.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if !matches!(spanned.token, Token::Finally) {
            continue;
        }
        let Some((open, close)) = brace_block_after_cancellable(tokens, index, cancellation) else {
            continue;
        };
        for (offset, token) in tokens[open + 1..close].iter().enumerate() {
            if offset % 256 == 0 && cancellation_requested(cancellation) {
                return Vec::new();
            }
            if matches!(
                token.token,
                Token::Return | Token::Throw | Token::Break | Token::Continue
            ) {
                diagnostics.push(RuleDiagnostic {
                    range: token.range.clone(),
                    message: "control flow inside `finally` can override the try/catch result"
                        .to_string(),
                    help: Some("move this out of the `finally` block".to_string()),
                    note: None,
                    fix: None,
                });
            }
        }
    }
    diagnostics
}

fn no_empty_catch(ctx: &LintContext) -> Vec<RuleDiagnostic> {
    no_empty_catch_cancellable(ctx, None)
}

fn no_empty_catch_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let tokens = ctx.tokens;
    let mut diagnostics = Vec::new();
    for (index, spanned) in tokens.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if !matches!(spanned.token, Token::Catch) {
            continue;
        }
        let Some((open_index, close_index)) =
            brace_block_after_cancellable(tokens, index, cancellation)
        else {
            continue;
        };
        let open = tokens[open_index].range.end;
        let close = tokens[close_index].range.start;
        if ctx
            .source
            .get(open..close)
            .is_some_and(|inner| inner.trim().is_empty())
        {
            diagnostics.push(RuleDiagnostic {
                range: spanned.range.clone(),
                message: "empty `catch` block".to_string(),
                help: Some(
                    "handle the error, or add a comment explaining why it is ignored".to_string(),
                ),
                note: None,
                fix: None,
            });
        }
    }
    diagnostics
}

fn no_empty_block(ctx: &LintContext) -> Vec<RuleDiagnostic> {
    no_empty_block_cancellable(ctx, None)
}

fn no_empty_block_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let tokens = ctx.tokens;
    let mut diagnostics = Vec::new();
    for (index, spanned) in tokens.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if !matches!(spanned.token, Token::LBrace) {
            continue;
        }
        let Some(next) = tokens.get(index + 1) else {
            continue;
        };
        if !matches!(next.token, Token::RBrace) {
            continue;
        }
        let previous = index.checked_sub(1).and_then(|prev| tokens.get(prev));
        let is_control_block = match previous.map(|token| &token.token) {
            Some(Token::RParen) => {
                let is_catch = is_catch_block_cancellable(tokens, index, cancellation);
                if cancellation_requested(cancellation) {
                    return Vec::new();
                }
                !is_catch
            }
            Some(Token::Else | Token::Do) => true,
            _ => false,
        };
        if !is_control_block {
            continue;
        }
        let inner = ctx
            .source
            .get(spanned.range.end..next.range.start)
            .unwrap_or("");
        if !inner.trim().is_empty() {
            continue;
        }
        diagnostics.push(RuleDiagnostic {
            range: spanned.range.start..next.range.end,
            message: "empty block".to_string(),
            help: Some(
                "remove it, or add a comment explaining why it is intentionally empty".to_string(),
            ),
            note: None,
            fix: None,
        });
    }
    diagnostics
}

fn is_catch_block_cancellable(
    tokens: &[SpannedToken],
    brace_index: usize,
    cancellation: Option<&AtomicBool>,
) -> bool {
    let mut cursor = match brace_index.checked_sub(1) {
        Some(cursor) => cursor,
        None => return false,
    };
    let mut depth = 0i32;
    let mut visited = 0usize;
    loop {
        visited += 1;
        if visited.is_multiple_of(256) && cancellation_requested(cancellation) {
            return false;
        }
        match tokens[cursor].token {
            Token::RParen => depth += 1,
            Token::LParen => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        cursor = match cursor.checked_sub(1) {
            Some(cursor) => cursor,
            None => return false,
        };
    }
    cursor
        .checked_sub(1)
        .is_some_and(|prev| matches!(tokens[prev].token, Token::Catch))
}

fn use_caught_error(ctx: &LintContext) -> Vec<RuleDiagnostic> {
    use_caught_error_cancellable(ctx, None)
}

fn use_caught_error_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let tokens = ctx.tokens;
    let Some(reference_offsets) = reference_offsets_by_name_with_cancellation(ctx, cancellation)
    else {
        return Vec::new();
    };
    let mut diagnostics = Vec::new();
    for (index, spanned) in tokens.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if !matches!(spanned.token, Token::Catch) {
            continue;
        }
        if !matches!(tokens.get(index + 1).map(|t| &t.token), Some(Token::LParen)) {
            continue;
        }
        let Some(binding) = tokens.get(index + 2) else {
            continue;
        };
        let Token::Ident(name) = &binding.token else {
            continue;
        };
        if name.starts_with('_') {
            continue;
        }
        let Some((open, close)) = brace_block_after_cancellable(tokens, index, cancellation) else {
            continue;
        };
        let body = tokens[open].range.end..tokens[close].range.start;
        let used = reference_exists_in_range(&reference_offsets, name, &body);
        if !used {
            let ignored_name = format!("_{name}");
            let mut has_collision = false;
            for (symbol_index, symbol) in ctx.analysis.symbols.iter().enumerate() {
                if symbol_index % 256 == 0 && cancellation_requested(cancellation) {
                    return Vec::new();
                }
                if symbol.name == ignored_name {
                    has_collision = true;
                    break;
                }
            }
            let fix = (!has_collision).then(|| LintFix {
                message: format!("Rename `{name}` to `{ignored_name}`"),
                range: binding.range.clone(),
                replacement: ignored_name,
            });
            diagnostics.push(RuleDiagnostic {
                range: binding.range.clone(),
                message: format!("caught error `{name}` is never used"),
                help: Some(format!(
                    "use the error, or rename it to `_{name}` to mark it intentionally ignored"
                )),
                note: None,
                fix,
            });
        }
    }
    diagnostics
}

fn no_duplicate_case(ctx: &LintContext) -> Vec<RuleDiagnostic> {
    no_duplicate_case_cancellable(ctx, None)
}

fn no_duplicate_case_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let tokens = ctx.tokens;
    let mut diagnostics = Vec::new();
    for (index, spanned) in tokens.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        match spanned.token {
            Token::Switch => {
                if let Some((open, close)) =
                    brace_block_after_cancellable(tokens, index, cancellation)
                {
                    collect_duplicates(
                        ctx,
                        open,
                        close,
                        Token::Case,
                        Token::Colon,
                        &mut diagnostics,
                        cancellation,
                    );
                }
            }
            Token::Match => {
                if let Some((open, close)) =
                    brace_block_after_cancellable(tokens, index, cancellation)
                {
                    collect_match_duplicates(ctx, open, close, &mut diagnostics, cancellation);
                }
            }
            _ => {}
        }
    }
    diagnostics
}

fn collect_match_duplicates(
    ctx: &LintContext,
    open: usize,
    close: usize,
    diagnostics: &mut Vec<RuleDiagnostic>,
    cancellation: Option<&AtomicBool>,
) {
    let tokens = ctx.tokens;
    let mut seen: HashSet<String> = HashSet::default();
    let mut index = open + 1;
    while index < close {
        if index.is_multiple_of(256) && cancellation_requested(cancellation) {
            return;
        }
        let start = index;
        let mut cursor = start;
        let mut depth = 1i32;
        while cursor < close {
            if cursor.is_multiple_of(256) && cancellation_requested(cancellation) {
                return;
            }
            match tokens[cursor].token {
                Token::LBrace | Token::LParen | Token::LBracket => depth += 1,
                Token::RBrace | Token::RParen | Token::RBracket => depth -= 1,
                Token::Arrow if depth == 1 => break,
                _ => {}
            }
            cursor += 1;
        }
        if cursor == close {
            break;
        }
        let span = tokens[start].range.start..tokens[cursor - 1].range.end;
        let key = normalize(ctx.source.get(span.clone()).unwrap_or(""));
        if !key.is_empty() && !seen.insert(key.clone()) {
            diagnostics.push(RuleDiagnostic {
                range: span,
                message: format!("duplicate case `{key}`"),
                help: Some("remove the duplicated case".to_string()),
                note: None,
                fix: None,
            });
        }
        index = cursor + 1;
        if matches!(
            tokens.get(index).map(|token| &token.token),
            Some(Token::LBrace)
        ) {
            let mut body_depth = 0i32;
            while index < close {
                if index.is_multiple_of(256) && cancellation_requested(cancellation) {
                    return;
                }
                match tokens[index].token {
                    Token::LBrace => body_depth += 1,
                    Token::RBrace => {
                        body_depth -= 1;
                        if body_depth == 0 {
                            index += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                index += 1;
            }
        } else {
            while index < close && !matches!(tokens[index].token, Token::Semicolon) {
                if index.is_multiple_of(256) && cancellation_requested(cancellation) {
                    return;
                }
                index += 1;
            }
            index += usize::from(index < close);
        }
    }
}

fn collect_duplicates(
    ctx: &LintContext,
    open: usize,
    close: usize,
    marker: Token,
    terminator: Token,
    diagnostics: &mut Vec<RuleDiagnostic>,
    cancellation: Option<&AtomicBool>,
) {
    let tokens = ctx.tokens;
    let mut seen: HashSet<String> = HashSet::default();
    let mut depth = 1i32;
    let mut index = open + 1;
    while index < close {
        if index.is_multiple_of(256) && cancellation_requested(cancellation) {
            return;
        }
        let token = &tokens[index].token;
        match token {
            Token::LBrace | Token::LParen | Token::LBracket => depth += 1,
            Token::RBrace | Token::RParen | Token::RBracket => depth -= 1,
            _ if *token == marker && depth == 1 => {
                let start = index + 1;
                let mut cursor = start;
                let mut label_depth = depth;
                while cursor < close {
                    if cursor.is_multiple_of(256) && cancellation_requested(cancellation) {
                        return;
                    }
                    match tokens[cursor].token {
                        Token::LBrace | Token::LParen | Token::LBracket => label_depth += 1,
                        Token::RBrace | Token::RParen | Token::RBracket => label_depth -= 1,
                        ref current if *current == terminator && label_depth == 1 => break,
                        _ => {}
                    }
                    cursor += 1;
                }
                if cursor > start && cursor <= close {
                    let span = tokens[start].range.start..tokens[cursor - 1].range.end;
                    let key = normalize(ctx.source.get(span.clone()).unwrap_or(""));
                    if !key.is_empty() && !seen.insert(key.clone()) {
                        diagnostics.push(RuleDiagnostic {
                            range: span,
                            message: format!("duplicate case `{key}`"),
                            help: Some("remove the duplicated case".to_string()),
                            note: None,
                            fix: None,
                        });
                    }
                }
                index = cursor;
                continue;
            }
            _ => {}
        }
        index += 1;
    }
}

fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn prefer_nullish_coalescing(ctx: &LintContext) -> Vec<RuleDiagnostic> {
    prefer_nullish_coalescing_cancellable(ctx, None)
}

fn prefer_nullish_coalescing_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let tokens = ctx.tokens;
    let mut diagnostics = Vec::new();
    for index in 0..tokens.len() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        let Token::Ident(checked) = &tokens[index].token else {
            continue;
        };
        let is_pattern = matches!(
            tokens.get(index + 1).map(|t| &t.token),
            Some(Token::NotEqEq)
        ) && matches!(tokens.get(index + 2).map(|t| &t.token), Some(Token::Null))
            && matches!(
                tokens.get(index + 3).map(|t| &t.token),
                Some(Token::Question)
            )
            && matches!(
                tokens.get(index + 4).map(|t| &t.token),
                Some(Token::Ident(then)) if then == checked
            );
        if !is_pattern {
            continue;
        }
        diagnostics.push(RuleDiagnostic {
            range: tokens[index].range.start..tokens[index + 4].range.end,
            message: "prefer `??` over a ternary that falls back on null".to_string(),
            help: Some(format!("rewrite as `{checked} ?? <fallback>`")),
            note: None,
            fix: None,
        });
    }
    if cancellation_requested(cancellation) {
        Vec::new()
    } else {
        diagnostics
    }
}

fn no_unnecessary_allocation(ctx: &LintContext) -> Vec<RuleDiagnostic> {
    no_unnecessary_allocation_cancellable(ctx, None)
}

fn no_unnecessary_allocation_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let tokens = ctx.tokens;
    let mut diagnostics = Vec::new();
    for index in 0..tokens.len() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if !matches!(tokens[index].token, Token::Dot) {
            continue;
        }
        let is_full_slice =
            matches!(
                tokens.get(index + 1).map(|t| &t.token),
                Some(Token::Ident(method)) if method == "slice"
            ) && matches!(tokens.get(index + 2).map(|t| &t.token), Some(Token::LParen))
                && matches!(
                    tokens.get(index + 3).map(|t| &t.token),
                    Some(Token::IntLit(0))
                )
                && matches!(tokens.get(index + 4).map(|t| &t.token), Some(Token::RParen));
        if !is_full_slice {
            continue;
        }
        diagnostics.push(RuleDiagnostic {
            range: tokens[index + 1].range.start..tokens[index + 4].range.end,
            message: "unnecessary allocation: `.slice(0)` copies the whole value".to_string(),
            help: Some("drop the call, or borrow the value instead of copying it".to_string()),
            note: None,
            fix: None,
        });
    }
    if cancellation_requested(cancellation) {
        Vec::new()
    } else {
        diagnostics
    }
}

fn no_unnecessary_await(ctx: &LintContext) -> Vec<RuleDiagnostic> {
    no_unnecessary_await_cancellable(ctx, None)
}

fn no_unnecessary_await_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let mut awaits = Vec::new();
    for (item_index, item) in ctx.program.items.iter().enumerate() {
        if item_index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        let Item::FnDecl {
            is_async: true,
            body,
            ..
        } = item
        else {
            continue;
        };
        for (statement_index, statement) in body.iter().enumerate() {
            if statement_index % 256 == 0 && cancellation_requested(cancellation) {
                return Vec::new();
            }
            let StmtKind::Return(Some(value)) = statement.kind() else {
                continue;
            };
            let ExprKind::Await(dependency) = value.kind() else {
                continue;
            };
            let start = ctx
                .tokens
                .partition_point(|token| token.range.start < value.span.start);
            let mut token = None;
            for (offset, candidate) in ctx.tokens[start..].iter().enumerate() {
                if offset % 256 == 0 && cancellation_requested(cancellation) {
                    return Vec::new();
                }
                if candidate.range.start >= dependency.span.start {
                    break;
                }
                if matches!(candidate.token, Token::Await)
                    && candidate.range.end <= dependency.span.start
                {
                    token = Some(candidate);
                    break;
                }
            }
            let Some(token) = token else {
                continue;
            };
            awaits.push(token.range.clone());
        }
    }
    if cancellation_requested(cancellation) {
        return Vec::new();
    }
    awaits
        .into_iter()
        .map(|range| RuleDiagnostic {
                range: range.clone(),
                message: "unnecessary `await` in an async return".to_string(),
                help: Some(
                    "return the native Promise directly; fulfillment, rejection, cancellation, and payload ownership are preserved"
                        .to_string(),
                ),
                note: None,
                fix: Some(LintFix {
                    message: "Remove the unnecessary `await`".to_string(),
                    range,
                    replacement: String::new(),
                }),
            })
        .collect()
}

fn no_string_concat_in_loop(ctx: &LintContext) -> Vec<RuleDiagnostic> {
    no_string_concat_in_loop_cancellable(ctx, None)
}

fn no_string_concat_in_loop_cancellable(
    ctx: &LintContext,
    cancellation: Option<&AtomicBool>,
) -> Vec<RuleDiagnostic> {
    let tokens = ctx.tokens;
    let mut string_bindings = HashSet::default();
    for (index, symbol) in ctx.analysis.symbols.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if matches!(symbol.kind, SymbolKind::Variable | SymbolKind::Parameter)
            && symbol.semantic_type.as_ref() == Some(&Type::Str)
        {
            string_bindings.insert(symbol.name.clone());
        }
    }
    let mut loop_opens: HashSet<usize> = HashSet::default();
    for (index, spanned) in tokens.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        if matches!(spanned.token, Token::For | Token::While | Token::Do)
            && let Some((open, _)) = brace_block_after_cancellable(tokens, index, cancellation)
        {
            loop_opens.insert(open);
        }
    }

    let mut diagnostics = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    for (index, spanned) in tokens.iter().enumerate() {
        if index % 256 == 0 && cancellation_requested(cancellation) {
            return Vec::new();
        }
        match spanned.token {
            Token::LBrace => stack.push(index),
            Token::RBrace => {
                stack.pop();
            }
            Token::PlusEq => {
                if !stack.iter().any(|open| loop_opens.contains(open)) {
                    continue;
                }
                let Some(target) = index.checked_sub(1).and_then(|prev| tokens.get(prev)) else {
                    continue;
                };
                let Token::Ident(name) = &target.token else {
                    continue;
                };
                if !string_bindings.contains(name) {
                    continue;
                }
                diagnostics.push(RuleDiagnostic {
                    range: target.range.start..spanned.range.end,
                    message: format!("string `{name}` is concatenated in a loop"),
                    help: Some(
                        "collect the parts in an array and `join` them after the loop".to_string(),
                    ),
                    note: None,
                    fix: None,
                });
            }
            _ => {}
        }
    }
    if cancellation_requested(cancellation) {
        Vec::new()
    } else {
        diagnostics
    }
}

fn brace_block_after_cancellable(
    tokens: &[SpannedToken],
    index: usize,
    cancellation: Option<&AtomicBool>,
) -> Option<(usize, usize)> {
    let mut cursor = index + 1;
    if matches!(tokens.get(cursor).map(|t| &t.token), Some(Token::LParen)) {
        let mut depth = 0i32;
        while let Some(token) = tokens.get(cursor) {
            if cursor.is_multiple_of(256) && cancellation_requested(cancellation) {
                return None;
            }
            match token.token {
                Token::LParen => depth += 1,
                Token::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        cursor += 1;
                        break;
                    }
                }
                _ => {}
            }
            cursor += 1;
        }
    }
    if !matches!(tokens.get(cursor).map(|t| &t.token), Some(Token::LBrace)) {
        return None;
    }
    let open = cursor;
    let mut depth = 0i32;
    while let Some(token) = tokens.get(cursor) {
        if cursor.is_multiple_of(256) && cancellation_requested(cancellation) {
            return None;
        }
        match token.token {
            Token::LBrace => depth += 1,
            Token::RBrace => {
                depth -= 1;
                if depth == 0 {
                    return Some((open, cursor));
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}
