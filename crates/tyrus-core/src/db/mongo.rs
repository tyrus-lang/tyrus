use crate::ast::{DbModelFieldMetadata, DbModelMetadata, Expr, ExprKind, Field, Type};
use crate::collections::HashMap;
use serde_json::{Map, Value, json};
use std::collections::HashSet;

#[derive(Clone, Copy)]
pub enum MongoReadKind {
    FindOne,
    FindMany,
    Count,
}

pub struct MongoModelRead {
    pub descriptor: String,
    pub parameters: Vec<Expr>,
    pub parameter_fields: Vec<String>,
}

pub struct MongoModelMutation {
    pub descriptor: String,
    pub parameters: Vec<Expr>,
    pub parameter_fields: Vec<String>,
}

pub fn mongo_model_result_type(model_fields: &[Field]) -> Type {
    let mut fields = model_fields.to_vec();
    if !fields.iter().any(|field| field.name == "_id") {
        fields.push(Field {
            name: "_id".to_string(),
            ty: Type::Named("ObjectId".to_string()),
            readonly: true,
            optional: false,
            decorators: Vec::new(),
            span: crate::span::Span::DUMMY,
        });
    }
    Type::Object(fields)
}

pub fn mongo_model_mutation_result_type(method: &str, model_fields: &[Field]) -> Option<Type> {
    let row = mongo_model_result_type(model_fields);
    match method {
        "insertOne" => Some(row),
        "insertMany" => Some(Type::Array(Box::new(row))),
        "updateOne" | "deleteOne" => Some(Type::Optional(Box::new(row))),
        "updateMany" | "deleteMany" => Some(Type::U64),
        _ => None,
    }
}

pub fn mongo_model_read(
    model: &DbModelMetadata,
    filter: &Expr,
    kind: MongoReadKind,
    interfaces: &HashMap<String, Vec<Field>>,
) -> Result<MongoModelRead, String> {
    let mut parameters = Vec::new();
    let mut parameter_fields = Vec::new();
    let encoded_filter = encode_filter(model, filter, &mut parameters)?;
    collect_filter_parameter_fields(model, filter, &mut parameter_fields)?;
    let fields = if matches!(kind, MongoReadKind::Count) {
        Vec::new()
    } else {
        let mut fields = model
            .fields
            .iter()
            .map(|field| {
                json!({
                    "logical": field.field,
                    "document": mongo_field_name(field),
                    "objectId": field_uses_object_id(field),
                    "objectIdArray": field_uses_object_id_array(field),
                    "objectIdArrayNullableItems": mongo_object_id_array_nullable_items(&field.ty),
                    "kind": mongo_bson_kind(&field.ty),
                    "schema": mongo_bson_schema(&field.ty, interfaces),
                    "nullable": mongo_type_is_optional(&field.ty),
                })
            })
            .collect::<Vec<_>>();
        if !model.fields.iter().any(|field| field.field == "_id") {
            fields.push(json!({ "logical": "_id", "document": "_id", "objectId": true, "kind": "objectId", "schema": { "kind": "objectId", "nullable": false }, "nullable": false }));
        }
        fields
    };
    let kind = match kind {
        MongoReadKind::FindOne => "findOne",
        MongoReadKind::FindMany => "findMany",
        MongoReadKind::Count => "count",
    };
    let descriptor = json!({
        "kind": kind,
        "collection": model.table,
        "filter": encoded_filter,
        "fields": fields,
    });
    let descriptor = serde_json::to_string(&descriptor)
        .map_err(|_| "could not encode MongoDB model operation".to_string())?;
    Ok(MongoModelRead {
        descriptor,
        parameters,
        parameter_fields,
    })
}

pub fn mongo_model_mutation(
    operation: &str,
    model: &DbModelMetadata,
    arguments: &[Expr],
    interfaces: &HashMap<String, Vec<Field>>,
) -> Result<MongoModelMutation, String> {
    validate_mutation_arguments(operation, model, arguments)?;
    let mut parameters = Vec::new();
    let mut parameter_fields = Vec::new();
    let mut descriptor = serde_json::Map::new();
    descriptor.insert("kind".to_string(), Value::String(operation.to_string()));
    descriptor.insert("collection".to_string(), Value::String(model.table.clone()));
    descriptor.insert("timestamps".to_string(), Value::Bool(model.timestamps));
    if model.timestamps {
        let timestamp_fields = model
            .fields
            .iter()
            .filter(|field| matches!(field.field.as_str(), "createdAt" | "updatedAt"))
            .map(|field| (field.field.clone(), Value::String(field.column.clone())))
            .collect::<serde_json::Map<_, _>>();
        descriptor.insert(
            "timestampFields".to_string(),
            Value::Object(timestamp_fields),
        );
    }
    descriptor.insert(
        "fieldTypes".to_string(),
        Value::Array(
            model
                .fields
                .iter()
                .map(|field| {
                    json!({
                        "name": mongo_field_name(field),
                        "id": field.id,
                        "kind": mongo_bson_kind(&field.ty),
                        "schema": mongo_bson_schema(&field.ty, interfaces),
                        "nullable": mongo_type_is_optional(&field.ty),
                        "nullableItems": mongo_object_id_array_nullable_items(&field.ty),
                        "required": !field.id
                            && field.default_value.is_none()
                            && !(model.timestamps && matches!(field.field.as_str(), "createdAt" | "updatedAt"))
                            && !mongo_type_is_optional(&field.ty),
                    })
                })
                .collect(),
        ),
    );
    descriptor.insert(
        "returnFields".to_string(),
        Value::Array(
            model
                .fields
                .iter()
                .map(|field| {
                    json!({
                        "logical": field.field,
                        "document": mongo_field_name(field),
                        "objectId": field_uses_object_id(field),
                        "objectIdArray": field_uses_object_id_array(field),
                        "objectIdArrayNullableItems": mongo_object_id_array_nullable_items(&field.ty),
                        "kind": mongo_bson_kind(&field.ty),
                        "schema": mongo_bson_schema(&field.ty, interfaces),
                        "nullable": mongo_type_is_optional(&field.ty),
                    })
                })
                .chain((!model.fields.iter().any(|field| field.field == "_id")).then_some(
                    json!({
                        "logical": "_id",
                        "document": "_id",
                        "objectId": true,
                        "kind": "objectId",
                        "schema": { "kind": "objectId", "nullable": false },
                        "nullable": false,
                    }),
                ))
                .collect(),
        ),
    );
    match operation {
        "insertOne" => {
            collect_document_parameter_fields(model, &arguments[0], &mut parameter_fields)?;
            descriptor.insert(
                "document".to_string(),
                encode_document(model, &arguments[0], &mut parameters)?,
            );
        }
        "insertMany" => {
            let ExprKind::ArrayLit(rows) = arguments[0].kind() else {
                return Err("`insertMany` requires an array of model objects".to_string());
            };
            let documents = rows
                .iter()
                .map(|row| {
                    collect_document_parameter_fields(model, row, &mut parameter_fields)?;
                    encode_document(model, row, &mut parameters)
                })
                .collect::<Result<Vec<_>, _>>()?;
            descriptor.insert("documents".to_string(), Value::Array(documents));
        }
        "updateOne" | "updateMany" => {
            collect_filter_parameter_fields(model, &arguments[0], &mut parameter_fields)?;
            collect_document_parameter_fields(model, &arguments[1], &mut parameter_fields)?;
            descriptor.insert(
                "filter".to_string(),
                encode_filter(model, &arguments[0], &mut parameters)?,
            );
            descriptor.insert(
                "changes".to_string(),
                encode_document(model, &arguments[1], &mut parameters)?,
            );
        }
        "deleteOne" | "deleteMany" => {
            collect_filter_parameter_fields(model, &arguments[0], &mut parameter_fields)?;
            descriptor.insert(
                "filter".to_string(),
                encode_filter(model, &arguments[0], &mut parameters)?,
            );
        }
        _ => return Err(format!("unsupported MongoDB model mutation `{operation}`")),
    }
    let descriptor = serde_json::to_string(&Value::Object(descriptor))
        .map_err(|_| "could not encode MongoDB model mutation".to_string())?;
    Ok(MongoModelMutation {
        descriptor,
        parameters,
        parameter_fields,
    })
}

fn validate_mutation_arguments(
    operation: &str,
    model: &DbModelMetadata,
    arguments: &[Expr],
) -> Result<(), String> {
    let expected = match operation {
        "insertOne" | "insertMany" | "deleteOne" | "deleteMany" => 1,
        "updateOne" | "updateMany" => 2,
        _ => return Err(format!("unsupported MongoDB model mutation `{operation}`")),
    };
    if arguments.len() != expected {
        return Err(format!(
            "`{operation}` received an invalid number of arguments"
        ));
    }
    match operation {
        "insertOne" => {
            let mut parameters = Vec::new();
            encode_document(model, &arguments[0], &mut parameters)?;
        }
        "insertMany" => {
            let ExprKind::ArrayLit(rows) = arguments[0].kind() else {
                return Err("`insertMany` requires an array of model objects".to_string());
            };
            if rows.is_empty() || rows.len() > super::orm::MAX_BATCH_ROWS {
                return Err(format!(
                    "insert batches must contain between 1 and {} rows",
                    super::orm::MAX_BATCH_ROWS
                ));
            }
            let mut parameters = Vec::new();
            let mut first_fields: Option<Vec<&str>> = None;
            for row in rows {
                let ExprKind::ObjectLit(fields) = row.kind() else {
                    return Err("insert values must be object literals".to_string());
                };
                let names = fields
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .collect::<Vec<_>>();
                if first_fields
                    .as_ref()
                    .is_some_and(|expected| *expected != names)
                {
                    return Err(
                        "every insertMany row must provide the same fields in the same order"
                            .to_string(),
                    );
                }
                first_fields.get_or_insert(names);
                encode_document(model, row, &mut parameters)?;
            }
        }
        "updateOne" | "updateMany" => {
            let mut parameters = Vec::new();
            let filter_fields = filter_fields_for_unique(model, &arguments[0])?;
            if filter_fields.is_empty() {
                return Err("update filters must not be empty".to_string());
            }
            if operation == "updateOne" && !filter_fields.iter().any(|unique| *unique) {
                return Err(
                    "`updateOne` requires equality on `@id`, `@unique`, or `_id`".to_string(),
                );
            }
            encode_filter(model, &arguments[0], &mut parameters)?;
            let ExprKind::ObjectLit(changes) = arguments[1].kind() else {
                return Err("update changes must be an object literal".to_string());
            };
            if model.timestamps {
                if changes.iter().any(|(name, _)| name == "updatedAt") {
                    return Err("Model field `updatedAt` is maintained automatically".to_string());
                }
                if changes.iter().any(|(name, _)| name == "createdAt") {
                    return Err("Model field `createdAt` cannot be changed".to_string());
                }
            }
            if changes.is_empty() {
                return Err("update changes must contain at least one field".to_string());
            }
            if changes.iter().any(|(name, _)| {
                model
                    .fields
                    .iter()
                    .any(|field| field.field == *name && field_is_model_id(field))
                    || name == "_id"
            }) {
                return Err("an update cannot change the model identifier".to_string());
            }
            encode_document(model, &arguments[1], &mut parameters)?;
        }
        "deleteOne" | "deleteMany" => {
            let mut parameters = Vec::new();
            let filter_fields = filter_fields_for_unique(model, &arguments[0])?;
            if filter_fields.is_empty() {
                return Err("delete filters must not be empty".to_string());
            }
            if operation == "deleteOne" && !filter_fields.iter().any(|unique| *unique) {
                return Err(
                    "`deleteOne` requires equality on `@id`, `@unique`, or `_id`".to_string(),
                );
            }
            encode_filter(model, &arguments[0], &mut parameters)?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn filter_fields_for_unique(
    model: &DbModelMetadata,
    expression: &Expr,
) -> Result<Vec<bool>, String> {
    let ExprKind::ObjectLit(fields) = expression.kind() else {
        return Err("MongoDB filters must be object literals".to_string());
    };
    fields
        .iter()
        .map(|(name, value)| {
            if name == "or" {
                return Ok(false);
            }
            if name == "_id" {
                return Ok(true);
            }
            let field = model
                .fields
                .iter()
                .find(|field| field.field == *name)
                .ok_or_else(|| format!("unknown model filter field `{name}`"))?;
            let equality = !matches!(value.kind(), ExprKind::ObjectLit(operators) if operators.len() != 1 || operators[0].0 != "eq");
            Ok((field.id || field.unique) && equality)
        })
        .collect()
}

fn encode_document(
    model: &DbModelMetadata,
    expression: &Expr,
    parameters: &mut Vec<Expr>,
) -> Result<Value, String> {
    let ExprKind::ObjectLit(fields) = expression.kind() else {
        return Err("model mutation values must be object literals".to_string());
    };
    let mut output = Map::new();
    for (name, value) in fields {
        let field = model
            .fields
            .iter()
            .find(|field| field.field == *name)
            .ok_or_else(|| format!("unknown model mutation field `{name}`"))?;
        let encoded = if field_uses_object_id_array(field) {
            parameter_ref_object_id_array(value, parameters)
        } else {
            parameter_ref(value, parameters, field_is_object_id(field))
        };
        if output
            .insert(mongo_field_name(field).to_string(), encoded)
            .is_some()
        {
            return Err("multiple model fields map to the same MongoDB field".to_string());
        }
    }
    Ok(Value::Object(output))
}

fn mongo_field_name(field: &DbModelFieldMetadata) -> &str {
    if field.id { "_id" } else { &field.column }
}

fn mongo_bson_kind(ty: &Type) -> &'static str {
    fn base_type(ty: &Type) -> &Type {
        match ty {
            Type::Optional(inner) | Type::Readonly(inner) => base_type(inner),
            ty => ty,
        }
    }
    match base_type(ty) {
        Type::Bool => "boolean",
        Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::I128
        | Type::Isize
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::U128
        | Type::Usize => "integer",
        Type::F32 | Type::F64 | Type::F128 | Type::Decimal => "number",
        Type::Char | Type::Str | Type::Regex => "string",
        Type::Array(element) if matches!(element.as_ref(), Type::U8) => "bytes",
        Type::Array(element) if field_has_object_id_scalar(element) => "objectIdArray",
        Type::Array(_) | Type::Tuple(_) => "array",
        Type::Object(_) => "document",
        Type::Named(name) if name == "ObjectId" => "objectId",
        Type::Named(name) if name == "Instant" => "instant",
        _ => "any",
    }
}

fn mongo_bson_schema(ty: &Type, interfaces: &HashMap<String, Vec<Field>>) -> Value {
    fn object_fields_schema(
        fields: &[Field],
        interfaces: &HashMap<String, Vec<Field>>,
        definitions: &mut serde_json::Map<String, Value>,
        building: &mut HashSet<String>,
        depth: usize,
    ) -> Value {
        Value::Array(
            fields
                .iter()
                .map(|field| {
                    json!({
                        "name": field.name,
                        "required": !field.optional && !mongo_type_is_optional(&field.ty),
                        "schema": schema_with_definitions(
                            &field.ty,
                            interfaces,
                            definitions,
                            building,
                            depth + 1,
                        ),
                    })
                })
                .collect(),
        )
    }

    fn schema_with_definitions(
        ty: &Type,
        interfaces: &HashMap<String, Vec<Field>>,
        definitions: &mut serde_json::Map<String, Value>,
        building: &mut HashSet<String>,
        depth: usize,
    ) -> Value {
        if depth > 100 {
            return json!({"kind": "any", "nullable": mongo_type_is_optional(ty)});
        }
        let mut base = ty;
        let mut nullable = false;
        loop {
            match base {
                Type::Optional(inner) => {
                    nullable = true;
                    base = inner;
                }
                Type::Readonly(inner) => base = inner,
                _ => break,
            }
        }
        let kind = match base {
            Type::Named(name) if name != "ObjectId" && name != "Instant" => {
                if let Some(fields) = interfaces.get(name) {
                    if !definitions.contains_key(name) && building.insert(name.clone()) {
                        let mut definition = serde_json::Map::new();
                        definition
                            .insert("kind".to_string(), Value::String("document".to_string()));
                        definition.insert("nullable".to_string(), Value::Bool(false));
                        definition.insert(
                            "fields".to_string(),
                            object_fields_schema(
                                fields,
                                interfaces,
                                definitions,
                                building,
                                depth + 1,
                            ),
                        );
                        definitions.insert(name.clone(), Value::Object(definition));
                        building.remove(name);
                    }
                    let mut reference = serde_json::Map::new();
                    reference.insert("kind".to_string(), Value::String("reference".to_string()));
                    reference.insert("name".to_string(), Value::String(name.clone()));
                    reference.insert("nullable".to_string(), Value::Bool(nullable));
                    return Value::Object(reference);
                }
                "any"
            }
            _ => mongo_bson_kind(base),
        };
        let mut schema = serde_json::Map::new();
        schema.insert("kind".to_string(), Value::String(kind.to_string()));
        schema.insert("nullable".to_string(), Value::Bool(nullable));
        match base {
            Type::Array(element) if !matches!(element.as_ref(), Type::U8) => {
                schema.insert(
                    "items".to_string(),
                    schema_with_definitions(element, interfaces, definitions, building, depth + 1),
                );
            }
            Type::Tuple(elements) => {
                schema.insert(
                    "items".to_string(),
                    Value::Array(
                        elements
                            .iter()
                            .map(|element| {
                                schema_with_definitions(
                                    element,
                                    interfaces,
                                    definitions,
                                    building,
                                    depth + 1,
                                )
                            })
                            .collect(),
                    ),
                );
                schema.insert("tuple".to_string(), Value::Bool(true));
            }
            Type::Object(fields) => {
                schema.insert(
                    "fields".to_string(),
                    object_fields_schema(fields, interfaces, definitions, building, depth + 1),
                );
            }
            _ => {}
        }
        Value::Object(schema)
    }
    let mut definitions = serde_json::Map::new();
    let schema = schema_with_definitions(ty, interfaces, &mut definitions, &mut HashSet::new(), 0);
    if !definitions.is_empty()
        && let Value::Object(mut schema) = schema
    {
        schema.insert("definitions".to_string(), Value::Object(definitions));
        return Value::Object(schema);
    }
    schema
}

fn mongo_type_is_optional(ty: &Type) -> bool {
    match ty {
        Type::Optional(_) => true,
        Type::Readonly(inner) => mongo_type_is_optional(inner),
        _ => false,
    }
}

fn field_is_object_id(field: &DbModelFieldMetadata) -> bool {
    field_is_model_id(field) || field_uses_object_id(field)
}

fn field_is_model_id(field: &DbModelFieldMetadata) -> bool {
    field.id || field.field == "_id" || field.column == "_id"
}

fn field_uses_object_id(field: &DbModelFieldMetadata) -> bool {
    field_is_model_id(field) || field_has_object_id_scalar(&field.ty)
}

fn field_has_object_id_scalar(ty: &Type) -> bool {
    match ty {
        Type::Named(name) => name == "ObjectId",
        Type::Optional(inner) | Type::Readonly(inner) => field_has_object_id_scalar(inner),
        _ => false,
    }
}

fn field_uses_object_id_array(field: &DbModelFieldMetadata) -> bool {
    fn is_object_id_array(ty: &Type) -> bool {
        match ty {
            Type::Array(inner) => field_has_object_id_scalar(inner),
            Type::Optional(inner) | Type::Readonly(inner) => is_object_id_array(inner),
            _ => false,
        }
    }
    is_object_id_array(&field.ty)
}

fn mongo_object_id_array_nullable_items(ty: &Type) -> bool {
    fn is_optional(ty: &Type) -> bool {
        match ty {
            Type::Optional(_) => true,
            Type::Readonly(inner) => is_optional(inner),
            _ => false,
        }
    }
    match ty {
        Type::Array(element) => is_optional(element),
        Type::Optional(inner) | Type::Readonly(inner) => {
            mongo_object_id_array_nullable_items(inner)
        }
        _ => false,
    }
}

fn collect_document_parameter_fields(
    model: &DbModelMetadata,
    expression: &Expr,
    output: &mut Vec<String>,
) -> Result<(), String> {
    let ExprKind::ObjectLit(fields) = expression.kind() else {
        return Err("model mutation values must be object literals".to_string());
    };
    for (name, value) in fields {
        let field = model
            .fields
            .iter()
            .find(|field| field.field == *name)
            .ok_or_else(|| format!("unknown model mutation field `{name}`"))?;
        let _ = value;
        output.push(field.field.clone());
    }
    Ok(())
}

fn collect_filter_parameter_fields(
    model: &DbModelMetadata,
    expression: &Expr,
    output: &mut Vec<String>,
) -> Result<(), String> {
    let ExprKind::ObjectLit(fields) = expression.kind() else {
        return Err("MongoDB filters must be object literals".to_string());
    };
    for (name, value) in fields {
        if name == "or" {
            let ExprKind::ArrayLit(alternatives) = value.kind() else {
                return Err("the `or` filter must be an array of filter objects".to_string());
            };
            for alternative in alternatives {
                collect_filter_parameter_fields(model, alternative, output)?;
            }
            continue;
        }
        let field_name = if name == "_id" {
            "_id"
        } else {
            &model
                .fields
                .iter()
                .find(|field| field.field == *name)
                .ok_or_else(|| format!("unknown model filter field `{name}`"))?
                .field
        };
        if let ExprKind::ObjectLit(operators) = value.kind() {
            for (operator, operand) in operators {
                if operator == "in" {
                    let ExprKind::ArrayLit(values) = operand.kind() else {
                        return Err("the `in` filter operator requires an array".to_string());
                    };
                    output.extend(values.iter().map(|_| field_name.to_string()));
                } else {
                    output.push(field_name.to_string());
                }
            }
        } else {
            output.push(field_name.to_string());
        }
    }
    Ok(())
}

fn encode_filter(
    model: &DbModelMetadata,
    expression: &Expr,
    parameters: &mut Vec<Expr>,
) -> Result<Value, String> {
    let ExprKind::ObjectLit(fields) = expression.kind() else {
        return Err("MongoDB filters must be object literals".to_string());
    };
    let mut output = Map::new();
    for (name, value) in fields {
        if name == "or" {
            let ExprKind::ArrayLit(alternatives) = value.kind() else {
                return Err("the `or` filter must be an array of filter objects".to_string());
            };
            if alternatives.is_empty() {
                return Err("the `or` filter must contain at least one alternative".to_string());
            }
            let alternatives = alternatives
                .iter()
                .map(|alternative| encode_filter(model, alternative, parameters))
                .collect::<Result<Vec<_>, _>>()?;
            output.insert("$or".to_string(), Value::Array(alternatives));
            continue;
        }
        let field = model.fields.iter().find(|field| field.field == *name);
        let (document_name, condition) = if let Some(field) = field {
            (
                mongo_field_name(field).to_string(),
                encode_condition(
                    value,
                    parameters,
                    field_is_object_id(field),
                    field_uses_object_id_array(field),
                )?,
            )
        } else if name == "_id" {
            (
                "_id".to_string(),
                encode_condition(value, parameters, true, false)?,
            )
        } else {
            return Err(format!("unknown model filter field `{name}`"));
        };
        if output.insert(document_name, condition).is_some() {
            return Err("multiple model fields map to the same MongoDB field".to_string());
        }
    }
    Ok(Value::Object(output))
}

fn encode_condition(
    value: &Expr,
    parameters: &mut Vec<Expr>,
    object_id: bool,
    object_id_array: bool,
) -> Result<Value, String> {
    if let ExprKind::ObjectLit(operators) = value.kind() {
        if operators.is_empty() {
            return Err("comparison filters must contain at least one operator".to_string());
        }
        if (object_id || object_id_array)
            && operators.iter().any(|(operator, _)| operator == "like")
        {
            return Err("MongoDB ObjectId filters do not support `like`".to_string());
        }
        let mut output = Map::new();
        for (operator, operand) in operators {
            let mongo_operator = match operator.as_str() {
                "eq" => "$eq",
                "ne" => "$ne",
                "gt" => "$gt",
                "gte" => "$gte",
                "lt" => "$lt",
                "lte" => "$lte",
                "in" => "$in",
                "like" => "$tyrusLike",
                _ => return Err(format!("unsupported filter operator `{operator}`")),
            };
            let encoded = if operator == "in" {
                let ExprKind::ArrayLit(values) = operand.kind() else {
                    return Err("the `in` filter operator requires an array".to_string());
                };
                if values.is_empty() {
                    return Err("the `in` filter operator requires at least one value".to_string());
                }
                Value::Array(
                    values
                        .iter()
                        .map(|value| parameter_ref(value, parameters, object_id || object_id_array))
                        .collect(),
                )
            } else if object_id_array {
                if !matches!(operator.as_str(), "eq" | "ne") {
                    return Err(
                        "ObjectId array filters support only `eq`, `ne`, and `in`".to_string()
                    );
                }
                parameter_ref_object_id_array(operand, parameters)
            } else {
                parameter_ref(operand, parameters, object_id)
            };
            output.insert(mongo_operator.to_string(), encoded);
        }
        Ok(Value::Object(output))
    } else {
        if object_id_array {
            Ok(parameter_ref_object_id_array(value, parameters))
        } else {
            Ok(parameter_ref(value, parameters, object_id))
        }
    }
}

fn parameter_ref(value: &Expr, parameters: &mut Vec<Expr>, object_id: bool) -> Value {
    let index = parameters.len();
    parameters.push(value.clone());
    if object_id {
        json!({"$tyrusObjectIdParam": index})
    } else {
        json!({"$tyrusParam": index})
    }
}

fn parameter_ref_object_id_array(value: &Expr, parameters: &mut Vec<Expr>) -> Value {
    let index = parameters.len();
    parameters.push(value.clone());
    json!({"$tyrusObjectIdArrayParam": index})
}
