use crate::ast::{Expr, ExprKind, Field, ResolvedEnum, Type, ValidatorDecorator};
use crate::collections::HashMap;
use crate::http_body::{BYTES_BODY, MULTIPART_BODY, STREAM_BODY, TEXT_BODY};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

pub struct SchemaTables<'a> {
    pub interfaces: &'a HashMap<String, Vec<Field>>,
    pub enums: &'a HashMap<String, ResolvedEnum>,
}

pub struct RouteDescription<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub tags: Vec<String>,
    pub request: Vec<(&'a str, &'a str)>,
    pub responses: Vec<(u16, Type)>,
}

pub fn openapi_path(path: &str) -> String {
    path.split('/')
        .map(|segment| match segment.strip_prefix(':') {
            Some(name) => format!("{{{name}}}"),
            None => segment.to_string(),
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn path_parameter_names(path: &str) -> Vec<&str> {
    path.split('/')
        .filter_map(|segment| segment.strip_prefix(':'))
        .collect()
}

struct Describer<'a> {
    tables: &'a SchemaTables<'a>,
    components: BTreeMap<String, Value>,
}

impl Describer<'_> {
    fn integer(format: &str) -> Value {
        json!({ "type": "integer", "format": format })
    }

    fn schema(&mut self, ty: &Type) -> Value {
        match ty {
            Type::Str | Type::Char => json!({ "type": "string" }),
            Type::Bool => json!({ "type": "boolean" }),
            Type::I8 | Type::I16 | Type::I32 => Self::integer("int32"),
            Type::U8 | Type::U16 | Type::U32 => {
                json!({ "type": "integer", "format": "int32", "minimum": 0 })
            }
            Type::I64 | Type::Isize => Self::integer("int64"),
            Type::U64 | Type::Usize => {
                json!({ "type": "integer", "format": "int64", "minimum": 0 })
            }
            Type::I128 | Type::U128 => json!({ "type": "integer" }),
            Type::F32 => json!({ "type": "number", "format": "float" }),
            Type::F64 | Type::F128 => json!({ "type": "number", "format": "double" }),
            Type::Decimal => json!({ "type": "string", "format": "decimal" }),
            Type::Array(element) => json!({ "type": "array", "items": self.schema(element) }),
            Type::Tuple(elements) => {
                let items: Vec<Value> = elements
                    .iter()
                    .map(|element| self.schema(element))
                    .collect();
                json!({ "type": "array", "prefixItems": items, "minItems": elements.len(), "maxItems": elements.len() })
            }
            Type::Readonly(inner) | Type::Ref(inner) | Type::MutRef(inner) => self.schema(inner),
            Type::Optional(inner) => self.schema(inner),
            Type::Map(_, value) => {
                json!({ "type": "object", "additionalProperties": self.schema(value) })
            }
            Type::Object(fields) => self.object(fields),
            Type::Union(variants) => {
                let variants: Vec<Value> = variants
                    .iter()
                    .map(|variant| self.schema(variant))
                    .collect();
                json!({ "anyOf": variants })
            }
            Type::Lit(literal) => match literal.kind() {
                ExprKind::StrLit(text) => json!({ "type": "string", "enum": [text] }),
                ExprKind::IntLit(value) => json!({ "type": "integer", "enum": [*value as i64] }),
                ExprKind::UIntLit(value) => json!({ "type": "integer", "enum": [*value as u64] }),
                ExprKind::BoolLit(value) => json!({ "type": "boolean", "enum": [value] }),
                _ => json!({}),
            },
            Type::Named(name) => self.named(name),
            _ => json!({}),
        }
    }

    fn named(&mut self, name: &str) -> Value {
        if name == "Instant" {
            return json!({ "type": "string", "format": "date-time" });
        }
        let public = crate::module_loader::original_global_name(name).to_string();
        let reference = json!({ "$ref": format!("#/components/schemas/{public}") });
        if self.components.contains_key(&public) {
            return reference;
        }
        if let Some(fields) = self.tables.interfaces.get(name) {
            self.components.insert(public.clone(), json!({}));
            let fields = fields.clone();
            let object = self.object(&fields);
            self.components.insert(public, object);
            return reference;
        }
        if let Some(info) = self.tables.enums.get(name) {
            let schema = enum_schema(info);
            self.components.insert(public, schema);
            return reference;
        }
        json!({})
    }

    fn object(&mut self, fields: &[Field]) -> Value {
        let mut properties = Map::new();
        let mut required = Vec::new();
        for field in fields {
            properties.insert(field.name.clone(), self.field_schema(field));
            if field_required(field) {
                required.push(Value::String(field.name.clone()));
            }
        }
        let mut object = json!({ "type": "object", "properties": properties });
        if !required.is_empty() {
            object["required"] = Value::Array(required);
        }
        object
    }

    fn field_schema(&mut self, field: &Field) -> Value {
        let mut schema = self.schema(&field.ty);
        apply_validators(&mut schema, &field.decorators);
        schema
    }

    fn fields_of(&self, schema: &str) -> Vec<Field> {
        self.tables
            .interfaces
            .get(schema)
            .cloned()
            .unwrap_or_default()
    }

    fn content(&mut self, schema: &Type) -> Option<Value> {
        match schema {
            Type::Void => None,
            Type::Named(name) if name == TEXT_BODY => {
                Some(json!({ "text/plain": { "schema": { "type": "string" } } }))
            }
            Type::Named(name) if name == BYTES_BODY || name == STREAM_BODY => Some(json!({
                "application/octet-stream": { "schema": { "type": "string", "format": "binary" } }
            })),
            Type::Named(name) if name == MULTIPART_BODY => Some(json!({
                "multipart/form-data": {
                    "schema": {
                        "type": "object",
                        "additionalProperties": { "type": "string", "format": "binary" },
                        "description": "Each part is delivered to the handler as a MultipartPart with its name, fileName, contentType and bytes."
                    }
                }
            })),
            other => {
                let schema = self.schema(other);
                Some(json!({ "application/json": { "schema": schema } }))
            }
        }
    }
}

fn field_required(field: &Field) -> bool {
    let named = |name: &str| {
        field
            .decorators
            .iter()
            .any(|decorator| decorator.name == name)
    };
    if named("required") || named("notNull") {
        return true;
    }
    if named("optional") {
        return false;
    }
    !field.optional && !matches!(field.ty, Type::Optional(_))
}

fn literal_value(expr: &Expr) -> Option<Value> {
    match expr.kind() {
        ExprKind::StrLit(text) => Some(Value::String(text.clone())),
        ExprKind::BoolLit(value) => Some(json!(value)),
        ExprKind::IntLit(value) => Some(json!(*value as i64)),
        ExprKind::UIntLit(value) => Some(json!(*value as u64)),
        ExprKind::FloatLit(value) => Some(json!(value.value())),
        ExprKind::Neg(inner) => match literal_value(inner)? {
            Value::Number(number) => number
                .as_i64()
                .map(|value| json!(-value))
                .or_else(|| number.as_f64().map(|value| json!(-value))),
            _ => None,
        },
        _ => None,
    }
}

fn set_keyword(
    schema: &mut Value,
    keyword: &str,
    value: Value,
    extensions: &mut Vec<Value>,
    display: &str,
) {
    match schema.get(keyword) {
        None => schema[keyword] = value,
        Some(_) => extensions.push(Value::String(display.to_string())),
    }
}

fn apply_validators(schema: &mut Value, decorators: &[ValidatorDecorator]) {
    use crate::validators::{
        integer_arg_value, literal_array_arg, numeric_arg_value, primitive_arg_expr,
        string_arg_value,
    };

    if decorators.is_empty() || !schema.is_object() {
        return;
    }
    let mut extensions = Vec::new();
    for decorator in decorators {
        let args = &decorator.args;
        let display = crate::validators::validator_display(decorator);
        let number = |index| numeric_arg_value(args, index).map(|value| json!(value));
        let integer = |index| integer_arg_value(args, index).map(|value| json!(value));
        let mut handled = true;
        match decorator.name.as_str() {
            "min" => match number(0) {
                Some(value) => set_keyword(schema, "minimum", value, &mut extensions, &display),
                None => handled = false,
            },
            "max" => match number(0) {
                Some(value) => set_keyword(schema, "maximum", value, &mut extensions, &display),
                None => handled = false,
            },
            "range" => match (number(0), number(1)) {
                (Some(low), Some(high)) => {
                    set_keyword(schema, "minimum", low, &mut extensions, &display);
                    set_keyword(schema, "maximum", high, &mut extensions, &display);
                }
                _ => handled = false,
            },
            "positive" => set_keyword(
                schema,
                "exclusiveMinimum",
                json!(0),
                &mut extensions,
                &display,
            ),
            "negative" => set_keyword(
                schema,
                "exclusiveMaximum",
                json!(0),
                &mut extensions,
                &display,
            ),
            "nonNegative" => set_keyword(schema, "minimum", json!(0), &mut extensions, &display),
            "nonPositive" => set_keyword(schema, "maximum", json!(0), &mut extensions, &display),
            "multipleOf" => match number(0) {
                Some(value) => set_keyword(schema, "multipleOf", value, &mut extensions, &display),
                None => handled = false,
            },
            "safeInteger" => {
                set_keyword(
                    schema,
                    "minimum",
                    json!(-9_007_199_254_740_991_i64),
                    &mut extensions,
                    &display,
                );
                set_keyword(
                    schema,
                    "maximum",
                    json!(9_007_199_254_740_991_i64),
                    &mut extensions,
                    &display,
                );
            }
            "port" => {
                set_keyword(schema, "minimum", json!(1024), &mut extensions, &display);
                set_keyword(schema, "maximum", json!(49151), &mut extensions, &display);
            }
            "statusCode" => {
                set_keyword(schema, "minimum", json!(100), &mut extensions, &display);
                set_keyword(schema, "maximum", json!(599), &mut extensions, &display);
            }
            "minLength" => match integer(0) {
                Some(value) => set_keyword(schema, "minLength", value, &mut extensions, &display),
                None => handled = false,
            },
            "maxLength" => match integer(0) {
                Some(value) => set_keyword(schema, "maxLength", value, &mut extensions, &display),
                None => handled = false,
            },
            "length" => match integer(0) {
                Some(value) => {
                    set_keyword(
                        schema,
                        "minLength",
                        value.clone(),
                        &mut extensions,
                        &display,
                    );
                    set_keyword(schema, "maxLength", value, &mut extensions, &display);
                }
                None => handled = false,
            },
            "notEmpty" => set_keyword(schema, "minLength", json!(1), &mut extensions, &display),
            "notBlank" => set_keyword(schema, "pattern", json!("\\S"), &mut extensions, &display),
            "startsWith" => match string_arg_value(args, 0) {
                Some(prefix) => set_keyword(
                    schema,
                    "pattern",
                    json!(format!("^{}", regex::escape(prefix))),
                    &mut extensions,
                    &display,
                ),
                None => handled = false,
            },
            "endsWith" => match string_arg_value(args, 0) {
                Some(suffix) => set_keyword(
                    schema,
                    "pattern",
                    json!(format!("{}$", regex::escape(suffix))),
                    &mut extensions,
                    &display,
                ),
                None => handled = false,
            },
            "includes" => match string_arg_value(args, 0) {
                Some(needle) => set_keyword(
                    schema,
                    "pattern",
                    json!(regex::escape(needle)),
                    &mut extensions,
                    &display,
                ),
                None => handled = false,
            },
            "regex" => match args.first().map(super::ast::Expr::kind) {
                Some(ExprKind::RegexLit { pattern, flags }) if flags.is_empty() => {
                    set_keyword(schema, "pattern", json!(pattern), &mut extensions, &display);
                }
                _ => handled = false,
            },
            "email" => set_keyword(schema, "format", json!("email"), &mut extensions, &display),
            "url" => set_keyword(schema, "format", json!("uri"), &mut extensions, &display),
            "uuid" => set_keyword(schema, "format", json!("uuid"), &mut extensions, &display),
            "ipv4" => set_keyword(schema, "format", json!("ipv4"), &mut extensions, &display),
            "ipv6" => set_keyword(schema, "format", json!("ipv6"), &mut extensions, &display),
            "minItems" => match integer(0) {
                Some(value) => set_keyword(schema, "minItems", value, &mut extensions, &display),
                None => handled = false,
            },
            "maxItems" => match integer(0) {
                Some(value) => set_keyword(schema, "maxItems", value, &mut extensions, &display),
                None => handled = false,
            },
            "lengthItems" => match integer(0) {
                Some(value) => {
                    set_keyword(schema, "minItems", value.clone(), &mut extensions, &display);
                    set_keyword(schema, "maxItems", value, &mut extensions, &display);
                }
                None => handled = false,
            },
            "notEmptyItems" => set_keyword(schema, "minItems", json!(1), &mut extensions, &display),
            "unique" => set_keyword(
                schema,
                "uniqueItems",
                json!(true),
                &mut extensions,
                &display,
            ),
            "in" => match literal_array_arg(args, 0)
                .and_then(|values| values.iter().map(literal_value).collect::<Option<Vec<_>>>())
            {
                Some(values) => set_keyword(
                    schema,
                    "enum",
                    Value::Array(values),
                    &mut extensions,
                    &display,
                ),
                None => handled = false,
            },
            "notIn" => match literal_array_arg(args, 0)
                .and_then(|values| values.iter().map(literal_value).collect::<Option<Vec<_>>>())
            {
                Some(values) => set_keyword(
                    schema,
                    "not",
                    json!({ "enum": values }),
                    &mut extensions,
                    &display,
                ),
                None => handled = false,
            },
            "equal" => match primitive_arg_expr(args, 0).as_ref().and_then(literal_value) {
                Some(value) => set_keyword(schema, "const", value, &mut extensions, &display),
                None => handled = false,
            },
            "notEqual" => match primitive_arg_expr(args, 0).as_ref().and_then(literal_value) {
                Some(value) => set_keyword(
                    schema,
                    "not",
                    json!({ "const": value }),
                    &mut extensions,
                    &display,
                ),
                None => handled = false,
            },
            "required" | "optional" | "notNull" => {}
            _ => handled = false,
        }
        if !handled {
            extensions.push(Value::String(display));
        }
    }
    if !extensions.is_empty() {
        schema["x-tyrus-validator"] = Value::Array(extensions);
    }
}

fn enum_schema(info: &ResolvedEnum) -> Value {
    let values: Vec<Value> = info
        .members
        .iter()
        .filter_map(|(_, value)| match value.kind() {
            ExprKind::StrLit(text) => Some(Value::String(text.clone())),
            ExprKind::IntLit(value) => Some(json!(*value as i64)),
            ExprKind::UIntLit(value) => Some(json!(*value as u64)),
            ExprKind::BoolLit(value) => Some(json!(value)),
            _ => None,
        })
        .collect();
    let kind = match info.repr {
        Type::Str => "string",
        Type::Bool => "boolean",
        Type::F32 | Type::F64 => "number",
        Type::Decimal => "string",
        _ => "integer",
    };
    let mut schema = json!({ "type": kind });
    if !values.is_empty() && values.len() == info.members.len() {
        schema["enum"] = Value::Array(values);
    }
    schema
}

fn status_description(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        413 => "Content Too Large",
        415 => "Unsupported Media Type",
        422 => "Unprocessable Content",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "Response",
    }
}

pub fn route_fragment(route: &RouteDescription, tables: &SchemaTables) -> String {
    let mut describer = Describer {
        tables,
        components: BTreeMap::new(),
    };
    let mut parameters = Vec::new();

    let params_schema = route
        .request
        .iter()
        .find(|(part, _)| *part == "params")
        .map(|(_, schema)| describer.fields_of(schema))
        .unwrap_or_default();
    for name in path_parameter_names(route.path) {
        let schema = params_schema
            .iter()
            .find(|field| field.name == name)
            .map(|field| field.ty.clone())
            .unwrap_or(Type::Str);
        let schema = describer.schema(&schema);
        parameters.push(json!({ "name": name, "in": "path", "required": true, "schema": schema }));
    }

    for (part, location) in [
        ("query", "query"),
        ("headers", "header"),
        ("cookies", "cookie"),
    ] {
        let Some((_, schema)) = route.request.iter().find(|(name, _)| *name == part) else {
            continue;
        };
        for field in describer.fields_of(schema) {
            let required = field_required(&field);
            let schema = describer.field_schema(&field);
            parameters.push(json!({
                "name": field.name,
                "in": location,
                "required": required,
                "schema": schema,
            }));
        }
    }

    let mut operation = Map::new();
    if !parameters.is_empty() {
        operation.insert("parameters".to_string(), Value::Array(parameters));
    }
    if let Some((_, schema)) = route.request.iter().find(|(part, _)| *part == "body")
        && let Some(content) = describer.content(&Type::Named((*schema).to_string()))
    {
        operation.insert(
            "requestBody".to_string(),
            json!({ "required": true, "content": content }),
        );
    }

    let mut responses = Map::new();
    for (status, ty) in &route.responses {
        let mut response = json!({ "description": status_description(*status) });
        if let Some(content) = describer.content(ty) {
            response["content"] = content;
        }
        responses.insert(status.to_string(), response);
    }
    if responses.is_empty() {
        responses.insert("default".to_string(), json!({ "description": "Response" }));
    }
    operation.insert("responses".to_string(), Value::Object(responses));
    if !route.tags.is_empty() {
        operation.insert("tags".to_string(), json!(route.tags));
    }

    json!({
        "path": openapi_path(route.path),
        "method": route.method,
        "operation": operation,
        "schemas": describer.components,
    })
    .to_string()
}

pub fn docs_auth_description(value: &Expr) -> Option<String> {
    let ExprKind::ObjectLit(fields) = value.kind() else {
        return None;
    };
    let text = |expr: &Expr| match expr.kind() {
        ExprKind::StrLit(text) => Some(Value::String(text.clone())),
        _ => None,
    };
    let object = |expr: &Expr| -> Option<Map<String, Value>> {
        let ExprKind::ObjectLit(entries) = expr.kind() else {
            return None;
        };
        let mut object = Map::new();
        for (key, entry) in entries {
            let value = match entry.kind() {
                ExprKind::ArrayLit(items) => Value::Array(items.iter().filter_map(text).collect()),
                _ => text(entry)?,
            };
            object.insert(key.clone(), value);
        }
        Some(object)
    };
    let mut auth = Map::new();
    let bearer = fields
        .iter()
        .any(|(key, entry)| key == "bearer" && matches!(entry.kind(), ExprKind::BoolLit(true)));
    if bearer {
        let mut scheme = Map::new();
        if let Some(format) = fields
            .iter()
            .find(|(key, _)| key == "bearerFormat")
            .and_then(|(_, entry)| text(entry))
        {
            scheme.insert("format".to_string(), format);
        }
        auth.insert("bearer".to_string(), Value::Object(scheme));
    }
    for scheme in ["apiKey", "oauth2"] {
        if let Some(entry) = fields
            .iter()
            .find(|(key, _)| key == scheme)
            .and_then(|(_, entry)| object(entry))
        {
            auth.insert(scheme.to_string(), Value::Object(entry));
        }
    }
    (!auth.is_empty()).then(|| Value::Object(auth).to_string())
}

pub fn websocket_fragment(path: &str) -> String {
    let parameters = path_parameter_names(path)
        .into_iter()
        .map(|name| json!({ "name": name, "in": "path", "required": true, "schema": { "type": "string" } }))
        .collect::<Vec<_>>();
    let mut operation = Map::new();
    if !parameters.is_empty() {
        operation.insert("parameters".to_string(), Value::Array(parameters));
    }
    operation.insert(
        "responses".to_string(),
        json!({ "101": { "description": "Switching Protocols" } }),
    );
    operation.insert("x-tyrus-websocket".to_string(), Value::Bool(true));
    json!({
        "path": openapi_path(path),
        "method": "get",
        "operation": operation,
        "schemas": {},
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Expr;
    use crate::span::Span;

    #[test]
    fn an_auth_literal_becomes_the_description_the_runtime_reads() {
        let string = |text: &str| Expr::synthetic(ExprKind::StrLit(text.to_string()));
        let auth = Expr::synthetic(ExprKind::ObjectLit(vec![
            (
                "bearer".to_string(),
                Expr::synthetic(ExprKind::BoolLit(true)),
            ),
            ("bearerFormat".to_string(), string("JWT")),
            (
                "oauth2".to_string(),
                Expr::synthetic(ExprKind::ObjectLit(vec![
                    (
                        "tokenUrl".to_string(),
                        string("https://auth.example.com/token"),
                    ),
                    (
                        "scopes".to_string(),
                        Expr::synthetic(ExprKind::ArrayLit(vec![string("read"), string("write")])),
                    ),
                ])),
            ),
        ]));
        let description: Value =
            serde_json::from_str(&docs_auth_description(&auth).unwrap()).unwrap();
        assert_eq!(description["bearer"]["format"], "JWT");
        assert_eq!(
            description["oauth2"]["tokenUrl"],
            "https://auth.example.com/token"
        );
        assert_eq!(description["oauth2"]["scopes"], json!(["read", "write"]));
        assert!(description.get("apiKey").is_none());
    }

    #[test]
    fn a_websocket_route_is_described_as_the_upgrade() {
        let fragment: Value = serde_json::from_str(&websocket_fragment("/rooms/:room")).unwrap();
        assert_eq!(fragment["path"], "/rooms/{room}");
        assert_eq!(fragment["method"], "get");
        assert_eq!(fragment["operation"]["x-tyrus-websocket"], true);
        assert_eq!(
            fragment["operation"]["responses"]["101"]["description"],
            "Switching Protocols"
        );
        assert_eq!(fragment["operation"]["parameters"][0]["name"], "room");
        assert_eq!(fragment["operation"]["parameters"][0]["in"], "path");
    }

    #[test]
    fn a_multipart_body_describes_its_parts_as_binary() {
        let interfaces = HashMap::default();
        let enums = HashMap::default();
        let tables = SchemaTables {
            interfaces: &interfaces,
            enums: &enums,
        };
        let fragment = route_fragment(
            &RouteDescription {
                method: "post",
                path: "/upload",
                tags: Vec::new(),
                request: vec![("body", MULTIPART_BODY)],
                responses: vec![(204, Type::Void)],
            },
            &tables,
        );
        let value: Value = serde_json::from_str(&fragment).unwrap();
        let schema = &value["operation"]["requestBody"]["content"]["multipart/form-data"]["schema"];
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["additionalProperties"]["format"], "binary");
    }

    fn field(name: &str, ty: Type, optional: bool) -> Field {
        Field {
            name: name.to_string(),
            ty,
            readonly: false,
            optional,
            decorators: Vec::new(),
            span: Span::DUMMY,
        }
    }

    #[test]
    fn a_route_path_uses_openapi_braces_for_its_parameters() {
        assert_eq!(
            openapi_path("/users/:id/posts/:post"),
            "/users/{id}/posts/{post}"
        );
        assert_eq!(openapi_path("/health"), "/health");
    }

    #[test]
    fn a_route_is_described_from_its_declared_contract() {
        let mut interfaces = HashMap::default();
        interfaces.insert(
            "UserParams".to_string(),
            vec![field("id", Type::U64, false)],
        );
        interfaces.insert(
            "UserQuery".to_string(),
            vec![field("verbose", Type::Optional(Box::new(Type::Bool)), true)],
        );
        interfaces.insert(
            "UserResponse".to_string(),
            vec![
                field("name", Type::Str, false),
                field("role", Type::Named("Role".to_string()), false),
                field("tags", Type::Array(Box::new(Type::Str)), false),
            ],
        );
        let mut enums = HashMap::default();
        enums.insert(
            "Role".to_string(),
            ResolvedEnum {
                repr: Type::Str,
                nominal: false,
                members: vec![
                    (
                        "Admin".to_string(),
                        Expr::synthetic(ExprKind::StrLit("admin".to_string())),
                    ),
                    (
                        "Guest".to_string(),
                        Expr::synthetic(ExprKind::StrLit("guest".to_string())),
                    ),
                ],
                member_kinds: Vec::new(),
                is_algebraic: false,
            },
        );
        let tables = SchemaTables {
            interfaces: &interfaces,
            enums: &enums,
        };
        let fragment = route_fragment(
            &RouteDescription {
                method: "get",
                path: "/users/:id",
                tags: Vec::new(),
                request: vec![("params", "UserParams"), ("query", "UserQuery")],
                responses: vec![
                    (200, Type::Named("UserResponse".to_string())),
                    (404, Type::Named(TEXT_BODY.to_string())),
                ],
            },
            &tables,
        );
        let value: Value = serde_json::from_str(&fragment).unwrap();

        assert_eq!(value["path"], "/users/{id}");
        assert_eq!(value["method"], "get");
        let parameters = value["operation"]["parameters"].as_array().unwrap();
        assert_eq!(parameters[0]["name"], "id");
        assert_eq!(parameters[0]["in"], "path");
        assert_eq!(parameters[0]["schema"]["format"], "int64");
        assert_eq!(parameters[1]["name"], "verbose");
        assert_eq!(parameters[1]["in"], "query");
        assert_eq!(parameters[1]["required"], false);

        let ok = &value["operation"]["responses"]["200"];
        assert_eq!(ok["description"], "OK");
        assert_eq!(
            ok["content"]["application/json"]["schema"]["$ref"],
            "#/components/schemas/UserResponse"
        );
        assert_eq!(
            value["operation"]["responses"]["404"]["content"]["text/plain"]["schema"]["type"],
            "string"
        );

        let user = &value["schemas"]["UserResponse"];
        assert_eq!(user["required"], json!(["name", "role", "tags"]));
        assert_eq!(user["properties"]["tags"]["items"]["type"], "string");
        assert_eq!(
            user["properties"]["role"]["$ref"],
            "#/components/schemas/Role"
        );
        assert_eq!(value["schemas"]["Role"]["enum"], json!(["admin", "guest"]));
    }

    #[test]
    fn a_request_body_is_described_by_its_content_type() {
        let interfaces = HashMap::default();
        let enums = HashMap::default();
        let tables = SchemaTables {
            interfaces: &interfaces,
            enums: &enums,
        };
        let describe = |schema: &str| -> Value {
            let fragment = route_fragment(
                &RouteDescription {
                    method: "post",
                    path: "/upload",
                    tags: Vec::new(),
                    request: vec![("body", schema)],
                    responses: vec![(204, Type::Void)],
                },
                &tables,
            );
            serde_json::from_str(&fragment).unwrap()
        };
        assert!(
            describe(BYTES_BODY)["operation"]["requestBody"]["content"]
                .get("application/octet-stream")
                .is_some()
        );
        assert!(
            describe(MULTIPART_BODY)["operation"]["requestBody"]["content"]
                .get("multipart/form-data")
                .is_some()
        );
        let no_content = describe(TEXT_BODY);
        assert!(
            no_content["operation"]["responses"]["204"]
                .get("content")
                .is_none()
        );
    }

    #[test]
    fn error_response_documents_its_required_code_field() {
        let mut interfaces = HashMap::default();
        interfaces.insert(
            "ApiError".to_string(),
            vec![
                field("code", Type::Str, false),
                field("message", Type::Str, false),
            ],
        );
        let enums = HashMap::default();
        let fragment = route_fragment(
            &RouteDescription {
                method: "get",
                path: "/users/:id",
                tags: Vec::new(),
                request: Vec::new(),
                responses: vec![(404, Type::Named("ApiError".to_string()))],
            },
            &SchemaTables {
                interfaces: &interfaces,
                enums: &enums,
            },
        );
        let value: Value = serde_json::from_str(&fragment).unwrap();
        let schema =
            &value["operation"]["responses"]["404"]["content"]["application/json"]["schema"];
        assert_eq!(schema["$ref"], "#/components/schemas/ApiError");
        assert_eq!(
            value["schemas"]["ApiError"]["properties"]["code"]["type"],
            "string"
        );
        assert_eq!(
            value["schemas"]["ApiError"]["required"],
            json!(["code", "message"])
        );
    }

    #[test]
    fn a_schema_that_refers_to_itself_ends_at_its_reference() {
        let mut interfaces = HashMap::default();
        interfaces.insert(
            "Node".to_string(),
            vec![
                field("value", Type::I32, false),
                field(
                    "next",
                    Type::Optional(Box::new(Type::Named("Node".to_string()))),
                    true,
                ),
            ],
        );
        let enums = HashMap::default();
        let tables = SchemaTables {
            interfaces: &interfaces,
            enums: &enums,
        };
        let fragment = route_fragment(
            &RouteDescription {
                method: "get",
                path: "/list",
                tags: Vec::new(),
                request: Vec::new(),
                responses: vec![(200, Type::Named("Node".to_string()))],
            },
            &tables,
        );
        let value: Value = serde_json::from_str(&fragment).unwrap();
        assert_eq!(
            value["schemas"]["Node"]["properties"]["next"]["$ref"],
            "#/components/schemas/Node"
        );
        assert_eq!(value["schemas"]["Node"]["required"], json!(["value"]));
    }

    #[test]
    fn a_schema_is_named_as_the_program_wrote_it() {
        let mut interfaces = HashMap::default();
        interfaces.insert(
            "__tyrus_module_0_UserResponse".to_string(),
            vec![field("name", Type::Str, false)],
        );
        let enums = HashMap::default();
        let tables = SchemaTables {
            interfaces: &interfaces,
            enums: &enums,
        };
        let fragment = route_fragment(
            &RouteDescription {
                method: "get",
                path: "/users",
                tags: Vec::new(),
                request: Vec::new(),
                responses: vec![(
                    200,
                    Type::Named("__tyrus_module_0_UserResponse".to_string()),
                )],
            },
            &tables,
        );
        let value: Value = serde_json::from_str(&fragment).unwrap();
        assert!(value["schemas"].get("UserResponse").is_some());
        assert_eq!(
            value["operation"]["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
            "#/components/schemas/UserResponse"
        );
    }

    fn decorator(name: &str, args: Vec<Expr>) -> ValidatorDecorator {
        ValidatorDecorator {
            name: name.to_string(),
            display_name: name.to_string(),
            args,
            argument_ranges: Vec::new(),
            span: Span::DUMMY,
        }
    }

    fn decorated(name: &str, ty: Type, decorators: Vec<ValidatorDecorator>) -> Field {
        Field {
            decorators,
            ..field(name, ty, false)
        }
    }

    #[test]
    fn validators_become_json_schema_keywords_or_named_extensions() {
        let int = |value: i128| Expr::synthetic(ExprKind::IntLit(value));
        let text = |value: &str| Expr::synthetic(ExprKind::StrLit(value.to_string()));
        let mut interfaces = HashMap::default();
        interfaces.insert(
            "CreateUser".to_string(),
            vec![
                decorated(
                    "name",
                    Type::Str,
                    vec![
                        decorator("minLength", vec![int(2)]),
                        decorator("maxLength", vec![int(40)]),
                    ],
                ),
                decorated("email", Type::Str, vec![decorator("email", Vec::new())]),
                decorated(
                    "age",
                    Type::I32,
                    vec![decorator(
                        "range",
                        vec![Expr::synthetic(ExprKind::Neg(Box::new(int(1)))), int(130)],
                    )],
                ),
                decorated("document", Type::Str, vec![decorator("cpf", Vec::new())]),
                decorated(
                    "role",
                    Type::Str,
                    vec![decorator(
                        "in",
                        vec![Expr::synthetic(ExprKind::ArrayLit(vec![
                            text("admin"),
                            text("user"),
                        ]))],
                    )],
                ),
                decorated(
                    "tags",
                    Type::Array(Box::new(Type::Str)),
                    vec![
                        decorator("unique", Vec::new()),
                        decorator("maxItems", vec![int(5)]),
                    ],
                ),
                decorated(
                    "nickname",
                    Type::Optional(Box::new(Type::Str)),
                    vec![decorator("required", Vec::new())],
                ),
            ],
        );
        let enums = HashMap::default();
        let tables = SchemaTables {
            interfaces: &interfaces,
            enums: &enums,
        };
        let fragment = route_fragment(
            &RouteDescription {
                method: "post",
                path: "/users",
                tags: Vec::new(),
                request: vec![("body", "CreateUser")],
                responses: vec![(201, Type::Void)],
            },
            &tables,
        );
        let value: Value = serde_json::from_str(&fragment).unwrap();
        let schema = &value["schemas"]["CreateUser"];
        let properties = &schema["properties"];
        assert_eq!(properties["name"]["minLength"], 2);
        assert_eq!(properties["name"]["maxLength"], 40);
        assert_eq!(properties["email"]["format"], "email");
        assert_eq!(properties["age"]["minimum"], -1.0);
        assert_eq!(properties["age"]["maximum"], 130.0);
        assert_eq!(
            properties["document"]["x-tyrus-validator"],
            json!(["@cpf()"])
        );
        assert_eq!(properties["role"]["enum"], json!(["admin", "user"]));
        assert_eq!(properties["tags"]["uniqueItems"], true);
        assert_eq!(properties["tags"]["maxItems"], 5);
        let required: Vec<&str> = schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert!(
            required.contains(&"nickname"),
            "`@required` makes an optional field required"
        );
    }
}
