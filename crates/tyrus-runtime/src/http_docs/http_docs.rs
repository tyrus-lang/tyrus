use crate::types::{HttpPreHandler, HttpRouter};
use axum::http::{HeaderMap, Uri, header};
use axum::response::{IntoResponse, Response};
use serde_json::{Map, Value, json};
use std::sync::{Arc, Mutex};
use tyrus_string::TyrusString;

const SWAGGER_UI_BUNDLE: &[u8] = include_bytes!("../../assets/swagger-ui/swagger-ui-bundle.js");
const SWAGGER_UI_CSS: &[u8] = include_bytes!("../../assets/swagger-ui/swagger-ui.css");

const DEFAULT_DOCS_PATH: &str = "/docs";
const DEFAULT_TITLE: &str = "API";
const DEFAULT_VERSION: &str = "1.0.0";

pub(crate) type RouteFragments = Arc<Mutex<Vec<String>>>;

struct DocsInfo {
    title: String,
    version: String,
    description: String,
    auth: Option<Value>,
    css: String,
}

const EXAMPLE_DEPTH: usize = 8;

fn example_for(schema: &Value, schemas: &Map<String, Value>, depth: usize) -> Option<Value> {
    if depth > EXAMPLE_DEPTH {
        return None;
    }
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        let name = reference.strip_prefix("#/components/schemas/")?;
        return example_for(schemas.get(name)?, schemas, depth + 1);
    }
    if let Some(value) = schema.get("const") {
        return Some(value.clone());
    }
    if let Some(first) = schema
        .get("enum")
        .and_then(Value::as_array)
        .and_then(|values| values.first())
    {
        return Some(first.clone());
    }
    for combinator in ["anyOf", "oneOf", "allOf"] {
        if let Some(first) = schema
            .get(combinator)
            .and_then(Value::as_array)
            .and_then(|variants| variants.first())
        {
            return example_for(first, schemas, depth + 1);
        }
    }
    let number = |key: &str| schema.get(key).and_then(Value::as_f64);
    match schema.get("type").and_then(Value::as_str) {
        Some("string") => {
            let text = match schema.get("format").and_then(Value::as_str) {
                Some("binary") => return None,
                Some("email") => "user@example.com".to_string(),
                Some("uri" | "url") => "https://example.com".to_string(),
                Some("uuid") => "00000000-0000-4000-8000-000000000000".to_string(),
                Some("ipv4") => "192.0.2.1".to_string(),
                Some("ipv6") => "2001:db8::1".to_string(),
                Some("decimal") => "0.00".to_string(),
                _ => {
                    let minimum =
                        schema.get("minLength").and_then(Value::as_u64).unwrap_or(0) as usize;
                    let maximum = schema
                        .get("maxLength")
                        .and_then(Value::as_u64)
                        .map(|value| value as usize);
                    let text = "string".to_string();
                    if text.len() < minimum {
                        "s".repeat(minimum)
                    } else {
                        match maximum {
                            Some(maximum) if text.len() > maximum => "s".repeat(maximum),
                            _ => text,
                        }
                    }
                }
            };
            Some(Value::String(text))
        }
        Some("integer") => {
            let value = if let Some(minimum) = number("minimum") {
                minimum.ceil()
            } else if let Some(minimum) = number("exclusiveMinimum") {
                minimum.floor() + 1.0
            } else if let Some(maximum) = number("maximum") {
                maximum.min(0.0).floor()
            } else {
                0.0
            };
            Some(json!(value as i64))
        }
        Some("number") => Some(json!(
            number("minimum")
                .or(number("exclusiveMinimum"))
                .unwrap_or(0.0)
        )),
        Some("boolean") => Some(Value::Bool(true)),
        Some("array") => {
            if let Some(items) = schema.get("prefixItems").and_then(Value::as_array) {
                return items
                    .iter()
                    .map(|item| example_for(item, schemas, depth + 1))
                    .collect::<Option<Vec<_>>>()
                    .map(Value::Array);
            }
            let item = schema
                .get("items")
                .and_then(|items| example_for(items, schemas, depth + 1));
            let count = schema
                .get("minItems")
                .and_then(Value::as_u64)
                .unwrap_or(1)
                .max(1) as usize;
            let unique = schema
                .get("uniqueItems")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            Some(Value::Array(match item {
                Some(item) if !unique => vec![item; count],
                Some(item) => vec![item],
                None => Vec::new(),
            }))
        }
        Some("object") => {
            let mut object = Map::new();
            if let Some(Value::Object(properties)) = schema.get("properties") {
                for (name, property) in properties {
                    if let Some(value) = example_for(property, schemas, depth + 1) {
                        object.insert(name.clone(), value);
                    }
                }
            }
            Some(Value::Object(object))
        }
        _ => None,
    }
}

fn add_examples(paths: &mut Map<String, Value>, schemas: &Map<String, Value>) {
    for operations in paths.values_mut() {
        let Value::Object(operations) = operations else {
            continue;
        };
        for operation in operations.values_mut() {
            let mut contents = Vec::new();
            if let Some(content) = operation.pointer_mut("/requestBody/content") {
                contents.push(content as *mut Value);
            }
            if let Some(Value::Object(responses)) = operation.get_mut("responses") {
                for response in responses.values_mut() {
                    if let Some(content) = response.get_mut("content") {
                        contents.push(content as *mut Value);
                    }
                }
            }
            for content in contents {
                // SAFETY: each pointer comes from a distinct `get_mut` on the
                // same operation, and none of them is used after this loop.
                let Value::Object(media) = (unsafe { &mut *content }) else {
                    continue;
                };
                let Some(json_media) = media.get_mut("application/json") else {
                    continue;
                };
                if json_media.get("example").is_some() {
                    continue;
                }
                let Some(example) = json_media
                    .get("schema")
                    .and_then(|schema| example_for(schema, schemas, 0))
                else {
                    continue;
                };
                json_media["example"] = example;
            }
        }
    }
}

fn security_from(auth: &Value) -> (Map<String, Value>, Vec<Value>) {
    let mut schemes = Map::new();
    let mut requirements = Vec::new();
    if let Some(bearer) = auth.get("bearer") {
        let mut scheme = json!({ "type": "http", "scheme": "bearer" });
        if let Some(format) = bearer.get("format").and_then(Value::as_str) {
            scheme["bearerFormat"] = Value::String(format.to_string());
        }
        schemes.insert("bearerAuth".to_string(), scheme);
        requirements.push(json!({ "bearerAuth": [] }));
    }
    if let Some(api_key) = auth.get("apiKey") {
        schemes.insert(
            "apiKeyAuth".to_string(),
            json!({ "type": "apiKey", "name": api_key["name"], "in": api_key["in"] }),
        );
        requirements.push(json!({ "apiKeyAuth": [] }));
    }
    if let Some(oauth2) = auth.get("oauth2") {
        let scopes = oauth2
            .get("scopes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let scope_map = scopes
            .iter()
            .filter_map(Value::as_str)
            .map(|scope| (scope.to_string(), Value::String(String::new())))
            .collect::<Map<_, _>>();
        let flow = match oauth2.get("authorizationUrl").and_then(Value::as_str) {
            Some(authorization_url) => json!({
                "authorizationCode": {
                    "authorizationUrl": authorization_url,
                    "tokenUrl": oauth2["tokenUrl"],
                    "scopes": scope_map,
                }
            }),
            None => json!({
                "clientCredentials": { "tokenUrl": oauth2["tokenUrl"], "scopes": scope_map }
            }),
        };
        schemes.insert(
            "oauth2Auth".to_string(),
            json!({ "type": "oauth2", "flows": flow }),
        );
        requirements.push(json!({ "oauth2Auth": scopes }));
    }
    (schemes, requirements)
}

fn build_document(fragments: &[String], info: &DocsInfo) -> Value {
    let mut paths = Map::new();
    let mut schemas = Map::new();
    let mut tags = std::collections::BTreeSet::new();
    for fragment in fragments {
        let Ok(mut fragment) = serde_json::from_str::<Value>(fragment) else {
            continue;
        };
        let mut renames = Vec::new();
        if let Value::Object(fragment_schemas) = &fragment["schemas"] {
            for (name, schema) in fragment_schemas {
                if !schemas.get(name).is_some_and(|existing| existing != schema) {
                    continue;
                }
                let mut suffix = 2;
                loop {
                    let candidate = format!("{name}{suffix}");
                    match schemas.get(&candidate) {
                        Some(existing) if existing != schema => suffix += 1,
                        _ => {
                            renames.push((name.clone(), candidate));
                            break;
                        }
                    }
                }
            }
        }
        if !renames.is_empty() {
            let mut text = fragment.to_string();
            for (from, to) in &renames {
                text = text.replace(
                    &format!("\"#/components/schemas/{from}\""),
                    &format!("\"#/components/schemas/{to}\""),
                );
            }
            let Ok(renamed) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            fragment = renamed;
            if let Value::Object(fragment_schemas) = &mut fragment["schemas"] {
                for (from, to) in &renames {
                    if let Some(schema) = fragment_schemas.remove(from) {
                        fragment_schemas.insert(to.clone(), schema);
                    }
                }
            }
        }
        let (Some(path), Some(method)) = (fragment["path"].as_str(), fragment["method"].as_str())
        else {
            continue;
        };
        let entry = paths
            .entry(path.to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if let Value::Object(operations) = entry {
            if let Some(operation_tags) = fragment["operation"]["tags"].as_array() {
                tags.extend(
                    operation_tags
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string),
                );
            }
            operations.insert(method.to_string(), fragment["operation"].clone());
        }
        if let Value::Object(fragment_schemas) = &fragment["schemas"] {
            for (name, schema) in fragment_schemas {
                schemas
                    .entry(name.clone())
                    .or_insert_with(|| schema.clone());
            }
        }
    }
    add_examples(&mut paths, &schemas);
    let mut info_value = json!({ "title": info.title, "version": info.version });
    if !info.description.is_empty() {
        info_value["description"] = Value::String(info.description.clone());
    }
    let mut document = json!({
        "openapi": "3.2.0",
        "info": info_value,
        "paths": paths,
    });
    if !tags.is_empty() {
        document["tags"] = json!(
            tags.into_iter()
                .map(|name| json!({ "name": name }))
                .collect::<Vec<_>>()
        );
    }
    let mut components = Map::new();
    if !schemas.is_empty() {
        components.insert("schemas".to_string(), Value::Object(schemas));
    }
    if let Some(auth) = &info.auth {
        let (schemes, requirements) = security_from(auth);
        if !schemes.is_empty() {
            components.insert("securitySchemes".to_string(), Value::Object(schemes));
            document["security"] = Value::Array(requirements);
        }
    }
    if !components.is_empty() {
        document["components"] = Value::Object(components);
    }
    document
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn docs_page(base: &str, title: &str, css: &str) -> String {
    let spec_url = serde_json::to_string(&format!("{base}/openapi.json"))
        .unwrap_or_else(|_| "\"/docs/openapi.json\"".to_string())
        .replace("</", "<\\/");
    let base = escape_html(base);
    let title = escape_html(title);
    let css = css.replace("</", "<\\/");
    format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<link rel="stylesheet" href="{base}/swagger-ui.css">
<style>{css}</style>
</head>
<body>
<div id="swagger-ui"></div>
<script src="{base}/swagger-ui-bundle.js"></script>
<script>
window.ui = SwaggerUIBundle({{ url: {spec_url}, dom_id: "#swagger-ui", persistAuthorization: true }});
</script>
</body>
</html>
"##
    )
}

pub(crate) fn refused_by_pre_handlers(
    pre_handlers: &[HttpPreHandler],
    route_params: &crate::http::RouteParams,
    headers: &HeaderMap,
    uri: &Uri,
    request_id: Option<&str>,
) -> Option<Response> {
    if pre_handlers.is_empty() {
        return None;
    }
    let path = uri.path().to_string();
    let query = crate::http::query_json(uri.query().unwrap_or_default());
    let params = crate::http::path_params_json(route_params, &path);
    let headers_text = crate::http::headers_json(headers);
    let cookies = crate::http::cookies_json(headers);
    let id = request_id.unwrap_or_default().to_string();
    // SAFETY: every source outlives the calls below, which end in this scope.
    let lend = |text: &str| unsafe { TyrusString::lend(text) }.unwrap_or_else(TyrusString::null);
    let request = crate::types::HttpRequest {
        id: lend(&id),
        method: lend("GET"),
        path: lend(&path),
        body: TyrusString::null(),
        params: lend(&params),
        query: lend(&query),
        headers: lend(&headers_text),
        cookies: lend(&cookies),
        raw_body: std::ptr::null(),
        raw_body_length: 0,
        parts: std::ptr::null(),
        parts_length: 0,
        body_stream: std::ptr::null(),
    };
    for pre_handler in pre_handlers {
        let mut response = crate::types::HttpResponse::empty();
        let key: *const crate::types::HttpResponse = &response;
        if unsafe { pre_handler(&request, &mut response) } != 0 {
            return Some(crate::http::finalize_response(
                response, key, false, false, None, None, None,
            ));
        }
    }
    None
}

#[derive(Clone, Copy)]
enum DocsAsset {
    Page,
    Document,
    Stylesheet,
    Script,
}

struct DocsRoute {
    base: String,
    info: DocsInfo,
    fragments: RouteFragments,
    pre_handlers: Vec<HttpPreHandler>,
    route_params: crate::http::RouteParams,
}

fn serve_asset(
    route: &DocsRoute,
    asset: DocsAsset,
    headers: &HeaderMap,
    uri: &Uri,
    request_id: Option<&str>,
) -> Response {
    if let Some(refusal) = refused_by_pre_handlers(
        &route.pre_handlers,
        &route.route_params,
        headers,
        uri,
        request_id,
    ) {
        return refusal;
    }
    match asset {
        DocsAsset::Page => (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            docs_page(&route.base, &route.info.title, &route.info.css),
        )
            .into_response(),
        DocsAsset::Document => {
            let fragments = match route.fragments.lock() {
                Ok(fragments) => fragments.clone(),
                Err(poisoned) => poisoned.into_inner().clone(),
            };
            let document = build_document(&fragments, &route.info);
            (
                [(header::CONTENT_TYPE, "application/json")],
                document.to_string(),
            )
                .into_response()
        }
        DocsAsset::Stylesheet => (
            [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
            SWAGGER_UI_CSS,
        )
            .into_response(),
        DocsAsset::Script => (
            [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
            SWAGGER_UI_BUNDLE,
        )
            .into_response(),
    }
}

fn text_or(value: &TyrusString, default: &str) -> Result<String, ()> {
    if value.is_null() {
        return Ok(default.to_string());
    }
    let text = crate::types::text(value)?;
    Ok(if text.is_empty() {
        default.to_string()
    } else {
        text
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_router_describe(
    router: *mut HttpRouter,
    fragment: TyrusString,
) -> i32 {
    let (Some(router), Ok(fragment)) = (unsafe { router.as_mut() }, crate::types::text(&fragment))
    else {
        return -1;
    };
    match router.openapi_fragments.lock() {
        Ok(mut fragments) => fragments.push(fragment),
        Err(poisoned) => poisoned.into_inner().push(fragment),
    }
    0
}

/// Every `app.docs()` option that is text, behind one pointer.
///
/// A `string` travels by value in two argument registers, and a call whose
/// string starts with only one register left is laid out differently by the
/// code the compiler emits and by this function: the strings after it are read
/// from the wrong stack slots. The page's `css` arrived as garbage when `auth`
/// was set too. Behind a pointer every option has the one layout `#[repr(C)]`
/// states. A null field is an option the program left out.
#[repr(C)]
pub struct DocsOptions {
    pub path: TyrusString,
    pub title: TyrusString,
    pub version: TyrusString,
    pub description: TyrusString,
    pub auth: TyrusString,
    pub css: TyrusString,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_http_router_docs(
    router: *mut HttpRouter,
    options: *const DocsOptions,
    pre_handlers: *const HttpPreHandler,
    pre_handlers_len: usize,
) -> i32 {
    let (Some(router), Some(options)) = (unsafe { router.as_mut() }, unsafe { options.as_ref() })
    else {
        crate::http::set_http_error("ERR_HTTP_DOCS_CONFIG", "router or docs options are invalid");
        return -1;
    };
    let (Ok(base), Ok(title), Ok(version), Ok(description), Ok(auth), Ok(css)) = (
        text_or(&options.path, DEFAULT_DOCS_PATH),
        text_or(&options.title, DEFAULT_TITLE),
        text_or(&options.version, DEFAULT_VERSION),
        text_or(&options.description, ""),
        text_or(&options.auth, ""),
        text_or(&options.css, ""),
    ) else {
        crate::http::set_http_error("ERR_HTTP_DOCS_CONFIG", "docs options contain invalid text");
        return -1;
    };
    let auth = if auth.is_empty() {
        None
    } else {
        match serde_json::from_str::<Value>(&auth) {
            Ok(auth) => Some(auth),
            Err(_) => {
                crate::http::set_http_error(
                    "ERR_HTTP_DOCS_CONFIG",
                    "docs auth description is invalid JSON",
                );
                return -1;
            }
        }
    };
    let base = base.trim_end_matches('/').to_string();
    if !base.starts_with('/') {
        crate::http::set_http_error("ERR_HTTP_DOCS_CONFIG", "docs path must start with `/`");
        return -1;
    }
    let route = Arc::new(DocsRoute {
        route_params: crate::http::RouteParams::parse(&base),
        base: base.clone(),
        info: DocsInfo {
            title,
            version,
            description,
            auth,
            css,
        },
        fragments: Arc::clone(&router.openapi_fragments),
        pre_handlers: if pre_handlers.is_null() || pre_handlers_len == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(pre_handlers, pre_handlers_len) }.to_vec()
        },
    });
    let mut current = std::mem::take(&mut router.router);
    for (suffix, asset) in [
        ("", DocsAsset::Page),
        ("/openapi.json", DocsAsset::Document),
        ("/swagger-ui.css", DocsAsset::Stylesheet),
        ("/swagger-ui-bundle.js", DocsAsset::Script),
    ] {
        let route = Arc::clone(&route);
        current = current.route(
            &format!("{base}{suffix}"),
            axum::routing::get(
                move |headers: HeaderMap,
                      uri: Uri,
                      request_id: Option<axum::Extension<crate::http::RequestId>>| {
                    let route = Arc::clone(&route);
                    async move {
                        let request_id = request_id.map(|axum::Extension(id)| id.0);
                        serve_asset(&route, asset, &headers, &uri, request_id.as_deref())
                    }
                },
            ),
        );
    }
    router.router = current;
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::StatusCode;
    use tower::ServiceExt;

    fn fragment(path: &str, method: &str, schema: &str) -> String {
        json!({
            "path": path,
            "method": method,
            "operation": { "responses": { "200": { "description": "OK" } } },
            "schemas": { schema: { "type": "object" } },
        })
        .to_string()
    }

    async fn get(
        router: &HttpRouter,
        path: &str,
        header: Option<(&str, &str)>,
    ) -> (StatusCode, String, String) {
        let mut request = axum::http::Request::get(path);
        if let Some((name, value)) = header {
            request = request.header(name, value);
        }
        let response = router
            .router
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let body = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
            .await
            .unwrap();
        (
            status,
            content_type,
            String::from_utf8_lossy(&body).into_owned(),
        )
    }

    #[test]
    fn a_document_merges_the_operations_and_schemas_of_every_route() {
        let fragments = vec![
            fragment("/users", "get", "User"),
            fragment("/users", "post", "User"),
            fragment("/health", "get", "Health"),
            "not json".to_string(),
        ];
        let document = build_document(
            &fragments,
            &DocsInfo {
                title: "Users API".to_string(),
                version: "2.0.0".to_string(),
                description: String::new(),
                auth: None,
                css: String::new(),
            },
        );
        assert_eq!(document["openapi"], "3.2.0");
        assert_eq!(document["info"]["title"], "Users API");
        assert!(document["info"].get("description").is_none());
        assert!(document["paths"]["/users"].get("get").is_some());
        assert!(document["paths"]["/users"].get("post").is_some());
        assert!(document["paths"]["/health"].get("get").is_some());
        assert!(document["components"]["schemas"].get("User").is_some());
        assert!(document["components"]["schemas"].get("Health").is_some());
    }

    fn info() -> DocsInfo {
        DocsInfo {
            title: "API".to_string(),
            version: "1.0.0".to_string(),
            description: String::new(),
            auth: None,
            css: String::new(),
        }
    }

    #[test]
    fn schemas_with_one_name_and_different_fields_are_kept_apart() {
        let first = json!({
            "path": "/a", "method": "get",
            "operation": { "responses": { "200": { "description": "OK", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/User" } } } } } },
            "schemas": { "User": { "type": "object", "properties": { "name": { "type": "string" } } } },
        }).to_string();
        let second = json!({
            "path": "/b", "method": "get",
            "operation": { "responses": { "200": { "description": "OK", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/User" } } } } } },
            "schemas": { "User": { "type": "object", "properties": { "id": { "type": "integer" } } } },
        }).to_string();
        let document = build_document(&[first, second], &info());
        let schemas = &document["components"]["schemas"];
        assert!(schemas["User"]["properties"].get("name").is_some());
        assert!(schemas["User2"]["properties"].get("id").is_some());
        assert_eq!(
            document["paths"]["/b"]["get"]["responses"]["200"]["content"]["application/json"]["schema"]
                ["$ref"],
            "#/components/schemas/User2"
        );
        assert_eq!(
            document["paths"]["/a"]["get"]["responses"]["200"]["content"]["application/json"]["schema"]
                ["$ref"],
            "#/components/schemas/User"
        );
    }

    #[test]
    fn json_bodies_get_an_example_that_satisfies_their_schema() {
        let fragment = json!({
            "path": "/users", "method": "post",
            "operation": {
                "requestBody": { "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CreateUser" } } } },
                "responses": { "201": { "description": "Created", "content": { "application/json": { "schema": { "type": "array", "items": { "type": "integer", "minimum": 5 }, "minItems": 2 } } } } },
            },
            "schemas": { "CreateUser": { "type": "object", "properties": {
                "email": { "type": "string", "format": "email" },
                "name": { "type": "string", "minLength": 10 },
                "role": { "type": "string", "enum": ["admin", "guest"] },
                "age": { "type": "integer", "exclusiveMinimum": 17 },
            } } },
        }).to_string();
        let document = build_document(&[fragment], &info());
        let request = &document["paths"]["/users"]["post"]["requestBody"]["content"]["application/json"]
            ["example"];
        assert_eq!(request["email"], "user@example.com");
        assert_eq!(request["name"].as_str().unwrap().len(), 10);
        assert_eq!(request["role"], "admin");
        assert_eq!(request["age"], 18);
        let response = &document["paths"]["/users"]["post"]["responses"]["201"]["content"]["application/json"]
            ["example"];
        assert_eq!(response, &json!([5, 5]));
    }

    #[test]
    fn a_schema_that_refers_to_itself_still_gets_a_finite_example() {
        let fragment = json!({
            "path": "/tree", "method": "get",
            "operation": { "responses": { "200": { "description": "OK", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Node" } } } } } },
            "schemas": { "Node": { "type": "object", "properties": { "children": { "type": "array", "items": { "$ref": "#/components/schemas/Node" } } } } },
        }).to_string();
        let document = build_document(&[fragment], &info());
        assert!(
            document["paths"]["/tree"]["get"]["responses"]["200"]["content"]["application/json"]
                .get("example")
                .is_some()
        );
    }

    #[test]
    fn auth_becomes_security_schemes_and_a_global_requirement() {
        let mut docs = info();
        docs.auth = Some(json!({
            "bearer": { "format": "JWT" },
            "apiKey": { "name": "x-api-key", "in": "header" },
            "oauth2": { "authorizationUrl": "https://auth.example.com/authorize", "tokenUrl": "https://auth.example.com/token", "scopes": ["read"] },
        }));
        let document = build_document(&[fragment("/users", "get", "User")], &docs);
        let schemes = &document["components"]["securitySchemes"];
        assert_eq!(schemes["bearerAuth"]["scheme"], "bearer");
        assert_eq!(schemes["bearerAuth"]["bearerFormat"], "JWT");
        assert_eq!(schemes["apiKeyAuth"]["name"], "x-api-key");
        assert_eq!(schemes["apiKeyAuth"]["in"], "header");
        assert_eq!(
            schemes["oauth2Auth"]["flows"]["authorizationCode"]["tokenUrl"],
            "https://auth.example.com/token"
        );
        assert!(
            schemes["oauth2Auth"]["flows"]["authorizationCode"]["scopes"]
                .get("read")
                .is_some()
        );
        let security = document["security"].as_array().unwrap();
        assert_eq!(security.len(), 3);
        assert!(document["components"]["schemas"].get("User").is_some());
    }

    #[test]
    fn custom_css_is_written_into_the_page_without_ending_the_style_element() {
        let page = docs_page(
            "/docs",
            "API",
            ".topbar { display: none; }</style><script>alert(1)</script>",
        );
        assert!(page.contains(".topbar { display: none; }"));
        assert!(!page.contains("</style><script>"));
        assert!(page.contains("persistAuthorization: true"));
    }

    #[test]
    fn the_docs_page_cannot_be_broken_out_of_by_its_title_or_path() {
        let page = docs_page("/docs", "<script>alert(1)</script>", "");
        assert!(!page.contains("<script>alert(1)</script>"));
        assert!(page.contains("&lt;script&gt;"));
        assert!(page.contains(r#"url: "/docs/openapi.json""#));
    }

    #[tokio::test]
    async fn docs_serve_the_document_page_and_assets_with_routes_added_later() {
        let router = crate::http::tyrus_http_router_new();
        let router_ref = unsafe { &mut *router };
        unsafe {
            let options = DocsOptions {
                path: TyrusString::literal("/reference/"),
                title: TyrusString::literal("Users API"),
                version: TyrusString::null(),
                description: TyrusString::null(),
                auth: TyrusString::null(),
                css: TyrusString::null(),
            };
            assert_eq!(
                tyrus_http_router_docs(router, &options, std::ptr::null(), 0,),
                0
            );
            // `literal` wants a `&'static str` and this fragment is built at
            // run time, so it has to outlive the call. A `Box::leak` gave it
            // that lifetime by abandoning the block, which LeakSanitizer counts
            // as a leak because nothing points at it any more. A static keeps a
            // root to the same one allocation, so it is bounded and reachable.
            static DESCRIBED: std::sync::OnceLock<String> = std::sync::OnceLock::new();
            let described = DESCRIBED.get_or_init(|| fragment("/users/{id}", "get", "User"));
            assert_eq!(
                tyrus_http_router_describe(router, TyrusString::literal(described.as_str())),
                0
            );
        }

        let (status, content_type, body) = get(router_ref, "/reference/openapi.json", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type, "application/json");
        let document: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(document["info"]["title"], "Users API");
        assert_eq!(document["info"]["version"], DEFAULT_VERSION);
        assert!(document["paths"]["/users/{id}"].get("get").is_some());

        let (status, content_type, body) = get(router_ref, "/reference", None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(content_type.starts_with("text/html"));
        assert!(body.contains("/reference/swagger-ui-bundle.js"));

        let (status, content_type, body) =
            get(router_ref, "/reference/swagger-ui-bundle.js", None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(content_type.starts_with("text/javascript"));
        assert_eq!(body.len(), SWAGGER_UI_BUNDLE.len());

        unsafe { crate::http::tyrus_http_router_free(router) };
    }

    unsafe extern "C" fn require_docs_token(
        request: *const crate::types::HttpRequest,
        response: *mut crate::types::HttpResponse,
    ) -> i32 {
        if unsafe { (*request).headers.as_str() }.contains("x-docs-token") {
            return 0;
        }
        unsafe {
            (*response).status = 403;
            (*response).body = crate::types::HttpBody::from_text(TyrusString::literal("forbidden"));
        }
        1
    }

    #[tokio::test]
    async fn a_pre_handler_guards_every_docs_path() {
        let router = crate::http::tyrus_http_router_new();
        let router_ref = unsafe { &mut *router };
        let pre_handlers: [HttpPreHandler; 1] = [require_docs_token];
        unsafe {
            assert_eq!(
                tyrus_http_router_docs(
                    router,
                    &DocsOptions {
                        path: TyrusString::null(),
                        title: TyrusString::null(),
                        version: TyrusString::null(),
                        description: TyrusString::null(),
                        auth: TyrusString::null(),
                        css: TyrusString::null(),
                    },
                    pre_handlers.as_ptr(),
                    pre_handlers.len(),
                ),
                0
            );
        }
        for path in [
            "/docs",
            "/docs/openapi.json",
            "/docs/swagger-ui.css",
            "/docs/swagger-ui-bundle.js",
        ] {
            let (status, _, _) = get(router_ref, path, None).await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "{path} was served without the token"
            );
        }
        let (status, _, _) = get(
            router_ref,
            "/docs/openapi.json",
            Some(("x-docs-token", "yes")),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        unsafe { crate::http::tyrus_http_router_free(router) };
    }
}
