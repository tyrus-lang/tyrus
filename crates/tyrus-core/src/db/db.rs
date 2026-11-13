use crate::ast::{DbModelFieldMetadata, Field, Item, Program, Type, TypeParam};
use crate::collections::HashMap;
use crate::module_loader::ExportInfo;
use crate::span::Span;

#[path = "call.rs"]
mod call;
#[path = "config.rs"]
mod config;
#[path = "exec_sql.rs"]
mod exec_sql;
#[path = "mongo.rs"]
mod mongo;
#[path = "orm.rs"]
mod orm;
#[path = "schema.rs"]
mod schema;
#[path = "sql.rs"]
mod sql;
pub use call::{CapturedSqlCall, SqlCallCaptureError};
pub use config::{DatabaseBackend, DatabaseConfig, DatabasePoolConfig, DatabaseTlsConfig};
pub use exec_sql::{
    SqlExec, SqlExecBinding, SqlExecKind, parse_exec_sql, validate_exec_against_snapshot,
};
pub use mongo::{
    MongoModelMutation, MongoModelRead, MongoReadKind, mongo_model_mutation,
    mongo_model_mutation_result_type, mongo_model_read, mongo_model_result_type,
};
pub use orm::{
    DbRelationInclude, INSTANT_ALIAS_PREFIX, JSON_RELATION_ALIAS_PREFIX, MAX_BATCH_ROWS,
    MAX_RELATION_ROWS, OrmSelect, UNSIGNED_ALIAS_PREFIX, count_query, find_many_cursor_query,
    find_many_page_query, find_many_query, find_many_with_relations_query,
    find_one_by_id_parameter_query, find_one_query, mutation_query, mutation_returning_clause,
    mysql_instant_projection_sql, postgresql_instant_projection_sql, project_model_result_type,
};
pub use schema::{
    DB_SCHEMA_FORMAT_VERSION, DbColumnSchema, DbForeignKeySchema, DbIndexSchema, DbSchemaCache,
    DbSchemaSnapshot, DbTableSchema, DbTriggerSchema,
};
pub use sql::{
    SqlAggregate, SqlDecoderField, SqlGroup, SqlJoin, SqlJoinKind, SqlLiteralType, SqlOrder,
    SqlParseError, SqlPredicate, SqlProjection, SqlSelect, SqlToken, SqlTokenKind,
    ValidatedSqlSelect, aggregate_projection_type, parse_select_sql, select_table_nullability,
    tokenize_sql, validate_select_against_snapshot, validate_select_against_snapshot_with_mappings,
};

pub fn db_error_fields() -> Vec<Field> {
    crate::builtin_interfaces::db_error_fields()
}

pub fn apply_backend_model_defaults(program: &mut Program, backend: Option<&DatabaseBackend>) {
    if !matches!(
        backend,
        Some(DatabaseBackend::Sqlite | DatabaseBackend::Mysql | DatabaseBackend::Postgresql)
    ) {
        return;
    }
    for item in &mut program.items {
        let Item::InterfaceDecl {
            fields,
            db_model: Some(model),
            ..
        } = item
        else {
            continue;
        };
        if model.fields.iter().any(|field| field.id) {
            continue;
        }
        if let Some(field) = fields.iter().find(|field| field.name == "id") {
            if let Some(metadata) = model
                .fields
                .iter_mut()
                .find(|metadata| metadata.field == "id")
            {
                metadata.id = true;
            } else {
                model.fields.push(DbModelFieldMetadata {
                    field: "id".to_string(),
                    column: "id".to_string(),
                    ty: field.ty.clone(),
                    id: true,
                    id_strategy: None,
                    unique: false,
                    default_value: None,
                    foreign_key: None,
                    span: field.span,
                });
            }
            if let Some(field) = fields.iter_mut().find(|field| field.name == "id") {
                field.readonly = true;
            }
            continue;
        }
        let span = Span::DUMMY;
        fields.push(Field {
            name: "id".to_string(),
            ty: Type::I64,
            readonly: true,
            optional: false,
            decorators: Vec::new(),
            span,
        });
        model.fields.push(DbModelFieldMetadata {
            field: "id".to_string(),
            column: "id".to_string(),
            ty: Type::I64,
            id: true,
            id_strategy: None,
            unique: false,
            default_value: None,
            foreign_key: None,
            span,
        });
    }
}

pub const QUERY_TARGET: &str = "__tyrus_db_query";
pub const BEGIN_TRANSACTION_TARGET: &str = "__tyrus_db_begin_transaction";
pub const COMMIT_TRANSACTION_TARGET: &str = "__tyrus_db_commit_transaction";
pub const ROLLBACK_TRANSACTION_TARGET: &str = "__tyrus_db_rollback_transaction";
pub const FIND_MANY_TARGET: &str = "__tyrus_db_find_many";
pub const FIND_ONE_TARGET: &str = "__tyrus_db_find_one";
pub const COUNT_TARGET: &str = "__tyrus_db_count";
pub const INSERT_ONE_TARGET: &str = "__tyrus_db_insert_one";
pub const INSERT_MANY_TARGET: &str = "__tyrus_db_insert_many";
pub const UPDATE_ONE_TARGET: &str = "__tyrus_db_update_one";
pub const UPDATE_MANY_TARGET: &str = "__tyrus_db_update_many";
pub const DELETE_ONE_TARGET: &str = "__tyrus_db_delete_one";
pub const DELETE_MANY_TARGET: &str = "__tyrus_db_delete_many";
pub const MAX_BOUND_PARAMETERS: usize = 999;
pub const SQLITE_SAFE_PARAMETER_LIMIT: usize = MAX_BOUND_PARAMETERS;

pub fn model_method_target(method: &str) -> Option<&'static str> {
    match method {
        "findOne" => Some(FIND_ONE_TARGET),
        "count" => Some(COUNT_TARGET),
        "insertOne" => Some(INSERT_ONE_TARGET),
        "insertMany" => Some(INSERT_MANY_TARGET),
        "updateOne" => Some(UPDATE_ONE_TARGET),
        "updateMany" => Some(UPDATE_MANY_TARGET),
        "deleteOne" => Some(DELETE_ONE_TARGET),
        "deleteMany" => Some(DELETE_MANY_TARGET),
        _ => None,
    }
}

pub fn model_method_name(target: &str) -> Option<&'static str> {
    match target {
        FIND_ONE_TARGET => Some("findOne"),
        COUNT_TARGET => Some("count"),
        INSERT_ONE_TARGET => Some("insertOne"),
        INSERT_MANY_TARGET => Some("insertMany"),
        UPDATE_ONE_TARGET => Some("updateOne"),
        UPDATE_MANY_TARGET => Some("updateMany"),
        DELETE_ONE_TARGET => Some("deleteOne"),
        DELETE_MANY_TARGET => Some("deleteMany"),
        _ => None,
    }
}

pub fn model_mutation_result_type(method: &str, model_name: &str) -> Option<Type> {
    let model = Type::Named(model_name.to_string());
    let success = match method {
        "insertOne" => model,
        "insertMany" => Type::Array(Box::new(model)),
        "updateOne" | "deleteOne" => Type::Optional(Box::new(model)),
        "updateMany" | "deleteMany" => Type::U64,
        _ => return None,
    };
    Some(success)
}

pub(crate) fn db_builtins() -> HashMap<String, ExportInfo> {
    [
        (
            "db".to_string(),
            ExportInfo {
                target: "__tyrus_db_namespace".to_string(),
                ty: Type::Object(Vec::new()),
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "DbError".to_string(),
            ExportInfo {
                target: "DbError".to_string(),
                ty: Type::Named("DbError".to_string()),
                interface_fields: Some(db_error_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "Page".to_string(),
            ExportInfo {
                target: "Page".to_string(),
                ty: Type::Named("Page".to_string()),
                interface_fields: Some(crate::builtin_interfaces::db_page_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "ObjectId".to_string(),
            ExportInfo {
                target: "ObjectId".to_string(),
                ty: Type::Named("ObjectId".to_string()),
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: Some(Type::Str),
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "Uuid".to_string(),
            ExportInfo {
                target: "Uuid".to_string(),
                ty: Type::Named("Uuid".to_string()),
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: Some(Type::Str),
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "Uuid7".to_string(),
            ExportInfo {
                target: "Uuid7".to_string(),
                ty: Type::Named("Uuid7".to_string()),
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: Some(Type::Str),
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "DbTransaction".to_string(),
            ExportInfo {
                target: "DbTransaction".to_string(),
                ty: Type::Named("DbTransaction".to_string()),
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: Some(Type::U64),
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "query".to_string(),
            ExportInfo {
                target: QUERY_TARGET.to_string(),
                ty: Type::GenericFunction {
                    type_params: vec![TypeParam {
                        name: "T".to_string(),
                        constraint: None,
                    }],
                    params: vec![
                        Type::Str,
                        Type::Array(Box::new(Type::Unknown)),
                        Type::Optional(Box::new(Type::Object(vec![Field {
                            name: "transaction".to_string(),
                            ty: Type::U64,
                            readonly: true,
                            optional: false,
                            decorators: Vec::new(),
                            span: crate::span::Span::default(),
                        }]))),
                    ],
                    ret: Box::new(Type::Apply {
                        name: "DbPromise".to_string(),
                        args: vec![Type::Result {
                            success: Box::new(Type::Array(Box::new(Type::Generic(
                                "T".to_string(),
                            )))),
                            error: Box::new(Type::Named("DbError".to_string())),
                        }],
                    }),
                },
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "beginTransaction".to_string(),
            db_transaction_function(BEGIN_TRANSACTION_TARGET, Vec::new(), Type::U64),
        ),
        (
            "commitTransaction".to_string(),
            db_transaction_function(COMMIT_TRANSACTION_TARGET, vec![Type::U64], Type::Bool),
        ),
        (
            "rollbackTransaction".to_string(),
            db_transaction_function(ROLLBACK_TRANSACTION_TARGET, vec![Type::U64], Type::Bool),
        ),
    ]
    .into_iter()
    .collect()
}

fn db_transaction_function(target: &str, params: Vec<Type>, success: Type) -> ExportInfo {
    ExportInfo {
        target: target.to_string(),
        ty: Type::Function {
            params,
            ret: Box::new(Type::Apply {
                name: "DbPromise".to_string(),
                args: vec![Type::Result {
                    success: Box::new(success),
                    error: Box::new(Type::Named("DbError".to_string())),
                }],
            }),
        },
        interface_fields: None,
        behavior_info: None,
        type_alias_ty: None,
        enum_info: None,
        throws: Vec::new(),
        db_model: None,
    }
}
