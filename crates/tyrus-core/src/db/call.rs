use crate::ast::{Expr, ExprKind, Field, Type};
use std::ops::Range;

use super::{
    DbSchemaSnapshot, SqlExec, SqlParseError, SqlSelect, ValidatedSqlSelect, parse_exec_sql,
    parse_select_sql, validate_exec_against_snapshot, validate_select_against_snapshot,
    validate_select_against_snapshot_with_mappings,
};

#[derive(Clone, Debug)]
pub struct CapturedSqlCall {
    pub sql: String,
    pub sql_literal_span: Range<usize>,
    pub parameter_spans: Vec<Range<usize>>,
    pub parameters: Vec<Expr>,
    sql_content_start: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqlCallCaptureError {
    pub message: String,
    pub span: Range<usize>,
}

impl CapturedSqlCall {
    pub fn capture_arguments(
        source: &str,
        arguments: &[Expr],
        call_span: Range<usize>,
    ) -> Result<Self, SqlCallCaptureError> {
        if arguments.len() != 2 {
            return Err(SqlCallCaptureError {
                message: format!(
                    "database query expects 2 arguments, but {} were provided",
                    arguments.len()
                ),
                span: call_span,
            });
        }
        Self::capture(source, &arguments[0], &arguments[1])
    }

    #[cfg(kani)]
    pub(crate) fn kani_range_fixture() -> Self {
        Self {
            sql: "abc".to_string(),
            sql_literal_span: 20..25,
            parameter_spans: Vec::new(),
            parameters: Vec::new(),
            sql_content_start: Some(21),
        }
    }

    pub fn capture(
        source: &str,
        sql_expression: &Expr,
        parameters_expression: &Expr,
    ) -> Result<Self, SqlCallCaptureError> {
        let ExprKind::StrLit(sql) = sql_expression.kind() else {
            return Err(SqlCallCaptureError {
                message: "SQL must be a string literal".to_string(),
                span: sql_expression.span.to_range(),
            });
        };
        let ExprKind::ArrayLit(parameters) = parameters_expression.kind() else {
            return Err(SqlCallCaptureError {
                message: "SQL parameters must be provided as an array literal".to_string(),
                span: parameters_expression.span.to_range(),
            });
        };
        let sql_literal_span = sql_expression.span.to_range();
        let sql_content_start = source
            .get(sql_literal_span.clone())
            .and_then(|literal| direct_literal_contents(literal, sql))
            .map(|_| sql_literal_span.start + literal_content_prefix(source, &sql_literal_span));
        Ok(Self {
            sql: sql.clone(),
            sql_literal_span,
            parameter_spans: parameters
                .iter()
                .map(|parameter| parameter.span.to_range())
                .collect(),
            parameters: parameters.clone(),
            sql_content_start,
        })
    }

    pub fn parse_select(&self) -> Result<SqlSelect, SqlParseError> {
        parse_select_sql(&self.sql)
    }

    pub fn parse_select_at_source(&self) -> Result<SqlSelect, SqlCallCaptureError> {
        self.parse_select()
            .map_err(|error| self.map_parse_error(error))
    }

    pub fn validate_select_at_source(
        &self,
        snapshot: &DbSchemaSnapshot,
        result_fields: &[Field],
        parameter_types: &[Type],
    ) -> Result<ValidatedSqlSelect, SqlCallCaptureError> {
        let query = self.parse_select_at_source()?;
        self.reject_extra_parameters(query.parameter_count, parameter_types.len())?;
        validate_select_against_snapshot(snapshot, &query, result_fields, parameter_types)
            .map_err(|error| self.map_parse_error(error))
    }

    pub fn validate_select_at_source_with_mappings(
        &self,
        snapshot: &DbSchemaSnapshot,
        result_fields: &[Field],
        parameter_types: &[Type],
        field_columns: &crate::collections::HashMap<String, String>,
    ) -> Result<ValidatedSqlSelect, SqlCallCaptureError> {
        let query = self.parse_select_at_source()?;
        self.reject_extra_parameters(query.parameter_count, parameter_types.len())?;
        validate_select_against_snapshot_with_mappings(
            snapshot,
            &query,
            result_fields,
            parameter_types,
            field_columns,
        )
        .map_err(|error| self.map_parse_error(error))
    }

    pub fn validate_exec_at_source(
        &self,
        snapshot: &DbSchemaSnapshot,
        parameter_types: &[Type],
    ) -> Result<SqlExec, SqlCallCaptureError> {
        let statement = parse_exec_sql(&self.sql).map_err(|error| self.map_parse_error(error))?;
        self.reject_extra_parameters(statement.bindings.len(), parameter_types.len())?;
        validate_exec_against_snapshot(snapshot, &statement, parameter_types)
            .map_err(|error| self.map_parse_error(error))?;
        Ok(statement)
    }

    fn reject_extra_parameters(
        &self,
        placeholder_count: usize,
        parameter_count: usize,
    ) -> Result<(), SqlCallCaptureError> {
        if parameter_count <= placeholder_count {
            return Ok(());
        }
        Err(SqlCallCaptureError {
            message: format!(
                "query has {placeholder_count} placeholder(s), but {parameter_count} parameter value(s) were provided"
            ),
            span: self
                .parameter_spans
                .get(placeholder_count)
                .cloned()
                .unwrap_or_else(|| self.sql_literal_span.clone()),
        })
    }

    fn map_parse_error(&self, error: SqlParseError) -> SqlCallCaptureError {
        SqlCallCaptureError {
            span: self.source_range_for_parse_error(&error),
            message: error.message,
        }
    }

    pub fn source_range_for_sql(&self, sql_range: Range<usize>) -> Range<usize> {
        if sql_range.start > sql_range.end || sql_range.end > self.sql.len() {
            return self.sql_literal_span.clone();
        }
        self.sql_content_start
            .map(|start| start + sql_range.start..start + sql_range.end)
            .unwrap_or_else(|| self.sql_literal_span.clone())
    }

    pub fn source_range_for_parse_error(&self, error: &SqlParseError) -> Range<usize> {
        self.source_range_for_sql(error.span.clone())
    }
}

fn literal_content_prefix(source: &str, span: &Range<usize>) -> usize {
    source
        .get(span.clone())
        .map(|literal| {
            if literal.starts_with("\"\"\"") || literal.starts_with("'''") {
                3
            } else {
                1
            }
        })
        .unwrap_or(0)
}

fn direct_literal_contents<'a>(literal: &'a str, decoded: &str) -> Option<&'a str> {
    let delimiter_len = if literal.starts_with("\"\"\"") || literal.starts_with("'''") {
        3
    } else if literal.starts_with('"') || literal.starts_with('\'') {
        1
    } else {
        return None;
    };
    if literal.len() < delimiter_len * 2 {
        return None;
    }
    let content = literal.get(delimiter_len..literal.len() - delimiter_len)?;
    (content == decoded).then_some(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

    fn string_expression(source: &str, literal: &str, decoded: &str) -> Expr {
        let start = source.find(literal).unwrap();
        Expr::new(
            ExprKind::StrLit(decoded.to_string()),
            Span::new(start, start + literal.len()),
        )
    }

    fn array_expression(source: &str, parameter: &str) -> Expr {
        let start = source.find('[').unwrap();
        let parameter_start = source.find(parameter).unwrap();
        Expr::new(
            ExprKind::ArrayLit(vec![Expr::new(
                ExprKind::Ident(parameter.to_string()),
                Span::new(parameter_start, parameter_start + parameter.len()),
            )]),
            Span::new(start, source[start..].find(']').unwrap() + start + 1),
        )
    }

    #[test]
    fn argument_capture_reports_wrong_arity_on_the_call() {
        let source = "db.query(\"DELETE FROM users WHERE id = ?\");";
        let call_span = 0..source.len();
        let error = CapturedSqlCall::capture_arguments(source, &[], call_span.clone()).unwrap_err();

        assert_eq!(
            error.message,
            "database query expects 2 arguments, but 0 were provided"
        );
        assert_eq!(error.span, call_span);
    }

    #[test]
    fn captures_sql_parameters_and_maps_sql_errors_to_source_ranges() {
        let source = "db.query<User>(\"SELECT * FROM users CROSS JOIN roles\", [userId]);";
        let sql_expression = string_expression(
            source,
            "\"SELECT * FROM users CROSS JOIN roles\"",
            "SELECT * FROM users CROSS JOIN roles",
        );
        let parameters_expression = array_expression(source, "userId");
        let call =
            CapturedSqlCall::capture(source, &sql_expression, &parameters_expression).unwrap();
        assert_eq!(call.sql, "SELECT * FROM users CROSS JOIN roles");
        assert_eq!(&source[call.parameter_spans[0].clone()], "userId");
        let error = call.parse_select_at_source().unwrap_err();
        assert_eq!(
            error.message,
            "unsupported SQL syntax after the supported SELECT clauses"
        );
        assert_eq!(&source[error.span], "CROSS");
    }

    #[test]
    fn uses_the_full_literal_range_when_escape_decoding_breaks_direct_offsets() {
        let source = "db.query<User>(\"SELECT\\n * FROM users\", []);";
        let sql_literal = "\"SELECT\\n * FROM users\"";
        let sql_expression = string_expression(source, sql_literal, "SELECT\n * FROM users");
        let parameters_expression = Expr::new(
            ExprKind::ArrayLit(Vec::new()),
            Span::new(source.find("[]").unwrap(), source.find("[]").unwrap() + 2),
        );
        let call =
            CapturedSqlCall::capture(source, &sql_expression, &parameters_expression).unwrap();
        let range = call.source_range_for_sql(0..6);
        assert_eq!(range, sql_expression.span.to_range());
    }

    #[test]
    fn parse_errors_with_escaped_sql_point_to_the_full_literal() {
        let source = "db.query<User>(\"SELECT\\n * FROM users CROSS JOIN roles\", []);";
        let sql_literal = "\"SELECT\\n * FROM users CROSS JOIN roles\"";
        let sql_expression = string_expression(
            source,
            sql_literal,
            "SELECT\n * FROM users CROSS JOIN roles",
        );
        let parameters_expression = Expr::new(
            ExprKind::ArrayLit(Vec::new()),
            Span::new(source.find("[]").unwrap(), source.find("[]").unwrap() + 2),
        );
        let call =
            CapturedSqlCall::capture(source, &sql_expression, &parameters_expression).unwrap();
        let error = call.parse_select_at_source().unwrap_err();

        assert_eq!(&source[error.span], sql_literal);
        assert_eq!(
            error.message,
            "unsupported SQL syntax after the supported SELECT clauses"
        );
    }

    #[test]
    fn exec_validation_maps_snapshot_errors_to_the_sql_literal() {
        use crate::db::{DbColumnSchema, DbSchemaSnapshot, DbTableSchema};

        let source = "db.query(\"DELETE FROM users WHERE missing = ?\", [userId]);";
        let sql_expression = string_expression(
            source,
            "\"DELETE FROM users WHERE missing = ?\"",
            "DELETE FROM users WHERE missing = ?",
        );
        let parameters_expression = array_expression(source, "userId");
        let captured =
            CapturedSqlCall::capture(source, &sql_expression, &parameters_expression).unwrap();
        let snapshot = DbSchemaSnapshot::new(
            "./app.db".to_string(),
            1,
            vec![DbTableSchema {
                name: "users".to_string(),
                engine: None,
                columns: vec![DbColumnSchema {
                    name: "id".to_string(),
                    logical_type: "i64".to_string(),
                    nullable: false,
                    ordinal: 0,
                    default: None,
                    native_type: None,
                }],
                primary_key: vec!["id".to_string()],
                unique: Vec::new(),
                foreign_keys: Vec::new(),
                indexes: Vec::new(),
                triggers: Vec::new(),
            }],
        );
        let error = captured
            .validate_exec_at_source(&snapshot, &[Type::I64])
            .unwrap_err();
        assert_eq!(&source[error.span], "missing");
    }

    #[test]
    fn extra_exec_parameter_points_to_the_extra_array_element() {
        use crate::db::{DbColumnSchema, DbSchemaSnapshot, DbTableSchema};

        let source = "db.query(\"DELETE FROM users WHERE id = ?\", [id, extra]);";
        let sql_expression = string_expression(
            source,
            "\"DELETE FROM users WHERE id = ?\"",
            "DELETE FROM users WHERE id = ?",
        );
        let array_start = source.find("[id, extra]").unwrap();
        let parameters_expression = Expr::new(
            ExprKind::ArrayLit(vec![
                Expr::new(
                    ExprKind::Ident("id".to_string()),
                    Span::new(array_start + 1, array_start + 3),
                ),
                Expr::new(
                    ExprKind::Ident("extra".to_string()),
                    Span::new(array_start + 5, array_start + 10),
                ),
            ]),
            Span::new(array_start, array_start + "[id, extra]".len()),
        );
        let captured =
            CapturedSqlCall::capture(source, &sql_expression, &parameters_expression).unwrap();
        let snapshot = DbSchemaSnapshot::new(
            "./app.db".to_string(),
            1,
            vec![DbTableSchema {
                name: "users".to_string(),
                engine: None,
                columns: vec![DbColumnSchema {
                    name: "id".to_string(),
                    logical_type: "i64".to_string(),
                    nullable: false,
                    ordinal: 0,
                    default: None,
                    native_type: None,
                }],
                primary_key: vec!["id".to_string()],
                unique: Vec::new(),
                foreign_keys: Vec::new(),
                indexes: Vec::new(),
                triggers: Vec::new(),
            }],
        );
        let error = captured
            .validate_exec_at_source(&snapshot, &[Type::I64, Type::I64])
            .unwrap_err();
        assert_eq!(&source[error.span], "extra");
    }

    #[test]
    fn rejects_non_literal_sql_and_non_literal_parameter_collections_at_the_argument() {
        let source = "db.query<User>(sql, parameters);";
        let sql_start = source.find("sql").unwrap();
        let sql_expression = Expr::new(
            ExprKind::Ident("sql".to_string()),
            Span::new(sql_start, sql_start + 3),
        );
        let params_start = source.find("parameters").unwrap();
        let parameters_expression = Expr::new(
            ExprKind::Ident("parameters".to_string()),
            Span::new(params_start, params_start + "parameters".len()),
        );
        let error =
            CapturedSqlCall::capture(source, &sql_expression, &parameters_expression).unwrap_err();
        assert_eq!(&source[error.span], "sql");
    }
}
