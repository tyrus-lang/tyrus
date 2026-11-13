use super::schema::DbSchemaSnapshot;
use super::sql::{SqlParseError, SqlToken, SqlTokenKind, tokenize_sql};
use crate::ast::Type;
use std::ops::Range;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SqlExecKind {
    Insert,
    Update,
    Delete,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqlExecBinding {
    pub column: String,
    pub column_span: Range<usize>,
    pub placeholder_span: Range<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqlExecStaticAssignment {
    pub column: String,
    pub column_span: Range<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqlExec {
    pub kind: SqlExecKind,
    pub table: String,
    pub table_span: Range<usize>,
    pub bindings: Vec<SqlExecBinding>,
    pub static_assignments: Vec<SqlExecStaticAssignment>,
    pub assignment_count: usize,
    pub source_len: usize,
}

pub fn parse_exec_sql(source: &str) -> Result<SqlExec, SqlParseError> {
    let tokens = tokenize_sql(source)?;
    let mut parser = ExecParser {
        tokens,
        cursor: 0,
        source_len: source.len(),
        static_assignments: Vec::new(),
    };
    let kind = if parser.consume_keyword("INSERT") {
        SqlExecKind::Insert
    } else if parser.consume_keyword("UPDATE") {
        SqlExecKind::Update
    } else if parser.consume_keyword("DELETE") {
        SqlExecKind::Delete
    } else {
        return Err(parser.error_here("db.query supports only INSERT, UPDATE, and DELETE"));
    };
    let (table, bindings, assignment_count) = match kind {
        SqlExecKind::Insert => parser.parse_insert()?,
        SqlExecKind::Update => parser.parse_update()?,
        SqlExecKind::Delete => parser.parse_delete()?,
    };
    if parser.peek().is_some() {
        return Err(parser.error_here("unsupported SQL syntax after the statement"));
    }
    Ok(SqlExec {
        kind,
        table: table.text,
        table_span: table.span,
        bindings,
        static_assignments: parser.static_assignments,
        assignment_count,
        source_len: source.len(),
    })
}

pub fn validate_exec_against_snapshot(
    snapshot: &DbSchemaSnapshot,
    statement: &SqlExec,
    parameter_types: &[Type],
) -> Result<(), SqlParseError> {
    let table = snapshot
        .tables
        .iter()
        .find(|table| table.name == statement.table)
        .ok_or_else(|| {
            error(
                format!("unknown table `{}`", statement.table),
                statement.table_span.clone(),
            )
        })?;
    for assignment in &statement.static_assignments {
        let column = table
            .columns
            .iter()
            .find(|column| column.name == assignment.column)
            .ok_or_else(|| {
                error(
                    format!("unknown column `{}` in `{}`", assignment.column, table.name),
                    assignment.column_span.clone(),
                )
            })?;
        if !matches!(column.logical_type.as_str(), "instant" | "string") {
            return Err(error(
                format!(
                    "`CURRENT_TIMESTAMP` can only update a temporal or text column, but `{}` has type `{}`",
                    column.name, column.logical_type
                ),
                assignment.column_span.clone(),
            ));
        }
    }
    if statement.bindings.len() != parameter_types.len() {
        let span = statement
            .bindings
            .get(parameter_types.len())
            .map_or(statement.source_len..statement.source_len, |binding| {
                binding.placeholder_span.clone()
            });
        return Err(error(
            format!(
                "query has {} placeholder(s), but {} parameter value(s) were provided",
                statement.bindings.len(),
                parameter_types.len()
            ),
            span,
        ));
    }
    let mut assigned = std::collections::HashSet::new();
    for (index, (binding, actual)) in statement.bindings.iter().zip(parameter_types).enumerate() {
        if statement.kind == SqlExecKind::Insert
            && statement.assignment_count > 0
            && index % statement.assignment_count == 0
        {
            assigned.clear();
        }
        if matches!(statement.kind, SqlExecKind::Insert | SqlExecKind::Update)
            && index < statement.assignment_count
            && !assigned.insert(binding.column.as_str())
        {
            return Err(error(
                format!("column `{}` is assigned more than once", binding.column),
                binding.column_span.clone(),
            ));
        }
        let column = table
            .columns
            .iter()
            .find(|column| column.name == binding.column)
            .ok_or_else(|| {
                error(
                    format!("unknown column `{}` in `{}`", binding.column, table.name),
                    binding.column_span.clone(),
                )
            })?;
        let expected = match column.logical_type.as_str() {
            "i64" => Type::I64,
            "u64" => Type::U64,
            "f64" => Type::F64,
            "boolean" => Type::Bool,
            "bytes" => Type::Array(Box::new(Type::U8)),
            "string" => Type::Str,
            "instant" => Type::Named("Instant".to_string()),
            other => {
                return Err(error(
                    format!(
                        "unsupported logical type `{other}` for column `{}`",
                        column.name
                    ),
                    binding.column_span.clone(),
                ));
            }
        };
        let compatible_type = |actual: &Type| {
            *actual == expected
                || matches!(
                    (&expected, actual),
                    (Type::I64, Type::I32) | (Type::U64, Type::U32) | (Type::F64, Type::F32)
                )
        };
        let compatible = if let Type::Optional(inner) = actual {
            column.nullable && compatible_type(inner)
        } else {
            compatible_type(actual)
        };
        if !compatible {
            return Err(error(
                format!(
                    "parameter {} for `{}` expects `{expected}`, found `{actual}`",
                    index + 1,
                    column.name
                ),
                binding.placeholder_span.clone(),
            ));
        }
    }
    Ok(())
}

struct ExecParser {
    tokens: Vec<SqlToken>,
    cursor: usize,
    source_len: usize,
    static_assignments: Vec<SqlExecStaticAssignment>,
}

impl ExecParser {
    fn parse_insert(&mut self) -> Result<(SqlToken, Vec<SqlExecBinding>, usize), SqlParseError> {
        self.expect_keyword("INTO")?;
        let table = self.expect_word("expected a table name after INTO")?;
        self.expect_kind(SqlTokenKind::LeftParen, "expected `(` after table name")?;
        let mut columns = Vec::new();
        loop {
            columns.push(self.expect_word("expected a column name in INSERT")?);
            if !self.consume_kind(SqlTokenKind::Comma) {
                break;
            }
        }
        self.expect_kind(
            SqlTokenKind::RightParen,
            "expected `)` after INSERT columns",
        )?;
        self.expect_keyword("VALUES")?;
        self.expect_kind(SqlTokenKind::LeftParen, "expected `(` after VALUES")?;
        let mut placeholders = Vec::new();
        loop {
            placeholders.push(self.expect_kind(
                SqlTokenKind::Placeholder,
                "INSERT values must use `?` parameters",
            )?);
            if !self.consume_kind(SqlTokenKind::Comma) {
                break;
            }
        }
        self.expect_kind(SqlTokenKind::RightParen, "expected `)` after INSERT values")?;
        if columns.len() != placeholders.len() {
            return Err(self.error_here("INSERT columns and values have different lengths"));
        }
        while self.consume_kind(SqlTokenKind::Comma) {
            self.expect_kind(
                SqlTokenKind::LeftParen,
                "expected `(` before the next INSERT row",
            )?;
            let start = placeholders.len();
            loop {
                placeholders.push(self.expect_kind(
                    SqlTokenKind::Placeholder,
                    "INSERT values must use `?` parameters",
                )?);
                if !self.consume_kind(SqlTokenKind::Comma) {
                    break;
                }
            }
            self.expect_kind(SqlTokenKind::RightParen, "expected `)` after INSERT values")?;
            if placeholders.len() - start != columns.len() {
                return Err(self.error_here("each INSERT row must match the column count"));
            }
        }
        let columns_len = columns.len();
        let bindings = placeholders
            .into_iter()
            .enumerate()
            .map(|(index, placeholder)| {
                let column = &columns[index % columns_len];
                SqlExecBinding {
                    column: column.text.clone(),
                    column_span: column.span.clone(),
                    placeholder_span: placeholder.span,
                }
            })
            .collect();
        Ok((table, bindings, columns_len))
    }

    fn parse_update(&mut self) -> Result<(SqlToken, Vec<SqlExecBinding>, usize), SqlParseError> {
        let table = self.expect_word("expected a table name after UPDATE")?;
        self.expect_keyword("SET")?;
        let mut bindings = Vec::new();
        loop {
            let column = self.expect_word("expected a column name in SET")?;
            self.expect_operator("=")?;
            if self.consume_keyword("CURRENT_TIMESTAMP") {
                self.static_assignments.push(SqlExecStaticAssignment {
                    column: column.text,
                    column_span: column.span,
                });
                if !self.consume_kind(SqlTokenKind::Comma) {
                    break;
                }
                continue;
            }
            let placeholder = self.expect_kind(
                SqlTokenKind::Placeholder,
                "UPDATE values must use `?` parameters",
            )?;
            bindings.push(SqlExecBinding {
                column: column.text,
                column_span: column.span,
                placeholder_span: placeholder.span,
            });
            if !self.consume_kind(SqlTokenKind::Comma) {
                break;
            }
        }
        self.expect_keyword("WHERE")?;
        let assignment_count = bindings.len();
        bindings.extend(self.parse_where()?);
        Ok((table, bindings, assignment_count))
    }

    fn parse_delete(&mut self) -> Result<(SqlToken, Vec<SqlExecBinding>, usize), SqlParseError> {
        self.expect_keyword("FROM")?;
        let table = self.expect_word("expected a table name after FROM")?;
        self.expect_keyword("WHERE")?;
        Ok((table, self.parse_where()?, 0))
    }

    fn parse_where(&mut self) -> Result<Vec<SqlExecBinding>, SqlParseError> {
        let mut bindings = Vec::new();
        loop {
            let column = self.expect_word("expected a column name in WHERE")?;
            let operator = self.expect_kind(
                SqlTokenKind::Operator,
                "expected a comparison operator in WHERE",
            )?;
            if !matches!(
                operator.text.as_str(),
                "=" | "!=" | "<>" | "<" | "<=" | ">" | ">="
            ) {
                return Err(error(
                    "unsupported comparison operator in WHERE",
                    operator.span,
                ));
            }
            let placeholder = self.expect_kind(
                SqlTokenKind::Placeholder,
                "WHERE values must use `?` parameters",
            )?;
            bindings.push(SqlExecBinding {
                column: column.text,
                column_span: column.span,
                placeholder_span: placeholder.span,
            });
            if !self.consume_keyword("AND") {
                break;
            }
        }
        Ok(bindings)
    }

    fn peek(&self) -> Option<&SqlToken> {
        self.tokens.get(self.cursor)
    }

    fn error_here(&self, message: &str) -> SqlParseError {
        let span = self
            .peek()
            .map_or(self.source_len..self.source_len, |token| token.span.clone());
        error(message, span)
    }

    fn consume_keyword(&mut self, keyword: &str) -> bool {
        if self.peek().is_some_and(|token| {
            token.kind == SqlTokenKind::Word && token.text.eq_ignore_ascii_case(keyword)
        }) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    fn expect_keyword(&mut self, keyword: &str) -> Result<(), SqlParseError> {
        if self.consume_keyword(keyword) {
            Ok(())
        } else {
            Err(self.error_here(&format!("expected `{keyword}`")))
        }
    }

    fn consume_kind(&mut self, kind: SqlTokenKind) -> bool {
        if self.peek().is_some_and(|token| token.kind == kind) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    fn expect_kind(
        &mut self,
        kind: SqlTokenKind,
        message: &str,
    ) -> Result<SqlToken, SqlParseError> {
        if self.peek().is_some_and(|token| token.kind == kind) {
            let token = self.tokens[self.cursor].clone();
            self.cursor += 1;
            Ok(token)
        } else {
            Err(self.error_here(message))
        }
    }

    fn expect_word(&mut self, message: &str) -> Result<SqlToken, SqlParseError> {
        self.expect_kind(SqlTokenKind::Word, message)
    }

    fn expect_operator(&mut self, expected: &str) -> Result<(), SqlParseError> {
        let token = self.expect_kind(SqlTokenKind::Operator, &format!("expected `{expected}`"))?;
        if token.text == expected {
            Ok(())
        } else {
            Err(error(format!("expected `{expected}`"), token.span))
        }
    }
}

fn error(message: impl Into<String>, span: Range<usize>) -> SqlParseError {
    SqlParseError {
        message: message.into(),
        span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{DbColumnSchema, DbSchemaSnapshot, DbTableSchema};

    fn snapshot() -> DbSchemaSnapshot {
        DbSchemaSnapshot::new(
            "./app.db".to_string(),
            1,
            vec![DbTableSchema {
                name: "users".to_string(),
                engine: None,
                columns: vec![
                    DbColumnSchema {
                        name: "id".to_string(),
                        logical_type: "i64".to_string(),
                        nullable: false,
                        ordinal: 0,
                        default: None,
                        native_type: None,
                    },
                    DbColumnSchema {
                        name: "name".to_string(),
                        logical_type: "string".to_string(),
                        nullable: true,
                        ordinal: 1,
                        default: None,
                        native_type: None,
                    },
                    DbColumnSchema {
                        name: "updated_at".to_string(),
                        logical_type: "instant".to_string(),
                        nullable: false,
                        ordinal: 2,
                        default: None,
                        native_type: None,
                    },
                ],
                primary_key: vec!["id".to_string()],
                unique: Vec::new(),
                foreign_keys: Vec::new(),
                indexes: Vec::new(),
                triggers: Vec::new(),
            }],
        )
    }

    #[test]
    fn validates_insert_update_and_delete_bindings() {
        let cases = [
            (
                "INSERT INTO users (id, name) VALUES (?, ?)",
                SqlExecKind::Insert,
                vec![Type::I64, Type::Optional(Box::new(Type::Str))],
                2,
            ),
            (
                "UPDATE users SET name = ? WHERE id = ?",
                SqlExecKind::Update,
                vec![Type::Str, Type::I64],
                1,
            ),
            (
                "DELETE FROM users WHERE id = ?",
                SqlExecKind::Delete,
                vec![Type::I64],
                0,
            ),
        ];
        for (sql, kind, parameters, assignment_count) in cases {
            let statement = parse_exec_sql(sql).unwrap();
            assert_eq!(statement.kind, kind);
            assert_eq!(statement.assignment_count, assignment_count);
            assert_eq!(statement.bindings.len(), parameters.len());
            assert_eq!(statement.table, "users");
            validate_exec_against_snapshot(&snapshot(), &statement, &parameters).unwrap();
        }
    }

    #[test]
    fn reports_the_offending_column_or_placeholder() {
        let unknown_sql = "UPDATE users SET missing = ? WHERE id = ?";
        let statement = parse_exec_sql(unknown_sql).unwrap();
        let error =
            validate_exec_against_snapshot(&snapshot(), &statement, &[Type::Str, Type::I64])
                .unwrap_err();
        assert_eq!(&unknown_sql[error.span], "missing");

        let wrong_type_sql = "DELETE FROM users WHERE id = ?";
        let statement = parse_exec_sql(wrong_type_sql).unwrap();
        let error =
            validate_exec_against_snapshot(&snapshot(), &statement, &[Type::Str]).unwrap_err();
        assert_eq!(&wrong_type_sql[error.span], "?");

        let count_error = validate_exec_against_snapshot(&snapshot(), &statement, &[]).unwrap_err();
        assert_eq!(&wrong_type_sql[count_error.span], "?");
    }

    #[test]
    fn accepts_current_timestamp_as_a_safe_update_expression() {
        let sql = "UPDATE users SET name = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?";
        let statement = parse_exec_sql(sql).unwrap();
        assert_eq!(statement.bindings.len(), 2);
        assert_eq!(statement.assignment_count, 1);
        validate_exec_against_snapshot(&snapshot(), &statement, &[Type::Str, Type::I64]).unwrap();
    }

    #[test]
    fn rejects_current_timestamp_for_non_instant_columns() {
        let sql = "UPDATE users SET id = CURRENT_TIMESTAMP WHERE id = ?";
        let statement = parse_exec_sql(sql).unwrap();
        let error =
            validate_exec_against_snapshot(&snapshot(), &statement, &[Type::I64]).unwrap_err();
        assert!(error.message.contains("temporal or text column"));
        assert_eq!(&sql[error.span], "id");
    }

    #[test]
    fn rejects_unbounded_mutation_literals_and_multiple_statements() {
        for sql in [
            "UPDATE users SET name = ?",
            "DELETE FROM users",
            "INSERT INTO users (name) VALUES ('raw')",
            "DELETE FROM users WHERE id = 7",
            "DELETE FROM users WHERE id = ?; DELETE FROM users WHERE id = ?",
        ] {
            assert!(parse_exec_sql(sql).is_err(), "{sql}");
        }
    }

    #[test]
    fn rejects_duplicate_assignments_and_invalid_optional_values() {
        let sql = "UPDATE users SET name = ?, name = ? WHERE id = ?";
        let statement = parse_exec_sql(sql).unwrap();
        let error = validate_exec_against_snapshot(
            &snapshot(),
            &statement,
            &[Type::Str, Type::Str, Type::I64],
        )
        .unwrap_err();
        assert_eq!(&sql[error.span], "name");

        let sql = "INSERT INTO users (id) VALUES (?)";
        let statement = parse_exec_sql(sql).unwrap();
        let error = validate_exec_against_snapshot(
            &snapshot(),
            &statement,
            &[Type::Optional(Box::new(Type::I64))],
        )
        .unwrap_err();
        assert_eq!(&sql[error.span], "?");
    }

    #[test]
    fn rejects_read_ddl_and_expression_statements_from_the_mutation_parser() {
        for sql in [
            "SELECT id FROM users",
            "CREATE TABLE users (id INTEGER)",
            "WITH selected AS (SELECT id FROM users) DELETE FROM users WHERE id = ?",
            "UPDATE users SET name = name || ? WHERE id = ?",
            "DELETE FROM users WHERE id = ? OR id = ?",
        ] {
            assert!(
                parse_exec_sql(sql).is_err(),
                "accepted unsupported SQL: {sql}"
            );
        }
    }

    #[test]
    fn validates_unknown_tables_and_missing_or_extra_parameters_at_precise_spans() {
        let unknown_table_sql = "DELETE FROM accounts WHERE id = ?";
        let unknown_table = parse_exec_sql(unknown_table_sql).unwrap();
        let error =
            validate_exec_against_snapshot(&snapshot(), &unknown_table, &[Type::I64]).unwrap_err();
        assert_eq!(&unknown_table_sql[error.span], "accounts");

        let sql = "UPDATE users SET name = ? WHERE id = ?";
        let statement = parse_exec_sql(sql).unwrap();
        let missing =
            validate_exec_against_snapshot(&snapshot(), &statement, &[Type::Str]).unwrap_err();
        assert_eq!(&sql[missing.span], "?");

        let extra = validate_exec_against_snapshot(
            &snapshot(),
            &statement,
            &[Type::Str, Type::I64, Type::I64],
        )
        .unwrap_err();
        assert_eq!(extra.span, sql.len()..sql.len());
    }
}
