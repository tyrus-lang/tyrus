use std::io::Read;
use std::path::Path;

use tyrus_core::ast::{Field, Type};
use tyrus_core::db::{
    DbColumnSchema, DbSchemaCache, DbSchemaSnapshot, SqlTokenKind, aggregate_projection_type,
    parse_exec_sql, parse_select_sql, select_table_nullability, tokenize_sql,
    validate_exec_against_snapshot, validate_select_against_snapshot,
};
use tyrus_core::project;
use tyrus_core::span::Span;

pub(super) fn run(project_root: &Path, sql_argument: Option<&str>) -> Result<(), String> {
    let mut stdin_sql = String::new();
    let sql = if let Some(sql) = sql_argument {
        sql
    } else {
        std::io::stdin()
            .read_to_string(&mut stdin_sql)
            .map_err(|_| "could not read SQL from stdin".to_string())?;
        &stdin_sql
    };
    let sql = sql.trim();
    if sql.is_empty() {
        return Err("provide one SQL statement as an argument or through stdin".to_string());
    }

    let config = project::load_project(project_root)
        .map_err(|_| "could not load tyrus.json database configuration".to_string())?
        .database
        .ok_or_else(|| "tyrus.json has no `database` configuration".to_string())?;
    let mut cache = DbSchemaCache::default();
    let snapshot = cache
        .load_configured(project_root, &config)
        .map_err(|error| {
            let path = config.resolve_schema_path(project_root).ok();
            if path.is_some_and(|path| !path.exists()) {
                "database schema snapshot is missing; run `tyrus db introspect` first".to_string()
            } else {
                error
            }
        })?;
    let (parameters, result_fields) = explain(&snapshot, sql)?;
    println!("SQL: {}", normalize_sql(sql)?);
    if parameters.is_empty() {
        println!("Parameters: none");
    } else {
        println!("Parameters:");
        for (index, ty) in parameters.iter().enumerate() {
            println!("  ?{}: {} (value hidden)", index + 1, type_name(ty));
        }
    }
    if let Some(fields) = result_fields {
        println!("Result fields:");
        for field in fields {
            println!("  {}: {}", field.name, type_name(&field.ty));
        }
    } else {
        println!("Result: void");
    }
    Ok(())
}

fn explain(
    snapshot: &DbSchemaSnapshot,
    sql: &str,
) -> Result<(Vec<Type>, Option<Vec<Field>>), String> {
    let tokens = tokenize_sql(sql).map_err(|error| error.message)?;
    let first = tokens
        .first()
        .ok_or_else(|| "provide one SQL statement as an argument or through stdin".to_string())?;
    if first.text.eq_ignore_ascii_case("SELECT") {
        explain_select(snapshot, sql)
    } else {
        explain_exec(snapshot, sql)
    }
}

fn explain_select(
    snapshot: &DbSchemaSnapshot,
    sql: &str,
) -> Result<(Vec<Type>, Option<Vec<Field>>), String> {
    let query = parse_select_sql(sql).map_err(|error| error.message)?;
    let mut parameters = vec![None; query.parameter_count];
    for predicate in &query.predicates {
        let Some(index) = predicate.placeholder else {
            continue;
        };
        let (_, column) = resolve_select_column(snapshot, &query, &predicate.column)?;
        parameters[index] = Some(column_type(column)?);
    }
    for index in [query.limit_placeholder, query.offset_placeholder]
        .into_iter()
        .flatten()
    {
        parameters[index] = Some(Type::I64);
    }
    let parameter_types = complete_parameters(parameters)?;
    let fields = select_result_fields(snapshot, &query)?;
    validate_select_against_snapshot(snapshot, &query, &fields, &parameter_types)
        .map_err(|error| error.message)?;
    Ok((parameter_types, Some(fields)))
}

fn explain_exec(
    snapshot: &DbSchemaSnapshot,
    sql: &str,
) -> Result<(Vec<Type>, Option<Vec<Field>>), String> {
    let statement = parse_exec_sql(sql).map_err(|error| error.message)?;
    let table = snapshot
        .tables
        .iter()
        .find(|table| table.name == statement.table)
        .ok_or_else(|| format!("unknown table `{}`", statement.table))?;
    let parameter_types = statement
        .bindings
        .iter()
        .map(|binding| find_column(table.columns.as_slice(), &binding.column).and_then(column_type))
        .collect::<Result<Vec<_>, _>>()?;
    validate_exec_against_snapshot(snapshot, &statement, &parameter_types)
        .map_err(|error| error.message)?;
    Ok((parameter_types, None))
}

fn select_result_fields(
    snapshot: &DbSchemaSnapshot,
    query: &tyrus_core::db::SqlSelect,
) -> Result<Vec<Field>, String> {
    let table_nullability = select_table_nullability(query);
    let selected = if query.select_all {
        select_tables(snapshot, query)?
            .into_iter()
            .enumerate()
            .flat_map(|(table_index, table)| {
                table
                    .2
                    .columns
                    .iter()
                    .map(|column| {
                        (
                            Some(column),
                            Some(table_index),
                            None,
                            column.name.to_string(),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    } else {
        query
            .projections
            .iter()
            .map(|projection| {
                if projection.aggregate.is_some() {
                    let ty = aggregate_projection_type(snapshot, query, projection)
                        .map_err(|error| error.message)?;
                    return Ok((
                        None,
                        None,
                        Some(ty),
                        projection_output_name(projection).to_string(),
                    ));
                }
                let (table_index, column) =
                    resolve_select_column(snapshot, query, &projection.name)?;
                Ok((
                    Some(column),
                    Some(table_index),
                    None,
                    projection_output_name(projection).to_string(),
                ))
            })
            .collect::<Result<Vec<_>, String>>()?
    };
    selected
        .into_iter()
        .map(|(column, table_index, aggregate_type, output_name)| {
            let ty = if let Some(ty) = aggregate_type {
                ty
            } else if let Some(column) = column {
                let mut ty = column_type(column)?;
                if column.nullable || table_index.is_some_and(|index| table_nullability[index]) {
                    ty = Type::Optional(Box::new(ty));
                }
                ty
            } else {
                return Err("aggregate projection has no result type".to_string());
            };
            Ok(Field {
                name: output_name,
                ty,
                readonly: false,
                optional: false,
                decorators: Vec::new(),
                span: Span::DUMMY,
            })
        })
        .collect()
}

type SelectedTable<'a> = (&'a str, Option<&'a str>, &'a tyrus_core::db::DbTableSchema);

fn select_tables<'a>(
    snapshot: &'a DbSchemaSnapshot,
    query: &'a tyrus_core::db::SqlSelect,
) -> Result<Vec<SelectedTable<'a>>, String> {
    let mut tables = Vec::with_capacity(query.joins.len() + 1);
    let base = snapshot
        .tables
        .iter()
        .find(|table| table.name == query.table)
        .ok_or_else(|| format!("unknown table `{}`", query.table))?;
    tables.push((query.table.as_str(), query.table_alias.as_deref(), base));
    for join in &query.joins {
        let table = snapshot
            .tables
            .iter()
            .find(|table| table.name == join.table)
            .ok_or_else(|| format!("unknown table `{}`", join.table))?;
        tables.push((join.table.as_str(), join.alias.as_deref(), table));
    }
    Ok(tables)
}

fn resolve_select_column<'a>(
    snapshot: &'a DbSchemaSnapshot,
    query: &'a tyrus_core::db::SqlSelect,
    reference: &str,
) -> Result<(usize, &'a DbColumnSchema), String> {
    let (qualifier, name) = reference
        .split_once('.')
        .map_or((None, reference), |(qualifier, name)| {
            (Some(qualifier), name)
        });
    let matches = select_tables(snapshot, query)?
        .into_iter()
        .enumerate()
        .filter(|(_, (table_name, alias, _))| {
            qualifier
                .is_none_or(|qualifier| alias.unwrap_or(table_name).eq_ignore_ascii_case(qualifier))
        })
        .filter_map(|(index, (_, _, table))| {
            table
                .columns
                .iter()
                .find(|column| column.name.eq_ignore_ascii_case(name))
                .map(|column| (index, column))
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [matched] => Ok(*matched),
        [] => Err(format!("unknown column `{reference}`")),
        _ => Err(format!(
            "column `{reference}` is ambiguous; qualify it with a table name or alias"
        )),
    }
}

fn projection_output_name(projection: &tyrus_core::db::SqlProjection) -> &str {
    projection.alias.as_deref().unwrap_or_else(|| {
        projection
            .name
            .rsplit_once('.')
            .map_or(projection.name.as_str(), |(_, column)| column)
    })
}

fn find_column<'a>(
    columns: &'a [DbColumnSchema],
    name: &str,
) -> Result<&'a DbColumnSchema, String> {
    columns
        .iter()
        .find(|column| column.name == name)
        .ok_or_else(|| format!("unknown column `{name}`"))
}

fn column_type(column: &DbColumnSchema) -> Result<Type, String> {
    match column.logical_type.as_str() {
        "i64" => Ok(Type::I64),
        "u64" => Ok(Type::U64),
        "f64" => Ok(Type::F64),
        "boolean" => Ok(Type::Bool),
        "bytes" => Ok(Type::Array(Box::new(Type::U8))),
        "string" => Ok(Type::Str),
        other => Err(format!("unsupported logical type `{other}`")),
    }
}

fn complete_parameters(parameters: Vec<Option<Type>>) -> Result<Vec<Type>, String> {
    parameters
        .into_iter()
        .enumerate()
        .map(|(index, ty)| {
            ty.ok_or_else(|| format!("could not infer the type of SQL parameter ?{}", index + 1))
        })
        .collect()
}

fn normalize_sql(sql: &str) -> Result<String, String> {
    let tokens = tokenize_sql(sql).map_err(|error| error.message)?;
    let mut output = String::new();
    let mut previous: Option<&tyrus_core::db::SqlToken> = None;
    for token in &tokens {
        if previous.is_some_and(|previous| needs_space(previous.kind.clone(), token.kind.clone())) {
            output.push(' ');
        }
        match token.kind {
            SqlTokenKind::String => output.push_str("<redacted:string>"),
            SqlTokenKind::Number => output.push_str("<redacted:number>"),
            SqlTokenKind::Word if is_keyword(&token.text) => {
                output.push_str(&token.text.to_ascii_uppercase())
            }
            _ => output.push_str(&token.text),
        }
        previous = Some(token);
    }
    Ok(output)
}

fn needs_space(previous: SqlTokenKind, current: SqlTokenKind) -> bool {
    !matches!(
        current,
        SqlTokenKind::Comma | SqlTokenKind::Dot | SqlTokenKind::RightParen
    ) && !matches!(previous, SqlTokenKind::Dot | SqlTokenKind::LeftParen)
}

fn is_keyword(word: &str) -> bool {
    [
        "SELECT", "FROM", "WHERE", "ORDER", "BY", "ASC", "DESC", "LIMIT", "OFFSET", "AS", "INSERT",
        "INTO", "VALUES", "UPDATE", "SET", "DELETE", "AND",
    ]
    .iter()
    .any(|keyword| word.eq_ignore_ascii_case(keyword))
}

fn type_name(ty: &Type) -> String {
    match ty {
        Type::I64 => "i64".to_string(),
        Type::U64 => "u64".to_string(),
        Type::F64 => "f64".to_string(),
        Type::Bool => "boolean".to_string(),
        Type::Str => "string".to_string(),
        Type::Array(inner) if **inner == Type::U8 => "bytes".to_string(),
        Type::Optional(inner) => format!("{}?", type_name(inner)),
        _ => ty.to_string(),
    }
}
